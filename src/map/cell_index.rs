//! Fixed native cell indexing — the 512-wide stride shared by every cell array.
//!
//! Map-owned (F05): the stride and linear-index rules are static facts of the
//! native cell grid, independent of any loaded map's playfield width.
//! `sim::cell_rect` re-exports these for its runtime consumers.
//!
//! ## Dependency rules
//! - Part of map/; depends on nothing above std.

/// Fixed cell-array stride — the engine indexes cells `y*0x200 + x` regardless of
/// the loaded map's playfield width. The valid linear range is `[0, MAX_CELL_INDEX]`.
/// This is NOT the loaded-map width index (that is `PathGrid`'s `y*width+x` cache).
pub const CELL_ROW_STRIDE: i64 = 0x200;
/// Highest valid linear cell index under the fixed 512-wide stride.
pub const MAX_CELL_INDEX: i64 = 0x3FFFF;

/// Cell coordinates cross the native seam as packed 16-bit words. Normalize at
/// that seam so the Rust-facing `i32` parameters cannot retain non-native high
/// bits.
pub(crate) const fn packed_cell_coord(x: i32, y: i32) -> (i32, i32) {
    (x as i16 as i32, y as i16 as i32)
}

/// Linear cell index using the fixed 512-wide stride (NOT the loaded-map width).
///
/// Returns `None` only when the index falls outside `[0, MAX_CELL_INDEX]`; the
/// dummy fallback (`get_cellclass_fallback`) turns that `None` into a non-null
/// reference, mirroring the engine's never-null `Get_CellClass`.
pub fn cell_linear_index(x: i32, y: i32) -> Option<i64> {
    let (x, y) = packed_cell_coord(x, y);
    let idx = (y as i64) * CELL_ROW_STRIDE + (x as i64);
    (0..=MAX_CELL_INDEX).contains(&idx).then_some(idx)
}

/// Canonical real slot selected by the packed fixed-stride Map lookup.
pub(crate) fn canonical_cell_coord(x: i32, y: i32) -> Option<(u16, u16)> {
    cell_linear_index(x, y).map(|index| {
        (
            (index % CELL_ROW_STRIDE) as u16,
            (index / CELL_ROW_STRIDE) as u16,
        )
    })
}
