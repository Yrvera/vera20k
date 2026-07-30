//! Installed-locomotor state: which class a unit runs, and how it got there.
//!
//! The read-only half of locomotion — class identity, the retail CLSID table,
//! capability and base-slot defaults — lives in [`crate::sim::substrate::locomotion`].
//! This module owns the *installed* side: resolving a type's locomotor at spawn
//! and naming the result.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/ and `sim::substrate::locomotion` only.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

pub mod install;
pub mod piggyback;
pub mod slot;

pub use install::{resolve_installed_class, resolve_installed_kind};
pub use piggyback::{BeginOutcome, StashedLocomotor};
pub use slot::LocomotorSlot;
