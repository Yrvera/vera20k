//! 3×3 cell grid iteration helper for area-of-effect superweapons.
//!
//! Exact native iteration and the remaining legacy GeneticConverter collector.
//! The two remain separate until GeneticConverter migrates its receiver traversal.
//!
//! ## Dependency rules
//! - Reads the map-owned CellClass lookup and world-owned cell membership.

use crate::sim::cell_rect::{CellRef, get_cellclass_fallback};
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::world::Simulation;

/// Runtime-initialized table at 0x00B0C038..0x00B0C05C. Native initializer
/// 0x006CAE00..0x006CAEBF; SuperClass::Launch adds each component as a word at
/// 0x006CCF39..0x006CCF44 (IC) and uses the same table for per-cell mutation.
/// Wrap each 16-bit component independently; never clamp an edge onto cell zero.
pub(super) fn native_cells_3x3(rx: u16, ry: u16) -> impl Iterator<Item = (i16, i16)> {
    const OFFSETS: [(i16, i16); 9] = [
        (0, 0),
        (1, 0),
        (1, -1),
        (0, -1),
        (-1, -1),
        (-1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];
    OFFSETS
        .into_iter()
        .map(move |(dx, dy)| ((rx as i16).wrapping_add(dx), (ry as i16).wrapping_add(dy)))
}

/// Select the canonical real cell and one live native object-list layer.
/// SuperClass::Launch tests CellClass+0x140 & 0x100, then reads +0xE8 or +0xE4
/// (IC 0x006CCF6A..0x006CCFEE). The existing GetCell facade owns fixed-stride
/// aliases, allocation and dummy stamping; requested coordinates are not keys.
/// The shared dummy has no represented object-list members in the Rust substrate.
pub(super) fn selected_cell_list(
    sim: &Simulation,
    x: i16,
    y: i16,
) -> Option<((u16, u16), MovementLayer)> {
    match get_cellclass_fallback(sim.resolved_terrain.as_ref(), i32::from(x), i32::from(y)) {
        CellRef::Real(cell) => Some((
            (cell.rx, cell.ry),
            if cell.bridge_facts.raw_flags & crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL != 0 {
                MovementLayer::Bridge
            } else {
                MovementLayer::Ground
            },
        )),
        CellRef::Dummy { .. } => {
            if sim.resolved_terrain.is_none() {
                sim.effective_shared_cell_dummy()
                    .stamp_coord(i32::from(x), i32::from(y));
            }
            None
        }
    }
}

/// Read an IC receiver's then-live link, including a callback that relayered
/// or relocated it. Retain the selected footprint cell while the object remains
/// on that list; after remove/re-add, its new membership owns the successor.
/// Native IC reads Object+0x30 after the call (0x006CD025); RemoveContent clears
/// it (0x0047EAF0), while PutContent may populate it from a different list.
pub(super) fn live_successor(
    sim: &Simulation,
    current: u64,
    visited_cell: (u16, u16),
    visited_layer: MovementLayer,
) -> Option<u64> {
    if let Some(cell) = sim.substrate.occupancy.get(visited_cell.0, visited_cell.1)
        && cell
            .iter_layer(visited_layer)
            .any(|member| member.entity_id == current)
    {
        return cell.next_on_layer(visited_layer, current);
    }
    let object = sim.substrate.entities.get(current)?;
    let layer = crate::sim::occupancy::cell_list_layer_for_entity(object)?;
    sim.substrate
        .occupancy
        .get(object.position.rx, object.position.ry)?
        .next_on_layer(layer, current)
}

/// Legacy row-major offsets. Not native visit order; retained only for the
/// unmigrated GeneticConverter coordinate collector.
pub const GRID_3X3_OFFSETS: [(i16, i16); 9] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (0, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// Iterate the 9 cells in a 3×3 grid around (center_rx, center_ry).
/// Coordinates are saturated to u16 bounds (underflow clamps to 0).
/// Caller is responsible for any further map-bounds filtering.
pub fn iter_cells_3x3(center_rx: u16, center_ry: u16) -> impl Iterator<Item = (u16, u16)> {
    GRID_3X3_OFFSETS.iter().map(move |(dx, dy)| {
        let rx = (center_rx as i32 + *dx as i32).max(0).min(u16::MAX as i32) as u16;
        let ry = (center_ry as i32 + *dy as i32).max(0).min(u16::MAX as i32) as u16;
        (rx, ry)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_nine_cells() {
        let cells: Vec<(u16, u16)> = iter_cells_3x3(10, 10).collect();
        assert_eq!(cells.len(), 9);
    }

    #[test]
    fn centered_at_given_cell() {
        let cells: Vec<(u16, u16)> = iter_cells_3x3(10, 10).collect();
        assert!(cells.contains(&(10, 10))); // center
        assert!(cells.contains(&(9, 9))); // NW
        assert!(cells.contains(&(11, 11))); // SE
    }

    #[test]
    fn saturates_at_zero() {
        let cells: Vec<(u16, u16)> = iter_cells_3x3(0, 0).collect();
        assert_eq!(cells.len(), 9);
        // All negative offsets clamp to 0.
        assert!(cells.iter().all(|(x, y)| *x <= 1 && *y <= 1));
    }

    #[test]
    fn saturates_at_max() {
        let cells: Vec<(u16, u16)> = iter_cells_3x3(u16::MAX, u16::MAX).collect();
        assert_eq!(cells.len(), 9);
        assert!(
            cells
                .iter()
                .all(|(x, y)| *x >= u16::MAX - 1 && *y >= u16::MAX - 1)
        );
    }
}
