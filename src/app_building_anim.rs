//! Building animation lifecycle, damage fire overlays, sidebar UI tick, and sound playback.
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
use crate::map::entities::EntityCategory;
use crate::sim::components::{
    AnimOverlayState, AnimRuntime, BuildingAnimOverlays, DamageFireAnim, DamageFireOverlays,
    GarrisonMuzzleFlash,
};
use crate::sim::intern::InternedId;
use crate::sim::production;
use crate::sim::rng::SimRng;
use crate::sim::world::Simulation;

const GARRISON_OCCUPANT_ANIM_Z_ADJUST: i32 = -200;
const DAMAGE_FIRE_SLOT_COUNT: usize = 8;
const DAMAGE_FIRE_HEIGHT_STEP_PX: i32 = 15;
const DAMAGE_FIRE_Z_ADJUST_BIAS: i32 = -10;

/// Advance one-shot building animation overlays stored as ECS components,
/// and the global idle animation timer.
///
/// ActiveAnim plays once (one-shot) when triggered by building placement.
/// When all frames have played, the component is removed and the
/// renderer falls back to frame 0 (idle pose).
/// IdleAnims are handled via a global elapsed timer (always looping).
pub(crate) fn tick_crane_animations(state: &mut AppState, dt_ms: u32) {
    if let Some(sim) = &mut state.simulation {
        // Advance all active building overlay animations using per-anim Rate.
        let keys: Vec<u64> = sim.entities().keys_sorted();
        for &id in &keys {
            let Some(entity) = sim.entities_mut().get_mut(id) else {
                continue;
            };
            let Some(overlays) = entity.building_anim_overlays.as_mut() else {
                continue;
            };
            for anim in overlays.anims.iter_mut() {
                if anim.finished {
                    continue;
                }
                anim.elapsed_ms += dt_ms;
                while anim.elapsed_ms >= anim.rate_ms {
                    anim.elapsed_ms -= anim.rate_ms;
                    anim.frame += 1;
                    if anim.frame >= anim.loop_end {
                        // One-shot: clamp to last frame and mark finished.
                        anim.frame = anim.loop_end.saturating_sub(1);
                        anim.finished = true;
                        break;
                    }
                }
            }
            // Remove finished anims from the vec.
            overlays.anims.retain(|a| !a.finished);
            if overlays.anims.is_empty() {
                entity.building_anim_overlays = None;
            }
        }
    }

    // Advance the global idle animation timer (looping anims: flags, smokestacks, etc.).
    state.idle_anim_elapsed_ms += dt_ms;
}

