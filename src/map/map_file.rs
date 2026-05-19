//! Parser for RA2 .map files (INI format with binary IsoMapPack5 terrain data).
//!
//! RA2 maps are INI files with special sections:
//! - `[Map]`: metadata (theater, size, local bounds)
//! - `[IsoMapPack5]`: base64-encoded, LZO-compressed terrain cell data
//!
//! Each terrain cell is 11 bytes: x(i16) y(i16) tile_index(i32)
//! sub_tile(u8) level(u8) ice_growth(u8). Cells describe the isometric tile grid.
//! (Confirmed by ModEnc IsoMapPack5 docs + FinalAlert2 EA source release.)
//!
//! ## Dependency rules
//! - Part of map/ — depends on assets/ (MIX archives), rules/ (INI parser), util/ (base64, lzo).

use std::collections::HashMap;
use std::path::Path;

use crate::assets::error::AssetError;
use crate::assets::mix_archive::MixArchive;
use crate::map::actions::{self, ActionMap};
use crate::map::basic::{self, BasicSection, SpecialFlagsSection};
use crate::map::briefing::{self, BriefingSection};
use crate::map::cell_tags::{self, CellTagMap};
use crate::map::entities::{self, MapEntity};
use crate::map::events::{self, EventMap};
use crate::map::overlay::{self, OverlayDataPack, OverlayEntry, TerrainObject};
use crate::map::preview::{self, PreviewSection};
use crate::map::tags::{self, TagMap};
use crate::map::trigger_graph::{self, TriggerGraph};
use crate::map::triggers::{self, TriggerMap};
use crate::map::tube_facts::TubeFact;
use crate::map::tubes;
use crate::map::variable_names::{self, LocalVariableMap};
use crate::map::waypoints::{self, Waypoint};
use crate::rules::error::RulesError;
use crate::rules::ini_parser::IniFile;
use crate::util::base64;
use crate::util::lzo::{self, LzoError};

/// Size of one terrain cell record in the decompressed IsoMapPack5 data.
const CELL_RECORD_SIZE: usize = 11;

/// Errors during map file parsing.
#[derive(Debug)]
pub enum MapError {
    Ini(RulesError),
    MissingSection { name: String },
    MissingField { section: String, key: String },
    MissingIsoMapPack,
    Base64(String),
    Lzo(LzoError),
    Asset(AssetError),
    CellDataTruncated { expected: usize, actual: usize },
    Io(std::io::Error),
}

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MapError::Ini(e) => write!(f, "Map INI error: {}", e),
            MapError::MissingSection { name } => write!(f, "Missing [{}] section in map", name),
            MapError::MissingField { section, key } => {
                write!(f, "Missing key '{}' in [{}]", key, section)
            }
            MapError::MissingIsoMapPack => write!(f, "No [IsoMapPack5] data in map"),
            MapError::Base64(e) => write!(f, "Base64 decode error: {}", e),
            MapError::Lzo(e) => write!(f, "LZO decompress error: {}", e),
            MapError::Asset(e) => write!(f, "Asset error: {}", e),
            MapError::CellDataTruncated { expected, actual } => {
                write!(
                    f,
                    "Cell data truncated: need {} bytes, got {}",
                    expected, actual
                )
            }
            MapError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for MapError {}

impl From<RulesError> for MapError {
    fn from(e: RulesError) -> Self {
        MapError::Ini(e)
    }
}
impl From<LzoError> for MapError {
    fn from(e: LzoError) -> Self {
        MapError::Lzo(e)
    }
}
impl From<AssetError> for MapError {
    fn from(e: AssetError) -> Self {
        MapError::Asset(e)
    }
}
impl From<std::io::Error> for MapError {
    fn from(e: std::io::Error) -> Self {
        MapError::Io(e)
    }
}

