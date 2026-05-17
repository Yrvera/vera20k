//! art.ini parsing and art-resolution helpers.
//!
//! This module is intentionally split into three layers:
//! - explicit rules/art data resolution
//! - original-engine filename conventions we intentionally keep
//! - repo-only fallback hooks that stay outside this module
//!
//! `ArtRegistry` owns only parsed metadata and exact ID/section lookups.
//! Filename candidate generation lives in free functions below so render code can
//! use convention helpers without re-mixing them into metadata resolution.

use std::collections::HashMap;

use crate::rules::flh::{Flh, parse_flh};
use crate::rules::ini_parser::{IniFile, IniSection};

/// Per-object art configuration parsed from an art.ini section.
#[derive(Debug, Clone)]
pub struct ArtEntry {
    /// Image filename override (from art.ini `Image=` key).
    /// None means use the section name as the filename base.
    pub image: Option<String>,
    /// Sidebar cameo/image override from `Cameo=`.
    pub cameo: Option<String>,
    /// Alternate sidebar cameo/image override from `AltCameo=`.
    pub alt_cameo: Option<String>,
    /// Replace 2nd char of filename with theater-specific letter.
    pub new_theater: bool,
    /// Use theater file extension (e.g., `.tem`) instead of `.shp`.
    /// Set by `Theater=yes` in art.ini. Distinct from `NewTheater=`.
    pub theater: bool,
    pub scorch: bool,
    pub crater: bool,
    pub force_big_craters: bool,
    /// SHP frame 0's visible-content bounding-rect width, in pixels.
    /// Used by the smudge dispatcher as a damage-tier proxy for size selection.
    /// Default 30 — matches the original engine's uncached first-call fallback;
    /// replaced with the actual SHP frame width by `populate_anim_frame_dims`
    /// for anims with a Crater/Scorch/ForceBigCraters spawn flag.
    pub frame_width: u16,
    /// SHP frame 0's visible-content bounding-rect height, in pixels.
    /// See `frame_width`.
    pub frame_height: u16,
    /// Render as VXL+HVA model (true) or SHP sprite (false).
    pub voxel: bool,
    /// Optional voxel turret/barrel forward/backward alignment tweak.
    pub turret_offset: i32,
    /// Extra Y pixel offset for sprite rendering.
    pub y_draw_offset: i32,
    /// Extra X pixel offset for sprite rendering.
    pub x_draw_offset: i32,
    /// Building animation overlays (ActiveAnim, IdleAnim, etc.).
    pub building_anims: Vec<BuildingAnimConfig>,
    /// Building foundation footprint (e.g., "4x4", "2x2").
    pub foundation: Option<String>,
    /// BibShape: separate SHP for the ground-level pad/bib under a building.
    pub bib_shape: Option<String>,
    /// Custom palette override from art.ini `Palette=`.
    /// Stored as a palette base name without `.pal`.
    pub palette: Option<String>,
    /// Infantry animation sequence definition name (e.g., "ConSequence").
    /// Points to a `[ConSequence]`-style section in art.ini with frame layouts.
    pub sequence: Option<String>,
    /// Infantry `Crawls=` art flag. Merged into ObjectType for sim stance speed.
    pub crawls: bool,
    /// Muzzle offset for primary weapon fire (from art.ini `PrimaryFireFLH=`).
    pub primary_fire_flh: Flh,
    /// Muzzle offset for secondary weapon fire (from art.ini `SecondaryFireFLH=`).
    pub secondary_fire_flh: Flh,
    /// Elite-rank override for primary fire offset (from art.ini `ElitePrimaryFireFLH=`).
    /// None means use `primary_fire_flh`.
    pub elite_primary_fire_flh: Option<Flh>,
    /// Elite-rank override for secondary fire offset (from art.ini `EliteSecondaryFireFLH=`).
    /// None means use `secondary_fire_flh`.
    pub elite_secondary_fire_flh: Option<Flh>,
    /// SHP vehicle: walk animation frame count per facing (from `WalkFrames=`).
    pub walk_frames: Option<u16>,
    /// SHP vehicle: firing animation frame count per facing (from `FiringFrames=`).
    pub firing_frames: Option<u16>,
    /// SHP vehicle: standing animation frame count per facing (from `StandingFrames=`).
    pub standing_frames: Option<u16>,
    /// SHP vehicle: number of facing directions (from `Facings=`, default 8).
    pub shp_facings: u8,
    /// Weapon discharge delay in animation frames (from `FireUp=`, default 0).
    /// Distinct from the `FireUp` sequence action in infantry sequences.
    pub fire_up: u8,
    /// Infantry primary prone discharge frame (`FireProne=`).
    /// Defaults to `FireUp` when absent, matching the InfantryType read fallback.
    pub fire_prone: u8,
    /// Infantry secondary standing discharge frame (`SecondaryFire=`).
    /// Defaults to `FireUp` when absent.
    pub secondary_fire: u8,
    /// Infantry secondary prone/deploy discharge frame (`SecondaryProne=`).
    /// Defaults to `SecondaryFire` when absent.
    pub secondary_prone: u8,
    /// Extra ambient light added to this building's cell (ExtraLight= in art.ini).
    /// Positive = brighten, negative = darken. Scale: 1000 ≈ 1.0 brightness unit.
    /// Retail values: GADPSA=350, GAICBM=-100.
    pub extra_light: i32,
    /// Harvester queueing cell offset from building origin (QueueingCell= in art.ini).
    /// Where miners wait outside the dock when it is occupied. e.g. `(4, 1)` for GAREFN.
    pub queueing_cell: Option<(u16, u16)>,
    /// All `DockingOffset%d` entries actually present in this art.ini section,
    /// in index order. Up to 8 (defensive ceiling for mod safety; retail uses
    /// up to 4). The art→rules merge in [`crate::rules::ruleset`] is what
    /// applies `NumberOfDocks`-aware sizing — see `ObjectType::pads` for the
    /// merged shape.
    pub pads: Vec<crate::rules::object_type::DockPad>,
    /// Pixel offsets where fire/smoke overlays appear when building health < ConditionYellow.
    /// Parsed from DamageFireOffset0=X,Y .. DamageFireOffset7=X,Y in art.ini. Max 8.
    pub damage_fire_offsets: Vec<(i32, i32)>,
    /// Building height in cell-height units (from `Height=` in art.ini).
    /// Used for health bar vertical positioning: Dimension2.Z = (fh + Height) * 256
    /// leptons, projected via CoordsToScreen as z_screen = (fh + Height) * 7.5 px.
    pub height: i32,
    /// Fire port pixel offsets for garrison muzzle flashes.
    /// Parsed from `MuzzleFlash0=X,Y` through `MuzzleFlash9=X,Y` in art.ini.
    /// Each entry is a screen-space offset from the building's center.
    pub muzzle_flash_positions: Vec<(i32, i32)>,
    /// Cells added to the rectangular foundation (AddOccupy1..N from art.ini).
    /// Signed offsets from the building's origin (rx, ry) — negative = west/north.
    pub add_occupy: Vec<(i16, i16)>,
    /// Cells removed from the rectangular foundation (RemoveOccupy1..N from art.ini).
    pub remove_occupy: Vec<(i16, i16)>,
    /// Middle integer of `Deploy=<start>,<frames>,<rate>` in the infantry
    /// sequence section referenced by `sequence`. `None` when the sequence
    /// is undefined or doesn't have a `Deploy` entry. Drives the per-type
    /// Deploying-phase duration via `sim::deploy::compute_anim_ticks`.
    pub deploy_frames: Option<u16>,
    /// Middle integer of `Undeploy=<start>,<frames>,<rate>` in the sequence.
    pub undeploy_frames: Option<u16>,
    /// Middle integer of `DeployedFire=<start>,<frames>,<rate>` in the sequence.
    pub deployed_fire_frames: Option<u16>,
}

