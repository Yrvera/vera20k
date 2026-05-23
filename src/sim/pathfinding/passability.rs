//! Cell passability matrix - the 13x8 movement-zone x reduced-ZoneType table.
//!
//! Extracted from the original engine (416 bytes = 13 x 8 x 4).
//! The zone flood-fill and pathfinder use this matrix to determine whether
//! a cell's reduced ZoneType is passable for a given movement profile.
//!
//! ## How it works
//! The binary matrix columns are reduced `ZoneType` values written by
//! `CellClass::RecalcZoneType`, not raw TMP `LandType` bytes.
//! Older helpers in this module still expose a local `LandType` compatibility
//! enum for terrain bytes that have not gone through reduced-zone classification.
//! Each unit has a **zone layer** derived from its MovementZone/SpeedType.
//! The matrix lookup `PASSABILITY_MATRIX[zone_layer][reduced_zone_type]` returns:
//! - 1 = passable
//! - 2 = blocked (dynamically, e.g. occupied)
//! - 3 = impassable (always blocked, e.g. rock)
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/locomotor_type (SpeedType, MovementZone).
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::rules::locomotor_type::{MovementZone, SpeedType};

/// Passability values from the matrix.
pub const PASS_OK: u8 = 1;
pub const PASS_BLOCKED: u8 = 2;
pub const PASS_IMPASSABLE: u8 = 3;

// ---------------------------------------------------------------------------
// LandType enum - compatibility terrain buckets used by older call sites
// ---------------------------------------------------------------------------

/// The 8 terrain classification buckets used by compatibility helpers.
///
/// These are not the binary reduced ZoneType meanings for every column.
/// Raw TMP `terrain_type` bytes (0-15) must be mapped to these via
/// `tmp_terrain_to_land_type()` before any matrix lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum LandType {
    Clear = 0,
    Road = 1,
    Rough = 2,
    Beach = 3,
    Water = 4,
    Tiberium = 5,
    Railroad = 6,
    Rock = 7,
}

impl LandType {
    /// Convert to the raw column index for passability matrix lookups.
    pub fn as_index(self) -> u8 {
        self as u8
    }
}

/// Map a raw TMP `terrain_type` byte (0-15) to its passability matrix column.
///
/// RA2/YR TMP files encode 16 terrain types inherited from Tiberian Sun.
/// The passability matrix only has 8 columns, so multiple TMP bytes map to
/// the same LandType:
///
/// | TMP byte | Name      | LandType  |
/// |----------|-----------|-----------|
/// | 0-4, 13  | Clear/Ice | Clear (0) |
/// | 5        | Tunnel    | Railroad (6) |
/// | 6        | Railroad  | Railroad (6) |
/// | 7-8      | Rock      | Rock (7)  |
/// | 9        | Water     | Water (4) |
/// | 10       | Beach     | Beach (3) |
/// | 11-12    | Road      | Clear (0) |
/// | 14       | Rough     | Rough (2) |
/// | 15       | Cliff     | Rock (7)  |
///
/// Road TMP terrain (11-12) maps to Clear, not Road. In the original engine,
/// RecalcZoneType (0x483C80) classifies road terrain without a road overlay
/// as ZoneType 0 (Ground). Reduced column 1 is assigned by overlay
/// `Crushable=yes`, not by road art or `Crate=yes`.
pub fn tmp_terrain_to_land_type(tmp_terrain_type: u8) -> LandType {
    match tmp_terrain_type {
        0..=4 | 13 => LandType::Clear,
        5 | 6 => LandType::Railroad,
        7 | 8 => LandType::Rock,
        9 => LandType::Water,
        10 => LandType::Beach,
        11 | 12 => LandType::Clear,
        14 => LandType::Rough,
        15 => LandType::Rock,
        // Unknown TMP bytes default to Clear (passable by all ground units).
        _ => LandType::Clear,
    }
}

/// Number of zone layers (rows) in the matrix.
pub const ZONE_LAYER_COUNT: usize = 13;

/// Number of reduced ZoneType columns in the binary matrix.
pub const TERRAIN_TYPE_COUNT: usize = 8;

