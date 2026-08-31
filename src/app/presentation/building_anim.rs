//! Building animation lifecycle, sidebar UI tick, and sound playback.
//!
//! These are per-frame runtime updates that run after the sim tick advances.
//! Split from `match_runtime::sim_tick` to separate animation/audio/UI concerns from
//! core simulation advancement.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use std::collections::HashMap;

use crate::app::AppState;
use crate::app::input::commands::preferred_local_owner_name;
use crate::app::types::SIM_TICK_MS;
use crate::sim::components::{AnimRuntime, GarrisonMuzzleFlash};
use crate::sim::production;
use crate::sim::world::Simulation;

const GARRISON_OCCUPANT_ANIM_Z_ADJUST: i32 = -200;

/// Advance the app-owned wall-clock terrain-overlay animation timer.
///
/// Building one-shot overlays now advance inside the authoritative simulation
/// frame; this independent timer only drives looping terrain presentation.
pub(crate) fn tick_terrain_overlay_animations(state: &mut AppState, dt_ms: u32) {
    state.match_state.match_presentation.idle_anim_elapsed_ms += dt_ms;
}

pub(crate) use crate::sim::world::building_anim::building_anim_rate_logic_frames;

pub(crate) struct BuildingAnimFrameView<'a> {
    pub(crate) anim_type: &'a str,
    pub(crate) loop_start: u16,
    pub(crate) loop_end: u16,
    pub(crate) loop_count: i32,
    pub(crate) start_frame: u16,
    pub(crate) ping_pong: bool,
}

pub(crate) fn selected_building_anim_view<'a>(
    anim: &'a crate::rules::art_data::BuildingAnimConfig,
    building_damage_state_active: bool,
    is_garrisoned: bool,
) -> BuildingAnimFrameView<'a> {
    let variant = if building_damage_state_active {
        anim.damaged_variant.as_ref()
    } else if is_garrisoned {
        anim.garrisoned_variant.as_ref()
    } else {
        None
    };
    match variant {
        Some(variant) => BuildingAnimFrameView {
            anim_type: &variant.anim_type,
            loop_start: variant.loop_start,
            loop_end: variant.loop_end,
            loop_count: variant.loop_count,
            start_frame: variant.start_frame,
            ping_pong: variant.ping_pong,
        },
        None => BuildingAnimFrameView {
            anim_type: &anim.anim_type,
            loop_start: anim.loop_start,
            loop_end: anim.loop_end,
            loop_count: anim.loop_count,
            start_frame: anim.start_frame,
            ping_pong: anim.ping_pong,
        },
    }
}

fn fresh_building_slot_runtime(
    anim_type: &str,
    art: &crate::rules::art_data::ArtRegistry,
    options: &crate::sim::game_options::GameOptions,
    frame_counts: &HashMap<String, u16>,
) -> Option<AnimRuntime> {
    if anim_type.is_empty() {
        return None;
    }
    let config = art.anim_runtime_config(anim_type)?;
    let mut runtime = garrison_occupant_anim_runtime(
        anim_type,
        config,
        anim_total_frames(frame_counts, anim_type),
    );
    runtime.reload_logic_frames = building_anim_rate_logic_frames(art, anim_type, Some(options));
    Some(runtime)
}

/// Recreate one occupied native slot for a damage-state edge. Only the old
/// AnimClass `CurrentFrame` field survives; constructor direction, cadence,
/// loop byte, and first-AI guard all come from the newly selected descriptor.
fn replace_occupied_slot_for_damage_state(
    slot: &mut Option<AnimRuntime>,
    anim: &crate::rules::art_data::BuildingAnimConfig,
    damaged: bool,
    art: &crate::rules::art_data::ArtRegistry,
    options: &crate::sim::game_options::GameOptions,
    frame_counts: &HashMap<String, u16>,
) -> bool {
    let Some(old) = slot.as_ref() else {
        return false;
    };
    let selected_type = if damaged {
        let Some(variant) = anim.damaged_variant.as_ref() else {
            return false;
        };
        variant.anim_type.as_str()
    } else {
        anim.anim_type.as_str()
    };
    let Some(mut replacement) =
        fresh_building_slot_runtime(selected_type, art, options, frame_counts)
    else {
        return false;
    };
    replacement.current_frame = old.current_frame;
    *slot = Some(replacement);
    true
}

pub(crate) fn app_owns_looping_building_slot(
    anim: &crate::rules::art_data::BuildingAnimConfig,
    damaged: bool,
    is_garrisoned: bool,
) -> bool {
    let selected = selected_building_anim_view(anim, damaged, is_garrisoned);
    matches!(anim.kind, crate::rules::art_data::BuildingAnimKind::Idle)
        || matches!(
            anim.kind,
            crate::rules::art_data::BuildingAnimKind::Active
                | crate::rules::art_data::BuildingAnimKind::Production
        ) && selected.loop_count < 0
}

/// Whether an `InfantryAbsorb` building's ActiveAnim slot is absent for the
/// current occupancy. The native branch owns only its first two Active slots.
pub(crate) fn infantry_absorb_active_slot_is_absent(
    active_slot_ordinal: usize,
    is_garrisoned: bool,
) -> bool {
    match active_slot_ordinal {
        0 => is_garrisoned,
        1 => !is_garrisoned,
        _ => false,
    }
}

/// Reconcile which app-owned looping native slots currently exist.
///
/// In particular, YAPOWR does not keep both mutually exclusive ActiveAnim
/// instances alive and merely hide one. It deletes the old slot and constructs
/// the newly selected slot from fresh AnimClass state whenever occupancy flips.
fn reconcile_looping_building_slot_occupancy(
    slots: &mut Vec<Option<AnimRuntime>>,
    anims: &[crate::rules::art_data::BuildingAnimConfig],
    infantry_absorb_dynamic: bool,
    damaged: bool,
    is_garrisoned: bool,
    art: &crate::rules::art_data::ArtRegistry,
    options: &crate::sim::game_options::GameOptions,
    frame_counts: &HashMap<String, u16>,
) {
    slots.resize_with(anims.len(), || None);
    slots.truncate(anims.len());

    let mut active_slot_ordinal = 0usize;
    for (anim, slot) in anims.iter().zip(slots) {
        let this_active_ordinal = active_slot_ordinal;
        if matches!(anim.kind, crate::rules::art_data::BuildingAnimKind::Active) {
            active_slot_ordinal += 1;
        }

        let occupancy_makes_absent = infantry_absorb_dynamic
            && matches!(anim.kind, crate::rules::art_data::BuildingAnimKind::Active)
            && infantry_absorb_active_slot_is_absent(this_active_ordinal, is_garrisoned);
        let should_exist =
            app_owns_looping_building_slot(anim, damaged, is_garrisoned) && !occupancy_makes_absent;

        if !should_exist {
            *slot = None;
        } else if slot.is_none() {
            let selected = selected_building_anim_view(anim, damaged, is_garrisoned);
            *slot = fresh_building_slot_runtime(selected.anim_type, art, options, frame_counts);
        }
    }
}

