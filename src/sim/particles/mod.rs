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
