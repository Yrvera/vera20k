//! Building animation lifecycle, sidebar UI tick, and sound playback.
//!
//! These are per-frame runtime updates that run after the sim tick advances.
//! Extracted from app_sim_tick.rs to separate animation/audio/UI concerns from
//! core simulation advancement.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use crate::app::AppState;
use crate::app_commands::preferred_local_owner_name;
use crate::app_types::SIM_TICK_MS;
use crate::sim::components::{AnimRuntime, GarrisonMuzzleFlash};
use crate::sim::production;
use crate::sim::world::Simulation;

const GARRISON_OCCUPANT_ANIM_Z_ADJUST: i32 = -200;

/// Advance the app-owned wall-clock terrain-overlay animation timer.
///
/// Building one-shot overlays now advance inside the authoritative simulation
/// frame; this independent timer only drives looping terrain presentation.
pub(crate) fn tick_terrain_overlay_animations(state: &mut AppState, dt_ms: u32) {
    state.idle_anim_elapsed_ms += dt_ms;
}

pub(crate) use crate::sim::world::building_anim::building_anim_rate_logic_frames;

/// Record the logic frame each structure's slot animations were created on, and
/// drop the record for structures that no longer exist.
///
/// gamemd builds a building's animation slots when the building is placed on the
/// map, and each slot's animation object bases its own frame timer on the frame
/// it was constructed. The looping animation therefore has a per-building phase:
/// two power plants raised a few seconds apart never pulse together. This map is
/// the app-side stand-in for that construction frame — recorded once, the first
/// logic frame the structure is seen.
pub(crate) fn refresh_building_anim_phase_bases(state: &mut AppState) {
    let Some(sim) = &state.simulation else {
        state.building_anim_phase_base.clear();
        return;
    };
    let tick = sim.session.tick;
    let live: Vec<u64> = sim
        .entities()
        .iter_sorted()
        .filter(|(_, entity)| entity.category == crate::map::entities::EntityCategory::Structure)
        .map(|(id, _)| id)
        .collect();
    record_building_anim_phase_bases(&mut state.building_anim_phase_base, &live, tick);
}

/// Insert a phase base for every newly seen structure and forget the ones that
/// are gone.
///
/// An existing entry is never re-stamped: the animation object outlives every
/// intervening frame, so re-basing it would restart the loop and put the whole
/// base back in step.
///
/// `live_structures` must be sorted ascending — `EntityStore::iter_sorted`
/// yields stable ids in that order.
fn record_building_anim_phase_bases(
    bases: &mut std::collections::BTreeMap<u64, u64>,
    live_structures: &[u64],
    tick: u64,
) {
    bases.retain(|id, _| live_structures.binary_search(id).is_ok());
    for id in live_structures {
        bases.entry(*id).or_insert(tick);
    }
}

/// Logic frames elapsed since a building's slot animations were created.
///
/// Falls back to zero for a structure with no recorded base, which renders the
/// animation's first loop frame rather than an arbitrary one.
pub(crate) fn building_anim_elapsed_logic_frames(state: &AppState, stable_id: u64) -> u32 {
    let Some(sim) = &state.simulation else {
        return 0;
    };
    state
        .building_anim_phase_base
        .get(&stable_id)
        .map(|base| sim.session.tick.saturating_sub(*base).min(u32::MAX as u64) as u32)
        .unwrap_or(0)
}

/// Tick the sidebar power bar animation (segment-by-segment transition).
pub(crate) fn update_power_bar_anim(state: &mut AppState) {
    let owner_name = preferred_local_owner_name(state);
    let (power_produced, power_drained) =
        match (&state.simulation, &state.rules, owner_name.as_deref()) {
            (Some(sim), Some(rules), Some(owner)) => {
                production::power_balance_for_owner(sim, rules, owner)
            }
            _ => (0, 0),
        };
    let theoretical = match (&state.simulation, owner_name.as_deref()) {
        (Some(sim), Some(owner)) => production::theoretical_power_for_owner(sim, owner),
        _ => 0,
    };

    // Compute bar height from sidebar layout.
    let spec = state.sidebar_layout_spec;
    let sw = state.render_width() as f32;
    let sh = state.render_height() as f32;
    let layout = crate::sidebar::compute_layout_with_spec(spec, sw, sh, 0);
    let region_bottom = layout.side3_y + spec.side3_height - spec.power_bar_bottom_y;
    let region_top = layout.tabs_y + spec.power_bar_top_y;
    let bar_height_px = (region_bottom - region_top).max(0.0) as i32;

    state.power_bar_anim.set_max_segments(bar_height_px);
    state
        .power_bar_anim
        .update(power_produced, power_drained, theoretical);
    state.power_bar_anim.tick();
}

