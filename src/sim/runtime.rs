//! SimRuntime — the construction/resource/API boundary around `Simulation`.
//!
//! F07: app, headless, and replay execution consume the simulation through
//! this owner. `SimResources` will absorb the immutable per-match inputs
//! (rules/art, overlay registry, height/map facts, trigger definitions, base
//! terrain template) one cone per commit; until a cone moves, the app still
//! passes that input per frame. `SimView` is the immutable read facade
//! presentation borrows — no world clone, no mutation.
//!
//! ## Dependency rules
//! - Part of sim/; NEVER depends on render/, ui/, sidebar/, audio/, net/.

use crate::sim::world::Simulation;

/// Immutable per-match resources bound at construction (F07 cones land here).
#[derive(Default)]
pub struct SimResources {
    /// Fixed per-cell terrain heights parsed from the loaded map.
    pub height_map: std::collections::BTreeMap<(u16, u16), u8>,
    /// Bridge-deck heights layered above the terrain heights.
    pub bridge_height_map: std::collections::BTreeMap<(u16, u16), u8>,
}

/// The runtime owner: one deterministic simulation plus its bound resources.
pub struct SimRuntime {
    pub simulation: Simulation,
    pub resources: SimResources,
}

impl SimRuntime {
    /// Wrap an already-constructed simulation. Scenario construction moves
    /// here in F09; this keeps the F07 slot move atomic and behavior-free.
    pub fn from_simulation(simulation: Simulation) -> Self {
        Self {
            simulation,
            resources: SimResources::default(),
        }
    }

    /// Immutable read facade for presentation and diagnostics.
    pub fn view(&self) -> SimView<'_> {
        SimView {
            simulation: &self.simulation,
        }
    }
}

/// Immutable borrow facade over the committed simulation state. Getters grow
/// per consumer cone in F10; keeping it minimal avoids a speculative API.
pub struct SimView<'a> {
    simulation: &'a Simulation,
}

impl<'a> SimView<'a> {
    pub fn simulation(&self) -> &'a Simulation {
        self.simulation
    }
}
