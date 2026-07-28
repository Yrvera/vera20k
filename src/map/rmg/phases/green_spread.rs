//! Green-LAT spread: green terrain grows into adjacent clear cells.
//!
//! Runs after region partition and before the first attribute recalc. The
//! candidate list is collected once, then up to `min(len / 3, 1000)` random
//! entries are converted to green, each conversion re-feeding the list with
//! the converted cell's clear cardinal neighbors.

use crate::map::rmg::grid::RmgGrid;
use crate::map::rmg::rng::RmgRng;
use crate::map::rmg::tiles::TileIds;

use super::CellRef;

/// Conversion cap per map.
const SPREAD_CAP: i32 = 1000;
/// The four even directions (N, E, S, W) the spread steps in.
const EVEN_DIRS: [usize; 4] = [0, 2, 4, 6];

pub fn run(grid: &mut RmgGrid, ids: &TileIds, rng: &mut RmgRng) {
    // Collect: every green cell's clear cardinal neighbors, in native scan
    // order. Duplicates are kept — the original appends without dedup.
    let mut list: Vec<CellRef> = Vec::new();
    let coords: Vec<(i32, i32)> = grid.native_cells().collect();
    for (x, y) in coords {
        let tile = grid
            .get(x, y)
            .expect("native_cells yields valid cells only")
            .tile;
        if ids.is_green_lat(tile) {
            append_clear_neighbors(grid, ids, x, y, &mut list);
        }
    }

    let count = (list.len() as i32 / 3).min(SPREAD_CAP);
    for _ in 0..count {
        // The draw scales by the LIVE list length. The list can only shrink
        // net-zero or grow here, never empty (each iteration removes one and
        // the initial length is at least 3 * count).
        let index = rng.uniform(0, list.len() as i32 - 1) as usize;
        let entry = list.remove(index);

        // Paint through the held reference: a border entry writes the shared
        // border cell no matter what coordinate it was appended from.
        match entry {
            CellRef::Cell(x, y) => {
                grid.get_mut(i32::from(x), i32::from(y))
                    .expect("list entries reference existing cells")
                    .tile = ids.green;
            }
            CellRef::Border => grid.border_cell_mut().tile = ids.green,
        }

        // Re-scan the converted cell's cardinal neighbors. The base
        // coordinate is re-read from the cell each step — for the border
        // cell that is its live coordinate slot, which an out-of-band step
        // in this very loop can move.
        for dir in EVEN_DIRS {
            let (bx, by) = match entry {
                CellRef::Cell(x, y) => (i32::from(x), i32::from(y)),
                CellRef::Border => grid.border_coord(),
            };
            let (nx, ny) = RmgGrid::step(bx, by, dir);
            let clear = ids.is_clear(grid.cell_native(nx, ny).tile);
            if clear {
                list.push(CellRef::at(grid, nx, ny));
            }
        }
    }
}

