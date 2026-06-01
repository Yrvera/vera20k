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
use crate::sim::entity_store::EntityStore;
use crate::sim::occupancy::OccupancyGrid;

/// Monotonic source for rebuilt CellClass-style object-list (enter) order. Each
/// entity stores the last value assigned when it entered a cell list; this counter
/// hands out the next one. The sole mutator is `next()` — callers cannot mis-increment
/// or skip the saturating semantics. Serialized + hashed at its `ObjectSubstrate` field
/// (a `#[serde(transparent)]` + derived-`Hash` newtype is byte- and hash-identical to the
/// bare `u64` it replaces).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct EnterOrderCounter(u64);

impl EnterOrderCounter {
    /// Fresh counter. Starts at 1; 0 is the reserved sentinel.
    pub(crate) const fn new() -> Self {
        Self(1)
    }

    /// Return the current order value and advance. Saturating — never wraps,
    /// matching the pre-consolidation `saturating_add(1)` at every assign-site.
    pub(crate) fn next(&mut self) -> u64 {
        let order = self.0;
        self.0 = self.0.saturating_add(1);
        order
    }
}

/// Owns the active-object order and the substrate's monotonic counters. Field
/// paths are `Simulation.substrate.*`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ObjectSubstrate {
    /// Monotonic per-instance id source (never reused). Each spawned entity
    /// draws the next value; a stale reference degrades to `None` rather than
    /// aliasing a reused slot.
    pub(crate) next_stable_entity_id: u64,
    /// Monotonic source for rebuilt CellClass-style object-list (enter) order.
    /// See `EnterOrderCounter`. `OccupancyGrid` itself is a skipped cache; each
    /// entity stores the last order value assigned when it entered a cell list.
    pub(crate) next_occupancy_enter_order: EnterOrderCounter,
    /// LogicClass active-object vector — the single authority on object order.
    /// Tail-append on reveal, compacting-remove on conceal. Serialized verbatim.
    #[serde(default)]
    pub(crate) logic: LogicVector,
    /// CellClass-style occupancy grid (per-cell object lists). A rebuilt cache:
    /// `#[serde(skip)]`, reconstructed from the entity store on load, so it never
    /// appears in the serialized snapshot and does not enter the state hash directly.
    #[serde(skip)]
    pub(crate) occupancy: OccupancyGrid,
    /// Plain-struct entity storage (`BTreeMap<u64, GameEntity>` + by_owner index).
    /// The authoritative object store — serialized verbatim (NOT skipped).
    pub(crate) entities: EntityStore,
}

impl ObjectSubstrate {
    /// Fresh substrate for a new world. Counters start at 1 (0 is a reserved
    /// sentinel), matching the pre-consolidation `Simulation::new` initializers.
    pub(crate) fn new() -> Self {
        Self {
            next_stable_entity_id: 1,
            next_occupancy_enter_order: EnterOrderCounter::new(),
            logic: LogicVector::new(),
            occupancy: OccupancyGrid::new(),
            entities: EntityStore::new(),
        }
    }
}

impl Default for ObjectSubstrate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_order_counter_new_starts_at_one() {
        let mut c = EnterOrderCounter::new();
        // First handout is 1 (0 is the reserved sentinel).
        assert_eq!(c.next(), 1);
    }

    #[test]
    fn enter_order_counter_next_returns_pre_increment_then_advances() {
        let mut c = EnterOrderCounter::new();
        assert_eq!(c.next(), 1);
        assert_eq!(c.next(), 2);
        assert_eq!(c.next(), 3);
    }

    #[test]
    fn enter_order_counter_saturates_at_max() {
        let mut c = EnterOrderCounter(u64::MAX);
        // Returns MAX, then stays MAX (saturating, never wraps to 0).
        assert_eq!(c.next(), u64::MAX);
        assert_eq!(c.next(), u64::MAX);
    }
}
