//! Authoritative building-overlay animation finalization.
//!
//! Placement, refinery-bale, and tank-bunker events are resolved after the
//! ordinary entity-animation pass and before the frame hash. The app may then
//! render the resulting entity components and particle systems without writing
//! back into `Simulation`.
//!
//! Refinery provenance: `Mission_Deploy_Building @ 0x0073D630` reaches the
//! due dump-gate particle emitter at `0x00459900`; see
//! `docs/research/miner/REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md`. This slice
//! preserves the existing Rust crane and bunker slot projections; their exact
//! native trigger selection remains UNCHECKED here.

use crate::rules::art_data::{ArtRegistry, BuildingAnimKind};
use crate::rules::ruleset::RuleSet;
use crate::sim::components::{AnimOverlayState, BuildingAnimOverlays};
use crate::sim::intern::InternedId;
use crate::sim::production;

use super::Simulation;

/// Finalize the building-animation effects produced by one master frame.
///
/// Keep this order stable: a successful placement arms the producer crane,
/// refinery bales reset their Special animation and spawn smoke, tank-bunker
/// events apply their ordered wall animations, and only then do all active
/// overlays receive this frame's logic visit.
pub(super) fn finalize(
    sim: &mut Simulation,
    placed_building_owners: &[InternedId],
    frame_committed: bool,
    rules: Option<&RuleSet>,
) {
    if !placed_building_owners.is_empty()
        && let Some(rules) = rules
    {
        let art = &rules.art_registry;
        let placement_owners: Vec<String> = placed_building_owners
            .iter()
            .map(|owner| sim.interner.resolve(*owner).to_string())
            .collect();
        for owner in placement_owners {
            trigger_crane_anim(sim, rules, art, &owner);
        }
    }

    if let Some(rules) = rules
        && !rules.art_registry.is_empty()
    {
        consume_bale_events(sim, rules, &rules.art_registry);
        consume_bunker_wall_events(sim, rules, &rules.art_registry);
    }

    tick_overlays(sim, if frame_committed { 1 } else { 0 });
}

/// Per-overlay delay, in logic frames, derived from the animation's own art
/// section and the match game-speed normalization rule.
pub(crate) fn building_anim_rate_logic_frames(
    art: &ArtRegistry,
    anim_type: &str,
    game_options: Option<&crate::sim::game_options::GameOptions>,
) -> u16 {
    let Some(config) = art.anim_runtime_config(anim_type) else {
        return crate::rules::art_data::DEFAULT_ART_RATE_LOGIC_FRAMES;
    };
    // RandomRate= would consume Scenario RNG per instance. No stock building
    // animation declares it, so this deterministic path uses the fixed rate.
    match (config.normalized, game_options) {
        (true, Some(options)) => options.normalized_anim_delay(config.rate_logic_frames),
        _ => config.rate_logic_frames,
    }
}

/// Recreate only the Building animation objects that currently have a Rust
/// runtime occupant. Native walks its fixed slot array and replaces each
/// non-null pointer when the yellow-condition body gate changes; absent slots
/// stay absent and unrelated runtime overlays keep their identity and order.
pub(crate) fn recreate_existing_slots_for_damage_state(
    sim: &mut Simulation,
    rules: &RuleSet,
    stable_id: u64,
) {
    struct Replacement {
        index: usize,
        anim_type: String,
        frame: u16,
        loop_start: u16,
        loop_end: u16,
        rate_logic_frames: u32,
    }

    let Some((type_name, rules_image, damaged, garrisoned, occupied)) =
        sim.substrate.entities.get(stable_id).and_then(|entity| {
            // BuildingClass::ReceiveDamage @ 0x00442230 returns before
            // SetDamagedState @ 0x00451EE0 once ObjectClass is no longer alive.
            // Fatal lifecycle may leave the represented entity resolvable until
            // the deferred-delete drain, so do not reconstruct stale slots.
            if !entity.lifecycle.object_alive || entity.health.current == 0 {
                return None;
            }
            let overlays = entity.building_anim_overlays.as_ref()?;
            let type_name = sim.interner.resolve(entity.type_ref).to_string();
            let rules_image = rules
                .object(&type_name)
                .map(|object| object.image.clone())
                .unwrap_or_else(|| type_name.clone());
            let occupied = overlays
                .anims
                .iter()
                .map(|state| {
                    (
                        sim.interner.resolve(state.anim_type).to_string(),
                        state.frame,
                    )
                })
                .collect::<Vec<_>>();
            Some((
                type_name,
                rules_image,
                entity.building_damage_state_active,
                entity
                    .passenger_role
                    .cargo()
                    .is_some_and(|cargo| !cargo.is_empty()),
                occupied,
            ))
        })
    else {
        return;
    };
    let Some(entry) = rules
        .art_registry
        .resolve_metadata_entry(&type_name, &rules_image)
    else {
        return;
    };

    let mut replacements = Vec::new();
    for (index, (occupied_name, occupied_frame)) in occupied.iter().enumerate() {
        let Some(config) =
            entry.building_anims.iter().find(|config| {
                config.anim_type.eq_ignore_ascii_case(occupied_name)
                    || config.damaged_variant.as_ref().is_some_and(|variant| {
                        variant.anim_type.eq_ignore_ascii_case(occupied_name)
                    })
                    || config.garrisoned_variant.as_ref().is_some_and(|variant| {
                        variant.anim_type.eq_ignore_ascii_case(occupied_name)
                    })
            })
        else {
            continue;
        };
        let old_start = if config.anim_type.eq_ignore_ascii_case(occupied_name) {
            config.start_frame
        } else if let Some(variant) = config
            .damaged_variant
            .as_ref()
            .filter(|variant| variant.anim_type.eq_ignore_ascii_case(occupied_name))
        {
            variant.start_frame
        } else if let Some(variant) = config
            .garrisoned_variant
            .as_ref()
            .filter(|variant| variant.anim_type.eq_ignore_ascii_case(occupied_name))
        {
            variant.start_frame
        } else {
            continue;
        };
        let (anim_type, loop_start, loop_end, start_frame) = if damaged {
            let Some(variant) = config.damaged_variant.as_ref() else {
                // Native's selected descriptor is null: retain this occupant.
                continue;
            };
            (
                variant.anim_type.as_str(),
                variant.loop_start,
                variant.loop_end,
                variant.start_frame,
            )
        } else if let Some(variant) = garrisoned
            .then_some(config.garrisoned_variant.as_ref())
            .flatten()
        {
            (
                variant.anim_type.as_str(),
                variant.loop_start,
                variant.loop_end,
                variant.start_frame,
            )
        } else {
            (
                config.anim_type.as_str(),
                config.loop_start,
                config.loop_end,
                config.start_frame,
            )
        };
        if anim_type.is_empty() {
            continue;
        }
        replacements.push(Replacement {
            index,
            anim_type: anim_type.to_ascii_uppercase(),
            frame: (i32::from(start_frame)
                .saturating_add(i32::from(*occupied_frame) - i32::from(old_start)))
            .clamp(0, i32::from(u16::MAX)) as u16,
            loop_start,
            loop_end,
            rate_logic_frames: u32::from(building_anim_rate_logic_frames(
                &rules.art_registry,
                anim_type,
                Some(&sim.session.game_options),
            )),
        });
    }

    for replacement in replacements {
        let anim_type = sim.interner.intern(&replacement.anim_type);
        let Some(state) = sim
            .substrate
            .entities
            .get_mut(stable_id)
            .and_then(|entity| entity.building_anim_overlays.as_mut())
            .and_then(|overlays| overlays.anims.get_mut(replacement.index))
        else {
            continue;
        };
        *state = AnimOverlayState {
            anim_type,
            frame: replacement.frame,
            loop_start: replacement.loop_start,
            loop_end: replacement.loop_end,
            rate_logic_frames: replacement.rate_logic_frames,
            elapsed_logic_frames: 0,
            finished: false,
        };
    }
}

