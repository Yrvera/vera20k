//! Per-match lighting lifetime and atomic visible-grid replacement.
//! Simulation supplies authoritative sources; this owner controls the derived
//! grid, identity, and deferred work across install, restore and live refresh.
use crate::map::lighting::{self, CellLightGrid, LightingConfig, LightingProfileUnits, PointLight};
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::ruleset::RuleSet;
use crate::sim::world::Simulation;

const CELL_LIGHT_GATHER_BUDGET: usize = 8_192;

pub(crate) struct MatchLighting {
    grid: CellLightGrid,
    config: LightingConfig,
    sources: Vec<PointLight>,
    profile: Option<LightingProfileUnits>,
    detail_level: u32,
    pending: Option<lighting::DeferredCellLightRefresh>,
    fingerprint: Option<u64>,
}

impl Default for MatchLighting {
    fn default() -> Self {
        Self {
            grid: CellLightGrid::new(),
            config: LightingConfig::default(),
            sources: Vec::new(),
            profile: None,
            detail_level: 2,
            pending: None,
            fingerprint: None,
        }
    }
}

impl MatchLighting {
    pub(crate) fn grid(&self) -> &CellLightGrid {
        &self.grid
    }

    /// Handoff replaces the complete old lighting lifetime. The live detail
    /// option supersedes the loader's default before the first tactical frame.
    pub(crate) fn install(
        &mut self,
        grid: CellLightGrid,
        config: LightingConfig,
        detail_level: u32,
        live: Option<(&ResolvedTerrainGrid, &Simulation, &RuleSet)>,
    ) {
        *self = Self {
            grid,
            config,
            detail_level: detail_level.min(2),
            ..Self::default()
        };
        if let Some((terrain, sim, rules)) = live {
            let view = derive_lighting_view(&self.config, Some(sim), Some(rules), detail_level);
            self.grid = build_lighting_grid_from_view(terrain, &view);
            self.fingerprint = Some(view.fingerprint);
            self.profile = Some(view.profile);
            self.detail_level = view.detail_level;
            self.sources = view.point_lights;
        }
    }

    /// Same-content restore immediately replaces visible lighting and cancels
    /// old pending work. Preserve the existing stale-identity policy: the next
    /// live refresh resamples the restored world through the deferred path.
    pub(crate) fn restore(
        &mut self,
        terrain: &ResolvedTerrainGrid,
        sim: &Simulation,
        rules: &RuleSet,
        detail_level: u32,
    ) {
        self.grid = rebuild_lighting_grid_from_sim(
            terrain,
            &self.config,
            Some(sim),
            Some(rules),
            detail_level,
        );
        self.pending = None;
        self.sources.clear();
        self.profile = None;
        self.detail_level = detail_level.min(2);
        self.fingerprint = None;
    }

