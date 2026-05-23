//! App-owned weapon fire presentation: non-garrison muzzle flashes and
//! FLH-positioned weapon report sounds.
//!
//! The sim emits deterministic fire facts. This module resolves rules/art
//! metadata into screen-space visuals and audio cues above the sim boundary.

use crate::app::AppState;
use crate::audio::events::GameSoundEvent;
use crate::map::entities::EntityCategory;
use crate::rules::art_data::{ArtEntry, ArtRegistry};
use crate::rules::ruleset::RuleSet;
use crate::sim::combat::TargetKind;
use crate::sim::combat::combat_weapon::WeaponSlot;
use crate::sim::components::{Position, WeaponMuzzleFlash};
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
    pub start_rx: u16,
    pub start_ry: u16,
    pub end_rx: u16,
    pub end_ry: u16,
    pub z: u8,
    pub frame: u16,
    pub duration_ms: u32,
    pub elapsed_ms: u32,
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

pub(crate) fn resolve_fire_origin_from_art(
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
        screen_x: position.screen_x + dx,
        screen_y: position.screen_y + dy,
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
    let sim = state.simulation.as_ref()?;
    let rules = state.rules.as_ref()?;
    let art_reg = state.art_registry.as_ref()?;
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
        let total_frames = sim
            .interner
            .get(anim_name)
            .and_then(|anim_id| sim.effect_frame_counts.get(&anim_id).copied())
            .unwrap_or(1);
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

fn target_fire_destination(sim: &Simulation, target: TargetKind) -> Option<FireOrigin> {
    match target {
        TargetKind::Entity(id) => {
            let entity = sim.entities.get(id)?;
            Some(FireOrigin {
                screen_x: entity.position.screen_x,
                screen_y: entity.position.screen_y,
                rx: entity.position.rx,
                ry: entity.position.ry,
                sub_x: entity.position.sub_x,
                sub_y: entity.position.sub_y,
                z: entity.position.z,
                branch: FireOriginBranch::Flh,
            })
        }
        TargetKind::Cell(rx, ry) => {
            let (screen_x, screen_y) = crate::util::lepton::lepton_to_screen(
                rx,
                ry,
                crate::util::lepton::CELL_CENTER_LEPTON,
                crate::util::lepton::CELL_CENTER_LEPTON,
                0,
            );
            Some(FireOrigin {
                screen_x,
                screen_y,
                rx,
                ry,
                sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
                sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
                z: 0,
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
        let frame_count = sim
            .interner
            .get(image)
            .and_then(|image_id| sim.effect_frame_counts.get(&image_id).copied())
            .unwrap_or(32);
        let duration_ms = projectile_duration_ms(&origin, &dest, weapon.speed)
            .clamp(MIN_PROJECTILE_VISUAL_MS, MAX_PROJECTILE_VISUAL_MS);
        visuals.push(ProjectileVisual {
            shp_name: image.to_string(),
            start_screen_x: origin.screen_x,
            start_screen_y: origin.screen_y,
            end_screen_x: dest.screen_x,
            end_screen_y: dest.screen_y,
            start_rx: origin.rx,
            start_ry: origin.ry,
            end_rx: dest.rx,
            end_ry: dest.ry,
            z: origin.z.max(dest.z),
            frame: projectile_direction_frame(&origin, &dest, frame_count),
            duration_ms,
            elapsed_ms: 0,
        });
    }

    visuals
}

pub(crate) fn spawn_non_garrison_fire_effects(state: &mut AppState, events: &[SimFireEvent]) {
    let (flashes, sounds, projectiles) = {
        let Some(sim) = state.simulation.as_ref() else {
            return;
        };
        let Some(rules) = state.rules.as_ref() else {
            return;
        };
        let Some(art_reg) = state.art_registry.as_ref() else {
            return;
        };
        let (flashes, sounds) = build_non_garrison_fire_effects(sim, rules, art_reg, events);
        let projectiles = build_projectile_visuals(sim, rules, art_reg, events);
        (flashes, sounds, projectiles)
    };

    state.weapon_muzzle_flashes.extend(flashes);
    state.projectile_visuals.extend(projectiles);
    for sound in sounds {
        state.sound_events.push(sound);
    }
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
    use crate::map::entities::EntityCategory;
    use crate::rules::ini_parser::IniFile;
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
        let mut position = Position {
            rx: 10,
            ry: 11,
            z: 0,
            sub_x: crate::util::lepton::CELL_CENTER_LEPTON,
            sub_y: crate::util::lepton::CELL_CENTER_LEPTON,
            screen_x: 100.0,
            screen_y: 200.0,
        };
        position.refresh_screen_coords();

        let primary = resolve_fire_origin_from_art(&position, entry, WeaponSlot::Primary, 0, 0);
        let secondary = resolve_fire_origin_from_art(&position, entry, WeaponSlot::Secondary, 0, 0);
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
        let anim = sim.interner.intern("MGUN-NE");
        sim.effect_frame_counts.insert(anim, 4);
        sim.entities.insert(GameEntity::new(
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
        let expected_origin =
            resolve_non_garrison_fire_origin_from_sim(&sim, &rules, &art, &events[0]).unwrap();
        let (flashes, sounds) = build_non_garrison_fire_effects(&sim, &rules, &art, &events);

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
    fn garrison_fire_event_does_not_spawn_non_garrison_effects() {
        let (sim, rules, art, mut events) = fire_effect_fixture();
        events[0].garrison_muzzle_index = Some(0);
        let (flashes, sounds) = build_non_garrison_fire_effects(&sim, &rules, &art, &events);
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
        let (flashes, sounds) = build_non_garrison_fire_effects(&sim, &rules, &art, &events);

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
}
