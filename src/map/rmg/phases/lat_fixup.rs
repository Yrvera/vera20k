//! LAT auto-tiling fixup over the generated grid (RNG-free).
//!
//! Ported from the LAT half of `CellClass::ApplyLAT_and_SlopeFixup`. Four ground
//! groups run in fixed order — **Rough → Sand → Green → Pave** — on every cell.
//! A cell whose tile is a group's base *or* an existing LAT variant gets a 4-bit
//! mask over its four **cardinal** neighbours (bit 0 = N, 1 = E, 2 = S, 3 = W,
//! from `DIRECTION_OFFSETS[0, 2, 4, 6]`); a neighbour sets its bit unless it is
//! in the same group (base or LAT range) or in one of the group's hardcoded
//! exemption ranges. Mask 0 → the base tile; otherwise `lat_base + mask` (a
//! variant in `1..=15`). Off-band neighbours read as the map-edge sentinel
//! (tile 0), so a base cell at the diamond edge always gets its edge variant.
//!
//! Group membership is closed under this rewrite (a base tile becomes a LAT
//! variant that is still inside the same base∪LAT range), so the four passes are
//! order-independent between cells — the in-place walk matches the original.
//!
//! ## Slope fixup is intentionally NOT ported here
//! The original function has a second half that rewrites ramp tiles from the
//! per-cell slope-type byte (`+0x11C`, values 0..4). The port's `GridCell.slope`
//! is a different quantity (the 0..18 ramp-variant index), so the slope-type
//! source must be established by a separate RE pass before that half can be
//! reproduced faithfully. It consumes no RNG (deferring it cannot desync the
//! draw stream) and only affects cliff-ramp tile visuals — tracked as an open
//! item, not silently dropped.

use crate::map::rmg::grid::RmgGrid;
use crate::map::rmg::tiles::TileIds;

/// LAT variant range length past the base: `[lat_base, lat_base + 0xF]`.
const LAT_LEN: i32 = 0xF;
/// The four cardinal directions as `DIRECTION_OFFSETS` indices (N, E, S, W).
/// Bit `i` of the mask comes from `CARDINAL_DIRS[i]`.
const CARDINAL_DIRS: [usize; 4] = [0, 2, 4, 6];
/// Off-band / off-map neighbour tile (`Get_CellClass(off_map)` → tile 0).
const EDGE_SENTINEL: i32 = 0;

/// Green exemption span past the base: shore pieces `+0x29` (42 tiles),
/// water bridge `+1` (2 tiles).
const SHORE_LEN: i32 = 0x29;
const WATER_BRIDGE_LEN: i32 = 1;
/// Pave exemption spans: misc-pave `+0xD` (14), medians `+0xD` (14),
/// paved roads `+0x14` (21).
const MISC_PAVE_LEN: i32 = 0xD;
const MEDIANS_LEN: i32 = 0xD;
const PAVED_ROADS_LEN: i32 = 0x14;

/// An inclusive exemption range `[lo, hi]`, disabled (never matches a real
/// tile) when its base is `-1`.
#[derive(Clone, Copy)]
struct Exempt {
    lo: i32,
    hi: i32,
}

impl Exempt {
    fn new(base: i32, len: i32) -> Self {
        if base == -1 {
            Self { lo: -1, hi: -1 }
        } else {
            Self {
                lo: base,
                hi: base + len,
            }
        }
    }

    fn contains(&self, tile: i32) -> bool {
        self.lo != -1 && self.lo <= tile && tile <= self.hi
    }
}

/// Apply the four-group LAT fixup to every in-band grid cell in place.
pub fn run(grid: &mut RmgGrid, ids: &TileIds) {
    for (x, y) in grid.native_cells().collect::<Vec<_>>() {
        apply_cell(grid, ids, x, y);
    }
}

/// Run all four group passes on one cell, in the original's fixed order.
fn apply_cell(grid: &mut RmgGrid, ids: &TileIds, x: i32, y: i32) {
    // Rough — unguarded: it runs even when its LAT base is undefined (in which
    // case the match test only accepts the bare rough base tile).
    lat_group(grid, x, y, ids.rough, ids.rough_lat, &[]);

    // Sand — guarded, no exemptions.
    if ids.sand_lat != -1 {
        lat_group(grid, x, y, ids.sand, ids.sand_lat, &[]);
    }

    // Green — guarded; exempts shore pieces and water-bridge connectors.
    if ids.green_lat != -1 {
        let exemptions = [
            Exempt::new(ids.shore, SHORE_LEN),
            Exempt::new(ids.water_bridge, WATER_BRIDGE_LEN),
        ];
        lat_group(grid, x, y, ids.green, ids.green_lat, &exemptions);
    }

    // Pave — guarded; exempts misc-pave, medians and paved roads.
    if ids.pave_lat != -1 {
        let exemptions = [
            Exempt::new(ids.misc_pave, MISC_PAVE_LEN),
            Exempt::new(ids.medians, MEDIANS_LEN),
            Exempt::new(ids.paved_roads, PAVED_ROADS_LEN),
        ];
        lat_group(grid, x, y, ids.pave, ids.pave_lat, &exemptions);
    }
}

