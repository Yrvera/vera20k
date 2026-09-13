//! Live high-body publication (576BA0/47E040). Authorities stay in Simulation
//! through synchronous fallout, including recursive DeathWeapon damage.
//!
//! Rim576770/576200 runs against live scalar cells and uses this same publisher.
//! Literal middle-tile replacement uses resident56EB80/47D2B0 inputs. Repair
//! constructors share this publisher; full engineer/zone/render delivery is
//! separately required before the bridge mechanism can close.

use super::*;
use crate::map::cell_index::NativeCellIdentity as Cell;
use crate::map::resolved_terrain::DynamicTerrainCellState;
use crate::sim::bridge_state::publication::{self, BridgePublicationHost, CellCoord};
use crate::sim::bridge_state::Phase;

#[path = "bridge_rim_publication.rs"]
mod rim_publication;

#[path = "bridge_tile_publication.rs"]
mod tile_publication;

#[path = "bridge_constructor_publication.rs"]
mod constructor_publication;

#[path = "bridge_pavement_publication.rs"]
mod pavement_publication;

#[path = "bridge_zone_publication.rs"]
mod zone_publication;

#[cfg(test)]
#[path = "bridge_pavement_publication_tests.rs"]
mod pavement_tests;

#[cfg(test)]
#[path = "bridge_publication_tests.rs"]
mod tests;

pub(super) struct BodyResult {
    pub returned: bool,
    pub collapsed: bool,
}

impl BodyResult {
    fn no_change() -> Self {
        Self {
            returned: false,
            collapsed: false,
        }
    }
}

/// Only the structural high-body continuation is migrated here. Overlay-first
/// dispatch, head entry and other bridge mechanisms retain their existing path.
pub(super) fn try_body(
    sim: &mut Simulation,
    rules: &RuleSet,
    registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    input: CellCoord,
) -> Option<BodyResult> {
    let terrain = sim.resolved_terrain.as_ref()?;
    let runtime = sim
        .bridge_state
        .as_ref()?
        .cell(input.0 as u16, input.1 as u16)?;
    if matches!(runtime.overlay_byte, 0x4a..=0x63 | 0xcd..=0xe6)
        || runtime.role == BridgeCellRole::Bridgehead
    {
        return None;
    }
    let selected = terrain.native_cell_identity(input);
    if terrain.native_cell_flags(selected) & BRIDGE_FLAG_STRUCTURAL == 0 {
        return Some(BodyResult::no_change());
    }
    let anchor = if terrain.native_cell_flags(selected) & BRIDGE_FLAG_ANCHOR_SELF != 0 {
        selected
    } else {
        match terrain.native_cell_anchor(selected) {
            Some(anchor) => anchor,
            None => return Some(BodyResult::no_change()),
        }
    };
    let overlay = match anchor {
        Cell::Real(index) => {
            let cell = &terrain.cells()[index];
            // Unmigrated high/head writers publish their identity in the same
            // runtime authority as their state. Its erased sentinel must win
            // over load-time terrain/OverlayGrid identities.
            sim.bridge_state
                .as_ref()
                .and_then(|state| state.cell(cell.rx, cell.ry))
                .map(|runtime| runtime.overlay_byte)
                .or(cell.bridge_facts.overlay_id)
        }
        Cell::Dummy => terrain.shared_cell_dummy().overlay_fields().0,
    };
    if !matches!(overlay, Some(0x18 | 0x19)) {
        return Some(BodyResult::no_change());
    }
    let mut host = LivePublication {
        sim,
        rules,
        registry,
        collapsed: false,
    };
    let returned = publication::advance_body_at_anchor(&mut host, input, anchor);
    Some(BodyResult {
        returned,
        collapsed: host.collapsed,
    })
}

struct LivePublication<'a> {
    sim: &'a mut Simulation,
    rules: &'a RuleSet,
    registry: Option<&'a crate::map::overlay_types::OverlayTypeRegistry>,
    collapsed: bool,
}

impl LivePublication<'_> {
    fn terrain(&self) -> &ResolvedTerrainGrid {
        self.sim
            .resolved_terrain
            .as_ref()
            .expect("live bridge terrain")
    }

    fn real_coord(&self, cell: Cell) -> Option<(u16, u16)> {
        match cell {
            Cell::Real(index) => {
                let cell = &self.terrain().cells()[index];
                Some((cell.rx, cell.ry))
            }
            Cell::Dummy => None,
        }
    }

    /// Capture current values after this scalar store, never an outer-frame
    /// snapshot that could overwrite a later recursive receiver's writes.
    fn retain_real_write(&mut self, cell: Cell) {
        let Cell::Real(index) = cell else { return };
        let terrain = self.sim.resolved_terrain.as_ref().expect("live terrain");
        if !terrain.bridge_flag_authority_matches_shape(&self.sim.real_cell_bridge_flags_0x1180) {
            self.sim.real_cell_bridge_flags_0x1180 =
                terrain.capture_real_cell_bridge_flags_0x1180();
        }
        let resolved = &terrain.cells()[index];
        let coord = (resolved.rx, resolved.ry);
        self.sim
            .real_cell_bridge_flags_0x1180
            .set_allocated_cell(index, resolved.bridge_facts.raw_flags);
        self.sim
            .dynamic_terrain_cells
            .insert(coord, DynamicTerrainCellState::capture(resolved));
    }

}