/// Spawn, remove, and advance DamageFireAnim overlays on buildings.
///
/// Gamemd creates the native damage-fire AnimClass slots when BuildingClass::Update
/// flips its cached damage-fire byte false->true. This app-side bridge keeps that
/// slot lifetime and RNG order as far as the current overlay surface allows.
pub(crate) fn tick_damage_fire_overlays(state: &mut AppState, dt_ms: u32) {
    let Some(rules) = state.rules.as_ref() else {
        return;
    };
    let condition_yellow = rules.general.condition_yellow;
    let condition_red = rules.general.condition_red;
    let fire_types: Vec<(String, u32)> = rules
        .general
        .damage_fire_types
        .iter()
        .map(|f| (f.name.clone(), f.rate_ms))
        .collect();

    if fire_types.is_empty() {
        return;
    }

    let spawn_plans: Vec<DamageFireSpawnPlan> = {
        let Some(sim) = state.simulation.as_ref() else {
            return;
        };
        let Some(art_reg) = state.art_registry.as_ref() else {
            return;
        };

        sim.entities()
            .values()
            .filter_map(|entity| {
                if entity.category != EntityCategory::Structure {
                    return None;
                }
                if entity.health.max == 0 || entity.damage_fire_overlays.is_some() {
                    return None;
                }
                let ratio = entity.health.current as f32 / entity.health.max as f32;
                let threshold = damage_fire_threshold_for_current_surface(
                    condition_yellow,
                    condition_red,
                    None,
                );
                if ratio > threshold {
                    return None;
                }

                let type_ref = sim.interner.resolve(entity.type_ref);
                let rules_obj = rules.object(type_ref);
                let rules_image = rules_obj.map(|obj| obj.image.as_str()).unwrap_or(type_ref);
                let art_entry = art_reg.resolve_metadata_entry(type_ref, rules_image)?;
                let offsets: Vec<(i32, i32)> = art_entry
                    .damage_fire_offsets
                    .iter()
                    .take(DAMAGE_FIRE_SLOT_COUNT)
                    .copied()
                    .collect();
                if offsets.is_empty() {
                    return None;
                }

                let foundation = rules_obj
                    .map(|obj| obj.foundation.as_str())
                    .or(art_entry.foundation.as_deref())
                    .unwrap_or("1x1");
                let (foundation_width, foundation_height) =
                    crate::rules::foundation::foundation_dimensions(foundation);

                Some(DamageFireSpawnPlan {
                    entity_id: entity.stable_id,
                    offsets,
                    foundation_width,
                    foundation_height,
                })
            })
            .collect()
    };

    let sim = match &mut state.simulation {
        Some(s) => s,
        None => return,
    };

    if !spawn_plans.is_empty() {
        let damage_fire_types: Vec<DamageFireTypePlan> = fire_types
            .iter()
            .map(|(name, rate_ms)| {
                let shp_name = sim.interner.intern(name);
                let total_frames = sim.effect_frame_counts.get(&shp_name).copied().unwrap_or(1);
                DamageFireTypePlan {
                    shp_name,
                    total_frames: total_frames.max(1),
                    rate_ms: *rate_ms,
                }
            })
            .collect();

        for plan in spawn_plans {
            let should_spawn = sim
                .entities()
                .get(plan.entity_id)
                .is_some_and(|entity| entity.damage_fire_overlays.is_none());
            if !should_spawn {
                continue;
            }

            let fires = create_damage_fire_slot_anims(
                sim.anim_rng(),
                &damage_fire_types,
                &plan.offsets,
                plan.foundation_width,
                plan.foundation_height,
            );
            if fires.is_empty() {
                continue;
            }
            if let Some(entity) = sim.entities_mut().get_mut(plan.entity_id) {
                if entity.damage_fire_overlays.is_none() {
                    entity.damage_fire_overlays = Some(DamageFireOverlays { fires });
                }
            }
        }
    }

    let keys: Vec<u64> = sim.entities().keys_sorted();
    for &id in &keys {
        let entity = match sim.entities_mut().get_mut(id) {
            Some(e) => e,
            None => continue,
        };
        if entity.category != EntityCategory::Structure || entity.health.max == 0 {
            continue;
        }
        let ratio = entity.health.current as f32 / entity.health.max as f32;
        let threshold =
            damage_fire_threshold_for_current_surface(condition_yellow, condition_red, None);

        if ratio > threshold {
            if entity.damage_fire_overlays.is_some() {
                entity.damage_fire_overlays = None;
            }
        } else if let Some(overlays) = entity.damage_fire_overlays.as_mut() {
            for fire in &mut overlays.fires {
                fire.elapsed_ms += dt_ms;
                while fire.elapsed_ms >= fire.rate_ms && fire.rate_ms > 0 {
                    fire.elapsed_ms -= fire.rate_ms;
                    fire.frame += 1;
                    if fire.frame >= fire.total_frames {
                        fire.frame = 0;
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct DamageFireSpawnPlan {
    entity_id: u64,
    offsets: Vec<(i32, i32)>,
    foundation_width: u16,
    foundation_height: u16,
}

#[derive(Debug, Clone, Copy)]
struct DamageFireTypePlan {
    shp_name: InternedId,
    total_frames: u16,
    rate_ms: u32,
}

fn create_damage_fire_slot_anims(
    rng: &mut SimRng,
    fire_types: &[DamageFireTypePlan],
    offsets: &[(i32, i32)],
    foundation_width: u16,
    foundation_height: u16,
) -> Vec<DamageFireAnim> {
    if fire_types.is_empty() {
        return Vec::new();
    }

    let mut fire_type_index = rng.next_range_u32(fire_types.len() as u32) as usize;
    let mut fires = Vec::with_capacity(offsets.len().min(DAMAGE_FIRE_SLOT_COUNT));
    for (slot, &(pixel_x, pixel_y)) in offsets.iter().take(DAMAGE_FIRE_SLOT_COUNT).enumerate() {
        let fire_type = fire_types[fire_type_index];
        let total_frames = fire_type.total_frames.max(1);
        let frame = rng.next_range_u32(total_frames as u32) as u16;
        fires.push(DamageFireAnim {
            slot: slot as u8,
            shp_name: fire_type.shp_name,
            pixel_x,
            pixel_y,
            frame,
            total_frames,
            rate_ms: fire_type.rate_ms,
            elapsed_ms: 0,
            z_adjust: damage_fire_z_adjust(pixel_y, foundation_width, foundation_height),
        });

        fire_type_index += 1;
        if fire_type_index >= fire_types.len() {
            fire_type_index = 0;
        }
    }
    fires
}

fn damage_fire_z_adjust(offset_y: i32, foundation_width: u16, foundation_height: u16) -> i32 {
    let foundation_sum = i32::from(foundation_width) + i32::from(foundation_height);
    let raw = ((offset_y - foundation_sum * DAMAGE_FIRE_HEIGHT_STEP_PX) * 3 >> 1)
        + DAMAGE_FIRE_Z_ADJUST_BIAS;
    raw.min(0)
}

fn damage_fire_threshold_for_current_surface(
    condition_yellow: f32,
    _condition_red: f32,
    _unresolved_type_0x157b: Option<bool>,
) -> f32 {
    // TODO(parity): expose the raw BuildingType+0x157B byte before selecting
    // ConditionRed. Current Rust has semantic fields with disputed labels, not
    // this verified raw selector, so keep the previous ConditionYellow fallback.
    condition_yellow
}

/// Trigger a one-shot crane animation on the active producer (ConYard) for an owner.
/// Called when a building is placed on the map. Creates/updates a BuildingAnimOverlays
/// ECS component on the producer entity.
pub(crate) fn trigger_crane_anim(state: &mut AppState, owner: &str) {
    // Gather data from immutable borrows first to avoid borrow conflicts.
    let (stable_id, type_id, rules_image) = {
        let (Some(sim), Some(rules)) = (&state.simulation, &state.rules) else {
            return;
        };
        let structure_cat = production::ProductionCategory::Building;
        let producer =
            production::active_producer_for_owner_category(sim, rules, owner, structure_cat);
        let Some(view) = producer else {
            log::info!(
                "trigger_crane_anim: no active Building producer for '{}'",
                owner
            );
            return;
        };
        // Use EntityStore O(1) lookup to find the type_id.
        let Some(ge) = sim.entities().get(view.stable_id) else {
            return;
        };
        let type_str = sim.interner.resolve(ge.type_ref);
        let rules_image: String = rules
            .object(type_str)
            .map(|o| o.image.clone())
            .unwrap_or_else(|| type_str.to_string());
        (view.stable_id, type_str.to_string(), rules_image)
    };

    let Some(art_reg) = &state.art_registry else {
        return;
    };
    let Some(entry) = art_reg.resolve_metadata_entry(&type_id, &rules_image) else {
        return;
    };

    // Collect one-shot anim overlay states to attach.
    let mut new_anims: Vec<AnimOverlayState> = Vec::new();
    for anim in &entry.building_anims {
        if !matches!(
            anim.kind,
            crate::rules::art_data::BuildingAnimKind::Active
                | crate::rules::art_data::BuildingAnimKind::Production
        ) {
            continue;
        }
        // Skip infinite-loop anims (LoopCount=-1) — those loop via idle timer.
        if anim.loop_count < 0 {
            continue;
        }
        // Skip anims with no loop range.
        if anim.loop_end <= anim.loop_start {
            continue;
        }
        let anim_upper: String = anim.anim_type.to_uppercase();
        let loop_end: u16 = anim.loop_end;
        let loop_start: u16 = anim.loop_start;
        let rate: u16 = anim.rate;
        let frame_count: u16 = loop_end - loop_start;

        log::info!(
            "Crane anim triggered: owner='{}' anim='{}' frames={}-{} ({} frames) rate={}ms duration={:.0}ms",
            owner,
            anim_upper,
            loop_start,
            loop_end,
            frame_count,
            rate,
            frame_count as f32 * rate as f32,
        );
        let anim_type_id = state
            .simulation
            .as_mut()
            .map(|s| s.interner.intern(&anim_upper))
            .unwrap_or_default();
        new_anims.push(AnimOverlayState {
            anim_type: anim_type_id,
            frame: anim.start_frame.max(loop_start),
            loop_start,
            loop_end,
            rate_ms: rate as u32,
            elapsed_ms: 0,
            finished: false,
        });
    }

    if new_anims.is_empty() {
        return;
    }

    // Attach or update the BuildingAnimOverlays on the producer entity.
    let Some(sim) = &mut state.simulation else {
        return;
    };
    let Some(ge) = sim.entities_mut().get_mut(stable_id) else {
        return;
    };
    if let Some(overlays) = ge.building_anim_overlays.as_mut() {
        // Merge: add new anims that aren't already playing.
        for new_anim in new_anims {
            let already_playing = overlays
                .anims
                .iter()
                .any(|a| a.anim_type == new_anim.anim_type);
            if !already_playing {
                overlays.anims.push(new_anim);
            }
        }
    } else {
        ge.building_anim_overlays = Some(BuildingAnimOverlays { anims: new_anims });
    }
}

/// Drain `Simulation::bale_events` and apply two visible side-effects:
/// - Trigger the refinery's SpecialAnim (slot 10) one-shot per bale.
/// - Spawn one particle system per non-zero RefinerySmokeOffsetN (up to 4).
pub(crate) fn consume_bale_events(state: &mut AppState) {
    // Collect events + lookups under shared borrows first, then mutate.
    struct PerEvent {
        building_id: u64,
        special_anim: Option<(crate::sim::intern::InternedId, u16, u16, u16, u16)>,
        particle_spawns: Vec<(
            crate::rules::particle_system_type::ParticleSystemTypeId,
            glam::IVec3,
        )>,
    }

    let prepared: Vec<PerEvent> = {
        let (Some(sim), Some(rules), Some(art_reg)) = (
            state.simulation.as_ref(),
            state.rules.as_ref(),
            state.art_registry.as_ref(),
        ) else {
            return;
        };
        if sim.bale_events.is_empty() {
            return;
        }
        let mut out: Vec<PerEvent> = Vec::with_capacity(sim.bale_events.len());
        for ev in &sim.bale_events {
            let Some(building) = sim.entities().get(ev.building_id) else {
                continue;
            };
            let type_str = sim.interner.resolve(building.type_ref);
            let Some(obj) = rules.object(type_str) else {
                continue;
            };
            let rules_image: &str = &obj.image;
            let art_entry = match art_reg.resolve_metadata_entry(type_str, rules_image) {
                Some(e) => e,
                None => continue,
            };

            // Find the SpecialAnim entry — slot 10 in gamemd's anim slot table.
            let special_anim = art_entry.building_anims.iter().find_map(|a| {
                if !matches!(a.kind, crate::rules::art_data::BuildingAnimKind::Special) {
                    return None;
                }
                if a.loop_end <= a.loop_start {
                    return None;
                }
                let upper = a.anim_type.to_uppercase();
                let id = sim.interner.get(&upper)?;
                Some((
                    id,
                    a.loop_start,
                    a.loop_end,
                    a.start_frame.max(a.loop_start),
                    a.rate,
                ))
            });

            // Resolve the particle system type id once. Skip if not configured.
            let mut particle_spawns: Vec<(
                crate::rules::particle_system_type::ParticleSystemTypeId,
                glam::IVec3,
            )> = Vec::new();
            if let Some(name) = obj.refinery_smoke_particle_system.as_deref() {
                if let Some(ps_id) = rules.ps_type_id_by_name(name) {
                    // BuildingClass::GetCoords returns cell CENTER per the
                    // original UndockUnit's (-0x80, +0x80) baseline. The +128
                    // is the lepton offset from cell NW corner to center.
                    let origin_x = building.position.rx as i32 * 256 + 128;
                    let origin_y = building.position.ry as i32 * 256 + 128;
                    for offset in obj.refinery_smoke_offsets.iter() {
                        if *offset == glam::IVec3::ZERO {
                            continue;
                        }
                        particle_spawns.push((
                            ps_id,
                            glam::IVec3::new(origin_x + offset.x, origin_y + offset.y, offset.z),
                        ));
                    }
                }
            }

            out.push(PerEvent {
                building_id: ev.building_id,
                special_anim,
                particle_spawns,
            });
        }
        out
    };

    // Apply side-effects.
    let Some(sim) = state.simulation.as_mut() else {
        return;
    };
    let Some(rules) = state.rules.as_ref() else {
        sim.bale_events.clear();
        return;
    };

    for ev in prepared {
        // 1) Push (or reset) the SpecialAnim in BuildingAnimOverlays.
        if let Some((anim_type, loop_start, loop_end, start_frame, rate)) = ev.special_anim {
            if let Some(building) = sim.entities_mut().get_mut(ev.building_id) {
                let new_state = AnimOverlayState {
                    anim_type,
                    frame: start_frame,
                    loop_start,
                    loop_end,
                    rate_ms: rate as u32,
                    elapsed_ms: 0,
                    finished: false,
                };
                if let Some(overlays) = building.building_anim_overlays.as_mut() {
                    if let Some(existing) =
                        overlays.anims.iter_mut().find(|a| a.anim_type == anim_type)
                    {
                        *existing = new_state;
                    } else {
                        overlays.anims.push(new_state);
                    }
                } else {
                    building.building_anim_overlays = Some(BuildingAnimOverlays {
                        anims: vec![new_state],
                    });
                }
            }
        }

        // 2) Spawn particle systems at each non-zero offset.
        for (ps_id, coords) in ev.particle_spawns {
            sim.spawn_particle_system(
                ps_id,
                coords,
                None,
                Some(ev.building_id),
                coords,
                None,
                rules,
            );
        }
    }

    sim.bale_events.clear();
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
            | GameSoundEvent::CannotDeployHere { .. } => {
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
    if !anim_trailer_cadence_matches(sim.tick, config.trailer_seperation) {
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
    use crate::sim::rng::SimRng;
    use crate::sim::world::Simulation;

    #[test]
    fn damage_fire_slot_creation_uses_rng_start_index_then_wraps() {
        let mut sim = Simulation::new();
        let fire01 = sim.interner.intern("FIRE01");
        let fire02 = sim.interner.intern("FIRE02");
        let fire03 = sim.interner.intern("FIRE03");
        let fire_types = [
            DamageFireTypePlan {
                shp_name: fire01,
                total_frames: 5,
                rate_ms: 450,
            },
            DamageFireTypePlan {
                shp_name: fire02,
                total_frames: 6,
                rate_ms: 450,
            },
            DamageFireTypePlan {
                shp_name: fire03,
                total_frames: 7,
                rate_ms: 450,
            },
        ];
        let offsets = [(-24, -1), (64, 36), (12, 8)];
        let mut rng = SimRng::new(1);
        let mut expected_rng = rng.clone();

        let start = expected_rng.next_range_u32(fire_types.len() as u32) as usize;
        let expected_indices = [start, (start + 1) % 3, (start + 2) % 3];
        let expected_frames = expected_indices
            .map(|idx| expected_rng.next_range_u32(fire_types[idx].total_frames as u32) as u16);

        let fires = create_damage_fire_slot_anims(&mut rng, &fire_types, &offsets, 4, 4);

        assert_eq!(fires.len(), 3);
        for (slot, fire) in fires.iter().enumerate() {
            let expected_type = fire_types[expected_indices[slot]];
            assert_eq!(fire.slot, slot as u8);
            assert_eq!(fire.shp_name, expected_type.shp_name);
            assert_eq!(fire.frame, expected_frames[slot]);
            assert_eq!(fire.total_frames, expected_type.total_frames);
            assert_eq!(fire.rate_ms, expected_type.rate_ms);
        }
        assert_eq!(rng.state(), expected_rng.state());
    }

    #[test]
    fn damage_fire_z_adjust_uses_native_formula_and_clamps_positive() {
        assert_eq!(damage_fire_z_adjust(30, 4, 4), -145);
        assert_eq!(damage_fire_z_adjust(100, 1, 1), 0);
    }

    #[test]
    fn damage_fire_threshold_keeps_yellow_until_raw_selector_is_exposed() {
        assert_eq!(
            damage_fire_threshold_for_current_surface(0.5, 0.25, None),
            0.5
        );
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
        let art = ArtRegistry::from_ini(&IniFile::from_str("[UCFLASH]\n"));

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
        let art = ArtRegistry::from_ini(&IniFile::from_str("[UCFLASH]\n"));
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
        sim.tick = 6;
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
        sim.tick = 8;
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
        sim.tick = 9;
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