/// Parse a sequence entry value of the form `<start>,<frames>,<rate>` and
/// return the middle integer (frame count). Returns `None` on malformed input.
fn parse_sequence_frames(value: &str) -> Option<u16> {
    let mut parts = value.split(',').map(str::trim);
    let _start = parts.next()?;
    parts.next()?.parse::<u16>().ok()
}

/// Which category of building animation this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingAnimKind {
    Active,
    ActiveGarrisoned,
    Idle,
    Super,
    Special,
    Production,
}

/// Configuration for a building animation overlay (ActiveAnim, IdleAnim, etc.).
#[derive(Debug, Clone)]
pub struct BuildingAnimConfig {
    pub anim_type: String,
    pub kind: BuildingAnimKind,
    /// True for the base key (e.g., `ActiveAnim`), false for suffixed variants
    /// (`ActiveAnimTwo`, `Three`, `Four`).  Used to gate the primary active anim
    /// on building ownership while letting secondary anims (flags) play always.
    pub is_primary: bool,
    pub x: i32,
    pub y: i32,
    pub y_sort: i32,
    pub z_adjust: i32,
    pub loop_start: u16,
    pub loop_end: u16,
    pub loop_count: i32,
    pub rate: u16,
    pub start_frame: u16,
    pub ping_pong: bool,
}

/// Convert art.ini `Rate=` value to milliseconds per frame.
///
/// The engine stores `900 / INI_Rate` as the game-logic-frame delay
/// (integer division). RA2 game logic runs at ~15fps at normal speed
/// (one game frame = ~67ms): `delay_ms = (900 / rate) * 1000 / 15`.
pub fn art_rate_to_delay_ms(ini_rate: i32) -> u32 {
    if ini_rate < 1 {
        return 0;
    }
    let delay_frames: u32 = 900 / ini_rate as u32;
    (delay_frames * 1000 / 15).max(1)
}

/// Default ms-per-frame when art.ini section has no `Rate=` key.
/// Matches gamemd constructor default of 1 game frame at ~15fps.
pub const DEFAULT_ART_RATE_MS: u16 = 67;

