//! Cell passability matrix - the 13x8 movement-zone x reduced-ZoneType table.
//!
//! Extracted from the original engine (416 bytes = 13 x 8 x 4).
//! The zone flood-fill and pathfinder use this matrix to determine whether
//! a cell's reduced ZoneType is passable for a given MovementZone row.
//!
//! ## How it works
//! The binary matrix columns are reduced `ZoneType` values written by
//! `CellClass::RecalcZoneType`, not raw TMP `LandType` bytes.
//! Older helpers in this module still expose a local `LandType` compatibility
//! enum for terrain bytes that have not gone through reduced-zone classification.
//! Native direct readers use the unit's **MovementZone** row, not SpeedType.
//! SpeedType is speed/cost-domain and only appears here in compatibility helpers.
//! The matrix lookup `MOVEMENT_ZONE_PASSABILITY[movement_zone][reduced_zone_type]` returns:
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
/// Native matrix lookups should use `ResolvedTerrainCell.zone_type` instead.
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
    /// Convert to the legacy compatibility bucket value.
    pub fn as_index(self) -> u8 {
        self as u8
    }
}

/// Map a raw TMP `terrain_type` byte (0-15) to a legacy terrain bucket.
///
/// RA2/YR TMP files encode 16 terrain types inherited from Tiberian Sun.
/// Several older call sites use these 8 buckets for speed/cost fallback. They
/// are not the native reduced ZoneType columns consumed by the matrix.
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

/// Verified native 13x8 passability matrix at `0x0082A594`.
///
/// Rows = MovementZone index (0-12). Binary columns are reduced ZoneType values
/// from `CellClass::RecalcZoneType`:
/// 0=Ground, 1=Crushable, 2=Wall, 3=Beach, 4=Water, 5=Building,
/// 6=Impassable, 7=Outside.
/// Values: 1 = passable, 2 = blocked, 3 = impassable (sentinel).
///
/// Some older helpers index this table with local `LandType` buckets. Terrain-
/// aware code should prefer `ResolvedTerrainCell.zone_type`, which is already
/// the reduced column written by the CellClass recalculation path.
///
/// Do not label column 1 as road or crate; the verified writer uses overlay
/// `Crushable=yes`.
pub const MOVEMENT_ZONE_PASSABILITY: [[u8; TERRAIN_TYPE_COUNT]; ZONE_LAYER_COUNT] = [
    // Reduced ZoneType:             Gnd Crs Wal Bch Wtr Bld Imp Out
    // Row  0 Normal:
    [1, 2, 2, 2, 2, 2, 2, 3],
    // Row  1 Crusher:
    [1, 1, 2, 2, 2, 2, 2, 3],
    // Row  2 Destroyer:
    [1, 1, 1, 2, 2, 2, 2, 3],
    // Row  3 AmphibiousDestroyer:
    [1, 1, 1, 1, 1, 1, 2, 3],
    // Row  4 AmphibiousCrusher:
    [1, 1, 2, 1, 1, 2, 2, 3],
    // Row  5 Amphibious:
    [1, 2, 2, 1, 1, 2, 2, 3],
    // Row  6 Subterranean:
    [1, 1, 1, 2, 2, 2, 1, 3],
    // Row  7 Infantry:
    [1, 2, 2, 2, 2, 1, 2, 3],
    // Row  8 InfantryDestroyer:
    [1, 1, 1, 2, 2, 1, 2, 3],
    // Row  9 Fly:
    [1, 1, 1, 1, 1, 1, 1, 3],
    // Row 10 Water:
    [2, 2, 2, 2, 1, 2, 2, 3],
    // Row 11 WaterBeach:
    [2, 2, 2, 1, 1, 2, 2, 3],
    // Row 12 CrusherAll:
    [1, 1, 1, 2, 2, 2, 2, 3],
];

