//! App-owned weapon fire presentation: non-garrison muzzle flashes and
//! FLH-positioned weapon report sounds.
//!
//! The sim emits deterministic fire facts. This module resolves rules/art
//! metadata into screen-space visuals and audio cues above the sim boundary.

use std::collections::HashMap;

use crate::app::AppState;
use crate::audio::events::GameSoundEvent;
use crate::map::entities::EntityCategory;
use crate::rules::art_data::{ArtEntry, ArtRegistry};
use crate::rules::ruleset::RuleSet;
use crate::sim::combat::TargetKind;
use crate::sim::combat::combat_weapon::WeaponSlot;
#[cfg(test)]
use crate::sim::components::Position;
use crate::sim::components::WeaponMuzzleFlash;
use crate::sim::world::{SimFireEvent, Simulation};
use crate::util::fixed_math::SimFixed;

const MUZZLE_FLASH_RATE_MS: u32 = 67;
const MIN_PROJECTILE_VISUAL_MS: u32 = 160;
const MAX_PROJECTILE_VISUAL_MS: u32 = 900;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FireOriginBranch {
    Flh,
    BuildingPixelOffset,
    GarrisonPort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FireOriginError {
    MissingArt,
    MissingGarrisonPort,
    BuildingTurretMetadataMissing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FireOrigin {
    pub screen_x: f32,
    pub screen_y: f32,
    pub rx: u16,
    pub ry: u16,
    pub sub_x: SimFixed,
    pub sub_y: SimFixed,
    pub z: u8,
    pub branch: FireOriginBranch,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectileVisual {
    pub shp_name: String,
    pub start_screen_x: f32,
    pub start_screen_y: f32,
    pub end_screen_x: f32,
    pub end_screen_y: f32,
    pub z: u8,
    pub frame: u16,
    pub duration_ms: u32,
    pub elapsed_ms: u32,
}

/// One persistent WaveClass draw resolved from simulation-owned registration state.
#[derive(Debug, Clone)]
pub(crate) struct WeaponWaveVisual {
    pub geometry: crate::render::wave_geometry::WaveGeometryInput,
    pub tint: [f32; 3],
}

impl ProjectileVisual {
    pub(crate) fn progress(&self) -> f32 {
        if self.duration_ms == 0 {
            return 1.0;
        }
        (self.elapsed_ms as f32 / self.duration_ms as f32).clamp(0.0, 1.0)
    }
}

pub(crate) fn select_weapon_muzzle_anim<'a>(anims: &'a [String], facing: u8) -> Option<&'a str> {
    match anims.len() {
        0 => None,
        8 => {
            let idx = ((((facing as u16) << 8) >> 12) + 1) >> 1;
            let idx = ((idx & 7) + 1) & 7;
            anims.get(idx as usize).map(String::as_str)
        }
        _ => anims.first().map(String::as_str),
    }
}

#[cfg(test)]
pub(crate) fn resolve_fire_origin_from_art(
    screen_origin: (f32, f32),
    position: &Position,
    art: &ArtEntry,
    slot: WeaponSlot,
    veterancy: u16,
    facing: u8,
) -> FireOrigin {
    let flh = crate::rules::flh::resolve_flh(
        art.primary_fire_flh,
        art.secondary_fire_flh,
        art.elite_primary_fire_flh,
        art.elite_secondary_fire_flh,
        matches!(slot, WeaponSlot::Primary),
        veterancy,
    );
    let (dx, dy) = crate::util::flh_transform::flh_to_screen_offset_32way(
        flh.forward,
        flh.lateral,
        flh.height,
        facing,
    );
    FireOrigin {
        screen_x: screen_origin.0 + dx,
        screen_y: screen_origin.1 + dy,
        rx: position.rx,
        ry: position.ry,
        sub_x: position.sub_x,
        sub_y: position.sub_y,
        z: position.z,
        branch: FireOriginBranch::Flh,
    }
}

fn snapshot_abs_leptons(ev: &SimFireEvent) -> (i64, i64) {
    (
        ev.origin_snapshot.rx as i64 * 256 + ev.origin_snapshot.sub_x.to_num::<i64>(),
        ev.origin_snapshot.ry as i64 * 256 + ev.origin_snapshot.sub_y.to_num::<i64>(),
    )
}

fn split_abs_leptons(abs_x: i64, abs_y: i64) -> (u16, u16, SimFixed, SimFixed) {
    let rx = abs_x.div_euclid(256).clamp(0, u16::MAX as i64) as u16;
    let ry = abs_y.div_euclid(256).clamp(0, u16::MAX as i64) as u16;
    let sub_x = SimFixed::from_num(abs_x.rem_euclid(256) as i32);
    let sub_y = SimFixed::from_num(abs_y.rem_euclid(256) as i32);
    (rx, ry, sub_x, sub_y)
}

fn fire_origin_from_world_delta(
    ev: &SimFireEvent,
    world_dx: f32,
    world_dy: f32,
    screen_y_lift: i32,
    branch: FireOriginBranch,
) -> FireOrigin {
    let (base_abs_x, base_abs_y) = snapshot_abs_leptons(ev);
    let abs_x = base_abs_x + world_dx.round() as i64;
    let abs_y = base_abs_y + world_dy.round() as i64;
    let (rx, ry, sub_x, sub_y) = split_abs_leptons(abs_x, abs_y);
    let (screen_x, screen_y) =
        crate::util::lepton::lepton_to_screen(rx, ry, sub_x, sub_y, ev.origin_snapshot.z);
    FireOrigin {
        screen_x,
        screen_y: screen_y - screen_y_lift as f32,
        rx,
        ry,
        sub_x,
        sub_y,
        z: ev.origin_snapshot.z,
        branch,
    }
}

fn iso_pixel_to_world_delta(pixel_x: i32, pixel_y: i32) -> (f32, f32) {
    let a = pixel_x as f32 * 256.0 / 30.0;
    let b = pixel_y as f32 * 256.0 / 15.0;
    ((a + b) / 2.0, (b - a) / 2.0)
}

fn resolve_event_art<'a>(
    sim: &Simulation,
    rules: &'a RuleSet,
    art_reg: &'a ArtRegistry,
    ev: &SimFireEvent,
) -> Option<(
    &'a ArtEntry,
    Option<&'a crate::rules::object_type::ObjectType>,
)> {
    let etype_str = sim.interner.resolve(ev.attacker_type_ref);
    let object = rules.object(etype_str);
    let rules_image = object
        .map(|o| o.image.clone())
        .unwrap_or_else(|| etype_str.to_string());
    let art = art_reg.resolve_metadata_entry(etype_str, &rules_image)?;
    Some((art, object))
}

