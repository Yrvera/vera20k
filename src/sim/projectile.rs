//! Authoritative in-flight projectile state.
//!
//! This module deliberately owns only delayed flight, target expiry, collision,
//! and detonation admission. Combat remains the authority for turning a returned
//! [`ProjectileDetonation`] into damage, warhead effects, and terrain changes.
//! Keeping that handoff narrow lets the world phase place this system at the
//! verified native frame rung without duplicating combat arithmetic here.
//!
//! ## `BulletClass::AI @ 0x004666E0` — the three flight arms
//!
//! Native selects an arm exactly twice, at `0x004668D1` on `ROT < 1` and then
//! at `0x004671D0` on `Vertical` (`BulletTypeClass+0x2C0`). Nothing else takes
//! part: `Arcing`, `SubjectToCliffs`, `SubjectToElevation`, `Proximity`,
//! `FlakScatter`, `Inviso` and `Cluster` are launch-time or detonation-time
//! keys. [`ProjectileTrajectory`] carries the same three arms.
//!
//! Detonation admission at `0x00467C70` is `impact || (!Dropping && fuse != 0)`
//! and contains NO `Arm=` test — `Arm=` is written once into the bullet's
//! embedded `ProximityDetector` at `BulletClass::Fire 0x00468A5D` and gates
//! that one fuse result. A collision, a ground or bridge contact, a
//! `DetonationAltitude` crossing, or a target reach all detonate an unarmed
//! bullet.
//!
//! Homing termination reads the OLD ObjectClass height, before committing the
//! new coordinate (`0x00466DF6`). Ground admission suppresses the arm's target
//! snap; the common tail then clamps a negative committed height and may snap
//! a mode-1 fuse to the target's location (`+0x48`, not its aim point `+0x58`).
//!
//! RESIDUAL (GSI-08.06) — the ballistic arm's `reached_target` test is
//! VERA-internal, gamemd equivalent UNCHECKED: native's `ROT < 1` arms
//! terminate only on the ground clamp, the wall/building probe and the fuse,
//! and its `0x00467CEC` "snap onto the target when within `max(128, 2|v|)`" is
//! a coordinate adjustment applied AFTER detonation is already admitted, not a
//! reason to detonate. Trigger: every arcing shell. Player effect: a shell can
//! detonate one frame early, in the air, instead of on the ground at its
//! landing point. Frequency: continuous for `Arcing` weapons. Downstream risk:
//! it partly cancels the new launch scatter for `[FlakTProj]`.

use std::collections::BTreeMap;

use crate::map::resolved_terrain::{ResolvedTerrainGrid, SharedCellDummy};
use crate::sim::intern::InternedId;
use crate::sim::movement::homing_movement::{
    atan2_bam, cos_bam, sidewinder_cos, sin_bam, step_toward_bam_inclusive,
};
use crate::sim::rng::SimRng;
use crate::util::fixed_math::SimFixed;

/// Lepton-space position for an in-flight projectile.
///
/// Cells contain 256 leptons. Keeping the flight state in signed integer
/// leptons avoids float math and makes the serialized state independent of
/// render coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ProjectileCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Integer velocity committed by `BulletClass::AI`, in leptons per frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ProjectileVelocity {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ProjectileVelocity {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ProjectileTrajectory {
    Straight,
    Ballistic {
        gravity: i32,
    },
    /// gamemd-derived: `BulletClass::AI @ 0x004666E0`, the `ROT < 1 &&
    /// Vertical` arm entered at `0x004671E0`. No gravity is applied there and
    /// the launch direction is never changed — only the magnitude ramps by
    /// `Acceleration=` (`BulletTypeClass+0x2D0`) toward `MaxSpeed`
    /// (`Bullet+0x110`), and flight ends on `z > DetonationAltitude`
    /// (`+0x2BC`, read at `0x00467334`), a negative altitude, or a bridge-deck
    /// crossing.
    ///
    /// The direction is kept as the launch (yaw, pitch) BAM pair rather than
    /// as an integer velocity: native renormalises a *double* velocity vector
    /// every frame, and at the 1-lepton launch magnitude an integer vector
    /// would quantise the direction away.
    Vertical {
        detonation_altitude: i32,
        acceleration: i32,
        max_speed: i32,
        heading_bam: u16,
        pitch_bam: u16,
    },
}

/// Persistent inputs and facing state for the `BulletClass::Update` ROT branch.
///
/// The native sidewinder phase includes a BulletClass-identity-derived value
/// whose derivation is not yet closed. `sidewinder_phase` is therefore an
/// explicit serialized seam rather than an invented stable-id formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ProjectileGuidance {
    pub rot: i32,
    pub missile_rot_var: SimFixed,
    /// `BulletTypeClass::CourseLockDuration` (`+0x2E0`). This is the authored
    /// duration, not a countdown: `BulletClass::AI @ 0x0046695B` compares its
    /// own frame counter against it and latches `IsCourseLocked` (`+0x105`,
    /// constructed to 1 at `0x004663B6`) off exactly once.
    pub course_lock_duration: u16,
    pub sidewinder_phase: u8,
    pub airburst: bool,
    /// `BulletTypeClass+0x2A2`, the common final-snap exclusion at `0x00467CDE`.
    /// The earlier homing reach snap at `0x00466E22` does not test this flag.
    pub inaccurate: bool,
    pub very_high: bool,
    pub level: bool,
    /// Flight heading as a math BAM (`0` = +X). Native keeps the direction
    /// implicitly in the double velocity vector it renormalises each frame at
    /// `0x004669F3`; VERA's velocity is integer leptons, so at the 1-lepton
    /// launch magnitude the direction has to be carried explicitly.
    /// `heading_bam == facing16 - 0x4000` for a native launch facing.
    pub heading_bam: u16,
    pub frames_elapsed: u32,
    /// `Bullet+0x110`, written from the weapon's `Speed=` (`WeaponTypeClass
    /// +0xA8`) at `TechnoClass::FireAt 0x006FEA46`. This is the ceiling the
    /// `Acceleration=` ramp climbs toward, never the launch speed.
    pub max_speed: u16,
    /// `BulletTypeClass::Acceleration` (`+0x2D0`, constructor default 3),
    /// consumed by the homing ramp at `BulletClass::AI 0x00466985`.
    pub acceleration: i32,
    /// The proximity fuse's reference coordinate, frozen at launch.
    /// `ProximityDetector::Setup @ 0x004E1130` (sole caller
    /// `BulletClass::Fire 0x00468A93`) copies the target's launch-time
    /// `GetTargetCoords` into detector `+0x18..+0x20` once;
    /// `ProximityDetector::Check @ 0x004E11F0` (sole caller
    /// `BulletClass::AI 0x00467C35`) measures the live bullet coordinate
    /// against that stored copy and never rewrites it.
    pub fuse_reference: ProjectileCoord,
    /// `Bullet+0x118`, the warm-up counter of the closing-rate detonation
    /// heuristic on the homing arm. Native runs the plain accumulate while it
    /// is below 60 and only then admits the decay test — `0x00466FB4
    /// MOV EAX,[EBP+0x118] / CMP EAX,0x3C / JGE`, then `INC EAX /
    /// MOV [EBP+0x118],EAX` on the warm-up side.
    pub closing_frames: u16,
    /// `Bullet+0x120`, the closing-rate accumulator of the same heuristic
    /// (`0x00466FD8 FLD double ptr [EBP+0x120]` / `0x00466FEE FST double ptr
    /// [EBP+0x120]`), held as native `double` bits because the native update is
    /// `accum * 0.9833333333333333 + delta` in x87 doubles.
    pub closing_accumulator_bits: u64,
}

/// The independently proved altitude-policy result of
/// `BulletClass::ComputeArcingTrajectoryStep` at `0x005B20F0`.
///
/// Its live floor/bridge probe is intentionally left to the world collision
/// substrate; ROT steering below does not substitute cell levels for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectileRotAltitudeDecision {
    pub control_terrain_clearance: bool,
    pub clearance_levels: i32,
    pub z_delta: i32,
    pub desired_pitch: Option<u16>,
}

/// Pack the proven high-byte ROT control word. Native FISTP rounding is used
/// before the low-byte truncation.
pub fn projectile_rot_turn_word(varied_rot: f64, target_distance: i32, course_locked: bool) -> u16 {
    if course_locked {
        return 0;
    }
    let rate = if target_distance < 256 {
        (varied_rot * 1.5).round_ties_even()
    } else {
        varied_rot.round_ties_even()
    } as i32;
    (rate as u16 & 0xff) << 8
}

fn projectile_rot_turn_word_fixed(
    varied_rot: SimFixed,
    target_distance: i32,
    course_locked: bool,
) -> u16 {
    if course_locked {
        return 0;
    }
    let varied_rot = if target_distance < 256 {
        varied_rot * SimFixed::lit("1.5")
    } else {
        varied_rot
    };
    let bits = i64::from(varied_rot.to_bits());
    let whole = bits / 65_536;
    let remainder = bits.unsigned_abs() % 65_536;
    let rounded = if remainder > 32_768 || (remainder == 32_768 && whole & 1 != 0) {
        whole + i64::from(bits.is_positive()) - i64::from(bits.is_negative())
    } else {
        whole
    };
    (rounded as u16 & 0xff) << 8
}

/// Apply the closed VeryHigh/Airburst clearance admission and error bands.
pub fn projectile_rot_altitude_decision(
    target_is_aircraft: bool,
    airburst: bool,
    very_high: bool,
    level: bool,
    horizontal_distance: i32,
    target_height_difference: i32,
    level_height: i32,
    current_clearance_error: i32,
    turn_word: u16,
) -> ProjectileRotAltitudeDecision {
    let close_threshold = if very_high { 6 } else { 3 } * 256;
    let turn_quantum = (((u32::from(turn_word) >> 7) + 1) >> 1) as u8;
    let clearance_levels = if airburst || very_high {
        10
    } else {
        (target_height_difference / 256).min(5)
    };
    if target_is_aircraft
        || (!airburst && horizontal_distance <= close_threshold)
        || turn_quantum <= 1
        || level
    {
        return ProjectileRotAltitudeDecision {
            control_terrain_clearance: false,
            clearance_levels,
            z_delta: 0,
            desired_pitch: None,
        };
    }

    let z_delta = if current_clearance_error < -20 {
        18
    } else if current_clearance_error > 20 {
        -18
    } else {
        0
    };
    let half_level = level_height / 2;
    let desired_pitch = if current_clearance_error < -half_level {
        0x2000
    } else if current_clearance_error > half_level {
        0x4800
    } else {
        0x4000
    };
    ProjectileRotAltitudeDecision {
        control_terrain_clearance: true,
        clearance_levels,
        z_delta,
        desired_pitch: Some(desired_pitch),
    }
}

/// Serialized `BulletClass` SHP animation bytes (`this+0x12c/+0x12d`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ProjectileVisualState {
    pub anim_low: u8,
    pub anim_high: u8,
    pub anim_rate: u8,
    pub runtime_frame: u8,
    pub runtime_countdown: u8,
}

impl ProjectileVisualState {
    pub const fn new(anim_low: u8, anim_high: u8, anim_rate: u8) -> Self {
        Self {
            anim_low,
            anim_high,
            anim_rate,
            runtime_frame: 0,
            runtime_countdown: anim_rate,
        }
    }

    pub fn advance(&mut self) {
        if self.anim_low == 0 && self.anim_high == 0 {
            return;
        }
        self.runtime_countdown = self.runtime_countdown.wrapping_sub(1);
        if self.runtime_countdown != 0 {
            return;
        }
        self.runtime_countdown = self.anim_rate;
        self.runtime_frame = self.runtime_frame.wrapping_add(1);
        if self.runtime_frame > self.anim_high {
            self.runtime_frame = self.anim_low;
        }
    }
}

impl ProjectileCoord {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// Resolve the current virtual `CellClass::GetTargetCoords` value for a stable
/// CellClass target. The cell identity is retained by the projectile; terrain
/// level/slope and the live CellClass structural bit are read again on every
/// visit.
pub(crate) fn cell_target_coord(
    terrain: Option<&ResolvedTerrainGrid>,
    rx: u16,
    ry: u16,
) -> ProjectileCoord {
    let x = i32::from(rx)
        .wrapping_mul(crate::sim::cell_kernel::LEPTONS_PER_CELL)
        .wrapping_add(crate::sim::cell_kernel::CELL_CENTER_LEPTONS);
    let y = i32::from(ry)
        .wrapping_mul(crate::sim::cell_kernel::LEPTONS_PER_CELL)
        .wrapping_add(crate::sim::cell_kernel::CELL_CENTER_LEPTONS);
    let z = terrain
        .and_then(|grid| grid.cell(rx, ry))
        .map(|cell| {
            // gamemd-derived: `CellClass::GetTargetCoords +0x58 @ 0x00486890`
            // delegates `+0x48 @ 0x00486840` to
            // `CellClass::ComputeGroundHeightAtCoord @ 0x0047B3A0`, then adds 416 iff
            // this CellClass's own `+0x140 & 0x100` is set. Bridge runtime
            // walkability is not consulted.
            crate::util::lepton::ground_height_leptons(cell.level, cell.slope_type, x, y)
                .expect("resolved CellClass target must have a supported slope")
                .wrapping_add(
                    if cell.bridge_facts.raw_flags
                        & crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL
                        != 0
                    {
                        crate::util::lepton::BRIDGE_HEIGHT_DELTA_LEPTONS as i32
                    } else {
                        0
                    },
                )
        })
        // A Cell target may legitimately name a cell with no allocated
        // CellClass: `MapClass::Get_CellClass @ 0x005657A0` answers those with
        // the shared dummy at `0x00ABDC50`. A `CellClass` is `0x148` bytes and
        // holds `Level` at `+0x11B` and `SlopeIndex` at `+0x11C`; all `0x148` bytes of
        // the dummy read zero, so native takes level 0 and slope 0 there and
        // lands on floor height 0 as well. Headless store tests reach the same
        // arm by having no map substrate.
        .unwrap_or(0);
    ProjectileCoord::new(x, y, z)
}

/// Resolve the current virtual `CellClass::GetTargetCoords` value for the one
/// shared fallback CellClass. Unlike a stable allocated cell, every later miss
/// can change the coordinate observed through this retained identity.
pub(crate) fn dummy_cell_target_coord(dummy: &SharedCellDummy) -> ProjectileCoord {
    let snapshot = dummy.snapshot();
    let x = snapshot
        .coord
        .0
        .wrapping_mul(crate::sim::cell_kernel::LEPTONS_PER_CELL)
        .wrapping_add(crate::sim::cell_kernel::CELL_CENTER_LEPTONS);
    let y = snapshot
        .coord
        .1
        .wrapping_mul(crate::sim::cell_kernel::LEPTONS_PER_CELL)
        .wrapping_add(crate::sim::cell_kernel::CELL_CENTER_LEPTONS);
    // `CellClass` target virtual +0x58 at `0x00486890` delegates +0x48 at
    // `0x00486840`, which calls
    // `CellClass::ComputeGroundHeightAtCoord @ 0x0047B3A0`.
    // Active retail initializes the Cell-owned scalar independently, but its
    // captured value is the same 104 used by the shared ground evaluator.
    let z =
        crate::util::lepton::ground_height_leptons(snapshot.level as u8, snapshot.slope_type, x, y)
            .expect("shared CellClass target must have a supported slope")
            // `CellClass::GetTargetCoords @ 0x00486890` adds the process-global
            // high-bridge delta when `CellClass+0x140 & 0x100` is live. The floor
            // beneath it remains the verified 104-lepton CellClass kernel above.
            .wrapping_add(
                if snapshot.bridge_flags_0x1180 & crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL
                    != 0
                {
                    crate::util::lepton::BRIDGE_HEIGHT_DELTA_LEPTONS as i32
                } else {
                    0
                },
            );
    ProjectileCoord::new(x, y, z)
}

/// The original target retained by a projectile after weapon fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ProjectileTarget {
    Entity(u64),
    /// Stable MapClass CellClass identity. Its target coordinate is resolved
    /// from live terrain state instead of freezing the cleanup-time Vec3.
    Cell {
        rx: u16,
        ry: u16,
    },
    /// Native null AbstractClass target. This is distinct from an expired
    /// entity lookup: BulletClass pointer cleanup has already handled the
    /// reference synchronously, so `TargetExpiryPolicy` must not run.
    None,
    /// MapClass's one process-global fallback CellClass at `0x00ABDC50`.
    /// The enum stores the pointer kind, not a coordinate snapshot; Simulation
    /// owns the live identity and BulletClass AI resolves it every visit.
    DummyCell,
}