impl BridgePublicationHost for LivePublication<'_> {
    type Cell = Cell;

    fn lookup(&mut self, coord: CellCoord) -> Cell {
        self.terrain().native_cell_identity(coord)
    }
    fn coord(&self, cell: Cell) -> CellCoord {
        self.terrain().native_cell_coord(cell)
    }
    fn flags(&self, cell: Cell) -> u32 {
        self.terrain().native_cell_flags(cell)
    }
    fn state(&self, cell: Cell) -> u8 {
        // Existing head/repair drivers still own their encoded runtime state.
        // Read that authority until those writers migrate; the map's initial
        // state byte alone would silently discard their completed transitions.
        if let Some((x, y)) = self.real_coord(cell)
            && let Some(runtime) = self.sim.bridge_state.as_ref().and_then(|s| s.cell(x, y))
            && let Some(axis) = runtime.axis
        {
            runtime.damage_state.to_state_byte(axis)
        } else {
            self.terrain().native_cell_state(cell)
        }
    }
    fn write_flags(&mut self, cell: Cell, flags: u32) {
        let structural_changed = (self.flags(cell) ^ flags) & BRIDGE_FLAG_STRUCTURAL != 0;
        self.sim
            .resolved_terrain
            .as_mut()
            .unwrap()
            .write_native_cell_flags(cell, flags);
        if structural_changed
            && let Some((x, y)) = self.real_coord(cell)
            && let Some(runtime) = self
                .sim
                .bridge_state
                .as_mut()
                .and_then(|s| s.cell_mut(x, y))
        {
            runtime.deck_present = flags & BRIDGE_FLAG_STRUCTURAL != 0;
        }
        self.retain_real_write(cell);
    }
    fn write_state(&mut self, cell: Cell, state: u8) {
        if let Some((x, y)) = self.real_coord(cell) {
            let flags = self.flags(cell);
            if let Some(runtime) = self
                .sim
                .bridge_state
                .as_mut()
                .and_then(|s| s.cell_mut(x, y))
            {
                if state <= 17 {
                    runtime.axis = Some(if state <= 8 { Axis::NS } else { Axis::EW });
                }
                runtime.damage_state = if state == 0 && flags & BRIDGE_FLAG_STRUCTURAL == 0 {
                    DamageState::Destroyed
                } else {
                    DamageState::from_state_byte(state).unwrap_or(runtime.damage_state)
                };
            }
            if let (Some(grid), Some(terrain)) = (
                self.sim.overlay_grid.as_mut(),
                self.sim.resolved_terrain.as_mut(),
            ) {
                grid.write_literal_bridge_state(terrain, x, y, state);
            } else {
                self.sim
                    .resolved_terrain
                    .as_mut()
                    .unwrap()
                    .write_native_cell_state(cell, state);
            }
        } else {
            self.sim
                .resolved_terrain
                .as_mut()
                .unwrap()
                .write_native_cell_state(cell, state);
        }
        self.retain_real_write(cell);
    }
    fn write_anchor(&mut self, cell: Cell, anchor: Option<Cell>) {
        self.sim
            .resolved_terrain
            .as_mut()
            .unwrap()
            .write_native_cell_anchor(cell, anchor);
        self.retain_real_write(cell);
    }
    fn clear_overlay(&mut self, cell: Cell) {
        self.sim
            .resolved_terrain
            .as_mut()
            .unwrap()
            .clear_native_cell_overlay(cell);
        if let Some((x, y)) = self.real_coord(cell) {
            if let Some(grid) = self.sim.overlay_grid.as_mut() {
                grid.clear_literal_bridge_identity(x, y);
            }
            if let Some(runtime) = self
                .sim
                .bridge_state
                .as_mut()
                .and_then(|s| s.cell_mut(x, y))
            {
                runtime.overlay_byte = 0xff;
            }
        }
        self.retain_real_write(cell);
    }
    fn fallout(&mut self, cell: Cell) {
        self.collapsed = true;
        let (x, y) = self.coord(cell);
        blow_up_bridge_cell_fallout(self.sim, self.rules, x as u16, y as u16, self.registry);
    }
    fn radar(&mut self, cell: Cell) {
        let (x, y) = self.coord(cell);
        self.sim
            .mark_radar_terrain_dirty_cells([(x as u16, y as u16)]);
    }
    fn perpendicular(&mut self, input: CellCoord, axis: Axis, phase: Phase, direction: u8) {
        let (dx, dy) = crate::util::direction::DIRECTION_DELTAS[usize::from(direction & 7)];
        // Native helpers retain this requested coordinate on their stack;
        // it is distinct from the retained allocation's current +24.
        let target_coord = (
            input.0.wrapping_add(dx as i16),
            input.1.wrapping_add(dy as i16),
        );
        let target = self.lookup(target_coord);
        if self.flags(target) & BRIDGE_FLAG_ANCHOR_SELF != 0
            && let Some(next) =
                crate::sim::bridge_specs::apply_ramp_transition(self.state(target), axis, phase)
        {
            if next == 0 {
                self.perpendicular(target_coord, axis, phase, direction);
                publication::set_bridge_direction(
                    self,
                    target,
                    if axis == Axis::NS { 0 } else { 6 },
                    false,
                );
            }
            self.write_state(target, next);
            if next == 0 {
                self.clear_overlay(target);
                self.radar(target);
            }
        }
        if let Err(error) = self.perpendicular_tile_tail(target_coord, target, axis, phase, direction) {
            // An unavailable/unadmitted input is not a successful native
            // sparse fallback. Preserve completed writes and report the gap.
            log::error!("bridge tile update at {target_coord:?} failed: {error}");
        }
    }
    fn rim(&mut self, coord: CellCoord) {
        rim_publication::update(self, coord);
    }
    fn zones(&mut self, _anchor: Cell) {
        refresh_bridge_zones_if_dirty(self.sim, self.rules, true);
    }
}
