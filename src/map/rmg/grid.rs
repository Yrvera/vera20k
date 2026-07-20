//! The generator's working cell grid and the native cell-iteration order.
//!
//! Phases mutate `GridCell`s (the subset of cell state the generator reads or
//! writes); `emit` projects the grid into a `MapFile` at the end. Iteration
//! order is load-bearing: every per-cell RNG draw happens in `DiamondScan`
//! order, so a different traversal silently reorders the whole draw stream.

use super::tiles::TILE_UNASSIGNED;

/// Clockwise-from-north neighbor offsets, index = direction code 0..7.
pub const DIRECTION_OFFSETS: [(i16, i16); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// One generated cell — the fields the generation phases touch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridCell {
    /// Flat tile index; 0 = clear ground, `TILE_UNASSIGNED` = untouched.
    pub tile: i32,
    /// Sub-tile index within a multi-cell tile block (0 = anchor/unset).
    pub sub_tile: u8,
    /// Ground level.
    pub level: u8,
    /// Slope index (0 flat, 1..18 ramps).
    pub slope: u8,
    /// Overlay type index, -1 = none.
    pub overlay: i32,
    /// Overlay density/frame byte.
    pub density: u8,
    /// An object occupies this cell (tree, tech building).
    pub occupied: bool,
    /// Start-cell marker set when a waypoint lands here.
    pub start_marker: bool,
}

impl Default for GridCell {
    fn default() -> Self {
        Self {
            tile: TILE_UNASSIGNED,
            sub_tile: 0,
            // The map-prep stage initialises generated cells to level 4.
            level: 4,
            slope: 0,
            overlay: -1,
            density: 0,
            occupied: false,
            start_marker: false,
        }
    }
}

/// Working grid, indexed by the same linear (x, y) scheme as the scratch
/// array. Cells outside the map's valid band read as `None`, which is what
/// terminates the native iteration.
#[derive(Debug, Clone)]
pub struct RmgGrid {
    width: usize,
    cells: Vec<GridCell>,
}

impl RmgGrid {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            cells: vec![GridCell::default(); width * width],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.width
    }

    pub fn get(&self, x: i32, y: i32) -> Option<&GridCell> {
        self.in_bounds(x, y)
            .then(|| &self.cells[y as usize * self.width + x as usize])
    }

    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut GridCell> {
        self.in_bounds(x, y)
            .then(|| &mut self.cells[y as usize * self.width + x as usize])
    }

    /// Neighbor coordinate one step in direction `dir` (0..7, masked like the
    /// original's `& 7`).
    pub fn step(x: i32, y: i32, dir: usize) -> (i32, i32) {
        let (dx, dy) = DIRECTION_OFFSETS[dir & 7];
        (x + i32::from(dx), y + i32::from(dy))
    }
}

/// The native cell-iteration order: anti-diagonal rows of the isometric map.
///
/// State machine transcribed from the original iterator: start at
/// `(x=1, y=d)`, walk each row by `(x+1, y-1)`, and when a row of `rem + 1`
/// cells is exhausted start the next row from the swapped coordinate with a
/// parity rule choosing between `x = old_y + 1` (row length `d`) and
/// `y = old_x + 1` (row length `d - 1`... `d`). The machine itself never
/// stops; the caller stops at the first coordinate that has no cell.
#[derive(Debug, Clone)]
pub struct DiamondScan {
    d: i32,
    x: i32,
    y: i32,
    rem: i32,
}

impl DiamondScan {
    pub fn new(d: i32) -> Self {
        Self {
            d,
            x: 1,
            y: d,
            rem: d - 1,
        }
    }