    /// YR LightSourceClass-style gather/commit boundary: the visible grid stays
    /// stable until every replacement cell has been sampled.
    pub(crate) fn refresh(
        &mut self,
        terrain: &ResolvedTerrainGrid,
        sim: &Simulation,
        rules: &RuleSet,
        detail_level: u32,
    ) {
        let changed_view = {
            let view = derive_lighting_view(&self.config, Some(sim), Some(rules), detail_level);
            if self.fingerprint == Some(view.fingerprint) {
                None
            } else {
                let profile_changed =
                    self.profile != Some(view.profile) || self.detail_level != view.detail_level;
                let affected_cells = if profile_changed {
                    terrain
                        .iter()
                        .map(|cell| ((cell.rx, cell.ry), cell.level))
                        .collect()
                } else {
                    // Source identity is not projected into PointLight. Enumerate
                    // the union of old and new source areas so identical colocated
                    // sources and multiplicity changes cannot disappear in a set diff.
                    let mut seen = std::collections::BTreeSet::new();
                    let mut cells = Vec::new();
                    for source in self.sources.iter().chain(view.point_lights.iter()) {
                        for record in crate::map::lighting::point_light_area_cells(
                            source,
                            terrain.width(),
                            terrain.height(),
                            |rx, ry| terrain.cell(rx, ry).map(|cell| cell.level),
                        ) {
                            if seen.insert(record.0) {
                                cells.push(record);
                            }
                        }
                    }
                    cells
                };
                Some((view, affected_cells))
            }
        };

        if let Some((view, affected_cells)) = changed_view {
            // After enumerating the replacement area, finish the old batch before installing the new one.
            if let Some(mut pending) = self.pending.take() {
                pending.gather_all();
                let committed = pending.commit_into(&mut self.grid);
                debug_assert!(committed);
            }
            self.fingerprint = Some(view.fingerprint);
            self.profile = Some(view.profile);
            self.detail_level = view.detail_level;
            self.sources = view.point_lights.clone();
            self.pending = (!affected_cells.is_empty()).then(|| {
                crate::map::lighting::DeferredCellLightRefresh::new_with_profile(
                    affected_cells,
                    view.profile,
                    view.detail_level,
                    view.point_lights,
                )
            });
        }

        let completed = self
            .pending
            .as_mut()
            .is_some_and(|pending| pending.gather(CELL_LIGHT_GATHER_BUDGET));
        if completed {
            let pending = self
                .pending
                .take()
                .expect("completed lighting refresh remains installed");
            let committed = pending.commit_into(&mut self.grid);
            debug_assert!(committed, "completed lighting refresh commits atomically");
        }
    }
}

/// Fully-derived render-facing lighting view. The simulation owns only the
/// scenario controller and source inputs; the per-cell grid remains app state.
pub(crate) struct DerivedLightingView {
    pub(crate) profile: LightingProfileUnits,
    pub(crate) point_lights: Vec<PointLight>,
    pub(crate) detail_level: u32,
    pub(crate) fingerprint: u64,
}

/// Derive the complete visible lighting input from one committed world view.
pub(crate) fn derive_lighting_view(
    lighting_config: &LightingConfig,
    simulation: Option<&Simulation>,
    rules: Option<&RuleSet>,
    detail_level: u32,
) -> DerivedLightingView {
    let mut fingerprint = LightingFingerprint::new();
    let profile = simulation.map_or_else(
        || lighting::normal_profile_units(lighting_config),
        |sim| {
            let state = &sim.session.lighting;
            let selected = match state.selected_profile {
                crate::sim::scenario_session::ScenarioLightingProfile::Normal => state.normal,
                crate::sim::scenario_session::ScenarioLightingProfile::Ion => state.ion,
            };
            fingerprint.mix_i32(state.target_ambient);
            fingerprint.mix_u64(match state.selected_profile {
                crate::sim::scenario_session::ScenarioLightingProfile::Normal => 0,
                crate::sim::scenario_session::ScenarioLightingProfile::Ion => 1,
            });
            fingerprint.mix_i32(state.transition_timer.start_frame());
            fingerprint.mix_i32(state.transition_timer.duration());
            LightingProfileUnits {
                ambient_percent: state.current_ambient,
                red_percent: selected.red_percent,
                green_percent: selected.green_percent,
                blue_percent: selected.blue_percent,
                ground_units: selected.ground_units,
                level_units: selected.level_units,
            }
        },
    );
    fingerprint.mix_profile(profile);
    fingerprint.mix_u64(u64::from(detail_level));

    let building_lights = collect_live_building_lights(simulation, rules, detail_level);
    let radiation_lights = match (simulation, rules) {
        (Some(sim), Some(rules)) => {
            crate::app::presentation::radiation_light::collect_radiation_lights(sim, rules)
        }
        _ => Vec::new(),
    };

    let mut point_lights = Vec::with_capacity(building_lights.len() + radiation_lights.len());
    for (stable_id, light) in building_lights {
        fingerprint.mix_u64(0x42);
        fingerprint.mix_u64(stable_id);
        fingerprint.mix_point_light(&light);
        point_lights.push(light);
    }
    for light in radiation_lights {
        fingerprint.mix_u64(0x52);
        fingerprint.mix_point_light(&light);
        point_lights.push(light);
    }

    DerivedLightingView {
        profile,
        point_lights,
        detail_level: detail_level.min(2),
        fingerprint: fingerprint.finish(),
    }
}

