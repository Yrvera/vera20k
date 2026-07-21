//! Scheduler-owned ordinary SHP animation objects.
//!
//! `AnimStore` owns animation storage while `world::LogicVector` owns live AI
//! order. This module implements only the verified ordinary non-bouncer
//! AnimClass lifecycle needed by building damage fire: constructor/reveal,
//! first-AI guard, logic-frame timing, loops, reverse/ping-pong, Next, trailer,
//! sound identity, conceal, and deferred deletion.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::rules::art_data::AnimTypeRuntimeConfig;
use crate::rules::ruleset::RuleSet;
use crate::sim::components::AnimClassSpawnDescriptor;
use crate::sim::intern::InternedId;
use crate::sim::world::{SimSoundEvent, Simulation};

pub type AnimId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnimWorldCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

const LEPTONS_PER_CELL: i32 = 256;
const ANIM_HEIGHT_LEVEL_LEPTONS: i32 = 128;
const TRAILER_DRAW_FLAGS: u32 = 0x600;
const BUILDING_RENDER_ORIGIN_LEPTONS: i32 = 128;
const DAMAGE_FIRE_SLOT_COUNT: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnimRuntime {
    pub current_frame: i32,
    pub frame_step: i32,
    pub delay_remaining: u16,
    pub rate_reload: u16,
    pub rate_elapsed: u16,
    pub loop_remaining: u8,
    pub first_ai_guard: bool,
    pub constructor_reverse: bool,
    pub inactive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnimObject {
    pub stable_id: AnimId,
    pub type_id: InternedId,
    /// Absolute world leptons. Z uses the animation constructor's 128-lepton
    /// height level, not combat's terrain-height conversion.
    pub world_coord: AnimWorldCoord,
    pub draw_flags: u32,
    pub z_adjust: i32,
    pub effective_end: i32,
    pub effective_loop_end: i32,
    pub runtime: AnimRuntime,
    pub in_logic_vector: bool,
    pub start_sound_active: bool,
    pub stop_sound_id: Option<InternedId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimStore(BTreeMap<AnimId, AnimObject>);

impl AnimStore {
    pub fn get(&self, id: AnimId) -> Option<&AnimObject> {
        self.0.get(&id)
    }

    pub(crate) fn get_mut(&mut self, id: AnimId) -> Option<&mut AnimObject> {
        self.0.get_mut(&id)
    }

    pub(crate) fn insert(&mut self, object: AnimObject) -> Option<AnimObject> {
        self.0.insert(object.stable_id, object)
    }

    pub(crate) fn remove(&mut self, id: AnimId) -> Option<AnimObject> {
        self.0.remove(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AnimId, &AnimObject)> {
        self.0.iter()
    }

    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut AnimObject> {
        self.0.values_mut()
    }

    pub fn contains_key(&self, id: AnimId) -> bool {
        self.0.contains_key(&id)
    }
}

#[derive(Debug, Error)]
pub enum AnimSpawnError {
    #[error("animation type id {0} does not resolve to bound runtime metadata")]
    MissingType(InternedId),
    #[error("animation type [{0}] has no bound SHP frame count")]
    UnboundType(String),
    #[error("animation stable id {0} collided with an existing object")]
    DuplicateId(AnimId),
}

enum VisitAction {
    None,
    Destroy,
    Next(String),
}

impl Simulation {
    pub fn anim(&self, id: AnimId) -> Option<&AnimObject> {
        self.substrate.anims.get(id)
    }

    pub fn anims(&self) -> impl Iterator<Item = (&AnimId, &AnimObject)> {
        self.substrate.anims.iter()
    }

    pub(crate) fn spawn_anim_object(
        &mut self,
        rules: &RuleSet,
        descriptor: AnimClassSpawnDescriptor,
    ) -> Result<AnimId, AnimSpawnError> {
        let world_coord = AnimWorldCoord {
            x: i32::from(descriptor.rx)
                .wrapping_mul(LEPTONS_PER_CELL)
                .wrapping_add(descriptor.sub_x.to_num::<i32>()),
            y: i32::from(descriptor.ry)
                .wrapping_mul(LEPTONS_PER_CELL)
                .wrapping_add(descriptor.sub_y.to_num::<i32>()),
            z: i32::from(descriptor.z).wrapping_mul(ANIM_HEIGHT_LEVEL_LEPTONS),
        };
        self.spawn_anim_at_world(rules, descriptor, world_coord)
    }

    pub(crate) fn spawn_anim_at_world(
        &mut self,
        rules: &RuleSet,
        descriptor: AnimClassSpawnDescriptor,
        world_coord: AnimWorldCoord,
    ) -> Result<AnimId, AnimSpawnError> {
        let type_name = self
            .interner
            .resolve(descriptor.type_name)
            .to_ascii_uppercase();
        let config = rules
            .art_registry
            .anim_runtime_config(&type_name)
            .cloned()
            .ok_or(AnimSpawnError::MissingType(descriptor.type_name))?;
        let (effective_end, effective_loop_end) = effective_bounds(&type_name, &config)?;
        let reverse = descriptor.reverse || config.reverse;
        let rate_reload = self.choose_anim_rate(&config);
        let stop_sound_id = config
            .stop_sound
            .as_deref()
            .map(|sound| self.interner.intern(sound));
        let stable_id = self.allocate_stable_id();
        if self.substrate.anims.contains_key(stable_id)
            || self.substrate.entities.contains(stable_id)
        {
            return Err(AnimSpawnError::DuplicateId(stable_id));
        }
        let object = AnimObject {
            stable_id,
            type_id: descriptor.type_name,
            world_coord,
            draw_flags: descriptor.draw_flags,
            z_adjust: descriptor.z_adjust,
            effective_end,
            effective_loop_end,
            runtime: AnimRuntime {
                current_frame: if reverse {
                    effective_loop_end.wrapping_sub(1)
                } else {
                    0
                },
                frame_step: if reverse { -1 } else { 1 },
                delay_remaining: descriptor.delay,
                rate_reload,
                rate_elapsed: 0,
                loop_remaining: native_loop_remaining(config.loop_count, descriptor.loop_count),
                first_ai_guard: true,
                constructor_reverse: descriptor.reverse,
                inactive: false,
            },
            in_logic_vector: false,
            start_sound_active: false,
            stop_sound_id,
        };
        debug_assert!(self.substrate.anims.insert(object).is_none());
        // Native registry insertion precedes Reveal, and Reveal precedes the
        // delay-zero constructor-time Middle call.
        self.reveal_anim(stable_id);
        if descriptor.delay == 0 {
            self.anim_middle(stable_id, &config);
        }
        Ok(stable_id)
    }

    pub(crate) fn visit_anim(&mut self, id: AnimId, rules: &RuleSet) {
        let Some((type_id, world_coord, first_guard, inactive)) =
            self.substrate.anims.get(id).map(|anim| {
                (
                    anim.type_id,
                    anim.world_coord,
                    anim.runtime.first_ai_guard,
                    anim.runtime.inactive,
                )
            })
        else {
            return;
        };
        if inactive {
            return;
        }
        let type_name = self.interner.resolve(type_id).to_ascii_uppercase();
        let Some(config) = rules.art_registry.anim_runtime_config(&type_name).cloned() else {
            self.destroy_anim(id);
            return;
        };

        if let Some(trailer_name) = config.trailer_anim.as_deref() {
            if trailer_cadence_matches(
                u64::from(self.session.binary_frame),
                config.trailer_seperation,
            ) {
                if let Some(trailer_type) = self.interner.get(trailer_name) {
                    let descriptor = AnimClassSpawnDescriptor {
                        type_name: trailer_type,
                        rx: 0,
                        ry: 0,
                        sub_x: crate::util::fixed_math::SIM_ZERO,
                        sub_y: crate::util::fixed_math::SIM_ZERO,
                        z: 0,
                        delay: 1,
                        loop_count: 1,
                        draw_flags: TRAILER_DRAW_FLAGS,
                        z_adjust: 0,
                        reverse: false,
                    };
                    self.spawn_anim_at_world(rules, descriptor, world_coord)
                        .expect("validated trailer closure must remain spawnable");
                }
            }
        }

        if first_guard {
            if let Some(anim) = self.substrate.anims.get_mut(id) {
                anim.runtime.first_ai_guard = false;
            }
            return;
        }

        let mut action = VisitAction::None;
        let mut random_loop_delay = None;
        {
            let Some(anim) = self.substrate.anims.get_mut(id) else {
                return;
            };
            if anim.runtime.delay_remaining > 0 {
                anim.runtime.delay_remaining -= 1;
                return;
            }
            if anim.runtime.rate_reload == 0 {
                return;
            }
            anim.runtime.rate_elapsed = anim.runtime.rate_elapsed.saturating_add(1);
            if anim.runtime.rate_elapsed < anim.runtime.rate_reload {
                return;
            }
            anim.runtime.rate_elapsed = 0;
            anim.runtime.current_frame = anim
                .runtime
                .current_frame
                .wrapping_add(anim.runtime.frame_step);

            if config.ping_pong && anim_at_boundary(anim, &config) {
                anim.runtime.frame_step = anim.runtime.frame_step.wrapping_neg();
                return;
            }
            if !anim_at_boundary(anim, &config) {
                return;
            }
            if anim.runtime.loop_remaining != 0 && anim.runtime.loop_remaining != u8::MAX {
                anim.runtime.loop_remaining = anim.runtime.loop_remaining.saturating_sub(1);
            }
            if anim.runtime.loop_remaining != 0 {
                reset_to_loop_start(anim, &config);
                random_loop_delay = config.random_loop_delay;
            } else if let Some(next) = config.next.clone() {
                action = VisitAction::Next(next);
            } else {
                action = VisitAction::Destroy;
            }
        }

        if let Some((low, high)) = random_loop_delay {
            let delay = self
                .scenario_rng
                .next_range_u32_inclusive(u32::from(low), u32::from(high))
                as u16;
            if let Some(anim) = self.substrate.anims.get_mut(id) {
                anim.runtime.delay_remaining = delay;
            }
        }
        match action {
            VisitAction::None => {}
            VisitAction::Destroy => self.destroy_anim(id),
            VisitAction::Next(next) => self.switch_anim_type(id, &next, rules),
        }
    }

    pub(crate) fn destroy_anim(&mut self, id: AnimId) {
        let Some((world, already_inactive, stop_sound)) = self
            .substrate
            .anims
            .get(id)
            .map(|anim| (anim.world_coord, anim.runtime.inactive, anim.stop_sound_id))
        else {
            return;
        };
        if already_inactive {
            return;
        }
        if let Some(anim) = self.substrate.anims.get_mut(id) {
            anim.runtime.inactive = true;
            anim.start_sound_active = false;
        }
        self.sound_events.push(SimSoundEvent::AnimationStopped {
            anim_id: id,
            stop_sound_id: stop_sound,
            world,
        });
        self.conceal_anim(id);
        self.substrate.pending_delete.push(id);
    }

    pub(crate) fn set_anim_frame_and_z_adjust(&mut self, id: AnimId, frame: i32, z_adjust: i32) {
        if let Some(anim) = self.substrate.anims.get_mut(id) {
            anim.runtime.current_frame = frame;
            anim.z_adjust = z_adjust;
        }
    }

    pub(crate) fn update_building_damage_fire(&mut self, building_id: u64, rules: &RuleSet) {
        let Some((current, maximum, type_ref, position, prior_state, category)) =
            self.substrate.entities.get(building_id).map(|entity| {
                (
                    entity.health.current,
                    entity.health.max,
                    entity.type_ref,
                    entity.position.clone(),
                    entity.damage_fire_state_active,
                    entity.category,
                )
            })
        else {
            return;
        };
        if category != crate::map::entities::EntityCategory::Structure {
            return;
        }
        let Some(object_type) = self.object_type(type_ref, rules) else {
            return;
        };
        let can_be_occupied = object_type.can_be_occupied;
        let image = object_type.image.clone();
        let foundation = object_type.foundation.clone();
        let ratio = if can_be_occupied {
            rules.general.damage_fire_occupied_ratio
        } else {
            rules.general.damage_fire_ordinary_ratio
        };
        let active = maximum > 0
            && current > 0
            && i64::from(current) * i64::from(ratio.denominator)
                <= i64::from(maximum) * i64::from(ratio.numerator);
        if active == prior_state {
            return;
        }
        if let Some(entity) = self.substrate.entities.get_mut(building_id) {
            entity.damage_fire_state_active = active;
        }
        if !active {
            self.clear_building_damage_fire_slots(building_id);
            return;
        }

        let type_count = rules.general.damage_fire_types.len();
        if type_count == 0 {
            return;
        }
        let mut type_index = self
            .scenario_rng
            .next_range_u32_inclusive(0, type_count.saturating_sub(1) as u32)
            as usize;
        let offsets = rules
            .art_registry
            .get(&image)
            .map(|entry| entry.damage_fire_offsets.clone())
            .unwrap_or_default();
        let (foundation_w, foundation_h) =
            crate::rules::foundation::foundation_dimensions(&foundation);
        let foundation_sum = i32::from(foundation_w).wrapping_add(i32::from(foundation_h));
        let base_x = i32::from(position.rx)
            .wrapping_mul(LEPTONS_PER_CELL)
            .wrapping_add(position.sub_x.to_num::<i32>())
            .wrapping_sub(BUILDING_RENDER_ORIGIN_LEPTONS);
        let base_y = i32::from(position.ry)
            .wrapping_mul(LEPTONS_PER_CELL)
            .wrapping_add(position.sub_y.to_num::<i32>())
            .wrapping_sub(BUILDING_RENDER_ORIGIN_LEPTONS);
        let base_z = i32::from(position.z).wrapping_mul(ANIM_HEIGHT_LEVEL_LEPTONS);

        for slot in 0..DAMAGE_FIRE_SLOT_COUNT {
            let occupied = self
                .substrate
                .entities
                .get(building_id)
                .and_then(|entity| entity.damage_fire_anim_ids[slot]);
            if occupied.is_some() {
                return;
            }
            let Some(offset) = offsets.get(slot).copied() else {
                return;
            };
            let fire_name = &rules.general.damage_fire_types[type_index].name;
            let fire_type = self.interner.intern(fire_name);
            let descriptor = AnimClassSpawnDescriptor {
                type_name: fire_type,
                rx: position.rx,
                ry: position.ry,
                sub_x: position.sub_x,
                sub_y: position.sub_y,
                z: position.z,
                delay: 0,
                loop_count: 1,
                draw_flags: TRAILER_DRAW_FLAGS,
                z_adjust: 0,
                reverse: false,
            };
            let world = AnimWorldCoord {
                x: base_x.wrapping_add(offset.world_dx),
                y: base_y.wrapping_add(offset.world_dy),
                z: base_z,
            };
            let anim_id = self
                .spawn_anim_at_world(rules, descriptor, world)
                .expect("validated stock damage-fire animation must spawn");
            if let Some(entity) = self.substrate.entities.get_mut(building_id) {
                entity.damage_fire_anim_ids[slot] = Some(anim_id);
            }

            let scaled = offset
                .pixel_y
                .wrapping_sub(foundation_sum.wrapping_mul(15))
                .wrapping_mul(3);
            let z_adjust = (scaled >> 1).wrapping_sub(10).min(0);
            let effective_end = self
                .substrate
                .anims
                .get(anim_id)
                .map_or(0, |anim| anim.effective_end);
            let frame = if effective_end > 0 {
                self.scenario_rng
                    .next_range_u32_inclusive(0, effective_end.wrapping_sub(1) as u32)
                    as i32
            } else {
                0
            };
            self.set_anim_frame_and_z_adjust(anim_id, frame, z_adjust);
            type_index += 1;
            if type_index == type_count {
                type_index = 0;
            }
        }
    }

    pub(crate) fn clear_building_damage_fire_slots(&mut self, building_id: u64) {
        for slot in 0..DAMAGE_FIRE_SLOT_COUNT {
            let anim_id = self
                .substrate
                .entities
                .get(building_id)
                .and_then(|entity| entity.damage_fire_anim_ids[slot]);
            let Some(anim_id) = anim_id else {
                continue;
            };
            self.destroy_anim(anim_id);
            if let Some(entity) = self.substrate.entities.get_mut(building_id) {
                entity.damage_fire_anim_ids[slot] = None;
            }
        }
    }

    fn choose_anim_rate(&mut self, config: &AnimTypeRuntimeConfig) -> u16 {
        config
            .random_rate_logic_frames
            .map_or(config.rate_logic_frames, |(a, b)| {
                self.scenario_rng
                    .next_range_u32_inclusive(u32::from(a), u32::from(b)) as u16
            })
    }

    fn anim_middle(&mut self, id: AnimId, config: &AnimTypeRuntimeConfig) {
        let sound_name = config
            .start_sound
            .as_ref()
            .or(config.report.as_ref())
            .cloned();
        let Some(sound_name) = sound_name else {
            return;
        };
        let sound_id = self.interner.intern(&sound_name);
        let Some(world) = self.substrate.anims.get(id).map(|anim| anim.world_coord) else {
            return;
        };
        if let Some(anim) = self.substrate.anims.get_mut(id) {
            anim.start_sound_active = true;
        }
        self.sound_events.push(SimSoundEvent::AnimationStarted {
            anim_id: id,
            sound_id,
            world,
        });
    }

    fn switch_anim_type(&mut self, id: AnimId, next: &str, rules: &RuleSet) {
        let Some(config) = rules.art_registry.anim_runtime_config(next).cloned() else {
            self.destroy_anim(id);
            return;
        };
        let Ok((effective_end, effective_loop_end)) = effective_bounds(next, &config) else {
            self.destroy_anim(id);
            return;
        };
        let Some(type_id) = self.interner.get(next) else {
            self.destroy_anim(id);
            return;
        };
        let rate_reload = self.choose_anim_rate(&config);
        let stop_sound_id = config
            .stop_sound
            .as_deref()
            .map(|sound| self.interner.intern(sound));
        let constructor_reverse = self
            .substrate
            .anims
            .get(id)
            .is_some_and(|anim| anim.runtime.constructor_reverse);
        let reverse = constructor_reverse || config.reverse;
        if let Some(anim) = self.substrate.anims.get_mut(id) {
            anim.type_id = type_id;
            anim.effective_end = effective_end;
            anim.effective_loop_end = effective_loop_end;
            anim.stop_sound_id = stop_sound_id;
            anim.runtime.current_frame = if reverse {
                effective_loop_end.wrapping_sub(1)
            } else {
                0
            };
            anim.runtime.frame_step = if reverse { -1 } else { 1 };
            anim.runtime.delay_remaining = 0;
            anim.runtime.rate_reload = rate_reload;
            anim.runtime.rate_elapsed = 0;
            anim.runtime.loop_remaining = native_loop_remaining(config.loop_count, 1);
            anim.runtime.first_ai_guard = false;
            anim.runtime.inactive = false;
        }
        self.anim_middle(id, &config);
    }
}

fn effective_bounds(
    type_name: &str,
    config: &AnimTypeRuntimeConfig,
) -> Result<(i32, i32), AnimSpawnError> {
    let raw = config
        .raw_shp_frame_count
        .ok_or_else(|| AnimSpawnError::UnboundType(type_name.to_string()))?;
    let effective_end = if config.end == -1 {
        if config.shadow { raw / 2 } else { raw }
    } else {
        config.end
    };
    let effective_loop_end = if config.loop_end == -1 {
        effective_end
    } else {
        config.loop_end
    };
    Ok((effective_end, effective_loop_end))
}

fn native_loop_remaining(loop_count: i32, constructor_loop: u8) -> u8 {
    let raw = (loop_count as u8).wrapping_mul(constructor_loop.max(1));
    if raw < 2 { 1 } else { raw }
}

fn trailer_cadence_matches(binary_frame: u64, separation: i32) -> bool {
    separation == 1 || (separation > 1 && (binary_frame as i32) % separation == 0)
}

fn anim_at_boundary(anim: &AnimObject, config: &AnimTypeRuntimeConfig) -> bool {
    if anim.runtime.frame_step >= 0 {
        let limit = if anim.runtime.loop_remaining < 2 {
            anim.effective_end
        } else {
            anim.effective_loop_end.wrapping_sub(config.start)
        };
        anim.runtime.current_frame >= limit
    } else {
        let limit = if anim.runtime.loop_remaining < 2 {
            config.start
        } else {
            config.loop_start.wrapping_sub(config.start)
        };
        anim.runtime.current_frame <= limit
    }
}

fn reset_to_loop_start(anim: &mut AnimObject, config: &AnimTypeRuntimeConfig) {
    if anim.runtime.frame_step >= 0 && !anim.runtime.constructor_reverse && !config.reverse {
        anim.runtime.current_frame = config.loop_start.wrapping_sub(config.start);
    } else {
        anim.runtime.current_frame = anim.effective_loop_end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::entities::EntityCategory;
    use crate::rules::art_data::ArtRegistry;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;

    #[test]
    fn loop_byte_wraps_clamps_and_preserves_infinite() {
        assert_eq!(native_loop_remaining(0, 1), 1);
        assert_eq!(native_loop_remaining(1, 1), 1);
        assert_eq!(native_loop_remaining(2, 1), 2);
        assert_eq!(native_loop_remaining(-1, 1), u8::MAX);
        assert_eq!(native_loop_remaining(128, 2), 1);
    }

    #[test]
    fn trailer_zero_separation_never_divides_or_spawns() {
        assert!(!trailer_cadence_matches(0, 0));
        assert!(trailer_cadence_matches(7, 1));
        assert!(trailer_cadence_matches(6, 3));
        assert!(!trailer_cadence_matches(7, 3));
    }

    fn damage_fire_fixture(can_be_occupied: bool) -> (Simulation, RuleSet, u64) {
        let rules_ini = IniFile::from_str(&format!(
            "[BuildingTypes]\n0=TESTBLD\n\n\
             [TESTBLD]\nStrength=100\nImage=TESTART\nCanBeOccupied={}\n\n\
             [General]\nDamageFireTypes=FIRE01,FIRE02,FIRE03\n\n\
             [AudioVisual]\nConditionYellow=50%\nConditionRed=25%\n",
            if can_be_occupied { "yes" } else { "no" },
        ));
        let mut rules = RuleSet::from_ini(&rules_ini).expect("damage-fire rules");
        let art_ini = IniFile::from_str(
            "[TESTART]\nFoundation=4x4\nDamageFireOffset0=-24,-1\nDamageFireOffset1=64,36\n\n\
             [FIRE01]\nRate=450\nLoopCount=-1\nStartSound=BuildingFireBig\n\n\
             [FIRE02]\nRate=450\nLoopCount=-1\nStartSound=BuildingFireMed\n\n\
             [FIRE03]\nRate=450\nLoopCount=-1\nStartSound=BuildingFireSmall\n",
        );
        let mut art = ArtRegistry::from_ini(&art_ini);
        art.bind_anim_frame_count_for_test("FIRE01", 30);
        art.bind_anim_frame_count_for_test("FIRE02", 64);
        art.bind_anim_frame_count_for_test("FIRE03", 30);
        rules.merge_art_data(&art);
        rules.art_registry = art;

        let mut sim = Simulation::new();
        let owner = sim.interner.intern("A");
        let type_ref = sim.interner.intern("TESTBLD");
        for name in ["FIRE01", "FIRE02", "FIRE03"] {
            sim.interner.intern(name);
        }
        let id = sim.allocate_stable_id();
        let mut building = GameEntity::new(
            id,
            10,
            10,
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
            false,
        );
        building.foundation = "4x4".to_string();
        sim.substrate.entities.insert(building);
        sim.reveal(id);
        (sim, rules, id)
    }

    fn runtime_rules(art_text: &str, frame_counts: &[(&str, i32)]) -> RuleSet {
        let mut rules = RuleSet::from_ini(&IniFile::from_str(
            "[General]\nDamageFireTypes=\n\n[AudioVisual]\nConditionYellow=50%\nConditionRed=25%\n",
        ))
        .unwrap();
        let mut art = ArtRegistry::from_ini(&IniFile::from_str(art_text));
        for &(name, frames) in frame_counts {
            art.bind_anim_frame_count_for_test(name, frames);
        }
        rules.art_registry = art;
        rules
    }

    fn runtime_descriptor(type_name: InternedId, delay: u16) -> AnimClassSpawnDescriptor {
        AnimClassSpawnDescriptor {
            type_name,
            rx: 0,
            ry: 0,
            sub_x: crate::util::fixed_math::SIM_ZERO,
            sub_y: crate::util::fixed_math::SIM_ZERO,
            z: 0,
            delay,
            loop_count: 1,
            draw_flags: TRAILER_DRAW_FLAGS,
            z_adjust: 0,
            reverse: false,
        }
    }

    #[test]
    fn delay_rate_and_first_guard_use_logic_visits_only() {
        let rules = runtime_rules("[TEST]\nRate=450\nEnd=3\nLoopCount=1\n", &[("TEST", 3)]);
        let mut sim = Simulation::new();
        let type_id = sim.interner.intern("TEST");
        let rng_before = sim.scenario_rng.logical_state();
        let id = sim
            .spawn_anim_object(&rules, runtime_descriptor(type_id, 1))
            .unwrap();

        sim.visit_anim(id, &rules); // constructor first-AI guard
        sim.visit_anim(id, &rules); // delay 1 -> 0
        sim.visit_anim(id, &rules); // rate elapsed 1/2
        assert_eq!(sim.anim(id).unwrap().runtime.current_frame, 0);
        sim.visit_anim(id, &rules); // rate elapsed 2/2 -> frame 1

        assert_eq!(sim.anim(id).unwrap().runtime.current_frame, 1);
        assert_eq!(sim.scenario_rng.logical_state(), rng_before);
    }

    #[test]
    fn next_reuses_identity_runs_middle_and_destroy_is_idempotent() {
        let rules = runtime_rules(
            "[FIRST]\nRate=900\nEnd=2\nLoopCount=1\nNext=SECOND\nStartSound=FirstStart\n\n\
             [SECOND]\nRate=900\nEnd=2\nLoopCount=1\nReport=SecondReport\nStopSound=SecondStop\n",
            &[("FIRST", 2), ("SECOND", 2)],
        );
        let mut sim = Simulation::new();
        let first = sim.interner.intern("FIRST");
        sim.interner.intern("SECOND");
        let id = sim
            .spawn_anim_object(&rules, runtime_descriptor(first, 0))
            .unwrap();
        assert!(matches!(
            sim.sound_events.as_slice(),
            [SimSoundEvent::AnimationStarted { anim_id, .. }] if *anim_id == id
        ));

        sim.visit_anim(id, &rules); // guard
        sim.visit_anim(id, &rules); // frame 1
        sim.visit_anim(id, &rules); // frame 2 -> SECOND in place + Middle
        let anim = sim.anim(id).unwrap();
        assert_eq!(sim.interner.resolve(anim.type_id), "SECOND");
        assert_eq!(anim.runtime.current_frame, 0);
        assert_eq!(
            sim.sound_events
                .iter()
                .filter(|event| matches!(event, SimSoundEvent::AnimationStarted { .. }))
                .count(),
            2,
        );

        sim.visit_anim(id, &rules); // SECOND frame 1 (Next does not restore guard)
        sim.visit_anim(id, &rules); // SECOND frame 2 -> destroy
        sim.destroy_anim(id);
        assert!(sim.anim(id).unwrap().runtime.inactive);
        assert!(!sim.live_object_order_snapshot().contains(&id));
        assert_eq!(
            sim.sound_events
                .iter()
                .filter(|event| matches!(event, SimSoundEvent::AnimationStopped { .. }))
                .count(),
            1,
        );
    }

    #[test]
    fn trailer_tail_append_is_visited_and_guarded_in_same_live_walk() {
        let rules = runtime_rules(
            "[PARENT]\nRate=0\nEnd=2\nTrailerAnim=CHILD\nTrailerSeperation=1\n\n\
             [CHILD]\nRate=900\nEnd=2\nLoopCount=1\n",
            &[("PARENT", 2), ("CHILD", 2)],
        );
        let mut sim = Simulation::new();
        let parent_type = sim.interner.intern("PARENT");
        sim.interner.intern("CHILD");
        let parent = sim
            .spawn_anim_object(&rules, runtime_descriptor(parent_type, 0))
            .unwrap();

        sim.for_each_live_object(|sim, id| sim.visit_anim(id, &rules));

        let order = sim.live_object_order_snapshot();
        assert_eq!(order.len(), 2);
        assert_eq!(order[0], parent);
        let child = sim.anim(order[1]).unwrap();
        assert_eq!(sim.interner.resolve(child.type_id), "CHILD");
        assert!(!child.runtime.first_ai_guard);
        assert_eq!(child.runtime.current_frame, 0);
    }

    #[test]
    fn building_damage_fire_uses_exact_threshold_slots_coords_and_depth() {
        let (mut sim, rules, building_id) = damage_fire_fixture(false);
        sim.substrate
            .entities
            .get_mut(building_id)
            .unwrap()
            .health
            .current = 50;
        let mut expected_rng = sim.scenario_rng.clone();
        let start_type = expected_rng.next_range_u32_inclusive(0, 2) as usize;
        let type_names = ["FIRE01", "FIRE02", "FIRE03"];
        let frame_counts = [30_u32, 64, 30];
        let expected_types = [start_type, (start_type + 1) % type_names.len()];
        let expected_frames = expected_types
            .map(|index| expected_rng.next_range_u32_inclusive(0, frame_counts[index] - 1) as i32);
        sim.update_building_damage_fire(building_id, &rules);

        let building = sim.substrate.entities.get(building_id).unwrap();
        assert!(building.damage_fire_state_active);
        let first = building.damage_fire_anim_ids[0].expect("slot zero");
        let second = building.damage_fire_anim_ids[1].expect("slot one");
        assert!(
            building.damage_fire_anim_ids[2..]
                .iter()
                .all(Option::is_none)
        );
        let first_anim = sim.anim(first).unwrap();
        assert_eq!(
            sim.interner.resolve(first_anim.type_id),
            type_names[expected_types[0]]
        );
        assert_eq!(first_anim.runtime.current_frame, expected_frames[0]);
        assert_eq!(
            first_anim.world_coord,
            AnimWorldCoord {
                x: 2450,
                y: 2653,
                z: 0
            }
        );
        assert_eq!(first_anim.z_adjust, -192);
        let second_anim = sim.anim(second).unwrap();
        assert_eq!(
            sim.interner.resolve(second_anim.type_id),
            type_names[expected_types[1]]
        );
        assert_eq!(second_anim.runtime.current_frame, expected_frames[1]);
        assert_eq!(
            second_anim.world_coord,
            AnimWorldCoord {
                x: 3140,
                y: 2594,
                z: 0
            }
        );
        assert_eq!(second_anim.z_adjust, -136);
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state()
        );
        assert_eq!(
            sim.live_object_order_snapshot(),
            vec![building_id, first, second]
        );
    }

    #[test]
    fn unchanged_damage_fire_cache_consumes_no_rng_and_recovery_clears_slots() {
        let (mut sim, rules, building_id) = damage_fire_fixture(false);
        sim.substrate
            .entities
            .get_mut(building_id)
            .unwrap()
            .health
            .current = 50;
        sim.update_building_damage_fire(building_id, &rules);
        let rng_after_spawn = sim.scenario_rng.logical_state();
        let ids = sim
            .substrate
            .entities
            .get(building_id)
            .unwrap()
            .damage_fire_anim_ids;

        sim.update_building_damage_fire(building_id, &rules);
        assert_eq!(sim.scenario_rng.logical_state(), rng_after_spawn);
        assert_eq!(
            sim.substrate
                .entities
                .get(building_id)
                .unwrap()
                .damage_fire_anim_ids,
            ids
        );

        sim.substrate
            .entities
            .get_mut(building_id)
            .unwrap()
            .health
            .current = 51;
        sim.update_building_damage_fire(building_id, &rules);
        let building = sim.substrate.entities.get(building_id).unwrap();
        assert!(!building.damage_fire_state_active);
        assert!(building.damage_fire_anim_ids.iter().all(Option::is_none));
        assert_eq!(sim.live_object_order_snapshot(), vec![building_id]);
    }

    #[test]
    fn empty_fire_type_list_sets_cache_without_rng_or_slots() {
        let (mut sim, mut rules, building_id) = damage_fire_fixture(false);
        rules.general.damage_fire_types.clear();
        sim.substrate
            .entities
            .get_mut(building_id)
            .unwrap()
            .health
            .current = 50;
        let rng_before = sim.scenario_rng.logical_state();

        sim.update_building_damage_fire(building_id, &rules);

        let building = sim.substrate.entities.get(building_id).unwrap();
        assert!(building.damage_fire_state_active);
        assert!(building.damage_fire_anim_ids.iter().all(Option::is_none));
        assert_eq!(sim.scenario_rng.logical_state(), rng_before);
    }

    #[test]
    fn occupied_first_slot_stops_after_initial_type_roll() {
        let (mut sim, rules, building_id) = damage_fire_fixture(false);
        let fire_type = sim.interner.get("FIRE01").unwrap();
        let occupied_id = sim
            .spawn_anim_at_world(
                &rules,
                AnimClassSpawnDescriptor {
                    type_name: fire_type,
                    rx: 0,
                    ry: 0,
                    sub_x: crate::util::fixed_math::SIM_ZERO,
                    sub_y: crate::util::fixed_math::SIM_ZERO,
                    z: 0,
                    delay: 0,
                    loop_count: 1,
                    draw_flags: TRAILER_DRAW_FLAGS,
                    z_adjust: 0,
                    reverse: false,
                },
                AnimWorldCoord { x: 0, y: 0, z: 0 },
            )
            .unwrap();
        let building = sim.substrate.entities.get_mut(building_id).unwrap();
        building.damage_fire_anim_ids[0] = Some(occupied_id);
        building.health.current = 50;
        let mut expected_rng = sim.scenario_rng.clone();
        let _ = expected_rng.next_range_u32_inclusive(0, 2);

        sim.update_building_damage_fire(building_id, &rules);

        let slots = sim
            .substrate
            .entities
            .get(building_id)
            .unwrap()
            .damage_fire_anim_ids;
        assert_eq!(slots[0], Some(occupied_id));
        assert!(slots[1..].iter().all(Option::is_none));
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state()
        );
    }

    #[test]
    fn zero_health_clears_owned_anims_and_stop_is_idempotent() {
        let (mut sim, rules, building_id) = damage_fire_fixture(false);
        sim.substrate
            .entities
            .get_mut(building_id)
            .unwrap()
            .health
            .current = 50;
        sim.update_building_damage_fire(building_id, &rules);
        sim.sound_events.clear();
        sim.substrate
            .entities
            .get_mut(building_id)
            .unwrap()
            .health
            .current = 0;

        sim.update_building_damage_fire(building_id, &rules);
        sim.update_building_damage_fire(building_id, &rules);

        let building = sim.substrate.entities.get(building_id).unwrap();
        assert!(!building.damage_fire_state_active);
        assert!(building.damage_fire_anim_ids.iter().all(Option::is_none));
        assert_eq!(
            sim.sound_events
                .iter()
                .filter(|event| matches!(event, SimSoundEvent::AnimationStopped { .. }))
                .count(),
            2,
        );
    }

    #[test]
    fn occupiable_building_selects_condition_red_boundary() {
        let (mut sim, rules, building_id) = damage_fire_fixture(true);
        sim.substrate
            .entities
            .get_mut(building_id)
            .unwrap()
            .health
            .current = 26;
        sim.update_building_damage_fire(building_id, &rules);
        assert!(
            !sim.substrate
                .entities
                .get(building_id)
                .unwrap()
                .damage_fire_state_active
        );
        sim.substrate
            .entities
            .get_mut(building_id)
            .unwrap()
            .health
            .current = 25;
        sim.update_building_damage_fire(building_id, &rules);
        assert!(
            sim.substrate
                .entities
                .get(building_id)
                .unwrap()
                .damage_fire_state_active
        );
    }

    #[test]
    fn first_anim_visit_only_clears_guard() {
        let (mut sim, rules, building_id) = damage_fire_fixture(false);
        sim.substrate
            .entities
            .get_mut(building_id)
            .unwrap()
            .health
            .current = 50;
        sim.update_building_damage_fire(building_id, &rules);
        let anim_id = sim
            .substrate
            .entities
            .get(building_id)
            .unwrap()
            .damage_fire_anim_ids[0]
            .unwrap();
        let frame = sim.anim(anim_id).unwrap().runtime.current_frame;
        sim.visit_anim(anim_id, &rules);
        let anim = sim.anim(anim_id).unwrap();
        assert_eq!(anim.runtime.current_frame, frame);
        assert!(!anim.runtime.first_ai_guard);
    }

    #[test]
    fn anim_store_slots_scheduler_and_hash_roundtrip() {
        let (mut sim, rules, building_id) = damage_fire_fixture(false);
        sim.substrate
            .entities
            .get_mut(building_id)
            .unwrap()
            .health
            .current = 50;
        sim.update_building_damage_fire(building_id, &rules);
        assert!(sim.substrate.pending_delete.is_empty());
        let expected_hash = sim.state_hash();
        let expected_order = sim.live_object_order_snapshot();
        let bytes = bincode::serialize(&sim).expect("serialize sim with AnimStore");
        let mut restored: Simulation = bincode::deserialize(&bytes).expect("deserialize AnimStore");
        restored.rebuild_logic_membership();
        assert_eq!(restored.live_object_order_snapshot(), expected_order);
        assert_eq!(restored.state_hash(), expected_hash);
        assert!(restored.sound_events.is_empty());
        assert_eq!(
            restored
                .substrate
                .entities
                .get(building_id)
                .unwrap()
                .damage_fire_anim_ids,
            sim.substrate
                .entities
                .get(building_id)
                .unwrap()
                .damage_fire_anim_ids,
        );
    }
}