/// Compatibility 13x8 passability matrix, adapted from the original engine (0x82A594).
///
/// Rows = MovementZone index (0-12). Binary columns are reduced ZoneType values
/// from `CellClass::RecalcZoneType`:
/// 0=Ground, 1=Crushable, 2=Wall, 3=Beach, 4=Water, 5=Building,
/// 6=Impassable, 7=Outside.
/// Values: 1 = passable, 2 = blocked, 3 = impassable (sentinel).
///
/// Some older helpers index this table with local `LandType` buckets. Terrain-
/// aware zone building should prefer `ResolvedTerrainCell.zone_type` and the
/// reduced-zone matrix in `zone_build.rs`.
///
/// Do not label column 1 as road or crate; the verified writer uses overlay
/// `Crushable=yes`.
pub static PASSABILITY_MATRIX: [[u8; TERRAIN_TYPE_COUNT]; ZONE_LAYER_COUNT] = [
    // Reduced ZoneType:             Gnd Crs Wal Bch Wtr Bld Imp Out
    // Row  0 Normal:
    [1, 2, 1, 2, 2, 2, 1, 2],
    // Row  1 Crusher:
    [1, 1, 1, 2, 2, 2, 1, 2],
    // Row  2 Destroyer:
    [1, 1, 1, 2, 2, 2, 1, 2],
    // Row  3 AmphibiousDestroyer:
    [1, 1, 1, 1, 1, 2, 1, 2],
    // Row  4 AmphibiousCrusher:
    [1, 1, 1, 1, 1, 2, 1, 2],
    // Row  5 Amphibious:
    [1, 2, 1, 1, 1, 2, 1, 2],
    // Row  6 Subterranean (can dig through rock and tiberium):
    [1, 1, 1, 2, 2, 1, 1, 1],
    // Row  7 Infantry:
    [1, 2, 1, 2, 2, 2, 1, 2],
    // Row  8 InfantryDestroyer:
    [1, 1, 1, 2, 2, 2, 1, 2],
    // Row  9 Fly (everything passable):
    [1, 1, 1, 1, 1, 1, 1, 1],
    // Row 10 Water:
    [2, 2, 2, 2, 1, 2, 2, 2],
    // Row 11 WaterBeach:
    [2, 2, 2, 1, 1, 2, 2, 2],
    // Row 12 CrusherAll:
    [1, 1, 1, 2, 2, 2, 1, 2],
];

/// Map a SpeedType to its zone layer index (row in the passability matrix).
///
/// Multiple SpeedTypes may share a layer. The mapping matches the original
/// engine's behavior.
pub fn zone_layer_for_speed_type(speed_type: SpeedType) -> usize {
    match speed_type {
        SpeedType::Foot => 2,       // clear + road + rough
        SpeedType::Track => 2,      // clear + road + rough
        SpeedType::Wheel => 1,      // clear + road only
        SpeedType::Float => 9,      // everything except rock (hover)
        SpeedType::FloatBeach => 4, // clear + road + beach + water
        SpeedType::Hover => 9,      // everything except rock
        SpeedType::Amphibious => 3, // land + water + beach + tiberium
        SpeedType::Winged => 9,     // everything except rock (fly)
    }
}

/// Map a MovementZone to its zone layer index (row in the passability matrix).
///
/// In the original engine, valid MovementZone values map directly to rows.
pub fn zone_layer_for_movement_zone(mz: MovementZone) -> usize {
    mz.matrix_row().unwrap_or(0)
}

/// Check if a terrain land type is passable for a given SpeedType.
///
/// Returns true if the matrix entry is PASS_OK (1), false for PASS_BLOCKED (2)
/// or PASS_IMPASSABLE (3).
pub fn is_passable_for_speed_type(land_type: u8, speed_type: SpeedType) -> bool {
    if land_type as usize >= TERRAIN_TYPE_COUNT {
        return false; // Out of range = impassable
    }
    let layer = zone_layer_for_speed_type(speed_type);
    PASSABILITY_MATRIX[layer][land_type as usize] == PASS_OK
}

/// Check if a terrain land type is passable for a given MovementZone.
///
/// Used by the zone flood-fill to partition the map into connectivity regions.
pub fn is_passable_for_zone(land_type: u8, mz: MovementZone) -> bool {
    if land_type as usize >= TERRAIN_TYPE_COUNT {
        return false;
    }
    let Some(layer) = mz.matrix_row() else {
        return false;
    };
    PASSABILITY_MATRIX[layer][land_type as usize] == PASS_OK
}

