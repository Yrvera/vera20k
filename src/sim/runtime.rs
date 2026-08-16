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

pub struct SimResources {
    /// Fixed per-cell terrain heights parsed from the loaded map.
    pub height_map: std::collections::BTreeMap<(u16, u16), u8>,
    /// Bridge-deck heights layered above the terrain heights.
    pub bridge_height_map: std::collections::BTreeMap<(u16, u16), u8>,
    /// Rules-semantic overlay registry for the loaded match.
    pub overlay_registry: crate::rules::overlay_types::OverlayTypeRegistry,
    /// The immutable base resolved-terrain template: source-derived, used for
    /// static rendering and snapshot restore. Never the live runtime grid -
    /// the simulation owns and rebuilds its own live resolved terrain (F08
    /// naming, bound as an F07 cone).
    pub terrain_template: Option<crate::map::resolved_terrain::ResolvedTerrainGrid>,
    /// The complete immutable match rules (including the sole ArtRegistry).
    pub rules: crate::rules::ruleset::RuleSet,
    /// Immutable trigger definitions parsed from the map; the runtime state
    /// machine lives in the simulation, these are bound once (F07: the app
    /// no longer passes definitions each frame).
    pub trigger_graph: crate::map::trigger_graph::TriggerGraph,
    pub triggers: crate::map::triggers::TriggerMap,
    pub events: crate::map::events::EventMap,
    pub actions: crate::map::actions::ActionMap,
}

impl SimResources {
    /// Empty pre-bind resources for fixture and fallback construction.
    pub fn empty() -> Self {
        Self {
            height_map: std::collections::BTreeMap::new(),
            bridge_height_map: std::collections::BTreeMap::new(),
            overlay_registry: crate::rules::overlay_types::OverlayTypeRegistry::empty(),
            rules: crate::rules::ruleset::RuleSet::from_ini(
                &crate::rules::ini_parser::IniFile::from_str(""),
            )
            .expect("empty rules parse"),
            terrain_template: None,
            trigger_graph: Default::default(),
            triggers: Default::default(),
            events: Default::default(),
            actions: Default::default(),
        }
    }
}

/// The runtime owner: one deterministic simulation plus its bound resources.
pub struct SimRuntime {
    pub simulation: Simulation,
    pub resources: SimResources,
}

impl SimRuntime {
    /// Wrap an already-constructed simulation. Scenario construction moves
    /// here in F09; this keeps the F07 slot move atomic and behavior-free.
    #[cfg(test)]
    pub(crate) fn from_simulation(simulation: Simulation) -> Self {
        Self {
            simulation,
            resources: SimResources::empty(),
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

impl SimRuntime {
    /// The production frame transaction: advance one lane-tagged frame using
    /// the bound immutable resources. Callers cannot substitute rules, maps,
    /// registries, definitions, or navigation (the simulation pins its own
    /// canonical path snapshot internally).
    pub(crate) fn advance_frame(
        &mut self,
        commands: &[crate::sim::command::CommandEnvelope],
        tick_ms: u32,
        lane: crate::sim::world::TickLane,
    ) -> crate::sim::world::SimFrameOutput {
        self.simulation.advance_app_frame(
            commands,
            Some(&self.resources.rules),
            &self.resources.height_map,
            Some(&self.resources.overlay_registry),
            tick_ms,
            lane,
            Some(crate::sim::world::TriggerInputs {
                graph: &self.resources.trigger_graph,
                triggers: &self.resources.triggers,
                events: &self.resources.events,
                actions: &self.resources.actions,
            }),
        )
    }
}

impl SimRuntime {
    /// Rebind a restored simulation over the surviving match resources
    /// (same-content in-scenario load: rules, heights, registries, and
    /// trigger definitions are immutable match inputs and MUST carry over —
    /// an empty rebind would silently break every bound-resource consumer).
    pub fn rebind_restored(
        previous: Option<SimRuntime>,
        simulation: Simulation,
    ) -> SimRuntime {
        let resources = previous
            .map(|rt| rt.resources)
            .unwrap_or_else(SimResources::empty);
        SimRuntime {
            simulation,
            resources,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// F07 matrix: the runtime always uses bound navigation and resources —
    /// both production install paths keep the match resources. The map-load
    /// install binds them from the load result; the in-scenario restore path
    /// (quickload / save-load panel) must CARRY the surviving resources, not
    /// rebind empty ones.
    #[test]
    fn runtime_always_uses_bound_navigation_and_resources() {
        let mut resources = SimResources::empty();
        resources.height_map.insert((3, 4), 7);
        let original = SimRuntime {
            simulation: Simulation::new(),
            resources,
        };

        let rebound = SimRuntime::rebind_restored(Some(original), Simulation::new());
        assert_eq!(
            rebound.resources.height_map.get(&(3, 4)),
            Some(&7),
            "restore must carry the match resources, never rebind empty"
        );

        // Without a surviving runtime (fixture-only path) the rebind is
        // explicitly empty rather than partially bound.
        let fresh = SimRuntime::rebind_restored(None, Simulation::new());
        assert!(fresh.resources.height_map.is_empty());
    }
}
