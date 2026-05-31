//! The object substrate: the single owner of the active-object vector and the
//! monotonic identity / enter-order counters that the lifecycle contract mutates.
//!
//! This is stage 1 of the substrate consolidation — it holds the
//! bookkeeping/ordering state only. The lifecycle methods
//! (`reveal`/`conceal`/`unlimbo`/`uninit`) stay on `Simulation` for now because
//! they also need `EntityStore`/`OccupancyGrid`; they reach this state by path
//! (`self.substrate.*`). Entity storage and the occupancy grid migrate into the
//! substrate in later stages.
//!
//! Dependency rules: part of sim/ — depends only on std + serde + the sibling
//! `LogicVector`.

use serde::{Deserialize, Serialize};

use super::LogicVector;
use crate::sim::occupancy::OccupancyGrid;

/// Owns the active-object order and the substrate's monotonic counters. Field
/// paths are `Simulation.substrate.*`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ObjectSubstrate {
    /// Monotonic per-instance id source (never reused). Each spawned entity
    /// draws the next value; a stale reference degrades to `None` rather than
    /// aliasing a reused slot.
    pub(crate) next_stable_entity_id: u64,
    /// Monotonic source for rebuilt CellClass-style object-list order.
    /// `OccupancyGrid` itself is a skipped cache; each entity stores the last
    /// order value assigned when it entered a cell list.
    pub(crate) next_occupancy_enter_order: u64,
    /// LogicClass active-object vector — the single authority on object order.
    /// Tail-append on reveal, compacting-remove on conceal. Serialized verbatim.
    #[serde(default)]
    pub(crate) logic: LogicVector,
    /// CellClass-style occupancy grid (per-cell object lists). A rebuilt cache:
    /// `#[serde(skip)]`, reconstructed from the entity store on load, so it never
    /// appears in the serialized snapshot and does not enter the state hash directly.
    #[serde(skip)]
    pub(crate) occupancy: OccupancyGrid,
}

impl ObjectSubstrate {
    /// Fresh substrate for a new world. Counters start at 1 (0 is a reserved
    /// sentinel), matching the pre-consolidation `Simulation::new` initializers.
    pub(crate) fn new() -> Self {
        Self {
            next_stable_entity_id: 1,
            next_occupancy_enter_order: 1,
            logic: LogicVector::new(),
            occupancy: OccupancyGrid::new(),
        }
    }
}

impl Default for ObjectSubstrate {
    fn default() -> Self {
        Self::new()
    }
}
