//! Retail multiplayer frame checksum.
//!
//! This is deliberately separate from [`Simulation::state_hash`]. The retail
//! network checksum is a narrow 32-bit diagnostic fold over class arrays,
//! display-layer order, LogicClass order, and one Scenario RNG sample. Computing
//! it also consumes one additional Scenario RNG sample for the native diagnostic
//! snapshot path.

use crate::map::entities::EntityCategory;
use crate::sim::game_entity::GameEntity;
use crate::sim::world::Simulation;

pub const DISPLAY_LAYER_COUNT: usize = 5;

const RTTI_UNIT: i32 = 1;
const RTTI_AIRCRAFT: i32 = 2;
const RTTI_ANIM: i32 = 4;
const RTTI_BUILDING: i32 = 6;
const RTTI_INFANTRY: i32 = 0x0f;
const RTTI_PARTICLE_SYSTEM: i32 = 0x18;
const SYNC_EXEMPT_ANIM_ID: i32 = -2;
const LEPTONS_PER_CELL: i32 = 256;

/// One ObjectClass entry as seen by a display-layer checksum pass.
///
/// `native_unique_id` matters only for AnimClass: the multiplayer-only
/// click-feedback sentinel is skipped in both the display and LogicClass folds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChecksumObject {
    pub world_x: i32,
    pub world_y: i32,
    pub what_am_i: i32,
    pub native_unique_id: i32,
}

impl ChecksumObject {
    pub const fn new(world_x: i32, world_y: i32, what_am_i: i32, native_unique_id: i32) -> Self {
        Self {
            world_x,
            world_y,
            what_am_i,
            native_unique_id,
        }
    }

    #[inline]
    const fn is_sync_exempt(self) -> bool {
        self.what_am_i == RTTI_ANIM && self.native_unique_id == SYNC_EXEMPT_ANIM_ID
    }
}

/// The value stored for one multiplayer frame plus the second, diagnostic-only
/// Scenario RNG sample consumed at the same native point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MultiplayerChecksumFrame {
    pub value: u32,
    pub diagnostic_rng_sample: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MultiplayerChecksumError {
    #[error("house registration order contains duplicate house {0:?}")]
    DuplicateHouse(crate::sim::intern::InternedId),
    #[error("house registration order references missing house {0:?}")]
    MissingHouse(crate::sim::intern::InternedId),
    #[error(
        "house registration order covers {registered} entries but the house store has {stored}"
    )]
    HouseOrderCoverage { registered: usize, stored: usize },
    #[error("LogicClass checksum entry {0} is absent from every object registry")]
    MissingLogicObject(u64),
}

/// Exact 32-bit fold used by the active multiplayer checksum.
///
/// Every input is folded as `rotate_left(acc, 1) + value`, with 32-bit wrapping.
/// Methods are split by native surface so callers cannot accidentally omit the
/// facing terms from the three class-array passes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RetailChecksumAccumulator {
    value: u32,
}

impl RetailChecksumAccumulator {
    pub const fn new() -> Self {
        Self { value: 0 }
    }

    #[inline]
    pub const fn value(self) -> u32 {
        self.value
    }

    #[inline]
    pub fn fold_value(&mut self, value: u32) {
        self.value = self.value.rotate_left(1).wrapping_add(value);
    }

    #[inline]
    pub fn fold_infantry(&mut self, world_x: i32, world_y: i32, primary_facing: u16) {
        let term = coordinate_term(world_x, world_y)
            .wrapping_add(u32::from(facing_checksum_byte(primary_facing)));
        self.fold_value(term);
    }

    #[inline]
    pub fn fold_unit(
        &mut self,
        world_x: i32,
        world_y: i32,
        primary_facing: u16,
        secondary_facing: u16,
    ) {
        let term = coordinate_term(world_x, world_y)
            .wrapping_add(u32::from(facing_checksum_byte(primary_facing)))
            .wrapping_add(u32::from(facing_checksum_byte(secondary_facing)));
        self.fold_value(term);
    }

    #[inline]
    pub fn fold_building(&mut self, world_x: i32, world_y: i32, primary_facing: u16) {
        let term = coordinate_term(world_x, world_y)
            .wrapping_add(u32::from(facing_checksum_byte(primary_facing)));
        self.fold_value(term);
    }