/// Get the raw passability value (1/2/3) for a zone layer and terrain type.
///
/// Returns PASS_IMPASSABLE for out-of-bounds inputs.
pub fn passability_value(zone_layer: usize, land_type: u8) -> u8 {
    if zone_layer >= ZONE_LAYER_COUNT || land_type as usize >= TERRAIN_TYPE_COUNT {
        return PASS_IMPASSABLE;
    }
    PASSABILITY_MATRIX[zone_layer][land_type as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_passable_for_all_ground() {
        // Terrain type 0 (Clear) should be passable for all non-water zone layers.
        for layer in 0..10 {
            assert_eq!(
                PASSABILITY_MATRIX[layer][0], PASS_OK,
                "Zone layer {} should pass on Clear terrain",
                layer
            );
        }
    }

    #[test]
    fn rock_blocked_except_subterranean_and_fly() {
        // Rock terrain maps to Impassable ZoneType in the original engine.
        // Subterranean (row 6) and Fly (row 9) can enter; all others blocked.
        for layer in 0..ZONE_LAYER_COUNT {
            let expected = if layer == 6 || layer == 9 {
                PASS_OK
            } else {
                PASS_BLOCKED
            };
            assert_eq!(
                PASSABILITY_MATRIX[layer][7], expected,
                "Zone layer {} on Rock terrain",
                layer
            );
        }
    }

    #[test]
    fn water_only_for_ships() {
        // Zone 10 (ships) should only pass on water (col 4).
        let row = PASSABILITY_MATRIX[10];
        assert_eq!(row[4], PASS_OK);
        assert_eq!(row[0], PASS_BLOCKED); // clear = blocked for ships
        assert_eq!(row[1], PASS_BLOCKED); // crushable overlay = blocked for ships
    }

    #[test]
    fn amphibious_destroyer_passes_land_and_water() {
        // Legacy compatibility buckets; terrain-aware zoning uses reduced ZoneType.
        let row = PASSABILITY_MATRIX[3];
        assert_eq!(row[0], PASS_OK); // clear
        assert_eq!(row[1], PASS_OK); // crushable overlay
        assert_eq!(row[2], PASS_OK); // rough
        assert_eq!(row[3], PASS_OK); // beach
        assert_eq!(row[4], PASS_OK); // water
        assert_eq!(row[5], PASS_BLOCKED); // building compatibility bucket
        assert_eq!(row[6], PASS_OK); // railroad = ground terrain
    }

    #[test]
    fn wheel_restricted() {
        // Zone 1 (Crusher/wheel) passes clear, crushable overlay, and rough.
        let row = PASSABILITY_MATRIX[1];
        assert_eq!(row[0], PASS_OK); // clear
        assert_eq!(row[1], PASS_OK); // crushable overlay
        assert_eq!(row[2], PASS_OK); // rough = ground
    }

    #[test]
    fn speed_type_foot_uses_zone_2() {
        assert_eq!(zone_layer_for_speed_type(SpeedType::Foot), 2);
        assert!(is_passable_for_speed_type(0, SpeedType::Foot)); // clear
        assert!(is_passable_for_speed_type(2, SpeedType::Foot)); // rough
        assert!(!is_passable_for_speed_type(4, SpeedType::Foot)); // water
    }

    #[test]
    fn speed_type_float_uses_zone_9() {
        assert_eq!(zone_layer_for_speed_type(SpeedType::Float), 9);
        assert!(is_passable_for_speed_type(0, SpeedType::Float)); // clear
        assert!(is_passable_for_speed_type(4, SpeedType::Float)); // water
        // Rock maps to Impassable ZoneType — Fly/hover CAN enter (row 9 col 6 = 1).
        assert!(is_passable_for_speed_type(7, SpeedType::Float)); // rock passable for hover
    }

    #[test]
    fn movement_zone_water_is_zone_10() {
        assert_eq!(zone_layer_for_movement_zone(MovementZone::Water), 10);
        assert!(!is_passable_for_zone(0, MovementZone::Water)); // clear blocked
        assert!(is_passable_for_zone(4, MovementZone::Water)); // water OK
    }

    #[test]
    fn movement_zone_is_direct_index() {
        // Valid MovementZone values map directly to passability matrix rows.
        assert_eq!(zone_layer_for_movement_zone(MovementZone::Normal), 0);
        assert_eq!(zone_layer_for_movement_zone(MovementZone::Crusher), 1);
        assert_eq!(zone_layer_for_movement_zone(MovementZone::Destroyer), 2);
        assert_eq!(
            zone_layer_for_movement_zone(MovementZone::AmphibiousDestroyer),
            3
        );
        assert_eq!(
            zone_layer_for_movement_zone(MovementZone::AmphibiousCrusher),
            4
        );
        assert_eq!(zone_layer_for_movement_zone(MovementZone::Amphibious), 5);
        assert_eq!(zone_layer_for_movement_zone(MovementZone::Subterranean), 6);
        assert_eq!(zone_layer_for_movement_zone(MovementZone::Infantry), 7);
        assert_eq!(
            zone_layer_for_movement_zone(MovementZone::InfantryDestroyer),
            8
        );
        assert_eq!(zone_layer_for_movement_zone(MovementZone::Fly), 9);
        assert_eq!(zone_layer_for_movement_zone(MovementZone::CrusherAll), 12);
    }

    #[test]
    fn invalid_movement_zone_is_not_passable() {
        assert_eq!(MovementZone::Invalid.matrix_row(), None);
        assert!(!is_passable_for_zone(0, MovementZone::Invalid));
    }

    #[test]
    fn out_of_range_land_type_impassable() {
        assert!(!is_passable_for_speed_type(8, SpeedType::Foot));
        assert!(!is_passable_for_speed_type(255, SpeedType::Float));
    }

    // -- LandType mapping tests --

    #[test]
    fn tmp_clear_variants_map_to_clear() {
        for byte in [0, 1, 2, 3, 4, 13] {
            assert_eq!(
                tmp_terrain_to_land_type(byte),
                LandType::Clear,
                "TMP byte {}",
                byte
            );
        }
    }

    #[test]
    fn tmp_water_maps_to_water() {
        assert_eq!(tmp_terrain_to_land_type(9), LandType::Water);
    }

    #[test]
    fn tmp_beach_maps_to_beach() {
        assert_eq!(tmp_terrain_to_land_type(10), LandType::Beach);
    }

    #[test]
    fn tmp_road_variants_map_to_clear() {
        // Road TMP terrain maps to Clear (Ground); reduced column 1 is
        // Crushable overlay, not road art.
        assert_eq!(tmp_terrain_to_land_type(11), LandType::Clear);
        assert_eq!(tmp_terrain_to_land_type(12), LandType::Clear);
    }

    #[test]
    fn tmp_rough_maps_to_rough() {
        assert_eq!(tmp_terrain_to_land_type(14), LandType::Rough);
    }

    #[test]
    fn tmp_rock_and_cliff_map_to_rock() {
        assert_eq!(tmp_terrain_to_land_type(7), LandType::Rock);
        assert_eq!(tmp_terrain_to_land_type(8), LandType::Rock);
        assert_eq!(tmp_terrain_to_land_type(15), LandType::Rock);
    }

    #[test]
    fn tmp_tunnel_and_railroad_map_to_railroad() {
        assert_eq!(tmp_terrain_to_land_type(5), LandType::Railroad);
        assert_eq!(tmp_terrain_to_land_type(6), LandType::Railroad);
    }

    #[test]
    fn tmp_unknown_bytes_default_to_clear() {
        for byte in 16..=255u8 {
            assert_eq!(
                tmp_terrain_to_land_type(byte),
                LandType::Clear,
                "TMP byte {}",
                byte
            );
        }
    }

    #[test]
    fn land_type_as_index_matches_repr() {
        assert_eq!(LandType::Clear.as_index(), 0);
        assert_eq!(LandType::Road.as_index(), 1);
        assert_eq!(LandType::Rough.as_index(), 2);
        assert_eq!(LandType::Beach.as_index(), 3);
        assert_eq!(LandType::Water.as_index(), 4);
        assert_eq!(LandType::Tiberium.as_index(), 5);
        assert_eq!(LandType::Railroad.as_index(), 6);
        assert_eq!(LandType::Rock.as_index(), 7);
    }

    #[test]
    fn mapped_land_types_work_with_passability_matrix() {
        // Water cells (TMP byte 9 → LandType::Water = 4) should be passable for ships.
        let water = tmp_terrain_to_land_type(9);
        assert!(is_passable_for_speed_type(
            water.as_index(),
            SpeedType::Float
        ));
        assert!(!is_passable_for_speed_type(
            water.as_index(),
            SpeedType::Track
        ));

        // Road TMP terrain (byte 11) → Clear (Ground). Passable for all ground units.
        let road_tmp = tmp_terrain_to_land_type(11);
        assert!(is_passable_for_speed_type(
            road_tmp.as_index(),
            SpeedType::Wheel
        ));

        // Beach cells (TMP byte 10 → LandType::Beach = 3) should be passable for amphibious.
        let beach = tmp_terrain_to_land_type(10);
        assert!(is_passable_for_speed_type(
            beach.as_index(),
            SpeedType::Amphibious
        ));
        assert!(!is_passable_for_speed_type(
            beach.as_index(),
            SpeedType::Track
        ));

        // Rock (TMP byte 7 → LandType::Rock = 7) maps to Impassable ZoneType.
        // Hover/Fly (row 9) CAN enter, but ground units cannot.
        let rock = tmp_terrain_to_land_type(7);
        assert!(is_passable_for_speed_type(
            rock.as_index(),
            SpeedType::Float // Float → row 9 (hover) → passable on Impassable terrain
        ));
        assert!(!is_passable_for_speed_type(
            rock.as_index(),
            SpeedType::Track // Track → row 2 → blocked on Impassable terrain
        ));
    }
}