/// Exact object-art resolution from rules + art metadata.
///
/// This is the data-driven layer only. It does not generate filenames.
#[derive(Debug, Clone)]
pub struct ResolvedObjectArt<'a> {
    /// Base art identity from rules `Image=` or the object type id.
    pub base_art_id: String,
    /// Final image id after art.ini `Image=` override.
    pub image_id: String,
    /// Section id whose metadata should be used for overlays/bibs/anims.
    pub metadata_section_id: String,
    /// Parsed art entry for `metadata_section_id`, if present.
    pub entry: Option<&'a ArtEntry>,
}

/// Lookup table for art.ini rendering data.
#[derive(Debug, Clone)]
pub struct ArtRegistry {
    /// image_id (uppercase) -> ArtEntry.
    entries: HashMap<String, ArtEntry>,
}

/// Hardcoded filename prefixes that always receive `NewTheater` treatment
/// regardless of the `NewTheater=` INI key.
const NEW_THEATER_PREFIXES: &[&str] = &["GA", "GT", "NA", "NT", "CA", "CT"];

/// `repo-derived`: theater name -> replacement letter for `NewTheater`.
const THEATER_LETTERS: &[(&str, char)] = &[
    ("TEMPERATE", 'T'),
    ("SNOW", 'A'),
    ("URBAN", 'U'),
    ("DESERT", 'D'),
    ("LUNAR", 'L'),
    ("NEWURBAN", 'N'),
];

/// `repo-derived`: generic fallback letter used by original-style building art.
const NEW_THEATER_GENERIC_LETTER: char = 'G';