/// Map header extracted from the [Map] INI section.
#[derive(Debug, Clone)]
pub struct MapHeader {
    /// Theater name: "TEMPERATE", "SNOW", "URBAN", etc.
    pub theater: String,
    /// Full map width (from Size= 3rd value).
    pub width: u32,
    /// Full map height (from Size= 4th value).
    pub height: u32,
    /// Playable area left (from LocalSize= 1st value).
    pub local_left: u32,
    /// Playable area top (from LocalSize= 2nd value).
    pub local_top: u32,
    /// Playable area width (from LocalSize= 3rd value).
    pub local_width: u32,
    /// Playable area height (from LocalSize= 4th value).
    pub local_height: u32,
}

/// A pre-placed smudge entry from the map's `[Smudge]` section.
///
/// Parsed format: `Key=TYPENAME,X,Y,IsBaked`.
/// Entries with `IsBaked != 0` are SKIPPED at parse time (they represent
/// smudges already baked into the underlying tile graphic).
#[derive(Debug, Clone)]
pub struct MapSmudgeEntry {
    pub type_name: String,
    pub rx: u16,
    pub ry: u16,
}

/// A single isometric terrain cell from IsoMapPack5.
///
/// Layout per ModEnc + FinalAlert2 source: 11 bytes total.
/// tile_index is a flat cumulative index into the theater's tileset list.
/// -1 (0xFFFFFFFF) means "no tile" (clear ground at level 0).
#[derive(Debug, Clone)]
pub struct MapCell {
    /// Isometric X coordinate (can be negative in some edge cases).
    pub rx: u16,
    /// Isometric Y coordinate.
    pub ry: u16,
    /// Flat index into the theater's tile list (i32, NOT u16).
    /// -1 = no tile / clear ground. Cumulative across all TileSet sections.
    pub tile_index: i32,
    /// Sub-tile index within a multi-cell TMP template (0 for single-cell tiles).
    pub sub_tile: u8,
    /// Elevation level (0 = ground, higher = elevated). Each level ~15px visual shift.
    pub z: u8,
}

/// A parsed RA2 map file.
#[derive(Debug)]
pub struct MapFile {
    pub header: MapHeader,
    /// Parsed `[Basic]` metadata such as title and briefing hooks.
    pub basic: BasicSection,
    /// Parsed ordered mission briefing lines from `[Briefing]`.
    pub briefing: BriefingSection,
    /// Parsed preview metadata from `[Preview]` / `[PreviewPack]`.
    pub preview: PreviewSection,
    pub cells: Vec<MapCell>,
    /// Entity placements from [Units], [Infantry], [Structures], [Aircraft] sections.
    pub entities: Vec<MapEntity>,
    /// Overlay objects from [OverlayPack] + [OverlayDataPack] (ore, walls, fences, etc.).
    pub overlays: Vec<OverlayEntry>,
    /// Full `[OverlayDataPack]` bytes. Presence is tracked because missing packs
    /// do not overwrite bridge state bytes in `gamemd.exe`.
    pub overlay_data: OverlayDataPack,
    /// Pre-placed smudges from the map's `[Smudge]` section.
    /// `IsBaked != 0` entries are filtered at parse time.
    pub smudges: Vec<MapSmudgeEntry>,
    /// Terrain objects from [Terrain] section (trees, cacti, rocks).
    pub terrain_objects: Vec<TerrainObject>,
    /// Waypoint index -> cell coordinate mapping from [Waypoints].
    pub waypoints: HashMap<u32, Waypoint>,
    /// Cell coordinate -> tag id mapping from [CellTags].
    pub cell_tags: CellTagMap,
    /// Tag id -> raw tag record from [Tags].
    pub tags: TagMap,
    /// Trigger id -> raw trigger record from [Triggers].
    pub triggers: TriggerMap,
    /// Event id -> raw event record from [Events].
    pub events: EventMap,
    /// Action id -> raw action record from [Actions].
    pub actions: ActionMap,
    /// Local variable definitions from [VariableNames].
    pub local_variables: LocalVariableMap,
    /// Normalized trigger-link graph derived from CellTags/Tags/Triggers/Events/Actions.
    pub trigger_graph: TriggerGraph,
    /// Parsed `[SpecialFlags]` section (TiberiumGrows, TiberiumSpreads overrides).
    pub special_flags: SpecialFlagsSection,
    /// Explicit full TubeClass records parsed from `[Tubes]`.
    pub explicit_tubes: Vec<TubeFact>,
    /// Full parsed INI for accessing additional sections (e.g., [Houses]).
    pub ini: IniFile,
}