    #[inline]
    pub fn fold_house(&mut self, map_is_clear: u8) {
        self.fold_value(u32::from(map_is_clear));
    }

    #[inline]
    pub fn fold_object(&mut self, object: ChecksumObject) {
        if object.is_sync_exempt() {
            return;
        }
        let term =
            coordinate_term(object.world_x, object.world_y).wrapping_add(object.what_am_i as u32);
        self.fold_value(term);
    }
}

/// Convert one live 16-bit FacingClass value to the byte mixed by the checksum.
#[inline]
pub fn facing_checksum_byte(facing: u16) -> u8 {
    ((((u32::from(facing) >> 7).wrapping_add(1)) >> 1) & 0xff) as u8
}

#[inline]
fn coordinate_term(world_x: i32, world_y: i32) -> u32 {
    let x = world_x / 10;
    let y = world_y / 10;
    (x as u32).wrapping_add((y as u32).wrapping_mul(0x1_0000))
}

#[inline]
fn entity_world_xy(entity: &GameEntity) -> (i32, i32) {
    (
        i32::from(entity.position.rx)
            .wrapping_mul(LEPTONS_PER_CELL)
            .wrapping_add(entity.position.sub_x.to_num::<i32>()),
        i32::from(entity.position.ry)
            .wrapping_mul(LEPTONS_PER_CELL)
            .wrapping_add(entity.position.sub_y.to_num::<i32>()),
    )
}

#[inline]
fn primary_facing(entity: &GameEntity, frame: u32) -> u16 {
    entity
        .body_facing
        .as_ref()
        .map_or(u16::from(entity.facing) << 8, |facing| {
            facing.current(frame)
        })
}

#[inline]
fn secondary_facing(entity: &GameEntity, frame: u32) -> u16 {
    entity
        .barrel_facing
        .as_ref()
        .map_or(0, |facing| facing.current(frame))
}

#[inline]
fn entity_rtti(category: EntityCategory) -> i32 {
    match category {
        EntityCategory::Unit => RTTI_UNIT,
        EntityCategory::Infantry => RTTI_INFANTRY,
        EntityCategory::Structure => RTTI_BUILDING,
        EntityCategory::Aircraft => RTTI_AIRCRAFT,
    }
}