/// What to do when an entity target no longer exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TargetExpiryPolicy {
    /// Remove the projectile without a damage handoff.
    Expire,
    /// Detonate at the last target coordinate observed by the projectile.
    DetonateAtLastKnown,
}

/// Terrain checks admitted for one `BulletClass` flight.
///
/// `native_cell_collision` models the ground-height / bridge-deck / building
/// block at `BulletClass::AI 0x004674AE`..`0x00467778`. That block lives inside
/// the `ROT < 1, Vertical = no` arm: `0x004671D0 MOV CL,[EAX+0x2C0] /
/// 0x004671D6 TEST CL,CL / 0x004671D8 JZ 0x00467402` reaches it only when
/// `Vertical == 0`, and a `Vertical` bullet takes the `0x004671DE`..`0x00467390`
/// arm instead, which runs its own `DetonationAltitude` / floor / bridge probe.
/// That is why the flag is narrowed on `!vertical` — not because a `Vertical`
/// bullet skips every terrain test.
///
/// RESIDUAL (GSI-08.08) — the shared post-move probe `FUN_00468BB0` is not
/// modelled for ANY trajectory. `BulletClass::AI 0x00467BD1` calls it for every
/// bullet whose impact flag is still clear (`0x00467BA4 MOV AL,[ESP+0x18] /
/// TEST AL,AL / JNZ 0x00467BF0` — there is no `Vertical` or `ROT` test), and it
/// detonates on `SubjectToCliffs`/`SubjectToWalls` cliff geometry
/// (`FUN_004CC360`), on an altitude below `DAT_0089DE70 * -4`, on descending
/// past an `AA` target, on the `Level` water branch, and on closing within
/// `0x80` of an air target. Trigger: any bullet crossing a cliff, or an `AA`
/// shot passing its target. Player effect: those shots keep flying where native
/// would have detonated them. Frequency: zero for the two `Vertical` stock
/// projectiles (`[BlimpBombP]`/`[BlimpBombPE]` author none of the gating keys);
/// otherwise cliff-crossing shots on hilly maps. Downstream risk: none beyond
/// the impact coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectileCollisionPolicy {
    /// `BulletTypeClass::Level`: detonate after entering a non-water cell.
    pub level_non_water: bool,
    /// `BulletTypeClass::SubjectToWalls`: detonate after entering a live wall.
    pub subject_to_walls: bool,
    /// Ordinary non-guided `BulletClass::Update` floor/bridge/content probe.
    pub native_cell_collision: bool,
}

