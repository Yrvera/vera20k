//! Authoritative in-flight projectile state.
//!
//! This module deliberately owns only delayed flight, target expiry, collision,
//! and detonation admission. Combat remains the authority for turning a returned
//! [`ProjectileDetonation`] into damage, warhead effects, and terrain changes.
//! Keeping that handoff narrow lets the world phase place this system at the
//! verified native frame rung without duplicating combat arithmetic here.

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
    Ballistic { gravity: i32 },
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
    pub course_lock_frames: u16,
    pub sidewinder_phase: u8,
    pub airburst: bool,
    pub very_high: bool,
    pub level: bool,
    pub pitch_bam: u16,
    pub frames_elapsed: u32,
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

/// Terrain checks admitted for one ordinary `BulletClass` flight.
///
/// The cases intentionally stay narrow: special trajectory kernels retain the
/// typed immediate path until their native coordinate contracts are ported.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialDetonationAction {
    MindControl,
    IvanBomb,
    ElectricAssault,
    Parasite,
    Temporal,
    Locomotor,
    Airstrike,
    Raw335,
    BombDisarm,
    MakesDisguise,
    NukeMaker,
    OrdinaryDamage,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpecialDetonationFlags {
    pub mind_control: bool,
    pub ivan_bomb: bool,
    pub electric_assault: bool,
    pub parasite: bool,
    pub temporal: bool,
    pub is_locomotor: bool,
    pub airstrike: bool,
    pub raw_335: bool,
    pub bomb_disarm: bool,
    pub makes_disguise: bool,
    pub nuke_maker: bool,
}