impl ArtRegistry {
    /// Parse all sections from an art.ini IniFile into the registry.
    pub fn from_ini(ini: &IniFile) -> Self {
        let mut entries: HashMap<String, ArtEntry> = HashMap::new();

        for section_name in ini.section_names() {
            let section = match ini.section(section_name) {
                Some(s) => s,
                None => continue,
            };

            let image: Option<String> = section.get("Image").map(|s| s.to_string());
            let cameo: Option<String> = section.get("Cameo").map(|s| s.to_string());
            let alt_cameo: Option<String> = section.get("AltCameo").map(|s| s.to_string());
            let new_theater: bool = section.get_bool("NewTheater").unwrap_or(false);
            let theater: bool = section.get_bool("Theater").unwrap_or(false);
            let scorch: bool = section.get_bool("Scorch").unwrap_or(false);
            let crater: bool = section.get_bool("Crater").unwrap_or(false);
            let force_big_craters: bool = section.get_bool("ForceBigCraters").unwrap_or(false);
            let voxel: bool = section.get_bool("Voxel").unwrap_or(false);
            let turret_offset: i32 = section.get_i32("TurretOffset").unwrap_or(0);
            let y_draw_offset: i32 = section.get_i32("YDrawOffset").unwrap_or(0);
            let x_draw_offset: i32 = section.get_i32("XDrawOffset").unwrap_or(0);
            let building_anims: Vec<BuildingAnimConfig> = parse_building_anims(section, ini);
            let foundation: Option<String> = section
                .get("Foundation")
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            let bib_shape: Option<String> = section
                .get("BibShape")
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            let palette: Option<String> = section
                .get("Palette")
                .filter(|s| !s.is_empty())
                .map(|s| s.to_ascii_lowercase());
            let sequence: Option<String> = section
                .get("Sequence")
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            // Pull per-phase frame counts from the referenced sequence section
            // (e.g. GuardianGISequence::Deploy=300,15,0 -> deploy_frames=15).
            let (deploy_frames, undeploy_frames, deployed_fire_frames) = sequence
                .as_deref()
                .and_then(|seq_name| ini.section(seq_name))
                .map(|seq_section| {
                    (
                        seq_section.get("Deploy").and_then(parse_sequence_frames),
                        seq_section.get("Undeploy").and_then(parse_sequence_frames),
                        seq_section
                            .get("DeployedFire")
                            .and_then(parse_sequence_frames),
                    )
                })
                .unwrap_or((None, None, None));
            let crawls = section.get_bool("Crawls").unwrap_or(false);
            let primary_fire_flh: Flh = parse_flh(section.get("PrimaryFireFLH"));
            let secondary_fire_flh: Flh = parse_flh(section.get("SecondaryFireFLH"));
            let elite_primary_fire_flh: Option<Flh> = section
                .get("ElitePrimaryFireFLH")
                .map(|v| parse_flh(Some(v)));
            let elite_secondary_fire_flh: Option<Flh> = section
                .get("EliteSecondaryFireFLH")
                .map(|v| parse_flh(Some(v)));

            // SHP vehicle frame tags (only meaningful when Voxel=no for vehicles).
            let walk_frames: Option<u16> = section.get_i32("WalkFrames").map(|v| v.max(0) as u16);
            let firing_frames: Option<u16> =
                section.get_i32("FiringFrames").map(|v| v.max(0) as u16);
            let standing_frames: Option<u16> =
                section.get_i32("StandingFrames").map(|v| v.max(0) as u16);
            let shp_facings: u8 = section
                .get_i32("Facings")
                .map(|v| v.clamp(1, 32) as u8)
                .unwrap_or(8);
            let fire_up: u8 = section
                .get_i32("FireUp")
                .map(|v| v.max(0) as u8)
                .unwrap_or(0);
            let fire_prone: u8 = section
                .get_i32("FireProne")
                .map(|v| v.max(0) as u8)
                .unwrap_or(fire_up);
            let secondary_fire: u8 = section
                .get_i32("SecondaryFire")
                .map(|v| v.max(0) as u8)
                .unwrap_or(fire_up);
            let secondary_prone: u8 = section
                .get_i32("SecondaryProne")
                .map(|v| v.max(0) as u8)
                .unwrap_or(secondary_fire);
            let extra_light: i32 = section.get_i32("ExtraLight").unwrap_or(0);
            let queueing_cell: Option<(u16, u16)> = section.get("QueueingCell").and_then(|s| {
                let mut parts = s.split(',');
                let x = parts.next()?.trim().parse::<u16>().ok()?;
                let y = parts.next()?.trim().parse::<u16>().ok()?;
                Some((x, y))
            });
            // Multi-pad parser: read DockingOffset0..7 from art.ini.
            // Over-reads here; the art→rules merge in ruleset.rs truncates or
            // zero-pads to match rules.ini NumberOfDocks. 8 is a defensive
            // ceiling for mod safety (retail uses up to 4).
            let pads: Vec<crate::rules::object_type::DockPad> = (0..8)
                .filter_map(|i| {
                    let key = format!("DockingOffset{}", i);
                    section.get(&key).and_then(|s| {
                        let mut parts = s.split(',');
                        let x = parts.next()?.trim().parse::<i32>().ok()?;
                        let y = parts.next()?.trim().parse::<i32>().ok()?;
                        let z = parts
                            .next()
                            .and_then(|v| v.trim().parse::<i32>().ok())
                            .unwrap_or(0);
                        Some(crate::rules::object_type::DockPad {
                            lepton_offset: (x, y, z),
                        })
                    })
                })
                .collect();
            let damage_fire_offsets: Vec<(i32, i32)> = {
                let mut offsets = Vec::new();
                for i in 0..8 {
                    let key = format!("DamageFireOffset{}", i);
                    if let Some(val) = section.get(&key) {
                        let mut parts = val.split(',');
                        if let (Some(x), Some(y)) = (
                            parts.next().and_then(|s| s.trim().parse::<i32>().ok()),
                            parts.next().and_then(|s| s.trim().parse::<i32>().ok()),
                        ) {
                            offsets.push((x, y));
                        }
                    } else {
                        break;
                    }
                }
                offsets
            };
            let height: i32 = section.get_i32("Height").unwrap_or(0);
            let muzzle_flash_positions: Vec<(i32, i32)> = {
                let mut positions = Vec::new();
                for i in 0..10 {
                    let key = format!("MuzzleFlash{}", i);
                    if let Some(val) = section.get(&key) {
                        let mut parts = val.split(',');
                        if let (Some(x), Some(y)) = (
                            parts.next().and_then(|s| s.trim().parse::<i32>().ok()),
                            parts.next().and_then(|s| s.trim().parse::<i32>().ok()),
                        ) {
                            positions.push((x, y));
                        }
                    } else {
                        break;
                    }
                }
                positions
            };
            let add_occupy: Vec<(i16, i16)> = {
                let mut offsets = Vec::new();
                for i in 1..=8 {
                    let key = format!("AddOccupy{}", i);
                    if let Some(val) = section.get(&key) {
                        let mut parts = val.split(',');
                        if let (Some(x), Some(y)) = (
                            parts.next().and_then(|s| s.trim().parse::<i16>().ok()),
                            parts.next().and_then(|s| s.trim().parse::<i16>().ok()),
                        ) {
                            offsets.push((x, y));
                        }
                    } else {
                        break;
                    }
                }
                offsets
            };
            let remove_occupy: Vec<(i16, i16)> = {
                let mut offsets = Vec::new();
                for i in 1..=8 {
                    let key = format!("RemoveOccupy{}", i);
                    if let Some(val) = section.get(&key) {
                        let mut parts = val.split(',');
                        if let (Some(x), Some(y)) = (
                            parts.next().and_then(|s| s.trim().parse::<i16>().ok()),
                            parts.next().and_then(|s| s.trim().parse::<i16>().ok()),
                        ) {
                            offsets.push((x, y));
                        }
                    } else {
                        break;
                    }
                }
                offsets
            };

            entries.insert(
                section_name.to_uppercase(),
                ArtEntry {
                    image,
                    cameo,
                    alt_cameo,
                    new_theater,
                    theater,
                    scorch,
                    crater,
                    force_big_craters,
                    frame_width: 30,
                    frame_height: 30,
                    voxel,
                    turret_offset,
                    y_draw_offset,
                    x_draw_offset,
                    building_anims,
                    foundation,
                    bib_shape,
                    palette,
                    sequence,
                    crawls,
                    primary_fire_flh,
                    secondary_fire_flh,
                    elite_primary_fire_flh,
                    elite_secondary_fire_flh,
                    walk_frames,
                    firing_frames,
                    standing_frames,
                    shp_facings,
                    fire_up,
                    fire_prone,
                    secondary_fire,
                    secondary_prone,
                    extra_light,
                    queueing_cell,
                    pads,
                    damage_fire_offsets,
                    height,
                    muzzle_flash_positions,
                    add_occupy,
                    remove_occupy,
                    deploy_frames,
                    undeploy_frames,
                    deployed_fire_frames,
                },
            );
        }

        log::info!("ArtRegistry: {} entries loaded from art.ini", entries.len());
        ArtRegistry { entries }
    }