pub(crate) fn resolve_fire_origin_from_sim(
    sim: &Simulation,
    rules: &RuleSet,
    art_reg: &ArtRegistry,
    ev: &SimFireEvent,
) -> Result<FireOrigin, FireOriginError> {
    let (art, object) =
        resolve_event_art(sim, rules, art_reg, ev).ok_or(FireOriginError::MissingArt)?;

    if let Some(muzzle_idx) = ev.garrison_muzzle_index {
        let Some((px, py)) = art.muzzle_flash_positions.get(muzzle_idx as usize).copied() else {
            return Err(FireOriginError::MissingGarrisonPort);
        };
        let (world_dx, world_dy) = iso_pixel_to_world_delta(px, py);
        return Ok(fire_origin_from_world_delta(
            ev,
            world_dx - 128.0,
            world_dy - 128.0,
            0,
            FireOriginBranch::GarrisonPort,
        ));
    }

    if ev.origin_snapshot.category == EntityCategory::Structure {
        let offset = match ev.weapon_slot {
            WeaponSlot::Primary => art.primary_fire_pixel_offset,
            WeaponSlot::Secondary => art.secondary_fire_pixel_offset,
        };
        if let Some((mut px, py)) = offset {
            if matches!(ev.weapon_slot, WeaponSlot::Primary)
                && art.primary_fire_dual_offset
                && ev.origin_snapshot.burst_index % 2 == 1
            {
                px = -px;
            }
            let (world_dx, world_dy) = iso_pixel_to_world_delta(px, py);
            return Ok(fire_origin_from_world_delta(
                ev,
                world_dx - 128.0,
                world_dy - 128.0,
                0,
                FireOriginBranch::BuildingPixelOffset,
            ));
        }
        if object.is_some_and(|obj| obj.has_turret && obj.turret_anim_is_voxel) {
            return Err(FireOriginError::BuildingTurretMetadataMissing);
        }
    }

    let flh = crate::rules::flh::resolve_flh(
        art.primary_fire_flh,
        art.secondary_fire_flh,
        art.elite_primary_fire_flh,
        art.elite_secondary_fire_flh,
        matches!(ev.weapon_slot, WeaponSlot::Primary),
        ev.veterancy,
    );
    let lateral = if ev.origin_snapshot.burst_index % 2 == 1 {
        -flh.lateral
    } else {
        flh.lateral
    };
    let (world_dx, world_dy) =
        crate::util::flh_transform::flh_to_world_offset_32way(flh.forward, lateral, ev.facing);
    Ok(fire_origin_from_world_delta(
        ev,
        world_dx,
        world_dy,
        crate::util::flh_transform::adjust_for_z_leptons(flh.height),
        FireOriginBranch::Flh,
    ))
}

#[allow(dead_code)]
pub(crate) fn resolve_non_garrison_fire_origin(
    state: &AppState,
    ev: &SimFireEvent,
) -> Option<FireOrigin> {
    let sim = state.sim_runtime.as_ref().map(|rt| &rt.simulation)?;
    let rules = state.rules()?;
    let art_reg = state.rules().map(|rules| &rules.art_registry)?;
    resolve_non_garrison_fire_origin_from_sim(sim, rules, art_reg, ev)
}

fn resolve_non_garrison_fire_origin_from_sim(
    sim: &Simulation,
    rules: &RuleSet,
    art_reg: &ArtRegistry,
    ev: &SimFireEvent,
) -> Option<FireOrigin> {
    if ev.garrison_muzzle_index.is_some() {
        return None;
    }
    resolve_fire_origin_from_sim(sim, rules, art_reg, ev).ok()
}

