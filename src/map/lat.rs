//! Native load-time LAT (Lateral Terrain) transition fixup.
//!
//! Theater `[General]` tileset ordinals are resolved to absolute first tile
//! ids. Every cell then runs the Rough, Sand, Green, and Pave groups in that
//! fixed order, using direct N/E/S/W mask bits. Only the LAT half is here;
//! range-disjoint ramp/slope smoothing remains a separate residual.
//!
//! ## Dependency rules
//! - Part of map/ -- depends on rules/ (ini_parser) and map/theater.

use std::collections::HashMap;

use crate::map::map_file::MapCell;
use crate::map::theater::TilesetLookup;
use crate::rules::ini_parser::IniFile;

const LAT_LAST_OFFSET: i32 = 0xF;
const EDGE_SENTINEL: i32 = 0;
const NO_TILE: i32 = 0xFFFF;
/// Direct bit order: N, E, S, W.
const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

const GREEN_EXEMPTIONS: [(&str, i32); 2] = [("ShorePieces", 0x29), ("WaterBridge", 1)];
const PAVE_EXEMPTIONS: [(&str, i32); 3] = [
    ("MiscPaveTile", 0xD),
    ("Medians", 0xD),
    ("PavedRoads", 0x14),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TileRange {
    first: i32,
    last: i32,
}

impl TileRange {
    fn new(first: i32, last_offset: i32) -> Self {
        Self {
            first,
            last: first + last_offset,
        }
    }

    fn contains(self, tile: i32) -> bool {
        self.first <= tile && tile <= self.last
    }
}

/// One LAT group expressed entirely as absolute tile ids.
#[derive(Debug, Clone)]
pub struct LatGroundType {
    pub name: String,
    pub base_tile: i32,
    pub lat_base: i32,
    exemptions: Vec<TileRange>,
}

impl LatGroundType {
    fn contains(&self, tile: i32) -> bool {
        tile == self.base_tile || (self.lat_base..=self.lat_base + LAT_LAST_OFFSET).contains(&tile)
    }

    fn exempts(&self, tile: i32) -> bool {
        self.exemptions.iter().any(|range| range.contains(tile))
    }
}

/// LAT groups in fixed Rough -> Sand -> Green -> Pave order.
#[derive(Debug, Clone)]
pub struct LatConfig {
    pub grounds: Vec<LatGroundType>,
}

fn tileset_bounds(
    lookup: &TilesetLookup,
    ordinal: Option<i32>,
) -> Option<&crate::map::theater::TilesetBounds> {
    let ordinal = ordinal?;
    (ordinal >= 0)
        .then(|| lookup.bounds().get(ordinal as usize))
        .flatten()
}

/// Parse and resolve the theater's load-time LAT configuration.
///
/// Both base and LAT keys are required for a group. Missing exemption keys
/// disable only that range. `*ConnectTo` keys are intentionally ignored.
pub fn parse_lat_config(ini_data: &[u8], lookup: &TilesetLookup) -> LatConfig {
    let Ok(ini) = IniFile::from_bytes(ini_data) else {
        return LatConfig {
            grounds: Vec::new(),
        };
    };
    let Some(general) = ini.section("General") else {
        log::info!("LAT: no [General] section in theater INI");
        return LatConfig {
            grounds: Vec::new(),
        };
    };

    // The array type keeps the empty Rough/Sand slices typed without a
    // speculative shared terrain abstraction.
    let definitions: [(&str, &str, &str, &[(&str, i32)]); 4] = [
        ("Rough", "RoughTile", "ClearToRoughLat", &[]),
        ("Sand", "SandTile", "ClearToSandLat", &[]),
        ("Green", "GreenTile", "ClearToGreenLat", &GREEN_EXEMPTIONS),
        ("Pave", "PaveTile", "ClearToPaveLat", &PAVE_EXEMPTIONS),
    ];

    let mut grounds = Vec::with_capacity(definitions.len());
    for (name, base_key, lat_key, exemption_defs) in definitions {
        let Some(base) = tileset_bounds(lookup, general.get_i32(base_key)) else {
            continue;
        };
        let Some(lat) = tileset_bounds(lookup, general.get_i32(lat_key)) else {
            continue;
        };
        let exemptions = exemption_defs
            .iter()
            .filter_map(|&(key, last_offset)| {
                let bounds = tileset_bounds(lookup, general.get_i32(key))?;
                // Retail Lunar leaves these two ordinal sections present with
                // zero tiles, while native theater init clears their effective
                // LAT globals. Do not expand their shared next-tile start into
                // bogus Green exemption spans.
                if name == "Green" && bounds.count == 0 {
                    return None;
                }
                Some(TileRange::new(i32::from(bounds.start), last_offset))
            })
            .collect::<Vec<_>>();

        grounds.push(LatGroundType {
            name: name.to_string(),
            base_tile: i32::from(base.start),
            lat_base: i32::from(lat.start),
            exemptions,
        });
    }

    log::info!("LAT: {} ground groups configured", grounds.len());
    LatConfig { grounds }
}

fn neighbor_tile(cells: &[MapCell], by_coord: &HashMap<(u16, u16), usize>, x: i32, y: i32) -> i32 {
    if x < 0 || y < 0 || x > i32::from(u16::MAX) || y > i32::from(u16::MAX) {
        return EDGE_SENTINEL;
    }
    by_coord
        .get(&(x as u16, y as u16))
        .map_or(EDGE_SENTINEL, |&index| {
            let tile = cells[index].tile_index;
            if tile < 0 || tile == NO_TILE {
                EDGE_SENTINEL
            } else {
                tile
            }
        })
}

/// Apply the load-time LAT half in place, preserving cell and group order.
///
/// The third parameter remains for the existing map-build API; all lookup
/// data needed by the pass has already been resolved into `lat_config`.
pub fn apply_lat(cells: &mut [MapCell], lat_config: &LatConfig, _lookup: &TilesetLookup) {
    if lat_config.grounds.is_empty() {
        return;
    }

    let by_coord = cells
        .iter()
        .enumerate()
        .map(|(index, cell)| ((cell.rx, cell.ry), index))
        .collect::<HashMap<_, _>>();
    let mut changes = 0u32;

    for cell_index in 0..cells.len() {
        let x = i32::from(cells[cell_index].rx);
        let y = i32::from(cells[cell_index].ry);
        for ground in &lat_config.grounds {
            if !ground.contains(cells[cell_index].tile_index) {
                continue;
            }

            let mut mask = 0u8;
            for (bit, (dx, dy)) in CARDINAL_OFFSETS.iter().copied().enumerate() {
                let neighbor = neighbor_tile(cells, &by_coord, x + dx, y + dy);
                if !ground.contains(neighbor) && !ground.exempts(neighbor) {
                    mask |= 1u8 << bit;
                }
            }

            let new_tile = if mask == 0 {
                ground.base_tile
            } else {
                ground.lat_base + i32::from(mask)
            };
            if cells[cell_index].tile_index != new_tile {
                changes += 1;
            }
            cells[cell_index].tile_index = new_tile;
            // Native changes tile identity only; preserve sub_tile.
        }
    }

    log::info!(
        "LAT: rewrote {} group/cell transitions across {} cells",
        changes,
        cells.len()
    );
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use super::*;

    const COUNTS: [u16; 16] = [1, 1, 16, 1, 16, 1, 16, 1, 16, 42, 2, 14, 14, 21, 1, 10];
    const FULL_GENERAL: &str = r#"
PaveTile=7
ClearToPaveLat=8
GreenTile=5
ClearToGreenLat=6
SandTile=3
ClearToSandLat=4
RoughTile=1
ClearToRoughLat=2
ShorePieces=9
WaterBridge=10
MiscPaveTile=11
Medians=12
PavedRoads=13
RoughConnectTo=14
"#;

    fn fixture(general: &str, counts: &[u16]) -> (TilesetLookup, LatConfig) {
        let mut text = format!("[General]\n{general}");
        for (index, count) in counts.iter().copied().enumerate() {
            let name = if index == 15 { "Water" } else { "Synthetic" };
            writeln!(
                text,
                "\n[TileSet{index:04}]\nSetName={name}\nFileName=t{index}\nTilesInSet={count}"
            )
            .unwrap();
        }
        let lookup = crate::map::theater::parse_tileset_ini(text.as_bytes(), "tem").unwrap();
        let config = parse_lat_config(text.as_bytes(), &lookup);
        (lookup, config)
    }

    fn dummy_lookup() -> TilesetLookup {
        fixture("", &[1]).0
    }

    fn ground(name: &str, base: i32, lat: i32, ranges: &[(i32, i32)]) -> LatGroundType {
        LatGroundType {
            name: name.to_string(),
            base_tile: base,
            lat_base: lat,
            exemptions: ranges
                .iter()
                .map(|&(first, last_offset)| TileRange::new(first, last_offset))
                .collect(),
        }
    }

    fn cell(rx: u16, ry: u16, tile_index: i32) -> MapCell {
        MapCell {
            rx,
            ry,
            tile_index,
            sub_tile: 0,
            z: 0,
        }
    }

    fn center_cells(center: i32, neighbors: [i32; 4]) -> Vec<MapCell> {
        vec![
            cell(10, 10, center),
            cell(10, 9, neighbors[0]),
            cell(11, 10, neighbors[1]),
            cell(10, 11, neighbors[2]),
            cell(9, 10, neighbors[3]),
        ]
    }

    fn center_result(center: i32, neighbors: [i32; 4], ground: LatGroundType) -> i32 {
        let mut cells = center_cells(center, neighbors);
        apply_lat(
            &mut cells,
            &LatConfig {
                grounds: vec![ground],
            },
            &dummy_lookup(),
        );
        cells[0].tile_index
    }

    #[test]
    fn config_uses_absolute_ids_fixed_order_and_exact_ranges() {
        let (_, config) = fixture(FULL_GENERAL, &COUNTS);
        assert_eq!(
            config
                .grounds
                .iter()
                .map(|ground| (ground.name.as_str(), ground.base_tile, ground.lat_base))
                .collect::<Vec<_>>(),
            vec![
                ("Rough", 1, 2),
                ("Sand", 18, 19),
                ("Green", 35, 36),
                ("Pave", 52, 53)
            ]
        );
        assert!(config.grounds[0].exemptions.is_empty());
        assert!(config.grounds[1].exemptions.is_empty());
        assert_eq!(
            config.grounds[2].exemptions,
            vec![TileRange::new(69, 0x29), TileRange::new(111, 1)]
        );
        assert_eq!(
            config.grounds[3].exemptions,
            vec![
                TileRange::new(113, 0xD),
                TileRange::new(127, 0xD),
                TileRange::new(141, 0x14)
            ]
        );

        let only_shore = "GreenTile=5\nClearToGreenLat=6\nShorePieces=9\n";
        let (_, missing_key) = fixture(only_shore, &COUNTS);
        assert_eq!(
            missing_key.grounds[0].exemptions,
            vec![TileRange::new(69, 0x29)],
            "missing WaterBridge disables only that exemption"
        );
    }

    #[test]
    fn zero_length_lunar_green_exemptions_are_disabled() {
        let mut counts = COUNTS;
        counts[9] = 0;
        counts[10] = 0;
        let general = "GreenTile=5\nClearToGreenLat=6\nShorePieces=9\nWaterBridge=10\n";
        let (lookup, config) = fixture(general, &counts);
        assert!(config.grounds[0].exemptions.is_empty());

        // Both zero-length sections resolve to the next real tile's start.
        // It must differ from Green rather than becoming a bogus exemption.
        let shared_start = i32::from(lookup.bounds()[9].start);
        let mut cells = center_cells(35, [shared_start, 35, 35, 35]);
        apply_lat(&mut cells, &config, &lookup);
        assert_eq!(cells[0].tile_index, 37);
    }

    #[test]
    fn native_load_lat_contract() {
        let rough = || ground("Rough", 700, 710, &[]);
        for (different, expected) in [(0, 711), (1, 712), (2, 714), (3, 718)] {
            let mut neighbors = [700; 4];
            neighbors[different] = 0;
            assert_eq!(center_result(700, neighbors, rough()), expected);
        }
        assert_eq!(center_result(715, [700; 4], rough()), 700);
        assert_eq!(
            center_result(700, [-1, NO_TILE, 700, 700], rough()),
            713,
            "no-tile N+E set direct bits 0+1"
        );

        let green = ground("Green", 900, 910, &[(400, 0x29), (450, 1)]);
        assert_eq!(center_result(900, [441, 451, 900, 0], green.clone()), 918);
        assert_eq!(center_result(900, [442, 452, 900, 900], green), 913);
        let pave = ground(
            "Pave",
            1000,
            1010,
            &[(1100, 0xD), (1300, 0xD), (1200, 0x14)],
        );
        assert_eq!(
            center_result(1000, [1113, 1313, 1220, 0], pave.clone()),
            1018
        );
        assert_eq!(center_result(1000, [1114, 1314, 1221, 1000], pave), 1017);

        let (lookup, config) = fixture(FULL_GENERAL, &COUNTS);
        assert!(lookup.is_water(163));
        let mut water = center_cells(35, [163, 35, 35, 35]);
        apply_lat(&mut water, &config, &lookup);
        assert_eq!(water[0].tile_index, 37, "generic water is not exempt");
        let mut connect_to = center_cells(1, [162, 1, 1, 1]);
        apply_lat(&mut connect_to, &config, &lookup);
        assert_eq!(connect_to[0].tile_index, 3, "RoughConnectTo is ignored");

        let mut edge = vec![cell(0, 0, 700), cell(1, 0, 700), cell(0, 1, 700)];
        apply_lat(
            &mut edge,
            &LatConfig {
                grounds: vec![rough()],
            },
            &lookup,
        );
        assert_eq!(edge[0].tile_index, 719, "off-map N+W set bits 0+3");

        let mut preserved = center_cells(700, [0, 700, 700, 700]);
        preserved[0].sub_tile = 7;
        let rough_config = LatConfig {
            grounds: vec![rough()],
        };
        apply_lat(&mut preserved, &rough_config, &lookup);
        assert_eq!((preserved[0].tile_index, preserved[0].sub_tile), (711, 7));
        let after_first = preserved
            .iter()
            .map(|cell| cell.tile_index)
            .collect::<Vec<_>>();
        apply_lat(&mut preserved, &rough_config, &lookup);
        assert_eq!(
            preserved
                .iter()
                .map(|cell| cell.tile_index)
                .collect::<Vec<_>>(),
            after_first,
            "second application is idempotent"
        );
    }
}
