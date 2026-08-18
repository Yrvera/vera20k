//! `BounceClass` — the rigid-body physics component that carries flying debris.
//!
//! gamemd-derived: `BounceClass::Init @ 0x004397E0`, `BounceClass::Update @
//! 0x00439B00`. RTTI `.?AVBounceClass@@` at `0x00845E38`. It is a component,
//! never a standalone object: it has no vtable use of its own, no `WhatAmI`, no
//! save slot, and is lifecycled entirely by its host. `VoxelAnimClass` embeds
//! one at `+0xB0`; `AnimClass` embeds one too, for the SHP debris that uses the
//! physics path.
//!
//! Do not confuse this with `Bouncer=yes` on an `AnimType`. That is a separate
//! system — `AnimClass::BounceAI @ 0x00425670`, a 2D homing drift toward an
//! attach target with no gravity and no elasticity — and it does not call
//! anything here.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on util/ and the rest of sim/.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use glam::IVec3;
use serde::{Deserialize, Serialize};

use crate::util::native_x87::{NativeF32Bits, NativeF64Bits};

/// The 0x50-byte physics body, field for field.
///
/// The three configuration values are `double` and the position, velocity and
/// quaternions are `float`, exactly as native stores them. That split is not
/// cosmetic: gravity is subtracted from a float Z each tick from a double
/// source, and the stop test compares a float magnitude against `2.5`, so
/// widening or narrowing either side changes how far debris travels and how
/// many times it bounces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BounceState {
    /// `+0x00`. Bounce coefficient from the type's `Elasticity`.
    pub elasticity: NativeF64Bits,
    /// `+0x08`. Subtracted from `velocity_z` every tick. 1.4 for voxel debris,
    /// hardcoded in `VoxelAnimClass::Constructor`; 3.0 in the terrain-meteor
    /// helper `FUN_00439690`.
    pub gravity: NativeF64Bits,
    /// `+0x10`. Clamp threshold for the velocity vector's length. Zero disables
    /// the clamp, which is the voxel-debris case.
    pub angular_velocity_magnitude: NativeF64Bits,
    /// `+0x18`, `+0x1C`, `+0x20`. World position in leptons, carried as float.
    pub position: [NativeF32Bits; 3],
    /// `+0x24`, `+0x28`, `+0x2C`. Velocity in leptons per tick.
    pub velocity: [NativeF32Bits; 3],
    /// `+0x30`. Orientation quaternion, integrated by `rotation_per_tick`.
    pub orientation: [NativeF32Bits; 4],
    /// `+0x40`. Per-tick rotation quaternion, built in `Init` from a random
    /// axis and the type's angular velocity. Its components are negated on
    /// every bounce, so a chunk tumbles the other way after each hit.
    pub rotation_per_tick: [NativeF32Bits; 4],
}

/// What one `BounceClass::Update` reported, from the switch in
/// `AnimClass::ProcessBounceResult @ 0x00423930`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BounceOutcome {
    /// Still in flight, no contact this tick.
    Falling,
    /// Hit the ground this tick. The host may play its `BounceAnim` and apply
    /// the impact damage.
    Bounced,
    /// Came to rest — the velocity magnitude fell below the stop threshold. The
    /// host expires itself.
    Stopped,
}

impl BounceState {
    /// The host's world coordinate, in leptons.
    ///
    /// `VoxelAnimClass::AI` refreshes its `ObjectClass` coordinate from these
    /// floats through `CoordStruct::FromDoubles` every tick, so this is the
    /// authoritative position and the object's own coordinate is a copy.
    pub fn position_leptons(&self) -> IVec3 {
        IVec3::new(
            f32::from_bits(self.position[0].bits()) as i32,
            f32::from_bits(self.position[1].bits()) as i32,
            f32::from_bits(self.position[2].bits()) as i32,
        )
    }

    /// The velocity vector, for the host's own reads.
    pub fn velocity_f32(&self) -> [f32; 3] {
        [
            f32::from_bits(self.velocity[0].bits()),
            f32::from_bits(self.velocity[1].bits()),
            f32::from_bits(self.velocity[2].bits()),
        ]
    }
}