/// Build the cell grid for an already-derived complete view.
pub(crate) fn build_lighting_grid_from_view(
    resolved_terrain: &ResolvedTerrainGrid,
    view: &DerivedLightingView,
) -> CellLightGrid {
    let mut grid = lighting::build_cell_light_grid_from_heights_and_units_with_detail(
        resolved_terrain
            .iter()
            .map(|cell| ((cell.rx, cell.ry), cell.level)),
        view.profile,
        view.detail_level,
    );
    lighting::accumulate_point_lights(&mut grid, &view.point_lights);
    grid
}

/// Rebuild transient app lighting from the selected scenario profile plus the
/// current live building and radiation sources.
pub(crate) fn rebuild_lighting_grid_from_sim(
    resolved_terrain: &ResolvedTerrainGrid,
    lighting_config: &LightingConfig,
    simulation: Option<&Simulation>,
    rules: Option<&RuleSet>,
    detail_level: u32,
) -> CellLightGrid {
    let view = derive_lighting_view(lighting_config, simulation, rules, detail_level);
    build_lighting_grid_from_view(resolved_terrain, &view)
}

fn collect_live_building_lights(
    simulation: Option<&Simulation>,
    rules: Option<&RuleSet>,
    detail_level: u32,
) -> Vec<(u64, PointLight)> {
    let (Some(sim), Some(rules)) = (simulation, rules) else {
        return Vec::new();
    };
    if detail_level < 2 {
        return Vec::new();
    }
    sim.entities()
        .values()
        .filter(|entity| {
            entity.category == crate::map::entities::EntityCategory::Structure
                && entity.lifecycle.object_alive
                && !entity.lifecycle.in_limbo
                && entity.lifecycle.cell_marked
                && !entity.dying
                && entity.health.current > 0
                && crate::sim::power_system::is_building_powered(
                    &sim.power_states,
                    rules,
                    entity,
                    &sim.interner,
                )
        })
        .filter_map(|entity| {
            let type_id = sim.interner.resolve(entity.type_ref());
            let obj = rules.object(type_id)?;
            let light = lighting::point_light_from_object(
                entity.position.rx,
                entity.position.ry,
                obj.light_visibility,
                obj.light_intensity,
                [
                    obj.light_red_tint,
                    obj.light_green_tint,
                    obj.light_blue_tint,
                ],
            )?;
            Some((entity.stable_id(), light))
        })
        .collect()
}

struct LightingFingerprint(u64);

impl LightingFingerprint {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        Self(Self::OFFSET)
    }

    fn mix_u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn mix_i32(&mut self, value: i32) {
        self.mix_u64(u64::from(value as u32));
    }

    fn mix_profile(&mut self, profile: LightingProfileUnits) {
        self.mix_i32(profile.ambient_percent);
        self.mix_i32(profile.red_percent);
        self.mix_i32(profile.green_percent);
        self.mix_i32(profile.blue_percent);
        self.mix_i32(profile.ground_units);
        self.mix_i32(profile.level_units);
    }

    fn mix_point_light(&mut self, light: &PointLight) {
        self.mix_u64(u64::from(light.rx));
        self.mix_u64(u64::from(light.ry));
        self.mix_i32(light.center_x);
        self.mix_i32(light.center_y);
        self.mix_i32(light.radius_leptons);
        self.mix_i32(light.intensity);
        for tint in light.tint {
            self.mix_i32(tint);
        }
        self.mix_u64(u64::from(u8::from(light.active)));
        self.mix_u64(u64::from(u8::from(light.detail)));
    }

    fn finish(self) -> u64 {
        self.0
    }
}
