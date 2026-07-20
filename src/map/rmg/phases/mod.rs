//! Generation phase bodies, one module per pipeline stage.
//!
//! Each phase is a plain function over the grid/scratch/rng owners and
//! commits state in the original's order. The stage sequencing itself lives
//! in `super::generate`.

pub mod blob;
pub mod green_spread;
pub mod shore;
pub mod water;

/// A cell reference held in a phase's work list.
///
/// The original keeps raw cell pointers; every out-of-band lookup returns the
/// one shared border cell, so list entries must distinguish "a real cell" from
/// "the border cell" to preserve the aliasing (a border entry re-reads the
/// border's coordinate slot at use time — it does not remember the coordinate
/// it was created from).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellRef {
    Cell(i16, i16),
    Border,
}

impl CellRef {
    pub fn at(grid: &super::grid::RmgGrid, x: i32, y: i32) -> Self {
        if grid.is_valid(x, y) {
            Self::Cell(x as i16, y as i16)
        } else {
            Self::Border
        }
    }
}
