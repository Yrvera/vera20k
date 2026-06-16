//! In-game update phase — advances fixed-step simulation, triggers, path grids, and atlases.
//!
//! Camera control lives in app_camera.rs. Building animations, damage fires, sidebar
//! UI tick, and sound playback live in app_building_anim.rs.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use std::collections::BTreeMap;
use std::time::Instant;

use crate::app::AppState;
use crate::app_commands::{preferred_local_owner, preferred_local_owner_name};
use crate::app_types::SIM_TICK_HZ;
use crate::app_types::SIM_TICK_MS;
use crate::assets::asset_manager::AssetManager;
use crate::assets::pal_file::Palette;
use crate::audio::events::GameSoundEvent;
use crate::map::entities::EntityCategory;
use crate::map::terrain;
use crate::render::sprite_atlas;
use crate::render::unit_atlas;
use crate::sim::animation::{self, SequenceSet};
use crate::sim::overlay_grid::{OverlayGrid, recalc_overlay_passability};
use crate::sim::pathfinding::PathGrid;
use crate::sim::production;
use crate::sim::replay::{ReplayHeader, ReplayLog};
use crate::sim::trigger_runtime::TriggerEffect;
use crate::sim::world::{SimFireEvent, SimSoundEvent};
use crate::ui::game_screen::GameScreen;

/// Prevent runaway catch-up loops after pauses/debugger stops.
const MAX_SIM_STEPS_PER_FRAME: u32 = 8;
/// Cap catch-up after lag spikes/breakpoints.
const MAX_UPDATE_DELTA_MS: u64 = 250;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FixedStepSchedule {
    steps: u32,
    remaining_accumulator_ms: u64,
}