    /// Create an empty registry (used when art.ini is unavailable).
    pub fn empty() -> Self {
        ArtRegistry {
            entries: HashMap::new(),
        }
    }

    /// Look up art entry for an image ID (case-insensitive).
    pub fn get(&self, image_id: &str) -> Option<&ArtEntry> {
        self.entries.get(&image_id.to_uppercase())
    }

    /// Number of entries in the registry.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get (x_draw_offset, y_draw_offset) for an object type. Returns (0, 0) if unknown.
    pub fn draw_offsets(&self, type_id: &str) -> (i32, i32) {
        self.get(type_id)
            .map(|e| (e.x_draw_offset, e.y_draw_offset))
            .unwrap_or((0, 0))
    }

    /// Resolve exact object-art identities from rules/art data only.
    pub fn resolve_object_art<'a>(
        &'a self,
        type_id: &str,
        rules_image: &str,
    ) -> ResolvedObjectArt<'a> {
        let type_upper: String = type_id.to_uppercase();
        let base_art_id: String = normalize_id(rules_image).unwrap_or_else(|| type_upper.clone());
        let image_id: String = self
            .get(&base_art_id)
            .and_then(|entry| normalize_id(entry.image.as_deref().unwrap_or_default()))
            .unwrap_or_else(|| base_art_id.clone());

        let metadata_section_id: String = if self.get(&base_art_id).is_some() {
            base_art_id.clone()
        } else if self.get(&type_upper).is_some() {
            type_upper.clone()
        } else {
            image_id.clone()
        };
        let entry: Option<&ArtEntry> = self.get(&metadata_section_id);

        ResolvedObjectArt {
            base_art_id,
            image_id,
            metadata_section_id,
            entry,
        }
    }

    /// Resolve the art metadata section for an object.
    pub fn resolve_metadata_entry<'a>(
        &'a self,
        type_id: &str,
        rules_image: &str,
    ) -> Option<&'a ArtEntry> {
        self.resolve_object_art(type_id, rules_image).entry
    }

    /// Resolve the effective image id for an object.
    pub fn resolve_effective_image_id(&self, type_id: &str, rules_image: &str) -> String {
        self.resolve_object_art(type_id, rules_image).image_id
    }

    /// Resolve the declared cameo id for an object.
    ///
    /// This stays in the exact-resolution layer: it only reads declared keys and
    /// falls back to the resolved image id. `ICON` filename guessing lives elsewhere.
    pub fn resolve_declared_cameo_id(&self, type_id: &str, rules_image: &str) -> String {
        let resolved: ResolvedObjectArt<'_> = self.resolve_object_art(type_id, rules_image);
        let type_upper: String = type_id.to_uppercase();

        // Check type-specific section first — e.g. [BFRT] declares its own Cameo
        // even though Image=SREF points to the Prism Tank's art section.
        for key in [type_upper.as_str(), resolved.image_id.as_str()] {
            if let Some(entry) = self.get(key) {
                if let Some(cameo) = normalize_id(entry.cameo.as_deref().unwrap_or_default()) {
                    return cameo;
                }
                if let Some(alt_cameo) =
                    normalize_id(entry.alt_cameo.as_deref().unwrap_or_default())
                {
                    return alt_cameo;
                }
            }
        }

        resolved.image_id
    }

    /// Resolve the declared palette id for an asset, if any.
    pub fn resolve_declared_palette_id(&self, type_id: &str, rules_image: &str) -> Option<String> {
        let resolved: ResolvedObjectArt<'_> = self.resolve_object_art(type_id, rules_image);
        let type_upper: String = type_id.to_uppercase();

        for key in [type_upper.as_str(), resolved.image_id.as_str()] {
            if let Some(entry) = self.get(key) {
                if let Some(ref pal) = entry.palette {
                    return Some(pal.clone());
                }
            }
        }

        None
    }

    /// Resolve the effective image id for an overlay.
    ///
    /// Follows the overlay resolution order: art `[NAME].Image=` first, then
    /// rules `[NAME].Image=`.
    pub fn resolve_overlay_image_id(&self, overlay_name: &str, rules_ini: &IniFile) -> String {
        let upper_name: String = overlay_name.to_uppercase();
        let mut image_id: String = upper_name.clone();

        if let Some(art_image) = self
            .get(&upper_name)
            .and_then(|entry| normalize_id(entry.image.as_deref().unwrap_or_default()))
        {
            image_id = art_image;
        }
        if let Some(rules_image) = rules_ini
            .section(overlay_name)
            .and_then(|section| section.get("Image"))
            .and_then(normalize_id)
        {
            image_id = rules_image;
        }

        image_id
    }

    /// Exact overlay convention flags used by filename generation.
    pub fn overlay_convention_flags(&self, overlay_name: &str, image_id: &str) -> (bool, bool) {
        let name_entry: Option<&ArtEntry> = self.get(overlay_name);
        let image_entry: Option<&ArtEntry> = self.get(image_id);

        let uses_theater: bool = image_entry.map(|e| e.theater).unwrap_or(false)
            || name_entry.map(|e| e.theater).unwrap_or(false);
        let uses_new_theater: bool = image_entry.map(|e| e.new_theater).unwrap_or(false)
            || name_entry.map(|e| e.new_theater).unwrap_or(false)
            || self.should_use_new_theater(image_id);

        (uses_theater, uses_new_theater)
    }

    /// Check whether `NewTheater` substitution should be applied.
    fn should_use_new_theater(&self, upper_image: &str) -> bool {
        if has_hardcoded_new_theater_prefix(upper_image) {
            return true;
        }
        self.get(upper_image)
            .map(|e| e.new_theater)
            .unwrap_or(false)
    }

    /// Iterate all entries with their canonical (uppercase) name keys.
    pub fn iter_entries(&self) -> impl Iterator<Item = (&str, &ArtEntry)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Mutable lookup; case-insensitive on the key.
    pub fn get_mut(&mut self, image_id: &str) -> Option<&mut ArtEntry> {
        self.entries.get_mut(&image_id.to_uppercase())
    }

    /// Eagerly populate `frame_width`/`frame_height` on entries whose anim has
    /// a smudge-spawn flag (Crater/Burn/ForceBigCraters). Reads frame 0 of
    /// each anim's SHP via the shared `anim_shp_candidates` filename
    /// pipeline. Anims without a loadable SHP keep the (30, 30) defaults
    /// from their initial parse.
    ///
    /// Returns `(populated, fallback)` for diagnostic logging:
    ///   `populated` = anims whose SHP was found and dims were stored
    ///   `fallback`  = smudge-flagged anims whose SHP failed to load
    pub fn populate_anim_frame_dims(
        &mut self,
        asset_manager: &crate::assets::asset_manager::AssetManager,
        theater_ext: &str,
        theater_name: &str,
    ) -> (u32, u32) {
        // Two-pass: collect (name, image_id) under &self, then mutate via
        // get_mut. Direct iter_mut would conflict with the &self call to
        // resolve_effective_image_id.
        let pending: Vec<(String, String)> = self
            .iter_entries()
            .filter(|(_name, entry)| entry.crater || entry.scorch || entry.force_big_craters)
            .map(|(name, _entry)| {
                let image_id: String = self.resolve_effective_image_id(name, name);
                (name.to_string(), image_id)
            })
            .collect();

        let mut populated: u32 = 0;
        let mut fallback: u32 = 0;
        for (name, image_id) in pending {
            let candidates: Vec<String> =
                anim_shp_candidates(Some(self), &name, &image_id, theater_ext, theater_name);
            let shp_bytes: Option<&[u8]> = candidates.iter().find_map(|c| asset_manager.get_ref(c));
            let Some(data) = shp_bytes else {
                fallback += 1;
                continue;
            };
            let Ok(shp) = crate::assets::shp_file::ShpFile::from_bytes(data) else {
                fallback += 1;
                continue;
            };
            let Some(frame) = shp.frames.first() else {
                fallback += 1;
                continue;
            };
            let fw = frame.frame_width;
            let fh = frame.frame_height;
            if let Some(entry) = self.get_mut(&name) {
                entry.frame_width = fw;
                entry.frame_height = fh;
                populated += 1;
            } else {
                fallback += 1;
            }
        }
        (populated, fallback)
    }
}