impl ProjectileCollisionPolicy {
    pub const NONE: Self = Self {
        level_non_water: false,
        subject_to_walls: false,
        native_cell_collision: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileBridgeCrossing {
    None,
    Up,
    Down,
}

pub fn projectile_bridge_crossing(
    previous_z: i32,
    candidate_z: i32,
    surface_z: i32,
) -> ProjectileBridgeCrossing {
    if previous_z < surface_z && candidate_z >= surface_z {
        ProjectileBridgeCrossing::Up
    } else if previous_z >= surface_z && candidate_z < surface_z {
        ProjectileBridgeCrossing::Down
    } else {
        ProjectileBridgeCrossing::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileCellObstacle {
    None,
    Building(u64),
    Overlay,
}

#[allow(clippy::too_many_arguments)]
pub fn projectile_cell_obstacle(
    candidate_z: i32,
    floor_z: i32,
    building_id: Option<u64>,
    overlay_connected: bool,
    building_is_target: bool,
    building_late_exemption: bool,
    building_vslot_exemption: bool,
    target_owner_allied_with_building: bool,
) -> ProjectileCellObstacle {
    if candidate_z < floor_z || candidate_z >= floor_z.saturating_add(150) {
        return ProjectileCellObstacle::None;
    }
    if let Some(building_id) = building_id {
        let exempt = building_is_target
            || building_late_exemption
            || building_vslot_exemption
            || target_owner_allied_with_building;
        return if exempt {
            ProjectileCellObstacle::None
        } else {
            ProjectileCellObstacle::Building(building_id)
        };
    }
    if overlay_connected {
        ProjectileCellObstacle::Overlay
    } else {
        ProjectileCellObstacle::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileCollisionResponse {
    TargetZClamp(ProjectileCoord),
    SlopeMatrixReflect {
        impact: ProjectileCoord,
        velocity: ProjectileVelocity,
    },
}

/// Named location: `BulletClass::Update @ 0x00467720`, null-target collision tail.
/// The exact Cell slope matrices are shared with the proved Spark behavior-3 table.
pub fn projectile_slope_reflect(
    velocity: ProjectileVelocity,
    slope_type: u8,
) -> Option<ProjectileVelocity> {
    let matrix = crate::sim::particles::spark_world::slope_matrix(slope_type).ok()?;
    let m = matrix.map(|value| f32::from_bits(value.bits()));
    let axis = [velocity.x as f32, -(velocity.y as f32), velocity.z as f32];
    let local = [
        m[0] * axis[0] + m[4] * axis[1] + m[8] * axis[2],
        m[1] * axis[0] + m[5] * axis[1] + m[9] * axis[2],
        m[2] * axis[0] + m[6] * axis[1] + m[10] * axis[2],
    ];
    let local = [local[0], -local[1], local[2]];
    let reflected = [
        m[0] * local[0] + m[1] * local[1] + m[2] * local[2],
        m[4] * local[0] + m[5] * local[1] + m[6] * local[2],
        m[8] * local[0] + m[9] * local[1] + m[10] * local[2],
    ];
    Some(ProjectileVelocity::new(
        reflected[0].trunc() as i32,
        (-reflected[1]).trunc() as i32,
        reflected[2].trunc() as i32,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectileBurstPlan {
    pub detonation_count: u32,
    pub random_radius_rolls: u32,
    pub random_coordinate_calls: u32,
}

pub fn projectile_burst_plan(airburst: bool, cluster: i32) -> ProjectileBurstPlan {
    if airburst {
        return ProjectileBurstPlan {
            detonation_count: 1,
            random_radius_rolls: 0,
            random_coordinate_calls: 0,
        };
    }
    let count = cluster.max(0) as u32;
    ProjectileBurstPlan {
        detonation_count: count,
        random_radius_rolls: count,
        random_coordinate_calls: count,
    }
}

/// One uniform BAM angle drawn the way `TechnoClass::FireAt` and
/// `BulletClass::Fire` draw theirs: a raw `Random__RandomRanged(0, 0x7FFFFFFE)`
/// mapped to `[0, 2π)`, pushed through the 16-bit facing quantisation, and
/// converted back to radians.
///
/// Named locations: `0x006FE83D`..`0x006FE872` (plain arm),
/// `0x006FE75B`..`0x006FE790` (flak arm), `0x00468807`..`0x00468840`
/// (`BulletClass::Fire`). Constants read from the image:
/// `[0x007E3570] = 1/(2^31 - 1)`, `[0x007E3CC0] = 2π`,
/// `[0x007E2820] = π/2`, `[0x007E2818] = -10430.06004058427`,
/// `[0x007E2810] = -9.587672516830327e-05`.
fn scatter_angle_radians(scenario_rng: &mut SimRng) -> f64 {
    const INVERSE_RANDOM_SPAN: f64 = 4.656_612_875_245_797e-10;
    const FACING_SCALE: f64 = -10430.060_040_584_27;
    const BAM_TO_RADIANS: f64 = -9.587_672_516_830_327e-05;
    let raw = scenario_rng.next_range_u32_inclusive(0, 0x7fff_fffe);
    let uniform = f64::from(raw as i32) * INVERSE_RANDOM_SPAN * std::f64::consts::TAU;
    let bam = ((uniform - std::f64::consts::FRAC_PI_2) * FACING_SCALE).trunc() as i32 as i16;
    f64::from(i32::from(bam) - 0x3fff) * BAM_TO_RADIANS
}

/// `TechnoClass::FireAt` launch-time scatter, gated by `Inaccurate && Arcing`
/// at `0x006FE67D`/`0x006FE68B`. Both arms consume exactly two Scenario RNG
/// draws — magnitude first, then angle — and both OFFSET the target delta
/// rather than replacing it (`FIADD` on X at `0x006FE7E5`, `FSUBR` on Y at
/// `0x006FE7C0`). Z is carried through untouched.
///
/// `flak` selects the range-scaled arm taken when
/// `FlakScatter && !Inviso` (`0x006FE699`/`0x006FE6A7`); stock `[FlakTProj]`
/// is its only user.
///
/// **The axis binding below is deliberate and is NOT the one the mapping ledger
/// records.** The ledger's §3a has sine and cosine swapped (`dX' = sin`,
/// `dY' = cos`). Walked in assembly: the `Math__CosFromTable @ 0x004CAD00`
/// result is the term that reaches the X `FIADD` at `0x006FE7E5`, and the
/// `Math__SinFromTable @ 0x004CACB0` result is the term that reaches the Y
/// `FSUBR` at `0x006FE7C0` — so `dX += cos(theta) * mag` and
/// `dY -= sin(theta) * mag`, as written. Do not "correct" this back to the
/// ledger.
pub fn projectile_launch_scatter(
    delta: (i32, i32, i32),
    ballistic_scatter: i32,
    weapon_range_leptons: i32,
    flak: bool,
    scenario_rng: &mut SimRng,
) -> (i32, i32, i32) {
    let magnitude = if flak {
        // `0x006FE6AD`..`0x006FE6F1` builds the distance from f32 components.
        let fx = delta.0 as f32;
        let fy = delta.1 as f32;
        let fz = delta.2 as f32;
        let distance = ((f64::from(fx) * f64::from(fx)
            + f64::from(fy) * f64::from(fy)
            + f64::from(fz) * f64::from(fz))
        .sqrt()) as f32;
        let roll = scenario_rng.next_range_u32_inclusive(0, ballistic_scatter.max(0) as u32) as i32;
        let scaled = i64::from(roll) * i64::from(distance.trunc() as i32);
        if weapon_range_leptons == 0 {
            0
        } else {
            (scaled / i64::from(weapon_range_leptons)) as i32
        }
    } else {
        // `0x006FE815`..`0x006FE81C`: `RandomRanged(BallisticScatter / 2,
        // BallisticScatter)`, the halving being an arithmetic `SAR` by one.
        let high = ballistic_scatter.max(0);
        scenario_rng.next_range_u32_inclusive((high / 2) as u32, high as u32) as i32
    };
    let theta = scatter_angle_radians(scenario_rng);
    let magnitude = f64::from(magnitude);
    (
        (theta.cos() * magnitude + f64::from(delta.0)).trunc() as i32,
        (f64::from(delta.1) - theta.sin() * magnitude).trunc() as i32,
        delta.2,
    )
}

/// Produce the next `BulletClass::Explode` cluster coordinate.
///
/// Named locations: `BulletClass::Explode @ 0x00468d80` draws the inclusive
/// radius first, then `MapClass::GetRandomCoordsNear @ 0x00566dc0` consumes one
/// raw Scenario RNG word and uses its low byte as the angle byte. The generated
/// coordinate is relative to the prior coordinate and is reverted when it
/// leaves the native 512-by-512 cell map.
pub fn projectile_next_cluster_coord(
    prior: ProjectileCoord,
    scenario_rng: &mut SimRng,
) -> ProjectileCoord {
    let distance = scenario_rng.next_range_u32_inclusive(256, 512) as f64;
    let angle_byte = (scenario_rng.next_u32() & 0xff) as i32;
    let angle_word = angle_byte << 8;
    let angle = f64::from(angle_word - 0x3fff) * -0.00009587672516830327;
    let candidate = ProjectileCoord::new(
        (f64::from(prior.x) + angle.cos() * distance).round_ties_even() as i32,
        (f64::from(prior.y) - angle.sin() * distance).round_ties_even() as i32,
        prior.z,
    );
    let inside_map = (candidate.x / 256) >= 0
        && (candidate.x / 256) < 0x200
        && (candidate.y / 256) >= 0
        && (candidate.y / 256) < 0x200;
    if inside_map { candidate } else { prior }
}

/// Native two-draw random-cell fallback used after hostile shrapnel targets.
pub fn projectile_random_shrapnel_cell(
    center_rx: i32,
    center_ry: i32,
    scenario_rng: &mut SimRng,
) -> (i32, i32) {
    let dx = scenario_rng.next_range_u32_inclusive(0, 4) as i32 - 2;
    let dy = scenario_rng.next_range_u32_inclusive(0, 4) as i32 - 2;
    (center_rx + dx, center_ry + dy)
}

pub fn projectile_shrapnel_count(
    configured_count: i32,
    has_firer: bool,
    distance_to_target_cells: i32,
) -> u32 {
    if configured_count >= 0 {
        return configured_count as u32;
    }
    if !has_firer {
        return 3;
    }
    configured_count
        .saturating_neg()
        .saturating_sub(distance_to_target_cells)
        .max(0) as u32
}

/// Which arm of the native special-detonation chain claims one impact.
///
/// gamemd-derived: `BulletClass::DetonateAtCoord @ 0x004690b0` is a strict
/// twelve-arm `else if` chain over `WarheadTypeClass` flag bytes. Each test's
/// fall-through `JZ` targets the *next* test, so the arms are mutually
/// exclusive and the order below is the native order, read from the flag tests
/// at `0x00469211`, `0x00469343`, `0x0046937a`, `0x004693d3`, `0x00469423`,
/// `0x004694cb`, `0x00469705`, `0x0046978e`, `0x004699ca`, `0x00469a03` and
/// `0x00469a2c`.
///
/// Only the final else at `0x00469a3f` reaches
/// `BulletClass::SpawnShrapnel @ 0x0046a310` and
/// `Apply_area_damage @ 0x00489280`, so any special arm suppresses both. What a
/// special arm does **not** suppress is the shared tail: every arm, including
/// its internal bail-outs (e.g. MindControl with no CaptureManager at
/// `0x00469235`), leaves through `JMP LAB_00469AA4`, and `LAB_00469AA4`
/// (`0x00469aa4 MOV EAX,[EBP+0x8]`) is not the epilogue — the epilogue is
/// `0x0046a290`. Unconditionally the tail is the `Inviso` visual re-scatter
/// (`0x00469ad7`), the explosion-anim *selection*
/// (`Warhead::SelectExplosionAnim` called at `0x00469bcf`), the `AnimList=`
/// `AnimClass` itself (built from `0x00469c46`), the debris loop and the
/// `Airburst=` sub-munition fan (`BulletClass::Init` at `0x00469f7f` /
/// `0x0046a150`). The ordinary arm reaches that same tail only behind the
/// `bullet+0x90` gate at `0x00469a94`; special arms bypass the gate.
///
/// **The combat-light / `CLDisable*` block is NOT part of that unconditional
/// tail.** `0x00469bf0..0x00469c45`, ending in the light spawn
/// `0x00469c41 CALL 0x0048a620`, runs only when `bullet+0xe0 != 0` **and**
/// `[ESP+0xf] == 0` — `0x00469bdc TEST AL,AL` / `0x00469be2 JZ 0x0046a299`
/// then `0x00469be8 TEST AL,AL` / `0x00469bea JNZ 0x0046a2a1` — and
/// `0x0046a29b JZ 0x00469c46` rejoins *past* it, so the `AnimList=` anim is
/// reached either way. `bullet+0xe0` is the per-**weapon** `Bright=` flag, not
/// a warhead key: `BulletClass::Init` stores its last stack argument there
/// (`0x004664e6 MOV byte ptr [ECX+0xe0], DL` from `[ESP+0x1c]`), fed from
/// `TechnoClass::FireAt` (`0x006fe53f MOV CL, byte ptr [EBX+0x12f]`) =
/// `WeaponTypeClass+0x12f`, written by `WeaponTypeClass::ReadINI` at
/// `0x00772817` from the key string at `0x00847dd0` = `"Bright"`. Whoever
/// ports `0x0048a620` must carry that gate, or every non-`Bright=` impact
/// flashes. The block itself folds warhead `+0x151`/`+0x152`/`+0x153`
/// (`CLDisableRed=`/`Green=`/`Blue=`, read at `0x00469bf8`/`0x00469c07`/
/// `0x00469c13`) into a 2/4/8 bit mask and passes it with the impact coord and
/// `bullet+0x6c`; the exact visual `0x0048a620` produces is UNCHECKED here.
///
/// RESIDUAL — **no special effect body is implemented in VERA.** Every variant
/// except `OrdinaryDamage` currently claims the detonation, suppresses damage
/// and shrapnel exactly as native does, and then runs the shared tail without
/// performing its effect. Per-variant native callees are named below; the
/// unimplemented effect families are ports M15b (Parasite), M15c (MindControl)
/// and M15d (the rest).
/// - Trigger: any impact whose warhead carries one of the eleven flags. In
///   stock `rulesmd.ini` that is 14 warhead sections, named by 24 weapon
///   sections through an exact-case `Warhead=`, of which 22 are actually
///   mounted (`TankMakeupKit` and `CRMakeupKit` are named by no mount key;
///   gamemd's INI lookup is case-sensitive, and there is no near-miss
///   spelling of any of the eleven keys or of `Warhead=` in stock) — carried
///   by Yuri/Yuri Prime/Psychic Tower/Mastermind, attack dogs, Terror Drones,
///   Giant Squids, Chrono Legionnaires, Crazy Ivan, Engineers, Spies,
///   Magnetrons, Tesla Troopers, the IFV's chrono weapon, Boris and the Weed
///   Guy.
/// - Player effect: the shot lands, plays its animation and leaves its crater,
///   but nobody is controlled, attached to, erased, bombed, defused, disguised
///   or lifted, and the target takes no damage from that shot.
/// - Frequency: continuous in ordinary skirmish — an attack dog appears in
///   almost every game and a Yuri beam fires every few seconds in any Yuri
///   game.
/// - Downstream risk: the ports add snapshotted, hashed entity state
///   (`ParasiteClass`, the victim-side mind-control link, `TemporalClass`,
///   `BombClass`), so each carries its own `SNAPSHOT_VERSION` bump.
///
/// RESIDUAL — **the `[ESP+0xf]` ordinary-arm visual bypass is not modelled.**
/// `0x004690c9` zeroes `[ESP+0xf]` on entry and `0x00469a9f` is its only
/// writer: it sets 1 when `Apply_area_damage` (called at `0x00469a83`) returns
/// exactly 2 and the `bullet+0x90` gate at `0x00469a94` passed
/// (`0x00469a9a CMP EAX,0x2` / `0x00469a9d JNZ 0x00469aa4`). When set, the tail
/// diverts at `0x00469bea` / `0x0046a299` to `0x0046a2a1`, which builds a
/// different `0x1c8`-byte object from global `[0x008871e0] + 0x350` and falls
/// straight into the epilogue — skipping the `AnimList=` anim, the combat
/// light, the debris loop and the `Airburst=` fan entirely. It is the same
/// `Apply_area_damage` return-value handling as the `bullet+0x90` gate above,
/// and it suppresses far more than that gate does.
/// - Trigger: an ordinary-arm impact where `Apply_area_damage` returns 2. What
///   that return value means is **UNCHECKED** — settled by reading
///   `Apply_area_damage @ 0x00489280`'s return contract.
/// - Player effect: on native such an impact draws the alternate object
///   instead of its explosion, crater, debris and `Airburst=` children; VERA
///   always draws the ordinary set.
/// - Frequency: UNCHECKED on the ordinary arm, and provably **zero on every
///   arm in this enum** — `0x00469a9f` is the flag's only writer and it sits
///   inside the final else at `0x00469a3f`, which `get_xrefs_to 0x00469a3f`
///   shows is entered by exactly one jump, the `NukeMaker` test's `JZ` at
///   `0x00469a34`. No special arm can reach it, so the flag is 0 for all of
///   them.
/// - Downstream risk: none for M15a. It belongs with the rest of the tail
///   (`Inviso`, debris, `Airburst=`) and with the `bullet+0x90` gate, i.e. to
///   whoever ports `Apply_area_damage`'s return contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialDetonationAction {
    /// `MindControl=` (`WarheadTypeClass+0x155`), test `0x00469211` ->
    /// `CaptureManagerClass::CaptureUnit @ 0x00471d40`. UNIMPLEMENTED (M15c).
    MindControl,
    /// `IvanBomb=` (`+0x157`), test `0x00469343` ->
    /// `BombClass::Attach @ 0x00438e70`. UNIMPLEMENTED (M15d).
    IvanBomb,
    /// `ElectricAssault=` (`+0x158`), test `0x0046937a` -> the Tesla-Coil
    /// charger-vector append at `0x00452820`. UNIMPLEMENTED (M15d).
    ElectricAssault,
    /// `Parasite=` (`+0x159`), test `0x004693d3` ->
    /// `ParasiteClass::AttachTo @ 0x0062a980`. UNIMPLEMENTED (M15b).
    Parasite,
    /// `Temporal=` (`+0x15a`), test `0x00469423` ->
    /// `TemporalClass::InitiateWarp @ 0x0071af20`. UNIMPLEMENTED (M15d).
    Temporal,
    /// `IsLocomotor=` (`+0x15b`), test `0x004694cb` -> the Magnetron
    /// deploy / chrono-warp arm. UNIMPLEMENTED (M15d).
    Locomotor,
    /// `Airstrike=` (`+0x16c`), test `0x00469705` ->
    /// `AirstrikeClass::SetTarget @ 0x0041d830`. UNIMPLEMENTED (M15d).
    ///
    /// The decompile shows this arm as conditional; the disassembly does not.
    /// `0x0046970d JZ 0x0046978e` is its only fall-through and fires on the
    /// flag alone — every sub-condition failure (`0x00469717`, `0x00469725`,
    /// `0x0046972f`, `0x0046973d`, `0x0046975b`) jumps to `0x00469aa4`
    /// instead. So Airstrike is flag-gated like the other ten.
    Airstrike,
    /// `DirectRocker=` (`+0x14f`), test `0x0046978e` ->
    /// `TechnoClass::ApplyRocker` through vtable `+0x3d8` at `0x004699a1`,
    /// plus the mutual `+0x2a8` link.
    ///
    /// The **only** conditional arm in the chain: `0x00469796` (flag),
    /// `0x004697a4` (`Target != 0`) and `0x004697b2`
    /// (`Target->What_Am_I() == 1`, `UnitClass`) all fall through to
    /// `0x004699c4`, the *next* test, so a `DirectRocker=yes` warhead that hits
    /// infantry, a building or a bare cell still takes ordinary damage.
    /// Failures *after* those three (`0x004697dc`, `0x004697e4`, `0x004697f6`)
    /// jump to the tail like every other arm.
    ///
    /// Dead in stock YR: `rulesmd.ini` carries no live `DirectRocker=` line
    /// (its one textual occurrence is inside a `;` comment), so the arm never
    /// fires in stock play. Kept correct anyway; the impulse body is
    /// deliberately NOT implemented.
    DirectRocker,
    /// `BombDisarm=` (`+0x16e`), test `0x004699ca` ->
    /// `BombClass::Defuse @ 0x004389b0`. UNIMPLEMENTED (M15d).
    BombDisarm,
    /// `MakesDisguise=` (`+0x175`), test `0x00469a03` -> the firer disguises as
    /// the target through owner vtable `+0x46c` at `0x00469a24`.
    /// UNIMPLEMENTED (M15d).
    MakesDisguise,
    /// `NukeMaker=` (`+0x176`), test `0x00469a2c` ->
    /// `BulletClass::SpawnDownwardNuke @ 0x0046b310`. UNIMPLEMENTED (M15d).
    NukeMaker,
    /// The final else at `0x00469a3f`: shrapnel plus `Apply_area_damage`.
    OrdinaryDamage,
}

impl SpecialDetonationAction {
    /// True for every arm that claims the detonation away from
    /// `Apply_area_damage @ 0x00489280` and
    /// `BulletClass::SpawnShrapnel @ 0x0046a310`.
    pub fn suppresses_ordinary_damage(self) -> bool {
        self != SpecialDetonationAction::OrdinaryDamage
    }
}

/// The eleven `WarheadTypeClass` flag bytes the native chain tests, in native
/// order. Offsets verified from `WarheadTypeClass::ReadINI` string->offset
/// writes (`0x0075d5d8`, `0x0075d7e0`, `0x0075d823`, `0x0075d82e`,
/// `0x0075d84e`, `0x0075d871`, `0x0075d87c`, `0x0075d8f0`, `0x0075d91b`,
/// `0x0075d969`, `0x0075d983`) and cross-checked against the reads in
/// `BulletClass::DetonateAtCoord @ 0x004690b0`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpecialDetonationFlags {
    pub mind_control: bool,
    pub ivan_bomb: bool,
    pub electric_assault: bool,
    pub parasite: bool,
    pub temporal: bool,
    pub is_locomotor: bool,
    pub airstrike: bool,
    pub direct_rocker: bool,
    pub bomb_disarm: bool,
    pub makes_disguise: bool,
    pub nuke_maker: bool,
}

/// Target-side context the native chain consults *inside* an arm predicate.
///
/// Only `DirectRocker` reads it. `0x0046979c`/`0x004697aa` load the bullet's
/// raw `+0x10c` target and call `What_Am_I` through vtable `+0x2c`; a null
/// target or any RTTI id other than 1 (`UnitClass::What_Am_I @ 0x00746e20`)
/// falls through to the BombDisarm test instead of claiming the detonation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpecialDetonationTarget {
    /// `Target != 0 && Target->What_Am_I() == 1`.
    pub is_unit: bool,
}

/// Resolve the one arm of `BulletClass::DetonateAtCoord @ 0x004690b0` that owns
/// this impact. The `else if` order is native; see [`SpecialDetonationAction`].
pub fn projectile_special_detonation_action(
    flags: SpecialDetonationFlags,
    target: SpecialDetonationTarget,
) -> SpecialDetonationAction {
    if flags.mind_control {
        SpecialDetonationAction::MindControl
    } else if flags.ivan_bomb {
        SpecialDetonationAction::IvanBomb
    } else if flags.electric_assault {
        SpecialDetonationAction::ElectricAssault
    } else if flags.parasite {
        SpecialDetonationAction::Parasite
    } else if flags.temporal {
        SpecialDetonationAction::Temporal
    } else if flags.is_locomotor {
        SpecialDetonationAction::Locomotor
    } else if flags.airstrike {
        // Flag-gated, not conditional — see the `Airstrike` variant doc.
        SpecialDetonationAction::Airstrike
    } else if flags.direct_rocker && target.is_unit {
        // The chain's only conditional arm: `0x004697a4` / `0x004697b2` fall
        // through to the BombDisarm test at `0x004699c4`, not to the tail.
        SpecialDetonationAction::DirectRocker
    } else if flags.bomb_disarm {
        SpecialDetonationAction::BombDisarm
    } else if flags.makes_disguise {
        SpecialDetonationAction::MakesDisguise
    } else if flags.nuke_maker {
        SpecialDetonationAction::NukeMaker
    } else {
        SpecialDetonationAction::OrdinaryDamage
    }
}

/// Stable projectile payload transferred to combat only at detonation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectilePayload {
    /// Damage after firing-side modifiers, before target/warhead resolution.
    pub base_damage: i32,
    pub warhead: InternedId,
    /// Weapon identity retained for impact-only effects such as radiation.
    pub weapon: InternedId,
    /// Firing house retained when the source dies before impact.
    pub owner: InternedId,
}

/// Immutable admission data for an ordinary, non-vertical projectile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectileSpawn {
    pub source_id: u64,
    pub origin: ProjectileCoord,
    pub target: ProjectileTarget,
    /// Target coordinate captured when the weapon fires. Non-homing shots keep
    /// this destination; homing shots replace it from the live target table.
    pub initial_target_position: ProjectileCoord,
    pub payload: ProjectilePayload,
    /// The bullet's CURRENT speed in leptons per frame — native's
    /// `ftol(|velocity|)`, not the weapon's `Speed=`. `TechnoClass::FireAt
    /// @ 0x006FEA3A` launches every `ROT > 0` or `Vertical` bullet at 1, and
    /// `BulletClass::Fire @ 0x00468B2C` renormalises a `ROT > 0` bullet's
    /// vector to magnitude 1.0 regardless; `Speed=` only becomes the ceiling
    /// stored at `Bullet+0x110`. Non-homing, non-vertical bullets do keep the
    /// launch speed here, clamped to `dist/2` at `0x006FE9FE`.
    pub speed_leptons_per_frame: u16,
    pub velocity: ProjectileVelocity,
    pub trajectory: ProjectileTrajectory,
    /// Present only for a `BulletTypeClass::ROT` guided flight.
    pub guidance: Option<ProjectileGuidance>,
    pub visual: ProjectileVisualState,
    /// A projectile may not detonate until this many frames have elapsed.
    pub arm_frames: u16,
    /// Optional fuse duration; zero means the fuse detonates on this advance.
    pub fuse_frames: Option<u16>,
    /// `ROT > 0 || Ranged` admits the native closest-approach fuse helper.
    pub ranged_fuse: bool,
    /// Only homing projectiles update their destination from a live target.
    pub tracks_target: bool,
    pub target_expiry: TargetExpiryPolicy,
    pub collision: ProjectileCollisionPolicy,
}

/// Persistent state corresponding to one native `BulletClass` instance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Projectile {
    pub id: u64,
    /// LogicClass membership is reconstructed from the serialized mixed order.
    #[serde(skip)]
    pub in_logic_vector: bool,
    pub source_id: u64,
    pub position: ProjectileCoord,
    pub target: ProjectileTarget,
    pub last_target_position: ProjectileCoord,
    pub payload: ProjectilePayload,
    pub speed_leptons_per_frame: u16,
    pub velocity: ProjectileVelocity,
    pub trajectory: ProjectileTrajectory,
    pub guidance: Option<ProjectileGuidance>,
    pub visual: ProjectileVisualState,
    pub arm_frames_remaining: u16,
    pub fuse_frames_remaining: Option<u16>,
    pub ranged_fuse: bool,
    pub last_distance_half: i32,
    pub tracks_target: bool,
    pub target_expiry: TargetExpiryPolicy,
    pub collision: ProjectileCollisionPolicy,
}

/// Why a projectile reached its combat detonation handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProjectileDetonationReason {
    ReachedTarget,
    Fuse,
    Collision,
    TargetExpired,
}

/// One deferred `BulletClass::Detonate` handoff for combat to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectileDetonation {
    pub projectile_id: u64,
    pub source_id: u64,
    pub target: ProjectileTarget,
    pub impact: ProjectileCoord,
    pub payload: ProjectilePayload,
    pub reason: ProjectileDetonationReason,
}

/// Results from one stable-order projectile pass.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ProjectileAdvanceResult {
    pub detonations: Vec<ProjectileDetonation>,
    pub expired: Vec<u64>,
}

