//! Per-tick particle-system AI dispatch.
//!
//! Drives every `ParticleSystem` in the store forward by one tick: spawns new
//! particles per the system's `BehavesLike`, advances per-particle state, and
//! cleans up dead systems / particles.
//!
//! Real implementation lands in Tasks C1–C4. This file currently only exposes
//! the entry point so Phase 5.5 of `Simulation::advance_tick` can wire to it.

use crate::sim::world::Simulation;

/// Advance every particle system in `sim.particle_systems` by one tick.
pub fn tick_particle_systems(_sim: &mut Simulation) {
    // Implemented in Tasks C1–C4.
}
