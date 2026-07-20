//! Flat tile ids and tile predicates the generation phases compare against.
//!
//! The original resolves theater `[General]` tileset ordinals to first-tile
//! ids at theater load and stores them in globals; phases then test cell tile
//! indices against those bases. `TileIds` carries the resolved values, with
//! `-1` for a missing key exactly like the native globals, so range tests can
//! be ported verbatim. Cliff/impassable classification is NOT here — it is
//! the existing `TheaterCliffRanges::is_cliff_or_impassable_tile`.

use crate::map::theater::TheaterData;

/// Value the original stores in a cell's tile field for "unassigned"; treated
/// as clear ground by the clear-tile test.
pub const TILE_UNASSIGNED: i32 = 0xFFFF;

/// Span of a LAT transition set (base..base+0x10).
const LAT_SPAN: i32 = 0x10;
/// Shore-piece set span: 42 tiles, the same length the LAT pass uses for its
/// green-group shore exemption.
const SHORE_SPAN: i32 = 42;
/// Fixed spans of the start-placement 6x6 gate ranges.
const PAVED_ROADS_SPAN: i32 = 15;
const MISC_PAVE_SPAN: i32 = 14;
const PAVE_SPAN: i32 = 16;

/// Resolved flat tile ids for one theater. `-1` = key absent.
#[derive(Debug, Clone, Copy)]
pub struct TileIds {
    pub clear: i32,
    pub ramp_base: i32,
    pub rough: i32,
    pub sand: i32,
    pub green: i32,
    pub rough_lat: i32,
    pub sand_lat: i32,
    pub green_lat: i32,
    pub pave_lat: i32,
    pub pave: i32,
    pub water_base: i32,
    pub shore: i32,
    pub misc_pave: i32,
    pub paved_roads: i32,
    pub medians: i32,
}

fn flat(id: Option<u16>) -> i32 {
    id.map_or(-1, i32::from)
}

impl TileIds {
    pub fn resolve(theater: &TheaterData) -> Self {
        Self::from_keys(&theater.rmg_tiles)
    }

    pub fn from_keys(keys: &crate::map::theater::RmgTileKeys) -> Self {
        Self {
            clear: flat(keys.clear_tile),
            ramp_base: flat(keys.ramp_base),
            rough: flat(keys.rough_tile),
            sand: flat(keys.sand_tile),
            green: flat(keys.green_tile),
            rough_lat: flat(keys.clear_to_rough_lat),
            sand_lat: flat(keys.clear_to_sand_lat),
            green_lat: flat(keys.clear_to_green_lat),
            pave_lat: flat(keys.clear_to_pave_lat),
            pave: flat(keys.pave_tile),
            water_base: flat(keys.water_set),
            shore: flat(keys.shore_pieces),
            misc_pave: flat(keys.misc_pave_tile),
            paved_roads: flat(keys.paved_roads),
            medians: flat(keys.medians),
        }
    }

    /// Clear-ground test: exactly tile 0 or the unassigned sentinel — NOT
    /// membership in the clear tileset.
    pub fn is_clear(&self, tile: i32) -> bool {
        tile == 0 || tile == TILE_UNASSIGNED
    }

    /// Green-terrain membership: the green base tile or its LAT range.
    pub fn is_green_lat(&self, tile: i32) -> bool {
        base_or_lat(tile, self.green, self.green_lat)
    }

    /// Sand-terrain membership: the sand base tile or its LAT range.
    pub fn is_sand_lat(&self, tile: i32) -> bool {
        base_or_lat(tile, self.sand, self.sand_lat)
    }

    /// Shore-piece set membership.
    pub fn is_shore_piece(&self, tile: i32) -> bool {
        in_span(tile, self.shore, SHORE_SPAN)
    }

    /// Paved-road range used by the start 6x6 passability gate.
    pub fn is_paved_road(&self, tile: i32) -> bool {
        in_span(tile, self.paved_roads, PAVED_ROADS_SPAN)
    }

    /// Misc-pave range used by the start 6x6 passability gate.
    pub fn is_misc_pave(&self, tile: i32) -> bool {
        in_span(tile, self.misc_pave, MISC_PAVE_SPAN)
    }

    /// Pave range used by the start 6x6 passability gate.
    pub fn is_pave(&self, tile: i32) -> bool {
        in_span(tile, self.pave, PAVE_SPAN)
    }
}

