//! Water finalizer: assigns visible water tile variants.
//!
//! For 2x2 all-water clusters (anchor + E/S/SE, none sub-tiled yet) a large
//! multi-cell variant may be placed; single cells get one of six band
//! variants. All three draws are scaled-FP chains with perturbed-mantissa
//! constants — NOT integer mods; a `%` implementation maps the same raw draw
//! to different variants.

use crate::map::rmg::grid::RmgGrid;
use crate::map::rmg::rng::{RANGE_K_BITS, RmgRng};
use crate::map::rmg::tiles::TileIds;
use crate::map::rmg::x87::{self, TruncF64};

use super::shore::TileBlocks;

/// ~10*2^-32 — 2x2-vs-single selector scale (with +1.0 offset).
const K10_BITS: u64 = 0x3E24_0000_0014_0000;
/// ~242*2^-32 — 2x2 variant draw scale.
const K242_BITS: u64 = 0x3E6E_4000_001E_4000;
/// ~201*2^-32 — single-cell band draw scale.
const K201_BITS: u64 = 0x3E69_2000_0019_2000;

fn scaled_draw(rng: &mut RmgRng, k_bits: u64, offset: f64, max: i32) -> i32 {
    let scale = TruncF64::from_f64(f64::from_bits(k_bits));
    loop {
        let value = x87::ftol(
            TruncF64::from_f64(f64::from(rng.next_u32()))
                .mul(scale)
                .add(TruncF64::from_f64(offset))
                .to_f64(),
        );
        if value <= max {
            return value;
        }
    }
}

pub fn run(grid: &mut RmgGrid, ids: &TileIds, blocks: &dyn TileBlocks, rng: &mut RmgRng) {
    let coords: Vec<(i32, i32)> = grid.native_cells().collect();
    for &(x, y) in &coords {
        let anchor_ok = {
            let cell = grid.get(x, y).expect("native cell");
            cell.tile == ids.water_base && cell.sub_tile == 0
        };
        if !anchor_ok {
            continue;
        }

        // E, S, SE neighbors form a 2x2 block with this cell as NW anchor.
        let cluster = [2usize, 4, 3].iter().all(|&dir| {
            let (nx, ny) = RmgGrid::step(x, y, dir);
            grid.get(nx, ny)
                .is_some_and(|cell| cell.tile == ids.water_base && cell.sub_tile == 0)
        });

        if cluster {
            let selector = scaled_draw(rng, K10_BITS, 1.0, 10);
            if selector != 1 {
                let draw = scaled_draw(rng, K242_BITS, 0.0, 241);
                let variant = if draw < 240 { draw / 10 } else { 0xF7 - draw };
                place_block(grid, blocks, ids.water_base + variant, (x, y));
                continue;
            }
        }

        // Single-cell path: six bands, band 5 only at draw 200.
        let draw = scaled_draw(rng, K201_BITS, 0.0, 200);
        grid.get_mut(x, y).expect("native cell").tile = ids.water_base + 8 + draw / 40;
    }
}

