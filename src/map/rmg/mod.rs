//! Random Map Generator: reproduces gamemd's `.SED`-driven map generation.
//!
//! Consumes `map::theater` for tile identities and `util::native_x87` for
//! deterministic float math, and emits an in-memory `map::map_file::MapFile`.
//! Pre-play map construction only — nothing in `sim/` depends on this module.

pub mod rng;

pub use rng::RmgRng;