/// Compatibility mapping from SpeedType to a matrix row.
///
/// Native direct matrix readers use MovementZone. This helper remains for older
/// fallback paths that have only a SpeedType and a compatibility terrain bucket.
pub fn zone_layer_for_speed_type(speed_type: SpeedType) -> usize {
    match speed_type {
        SpeedType::Foot => 2,
        SpeedType::Track => 2,
        SpeedType::Wheel => 1,
        SpeedType::Float => 9,
        SpeedType::FloatBeach => 4,
        SpeedType::Hover => 9,
        SpeedType::Amphibious => 3,
        SpeedType::Winged => 9,
    }
}

/// Map a MovementZone to its zone layer index (row in the passability matrix).
///
/// In the original engine, valid MovementZone values map directly to rows.
pub fn zone_layer_for_movement_zone(mz: MovementZone) -> usize {
    mz.matrix_row().unwrap_or(0)
}

/// Compatibility check for older call sites that only have a terrain bucket.
///
/// Returns true if the matrix entry is PASS_OK (1), false for PASS_BLOCKED (2)
/// or PASS_IMPASSABLE (3).
pub fn is_passable_for_speed_type(compat_land_type: u8, speed_type: SpeedType) -> bool {
    if compat_land_type as usize >= TERRAIN_TYPE_COUNT {
        return false; // Out of range = impassable
    }
    let layer = zone_layer_for_speed_type(speed_type);
    MOVEMENT_ZONE_PASSABILITY[layer][compat_land_type as usize] == PASS_OK
}

/// Check if a reduced ZoneType is passable for a given MovementZone.
///
/// Used by the zone flood-fill to partition the map into connectivity regions.
pub fn is_passable_for_zone(reduced_zone_type: u8, mz: MovementZone) -> bool {
    if reduced_zone_type as usize >= TERRAIN_TYPE_COUNT {
        return false;
    }
    let Some(layer) = mz.matrix_row() else {
        return false;
    };
    MOVEMENT_ZONE_PASSABILITY[layer][reduced_zone_type as usize] == PASS_OK
}

