//! Per-cell working state shared by every generation phase.
//!
//! The original keeps one fixed-size record per cell in a linear width x width
//! array. This mirrors the fields the phases actually use as a Rust struct
//! rather than raw offsets, but keeps the same linear indexing and the same
//! initial values, because phases read each other's leftovers.

/// One scratch cell.
///
/// Field names map to the original record's documented offsets: coordinate at
/// +0x00, height +0x08, velocity +0x10, rough probability +0x18, green +0x20,
/// sand +0x28, region id +0x38, stamp +0x3C, water lock +0x45, visited +0x47.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScratchCell {
    pub x: i16,
    pub y: i16,
    /// Height delta the hill pass accumulates, in levels.
    pub height: f64,
    /// Random-walk velocity carried between neighbours by the hill pass.
    pub velocity: f64,
    pub p_rough: f64,
    pub p_green: f64,
    pub p_sand: f64,
    /// Owning region, or -1 when unassigned.
    pub region: i32,
    /// Per-pass marker (patch id, BFS stamp); -1 when unset.
    pub stamp: i32,
    /// Set on cells that are water or water-adjacent. Locks them against
    /// height changes and patch placement.
    pub water_lock: bool,
    /// Visited marker for the tree-region walk.
    pub visited: bool,
}

impl Default for ScratchCell {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            height: 0.0,
            velocity: 0.0,
            p_rough: 0.0,
            p_green: 0.0,
            p_sand: 0.0,
            region: -1,
            stamp: -1,
            water_lock: false,
            visited: false,
        }
    }
}

/// The generator's working grid plus the isometric diamond that bounds it.
#[derive(Debug, Clone)]
pub struct RmgScratch {
    width: usize,
    cells: Vec<ScratchCell>,
    diamond_min: i32,
    diamond_max: i32,
}

impl RmgScratch {
    /// Build a `width` x `width` grid bounded by the given diamond limits.
    pub fn new(width: usize, diamond_min: i32, diamond_max: i32) -> Self {
        let mut cells = vec![ScratchCell::default(); width * width];
        // Every record carries its own coordinate; phases read it back rather
        // than recomputing from the index.
        for y in 0..width {
            for x in 0..width {
                let cell = &mut cells[y * width + x];
                cell.x = x as i16;
                cell.y = y as i16;
            }
        }
        Self {
            width,
            cells,
            diamond_min,
            diamond_max,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn diamond_bounds(&self) -> (i32, i32) {
        (self.diamond_min, self.diamond_max)
    }

    /// Linear index used by every phase.
    pub fn index(&self, x: i32, y: i32) -> usize {
        y as usize * self.width + x as usize
    }

    pub fn get(&self, x: i32, y: i32) -> &ScratchCell {
        &self.cells[self.index(x, y)]
    }

    pub fn get_mut(&mut self, x: i32, y: i32) -> &mut ScratchCell {
        let index = self.index(x, y);
        &mut self.cells[index]
    }

    pub fn cells(&self) -> &[ScratchCell] {
        &self.cells
    }

    pub fn cells_mut(&mut self) -> &mut [ScratchCell] {
        &mut self.cells
    }

    /// The in-playfield test every phase gates on.
    ///
    /// Three of the four comparisons are strict and the fourth is inclusive.
    /// That asymmetry is load-bearing: a cell on the far diagonal is inside,
    /// one on the near diagonal is not.
    pub fn in_diamond(&self, x: i32, y: i32) -> bool {
        self.diamond_min < x + y
            && x - y < self.diamond_min
            && y - x < self.diamond_min
            && x + y <= self.diamond_max
    }

    /// Clear region ownership and pass markers between phases.
    ///
    /// Deliberately leaves `water_lock` alone: it is established once by the
    /// water pass and every later phase depends on it surviving.
    pub fn reset_region_ids(&mut self) {
        for cell in &mut self.cells {
            cell.region = -1;
            cell.stamp = -1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_original_initial_state() {
        let cell = ScratchCell::default();
        assert_eq!(cell.region, -1, "region starts unassigned");
        assert_eq!(cell.stamp, -1);
        assert!(!cell.water_lock);
        assert!(!cell.visited);
        assert_eq!(cell.height, 0.0);
        assert_eq!(cell.velocity, 0.0);
    }

    #[test]
    fn cells_carry_their_own_coordinates() {
        let scratch = RmgScratch::new(8, 0, 100);
        assert_eq!((scratch.get(3, 5).x, scratch.get(3, 5).y), (3, 5));
        assert_eq!((scratch.get(0, 0).x, scratch.get(0, 0).y), (0, 0));
    }

    #[test]
    fn diamond_bounds_are_asymmetric() {
        let scratch = RmgScratch::new(16, 4, 12);
        // x+y must exceed the minimum, but may equal the maximum.
        assert!(!scratch.in_diamond(2, 2), "x+y == min is outside");
        assert!(scratch.in_diamond(3, 2), "x+y > min is inside");
        assert!(scratch.in_diamond(6, 6), "x+y == max is inside");
        assert!(!scratch.in_diamond(7, 6), "x+y > max is outside");
    }

    #[test]
    fn diamond_excludes_cells_too_far_off_axis() {
        let scratch = RmgScratch::new(16, 4, 24);
        // |x-y| must stay under the minimum bound on both diagonals.
        assert!(!scratch.in_diamond(9, 1), "x-y == 8 exceeds min 4");
        assert!(!scratch.in_diamond(1, 9), "y-x == 8 exceeds min 4");
        assert!(scratch.in_diamond(6, 5), "near the centre line is inside");
    }

    #[test]
    fn reset_clears_region_and_stamp_but_keeps_water_lock() {
        let mut scratch = RmgScratch::new(4, 0, 100);
        let cell = scratch.get_mut(1, 1);
        cell.region = 5;
        cell.stamp = 9;
        cell.water_lock = true;
        cell.height = 2.5;

        scratch.reset_region_ids();

        let cell = scratch.get(1, 1);
        assert_eq!(cell.region, -1);
        assert_eq!(cell.stamp, -1);
        assert!(
            cell.water_lock,
            "the water lock must survive a region reset"
        );
        assert_eq!(cell.height, 2.5, "heights survive a region reset");
    }

    #[test]
    fn index_is_row_major() {
        let scratch = RmgScratch::new(10, 0, 100);
        assert_eq!(scratch.index(0, 0), 0);
        assert_eq!(scratch.index(3, 0), 3);
        assert_eq!(scratch.index(0, 1), 10);
        assert_eq!(scratch.index(4, 2), 24);
    }
}