/// Build animation sequences for entity types in the ECS world.
///
/// For infantry, looks up the `Sequence=` key from art.ini to find the per-type
/// sequence definition (e.g., `[ConSequence]`). Falls back to the hardcoded default
/// layout if no sequence is found. Buildings always use the default single-frame set.
pub(crate) fn build_animation_sequences(
    simulation: Option<&crate::sim::world::Simulation>,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    art_registry: Option<&crate::rules::art_data::ArtRegistry>,
    infantry_sequences: &crate::rules::infantry_sequence::InfantrySequenceRegistry,
) -> BTreeMap<String, SequenceSet> {
    let mut sequences: BTreeMap<String, SequenceSet> = BTreeMap::new();
    let Some(sim) = simulation else {
        return sequences;
    };

    let mut data_driven_count: usize = 0;

    for entity in sim.entities().values() {
        let type_str = sim.interner.resolve(entity.type_ref);
        if sequences.contains_key(type_str) {
            continue;
        }
        // Resolve the art-registry key. Type IDs (e.g. "E1") differ from image
        // IDs (e.g. "GI") — rules.ini's `Image=` is the bridge. Fall back to
        // the type ID when rules can't resolve the image (e.g. preview
        // contexts), since for many types the image defaults to the ID.
        let image_id: String = rules
            .and_then(|r| r.object(type_str))
            .map(|obj| obj.image.clone())
            .unwrap_or_else(|| type_str.to_string());
        let seq: SequenceSet = match entity.category {
            EntityCategory::Infantry => {
                // Look up Sequence= from art.ini for this type's image.
                let seq_name: Option<&str> = art_registry
                    .and_then(|a| a.get(&image_id))
                    .and_then(|e| e.sequence.as_deref());

                if let Some(name) = seq_name {
                    let key: String = name.to_uppercase();
                    if let Some(seq_def) = infantry_sequences.get(&key) {
                        let built: SequenceSet =
                            crate::rules::infantry_sequence::build_sequence_set(seq_def);
                        if !built.is_empty() {
                            data_driven_count += 1;
                            built
                        } else {
                            log::warn!(
                                "Sequence '{}' for type '{}' (image '{}') parsed to 0 entries — using defaults",
                                name,
                                type_str,
                                image_id
                            );
                            animation::default_infantry_sequences()
                        }
                    } else {
                        log::warn!(
                            "Sequence '{}' not found in art.ini for type '{}' (image '{}')",
                            name,
                            type_str,
                            image_id
                        );
                        animation::default_infantry_sequences()
                    }
                } else {
                    log::warn!(
                        "No Sequence= in art.ini for infantry type '{}' (image '{}') — falling back to defaults",
                        type_str,
                        image_id
                    );
                    animation::default_infantry_sequences()
                }
            }
            EntityCategory::Structure => animation::default_building_sequences(),
            // SHP vehicles (Voxel=no): build sequences from WalkFrames/FiringFrames tags.
            EntityCategory::Unit | EntityCategory::Aircraft if !entity.is_voxel => {
                let art_entry = art_registry.and_then(|a| a.get(&image_id));
                if let Some(art) = art_entry {
                    if art.walk_frames.is_some() || art.firing_frames.is_some() {
                        data_driven_count += 1;
                        crate::rules::shp_vehicle_sequence::build_shp_vehicle_sequences(art)
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        sequences.insert(type_str.to_string(), seq);
    }

    log::info!(
        "Built animation sequences for {} entity types ({} data-driven from art.ini)",
        sequences.len(),
        data_driven_count
    );
    sequences
}

pub(crate) fn update_elapsed_ms(state: &mut AppState, now: Instant) -> u64 {
    let elapsed_ms: u64 = now.duration_since(state.last_update_time).as_millis() as u64;
    state.last_update_time = now;
    elapsed_ms
}

/// Front-end session mode, as the modal pump reads it to decide whether the
/// simulation advances behind an open modal dialog. Mirrors gamemd's `g_GameMode`
/// discriminator; the values are writer-proofed — the active engine only ever
/// writes 0/3/4/5, so any other raw value is a legacy (modem/serial) or
/// uninitialized mode the pump treats conservatively as non-advancing. This type
/// is read ONLY by the app loop, never by `sim/` (the layering rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMode {
    /// Campaign / single-player. The modal pump freezes the world.
    Campaign,
    /// LAN / IPX network. The modal pump keeps the world advancing.
    Lan,
    /// WOL / Internet network. The modal pump keeps the world advancing.
    Wol,
    /// Offline skirmish. The modal pump freezes the world.
    Skirmish,
    /// Any raw value the active engine never writes (legacy modem/serial, or
    /// uninitialized). Treated as non-advancing.
    Other,
}

impl SessionMode {
    /// Map gamemd's raw `g_GameMode` int to a session mode. Writer-proofed:
    /// 0=Campaign, 3=Lan, 4=Wol, 5=Skirmish; every other value is `Other`.
    pub fn from_game_mode(raw: i32) -> Self {
        match raw {
            0 => SessionMode::Campaign,
            3 => SessionMode::Lan,
            4 => SessionMode::Wol,
            5 => SessionMode::Skirmish,
            _ => SessionMode::Other,
        }
    }

    /// Whether this is a network session (LAN/WOL). Only network sessions keep the
    /// simulation advancing behind an open modal; offline modes freeze it.
    pub fn is_network(self) -> bool {
        matches!(self, SessionMode::Lan | SessionMode::Wol)
    }
}

/// Pure modal-pump decision: should the fixed simulation advance one step while a
/// modal dialog is open? Mirrors gamemd's pump body — advance only on the network
/// branch (LAN/WOL), and only when no fixed tick is already in progress (the native
/// reentrancy guard). Offline campaign/skirmish freeze the world; message, input,
/// and repaint still run in the surrounding loop. Pure and total, so it is
/// unit-tested without an `AppState`. The live app-layer consumer is
/// `service_tick_should_advance_sim`, which reads the running session mode and
/// gates `advance_fixed_simulation` inside `advance_in_game_runtime`.
pub fn modal_pump_should_advance_sim(mode: SessionMode, reentrancy_in_progress: bool) -> bool {
    mode.is_network() && !reentrancy_in_progress
}

/// Live front-end session mode for the running client. This build is offline
/// only, and offline campaign and skirmish freeze the world identically behind a
/// modal, so it reports `Skirmish`. When networking lands, this reads the live
/// game-mode discriminator and maps it via `SessionMode::from_game_mode`.
fn current_session_mode(_state: &AppState) -> SessionMode {
    SessionMode::Skirmish
}

/// App-layer modal-pump service decision: should the fixed simulation advance
/// this frame? While the in-game Options modal is open (`state.paused` is the
/// 0xBBB modal in this port), the verified pump contract decides — offline
/// campaign/skirmish freeze, network LAN/WOL advance. With no modal open the
/// world always runs. Re-entrancy is always clear here: the single-threaded
/// frame loop never re-enters a fixed tick mid-advance.
fn service_tick_should_advance_sim(state: &AppState) -> bool {
    if state.paused {
        modal_pump_should_advance_sim(current_session_mode(state), false)
    } else {
        true
    }
}

pub(crate) fn advance_in_game_runtime(state: &mut AppState, elapsed_ms: u64) {
    // Frame-step: when paused, advance exactly one tick on request.
    let frame_stepping = state.debug_frame_step_requested;
    let run_sim = if frame_stepping {
        state.debug_frame_step_requested = false;
        true
    } else {
        // Modal-pump contract: while the in-game Options modal is open
        // (`state.paused`), offline freezes and network advances; otherwise run.
        // Offline-identical to the prior `!state.paused`.
        service_tick_should_advance_sim(state)
    };

    // When frame-stepping, inject exactly one tick instead of using wall-clock elapsed time.
    let sim_elapsed = if frame_stepping {
        SIM_TICK_MS as u64
    } else {
        elapsed_ms
    };

    if run_sim {
        let garrison_flash_start_tick = state.simulation.as_ref().map(|sim| sim.session.tick).unwrap_or(0);
        advance_fixed_simulation(state, sim_elapsed);
        let garrison_flash_elapsed_ticks = state
            .simulation
            .as_ref()
            .map(|sim| sim.session.tick.saturating_sub(garrison_flash_start_tick))
            .unwrap_or(0);
        crate::app_building_anim::drain_sound_events(state);
        // Drain bale events into building anim overlays + particle bursts before
        // the per-frame anim tick so the SpecialAnim is visible this same frame.
        crate::app_building_anim::consume_bale_events(state);
        // Drain tank-bunker wall-anim events into SpecialAnim overlays the same
        // frame so the walls rise/fall in step with the install/teardown.
        crate::app_building_anim::consume_bunker_wall_events(state);
        // Use real wall-clock delta (capped to prevent jumps after pauses/debugger).
        // Previously this passed SIM_TICK_MS (66ms) per render frame, causing building
        // idle animations to play ~3-4× too fast (60fps × 66ms = 3960ms/sec).
        crate::app_building_anim::tick_crane_animations(
            state,
            sim_elapsed.min(MAX_UPDATE_DELTA_MS) as u32,
        );
        crate::app_building_anim::tick_damage_fire_overlays(
            state,
            sim_elapsed.min(MAX_UPDATE_DELTA_MS) as u32,
        );
        crate::app_building_anim::tick_garrison_muzzle_flashes(
            state,
            garrison_flash_elapsed_ticks.saturating_mul(u64::from(SIM_TICK_MS)) as u32,
        );
        finish_fire_effect_batch(&mut state.pending_fire_effects);
        crate::app_fire_effects::tick_weapon_muzzle_flashes(
            state,
            sim_elapsed.min(MAX_UPDATE_DELTA_MS) as u32,
        );
        crate::app_chute_anim::tick_parachute_anims(
            state,
            sim_elapsed.min(MAX_UPDATE_DELTA_MS) as u32,
        );
    }

    crate::app_building_anim::update_radar_state(state, SIM_TICK_MS as f32);
    crate::app_building_anim::update_power_bar_anim(state);
    crate::app_sidebar_gadgets::update_sidebar_gadget_state(state);
    // Per-frame gadget idle tick (G22 rows 2/3 drag-off/drag-back tracking).
    crate::app_gadget_input::idle_tick(state);
    if let (Some(player), Some(assets)) = (&mut state.music_player, &state.asset_manager) {
        player.update(assets);
    }
    crate::app_camera::update_camera(state);
    update_building_placement_preview(state);
    let sw = state.render_width() as f32;
    let sh = state.render_height() as f32;
    state.batch_renderer.update_camera(
        &state.gpu,
        sw,
        sh,
        state.camera_x,
        state.camera_y,
        state.zoom_level,
    );
}

/// Tick simulation: advance movement and animation systems.
pub(crate) fn advance_fixed_simulation(state: &mut AppState, elapsed_ms: u64) {
    // Scale elapsed time by speed ratio to change effective ticks-per-second.
    // Per-tick dt stays constant — speed change comes from more/fewer ticks per wall-clock second.
    let scaled_elapsed = elapsed_ms * state.sim_speed_tps as u64 / SIM_TICK_HZ as u64;
    // Allow more steps per frame at high speeds so the sim can keep up.
    let max_steps = ((state.sim_speed_tps as u64 * MAX_SIM_STEPS_PER_FRAME as u64
        / SIM_TICK_HZ as u64) as u32)
        .max(MAX_SIM_STEPS_PER_FRAME)
        .min(64);
    let schedule = schedule_fixed_steps(state.sim_accumulator_ms, scaled_elapsed, max_steps);
    state.sim_accumulator_ms = schedule.remaining_accumulator_ms;

    let mut refresh_after_tick = false;
    let mut crane_owners: Vec<String> = Vec::new();
    // (rx, ry, type_id) for wall buildings placed this frame — injected into state.overlays.
    let mut placed_walls: Vec<(u16, u16, String)> = Vec::new();
    let runtime_active = state.simulation.is_some() || !state.trigger_graph.triggers.is_empty();
    if !runtime_active {
        return;
    }

    if let Some(sim) = &mut state.simulation {
        if sim.replay_log.is_none() {
            sim.replay_log = Some(ReplayLog::new(ReplayHeader {
                version: 1,
                tick_hz: SIM_TICK_HZ,
                seed: sim.session.seed,
                // Scenario identity is session state — the header derives
                // from the sim, not from app-resident view fields.
                map_name: sim.session.map_name.clone(),
                rules_hash: state.rules.as_ref().map(rules_hash).unwrap_or(0),
            }));
        }
    }

    begin_fire_effect_batch(&mut state.pending_fire_effects);

    for _ in 0..schedule.steps {
        // Compute local owner before mutable borrow of simulation.
        let local_owner_for_fog = preferred_local_owner_name(state);

        // Cache local owner name before mutable sim borrow (avoids borrow conflict).
        let local_owner_name = crate::app_commands::preferred_local_owner_name(state);
        let mut drained_fire_events: Vec<SimFireEvent> = Vec::new();
        if let Some(sim) = &mut state.simulation {
            // Clear AI players when disabled — prevents computer houses from acting.
            if state.disable_ai && !sim.ai_players.is_empty() {
                log::info!("AI disabled — clearing {} AI players", sim.ai_players.len());
                sim.ai_players.clear();
            }
            sim.sound_events.clear();
            let due_commands = sim.take_due_commands();
            let tick_result = sim.advance_tick(
                &due_commands,
                state.rules.as_ref(),
                &state.height_map,
                state.path_grid.as_ref(),
                state.overlay_registry.as_ref(),
                SIM_TICK_MS,
            );
            let (ents, interner) = sim.entities_mut_and_interner();
            let death_finished = animation::tick_animations(
                ents,
                &state.animation_sequences,
                SIM_TICK_MS,
                interner,
            );
            // Despawn entities whose death animation has completed.
            for dead_id in &death_finished {
                // Remove from occupancy before despawning.
                if let Some(entity) = sim.entities().get(*dead_id) {
                    let rx = entity.position.rx;
                    let ry = entity.position.ry;
                    sim.occupancy_mut().remove(rx, ry, *dead_id);
                }
                sim.uninit(*dead_id);
            }
            if !death_finished.is_empty() {
                // Anim-end corpses were uninit'd above (enqueued). Drain now so they
                // free at exactly this frame — the deferred queue must not carry an
                // animated death into the next tick.
                sim.flush_pending_delete();
                refresh_after_tick = true;
            }
            animation::tick_voxel_animations(sim.entities_mut(), SIM_TICK_MS);
            animation::tick_harvest_overlays(sim.entities_mut(), SIM_TICK_MS);
            // Pre-merge fog visibility for local owner so render queries are O(1).
            if let Some(owner) = &local_owner_for_fog {
                if sim.session.tick == 1 {
                    log::info!("Fog merged for local owner: '{}'", owner);
                }
                if let Some(owner_id) = sim.interner.get(owner) {
                    sim.fog.build_merged_for(owner_id, &sim.interner);
                }
            }
            // Drain fire events for render-side muzzle flash / projectile origin.
            drained_fire_events.extend(sim.fire_events.drain(..));
            append_fire_effect_batch(&mut state.pending_fire_effects, &drained_fire_events);
            // Convert sim sound events to app-layer sound events for playback.
            for sim_event in sim.sound_events.drain(..) {
                let app_event: GameSoundEvent = match sim_event {
                    SimSoundEvent::WeaponFired {
                        report_sound_id,
                        rx,
                        ry,
                    } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::WeaponFired {
                            sound_id: sim.interner.resolve(report_sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::EntityDied {
                        die_sound_id,
                        rx,
                        ry,
                    } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::EntityDestroyed {
                            sound_id: sim.interner.resolve(die_sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::EntityCrushed {
                        crush_sound_id,
                        rx,
                        ry,
                    } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::EntityCrushed {
                            sound_id: sim.interner.resolve(crush_sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::EntityDeployed {
                        deploy_sound_id,
                        rx,
                        ry,
                    } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::EntityDeployed {
                            sound_id: sim.interner.resolve(deploy_sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::EntityUndeployed {
                        undeploy_sound_id,
                        rx,
                        ry,
                    } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::EntityUndeployed {
                            sound_id: sim.interner.resolve(undeploy_sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::DockDeploy { .. } => {
                        // TODO: resolve building's deploy sound from art.ini
                        // and select healthy/damaged variant based on health ratio.
                        continue;
                    }
                    SimSoundEvent::ChronoTeleport { sound_id, rx, ry } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::ChronoTeleport {
                            sound_id: sim.interner.resolve(sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::BuildingComplete { owner } => {
                        // Only play EVA for the local player's production.
                        let owner_str = sim.interner.resolve(owner);
                        if !local_owner_name
                            .as_deref()
                            .map_or(false, |l| l.eq_ignore_ascii_case(owner_str))
                        {
                            continue;
                        }
                        let faction = crate::app_building_anim::eva_faction_key(
                            owner_str,
                            &state.house_roster,
                        );
                        let sound_id = state
                            .eva_registry
                            .get("EVA_ConstructionComplete", faction)
                            .unwrap_or("ceva048")
                            .to_string();
                        GameSoundEvent::BuildingReady { sound_id }
                    }
                    SimSoundEvent::SuperWeaponLaunched { .. } => {
                        // TODO: play EVA superweapon warning sound.
                        continue;
                    }
                    SimSoundEvent::SuperWeaponStrike { .. } => {
                        // TODO: play lightning bolt thunder sound.
                        continue;
                    }
                    SimSoundEvent::UnitComplete { owner } => {
                        let owner_str = sim.interner.resolve(owner);
                        if !local_owner_name
                            .as_deref()
                            .map_or(false, |l| l.eq_ignore_ascii_case(owner_str))
                        {
                            continue;
                        }
                        let faction = crate::app_building_anim::eva_faction_key(
                            owner_str,
                            &state.house_roster,
                        );
                        let sound_id = state
                            .eva_registry
                            .get("EVA_UnitReady", faction)
                            .unwrap_or("ceva062")
                            .to_string();
                        GameSoundEvent::UnitReady { sound_id }
                    }
                    SimSoundEvent::CannotDeployHere { owner } => {
                        let owner_str = sim.interner.resolve(owner);
                        if !local_owner_name
                            .as_deref()
                            .map_or(false, |l| l.eq_ignore_ascii_case(owner_str))
                        {
                            continue;
                        }
                        let faction = crate::app_building_anim::eva_faction_key(
                            owner_str,
                            &state.house_roster,
                        );
                        let sound_id = state
                            .eva_registry
                            .get("EVA_CannotDeployHere", faction)
                            .unwrap_or("ceva063")
                            .to_string();
                        GameSoundEvent::CannotDeployHere { sound_id }
                    }
                    SimSoundEvent::StructureGarrisoned { owner } => {
                        // EVA cue: only play for the local human player.
                        let owner_str = sim.interner.resolve(owner);
                        if !local_owner_name
                            .as_deref()
                            .map_or(false, |l| l.eq_ignore_ascii_case(owner_str))
                        {
                            continue;
                        }
                        let faction = crate::app_building_anim::eva_faction_key(
                            owner_str,
                            &state.house_roster,
                        );
                        let sound_id = state
                            .eva_registry
                            .get("EVA_StructureGarrisoned", faction)
                            .unwrap_or("ceva107")
                            .to_string();
                        GameSoundEvent::StructureGarrisoned { sound_id }
                    }
                    SimSoundEvent::StructureAbandoned { owner } => {
                        let owner_str = sim.interner.resolve(owner);
                        if !local_owner_name
                            .as_deref()
                            .map_or(false, |l| l.eq_ignore_ascii_case(owner_str))
                        {
                            continue;
                        }
                        let faction = crate::app_building_anim::eva_faction_key(
                            owner_str,
                            &state.house_roster,
                        );
                        let sound_id = state
                            .eva_registry
                            .get("EVA_StructureAbandoned", faction)
                            .unwrap_or("ceva108")
                            .to_string();
                        GameSoundEvent::StructureAbandoned { sound_id }
                    }
                    SimSoundEvent::BuildingGarrisonedSfx { owner, rx, ry } => {
                        // Positional SFX: only audible to the local human player
                        // (matches gamemd VocClass::PlayAt with IsHumanPlayer gate).
                        let owner_str = sim.interner.resolve(owner);
                        if !local_owner_name
                            .as_deref()
                            .map_or(false, |l| l.eq_ignore_ascii_case(owner_str))
                        {
                            continue;
                        }
                        let sound_id = match state
                            .rules
                            .as_ref()
                            .and_then(|r| r.general.building_garrisoned_sound.as_deref())
                        {
                            Some(s) if !s.is_empty() => s.to_string(),
                            _ => continue,
                        };
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::BuildingGarrisonedSfx {
                            sound_id,
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::ChuteSound { rx, ry } => {
                        let sound_id = match state
                            .rules
                            .as_ref()
                            .and_then(|r| r.general.chute_sound.as_deref())
                        {
                            Some(s) if !s.is_empty() => s.to_string(),
                            _ => continue,
                        };
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::ChuteSound {
                            sound_id,
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::C4Planted { rx, ry } => {
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::C4Planted {
                            sound_id: "SealPlaceBomb".to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::RefineryExitSfx { rx, ry } => {
                        // Positional SFX from [AudioVisual] BunkerWallsDownSound.
                        // Skip when rules don't configure the sound (matches
                        // gamemd's `RulesClass+0x244 != -1` guard).
                        let sound_id = match state
                            .rules
                            .as_ref()
                            .and_then(|r| r.general.bunker_walls_down_sound.as_deref())
                        {
                            Some(s) if !s.is_empty() => s.to_string(),
                            _ => continue,
                        };
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::RefineryExitSfx {
                            sound_id,
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::BunkerWallsUp { rx, ry } => {
                        // Walls-up cue on install; skip when the rules key is empty.
                        let sound_id = match state
                            .rules
                            .as_ref()
                            .and_then(|r| r.general.bunker_walls_up_sound.as_deref())
                        {
                            Some(s) if !s.is_empty() => s.to_string(),
                            _ => continue,
                        };
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::BunkerWalls {
                            sound_id,
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::BunkerWallsDown { rx, ry } => {
                        // Walls-down cue on normal exit / clear teardown.
                        let sound_id = match state
                            .rules
                            .as_ref()
                            .and_then(|r| r.general.bunker_walls_down_sound.as_deref())
                        {
                            Some(s) if !s.is_empty() => s.to_string(),
                            _ => continue,
                        };
                        let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                        GameSoundEvent::BunkerWalls {
                            sound_id,
                            screen_pos: Some((sx, sy)),
                        }
                    }
                    SimSoundEvent::BridgeRepaired {
                        rx,
                        ry,
                        owner,
                        eva_allowed,
                    } => {
                        // Spatial SFX gated on rules.bridge_rules.repair_sound
                        // being set (the original game gates on
                        // `RulesClass+0x248 != -1`).
                        let sound_id = state
                            .rules
                            .as_ref()
                            .and_then(|r| r.bridge_rules.repair_sound.clone())
                            .unwrap_or_default();
                        let screen_pos = if sound_id.is_empty() {
                            None
                        } else {
                            let (sx, sy) = crate::map::terrain::iso_to_screen(rx, ry, 0);
                            Some((sx, sy))
                        };
                        // EVA cue gated on local-human owner. Resolves
                        // `EVA_BridgeRepaired` from the registry (no faction
                        // fallback needed — bridge repair is faction-agnostic).
                        let owner_str = sim.interner.resolve(owner);
                        let eva_sound_id = if eva_allowed
                            && local_owner_name
                                .as_deref()
                                .is_some_and(|l| l.eq_ignore_ascii_case(owner_str))
                        {
                            let faction = crate::app_building_anim::eva_faction_key(
                                owner_str,
                                &state.house_roster,
                            );
                            state
                                .eva_registry
                                .get("EVA_BridgeRepaired", faction)
                                .map(|s| s.to_string())
                        } else {
                            None
                        };
                        if sound_id.is_empty() && eva_sound_id.is_none() {
                            continue;
                        }
                        GameSoundEvent::BridgeRepaired {
                            sound_id,
                            screen_pos,
                            eva_sound_id,
                        }
                    }
                    SimSoundEvent::WorldEffectStarted {
                        sound_id,
                        rx,
                        ry,
                        sub_x,
                        sub_y,
                        z,
                    } => {
                        let (sx, sy) =
                            crate::util::lepton::lepton_to_screen(rx, ry, sub_x, sub_y, z);
                        GameSoundEvent::WorldEffectStarted {
                            sound_id: sim.interner.resolve(sound_id).to_string(),
                            screen_pos: Some((sx, sy)),
                        }
                    }
                };
                state.sound_events.push(app_event);
            }
            if tick_result.destroyed_structure {
                refresh_after_tick = true;
            }
            if tick_result.bridge_state_changed {
                refresh_after_tick = true;
            }
            if tick_result.ownership_changed {
                refresh_after_tick = true;
            }
            if tick_result.spawned_entities {
                refresh_after_tick = true;
                log::debug!(
                    "spawned_entities=true, checking {} due_commands for PlaceReadyBuilding",
                    due_commands.len()
                );
                for cmd in &due_commands {
                    if let crate::sim::command::Command::PlaceReadyBuilding {
                        owner,
                        type_id,
                        rx,
                        ry,
                    } = &cmd.payload
                    {
                        // Trigger one-shot crane animation on ConYard for each owner that placed a building.
                        let owner_str = sim.interner.resolve(*owner).to_string();
                        let type_str = sim.interner.resolve(*type_id).to_string();
                        crane_owners.push(owner_str);
                        // Walls are overlays — inject OverlayEntry so the overlay renderer
                        // draws them with auto-tiled connectivity frames.
                        let is_wall = state
                            .rules
                            .as_ref()
                            .and_then(|r| r.object(&type_str))
                            .map(|o| o.wall)
                            .unwrap_or(false);
                        if is_wall {
                            placed_walls.push((*rx, *ry, type_str));
                        }
                    }
                }
            }
            if let Some(log) = &mut sim.replay_log {
                log.record_tick(tick_result.tick, due_commands, tick_result.state_hash);
            }
        }
        crate::app_fire_effects::spawn_non_garrison_fire_effects(state, &drained_fire_events);

        let trigger_effects = if let Some(sim) = &mut state.simulation {
            sim.advance_triggers(
                &state.trigger_graph,
                &state.triggers,
                &state.events,
                &state.actions,
            )
        } else {
            Vec::new()
        };
        apply_trigger_effects(state, &trigger_effects);

        // Drain overlay dirty cells and recompute passability. If any cell's
        // passability flipped, trigger zone rebuild via `refresh_after_tick`.
        // Uses the existing `rebuild_dynamic_path_grid` → `rebuild_zone_grid`
        // path; no new zone-rebuild plumbing.
        //
        // Also collects info for any cells that gained an overlay this tick
        // (e.g. TIBTRE-spawned ore, ore_growth-spread ore). The renderer iterates
        // `state.overlays` (the static map list), so without this sync new cells
        // are invisible even though their sim state and OverlayGrid entries are
        // correct. We dedupe against `state.overlays` after the sim borrow drops.
        let new_render_overlays: Vec<crate::map::overlay::OverlayEntry> = {
            let mut collected: Vec<crate::map::overlay::OverlayEntry> = Vec::new();
            if let (Some(sim), Some(registry)) =
                (state.simulation.as_mut(), state.overlay_registry.as_ref())
            {
                if let (Some(overlay_grid), Some(terrain)) =
                    (sim.overlay_grid.as_mut(), sim.resolved_terrain.as_mut())
                {
                    let dirty = overlay_grid.take_dirty_cells();
                    if !dirty.is_empty() {
                        let overlay_ref: &OverlayGrid = overlay_grid;
                        let mut passability_changed = false;
                        for &(rx, ry) in &dirty {
                            if recalc_overlay_passability(overlay_ref, terrain, registry, rx, ry) {
                                passability_changed = true;
                            }
                        }
                        if passability_changed {
                            refresh_after_tick = true;
                        }
                        for &(rx, ry) in &dirty {
                            let cell = overlay_ref.cell(rx, ry);
                            if let Some(overlay_id) = cell.overlay_id {
                                collected.push(crate::map::overlay::OverlayEntry {
                                    rx,
                                    ry,
                                    overlay_id,
                                    frame: cell.overlay_data,
                                });
                            }
                        }
                    }
                }
            }
            collected
        };
        if !new_render_overlays.is_empty() {
            sync_new_overlay_cells_to_render_list(state, new_render_overlays);
        }
    }

    // Trigger one-shot crane animations for owners that placed buildings this frame.
    if !crane_owners.is_empty() {
        log::info!(
            "Triggering crane anims for {} owners: {:?}",
            crane_owners.len(),
            crane_owners
        );
    }
    for owner in &crane_owners {
        crate::app_building_anim::trigger_crane_anim(state, owner);
    }

    // Inject overlay entries for walls placed this frame, then recompute connectivity.
    if !placed_walls.is_empty() {
        inject_placed_wall_overlays(state, &placed_walls);
    }

    if refresh_after_tick {
        rebuild_dynamic_path_grid(state);
        refresh_entity_atlases(state);
    }
}

fn schedule_fixed_steps(accumulator_ms: u64, elapsed_ms: u64, max_steps: u32) -> FixedStepSchedule {
    // Scale the delta cap proportionally to the max steps allowed, so high-speed
    // modes don't get clamped to the base 250ms cap.
    let scaled_delta_cap = MAX_UPDATE_DELTA_MS * max_steps as u64 / MAX_SIM_STEPS_PER_FRAME as u64;
    let mut remaining_accumulator_ms =
        accumulator_ms.saturating_add(elapsed_ms.min(scaled_delta_cap));
    let mut steps = 0;

    while remaining_accumulator_ms >= SIM_TICK_MS as u64 && steps < max_steps {
        remaining_accumulator_ms -= SIM_TICK_MS as u64;
        steps += 1;
    }

    if steps == max_steps && remaining_accumulator_ms >= SIM_TICK_MS as u64 {
        remaining_accumulator_ms = SIM_TICK_MS as u64;
    }

    FixedStepSchedule {
        steps,
        remaining_accumulator_ms,
    }
}

fn begin_fire_effect_batch(pending: &mut Vec<SimFireEvent>) {
    pending.clear();
}

fn append_fire_effect_batch(pending: &mut Vec<SimFireEvent>, events: &[SimFireEvent]) {
    pending.extend(events.iter().cloned());
}

fn finish_fire_effect_batch(pending: &mut Vec<SimFireEvent>) {
    pending.clear();
}

fn apply_trigger_effects(state: &mut AppState, effects: &[TriggerEffect]) {
    for effect in effects {
        match effect {
            TriggerEffect::CenterCameraAtWaypoint {
                waypoint,
                immediate: _,
            } => center_camera_on_waypoint(state, *waypoint),
            TriggerEffect::MissionAnnouncement { text } => {
                // gamemd routes trigger text through the message list
                // (contract lane §4.5: the native trigger-text path is a
                // message-list producer).
                crate::app_messages::post_system_message(state, text);
            }
            TriggerEffect::MissionResult { title, detail } => {
                state.screen = GameScreen::MissionResult {
                    title: title.clone(),
                    detail: detail.clone(),
                };
            }
        }
    }
}

fn center_camera_on_waypoint(state: &mut AppState, waypoint_index: u32) {
    let Some(waypoint) = state.waypoints.get(&waypoint_index) else {
        log::warn!(
            "Trigger action requested missing waypoint {} for camera centering",
            waypoint_index
        );
        return;
    };
    let wp_z = state
        .height_map
        .get(&(waypoint.rx, waypoint.ry))
        .copied()
        .unwrap_or(0);
    let (sx, sy) = terrain::iso_to_screen(waypoint.rx, waypoint.ry, wp_z);
    let sw = state.render_width() as f32;
    let sh = state.render_height() as f32;
    let z = state.zoom_level;
    state.camera_x = sx - sw / (2.0 * z);
    state.camera_y = sy - sh / (2.0 * z);
}

pub(crate) fn rebuild_dynamic_path_grid(state: &mut AppState) {
    // Build fresh from terrain + current bridge_state every time. Bridge
    // runtime walkability mutates during gameplay (collapse/repair), so a
    // cached "terrain-only" base would silently go stale.
    let Some(rules) = state.rules.as_ref() else {
        return;
    };
    let Some(ref sim) = state.simulation else {
        return;
    };
    let Some(terrain) = sim.resolved_terrain.as_ref() else {
        return;
    };

    let mut grid: PathGrid =
        PathGrid::from_resolved_terrain_with_bridges(terrain, sim.bridge_state.as_ref());

    let mut structures: Vec<(u16, u16, String)> = sim
        .entities()
        .values()
        .filter_map(|entity| {
            (entity.category == EntityCategory::Structure).then_some((
                entity.position.rx,
                entity.position.ry,
                sim.interner.resolve(entity.type_ref).to_string(),
            ))
        })
        .collect();
    structures.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });

    for (rx, ry, type_id) in &structures {
        let obj = rules.object(type_id);
        let foundation = obj.map(|o| o.foundation.as_str()).unwrap_or("1x1");
        let has_bib: bool = obj.map(|o| o.bib).unwrap_or(false);
        grid.block_building_movement_cells(*rx, *ry, foundation, has_bib);
    }

    // Block wall overlay cells (auto-filled walls have no entity but still block movement).
    if let Some(registry) = &state.overlay_registry {
        for entry in &state.overlays {
            let is_wall = registry
                .flags(entry.overlay_id)
                .map(|f| f.wall)
                .unwrap_or(false);
            if is_wall {
                grid.block_building_movement_cells(entry.rx, entry.ry, "1x1", false);
            }
        }
    }

    state.path_grid = Some(grid);

    // Rebuild zone connectivity map for instant unreachability detection.
    // The unified PathGrid already contains building/wall/bridge data from
    // resolved terrain, so no separate sync step is needed.
    if let Some(ref mut sim) = state.simulation {
        if let Some(ref grid) = state.path_grid {
            sim.rebuild_zone_grid(grid);
        }
    }
}

pub(crate) fn update_building_placement_preview(state: &mut AppState) {
    let Some(type_id) = state.armed_building_type() else {
        state.building_placement_preview = None;
        return;
    };
    let owner: String = preferred_local_owner(state).unwrap_or_else(|| "Americans".to_string());
    let (Some(sim), Some(rules)) = (&state.simulation, &state.rules) else {
        state.building_placement_preview = None;
        return;
    };
    // Offset so the foundation shadow centers on the cursor, not top-left corner.
    let (fw, fh, foundation_str) = rules
        .object(type_id)
        .map(|obj| {
            let (w, h) = production::foundation_dimensions(&obj.foundation);
            (w, h, obj.foundation.clone())
        })
        .unwrap_or((1, 1, "1x1".to_string()));
    // Log at info level once per type_id change so it's visible in console.
    static LAST_LOG: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let hash: u64 = type_id
        .as_bytes()
        .iter()
        .fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64));
    if LAST_LOG.swap(hash, std::sync::atomic::Ordering::Relaxed) != hash {
        log::info!(
            "Placement preview: type={} foundation=\"{}\" → {}x{}",
            type_id,
            foundation_str,
            fw,
            fh,
        );
    }
    // Place the foundation with cursor cell as the top-left corner.
    // The building sprite is anchored to iso_to_screen(rx, ry) — same as the first
    // diamond cell — so the preview and the placed building always align.
    let (rx, ry) = screen_point_to_world_cell(state, state.cursor_x, state.cursor_y);
    state.building_placement_preview = production::placement_preview_for_owner(
        sim,
        rules,
        &owner,
        type_id,
        rx,
        ry,
        state.path_grid.as_ref(),
        &state.height_map,
    );
}

/// Refresh entity atlases after new entities are spawned.
///
/// Uses an incremental approach: first checks if the existing atlases already
/// contain all needed sprite keys. If so, skips the expensive rebuild entirely.
/// Only performs a full rebuild when genuinely new sprite types appear.
/// Reuses `state.asset_manager` instead of creating a new one (avoids re-opening
/// all MIX archives from disk).
pub(crate) fn refresh_entity_atlases(state: &mut AppState) {
    state.animation_sequences = build_animation_sequences(
        state.simulation.as_ref(),
        state.rules.as_ref(),
        state.art_registry.as_ref(),
        &state.infantry_sequences,
    );
    let Some(sim) = &state.simulation else { return };
    let Some(asset_manager) = &state.asset_manager else {
        log::warn!("Atlas refresh skipped: no asset manager available");
        return;
    };

    // Check if unit atlas needs rebuilding (new voxel entity types appeared).
    let unit_needed = unit_atlas::collect_needed_unit_keys(
        sim.entities(),
        asset_manager,
        state.rules.as_ref(),
        state.art_registry.as_ref(),
        Some(&sim.interner),
    );
    let unit_rebuild: bool = match &state.unit_atlas {
        Some(atlas) => !atlas.has_all_keys(&unit_needed),
        None => !unit_needed.is_empty(),
    };

    // Check if sprite atlas needs rebuilding (new SHP entity types appeared).
    let extra_buildings: Vec<&str> = crate::app_skirmish::deployable_building_types(
        sim.entities(),
        state.rules.as_ref(),
        Some(&sim.interner),
    );
    let sprite_base_keys = sprite_atlas::collect_needed_base_keys(
        sim.entities(),
        &state.house_color_map,
        &extra_buildings,
        Some(&sim.interner),
    );
    let sprite_rebuild: bool = match &state.sprite_atlas {
        Some(atlas) => !sprite_atlas::atlas_covers_base_keys(atlas, &sprite_base_keys),
        None => !sprite_base_keys.is_empty(),
    };

    // Early out: no new sprite types → skip the expensive atlas rebuild.
    if !unit_rebuild && !sprite_rebuild {
        log::debug!("Atlas refresh: no new sprite types — skipping rebuild");
        return;
    }

    let unit_palette = load_unit_palette(asset_manager, &state.theater_ext);
    let Some(palette) = unit_palette else {
        log::warn!("Atlas refresh skipped: unit palette not found");
        return;
    };

    if unit_rebuild {
        log::info!("Rebuilding unit atlas: new voxel entity types detected");
        let existing = state.unit_atlas.take();
        if let Some(new_unit_atlas) = unit_atlas::build_unit_atlas(
            &state.gpu,
            &state.batch_renderer,
            sim.entities(),
            asset_manager,
            state.rules.as_ref(),
            state.art_registry.as_ref(),
            existing,
            state.vxl_compute.as_mut(),
            Some(&sim.interner),
        ) {
            state.unit_atlas = Some(new_unit_atlas);
        }
    }

    if sprite_rebuild {
        log::warn!(">>> SPRITE ATLAS REBUILD TRIGGERED — new SHP entity types detected <<<");
        let existing = state.sprite_atlas.take();
        if let Some(new_sprite_atlas) = sprite_atlas::build_sprite_atlas(
            &state.gpu,
            &state.batch_renderer,
            sim.entities(),
            asset_manager,
            &palette,
            &state.theater_ext,
            &state.theater_name,
            state.rules.as_ref(),
            state.art_registry.as_ref(),
            &state.house_color_map,
            &extra_buildings,
            &state.infantry_sequences,
            existing,
            Some(&sim.interner),
        ) {
            state.sprite_atlas = Some(new_sprite_atlas);
        }
    }
}

/// Inject newly-spawned ore cells (TIBTRE, ore-spread) into `state.overlays`.
///
/// Background: the overlay renderer iterates `state.overlays`, the static list
/// loaded from the map's `[OverlayPack]`. Sim-side mutations that create new
/// overlay cells (TIBTRE ore spawn, ore_growth spread) update `OverlayGrid`
/// but never touched `state.overlays`, so the new cells were invisible even
/// though their sim state and pathfinding were correct. This sync closes that
/// gap by injecting an `OverlayEntry` for each newly-overlaid cell, deduping
/// against existing entries.
///
/// Cells whose overlay data merely changed (e.g. ore density grew on an
/// already-overlaid cell, wall took damage) are not handled here — the
/// renderer reads live frames from `OverlayGrid` for entries already in
/// `state.overlays`.
fn sync_new_overlay_cells_to_render_list(
    state: &mut AppState,
    candidates: Vec<crate::map::overlay::OverlayEntry>,
) {
    let new_entries = filter_new_overlay_entries(&state.overlays, candidates);
    if !new_entries.is_empty() {
        log::trace!(
            "Synced {} newly-overlaid cells from OverlayGrid to state.overlays",
            new_entries.len()
        );
        state.overlays.extend(new_entries);
    }
}

/// Pure helper: filter out candidate `OverlayEntry`s whose `(rx, ry)` is
/// already represented in `existing`, and dedup within the candidate list.
///
/// Pulled out for unit testing — the wrapper above does the `state.overlays`
/// extend and logging.
fn filter_new_overlay_entries(
    existing: &[crate::map::overlay::OverlayEntry],
    candidates: Vec<crate::map::overlay::OverlayEntry>,
) -> Vec<crate::map::overlay::OverlayEntry> {
    let existing_set: std::collections::HashSet<(u16, u16)> =
        existing.iter().map(|e| (e.rx, e.ry)).collect();
    let mut seen: std::collections::HashSet<(u16, u16)> = std::collections::HashSet::new();
    let mut out: Vec<crate::map::overlay::OverlayEntry> = Vec::new();
    for entry in candidates {
        let key = (entry.rx, entry.ry);
        if existing_set.contains(&key) || !seen.insert(key) {
            continue;
        }
        out.push(entry);
    }
    out
}

/// Inject newly placed wall buildings as OverlayEntry items into state.overlays,
/// then recompute wall connectivity for all walls so frames auto-tile correctly.
///
/// In RA2, walls (GAWALL, NAWALL) are both [BuildingTypes] and [OverlayTypes].
/// The sim spawns them as GameEntity for health/ownership/combat, but the visual
/// is rendered via the overlay atlas (connectivity bitmask frames 0–15).
/// Without this step, placed walls appear in state.overlays as isolated pillars
/// and never connect to adjacent walls from the map or prior placements.
fn inject_placed_wall_overlays(state: &mut AppState, placed: &[(u16, u16, String)]) {
    let Some(registry) = &state.overlay_registry else {
        return;
    };
    // Collect new entries — need registry borrow released before mutable borrow of overlays.
    let new_entries: Vec<crate::map::overlay::OverlayEntry> = placed
        .iter()
        .filter_map(|(rx, ry, type_id)| {
            let overlay_id = registry.id_for_name(type_id)?;
            // Don't add duplicate — wall may have been on map already.
            let already_present = state
                .overlays
                .iter()
                .any(|e| e.rx == *rx && e.ry == *ry && e.overlay_id == overlay_id);
            if already_present {
                return None;
            }
            Some(crate::map::overlay::OverlayEntry {
                rx: *rx,
                ry: *ry,
                overlay_id,
                frame: 0,
            })
        })
        .collect();

    if new_entries.is_empty() {
        return;
    }

    log::info!(
        "Injecting {} placed wall overlay entries into state.overlays",
        new_entries.len()
    );
    state.overlays.extend(new_entries);

    // Recompute connectivity bitmasks for ALL walls (existing + newly placed).
    if let Some(registry) = &state.overlay_registry {
        let updated = crate::map::overlay::compute_wall_connectivity(&mut state.overlays, registry);
        if updated > 0 {
            log::info!(
                "Wall connectivity recomputed: {} entries updated after placement",
                updated
            );
        }
    }

    // Write placed walls to OverlayGrid and sync connectivity frames.
    if let Some(registry) = &state.overlay_registry {
        if let Some(sim) = &mut state.simulation {
            if let Some(grid) = &mut sim.overlay_grid {
                // Place new wall overlays.
                for (rx, ry, type_id) in placed {
                    if let Some(overlay_id) = registry.id_for_name(type_id) {
                        grid.place_overlay(*rx, *ry, overlay_id, 0);
                    }
                }
                // Sync connectivity frames from state.overlays to OverlayGrid.
                for entry in &state.overlays {
                    if registry.flags(entry.overlay_id).is_some_and(|f| f.wall) {
                        grid.set_overlay_data(entry.rx, entry.ry, entry.frame);
                    }
                }
            }
        }
    }

    // Also register the overlay name in overlay_names so the renderer can look it up.
    if let Some(registry) = &state.overlay_registry {
        for (_, _, type_id) in placed {
            if let Some(overlay_id) = registry.id_for_name(type_id) {
                state
                    .overlay_names
                    .entry(overlay_id)
                    .or_insert_with(|| type_id.clone());
            }
        }
    }
}

fn load_unit_palette(asset_manager: &AssetManager, theater_ext: &str) -> Option<Palette> {
    let themed = format!("unit{}.pal", theater_ext.to_ascii_lowercase());
    let candidates = [
        themed.as_str(),
        "unittem.pal",
        "unitsno.pal",
        "uniturb.pal",
        "unit.pal",
        "temperat.pal",
    ];
    for name in candidates {
        let Some(data) = asset_manager.get(name) else {
            continue;
        };
        if let Ok(pal) = Palette::from_bytes(&data) {
            return Some(pal);
        }
    }
    None
}

/// Check if a cell is walkable on either the ground or bridge layer.
/// Delegates to the unified `PathGrid::is_any_layer_walkable()` method.
pub(crate) fn is_any_layer_walkable(
    grid: &crate::sim::pathfinding::PathGrid,
    x: u16,
    y: u16,
) -> bool {
    grid.is_any_layer_walkable(x, y)
}

pub(crate) fn screen_point_to_world(state: &AppState, screen_x: f32, screen_y: f32) -> (f32, f32) {
    // Screen pixel / zoom = world offset from camera top-left.
    (
        screen_x / state.zoom_level + state.camera_x,
        screen_y / state.zoom_level + state.camera_y,
    )
}

/// Shared owner for world-space point -> map-cell resolution in the app layer.
///
/// Any app code that already has world coordinates should use this instead of
/// re-calling the tactical inverse inline.
pub(crate) fn world_point_to_cell(
    world_x: f32,
    world_y: f32,
    height_map: &std::collections::BTreeMap<(u16, u16), u8>,
    bridge_cells: Option<
        &std::collections::BTreeMap<(u16, u16), crate::map::terrain::TacticalBridgeCell>,
    >,
) -> (u16, u16) {
    let inverse = terrain::screen_to_cell_tactical_inverse(
        world_x,
        world_y,
        terrain::TacticalInverseContext {
            height_map,
            bridge_cells,
            viewport_offset_x: 0.0,
            viewport_offset_y: 0.0,
        },
    );
    let (iso_rx, iso_ry) = match inverse {
        terrain::TacticalInverseResult::Cell { rx, ry }
        | terrain::TacticalInverseResult::Fallback { rx, ry } => (rx, ry),
    };
    (
        // Current Rust app callers expect a concrete in-map cell. Keep this
        // clamp isolated here until off-map sentinel behavior is modeled.
        iso_rx.round().max(0.0) as u16,
        iso_ry.round().max(0.0) as u16,
    )
}

/// Shared owner for screen-space cursor -> map-cell resolution in the app layer.
///
/// This is the entry point UI/input code should use when starting from screen
/// coordinates and the current camera.
pub(crate) fn screen_point_to_world_cell(
    state: &AppState,
    screen_x: f32,
    screen_y: f32,
) -> (u16, u16) {
    let (world_x, world_y) = screen_point_to_world(state, screen_x, screen_y);
    world_point_to_cell(
        world_x,
        world_y,
        &state.height_map,
        Some(&state.tactical_bridge_inverse_map),
    )
}

pub(crate) fn nearest_walkable_cell(
    grid: &crate::sim::pathfinding::PathGrid,
    start: (u16, u16),
    max_radius: u16,
) -> Option<(u16, u16)> {
    grid.nearest_walkable_any_layer(start.0, start.1, max_radius, None, None)
}

pub(crate) fn nearest_walkable_cell_layered(
    grid: &crate::sim::pathfinding::PathGrid,
    start: (u16, u16),
    max_radius: u16,
) -> Option<(u16, u16)> {
    grid.nearest_walkable_any_layer(start.0, start.1, max_radius, None, None)
}

pub(crate) fn clamp_cell_to_grid(
    grid: &crate::sim::pathfinding::PathGrid,
    cell: (u16, u16),
) -> (u16, u16) {
    let max_x = grid.width().saturating_sub(1);
    let max_y = grid.height().saturating_sub(1);
    (cell.0.min(max_x), cell.1.min(max_y))
}

pub(crate) fn rules_hash(rules: &crate::rules::ruleset::RuleSet) -> u64 {
    // The source-INI hash covers the whole merged rules set — type-registry
    // lists AND every scalar value, including a map's value overrides. The
    // former registry-only hash missed those, so a map that overrode e.g.
    // [General]/[CombatDamage] values produced an identical hash and a replay
    // recorded under it could play back against base rules undetected.
    rules.source_ini_hash()
}

#[cfg(test)]
mod tests {
    use super::{
        FixedStepSchedule, MAX_SIM_STEPS_PER_FRAME, append_fire_effect_batch,
        begin_fire_effect_batch, filter_new_overlay_entries, finish_fire_effect_batch,
        schedule_fixed_steps, world_point_to_cell,
    };
    use crate::app_types::SIM_TICK_MS;
    use crate::map::entities::EntityCategory;
    use crate::map::overlay::OverlayEntry;
    use crate::sim::combat::TargetKind;
    use crate::sim::combat::combat_weapon::WeaponSlot;
    use crate::sim::intern::{InternedId, test_intern};
    use crate::sim::world::{FireOriginSnapshot, SimFireEvent};
    use crate::util::fixed_math::SimFixed;
    use std::collections::BTreeMap;

    fn entry(rx: u16, ry: u16, overlay_id: u8, frame: u8) -> OverlayEntry {
        OverlayEntry {
            rx,
            ry,
            overlay_id,
            frame,
        }
    }

    fn fire_event(attacker_id: u64, occupant_anim: Option<InternedId>) -> SimFireEvent {
        SimFireEvent {
            attacker_id,
            attacker_type_ref: test_intern("CABHUT"),
            weapon_slot: WeaponSlot::Primary,
            weapon_id: test_intern("UCWEAPON"),
            facing: 0,
            veterancy: 0,
            origin_snapshot: FireOriginSnapshot {
                rx: 10,
                ry: 20,
                sub_x: SimFixed::ZERO,
                sub_y: SimFixed::ZERO,
                z: 0,
                facing: 0,
                category: EntityCategory::Structure,
                burst_index: 0,
            },
            target: TargetKind::Cell(12, 20),
            report_sound_id: None,
            garrison_muzzle_index: occupant_anim.map(|_| 0),
            occupant_anim,
        }
    }

    #[test]
    fn fire_effect_batch_accumulates_fixed_tick_events_until_finish() {
        let mut pending = vec![fire_event(99, Some(test_intern("STALE")))];
        begin_fire_effect_batch(&mut pending);
        assert!(pending.is_empty());

        append_fire_effect_batch(&mut pending, &[fire_event(1, Some(test_intern("UCFLASH")))]);
        append_fire_effect_batch(
            &mut pending,
            &[
                fire_event(2, Some(test_intern("UCCONS"))),
                fire_event(3, None),
            ],
        );

        let attacker_ids: Vec<u64> = pending.iter().map(|ev| ev.attacker_id).collect();
        assert_eq!(attacker_ids, vec![1, 2, 3]);
        assert_eq!(pending[0].occupant_anim, Some(test_intern("UCFLASH")));
        assert_eq!(pending[1].occupant_anim, Some(test_intern("UCCONS")));
        assert_eq!(pending[2].occupant_anim, None);

        finish_fire_effect_batch(&mut pending);
        assert!(pending.is_empty());
    }

    #[test]
    fn filter_skips_entries_already_in_existing() {
        let existing = vec![entry(5, 5, 2, 0)];
        let candidates = vec![entry(5, 5, 2, 3), entry(6, 6, 2, 0)];
        let new_entries = filter_new_overlay_entries(&existing, candidates);
        assert_eq!(new_entries.len(), 1);
        assert_eq!((new_entries[0].rx, new_entries[0].ry), (6, 6));
    }

    #[test]
    fn filter_dedups_within_candidate_list() {
        let existing: Vec<OverlayEntry> = Vec::new();
        let candidates = vec![entry(7, 7, 2, 0), entry(7, 7, 2, 5), entry(8, 8, 2, 0)];
        let new_entries = filter_new_overlay_entries(&existing, candidates);
        assert_eq!(new_entries.len(), 2);
        // Order preserved; first occurrence kept.
        assert_eq!((new_entries[0].rx, new_entries[0].ry), (7, 7));
        assert_eq!(new_entries[0].frame, 0);
        assert_eq!((new_entries[1].rx, new_entries[1].ry), (8, 8));
    }

    #[test]
    fn filter_empty_inputs() {
        let existing: Vec<OverlayEntry> = Vec::new();
        let candidates: Vec<OverlayEntry> = Vec::new();
        assert!(filter_new_overlay_entries(&existing, candidates).is_empty());
    }

    #[test]
    fn filter_all_candidates_already_in_existing() {
        let existing = vec![entry(1, 1, 2, 0), entry(2, 2, 3, 5)];
        let candidates = vec![entry(1, 1, 2, 7), entry(2, 2, 3, 0)];
        assert!(filter_new_overlay_entries(&existing, candidates).is_empty());
    }

    #[test]
    fn fixed_step_schedule_is_invariant_across_frame_profiles() {
        let profile_a = [16_u64, 16, 16, 16];
        let profile_b = [32_u64, 32];

        let mut state_a = FixedStepSchedule {
            steps: 0,
            remaining_accumulator_ms: 0,
        };
        let mut total_steps_a = 0;
        for frame_ms in profile_a {
            state_a = schedule_fixed_steps(
                state_a.remaining_accumulator_ms,
                frame_ms,
                MAX_SIM_STEPS_PER_FRAME,
            );
            total_steps_a += state_a.steps;
        }

        let mut state_b = FixedStepSchedule {
            steps: 0,
            remaining_accumulator_ms: 0,
        };
        let mut total_steps_b = 0;
        for frame_ms in profile_b {
            state_b = schedule_fixed_steps(
                state_b.remaining_accumulator_ms,
                frame_ms,
                MAX_SIM_STEPS_PER_FRAME,
            );
            total_steps_b += state_b.steps;
        }

        assert_eq!(total_steps_a, total_steps_b);
        assert_eq!(
            state_a.remaining_accumulator_ms,
            state_b.remaining_accumulator_ms
        );
    }

    #[test]
    fn fixed_step_schedule_caps_large_catch_up_bursts() {
        // MAX_UPDATE_DELTA_MS=250 clamps the elapsed time. At SIM_TICK_MS per tick,
        // 250ms / SIM_TICK_MS gives the number of steps (capped to MAX_SIM_STEPS_PER_FRAME).
        // If max_steps is hit and remaining >= SIM_TICK_MS, remaining is clamped to
        // SIM_TICK_MS to prevent accumulator runaway.
        let state = schedule_fixed_steps(0, 1_000, MAX_SIM_STEPS_PER_FRAME);
        let expected_steps = (250 / SIM_TICK_MS).min(MAX_SIM_STEPS_PER_FRAME);
        assert_eq!(state.steps, expected_steps);
        let raw_remaining = 250 - expected_steps as u64 * SIM_TICK_MS as u64;
        let expected_remaining =
            if expected_steps == MAX_SIM_STEPS_PER_FRAME && raw_remaining >= SIM_TICK_MS as u64 {
                SIM_TICK_MS as u64
            } else {
                raw_remaining
            };
        assert_eq!(state.remaining_accumulator_ms, expected_remaining);
    }

    #[test]
    fn fixed_step_schedule_preserves_partial_tick_progress() {
        // Elapsed less than one tick → accumulates without stepping.
        let partial = (SIM_TICK_MS as u64) - 1; // 1ms short of a full tick
        let state = schedule_fixed_steps(0, partial, MAX_SIM_STEPS_PER_FRAME);
        assert_eq!(state.steps, 0);
        assert_eq!(state.remaining_accumulator_ms, partial);

        // Adding 1ms more crosses the threshold → 1 step.
        let next = schedule_fixed_steps(state.remaining_accumulator_ms, 1, MAX_SIM_STEPS_PER_FRAME);
        assert_eq!(next.steps, 1);
        assert_eq!(next.remaining_accumulator_ms, 0);
    }

    #[test]
    fn fixed_step_schedule_reset_prevents_pause_burst_on_resume() {
        let expected_steps = (250 / SIM_TICK_MS).min(MAX_SIM_STEPS_PER_FRAME);
        let burst = schedule_fixed_steps(0, 1_000, MAX_SIM_STEPS_PER_FRAME);
        assert_eq!(burst.steps, expected_steps);

        let resumed = schedule_fixed_steps(0, 0, MAX_SIM_STEPS_PER_FRAME);
        assert_eq!(resumed.steps, 0);
        assert_eq!(resumed.remaining_accumulator_ms, 0);
    }

    #[test]
    fn fixed_step_schedule_reset_prevents_stale_transition_delta() {
        let expected_steps = (250 / SIM_TICK_MS).min(MAX_SIM_STEPS_PER_FRAME);
        let stale_menu_time = schedule_fixed_steps(0, 500, MAX_SIM_STEPS_PER_FRAME);
        assert_eq!(stale_menu_time.steps, expected_steps);

        let first_ingame_update = schedule_fixed_steps(0, 0, MAX_SIM_STEPS_PER_FRAME);
        assert_eq!(first_ingame_update.steps, 0);
        assert_eq!(first_ingame_update.remaining_accumulator_ms, 0);
    }

    #[test]
    fn world_point_to_cell_round_trips_ground_iso_anchor() {
        let (rx, ry, z) = (10_u16, 5_u16, 4_u8);
        let (world_x, world_y) = (150.0, 180.0);
        let mut height_map = BTreeMap::new();
        for hx in 8..=12 {
            for hy in 3..=7 {
                height_map.insert((hx, hy), z);
            }
        }

        assert_eq!(
            world_point_to_cell(world_x, world_y, &height_map, None),
            (rx, ry)
        );
    }

    #[test]
    fn world_point_to_cell_forwards_tactical_bridge_inverse_map() {
        let (world_x, world_y) = (150.0, 180.0);
        let height_map = BTreeMap::new();
        let bridge_cells = BTreeMap::from([(
            (10, 5),
            crate::map::terrain::TacticalBridgeCell {
                deck_z: 4,
                structural: true,
                direction_zero: true,
            },
        )]);
        let expected = crate::map::terrain::screen_to_cell_tactical_inverse(
            world_x,
            world_y,
            crate::map::terrain::TacticalInverseContext {
                height_map: &height_map,
                bridge_cells: Some(&bridge_cells),
                viewport_offset_x: 0.0,
                viewport_offset_y: 0.0,
            },
        );
        let (expected_rx, expected_ry) = match expected {
            crate::map::terrain::TacticalInverseResult::Cell { rx, ry }
            | crate::map::terrain::TacticalInverseResult::Fallback { rx, ry } => (rx, ry),
        };

        assert_eq!(
            world_point_to_cell(world_x, world_y, &height_map, Some(&bridge_cells)),
            (
                expected_rx.round().max(0.0) as u16,
                expected_ry.round().max(0.0) as u16,
            )
        );
    }

    #[test]
    fn world_point_to_cell_clamps_negative_results_to_zero() {
        let height_map = BTreeMap::new();
        assert_eq!(
            world_point_to_cell(-500.0, -500.0, &height_map, None),
            (0, 0)
        );
    }
}

#[cfg(test)]
mod modal_pump_tests {
    use super::{SessionMode, modal_pump_should_advance_sim};

    #[test]
    fn session_mode_maps_writer_proofed_game_mode_values() {
        assert_eq!(SessionMode::from_game_mode(0), SessionMode::Campaign);
        assert_eq!(SessionMode::from_game_mode(3), SessionMode::Lan);
        assert_eq!(SessionMode::from_game_mode(4), SessionMode::Wol);
        assert_eq!(SessionMode::from_game_mode(5), SessionMode::Skirmish);
        // Legacy modem/serial (1/2) and any unrecognized value -> Other. The active
        // engine never writes these, so the pump treats them as non-advancing.
        assert_eq!(SessionMode::from_game_mode(1), SessionMode::Other);
        assert_eq!(SessionMode::from_game_mode(2), SessionMode::Other);
        assert_eq!(SessionMode::from_game_mode(-1), SessionMode::Other);
        assert_eq!(SessionMode::from_game_mode(99), SessionMode::Other);
    }

    #[test]
    fn only_network_modes_advance_behind_a_modal() {
        // {3 LAN, 4 WOL} advance; {0 campaign, 5 skirmish} + Other freeze.
        assert!(modal_pump_should_advance_sim(SessionMode::Lan, false));
        assert!(modal_pump_should_advance_sim(SessionMode::Wol, false));
        assert!(!modal_pump_should_advance_sim(SessionMode::Campaign, false));
        assert!(!modal_pump_should_advance_sim(SessionMode::Skirmish, false));
        assert!(!modal_pump_should_advance_sim(SessionMode::Other, false));
    }

    #[test]
    fn reentrancy_guard_blocks_advance_even_on_network() {
        // The native reentrancy guard: a fixed tick already in progress means the
        // pump skips advancing, even on the network branch.
        assert!(!modal_pump_should_advance_sim(SessionMode::Lan, true));
        assert!(!modal_pump_should_advance_sim(SessionMode::Wol, true));
    }

    #[test]
    fn pumped_tick_delta_is_zero_offline_and_n_on_network() {
        // C2 acceptance at the decision level: the pump decision drives whether the
        // world's fixed tick advances per pumped frame. `tick` stands in for
        // `sim.session.tick`; the full headless-World assertion (incl. "no
        // battlefield recomposite offline") lands with the live `service_tick` swap.
        const FRAMES: u64 = 7;
        let pumped = |mode: SessionMode| -> u64 {
            let mut tick = 0u64;
            for _ in 0..FRAMES {
                if modal_pump_should_advance_sim(mode, false) {
                    tick += 1; // one fixed step advanced this pumped frame
                }
            }
            tick
        };
        // Offline freezes: zero advance over N pumped frames.
        assert_eq!(pumped(SessionMode::Skirmish), 0);
        assert_eq!(pumped(SessionMode::Campaign), 0);
        // Network advances once per pumped frame.
        assert_eq!(pumped(SessionMode::Lan), FRAMES);
        assert_eq!(pumped(SessionMode::Wol), FRAMES);
    }

    #[test]
    fn pumped_world_tick_freezes_offline_and_advances_on_network() {
        use crate::sim::world::Simulation;
        use std::collections::BTreeMap;

        // C2 acceptance with a real headless World: drive `advance_tick` exactly
        // when the pump decision is true, and assert `session.tick` motion.
        // `advance_tick` commits one tick per call (no entities/rules needed).
        const FRAMES: u64 = 7;
        let pumped_world_delta = |mode: SessionMode| -> u64 {
            let mut sim = Simulation::new();
            let start = sim.session.tick;
            let height_map: BTreeMap<(u16, u16), u8> = BTreeMap::new();
            for _ in 0..FRAMES {
                if modal_pump_should_advance_sim(mode, false) {
                    // `tick_ms` does not affect the asserted tick delta; a literal
                    // matches the sim-test style and avoids the const dependency.
                    sim.advance_tick(&[], None, &height_map, None, None, 33);
                }
            }
            sim.session.tick - start
        };

        // Offline modes freeze the world behind the modal: zero tick advance.
        assert_eq!(pumped_world_delta(SessionMode::Skirmish), 0);
        assert_eq!(pumped_world_delta(SessionMode::Campaign), 0);
        // Network modes advance one fixed tick per pumped frame (dead code this
        // build; proves the contract for when multiplayer lands).
        assert_eq!(pumped_world_delta(SessionMode::Lan), FRAMES);
        assert_eq!(pumped_world_delta(SessionMode::Wol), FRAMES);
    }
}