pub fn projectile_special_detonation_action(
    flags: SpecialDetonationFlags,
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
        SpecialDetonationAction::Airstrike
    } else if flags.raw_335 {
        SpecialDetonationAction::Raw335
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
    /// Native weapon speed expressed by the caller in leptons per logical frame.
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

    /// Admit one ordinary projectile. Vertical, airburst, cluster, and other
    /// special trajectories remain outside this bounded foundation.
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
                last_distance_half: i32::MAX,
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
        target_positions: &BTreeMap<u64, ProjectileCoord>,
        terrain: Option<&ResolvedTerrainGrid>,
        shared_cell_dummy: &SharedCellDummy,
        collides_at: impl FnMut(&Projectile, ProjectileCoord) -> Option<ProjectileCollisionResponse>,
    ) -> ProjectileAdvanceResult {
        let ids: Vec<u64> = self.projectiles.keys().copied().collect();
        self.advance_selected(
            &ids,
            |id| target_positions.get(&id).copied(),
            terrain,
            shared_cell_dummy,
            collides_at,
            true,
        )
    }

    pub(crate) fn advance_one(
        &mut self,
        id: u64,
        target_position: impl FnMut(u64) -> Option<ProjectileCoord>,
        terrain: Option<&ResolvedTerrainGrid>,
        shared_cell_dummy: &SharedCellDummy,
        collides_at: impl FnMut(&Projectile, ProjectileCoord) -> Option<ProjectileCollisionResponse>,
    ) -> Option<ProjectileAdvanceResult> {
        if !self.projectiles.contains_key(&id) {
            return None;
        }
        Some(self.advance_selected(
            &[id],
            target_position,
            terrain,
            shared_cell_dummy,
            collides_at,
            false,
        ))
    }

    fn advance_selected(
        &mut self,
        ids: &[u64],
        mut target_position: impl FnMut(u64) -> Option<ProjectileCoord>,
        terrain: Option<&ResolvedTerrainGrid>,
        shared_cell_dummy: &SharedCellDummy,
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
                ProjectileTarget::Entity(target_id) => match target_position(target_id) {
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
                        TargetExpiryPolicy::DetonateAtLastKnown => {
                            if projectile.arm_frames_remaining == 0 {
                                result.detonations.push(detonation(
                                    projectile,
                                    projectile.last_target_position,
                                    ProjectileDetonationReason::TargetExpired,
                                ));
                            } else {
                                result.expired.push(id);
                            }
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

            let candidate = if let Some(guidance) = projectile.guidance.as_mut() {
                // `BulletClass::Update` @ 0x00466BD0 enters 0x005B20F0 only
                // from the ROT-positive branch; ordinary straight flight does
                // not share this steering decision.
                let target_distance = horizontal_distance(projectile.position, target_position);
                let phase = guidance
                    .frames_elapsed
                    .wrapping_add(u32::from(guidance.sidewinder_phase));
                let varied_rot = (sidewinder_cos(phase) * guidance.missile_rot_var
                    + guidance.missile_rot_var
                    + SimFixed::from_num(1))
                    * SimFixed::from_num(guidance.rot);
                let turn_word = projectile_rot_turn_word_fixed(
                    varied_rot,
                    target_distance,
                    guidance.course_lock_frames != 0,
                );
                let current_yaw = atan2_bam(
                    SimFixed::from_num(projectile.velocity.y),
                    SimFixed::from_num(projectile.velocity.x),
                );
                let desired_yaw = atan2_bam(
                    SimFixed::from_num(target_position.y - projectile.position.y),
                    SimFixed::from_num(target_position.x - projectile.position.x),
                );
                let yaw = step_toward_bam_inclusive(current_yaw, desired_yaw, turn_word);
                let horizontal_speed = i64::from(projectile.velocity.x)
                    .pow(2)
                    .saturating_add(i64::from(projectile.velocity.y).pow(2))
                    .isqrt() as i32;
                let horizontal_speed = if horizontal_speed == 0 {
                    i32::from(projectile.speed_leptons_per_frame)
                } else {
                    horizontal_speed
                };
                projectile.velocity.x =
                    (SimFixed::from_num(horizontal_speed) * cos_bam(yaw)).to_num::<i32>();
                projectile.velocity.y =
                    (SimFixed::from_num(horizontal_speed) * sin_bam(yaw)).to_num::<i32>();
                guidance.frames_elapsed = guidance.frames_elapsed.wrapping_add(1);
                if guidance.course_lock_frames > 0 {
                    guidance.course_lock_frames -= 1;
                }
                ProjectileCoord::new(
                    projectile.position.x.saturating_add(projectile.velocity.x),
                    projectile.position.y.saturating_add(projectile.velocity.y),
                    projectile.position.z.saturating_add(projectile.velocity.z),
                )
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
                }
            };
            if projectile.ranged_fuse && projectile.arm_frames_remaining == 0 {
                let dx = f64::from(candidate.x - target_position.x);
                let dy = f64::from(candidate.y - target_position.y);
                let dz = f64::from(candidate.z - target_position.z);
                let distance = dx.hypot(dy).hypot(dz).trunc() as i32;
                let (fuse_mode, next_distance) =
                    ranged_fuse_distance_step(distance, projectile.last_distance_half);
                projectile.last_distance_half = next_distance;
                if fuse_mode != 0 {
                    result.detonations.push(detonation(
                        projectile,
                        candidate,
                        ProjectileDetonationReason::Fuse,
                    ));
                    continue;
                }
            }
            if let Some(response) = collides_at(projectile, candidate) {
                let impact = match response {
                    ProjectileCollisionResponse::TargetZClamp(impact) => impact,
                    ProjectileCollisionResponse::SlopeMatrixReflect { impact, velocity } => {
                        projectile.velocity = velocity;
                        impact
                    }
                };
                if projectile.arm_frames_remaining == 0 {
                    result.detonations.push(detonation(
                        projectile,
                        impact,
                        ProjectileDetonationReason::Collision,
                    ));
                } else {
                    result.expired.push(id);
                }
                continue;
            }

            projectile.position = candidate;
            if projectile.arm_frames_remaining > 0 {
                projectile.arm_frames_remaining -= 1;
            }
            let reached_target = match projectile.trajectory {
                ProjectileTrajectory::Straight if projectile.guidance.is_none() => {
                    candidate == target_position
                }
                ProjectileTrajectory::Ballistic { .. } => {
                    candidate.z <= target_position.z
                        && squared_horizontal_distance(candidate, target_position)
                            <= i64::from(projectile.speed_leptons_per_frame).pow(2)
                }
                ProjectileTrajectory::Straight => false,
            };
            if reached_target && projectile.arm_frames_remaining == 0 {
                result.detonations.push(detonation(
                    projectile,
                    candidate,
                    ProjectileDetonationReason::ReachedTarget,
                ));
            }
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
        guided.guidance = Some(ProjectileGuidance {
            rot: 4,
            missile_rot_var: SimFixed::from_num(0),
            course_lock_frames: 0,
            sidewinder_phase: 0,
            airburst: false,
            very_high: true,
            level: false,
            pitch_bam: 0x4000,
            frames_elapsed: 0,
        });
        let id = store.spawn(1, guided);

        store.advance(&BTreeMap::new(), None, &SharedCellDummy::fresh(), |_, _| {
            None
        });

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

        let result = store.advance(&BTreeMap::new(), None, &SharedCellDummy::fresh(), |_, _| {
            None
        });

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

        store.advance(&targets, None, &SharedCellDummy::fresh(), |_, _| None);

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
        store.advance(&targets, None, &SharedCellDummy::fresh(), |_, _| None);

        let result = store.advance(&BTreeMap::new(), None, &SharedCellDummy::fresh(), |_, _| {
            None
        });

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
            projectile_special_detonation_action(SpecialDetonationFlags {
                mind_control: true,
                nuke_maker: true,
                ..SpecialDetonationFlags::default()
            }),
            SpecialDetonationAction::MindControl
        );
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

    #[test]
    fn flat_slope_uses_native_axis_reflection_order() {
        assert_eq!(
            projectile_slope_reflect(ProjectileVelocity::new(10, 20, 30), 0),
            Some(ProjectileVelocity::new(10, -20, 30))
        );
    }
}