/// Tick each represented looping Building animation once per committed logic
/// frame and reconcile per-slot damage replacement edges.
pub(crate) fn refresh_building_anim_phase_bases(state: &mut AppState, frame_committed: bool) {
    if !frame_committed {
        return;
    }

    let match_state = &mut state.match_state;
    let Some(runtime) = match_state.sim_runtime.as_ref() else {
        match_state
            .match_presentation
            .building_anim_phase_base
            .clear();
        return;
    };
    let sim = &runtime.simulation;
    let rules = &runtime.resources.rules;
    let art = &rules.art_registry;
    let options = &sim.session.game_options;
    let (atlas, phase_bases) = (
        &match_state.match_presentation.sprite_atlas,
        &mut match_state.match_presentation.building_anim_phase_base,
    );
    let Some(atlas) = atlas.as_ref() else {
        return;
    };
    let frame_counts = &atlas.active_anim_frame_counts;
    let live_ids: Vec<u64> = sim
        .entities()
        .iter_sorted()
        .filter(|(_, entity)| entity.category == crate::map::entities::EntityCategory::Structure)
        .map(|(stable_id, _)| stable_id)
        .collect();
    phase_bases.retain(|stable_id, _| live_ids.binary_search(stable_id).is_ok());

    for stable_id in live_ids {
        let Some(entity) = sim.entities().get(stable_id) else {
            continue;
        };
        let building_type = sim.interner.resolve(entity.type_ref);
        let rules_image = rules
            .object(building_type)
            .map(|object| object.image.as_str())
            .unwrap_or(building_type);
        let infantry_absorb_dynamic = rules
            .object(building_type)
            .is_some_and(|object| object.infantry_absorb && object.extra_power > 0);
        let Some(entry) = art.resolve_metadata_entry(building_type, rules_image) else {
            phase_bases.remove(&stable_id);
            continue;
        };
        let damaged = entity.building_damage_state_active;
        let is_garrisoned = entity
            .passenger_role
            .cargo()
            .is_some_and(|cargo| !cargo.is_empty());
        let reset_revision = entity.building_anim_reset_revision;

        use std::collections::btree_map::Entry;
        let phase = match phase_bases.entry(stable_id) {
            Entry::Vacant(vacant) => {
                vacant.insert(crate::app::presentation::state::BuildingAnimPhaseBase {
                    observed_reset_revision: reset_revision,
                    slots: vec![None; entry.building_anims.len()],
                })
            }
            Entry::Occupied(occupied) => occupied.into_mut(),
        };

        if phase.observed_reset_revision != reset_revision {
            for (anim, slot) in entry.building_anims.iter().zip(&mut phase.slots) {
                let _ = replace_occupied_slot_for_damage_state(
                    slot,
                    anim,
                    damaged,
                    art,
                    options,
                    frame_counts,
                );
            }
            phase.observed_reset_revision = reset_revision;
        }

        reconcile_looping_building_slot_occupancy(
            &mut phase.slots,
            &entry.building_anims,
            infantry_absorb_dynamic,
            damaged,
            is_garrisoned,
            art,
            options,
            frame_counts,
        );

        for slot in &mut phase.slots {
            if let Some(runtime) = slot.as_mut() {
                advance_anim_runtime_visit(runtime, sim, art, frame_counts);
            }
            if slot.as_ref().is_some_and(|runtime| runtime.expired) {
                *slot = None;
            }
        }
    }
}

/// Tick the sidebar power bar animation (segment-by-segment transition).
pub(crate) fn update_power_bar_anim(state: &mut AppState) {
    let owner_name = preferred_local_owner_name(state);
    let (power_produced, power_drained) = match (
        state
            .match_state
            .sim_runtime
            .as_ref()
            .map(|rt| &rt.simulation),
        state.rules(),
        owner_name.as_deref(),
    ) {
        (Some(sim), Some(rules), Some(owner)) => {
            production::power_balance_for_owner(sim, rules, owner)
        }
        _ => (0, 0),
    };
    let theoretical = match (
        state
            .match_state
            .sim_runtime
            .as_ref()
            .map(|rt| &rt.simulation),
        owner_name.as_deref(),
    ) {
        (Some(sim), Some(owner)) => production::theoretical_power_for_owner(sim, owner),
        _ => 0,
    };

    // Compute bar height from sidebar layout.
    let spec = state.match_state.match_presentation.sidebar_layout_spec;
    let sw = state.render_width() as f32;
    let sh = state.render_height() as f32;
    let layout = crate::sidebar::compute_layout_with_spec(spec, sw, sh, 0);
    let region_bottom = layout.side3_y + spec.side3_height - spec.power_bar_bottom_y;
    let region_top = layout.tabs_y + spec.power_bar_top_y;
    let bar_height_px = (region_bottom - region_top).max(0.0) as i32;

    state
        .match_state
        .match_presentation
        .power_bar_anim
        .set_max_segments(bar_height_px);
    state.match_state.match_presentation.power_bar_anim.update(
        power_produced,
        power_drained,
        theoretical,
    );
    state.match_state.match_presentation.power_bar_anim.tick();
}

/// Update radar availability from ECS and tick the radar chrome animation.
pub(crate) fn update_radar_state(state: &mut AppState, dt_ms: f32) {
    let new_has_radar: bool = match (
        state
            .match_state
            .sim_runtime
            .as_ref()
            .map(|rt| &rt.simulation),
        state.rules(),
        preferred_local_owner_name(state).as_deref(),
    ) {
        (Some(sim), Some(rules), Some(owner)) => {
            crate::sim::radar::has_radar_for_owner(sim, rules, owner)
        }
        _ => false,
    };
    state.match_state.match_presentation.has_radar = new_has_radar;

    if let Some(ref mut ra) = state.match_state.match_presentation.radar_anim {
        ra.set_has_radar(new_has_radar);
        ra.tick(&state.renderer.gpu, dt_ms);
    }
}

/// Map an owner's country name to the EVA faction key used in eva.ini sections.
///
/// Returns "Allied", "Russian", or "Yuri" for lookup in `EvaRegistry::get()`.
pub(crate) fn eva_faction_key(
    owner: &str,
    house_roster: &crate::map::houses::HouseRoster,
) -> &'static str {
    // Find the house's country name from the roster.
    let country = house_roster
        .houses
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case(owner))
        .and_then(|h| h.country.as_deref())
        .unwrap_or(owner);

    // Map country to EVA faction key.
    // Soviet countries use "Russian" (the key name in eva.ini).
    match country.to_ascii_lowercase().as_str() {
        "yuricountry" => "Yuri",
        "russians" | "confederation" | "africans" | "arabs" => "Russian",
        _ => "Allied",
    }
}