fn tick_overlays(sim: &mut Simulation, dt_logic_frames: u32) {
    let keys = sim.entities().keys_sorted();
    for id in keys {
        let Some(entity) = sim.entities_mut().get_mut(id) else {
            continue;
        };
        let Some(overlays) = entity.building_anim_overlays.as_mut() else {
            continue;
        };
        for anim in &mut overlays.anims {
            if anim.finished || anim.rate_logic_frames == 0 {
                continue;
            }
            anim.elapsed_logic_frames += dt_logic_frames;
            while anim.elapsed_logic_frames >= anim.rate_logic_frames {
                anim.elapsed_logic_frames -= anim.rate_logic_frames;
                anim.frame += 1;
                if anim.frame >= anim.loop_end {
                    anim.frame = anim.loop_end.saturating_sub(1);
                    anim.finished = true;
                    break;
                }
            }
        }
        overlays.anims.retain(|anim| !anim.finished);
        if overlays.anims.is_empty() {
            entity.building_anim_overlays = None;
        }
    }
}

fn trigger_crane_anim(sim: &mut Simulation, rules: &RuleSet, art: &ArtRegistry, owner: &str) {
    let (stable_id, type_id, rules_image) = {
        let producer = production::active_producer_for_owner_category(
            sim,
            rules,
            owner,
            production::ProductionCategory::Building,
        );
        let Some(producer) = producer else {
            log::info!("trigger_crane_anim: no active Building producer for '{owner}'");
            return;
        };
        let Some(entity) = sim.entities().get(producer.stable_id) else {
            return;
        };
        let type_id = sim.interner.resolve(entity.type_ref).to_string();
        let rules_image = rules
            .object(&type_id)
            .map(|object| object.image.clone())
            .unwrap_or_else(|| type_id.clone());
        (producer.stable_id, type_id, rules_image)
    };

    let Some(entry) = art.resolve_metadata_entry(&type_id, &rules_image) else {
        return;
    };
    let game_options = sim.session.game_options.clone();
    let mut new_anims = Vec::new();
    for anim in &entry.building_anims {
        if !matches!(
            anim.kind,
            BuildingAnimKind::Active | BuildingAnimKind::Production
        ) || anim.loop_count < 0
            || anim.loop_end <= anim.loop_start
        {
            continue;
        }
        let anim_upper = anim.anim_type.to_uppercase();
        let rate = building_anim_rate_logic_frames(art, &anim.anim_type, Some(&game_options));
        log::info!(
            "Crane anim triggered: owner='{owner}' anim='{anim_upper}' frames={}-{} ({} frames) rate={} logic frames",
            anim.loop_start,
            anim.loop_end,
            anim.loop_end - anim.loop_start,
            rate,
        );
        new_anims.push(AnimOverlayState {
            anim_type: sim.interner.intern(&anim_upper),
            frame: anim.start_frame.max(anim.loop_start),
            loop_start: anim.loop_start,
            loop_end: anim.loop_end,
            rate_logic_frames: u32::from(rate),
            elapsed_logic_frames: 0,
            finished: false,
        });
    }
    if new_anims.is_empty() {
        return;
    }

    let Some(entity) = sim.entities_mut().get_mut(stable_id) else {
        return;
    };
    if let Some(overlays) = entity.building_anim_overlays.as_mut() {
        for new_anim in new_anims {
            if !overlays
                .anims
                .iter()
                .any(|active| active.anim_type == new_anim.anim_type)
            {
                overlays.anims.push(new_anim);
            }
        }
    } else {
        entity.building_anim_overlays = Some(BuildingAnimOverlays { anims: new_anims });
    }
}