impl MapFile {
    /// Parse a map from raw INI bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, MapError> {
        let ini: IniFile = IniFile::from_bytes(data)?;
        let header: MapHeader = parse_header(&ini)?;
        let basic: BasicSection = basic::parse_basic_section(&ini);
        let special_flags: SpecialFlagsSection = basic::parse_special_flags_section(&ini);
        let briefing: BriefingSection = briefing::parse_briefing_section(&ini);
        let preview: PreviewSection = preview::parse_preview_section(&ini);
        let cells: Vec<MapCell> = parse_iso_map_pack(&ini)?;
        let entities: Vec<MapEntity> = entities::parse_map_entities(&ini);
        let overlay_packs = overlay::parse_overlay_packs(&ini);
        let terrain_objects: Vec<TerrainObject> = overlay::parse_terrain_objects(&ini);
        let smudges: Vec<MapSmudgeEntry> = parse_map_smudges(&ini);
        let waypoints: HashMap<u32, Waypoint> = waypoints::parse_waypoints(&ini);
        let cell_tags: CellTagMap = cell_tags::parse_cell_tags(&ini);
        let tags: TagMap = tags::parse_tags(&ini);
        let triggers: TriggerMap = triggers::parse_triggers(&ini);
        let events: EventMap = events::parse_events(&ini);
        let actions: ActionMap = actions::parse_actions(&ini);
        let local_variables: LocalVariableMap = variable_names::parse_local_variables(&ini);
        let explicit_tubes: Vec<TubeFact> = tubes::parse_tubes(&ini);
        let trigger_graph: TriggerGraph =
            trigger_graph::build_trigger_graph(&cell_tags, &tags, &triggers, &events, &actions);
        Ok(MapFile {
            header,
            basic,
            briefing,
            preview,
            cells,
            entities,
            overlays: overlay_packs.entries,
            overlay_data: overlay_packs.data,
            smudges,
            terrain_objects,
            waypoints,
            cell_tags,
            tags,
            triggers,
            events,
            actions,
            local_variables,
            trigger_graph,
            special_flags,
            explicit_tubes,
            ini,
        })
    }

    pub fn overlay_data_at(&self, rx: u16, ry: u16) -> u8 {
        self.overlay_data.byte_at(rx, ry)
    }

    pub fn has_overlay_data_pack(&self) -> bool {
        self.overlay_data.is_present()
    }
}

/// Load a map file from disk, auto-detecting MIX-wrapped vs raw INI.
///
/// Retail map files (`.mmx`, `.yro`, sometimes `.map`) are MIX archives
/// containing two entries: the actual map INI and a tiny `[MultiMaps]`
/// description stub. Editor-saved maps (`.map`, `.mpr`, `.yrm`) are raw INI
/// text. We dispatch on the first two header bytes — `00 00` is the
/// new-format MIX marker; anything else is treated as INI text.
pub fn load_from_path(path: &Path) -> Result<MapFile, MapError> {
    let bytes: Vec<u8> = std::fs::read(path)?;
    if is_mix_header(&bytes) {
        let archive: MixArchive = MixArchive::load(path)?;
        let id: i32 = pick_map_entry_id(&archive)?;
        let data: &[u8] = archive.get_by_id(id).ok_or(MapError::MissingIsoMapPack)?;
        MapFile::from_bytes(data)
    } else {
        MapFile::from_bytes(&bytes)
    }
}

/// Backwards-compatible alias for callers still naming the MIX path explicitly.
pub fn load_mmx(path: &Path) -> Result<MapFile, MapError> {
    load_from_path(path)
}