    /// The next coordinate in native order. Infinite by design — the caller
    /// owns termination (the original stops on the first null cell).
    pub fn next_coord(&mut self) -> (i32, i32) {
        let current = (self.x, self.y);
        if self.rem != 0 {
            self.x += 1;
            self.y -= 1;
            self.rem -= 1;
        } else {
            let (old_x, old_y) = (self.x, self.y);
            // Swap, then nudge one axis by parity of (x + y - d - 1).
            self.x = old_y;
            self.y = old_x;
            if (old_y + old_x - self.d - 1) & 1 == 0 {
                self.rem = self.d - 2;
                self.x = old_y + 1;
            } else {
                self.rem = self.d - 1;
                self.y = old_x + 1;
            }
        }
        current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_offsets_are_clockwise_from_north() {
        assert_eq!(DIRECTION_OFFSETS[0], (0, -1), "N");
        assert_eq!(DIRECTION_OFFSETS[2], (1, 0), "E");
        assert_eq!(DIRECTION_OFFSETS[4], (0, 1), "S");
        assert_eq!(DIRECTION_OFFSETS[6], (-1, 0), "W");
        assert_eq!(DIRECTION_OFFSETS[3], (1, 1), "SE");
        assert_eq!(DIRECTION_OFFSETS[7], (-1, -1), "NW");
    }

    #[test]
    fn step_masks_the_direction_like_the_original() {
        assert_eq!(RmgGrid::step(5, 5, 2), (6, 5));
        assert_eq!(RmgGrid::step(5, 5, 8 + 2), (6, 5), "dir & 7 wraps");
    }

    /// Hand-walked from the original iterator's state machine with d = 4.
    /// Rows alternate d and d-1 cells; anti-diagonal sums increase by one per
    /// row. Any deviation here reorders every per-cell draw in the pipeline.
    #[test]
    fn scan_order_matches_the_native_state_machine() {
        let mut scan = DiamondScan::new(4);
        let seq: Vec<(i32, i32)> = (0..14).map(|_| scan.next_coord()).collect();
        assert_eq!(
            seq,
            vec![
                (1, 4),
                (2, 3),
                (3, 2),
                (4, 1), // row sum 5, 4 cells
                (2, 4),
                (3, 3),
                (4, 2), // row sum 6, 3 cells
                (2, 5),
                (3, 4),
                (4, 3),
                (5, 2), // row sum 7, 4 cells
                (3, 5),
                (4, 4),
                (5, 3), // row sum 8, 3 cells
            ]
        );
    }

    #[test]
    fn scan_rows_alternate_between_full_and_short() {
        let mut scan = DiamondScan::new(6);
        let mut rows: Vec<Vec<(i32, i32)>> = Vec::new();
        let mut row: Vec<(i32, i32)> = Vec::new();
        for _ in 0..40 {
            let coord = scan.next_coord();
            if let Some(&(px, py)) = row.last()
                && coord.0 + coord.1 != px + py
            {
                rows.push(std::mem::take(&mut row));
            }
            row.push(coord);
        }
        for (index, row) in rows.iter().enumerate() {
            let expected = if index % 2 == 0 { 6 } else { 5 };
            assert_eq!(row.len(), expected, "row {index} length");
            let sum = row[0].0 + row[0].1;
            assert!(row.iter().all(|(x, y)| x + y == sum), "row {index} sum");
        }
    }

    #[test]
    fn default_cell_matches_map_prep_initial_state() {
        let cell = GridCell::default();
        assert_eq!(cell.tile, TILE_UNASSIGNED);
        assert_eq!(cell.level, 4, "generated cells initialise to level 4");
        assert_eq!(cell.overlay, -1);
        assert_eq!(cell.sub_tile, 0);
        assert!(!cell.occupied);
    }

    #[test]
    fn grid_bounds_and_indexing() {
        let mut grid = RmgGrid::new(8);
        assert!(grid.get(7, 7).is_some());
        assert!(grid.get(8, 0).is_none());
        assert!(grid.get(-1, 0).is_none());
        grid.get_mut(3, 2).unwrap().tile = 42;
        assert_eq!(grid.get(3, 2).unwrap().tile, 42);
        assert_eq!(grid.get(2, 3).unwrap().tile, TILE_UNASSIGNED);
    }
}