fn base_or_lat(tile: i32, base: i32, lat_base: i32) -> bool {
    if base != -1 && tile == base {
        return true;
    }
    lat_base != -1 && tile >= lat_base && tile < lat_base + LAT_SPAN
}

fn in_span(tile: i32, base: i32, span: i32) -> bool {
    base != -1 && tile >= base && tile < base + span
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::theater::{RmgTileKeys, parse_tileset_ini};

    /// Retail theater INI: the tile-identity keys and their set bounds are
    /// the ground truth these predicates are checked against.
    const TEMPERATE_INI: &str = include_str!("../../../ini/temperatmd.ini");

    fn temperate_ids() -> TileIds {
        let lookup = parse_tileset_ini(TEMPERATE_INI.as_bytes(), "tem").unwrap();
        let bounds = lookup.bounds();
        let start = |set: usize| Some(bounds[set].start);
        // Ordinals verbatim from temperatmd.ini [General].
        let keys = RmgTileKeys {
            clear_tile: start(0),
            ramp_base: start(9),
            rough_tile: start(13),
            clear_to_rough_lat: start(14),
            sand_tile: start(33),
            clear_to_sand_lat: start(34),
            green_tile: start(41),
            clear_to_green_lat: start(42),
            pave_tile: start(46),
            misc_pave_tile: start(38),
            clear_to_pave_lat: start(39),
            water_set: start(21),
            shore_pieces: start(12),
            paved_roads: start(20),
            medians: start(40),
        };
        let ids = TileIds::from_keys(&keys);
        assert!(ids.green > 0 && ids.water_base > 0, "retail sets resolve");
        ids
    }

    #[test]
    fn clear_test_is_exact_not_set_membership() {
        let ids = temperate_ids();
        assert!(ids.is_clear(0));
        assert!(ids.is_clear(TILE_UNASSIGNED));
        assert!(!ids.is_clear(1), "tile 1 is in the clear SET but not clear");
        assert!(!ids.is_clear(ids.green));
    }

    #[test]
    fn green_lat_membership_covers_base_and_transition_range() {
        let ids = temperate_ids();
        assert!(ids.is_green_lat(ids.green));
        assert!(ids.is_green_lat(ids.green_lat));
        assert!(ids.is_green_lat(ids.green_lat + 0xF));
        assert!(!ids.is_green_lat(ids.green_lat + 0x10));
        assert!(!ids.is_green_lat(0));
        assert!(!ids.is_green_lat(ids.sand));
    }

    #[test]
    fn sand_lat_membership_mirrors_green() {
        let ids = temperate_ids();
        assert!(ids.is_sand_lat(ids.sand));
        assert!(ids.is_sand_lat(ids.sand_lat + 0xF));
        assert!(!ids.is_sand_lat(ids.sand_lat + 0x10));
        assert!(!ids.is_sand_lat(ids.green));
    }

    #[test]
    fn missing_keys_never_match() {
        let ids = TileIds {
            clear: -1,
            ramp_base: -1,
            rough: -1,
            sand: -1,
            green: -1,
            rough_lat: -1,
            sand_lat: -1,
            green_lat: -1,
            pave_lat: -1,
            pave: -1,
            water_base: -1,
            shore: -1,
            misc_pave: -1,
            paved_roads: -1,
            medians: -1,
        };
        // -1 bases must not create a range around -1; and the unassigned
        // sentinel must not collide with the missing-key value.
        assert!(!ids.is_green_lat(-1));
        assert!(!ids.is_shore_piece(-1));
        assert!(!ids.is_paved_road(0));
        assert!(ids.is_clear(TILE_UNASSIGNED));
    }

    #[test]
    fn shore_and_gate_ranges_use_fixed_spans() {
        let ids = temperate_ids();
        assert!(ids.is_shore_piece(ids.shore));
        assert!(ids.is_shore_piece(ids.shore + 41));
        assert!(!ids.is_shore_piece(ids.shore + 42));
        assert!(ids.is_paved_road(ids.paved_roads + 14));
        assert!(!ids.is_paved_road(ids.paved_roads + 15));
        assert!(ids.is_misc_pave(ids.misc_pave + 13));
        assert!(!ids.is_misc_pave(ids.misc_pave + 14));
        assert!(ids.is_pave(ids.pave + 15));
        assert!(!ids.is_pave(ids.pave + 16));
    }
}