/// New-format MIX marker: first two bytes are `0x00 0x00`.
fn is_mix_header(bytes: &[u8]) -> bool {
    bytes.len() >= 2 && bytes[0] == 0 && bytes[1] == 0
}

/// Pick the inner MIX entry that contains the actual map INI.
///
/// Retail map MIXes always pack two entries: the real map (~100 KB+) and a
/// ~120-byte `[MultiMaps]` description stub. We try entries in descending
/// size order and return the first one whose bytes parse as INI containing
/// a `[Map]` section.
fn pick_map_entry_id(archive: &MixArchive) -> Result<i32, MapError> {
    let mut entries: Vec<_> = archive.entries().to_vec();
    if entries.is_empty() {
        return Err(MapError::MissingIsoMapPack);
    }
    entries.sort_by(|a, b| b.size.cmp(&a.size));
    for entry in &entries {
        let Some(data) = archive.get_by_id(entry.id) else {
            continue;
        };
        if let Ok(ini) = IniFile::from_bytes(data) {
            if ini.section("Map").is_some() {
                return Ok(entry.id);
            }
        }
    }
    Err(MapError::MissingIsoMapPack)
}

/// Extract the [Map] header fields.
fn parse_header(ini: &IniFile) -> Result<MapHeader, MapError> {
    let map_section = ini
        .section("Map")
        .ok_or(MapError::MissingSection { name: "Map".into() })?;

    let theater: String = map_section
        .get("Theater")
        .ok_or(MapError::MissingField {
            section: "Map".into(),
            key: "Theater".into(),
        })?
        .to_uppercase();

    // Size=left,top,width,height
    let size_str: &str = map_section.get("Size").ok_or(MapError::MissingField {
        section: "Map".into(),
        key: "Size".into(),
    })?;
    let size_parts: Vec<u32> = parse_csv_u32(size_str, "Size")?;
    if size_parts.len() < 4 {
        return Err(MapError::MissingField {
            section: "Map".into(),
            key: "Size (need 4 values)".into(),
        });
    }

    // LocalSize=left,top,width,height
    let local_str: &str = map_section.get("LocalSize").ok_or(MapError::MissingField {
        section: "Map".into(),
        key: "LocalSize".into(),
    })?;
    let local_parts: Vec<u32> = parse_csv_u32(local_str, "LocalSize")?;
    if local_parts.len() < 4 {
        return Err(MapError::MissingField {
            section: "Map".into(),
            key: "LocalSize (need 4 values)".into(),
        });
    }

    Ok(MapHeader {
        theater,
        width: size_parts[2],
        height: size_parts[3],
        local_left: local_parts[0],
        local_top: local_parts[1],
        local_width: local_parts[2],
        local_height: local_parts[3],
    })
}

/// Parse comma-separated u32 values from an INI value string.
fn parse_csv_u32(s: &str, field_name: &str) -> Result<Vec<u32>, MapError> {
    s.split(',')
        .map(|part| {
            part.trim()
                .parse::<u32>()
                .map_err(|_| MapError::MissingField {
                    section: "Map".into(),
                    key: format!("{} (invalid number: '{}')", field_name, part.trim()),
                })
        })
        .collect()
}

