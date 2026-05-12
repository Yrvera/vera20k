//! Body rocking and slope-transition simulation.
//!
//! Implements the spring-damper that drives `RockingState::angle_*` toward zero
//! each tick, plus the 3-tick slope-transition tracker. The renderer reads the
//! resulting angles + slope-blend matrix to compose the body matrix per frame.
//!
//! ## Dependency rules
//! - Part of sim/ — depends only on sim/components, sim/entity_store, map,
//!   rules, util/fixed_math. Never on render/, ui/, audio/, net/.

pub mod impulse;
pub mod rocking_system;
pub mod self_destruct;

#[cfg(test)]
#[path = "rocking_tests.rs"]
mod rocking_tests;

pub use impulse::apply_rocker_impulse;
pub use rocking_system::tick;
pub use self_destruct::{NoopSelfDestruct, SelfDestructHook, check_and_fire};
