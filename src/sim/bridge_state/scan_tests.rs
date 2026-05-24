//! Tests for bridge repair scan coordinate enumeration.

use super::cells_in_5x5_scan;

#[test]
fn cells_in_5x5_scan_interior_yields_25_cells() {
    let cells: Vec<(u16, u16)> = cells_in_5x5_scan((10, 10)).collect();
    assert_eq!(cells.len(), 25);
    assert!(cells.contains(&(8, 8)));
    assert!(cells.contains(&(12, 12)));
    assert!(cells.contains(&(10, 10)));
}

#[test]
fn cells_in_5x5_scan_at_origin_clamps_negative() {
    let cells: Vec<(u16, u16)> = cells_in_5x5_scan((0, 0)).collect();
    // Only (0..=2, 0..=2) range — 3×3 = 9 cells.
    assert_eq!(cells.len(), 9);
    assert!(cells.contains(&(0, 0)));
    assert!(cells.contains(&(2, 2)));
    assert!(!cells.iter().any(|(x, _)| *x > 2));
}

#[test]
fn cells_in_5x5_scan_at_edge_clamps_one_side() {
    let cells: Vec<(u16, u16)> = cells_in_5x5_scan((1, 5)).collect();
    // X range: [0..=3] = 4. Y range: [3..=7] = 5. Total = 20.
    assert_eq!(cells.len(), 20);
}