/// Extract and decode the [IsoMapPack5] terrain data.
///
/// 1. Concatenate all numbered key values from the section.
/// 2. Base64 decode the concatenated string.
/// 3. LZO decompress the chunks.
/// 4. Parse 11-byte terrain cell records.
fn parse_iso_map_pack(ini: &IniFile) -> Result<Vec<MapCell>, MapError> {
    let section = ini
        .section("IsoMapPack5")
        .ok_or(MapError::MissingIsoMapPack)?;

    // Concatenate all values in key order (keys are "1", "2", "3", ...).
    let mut b64_data: String = String::new();
    for key in section.keys() {
        if let Some(val) = section.get(key) {
            b64_data.push_str(val);
        }
    }

    if b64_data.is_empty() {
        return Err(MapError::MissingIsoMapPack);
    }

    // Base64 decode → LZO decompress.
    let compressed: Vec<u8> = base64::base64_decode(&b64_data).map_err(MapError::Base64)?;
    let decompressed: Vec<u8> = lzo::decompress_chunks(&compressed)?;

    // Parse 11-byte cell records.
    let cell_count: usize = decompressed.len() / CELL_RECORD_SIZE;
    let mut cells: Vec<MapCell> = Vec::with_capacity(cell_count);

    for i in 0..cell_count {
        let offset: usize = i * CELL_RECORD_SIZE;
        if offset + CELL_RECORD_SIZE > decompressed.len() {
            break;
        }
        let d: &[u8] = &decompressed[offset..offset + CELL_RECORD_SIZE];

        let rx: u16 = u16::from_le_bytes([d[0], d[1]]);
        let ry: u16 = u16::from_le_bytes([d[2], d[3]]);
        let raw_tile_index: i32 = i32::from_le_bytes([d[4], d[5], d[6], d[7]]);
        let sub_tile: u8 = d[8];
        let z: u8 = d[9];
        // d[10] = ice_growth (TS Snow only, always 0 in RA2)

        // Skip termination sentinel: x=0, y=0 marks end of data.
        if rx == 0 && ry == 0 {
            continue;
        }

        // Normalize "no tile" sentinels to -1. Westwood-saved maps use
        // -1 (0xFFFFFFFF) but FinalAlert2-saved .yro maps write 0xFFFF
        // (only the low u16 set), which would otherwise parse as +65535
        // and be treated as a real tile_id beyond every theater's lookup.
        let tile_index: i32 = if raw_tile_index == -1 || raw_tile_index == 0xFFFF {
            -1
        } else {
            raw_tile_index
        };

        cells.push(MapCell {
            rx,
            ry,
            tile_index,
            sub_tile,
            z,
        });
    }

    // Diagnostic: tile_index distribution. Lets a reader of the load logs see
    // how high a map's IsoMapPack5 reaches vs. what the theater INI defines.
    let mut min_pos: i32 = i32::MAX;
    let mut max_idx: i32 = -1;
    let mut no_tile: usize = 0;
    let mut distinct: std::collections::HashSet<i32> = std::collections::HashSet::new();
    for c in &cells {
        if c.tile_index < 0 {
            no_tile += 1;
        } else {
            if c.tile_index < min_pos {
                min_pos = c.tile_index;
            }
            if c.tile_index > max_idx {
                max_idx = c.tile_index;
            }
        }
        distinct.insert(c.tile_index);
    }
    log::info!(
        "IsoMapPack5: {} cells, {} no-tile, tile_index min={}, max={}, distinct={}",
        cells.len(),
        no_tile,
        if min_pos == i32::MAX { -1 } else { min_pos },
        max_idx,
        distinct.len()
    );

    Ok(cells)
}

/// Parse pre-placed smudges from the map's `[Smudge]` section.
///
/// Section uses numbered keys: `0=TYPENAME,X,Y,IsBaked`. Entries with
/// `IsBaked != 0` are skipped (they're baked into the terrain graphic and
/// shouldn't be re-instantiated as runtime smudges).
fn parse_map_smudges(ini: &IniFile) -> Vec<MapSmudgeEntry> {
    let Some(section) = ini.section("Smudge") else {
        return Vec::new();
    };
    let mut out: Vec<MapSmudgeEntry> = Vec::new();
    for value in section.get_values() {
        let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
        if parts.len() < 4 {
            continue;
        }
        let is_baked: i32 = parts[3].parse::<i32>().unwrap_or(0);
        if is_baked != 0 {
            continue;
        }
        let rx: u16 = match parts[1].parse::<u16>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ry: u16 = match parts[2].parse::<u16>() {
            Ok(v) => v,
            Err(_) => continue,
        };
        out.push(MapSmudgeEntry {
            type_name: parts[0].to_uppercase(),
            rx,
            ry,
        });
    }
    out
}

#[cfg(test)]
mod smudge_parse_tests {
    use super::*;

