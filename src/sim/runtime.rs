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

use crate::map::resolved_terrain::TerrainTileAnimation;
use crate::sim::anim_class::{AnimDrawRuntime, AnimWorldCoord};
use crate::sim::components::AnimClassSpawnDescriptor;
use crate::rules::ruleset::RuleSet;
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
/// per consumer cone (F10); presentation code reads through these instead of
/// reaching into `Simulation` fields directly.
pub struct SimView<'a> {
    simulation: &'a Simulation,
}

impl<'a> SimView<'a> {
    /// Escape hatch for not-yet-migrated consumers; cones retire it (F10).
    pub fn simulation(&self) -> &'a Simulation {
        self.simulation
    }

    pub fn interner(&self) -> &'a crate::sim::intern::StringInterner {
        &self.simulation.interner
    }

    pub fn entities(&self) -> &'a crate::sim::entity_store::EntityStore {
        self.simulation.entities()
    }

    pub fn session(&self) -> &'a crate::sim::scenario_session::ScenarioSession {
        &self.simulation.session
    }

    pub fn fog(&self) -> &'a crate::sim::vision::FogState {
        &self.simulation.fog
    }

    pub fn houses(
        &self,
    ) -> &'a std::collections::BTreeMap<crate::sim::intern::InternedId, crate::sim::house_state::HouseState>
    {
        &self.simulation.houses
    }

    pub fn path_grid(&self) -> Option<&'a crate::sim::pathfinding::PathGrid> {
        self.simulation.path_grid()
    }

    pub fn radar_events(&self) -> &'a crate::sim::radar::RadarEventQueue {
        &self.simulation.radar_events
    }

    pub fn bridge_state(&self) -> Option<&'a crate::sim::bridge_state::BridgeRuntimeState> {
        self.simulation.bridge_state.as_ref()
    }

    /// LogicClass active-object order — presentation draws in this order.
    pub(crate) fn tactical_registration_order(&self) -> &'a [u64] {
        self.simulation.tactical_registration_order()
    }

    /// Radar terrain invalidation plumbing for the minimap dirty-gate.
    pub(crate) fn radar_terrain_dirty(&self) -> (&'a [(u16, u16)], u64) {
        (
            &self.simulation.radar_terrain_dirty_cells,
            self.simulation.radar_terrain_dirty_generation,
        )
    }
}