fn build_non_garrison_fire_effects(
    sim: &Simulation,
    rules: &RuleSet,
    art_reg: &ArtRegistry,
    frame_counts: Option<&HashMap<String, u16>>,
    events: &[SimFireEvent],
) -> (Vec<WeaponMuzzleFlash>, Vec<GameSoundEvent>) {
    let mut flashes = Vec::new();
    let mut sounds = Vec::new();

    for ev in events {
        let Ok(origin) = resolve_fire_origin_from_sim(sim, rules, art_reg, ev) else {
            continue;
        };
        if let Some(report_id) = ev.report_sound_id {
            sounds.push(GameSoundEvent::WeaponFired {
                sound_id: sim.interner.resolve(report_id).to_string(),
                screen_pos: Some((origin.screen_x, origin.screen_y)),
            });
        }
        if ev.garrison_muzzle_index.is_some() {
            continue;
        }
        let Some(weapon) = rules.weapon(sim.interner.resolve(ev.weapon_id)) else {
            continue;
        };
        let Some(anim_name) = select_weapon_muzzle_anim(&weapon.anim, ev.facing) else {
            continue;
        };
        let total_frames = presentation_effect_frame_count(frame_counts, anim_name).unwrap_or(1);
        flashes.push(WeaponMuzzleFlash {
            attacker_id: ev.attacker_id,
            shp_name: anim_name.to_string(),
            screen_x: origin.screen_x,
            screen_y: origin.screen_y,
            rx: origin.rx,
            ry: origin.ry,
            z: origin.z,
            frame: 0,
            total_frames,
            rate_ms: MUZZLE_FLASH_RATE_MS,
            elapsed_ms: 0,
        });
    }

    (flashes, sounds)
}

/// Height a shot aimed at a bare ground cell lands on, in **height levels**
/// (signed, the unit `lepton_to_screen` and `FireOrigin::z` speak).
///
/// This derives nothing. It reads `sim::combat::attack_impact_z` — the same
/// single value the detonation damaged at and the same value the impact
/// animation is placed on — and narrows it with the sim's own
/// `impact_z_byte`. The original engine forms one impact coordinate per
/// detonation and hands that one coordinate to both area damage and animation
/// placement, so a second derivation on the presentation side could only agree
/// with the sim by coincidence; when this file derived its own, tracer and
/// explosion sat 60 px apart on structural-bridge cells.
///
/// Two consequences, both deliberate: a level-0 cell still resolves to 0, so
/// flat maps are unchanged; and the structural-bridge deck offset is absent
/// here because it is absent in the sim — that shared residual, and the step
/// that would settle it, are recorded on `attack_impact_z`.
///
/// Missing terrain (no map resolved yet) yields the same zero the sim helper
/// returns; presentation has no better answer before load.
fn cell_target_height_level(sim: &Simulation, rx: u16, ry: u16) -> u8 {
    crate::sim::combat::impact_z_byte(crate::sim::combat::attack_impact_z(
        TargetKind::Cell(rx, ry),
        sim.entities(),
        sim.resolved_terrain.as_ref(),
    ))
}

fn target_fire_destination(sim: &Simulation, target: TargetKind) -> Option<FireOrigin> {
    match target {
        TargetKind::Entity(id) => {
            let entity = sim.entities().get(id)?;
            let (screen_x, screen_y) = crate::render::locomotor_visual::screen_position(entity);
            Some(FireOrigin {
                screen_x,
                screen_y,
                rx: entity.position.rx,
                ry: entity.position.ry,
                sub_x: entity.position.sub_x,
                sub_y: entity.position.sub_y,
                z: entity.position.z,
                branch: FireOriginBranch::Flh,
            })
        }
        TargetKind::Cell(rx, ry) => {
            // The entity arm above reads the target's own height; a bare cell has
            // no object to ask, so the terrain answers for it. Height enters
            // screen Y alone, so getting it wrong drops the impact straight down
            // by a half tile per level with no sideways drift.
            let z = cell_target_height_level(sim, rx, ry);
            let (screen_x, screen_y) = crate::util::lepton::lepton_to_screen(
                rx,
                ry,
                crate::util::lepton::CELL_CENTER_LEPTON,
                crate::util::lepton::CELL_CENTER_LEPTON,
                z,
            );
            Some(FireOrigin {
                screen_x,
                screen_y,
                rx,
                ry,
                sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
                sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
                z,
                branch: FireOriginBranch::Flh,
            })
        }
    }
}

fn projectile_direction_frame(origin: &FireOrigin, dest: &FireOrigin, frame_count: u16) -> u16 {
    if frame_count == 0 {
        return 0;
    }
    let dx = (dest.rx as i32 * 256 + dest.sub_x.to_num::<i32>())
        - (origin.rx as i32 * 256 + origin.sub_x.to_num::<i32>());
    let dy = (dest.ry as i32 * 256 + dest.sub_y.to_num::<i32>())
        - (origin.ry as i32 * 256 + origin.sub_y.to_num::<i32>());
    let facing = crate::sim::movement::facing_from_delta(dx, dy);
    (((facing as u32 * frame_count as u32) / 256) as u16).min(frame_count.saturating_sub(1))
}

fn projectile_duration_ms(origin: &FireOrigin, dest: &FireOrigin, weapon_speed: i32) -> u32 {
    let dx = (dest.rx as f32 * 256.0 + dest.sub_x.to_num::<f32>())
        - (origin.rx as f32 * 256.0 + origin.sub_x.to_num::<f32>());
    let dy = (dest.ry as f32 * 256.0 + dest.sub_y.to_num::<f32>())
        - (origin.ry as f32 * 256.0 + origin.sub_y.to_num::<f32>());
    let distance_cells = ((dx * dx + dy * dy).sqrt() / 256.0).max(1.0);
    let speed = weapon_speed.max(1) as f32;
    ((distance_cells / speed) * 1000.0) as u32
}

