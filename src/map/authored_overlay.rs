//! Live CellClass overlay identity/state surface for fresh map finalization.
//!
//! The authored reader mutates this surface synchronously in native fixed-grid
//! lookup order. Only after OverlayData, the shared drain, and the first live
//! Recalc sweep does it move a linear payload into the simulation OverlayGrid.

use crate::map::resolved_terrain::{ResolvedTerrainGrid, SharedCellDummy};

pub(crate) const NO_OVERLAY_IDENTITY: i32 = -1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FinalizedOverlayCell {
    identity: i32,
    state: u8,
}

impl Default for FinalizedOverlayCell {
    fn default() -> Self {
        Self {
            identity: NO_OVERLAY_IDENTITY,
            state: 0,
        }
    }
}

impl FinalizedOverlayCell {
    pub(crate) const fn identity(self) -> i32 {
        self.identity
    }

    pub(crate) const fn overlay_id(self) -> Option<u8> {
        if self.identity >= 0 && self.identity <= u8::MAX as i32 - 1 {
            Some(self.identity as u8)
        } else {
            None
        }
    }

    pub(crate) const fn state(self) -> u8 {
        self.state
    }
}

/// Consumed-once final identity/state authority. Deliberately not `Clone`.
#[derive(Debug)]
pub(crate) struct FinalizedOverlayPayload {
    width: u16,
    height: u16,
    cells: Vec<FinalizedOverlayCell>,
}

impl FinalizedOverlayPayload {
    pub(crate) fn into_parts(self) -> (u16, u16, Vec<FinalizedOverlayCell>) {
        (self.width, self.height, self.cells)
    }

    #[cfg(test)]
    pub(crate) fn from_cells_for_test(
        width: u16,
        height: u16,
        cells: Vec<(i32, u8)>,
    ) -> Self {
        assert_eq!(cells.len(), usize::from(width) * usize::from(height));
        Self {
            width,
            height,
            cells: cells
                .into_iter()
                .map(|(identity, state)| FinalizedOverlayCell { identity, state })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeOverlayCellTarget {
    Real(usize),
    Dummy,
}

/// Mutable, load-local native overlay cell surface. It cannot escape except by
/// moving its real-cell values into `FinalizedOverlayPayload`.
#[derive(Debug)]
pub(crate) struct LiveOverlayCells {
    width: u16,
    height: u16,
    cells: Vec<FinalizedOverlayCell>,
    shared_dummy: SharedCellDummy,
}

impl LiveOverlayCells {
    pub(crate) fn empty_for_terrain(terrain: &ResolvedTerrainGrid) -> Self {
        let width = terrain.width();
        let height = terrain.height();
        Self {
            width,
            height,
            cells: vec![FinalizedOverlayCell::default();
                usize::from(width) * usize::from(height)],
            shared_dummy: terrain.shared_cell_dummy(),
        }
    }

    /// `MapClass::Get_CellClass` narrows both operands to signed words before
    /// sign-extending `y * 512 + x`. A true miss stamps only dummy coordinates.
    pub(crate) fn target(
        &self,
        terrain: &ResolvedTerrainGrid,
        x: i16,
        y: i16,
    ) -> NativeOverlayCellTarget {
        if let Some(index) = terrain.native_fixed_cell_index(x, y) {
            NativeOverlayCellTarget::Real(index)
        } else {
            self.shared_dummy.stamp_coord(i32::from(x), i32::from(y));
            NativeOverlayCellTarget::Dummy
        }
    }

    pub(crate) fn read(&self, target: NativeOverlayCellTarget) -> FinalizedOverlayCell {
        match target {
            NativeOverlayCellTarget::Real(index) => self.cells[index],
            NativeOverlayCellTarget::Dummy => {
                let (identity, state) = self.shared_dummy.overlay_identity_state();
                FinalizedOverlayCell { identity, state }
            }
        }
    }

    pub(crate) fn write_identity(
        &mut self,
        target: NativeOverlayCellTarget,
        identity: i32,
    ) {
        match target {
            NativeOverlayCellTarget::Real(index) => self.cells[index].identity = identity,
            NativeOverlayCellTarget::Dummy => {
                self.shared_dummy.write_overlay_identity(identity);
            }
        }
    }

    pub(crate) fn write_state(&mut self, target: NativeOverlayCellTarget, state: u8) {
        match target {
            NativeOverlayCellTarget::Real(index) => self.cells[index].state = state,
            NativeOverlayCellTarget::Dummy => self.shared_dummy.write_overlay_state(state),
        }
    }

    pub(crate) fn write(
        &mut self,
        target: NativeOverlayCellTarget,
        identity: i32,
        state: u8,
    ) {
        match target {
            NativeOverlayCellTarget::Real(index) => {
                self.cells[index] = FinalizedOverlayCell { identity, state };
            }
            NativeOverlayCellTarget::Dummy => {
                self.shared_dummy
                    .write_overlay_identity_state(identity, state);
            }
        }
    }

    pub(crate) fn finish(self) -> FinalizedOverlayPayload {
        FinalizedOverlayPayload {
            width: self.width,
            height: self.height,
            cells: self.cells,
        }
    }

    #[cfg(test)]
    pub(crate) fn real_cell(&self, index: usize) -> FinalizedOverlayCell {
        self.cells[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::resolved_terrain::SharedCellDummy;

    #[test]
    fn shared_dummy_overlay_pair_persists_across_coordinate_misses_and_resets_on_resize() {
        let dummy = SharedCellDummy::fresh();
        let retained = dummy.clone();
        dummy.write_overlay_identity_state(0x5c, 2);
        dummy.stamp_coord(-510, 2);

        assert!(dummy.same_identity(&retained));
        assert_eq!(retained.overlay_identity_state(), (0x5c, 2));
        assert_eq!(retained.snapshot().coord, (-510, 2));

        dummy.reconstruct_for_map_resize();
        assert_eq!(retained.overlay_identity_state(), (NO_OVERLAY_IDENTITY, 0));
        assert_eq!(retained.snapshot().coord, (0, 0));
    }

    #[test]
    fn payload_retains_signed_identity_and_independent_state_until_consumed() {
        let payload = FinalizedOverlayPayload::from_cells_for_test(
            2,
            1,
            vec![(NO_OVERLAY_IDENTITY, 41), (0xee, 9)],
        );
        let (width, height, cells) = payload.into_parts();

        assert_eq!((width, height), (2, 1));
        assert_eq!(cells[0].overlay_id(), None);
        assert_eq!(cells[0].state(), 41);
        assert_eq!(cells[1].overlay_id(), Some(0xee));
        assert_eq!(cells[1].state(), 9);
    }
}
