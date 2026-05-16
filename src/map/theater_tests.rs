//! Tests for theater INI parsing and tileset lookups.
//!
//! Extracted from theater.rs to stay under the 400-line limit.

use super::*;

fn make_test_ini() -> String {
    "[TileSet0000]\nSetName=Grass\nFileName=clear\nTilesInSet=1\n\n\
     [TileSet0001]\nSetName=Blank\nFileName=\nTilesInSet=1\n\n\
     [TileSet0002]\nSetName=Cliffs\nFileName=cliff\nTilesInSet=3\n"
        .to_string()
}

#[test]
fn test_parse_tileset_ini_basic() {
    let ini: &str = &make_test_ini();
    let lookup: TilesetLookup = parse_tileset_ini(ini.as_bytes(), "tem").expect("Should parse");

    assert_eq!(lookup.len(), 5); // 1 + 1 + 3
    assert_eq!(lookup.filename(0), Some("clear01.tem"));
    assert_eq!(lookup.filename(1), None); // blank
    assert_eq!(lookup.filename(2), Some("cliff01.tem"));
    assert_eq!(lookup.filename(4), Some("cliff03.tem"));

    // Tileset boundary/index lookups.
    assert_eq!(lookup.bounds().len(), 3);
    assert_eq!(lookup.tileset_index(0), Some(0)); // tile 0 → TileSet0000
    assert_eq!(lookup.tileset_index(1), Some(1)); // tile 1 → TileSet0001
    assert_eq!(lookup.tileset_index(2), Some(2)); // tile 2 → TileSet0002
    assert_eq!(lookup.tileset_index(4), Some(2)); // tile 4 → TileSet0002
    assert_eq!(lookup.tileset_index(99), None); // out of range

    // Edge cases: NO_TILE, negative, and far out-of-range.
    assert_eq!(lookup.filename(NO_TILE), None);
    assert_eq!(lookup.filename(-1), None);
    assert_eq!(lookup.filename(999), None);
}

#[test]
fn test_collect_used_tiles() {
    let cells: Vec<(i32, u8)> = vec![(0, 0), (1, 0), (0, 0), (NO_TILE, 0), (2, 1)];
    let used: HashSet<TileKey> = collect_used_tiles(&cells);
    assert_eq!(used.len(), 3); // (0,0), (1,0), (2,1) — deduped, NO_TILE excluded.
    assert!(used.contains(&TileKey {
        tile_id: 0,
        sub_tile: 0,
        variant: 0,
    }));
    assert!(used.contains(&TileKey {
        tile_id: 2,
        sub_tile: 1,
        variant: 0,
    }));
}

#[test]
fn test_theater_def_lookup() {
    assert!(theater_def("TEMPERATE").is_some());
    assert!(theater_def("temperate").is_some());
    assert!(theater_def("SNOW").is_some());
    assert!(theater_def("URBAN").is_some());
    assert!(theater_def("DESERT").is_none());
}

#[test]
fn test_is_water_and_cliff() {
    let ini_str: &str = "\
[TileSet0000]\nSetName=Grass\nFileName=clear\nTilesInSet=2\n\n\
[TileSet0001]\nSetName=Water\nFileName=water\nTilesInSet=3\n\n\
[TileSet0002]\nSetName=Water Cliffs\nFileName=wcliff\nTilesInSet=2\n\n\
[TileSet0003]\nSetName=Cliffs\nFileName=cliff\nTilesInSet=1\n";
    let lookup: TilesetLookup = parse_tileset_ini(ini_str.as_bytes(), "tem").expect("Should parse");

    // Grass (tile_ids 0-1): not water, not cliff.
    assert!(!lookup.is_water(0));
    assert!(!lookup.is_cliff(0));
    // Water (tile_ids 2-4): water but not cliff.
    assert!(lookup.is_water(2));
    assert!(lookup.is_water(4));
    assert!(!lookup.is_cliff(2));
    // Water Cliffs (tile_ids 5-6): both water and cliff.
    assert!(lookup.is_water(5));
    assert!(lookup.is_cliff(5));
    // Cliffs (tile_id 7): cliff but not water.
    assert!(!lookup.is_water(7));
    assert!(lookup.is_cliff(7));
    // Out of range: neither.
    assert!(!lookup.is_water(99));
    assert!(!lookup.is_cliff(99));
}

#[test]
fn parses_morphable_flag_per_tileset() {
    let ini = b"[TileSet0000]\n\
                FileName=foo\n\
                TilesInSet=1\n\
                SetName=Foo\n\
                Morphable=yes\n\
                \n\
                [TileSet0001]\n\
                FileName=bar\n\
                TilesInSet=1\n\
                SetName=Bar\n\
                \n\
                [TileSet0002]\n\
                TilesInSet=-1\n";
    let lookup = parse_tileset_ini(ini, "tem").unwrap();
    // tile_id 0 = first tile of TileSet0000 (Morphable=yes)
    assert!(lookup.is_morphable(0));
    // tile_id 1 = first tile of TileSet0001 (Morphable= unset → default false)
    assert!(!lookup.is_morphable(1));
}