/// Serialized, stable-id ordered projectile collection.
///
/// `BTreeMap` makes creation-order IDs and processing order explicit. New
/// projectiles are only advanced by the next call, matching the usual
/// object-pass boundary instead of recursively advancing a newly fired shot.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectileStore {
    projectiles: BTreeMap<u64, Projectile>,
}

impl ProjectileStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.projectiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.projectiles.is_empty()
    }

    pub fn get(&self, id: u64) -> Option<&Projectile> {
        self.projectiles.get(&id)
    }

    pub(crate) fn get_mut(&mut self, id: u64) -> Option<&mut Projectile> {
        self.projectiles.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u64, &Projectile)> {
        self.projectiles.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (&u64, &mut Projectile)> {
        self.projectiles.iter_mut()
    }

    pub(crate) fn remove(&mut self, id: u64) -> Option<Projectile> {
        self.projectiles.remove(&id)
    }

    /// Apply `BulletClass::PointerExpired @ 0x004684E0` to one stored Bullet.
    ///
    /// Source and target are independent arms: when both match the expired
    /// object, both change in the same synchronous callback. The projectile
    /// itself remains stored and registered; repeated callbacks are no-ops.
    pub(crate) fn pointer_expired(
        &mut self,
        projectile_id: u64,
        expired_id: u64,
        replacement_target: ProjectileTarget,
    ) -> bool {
        let Some(projectile) = self.projectiles.get_mut(&projectile_id) else {
            return false;
        };
        if projectile.source_id == expired_id {
            projectile.source_id = crate::sim::combat::RAD_NO_ATTACKER;
        }
        if projectile.target == ProjectileTarget::Entity(expired_id) {
            projectile.target = replacement_target;
        }
        true
    }

    /// Admit one projectile. All three `BulletClass::AI` flight arms are
    /// represented — `ROT >= 1` homing, the `ROT < 1` ballistic arm, and the
    /// `ROT < 1, Vertical` arm; an `Inviso` bullet still resolves on combat's
    /// immediate path because native's `BulletClass::Fire` scales its velocity
    /// by `0.0 / |v|` at `0x00468A0E` and resolves the impact in the same call.
    // AbstractClass::AssignUniqueID @ 0x00410230 obtains this identity from
    // ScenarioClass::NextUniqueID @ 0x0068BCB0; the store never owns a second
    // allocator.
    pub fn spawn(&mut self, id: u64, spawn: ProjectileSpawn) -> u64 {
        self.projectiles.insert(
            id,
            Projectile {
                id,
                in_logic_vector: false,
                source_id: spawn.source_id,
                position: spawn.origin,
                target: spawn.target,
                last_target_position: spawn.initial_target_position,
                payload: spawn.payload,
                speed_leptons_per_frame: spawn.speed_leptons_per_frame,
                velocity: spawn.velocity,
                trajectory: spawn.trajectory,
                guidance: spawn.guidance,
                visual: spawn.visual,
                arm_frames_remaining: spawn.arm_frames,
                fuse_frames_remaining: spawn.fuse_frames,
                ranged_fuse: spawn.ranged_fuse,
                // `ProximityDetector::Setup @ 0x004E1130` seeds `+0x24` with
                // the FULL launch distance from the bullet to its reference
                // coordinate, not the halved form `Check` writes afterwards.
                last_distance_half: coord_distance(spawn.origin, spawn.initial_target_position),
                tracks_target: spawn.tracks_target,
                target_expiry: spawn.target_expiry,
                collision: spawn.collision,
            },
        );
        id
    }

    /// Advance every currently admitted projectile in ascending stable id.
    ///
    /// `target_positions` must contain live entity targets in lepton space.
    /// `terrain` supplies the current CellClass ground surface and live
    /// structural bit for stable cell targets; headless callers may omit it
    /// and receive the flat fallback.
    /// `collides_at` is a world-owned terrain/wall admission predicate for the
    /// candidate next coordinate; object collision remains a later port.
    pub fn advance(
        &mut self,
        binary_frame: u32,
        target_positions: &BTreeMap<u64, ProjectileCoord>,
        terrain: Option<&ResolvedTerrainGrid>,
        shared_cell_dummy: &SharedCellDummy,
        collides_at: impl FnMut(&Projectile, ProjectileCoord) -> Option<ProjectileCollisionResponse>,
    ) -> ProjectileAdvanceResult {
        let ids: Vec<u64> = self.projectiles.keys().copied().collect();
        self.advance_selected(
            &ids,
            binary_frame,
            |id| target_positions.get(&id).copied(),
            terrain,
            shared_cell_dummy,
            false,
            collides_at,
            true,
        )
    }

    pub(crate) fn advance_one(
        &mut self,
        id: u64,
        binary_frame: u32,
        target_position: impl FnMut(u64) -> Option<ProjectileCoord>,
        terrain: Option<&ResolvedTerrainGrid>,
        shared_cell_dummy: &SharedCellDummy,
        source_is_jumpjet: bool,
        collides_at: impl FnMut(&Projectile, ProjectileCoord) -> Option<ProjectileCollisionResponse>,
    ) -> Option<ProjectileAdvanceResult> {
        if !self.projectiles.contains_key(&id) {
            return None;
        }
        Some(self.advance_selected(
            &[id],
            binary_frame,
            target_position,
            terrain,
            shared_cell_dummy,
            source_is_jumpjet,
            collides_at,
            false,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn advance_selected(
        &mut self,
        ids: &[u64],
        binary_frame: u32,
        mut resolve_target_position: impl FnMut(u64) -> Option<ProjectileCoord>,
        terrain: Option<&ResolvedTerrainGrid>,
        shared_cell_dummy: &SharedCellDummy,
        source_is_jumpjet: bool,
        mut collides_at: impl FnMut(&Projectile, ProjectileCoord) -> Option<ProjectileCollisionResponse>,
        remove_terminal: bool,
    ) -> ProjectileAdvanceResult {
        let mut result = ProjectileAdvanceResult::default();

        for &id in ids {
            let Some(projectile) = self.projectiles.get_mut(&id) else {
                continue;
            };

            let target_position = match projectile.target {
                ProjectileTarget::Cell { rx, ry } => cell_target_coord(terrain, rx, ry),
                // BulletClass::AI resolves a null AbstractClass target through
                // the process-global zero CoordStruct before steering, fuse,
                // collision, and reached-target decisions.
                ProjectileTarget::None => ProjectileCoord::new(0, 0, 0),
                ProjectileTarget::DummyCell => dummy_cell_target_coord(shared_cell_dummy),
                ProjectileTarget::Entity(target_id) => match resolve_target_position(target_id) {
                    Some(position) => {
                        if projectile.tracks_target {
                            projectile.last_target_position = position;
                        }
                        projectile.last_target_position
                    }
                    None => match projectile.target_expiry {
                        TargetExpiryPolicy::Expire => {
                            result.expired.push(id);
                            continue;
                        }
                        // `Arm=` is NOT consulted here. The detonation
                        // admission at `BulletClass::AI 0x00467C70` tests only
                        // the impact flag and the proximity-fuse result; the
                        // Arm counter lives inside `ProximityDetector::Check`
                        // and gates that one result alone.
                        TargetExpiryPolicy::DetonateAtLastKnown => {
                            result.detonations.push(detonation(
                                projectile,
                                projectile.last_target_position,
                                ProjectileDetonationReason::TargetExpired,
                            ));
                            continue;
                        }
                    },
                },
            };

            if let Some(fuse) = projectile.fuse_frames_remaining.as_mut() {
                if *fuse == 0 {
                    result.detonations.push(detonation(
                        projectile,
                        projectile.position,
                        ProjectileDetonationReason::Fuse,
                    ));
                    continue;
                }
                *fuse -= 1;
            }

            // YR BulletClass::Update @ 0x004666e0 advances the image bytes
            // before entering the trajectory portion of BulletClass::AI.
            projectile.visual.advance();

            // The arm gate is a WALL-CLOCK frame delta, not a count of AI
            // visits: `ProximityDetector::Setup @ 0x004E1130` stores
            // `g_CurrentFrameCounter` into detector `+0x0C` at launch and
            // `ProximityDetector::Check @ 0x004E11F0` admits the fuse on
            // `Arm (+0x14) <= g_CurrentFrameCounter - [+0x0C]`. VERA admits a
            // projectile only AFTER the live-object pass
            // (`World::advance_tick` -> `tick_combat_with_fatal_lifecycle` ->
            // `Simulation::admit_projectile`, `world/mod.rs`; the follow-up
            // tail walk `visit_combat_appended_wave_tail` visits Waves only),
            // so a projectile's first `advance_one` runs on the tick after
            // launch, where the native delta is already 1. Decrementing at the
            // top of the frame therefore reproduces `Arm <= elapsed` on every
            // frame — and it does so whether or not native's LogicClass pass
            // also runs a launch-frame `BulletClass::AI`, because that question
            // changes how many AI visits precede a frame, not the frame delta.
            // Nothing else consults `Arm`: the detonation admission at
            // `BulletClass::AI 0x00467C70` never tests it.
            //
            // One exception to "only AFTER the live-object pass", noted for
            // accuracy rather than as a defect: shrapnel children are admitted
            // from INSIDE the walk — `object_ai_visit_one` calls
            // `commit_logic_projectile_detonations` (`world/techno_ai.rs`) and
            // `for_each_live_object` reloads the length each iteration — so
            // such a child IS visited in its own launch tick, where a
            // decrement-at-top would arm it one frame early. Inert in stock:
            // the only `ShrapnelWeapon=` chains are
            // `SuperCometFragment`/`TeslaFragment`/`CometFragment` ->
            // `[SuperSmallCometP]`/`[SmallTeslaP]`/`[SmallCometP]`, all
            // `Inviso=yes` with no `Ranged=` and no `Arm=`, so no shrapnel child
            // carries a fuse and this counter never runs for one.
            //
            // RESIDUAL (VERA-internal, gamemd equivalent UNCHECKED): native's
            // `LogicClass::PerTickUpdate @ 0x0055AFB0` object loop reloads its
            // count after every `vtable+0x5C` callback and
            // `LogicClass::RegisterObject @ 0x0055BAA0` tail-appends, so a
            // bullet fired from an earlier index MAY take a launch-frame AI
            // that VERA skips; the Ghidra plate comment on `BulletClass::AI`
            // records same-pass execution of a concrete appended object as
            // UNCHECKED. Trigger: every persistent projectile. Player effect:
            // the shot's whole flight is one frame behind native, so it lands
            // one frame late — at the new 1-4 leptons/frame launch speed that
            // is a few leptons of travel. Frequency: every tracked shot.
            // Downstream risk: deterministic state only; the arm gate above is
            // unaffected because it is keyed on the frame counter.
            if projectile.arm_frames_remaining > 0 {
                projectile.arm_frames_remaining -= 1;
            }

            let previous_position = projectile.position;
            // Native `local_198` — set by any arm that hits something. It is
            // the only value besides the fuse that admits a detonation.
            let mut impact_flag = false;
            // Native carries one impact flag; VERA keeps the reason beside it
            // purely as diagnostic provenance for the combat handoff.
            let mut impact_reason = ProjectileDetonationReason::Collision;
            // Native `local_190 == 2`: `FUN_00568350` says the new coordinate
            // left the map, and `LAB_00467B7A` removes the bullet through
            // `vtable+0x124(2)` and `ObjectClass::UnInit` with NO detonation.
            let mut left_the_map = false;
            let mut snap_impact: Option<ProjectileCoord> = None;

            let candidate = if let Some(mut guidance) = projectile.guidance {
                // ---- ARM C, `BulletClass::AI 0x004668D9`..`0x00466A33` ----
                // The speed ramp runs before any steering. `Bullet+0x110` is
                // MaxSpeed; the current magnitude is `ftol(|v|)`, which native
                // keeps integral from the 1.0 launch onward.
                let max_speed = i32::from(guidance.max_speed);
                let current_speed = i32::from(projectile.speed_leptons_per_frame);
                // `0x00466925`..`0x00466978`: with `CourseLockDuration == 0`
                // the lock clears once MaxSpeed reaches 40 (`CMP ...,0x28` /
                // `JGE`) or the magnitude comes within 0.5 of MaxSpeed;
                // otherwise a per-bullet counter runs the authored duration
                // out. Both releases latch, and both are monotone in the
                // ramping speed, so deriving them per frame is equivalent to
                // native's stored `IsCourseLocked` byte.
                let course_locked = if guidance.course_lock_duration == 0 {
                    max_speed < 40 && max_speed > current_speed
                } else {
                    guidance.frames_elapsed.saturating_add(1)
                        < u32::from(guidance.course_lock_duration)
                };
                // `0x0046699D`: a still-locked bullet with no authored
                // duration ramps at one lepton on even frames and none on odd.
                let acceleration = if course_locked && guidance.course_lock_duration == 0 {
                    i32::from(binary_frame.is_multiple_of(2))
                } else {
                    guidance.acceleration
                };
                let next_speed = if current_speed < max_speed {
                    (current_speed + acceleration).min(max_speed)
                } else if current_speed > max_speed {
                    // `0x00466A33`: overspeed sheds `Acceleration / 2` a frame
                    // and floors at zero.
                    (current_speed - acceleration / 2).max(0)
                } else {
                    current_speed
                };

                // `BulletClass::HomingTrack` steering. The turn word is zero
                // while course-locked (`bVar4 &= ~-(IsCourseLocked != 0)`).
                let target_distance = horizontal_distance(projectile.position, target_position);
                let phase = guidance
                    .frames_elapsed
                    .wrapping_add(u32::from(guidance.sidewinder_phase));
                let varied_rot = (sidewinder_cos(phase) * guidance.missile_rot_var
                    + guidance.missile_rot_var
                    + SimFixed::from_num(1))
                    * SimFixed::from_num(guidance.rot);
                let turn_word =
                    projectile_rot_turn_word_fixed(varied_rot, target_distance, course_locked);
                let desired_yaw = atan2_bam(
                    SimFixed::from_num(target_position.y - projectile.position.y),
                    SimFixed::from_num(target_position.x - projectile.position.x),
                );
                let yaw = step_toward_bam_inclusive(guidance.heading_bam, desired_yaw, turn_word);
                guidance.heading_bam = yaw;
                projectile.velocity.x =
                    (SimFixed::from_num(next_speed) * cos_bam(yaw)).to_num::<i32>();
                projectile.velocity.y =
                    (SimFixed::from_num(next_speed) * sin_bam(yaw)).to_num::<i32>();
                projectile.speed_leptons_per_frame =
                    next_speed.clamp(0, i32::from(u16::MAX)) as u16;
                guidance.frames_elapsed = guidance.frames_elapsed.wrapping_add(1);

                let candidate = ProjectileCoord::new(
                    projectile.position.x.saturating_add(projectile.velocity.x),
                    projectile.position.y.saturating_add(projectile.velocity.y),
                    projectile.position.z.saturating_add(projectile.velocity.z),
                );

                // gamemd-derived: `BulletClass::AI 0x00466DB1..0x00466E6B`.
                // The +0x1C8 receiver is ObjectClass::GetHeight @ 0x005F5F40
                // on the OLD object coordinate; HomingTrack has only updated
                // the stack candidate. Stock persistent (non-Inviso) bullets
                // retain the constructor's OnBridge=false, so this height is
                // above terrain, even when the cell contains a bridge.
                let reached_distance = coord_distance(candidate, target_position);
                let old_height = previous_position.z.wrapping_sub(projectile_ground_z(
                    terrain,
                    shared_cell_dummy,
                    previous_position,
                ));
                let (admit_impact, snap_to_target) = homing_impact_admission(
                    reached_distance,
                    projectile.velocity,
                    old_height,
                    guidance.airburst,
                    target_position != ProjectileCoord::new(0, 0, 0),
                );
                if admit_impact {
                    impact_flag = true;
                    impact_reason = if old_height <= 0 {
                        ProjectileDetonationReason::Collision
                    } else {
                        ProjectileDetonationReason::ReachedTarget
                    };
                    if snap_to_target {
                        snap_impact = Some(target_position);
                    }
                }

                // The closing-rate heuristic. `delta` is how much closer this
                // step got; native accumulates it raw for the first 60 frames
                // and then decays, detonating inside the `[0, 60)` window when
                // the bullet is neither `Airburst` nor `VeryHigh`. It runs
                // only once the course lock has cleared.
                if !course_locked {
                    let delta = f64::from(
                        coord_distance(previous_position, target_position) - reached_distance,
                    );
                    let accumulator = f64::from_bits(guidance.closing_accumulator_bits);
                    if guidance.closing_frames < 60 {
                        guidance.closing_frames += 1;
                        guidance.closing_accumulator_bits = (accumulator + delta).to_bits();
                    } else {
                        let decayed = accumulator * 0.983_333_333_333_333_3 + delta;
                        guidance.closing_accumulator_bits = decayed.to_bits();
                        if (0.0..60.0).contains(&decayed)
                            && !guidance.airburst
                            && !guidance.very_high
                        {
                            impact_flag = true;
                            impact_reason = ProjectileDetonationReason::ReachedTarget;
                        }
                    }
                }

                projectile.guidance = Some(guidance);
                candidate
            } else {
                match projectile.trajectory {
                    ProjectileTrajectory::Straight => {
                        let candidate = step_toward(
                            projectile.position,
                            target_position,
                            i32::from(projectile.speed_leptons_per_frame),
                        );
                        projectile.velocity = ProjectileVelocity::new(
                            candidate.x - projectile.position.x,
                            candidate.y - projectile.position.y,
                            candidate.z - projectile.position.z,
                        );
                        candidate
                    }
                    ProjectileTrajectory::Ballistic { gravity } => {
                        projectile.velocity.z = projectile.velocity.z.saturating_sub(gravity);
                        ProjectileCoord::new(
                            projectile.position.x.saturating_add(projectile.velocity.x),
                            projectile.position.y.saturating_add(projectile.velocity.y),
                            projectile.position.z.saturating_add(projectile.velocity.z),
                        )
                    }
                    // ---- ARM A, `BulletClass::AI 0x004671E0`..`0x00467390` ----
                    ProjectileTrajectory::Vertical {
                        detonation_altitude,
                        acceleration,
                        max_speed,
                        heading_bam,
                        pitch_bam,
                    } => {
                        // `0x00467211`: the ramp uses the truncated integer
                        // magnitude and applies NO clamp on the crossing frame,
                        // so the magnitude may overshoot MaxSpeed once.
                        let current_speed = i32::from(projectile.speed_leptons_per_frame);
                        let next_speed = if current_speed < max_speed {
                            current_speed + acceleration
                        } else {
                            current_speed
                        };
                        projectile.speed_leptons_per_frame =
                            next_speed.clamp(0, i32::from(u16::MAX)) as u16;
                        let horizontal = SimFixed::from_num(next_speed) * cos_bam(pitch_bam);
                        projectile.velocity = ProjectileVelocity::new(
                            (horizontal * cos_bam(heading_bam)).to_num::<i32>(),
                            (horizontal * sin_bam(heading_bam)).to_num::<i32>(),
                            (SimFixed::from_num(next_speed) * sin_bam(pitch_bam)).to_num::<i32>(),
                        );
                        let candidate = ProjectileCoord::new(
                            projectile.position.x.saturating_add(projectile.velocity.x),
                            projectile.position.y.saturating_add(projectile.velocity.y),
                            projectile.position.z.saturating_add(projectile.velocity.z),
                        );
                        // `0x00467334`: `DetonationAltitude` is compared
                        // against the new WORLD z, not against an altitude
                        // above ground. Then a negative altitude, then the
                        // bridge-deck crossing test. No gravity is applied on
                        // this arm at all.
                        if candidate.z > detonation_altitude {
                            impact_flag = true;
                        } else if terrain_floor_z(terrain, candidate.x, candidate.y)
                            .is_some_and(|floor| candidate.z < floor)
                        {
                            impact_flag = true;
                        } else if let Some(surface) =
                            bridge_surface_z(terrain, previous_position, candidate)
                            && projectile_bridge_crossing(previous_position.z, candidate.z, surface)
                                != ProjectileBridgeCrossing::None
                        {
                            impact_flag = true;
                        }
                        candidate
                    }
                }
            };

            // `FUN_00568350(newCoord) == 0` -> terminal reason 2. Only the
            // `ROT < 1` arms run this probe in native, so a homing bullet is
            // never silently removed for leaving the map.
            if projectile.guidance.is_none()
                && let Some(grid) = terrain
                && !coord_is_on_grid(grid, candidate)
            {
                left_the_map = true;
            }

            if left_the_map {
                result.expired.push(id);
                continue;
            }

            // The homing arm's admitted impact bypasses the post-move probe
            // (`0x00467BA4..0x00467BD1`). ROT<1 collision has its own earlier
            // block and retains its existing callback order.
            let collision_impact = (!(projectile.guidance.is_some() && impact_flag))
                .then(|| collides_at(projectile, candidate))
                .flatten()
                .map(|response| match response {
                    ProjectileCollisionResponse::TargetZClamp(impact) => impact,
                    ProjectileCollisionResponse::SlopeMatrixReflect { impact, velocity } => {
                        projectile.velocity = velocity;
                        impact
                    }
                });
            if let Some(impact) = collision_impact {
                impact_flag = true;
                impact_reason = ProjectileDetonationReason::Collision;
                snap_impact = Some(impact);
            }

            let mut impact = snap_impact.unwrap_or(candidate);
            if projectile.guidance.is_some() && impact_flag {
                // `0x00467BF0..0x00467C06`: the getter and, when negative,
                // setter both query the committed coordinate before the fuse.
                // The setter does not rewrite the stack coordinate used below.
                let floor = projectile_ground_z(terrain, shared_cell_dummy, impact);
                if impact.z.wrapping_sub(floor) < 0 {
                    impact.z = projectile_ground_z(terrain, shared_cell_dummy, impact);
                }
            }

            // `ProximityDetector::Check @ 0x004E11F0`. The reference is the
            // coordinate frozen at launch, the Arm counter gates this result
            // and nothing else, and `Dropping=` suppresses it entirely at
            // `0x00467C78` — folded into `ranged_fuse` at construction.
            let mut fuse_mode = 0;
            if projectile.ranged_fuse {
                let reference = projectile
                    .guidance
                    .map_or(projectile.last_target_position, |guidance| {
                        guidance.fuse_reference
                    });
                if projectile.arm_frames_remaining == 0 {
                    let distance = coord_distance(snap_impact.unwrap_or(candidate), reference);
                    let (mode, next_distance) =
                        ranged_fuse_distance_step(distance, projectile.last_distance_half);
                    projectile.last_distance_half = next_distance;
                    fuse_mode = mode;
                }
            }
            // `0x00467C3C..0x00467C66`: Bullet+0xB0 is the live firer,
            // whose type +0xD94 (JumpJet) changes overshoot mode 2 into 1.
            // Pointer cleanup can clear the source between flight visits.
            fuse_mode = projectile_fuse_mode(fuse_mode, source_is_jumpjet);

            if !impact_flag && fuse_mode == 0 {
                projectile.position = candidate;
                let reached_target = match projectile.trajectory {
                    ProjectileTrajectory::Ballistic { .. } => {
                        candidate.z <= target_position.z
                            && squared_horizontal_distance(candidate, target_position)
                                <= i64::from(projectile.speed_leptons_per_frame).pow(2)
                    }
                    ProjectileTrajectory::Straight if projectile.guidance.is_none() => {
                        candidate == target_position
                    }
                    ProjectileTrajectory::Straight | ProjectileTrajectory::Vertical { .. } => false,
                };
                if reached_target {
                    result.detonations.push(detonation(
                        projectile,
                        candidate,
                        ProjectileDetonationReason::ReachedTarget,
                    ));
                }
                continue;
            }

            if let Some(guidance) = projectile.guidance {
                // `0x00467CA9..0x00467E4D`: a mode-1 fuse unconditionally
                // selects target +0x48 after the Airburst/Inaccurate gates.
                // Re-read a shared dummy here: earlier ground lookups may
                // have stamped a different coordinate into that same target.
                if fuse_mode == 1 && !guidance.airburst && !guidance.inaccurate {
                    let location = match projectile.target {
                        ProjectileTarget::Entity(id) => resolve_target_position(id),
                        ProjectileTarget::Cell { rx, ry } => {
                            let mut location = cell_target_coord(terrain, rx, ry);
                            location.z = projectile_ground_z(terrain, shared_cell_dummy, location);
                            Some(location)
                        }
                        ProjectileTarget::DummyCell => {
                            let mut location = dummy_cell_target_coord(shared_cell_dummy);
                            let snapshot = shared_cell_dummy.snapshot();
                            location.z = crate::util::lepton::ground_height_leptons(
                                snapshot.level as u8,
                                snapshot.slope_type,
                                location.x,
                                location.y,
                            )
                            .expect("shared CellClass target must have a supported slope");
                            Some(location)
                        }
                        ProjectileTarget::None => None,
                    };
                    if let Some(location) = location {
                        impact = location;
                    }
                }
            }
            let reason = if impact_flag {
                impact_reason
            } else {
                ProjectileDetonationReason::Fuse
            };
            result
                .detonations
                .push(detonation(projectile, impact, reason));
        }

        if remove_terminal {
            for id in result.expired.iter().chain(
                result
                    .detonations
                    .iter()
                    .map(|detonation| &detonation.projectile_id),
            ) {
                self.projectiles.remove(id);
            }
        }
        result
    }
}

/// YR ranged/ROT helper at `BulletClass::AI`: 0 continues, 1 is within the
/// 0x20 half-distance threshold, and 2 has passed closest approach nearby.
pub fn ranged_fuse_distance_step(distance_fistp: i32, last_distance: i32) -> (i32, i32) {
    let distance_half = (distance_fistp - (distance_fistp >> 31)) >> 1;
    if distance_half < 0x20 {
        (1, last_distance)
    } else if distance_half < 0x100 && distance_half > last_distance {
        (2, last_distance)
    } else {
        (0, distance_half)
    }
}

/// `Math__ftol(sqrt(dx*dx + dy*dy + dz*dz))` — the truncating straight-line
/// lepton distance native uses for the proximity fuse, the homing reach test
/// and the closing-rate accumulator.
fn coord_distance(a: ProjectileCoord, b: ProjectileCoord) -> i32 {
    let dx = f64::from(a.x - b.x);
    let dy = f64::from(a.y - b.y);
    let dz = f64::from(a.z - b.z);
    (dx * dx + dy * dy + dz * dz).sqrt().trunc() as i32
}

/// `CellClass::GetGroundHeight @ 0x00578080`: signed /256, fixed-512
/// allocation lookup, and live shared-dummy fallback, evaluated at the original
/// lepton XY. The fallback stamp is observable by a retained DummyCell target.
fn projectile_ground_z(
    terrain: Option<&ResolvedTerrainGrid>,
    shared_cell_dummy: &SharedCellDummy,
    coord: ProjectileCoord,
) -> i32 {
    use crate::sim::cell_rect::{CellRef, get_cellclass_fallback_leptons};
    let (level, slope) = if let Some(terrain) = terrain {
        match get_cellclass_fallback_leptons(Some(terrain), coord.x, coord.y) {
            CellRef::Real(cell) => (cell.level, cell.slope_type),
            CellRef::Dummy { cell } => {
                let snapshot = cell.snapshot();
                (snapshot.level as u8, snapshot.slope_type)
            }
        }
    } else {
        shared_cell_dummy.stamp_coord(coord.x / 256, coord.y / 256);
        let snapshot = shared_cell_dummy.snapshot();
        (snapshot.level as u8, snapshot.slope_type)
    };
    crate::util::lepton::ground_height_leptons(level, slope, coord.x, coord.y)
        .expect("projectile ground probe requires a supported CellClass slope")
}

/// `BulletClass::AI 0x00466DB1..0x00466E6B`, with its already-computed
/// distance, represented velocity and OLD ObjectClass height as inputs.
/// Native retains a double vector; upstream VERA flight still quantizes its
/// components, an open trajectory discrepancy rather than a scalar-speed
/// substitute at this predicate.
fn homing_impact_admission(
    distance: i32,
    velocity: ProjectileVelocity,
    old_height: i32,
    airburst: bool,
    nonzero_target: bool,
) -> (bool, bool) {
    let x = f64::from(velocity.x);
    let y = f64::from(velocity.y);
    let z = f64::from(velocity.z);
    let half_speed = (x * x + y * y + z * z).sqrt() * 0.5;
    let impact = f64::from(distance) <= half_speed || old_height <= 0;
    (
        impact,
        impact && old_height > 0 && !airburst && nonzero_target,
    )
}

fn projectile_fuse_mode(detector_mode: i32, source_is_jumpjet: bool) -> i32 {
    if source_is_jumpjet && detector_mode == 2 {
        1
    } else {
        detector_mode
    }
}

/// `CellClass::GetGroundHeight` at a lepton coordinate, or `None` when the
/// coordinate has no allocated CellClass (headless fixtures and off-map).
fn terrain_floor_z(terrain: Option<&ResolvedTerrainGrid>, x: i32, y: i32) -> Option<i32> {
    let rx = u16::try_from(x.div_euclid(256)).ok()?;
    let ry = u16::try_from(y.div_euclid(256)).ok()?;
    let cell = terrain?.cell(rx, ry)?;
    crate::util::lepton::ground_height_leptons(cell.level, cell.slope_type, x, y).ok()
}

/// The bridge deck height for `BulletClass::AI`'s crossing test: the ground
/// height plus `DAT_0089DE64`, admitted only when either the previous or the
/// new cell carries the structural bridge bit (`CellClass+0x140 & 0x100`).
fn bridge_surface_z(
    terrain: Option<&ResolvedTerrainGrid>,
    previous: ProjectileCoord,
    candidate: ProjectileCoord,
) -> Option<i32> {
    let grid = terrain?;
    let structural = |coord: ProjectileCoord| -> bool {
        let (Ok(rx), Ok(ry)) = (
            u16::try_from(coord.x.div_euclid(256)),
            u16::try_from(coord.y.div_euclid(256)),
        ) else {
            return false;
        };
        grid.cell(rx, ry).is_some_and(|cell| {
            cell.bridge_facts.raw_flags & crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL != 0
        })
    };
    if !structural(previous) && !structural(candidate) {
        return None;
    }
    terrain_floor_z(terrain, candidate.x, candidate.y)
        .map(|floor| floor + crate::util::lepton::BRIDGE_HEIGHT_DELTA_LEPTONS as i32)
}

/// `FUN_00568350`: whether the coordinate is still inside the playfield.
fn coord_is_on_grid(grid: &ResolvedTerrainGrid, coord: ProjectileCoord) -> bool {
    let (Ok(rx), Ok(ry)) = (
        u16::try_from(coord.x.div_euclid(256)),
        u16::try_from(coord.y.div_euclid(256)),
    ) else {
        return false;
    };
    grid.cell(rx, ry).is_some()
}

fn squared_horizontal_distance(a: ProjectileCoord, b: ProjectileCoord) -> i64 {
    let dx = i64::from(a.x) - i64::from(b.x);
    let dy = i64::from(a.y) - i64::from(b.y);
    dx * dx + dy * dy
}

fn horizontal_distance(a: ProjectileCoord, b: ProjectileCoord) -> i32 {
    squared_horizontal_distance(a, b)
        .isqrt()
        .min(i64::from(i32::MAX)) as i32
}

/// YR `sub_47F9B0`: closed-form arcing launch solver. Runtime stores only the
/// resulting integer velocity, keeping subsequent simulation fixed/integer.
pub fn ballistic_launch_velocity(
    origin: ProjectileCoord,
    target: ProjectileCoord,
    speed: i32,
    gravity: i32,
    high_arc: bool,
) -> Option<ProjectileVelocity> {
    if speed <= 0 || gravity < 0 {
        return None;
    }
    let dx = f64::from(target.x - origin.x);
    let dy = f64::from(target.y - origin.y);
    let dz = f64::from(target.z - origin.z);
    let range = dx.hypot(dy);
    let speed_f = f64::from(speed);
    let gravity_f = f64::from(gravity);
    let angle = if dz > 0.0 {
        std::f64::consts::FRAC_PI_4
    } else {
        let speed2 = speed_f * speed_f;
        let discriminant =
            speed2 * speed2 - 2.0 * speed2 * dz * gravity_f - gravity_f * gravity_f * range * range;
        let denominator = 2.0
            * (if range == 0.0 {
                1.0
            } else {
                dz * dz / (range * range) + 1.0
            });
        if discriminant < 0.0 || denominator == 0.0 {
            return None;
        }
        let numerator = speed2 - dz * gravity_f
            + if high_arc {
                -discriminant.sqrt()
            } else {
                discriminant.sqrt()
            };
        if numerator < 0.0 {
            return None;
        }
        ((numerator / denominator).sqrt() / speed_f).asin()
    };
    let horizontal_speed = speed_f * angle.cos();
    let facing = dy.atan2(dx);
    Some(ProjectileVelocity::new(
        (horizontal_speed * facing.cos()).trunc() as i32,
        (horizontal_speed * facing.sin()).trunc() as i32,
        (speed_f * angle.sin()).trunc() as i32,
    ))
}

/// YR `BulletClass_GetAnimFrame` @ 0x00468000.
pub fn projectile_shp_frame(projectile: &Projectile) -> u8 {
    if projectile.visual.anim_low != 0 || projectile.visual.anim_high != 0 {
        return projectile.visual.runtime_frame;
    }
    const FACING_FRAMES: [u8; 32] = [
        28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5,
        4, 3, 2, 1, 0, 31, 30, 29,
    ];
    let angle = f64::from(-projectile.velocity.y).atan2(f64::from(projectile.velocity.x));
    let word =
        (((angle - std::f64::consts::FRAC_PI_2) * -10430.06004058427).trunc() as i32) & 0xffff;
    let bucket = ((((word as u32 >> 10) + 1) >> 1) & 31) as usize;
    FACING_FRAMES[bucket]
}

// YR BulletClass::AI linkage: this is the bounded ordinary-flight rung; exact trajectory kernels remain separate.
fn step_toward(from: ProjectileCoord, target: ProjectileCoord, speed: i32) -> ProjectileCoord {
    if speed <= 0 || from == target {
        return from;
    }
    let dx = target.x - from.x;
    let dy = target.y - from.y;
    let dz = target.z - from.z;
    let max_delta = dx.abs().max(dy.abs()).max(dz.abs());
    if max_delta <= speed {
        return target;
    }
    ProjectileCoord::new(
        from.x + ((i64::from(dx) * i64::from(speed)) / i64::from(max_delta)) as i32,
        from.y + ((i64::from(dy) * i64::from(speed)) / i64::from(max_delta)) as i32,
        from.z + ((i64::from(dz) * i64::from(speed)) / i64::from(max_delta)) as i32,
    )
}

// YR BulletClass::Detonate linkage: only this handoff permits combat damage/effects.
fn detonation(
    projectile: &Projectile,
    impact: ProjectileCoord,
    reason: ProjectileDetonationReason,
) -> ProjectileDetonation {
    ProjectileDetonation {
        projectile_id: projectile.id,
        source_id: projectile.source_id,
        target: projectile.target,
        impact,
        payload: projectile.payload,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn homing_impact_admission_matches_executed_retail_vectors() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../tools/projectile_oracle/homing_impact_vectors.json"
        ))
        .unwrap();
        for row in vectors["admissions"].as_array().unwrap() {
            let (admit, snap) = homing_impact_admission(
                row["distance"].as_i64().unwrap() as i32,
                ProjectileVelocity::new(
                    row["velocity"][0].as_i64().unwrap() as i32,
                    row["velocity"][1].as_i64().unwrap() as i32,
                    row["velocity"][2].as_i64().unwrap() as i32,
                ),
                row["height"].as_i64().unwrap() as i32,
                row["airburst"].as_bool().unwrap(),
                !row["empty_target"].as_bool().unwrap(),
            );
            assert_eq!(admit, row["impact"].as_bool().unwrap(), "{row}");
            let candidate = if snap {
                [640, 128, 624]
            } else {
                [504, 128, 207]
            };
            assert_eq!(serde_json::json!(candidate), row["candidate"], "{row}");
        }
    }

    #[test]
    fn homing_impact_handoff_matches_executed_retail_vectors() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../tools/projectile_oracle/homing_impact_vectors.json"
        ))
        .unwrap();
        for row in vectors["handoffs"].as_array().unwrap() {
            let height = row["height"].as_i64().unwrap() as i32;
            let mode = row["fuse_mode"].as_i64().unwrap();
            let mut shot = guided_spawn(4, 0);
            shot.origin = ProjectileCoord::new(500, 128, 208 + height);
            shot.speed_leptons_per_frame = 4;
            shot.initial_target_position = ProjectileCoord::new(640, 128, 208);
            shot.target = if row["target_present"].as_bool().unwrap() {
                ProjectileTarget::Entity(42)
            } else {
                ProjectileTarget::None
            };
            shot.ranged_fuse = true;
            let guidance = shot.guidance.as_mut().unwrap();
            guidance.rot = 0; // Keep the externally supplied candidate fixed.
            guidance.airburst = row["airburst"].as_bool().unwrap();
            guidance.inaccurate = row["inaccurate"].as_bool().unwrap();
            guidance.fuse_reference = ProjectileCoord::new(
                match mode {
                    1 => 504,
                    2 => 604,
                    _ => 4000,
                },
                128,
                208 + height,
            );
            let dummy = SharedCellDummy::fresh();
            dummy.set_level_slope(2, 0);
            let mut store = ProjectileStore::new();
            let id = store.spawn(1, shot);
            store.projectiles.get_mut(&id).unwrap().last_distance_half = 0;
            let targets = BTreeMap::from([(42, ProjectileCoord::new(640, 128, 208))]);
            let result = store.advance(0, &targets, None, &dummy, |_, candidate| {
                Some(ProjectileCollisionResponse::TargetZClamp(candidate))
            });
            let impact = result.detonations[0].impact;
            assert_eq!(
                serde_json::json!([impact.x, impact.y, impact.z]),
                row["impact"],
                "{row}"
            );
        }
    }

    #[test]
    fn homing_source_fuse_mode_matches_executed_retail_vectors() {
        let vectors: serde_json::Value = serde_json::from_str(include_str!(
            "../../tools/projectile_oracle/homing_impact_vectors.json"
        ))
        .unwrap();
        for row in vectors["source_modes"].as_array().unwrap() {
            let mode = projectile_fuse_mode(
                row["detector_mode"].as_i64().unwrap() as i32,
                row["source_present"].as_bool().unwrap()
                    && row["source_jumpjet"].as_bool().unwrap(),
            );
            assert_eq!(
                i64::from(mode),
                row["admitted_mode"].as_i64().unwrap(),
                "{row}"
            );
        }
    }

    fn spawn(target: ProjectileTarget) -> ProjectileSpawn {
        ProjectileSpawn {
            source_id: 7,
            origin: ProjectileCoord::new(0, 0, 0),
            target,
            initial_target_position: match target {
                ProjectileTarget::Entity(_) => ProjectileCoord::new(128, 0, 0),
                ProjectileTarget::Cell { rx, ry } => cell_target_coord(None, rx, ry),
                ProjectileTarget::None => ProjectileCoord::new(0, 0, 0),
                ProjectileTarget::DummyCell => ProjectileCoord::new(0, 0, 0),
            },
            payload: ProjectilePayload {
                base_damage: 40,
                warhead: InternedId::from_index(3),
                weapon: InternedId::from_index(4),
                owner: InternedId::from_index(5),
            },
            speed_leptons_per_frame: 64,
            velocity: ProjectileVelocity::new(64, 0, 0),
            trajectory: ProjectileTrajectory::Straight,
            guidance: None,
            visual: ProjectileVisualState::new(0, 0, 0),
            arm_frames: 0,
            fuse_frames: None,
            ranged_fuse: false,
            tracks_target: true,
            target_expiry: TargetExpiryPolicy::DetonateAtLastKnown,
            collision: ProjectileCollisionPolicy::NONE,
        }
    }

    #[test]
    fn guided_rot_turn_word_matches_closed_vectors() {
        assert_eq!(projectile_rot_turn_word(4.0, 255, false), 0x0600);
        assert_eq!(projectile_rot_turn_word(4.0, 256, false), 0x0400);
        assert_eq!(projectile_rot_turn_word(4.0, 10, true), 0);
    }

    #[test]
    fn guided_rot_altitude_policy_matches_closed_very_high_vectors() {
        assert_eq!(
            projectile_rot_altitude_decision(false, false, false, false, 769, 2048, 104, -53, 1024),
            ProjectileRotAltitudeDecision {
                control_terrain_clearance: true,
                clearance_levels: 5,
                z_delta: 18,
                desired_pitch: Some(0x2000),
            }
        );
        assert_eq!(
            projectile_rot_altitude_decision(false, false, true, false, 1536, 0, 104, 100, 1024),
            ProjectileRotAltitudeDecision {
                control_terrain_clearance: false,
                clearance_levels: 10,
                z_delta: 0,
                desired_pitch: None,
            }
        );
        assert_eq!(
            projectile_rot_altitude_decision(false, true, false, false, 100, 0, 104, 53, 1024),
            ProjectileRotAltitudeDecision {
                control_terrain_clearance: true,
                clearance_levels: 10,
                z_delta: -18,
                desired_pitch: Some(0x4800),
            }
        );
        assert_eq!(
            projectile_rot_altitude_decision(false, false, false, true, 1000, 2048, 104, 100, 1024),
            ProjectileRotAltitudeDecision {
                control_terrain_clearance: false,
                clearance_levels: 5,
                z_delta: 0,
                desired_pitch: None,
            }
        );
        assert_eq!(
            projectile_rot_altitude_decision(false, false, false, false, 1000, 0, 104, 20, 1024),
            ProjectileRotAltitudeDecision {
                control_terrain_clearance: true,
                clearance_levels: 0,
                z_delta: 0,
                desired_pitch: Some(0x4000),
            }
        );
    }

    #[test]
    fn guided_projectile_turns_with_persisted_rot_state() {
        let mut store = ProjectileStore::new();
        let mut guided = spawn(ProjectileTarget::Cell { rx: 0, ry: 4 });
        guided.origin.z = 1; // Steering fixture: above the native ground-impact plane.
        guided.guidance = Some(ProjectileGuidance {
            rot: 4,
            missile_rot_var: SimFixed::from_num(0),
            course_lock_duration: 0,
            sidewinder_phase: 0,
            airburst: false,
            inaccurate: false,
            very_high: true,
            level: false,
            heading_bam: 0,
            max_speed: 0,
            acceleration: 3,
            fuse_reference: ProjectileCoord::new(0, 0, 0),
            closing_frames: 0,
            closing_accumulator_bits: 0,
            frames_elapsed: 0,
        });
        let id = store.spawn(1, guided);

        store.advance(
            0,
            &BTreeMap::new(),
            None,
            &SharedCellDummy::fresh(),
            |_, _| None,
        );

        let guided = store
            .get(id)
            .expect("guided projectile survives first turn");
        assert!(guided.velocity.y > 0, "ROT turns toward the +Y target");
        assert_eq!(guided.guidance.unwrap().frames_elapsed, 1);
    }

    #[test]
    fn gsi_04_01_dummy_target_reads_live_coord_level_and_slope() {
        let dummy = SharedCellDummy::fresh();
        dummy.set_level_slope(-1, 0);
        dummy.stamp_coord(0, 0);
        let flat = dummy_cell_target_coord(&dummy);
        assert_eq!(
            flat,
            ProjectileCoord::new(128, 128, -103),
            "CellClass::GetGroundHeight uses the verified 104-lepton domain"
        );

        dummy.set_level_slope(-1, 1);
        dummy.stamp_coord(4, 5);
        let target = dummy_cell_target_coord(&dummy);
        assert_eq!((target.x, target.y), (4 * 256 + 128, 5 * 256 + 128));
        assert_eq!(
            target.z,
            crate::util::lepton::ground_height_leptons(0xff, 1, target.x, target.y).unwrap()
        );
        assert_ne!(
            target.z, flat.z,
            "the live slope byte participates in dummy floor resolution"
        );

        dummy.stamp_coord(-2, 7);
        let moved = dummy_cell_target_coord(&dummy);
        assert_eq!((moved.x, moved.y), (-2 * 256 + 128, 7 * 256 + 128));
        assert_eq!(
            moved.z,
            crate::util::lepton::ground_height_leptons(0xff, 1, moved.x, moved.y).unwrap(),
            "coord stamps preserve and reuse the level/slope bytes"
        );
    }

    #[test]
    fn gsi_04_01_dummy_target_adds_native_high_bridge_height() {
        let dummy = SharedCellDummy::fresh();
        dummy.set_level_slope(2, 0);
        dummy.stamp_coord(4, 5);
        let ground = dummy_cell_target_coord(&dummy);

        dummy.apply_bridge_flag_slot(crate::map::bridge_facts::BridgeStampSlot::Anchor, true);
        let bridge = dummy_cell_target_coord(&dummy);

        assert_eq!(bridge.x, ground.x);
        assert_eq!(bridge.y, ground.y);
        assert_eq!(
            bridge.z - ground.z,
            crate::util::lepton::BRIDGE_HEIGHT_DELTA_LEPTONS as i32
        );
    }

    #[test]
    fn advance_preserves_stable_creation_order_and_delays_new_projectiles() {
        let mut store = ProjectileStore::new();
        let mut first_spawn = spawn(ProjectileTarget::Cell { rx: 0, ry: 0 });
        first_spawn.speed_leptons_per_frame = 256;
        let first = store.spawn(1, first_spawn);
        let second = store.spawn(2, spawn(ProjectileTarget::Cell { rx: 1, ry: 0 }));

        let result = store.advance(
            0,
            &BTreeMap::new(),
            None,
            &SharedCellDummy::fresh(),
            |_, _| None,
        );

        assert_eq!(
            result
                .detonations
                .iter()
                .map(|detonation| detonation.projectile_id)
                .collect::<Vec<_>>(),
            vec![first]
        );
        assert!(store.get(first).is_none());
        assert_ne!(
            store.get(second).unwrap().position,
            ProjectileCoord::new(0, 0, 0)
        );
    }

    #[test]
    fn homing_projectile_uses_current_target_position() {
        let mut store = ProjectileStore::new();
        let id = store.spawn(1, spawn(ProjectileTarget::Entity(42)));
        let targets = BTreeMap::from([(42, ProjectileCoord::new(128, 128, 0))]);

        store.advance(0, &targets, None, &SharedCellDummy::fresh(), |_, _| None);

        assert_eq!(
            store.get(id).unwrap().position,
            ProjectileCoord::new(64, 64, 0)
        );
    }

    #[test]
    fn target_expiry_detonates_at_last_known_position() {
        let mut store = ProjectileStore::new();
        let id = store.spawn(1, spawn(ProjectileTarget::Entity(42)));
        let targets = BTreeMap::from([(42, ProjectileCoord::new(128, 0, 0))]);
        store.advance(0, &targets, None, &SharedCellDummy::fresh(), |_, _| None);

        let result = store.advance(
            0,
            &BTreeMap::new(),
            None,
            &SharedCellDummy::fresh(),
            |_, _| None,
        );

        assert_eq!(result.detonations.len(), 1);
        assert_eq!(result.detonations[0].projectile_id, id);
        assert_eq!(
            result.detonations[0].impact,
            ProjectileCoord::new(128, 0, 0)
        );
        assert_eq!(
            result.detonations[0].reason,
            ProjectileDetonationReason::TargetExpired
        );
    }

    #[test]
    fn fuse_and_collision_are_deferred_detonations() {
        let mut store = ProjectileStore::new();
        let mut fused = spawn(ProjectileTarget::Cell { rx: 1, ry: 0 });
        fused.fuse_frames = Some(0);
        let fuse_id = store.spawn(1, fused);
        let collision_id = store.spawn(2, spawn(ProjectileTarget::Cell { rx: 1, ry: 0 }));

        let result = store.advance(
            0,
            &BTreeMap::new(),
            None,
            &SharedCellDummy::fresh(),
            |projectile, coord| {
                (projectile.id == collision_id)
                    .then_some(ProjectileCollisionResponse::TargetZClamp(coord))
            },
        );

        assert_eq!(result.detonations.len(), 2);
        assert_eq!(result.detonations[0].projectile_id, fuse_id);
        assert_eq!(
            result.detonations[0].reason,
            ProjectileDetonationReason::Fuse
        );
        assert_eq!(result.detonations[1].projectile_id, collision_id);
        assert_eq!(
            result.detonations[1].reason,
            ProjectileDetonationReason::Collision
        );
    }

    #[test]
    fn store_round_trips_through_snapshot_serialization() {
        let mut store = ProjectileStore::new();
        store.spawn(1, spawn(ProjectileTarget::Cell { rx: 1, ry: 0 }));

        let bytes = bincode::serialize(&store).unwrap();
        let restored: ProjectileStore = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored, store);
    }

    #[test]
    fn simulation_hash_and_save_preserve_pending_projectile() {
        let mut empty = crate::sim::world::Simulation::new();
        let mut sim = crate::sim::world::Simulation::new();
        let stable_id = sim.allocate_stable_id();
        sim.admit_projectile(stable_id, spawn(ProjectileTarget::Cell { rx: 1, ry: 0 }));
        // Native in-scenario load restarts Scenario RNG from Seed0. Normalize
        // both controls so this fixture isolates projectile persistence/hash.
        empty.scenario_rng = crate::sim::rng::SimRng::new(0);
        sim.scenario_rng = crate::sim::rng::SimRng::new(0);
        let expected_hash = sim.state_hash();

        assert_ne!(empty.state_hash(), expected_hash);
        let bytes = crate::sim::snapshot::GameSnapshot::save(&sim, 0, 0, "projectile", 0);
        let restored = crate::sim::snapshot::GameSnapshot::load(&bytes)
            .expect("projectile snapshot should load")
            .sim;

        assert_eq!(restored.state_hash(), expected_hash);
        assert_eq!(restored.projectiles.len(), 1);
    }

    #[test]
    fn ballistic_launch_matches_closed_re_vectors() {
        let origin = ProjectileCoord::new(0, 0, 0);
        let velocity =
            ballistic_launch_velocity(origin, ProjectileCoord::new(500, 0, 0), 100, 6, false)
                .unwrap();
        assert_eq!(velocity, ProjectileVelocity::new(15, 0, 98));
        assert!(
            ballistic_launch_velocity(origin, ProjectileCoord::new(5000, 0, 0), 10, 6, false)
                .is_none()
        );
    }

    #[test]
    fn shp_facing_and_animation_match_yr_vectors() {
        let mut store = ProjectileStore::new();
        let id = store.spawn(1, spawn(ProjectileTarget::Cell { rx: 0, ry: 0 }));
        assert_eq!(projectile_shp_frame(store.get(id).unwrap()), 20);

        let projectile = store.projectiles.get_mut(&id).unwrap();
        projectile.visual = ProjectileVisualState {
            anim_low: 2,
            anim_high: 4,
            anim_rate: 3,
            runtime_frame: 4,
            runtime_countdown: 1,
        };
        projectile.visual.advance();
        assert_eq!(projectile.visual.runtime_frame, 2);
        assert_eq!(projectile.visual.runtime_countdown, 3);
    }

    #[test]
    fn ranged_fuse_thresholds_match_executable_spec() {
        assert_eq!(ranged_fuse_distance_step(63, 999), (1, 999));
        assert_eq!(ranged_fuse_distance_step(64, 999), (0, 32));
        assert_eq!(ranged_fuse_distance_step(80, 10), (2, 10));
        assert_eq!(ranged_fuse_distance_step(80, 50), (0, 40));
    }

    #[test]
    fn closed_collision_predicate_vectors_match_yr() {
        assert_eq!(
            projectile_bridge_crossing(99, 100, 100),
            ProjectileBridgeCrossing::Up
        );
        assert_eq!(
            projectile_bridge_crossing(100, 99, 100),
            ProjectileBridgeCrossing::Down
        );
        assert_eq!(
            projectile_bridge_crossing(100, 100, 100),
            ProjectileBridgeCrossing::None
        );
        assert_eq!(
            projectile_cell_obstacle(249, 100, None, true, false, false, false, false),
            ProjectileCellObstacle::Overlay
        );
        assert_eq!(
            projectile_cell_obstacle(250, 100, None, true, false, false, false, false),
            ProjectileCellObstacle::None
        );
        assert_eq!(
            projectile_cell_obstacle(100, 100, Some(7), true, true, false, false, false),
            ProjectileCellObstacle::None
        );
    }

    #[test]
    fn burst_shrapnel_and_special_priority_match_closed_vectors() {
        assert_eq!(
            projectile_burst_plan(true, 8),
            ProjectileBurstPlan {
                detonation_count: 1,
                random_radius_rolls: 0,
                random_coordinate_calls: 0
            }
        );
        assert_eq!(
            projectile_burst_plan(false, 3),
            ProjectileBurstPlan {
                detonation_count: 3,
                random_radius_rolls: 3,
                random_coordinate_calls: 3
            }
        );
        assert_eq!(projectile_shrapnel_count(-8, true, 3), 5);
        assert_eq!(projectile_shrapnel_count(-8, false, 99), 3);
        assert_eq!(
            projectile_special_detonation_action(
                SpecialDetonationFlags {
                    mind_control: true,
                    nuke_maker: true,
                    ..SpecialDetonationFlags::default()
                },
                SpecialDetonationTarget::default()
            ),
            SpecialDetonationAction::MindControl
        );
    }

    /// `BulletClass::DetonateAtCoord @ 0x0046978e` is the chain's only
    /// conditional arm: `0x00469796` (flag), `0x004697a4` (`Target != 0`) and
    /// `0x004697b2` (`Target->What_Am_I() == 1`) all fall through to the
    /// BombDisarm test at `0x004699c4`, so a `DirectRocker=yes` warhead that
    /// misses a vehicle still runs ordinary damage. Airstrike only *looks*
    /// conditional in the decompile — `0x0046970d` is its sole fall-through
    /// and fires on the flag alone.
    #[test]
    fn direct_rocker_is_the_only_conditional_arm() {
        let rocker = SpecialDetonationFlags {
            direct_rocker: true,
            ..SpecialDetonationFlags::default()
        };
        assert_eq!(
            projectile_special_detonation_action(rocker, SpecialDetonationTarget { is_unit: true }),
            SpecialDetonationAction::DirectRocker
        );
        assert_eq!(
            projectile_special_detonation_action(
                rocker,
                SpecialDetonationTarget { is_unit: false }
            ),
            SpecialDetonationAction::OrdinaryDamage
        );

        // A non-vehicle target falls through to the NEXT test, not to the
        // tail: BombDisarm still claims the impact.
        assert_eq!(
            projectile_special_detonation_action(
                SpecialDetonationFlags {
                    direct_rocker: true,
                    bomb_disarm: true,
                    ..SpecialDetonationFlags::default()
                },
                SpecialDetonationTarget { is_unit: false }
            ),
            SpecialDetonationAction::BombDisarm
        );

        // Airstrike is entered on the flag alone, whatever the target is.
        for is_unit in [false, true] {
            assert_eq!(
                projectile_special_detonation_action(
                    SpecialDetonationFlags {
                        airstrike: true,
                        ..SpecialDetonationFlags::default()
                    },
                    SpecialDetonationTarget { is_unit }
                ),
                SpecialDetonationAction::Airstrike
            );
        }
    }

    #[test]
    fn cluster_and_random_shrapnel_consume_native_draw_counts() {
        let mut cluster_rng = SimRng::new(0x46_8d80);
        let mut cluster_reference = cluster_rng.clone();
        let _ = cluster_reference.next_range_u32_inclusive(256, 512);
        let _ = cluster_reference.next_u32();
        let coordinate = projectile_next_cluster_coord(
            ProjectileCoord::new(20 * 256 + 128, 20 * 256 + 128, 77),
            &mut cluster_rng,
        );
        assert_eq!(
            cluster_rng.logical_state(),
            cluster_reference.logical_state()
        );
        assert_eq!(coordinate.z, 77);

        let mut shrapnel_rng = SimRng::new(0x46_a310);
        let mut shrapnel_reference = shrapnel_rng.clone();
        let expected_x = shrapnel_reference.next_range_u32_inclusive(0, 4) as i32 - 2;
        let expected_y = shrapnel_reference.next_range_u32_inclusive(0, 4) as i32 - 2;
        assert_eq!(
            projectile_random_shrapnel_cell(10, 20, &mut shrapnel_rng),
            (10 + expected_x, 20 + expected_y)
        );
        assert_eq!(
            shrapnel_rng.logical_state(),
            shrapnel_reference.logical_state()
        );
    }

    fn guided_spawn(max_speed: u16, acceleration: i32) -> ProjectileSpawn {
        let mut spawn = spawn(ProjectileTarget::Entity(42));
        spawn.origin.z = 1; // Flight/fuse fixtures must not start in ground contact.
        spawn.speed_leptons_per_frame = 1;
        spawn.velocity = ProjectileVelocity::new(1, 0, 0);
        spawn.guidance = Some(ProjectileGuidance {
            rot: 4,
            missile_rot_var: SimFixed::from_num(0),
            course_lock_duration: 0,
            sidewinder_phase: 0,
            airburst: false,
            inaccurate: false,
            very_high: false,
            level: false,
            heading_bam: 0,
            frames_elapsed: 0,
            max_speed,
            acceleration,
            fuse_reference: ProjectileCoord::new(0, 0, 0),
            closing_frames: 0,
            closing_accumulator_bits: 0,
        });
        spawn
    }

    /// `TechnoClass::FireAt 0x006FEA3A` launches at one lepton per frame and
    /// `BulletClass::AI 0x004669CD` adds `Acceleration=` per frame up to
    /// `Bullet+0x110`. A `MaxSpeed >= 40` bullet clears the course lock on its
    /// first AI frame (`CMP [EBP+0x110],0x28` / `JGE` at `0x00466936`), so it
    /// gets the full acceleration immediately.
    #[test]
    fn gsi_08_06_fast_missile_ramps_from_one_by_full_acceleration() {
        let mut store = ProjectileStore::new();
        let id = store.spawn(1, guided_spawn(100, 3));
        let targets = BTreeMap::from([(42, ProjectileCoord::new(10_000, 0, 0))]);

        store.advance(0, &targets, None, &SharedCellDummy::fresh(), |_, _| None);
        let after_one = store.get(id).expect("missile still flying");
        assert_eq!(after_one.speed_leptons_per_frame, 4);
        assert_eq!(after_one.position, ProjectileCoord::new(4, 0, 1));

        store.advance(1, &targets, None, &SharedCellDummy::fresh(), |_, _| None);
        let after_two = store.get(id).expect("missile still flying");
        assert_eq!(after_two.speed_leptons_per_frame, 7);
        assert_eq!(after_two.position, ProjectileCoord::new(11, 0, 1));
    }

    /// With `CourseLockDuration = 0` and `MaxSpeed < 40` the lock survives, and
    /// `0x0046699D` replaces `Acceleration=` with one lepton on even global
    /// frames and none on odd ones. The parity is the GLOBAL frame counter's,
    /// not the bullet's own age.
    #[test]
    fn gsi_08_06_course_locked_missile_ramps_at_half_rate_on_even_frames() {
        let mut store = ProjectileStore::new();
        let id = store.spawn(1, guided_spawn(25, 3));
        let targets = BTreeMap::from([(42, ProjectileCoord::new(10_000, 0, 0))]);

        for (frame, expected) in [(0u32, 2u16), (1, 2), (2, 3), (3, 3), (4, 4)] {
            store.advance(frame, &targets, None, &SharedCellDummy::fresh(), |_, _| {
                None
            });
            assert_eq!(
                store
                    .get(id)
                    .expect("missile still flying")
                    .speed_leptons_per_frame,
                expected,
                "frame {frame}"
            );
        }
    }

    /// `BulletClass::AI 0x004671E0`: the `Vertical` arm applies no gravity,
    /// preserves the launch direction, ramps the magnitude by `Acceleration=`
    /// with no clamp on the crossing frame, and ends when the new world z
    /// passes `DetonationAltitude` (`0x00467334`).
    #[test]
    fn gsi_08_08_vertical_projectile_climbs_and_ends_at_detonation_altitude() {
        let mut store = ProjectileStore::new();
        let mut vertical = spawn(ProjectileTarget::Cell { rx: 0, ry: 0 });
        vertical.speed_leptons_per_frame = 1;
        vertical.origin = ProjectileCoord::new(0, 0, 0);
        vertical.trajectory = ProjectileTrajectory::Vertical {
            detonation_altitude: 10,
            acceleration: 1,
            max_speed: 50,
            heading_bam: 0,
            pitch_bam: 0x4000,
        };
        let id = store.spawn(1, vertical);

        let mut heights = Vec::new();
        for frame in 0..8u32 {
            let result = store.advance(
                frame,
                &BTreeMap::new(),
                None,
                &SharedCellDummy::fresh(),
                |_, _| None,
            );
            if let Some(detonation) = result.detonations.first() {
                assert_eq!(detonation.projectile_id, id);
                assert!(
                    detonation.impact.z > 10,
                    "the arm ends only once z passes DetonationAltitude"
                );
                assert_eq!(detonation.impact.x, 0, "no horizontal drift straight up");
                assert_eq!(heights, vec![2, 5, 9], "1 + 1, then +3, then +4");
                return;
            }
            heights.push(store.get(id).expect("still climbing").position.z);
        }
        panic!("the vertical arm never reached DetonationAltitude");
    }

    /// `BulletClass::AI 0x00467C70` detonates on ANY impact flag with no `Arm`
    /// test at all — `Arm=` is written once into the ProximityDetector at
    /// `BulletClass::Fire 0x00468A5D` and gates only that fuse. An unarmed
    /// shot that hits a wall must explode, not vanish.
    #[test]
    fn gsi_08_07_unarmed_collision_detonates_instead_of_vanishing() {
        let mut store = ProjectileStore::new();
        let mut unarmed = spawn(ProjectileTarget::Cell { rx: 4, ry: 0 });
        unarmed.arm_frames = 5;
        let id = store.spawn(1, unarmed);

        let result = store.advance(
            0,
            &BTreeMap::new(),
            None,
            &SharedCellDummy::fresh(),
            |_, coord| Some(ProjectileCollisionResponse::TargetZClamp(coord)),
        );

        assert!(result.expired.is_empty(), "an unarmed hit is not a vanish");
        assert_eq!(result.detonations.len(), 1);
        assert_eq!(result.detonations[0].projectile_id, id);
        assert_eq!(
            result.detonations[0].reason,
            ProjectileDetonationReason::Collision
        );
    }

    /// `ProximityDetector::Check @ 0x004E11F0` measures against the coordinate
    /// `ProximityDetector::Setup @ 0x004E1130` froze at launch, never against
    /// the live target.
    #[test]
    fn gsi_08_07_proximity_fuse_measures_the_frozen_launch_reference() {
        let targets = BTreeMap::from([(42, ProjectileCoord::new(10_000, 0, 0))]);

        let mut frozen_near = ProjectileStore::new();
        let mut spawn_near = guided_spawn(1, 3);
        spawn_near.ranged_fuse = true;
        if let Some(guidance) = spawn_near.guidance.as_mut() {
            guidance.fuse_reference = ProjectileCoord::new(1, 0, 0);
        }
        frozen_near.spawn(1, spawn_near);
        let near = frozen_near.advance(0, &targets, None, &SharedCellDummy::fresh(), |_, _| None);
        assert_eq!(near.detonations.len(), 1, "the frozen reference is reached");
        assert_eq!(near.detonations[0].reason, ProjectileDetonationReason::Fuse);

        let mut frozen_far = ProjectileStore::new();
        let mut spawn_far = guided_spawn(1, 3);
        spawn_far.ranged_fuse = true;
        if let Some(guidance) = spawn_far.guidance.as_mut() {
            guidance.fuse_reference = ProjectileCoord::new(10_000, 0, 0);
        }
        frozen_far.spawn(1, spawn_far);
        let far = frozen_far.advance(0, &targets, None, &SharedCellDummy::fresh(), |_, _| None);
        assert!(
            far.detonations.is_empty(),
            "a distant frozen reference keeps the shot flying even though the \
             bullet is one lepton from where it started"
        );
    }

    /// Both `TechnoClass::FireAt` scatter arms consume exactly two Scenario
    /// draws — magnitude then angle — and both OFFSET the target delta,
    /// leaving z untouched.
    #[test]
    fn gsi_08_07_launch_scatter_consumes_two_draws_and_offsets_the_delta() {
        let mut rng = SimRng::new(0x6f_e7fe);
        let mut reference = rng.clone();
        let _ = reference.next_range_u32_inclusive(128, 256);
        let _ = reference.next_range_u32_inclusive(0, 0x7fff_fffe);

        let scattered = projectile_launch_scatter((1024, 0, 77), 256, 1280, false, &mut rng);
        assert_eq!(rng.logical_state(), reference.logical_state());
        assert_eq!(scattered.2, 77, "z is carried through unchanged");
        let offset_x = scattered.0 - 1024;
        let offset_y = scattered.1;
        let magnitude = ((offset_x * offset_x + offset_y * offset_y) as f64).sqrt();
        assert!(
            (128.0..=257.0).contains(&magnitude),
            "the plain arm draws its magnitude in [BallisticScatter/2, BallisticScatter], got {magnitude}"
        );

        // The flak arm scales by distance over weapon range, so a target at
        // exactly the weapon's range can be displaced by the full scatter.
        let mut flak_rng = SimRng::new(0x6f_e6ad);
        let mut flak_reference = flak_rng.clone();
        let _ = flak_reference.next_range_u32_inclusive(0, 256);
        let _ = flak_reference.next_range_u32_inclusive(0, 0x7fff_fffe);
        let flak = projectile_launch_scatter((1280, 0, 0), 256, 1280, true, &mut flak_rng);
        assert_eq!(flak_rng.logical_state(), flak_reference.logical_state());
        let flak_magnitude = (((flak.0 - 1280) * (flak.0 - 1280) + flak.1 * flak.1) as f64).sqrt();
        assert!(
            flak_magnitude <= 257.0,
            "at exactly one weapon range the flak arm cannot exceed BallisticScatter, got {flak_magnitude}"
        );
    }

    #[test]
    fn flat_slope_uses_native_axis_reflection_order() {
        assert_eq!(
            projectile_slope_reflect(ProjectileVelocity::new(10, 20, 30), 0),
            Some(ProjectileVelocity::new(10, -20, 30))
        );
    }
}
