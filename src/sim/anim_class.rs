//! Scheduler-owned ordinary SHP animation objects.
//!
//! `AnimStore` owns animation storage while `world::LogicVector` owns live AI
//! order. This module implements only the verified ordinary non-bouncer
//! AnimClass lifecycle needed by building damage fire and destruction effects:
//! constructor/reveal, first-AI guard, logic-frame timing, loops,
//! reverse/ping-pong, Next, trailer, sound identity, conceal, and deferred
//! deletion.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::rules::art_data::{AnimLayer, AnimTypeRuntimeConfig};
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

impl AnimWorldCoord {
    /// Decompose the absolute lepton coordinate into the (cell, sub-cell,
    /// height-level) tuple the app projection consumes. The single owner of
    /// the decomposition and of the anim Z scale
    /// (`ANIM_HEIGHT_LEVEL_LEPTONS`): the anim sound path and the anim
    /// sprite path must decompose identically or a sound drifts away from
    /// its sprite.
    pub(crate) fn to_cell_sub_z(
        &self,
    ) -> (
        u16,
        u16,
        crate::util::fixed_math::SimFixed,
        crate::util::fixed_math::SimFixed,
        u8,
    ) {
        let rx = self
            .x
            .div_euclid(LEPTONS_PER_CELL)
            .clamp(0, i32::from(u16::MAX)) as u16;
        let ry = self
            .y
            .div_euclid(LEPTONS_PER_CELL)
            .clamp(0, i32::from(u16::MAX)) as u16;
        let sub_x =
            crate::util::fixed_math::SimFixed::from_num(self.x.rem_euclid(LEPTONS_PER_CELL));
        let sub_y =
            crate::util::fixed_math::SimFixed::from_num(self.y.rem_euclid(LEPTONS_PER_CELL));
        let z = self
            .z
            .div_euclid(ANIM_HEIGHT_LEVEL_LEPTONS)
            .clamp(0, i32::from(u8::MAX)) as u8;
        (rx, ry, sub_x, sub_y, z)
    }
}

const LEPTONS_PER_CELL: i32 = crate::util::lepton::LEPTONS_PER_CELL_I32;
const ANIM_HEIGHT_LEVEL_LEPTONS: i32 = 128;
const TRAILER_DRAW_FLAGS: u32 = 0x600;
const BUILDING_RENDER_ORIGIN_LEPTONS: i32 = 128;
const BUILDING_DESTRUCTION_SCATTER_RADIUS: i32 = 0x40;
const BUILDING_DESTRUCTION_DRAW_FLAGS: u32 = 0x600;
const DAMAGE_FIRE_SLOT_COUNT: usize = 8;
// Retained with the verified multiplayer-feedback spawn seam until command
// feedback owns its production call site.
#[allow(dead_code)]
const MULTIPLAYER_FEEDBACK_Z_ADJUST: i32 = -5000;
#[allow(dead_code)]
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
    /// Constructor-bound `AnimTypeClass+0x364` layer. Owner attachment still
    /// overrides this to Ground at query time, exactly as GetLayer does.
    pub native_display_layer: i8,
    /// World leptons — but **owner-relative whenever `owner_entity` is set**,
    /// exactly as `AnimClass::SetOwnerObject @ 0x00424B50` stores it. Read it
    /// through [`Simulation::anim_absolute_coord`] for a world position; read
    /// the field directly only where native reads the stored `ObjectClass`
    /// coordinate directly, which is the multiplayer sync checksum
    /// (`Compute_Game_Sync_Checksum @ 0x0064DAB0` folds `+0x9c`/`+0xa0`) and
    /// the state hash. Z uses the animation constructor's 128-lepton height
    /// level, not combat's terrain-height conversion.
    pub world_coord: AnimWorldCoord,
    pub draw_flags: u32,
    pub z_adjust: i32,
    pub effective_end: i32,
    pub effective_loop_end: i32,
    pub runtime: AnimRuntime,
    pub draw_runtime: AnimDrawRuntime,
    /// AnimClass `+0x196`: use the containing CellClass draw/palette authority.
    #[serde(default)]
    pub use_cell_drawer: bool,
    /// AnimClass `+0x197`: created from a terrain tile animation descriptor.
    #[serde(default)]
    pub terrain_attached: bool,
    /// Exact producer identity for Building `Explosion=` Start scorch/crater
    /// work. This cannot be inferred from AnimType because the same type may be
    /// constructed by a caller whose Start subset has not yet been audited.
    #[serde(default)]
    pub building_explosion_start_smudge: bool,
    /// LogicClass membership is reconstructed from the serialized vector.
    /// ObjectClass::Save does not persist its local membership byte.
    #[serde(skip)]
    pub in_logic_vector: bool,
    pub owner_entity: Option<u64>,
    pub start_sound_active: bool,
    pub stop_sound_id: Option<InternedId>,
}

