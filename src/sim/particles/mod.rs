//! Particle systems — authoritative sim state for visual + damage particle effects.
//!
//! Two-tier model:
//!   - `ParticleSystem` — container that owns a `Vec<Particle>`, manages spawning,
//!     dispatches per-tick AI based on its `ParticleSystemBehavesLike` type.
//!   - `Particle` — individual entity with position, velocity, lifetime, animation
//!     state, optionally dealing damage to cell occupants (gas / fire variants).
//!
//! Stored in `Simulation::particle_systems: ParticleSystemStore` (BTreeMap).
//! Particles never enter `EntityStore` — they're owned by their parent PSC.
//!
//! Tier 2 implements Smoke / Gas / Fire via the existing SHP render pipeline.
//! Spark / Railgun are parsed but spawn returns None (warn + skip).
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/ and util/ only.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::rules::particle_system_type::ParticleSystemTypeId;
use crate::rules::particle_type::ParticleTypeId;
use crate::sim::intern::InternedId;
use crate::util::fixed_math::SimFixed;
use glam::IVec3;
use std::collections::BTreeMap;

pub mod gas;
pub mod smoke;
pub mod spawn;
pub mod system_ai;
pub mod wind;

#[derive(Debug, Clone)]
pub struct ParticleSystem {
    pub stable_id: u64,
    pub type_id: ParticleSystemTypeId,
    pub coords: IVec3,
    pub offset: IVec3,
    pub particles: Vec<Particle>,
    pub spawn_timer: SimFixed,
    pub lifetime: i32,
    pub spark_spawn_frames: i32,
    pub facing: u8,
    pub marked_for_deletion: bool,
    pub directionless: bool,
    pub attached_entity: Option<u64>,
    pub owner_entity: Option<u64>,
    pub target_coords: IVec3,
    pub owner_house: Option<InternedId>,
    pub done_spawning: bool,
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub type_id: ParticleTypeId,
    pub coords: IVec3,
    pub previous_coords: IVec3,
    pub origin: IVec3,
    pub direction: [SimFixed; 3],
    pub velocity: SimFixed,
    pub lifetime_remaining: i16,
    pub damage_counter: i16,
    pub state_ai_advance: u8,
    pub animation_state: u8,
    pub translucency: u8,
    pub hit_ground: bool,
    pub marked_for_deletion: bool,

    pub drift_x: i32,
    pub drift_y: i32,
    pub drift_z: i32,

    pub current_color: [u8; 3],
    pub color_index: u8,
    pub color_accumulator: SimFixed,
}

impl ParticleSystem {
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }
}

/// Deterministic store for `ParticleSystem` instances.
///
/// Mirrors `EntityStore`: BTreeMap-backed so iteration is always sorted by
/// `stable_id`. Stable IDs are monotonically increasing and never reused —
/// `reinsert` re-uses an existing id when a tick borrow-juggle round-trips
/// a system through ownership.
#[derive(Debug, Clone, Default)]
pub struct ParticleSystemStore {
    systems: BTreeMap<u64, ParticleSystem>,
    next_id: u64,
}

impl ParticleSystemStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u64, &ParticleSystem)> + '_ {
        self.systems.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&u64, &mut ParticleSystem)> + '_ {
        self.systems.iter_mut()
    }

    pub fn get(&self, id: u64) -> Option<&ParticleSystem> {
        self.systems.get(&id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut ParticleSystem> {
        self.systems.get_mut(&id)
    }

    /// Inserts a new system; assigns and returns the next stable id.
    pub fn insert(&mut self, mut sys: ParticleSystem) -> u64 {
        self.next_id += 1;
        sys.stable_id = self.next_id;
        let id = self.next_id;
        self.systems.insert(id, sys);
        id
    }

    /// Re-inserts a system at its existing stable id (used by tick borrow-juggle).
    pub fn reinsert(&mut self, sys: ParticleSystem) {
        let id = sys.stable_id;
        debug_assert!(id > 0, "reinsert requires a previously-assigned stable_id");
        self.systems.insert(id, sys);
    }

    pub fn remove(&mut self, id: u64) -> Option<ParticleSystem> {
        self.systems.remove(&id)
    }

    pub fn len(&self) -> usize {
        self.systems.len()
    }

    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }

    /// Snapshot of stable IDs for tick traversal — collects to a `Vec` so
    /// the caller can mutate the store while iterating.
    pub fn ids(&self) -> Vec<u64> {
        self.systems.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_system() -> ParticleSystem {
        ParticleSystem {
            stable_id: 0,
            type_id: ParticleSystemTypeId(0),
            coords: IVec3::ZERO,
            offset: IVec3::ZERO,
            particles: Vec::new(),
            spawn_timer: SimFixed::from_num(0),
            lifetime: -1,
            spark_spawn_frames: 0,
            facing: 0x1D,
            marked_for_deletion: false,
            directionless: false,
            attached_entity: None,
            owner_entity: None,
            target_coords: IVec3::ZERO,
            owner_house: None,
            done_spawning: false,
        }
    }

    #[test]
    fn insert_assigns_increasing_ids() {
        let mut store = ParticleSystemStore::new();
        let a = store.insert(fake_system());
        let b = store.insert(fake_system());
        assert!(b > a);
    }

    #[test]
    fn iteration_is_sorted_by_id() {
        let mut store = ParticleSystemStore::new();
        let _ = store.insert(fake_system());
        let _ = store.insert(fake_system());
        let _ = store.insert(fake_system());
        let ids: Vec<u64> = store.iter().map(|(id, _)| *id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
    }

    #[test]
    fn reinsert_preserves_id() {
        let mut store = ParticleSystemStore::new();
        let id = store.insert(fake_system());
        let sys = store.remove(id).unwrap();
        store.reinsert(sys);
        assert!(store.get(id).is_some());
        assert_eq!(store.len(), 1);
    }
}