/// Generate filename candidates for standard SHP objects.
///
/// `repo-derived`: candidate ordering mirrors the original-style behavior already
/// used by the repo. Inputs must already be exact resolved ids.
pub fn object_shp_candidates(
    art: Option<&ArtRegistry>,
    image_id: &str,
    theater_ext: &str,
    theater_name: &str,
) -> Vec<String> {
    let upper: String = image_id.to_uppercase();
    let mut candidates: Vec<String> = Vec::with_capacity(6);
    let use_new_theater: bool = art
        .map(|registry| registry.should_use_new_theater(&upper))
        .unwrap_or_else(|| has_hardcoded_new_theater_prefix(&upper));

    if use_new_theater {
        let subbed: String = apply_theater_letter(&upper, theater_name);
        push_shp_pair(&mut candidates, &subbed, theater_ext);

        let generic: String = apply_generic_letter(&upper);
        if generic != subbed && generic != upper {
            push_shp_pair(&mut candidates, &generic, theater_ext);
        }
    }

    push_shp_pair(&mut candidates, &upper, theater_ext);
    candidates
}

/// Generate filename candidates for building make/build-up art.
pub fn make_shp_candidates(
    art: Option<&ArtRegistry>,
    image_id: &str,
    theater_ext: &str,
    theater_name: &str,
) -> Vec<String> {
    let upper: String = image_id.to_uppercase();
    let mut candidates: Vec<String> = Vec::with_capacity(6);
    let use_new_theater: bool = art
        .map(|registry| registry.should_use_new_theater(&upper))
        .unwrap_or_else(|| has_hardcoded_new_theater_prefix(&upper));

    if use_new_theater {
        let subbed: String = apply_theater_letter(&upper, theater_name);
        push_shp_pair(&mut candidates, &format!("{}MK", subbed), theater_ext);

        let generic: String = apply_generic_letter(&upper);
        if generic != subbed && generic != upper {
            push_shp_pair(&mut candidates, &format!("{}MK", generic), theater_ext);
        }
    }

    push_shp_pair(&mut candidates, &format!("{}MK", upper), theater_ext);
    candidates
}