fn build_projectile_visuals(
    sim: &Simulation,
    rules: &RuleSet,
    art_reg: &ArtRegistry,
    frame_counts: Option<&HashMap<String, u16>>,
    events: &[SimFireEvent],
) -> Vec<ProjectileVisual> {
    let mut visuals = Vec::new();

    for ev in events {
        let Some(weapon) = rules.weapon(sim.interner.resolve(ev.weapon_id)) else {
            continue;
        };
        let Some(projectile_id) = weapon.projectile.as_deref() else {
            continue;
        };
        let Some(projectile) = rules.projectile(projectile_id) else {
            continue;
        };
        if crate::sim::combat::projectile_uses_authoritative_flight(weapon, rules) {
            // YR BulletClass::AI linkage: persistent shots render from the
            // simulation store, never a parallel app-local interpolation.
            continue;
        }
        if projectile.inviso {
            continue;
        }
        let Some(image) = projectile.image.as_deref() else {
            continue;
        };
        let Ok(origin) = resolve_fire_origin_from_sim(sim, rules, art_reg, ev) else {
            continue;
        };
        let Some(dest) = target_fire_destination(sim, ev.target) else {
            continue;
        };
        let frame_count = presentation_effect_frame_count(frame_counts, image).unwrap_or(32);
        let duration_ms = projectile_duration_ms(&origin, &dest, weapon.speed)
            .clamp(MIN_PROJECTILE_VISUAL_MS, MAX_PROJECTILE_VISUAL_MS);
        visuals.push(ProjectileVisual {
            shp_name: image.to_string(),
            start_screen_x: origin.screen_x,
            start_screen_y: origin.screen_y,
            end_screen_x: dest.screen_x,
            end_screen_y: dest.screen_y,
            z: origin.z.max(dest.z),
            frame: projectile_direction_frame(&origin, &dest, frame_count),
            duration_ms,
            elapsed_ms: 0,
        });
    }

    visuals
}

pub(crate) fn build_weapon_wave_visuals(
    sim: &Simulation,
    observer: Option<crate::sim::intern::InternedId>,
) -> Vec<WeaponWaveVisual> {
    sim.waves
        .iter()
        .filter_map(|(_, wave)| {
            let (kind, wave_type) = match wave.wave_type {
                0 => (
                    crate::render::wave_geometry::WaveGeometryKind::NonMagnetic,
                    0,
                ),
                3 => (crate::render::wave_geometry::WaveGeometryKind::Magnetic, 3),
                // Types 1/2 use the fixed laser rasterizer, which the current
                // white-pixel beam backend does not emulate.
                _ => return None,
            };
            let cell = |point: crate::sim::projectile::ProjectileCoord| {
                let rx = u16::try_from(point.x.div_euclid(256)).ok()?;
                let ry = u16::try_from(point.y.div_euclid(256)).ok()?;
                Some((rx, ry))
            };
            if let Some(observer) = observer {
                let (source_rx, source_ry) = cell(wave.source)?;
                let (target_rx, target_ry) = cell(wave.target)?;
                let source_fogged = !sim.fog.is_cell_revealed(observer, source_rx, source_ry);
                let target_fogged = !sim.fog.is_cell_revealed(observer, target_rx, target_ry);
                if !wave.visible_through_fog(
                    sim.session.game_options.fog_of_war,
                    source_fogged,
                    target_fogged,
                ) {
                    return None;
                }
            }
            Some(WeaponWaveVisual {
                geometry: crate::render::wave_geometry::WaveGeometryInput {
                    kind,
                    wave_type,
                    a: crate::render::wave_geometry::WavePoint {
                        x: wave.source.x,
                        y: wave.source.y,
                        z: wave.source.z,
                    },
                    b: crate::render::wave_geometry::WavePoint {
                        x: wave.target.x,
                        y: wave.target.y,
                        z: wave.target.z,
                    },
                },
                // Sonic and Magnetron sample the destination framebuffer;
                // they never select a house remap. The current sprite batch
                // has no framebuffer-distortion input, so it remains neutral.
                tint: [1.0, 1.0, 1.0],
            })
        })
        .collect()
}

pub(crate) fn spawn_non_garrison_fire_effects(state: &mut AppState, events: &[SimFireEvent]) {
    let (flashes, sounds, projectiles) = {
        let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
            return;
        };
        let Some(rules) = state.rules() else {
            return;
        };
        let Some(art_reg) = state.rules().map(|rules| &rules.art_registry) else {
            return;
        };
        let frame_counts = state
            .match_presentation.sprite_atlas
            .as_ref()
            .map(|atlas| &atlas.active_anim_frame_counts);
        let (flashes, sounds) =
            build_non_garrison_fire_effects(sim, rules, art_reg, frame_counts, events);
        let projectiles = build_projectile_visuals(sim, rules, art_reg, frame_counts, events);
        (flashes, sounds, projectiles)
    };

    state.weapon_muzzle_flashes.extend(flashes);
    state.projectile_visuals.extend(projectiles);
    for sound in sounds {
        state.match_audio.sound_events.push(sound);
    }
}

fn presentation_effect_frame_count(
    frame_counts: Option<&HashMap<String, u16>>,
    type_name: &str,
) -> Option<u16> {
    let frame_counts = frame_counts?;
    frame_counts.get(type_name).copied().or_else(|| {
        let canonical = type_name.to_ascii_uppercase();
        frame_counts.get(&canonical).copied()
    })
}

pub(crate) fn tick_weapon_muzzle_flashes(state: &mut AppState, dt_ms: u32) {
    tick_weapon_muzzle_flash_list(&mut state.weapon_muzzle_flashes, dt_ms);
    tick_projectile_visuals(&mut state.projectile_visuals, dt_ms);
}

