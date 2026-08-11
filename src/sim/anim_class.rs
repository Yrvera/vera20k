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
use crate::sim::occupancy::{RawCellOccupationGrid, infantry_raw_occupation_mask};
use crate::sim::timer::CdTimer;
use crate::sim::world::{LifecycleOutput, SimSoundEvent, Simulation};
use crate::util::fixed_math::SimFixed;
use crate::util::lepton::{BRIDGE_HEIGHT_DELTA_LEPTONS, ground_height_leptons};

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
const MULTIPLAYER_FEEDBACK_Z_ADJUST: i32 = -5000;
const SYNC_EXEMPT_NATIVE_UNIQUE_ID: i32 = -2;

/// Pure YR `AnimClass_UpdateBouncePhysics` directional-frame projection.
pub fn directional_tumble_frame(running_frames: i32, bucket8: i32, global_frame: i32) -> i32 {
    let running = running_frames.max(1);
    running * ((-1 - bucket8) & 7) + (global_frame / 3).rem_euclid(running)
}

pub fn settled_bounce_frame(running_frames: i32) -> i32 {
    running_frames.wrapping_mul(8).wrapping_add(1)
}

/// `AnimClass_Update` @ 0x00423f37: landing consumes two inclusive rolls.
pub fn bounce_spawn_count(has_spawns: bool, spawn_count: i32, roll_a: i32, roll_b: i32) -> i32 {
    if !has_spawns || spawn_count <= 0 {
        return 0;
    }
    assert!((0..=spawn_count).contains(&roll_a));
    assert!((0..=spawn_count).contains(&roll_b));
    roll_a.wrapping_add(roll_b)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimDrawDetailInput {
    pub frame_rate_below_minimum: bool,
    pub type_detail_level: i32,
    pub game_detail_level: i32,
    pub hidden: bool,
    pub special_hidden: bool,
    pub type_special_hide: bool,
}

/// `AnimClass__DrawIt` @ 0x00422fd8: visibility gates precede flag selection.
pub fn anim_draw_detail_visible(input: AnimDrawDetailInput) -> bool {
    !(input.frame_rate_below_minimum && input.type_detail_level > 1)
        && !input.hidden
        && input.type_detail_level <= input.game_detail_level
        && !(input.special_hidden && input.type_special_hide)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimTranslucencyInput {
    pub base_flags: u32,
    pub forced_translucent: bool,
    pub forced_uses_75: bool,
    pub translucency_detail_level: i32,
    pub game_detail_level: i32,
    pub translucent_ramp: bool,
    pub current_frame: i32,
    pub frame_count: i32,
    pub explicit_translucency: i32,
    pub instance_ramp: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimTranslucencyResult {
    pub draw: bool,
    pub flags: u32,
}

fn translucency_flag(level: i32) -> u32 {
    match level {
        25 => 2,
        50 => 4,
        75 => 6,
        _ => 0,
    }
}

/// `AnimClass__DrawIt` @ 0x00423061: preserve native 25/50/75 flag values.
pub fn anim_translucency_selection(input: AnimTranslucencyInput) -> AnimTranslucencyResult {
    let mut flags = input.base_flags;
    if input.forced_translucent {
        flags |= if input.forced_uses_75 { 6 } else { 4 };
        return AnimTranslucencyResult { draw: true, flags };
    }
    if input.translucency_detail_level > input.game_detail_level {
        return AnimTranslucencyResult { draw: true, flags };
    }
    if input.translucent_ramp {
        if input.instance_ramp >= 15 {
            return AnimTranslucencyResult { draw: false, flags };
        }
        let frame = i64::from(input.current_frame);
        let frame_count = i64::from(input.frame_count);
        flags |= if frame * 5 > frame_count * 3 {
            6
        } else if frame * 5 > frame_count * 2 {
            4
        } else if frame * 5 > frame_count {
            2
        } else {
            0
        };
        return AnimTranslucencyResult { draw: true, flags };
    }
    if input.explicit_translucency > 0 {
        return AnimTranslucencyResult {
            draw: input.instance_ramp < 15,
            flags: flags | translucency_flag(input.explicit_translucency),
        };
    }
    if input.instance_ramp > 15 {
        return AnimTranslucencyResult { draw: false, flags };
    }
    if input.instance_ramp > 5 {
        flags |= 4;
    }
    AnimTranslucencyResult { draw: true, flags }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BuildingAnimPowerFlags {
    pub powered: bool,
    pub powered_light: bool,
    pub powered_effect: bool,
    pub powered_special: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingAnimSlotAction {
    None,
    Pause,
    Destroy,
    DestroyMarkEffectReplay,
    PlayActiveSlot3,
    PlayLowPowerSlot19,
    PlaySuperLowPowerSlot20,
    ReplayPoweredSpecial,
}

pub fn building_storage_fill_level(stored_amount: f64, capacity: i32) -> i32 {
    if capacity <= 0 {
        return 0;
    }
    let stored = stored_amount.trunc() as i32;
    ((f64::from(stored.wrapping_mul(4)) / f64::from(capacity)) + 0.5)
        .trunc()
        .clamp(0.0, 3.0) as i32
}

/// `BuildingClass__UpdateAnimVisibilityStates` @ 0x004547c0 action projection.
pub fn unpowered_building_anim_actions(
    slot: usize,
    has_anim: bool,
    flags: BuildingAnimPowerFlags,
    storage_active_gate: bool,
    active_slot3_powered: bool,
    super_low_power_available: bool,
) -> Vec<BuildingAnimSlotAction> {
    if !has_anim {
        return vec![BuildingAnimSlotAction::None];
    }
    if flags.powered {
        return vec![BuildingAnimSlotAction::Pause];
    }
    if flags.powered_light {
        let mut actions = vec![BuildingAnimSlotAction::Destroy];
        if slot == 10 && storage_active_gate && active_slot3_powered {
            actions.push(BuildingAnimSlotAction::PlayActiveSlot3);
        }
        return actions;
    }
    if flags.powered_effect {
        let mut actions = vec![BuildingAnimSlotAction::DestroyMarkEffectReplay];
        if slot == 16 && super_low_power_available {
            actions.push(BuildingAnimSlotAction::PlaySuperLowPowerSlot20);
        }
        return actions;
    }
    vec![BuildingAnimSlotAction::None]
}

pub fn powered_special_actions(
    records: &[BuildingAnimPowerFlags],
    restoring: bool,
    low_power_available: bool,
) -> Vec<(usize, BuildingAnimSlotAction)> {
    let mut actions = Vec::new();
    if restoring {
        actions.push((19, BuildingAnimSlotAction::Destroy));
    } else if low_power_available {
        actions.push((19, BuildingAnimSlotAction::PlayLowPowerSlot19));
    }
    actions.extend(records.iter().enumerate().filter_map(|(slot, flags)| {
        flags.powered_special.then_some((
            slot,
            if restoring {
                BuildingAnimSlotAction::ReplayPoweredSpecial
            } else {
                BuildingAnimSlotAction::Destroy
            },
        ))
    }));
    actions
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnimRuntime {
    pub current_frame: i32,
    pub frame_step: i32,
    pub delay_remaining: u16,
    pub rate_reload: u16,
    pub frame_timer: CdTimer,
    pub loop_remaining: u8,
    pub first_ai_guard: bool,
    pub constructor_reverse: bool,
    pub inactive: bool,
}

/// Per-instance `AnimClass::DrawIt` bytes that are independent of the art
/// type. They remain serialized simulation state because native effects may
/// set them between frames; presentation only reads this resolved input.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnimDrawRuntime {
    /// AnimClass `+0x19d`: unconditional draw suppression.
    pub hidden: bool,
    /// AnimClass `+0x199`: applies only with the unresolved type `+0x374` bit.
    pub special_hidden: bool,
    /// AnimClass `+0x178`: ramp/age input used by translucency selection.
    pub translucency_ramp: u8,
    /// AnimClass `+0x119`: force the type-selected 50/75% draw family.
    pub forced_translucent: bool,
    /// Producer-supplied type `+0x368` interpretation; the art-key mapping is
    /// not closed, so this is deliberately not inferred from other fields.
    pub forced_uses_75: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnimObject {
    pub stable_id: AnimId,
    pub native_unique_id: i32,
    pub type_id: InternedId,
    /// Absolute world leptons. Z uses the animation constructor's 128-lepton
    /// height level, not combat's terrain-height conversion.
    pub world_coord: AnimWorldCoord,
    pub draw_flags: u32,
    pub z_adjust: i32,
    pub effective_end: i32,
    pub effective_loop_end: i32,
    pub runtime: AnimRuntime,
    pub draw_runtime: AnimDrawRuntime,
    /// LogicClass membership is reconstructed from the serialized vector.
    /// ObjectClass::Save does not persist its local membership byte.
    #[serde(skip)]
    pub in_logic_vector: bool,
    pub owner_entity: Option<u64>,
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

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn key_at(&self, index: usize) -> Option<AnimId> {
        self.0.keys().nth(index).copied()
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
    DestroyAfterMakeInfantryClear,
    Next(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnimOccupationOperation {
    Mark,
    Clear,
}

fn apply_anim_raw_occupation(
    grid: &mut RawCellOccupationGrid,
    rx: u16,
    ry: u16,
    mask: u8,
    world_z: i32,
    ground_z: i32,
    live_structural_bridge: bool,
    operation: AnimOccupationOperation,
) {
    let reaches_deck = world_z >= ground_z.wrapping_add(BRIDGE_HEIGHT_DELTA_LEPTONS as i32);
    let use_deck = match operation {
        AnimOccupationOperation::Mark => reaches_deck && live_structural_bridge,
        // AnimClass::ClearCellOccupancy deliberately ignores Cell+0x140 bit
        // 0x100. This can leave a ground bit stale when a high animation was
        // marked after structural bridge state disappeared.
        AnimOccupationOperation::Clear => reaches_deck,
    };
    match (operation, use_deck) {
        (AnimOccupationOperation::Mark, false) => grid.mark_ground(rx, ry, mask),
        (AnimOccupationOperation::Mark, true) => grid.mark_deck(rx, ry, mask),
        (AnimOccupationOperation::Clear, false) => grid.clear_ground(rx, ry, mask),
        (AnimOccupationOperation::Clear, true) => grid.clear_deck(rx, ry, mask),
    }
}

impl Simulation {
    pub fn anim(&self, id: AnimId) -> Option<&AnimObject> {
        self.substrate
            .anims
            .get(id)
            .or_else(|| self.substrate.multiplayer_feedback_anims.get(id))
    }

    pub fn anims(&self) -> impl Iterator<Item = (&AnimId, &AnimObject)> {
        self.substrate
            .anims
            .iter()
            .chain(self.substrate.multiplayer_feedback_anims.iter())
    }

    pub fn multiplayer_feedback_anims(&self) -> impl Iterator<Item = (&AnimId, &AnimObject)> {
        self.substrate.multiplayer_feedback_anims.iter()
    }

    fn anim_mut_by_id(&mut self, id: AnimId) -> Option<&mut AnimObject> {
        if self.substrate.anims.contains_key(id) {
            self.substrate.anims.get_mut(id)
        } else {
            self.substrate.multiplayer_feedback_anims.get_mut(id)
        }
    }

    fn is_multiplayer_feedback_anim(&self, id: AnimId) -> bool {
        self.substrate.multiplayer_feedback_anims.contains_key(id)
    }

    fn apply_make_infantry_raw_occupation(
        &mut self,
        world: AnimWorldCoord,
        operation: AnimOccupationOperation,
    ) {
        let cell_x = world.x >> 8;
        let cell_y = world.y >> 8;
        let (Ok(rx), Ok(ry)) = (u16::try_from(cell_x), u16::try_from(cell_y)) else {
            // Native writes its shared dummy cell for out-of-map coordinates;
            // that dummy is not part of Rust's serialized map substrate.
            return;
        };
        let mask = infantry_raw_occupation_mask(
            SimFixed::from_num(world.x & 0xff),
            SimFixed::from_num(world.y & 0xff),
        );
        let (ground_z, live_structural_bridge) = self
            .resolved_terrain
            .as_ref()
            .and_then(|terrain| terrain.cell(rx, ry))
            .and_then(|cell| {
                ground_height_leptons(cell.level, cell.slope_type, world.x, world.y)
                    .ok()
                    .map(|ground_z| {
                        let live_structural_bridge = cell.bridge_facts.has_structural_bridge()
                            && self
                                .bridge_state
                                .as_ref()
                                .is_some_and(|state| state.is_bridge_walkable(rx, ry));
                        (ground_z, live_structural_bridge)
                    })
            })
            .unwrap_or((0, false));
        apply_anim_raw_occupation(
            &mut self.substrate.raw_cell_occupation,
            rx,
            ry,
            mask,
            world.z,
            ground_z,
            live_structural_bridge,
            operation,
        );
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
        let frame_timer =
            CdTimer::started(self.session.binary_frame as i32, i32::from(rate_reload));
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
            native_unique_id: stable_id as i32,
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
                frame_timer,
                loop_remaining: native_loop_remaining(config.loop_count, descriptor.loop_count),
                first_ai_guard: true,
                constructor_reverse: descriptor.reverse,
                inactive: false,
            },
            draw_runtime: descriptor.draw_runtime,
            in_logic_vector: false,
            owner_entity: None,
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

    pub(crate) fn spawn_multiplayer_feedback_anim_at_world(
        &mut self,
        rules: &RuleSet,
        world_coord: AnimWorldCoord,
    ) -> Result<AnimId, AnimSpawnError> {
        let type_id = self.interner.intern(&rules.general.move_flash.name);
        let type_name = self.interner.resolve(type_id).to_ascii_uppercase();
        let config = rules
            .art_registry
            .anim_runtime_config(&type_name)
            .cloned()
            .ok_or(AnimSpawnError::MissingType(type_id))?;
        let (effective_end, effective_loop_end) = effective_bounds(&type_name, &config)?;
        let reverse = config.reverse;
        let rate_reload = self.choose_anim_rate(&config);
        let frame_timer =
            CdTimer::started(self.session.binary_frame as i32, i32::from(rate_reload));
        let stop_sound_id = config
            .stop_sound
            .as_deref()
            .map(|sound| self.interner.intern(sound));
        let stable_id = self.substrate.next_multiplayer_feedback_anim_id;
        self.substrate.next_multiplayer_feedback_anim_id = stable_id.wrapping_add(1);
        if self
            .substrate
            .multiplayer_feedback_anims
            .contains_key(stable_id)
        {
            return Err(AnimSpawnError::DuplicateId(stable_id));
        }

        let object = AnimObject {
            stable_id,
            native_unique_id: SYNC_EXEMPT_NATIVE_UNIQUE_ID,
            type_id,
            world_coord,
            draw_flags: TRAILER_DRAW_FLAGS,
            z_adjust: MULTIPLAYER_FEEDBACK_Z_ADJUST,
            effective_end,
            effective_loop_end,
            runtime: AnimRuntime {
                current_frame: if reverse {
                    effective_loop_end.wrapping_sub(1)
                } else {
                    0
                },
                frame_step: if reverse { -1 } else { 1 },
                delay_remaining: 0,
                rate_reload,
                frame_timer,
                loop_remaining: native_loop_remaining(config.loop_count, 1),
                first_ai_guard: true,
                constructor_reverse: false,
                inactive: false,
            },
            draw_runtime: AnimDrawRuntime::default(),
            in_logic_vector: false,
            owner_entity: None,
            start_sound_active: false,
            stop_sound_id,
        };
        debug_assert!(
            self.substrate
                .multiplayer_feedback_anims
                .insert(object)
                .is_none()
        );
        self.anim_middle(stable_id, &config);
        Ok(stable_id)
    }

    pub(crate) fn for_each_multiplayer_feedback_anim<F>(&mut self, mut body: F)
    where
        F: FnMut(&mut Simulation, AnimId),
    {
        let mut index = 0;
        while index < self.substrate.multiplayer_feedback_anims.len() {
            let Some(id) = self.substrate.multiplayer_feedback_anims.key_at(index) else {
                break;
            };
            body(self, id);
            index += 1;
        }
    }

    pub(crate) fn visit_anim(&mut self, id: AnimId, rules: &RuleSet) {
        let Some((type_id, world_coord, first_guard, inactive)) = self.anim(id).map(|anim| {
            (
                anim.type_id,
                anim.world_coord,
                anim.runtime.first_ai_guard,
                anim.runtime.inactive,
            )
        }) else {
            return;
        };
        let type_name = self.interner.resolve(type_id).to_ascii_uppercase();
        let Some(config) = rules.art_registry.anim_runtime_config(&type_name).cloned() else {
            self.destroy_anim(id);
            return;
        };

        // AnimClass::AI performs this before its first-AI, inactive, delay,
        // visibility, and frame-timer gates. Repeated visits OR the same raw
        // bit; there is deliberately no contributor count.
        if config.make_infantry != -1 {
            self.apply_make_infantry_raw_occupation(world_coord, AnimOccupationOperation::Mark);
        }
        if inactive {
            self.destroy_anim(id);
            return;
        }

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
                        draw_runtime: AnimDrawRuntime::default(),
                    };
                    self.spawn_anim_at_world(rules, descriptor, world_coord)
                        .expect("validated trailer closure must remain spawnable");
                }
            }
        }

        if first_guard {
            if let Some(anim) = self.anim_mut_by_id(id) {
                anim.runtime.first_ai_guard = false;
            }
            return;
        }

        let mut action = VisitAction::None;
        let mut random_loop_delay = None;
        let current_frame = self.session.binary_frame as i32;
        {
            let Some(anim) = self.anim_mut_by_id(id) else {
                return;
            };
            if anim.runtime.delay_remaining > 0 {
                anim.runtime.delay_remaining -= 1;
                return;
            }
            if anim.runtime.rate_reload == 0 {
                return;
            }
            if !anim.runtime.frame_timer.expired(current_frame) {
                return;
            }
            anim.runtime
                .frame_timer
                .start(current_frame, i32::from(anim.runtime.rate_reload));
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
            } else if config.make_infantry != -1 {
                action = VisitAction::DestroyAfterMakeInfantryClear;
            } else {
                action = VisitAction::Destroy;
            }
        }

        if let Some((low, high)) = random_loop_delay {
            let delay = self
                .scenario_rng
                .next_range_u32_inclusive(u32::from(low), u32::from(high))
                as u16;
            if let Some(anim) = self.anim_mut_by_id(id) {
                anim.runtime.delay_remaining = delay;
            }
        }
        match action {
            VisitAction::None => {}
            VisitAction::Destroy => self.destroy_anim(id),
            VisitAction::DestroyAfterMakeInfantryClear => {
                // Native clears before validating AnimToInfantry, resolving an
                // owner, allocating the infantry, or attempting Unlimbo. The
                // downstream factory/retry path belongs to the entity-runtime
                // implementation item; this Phase-3 slice owns its preceding
                // authoritative cell-byte transition.
                self.apply_make_infantry_raw_occupation(
                    world_coord,
                    AnimOccupationOperation::Clear,
                );
                self.destroy_anim(id);
            }
            VisitAction::Next(next) => self.switch_anim_type(id, &next, rules),
        }
    }

    pub(crate) fn destroy_anim(&mut self, id: AnimId) {
        let is_feedback = self.is_multiplayer_feedback_anim(id);
        let already_queued = if is_feedback {
            self.substrate
                .multiplayer_feedback_pending_delete
                .contains(&id)
        } else {
            self.substrate.pending_delete.contains(&id)
        };
        if already_queued {
            return;
        }
        let Some((world, stop_sound)) = self
            .anim(id)
            .map(|anim| (anim.world_coord, anim.stop_sound_id))
        else {
            return;
        };
        self.detach_anim_from_owner(id);
        if let Some(anim) = self.anim_mut_by_id(id) {
            anim.runtime.inactive = true;
            anim.start_sound_active = false;
        }
        self.sound_events.push(SimSoundEvent::AnimationStopped {
            anim_id: id,
            stop_sound_id: stop_sound,
            world,
        });
        if is_feedback {
            self.substrate.multiplayer_feedback_pending_delete.push(id);
        } else {
            self.conceal_anim(id);
            self.substrate.pending_delete.push(id);
        }
    }

    pub(crate) fn detach_anim_from_owner(&mut self, id: AnimId) -> Option<u64> {
        let owner_id = self.anim(id).and_then(|anim| anim.owner_entity)?;
        if let Some(owner) = self.substrate.entities.get_mut(owner_id) {
            for slot in &mut owner.damage_fire_anim_ids {
                if *slot == Some(id) {
                    *slot = None;
                }
            }
        }
        if let Some(anim) = self.anim_mut_by_id(id) {
            anim.owner_entity = None;
        }
        Some(owner_id)
    }

    pub(crate) fn expire_anim_owner_reference(&mut self, id: AnimId, expired_id: u64) -> bool {
        if self.anim(id).and_then(|anim| anim.owner_entity) != Some(expired_id) {
            return false;
        }
        self.lifecycle_outputs
            .push(LifecycleOutput::DisplayRemove { stable_id: id });
        self.detach_anim_from_owner(id);
        if let Some(anim) = self.anim_mut_by_id(id) {
            anim.runtime.inactive = true;
        }
        true
    }

    pub(crate) fn set_anim_frame_and_z_adjust(&mut self, id: AnimId, frame: i32, z_adjust: i32) {
        if let Some(anim) = self.anim_mut_by_id(id) {
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
                draw_runtime: AnimDrawRuntime::default(),
            };
            let world = AnimWorldCoord {
                x: base_x.wrapping_add(offset.world_dx),
                y: base_y.wrapping_add(offset.world_dy),
                z: base_z,
            };
            let anim_id = self
                .spawn_anim_at_world(rules, descriptor, world)
                .expect("validated stock damage-fire animation must spawn");
            if let Some(anim) = self.anim_mut_by_id(anim_id) {
                anim.owner_entity = Some(building_id);
            }
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
        let delay = config
            .random_rate_logic_frames
            .map_or(config.rate_logic_frames, |(a, b)| {
                self.scenario_rng
                    .next_range_u32_inclusive(u32::from(a), u32::from(b)) as u16
            });
        if config.normalized {
            self.session.game_options.normalized_anim_delay(delay)
        } else {
            delay
        }
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
        let Some(world) = self.anim(id).map(|anim| anim.world_coord) else {
            return;
        };
        if let Some(anim) = self.anim_mut_by_id(id) {
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
        let frame_timer =
            CdTimer::started(self.session.binary_frame as i32, i32::from(rate_reload));
        let stop_sound_id = config
            .stop_sound
            .as_deref()
            .map(|sound| self.interner.intern(sound));
        let constructor_reverse = self
            .anim(id)
            .is_some_and(|anim| anim.runtime.constructor_reverse);
        let reverse = constructor_reverse || config.reverse;
        if let Some(anim) = self.anim_mut_by_id(id) {
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
            anim.runtime.frame_timer = frame_timer;
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
mod long_tail_contract_tests {
    use super::*;

    #[test]
    fn yr_long_tail_vectors() {
        assert_eq!(directional_tumble_frame(5, 0, 9), 38);
        assert_eq!(directional_tumble_frame(5, 7, 14), 4);
        assert_eq!(settled_bounce_frame(5), 41);
        assert_eq!(bounce_spawn_count(true, 4, 2, 3), 5);
        assert_eq!(building_storage_fill_level(12.9, 100), 0);
        assert_eq!(building_storage_fill_level(13.0, 100), 1);
        assert_eq!(building_storage_fill_level(63.0, 100), 3);
        assert_eq!(
            anim_translucency_selection(AnimTranslucencyInput {
                base_flags: 0,
                forced_translucent: false,
                forced_uses_75: false,
                translucency_detail_level: 1,
                game_detail_level: 2,
                translucent_ramp: true,
                current_frame: 7,
                frame_count: 10,
                explicit_translucency: 0,
                instance_ramp: 0,
            }),
            AnimTranslucencyResult {
                draw: true,
                flags: 6
            }
        );
    }

    #[test]
    fn yr_power_slot_actions_are_ordered() {
        assert_eq!(
            unpowered_building_anim_actions(
                10,
                true,
                BuildingAnimPowerFlags {
                    powered_light: true,
                    ..Default::default()
                },
                true,
                true,
                false,
            ),
            vec![
                BuildingAnimSlotAction::Destroy,
                BuildingAnimSlotAction::PlayActiveSlot3
            ]
        );
        let records = [
            BuildingAnimPowerFlags::default(),
            BuildingAnimPowerFlags {
                powered_special: true,
                ..Default::default()
            },
        ];
        assert_eq!(
            powered_special_actions(&records, false, true),
            vec![
                (19, BuildingAnimSlotAction::PlayLowPowerSlot19),
                (1, BuildingAnimSlotAction::Destroy),
            ]
        );
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
        let mut building = GameEntity::new_at_frame_zero_for_test(
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
            draw_runtime: AnimDrawRuntime::default(),
        }
    }

    #[test]
    fn gsi_04_12_anim_make_infantry_ini_preserves_native_default_and_signed_value() {
        let rules = runtime_rules(
            "[DEFAULT]\nRate=900\nEnd=1\n\n[EXPLICIT]\nRate=900\nEnd=1\nMakeInfantry=-2\n",
            &[("DEFAULT", 1), ("EXPLICIT", 1)],
        );

        assert_eq!(
            rules
                .art_registry
                .anim_runtime_config("DEFAULT")
                .unwrap()
                .make_infantry,
            -1
        );
        assert_eq!(
            rules
                .art_registry
                .anim_runtime_config("EXPLICIT")
                .unwrap()
                .make_infantry,
            -2
        );
    }

    #[test]
    fn draw_runtime_round_trips_and_changes_the_authoritative_hash() {
        let rules = runtime_rules("[DRAW]\nRate=900\nEnd=1\n", &[("DRAW", 1)]);
        let mut sim = Simulation::new();
        let mut descriptor = runtime_descriptor(sim.interner.intern("DRAW"), 0);
        descriptor.draw_runtime = AnimDrawRuntime {
            hidden: false,
            special_hidden: true,
            translucency_ramp: 6,
            forced_translucent: true,
            forced_uses_75: true,
        };
        let draw_runtime = descriptor.draw_runtime;
        let id = sim.spawn_anim_object(&rules, descriptor).unwrap();
        assert_eq!(sim.anim(id).unwrap().draw_runtime, draw_runtime);

        let serialized = bincode::serialize(&sim.substrate.anims).unwrap();
        let restored: AnimStore = bincode::deserialize(&serialized).unwrap();
        assert_eq!(restored.get(id).unwrap().draw_runtime, draw_runtime);

        let before = sim.state_hash();
        sim.anim_mut_by_id(id).unwrap().draw_runtime.hidden = true;
        assert_ne!(sim.state_hash(), before);
    }

    #[test]
    fn gsi_04_12_anim_make_infantry_marks_before_first_ai_and_clears_on_natural_end() {
        let rules = runtime_rules(
            "[GENDEATH]\nRate=900\nEnd=1\nLoopCount=1\nMakeInfantry=0\n",
            &[("GENDEATH", 1)],
        );
        let mut sim = Simulation::new();
        let type_id = sim.interner.intern("GENDEATH");
        let mut descriptor = runtime_descriptor(type_id, 0);
        descriptor.rx = 3;
        descriptor.ry = 4;
        descriptor.sub_x = SimFixed::from_num(192);
        descriptor.sub_y = SimFixed::from_num(64);
        let id = sim.spawn_anim_object(&rules, descriptor).unwrap();

        assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
        sim.visit_anim(id, &rules);
        assert_eq!(
            sim.substrate.raw_cell_occupation.ground_bits(3, 4),
            0x04,
            "the first AI guard runs after MakeInfantry raw marking"
        );
        assert_eq!(sim.anim(id).unwrap().runtime.current_frame, 0);

        sim.session.binary_frame = 1;
        sim.visit_anim(id, &rules);

        assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(3, 4), 0);
        assert!(sim.substrate.pending_delete.contains(&id));
    }

    #[test]
    fn gsi_04_12_anim_make_infantry_next_and_early_destroy_leave_destructive_mark_stale() {
        let rules = runtime_rules(
            "[GENDEATH]\nRate=900\nEnd=1\nLoopCount=1\nMakeInfantry=0\nNext=PLAIN\n\n\
             [PLAIN]\nRate=900\nEnd=1\nLoopCount=1\n",
            &[("GENDEATH", 1), ("PLAIN", 1)],
        );
        let mut sim = Simulation::new();
        let gen_type = sim.interner.intern("GENDEATH");
        let plain = sim.interner.intern("PLAIN");
        let mut descriptor = runtime_descriptor(gen_type, 0);
        descriptor.rx = 5;
        descriptor.ry = 6;
        descriptor.sub_x = SimFixed::from_num(64);
        descriptor.sub_y = SimFixed::from_num(192);
        let id = sim.spawn_anim_object(&rules, descriptor).unwrap();

        sim.visit_anim(id, &rules);
        assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(5, 6), 0x08);
        sim.session.binary_frame = 1;
        sim.visit_anim(id, &rules);

        assert_eq!(sim.anim(id).unwrap().type_id, plain);
        assert_eq!(
            sim.substrate.raw_cell_occupation.ground_bits(5, 6),
            0x08,
            "Next takes priority and performs no MakeInfantry clear"
        );
        sim.destroy_anim(id);
        assert_eq!(
            sim.substrate.raw_cell_occupation.ground_bits(5, 6),
            0x08,
            "generic Anim destruction does not repair the raw byte"
        );
    }

    #[test]
    fn gsi_04_12_anim_make_infantry_mark_and_clear_keep_native_bridge_asymmetry() {
        let mut grid = RawCellOccupationGrid::new();

        apply_anim_raw_occupation(
            &mut grid,
            7,
            8,
            0x10,
            416,
            0,
            true,
            AnimOccupationOperation::Mark,
        );
        assert_eq!(grid.ground_bits(7, 8), 0);
        assert_eq!(grid.deck_bits(7, 8), 0x10);
        apply_anim_raw_occupation(
            &mut grid,
            7,
            8,
            0x10,
            416,
            0,
            false,
            AnimOccupationOperation::Clear,
        );
        assert_eq!(grid.deck_bits(7, 8), 0);

        apply_anim_raw_occupation(
            &mut grid,
            7,
            8,
            0x10,
            416,
            0,
            false,
            AnimOccupationOperation::Mark,
        );
        assert_eq!(grid.ground_bits(7, 8), 0x10);
        apply_anim_raw_occupation(
            &mut grid,
            7,
            8,
            0x10,
            416,
            0,
            false,
            AnimOccupationOperation::Clear,
        );
        assert_eq!(
            grid.ground_bits(7, 8),
            0x10,
            "height-only clear targets deck after a nonstructural ground mark"
        );
        assert_eq!(grid.deck_bits(7, 8), 0);
    }

    #[test]
    fn delay_guard_and_rate_use_passive_frame_anchor() {
        let rules = runtime_rules("[TEST]\nRate=450\nEnd=3\nLoopCount=1\n", &[("TEST", 3)]);
        let mut sim = Simulation::new();
        let type_id = sim.interner.intern("TEST");
        let rng_before = sim.scenario_rng.logical_state();
        let id = sim
            .spawn_anim_object(&rules, runtime_descriptor(type_id, 1))
            .unwrap();

        sim.visit_anim(id, &rules); // constructor first-AI guard at frame 0
        sim.session.binary_frame = 1;
        sim.visit_anim(id, &rules); // delay 1 -> 0
        assert_eq!(sim.anim(id).unwrap().runtime.current_frame, 0);
        sim.session.binary_frame = 2;
        sim.visit_anim(id, &rules); // constructor-anchored timer is already due

        assert_eq!(sim.anim(id).unwrap().runtime.current_frame, 1);
        sim.visit_anim(id, &rules); // a second visit in the same frame cannot advance again
        assert_eq!(sim.anim(id).unwrap().runtime.current_frame, 1);
        assert_eq!(sim.scenario_rng.logical_state(), rng_before);
    }

    #[test]
    fn normalized_rate_uses_live_speed_after_random_rate_selection() {
        let rules = runtime_rules(
            "[TEST]\nRate=900\nRandomRate=180,225\nNormalized=yes\nEnd=3\nLoopCount=1\n",
            &[("TEST", 3)],
        );
        let mut sim = Simulation::new();
        sim.session.game_options.game_speed = 1;
        let type_id = sim.interner.intern("TEST");
        let expected_rng = sim.scenario_rng.clone();
        let expected_rate = sim.session.game_options.normalized_anim_delay(4);

        let id = sim
            .spawn_anim_object(&rules, runtime_descriptor(type_id, 0))
            .unwrap();
        let runtime = &sim.anim(id).unwrap().runtime;

        assert_eq!(runtime.rate_reload, expected_rate);
        assert_eq!(runtime.frame_timer.start_frame(), 0);
        assert_eq!(runtime.frame_timer.duration(), i32::from(expected_rate));
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state()
        );
    }

    #[test]
    fn stock_naweap_a_rate_normalizes_to_six_frames_at_speed_one() {
        let rules = runtime_rules(
            "[NAWEAP_A]\nRate=200\nNormalized=yes\nEnd=12\nLoopCount=-1\n",
            &[("NAWEAP_A", 12)],
        );
        let mut sim = Simulation::new();
        sim.session.game_options.game_speed = 1;
        let type_id = sim.interner.intern("NAWEAP_A");
        let id = sim
            .spawn_anim_object(&rules, runtime_descriptor(type_id, 0))
            .unwrap();

        assert_eq!(sim.anim(id).unwrap().runtime.rate_reload, 6);
        sim.visit_anim(id, &rules);
        sim.session.binary_frame = 5;
        sim.visit_anim(id, &rules);
        assert_eq!(sim.anim(id).unwrap().runtime.current_frame, 0);
        sim.session.binary_frame = 6;
        sim.visit_anim(id, &rules);
        assert_eq!(sim.anim(id).unwrap().runtime.current_frame, 1);
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
        sim.session.binary_frame = 1;
        sim.visit_anim(id, &rules); // frame 1
        sim.session.binary_frame = 2;
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

        sim.session.binary_frame = 3;
        sim.visit_anim(id, &rules); // SECOND frame 1 (Next does not restore guard)
        sim.session.binary_frame = 4;
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
    fn multiplayer_feedback_uses_sync_exempt_registry_without_global_id_or_logic_membership() {
        let rules = runtime_rules("[RING]\nRate=900\nEnd=1\nLoopCount=1\n", &[("RING", 1)]);
        let mut sim = Simulation::new();
        let next_global_id = sim.substrate.next_stable_object_id;
        let id = sim
            .spawn_multiplayer_feedback_anim_at_world(
                &rules,
                AnimWorldCoord {
                    x: 512,
                    y: 768,
                    z: 32,
                },
            )
            .unwrap();

        assert_eq!(sim.substrate.next_stable_object_id, next_global_id);
        assert!(!sim.substrate.anims.contains_key(id));
        assert!(sim.live_object_order_snapshot().is_empty());
        let anim = sim.anim(id).unwrap();
        assert_eq!(anim.native_unique_id, SYNC_EXEMPT_NATIVE_UNIQUE_ID);
        assert_eq!(anim.z_adjust, MULTIPLAYER_FEEDBACK_Z_ADJUST);
        assert!(!anim.in_logic_vector);
        let hash_with_feedback = sim.state_hash();
        let feedback = sim.substrate.multiplayer_feedback_anims.remove(id).unwrap();
        assert_eq!(sim.state_hash(), hash_with_feedback);
        assert!(
            sim.substrate
                .multiplayer_feedback_anims
                .insert(feedback)
                .is_none()
        );

        sim.for_each_multiplayer_feedback_anim(|sim, id| sim.visit_anim(id, &rules));
        assert!(!sim.anim(id).unwrap().runtime.first_ai_guard);
        sim.session.binary_frame = 1;
        sim.for_each_multiplayer_feedback_anim(|sim, id| sim.visit_anim(id, &rules));
        assert!(sim.anim(id).unwrap().runtime.inactive);
        assert_eq!(sim.substrate.multiplayer_feedback_pending_delete, vec![id]);

        sim.process_pending_delete();
        assert!(sim.anim(id).is_none());
        assert!(sim.substrate.multiplayer_feedback_pending_delete.is_empty());
    }

    #[test]
    fn owner_expiry_marks_anim_inactive_until_its_next_ai_visit() {
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
        assert_eq!(sim.anim(anim_id).unwrap().owner_entity, Some(building_id));

        assert!(sim.expire_anim_owner_reference(anim_id, building_id));
        assert!(
            sim.substrate
                .entities
                .get(building_id)
                .unwrap()
                .damage_fire_anim_ids[0]
                .is_none()
        );
        assert!(sim.anim(anim_id).unwrap().runtime.inactive);
        assert!(sim.live_object_order_snapshot().contains(&anim_id));
        assert!(!sim.substrate.pending_delete.contains(&anim_id));

        sim.visit_anim(anim_id, &rules);
        assert!(!sim.live_object_order_snapshot().contains(&anim_id));
        assert_eq!(sim.substrate.pending_delete, vec![anim_id]);
    }

    #[test]
    fn finalizer_defensively_clears_remaining_owner_slot() {
        let (mut sim, rules, building_id) = damage_fire_fixture(false);
        let fire_type = sim.interner.get("FIRE01").unwrap();
        let anim_id = sim
            .spawn_anim_at_world(
                &rules,
                runtime_descriptor(fire_type, 0),
                AnimWorldCoord { x: 0, y: 0, z: 0 },
            )
            .unwrap();
        sim.anim_mut_by_id(anim_id).unwrap().owner_entity = Some(building_id);
        sim.substrate
            .entities
            .get_mut(building_id)
            .unwrap()
            .damage_fire_anim_ids[0] = Some(anim_id);
        sim.anim_mut_by_id(anim_id).unwrap().runtime.inactive = true;
        sim.substrate.pending_delete.push(anim_id);

        sim.process_pending_delete();

        assert!(sim.anim(anim_id).is_none());
        assert!(!sim.live_object_order_snapshot().contains(&anim_id));
        assert!(
            sim.substrate
                .entities
                .get(building_id)
                .unwrap()
                .damage_fire_anim_ids[0]
                .is_none()
        );
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
        assert_eq!(first_anim.owner_entity, Some(building_id));
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
        assert_eq!(second_anim.owner_entity, Some(building_id));
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
                    draw_runtime: AnimDrawRuntime::default(),
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
