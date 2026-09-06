//! World authority lent to synchronous damage receivers.
//!
//! Combat, Bullet, Wave and direct area damage share this transfer protocol.
//! Inline lifecycle hooks temporarily swap these same authorities back into the
//! world; their re-entry protocol remains owned by `SimulationCombatInlineHooks`.

use std::collections::BTreeMap;

use super::{HouseState, InternedId, SimRng, SimSoundEvent, Simulation};
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::sim::bridge_state::BridgeRuntimeState;
use crate::sim::entity_store::EntityStore;
use crate::sim::intern::StringInterner;
use crate::sim::miner::ResourceNode;
use crate::sim::occupancy::OccupancyGrid;
use crate::sim::overlay_grid::OverlayGrid;
use crate::sim::terrain_object::TerrainAreaState;

/// One explicit, consuming transfer back to the world after receivers return.
/// This is not an unwind guard: restoring through Drop would introduce a new
/// partial-transaction policy. Callers retain their distinct radiation, smudge,
/// missile, visibility and scheduler inputs.
#[must_use = "receiver authority must be returned to the world with finish"]
pub(super) struct ReceiverTransaction {
    pub entities: EntityStore,
    pub occupancy: OccupancyGrid,
    pub interner: StringInterner,
    pub main_rng: SimRng,
    pub scenario_rng: SimRng,
    pub resource_nodes: BTreeMap<(u16, u16), ResourceNode>,
    pub overlay_grid: Option<OverlayGrid>,
    pub resolved_terrain: Option<ResolvedTerrainGrid>,
    pub bridge_state: Option<BridgeRuntimeState>,
    pub sound_events: Vec<SimSoundEvent>,
    pub terrain_area_state: TerrainAreaState,
    /// Receiver-owned fields are merged, never assigned over live lifecycle state.
    pub houses: BTreeMap<InternedId, HouseState>,
}

impl ReceiverTransaction {
    pub(super) fn take_from(sim: &mut Simulation) -> Self {
        Self {
            entities: std::mem::take(&mut sim.substrate.entities),
            occupancy: std::mem::take(&mut sim.substrate.occupancy),
            interner: std::mem::take(&mut sim.interner),
            main_rng: std::mem::replace(&mut sim.main_rng, SimRng::new(0)),
            scenario_rng: std::mem::replace(&mut sim.scenario_rng, SimRng::new(0)),
            resource_nodes: std::mem::take(&mut sim.production.resource_nodes),
            overlay_grid: sim.overlay_grid.take(),
            resolved_terrain: sim.resolved_terrain.take(),
            bridge_state: sim.bridge_state.take(),
            sound_events: std::mem::take(&mut sim.sound_events),
            terrain_area_state: TerrainAreaState::take_from(
                &mut sim.production,
                &mut sim.substrate.raw_cell_occupation,
            ),
            houses: sim.houses.clone(),
        }
    }

    /// Restore terrain and retire inactive Logic slots BEFORE returning the
    /// entity/interner authorities, preserving the existing receiver commit
    /// order. Return the ordered navigation changes for the caller's own tail.
    pub(super) fn finish(self, sim: &mut Simulation) -> Vec<(u16, u16)> {
        let inactive_terrain_logic_ids = self.terrain_area_state.inactive_logic_ids();
        let navigation_changed_cells = self
            .terrain_area_state
            .restore_into(&mut sim.production, &mut sim.substrate.raw_cell_occupation);
        for stable_id in inactive_terrain_logic_ids {
            let retired = sim.retire_non_entity_object(stable_id);
            debug_assert!(retired);
        }

        sim.substrate.entities = self.entities;
        sim.substrate.occupancy = self.occupancy;
        sim.interner = self.interner;
        sim.main_rng = self.main_rng;
        sim.scenario_rng = self.scenario_rng;
        sim.production.resource_nodes = self.resource_nodes;
        sim.overlay_grid = self.overlay_grid;
        sim.resolved_terrain = self.resolved_terrain;
        sim.bridge_state = self.bridge_state;
        sim.sound_events = self.sound_events;
        sim.merge_receiver_house_state(&self.houses);
        navigation_changed_cells
    }
}