#[test]
fn parse_general_int_finds_bridge_middle_keys() {
    let ini = "[General]\nBridgeSet=5\nBridgeMiddle1=7\nBridgeMiddle2=12\n\n[TileSet0000]\nTilesInSet=1\nFileName=clear\n";
    assert_eq!(super::parse_general_int(ini, "BridgeMiddle1"), Some(7));
    assert_eq!(super::parse_general_int(ini, "BridgeMiddle2"), Some(12));
}

#[test]
fn parse_general_int_missing_bridge_middle_returns_none() {
    let ini = "[General]\nBridgeSet=5\n\n[TileSet0000]\nTilesInSet=1\nFileName=clear\n";
    assert_eq!(super::parse_general_int(ini, "BridgeMiddle1"), None);
    assert_eq!(super::parse_general_int(ini, "BridgeMiddle2"), None);
}

/// Helper: build a minimal TheaterData for variant-table tests. BridgeSet
/// at tileset index 0 with 20 tiles starting at tile_id 0. Palettes are
/// all-zero (tests never read pixels).
fn synthetic_theater_with_bridge_keys(
    bridge_middle_1: Option<u8>,
    bridge_middle_2: Option<u8>,
) -> super::TheaterData {
    let ini = b"[TileSet0000]\nTilesInSet=20\nFileName=bridge\nSetName=Bridge\n";
    let lookup = super::parse_tileset_ini(ini, "tem").unwrap();
    let empty_palette = crate::assets::pal_file::Palette::from_bytes(&[0u8; 768])
        .expect("768-byte zero palette parses");
    super::TheaterData {
        lookup,
        iso_palette: empty_palette.clone(),
        unit_palette: empty_palette.clone(),
        tiberium_palette: empty_palette,
        extension: "tem",
        ini_data: Vec::new(),
        bridge_set: Some(0),
        wood_bridge_set: None,
        bridge_top_left_1: Some(1),
        bridge_top_left_2: Some(2),
        bridge_top_right_1: Some(4),
        bridge_top_right_2: Some(5),
        bridge_middle_1,
        bridge_middle_2,
        tunnels: None,
        track_tunnels: None,
        dirt_tunnels: None,
        dirt_track_tunnels: None,
    }
}

#[test]
fn ramp_tile_table_matches_binary_height_predicates() {
    use crate::map::bridge_facts::BridgeRampKind;

    let td = synthetic_theater_with_bridge_keys(Some(7), Some(12));
    let table = BridgeRampTileTable::from_theater(&td).expect("ramp table");

    assert_eq!(
        table.match_relative_tile(4, 0x0C).map(|r| r.kind),
        Some(BridgeRampKind::TopRight)
    );
    assert_eq!(table.match_relative_tile(4, 0x08).map(|r| r.kind), None);
    assert_eq!(
        table.match_relative_tile(1, 0x08).map(|r| r.kind),
        Some(BridgeRampKind::TopLeft)
    );
    assert_eq!(
        table.match_relative_tile(7, 0x04).map(|r| r.kind),
        Some(BridgeRampKind::Middle1)
    );
    assert_eq!(
        table.match_relative_tile(10, 0x04).map(|r| r.kind),
        Some(BridgeRampKind::Middle1)
    );
    assert_eq!(table.match_relative_tile(11, 0x04), None);
    assert_eq!(
        table.match_relative_tile(12, 0x02).map(|r| r.kind),
        Some(BridgeRampKind::Middle2)
    );
}

#[test]
fn ramp_tile_match_tile_id_uses_one_based_bridge_key() {
    use crate::map::bridge_facts::BridgeRampKind;

    let td = synthetic_theater_with_bridge_keys(Some(7), Some(12));
    let table = BridgeRampTileTable::from_theater(&td).expect("ramp table");

    assert_eq!(
        table.match_tile_id(103, 100, 20, 0x0C).map(|r| r.kind),
        Some(BridgeRampKind::TopRight)
    );
    assert_eq!(
        table
            .match_tile_id(103, 100, 20, 0x0C)
            .map(|r| r.relative_tile_index),
        Some(4)
    );
}

#[test]
fn ramp_tile_match_tile_id_rejects_tile_before_bridge_set() {
    let td = synthetic_theater_with_bridge_keys(Some(7), Some(12));
    let table = BridgeRampTileTable::from_theater(&td).expect("ramp table");

    assert_eq!(table.match_tile_id(99, 100, 20, 0x0C), None);
}

#[test]
fn ramp_tile_match_tile_id_rejects_tile_at_bridge_set_end() {
    let table = BridgeRampTileTable {
        top_right_1: Some(4),
        top_right_2: None,
        top_left_1: None,
        top_left_2: None,
        middle_1: None,
        middle_2: None,
    };

    assert_eq!(table.match_tile_id(120, 100, 20, 0x0C), None);
    assert_eq!(table.match_tile_id(104, 100, 20, 0x0C), None);
}

