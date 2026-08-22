//! Native bridge-pavement damage-selector flood fill.
//!
//! This is sim-owned CellClass state. Presentation consumes the returned
//! ordered cells through the generic radar-terrain dirty channel.

use super::BridgeRuntimeState;
use crate::map::resolved_terrain::ResolvedTerrainGrid;

/// `g_DirectionOffsets[0..8]`: N, NE, E, SE, S, SW, W, NW.
const EIGHT_NEIGHBOR_OFFSETS: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

pub(super) fn extend_unique_cells(
    target: &mut Vec<(u16, u16)>,
    cells: impl IntoIterator<Item = (u16, u16)>,
) {
    for cell in cells {
        if !target.contains(&cell) {
            target.push(cell);
        }
    }
}

impl BridgeRuntimeState {
    /// Propagate the damaged-variant bit across an 8-neighbor region bounded
    /// by underlying-terrain `final_tile_index` equality. The kickoff call
    /// gates on the seed cell's `has_damaged_data`; recursive calls skip it.
    ///
    /// `MapClass::ToggleBridgePavement @ 0x0056E990` marks the seed before
    /// recursively visiting `g_DirectionOffsets[0..8]`. The returned cells
    /// retain that exact pre-order for the radar-terrain dirty queue.
    pub fn apply_damaged_variant_flood_fill(
        &mut self,
        rx: u16,
        ry: u16,
        state: bool,
        terrain: &ResolvedTerrainGrid,
    ) -> Vec<(u16, u16)> {
        let mut changed = Vec::new();
        self.apply_damaged_variant_flood_fill_internal(
            rx,
            ry,
            state,
            terrain,
            true,
            &mut changed,
        );
        changed
    }

    fn apply_damaged_variant_flood_fill_internal(
        &mut self,
        rx: u16,
        ry: u16,
        state: bool,
        terrain: &ResolvedTerrainGrid,
        kickoff: bool,
        changed: &mut Vec<(u16, u16)>,
    ) {
        let Some(cell_state) = self.cell(rx, ry).map(|cell| cell.damaged_variant) else {
            return;
        };
        if cell_state == state {
            return;
        }

        let Some(resolved) = terrain.cell(rx, ry) else {
            return;
        };
        let seed_tile_id = resolved.final_tile_index;
        if seed_tile_id == 0xFFFF || seed_tile_id < 0 || kickoff && !resolved.has_damaged_data {
            return;
        }

        if let Some(cell) = self.cell_mut(rx, ry) {
            cell.damaged_variant = state;
        }
        changed.push((rx, ry));

        for (dx, dy) in EIGHT_NEIGHBOR_OFFSETS {
            let nx_i = i32::from(rx) + dx;
            let ny_i = i32::from(ry) + dy;
            if nx_i < 0 || ny_i < 0 {
                continue;
            }
            let nx = nx_i as u16;
            let ny = ny_i as u16;
            if terrain
                .cell(nx, ny)
                .is_some_and(|neighbor| neighbor.final_tile_index == seed_tile_id)
            {
                self.apply_damaged_variant_flood_fill_internal(
                    nx, ny, state, terrain, false, changed,
                );
            }
        }
    }
}
