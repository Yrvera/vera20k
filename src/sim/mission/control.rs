//! Compatibility path for rules-owned immutable mission-control data.
//!
//! Canonical definitions and INI parsing live in `rules::mission_data`.
//! Runtime mission scheduling remains in the parent `sim::mission` module.

pub use crate::rules::mission_data::{MissionControl, MissionControlEntry};
