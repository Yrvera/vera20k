//! Live CellClass pavement authority for56E990. Bit13 is independent of
//! structural bridge membership and has no implicit Recalc/projection effects.

use super::*;
use crate::map::bridge_pavement::{self, Coord, DAMAGED_PAVEMENT, PavementHost};

impl ResolvedTerrainGrid {
    /// Admission for the remaining legacy class display projection. Endpoints
    /// keep their own raw TMP identity even when they have structural roles.
    pub(crate) fn is_current_bridge_middle(&self, rx: u16, ry: u16, axis: usize) -> bool {
        let Some(keys) = self.high_bridge_rim_tiles() else {
            return false;
        };
        let Some(cell) = self.cell(rx, ry) else {
            return false;
        };
        [keys.base, self.wood_bridge_set_base()]
            .into_iter()
            .any(|base| {
                let relative = cell.final_tile_index.wrapping_sub(base).wrapping_add(1);
                (0..4).any(|variant| relative == keys.middle[axis].wrapping_add(variant))
            })
    }

    pub(crate) fn pavement_damaged_at(&self, rx: u16, ry: u16) -> bool {
        self.cell(rx, ry)
            .is_some_and(|cell| cell.bridge_facts.raw_flags & DAMAGED_PAVEMENT != 0)
    }

    pub(crate) fn write_pavement_flags(&mut self, cell: NativeCellIdentity, flags: u32) {
        // The core preserves every other flag. Do not route through the bridge
        // setter projection:56E990 writes only the word and marks display dirt.
        match cell {
            NativeCellIdentity::Real(index) => self.cells[index].bridge_facts.raw_flags = flags,
            NativeCellIdentity::Dummy => self.shared_cell_dummy.write_raw_flags(flags),
        }
    }

    pub(crate) fn pavement_gate(&self, cell: NativeCellIdentity) -> bool {
        let NativeCellIdentity::Real(index) = cell else {
            return false;
        };
        let cell = &self.cells[index];
        if self.tile_registry_len.is_none() {
            // Explicit synthetic/from_cells authority; production owns the
            // pristine registered catalogue, including modulo/sparse semantics.
            return cell.has_damaged_data;
        }
        self.native_tmp_has_damaged_data(cell.final_tile_index, cell.final_sub_tile)
            .unwrap_or_else(|| {
                log::error!(
                    "pavement update lacks resident pristine tile {}",
                    cell.final_tile_index
                );
                false
            })
    }

    pub(crate) fn apply_native_pavement(
        &mut self,
        requested: Coord,
        state: bool,
    ) -> Vec<(u16, u16)> {
        let mut host = GridPavement {
            terrain: self,
            changed: Vec::new(),
        };
        bridge_pavement::set_connected(&mut host, requested, state);
        host.changed
    }

    /// Native480350 checks file count before the pristine damaged-data gate.
    /// None leaves the ordinary coordinate-selected file choice to its caller.
    pub(crate) fn pavement_draw_variant(&self, rx: u16, ry: u16) -> Option<u8> {
        let cell = self.cell(rx, ry)?;
        if let Some(count) = self.native_tmp_file_count(cell.final_tile_index) {
            if count < 2 {
                return Some(0);
            }
            if !self.native_tmp_has_damaged_data(cell.final_tile_index, cell.final_sub_tile)? {
                return None;
            }
        } else if !cell.has_damaged_data {
            return None;
        }
        Some(u8::from(self.pavement_damaged_at(rx, ry)))
    }
}

struct GridPavement<'a> {
    terrain: &'a mut ResolvedTerrainGrid,
    changed: Vec<(u16, u16)>,
}

impl PavementHost for GridPavement<'_> {
    type Cell = NativeCellIdentity;
    fn lookup(&mut self, requested: Coord) -> Self::Cell {
        self.terrain.native_cell_identity(requested)
    }
    fn tile(&self, cell: Self::Cell) -> i32 {
        match cell {
            NativeCellIdentity::Real(index) => self.terrain.cells[index].final_tile_index,
            NativeCellIdentity::Dummy => 0xffff,
        }
    }
    fn flags(&self, cell: Self::Cell) -> u32 {
        self.terrain.native_cell_flags(cell)
    }
    fn write_flags(&mut self, cell: Self::Cell, flags: u32) {
        self.terrain.write_pavement_flags(cell, flags);
    }
    fn has_damaged_data(&mut self, cell: Self::Cell) -> bool {
        self.terrain.pavement_gate(cell)
    }
    fn initial_screen(&mut self, _: Coord, _: Self::Cell) {
        // VERA rebuilds terrain instances every frame. The live raw flag is
        // consumed there even if this kickoff makes no new scalar change.
    }
    fn radar(&mut self, cell: Self::Cell) {
        let (x, y) = self.terrain.native_cell_coord(cell);
        self.changed.push((x as u16, y as u16));
    }
}