/// Generate filename candidates for building animation SHPs.
///
/// `repo-derived`: uses the anim section's own `Theater=` / `NewTheater=` flags.
pub fn anim_shp_candidates(
    art: Option<&ArtRegistry>,
    anim_type: &str,
    image_id: &str,
    theater_ext: &str,
    theater_name: &str,
) -> Vec<String> {
    let upper_anim: String = anim_type.to_uppercase();
    let upper_image: String = image_id.to_uppercase();
    let entry: Option<&ArtEntry> = art.and_then(|registry| registry.get(&upper_anim));
    let uses_new_theater: bool = entry.map(|e| e.new_theater).unwrap_or(false);
    let uses_theater: bool = entry.map(|e| e.theater).unwrap_or(false);
    let mut candidates: Vec<String> = Vec::with_capacity(6);

    if uses_new_theater {
        let subbed: String = apply_theater_letter(&upper_image, theater_name);
        push_shp_pair(&mut candidates, &subbed, theater_ext);

        let generic: String = apply_generic_letter(&upper_image);
        if generic != subbed && generic != upper_image {
            push_candidate(&mut candidates, format!("{}.SHP", generic));
        }
    }

    if uses_theater {
        push_candidate(
            &mut candidates,
            format!("{}.{}", upper_image, theater_ext.to_ascii_uppercase()),
        );
    }
    push_candidate(&mut candidates, format!("{}.SHP", upper_image));
    if !uses_theater {
        push_candidate(
            &mut candidates,
            format!("{}.{}", upper_image, theater_ext.to_ascii_uppercase()),
        );
    }

    candidates
}

/// Generate filename candidates for overlay SHPs.
///
/// This function only applies conventions. Callers should resolve `image_id`
/// through `ArtRegistry::resolve_overlay_image_id()` first.
pub fn overlay_shp_candidates(
    art: Option<&ArtRegistry>,
    overlay_name: &str,
    image_id: &str,
    theater_ext: &str,
    theater_name: &str,
) -> Vec<String> {
    let upper_name: String = overlay_name.to_uppercase();
    let upper_image: String = image_id.to_uppercase();
    let (uses_theater, uses_new_theater): (bool, bool) = art
        .map(|registry| registry.overlay_convention_flags(&upper_name, &upper_image))
        .unwrap_or((false, has_hardcoded_new_theater_prefix(&upper_image)));
    let mut candidates: Vec<String> = Vec::with_capacity(6);

    if uses_new_theater {
        let subbed: String = apply_theater_letter(&upper_image, theater_name);
        push_candidate(
            &mut candidates,
            format!("{}.{}", subbed, theater_ext.to_ascii_uppercase()),
        );
        push_candidate(&mut candidates, format!("{}.SHP", subbed));

        let generic: String = apply_generic_letter(&upper_image);
        if generic != subbed && generic != upper_image {
            push_candidate(
                &mut candidates,
                format!("{}.{}", generic, theater_ext.to_ascii_uppercase()),
            );
            push_candidate(&mut candidates, format!("{}.SHP", generic));
        }
    }

    if uses_theater {
        push_candidate(
            &mut candidates,
            format!("{}.{}", upper_image, theater_ext.to_ascii_uppercase()),
        );
        push_candidate(&mut candidates, format!("{}.SHP", upper_image));
    } else {
        push_candidate(&mut candidates, format!("{}.SHP", upper_image));
        push_candidate(
            &mut candidates,
            format!("{}.{}", upper_image, theater_ext.to_ascii_uppercase()),
        );
    }

    candidates
}

/// Generate VXL/HVA filenames for a voxel model.
pub fn voxel_asset_names(image_id: &str) -> (String, String) {
    let upper: String = image_id.to_uppercase();
    (format!("{}.VXL", upper), format!("{}.HVA", upper))
}