fn native_anim_display_layer(layer: AnimLayer) -> i8 {
    match layer {
        AnimLayer::Ground => 2,
        AnimLayer::Air => 3,
        AnimLayer::Top => 4,
        AnimLayer::Other(layer) => i8::try_from(layer).unwrap_or(-1),
    }
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

    #[cfg(test)]
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
        self.spawn_anim_at_world_with_overlay_registry(rules, descriptor, world_coord, None)
    }

    pub(crate) fn spawn_anim_at_world_with_overlay_registry(
        &mut self,
        rules: &RuleSet,
        descriptor: AnimClassSpawnDescriptor,
        world_coord: AnimWorldCoord,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
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
            native_display_layer: native_anim_display_layer(config.layer),
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
            use_cell_drawer: descriptor.use_cell_drawer,
            terrain_attached: descriptor.terrain_attached,
            building_explosion_start_smudge: descriptor.building_explosion_start_smudge,
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
            self.anim_middle(stable_id, rules, &config, overlay_registry);
        }
        Ok(stable_id)
    }

    /// Construct the two verified Building destruction animation arms inline.
    /// The returned IDs are in native construction order and exist mainly for
    /// focused executable checks; production ownership is the AnimStore and
    /// global LogicVector established by `spawn_anim_at_world`.
    ///
    /// gamemd-derived: `BuildingClass::DestructionEffects @ 0x004415F0`.
    /// The per-foundation arm is `0x0044194D..0x00441A24`; the DestroyAnim arm
    /// is `0x00441CB2..0x00441D66`.
    pub(crate) fn spawn_building_destruction_anims(
        &mut self,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
        location: AnimWorldCoord,
        foundation: &str,
        explosion_anims: &[String],
        destroy_anims: &[String],
    ) -> Vec<AnimId> {
        let mut spawned = Vec::new();
        let render_x = location.x.wrapping_sub(BUILDING_RENDER_ORIGIN_LEPTONS);
        let render_y = location.y.wrapping_sub(BUILDING_RENDER_ORIGIN_LEPTONS);

        for (dx, dy) in crate::rules::foundation::foundation_cell_offsets(foundation) {
            if explosion_anims.is_empty() {
                continue;
            }

            let center_x = (render_x >> 8)
                .wrapping_add(i32::from(dx))
                .wrapping_mul(LEPTONS_PER_CELL)
                .wrapping_add(crate::util::lepton::CELL_CENTER_LEPTON_I32);
            let center_y = (render_y >> 8)
                .wrapping_add(i32::from(dy))
                .wrapping_mul(LEPTONS_PER_CELL)
                .wrapping_add(crate::util::lepton::CELL_CENTER_LEPTON_I32);
            let (world_x, world_y) = crate::sim::combat::random_direction_coord(
                &mut self.scenario_rng,
                center_x,
                center_y,
                BUILDING_DESTRUCTION_SCATTER_RADIUS,
            );

            // Rust's allocator has no recoverable native null-allocation seam.
            // On the ordinary success path the ranged delay and raw modulo
            // selection follow the already-consumed scatter draw exactly.
            let delay = self.scenario_rng.next_range_u32_inclusive(0, 3) as u16;
            let index = (self.scenario_rng.next_u32() % explosion_anims.len() as u32) as usize;
            let type_name = self.interner.intern(&explosion_anims[index]);
            let mut descriptor = AnimClassSpawnDescriptor::new(
                type_name,
                0,
                0,
                crate::util::fixed_math::SIM_ZERO,
                crate::util::fixed_math::SIM_ZERO,
                0,
            );
            descriptor.delay = delay;
            descriptor.draw_flags = BUILDING_DESTRUCTION_DRAW_FLAGS;
            descriptor.building_explosion_start_smudge = true;
            let id = self
                .spawn_anim_at_world_with_overlay_registry(
                    rules,
                    descriptor,
                    AnimWorldCoord {
                        x: world_x,
                        y: world_y,
                        z: location.z,
                    },
                    overlay_registry,
                )
                .expect("Building Explosion AnimType roots must be bound during scenario load");
            spawned.push(id);
        }

        if !destroy_anims.is_empty() {
            let index = (self.scenario_rng.next_u32() % destroy_anims.len() as u32) as usize;
            let type_name = self.interner.intern(&destroy_anims[index]);
            let mut descriptor = AnimClassSpawnDescriptor::new(
                type_name,
                0,
                0,
                crate::util::fixed_math::SIM_ZERO,
                crate::util::fixed_math::SIM_ZERO,
                0,
            );
            descriptor.draw_flags = BUILDING_DESTRUCTION_DRAW_FLAGS;
            let id = self
                .spawn_anim_at_world_with_overlay_registry(
                    rules,
                    descriptor,
                    AnimWorldCoord {
                        x: render_x,
                        y: render_y,
                        z: location.z,
                    },
                    overlay_registry,
                )
                .expect("Building DestroyAnim roots must be bound during scenario load");
            spawned.push(id);
        }

        spawned
    }

    // The move-feedback producer is not wired yet; keep the verified
    // sync-exempt allocation path available for that activation slice.
    #[allow(dead_code)]
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
            native_display_layer: native_anim_display_layer(config.layer),
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
            use_cell_drawer: false,
            terrain_attached: false,
            building_explosion_start_smudge: false,
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
        self.anim_middle(stable_id, rules, &config, None);
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
        self.visit_anim_with_overlay_registry(id, rules, None);
    }

    pub(crate) fn visit_anim_with_overlay_registry(
        &mut self,
        id: AnimId,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) {
        // `AnimClass::GetCoords @ 0x00422BE0`, not the stored field: an
        // owner-attached anim stores an owner-relative delta.
        let Some(world_coord) = self.anim_absolute_coord(id) else {
            return;
        };
        let Some((type_id, first_guard, inactive)) = self.anim(id).map(|anim| {
            (
                anim.type_id,
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
            ) && rules
                .art_registry
                .anim_runtime_config(trailer_name)
                .is_some()
            {
                let trailer_type = self.interner.intern(trailer_name);
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
                    use_cell_drawer: false,
                    terrain_attached: false,
                    building_explosion_start_smudge: false,
                    draw_runtime: AnimDrawRuntime::default(),
                };
                self.spawn_anim_at_world(rules, descriptor, world_coord)
                    .expect("validated trailer closure must remain spawnable");
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
        let delay_transition = {
            let Some(anim) = self.anim_mut_by_id(id) else {
                return;
            };
            if anim.runtime.delay_remaining > 0 {
                anim.runtime.delay_remaining -= 1;
                Some(anim.runtime.delay_remaining == 0)
            } else {
                None
            }
        };
        if let Some(reached_zero) = delay_transition {
            if reached_zero {
                self.anim_middle(id, rules, &config, overlay_registry);
            }
            // Native returns from the AI visit that handles the delay whether
            // or not it reached zero; frame advancement begins on a later visit.
            return;
        }
        {
            let Some(anim) = self.anim_mut_by_id(id) else {
                return;
            };
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
            VisitAction::Next(next) => self.switch_anim_type(id, &next, rules, overlay_registry),
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
        // `AnimClass::GetCoords @ 0x00422BE0` — the sound plays at the anim's
        // resolved world position, not its owner-relative stored delta.
        let Some(world) = self.anim_absolute_coord(id) else {
            return;
        };
        let Some(stop_sound) = self.anim(id).map(|anim| anim.stop_sound_id) else {
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

    /// Clear the owner link both ways. This is the `AnimClass::Destroy @
    /// 0x004255B0` order — owner callback (the owner's vtable `+0x60`, whose
    /// Techno/Object implementation is `FUN_00710410`; here the
    /// `damage_fire_anim_ids` slot clear) before the owner pointer itself.
    ///
    /// gamemd-derived: `AnimClass::SetOwnerObject @ 0x00424B50` — the attach
    /// half. Native stores an attached anim's coordinate RELATIVE to its owner
    /// and resolves it back on read, so the anim follows a moving owner for
    /// free. The disassembly is explicit about the sign and the axes: at
    /// `0x00424C16` it takes the anim's own `GetCoords` (vtable `+0x48`, with
    /// the owner pointer still null so this is the stored absolute), at
    /// `0x00424C37` it writes the owner into `Anim+0xCC`, at `0x00424C46` it
    /// takes the owner's `GetCoords`, then `SUB EBX,ECX` / `SUB EBP,EDI` /
    /// `SUB ECX,EDX` (`0x00424C51`, `0x00424C5F`, `0x00424C5B`) form
    /// `anim_abs - owner_abs` on X, Y and Z and hand that to `SetCoords`
    /// (vtable `+0x1B4`) at `0x00424C70`. The detach half at `0x00424BBD`
    /// mirrors it: read `GetCoords` while the owner is still set — so it comes
    /// back absolute — null `Anim+0xCC`, and store the absolute.
    ///
    /// Three native steps are deliberately not modelled, and it matters what
    /// each one actually is:
    /// - The owner's "has an anim attached" byte. Attach sets
    ///   `[owner+0x84] = 1` at `0x00424C30`; detach clears it at `0x00424BB6`,
    ///   but only when the `g_AnimClass_Array` scan at `0x00424B85` finds no
    ///   OTHER anim sharing that owner. Nothing in this engine reads
    ///   `Object+0x84`, so neither write has an observable consequence here.
    ///   The multi-slot case the scan exists for is already correct for a
    ///   different reason: [`Self::detach_anim_from_owner`] clears only the
    ///   slot equal to the departing anim.
    /// - The same guard also gates a virtual call on the OWNER,
    ///   `CALL [owner_vtable+0x17C]` at `0x00424BB0` — read directly, not
    ///   inferred: BuildingClass's primary vtable base is `0x007E3EBC` and
    ///   `+0x17C` there is `0x005F43C0`, whose body is a bare `RET`. So for the
    ///   only producer this engine has, the skipped call does nothing. A
    ///   non-building owner may override it; whoever adds the first such
    ///   producer must read `+0x17C` on that class before reusing this.
    /// - The `DisplayClass::RemoveFromLayer` / `Submit_Object` re-registration
    ///   pair either side of the pointer write (`0x004A9770` / `0x004A9720`).
    ///   The persistent flat-display registry below performs that exact
    ///   remove/reappend when owner attachment changes GetLayer.
    ///
    /// Returns `false` when the anim does not exist, or when the requested
    /// owner does not.
    pub(crate) fn set_anim_owner_object(&mut self, id: AnimId, new_owner: Option<u64>) -> bool {
        if self.anim(id).is_none() {
            return false;
        }

        // Detach half: resolve back to absolute *before* dropping the owner.
        if self.anim(id).and_then(|anim| anim.owner_entity).is_some() {
            let absolute = self
                .anim_absolute_coord(id)
                .expect("anim exists: checked at the head of this function");
            if let Some(anim) = self.anim_mut_by_id(id) {
                anim.owner_entity = None;
                anim.world_coord = absolute;
            }
        }

        // Attach half: the stored coordinate becomes the owner-relative delta.
        if let Some(owner_id) = new_owner {
            let Some(owner_coord) = self.anim_owner_coords(owner_id) else {
                return false;
            };
            if let Some(anim) = self.anim_mut_by_id(id) {
                anim.world_coord = AnimWorldCoord {
                    x: anim.world_coord.x.wrapping_sub(owner_coord.x),
                    y: anim.world_coord.y.wrapping_sub(owner_coord.y),
                    z: anim.world_coord.z.wrapping_sub(owner_coord.z),
                };
                anim.owner_entity = Some(owner_id);
            }
        }
        self.sync_display_layer_for_object(id);
        true
    }

    /// gamemd-derived: `AnimClass::GetCoords @ 0x00422BE0` — with an owner at
    /// `Anim+0xCC` it returns `stored + owner->GetCoords()`, otherwise the
    /// stored coordinate unchanged. This is the only correct way to read an
    /// anim's world position: [`AnimObject::world_coord`] is owner-relative
    /// whenever `owner_entity` is set, exactly as native stores it.
    ///
    /// Returns `None` only when the anim does not exist.
    ///
    /// VERA-internal, gamemd equivalent UNREACHABLE: an anim naming an owner
    /// the store no longer holds is treated as unattached, so the stored
    /// coordinate is returned as-is. Native cannot produce that state —
    /// `AnimClass::Detach @ 0x00425150` runs from the owner's own uninit, via
    /// `ObjectClass::Detach_From_All_Lists`, before the pointer can dangle, and
    /// this engine mirrors it in `expire_anim_owner_reference`. The fallback
    /// exists so every caller agrees on one behaviour instead of one panicking
    /// and another silently dropping the anim; treating a dangling owner as no
    /// owner is also the only reading under which the stored coordinate means
    /// anything.
    pub fn anim_absolute_coord(&self, id: AnimId) -> Option<AnimWorldCoord> {
        let anim = self.anim(id)?;
        let Some(owner_coord) = anim
            .owner_entity
            .and_then(|owner| self.anim_owner_coords(owner))
        else {
            return Some(anim.world_coord);
        };
        Some(AnimWorldCoord {
            x: anim.world_coord.x.wrapping_add(owner_coord.x),
            y: anim.world_coord.y.wrapping_add(owner_coord.y),
            z: anim.world_coord.z.wrapping_add(owner_coord.z),
        })
    }

    /// The owner side of `AnimClass::GetCoords`: `ObjectClass::GetCoords` on
    /// the attached-to object, in the anim coordinate frame.
    ///
    /// X and Y are leptons, with `BuildingClass::GetCoords @ 0x00447AC0`'s
    /// `(W-1) * 128` / `(H-1) * 128` shift off the stored NW anchor onto the
    /// geometric foundation centre — the same derivation
    /// `world/lifecycle.rs`'s `object_get_coords_cell` and `combat`'s
    /// `target_coords` use.
    ///
    /// Z is native ObjectClass coordinate leptons. Descriptor construction from
    /// an authored AnimType height level uses 128 per level at that separate
    /// boundary, but owner attachment subtracts the owner's exact live Object
    /// coordinate. This distinction becomes active for CaptureUnit rings on
    /// airborne/moving victims.
    pub(crate) fn anim_owner_coords(&self, owner_id: u64) -> Option<AnimWorldCoord> {
        let owner = self.substrate.entities.get(owner_id)?;
        let mut x = i32::from(owner.position.rx)
            .wrapping_mul(LEPTONS_PER_CELL)
            .wrapping_add(owner.position.sub_x.to_num::<i32>());
        let mut y = i32::from(owner.position.ry)
            .wrapping_mul(LEPTONS_PER_CELL)
            .wrapping_add(owner.position.sub_y.to_num::<i32>());
        if owner.category == crate::map::entities::EntityCategory::Structure {
            let (width, height) =
                crate::rules::foundation::foundation_dimensions(&owner.foundation);
            x = x.wrapping_add(
                i32::from(width.saturating_sub(1)).wrapping_mul(BUILDING_RENDER_ORIGIN_LEPTONS),
            );
            y = y.wrapping_add(
                i32::from(height.saturating_sub(1)).wrapping_mul(BUILDING_RENDER_ORIGIN_LEPTONS),
            );
        }
        Some(AnimWorldCoord {
            x,
            y,
            z: owner.position.exact_z_leptons.unwrap_or_else(|| {
                i32::from(owner.position.z)
                    .wrapping_mul(crate::util::lepton::GROUND_LEVEL_HEIGHT_LEPTONS)
            }),
        })
    }

    /// gamemd-derived: `AnimClass::SetOwnerObject @ 0x00424B50` detach half,
    /// plus this engine's damage-fire slot bookkeeping. See
    /// [`Self::set_anim_owner_object`] for the coordinate contract; the slot
    /// clear below is why the native `g_AnimClass_Array` shared-owner scan has
    /// nothing to guard here.
    pub(crate) fn detach_anim_from_owner(&mut self, id: AnimId) -> Option<u64> {
        let owner_id = self.anim(id).and_then(|anim| anim.owner_entity)?;
        if let Some(owner) = self.substrate.entities.get_mut(owner_id) {
            for slot in &mut owner.damage_fire_anim_ids {
                if *slot == Some(id) {
                    *slot = None;
                }
            }
            if owner.mind_control_anim_id == Some(id) {
                owner.mind_control_anim_id = None;
            }
        }
        self.set_anim_owner_object(id, None);
        Some(owner_id)
    }

    /// Owner expiry, dispatched from `ObjectClass::UnInit -> Detach_From_All_Lists`
    /// before the owner's conceal and alive clear. `AnimClass::Detach @
    /// 0x00425150` removes the anim from its display layer, calls the owner
    /// callback, clears the owner pointer, sets the detached marker
    /// `AnimClass+0x19B = 1`, and calls anim vtable `+0x124(0)`.
    ///
    /// `Detach` itself neither destroys nor deactivates — that is deferred into
    /// the marker. `AnimClass::AI @ 0x00423AC0` reloads `+0x19B` at `0x0042435F`
    /// and `JNZ 0x00424B38`, whose two instructions call anim vtable `+0xF8`
    /// (`AnimClass::Destroy`) and return. `runtime.inactive` is that marker: set
    /// here, checked at the head of `visit_anim`, which calls `destroy_anim` and
    /// returns. The `StopSound` frame and the pending-delete frame therefore
    /// land where native puts them, because `destroy_anim` owns both; draw
    /// suppression is owned separately by the `DisplayRemove` push below plus
    /// the `runtime.inactive` skip in
    /// `app/presentation/instances/overlays.rs`, mirroring native hiding
    /// through `DisplayClass::RemoveFromLayer` rather than through a `DrawIt`
    /// branch on `+0x19B`. The marker is serialized, matching native
    /// `SaveExtras`; it suppresses the trailer spawn, because `visit_anim`
    /// gates ahead of the trailer block exactly as native guards its trailer
    /// block on `+0x19B == 0` at `0x004242B0`; and the `Next=` transition
    /// clears it, matching native's `+0x19B = 0` write there.
    ///
    /// DRIFT (GSI-05.12) — the marker is checked earlier in the visit than
    /// native checks it, and the difference has no observable surface here yet.
    /// The gate sits 0x89F bytes into `AnimClass::AI`
    /// (`0x00423AC0`..`0x0042435F`), so native runs a prefix on a detached anim
    /// that `visit_anim` skips: `AnimClass::UpdateLoopingSound @ 0x00750D40`
    /// (entered on `Anim+0x198 == 0` and `AnimType+0x2F8 != -1`); `BounceAI`
    /// plus the `AnimType+0x354` `ObjectClass::AI` call; the bouncer-impact
    /// block on instance byte `+0x194`, which itself contains
    /// `AnimClass::Constructor` calls and an `Apply_area_damage` call; the
    /// `+0x19D` draw-suppression writers at `0x00423B5C`,
    /// `0x00423B7F`/`0x00423B88` and `0x00423BB8`/`0x00423BC1`; and the `+0x11B`
    /// frame-equality cleanup at `0x00423C03`..`0x00423C1D`. `visit_anim` runs
    /// only the make-infantry occupation mark before its own gate.
    ///
    /// Four of those five items are inert in gamemd itself for this trigger,
    /// and the fifth is inert only here, which is why moving the gate now would
    /// be a no-op rather than a fix:
    /// - `BounceAI` plus the `AnimType+0x354` `ObjectClass::AI` call never run.
    ///   `AnimType+0x354` is `IsFlamingGuy=` (`AnimTypeClass::ReadINI` read @
    ///   `0x004282E2`, store @ `0x004282FC`, key string @ `0x818448`), and
    ///   retail `artmd.ini` sets it on `[FLAMEGUY]` alone — never on
    ///   `FIRE01/02/03`, the anims this trigger detaches.
    /// - The `+0x11B` frame-equality cleanup at `0x00423C03`..`0x00423C1D` is
    ///   dead code in gamemd. The byte is never written non-zero anywhere in
    ///   the image: its only AnimClass writers are `AnimClass::Constructor @
    ///   0x00421F5F` and `@ 0x004227A7`, both storing `BL` inside zero-init
    ///   runs dominated by `XOR EBX,EBX` (`0x00421EB6` / `0x004225FC`), plus
    ///   the `= 0` at `AnimClass::AI @ 0x00423C1D`.
    /// - The bouncer-impact block needs a bounce simulation, and there is none:
    ///   `Bouncer=` is parsed into `art_data.rs`'s `bouncer` flag with no sim
    ///   consumer, and damage-fire anims are not bouncers in any case.
    /// - The `+0x19D` writes cannot reach a `DrawIt` even in native — the same
    ///   call destroys the anim a few instructions later.
    /// - `UpdateLoopingSound` is the one item that really runs in native:
    ///   `[FIRE01]`/`[FIRE02]` carry `StartSound=BuildingFireBig` and
    ///   `[FIRE03]` `StartSound=BuildingFireMed`, so `AnimType+0x2F8 != -1`
    ///   holds. It is pure maintenance of a loop handle — revalidate, recompute
    ///   volume and pan through `VocClass::CalcVolumeAndPan`, stop the loop when
    ///   the volume comes back non-positive — and this engine has no loop-handle
    ///   mechanism to maintain (recorded on `audio/sfx.rs`), so there is nothing
    ///   for the extra pass to act on.
    ///
    /// - Trigger: a building destroyed while its damage-fire anims are live —
    ///   in this engine, `expire_anim_owner_reference` is the only producer of
    ///   the marked state.
    /// - Player effect: none today; audible once loop handles exist, and what
    ///   that final pass emits is the corpus's own open item, OQ-09 in
    ///   `ANIMCLASS_DETACHEDOWNER_MARKER_0X19B_CONSUMERS_GHIDRA_REPORT.md`.
    /// - Frequency: every destroyed building that had reached a damage-fire
    ///   threshold, so several times in an ordinary skirmish — with zero
    ///   observable output until loop handles exist.
    /// - Downstream risk: this is sequenced, not open-ended. It becomes
    ///   observable exactly when `GSI-15.03`'s loop-handle mechanism lands, and
    ///   the gate must move in the same slice that lands it. Only
    ///   `UpdateLoopingSound` has to be re-argued when that slice arrives; the
    ///   other four are settled above.
    ///
    /// Separately unmodelled, and not a consequence of the ordering above: the
    /// `Rules+0x147C` occupied-cell path at `0x00424322`..`0x0042435E` is a
    /// second writer of the marker — native re-reads coords, resolves the cell
    /// through `MapClass::Get_CellClass_At_Coord @ 0x00565730`, tests it via
    /// `0x0047C520`, and sets `+0x19B = 1` at `0x00424358`. A third writer, the
    /// `AnimType+0x360` path entered at `0x004243C2` and writing at
    /// `0x00424427` sits after the gate. `AnimType+0x360` is
    /// `IsAnimatedTiberium=`, and that writer is the one native path which
    /// routinely runs a full prefix in the marked state — unmodelled here only
    /// because `art_data.rs`'s `is_animated_tiberium`/`hide_if_no_ore` have no
    /// `sim/` consumer. Neither writer has an analogue in this store.
    ///
    /// DRIFT (GSI-05.12, structural) — `runtime.inactive` doubles as this
    /// store's "ready for pending delete" predicate
    /// (`world/lifecycle.rs pending_object_is_ready`), so the marker and object
    /// liveness are one field where native keeps display-layer membership
    /// separate from both.
    /// - Trigger: an attached anim that must outlive its owner.
    /// - Player effect: none. No such producer exists — every native attach
    ///   producer either dies with its owner or is not built here.
    /// - Frequency: zero occurrences in this build.
    /// - Downstream risk: recorded because splitting the two fields is a
    ///   prerequisite for any future producer whose anim survives its owner;
    ///   nothing else depends on it.
    pub(crate) fn expire_anim_owner_reference(&mut self, id: AnimId, expired_id: u64) -> bool {
        if self.anim(id).and_then(|anim| anim.owner_entity) != Some(expired_id) {
            return false;
        }
        self.lifecycle_outputs
            .push(LifecycleOutput::DisplayRemove { stable_id: id });
        self.substrate.flat_display_order.remove(id);
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

    /// Apply CellClass's producer-owned `AnimClass +0x100` write after the
    /// delay-zero constructor has already run `Middle`.
    pub(crate) fn set_terrain_anim_z_adjust_after_construction(
        &mut self,
        id: AnimId,
        z_adjust: i32,
    ) -> bool {
        let Some(anim) = self.anim_mut_by_id(id) else {
            return false;
        };
        if !anim.terrain_attached || anim.z_adjust != 0 {
            return false;
        }
        anim.z_adjust = z_adjust;
        true
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
                use_cell_drawer: false,
                terrain_attached: false,
                building_explosion_start_smudge: false,
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
            self.set_anim_owner_object(anim_id, Some(building_id));
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

    fn anim_middle(
        &mut self,
        id: AnimId,
        rules: &RuleSet,
        config: &AnimTypeRuntimeConfig,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) {
        let sound_name = config
            .start_sound
            .as_ref()
            .or(config.report.as_ref())
            .cloned();
        if let Some(sound_name) = sound_name {
            let sound_id = self.interner.intern(&sound_name);
            if let Some(world) = self.anim_absolute_coord(id) {
                if let Some(anim) = self.anim_mut_by_id(id) {
                    anim.start_sound_active = true;
                }
                self.sound_events.push(SimSoundEvent::AnimationStarted {
                    anim_id: id,
                    sound_id,
                    world,
                });
            }
        }
        if config.start == 0
            && self
                .anim(id)
                .is_some_and(|anim| anim.building_explosion_start_smudge)
        {
            self.dispatch_building_explosion_anim_start_smudge(id, rules, overlay_registry);
        }
    }

    fn switch_anim_type(
        &mut self,
        id: AnimId,
        next: &str,
        rules: &RuleSet,
        overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    ) {
        let Some(config) = rules.art_registry.anim_runtime_config(next).cloned() else {
            self.destroy_anim(id);
            return;
        };
        let Ok((effective_end, effective_loop_end)) = effective_bounds(next, &config) else {
            self.destroy_anim(id);
            return;
        };
        let type_id = self.interner.intern(next);
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
        self.anim_middle(id, rules, &config, overlay_registry);
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

fn native_loop_remaining(loop_count: i32, constructor_loop: i32) -> u8 {
    // gamemd-derived: `AnimClass::Constructor @ 0x00421EA0`, branch at
    // 0x004226BF. The constructor argument is compared as signed before its
    // low byte participates in the wrapping LoopCount multiplication.
    let constructor_factor = if constructor_loop > 1 {
        constructor_loop as u8
    } else {
        1
    };
    (loop_count as u8).wrapping_mul(constructor_factor).max(1)
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
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::art_data::ArtRegistry;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;

    #[test]
    fn gsi_13_04_signed_constructor_loop_preserves_negative_one_distinction() {
        assert_eq!(native_loop_remaining(0, 1), 1);
        assert_eq!(native_loop_remaining(1, 1), 1);
        assert_eq!(native_loop_remaining(2, 1), 2);
        assert_eq!(native_loop_remaining(-1, 1), u8::MAX);
        assert_eq!(native_loop_remaining(128, 2), 1);
        assert_eq!(native_loop_remaining(-1, -1), u8::MAX);
        assert_eq!(native_loop_remaining(-1, 255), 1);
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

    fn building_start_smudge_rules() -> (
        RuleSet,
        crate::map::overlay_types::OverlayTypeRegistry,
    ) {
        let ini = IniFile::from_str(
            "[Tiberiums]\n0=Riparius\n\
             [Riparius]\nImage=1\nValue=25\n\
             [OverlayTypes]\n0=ORE\n\
             [ORE]\nTiberium=yes\n\
             [SmudgeTypes]\n1=CR1\n2=BURN1\n\
             [CR1]\nCrater=yes\nWidth=1\nHeight=1\n\
             [BURN1]\nBurn=yes\nWidth=1\nHeight=1\n",
        );
        let mut rules = RuleSet::from_ini(&ini).expect("Building Start-smudge rules");
        let mut art = ArtRegistry::from_ini(&IniFile::from_str(
            "[EXP]\nRate=450\nEnd=4\nScorch=yes\nCrater=yes\n\
             FrameWidth=100\nFrameHeight=100\n",
        ));
        art.bind_anim_frame_count_for_test("EXP", 4);
        rules.art_registry = art;
        let overlay_registry =
            crate::map::overlay_types::OverlayTypeRegistry::from_ini(&ini, None);
        (rules, overlay_registry)
    }

    fn building_start_flat_terrain(width: u16, height: u16) -> ResolvedTerrainGrid {
        let mut cells = Vec::with_capacity(usize::from(width) * usize::from(height));
        for ry in 0..height {
            for rx in 0..width {
                cells.push(ResolvedTerrainCell {
                    rx,
                    ry,
                    source_tile_index: 0,
                    source_sub_tile: 0,
                    final_tile_index: 0,
                    final_sub_tile: 0,
                    is_wood_bridge_repair_tile: false,
                    level: 0,
                    filled_clear: true,
                    tileset_index: Some(0),
                    land_type: 0,
                    yr_cell_land_type: 0,
                    slope_type: 0,
                    template_height: 0,
                    height_in_pixels: 0,
                    render_offset_x: 0,
                    render_offset_y: 0,
                    terrain_class: Default::default(),
                    speed_costs: crate::rules::terrain_rules::SpeedCostProfile {
                        track: Some(100),
                        ..Default::default()
                    },
                    is_water: false,
                    is_cliff_like: false,
                    is_rough: false,
                    is_road: false,
                    accepts_smudge: true,
                    allows_tiberium: true,
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
                    bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
                    tube_index: None,
                    radar_left: [0; 3],
                    radar_right: [0; 3],
                    has_damaged_data: false,
                    bridgehead_anchor_class_at_load: None,
                });
            }
        }
        ResolvedTerrainGrid::from_cells(width, height, cells)
    }

    fn install_building_start_authorities(
        sim: &mut Simulation,
        overlay_registry: &crate::map::overlay_types::OverlayTypeRegistry,
        ore_cells: &[(u16, u16)],
    ) {
        const MAP_SIZE: u16 = 32;
        let ore_id = overlay_registry.id_for_name("ORE").expect("ORE overlay");
        let mut overlay_grid = crate::sim::overlay_grid::OverlayGrid::new(MAP_SIZE, MAP_SIZE);
        for &(rx, ry) in ore_cells {
            overlay_grid.place_overlay(rx, ry, ore_id, 5);
        }
        sim.overlay_grid = Some(overlay_grid);
        sim.resolved_terrain = Some(building_start_flat_terrain(MAP_SIZE, MAP_SIZE));
        sim.smudge_grid = Some(crate::sim::smudge_grid::SmudgeGrid::new(
            MAP_SIZE, MAP_SIZE,
        ));
        sim.production.ore_growth_state =
            crate::sim::ore_growth::OreGrowthState::new(MAP_SIZE, MAP_SIZE);
    }

    fn oracle_building_scatter(
        rng: &mut crate::sim::rng::SimRng,
        center_x: i32,
        center_y: i32,
    ) -> (i32, i32) {
        let byte = (rng.next_u32() & 0xff) as u8;
        crate::sim::combat::random_direction_coord_for_byte(
            byte,
            center_x,
            center_y,
            BUILDING_DESTRUCTION_SCATTER_RADIUS,
        )
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
            use_cell_drawer: false,
            terrain_attached: false,
            building_explosion_start_smudge: false,
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
    fn gsi_13_04_draw_and_terrain_attachment_state_roundtrip_and_hash() {
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
        descriptor.use_cell_drawer = true;
        descriptor.terrain_attached = true;
        descriptor.building_explosion_start_smudge = true;
        let draw_runtime = descriptor.draw_runtime;
        let id = sim.spawn_anim_object(&rules, descriptor).unwrap();
        assert_eq!(sim.anim(id).unwrap().draw_runtime, draw_runtime);

        let serialized = bincode::serialize(&sim.substrate.anims).unwrap();
        let restored: AnimStore = bincode::deserialize(&serialized).unwrap();
        assert_eq!(restored.get(id).unwrap().draw_runtime, draw_runtime);
        assert!(restored.get(id).unwrap().use_cell_drawer);
        assert!(restored.get(id).unwrap().terrain_attached);
        assert!(restored.get(id).unwrap().building_explosion_start_smudge);

        let before = sim.state_hash();
        sim.anim_mut_by_id(id)
            .unwrap()
            .building_explosion_start_smudge = false;
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
    fn building_explosion_delays_one_through_three_run_middle_on_zero_transition_once() {
        let rules = runtime_rules(
            "[TEST]\nRate=450\nEnd=3\nLoopCount=1\nReport=ExplosionReport\n",
            &[("TEST", 3)],
        );

        for delay in 1..=3 {
            let mut sim = Simulation::new();
            let type_id = sim.interner.intern("TEST");
            let id = sim
                .spawn_anim_object(&rules, runtime_descriptor(type_id, delay))
                .unwrap();
            assert!(sim.sound_events.is_empty());

            sim.visit_anim(id, &rules); // constructor first-AI guard
            assert!(sim.sound_events.is_empty());
            for visit in 1..=delay {
                sim.session.binary_frame = u32::from(visit);
                sim.visit_anim(id, &rules);
                if visit < delay {
                    assert!(sim.sound_events.is_empty());
                }
            }

            assert_eq!(sim.anim(id).unwrap().runtime.delay_remaining, 0);
            assert_eq!(sim.anim(id).unwrap().runtime.current_frame, 0);
            assert!(matches!(
                sim.sound_events.as_slice(),
                [SimSoundEvent::AnimationStarted { anim_id, .. }] if *anim_id == id
            ));

            sim.visit_anim(id, &rules);
            assert_eq!(
                sim.sound_events.len(),
                1,
                "Middle must not repeat after delay {delay} reaches zero"
            );
        }
    }

    #[test]
    fn phase3_building_delay_zero_start_smudge_interleaves_before_next_foundation_cell() {
        let (rules, overlay_registry) = building_start_smudge_rules();
        const SEED: u64 = 17;
        let mut sim = Simulation::new();
        sim.scenario_rng = crate::sim::rng::SimRng::new(SEED);
        install_building_start_authorities(&mut sim, &overlay_registry, &[(10, 20)]);

        let location = AnimWorldCoord {
            x: 10 * LEPTONS_PER_CELL + BUILDING_RENDER_ORIGIN_LEPTONS,
            y: 20 * LEPTONS_PER_CELL + BUILDING_RENDER_ORIGIN_LEPTONS,
            z: 0,
        };
        let mut oracle = crate::sim::rng::SimRng::new(SEED);
        let first_world = oracle_building_scatter(
            &mut oracle,
            10 * LEPTONS_PER_CELL + crate::util::lepton::CELL_CENTER_LEPTON_I32,
            20 * LEPTONS_PER_CELL + crate::util::lepton::CELL_CENTER_LEPTON_I32,
        );
        assert_eq!(oracle.next_range_u32_inclusive(0, 3), 0);
        oracle.next_u32(); // raw modulo selection from the one-entry Explosion list
        let start_roll = oracle.next_range_u32_inclusive(0, 0x7fff_fffe);
        assert!(
            start_roll >= 0x4000_0000,
            "fixture takes the crater arm so the production ore authority mutates"
        );
        let state_after_first_start = oracle.logical_state();
        let second_world = oracle_building_scatter(
            &mut oracle,
            11 * LEPTONS_PER_CELL + crate::util::lepton::CELL_CENTER_LEPTON_I32,
            20 * LEPTONS_PER_CELL + crate::util::lepton::CELL_CENTER_LEPTON_I32,
        );
        let second_delay = oracle.next_range_u32_inclusive(0, 3) as u16;
        assert_ne!(second_delay, 0, "only cell 1 may Start during construction");
        oracle.next_u32(); // cell 2 raw list selection

        let mut incorrectly_deferred = crate::sim::rng::SimRng::new(SEED);
        incorrectly_deferred.next_u32();
        incorrectly_deferred.next_range_u32_inclusive(0, 3);
        incorrectly_deferred.next_u32();
        let incorrectly_deferred_second_world = oracle_building_scatter(
            &mut incorrectly_deferred,
            11 * LEPTONS_PER_CELL + crate::util::lepton::CELL_CENTER_LEPTON_I32,
            20 * LEPTONS_PER_CELL + crate::util::lepton::CELL_CENTER_LEPTON_I32,
        );
        let incorrectly_deferred_second_delay =
            incorrectly_deferred.next_range_u32_inclusive(0, 3) as u16;
        incorrectly_deferred.next_u32();
        incorrectly_deferred.next_range_u32_inclusive(0, 0x7fff_fffe);

        let ids = sim.spawn_building_destruction_anims(
            &rules,
            Some(&overlay_registry),
            location,
            "2x1",
            &["EXP".to_string()],
            &[],
        );

        assert_eq!(ids.len(), 2);
        assert_eq!(
            sim.anim(ids[0]).unwrap().world_coord,
            AnimWorldCoord {
                x: first_world.0,
                y: first_world.1,
                z: 0,
            }
        );
        assert_eq!(
            sim.anim(ids[1]).unwrap().world_coord,
            AnimWorldCoord {
                x: second_world.0,
                y: second_world.1,
                z: 0,
            },
            "cell 2 scatter must observe cell 1's synchronous Start RNG draw"
        );
        assert_eq!(sim.anim(ids[0]).unwrap().runtime.delay_remaining, 0);
        assert_eq!(
            sim.anim(ids[1]).unwrap().runtime.delay_remaining,
            second_delay
        );
        assert_eq!(sim.scenario_rng.logical_state(), oracle.logical_state());
        assert_ne!(
            (second_world, second_delay),
            (
                incorrectly_deferred_second_world,
                incorrectly_deferred_second_delay
            ),
            "deferring Start would assign its RNG draw to cell 2 scatter/delay instead"
        );
        assert_ne!(
            sim.scenario_rng.logical_state(),
            state_after_first_start,
            "cell 2 scatter/delay/selection must follow the synchronous Start"
        );

        let crater_id = rules.smudge_types.find_by_name("CR1").unwrap();
        assert_eq!(
            sim.smudge_grid.as_ref().unwrap().cell(10, 20).type_id,
            Some(crater_id)
        );
        assert_eq!(
            sim.overlay_grid.as_ref().unwrap().cell(10, 20).overlay_id,
            None,
            "AnimClass::Start crater removes retail-style ore density 5 before smudge placement"
        );
    }

    #[test]
    fn phase3_building_delay_one_start_smudge_waits_for_zero_transition_once() {
        let (rules, overlay_registry) = building_start_smudge_rules();
        const SEED: u64 = 14;
        let mut sim = Simulation::new();
        sim.scenario_rng = crate::sim::rng::SimRng::new(SEED);
        install_building_start_authorities(&mut sim, &overlay_registry, &[(10, 20)]);
        let mut oracle = crate::sim::rng::SimRng::new(SEED);
        let world = oracle_building_scatter(
            &mut oracle,
            10 * LEPTONS_PER_CELL + crate::util::lepton::CELL_CENTER_LEPTON_I32,
            20 * LEPTONS_PER_CELL + crate::util::lepton::CELL_CENTER_LEPTON_I32,
        );
        assert_eq!(oracle.next_range_u32_inclusive(0, 3), 1);
        oracle.next_u32(); // raw one-entry list selection
        let construction_state = oracle.logical_state();

        let ids = sim.spawn_building_destruction_anims(
            &rules,
            Some(&overlay_registry),
            AnimWorldCoord {
                x: 10 * LEPTONS_PER_CELL + BUILDING_RENDER_ORIGIN_LEPTONS,
                y: 20 * LEPTONS_PER_CELL + BUILDING_RENDER_ORIGIN_LEPTONS,
                z: 0,
            },
            "1x1",
            &["EXP".to_string()],
            &[],
        );
        let id = ids[0];
        assert_eq!(
            sim.anim(id).unwrap().world_coord,
            AnimWorldCoord {
                x: world.0,
                y: world.1,
                z: 0,
            }
        );
        assert_eq!(sim.scenario_rng.logical_state(), construction_state);
        assert!(sim.smudge_grid.as_ref().unwrap().iter_occupied().next().is_none());
        assert_eq!(
            sim.overlay_grid.as_ref().unwrap().cell(10, 20).overlay_data,
            5
        );

        sim.visit_anim_with_overlay_registry(id, &rules, Some(&overlay_registry));
        assert_eq!(
            sim.scenario_rng.logical_state(),
            construction_state,
            "constructor first-AI guard must not run Start or consume RNG"
        );
        assert!(sim.smudge_grid.as_ref().unwrap().iter_occupied().next().is_none());
        assert_eq!(
            sim.overlay_grid.as_ref().unwrap().cell(10, 20).overlay_data,
            5
        );

        sim.session.binary_frame = 1;
        let start_roll = oracle.next_range_u32_inclusive(0, 0x7fff_fffe);
        assert!(start_roll >= 0x4000_0000, "fixture takes the crater arm");
        sim.visit_anim_with_overlay_registry(id, &rules, Some(&overlay_registry));
        assert_eq!(sim.anim(id).unwrap().runtime.delay_remaining, 0);
        assert_eq!(sim.scenario_rng.logical_state(), oracle.logical_state());
        let crater_id = rules.smudge_types.find_by_name("CR1").unwrap();
        assert_eq!(
            sim.smudge_grid.as_ref().unwrap().cell(10, 20).type_id,
            Some(crater_id)
        );
        assert_eq!(
            sim.overlay_grid.as_ref().unwrap().cell(10, 20).overlay_id,
            None
        );

        let state_after_start = sim.scenario_rng.logical_state();
        sim.visit_anim_with_overlay_registry(id, &rules, Some(&overlay_registry));
        assert_eq!(
            sim.scenario_rng.logical_state(),
            state_after_start,
            "Start must not repeat after the delay transition reaches zero"
        );
        assert_eq!(sim.smudge_grid.as_ref().unwrap().iter_occupied().count(), 1);
        assert_eq!(
            sim.overlay_grid.as_ref().unwrap().cell(10, 20).overlay_id,
            None
        );
    }

    #[test]
    fn phase3_building_destruction_walk_is_row_major_scenario_owned_and_scheduler_persistent() {
        let rules = runtime_rules(
            "[EXP_A]\nRate=450\nEnd=4\nReport=ExplosionA\n\
             [EXP_B]\nRate=450\nEnd=4\nReport=ExplosionB\n\
             [DEST]\nRate=300\nEnd=6\nShadow=yes\nAltPalette=yes\nLayer=ground\n",
            &[("EXP_A", 4), ("EXP_B", 4), ("DEST", 12)],
        );
        let mut sim = Simulation::new();
        sim.scenario_rng = crate::sim::rng::SimRng::new(0);
        sim.main_rng = crate::sim::rng::SimRng::new(91);
        let main_before = sim.main_rng.logical_state();
        let mut expected_rng = sim.scenario_rng.clone();
        let location = AnimWorldCoord {
            x: 10 * LEPTONS_PER_CELL + 128,
            y: 20 * LEPTONS_PER_CELL + 128,
            z: 731,
        };
        let explosions = vec!["EXP_A".to_string(), "EXP_B".to_string()];
        let destroys = vec!["DEST".to_string()];
        let mut expected = Vec::new();
        for (dx, dy) in crate::rules::foundation::foundation_cell_offsets("3x3Refinery") {
            let center_x = (location.x.wrapping_sub(128) >> 8)
                .wrapping_add(i32::from(dx))
                .wrapping_mul(LEPTONS_PER_CELL)
                .wrapping_add(128);
            let center_y = (location.y.wrapping_sub(128) >> 8)
                .wrapping_add(i32::from(dy))
                .wrapping_mul(LEPTONS_PER_CELL)
                .wrapping_add(128);
            let (x, y) = crate::sim::combat::random_direction_coord(
                &mut expected_rng,
                center_x,
                center_y,
                BUILDING_DESTRUCTION_SCATTER_RADIUS,
            );
            let delay = expected_rng.next_range_u32_inclusive(0, 3) as u16;
            let index = (expected_rng.next_u32() % explosions.len() as u32) as usize;
            expected.push((
                explosions[index].clone(),
                AnimWorldCoord { x, y, z: 731 },
                delay,
            ));
        }
        let destroy_index = (expected_rng.next_u32() % destroys.len() as u32) as usize;
        expected.push((
            destroys[destroy_index].clone(),
            AnimWorldCoord {
                x: location.x - 128,
                y: location.y - 128,
                z: 731,
            },
            0,
        ));

        let ids = sim.spawn_building_destruction_anims(
            &rules,
            None,
            location,
            "3x3Refinery",
            &explosions,
            &destroys,
        );

        assert_eq!(
            ids.len(),
            9,
            "eight holed-foundation cells plus DestroyAnim"
        );
        assert_eq!(
            sim.scenario_rng.logical_state(),
            expected_rng.logical_state()
        );
        assert_eq!(sim.main_rng.logical_state(), main_before);
        assert_eq!(sim.live_object_order_snapshot(), ids);
        for (index, (&id, (name, world, delay))) in ids.iter().zip(&expected).enumerate() {
            let anim = sim.anim(id).expect("scheduler-owned building AnimClass");
            assert_eq!(sim.interner.resolve(anim.type_id), name);
            assert_eq!(anim.world_coord, *world);
            assert_eq!(anim.draw_flags, 0x600);
            assert_eq!(anim.z_adjust, 0);
            assert_eq!(anim.runtime.delay_remaining, *delay);
            assert_eq!(anim.runtime.loop_remaining, 1);
            assert_eq!(anim.runtime.constructor_reverse, false);
            assert_eq!(anim.building_explosion_start_smudge, index < 8);
            assert!(anim.in_logic_vector);
        }
        assert!(
            expected[..8].iter().any(|(_, _, delay)| *delay == 0),
            "fixture must exercise synchronous constructor-time Middle"
        );
        let destroy = sim.anim(*ids.last().expect("DestroyAnim id")).unwrap();
        let destroy_config = rules.art_registry.anim_runtime_config("DEST").unwrap();
        assert_eq!(destroy.effective_end, 6, "Shadow halves the SHP body range");
        assert_eq!(
            destroy_config.layer,
            crate::rules::art_data::AnimLayer::Ground
        );
        assert!(destroy_config.shadow);
        assert!(destroy_config.alt_palette);

        let snapshot =
            crate::sim::snapshot::GameSnapshot::save(&sim, 0, 0, "building-destruction-anims", 0);
        let mut restored = crate::sim::snapshot::GameSnapshot::load(&snapshot)
            .expect("building animation snapshot")
            .sim;
        restored
            .restore_after_snapshot_load()
            .expect("building animation scheduler restore");
        assert_eq!(restored.live_object_order_snapshot(), ids);
        for &id in &ids {
            let before = sim.anim(id).unwrap();
            let after = restored.anim(id).unwrap();
            assert_eq!(after.type_id, before.type_id);
            assert_eq!(after.world_coord, before.world_coord);
            assert_eq!(after.draw_flags, before.draw_flags);
            assert_eq!(
                after.runtime.delay_remaining,
                before.runtime.delay_remaining
            );
            assert_eq!(
                after.building_explosion_start_smudge,
                before.building_explosion_start_smudge,
            );
        }
        let restored_hash = restored.state_hash();
        restored.anim_mut_by_id(ids[0]).unwrap().world_coord.x += 1;
        assert_ne!(restored.state_hash(), restored_hash);
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
    fn next_without_preintern_reuses_identity_runs_middle_and_destroy_is_idempotent() {
        let rules = runtime_rules(
            "[FIRST]\nRate=900\nEnd=2\nLoopCount=1\nNext=SECOND\nStartSound=FirstStart\n\n\
             [SECOND]\nRate=900\nEnd=2\nLoopCount=1\nReport=SecondReport\nStopSound=SecondStop\n",
            &[("FIRST", 2), ("SECOND", 2)],
        );
        let mut sim = Simulation::new();
        let first = sim.interner.intern("FIRST");
        assert!(sim.interner.get("SECOND").is_none());
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
    fn trailer_without_preintern_is_visited_and_guarded_in_same_live_walk() {
        let rules = runtime_rules(
            "[PARENT]\nRate=0\nEnd=2\nTrailerAnim=CHILD\nTrailerSeperation=1\n\n\
             [CHILD]\nRate=900\nEnd=2\nLoopCount=1\n",
            &[("PARENT", 2), ("CHILD", 2)],
        );
        let mut sim = Simulation::new();
        let parent_type = sim.interner.intern("PARENT");
        assert!(sim.interner.get("CHILD").is_none());
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
    fn gsi_05_12_attached_anim_follows_a_moving_owner_and_detaches_absolute() {
        // The point of owner-relative storage: `AnimClass::GetCoords @
        // 0x00422BE0` adds the owner's live coordinate, so the anim tracks the
        // owner without anyone rewriting the anim. `SetOwnerObject @
        // 0x00424B50` then writes the resolved absolute back on detach, so the
        // anim stays where it was standing.
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
            .expect("slot zero");
        let before = sim.anim_absolute_coord(anim_id).expect("attached anim");
        let stored_before = sim.anim(anim_id).unwrap().world_coord;

        // Move the owner one cell east and one cell south.
        {
            let owner = sim.substrate.entities.get_mut(building_id).unwrap();
            owner.position.rx += 1;
            owner.position.ry += 1;
        }

        assert_eq!(
            sim.anim(anim_id).unwrap().world_coord,
            stored_before,
            "the stored delta is untouched by owner movement"
        );
        assert_eq!(
            sim.anim_absolute_coord(anim_id).unwrap(),
            AnimWorldCoord {
                x: before.x + LEPTONS_PER_CELL,
                y: before.y + LEPTONS_PER_CELL,
                z: before.z
            },
            "the resolved coordinate follows the owner one cell on each axis"
        );

        let moved = sim.anim_absolute_coord(anim_id).unwrap();
        assert_eq!(sim.detach_anim_from_owner(anim_id), Some(building_id));
        assert!(sim.anim(anim_id).unwrap().owner_entity.is_none());
        assert_eq!(
            sim.anim(anim_id).unwrap().world_coord,
            moved,
            "detach writes the resolved absolute back into the stored field"
        );
        assert_eq!(sim.anim_absolute_coord(anim_id).unwrap(), moved);
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
        // `AnimClass::SetOwnerObject @ 0x00424B50` stores the coordinate
        // owner-relative; `GetCoords @ 0x00422BE0` resolves it back. The
        // absolute is what the draw and the sound see, and it is unchanged.
        assert_eq!(
            sim.anim_absolute_coord(first).unwrap(),
            AnimWorldCoord {
                x: 2450,
                y: 2653,
                z: 0
            }
        );
        assert_eq!(
            first_anim.world_coord,
            AnimWorldCoord {
                x: 2450 - 3072,
                y: 2653 - 3072,
                z: 0
            },
            "stored coordinate is the owner-relative delta native writes"
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
            sim.anim_absolute_coord(second).unwrap(),
            AnimWorldCoord {
                x: 3140,
                y: 2594,
                z: 0
            }
        );
        assert_eq!(
            second_anim.world_coord,
            AnimWorldCoord {
                x: 3140 - 3072,
                y: 2594 - 3072,
                z: 0
            },
            "stored coordinate is the owner-relative delta native writes"
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
                    use_cell_drawer: false,
                    terrain_attached: false,
                    building_explosion_start_smudge: false,
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
        // Native in-scenario load restarts Scenario RNG from Seed0; isolate
        // AnimStore/scheduler persistence on that same post-load cursor.
        sim.scenario_rng = crate::sim::rng::SimRng::new(0);
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