/// Whether `tile` belongs to a group (its base tile or its LAT variant range).
fn in_group(tile: i32, base: i32, lat_base: i32) -> bool {
    tile == base || (lat_base != -1 && lat_base <= tile && tile <= lat_base + LAT_LEN)
}

/// One group pass on one cell: if the cell is in the group, rebuild its tile
/// from the cardinal-neighbour mask.
fn lat_group(grid: &mut RmgGrid, x: i32, y: i32, base: i32, lat_base: i32, exemptions: &[Exempt]) {
    let tile = grid.get(x, y).map_or(EDGE_SENTINEL, |cell| cell.tile);
    if !in_group(tile, base, lat_base) {
        return;
    }

    let mut mask = 0u32;
    for (bit, &dir) in CARDINAL_DIRS.iter().enumerate() {
        let (nx, ny) = RmgGrid::step(x, y, dir);
        let neighbor = grid.get(nx, ny).map_or(EDGE_SENTINEL, |cell| cell.tile);
        // The neighbour sets the bit unless it reads as the same ground — in
        // this group, or inside an exemption range.
        let same =
            in_group(neighbor, base, lat_base) || exemptions.iter().any(|r| r.contains(neighbor));
        if !same {
            mask |= 1 << bit;
        }
    }

    let new_tile = if mask == 0 {
        base
    } else {
        lat_base + mask as i32
    };
    if let Some(cell) = grid.get_mut(x, y) {
        cell.tile = new_tile;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic ids: distinct bases and LAT ranges per group.
    fn ids() -> TileIds {
        TileIds {
            clear: 0,
            ramp_base: 600,
            rough: 700,
            sand: 800,
            green: 900,
            rough_lat: 710,
            sand_lat: 810,
            green_lat: 910,
            pave_lat: 1010,
            pave: 1000,
            water_base: 500,
            shore: 400,
            water_bridge: 450,
            misc_pave: 1100,
            paved_roads: 1200,
            medians: 1300,
            paved_road_ends: -1,
            waterfalls: [-1; 4],
        }
    }

    fn world() -> RmgGrid {
        let (map_w, map_h) = (44, 48);
        let stride = (map_w + map_h + 1) as usize;
        let (dmin, dmax) = (map_w, map_w + 2 * map_h);
        let mut grid = RmgGrid::new(stride, dmin, dmax);
        // Start every in-band cell as clear ground (tile 0).
        for (x, y) in grid.native_cells().collect::<Vec<_>>() {
            grid.get_mut(x, y).unwrap().tile = 0;
        }
        grid
    }

    fn set(grid: &mut RmgGrid, x: i32, y: i32, tile: i32) {
        grid.get_mut(x, y).unwrap().tile = tile;
    }

    /// Set the cell and its 4 cardinal neighbours, run, return the cell's tile.
    /// `[n, e, s, w]` are the neighbour tiles.
    fn one_cell(center_tile: i32, neigh: [i32; 4]) -> i32 {
        let mut grid = world();
        let (cx, cy) = (46, 46); // interior: all cardinal neighbours in-band
        assert!(grid.is_valid(cx, cy));
        set(&mut grid, cx, cy, center_tile);
        set(&mut grid, cx, cy - 1, neigh[0]); // N
        set(&mut grid, cx + 1, cy, neigh[1]); // E
        set(&mut grid, cx, cy + 1, neigh[2]); // S
        set(&mut grid, cx - 1, cy, neigh[3]); // W
        run(&mut grid, &ids());
        grid.get(cx, cy).unwrap().tile
    }

    #[test]
    fn isolated_base_cell_stays_base() {
        // All neighbours are the same rough ground → mask 0 → base tile.
        assert_eq!(one_cell(700, [700, 700, 700, 700]), 700);
    }

    #[test]
    fn each_cardinal_maps_to_its_own_bit() {
        // Bit 0 = N (→ +1), 1 = E (→ +2), 2 = S (→ +4), 3 = W (→ +8).
        assert_eq!(one_cell(700, [0, 700, 700, 700]), 710 + 1, "N → bit0");
        assert_eq!(one_cell(700, [700, 0, 700, 700]), 710 + 2, "E → bit1");
        assert_eq!(one_cell(700, [700, 700, 0, 700]), 710 + 4, "S → bit2");
        assert_eq!(one_cell(700, [700, 700, 700, 0]), 710 + 8, "W → bit3");
    }

    #[test]
    fn all_neighbours_differ_gives_island_variant() {
        // Every cardinal differs → mask 0xF → lat_base + 15.
        assert_eq!(one_cell(700, [0, 0, 0, 0]), 710 + 15);
    }

    #[test]
    fn an_existing_lat_variant_collapses_to_base_when_surrounded() {
        // A rough LAT variant whose neighbours are all rough → back to base.
        assert_eq!(one_cell(715, [700, 700, 700, 700]), 700);
    }

    #[test]
    fn green_exempts_shore_and_water_bridge() {
        // Green next to a shore piece and a water-bridge tile: neither sets a
        // bit; a third (clear) neighbour does.
        // N = shore, E = water-bridge, S = green (same), W = clear (differs).
        let tile = one_cell(900, [410, 451, 900, 0]);
        assert_eq!(tile, 910 + 8, "only the clear W neighbour sets a bit");
    }

    #[test]
    fn rough_has_no_exemptions() {
        // Rough next to a shore piece: shore is NOT exempt for rough → bit set.
        assert_eq!(one_cell(700, [410, 700, 700, 700]), 710 + 1);
    }

    #[test]
    fn pave_exempts_road_median_and_misc() {
        // Pave next to misc-pave (N), median (E), paved-road (S): all exempt;
        // clear W sets the only bit.
        let tile = one_cell(1000, [1105, 1305, 1205, 0]);
        assert_eq!(tile, 1010 + 8, "only the clear W neighbour sets a bit");
    }

    #[test]
    fn groups_are_disjoint_only_one_applies() {
        // A sand cell surrounded by green must be treated by the Sand pass
        // (green neighbours differ from sand) and untouched by other passes.
        // N green (differ), others sand → mask bit0 → sand_lat + 1.
        assert_eq!(one_cell(800, [900, 800, 800, 800]), 810 + 1);
    }

    #[test]
    fn clear_cells_are_never_rewritten() {
        // A clear cell (tile 0) matches no group base, so it is left alone even
        // when surrounded by ground tiles.
        assert_eq!(one_cell(0, [700, 800, 900, 1000]), 0);
    }

    #[test]
    fn map_edge_forces_bits_via_sentinel() {
        // At a near-tip cell two cardinal neighbours fall off the diamond and
        // read as the sentinel (tile 0 → differs). (23,22): N=(23,21) and
        // W=(22,22) are both at x+y == diamond_min, i.e. out of band.
        let mut grid = world();
        let (cx, cy) = (23, 22);
        assert!(grid.is_valid(cx, cy));
        assert!(!grid.is_valid(cx, cy - 1), "N is off-band");
        assert!(!grid.is_valid(cx - 1, cy), "W is off-band");
        set(&mut grid, cx, cy, 700); // rough base
        set(&mut grid, cx + 1, cy, 700); // E: rough (same)
        set(&mut grid, cx, cy + 1, 700); // S: rough (same)
        run(&mut grid, &ids());
        // N (bit0) and W (bit3) forced by the sentinel → mask 9.
        assert_eq!(grid.get(cx, cy).unwrap().tile, 710 + 9);
    }

    #[test]
    fn fixup_is_idempotent() {
        // Running the pass twice yields the same tiles: a LAT variant stays in
        // its own group range, so the second pass reproduces the first result.
        let mut grid = world();
        set(&mut grid, 46, 46, 700);
        set(&mut grid, 46, 45, 0); // N clear
        set(&mut grid, 47, 46, 800); // E sand
        set(&mut grid, 46, 47, 700); // S rough
        set(&mut grid, 45, 46, 700); // W rough
        run(&mut grid, &ids());
        let after_first = grid.get(46, 46).unwrap().tile;
        run(&mut grid, &ids());
        assert_eq!(grid.get(46, 46).unwrap().tile, after_first, "idempotent");
    }

    #[test]
    fn disabled_lat_group_is_skipped() {
        // With sand_lat == -1 the Sand pass never runs: a sand base cell keeps
        // its tile even next to clear ground.
        let mut ids = ids();
        ids.sand_lat = -1;
        let mut grid = world();
        set(&mut grid, 46, 46, 800); // sand base
        set(&mut grid, 46, 45, 0); // N clear (would set a bit if Sand ran)
        run(&mut grid, &ids);
        assert_eq!(grid.get(46, 46).unwrap().tile, 800, "sand pass disabled");
    }
}
