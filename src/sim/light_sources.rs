//! Retained radius-light ownership and ordered presentation inputs.
//!
//! Active-retail Building614 owns a nullable LightSource, whose48 active byte
//! changes at explicit lifecycle sites. Ordinary House power changes do not
//! rebuild this state. These runtime caches/outputs never feed gameplay or RNG;
//! Cell sampling and palette ownership remain outside Simulation.
use std::collections::BTreeMap;

use crate::map::lighting::{PointLight, point_light_from_object};
use crate::rules::ruleset::RuleSet;
use crate::sim::scenario_session::{ScenarioLightingProfile, ScenarioLightingState};
use crate::sim::world::Simulation;

#[derive(Debug, Clone)]
pub(crate) enum LightingEvent {
    Building {
        id: u64,
        source: Option<PointLight>,
    },
    Radiation {
        center: (u16, u16),
        source: Option<PointLight>,
    },
    Global(ScenarioLightingState),
}

/// Native source pointers are discarded during Building load (454174), so
/// neither these records nor outgoing operations are serialized as pointers.
/// The existing eager restore path is a compatibility reconstruction, not a
/// claim to native Cell34 lazy reinitialization.
#[derive(Debug, Clone, Default)]
pub(crate) struct LightingSources {
    pub(crate) buildings: BTreeMap<u64, PointLight>,
    pub(crate) pending: Vec<LightingEvent>,
}

impl Simulation {
    /// Preserve the existing eager restore projection while resetting the
    /// outgoing timeline. Native lazy post-load reconstruction remains open.
    pub(crate) fn rebuild_lighting_sources_after_load(&mut self, rules: &RuleSet) {
        self.lighting_sources = LightingSources::default();
        self.radiation.take_lighting_events();
        let ids: Vec<_> = self
            .entities()
            .values()
            .filter(|entity| {
                entity.lifecycle.object_alive
                    && !entity.lifecycle.in_limbo
                    && entity.lifecycle.cell_marked
                    && !entity.dying
                    && entity.health.current > 0
            })
            .map(|entity| entity.stable_id())
            .collect();
        for id in ids {
            self.allocate_building_light(id, rules);
        }
        self.lighting_sources.pending.clear();
    }

    pub(crate) fn discard_lighting_events(&mut self) {
        self.radiation.take_lighting_events();
        self.lighting_sources.pending.clear();
    }

    /// Successful Unlimbo440DFD / construction446767 allocates only when614
    /// is null, then calls554A60(0). Parameter conversion is the same existing
    /// map-light conversion; its floating inputs have presentation-only use.
    pub(crate) fn allocate_building_light(&mut self, id: u64, rules: &RuleSet) {
        if self.lighting_sources.buildings.contains_key(&id) {
            return;
        }
        let Some(entity) = self.entities().get(id) else {
            return;
        };
        if entity.category != crate::map::entities::EntityCategory::Structure {
            return;
        }
        let Some(object) = rules.object(self.interner.resolve(entity.type_ref())) else {
            return;
        };
        let Some(source) = point_light_from_object(
            entity.position.rx,
            entity.position.ry,
            object.light_visibility,
            object.light_intensity,
            [
                object.light_red_tint,
                object.light_green_tint,
                object.light_blue_tint,
            ],
        ) else {
            return;
        };
        self.flush_radiation_lighting();
        self.lighting_sources.buildings.insert(id, source.clone());
        self.lighting_sources.pending.push(LightingEvent::Building {
            id,
            source: Some(source),
        });
    }

    /// Native554A60/A80 suppress repeated writes to an already matching48.
    /// Death44264C and sell449F1E/44A20B disable before destruction effects.
    pub(crate) fn set_building_light_active(&mut self, id: u64, active: bool) {
        let Some(source) = self.lighting_sources.buildings.get_mut(&id) else {
            return;
        };
        if source.active == active {
            return;
        }
        source.active = active;
        let source = source.clone();
        self.flush_radiation_lighting();
        self.lighting_sources.pending.push(LightingEvent::Building {
            id,
            source: Some(source),
        });
    }

    /// Building destructor43BD47 disables, destroys, and clears614. Generic
    /// Limbo is deliberately not used as an invented source-disable callback.
    pub(crate) fn destroy_building_light(&mut self, id: u64) {
        if self.lighting_sources.buildings.remove(&id).is_some() {
            self.flush_radiation_lighting();
            self.lighting_sources
                .pending
                .push(LightingEvent::Building { id, source: None });
        }
    }

    pub(crate) fn flush_radiation_lighting(&mut self) {
        self.lighting_sources.pending.extend(
            self.radiation
                .take_lighting_events()
                .into_iter()
                .map(|(center, source)| LightingEvent::Radiation { center, source }),
        );
    }

    /// Logic55B4C6 and profile53C280->53AD00 must remain in order with source
    /// invalidations. A final-frame fingerprint cannot recover that history.
    pub(crate) fn publish_global_lighting(&mut self) {
        self.flush_radiation_lighting();
        self.lighting_sources
            .pending
            .push(LightingEvent::Global(self.session.lighting));
    }

    pub(crate) fn select_lighting_profile(&mut self, profile: ScenarioLightingProfile) {
        match profile {
            ScenarioLightingProfile::Normal => self.session.lighting.select_normal(),
            ScenarioLightingProfile::Ion => self.session.lighting.select_ion(),
        }
        self.publish_global_lighting();
    }
}