    #[test]
    fn parses_smudge_section_skips_isbaked_nonzero() {
        let ini = IniFile::from_bytes(
            b"[Smudge]\n\
              0=CR1,5,6,0\n\
              1=BURN01,7,8,1\n\
              2=CR2,9,10,0\n",
        )
        .unwrap();
        let smudges = parse_map_smudges(&ini);
        assert_eq!(smudges.len(), 2);
        assert_eq!(smudges[0].type_name, "CR1");
        assert_eq!(smudges[0].rx, 5);
        assert_eq!(smudges[0].ry, 6);
        assert_eq!(smudges[1].type_name, "CR2");
    }

    #[test]
    fn handles_missing_section() {
        let ini = IniFile::from_bytes(b"[Other]\nFoo=Bar\n").unwrap();
        let smudges = parse_map_smudges(&ini);
        assert!(smudges.is_empty());
    }

    #[test]
    fn rejects_malformed_entries() {
        let ini = IniFile::from_bytes(
            b"[Smudge]\n\
              0=CR1,5,6\n\
              1=,5,6,0\n\
              2=CR1,X,6,0\n\
              3=CR1,5,6,0\n",
        )
        .unwrap();
        let smudges = parse_map_smudges(&ini);
        // Only entry 3 fully valid; entry 1 has empty type_name (kept as "" — uppercase of empty).
        // Entry 0 fails (only 3 parts), entry 2 fails (X not a number).
        // Entry 1: empty type_name accepted by parser but won't resolve to a registered SmudgeType later.
        assert_eq!(smudges.len(), 2);
        assert_eq!(smudges[0].type_name, "");
        assert_eq!(smudges[1].type_name, "CR1");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_header_from_ini() {
        let text: &str = "\
[Map]
Theater=TEMPERATE
Size=0,0,100,100
LocalSize=2,4,96,92
";
        let ini: IniFile = IniFile::from_str(text);
        let header: MapHeader = parse_header(&ini).expect("Should parse header");
        assert_eq!(header.theater, "TEMPERATE");
        assert_eq!(header.width, 100);
        assert_eq!(header.height, 100);
        assert_eq!(header.local_left, 2);
        assert_eq!(header.local_top, 4);
        assert_eq!(header.local_width, 96);
        assert_eq!(header.local_height, 92);
    }

    #[test]
    fn test_parse_cells_from_raw_bytes() {
        // Build a fake 11-byte cell record matching the correct format:
        // i16 x, i16 y, i32 tile_index, u8 sub_tile, u8 level, u8 ice_growth
        let mut cell_bytes: Vec<u8> = Vec::new();
        cell_bytes.extend_from_slice(&10u16.to_le_bytes()); // rx
        cell_bytes.extend_from_slice(&20u16.to_le_bytes()); // ry
        cell_bytes.extend_from_slice(&5i32.to_le_bytes()); // tile_index (i32!)
        cell_bytes.push(3); // sub_tile
        cell_bytes.push(2); // z (level)
        cell_bytes.push(0); // ice_growth

        assert_eq!(cell_bytes.len(), CELL_RECORD_SIZE);

        let d: &[u8] = &cell_bytes[0..CELL_RECORD_SIZE];
        let rx: u16 = u16::from_le_bytes([d[0], d[1]]);
        let ry: u16 = u16::from_le_bytes([d[2], d[3]]);
        let tile_index: i32 = i32::from_le_bytes([d[4], d[5], d[6], d[7]]);
        let sub_tile: u8 = d[8];
        let z: u8 = d[9];

        assert_eq!(rx, 10);
        assert_eq!(ry, 20);
        assert_eq!(tile_index, 5);
        assert_eq!(sub_tile, 3);
        assert_eq!(z, 2);
    }

    #[test]
    fn test_missing_map_section() {
        let text: &str = "[General]\nKey=Value\n";
        let ini: IniFile = IniFile::from_str(text);
        let result = parse_header(&ini);
        assert!(result.is_err());
    }
}
