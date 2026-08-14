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
use crate::sim::anim_class::AnimStore;
use crate::sim::cell_rect::CellReservationGrid;
use crate::sim::entity_store::EntityStore;
use crate::sim::occupancy::{
    CellOccupationGrid, HiddenOccupationGrid, OccupancyGrid, RawCellOccupationGrid,
};
use crate::sim::particles::ParticleSystemStore;

const FIRST_MULTIPLAYER_FEEDBACK_ANIM_ID: u64 = 1 << 63;

const fn first_multiplayer_feedback_anim_id() -> u64 {
    FIRST_MULTIPLAYER_FEEDBACK_ANIM_ID
}

/// Monotonic source for rebuilt CellClass-style object-list (enter) order. Each
/// entity stores the last value assigned when it entered a cell list; this counter
/// hands out the next one. The sole mutator is `next()` — callers cannot mis-increment
/// or skip the saturating semantics. Serialized + hashed at its `ObjectSubstrate` field
/// (a `#[serde(transparent)]` + derived-`Hash` newtype is byte- and hash-identical to the
/// bare `u64` it replaces).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EnterOrderCounter(u64);

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

    /// Next value that will be handed out. Snapshot restoration uses this to
    /// reject a counter that could reuse an already-restored cell-entry order.
    pub(crate) const fn current(self) -> u64 {
        self.0
    }
}

/// Owns the active-object order and the substrate's monotonic counters. Field
/// paths are `Simulation.substrate.*`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ObjectSubstrate {
    /// Monotonic per-instance id source (never reused). Each spawned entity
    /// draws the next value; a stale reference degrades to `None` rather than
    /// aliasing a reused slot.
    pub(crate) next_stable_object_id: u64,
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
    /// Independent ground/deck vehicle-occupation bit planes. Rebuilt from
    /// entity lifecycle and serialized Drive footprint state after load.
    #[serde(skip)]
    pub(crate) cell_occupation: CellOccupationGrid,
    /// Authoritative raw CellClass occupation bytes. Unlike the owner-aware
    /// Drive compatibility cache above, these destructive OR/AND-not bytes are
    /// serialized verbatim and are never rebuilt from entity lists.
    #[serde(default)]
    pub(crate) raw_cell_occupation: RawCellOccupationGrid,
    /// Separate authoritative building hidden-object counters. Serialized
    /// verbatim because RemoveOccupy's enter-only cancellation is not
    /// reconstructible from the currently placed entity set.
    #[serde(default)]
    pub(crate) hidden_occupation: HiddenOccupationGrid,
    /// Authoritative CellClass `+0xDC` per-house Building base reservations,
    /// including the single shared dummy CellClass mask.
    #[serde(default)]
    pub(crate) base_reservations: CellReservationGrid,
    /// Plain-struct entity storage (`BTreeMap<u64, GameEntity>` + by_owner index).
    /// The authoritative object store — serialized verbatim (NOT skipped).
    pub(crate) entities: EntityStore,
    /// Separate AnimClass registry sharing the global object ID namespace and
    /// LogicVector with entities.
    #[serde(default)]
    pub(crate) anims: AnimStore,
    /// Multiplayer click-feedback animations use a separate, sync-exempt
    /// registry and never enter the ordinary LogicVector.
    #[serde(skip)]
    pub(crate) multiplayer_feedback_anims: AnimStore,
    // Reserved for the verified sync-exempt feedback spawn path, which is not wired yet.
    #[allow(dead_code)]
    #[serde(skip, default = "first_multiplayer_feedback_anim_id")]
    pub(crate) next_multiplayer_feedback_anim_id: u64,
    #[serde(skip)]
    pub(crate) multiplayer_feedback_pending_delete: Vec<u64>,
    /// ParticleSystemClass registry. Systems share the global object-ID
    /// namespace and LogicVector; individual particles remain container-owned.
    #[serde(default)]
    pub(crate) particle_systems: ParticleSystemStore,
    /// Deferred-delete queue (the native `PendingDeleteList`). Ordered IDs may
    /// survive a Rust snapshot boundary: between enqueue and the ordinary late
    /// drain an entity remains resolvable in storage while its independent
    /// lifecycle facts describe dead/limbo/cell/logic state. Serialized verbatim;
    /// the state hash folds the queue length followed by IDs in insertion order.
    #[serde(default)]
    pub(crate) pending_delete: Vec<u64>,
}

impl ObjectSubstrate {
    /// Fresh substrate for a new world. Counters start at 1 (0 is a reserved
    /// sentinel), matching the pre-consolidation `Simulation::new` initializers.
    pub(crate) fn new() -> Self {
        Self {
            next_stable_object_id: 1,
            next_occupancy_enter_order: EnterOrderCounter::new(),
            logic: LogicVector::new(),
            occupancy: OccupancyGrid::new(),
            cell_occupation: CellOccupationGrid::new(),
            raw_cell_occupation: RawCellOccupationGrid::new(),
            hidden_occupation: HiddenOccupationGrid::new(),
            base_reservations: CellReservationGrid::new(),
            entities: EntityStore::new(),
            anims: AnimStore::default(),
            multiplayer_feedback_anims: AnimStore::default(),
            next_multiplayer_feedback_anim_id: FIRST_MULTIPLAYER_FEEDBACK_ANIM_ID,
            multiplayer_feedback_pending_delete: Vec::new(),
            particle_systems: ParticleSystemStore::default(),
            pending_delete: Vec::new(),
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