impl SimRuntime {
    /// One command-free Ordinary-lane frame for side binaries (parity-digest):
    /// the same bound-resource transaction as `advance_frame`, with the
    /// crate-private frame output discarded so no internal type goes public.
    pub fn advance_idle_frame_for_tooling(&mut self, tick_ms: u32) {
        let _ = self.advance_frame(&[], tick_ms, crate::sim::world::TickLane::Ordinary);
    }

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

/// Construct the post-map-section terrain-attached AnimClass set.
///
/// gamemd-derived: the final `MapClass::InitCellAttributes @ 0x00568BB0`
/// anti-diagonal pass calls `CellClass::RecalcAttributes @ 0x0047D2B0`; the
/// constructor row at 0x0047DA3B..0x0047DA5F supplies delay 0, signed loop -1,
/// flags 0x1600 and constructor ZAdjust 0. The delay-zero constructor calls
/// `Middle`, then the producer writes the tile ZAdjust; the descriptor carries
/// the producer's explicit +0x196/+0x197 state into the Rust object.
pub(crate) fn spawn_terrain_tile_animations(
    sim: &mut Simulation,
    rules: &RuleSet,
    tile_animations: &[TerrainTileAnimation],
) -> Vec<u64> {
    const TILE_ANIM_DRAW_FLAGS: u32 = 0x1600;
    let mut spawned = Vec::with_capacity(tile_animations.len());
    for tile in tile_animations {
        let type_name = sim.interner.intern(&tile.anim_name);
        let descriptor = AnimClassSpawnDescriptor {
            type_name,
            rx: tile.rx,
            ry: tile.ry,
            sub_x: crate::util::fixed_math::SimFixed::from_num(
                tile.world_x
                    .wrapping_sub(i32::from(tile.rx).wrapping_mul(256)),
            ),
            sub_y: crate::util::fixed_math::SimFixed::from_num(
                tile.world_y
                    .wrapping_sub(i32::from(tile.ry).wrapping_mul(256)),
            ),
            z: u8::try_from(
                tile.world_z
                    .div_euclid(crate::util::lepton::GROUND_LEVEL_HEIGHT_LEPTONS),
            )
            .unwrap_or(0),
            delay: 0,
            loop_count: -1,
            draw_flags: TILE_ANIM_DRAW_FLAGS,
            z_adjust: 0,
            reverse: false,
            use_cell_drawer: true,
            terrain_attached: true,
            draw_runtime: AnimDrawRuntime::default(),
        };
        let id = sim
            .spawn_anim_at_world(
                rules,
                descriptor,
                AnimWorldCoord {
                    x: tile.world_x,
                    y: tile.world_y,
                    z: tile.world_z,
                },
            )
            .unwrap_or_else(|error| {
                panic!(
                    "resolved terrain animation [{}] must be bound before map spawn: {error}",
                    tile.anim_name
                )
            });
        assert!(
            sim.set_terrain_anim_z_adjust_after_construction(id, tile.z_adjust),
            "terrain animation {id} disappeared before its producer ZAdjust write"
        );
        spawned.push(id);
    }
    spawned
}

/// GPU-free scenario construction shared by app and headless (F09): native
/// order preserved — bootstrap RNG into the session, houses before every
/// object section (ScenarioClass__Create_Houses @ 0x00687F10 precedes
/// TerrainClass__Read_Map_Section @ 0x0071CA70), terrain objects before map
/// entities, then terrain-attached animations. Presentation atlases consume
/// the constructed simulation afterward; nothing here touches the GPU.
#[allow(clippy::too_many_arguments)]
pub(crate) fn construct_scenario<F>(
    map_data: &crate::map::map_file::MapFile,
    resolved_terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    theater_name: &str,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    art: Option<&crate::rules::art_data::ArtRegistry>,
    height_map: &std::collections::BTreeMap<(u16, u16), u8>,
    bridge_destroyability_mode: crate::map::basic::BridgeDestroyabilityMode,
    descriptor: &crate::sim::scenario_session::ScenarioDescriptor,
    bootstrap_rng: crate::sim::scenario_bootstrap::ScenarioBootstrapRng,
    initialize_houses_before_objects: F,
) -> Simulation
where
    F: FnOnce(&mut Simulation),
{
    let mut sim: Simulation = bootstrap_rng.into_simulation(descriptor);
    // Active YR `ScenarioClass__Full_Init @ 0x00686B20` calls
    // `ScenarioClass__Create_Houses @ 0x00687F10` before
    // `TerrainClass__Read_Map_Section @ 0x0071CA70` and every Techno section.
    // Keep the app-specific roster construction outside sim while making that
    // order an explicit prerequisite of the shared object-construction funnel.
    initialize_houses_before_objects(&mut sim);
    // Frame tripwire: every MP start waypoint must sit inside the session
    // bounds (= the fog window, cell-array frame). A start outside means the
    // descriptor was fed wrong-frame bounds (e.g. raw [Map] Size=) and the
    // player's own base would be permanently shrouded.
    for (idx, (rx, ry)) in &descriptor.mp_start_waypoints {
        if *rx >= descriptor.map_width || *ry >= descriptor.map_height {
            log::error!(
                "MP start waypoint {idx} at ({rx},{ry}) lies outside session bounds {}x{} — wrong coordinate frame?",
                descriptor.map_width,
                descriptor.map_height
            );
            debug_assert!(
                false,
                "start waypoint outside session bounds (coordinate-frame mismatch)"
            );
        }
    }
    sim.resolved_terrain = Some(resolved_terrain.clone());
    // Wire the cliff/slope coefficients from [General] into the live World config;
    // it otherwise holds compiled vanilla defaults and never sees a modded INI.
    if let Some(rules) = rules {
        sim.terrain_speed_config =
            crate::sim::pathfinding::terrain_speed::TerrainSpeedConfig::from_general(
                rules.general.tracked_uphill,
                rules.general.tracked_downhill,
                rules.general.wheeled_uphill,
                rules.general.wheeled_downhill,
            );
        sim.radar_events =
            crate::sim::radar::RadarEventQueue::from_config(&rules.radar_event_config);
    }
    // Normal loading first clips/normalizes LocalSize through
    // `MapClass::Set_Clipped_LocalSize @ 0x00567230`; every playfield consumer
    // then shares those stored fields and the same isometric-diamond test.
    sim.install_playfield_from_map_header(&map_data.header);
    let bridge_destroyable = map_data
        .special_flags
        .effective_destroyable_bridges(bridge_destroyability_mode);
    let bridge_strength = rules
        .map(|rules| rules.bridge_rules.strength)
        .unwrap_or(1500);
    sim.bridge_state = Some(
        crate::sim::bridge_state::BridgeRuntimeState::from_resolved_terrain(
            resolved_terrain,
            bridge_destroyable,
            bridge_strength,
        ),
    );
    sim.bridge_explosions = rules
        .map(|r| {
            r.bridge_rules
                .explosions
                .iter()
                .map(|s| sim.interner.intern(s))
                .collect()
        })
        .unwrap_or_default();
    sim.metallic_debris = rules
        .map(|r| {
            r.general
                .metallic_debris
                .iter()
                .map(|s| sim.interner.intern(s))
                .collect()
        })
        .unwrap_or_default();
    if let Some(art) = art {
        let mut bridge_anim_sounds = std::collections::BTreeMap::new();
        for anim_id in sim
            .bridge_explosions
            .iter()
            .chain(sim.metallic_debris.iter())
            .copied()
        {
            let anim_name = sim.interner.resolve(anim_id);
            if let Some(entry) = art.get(anim_name) {
                let sound_name = entry.start_sound.as_ref().or(entry.report.as_ref());
                if let Some(sound_name) = sound_name {
                    let sound_id = sim.interner.intern(sound_name);
                    bridge_anim_sounds.insert(anim_id, sound_id);
                }
            }
        }
        sim.bridge_anim_sounds = bridge_anim_sounds;
    }
    // gamemd `TerrainClass::Read_Map_Section` runs while the map sections are
    // walked, ahead of `[Units]`/`[Aircraft]`/`[Infantry]`/`[Structures]`: every
    // tree owns its cell before the first map object is placed on it. The
    // ore-spawner animation index is attached later, once the terrain SHP frame
    // counts are known.
    if let Some(rules) = rules {
        let constructed = crate::sim::terrain_spawn::construct_terrain_objects(
            &mut sim,
            &map_data.terrain_objects,
            rules,
            theater_name.eq_ignore_ascii_case("SNOW"),
        );
        if constructed > 0 {
            log::info!("Constructed {constructed} map terrain objects before map entities");
        }
    } else {
        log::warn!("No rules loaded — skipping terrain object construction");
    }
    if !map_data.entities.is_empty() {
        let _count: u32 = sim.spawn_from_map_with_resolved(
            &map_data.entities,
            rules,
            height_map,
            Some(resolved_terrain),
        );
        let miner_count: usize = sim
            .entities()
            .values()
            .filter(|e| e.miner.is_some())
            .count();
        log::info!("Miner components attached: {}", miner_count);
    }
    if !resolved_terrain.tile_animations().is_empty() {
        let rules = rules.expect("resolved terrain animations require bound art/rules data");
        let spawned =
            spawn_terrain_tile_animations(&mut sim, rules, resolved_terrain.tile_animations());
        log::info!(
            "Spawned {} terrain-attached animations after map objects",
            spawned.len()
        );
    }
    sim
}

/// BuildingClass::GetCoords projects the stored north-west anchor to the
/// foundation centre before distance consumers receive it.
fn project_building_get_coords_xy(
    northwest_x: i32,
    northwest_y: i32,
    foundation_width: u16,
    foundation_height: u16,
) -> (i32, i32) {
    let x_offset = i32::from(foundation_width)
        .wrapping_sub(1)
        .wrapping_mul(128);
    let y_offset = i32::from(foundation_height)
        .wrapping_sub(1)
        .wrapping_mul(128);
    (
        northwest_x.wrapping_add(x_offset),
        northwest_y.wrapping_add(y_offset),
    )
}

pub(crate) fn map_wall_owner_candidate_from_building(
    entity: &crate::sim::game_entity::GameEntity,
    resolved_terrain: &crate::map::resolved_terrain::ResolvedTerrainGrid,
    house_wall_owner: bool,
) -> crate::sim::overlay_grid::MapWallOwnerCandidate {
    let northwest_x = i32::from(entity.position.rx)
        .wrapping_mul(256)
        .wrapping_add(entity.position.sub_x.to_num::<i32>());
    let northwest_y = i32::from(entity.position.ry)
        .wrapping_mul(256)
        .wrapping_add(entity.position.sub_y.to_num::<i32>());
    let world_z = resolved_terrain
        .cell(entity.position.rx, entity.position.ry)
        .and_then(|cell| {
            crate::util::lepton::ground_height_leptons(
                cell.level,
                cell.slope_type,
                northwest_x,
                northwest_y,
            )
            .ok()
        })
        .unwrap_or(i32::from(entity.position.z) * crate::util::lepton::LEPTONS_PER_LEVEL as i32);
    let (foundation_width, foundation_height) =
        crate::sim::production::foundation_dimensions(&entity.foundation);
    let (world_x, world_y) = project_building_get_coords_xy(
        northwest_x,
        northwest_y,
        foundation_width,
        foundation_height,
    );

    crate::sim::overlay_grid::MapWallOwnerCandidate {
        owner: entity.owner,
        world_x,
        world_y,
        world_z,
        foundation_width,
        foundation_height,
        object_alive: entity.lifecycle.object_alive,
        cell_marked: entity.lifecycle.cell_marked,
        house_wall_owner,
    }
}

/// Post-funnel scenario finalization shared by the app loader and the
/// headless loader (F09). The ordering is part of the construction contract:
/// ore-spawner terrain seeding, map-wall owner reconstruction from the
/// spawned structures, overlay-grid installation, smudge-grid seeding, then
/// the authoritative post-map tail (`ScenarioClass::Post_Map_Init` cone).
pub(crate) fn finalize_constructed_scenario(
    sim: &mut Simulation,
    map_data: &crate::map::map_file::MapFile,
    rules: &RuleSet,
    overlay_registry: &crate::map::overlay_types::OverlayTypeRegistry,
    mut overlay_grid: crate::sim::overlay_grid::OverlayGrid,
    house_roster: &crate::map::houses::HouseRoster,
    skirmish_session: Option<&crate::sim::scenario_bootstrap::MatchLaunchDescriptor>,
) -> crate::sim::scenario_post_map::ScenarioPostMapOutput {
    // Attach the TIBTRE ore-spawner animation index to the terrain objects
    // constructed ahead of the map entities. Its authoritative raw SHP count
    // is rules-owned; presentation atlases retain only body-frame ranges.
    let seeded_terrain =
        crate::sim::terrain_spawn::seed_terrain_spawner_animation(sim, rules, overlay_registry);
    if seeded_terrain > 0 {
        log::info!(
            "Seeded {} ore-spawning terrain objects (TIBTRE)",
            seeded_terrain,
        );
    }
    // Move the already-resolved CellClass overlay state into Simulation,
    // then reconstruct map-wall ownership now that buildings exist.
    if sim.resolved_terrain.is_some() {
        let (grid_width, grid_height) = (overlay_grid.width(), overlay_grid.height());
        let rt = sim
            .resolved_terrain
            .as_ref()
            .expect("terrain checked above");
        let buildings: Vec<crate::sim::overlay_grid::MapWallOwnerCandidate> = sim
            .substrate
            .entities
            .values()
            .filter(|entity| entity.category == crate::map::entities::EntityCategory::Structure)
            .map(|entity| {
                let country = sim
                    .houses
                    .get(&entity.owner)
                    .and_then(|house| house.country)
                    .map(|country| sim.interner.resolve(country));
                map_wall_owner_candidate_from_building(
                    entity,
                    rt,
                    crate::sim::house_state::resolve_wall_owner(Some(rules), country),
                )
            })
            .collect();
        overlay_grid.reconstruct_map_wall_owners(rt, overlay_registry, &buildings);
        sim.overlay_grid = Some(overlay_grid);
        log::info!(
            "Overlay grid initialized: {}x{}, {} entries",
            grid_width,
            grid_height,
            map_data.overlays.len(),
        );
    }
    // Seed smudge grid from map [Smudge] entries. Requires terrain +
    // overlay grids built above so placement gates (slope, overlay,
    // accepts_smudge) can reject invalid map entries at load.
    if let (Some(rt), Some(overlay)) = (sim.resolved_terrain.as_ref(), sim.overlay_grid.as_ref()) {
        let grid_width = rt.width();
        let grid_height = rt.height();
        sim.smudge_grid = Some(crate::sim::smudge_grid::SmudgeGrid::from_map_entries(
            &map_data.smudges,
            &rules.smudge_types,
            rt,
            overlay,
            grid_width,
            grid_height,
        ));
        sim.flush_smudge_dirty();
        log::info!(
            "Smudge grid initialized: {}x{}, {} entries",
            grid_width,
            grid_height,
            map_data.smudges.len(),
        );
    }
    // The caller submits one immutable initialization command; Simulation owns
    // every match-affecting write and Scenario RNG draw in the post-map tail.
    sim.finalize_scenario_post_map(crate::sim::scenario_post_map::ScenarioPostMapInput {
        map_width: map_data.header.width as u16,
        map_height: map_data.header.height as u16,
        basic: &map_data.basic,
        special_flags: &map_data.special_flags,
        rules,
        overlay_registry,
        house_roster,
        skirmish_session,
    })
}