/// Drain pending sound events from the queue and play them through the SFX player.
///
/// Voice events (VoiceSelect, VoiceMove, VoiceAttack) are routed to the dedicated
/// voice slot which cuts off the previous voice. All other sounds go to the SFX pool.
pub(crate) fn drain_sound_events(state: &mut AppState) {
    use crate::audio::events::GameSoundEvent;
    use crate::audio::sfx::calc_spatial_volume;

    let events = state.match_state.match_audio.sound_events.drain();
    if events.is_empty() {
        if let Some(sfx) = &mut state.audio.sfx_player {
            sfx.advance_voice_queue();
        }
        return;
    }
    let vp_w = state.render_width() as f32;
    let vp_h = state.render_height() as f32;
    let (Some(sfx), Some(assets)) = (&mut state.audio.sfx_player, state.process_assets.manager())
    else {
        return;
    };
    let cam_x = state.match_state.input.camera_x;
    let cam_y = state.match_state.input.camera_y;
    sfx.advance_voice_queue();

    for event in &events {
        match event {
            // Voice events — always full volume (non-positional), use dedicated voice slot.
            GameSoundEvent::UnitSelected { .. }
            | GameSoundEvent::UnitMoveOrder { .. }
            | GameSoundEvent::UnitAttackOrder { .. } => {
                sfx.play_voice_sound(
                    event.sound_id(),
                    &state.audio.sound_registry,
                    assets,
                    &state.audio.audio_indices,
                );
            }
            // STANDARD EVA cues are fire-and-forget: play only if voice is idle.
            GameSoundEvent::BuildingReady { .. }
            | GameSoundEvent::UnitReady { .. }
            | GameSoundEvent::CannotDeployHere { .. }
            | GameSoundEvent::OutcomeEva { .. } => {
                sfx.play_standard_eva_sound(
                    event.sound_id(),
                    &state.audio.sound_registry,
                    assets,
                    &state.audio.audio_indices,
                );
            }
            // Garrison EVA cues are evamd.ini Type=QUEUE.
            GameSoundEvent::StructureGarrisoned { .. }
            | GameSoundEvent::StructureAbandoned { .. } => {
                sfx.queue_eva_sound(
                    event.sound_id(),
                    &state.audio.sound_registry,
                    assets,
                    &state.audio.audio_indices,
                );
            }
            // UI events — always full volume (non-positional).
            GameSoundEvent::UiSound { .. } => {
                sfx.play_sound(
                    event.sound_id(),
                    &state.audio.sound_registry,
                    assets,
                    &state.audio.audio_indices,
                );
            }
            GameSoundEvent::AnimationStarted {
                anim_id,
                sound_id,
                screen_pos,
            } => {
                let spatial_vol = if let Some((sx, sy)) = screen_pos {
                    let (range, min_vol) = state
                        .audio
                        .sound_registry
                        .get(sound_id)
                        .map(|entry| (entry.range, entry.min_volume))
                        .unwrap_or((crate::audio::sfx::DEFAULT_RANGE_CELLS, 0));
                    calc_spatial_volume(*sx, *sy, vp_w, vp_h, cam_x, cam_y, range, min_vol)
                } else {
                    1.0
                };
                if spatial_vol > 0.0 {
                    sfx.play_animation_sound_with_volume(
                        *anim_id,
                        sound_id,
                        spatial_vol,
                        &state.audio.sound_registry,
                        assets,
                        &state.audio.audio_indices,
                    );
                }
            }
            GameSoundEvent::AnimationStopped {
                anim_id,
                stop_sound_id,
                screen_pos,
            } => {
                sfx.stop_animation_sound(*anim_id);
                if let Some(stop_sound_id) = stop_sound_id.as_deref().filter(|id| !id.is_empty()) {
                    let spatial_vol = if let Some((sx, sy)) = screen_pos {
                        let (range, min_vol) = state
                            .audio
                            .sound_registry
                            .get(stop_sound_id)
                            .map(|entry| (entry.range, entry.min_volume))
                            .unwrap_or((crate::audio::sfx::DEFAULT_RANGE_CELLS, 0));
                        calc_spatial_volume(*sx, *sy, vp_w, vp_h, cam_x, cam_y, range, min_vol)
                    } else {
                        1.0
                    };
                    if spatial_vol > 0.0 {
                        sfx.play_sound_with_volume(
                            stop_sound_id,
                            spatial_vol,
                            &state.audio.sound_registry,
                            assets,
                            &state.audio.audio_indices,
                        );
                    }
                }
            }
            GameSoundEvent::CloakSound {
                sound_id,
                screen_pos,
            } => {
                // RulesClass::ReadAudioVisual @ 0x006691E0 stores only the
                // VocClass::FindByName @ 0x007514D0 result. An invalid name is
                // silent; it must not enter the generic raw audio-bag fallback.
                let Some(entry) = state.audio.sound_registry.get(sound_id) else {
                    continue;
                };
                let spatial_vol = if let Some((sx, sy)) = screen_pos {
                    calc_spatial_volume(
                        *sx,
                        *sy,
                        vp_w,
                        vp_h,
                        cam_x,
                        cam_y,
                        entry.range,
                        entry.min_volume,
                    )
                } else {
                    1.0
                };
                if spatial_vol > 0.0 {
                    sfx.play_registered_sound_with_volume(
                        sound_id,
                        spatial_vol,
                        &state.audio.sound_registry,
                        assets,
                        &state.audio.audio_indices,
                    );
                }
            }
            GameSoundEvent::BridgeRepaired {
                sound_id,
                screen_pos,
                eva_sound_id,
            } => {
                if !sound_id.is_empty() {
                    let spatial_vol = if let Some((sx, sy)) = screen_pos {
                        let (range, min_vol) = state
                            .audio
                            .sound_registry
                            .get(sound_id)
                            .map(|e| (e.range, e.min_volume))
                            .unwrap_or((crate::audio::sfx::DEFAULT_RANGE_CELLS, 0));
                        calc_spatial_volume(*sx, *sy, vp_w, vp_h, cam_x, cam_y, range, min_vol)
                    } else {
                        1.0
                    };
                    if spatial_vol > 0.0 {
                        sfx.play_sound_with_volume(
                            sound_id,
                            spatial_vol,
                            &state.audio.sound_registry,
                            assets,
                            &state.audio.audio_indices,
                        );
                    }
                }
                if let Some(eva_sound_id) = eva_sound_id.as_deref().filter(|s| !s.is_empty()) {
                    sfx.play_standard_eva_sound(
                        eva_sound_id,
                        &state.audio.sound_registry,
                        assets,
                        &state.audio.audio_indices,
                    );
                }
            }
            GameSoundEvent::UnderAttackEva { eva_sound_id } => {
                // Voice-queued (not immediate): under-attack announcements
                // wait behind whatever EVA line is currently speaking.
                let _ = sfx.queue_eva_sound(
                    eva_sound_id,
                    &state.audio.sound_registry,
                    assets,
                    &state.audio.audio_indices,
                );
            }
            // Spatial events — apply distance-based volume scaling using
            // per-sound Range and MinVolume from sound.ini.
            _ => {
                let spatial_vol = if let Some((sx, sy)) = event.screen_pos() {
                    let (range, min_vol) = state
                        .audio
                        .sound_registry
                        .get(event.sound_id())
                        .map(|e| (e.range, e.min_volume))
                        .unwrap_or((crate::audio::sfx::DEFAULT_RANGE_CELLS, 0));
                    calc_spatial_volume(sx, sy, vp_w, vp_h, cam_x, cam_y, range, min_vol)
                } else {
                    1.0
                };

                if spatial_vol > 0.0 {
                    sfx.play_sound_with_volume(
                        event.sound_id(),
                        spatial_vol,
                        &state.audio.sound_registry,
                        assets,
                        &state.audio.audio_indices,
                    );
                }
            }
        }
    }
}

