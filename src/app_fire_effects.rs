//! App-owned weapon fire presentation: non-garrison muzzle flashes and
//! FLH-positioned weapon report sounds.
//!
//! The sim emits deterministic fire facts. This module resolves rules/art
//! metadata into screen-space visuals and audio cues above the sim boundary.

use crate::app::AppState;
use crate::audio::events::GameSoundEvent;
use crate::rules::art_data::{ArtEntry, ArtRegistry};
use crate::rules::ruleset::RuleSet;
use crate::sim::combat::combat_weapon::WeaponSlot;
use crate::sim::components::{Position, WeaponMuzzleFlash};
use crate::sim::world::{SimFireEvent, Simulation};

const MUZZLE_FLASH_RATE_MS: u32 = 67;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FireOrigin {
    pub screen_x: f32,
    pub screen_y: f32,
    pub rx: u16,
    pub ry: u16,
    pub z: u8,
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
        z: position.z,
    }
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
    let entity = sim.entities.get(ev.attacker_id)?;
    let etype_str = sim.interner.resolve(ev.attacker_type_ref);
    let rules_image = rules
        .object(etype_str)
        .map(|o| o.image.clone())
        .unwrap_or_else(|| etype_str.to_string());
    let art = art_reg.resolve_metadata_entry(etype_str, &rules_image)?;
    Some(resolve_fire_origin_from_art(
        &entity.position,
        art,
        ev.weapon_slot,
        ev.veterancy,
        ev.facing,
    ))
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
        if ev.garrison_muzzle_index.is_some() {
            continue;
        }
        let Some(origin) = resolve_non_garrison_fire_origin_from_sim(sim, rules, art_reg, ev)
        else {
            continue;
        };
        if let Some(report_id) = ev.report_sound_id {
            sounds.push(GameSoundEvent::WeaponFired {
                sound_id: sim.interner.resolve(report_id).to_string(),
                screen_pos: Some((origin.screen_x, origin.screen_y)),
            });
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

pub(crate) fn spawn_non_garrison_fire_effects(state: &mut AppState, events: &[SimFireEvent]) {
    let (flashes, sounds) = {
        let Some(sim) = state.simulation.as_ref() else {
            return;
        };
        let Some(rules) = state.rules.as_ref() else {
            return;
        };
        let Some(art_reg) = state.art_registry.as_ref() else {
            return;
        };
        build_non_garrison_fire_effects(sim, rules, art_reg, events)
    };

    state.weapon_muzzle_flashes.extend(flashes);
    for sound in sounds {
        state.sound_events.push(sound);
    }
}

pub(crate) fn tick_weapon_muzzle_flashes(state: &mut AppState, dt_ms: u32) {
    tick_weapon_muzzle_flash_list(&mut state.weapon_muzzle_flashes, dt_ms);
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