impl Simulation {
    /// Compute one active-retail multiplayer checksum.
    ///
    /// `display_layers` must be the five native display vectors in their stored
    /// order. Rust does not currently own those vectors in `sim`, so the
    /// presentation owner supplies the exact point-in-time views.
    /// This method must be called only by the admitted multiplayer-frame path:
    /// it consumes exactly two Scenario RNG samples and therefore must never run
    /// for an offline frame or a diagnostic-only `state_hash()` request.
    pub fn compute_retail_multiplayer_checksum(
        &mut self,
        display_layers: [&[ChecksumObject]; DISPLAY_LAYER_COUNT],
    ) -> Result<MultiplayerChecksumFrame, MultiplayerChecksumError> {
        let frame = self.session.binary_frame;
        let mut checksum = RetailChecksumAccumulator::new();

        // Native class-array order is constructor/registration order. Stable
        // object IDs are monotonic construction IDs, so filtering the ordered
        // store preserves each class array's order without a second registry.
        for entity in self.substrate.entities.values_sorted() {
            if entity.category == EntityCategory::Infantry {
                let (x, y) = entity_world_xy(entity);
                checksum.fold_infantry(x, y, primary_facing(entity, frame));
            }
        }
        for entity in self.substrate.entities.values_sorted() {
            if entity.category == EntityCategory::Unit {
                let (x, y) = entity_world_xy(entity);
                checksum.fold_unit(
                    x,
                    y,
                    primary_facing(entity, frame),
                    secondary_facing(entity, frame),
                );
            }
        }
        for entity in self.substrate.entities.values_sorted() {
            if entity.category == EntityCategory::Structure {
                let (x, y) = entity_world_xy(entity);
                checksum.fold_building(x, y, primary_facing(entity, frame));
            }
        }

        if self.session.house_order.len() != self.houses.len() {
            return Err(MultiplayerChecksumError::HouseOrderCoverage {
                registered: self.session.house_order.len(),
                stored: self.houses.len(),
            });
        }
        for (index, owner) in self.session.house_order.iter().copied().enumerate() {
            if self.session.house_order[..index].contains(&owner) {
                return Err(MultiplayerChecksumError::DuplicateHouse(owner));
            }
            let house = self
                .houses
                .get(&owner)
                .ok_or(MultiplayerChecksumError::MissingHouse(owner))?;
            checksum.fold_house(u8::from(house.map_is_clear));
        }

        for layer in display_layers {
            for &object in layer {
                checksum.fold_object(object);
            }
        }

        for &stable_id in self.substrate.logic.as_slice() {
            let object = if let Some(entity) = self.substrate.entities.get(stable_id) {
                let (world_x, world_y) = entity_world_xy(entity);
                ChecksumObject::new(world_x, world_y, entity_rtti(entity.category), 0)
            } else if let Some(anim) = self.substrate.anims.get(stable_id) {
                ChecksumObject::new(
                    anim.world_coord.x,
                    anim.world_coord.y,
                    RTTI_ANIM,
                    anim.native_unique_id,
                )
            } else if let Some(system) = self.substrate.particle_systems.get(stable_id) {
                ChecksumObject::new(system.coords.x, system.coords.y, RTTI_PARTICLE_SYSTEM, 0)
            } else {
                return Err(MultiplayerChecksumError::MissingLogicObject(stable_id));
            };
            checksum.fold_object(object);
        }

        let checksum_rng_sample = self.scenario_rng.next_u32();
        checksum.fold_value(checksum_rng_sample);
        let diagnostic_rng_sample = self.scenario_rng.next_u32();

        Ok(MultiplayerChecksumFrame {
            value: checksum.value(),
            diagnostic_rng_sample,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ChecksumObject, RetailChecksumAccumulator, SYNC_EXEMPT_ANIM_ID, facing_checksum_byte,
    };
    use crate::sim::world::Simulation;

    #[test]
    fn representative_fold_matches_the_native_32_bit_trace() {
        let mut checksum = RetailChecksumAccumulator::new();
        checksum.fold_infantry(125, 245, 0x3f80);
        checksum.fold_unit(-125, 305, 0x8000, 0xffff);
        checksum.fold_building(999, -101, 0x0100);
        checksum.fold_house(1);
        checksum.fold_object(ChecksumObject::new(321, 654, 6, 17));

        assert_eq!(checksum.value(), 0x0289_0a18);
    }

    #[test]
    fn rotate_carry_negative_division_and_facing_rounding_are_exact() {
        let mut checksum = RetailChecksumAccumulator::new();
        checksum.fold_value(0x8000_0000);
        checksum.fold_value(0);
        assert_eq!(checksum.value(), 1, "fold is rotate-left, not shift-left");

        let mut negative = RetailChecksumAccumulator::new();
        negative.fold_object(ChecksumObject::new(-19, -19, 1, 9));
        assert_eq!(
            negative.value(),
            // The truncated coordinate term is -65537; WhatAmI(1) joins it.
            0xffff_0000,
            "signed division truncates toward zero before wrapping"
        );

        assert_eq!(facing_checksum_byte(0x007f), 0);
        assert_eq!(facing_checksum_byte(0x0080), 1);
        assert_eq!(facing_checksum_byte(0xffff), 0);
    }

    #[test]
    fn only_the_multiplayer_feedback_anim_sentinel_is_skipped() {
        let mut checksum = RetailChecksumAccumulator::new();
        checksum.fold_object(ChecksumObject::new(100, 200, 4, SYNC_EXEMPT_ANIM_ID));
        assert_eq!(checksum.value(), 0);

        checksum.fold_object(ChecksumObject::new(100, 200, 4, -1));
        assert_ne!(checksum.value(), 0);
    }

    #[test]
    fn multiplayer_frame_consumes_exactly_two_scenario_rng_samples() {
        let mut sim = Simulation::with_seed(0x1234);
        let mut reference = sim.scenario_rng.clone();
        let checksum_sample = reference.next_u32();
        let diagnostic_sample = reference.next_u32();

        let frame = sim
            .compute_retail_multiplayer_checksum([&[], &[], &[], &[], &[]])
            .unwrap();

        assert_eq!(frame.value, checksum_sample);
        assert_eq!(frame.diagnostic_rng_sample, diagnostic_sample);
        assert_eq!(sim.scenario_rng.next_u32(), reference.next_u32());
    }
}