/// Multi-cell tile placement: writes tile, sub-tile index, and the block's
/// height byte per covered cell (the region/player args of the original are
/// inert on this path).
fn place_block(grid: &mut RmgGrid, blocks: &dyn TileBlocks, tile: i32, anchor: (i32, i32)) {
    let Some(block) = blocks.block(tile) else {
        return;
    };
    let block = block.clone();
    for j in 0..block.height {
        for i in 0..block.width {
            let Some(sub) = block.subtiles[(j * block.width + i) as usize] else {
                continue;
            };
            if let Some(cell) = grid.get_mut(anchor.0 + i, anchor.1 + j) {
                cell.tile = tile;
                cell.sub_tile = (j * block.width + i) as u8;
                cell.level = sub.height;
                cell.slope = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::rmg::phases::shore::{SubTile, TileBlock};

    struct TwoByTwo(TileBlock);

    impl TileBlocks for TwoByTwo {
        fn block(&self, _tile: i32) -> Option<&TileBlock> {
            Some(&self.0)
        }
    }

    fn blocks() -> TwoByTwo {
        TwoByTwo(TileBlock {
            width: 2,
            height: 2,
            subtiles: vec![Some(SubTile { height: 0, terrain: 0 }); 4],
        })
    }

    fn ids() -> TileIds {
        TileIds {
            clear: 0,
            ramp_base: -1,
            rough: -1,
            sand: -1,
            green: -1,
            rough_lat: -1,
            sand_lat: -1,
            green_lat: -1,
            pave_lat: -1,
            pave: -1,
            water_base: 500,
            shore: 400,
            misc_pave: -1,
            paved_roads: -1,
            paved_road_ends: -1,
            medians: -1,
        }
    }

    fn water_grid() -> RmgGrid {
        let mut grid = RmgGrid::new(30, 10, 26);
        let coords: Vec<(i32, i32)> = grid.native_cells().collect();
        for (x, y) in coords {
            grid.get_mut(x, y).unwrap().tile = 500;
        }
        grid
    }

    #[test]
    fn every_water_cell_leaves_the_base_tile() {
        let mut grid = water_grid();
        let identity = ids();
        let block_table = blocks();
        let mut rng = RmgRng::new(1234);
        run(&mut grid, &identity, &block_table, &mut rng);

        for (x, y) in grid.native_cells().collect::<Vec<_>>() {
            let cell = grid.get(x, y).unwrap();
            assert_ne!(
                (cell.tile, cell.sub_tile),
                (500, 0),
                "({x},{y}) still holds the untouched base water tile"
            );
            assert!(
                cell.tile >= 500 && cell.tile < 500 + 24,
                "({x},{y}) holds a water variant (2x2 variants span 0..24)"
            );
        }
    }

    #[test]
    fn single_cells_use_the_six_band_variants() {
        let mut grid = water_grid();
        let identity = ids();
        let block_table = blocks();
        // Turn everything but one isolated water cell to land.
        let coords: Vec<(i32, i32)> = grid.native_cells().collect();
        for &(x, y) in &coords {
            if (x, y) != (10, 10) {
                grid.get_mut(x, y).unwrap().tile = 0;
            }
        }
        let mut rng = RmgRng::new(7);
        run(&mut grid, &identity, &block_table, &mut rng);
        let tile = grid.get(10, 10).unwrap().tile;
        assert!(
            (508..=513).contains(&tile),
            "isolated water gets a band variant, got {tile}"
        );
    }

    #[test]
    fn band_math_uses_the_fp_chain_not_modulo() {
        // Raw draw 0x80000000: the FP chain gives trunc(2^31 * ~201*2^-32)
        // = 100 -> band 2; a modulo implementation would give 3 -> band 0.
        let scale = TruncF64::from_f64(f64::from_bits(K201_BITS));
        let value = x87::ftol(
            TruncF64::from_f64(f64::from(0x8000_0000u32))
                .mul(scale)
                .to_f64(),
        );
        assert_eq!(value, 100, "scaled-draw semantics");
        assert_ne!(value, 0x8000_0000u32 as i32 % 201, "modulo would differ");
    }

    #[test]
    fn selector_range_is_one_to_ten() {
        let mut rng = RmgRng::new(99);
        for _ in 0..500 {
            let v = scaled_draw(&mut rng, K10_BITS, 1.0, 10);
            assert!((1..=10).contains(&v));
        }
    }

    #[test]
    fn finalize_is_deterministic() {
        let tiles = |seed| {
            let mut grid = water_grid();
            let identity = ids();
            let block_table = blocks();
            let mut rng = RmgRng::new(seed);
            run(&mut grid, &identity, &block_table, &mut rng);
            grid.native_cells()
                .collect::<Vec<_>>()
                .iter()
                .map(|&(x, y)| {
                    let cell = grid.get(x, y).unwrap();
                    (cell.tile, cell.sub_tile)
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(tiles(4321), tiles(4321));
    }
}
