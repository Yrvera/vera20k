//! Map/cell-substrate read services. First member: the bridge topology service.
//!
//! These are read-only accessors over the canonical post-map-load cell store.
//! They own the gamemd-native bit semantics, signed height math, and the
//! cell-list layer selectors so that movement, combat, occupancy, and
//! pathfinding read one consolidated owner instead of re-deriving each
//! primitive at its call site.
//!
//! ## Dependency rules
//! - Part of sim/ — may depend on map/ (bridge_facts flag bits, resolved_terrain)
//!   and other sim/ modules.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/ (invariant #1).
//!   The render draw-offset lives behind a render-facing trait in render/, so the
//!   sim-side service never gains a render dependency.
pub mod bridge_topology;

/// SHADOW two-layer object-list occupancy (GATE A2). NOT authoritative — the live
/// occupancy store stays `sim::occupancy::OccupancyGrid`. Parallel gamemd-faithful
/// repr used to validate the bridge two-layer/order/transition contract via tests.
pub mod bridge_occupancy_shadow;