/// Update radar availability from ECS and tick the radar chrome animation.
pub(crate) fn update_radar_state(state: &mut AppState, dt_ms: f32) {
    let new_has_radar: bool = match (
        &state.simulation,
        &state.rules,
        preferred_local_owner_name(state).as_deref(),
    ) {
        (Some(sim), Some(rules), Some(owner)) => {
            crate::sim::radar::has_radar_for_owner(sim, rules, owner)
        }
        _ => false,
    };
    state.has_radar = new_has_radar;

    if let Some(ref mut ra) = state.radar_anim {
        ra.set_has_radar(new_has_radar);
        ra.tick(&state.gpu, dt_ms);
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

    let events = state.sound_events.drain();
    if events.is_empty() {
        if let Some(sfx) = &mut state.sfx_player {
            sfx.advance_voice_queue();
        }
        return;
    }
    let vp_w = state.render_width() as f32;
    let vp_h = state.render_height() as f32;
    let (Some(sfx), Some(assets)) = (&mut state.sfx_player, &state.asset_manager) else {
        return;
    };
    let cam_x = state.camera_x;
    let cam_y = state.camera_y;
    sfx.advance_voice_queue();

    for event in &events {
        match event {
            // Voice events — always full volume (non-positional), use dedicated voice slot.
            GameSoundEvent::UnitSelected { .. }
            | GameSoundEvent::UnitMoveOrder { .. }
            | GameSoundEvent::UnitAttackOrder { .. } => {
                sfx.play_voice_sound(
                    event.sound_id(),
                    &state.sound_registry,
                    assets,
                    &state.audio_indices,
                );
            }
            // STANDARD EVA cues are fire-and-forget: play only if voice is idle.
            GameSoundEvent::BuildingReady { .. }
            | GameSoundEvent::UnitReady { .. }
            | GameSoundEvent::CannotDeployHere { .. }
            | GameSoundEvent::OutcomeEva { .. } => {
                sfx.play_standard_eva_sound(
                    event.sound_id(),
                    &state.sound_registry,
                    assets,
                    &state.audio_indices,
                );
            }
            // Garrison EVA cues are evamd.ini Type=QUEUE.
            GameSoundEvent::StructureGarrisoned { .. }
            | GameSoundEvent::StructureAbandoned { .. } => {
                sfx.queue_eva_sound(
                    event.sound_id(),
                    &state.sound_registry,
                    assets,
                    &state.audio_indices,
                );
            }
            // UI events — always full volume (non-positional).
            GameSoundEvent::UiSound { .. } => {
                sfx.play_sound(
                    event.sound_id(),
                    &state.sound_registry,
                    assets,
                    &state.audio_indices,
                );
            }
            GameSoundEvent::AnimationStarted {
                anim_id,
                sound_id,
                screen_pos,
            } => {
                let spatial_vol = if let Some((sx, sy)) = screen_pos {
                    let (range, min_vol) = state
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
                        &state.sound_registry,
                        assets,
                        &state.audio_indices,
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
                            &state.sound_registry,
                            assets,
                            &state.audio_indices,
                        );
                    }
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
                            &state.sound_registry,
                            assets,
                            &state.audio_indices,
                        );
                    }
                }
                if let Some(eva_sound_id) = eva_sound_id.as_deref().filter(|s| !s.is_empty()) {
                    sfx.play_standard_eva_sound(
                        eva_sound_id,
                        &state.sound_registry,
                        assets,
                        &state.audio_indices,
                    );
                }
            }
            GameSoundEvent::UnderAttackEva { eva_sound_id } => {
                // Voice-queued (not immediate): under-attack announcements
                // wait behind whatever EVA line is currently speaking.
                let _ = sfx.queue_eva_sound(
                    eva_sound_id,
                    &state.sound_registry,
                    assets,
                    &state.audio_indices,
                );
            }
            // Spatial events — apply distance-based volume scaling using
            // per-sound Range and MinVolume from sound.ini.
            _ => {
                let spatial_vol = if let Some((sx, sy)) = event.screen_pos() {
                    let (range, min_vol) = state
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
                        &state.sound_registry,
                        assets,
                        &state.audio_indices,
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
        let sim = match &state.simulation {
            Some(s) => s,
            None => {
                state.garrison_muzzle_flashes.clear();
                return;
            }
        };
        let art_reg = match &state.art_registry {
            Some(a) => a,
            None => {
                state.garrison_muzzle_flashes.clear();
                return;
            }
        };
        let rules = match &state.rules {
            Some(r) => r,
            None => {
                state.garrison_muzzle_flashes.clear();
                return;
            }
        };
        state
            .pending_fire_effects
            .iter()
            .filter_map(|ev| {
                let anim_name = ev.occupant_anim.as_ref()?;
                let anim_section = sim.interner.resolve(*anim_name).to_ascii_uppercase();
                let origin =
                    crate::app_fire_effects::resolve_fire_origin_from_sim(sim, rules, art_reg, ev)
                        .ok()?;
                let runtime_config = art_reg.anim_runtime_config(&anim_section)?;
                let total_frames = sim.effect_frame_counts.get(anim_name).copied().unwrap_or(1);
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
    state.garrison_muzzle_flashes.extend(new_flashes);

    // Phase 2: advance all flashes and remove finished ones. This is fed from
    // completed fixed sim ticks, not render-frame wall time.
    let (Some(sim), Some(art_reg)) = (&state.simulation, &state.art_registry) else {
        state.garrison_muzzle_flashes.clear();
        return;
    };
    state
        .garrison_muzzle_flashes
        .retain_mut(|flash| advance_garrison_muzzle_flash(flash, dt_ms, sim, art_reg));
}

fn advance_garrison_muzzle_flash(
    flash: &mut GarrisonMuzzleFlash,
    dt_ms: u32,
    sim: &Simulation,
    art_reg: &crate::rules::art_data::ArtRegistry,
) -> bool {
    flash.runtime.elapsed_logic_ms = flash.runtime.elapsed_logic_ms.saturating_add(dt_ms);
    while flash.runtime.elapsed_logic_ms >= SIM_TICK_MS && !flash.runtime.expired {
        flash.runtime.elapsed_logic_ms -= SIM_TICK_MS;
        advance_anim_runtime_visit(&mut flash.runtime, sim, art_reg);
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
) {
    advance_anim_runtime_visit_with_events(runtime, sim, art_reg, None);
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
    if config.ping_pong && anim_runtime_at_boundary(runtime, config, sim) {
        runtime.frame_step = -runtime.frame_step;
        return;
    }
    if !anim_runtime_at_boundary(runtime, config, sim) {
        return;
    }
    if runtime.loop_remaining != 0 && runtime.loop_remaining != u8::MAX {
        runtime.loop_remaining = runtime.loop_remaining.saturating_sub(1);
    }
    if runtime.loop_remaining != 0 {
        reset_anim_runtime_to_loop_start(runtime, config, sim);
        return;
    }
    if let Some(next) = &config.next {
        switch_anim_runtime_type(runtime, next, sim, art_reg, &mut events);
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
    sim: &Simulation,
) -> bool {
    let end = effective_anim_end(config, anim_total_frames(sim, &runtime.type_name));
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
    sim: &Simulation,
) {
    if runtime.frame_step >= 0 && !runtime.constructor_reverse && !config.reverse {
        runtime.current_frame = config.loop_start - config.start;
    } else {
        let end = effective_anim_end(config, anim_total_frames(sim, &runtime.type_name));
        runtime.current_frame = effective_anim_loop_end(config, end);
    }
}

fn switch_anim_runtime_type(
    runtime: &mut AnimRuntime,
    next: &str,
    sim: &Simulation,
    art_reg: &crate::rules::art_data::ArtRegistry,
    events: &mut Option<&mut Vec<AnimRuntimeVisitEvent>>,
) {
    let Some(next_config) = art_reg.anim_runtime_config(next) else {
        runtime.expired = true;
        return;
    };
    let previous_type = runtime.type_name.clone();
    let total_frames = anim_total_frames(sim, next);
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

fn anim_total_frames(sim: &Simulation, type_name: &str) -> u16 {
    sim.interner
        .get(type_name)
        .and_then(|id| sim.effect_frame_counts.get(&id).copied())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::art_data::{ArtRegistry, DEFAULT_ART_RATE_LOGIC_FRAMES};
    use crate::rules::ini_parser::IniFile;
    use crate::sim::world::Simulation;

    #[test]
    fn building_anim_phase_base_is_stamped_once_and_never_rebased() {
        // Two power plants placed 15 logic frames apart keep their own bases for
        // as long as they live, which is what holds their loops out of phase.
        let mut bases = std::collections::BTreeMap::new();

        record_building_anim_phase_bases(&mut bases, &[10], 100);
        record_building_anim_phase_bases(&mut bases, &[10, 11], 115);
        record_building_anim_phase_bases(&mut bases, &[10, 11], 130);

        assert_eq!(bases.get(&10), Some(&100));
        assert_eq!(bases.get(&11), Some(&115));
    }

    #[test]
    fn building_anim_phase_base_is_dropped_when_the_building_dies() {
        let mut bases = std::collections::BTreeMap::new();

        record_building_anim_phase_bases(&mut bases, &[10, 11], 100);
        record_building_anim_phase_bases(&mut bases, &[11], 140);

        assert_eq!(bases.get(&10), None);
        assert_eq!(bases.get(&11), Some(&100));
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
        let art =
            ArtRegistry::from_ini(&IniFile::from_str("[UCFLASH]\nFixtureOnly=1\n"));

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
        let mut sim = Simulation::new();
        let ucflash = sim.interner.intern("UCFLASH");
        sim.effect_frame_counts.insert(ucflash, 3);
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
            &art
        ));
        assert_eq!(flash.runtime.current_frame, 0);
        assert!(!flash.runtime.first_ai_guard);
        assert_eq!(flash.runtime.elapsed_logic_ms, 0);
    }

    #[test]
    fn garrison_muzzle_flash_omitted_end_does_not_play_to_shp_frame_count() {
        let mut sim = Simulation::new();
        let ucflash = sim.interner.intern("UCFLASH");
        sim.effect_frame_counts.insert(ucflash, 3);
        let art =
            ArtRegistry::from_ini(&IniFile::from_str("[UCFLASH]\nFixtureOnly=1\n"));
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
            &art
        ));
        assert!(!advance_garrison_muzzle_flash(
            &mut flash,
            SIM_TICK_MS,
            &sim,
            &art
        ));
        assert!(flash.runtime.expired);
        assert_eq!(flash.runtime.current_frame, 1);
    }

    #[test]
    fn garrison_muzzle_flash_rate_zero_never_advances() {
        let mut sim = Simulation::new();
        let ucflash = sim.interner.intern("UCFLASH");
        sim.effect_frame_counts.insert(ucflash, 3);
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
            &art
        ));
        assert_eq!(flash.runtime.current_frame, 0);
        assert!(!flash.runtime.expired);
    }

    #[test]
    fn garrison_muzzle_flash_loopcount_ff_is_infinite_sentinel() {
        let mut sim = Simulation::new();
        let ucflash = sim.interner.intern("UCFLASH");
        sim.effect_frame_counts.insert(ucflash, 3);
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
            &art
        ));
        assert_eq!(flash.runtime.loop_remaining, u8::MAX);
        assert_eq!(flash.runtime.current_frame, 0);
        assert!(!flash.runtime.expired);
    }

    #[test]
    fn garrison_muzzle_flash_next_switches_same_runtime() {
        let mut sim = Simulation::new();
        let ucflash = sim.interner.intern("UCFLASH");
        let mynext = sim.interner.intern("MYNEXT");
        sim.effect_frame_counts.insert(ucflash, 2);
        sim.effect_frame_counts.insert(mynext, 2);
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
            &art
        ));
        assert_eq!(flash.runtime.type_name, "MYNEXT");
        assert_eq!(flash.runtime.current_frame, 0);
        assert!(!flash.runtime.expired);
    }

    #[test]
    fn anim_runtime_trailer_emits_before_first_ai_guard_and_frame_advance() {
        let mut sim = Simulation::new();
        sim.session.tick = 6;
        let parent = sim.interner.intern("PARENT");
        sim.effect_frame_counts.insert(parent, 3);
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[PARENT]\nEnd=2\nRate=100\nTrailerAnim=SMOKEY2\nTrailerSeperation=2\n",
        ));
        let config = art.anim_runtime_config("PARENT").unwrap();
        let mut runtime = garrison_occupant_anim_runtime("PARENT", config, 3);
        let mut events = Vec::new();

        advance_anim_runtime_visit_with_events(&mut runtime, &sim, &art, Some(&mut events));

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
        let old = sim.interner.intern("OLDANIM");
        let next = sim.interner.intern("NEXTANIM");
        sim.effect_frame_counts.insert(old, 2);
        sim.effect_frame_counts.insert(next, 2);
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[OLDANIM]\nEnd=1\nRate=900\nNext=NEXTANIM\nTrailerAnim=OLDTRAIL\nTrailerSeperation=1\n\
             [NEXTANIM]\nEnd=1\nRate=900\nTrailerAnim=NEWTRAIL\nTrailerSeperation=1\n",
        ));
        let config = art.anim_runtime_config("OLDANIM").unwrap();
        let mut runtime = garrison_occupant_anim_runtime("OLDANIM", config, 2);
        runtime.first_ai_guard = false;
        let mut events = Vec::new();

        advance_anim_runtime_visit_with_events(&mut runtime, &sim, &art, Some(&mut events));

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
        let boom = sim.interner.intern("BOOM");
        sim.effect_frame_counts.insert(boom, 2);
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[BOOM]\nEnd=1\nRate=900\nBounceAnim=BOUNCEFX\nExpireAnim=EXPIREFX\n",
        ));
        let config = art.anim_runtime_config("BOOM").unwrap();
        let mut runtime = garrison_occupant_anim_runtime("BOOM", config, 2);
        runtime.first_ai_guard = false;
        let mut events = Vec::new();

        advance_anim_runtime_visit_with_events(&mut runtime, &sim, &art, Some(&mut events));

        assert_eq!(
            events,
            vec![AnimRuntimeVisitEvent::NormalDestroy {
                type_name: "BOOM".to_string(),
            }]
        );
        assert!(runtime.expired);
    }
}