/// Spawn new garrison muzzle flash animations from pending fire events and
/// advance existing ones. One-shot flashes are removed when their animation
/// completes.
///
/// Fire events with `garrison_muzzle_index` and `occupant_anim` produce a
/// short OccupantAnim SHP (e.g., UCFLASH) at the building's MuzzleFlash
/// pixel offset from art.ini.
pub(crate) fn tick_garrison_muzzle_flashes(state: &mut AppState, dt_ms: u32) {
    // Phase 1: spawn new flashes from pending fire events.
    let new_flashes: Vec<GarrisonMuzzleFlash> = {
        let sim = match state
            .match_state
            .sim_runtime
            .as_ref()
            .map(|rt| &rt.simulation)
        {
            Some(s) => s,
            None => {
                state
                    .match_state
                    .match_presentation
                    .garrison_muzzle_flashes
                    .clear();
                return;
            }
        };
        let art_reg = match state.rules().map(|rules| &rules.art_registry) {
            Some(a) => a,
            None => {
                state
                    .match_state
                    .match_presentation
                    .garrison_muzzle_flashes
                    .clear();
                return;
            }
        };
        let rules = match state.rules() {
            Some(r) => r,
            None => {
                state
                    .match_state
                    .match_presentation
                    .garrison_muzzle_flashes
                    .clear();
                return;
            }
        };
        let frame_counts = state
            .match_state
            .match_presentation
            .sprite_atlas
            .as_ref()
            .map(|atlas| &atlas.active_anim_frame_counts);
        state
            .match_state
            .match_presentation
            .pending_fire_effects
            .iter()
            .filter_map(|ev| {
                let anim_name = ev.occupant_anim.as_ref()?;
                let anim_section = sim.interner.resolve(*anim_name).to_ascii_uppercase();
                let origin = crate::app::presentation::fire_effects::resolve_fire_origin_from_sim(
                    sim, rules, art_reg, ev,
                )
                .ok()?;
                let runtime_config = art_reg.anim_runtime_config(&anim_section)?;
                let total_frames =
                    presentation_anim_frame_count(frame_counts, &anim_section).unwrap_or(1);
                Some(GarrisonMuzzleFlash {
                    building_id: ev.attacker_id,
                    runtime: garrison_occupant_anim_runtime(
                        &anim_section,
                        runtime_config,
                        total_frames,
                    ),
                    pixel_x: 0,
                    pixel_y: 0,
                    screen_x: origin.screen_x,
                    screen_y: origin.screen_y,
                    rx: origin.rx,
                    ry: origin.ry,
                    z: origin.z,
                    z_adjust: GARRISON_OCCUPANT_ANIM_Z_ADJUST,
                })
            })
            .collect()
    };
    state
        .match_state
        .match_presentation
        .garrison_muzzle_flashes
        .extend(new_flashes);

    // Phase 2: advance all flashes and remove finished ones. This is fed from
    // completed fixed sim ticks, not render-frame wall time.
    let Some((sim, art_reg)) = state
        .match_state
        .sim_runtime
        .as_ref()
        .map(|rt| (&rt.simulation, &rt.resources.rules.art_registry))
    else {
        state
            .match_state
            .match_presentation
            .garrison_muzzle_flashes
            .clear();
        return;
    };
    let empty_frame_counts = HashMap::new();
    let frame_counts = state
        .match_state
        .match_presentation
        .sprite_atlas
        .as_ref()
        .map(|atlas| &atlas.active_anim_frame_counts)
        .unwrap_or(&empty_frame_counts);
    state
        .match_state
        .match_presentation
        .garrison_muzzle_flashes
        .retain_mut(|flash| {
            advance_garrison_muzzle_flash(flash, dt_ms, sim, art_reg, frame_counts)
        });
}

fn advance_garrison_muzzle_flash(
    flash: &mut GarrisonMuzzleFlash,
    dt_ms: u32,
    sim: &Simulation,
    art_reg: &crate::rules::art_data::ArtRegistry,
    frame_counts: &HashMap<String, u16>,
) -> bool {
    flash.runtime.elapsed_logic_ms = flash.runtime.elapsed_logic_ms.saturating_add(dt_ms);
    while flash.runtime.elapsed_logic_ms >= SIM_TICK_MS && !flash.runtime.expired {
        flash.runtime.elapsed_logic_ms -= SIM_TICK_MS;
        advance_anim_runtime_visit(&mut flash.runtime, sim, art_reg, frame_counts);
    }
    !flash.runtime.expired
}

fn garrison_occupant_anim_runtime(
    anim_section: &str,
    config: &crate::rules::art_data::AnimTypeRuntimeConfig,
    total_frames: u16,
) -> AnimRuntime {
    let end = effective_anim_end(config, total_frames);
    let loop_end = effective_anim_loop_end(config, end);
    let reverse = config.reverse;
    AnimRuntime {
        type_name: anim_section.to_ascii_uppercase(),
        current_frame: if reverse { loop_end - 1 } else { 0 },
        frame_step: if reverse { -1 } else { 1 },
        delay_logic_frames: 0,
        reload_logic_frames: config.rate_logic_frames,
        rate_elapsed_logic_frames: 0,
        loop_remaining: native_loop_remaining(config.loop_count, 1),
        first_ai_guard: true,
        expired: false,
        constructor_reverse: false,
        elapsed_logic_ms: 0,
    }
}

#[cfg(test)]
fn garrison_occupant_anim_rate_logic_frames(
    sim: &Simulation,
    art_reg: &crate::rules::art_data::ArtRegistry,
    anim_name: crate::sim::intern::InternedId,
) -> Option<u16> {
    let anim_section = sim.interner.resolve(anim_name);
    art_reg
        .anim_runtime_config(anim_section)
        .map(|config| config.rate_logic_frames)
}