/// Get the raw passability value (1/2/3) for a row and reduced ZoneType.
///
/// Returns PASS_IMPASSABLE for out-of-bounds inputs.
pub fn passability_value(zone_layer: usize, reduced_zone_type: u8) -> u8 {
    if zone_layer >= ZONE_LAYER_COUNT || reduced_zone_type as usize >= TERRAIN_TYPE_COUNT {
        return PASS_IMPASSABLE;
    }
    MOVEMENT_ZONE_PASSABILITY[zone_layer][reduced_zone_type as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERIFIED_NATIVE_ROWS: [[u8; TERRAIN_TYPE_COUNT]; ZONE_LAYER_COUNT] = [
        [1, 2, 2, 2, 2, 2, 2, 3],
        [1, 1, 2, 2, 2, 2, 2, 3],
        [1, 1, 1, 2, 2, 2, 2, 3],
        [1, 1, 1, 1, 1, 1, 2, 3],
        [1, 1, 2, 1, 1, 2, 2, 3],
        [1, 2, 2, 1, 1, 2, 2, 3],
        [1, 1, 1, 2, 2, 2, 1, 3],
        [1, 2, 2, 2, 2, 1, 2, 3],
        [1, 1, 1, 2, 2, 1, 2, 3],
        [1, 1, 1, 1, 1, 1, 1, 3],
        [2, 2, 2, 2, 1, 2, 2, 3],
        [2, 2, 2, 1, 1, 2, 2, 3],
        [1, 1, 1, 2, 2, 2, 2, 3],
    ];

    #[test]
    fn matrix_matches_verified_native_dump() {
        assert_eq!(MOVEMENT_ZONE_PASSABILITY, VERIFIED_NATIVE_ROWS);
    }

    #[test]
    fn clear_passable_for_all_ground() {
        // Terrain type 0 (Clear) should be passable for all non-water zone layers.
        for layer in 0..10 {
            assert_eq!(
                MOVEMENT_ZONE_PASSABILITY[layer][0], PASS_OK,
                "Zone layer {} should pass on Clear terrain",
                layer
            );
        }
    }

    #[test]
    fn impassable_zone_type_blocked_except_subterranean_and_fly() {
        // Reduced ZoneType 6 is the native Impassable column.
        // Subterranean (row 6) and Fly (row 9) can enter; all others blocked.
        for layer in 0..ZONE_LAYER_COUNT {
            let expected = if layer == 6 || layer == 9 {
                PASS_OK
            } else {
                PASS_BLOCKED
            };
            assert_eq!(
                MOVEMENT_ZONE_PASSABILITY[layer][6], expected,
                "Zone layer {} on Impassable ZoneType",
                layer
            );
        }
    }

    #[test]
    fn outside_zone_type_blocks_all_rows() {
        for layer in 0..ZONE_LAYER_COUNT {
            assert_eq!(
                MOVEMENT_ZONE_PASSABILITY[layer][7], PASS_IMPASSABLE,
                "Zone layer {} on Outside ZoneType",
                layer
            );
        }
    }

    #[test]
    fn water_only_for_ships() {
        // Zone 10 (ships) should only pass on water (col 4).
        let row = MOVEMENT_ZONE_PASSABILITY[10];
        assert_eq!(row[4], PASS_OK);
        assert_eq!(row[0], PASS_BLOCKED); // clear = blocked for ships
        assert_eq!(row[1], PASS_BLOCKED); // crushable overlay = blocked for ships
    }

    #[test]
    fn amphibious_destroyer_passes_land_and_water() {
        // Legacy compatibility buckets; terrain-aware zoning uses reduced ZoneType.
        let row = MOVEMENT_ZONE_PASSABILITY[3];
        assert_eq!(row[0], PASS_OK); // clear
        assert_eq!(row[1], PASS_OK); // crushable overlay
        assert_eq!(row[2], PASS_OK); // rough
        assert_eq!(row[3], PASS_OK); // beach
        assert_eq!(row[4], PASS_OK); // water
        assert_eq!(row[5], PASS_OK); // building
        assert_eq!(row[6], PASS_BLOCKED); // impassable
        assert_eq!(row[7], PASS_IMPASSABLE); // outside
    }

    #[test]
    fn wheel_restricted() {
        // Row 1 (Crusher/wheel compatibility) passes ground and crushable only.
        let row = MOVEMENT_ZONE_PASSABILITY[1];
        assert_eq!(row[0], PASS_OK); // clear
        assert_eq!(row[1], PASS_OK); // crushable overlay
        assert_eq!(row[2], PASS_BLOCKED); // wall
    }

    #[test]
    fn speed_type_foot_uses_zone_2() {
        assert_eq!(zone_layer_for_speed_type(SpeedType::Foot), 2);
        assert!(is_passable_for_speed_type(0, SpeedType::Foot)); // clear
        assert!(is_passable_for_speed_type(2, SpeedType::Foot)); // reduced wall column
        assert!(!is_passable_for_speed_type(4, SpeedType::Foot)); // water
    }

    #[test]
    fn speed_type_float_uses_zone_9() {
        assert_eq!(zone_layer_for_speed_type(SpeedType::Float), 9);
        assert!(is_passable_for_speed_type(0, SpeedType::Float)); // clear
        assert!(is_passable_for_speed_type(4, SpeedType::Float)); // water
        // Row 9 Fly passes native Impassable (6) but blocks Outside (7).
        assert!(is_passable_for_speed_type(6, SpeedType::Float)); // impassable reduced ZoneType
        assert!(!is_passable_for_speed_type(7, SpeedType::Float)); // outside sentinel
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

        // LandType::Rock is a legacy terrain bucket. Native reduced Impassable
        // is column 6; column 7 is Outside and must block even for row 9 Fly.
        let rock = tmp_terrain_to_land_type(7);
        assert!(is_passable_for_speed_type(
            6,
            SpeedType::Float // Float -> row 9 -> passable on native Impassable column
        ));
        assert!(!is_passable_for_speed_type(
            rock.as_index(),
            SpeedType::Float // Rock bucket value 7 is Outside in the native matrix
        ));
        assert!(!is_passable_for_speed_type(
            6,
            SpeedType::Track // Track -> row 2 -> blocked on Impassable terrain
        ));
    }
}