fn consume_bale_events(sim: &mut Simulation, rules: &RuleSet, art: &ArtRegistry) {
    if sim.bale_events.is_empty() {
        return;
    }

    struct PreparedBale {
        building_id: u64,
        special_anim: Option<(String, u16, u16, u16, u16)>,
        particle_spawns: Vec<(
            crate::rules::particle_system_type::ParticleSystemTypeId,
            glam::IVec3,
        )>,
    }

    let prepared = {
        let mut prepared = Vec::with_capacity(sim.bale_events.len());
        for event in &sim.bale_events {
            let Some(building) = sim.entities().get(event.building_id) else {
                continue;
            };
            let type_name = sim.interner.resolve(building.type_ref);
            let Some(object) = rules.object(type_name) else {
                continue;
            };
            let Some(art_entry) = art.resolve_metadata_entry(type_name, &object.image) else {
                continue;
            };

            let special_anim = art_entry.building_anims.iter().find_map(|anim| {
                if !matches!(anim.kind, BuildingAnimKind::Special)
                    || anim.loop_end <= anim.loop_start
                {
                    return None;
                }
                Some((
                    anim.anim_type.to_uppercase(),
                    anim.loop_start,
                    anim.loop_end,
                    anim.start_frame.max(anim.loop_start),
                    building_anim_rate_logic_frames(
                        art,
                        &anim.anim_type,
                        Some(&sim.session.game_options),
                    ),
                ))
            });

            let mut particle_spawns = Vec::new();
            if let Some(name) = object.refinery_smoke_particle_system.as_deref()
                && let Some(particle_type) = rules.ps_type_id_by_name(name)
            {
                let origin_x = i32::from(building.position.rx) * 256 + 128;
                let origin_y = i32::from(building.position.ry) * 256 + 128;
                for offset in &object.refinery_smoke_offsets {
                    if *offset != glam::IVec3::ZERO {
                        particle_spawns.push((
                            particle_type,
                            glam::IVec3::new(origin_x + offset.x, origin_y + offset.y, offset.z),
                        ));
                    }
                }
            }
            prepared.push(PreparedBale {
                building_id: event.building_id,
                special_anim,
                particle_spawns,
            });
        }
        prepared
    };

    for event in prepared {
        if let Some((anim_name, loop_start, loop_end, start_frame, rate)) = event.special_anim {
            let anim_type = sim.interner.intern(&anim_name);
            let new_state = AnimOverlayState {
                anim_type,
                frame: start_frame,
                loop_start,
                loop_end,
                rate_logic_frames: u32::from(rate),
                elapsed_logic_frames: 0,
                finished: false,
            };
            if let Some(building) = sim.entities_mut().get_mut(event.building_id) {
                if let Some(overlays) = building.building_anim_overlays.as_mut() {
                    if let Some(existing) = overlays
                        .anims
                        .iter_mut()
                        .find(|active| active.anim_type == anim_type)
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

        for (particle_type, coords) in event.particle_spawns {
            sim.spawn_particle_system(
                particle_type,
                coords,
                None,
                Some(event.building_id),
                coords,
                None,
                rules,
            );
        }
    }
    sim.bale_events.clear();
}

struct PreparedAnimOverlay {
    anim_type: String,
    frame: u16,
    loop_start: u16,
    loop_end: u16,
    rate_logic_frames: u32,
}

fn prepare_bunker_special_overlay(
    sim: &Simulation,
    art: &ArtRegistry,
    config: &crate::rules::art_data::BuildingAnimConfig,
    damaged: bool,
) -> Option<PreparedAnimOverlay> {
    let (anim_type, loop_start, loop_end, start_frame) = match (damaged, &config.damaged_variant) {
        (true, Some(variant)) => (
            variant.anim_type.as_str(),
            variant.loop_start,
            variant.loop_end,
            variant.start_frame.max(variant.loop_start),
        ),
        _ => (
            config.anim_type.as_str(),
            config.loop_start,
            config.loop_end,
            config.start_frame.max(config.loop_start),
        ),
    };
    if loop_end <= loop_start {
        return None;
    }
    let rate = building_anim_rate_logic_frames(art, anim_type, Some(&sim.session.game_options));
    Some(PreparedAnimOverlay {
        anim_type: anim_type.to_uppercase(),
        frame: start_frame,
        loop_start,
        loop_end,
        rate_logic_frames: u32::from(rate),
    })
}

fn consume_bunker_wall_events(sim: &mut Simulation, rules: &RuleSet, art: &ArtRegistry) {
    if sim.bunker_wall_events.is_empty() {
        return;
    }

    struct PreparedBunker {
        building_id: u64,
        clear_anim_types: Vec<String>,
        new_states: Vec<PreparedAnimOverlay>,
    }

    let prepared = {
        let mut prepared = Vec::with_capacity(sim.bunker_wall_events.len());
        for event in &sim.bunker_wall_events {
            let Some(building) = sim.entities().get(event.building_id) else {
                continue;
            };
            let type_name = sim.interner.resolve(building.type_ref);
            let Some(object) = rules.object(type_name) else {
                continue;
            };
            let Some(art_entry) = art.resolve_metadata_entry(type_name, &object.image) else {
                continue;
            };
            let specials: Vec<&crate::rules::art_data::BuildingAnimConfig> = art_entry
                .building_anims
                .iter()
                .filter(|anim| matches!(anim.kind, BuildingAnimKind::Special))
                .collect();
            let (pick, clear): (&[usize], &[usize]) = if event.up {
                (&[0, 1], &[])
            } else {
                (&[2, 3], &[0, 1])
            };
            let new_states = pick
                .iter()
                .filter_map(|index| specials.get(*index))
                .filter_map(|config| {
                    prepare_bunker_special_overlay(sim, art, config, event.damaged)
                })
                .collect();
            let clear_anim_types = clear
                .iter()
                .filter_map(|index| specials.get(*index))
                .flat_map(|config| {
                    let mut names = vec![config.anim_type.to_uppercase()];
                    if let Some(variant) = &config.damaged_variant {
                        names.push(variant.anim_type.to_uppercase());
                    }
                    names
                })
                .collect();
            prepared.push(PreparedBunker {
                building_id: event.building_id,
                clear_anim_types,
                new_states,
            });
        }
        prepared
    };

    for event in prepared {
        let clear_anim_types: Vec<_> = event
            .clear_anim_types
            .iter()
            .map(|name| sim.interner.intern(name))
            .collect();
        let new_states: Vec<_> = event
            .new_states
            .into_iter()
            .map(|state| AnimOverlayState {
                anim_type: sim.interner.intern(&state.anim_type),
                frame: state.frame,
                loop_start: state.loop_start,
                loop_end: state.loop_end,
                rate_logic_frames: state.rate_logic_frames,
                elapsed_logic_frames: 0,
                finished: false,
            })
            .collect();
        let Some(building) = sim.entities_mut().get_mut(event.building_id) else {
            continue;
        };
        if !clear_anim_types.is_empty()
            && let Some(overlays) = building.building_anim_overlays.as_mut()
        {
            overlays
                .anims
                .retain(|active| !clear_anim_types.contains(&active.anim_type));
        }
        for new_state in new_states {
            if let Some(overlays) = building.building_anim_overlays.as_mut() {
                if let Some(existing) = overlays
                    .anims
                    .iter_mut()
                    .find(|active| active.anim_type == new_state.anim_type)
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
    sim.bunker_wall_events.clear();
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::map::entities::EntityCategory;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::components::{BaleDepositEvent, Health};
    use crate::sim::game_entity::GameEntity;

    fn insert_building(sim: &mut Simulation, stable_id: u64, type_name: &str, rx: u16, ry: u16) {
        let owner = sim.interner.intern("Americans");
        let type_ref = sim.interner.intern(type_name);
        let entity = GameEntity::new_at_frame_zero_for_test(
            stable_id,
            rx,
            ry,
            0,
            0,
            owner,
            Health {
                current: 100,
                max: 100,
            },
            type_ref,
            EntityCategory::Structure,
            0,
            5,
            true,
        );
        sim.entities_mut().insert(entity);
    }

    #[test]
    fn finalize_committed_overlay_progress_is_hashed_and_terminal_frame_does_not_advance() {
        let mut sim = Simulation::new();
        let anim_type = sim.interner.intern("TEST_SPECIAL");
        insert_building(&mut sim, 17, "TEST_BUILDING", 4, 5);
        sim.entities_mut()
            .get_mut(17)
            .expect("test building")
            .building_anim_overlays = Some(BuildingAnimOverlays {
            anims: vec![AnimOverlayState {
                anim_type,
                frame: 2,
                loop_start: 2,
                loop_end: 4,
                rate_logic_frames: 1,
                elapsed_logic_frames: 0,
                finished: false,
            }],
        });

        let initial_hash = sim.state_hash();
        finalize(&mut sim, &[], false, None);
        let terminal_overlay = &sim
            .entities()
            .get(17)
            .expect("test building")
            .building_anim_overlays
            .as_ref()
            .expect("terminal frame keeps overlay")
            .anims[0];
        assert_eq!(terminal_overlay.frame, 2);
        assert_eq!(terminal_overlay.elapsed_logic_frames, 0);
        assert_eq!(sim.state_hash(), initial_hash);

        finalize(&mut sim, &[], true, None);
        let advanced_hash = sim.state_hash();
        let advanced_overlay = &sim
            .entities()
            .get(17)
            .expect("test building")
            .building_anim_overlays
            .as_ref()
            .expect("one frame remains")
            .anims[0];
        assert_eq!(advanced_overlay.frame, 3);
        assert_eq!(advanced_overlay.elapsed_logic_frames, 0);
        assert_ne!(advanced_hash, initial_hash);

        finalize(&mut sim, &[], true, None);
        assert!(
            sim.entities()
                .get(17)
                .expect("test building")
                .building_anim_overlays
                .is_none(),
            "the overlay component disappears when its final frame completes"
        );
        assert_ne!(sim.state_hash(), advanced_hash);
    }

    fn refinery_rules_and_art() -> RuleSet {
        let mut rules = RuleSet::from_ini(&IniFile::from_str(
            "[BuildingTypes]\n\
             0=GAREFN\n\
             1=GAWALL\n\
             [Particles]\n\
             0=RefSmokeParticle\n\
             [ParticleSystems]\n\
             0=RefSmokeSystem\n\
             [GAREFN]\n\
             Image=GAREFN\n\
             RefinerySmokeParticleSystem=RefSmokeSystem\n\
             RefinerySmokeOffsetOne=10,-20,30\n\
             [GAWALL]\n\
             Wall=yes\n\
             [RefSmokeParticle]\n\
             BehavesLike=Smoke\n\
             MaxEC=10\n\
             MaxDC=4\n\
             StartStateAI=0\n\
             EndStateAI=10\n\
             StateAIAdvance=4\n\
             [RefSmokeSystem]\n\
             BehavesLike=Smoke\n\
             HoldsWhat=RefSmokeParticle\n\
             Spawns=yes\n\
             ParticleCap=10\n\
             SpawnFrames=1\n\
             Lifetime=200\n",
        ))
        .expect("refinery animation rules");
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[GAREFN]\n\
             SpecialAnim=GAREFN_B\n\
             [GAREFN_B]\n\
             Start=2\n\
             LoopStart=1\n\
             LoopEnd=5\n\
             Rate=300\n",
        ));
        rules.merge_art_data(&art);
        rules
    }

    #[test]
    fn damage_gate_recreates_only_occupied_retail_slot_descriptors_in_place() {
        let mut rules = RuleSet::from_ini(&IniFile::from_str(
            "[BuildingTypes]\n0=YAGNTC\n[YAGNTC]\nStrength=1000\nCost=2500\nArmor=concrete\n",
        ))
        .expect("YAGNTC rules");
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[YAGNTC]\n\
             SuperAnim=YAGNTC_E\nSuperAnimDamaged=YAGNTC_ED\n\
             SuperAnimTwo=YAGNTC_F\nSuperAnimTwoDamaged=YAGNTC_FD\n\
             SuperAnimThree=YAGNTC_G\nSuperAnimThreeDamaged=YAGNTC_GD\n\
             SuperAnimFour=YAGNTC_H\nSuperAnimFourDamaged=YAGNTC_HD\n\
             SuperLowPower=YAGNTC_P\nSuperLowPowerDamaged=YAGNTC_PD\n\
             [YAGNTC_F]\nStart=2\nLoopStart=1\nLoopEnd=8\nRate=300\n\
             [YAGNTC_FD]\nStart=12\nLoopStart=11\nLoopEnd=18\nRate=150\n\
             [YAGNTC_G]\nStart=3\nLoopStart=1\nLoopEnd=9\nRate=300\n\
             [YAGNTC_GD]\nStart=13\nLoopStart=11\nLoopEnd=19\nRate=150\n\
             [YAGNTC_H]\nStart=4\nLoopStart=1\nLoopEnd=10\nRate=300\n\
             [YAGNTC_HD]\nStart=14\nLoopStart=11\nLoopEnd=20\nRate=150\n\
             [YAGNTC_P]\nStart=5\nLoopStart=2\nLoopEnd=11\nRate=300\n\
             [YAGNTC_PD]\nStart=15\nLoopStart=12\nLoopEnd=21\nRate=150\n",
        ));
        rules.merge_art_data(&art);
        let mut sim = Simulation::new();
        insert_building(&mut sim, 17, "YAGNTC", 4, 5);
        let occupied = [
            "YAGNTC_FD",
            "UNRELATED",
            "YAGNTC_GD",
            "YAGNTC_HD",
            "YAGNTC_PD",
        ];
        let old_frames = [15, 99, 16, 17, 18];
        let states = occupied
            .iter()
            .zip(old_frames)
            .map(|(name, frame)| AnimOverlayState {
                anim_type: sim.interner.intern(name),
                frame,
                loop_start: 90,
                loop_end: 100,
                rate_logic_frames: 77,
                elapsed_logic_frames: 66,
                finished: true,
            })
            .collect();
        sim.entities_mut()
            .get_mut(17)
            .unwrap()
            .building_anim_overlays = Some(BuildingAnimOverlays { anims: states });

        recreate_existing_slots_for_damage_state(&mut sim, &rules, 17);

        let overlays = &sim
            .entities()
            .get(17)
            .unwrap()
            .building_anim_overlays
            .as_ref()
            .unwrap()
            .anims;
        let names = overlays
            .iter()
            .map(|state| sim.interner.resolve(state.anim_type))
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec!["YAGNTC_F", "UNRELATED", "YAGNTC_G", "YAGNTC_H", "YAGNTC_P"]
        );
        assert_eq!(
            overlays.iter().map(|state| state.frame).collect::<Vec<_>>(),
            vec![5, 99, 6, 7, 8]
        );
        for state in overlays
            .iter()
            .filter(|state| sim.interner.resolve(state.anim_type) != "UNRELATED")
        {
            assert_eq!(state.rate_logic_frames, 3);
            assert_eq!(state.elapsed_logic_frames, 0);
            assert!(!state.finished);
        }
        assert_eq!(
            overlays.len(),
            occupied.len(),
            "absent YAGNTC_E stays absent"
        );
    }

    #[test]
    fn fatal_building_does_not_recreate_resolvable_slots_before_delete_drain() {
        let mut rules = RuleSet::from_ini(&IniFile::from_str(
            "[BuildingTypes]\n0=GAPOWR\n[GAPOWR]\nStrength=100\nArmor=wood\n",
        ))
        .expect("GAPOWR rules");
        rules.merge_art_data(&ArtRegistry::from_ini(&IniFile::from_str(
            "[GAPOWR]\nActiveAnim=GAPOWR_A\nActiveAnimDamaged=GAPOWR_AD\n\
             [GAPOWR_A]\nStart=2\nLoopStart=2\nLoopEnd=8\nRate=300\n\
             [GAPOWR_AD]\nStart=12\nLoopStart=12\nLoopEnd=18\nRate=150\n",
        )));
        let mut sim = Simulation::new();
        insert_building(&mut sim, 19, "GAPOWR", 4, 5);
        let damaged_anim = sim.interner.intern("GAPOWR_AD");
        let building = sim.entities_mut().get_mut(19).unwrap();
        building.health.current = 0;
        building.building_damage_state_active = false;
        building.building_anim_overlays = Some(BuildingAnimOverlays {
            anims: vec![AnimOverlayState {
                anim_type: damaged_anim,
                frame: 15,
                loop_start: 12,
                loop_end: 18,
                rate_logic_frames: 6,
                elapsed_logic_frames: 2,
                finished: false,
            }],
        });
        assert_eq!(sim.interner.get("GAPOWR_A"), None);

        recreate_existing_slots_for_damage_state(&mut sim, &rules, 19);

        let overlay = &sim
            .entities()
            .get(19)
            .expect("deferred-deletion object remains resolvable")
            .building_anim_overlays
            .as_ref()
            .unwrap()
            .anims[0];
        assert_eq!(sim.interner.resolve(overlay.anim_type), "GAPOWR_AD");
        assert_eq!(overlay.frame, 15);
        assert_eq!(sim.interner.get("GAPOWR_A"), None);
    }

    #[test]
    fn hostile_hit_crossing_yellow_recreates_occupied_slot_after_combat_restore() {
        let mut rules = RuleSet::from_ini(&IniFile::from_str(
            "[VehicleTypes]\n0=MTNK\n\
             [BuildingTypes]\n0=GAPOWR\n\
             [Warheads]\n0=TESTWH\n\
             [MTNK]\nStrength=300\nArmor=heavy\n\
             [GAPOWR]\nStrength=100\nArmor=wood\n\
             [TESTWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n\
             [AudioVisual]\nConditionYellow=50%\n",
        ))
        .expect("combat animation rules");
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[GAPOWR]\nActiveAnim=GAPOWR_A\nActiveAnimDamaged=GAPOWR_AD\n\
             [GAPOWR_A]\nStart=2\nLoopStart=2\nLoopEnd=8\nLoopCount=-1\nRate=300\n\
             [GAPOWR_AD]\nStart=12\nLoopStart=12\nLoopEnd=18\nLoopCount=-1\nRate=150\n",
        ));
        rules.merge_art_data(&art);

        let mut sim = Simulation::new();
        let attacker_owner = sim.interner.intern("Americans");
        let target_owner = sim.interner.intern("Russians");
        let attacker_type = sim.interner.intern("MTNK");
        let target_type = sim.interner.intern("GAPOWR");
        let healthy_anim = sim.interner.intern("GAPOWR_A");
        let warhead = sim.interner.intern("TESTWH");
        let mut attacker = GameEntity::new_at_frame_zero_for_test(
            1,
            4,
            5,
            0,
            0,
            attacker_owner,
            Health {
                current: 300,
                max: 300,
            },
            attacker_type,
            EntityCategory::Unit,
            0,
            5,
            true,
        );
        attacker.lifecycle.in_limbo = false;
        let mut target = GameEntity::new_at_frame_zero_for_test(
            2,
            6,
            5,
            0,
            0,
            target_owner,
            Health {
                current: 60,
                max: 100,
            },
            target_type,
            EntityCategory::Structure,
            0,
            5,
            true,
        );
        target.lifecycle.in_limbo = false;
        target.building_anim_overlays = Some(BuildingAnimOverlays {
            anims: vec![AnimOverlayState {
                anim_type: healthy_anim,
                frame: 5,
                loop_start: 2,
                loop_end: 8,
                rate_logic_frames: 3,
                elapsed_logic_frames: 2,
                finished: false,
            }],
        });
        sim.entities_mut().insert(attacker);
        sim.entities_mut().insert(target);

        let result = sim.tick_combat_with_fatal_lifecycle(
            &rules,
            None,
            100,
            &[],
            &BTreeSet::new(),
            &[],
            &[crate::sim::wave::WaveDamageEvent {
                wave_id: 77,
                target_id: 2,
                payload: crate::sim::wave::WaveDamagePayload {
                    firer_id: 1,
                    base_damage: 20,
                    warhead,
                },
            }],
        );

        assert_eq!(result.building_anim_reset_ids, vec![2]);
        let target = sim.entities().get(2).expect("surviving GAPOWR");
        assert_eq!(target.health.current, 40);
        assert!(target.building_damage_state_active);
        assert!(target.was_attacked_by_enemy);
        assert_eq!(target.building_anim_reset_revision, 1);
        let overlay = &target
            .building_anim_overlays
            .as_ref()
            .expect("occupied animation slot")
            .anims[0];
        assert_eq!(sim.interner.resolve(overlay.anim_type), "GAPOWR_AD");
        assert_eq!(overlay.frame, 15, "relative CurrentFrame=3 survives");
        assert_eq!(overlay.loop_start, 12);
        assert_eq!(overlay.loop_end, 18);
        assert_eq!(overlay.rate_logic_frames, 6);
        assert_eq!(overlay.elapsed_logic_frames, 0);
    }

    #[test]
    fn yellow_edge_then_fatal_hit_does_not_recreate_slots_before_lifecycle_uninit() {
        let mut rules = RuleSet::from_ini(&IniFile::from_str(
            "[VehicleTypes]\n0=MTNK\n\
             [BuildingTypes]\n0=GAPOWR\n\
             [Warheads]\n0=TESTWH\n\
             [MTNK]\nStrength=300\nArmor=heavy\n\
             [GAPOWR]\nStrength=100\nArmor=wood\n\
             [TESTWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n\
             [AudioVisual]\nConditionYellow=50%\n",
        ))
        .expect("fatal combat animation rules");
        rules.merge_art_data(&ArtRegistry::from_ini(&IniFile::from_str(
            "[GAPOWR]\nActiveAnim=GAPOWR_A\nActiveAnimDamaged=GAPOWR_AD\n\
             [GAPOWR_A]\nStart=2\nLoopStart=2\nLoopEnd=8\nLoopCount=-1\nRate=300\n\
             [GAPOWR_AD]\nStart=12\nLoopStart=12\nLoopEnd=18\nLoopCount=-1\nRate=150\n",
        )));

        let mut sim = Simulation::new();
        let attacker_owner = sim.interner.intern("Americans");
        let target_owner = sim.interner.intern("Russians");
        let attacker_type = sim.interner.intern("MTNK");
        let target_type = sim.interner.intern("GAPOWR");
        let healthy_anim = sim.interner.intern("GAPOWR_A");
        let warhead = sim.interner.intern("TESTWH");
        let mut attacker = GameEntity::new_at_frame_zero_for_test(
            1,
            4,
            5,
            0,
            0,
            attacker_owner,
            Health {
                current: 300,
                max: 300,
            },
            attacker_type,
            EntityCategory::Unit,
            0,
            5,
            true,
        );
        attacker.lifecycle.in_limbo = false;
        let mut target = GameEntity::new_at_frame_zero_for_test(
            2,
            6,
            5,
            0,
            0,
            target_owner,
            Health {
                current: 60,
                max: 100,
            },
            target_type,
            EntityCategory::Structure,
            0,
            5,
            true,
        );
        target.lifecycle.in_limbo = false;
        target.building_anim_overlays = Some(BuildingAnimOverlays {
            anims: vec![AnimOverlayState {
                anim_type: healthy_anim,
                frame: 5,
                loop_start: 2,
                loop_end: 8,
                rate_logic_frames: 3,
                elapsed_logic_frames: 2,
                finished: false,
            }],
        });
        sim.entities_mut().insert(attacker);
        sim.entities_mut().insert(target);

        let result = sim.tick_combat_with_fatal_lifecycle(
            &rules,
            None,
            100,
            &[],
            &BTreeSet::new(),
            &[],
            &[
                crate::sim::wave::WaveDamageEvent {
                    wave_id: 77,
                    target_id: 2,
                    payload: crate::sim::wave::WaveDamagePayload {
                        firer_id: 1,
                        base_damage: 20,
                        warhead,
                    },
                },
                crate::sim::wave::WaveDamageEvent {
                    wave_id: 78,
                    target_id: 2,
                    payload: crate::sim::wave::WaveDamagePayload {
                        firer_id: 1,
                        base_damage: 40,
                        warhead,
                    },
                },
            ],
        );

        assert_eq!(result.building_anim_reset_ids, vec![2]);
        assert!(
            !result.immediate_uninit_ids.contains(&2),
            "the world inline hook already consumed the fatal handoff"
        );
        let target = sim
            .entities()
            .get(2)
            .expect("physical deletion is deferred");
        assert_eq!(target.health.current, 0);
        assert!(!target.lifecycle.object_alive);
        assert!(target.lifecycle.in_limbo);
        assert!(sim.substrate.pending_delete.contains(&2));
        let overlay = &target
            .building_anim_overlays
            .as_ref()
            .expect("doomed occupied slot is not reconstructed")
            .anims[0];
        assert_eq!(sim.interner.resolve(overlay.anim_type), "GAPOWR_A");
        assert_eq!(overlay.frame, 5);
        assert_eq!(sim.interner.get("GAPOWR_AD"), None);
    }

    #[test]
    fn entering_damage_without_a_damaged_descriptor_retains_the_occupant() {
        let mut rules = RuleSet::from_ini(&IniFile::from_str(
            "[BuildingTypes]\n0=TEST\n[TEST]\nStrength=100\nCost=100\nArmor=wood\n",
        ))
        .unwrap();
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[TEST]\nActiveAnim=TEST_A\n[TEST_A]\nStart=2\nLoopStart=2\nLoopEnd=8\n",
        ));
        rules.merge_art_data(&art);
        let mut sim = Simulation::new();
        insert_building(&mut sim, 9, "TEST", 2, 2);
        let anim_type = sim.interner.intern("TEST_A");
        let original = AnimOverlayState {
            anim_type,
            frame: 6,
            loop_start: 2,
            loop_end: 8,
            rate_logic_frames: 5,
            elapsed_logic_frames: 4,
            finished: false,
        };
        let entity = sim.entities_mut().get_mut(9).unwrap();
        entity.building_damage_state_active = true;
        entity.building_anim_overlays = Some(BuildingAnimOverlays {
            anims: vec![original.clone()],
        });

        recreate_existing_slots_for_damage_state(&mut sim, &rules, 9);

        let state = &sim
            .entities()
            .get(9)
            .unwrap()
            .building_anim_overlays
            .as_ref()
            .unwrap()
            .anims[0];
        assert_eq!(state.anim_type, original.anim_type);
        assert_eq!(state.frame, original.frame);
        assert_eq!(state.elapsed_logic_frames, original.elapsed_logic_frames);
    }

    #[test]
    fn placement_owner_fact_requires_success_and_skips_walls() {
        use crate::sim::command::{Command, CommandEnvelope};

        let rules = refinery_rules_and_art();
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        let refinery = sim.interner.intern("GAREFN");
        let wall = sim.interner.intern("GAWALL");
        let refinery_command = CommandEnvelope::new(
            owner,
            1,
            Command::PlaceReadyBuilding {
                owner,
                type_id: refinery,
                rx: 4,
                ry: 5,
            },
        );
        let wall_command = CommandEnvelope::new(
            owner,
            2,
            Command::PlaceReadyBuilding {
                owner,
                type_id: wall,
                rx: 6,
                ry: 5,
            },
        );

        assert_eq!(
            sim.successful_non_wall_placement_owner(&refinery_command, false, Some(&rules)),
            None,
            "a rejected placement cannot borrow an unrelated spawn signal"
        );
        assert_eq!(
            sim.successful_non_wall_placement_owner(&wall_command, true, Some(&rules)),
            None,
            "wall overlays do not arm a producer crane"
        );
        assert_eq!(
            sim.successful_non_wall_placement_owner(&refinery_command, true, Some(&rules)),
            Some(owner)
        );
    }

    #[test]
    fn finalize_bale_event_arms_special_overlay_and_spawns_smoke_exactly_once() {
        let rules = refinery_rules_and_art();
        let mut sim = Simulation::new();
        insert_building(&mut sim, 41, "GAREFN", 7, 9);
        sim.bale_events.push(BaleDepositEvent {
            building_id: 41,
            tick: 12,
        });

        let queued_hash = sim.state_hash();
        finalize(&mut sim, &[], true, Some(&rules));

        assert!(sim.bale_events.is_empty());
        let overlay = &sim
            .entities()
            .get(41)
            .expect("refinery")
            .building_anim_overlays
            .as_ref()
            .expect("bale arms SpecialAnim")
            .anims[0];
        assert_eq!(sim.interner.resolve(overlay.anim_type), "GAREFN_B");
        assert_eq!(overlay.frame, 2);
        assert_eq!(overlay.loop_start, 1);
        assert_eq!(overlay.loop_end, 5);
        assert_eq!(overlay.rate_logic_frames, 3);
        assert_eq!(overlay.elapsed_logic_frames, 1);

        assert_eq!(sim.particle_systems().len(), 1);
        let particle_system = sim
            .particle_systems()
            .iter()
            .next()
            .map(|(_, system)| system)
            .expect("refinery smoke system");
        assert_eq!(
            particle_system.coords,
            glam::IVec3::new(7 * 256 + 128 + 10, 9 * 256 + 128 - 20, 30)
        );
        assert_eq!(particle_system.owner_entity, Some(41));
        assert_eq!(
            rules.particle_system_type(particle_system.type_id).name,
            "RefSmokeSystem"
        );

        let finalized_hash = sim.state_hash();
        assert_ne!(finalized_hash, queued_hash);

        finalize(&mut sim, &[], false, Some(&rules));
        assert_eq!(sim.particle_systems().len(), 1);
        assert_eq!(sim.state_hash(), finalized_hash);
    }

    #[test]
    fn bale_event_waits_for_complete_rules_art_then_drains_once() {
        let rules = refinery_rules_and_art();
        let rules_without_art = RuleSet::from_ini(&IniFile::from_str(
            "[BuildingTypes]\n0=GAREFN\n[GAREFN]\nImage=GAREFN\n",
        ))
        .expect("rules without merged art");
        let mut sim = Simulation::new();
        insert_building(&mut sim, 41, "GAREFN", 7, 9);
        sim.bale_events.push(BaleDepositEvent {
            building_id: 41,
            tick: 12,
        });

        finalize(&mut sim, &[], true, None);
        assert_eq!(sim.bale_events.len(), 1);
        assert!(sim.particle_systems().is_empty());

        finalize(&mut sim, &[], true, Some(&rules_without_art));
        assert_eq!(sim.bale_events.len(), 1);
        assert!(sim.particle_systems().is_empty());

        finalize(&mut sim, &[], true, Some(&rules));
        assert!(sim.bale_events.is_empty());
        assert_eq!(sim.particle_systems().len(), 1);

        finalize(&mut sim, &[], true, Some(&rules));
        assert_eq!(sim.particle_systems().len(), 1);
    }

    fn refinery_sim_with_bale() -> Simulation {
        let mut sim = Simulation::new();
        insert_building(&mut sim, 41, "GAREFN", 7, 9);
        sim.bale_events.push(BaleDepositEvent {
            building_id: 41,
            tick: 12,
        });
        sim
    }

    #[test]
    fn headless_and_app_frames_share_bale_authority_and_defer_particle_ai() {
        let rules = refinery_rules_and_art();
        let height_map = std::collections::BTreeMap::new();
        let mut app_sim = refinery_sim_with_bale();
        let mut headless_sim = refinery_sim_with_bale();

        let app_output = app_sim.advance_app_frame(
            &[],
            Some(&rules),
            &height_map,
            None,
            67,
            crate::sim::world::TickLane::Ordinary,
            None,
        );
        let headless_tick =
            headless_sim.advance_tick(&[], Some(&rules), &height_map, None, None, 67);

        assert_eq!(app_output.tick.state_hash, headless_tick.state_hash);
        assert_eq!(app_output.tick.state_hash, app_sim.state_hash());
        assert_eq!(headless_tick.state_hash, headless_sim.state_hash());
        let first_frame_system = headless_sim
            .particle_systems()
            .iter()
            .next()
            .map(|(_, system)| system)
            .expect("refinery smoke system created in the frame tail");
        assert!(
            first_frame_system.particles.is_empty(),
            "a frame-tail particle system must not receive an earlier AI visit"
        );

        headless_sim.advance_tick(&[], Some(&rules), &height_map, None, None, 67);
        let next_frame_system = headless_sim
            .particle_systems()
            .iter()
            .next()
            .map(|(_, system)| system)
            .expect("refinery smoke system survives its first AI visit");
        assert_eq!(next_frame_system.particles.len(), 1);
    }

    #[test]
    fn app_frame_hash_includes_bale_overlay_and_particle_finalization() {
        let rules = refinery_rules_and_art();
        let mut sim = Simulation::new();
        insert_building(&mut sim, 41, "GAREFN", 7, 9);
        sim.bale_events.push(BaleDepositEvent {
            building_id: 41,
            tick: 12,
        });

        let output = sim.advance_app_frame(
            &[],
            Some(&rules),
            &std::collections::BTreeMap::new(),
            None,
            67,
            crate::sim::world::TickLane::Ordinary,
            None,
        );

        assert!(output.tick.frame_committed);
        assert!(sim.bale_events.is_empty());
        assert!(
            sim.entities()
                .get(41)
                .expect("refinery")
                .building_anim_overlays
                .is_some()
        );
        assert_eq!(sim.particle_systems().len(), 1);
        assert_eq!(output.tick.state_hash, sim.state_hash());
    }
}