/// Building animation key names and their suffixes.
const BUILDING_ANIM_KEYS: &[(&str, &[&str])] = &[
    ("ActiveAnim", &["", "Two", "Three", "Four"]),
    ("ActiveAnimGarrisoned", &[""]),
    ("IdleAnim", &["", "Two"]),
    ("SuperAnim", &[""]),
    ("SpecialAnim", &["", "Two", "Three"]),
    ("ProductionAnim", &[""]),
];

fn parse_building_anims(section: &IniSection, ini: &IniFile) -> Vec<BuildingAnimConfig> {
    let mut anims: Vec<BuildingAnimConfig> = Vec::new();

    for &(base, suffixes) in BUILDING_ANIM_KEYS {
        let kind: BuildingAnimKind = match base {
            "ActiveAnim" => BuildingAnimKind::Active,
            "ActiveAnimGarrisoned" => BuildingAnimKind::ActiveGarrisoned,
            "IdleAnim" => BuildingAnimKind::Idle,
            "SuperAnim" => BuildingAnimKind::Super,
            "SpecialAnim" => BuildingAnimKind::Special,
            "ProductionAnim" => BuildingAnimKind::Production,
            _ => BuildingAnimKind::Idle,
        };
        for &suffix in suffixes {
            let key: String = format!("{}{}", base, suffix);
            let anim_type: String = match section.get(&key) {
                Some(v) if !v.is_empty() => v.to_string(),
                _ => continue,
            };

            let x: i32 = section
                .get_i32(&format!("{}{}X", base, suffix))
                .unwrap_or(0);
            let y: i32 = section
                .get_i32(&format!("{}{}Y", base, suffix))
                .unwrap_or(0);
            let y_sort: i32 = section
                .get_i32(&format!("{}{}YSort", base, suffix))
                .unwrap_or(0);
            let z_adjust: i32 = section
                .get_i32(&format!("{}{}ZAdjust", base, suffix))
                .unwrap_or(0);

            let anim_section = ini.section(&anim_type);
            let loop_start: u16 = anim_section
                .and_then(|s| s.get_i32("LoopStart"))
                .unwrap_or(0) as u16;
            let loop_end: u16 = anim_section.and_then(|s| s.get_i32("LoopEnd")).unwrap_or(0) as u16;
            let loop_count: i32 = anim_section
                .and_then(|s| s.get_i32("LoopCount"))
                .unwrap_or(0);
            let rate: u16 = anim_section
                .and_then(|s| s.get_i32("Rate"))
                .map(|r| art_rate_to_delay_ms(r) as u16)
                .unwrap_or(DEFAULT_ART_RATE_MS);
            let start_frame: u16 =
                anim_section.and_then(|s| s.get_i32("Start")).unwrap_or(0) as u16;
            let ping_pong: bool = anim_section
                .and_then(|s| s.get_bool("PingPong"))
                .unwrap_or(false);

            anims.push(BuildingAnimConfig {
                anim_type,
                kind,
                is_primary: suffix.is_empty(),
                x,
                y,
                y_sort,
                z_adjust,
                loop_start,
                loop_end,
                loop_count,
                rate,
                start_frame,
                ping_pong,
            });
        }
    }

    anims
}

/// Replace the 2nd character of a filename with the theater-specific letter.
fn apply_theater_letter(name: &str, theater_name: &str) -> String {
    if name.len() < 2 {
        return name.to_string();
    }

    let upper_theater: String = theater_name.to_ascii_uppercase();
    let letter: char = match THEATER_LETTERS.iter().find(|(t, _)| *t == upper_theater) {
        Some((_, ch)) => *ch,
        None => return name.to_string(),
    };
    let mut chars: Vec<char> = name.chars().collect();
    chars[1] = letter;
    chars.into_iter().collect()
}

/// Replace the 2nd character of a filename with the generic letter `G`.
fn apply_generic_letter(name: &str) -> String {
    if name.len() < 2 {
        return name.to_string();
    }

    let mut chars: Vec<char> = name.chars().collect();
    chars[1] = NEW_THEATER_GENERIC_LETTER;
    chars.into_iter().collect()
}

fn normalize_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_uppercase())
    }
}

fn has_hardcoded_new_theater_prefix(upper_image: &str) -> bool {
    upper_image.len() >= 2 && NEW_THEATER_PREFIXES.iter().any(|&p| p == &upper_image[..2])
}

fn push_shp_pair(candidates: &mut Vec<String>, base_name: &str, theater_ext: &str) {
    push_candidate(candidates, format!("{}.SHP", base_name));
    push_candidate(
        candidates,
        format!("{}.{}", base_name, theater_ext.to_ascii_uppercase()),
    );
}

fn push_candidate(candidates: &mut Vec<String>, candidate: String) {
    if !candidates.contains(&candidate) {
        candidates.push(candidate);
    }
}

#[cfg(test)]
#[path = "art_data_tests.rs"]
mod tests;