fn tick_weapon_muzzle_flash_list(flashes: &mut Vec<WeaponMuzzleFlash>, dt_ms: u32) {
    flashes.retain_mut(|flash| {
        flash.elapsed_ms = flash.elapsed_ms.saturating_add(dt_ms);
        while flash.rate_ms > 0 && flash.elapsed_ms >= flash.rate_ms {
            flash.elapsed_ms -= flash.rate_ms;
            flash.frame = flash.frame.saturating_add(1);
        }
        flash.frame < flash.total_frames
    });
}

fn tick_projectile_visuals(projectiles: &mut Vec<ProjectileVisual>, dt_ms: u32) {
    projectiles.retain_mut(|projectile| {
        projectile.elapsed_ms = projectile.elapsed_ms.saturating_add(dt_ms);
        projectile.elapsed_ms < projectile.duration_ms
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::bridge_facts::{BRIDGE_FLAG_STRUCTURAL, BridgeCellFacts};
    use crate::map::entities::EntityCategory;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::ini_parser::IniFile;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;

    fn weapon_anim_names() -> Vec<String> {
        ["N", "NE", "E", "SE", "S", "SW", "W", "NW"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn selects_none_for_empty_weapon_anim_list() {
        assert_eq!(select_weapon_muzzle_anim(&[], 0), None);
    }

    #[test]
    fn selects_first_for_non_directional_list() {
        let anims = vec!["GUNFIRE".to_string(), "ALT".to_string()];
        assert_eq!(select_weapon_muzzle_anim(&anims, 64), Some("GUNFIRE"));
    }

    #[test]
    fn selects_documented_8way_indices() {
        let anims = weapon_anim_names();
        assert_eq!(select_weapon_muzzle_anim(&anims, 0), Some("NE"));
        assert_eq!(select_weapon_muzzle_anim(&anims, 32), Some("E"));
        assert_eq!(select_weapon_muzzle_anim(&anims, 64), Some("SE"));
        assert_eq!(select_weapon_muzzle_anim(&anims, 128), Some("SW"));
        assert_eq!(select_weapon_muzzle_anim(&anims, 192), Some("NW"));
    }

    #[test]
    fn fire_origin_uses_primary_and_secondary_flh() {
        let art_ini =
            IniFile::from_str("[GI]\nPrimaryFireFLH=80,0,105\nSecondaryFireFLH=80,0,90\n");
        let art = ArtRegistry::from_ini(&art_ini);
        let entry = art.resolve_metadata_entry("GI", "GI").unwrap();
        let position = Position {
            rx: 10,
            ry: 11,
            z: 0,
            exact_z_leptons: None,
            sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
            sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
        };

        let origin = (100.0, 200.0);
        let primary =
            resolve_fire_origin_from_art(origin, &position, entry, WeaponSlot::Primary, 0, 0);
        let secondary =
            resolve_fire_origin_from_art(origin, &position, entry, WeaponSlot::Secondary, 0, 0);
        assert_ne!(primary.screen_y, secondary.screen_y);
        assert_eq!((primary.rx, primary.ry, primary.z), (10, 11, 0));
    }

    fn fire_effect_fixture() -> (Simulation, RuleSet, ArtRegistry, Vec<SimFireEvent>) {
        let rules_ini = IniFile::from_str(
            "\
[InfantryTypes]\n0=E1\n\n\
[VehicleTypes]\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n\n\
[E1]\nStrength=125\nArmor=flak\nSpeed=4\nImage=GI\nPrimary=M60\n\n\
[M60]\nDamage=25\nROF=20\nRange=5\nWarhead=SA\nReport=GIAttack\nAnim=MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,MGUN-W,MGUN-NW\n\n\
[SA]\nVerses=100%,100%,100%,90%,70%,25%,100%,25%,25%,0%,0%\n",
        );
        let rules = RuleSet::from_ini(&rules_ini).unwrap();
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[GI]\nPrimaryFireFLH=80,0,105\nSecondaryFireFLH=80,0,90\n",
        ));
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        let e1 = sim.interner.intern("E1");
        let weapon = sim.interner.intern("M60");
        let report = sim.interner.intern("GIAttack");
        sim.entities_mut()
            .insert(GameEntity::new_at_frame_zero_for_test(
                1,
                10,
                11,
                0,
                0,
                owner,
                Health {
                    current: 125,
                    max: 125,
                },
                e1,
                EntityCategory::Infantry,
                0,
                5,
                false,
            ));
        let events = vec![SimFireEvent {
            attacker_id: 1,
            attacker_type_ref: e1,
            weapon_slot: WeaponSlot::Primary,
            weapon_id: weapon,
            facing: 0,
            veterancy: 0,
            origin_snapshot: crate::sim::world::FireOriginSnapshot {
                rx: 10,
                ry: 11,
                sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
                sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
                z: 0,
                facing: 0,
                category: EntityCategory::Infantry,
                burst_index: 0,
            },
            target: crate::sim::combat::TargetKind::Entity(2),
            report_sound_id: Some(report),
            garrison_muzzle_index: None,
            occupant_anim: None,
        }];
        (sim, rules, art, events)
    }

    #[test]
    fn builds_non_garrison_flash_and_flh_report_sound() {
        let (sim, rules, art, events) = fire_effect_fixture();
        let frame_counts = HashMap::from([("MGUN-NE".to_string(), 4)]);
        let expected_origin =
            resolve_non_garrison_fire_origin_from_sim(&sim, &rules, &art, &events[0]).unwrap();
        let (flashes, sounds) =
            build_non_garrison_fire_effects(&sim, &rules, &art, Some(&frame_counts), &events);

        assert_eq!(flashes.len(), 1);
        assert_eq!(flashes[0].shp_name, "MGUN-NE");
        assert_eq!(flashes[0].total_frames, 4);
        assert_eq!(flashes[0].screen_x, expected_origin.screen_x);
        assert_eq!(sounds.len(), 1);
        match &sounds[0] {
            GameSoundEvent::WeaponFired {
                sound_id,
                screen_pos,
            } => {
                assert_eq!(sound_id, "GIAttack");
                assert_eq!(
                    *screen_pos,
                    Some((expected_origin.screen_x, expected_origin.screen_y))
                );
            }
            other => panic!("unexpected sound event: {other:?}"),
        }
    }

    #[test]
    fn persistent_sonic_wave_produces_geometry() {
        let (mut sim, _rules, _art, _events) = fire_effect_fixture();
        let wave_id = sim.allocate_stable_id();
        sim.admit_wave(
            wave_id,
            crate::sim::wave::Wave::new(
                0,
                crate::sim::projectile::ProjectileCoord::new(10 * 256, 10 * 256, 0),
                crate::sim::projectile::ProjectileCoord::new(14 * 256, 11 * 256, 0),
            ),
        );

        let waves = build_weapon_wave_visuals(&sim, None);
        assert_eq!(waves.len(), 1);
        assert_eq!(waves[0].geometry.wave_type, 0);
        assert_eq!(
            waves[0].geometry.kind,
            crate::render::wave_geometry::WaveGeometryKind::NonMagnetic
        );
        assert_eq!(
            crate::render::wave_geometry::draw_order(waves[0].geometry).len(),
            6
        );
    }

    #[test]
    fn garrison_fire_event_does_not_spawn_non_garrison_effects() {
        let (sim, rules, art, mut events) = fire_effect_fixture();
        events[0].garrison_muzzle_index = Some(0);
        let (flashes, sounds) = build_non_garrison_fire_effects(&sim, &rules, &art, None, &events);
        assert!(flashes.is_empty());
        assert!(sounds.is_empty());
    }

    #[test]
    fn burst_index_flips_lateral_flh_side() {
        let (sim, rules, _art, mut events) = fire_effect_fixture();
        let e1 = events[0].attacker_type_ref;
        let art = ArtRegistry::from_ini(&IniFile::from_str("[GI]\nPrimaryFireFLH=80,24,105\n"));
        events[0].origin_snapshot.burst_index = 0;
        let first = resolve_fire_origin_from_sim(&sim, &rules, &art, &events[0]).unwrap();
        events[0].origin_snapshot.burst_index = 1;
        events[0].attacker_type_ref = e1;
        let second = resolve_fire_origin_from_sim(&sim, &rules, &art, &events[0]).unwrap();

        assert_eq!(first.branch, FireOriginBranch::Flh);
        assert_eq!(second.branch, FireOriginBranch::Flh);
        assert_ne!(first.screen_x, second.screen_x);
    }

    #[test]
    fn building_fire_pixel_offset_resolves_world_origin() {
        let rules = RuleSet::from_ini(&IniFile::from_str(
            "\
[BuildingTypes]\n0=ATESLA\n\n\
[InfantryTypes]\n\n[VehicleTypes]\n\n[AircraftTypes]\n\n\
[ATESLA]\nStrength=600\nArmor=steel\nPrimary=TeslaWeapon\n\n\
[TeslaWeapon]\nDamage=100\nROF=80\nRange=7\nWarhead=TeslaWH\nReport=TeslaAttack\n\n\
[TeslaWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        ))
        .unwrap();
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[ATESLA]\nPrimaryFirePixelOffset=11,-26\n",
        ));
        let mut sim = Simulation::new();
        let atesla = sim.interner.intern("ATESLA");
        let weapon = sim.interner.intern("TeslaWeapon");
        let ev = SimFireEvent {
            attacker_id: 7,
            attacker_type_ref: atesla,
            weapon_slot: WeaponSlot::Primary,
            weapon_id: weapon,
            facing: 0,
            veterancy: 0,
            origin_snapshot: crate::sim::world::FireOriginSnapshot {
                rx: 20,
                ry: 20,
                sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
                sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
                z: 0,
                facing: 0,
                category: EntityCategory::Structure,
                burst_index: 0,
            },
            target: TargetKind::Cell(23, 20),
            report_sound_id: None,
            garrison_muzzle_index: None,
            occupant_anim: None,
        };

        let origin = resolve_fire_origin_from_sim(&sim, &rules, &art, &ev).unwrap();
        assert_eq!(origin.branch, FireOriginBranch::BuildingPixelOffset);
        assert_ne!(
            (origin.rx, origin.ry, origin.sub_x, origin.sub_y),
            (
                ev.origin_snapshot.rx,
                ev.origin_snapshot.ry,
                ev.origin_snapshot.sub_x,
                ev.origin_snapshot.sub_y
            )
        );
    }

    #[test]
    fn garrison_report_sound_uses_muzzle_port_origin() {
        let (sim, rules, _art, mut events) = fire_effect_fixture();
        let art = ArtRegistry::from_ini(&IniFile::from_str("[GI]\nMuzzleFlash0=30,15\n"));
        events[0].garrison_muzzle_index = Some(0);
        let (flashes, sounds) = build_non_garrison_fire_effects(&sim, &rules, &art, None, &events);

        assert!(flashes.is_empty());
        assert_eq!(sounds.len(), 1);
        let origin = resolve_fire_origin_from_sim(&sim, &rules, &art, &events[0]).unwrap();
        match &sounds[0] {
            GameSoundEvent::WeaponFired { screen_pos, .. } => {
                assert_eq!(*screen_pos, Some((origin.screen_x, origin.screen_y)));
            }
            other => panic!("unexpected sound event: {other:?}"),
        }
    }

    #[test]
    fn tick_removes_finished_weapon_muzzle_flash() {
        let mut flashes = vec![WeaponMuzzleFlash {
            attacker_id: 1,
            shp_name: "MGUN-N".to_string(),
            screen_x: 100.0,
            screen_y: 200.0,
            rx: 10,
            ry: 12,
            z: 0,
            frame: 0,
            total_frames: 1,
            rate_ms: MUZZLE_FLASH_RATE_MS,
            elapsed_ms: 0,
        }];
        tick_weapon_muzzle_flash_list(&mut flashes, MUZZLE_FLASH_RATE_MS);
        assert!(flashes.is_empty());
    }

    /// Regression for the reported projectile/impact one-cell visual
    /// discrepancy. Exercise both production projectors with the exact same
    /// native cell-center CoordStruct.
    #[test]
    fn coordinate_runtime_trace_world_effect_anchor_matches_projectile_endpoint() {
        let sim = Simulation::new();

        for (rx, ry) in [(10_u16, 10_u16), (23_u16, 20_u16), (41_u16, 17_u16)] {
            let projectile =
                target_fire_destination(&sim, TargetKind::Cell(rx, ry)).expect("cell target");
            let world_effect = crate::app::presentation::instances::world_effect_screen_position(
                rx,
                ry,
                crate::util::lepton::CELL_CENTER_LEPTON,
                crate::util::lepton::CELL_CENTER_LEPTON,
                0,
            );

            eprintln!(
                "FIRE_TRACE cell=({rx},{ry}) projectile=({:.1},{:.1}) \
                 world_effect=({:.1},{:.1}) delta=({:.1},{:.1})",
                projectile.screen_x,
                projectile.screen_y,
                world_effect.0,
                world_effect.1,
                world_effect.0 - projectile.screen_x,
                world_effect.1 - projectile.screen_y,
            );

            assert_eq!(world_effect.0, projectile.screen_x);
            assert_eq!(
                world_effect.1, projectile.screen_y,
                "WorldEffect must land on its projectile endpoint"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Ground-impact height
    // -----------------------------------------------------------------------

    fn flat_terrain_cell(rx: u16, ry: u16, level: u8) -> ResolvedTerrainCell {
        ResolvedTerrainCell {
            rx,
            ry,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level,
            filled_clear: false,
            tileset_index: Some(0),
            land_type: 0,
            yr_cell_land_type: 0,
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Clear,
            speed_costs: SpeedCostProfile::default(),
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            accepts_smudge: false,
            allows_tiberium: false,
            height_in_pixels: 0,
            variant: 0,
            has_ramp: false,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            terrain_object_occupation: None,
            overlay_blocks: false,
            overlay_zone_type: None,
            outside_playfield: false,
            zone_type: 0,
            base_ground_walk_blocked: false,
            base_build_blocked: false,
            base_land_type: 0,
            base_yr_cell_land_type: 0,
            base_terrain_class: Default::default(),
            base_speed_costs: Default::default(),
            build_blocked: false,
            has_bridge_deck: false,
            bridge_walkable: false,
            bridge_transition: false,
            bridge_deck_level: 0,
            bridge_layer: None,
            bridge_facts: BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    /// A square map whose every cell sits at `level`.
    fn sim_on_ground_at_level(level: u8) -> Simulation {
        const SIZE: u16 = 32;
        let mut cells = Vec::new();
        for ry in 0..SIZE {
            for rx in 0..SIZE {
                cells.push(flat_terrain_cell(rx, ry, level));
            }
        }
        let mut sim = Simulation::new();
        sim.resolved_terrain = Some(ResolvedTerrainGrid::from_cells(SIZE, SIZE, cells));
        sim
    }

    /// The same flat map with one structural-bridge span crossing `(rx, ry)`.
    /// This is the cell the two halves disagreed on.
    fn sim_with_structural_bridge_at(level: u8, rx: u16, ry: u16) -> Simulation {
        let mut sim = sim_on_ground_at_level(level);
        sim.resolved_terrain
            .as_mut()
            .and_then(|grid| grid.cell_mut(rx, ry))
            .expect("bridge cell must be inside the fixture grid")
            .bridge_facts = BridgeCellFacts {
            raw_flags: BRIDGE_FLAG_STRUCTURAL,
            ..BridgeCellFacts::default()
        };
        sim
    }

    /// The impact height the sim resolved for this cell, narrowed exactly as
    /// production narrows it. Tests anchor on this, never on the render half's
    /// own output — an assertion that feeds a projection its own result proves
    /// nothing.
    fn sim_impact_z_byte(sim: &Simulation, rx: u16, ry: u16) -> u8 {
        crate::sim::combat::impact_z_byte(crate::sim::combat::attack_impact_z(
            TargetKind::Cell(rx, ry),
            sim.entities(),
            sim.resolved_terrain.as_ref(),
        ))
    }

    /// Level-0 ground is the case the old hardcoded zero got right, and it must
    /// stay right: resolving the terrain must not shift a flat-map impact.
    ///
    /// Catches a fix that lifts every impact by a constant instead of by the
    /// cell's own height.
    #[test]
    fn ground_impact_on_level_zero_terrain_is_unshifted() {
        let sim = sim_on_ground_at_level(0);

        for (rx, ry) in [(10_u16, 10_u16), (23_u16, 20_u16), (5_u16, 30_u16)] {
            let dest =
                target_fire_destination(&sim, TargetKind::Cell(rx, ry)).expect("cell target");
            let flat = crate::util::lepton::lepton_to_screen(
                rx,
                ry,
                crate::util::lepton::CELL_CENTER_LEPTON,
                crate::util::lepton::CELL_CENTER_LEPTON,
                0,
            );
            assert_eq!(dest.z, 0, "level-0 cell ({rx},{ry}) must resolve height 0");
            assert_eq!((dest.screen_x, dest.screen_y), flat);
        }
    }

    /// The reported bug: a shot landing on raised ground drew a whole tile low,
    /// straight down with no sideways drift, because the cell arm answered 0 for
    /// every terrain height.
    ///
    /// Height enters screen Y alone, one half tile (15 px) per level — so this
    /// pins the exact per-level cadence and pins screen X as untouched, which is
    /// the asymmetry that identified the fault as a height fault rather than a
    /// planar one. Catches both a reintroduced constant height and a wrong
    /// per-level step (e.g. a whole tile, or leptons fed in where levels belong).
    #[test]
    fn ground_impact_lifts_one_half_tile_per_terrain_height_level() {
        const HALF_TILE_PX: f32 = crate::map::terrain::TILE_HEIGHT / 2.0;
        let (rx, ry) = (23_u16, 20_u16);
        let ground_row =
            target_fire_destination(&sim_on_ground_at_level(0), TargetKind::Cell(rx, ry))
                .expect("cell target");

        for level in 0..=6_u8 {
            let dest =
                target_fire_destination(&sim_on_ground_at_level(level), TargetKind::Cell(rx, ry))
                    .expect("cell target");
            assert_eq!(dest.z, level, "cell height must reach the impact point");
            assert_eq!(
                dest.screen_x, ground_row.screen_x,
                "height must not drift the impact sideways at level {level}",
            );
            assert_eq!(
                dest.screen_y,
                ground_row.screen_y - HALF_TILE_PX * f32::from(level),
                "level {level} must lift the impact by {level} half-tiles",
            );
        }
    }

    /// A structural bridge span must not move the tracer endpoint on its own.
    ///
    /// The impact coordinate is the projectile's location clamped to the cell's
    /// ground height; the deck-adding accessor is the *aim* point for a live
    /// object target, a different quantity. Presentation therefore takes the
    /// sim's impact height unchanged, deck or no deck.
    ///
    /// Catches the split this test replaced: the render half calling the
    /// deck-adjusted helper while the sim half returns the bare floor, which
    /// ends the tracer four levels — 60 px, two full tiles — above its own
    /// explosion.
    #[test]
    fn ground_impact_on_structural_bridge_cell_takes_the_sim_impact_height() {
        let (rx, ry) = (12_u16, 9_u16);
        let open_water = sim_on_ground_at_level(0);
        let spanned = sim_with_structural_bridge_at(0, rx, ry);

        let deck =
            target_fire_destination(&spanned, TargetKind::Cell(rx, ry)).expect("cell target");
        let surface =
            target_fire_destination(&open_water, TargetKind::Cell(rx, ry)).expect("cell target");

        assert_eq!(
            deck.z,
            sim_impact_z_byte(&spanned, rx, ry),
            "presentation must carry the sim's impact height, not one of its own",
        );
        assert_eq!(
            (deck.screen_x, deck.screen_y),
            (surface.screen_x, surface.screen_y),
            "a span alone moves neither axis while the deck term is unmodelled \
             on both halves",
        );
        assert_ne!(
            i32::from(deck.z as i8),
            crate::sim::combat::combat_aoe::bridge_adjusted_impact_z(
                spanned.resolved_terrain.as_ref(),
                rx,
                ry,
            ),
            "the aim-point helper is a different quantity; if presentation ever \
             matches it, the deck residual was closed on one half only",
        );
    }

    /// The impact animation and the projectile that produced it must land on the
    /// same pixel — the two projectors are reached by different call paths and
    /// drifted apart before.
    ///
    /// The expected pixel is projected from the **sim's** impact height, so this
    /// is an agreement check between the two halves rather than a projection fed
    /// its own output. The structural-bridge row is the case that fails when
    /// either half derives its own height.
    #[test]
    fn world_effect_anchor_matches_the_sim_impact_height() {
        let cases: [(&str, Simulation); 5] = [
            ("flat", sim_on_ground_at_level(0)),
            ("level 1", sim_on_ground_at_level(1)),
            ("level 2", sim_on_ground_at_level(2)),
            ("level 5", sim_on_ground_at_level(5)),
            (
                "structural bridge over level-0 ground",
                sim_with_structural_bridge_at(0, 10, 10),
            ),
        ];

        for (label, sim) in cases {
            for (rx, ry) in [(10_u16, 10_u16), (23_u16, 20_u16)] {
                let projectile =
                    target_fire_destination(&sim, TargetKind::Cell(rx, ry)).expect("cell target");
                let world_effect = crate::app::presentation::instances::world_effect_screen_position(
                    rx,
                    ry,
                    crate::util::lepton::CELL_CENTER_LEPTON,
                    crate::util::lepton::CELL_CENTER_LEPTON,
                    sim_impact_z_byte(&sim, rx, ry),
                );
                assert_eq!(
                    (world_effect.0, world_effect.1),
                    (projectile.screen_x, projectile.screen_y),
                    "{label}: cell ({rx},{ry}) — the explosion the sim places and \
                     the tracer endpoint the app draws must be one point",
                );
            }
        }
    }
}