fn advance_anim_runtime_visit(
    runtime: &mut AnimRuntime,
    sim: &Simulation,
    art_reg: &crate::rules::art_data::ArtRegistry,
    frame_counts: &HashMap<String, u16>,
) {
    advance_anim_runtime_visit_with_events(runtime, sim, art_reg, frame_counts, None);
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AnimRuntimeVisitEvent {
    TrailerSpawn {
        parent_type: String,
        trailer_type: String,
    },
    NextInPlace {
        previous_type: String,
        next_type: String,
    },
    NormalDestroy {
        type_name: String,
    },
}

fn advance_anim_runtime_visit_with_events(
    runtime: &mut AnimRuntime,
    sim: &Simulation,
    art_reg: &crate::rules::art_data::ArtRegistry,
    frame_counts: &HashMap<String, u16>,
    mut events: Option<&mut Vec<AnimRuntimeVisitEvent>>,
) {
    if runtime.expired {
        return;
    }
    if let Some(config) = art_reg.anim_runtime_config(&runtime.type_name) {
        emit_anim_runtime_trailer(runtime, config, sim, &mut events);
    }
    if runtime.first_ai_guard {
        runtime.first_ai_guard = false;
        return;
    }
    if runtime.delay_logic_frames > 0 {
        runtime.delay_logic_frames -= 1;
        return;
    }
    if runtime.reload_logic_frames == 0 {
        return;
    }
    runtime.rate_elapsed_logic_frames = runtime.rate_elapsed_logic_frames.saturating_add(1);
    if runtime.rate_elapsed_logic_frames < runtime.reload_logic_frames {
        return;
    }
    runtime.rate_elapsed_logic_frames = 0;
    runtime.current_frame += runtime.frame_step;

    let Some(config) = art_reg.anim_runtime_config(&runtime.type_name) else {
        runtime.expired = true;
        return;
    };
    if config.ping_pong && anim_runtime_at_boundary(runtime, config, frame_counts) {
        runtime.frame_step = -runtime.frame_step;
        return;
    }
    if !anim_runtime_at_boundary(runtime, config, frame_counts) {
        return;
    }
    if runtime.loop_remaining != 0 && runtime.loop_remaining != u8::MAX {
        runtime.loop_remaining = runtime.loop_remaining.saturating_sub(1);
    }
    if runtime.loop_remaining != 0 {
        reset_anim_runtime_to_loop_start(runtime, config, frame_counts);
        return;
    }
    if let Some(next) = &config.next {
        switch_anim_runtime_type(runtime, next, art_reg, frame_counts, &mut events);
    } else {
        if let Some(events) = events.as_deref_mut() {
            events.push(AnimRuntimeVisitEvent::NormalDestroy {
                type_name: runtime.type_name.clone(),
            });
        }
        runtime.expired = true;
    }
}

fn emit_anim_runtime_trailer(
    runtime: &AnimRuntime,
    config: &crate::rules::art_data::AnimTypeRuntimeConfig,
    sim: &Simulation,
    events: &mut Option<&mut Vec<AnimRuntimeVisitEvent>>,
) {
    let Some(trailer_type) = &config.trailer_anim else {
        return;
    };
    if !anim_trailer_cadence_matches(sim.session.tick, config.trailer_seperation) {
        return;
    }
    if let Some(events) = events.as_deref_mut() {
        events.push(AnimRuntimeVisitEvent::TrailerSpawn {
            parent_type: runtime.type_name.clone(),
            trailer_type: trailer_type.clone(),
        });
    }
}

fn anim_trailer_cadence_matches(global_frame: u64, separation: i32) -> bool {
    separation == 1 || (global_frame as i32) % separation == 0
}

fn anim_runtime_at_boundary(
    runtime: &AnimRuntime,
    config: &crate::rules::art_data::AnimTypeRuntimeConfig,
    frame_counts: &HashMap<String, u16>,
) -> bool {
    let end = effective_anim_end(config, anim_total_frames(frame_counts, &runtime.type_name));
    let loop_end = effective_anim_loop_end(config, end);
    if runtime.frame_step >= 0 {
        let limit = if runtime.loop_remaining < 2 {
            end
        } else {
            loop_end - config.start
        };
        runtime.current_frame >= limit
    } else {
        let limit = if runtime.loop_remaining < 2 {
            config.start
        } else {
            config.loop_start - config.start
        };
        runtime.current_frame <= limit
    }
}

fn reset_anim_runtime_to_loop_start(
    runtime: &mut AnimRuntime,
    config: &crate::rules::art_data::AnimTypeRuntimeConfig,
    frame_counts: &HashMap<String, u16>,
) {
    if runtime.frame_step >= 0 && !runtime.constructor_reverse && !config.reverse {
        runtime.current_frame = config.loop_start - config.start;
    } else {
        let end = effective_anim_end(config, anim_total_frames(frame_counts, &runtime.type_name));
        runtime.current_frame = effective_anim_loop_end(config, end);
    }
}

fn switch_anim_runtime_type(
    runtime: &mut AnimRuntime,
    next: &str,
    art_reg: &crate::rules::art_data::ArtRegistry,
    frame_counts: &HashMap<String, u16>,
    events: &mut Option<&mut Vec<AnimRuntimeVisitEvent>>,
) {
    let Some(next_config) = art_reg.anim_runtime_config(next) else {
        runtime.expired = true;
        return;
    };
    let previous_type = runtime.type_name.clone();
    let total_frames = anim_total_frames(frame_counts, next);
    let end = effective_anim_end(next_config, total_frames);
    let loop_end = effective_anim_loop_end(next_config, end);
    let reverse = next_config.reverse || runtime.constructor_reverse;
    runtime.type_name = next.to_ascii_uppercase();
    runtime.current_frame = if reverse { loop_end - 1 } else { 0 };
    runtime.frame_step = if reverse { -1 } else { 1 };
    runtime.delay_logic_frames = 0;
    runtime.reload_logic_frames = next_config.rate_logic_frames;
    runtime.rate_elapsed_logic_frames = 0;
    runtime.loop_remaining = native_loop_remaining(next_config.loop_count, 1);
    runtime.first_ai_guard = false;
    runtime.expired = false;
    if let Some(events) = events.as_deref_mut() {
        events.push(AnimRuntimeVisitEvent::NextInPlace {
            previous_type,
            next_type: runtime.type_name.clone(),
        });
    }
}

fn native_loop_remaining(loop_count: i32, constructor_loop: u8) -> u8 {
    let raw = (loop_count as u8).wrapping_mul(constructor_loop.max(1));
    if raw < 2 { 1 } else { raw }
}

fn effective_anim_end(
    config: &crate::rules::art_data::AnimTypeRuntimeConfig,
    total_frames: u16,
) -> i32 {
    if config.end == -1 {
        let frames = i32::from(total_frames);
        if config.shadow { frames / 2 } else { frames }
    } else {
        config.end
    }
}

fn effective_anim_loop_end(
    config: &crate::rules::art_data::AnimTypeRuntimeConfig,
    effective_end: i32,
) -> i32 {
    if config.loop_end == -1 {
        effective_end
    } else {
        config.loop_end
    }
}

fn presentation_anim_frame_count(
    frame_counts: Option<&HashMap<String, u16>>,
    type_name: &str,
) -> Option<u16> {
    let frame_counts = frame_counts?;
    frame_counts.get(type_name).copied().or_else(|| {
        let canonical = type_name.to_ascii_uppercase();
        frame_counts.get(&canonical).copied()
    })
}

fn anim_total_frames(frame_counts: &HashMap<String, u16>, type_name: &str) -> u16 {
    presentation_anim_frame_count(Some(frame_counts), type_name).unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::art_data::{ArtRegistry, DEFAULT_ART_RATE_LOGIC_FRAMES};
    use crate::rules::ini_parser::IniFile;
    use crate::sim::world::Simulation;

    fn building_slot_replacement_art() -> ArtRegistry {
        ArtRegistry::from_ini(&IniFile::from_str(
            "[GAPOWR]\n\
             ActiveAnim=GAPOWR_A\n\
             ActiveAnimDamaged=GAPOWR_AD\n\
             ActiveAnimTwo=UNCHANGED_A\n\
             [GAPOWR_A]\nStart=2\nLoopStart=1\nLoopEnd=8\nLoopCount=-1\nRate=300\n\
             [GAPOWR_AD]\nStart=12\nLoopStart=11\nLoopEnd=18\nLoopCount=-1\nRate=150\n\
             [UNCHANGED_A]\nStart=4\nLoopStart=4\nLoopEnd=9\nLoopCount=-1\nRate=300\n",
        ))
    }

    fn infantry_absorb_slot_art() -> ArtRegistry {
        ArtRegistry::from_ini(&IniFile::from_str(
            "[YAPOWR]\n\
             ActiveAnim=YAPOWR_A\n\
             ActiveAnimDamaged=YAPOWR_AD\n\
             ActiveAnimTwo=YAPOWR_B\n\
             ActiveAnimTwoDamaged=YAPOWR_BD\n\
             ActiveAnimThree=YAPOWR_C\n\
             IdleAnim=YAPOWR_IDLE\n\
             [YAPOWR_A]\nStart=0\nLoopStart=0\nLoopEnd=8\nLoopCount=-1\nRate=300\n\
             [YAPOWR_AD]\nStart=10\nLoopStart=10\nLoopEnd=18\nLoopCount=-1\nRate=150\n\
             [YAPOWR_B]\nStart=0\nLoopStart=0\nLoopEnd=8\nLoopCount=-1\nRate=300\n\
             [YAPOWR_BD]\nStart=10\nLoopStart=10\nLoopEnd=18\nLoopCount=-1\nRate=150\n\
             [YAPOWR_C]\nStart=0\nLoopStart=0\nLoopEnd=8\nLoopCount=-1\nRate=300\n\
             [YAPOWR_IDLE]\nStart=0\nLoopStart=0\nLoopEnd=8\nLoopCount=-1\nRate=300\n",
        ))
    }

    fn infantry_absorb_slot_frame_counts() -> HashMap<String, u16> {
        [
            "YAPOWR_A",
            "YAPOWR_AD",
            "YAPOWR_B",
            "YAPOWR_BD",
            "YAPOWR_C",
            "YAPOWR_IDLE",
        ]
        .into_iter()
        .map(|name| (name.to_string(), 20))
        .collect()
    }

    #[test]
    fn building_slot_damage_replacement_carries_only_relative_current_frame() {
        let art = building_slot_replacement_art();
        let options = crate::sim::game_options::GameOptions::default();
        let frame_counts =
            HashMap::from([("GAPOWR_A".to_string(), 20), ("GAPOWR_AD".to_string(), 20)]);
        let anim = &art.get("GAPOWR").expect("building art").building_anims[0];
        let mut old = fresh_building_slot_runtime("GAPOWR_AD", &art, &options, &frame_counts)
            .expect("damaged runtime");
        // Native absolute frame 15 with Start=12 is CurrentFrame=3.
        old.current_frame = 3;
        old.frame_step = -1;
        old.delay_logic_frames = 7;
        old.rate_elapsed_logic_frames = 2;
        old.loop_remaining = 3;
        old.first_ai_guard = false;
        let mut slot = Some(old);

        assert!(replace_occupied_slot_for_damage_state(
            &mut slot,
            anim,
            false,
            &art,
            &options,
            &frame_counts,
        ));
        let replacement = slot.expect("healthy replacement");
        assert_eq!(replacement.type_name, "GAPOWR_A");
        assert_eq!(replacement.current_frame, 3);
        assert_eq!(replacement.frame_step, 1);
        assert_eq!(replacement.delay_logic_frames, 0);
        assert_eq!(replacement.rate_elapsed_logic_frames, 0);
        assert_eq!(replacement.loop_remaining, u8::MAX);
        assert!(replacement.first_ai_guard);
        assert_eq!(replacement.reload_logic_frames, 3);
    }

    #[test]
    fn building_slot_missing_damaged_descriptor_and_absent_slot_are_untouched() {
        let art = building_slot_replacement_art();
        let options = crate::sim::game_options::GameOptions::default();
        let frame_counts = HashMap::from([("UNCHANGED_A".to_string(), 10)]);
        let anim = &art.get("GAPOWR").expect("building art").building_anims[1];
        let old = fresh_building_slot_runtime("UNCHANGED_A", &art, &options, &frame_counts)
            .expect("base runtime");
        let mut occupied = Some(old.clone());
        let mut absent = None;

        assert!(!replace_occupied_slot_for_damage_state(
            &mut occupied,
            anim,
            true,
            &art,
            &options,
            &frame_counts,
        ));
        assert_eq!(occupied, Some(old));
        assert!(!replace_occupied_slot_for_damage_state(
            &mut absent,
            anim,
            false,
            &art,
            &options,
            &frame_counts,
        ));
        assert_eq!(absent, None);
    }

    #[test]
    fn infantry_absorb_slots_swap_fresh_empty_occupied_empty() {
        let art = infantry_absorb_slot_art();
        let options = crate::sim::game_options::GameOptions::default();
        let frame_counts = infantry_absorb_slot_frame_counts();
        let anims = &art.get("YAPOWR").expect("building art").building_anims;
        let mut slots = vec![None; anims.len()];

        reconcile_looping_building_slot_occupancy(
            &mut slots,
            anims,
            true,
            false,
            false,
            &art,
            &options,
            &frame_counts,
        );
        assert_eq!(slots.len(), 4);
        assert_eq!(
            slots[0].as_ref().map(|slot| slot.type_name.as_str()),
            Some("YAPOWR_A")
        );
        assert!(slots[1].is_none());
        assert_eq!(
            slots[2].as_ref().map(|slot| slot.type_name.as_str()),
            Some("YAPOWR_C")
        );
        assert_eq!(
            slots[3].as_ref().map(|slot| slot.type_name.as_str()),
            Some("YAPOWR_IDLE")
        );

        let old_a = slots[0].as_ref().expect("empty runtime").clone();
        slots[0].as_mut().expect("empty runtime").current_frame = 6;
        slots[0].as_mut().expect("empty runtime").first_ai_guard = false;
        let later_active = slots[2].as_ref().expect("later active").clone();
        let idle = slots[3].as_ref().expect("idle").clone();

        reconcile_looping_building_slot_occupancy(
            &mut slots,
            anims,
            true,
            false,
            true,
            &art,
            &options,
            &frame_counts,
        );
        assert!(slots[0].is_none());
        let b = slots[1].as_ref().expect("occupied runtime");
        assert_eq!(b.type_name, "YAPOWR_B");
        assert_eq!(b.current_frame, 0);
        assert_eq!(b.delay_logic_frames, 0);
        assert_eq!(b.rate_elapsed_logic_frames, 0);
        assert!(b.first_ai_guard);
        assert_eq!(slots[2], Some(later_active.clone()));
        assert_eq!(slots[3], Some(idle.clone()));

        slots[1].as_mut().expect("occupied runtime").current_frame = 5;
        slots[1]
            .as_mut()
            .expect("occupied runtime")
            .rate_elapsed_logic_frames = 2;
        reconcile_looping_building_slot_occupancy(
            &mut slots,
            anims,
            true,
            false,
            false,
            &art,
            &options,
            &frame_counts,
        );
        assert_eq!(slots[0], Some(old_a));
        assert!(slots[1].is_none());
        assert_eq!(slots[2], Some(later_active));
        assert_eq!(slots[3], Some(idle));
    }

    #[test]
    fn infantry_absorb_repair_recreates_only_present_slot() {
        let art = infantry_absorb_slot_art();
        let options = crate::sim::game_options::GameOptions::default();
        let frame_counts = infantry_absorb_slot_frame_counts();
        let anims = &art.get("YAPOWR").expect("building art").building_anims;
        let mut slots = vec![None; anims.len()];

        reconcile_looping_building_slot_occupancy(
            &mut slots,
            anims,
            true,
            true,
            true,
            &art,
            &options,
            &frame_counts,
        );
        assert!(slots[0].is_none());
        let damaged = slots[1].as_mut().expect("occupied damaged runtime");
        assert_eq!(damaged.type_name, "YAPOWR_BD");
        damaged.current_frame = 3;
        damaged.frame_step = -1;
        damaged.delay_logic_frames = 7;
        damaged.rate_elapsed_logic_frames = 2;
        damaged.loop_remaining = 3;
        damaged.first_ai_guard = false;

        for (anim, slot) in anims.iter().zip(&mut slots) {
            let _ = replace_occupied_slot_for_damage_state(
                slot,
                anim,
                false,
                &art,
                &options,
                &frame_counts,
            );
        }
        assert!(slots[0].is_none());
        let healthy = slots[1].as_ref().expect("occupied healthy runtime");
        assert_eq!(healthy.type_name, "YAPOWR_B");
        assert_eq!(healthy.current_frame, 3);
        assert_eq!(healthy.frame_step, 1);
        assert_eq!(healthy.delay_logic_frames, 0);
        assert_eq!(healthy.rate_elapsed_logic_frames, 0);
        assert_eq!(healthy.loop_remaining, u8::MAX);
        assert!(healthy.first_ai_guard);
        let after_repair = healthy.clone();

        reconcile_looping_building_slot_occupancy(
            &mut slots,
            anims,
            true,
            false,
            true,
            &art,
            &options,
            &frame_counts,
        );
        assert!(slots[0].is_none());
        assert_eq!(slots[1], Some(after_repair));
    }

    #[test]
    fn building_slot_runtimes_keep_independent_cadence() {
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[SLOT_FAST]\nStart=0\nLoopStart=0\nLoopEnd=8\nLoopCount=-1\nRate=300\n\
             [SLOT_SLOW]\nStart=0\nLoopStart=0\nLoopEnd=8\nLoopCount=-1\nRate=150\n",
        ));
        let options = crate::sim::game_options::GameOptions::default();
        let frame_counts =
            HashMap::from([("SLOT_FAST".to_string(), 8), ("SLOT_SLOW".to_string(), 8)]);
        let sim = Simulation::new();
        let mut fast = fresh_building_slot_runtime("SLOT_FAST", &art, &options, &frame_counts)
            .expect("fast slot runtime");
        let mut slow = fresh_building_slot_runtime("SLOT_SLOW", &art, &options, &frame_counts)
            .expect("slow slot runtime");

        // First visit consumes each constructor guard; the next three visits
        // complete only the fast slot's independent 900/Rate timer.
        for _ in 0..4 {
            advance_anim_runtime_visit(&mut fast, &sim, &art, &frame_counts);
            advance_anim_runtime_visit(&mut slow, &sim, &art, &frame_counts);
        }

        assert_eq!(fast.current_frame, 1);
        assert_eq!(fast.rate_elapsed_logic_frames, 0);
        assert_eq!(slow.current_frame, 0);
        assert_eq!(slow.rate_elapsed_logic_frames, 3);
    }

    #[test]
    fn garrison_occupant_anim_rate_uses_art_section_rate_logic_frames() {
        let mut sim = Simulation::new();
        let ucflash = sim.interner.intern("UCFLASH");
        let art = ArtRegistry::from_ini(&IniFile::from_str("[UCFLASH]\nRate=300\n"));

        assert_eq!(
            garrison_occupant_anim_rate_logic_frames(&sim, &art, ucflash),
            Some(3)
        );
    }

    #[test]
    fn garrison_occupant_anim_rate_uses_animtype_default_logic_tick_when_rate_missing() {
        let mut sim = Simulation::new();
        let ucflash = sim.interner.intern("UCFLASH");
        let art = ArtRegistry::from_ini(&IniFile::from_str("[UCFLASH]\nFixtureOnly=1\n"));

        assert_eq!(
            garrison_occupant_anim_rate_logic_frames(&sim, &art, ucflash),
            Some(DEFAULT_ART_RATE_LOGIC_FRAMES)
        );
    }

    #[test]
    fn garrison_occupant_anim_rate_requires_art_section() {
        let mut sim = Simulation::new();
        let ucflash = sim.interner.intern("UCFLASH");
        let art = ArtRegistry::empty();

        assert_eq!(
            garrison_occupant_anim_rate_logic_frames(&sim, &art, ucflash),
            None
        );
    }

    #[test]
    fn garrison_muzzle_flash_first_ai_guard_does_not_advance_on_first_fixed_tick() {
        let sim = Simulation::new();
        let frame_counts = HashMap::from([("UCFLASH".to_string(), 3)]);
        let art = ArtRegistry::from_ini(&IniFile::from_str("[UCFLASH]\nEnd=-1\n"));
        let config = art.anim_runtime_config("UCFLASH").unwrap();
        let mut flash = GarrisonMuzzleFlash {
            building_id: 1,
            runtime: garrison_occupant_anim_runtime("UCFLASH", config, 3),
            pixel_x: 0,
            pixel_y: 0,
            screen_x: 0.0,
            screen_y: 0.0,
            rx: 0,
            ry: 0,
            z: 0,
            z_adjust: GARRISON_OCCUPANT_ANIM_Z_ADJUST,
        };

        assert!(advance_garrison_muzzle_flash(
            &mut flash,
            SIM_TICK_MS,
            &sim,
            &art,
            &frame_counts,
        ));
        assert_eq!(flash.runtime.current_frame, 0);
        assert!(!flash.runtime.first_ai_guard);
        assert_eq!(flash.runtime.elapsed_logic_ms, 0);
    }

    #[test]
    fn garrison_muzzle_flash_omitted_end_does_not_play_to_shp_frame_count() {
        let sim = Simulation::new();
        let frame_counts = HashMap::from([("UCFLASH".to_string(), 3)]);
        let art = ArtRegistry::from_ini(&IniFile::from_str("[UCFLASH]\nFixtureOnly=1\n"));
        let config = art.anim_runtime_config("UCFLASH").unwrap();
        let mut flash = GarrisonMuzzleFlash {
            building_id: 1,
            runtime: garrison_occupant_anim_runtime("UCFLASH", config, 3),
            pixel_x: 0,
            pixel_y: 0,
            screen_x: 0.0,
            screen_y: 0.0,
            rx: 0,
            ry: 0,
            z: 0,
            z_adjust: GARRISON_OCCUPANT_ANIM_Z_ADJUST,
        };

        assert!(advance_garrison_muzzle_flash(
            &mut flash,
            SIM_TICK_MS,
            &sim,
            &art,
            &frame_counts,
        ));
        assert!(!advance_garrison_muzzle_flash(
            &mut flash,
            SIM_TICK_MS,
            &sim,
            &art,
            &frame_counts,
        ));
        assert!(flash.runtime.expired);
        assert_eq!(flash.runtime.current_frame, 1);
    }

    #[test]
    fn garrison_muzzle_flash_rate_zero_never_advances() {
        let sim = Simulation::new();
        let frame_counts = HashMap::from([("UCFLASH".to_string(), 3)]);
        let art = ArtRegistry::from_ini(&IniFile::from_str("[UCFLASH]\nEnd=-1\nRate=0\n"));
        let config = art.anim_runtime_config("UCFLASH").unwrap();
        let mut flash = GarrisonMuzzleFlash {
            building_id: 1,
            runtime: garrison_occupant_anim_runtime("UCFLASH", config, 3),
            pixel_x: 0,
            pixel_y: 0,
            screen_x: 0.0,
            screen_y: 0.0,
            rx: 0,
            ry: 0,
            z: 0,
            z_adjust: GARRISON_OCCUPANT_ANIM_Z_ADJUST,
        };

        assert!(advance_garrison_muzzle_flash(
            &mut flash,
            SIM_TICK_MS * 4,
            &sim,
            &art,
            &frame_counts,
        ));
        assert_eq!(flash.runtime.current_frame, 0);
        assert!(!flash.runtime.expired);
    }

    #[test]
    fn garrison_muzzle_flash_loopcount_ff_is_infinite_sentinel() {
        let sim = Simulation::new();
        let frame_counts = HashMap::from([("UCFLASH".to_string(), 3)]);
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[UCFLASH]\nEnd=2\nLoopStart=0\nLoopEnd=2\nLoopCount=-1\n",
        ));
        let config = art.anim_runtime_config("UCFLASH").unwrap();
        let mut flash = GarrisonMuzzleFlash {
            building_id: 1,
            runtime: garrison_occupant_anim_runtime("UCFLASH", config, 3),
            pixel_x: 0,
            pixel_y: 0,
            screen_x: 0.0,
            screen_y: 0.0,
            rx: 0,
            ry: 0,
            z: 0,
            z_adjust: GARRISON_OCCUPANT_ANIM_Z_ADJUST,
        };

        assert!(advance_garrison_muzzle_flash(
            &mut flash,
            SIM_TICK_MS * 3,
            &sim,
            &art,
            &frame_counts,
        ));
        assert_eq!(flash.runtime.loop_remaining, u8::MAX);
        assert_eq!(flash.runtime.current_frame, 0);
        assert!(!flash.runtime.expired);
    }

    #[test]
    fn garrison_muzzle_flash_next_switches_same_runtime() {
        let sim = Simulation::new();
        let frame_counts = HashMap::from([("UCFLASH".to_string(), 2), ("MYNEXT".to_string(), 2)]);
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[UCFLASH]\nEnd=1\nNext=MYNEXT\n[MYNEXT]\nEnd=-1\n",
        ));
        let config = art.anim_runtime_config("UCFLASH").unwrap();
        let mut flash = GarrisonMuzzleFlash {
            building_id: 1,
            runtime: garrison_occupant_anim_runtime("UCFLASH", config, 2),
            pixel_x: 0,
            pixel_y: 0,
            screen_x: 0.0,
            screen_y: 0.0,
            rx: 0,
            ry: 0,
            z: 0,
            z_adjust: GARRISON_OCCUPANT_ANIM_Z_ADJUST,
        };

        assert!(advance_garrison_muzzle_flash(
            &mut flash,
            SIM_TICK_MS * 2,
            &sim,
            &art,
            &frame_counts,
        ));
        assert_eq!(flash.runtime.type_name, "MYNEXT");
        assert_eq!(flash.runtime.current_frame, 0);
        assert!(!flash.runtime.expired);
    }

    #[test]
    fn anim_runtime_trailer_emits_before_first_ai_guard_and_frame_advance() {
        let mut sim = Simulation::new();
        sim.session.tick = 6;
        let frame_counts = HashMap::from([("PARENT".to_string(), 3)]);
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[PARENT]\nEnd=2\nRate=100\nTrailerAnim=SMOKEY2\nTrailerSeperation=2\n",
        ));
        let config = art.anim_runtime_config("PARENT").unwrap();
        let mut runtime = garrison_occupant_anim_runtime("PARENT", config, 3);
        let mut events = Vec::new();

        advance_anim_runtime_visit_with_events(
            &mut runtime,
            &sim,
            &art,
            &frame_counts,
            Some(&mut events),
        );

        assert_eq!(
            events,
            vec![AnimRuntimeVisitEvent::TrailerSpawn {
                parent_type: "PARENT".to_string(),
                trailer_type: "SMOKEY2".to_string(),
            }]
        );
        assert_eq!(runtime.current_frame, 0);
        assert!(!runtime.first_ai_guard);
        assert!(!runtime.expired);
    }

    #[test]
    fn anim_runtime_trailer_cadence_uses_signed_global_frame_modulo() {
        assert!(anim_trailer_cadence_matches(7, 1));
        assert!(anim_trailer_cadence_matches(10, -5));
        assert!(!anim_trailer_cadence_matches(11, -5));
    }

    #[test]
    fn anim_runtime_trailer_uses_old_type_before_next_and_not_new_type_same_visit() {
        let mut sim = Simulation::new();
        sim.session.tick = 8;
        let frame_counts = HashMap::from([("OLDANIM".to_string(), 2), ("NEXTANIM".to_string(), 2)]);
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[OLDANIM]\nEnd=1\nRate=900\nNext=NEXTANIM\nTrailerAnim=OLDTRAIL\nTrailerSeperation=1\n\
             [NEXTANIM]\nEnd=1\nRate=900\nTrailerAnim=NEWTRAIL\nTrailerSeperation=1\n",
        ));
        let config = art.anim_runtime_config("OLDANIM").unwrap();
        let mut runtime = garrison_occupant_anim_runtime("OLDANIM", config, 2);
        runtime.first_ai_guard = false;
        let mut events = Vec::new();

        advance_anim_runtime_visit_with_events(
            &mut runtime,
            &sim,
            &art,
            &frame_counts,
            Some(&mut events),
        );

        assert_eq!(
            events,
            vec![
                AnimRuntimeVisitEvent::TrailerSpawn {
                    parent_type: "OLDANIM".to_string(),
                    trailer_type: "OLDTRAIL".to_string(),
                },
                AnimRuntimeVisitEvent::NextInPlace {
                    previous_type: "OLDANIM".to_string(),
                    next_type: "NEXTANIM".to_string(),
                },
            ]
        );
        assert_eq!(runtime.type_name, "NEXTANIM");
        assert_eq!(runtime.current_frame, 0);
        assert!(!runtime.first_ai_guard);
        assert!(!runtime.expired);
    }

    #[test]
    fn anim_runtime_normal_destroy_does_not_emit_bounce_or_expire_anim_outputs() {
        let mut sim = Simulation::new();
        sim.session.tick = 9;
        let frame_counts = HashMap::from([("BOOM".to_string(), 2)]);
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[BOOM]\nEnd=1\nRate=900\nBounceAnim=BOUNCEFX\nExpireAnim=EXPIREFX\n",
        ));
        let config = art.anim_runtime_config("BOOM").unwrap();
        let mut runtime = garrison_occupant_anim_runtime("BOOM", config, 2);
        runtime.first_ai_guard = false;
        let mut events = Vec::new();

        advance_anim_runtime_visit_with_events(
            &mut runtime,
            &sim,
            &art,
            &frame_counts,
            Some(&mut events),
        );

        assert_eq!(
            events,
            vec![AnimRuntimeVisitEvent::NormalDestroy {
                type_name: "BOOM".to_string(),
            }]
        );
        assert!(runtime.expired);
    }
}