#[test]
fn variant_table_temperate_values() {
    use super::BridgeAnchorVariantTable;
    let td = synthetic_theater_with_bridge_keys(Some(7), Some(12));
    let table = BridgeAnchorVariantTable::from_theater(&td).expect("table");
    // BridgeSet starts at tile_id 0 (TilesInSet=20, first tileset). NS
    // variants: BS + M1 + {-1, 0, 1, 2} = {6, 7, 8, 9}. EW: {11, 12, 13, 14}.
    assert_eq!(table.ns, [6, 7, 8, 9]);
    assert_eq!(table.ew, [11, 12, 13, 14]);
}

#[test]
fn variant_table_returns_none_on_missing_middle_1() {
    use super::BridgeAnchorVariantTable;
    let td = synthetic_theater_with_bridge_keys(None, Some(12));
    assert!(BridgeAnchorVariantTable::from_theater(&td).is_none());
}

#[test]
fn variant_table_returns_none_on_missing_middle_2() {
    use super::BridgeAnchorVariantTable;
    let td = synthetic_theater_with_bridge_keys(Some(7), None);
    assert!(BridgeAnchorVariantTable::from_theater(&td).is_none());
}

#[test]
fn variant_table_returns_none_on_zero_middle() {
    use super::BridgeAnchorVariantTable;
    let td = synthetic_theater_with_bridge_keys(Some(0), Some(12));
    assert!(BridgeAnchorVariantTable::from_theater(&td).is_none());
}

#[test]
fn variant_table_returns_none_on_out_of_bounds() {
    use super::BridgeAnchorVariantTable;
    // TilesInSet=20 → max tile_id 19. BridgeMiddle1=18 → 4th variant
    // = 0+18-1+3 = 20 (OOB).
    let td = synthetic_theater_with_bridge_keys(Some(18), Some(12));
    assert!(BridgeAnchorVariantTable::from_theater(&td).is_none());
}

#[test]
fn tile_id_for_variant0_returns_none() {
    use super::BridgeAnchorVariantTable;
    use crate::sim::bridge_state::{Axis, BridgeheadAnchorClass};
    let td = synthetic_theater_with_bridge_keys(Some(7), Some(12));
    let table = BridgeAnchorVariantTable::from_theater(&td).unwrap();
    assert_eq!(
        table.tile_id_for(Axis::NS, BridgeheadAnchorClass::Variant0),
        None
    );
    assert_eq!(
        table.tile_id_for(Axis::EW, BridgeheadAnchorClass::Variant0),
        None
    );
}

#[test]
fn tile_id_for_each_class_per_axis() {
    use super::BridgeAnchorVariantTable;
    use crate::sim::bridge_state::{Axis, BridgeheadAnchorClass};
    let td = synthetic_theater_with_bridge_keys(Some(7), Some(12));
    let table = BridgeAnchorVariantTable::from_theater(&td).unwrap();
    assert_eq!(
        table.tile_id_for(Axis::NS, BridgeheadAnchorClass::Variant1),
        Some(7)
    );
    assert_eq!(
        table.tile_id_for(Axis::NS, BridgeheadAnchorClass::Damaged),
        Some(8)
    );
    assert_eq!(
        table.tile_id_for(Axis::NS, BridgeheadAnchorClass::AboutToFall),
        Some(9)
    );
    assert_eq!(
        table.tile_id_for(Axis::EW, BridgeheadAnchorClass::AboutToFall),
        Some(14)
    );
}

#[test]
fn match_tile_id_round_trip_all_variants() {
    use super::BridgeAnchorVariantTable;
    use crate::sim::bridge_state::{Axis, BridgeheadAnchorClass};
    let td = synthetic_theater_with_bridge_keys(Some(7), Some(12));
    let table = BridgeAnchorVariantTable::from_theater(&td).unwrap();
    const CLASS_ORDER: [BridgeheadAnchorClass; 4] = [
        BridgeheadAnchorClass::Variant0,
        BridgeheadAnchorClass::Variant1,
        BridgeheadAnchorClass::Damaged,
        BridgeheadAnchorClass::AboutToFall,
    ];
    for (axis, expected_arr) in [(Axis::NS, &table.ns), (Axis::EW, &table.ew)] {
        for (slot, &tid) in expected_arr.iter().enumerate() {
            let (got_axis, got_class) = table.match_tile_id(tid).expect("matched");
            assert_eq!(got_axis, axis);
            assert_eq!(got_class, CLASS_ORDER[slot]);
        }
    }
}

#[test]
fn match_tile_id_rejects_non_variant() {
    use super::BridgeAnchorVariantTable;
    let td = synthetic_theater_with_bridge_keys(Some(7), Some(12));
    let table = BridgeAnchorVariantTable::from_theater(&td).unwrap();
    // BS+5 (one before Variant0 NS), BS+10 (between NS and EW), BS+15
    // (post-AboutToFall EW), 999 (outside BridgeSet).
    assert_eq!(table.match_tile_id(5), None);
    assert_eq!(table.match_tile_id(10), None);
    assert_eq!(table.match_tile_id(15), None);
    assert_eq!(table.match_tile_id(999), None);
}