fn append_clear_neighbors(
    grid: &mut RmgGrid,
    ids: &TileIds,
    x: i32,
    y: i32,
    list: &mut Vec<CellRef>,
) {
    for dir in EVEN_DIRS {
        let (nx, ny) = RmgGrid::step(x, y, dir);
        if ids.is_clear(grid.cell_native(nx, ny).tile) {
            list.push(CellRef::at(grid, nx, ny));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::rmg::grid::GridCell;
    use crate::map::rmg::tiles::SpecialTerrain;

    /// Minimal identity table: green base 100, green LAT 110, everything else
    /// far away so no accidental range overlaps.
    fn test_ids() -> TileIds {
        TileIds {
            clear: 0,
            ramp_base: -1,
            rough: -1,
            sand: 200,
            green: 100,
            rough_lat: -1,
            sand_lat: 210,
            green_lat: 110,
            pave_lat: -1,
            pave: -1,
            water_base: 300,
            shore: -1,
            water_bridge: -1,
            misc_pave: -1,
            paved_roads: -1,
            paved_road_ends: -1,
            medians: -1,
            special: SpecialTerrain::default(),
        }
    }

    fn grid_with_water() -> RmgGrid {
        let mut grid = RmgGrid::new(32, 8, 24);
        let coords: Vec<(i32, i32)> = grid.native_cells().collect();
        for (x, y) in coords {
            grid.get_mut(x, y).unwrap().tile = 300;
        }
        grid
    }

    fn paint(grid: &mut RmgGrid, cells: &[(i32, i32)], tile: i32) {
        for &(x, y) in cells {
            grid.get_mut(x, y).unwrap().tile = tile;
        }
    }

    #[test]
    fn conversion_count_is_a_third_of_the_candidate_list() {
        let mut grid = grid_with_water();
        // One green cell surrounded by 4 clear cardinals -> list of 4,
        // count = 4/3 = 1 conversion.
        paint(&mut grid, &[(8, 8)], 100);
        paint(&mut grid, &[(8, 7), (9, 8), (8, 9), (7, 8)], 0);

        let ids = test_ids();
        let mut rng = RmgRng::new(7);
        run(&mut grid, &ids, &mut rng);

        let greens = [(8, 7), (9, 8), (8, 9), (7, 8)]
            .iter()
            .filter(|&&(x, y)| grid.get(x, y).unwrap().tile == 100)
            .count();
        assert_eq!(greens, 1, "list of 4 yields exactly one conversion");
    }

    #[test]
    fn short_lists_convert_nothing() {
        let mut grid = grid_with_water();
        // Two clear neighbors -> len 2 -> 2/3 = 0 conversions.
        paint(&mut grid, &[(8, 8)], 100);
        paint(&mut grid, &[(8, 7), (9, 8)], 0);

        let ids = test_ids();
        let mut rng = RmgRng::new(7);
        let before = rng.clone();
        run(&mut grid, &ids, &mut rng);

        assert_eq!(grid.get(8, 7).unwrap().tile, 0);
        assert_eq!(grid.get(9, 8).unwrap().tile, 0);
        assert_eq!(
            rng.next_u32(),
            before.clone().next_u32(),
            "zero conversions must consume zero draws"
        );
    }

    #[test]
    fn converted_cells_refeed_the_list() {
        let mut grid = grid_with_water();
        // A clear corridor: enough green sources that the corridor cells all
        // enter the list, and conversions append their clear neighbors.
        paint(&mut grid, &[(10, 8), (10, 10), (12, 8)], 100);
        let corridor: Vec<(i32, i32)> = (8..14).map(|x| (x, 9)).collect();
        paint(&mut grid, &corridor, 0);
        paint(&mut grid, &[(10, 9)], 0);
        // Give each green its cardinal clears.
        paint(&mut grid, &[(10, 7), (9, 8), (11, 8)], 0);
        paint(&mut grid, &[(9, 10), (11, 10), (10, 11)], 0);
        paint(&mut grid, &[(12, 7), (13, 8), (11, 8)], 0);

        let ids = test_ids();
        let mut rng = RmgRng::new(1234);
        run(&mut grid, &ids, &mut rng);

        let converted = grid
            .native_cells()
            .collect::<Vec<_>>()
            .into_iter()
            .filter(|&(x, y)| grid.get(x, y).unwrap().tile == 100)
            .count();
        assert!(converted > 3, "spread grew beyond the seed greens");
    }

    #[test]
    fn spread_is_deterministic_per_seed() {
        let make = || {
            let mut grid = grid_with_water();
            paint(&mut grid, &[(10, 8), (12, 10)], 100);
            let clears: Vec<(i32, i32)> =
                (8..15).flat_map(|x| (7..12).map(move |y| (x, y))).collect();
            let clears: Vec<(i32, i32)> = clears
                .into_iter()
                .filter(|&(x, y)| grid.is_valid(x, y) && grid.get(x, y).unwrap().tile == 300)
                .collect();
            paint(&mut grid, &clears, 0);
            grid
        };
        let ids = test_ids();

        let mut first = make();
        run(&mut first, &ids, &mut RmgRng::new(99));
        let mut second = make();
        run(&mut second, &ids, &mut RmgRng::new(99));

        let tiles = |grid: &RmgGrid| -> Vec<i32> {
            grid.native_cells()
                .collect::<Vec<_>>()
                .iter()
                .map(|&(x, y)| grid.get(x, y).unwrap().tile)
                .collect()
        };
        assert_eq!(tiles(&first), tiles(&second));
    }

    #[test]
    fn edge_green_can_paint_the_border_cell() {
        let mut grid = grid_with_water();
        // A green cell on the first scan row: its north neighbor is out of
        // band, so the (clear-tiled) border cell joins the list; with a list
        // of exactly 3 the single conversion may hit any entry, but the
        // border cell must at least have been considered clear and appended.
        let (x, y) = (4, 5); // sum 9 > 8, first row region
        assert!(grid.is_valid(x, y));
        assert!(!grid.is_valid(x, y - 1), "north neighbor is out of band");
        paint(&mut grid, &[(x, y)], 100);
        paint(&mut grid, &[(x + 1, y), (x, y + 1)], 0);

        let ids = test_ids();
        // List: border (N), (x+1,y) E, (x,y+1) S -> count exactly 1.
        let mut rng = RmgRng::new(3);
        run(&mut grid, &ids, &mut rng);

        let converted_real =
            grid.get(x + 1, y).unwrap().tile == 100 || grid.get(x, y + 1).unwrap().tile == 100;
        let converted_border = {
            let mut probe = grid;
            probe.cell_native(0, 0).tile == 100
        };
        assert!(
            converted_real || converted_border,
            "exactly one of the three entries (incl. the border cell) converted"
        );
    }
}
