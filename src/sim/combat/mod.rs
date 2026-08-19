//! Combat system — attack targeting, weapon firing, and damage application.
//!
//! Handles the combat loop: units with an AttackTarget component fire their
//! primary weapon at the target each tick (respecting ROF cooldown). Damage
//! is computed from weapon damage * warhead verses[armor_index]. Entities
//! at 0 health are despawned.
//!
//! ## RA2 damage formula
//! `actual_damage = weapon.damage * warhead.verses[armor_index]`
//! where armor_index is looked up from the target's Armor string.
//!
//! ## Rate of fire
//! ROF in rules.ini is measured in game frames (at 15 fps in original RA2).
//! We convert to simulation ticks using integer math.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/components and rules/ (RuleSet).
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

pub(crate) mod cell_spread;
pub(crate) mod combat_aoe;
pub(crate) mod combat_fire_gate;
pub(crate) mod combat_targeting;
pub(crate) mod combat_weapon;
pub(crate) mod damage;
pub(crate) mod fire_decision;
pub(crate) mod in_range;
mod inviso_scatter;
pub mod smudge_dispatch;
pub(crate) mod threat_range;

#[cfg(test)]
#[path = "combat_tests.rs"]
mod combat_tests;

#[cfg(test)]
#[path = "combat_force_fire_cell_tests.rs"]
mod combat_force_fire_cell_tests;

#[cfg(test)]
#[path = "combat_pursuit_tests.rs"]
mod combat_pursuit_tests;

#[cfg(test)]
#[path = "combat_turret_facing_tests.rs"]
mod combat_turret_facing_tests;

#[cfg(test)]
#[path = "delayed_building_fire_tests.rs"]
mod delayed_building_fire_tests;

use std::collections::{BTreeMap, BTreeSet};

use crate::sim::miner::ResourceNode;

use self::combat_weapon::{
    WeaponSlot, select_deploy_fire_weapon, select_weapon_slot, select_weapon_with_override,
};
use crate::map::entities::EntityCategory;
use crate::map::houses::HouseAllianceMap;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::RuleSet;
use crate::rules::warhead_type::WarheadType;
use crate::sim::bridge_state::{BridgeDamageEvent, BridgeRuntimeState};
use crate::sim::entity_store::EntityStore;
use crate::sim::house_state::HouseState;
use crate::sim::infantry;
use crate::sim::intern::{InternedId, StringInterner};
use crate::sim::map::bridge_topology::BRIDGE_DECK_HEIGHT_LEPTONS;
use crate::sim::mission::authority::{
    override_mission_on_damage_response, queue_entity_mission_deferred,
};
use crate::sim::mission::concrete_effects::represented_assign_target;
use crate::sim::mission::{MissionId, MissionType};
use crate::sim::overlay_grid::{OverlayGrid, WallMutation};
use crate::sim::power_system::PowerState;
use crate::sim::projectile::{
    ProjectileCollisionPolicy, ProjectileCoord, ProjectileDetonation, ProjectileGuidance,
    ProjectilePayload, ProjectileSpawn, ProjectileTarget, ProjectileTrajectory, ProjectileVelocity,
    ProjectileVisualState, SpecialDetonationAction, SpecialDetonationFlags, TargetExpiryPolicy,
    ballistic_launch_velocity, projectile_next_cluster_coord, projectile_random_shrapnel_cell,
    projectile_shrapnel_count, projectile_special_detonation_action,
};
use crate::sim::rng::SimRng;
use crate::sim::terrain_object::{TerrainAreaReceiveResult, TerrainAreaState};
use crate::sim::vision::FogState;
use crate::sim::wave::WaveDamageEvent;
use crate::sim::world::{FireOriginSnapshot, SimFireEvent, SimSoundEvent};
use crate::util::fixed_math::{SIM_ZERO, SimFixed, sim_to_i32};
use crate::util::lepton::{LEPTONS_PER_LEVEL, ground_height_leptons};
use crate::util::native_x87::{NativeF32Bits, NativeF64Bits, X87Chop53};

use super::animation::SequenceKind;
use super::deploy::DeployPhase;
use super::game_entity::{GameEntity, PendingBuildingFire};
use super::occupancy::OccupancyGrid;
use super::production::foundation_dimensions;

/// RA2 runs at 15 logical frames per second. ROF values are in frames.
/// Radius in cells that RevealOnFire clears shroud around the fire location.
const REVEAL_ON_FIRE_RADIUS: u16 = 3;
/// Step size for selecting explosion anim from a warhead's AnimList: idx = damage / 25.
const ANIM_LIST_DAMAGE_STEP: u16 = 25;

/// Explicitly classified delivery decision at weapon fire.
///
/// Unsupported projectile behaviors intentionally remain on the established
/// immediate path until their own native trajectory contracts are ported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectileDelivery {
    Persistent {
        arm_frames: u16,
        tracks_target: bool,
        collision: ProjectileCollisionPolicy,
        ballistic: bool,
        guidance: Option<ProjectileGuidance>,
    },
    Immediate(ImmediateProjectileReason),
}

/// The bounded lifecycle never silently treats an unsupported bullet as a
/// straight ordinary shot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImmediateProjectileReason {
    NoProjectile,
    MissingProjectileType,
    Invisible,
    InstantSpeed,
    Vertical,
    ObstacleTrajectory,
    SpecialTrajectory,
}

/// RESIDUAL (GSI-08.06/08.07/08.08) — this classifier is also the flight-model
/// gate, and most of the native flight keys reach it only as reasons to opt
/// *out* of authoritative flight rather than as models. `Vertical=` (5 stock),
/// `Inaccurate=` (2), `Proximity=` (18), `SubjectToCliffs=` (30) and
/// `SubjectToElevation=` (31) each disqualify a shot here;
/// `ProjectileTrajectory` itself has only `Straight` and `Ballistic`, so
/// vertical launch, scatter, proximity detection and cliff collision have no
/// implementation to fall back on. `DetonationAltitude=` (4) is parsed with no
/// reader, `Acceleration=` (6) is carried but unused by the advance, and
/// `AirburstWeapon=` (1) has no consumer. The `airburst` bool it sits beside is
/// carried into `ProjectileGuidance` and hashed, but its only behavioural read
/// suppresses the cluster walk — it spawns no child, which is the gap.
///
/// Two nearby keys are NOT gaps: `Floater=` (2) is read on both ballistic
/// launch paths to halve gravity and is carried and hashed by the homing
/// state, and `ShrapnelWeapon=` (5) is fully wired through
/// `emit_projectile_shrapnel`, scenario-RNG cell selection included.
/// - Trigger: firing any weapon whose projectile carries one of those keys —
///   V3 and Dreadnought missiles, the nuke, Boomer torpedoes, prism scatter.
/// - Player effect: those shots fall back to the non-authoritative path, so
///   they resolve without the flight the player expects: no vertical climb, no
///   scatter, no proximity detonation, and no airburst children.
/// - Frequency: bounded by the types above rather than continuous, but V3s and
///   Dreadnoughts are standard Soviet play and the nuke fires in most long
///   matches.
/// - Downstream risk: each missing model is its own trajectory implementation
///   against `BulletClass::AI`; they share the projectile store but not the
///   math, so they want separate slices. Launch-side effects are recorded
///   separately: `spawn_manager.rs` already flags its launch-position drift, no
///   `MuzzleFlash=` art anim is spawned for non-garrison weapons, and `Ammo=`
///   is decremented only for aircraft.
fn classify_projectile_delivery(
    weapon: &crate::rules::weapon_type::WeaponType,
    rules: &RuleSet,
) -> ProjectileDelivery {
    let Some(projectile_id) = weapon.projectile.as_deref() else {
        return ProjectileDelivery::Immediate(ImmediateProjectileReason::NoProjectile);
    };
    let Some(projectile) = rules.projectile(projectile_id) else {
        return ProjectileDelivery::Immediate(ImmediateProjectileReason::MissingProjectileType);
    };
    if projectile.inviso {
        return ProjectileDelivery::Immediate(ImmediateProjectileReason::Invisible);
    }
    if weapon.speed <= 0 {
        return ProjectileDelivery::Immediate(ImmediateProjectileReason::InstantSpeed);
    }
    if projectile.vertical {
        return ProjectileDelivery::Immediate(ImmediateProjectileReason::Vertical);
    }
    // YR BulletClass::AI linkage: Level's current-cell water predicate and
    // wall entry are now owned by the world collision rung. Cliff/elevation
    // kernels still need their native coordinate contracts.
    let ballistic = projectile.arcing || weapon.lobber;
    if !ballistic && (projectile.subject_to_cliffs || projectile.subject_to_elevation) {
        return ProjectileDelivery::Immediate(ImmediateProjectileReason::ObstacleTrajectory);
    }
    if projectile.dropping
        || (projectile.very_high && projectile.rot <= 0)
        || projectile.proximity
        || projectile.flak_scatter
        || projectile.inaccurate
        || projectile.degenerates
        || projectile.bouncy
    {
        return ProjectileDelivery::Immediate(ImmediateProjectileReason::SpecialTrajectory);
    }
    ProjectileDelivery::Persistent {
        arm_frames: projectile.arm.max(0).min(u16::MAX as i32) as u16,
        tracks_target: projectile.rot > 0,
        collision: ProjectileCollisionPolicy {
            level_non_water: projectile.level,
            subject_to_walls: projectile.subject_to_walls,
            native_cell_collision: projectile.rot <= 0,
        },
        ballistic,
        guidance: (!ballistic && projectile.rot > 0).then_some(ProjectileGuidance {
            rot: projectile.rot,
            missile_rot_var: rules.general.missile_rot_var,
            course_lock_frames: projectile
                .course_lock_duration
                .clamp(0, i32::from(u16::MAX)) as u16,
            // The RE contract proves this is BulletClass-identity-derived but
            // not its formula. Keep the raw phase as an explicit live seam.
            sidewinder_phase: 0,
            airburst: projectile.airburst,
            very_high: projectile.very_high,
            level: projectile.level,
            pitch_bam: 0x4000,
            frames_elapsed: 0,
        }),
    }
}

/// Whether a fire event's visible projectile is represented by the serialized
/// world `ProjectileStore`, rather than the legacy app-local interpolation.
pub(crate) fn projectile_uses_authoritative_flight(
    weapon: &crate::rules::weapon_type::WeaponType,
    rules: &RuleSet,
) -> bool {
    matches!(
        classify_projectile_delivery(weapon, rules),
        ProjectileDelivery::Persistent { .. }
    )
}

#[cfg(test)]
mod projectile_delivery_tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;

    #[test]
    fn wall_projectile_uses_the_authoritative_collision_path() {
        let ini = IniFile::from_str(
            "[VehicleTypes]\n0=TEST\n\n[TEST]\nStrength=100\nArmor=heavy\nPrimary=GUN\n\n[GUN]\nDamage=20\nROF=10\nRange=5\nSpeed=30\nProjectile=SHELL\nWarhead=WH\n\n[SHELL]\nSubjectToWalls=yes\n\n[WH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("projectile rules");
        let weapon = rules.weapon("GUN").expect("weapon");

        assert_eq!(
            classify_projectile_delivery(weapon, &rules),
            ProjectileDelivery::Persistent {
                arm_frames: 0,
                tracks_target: false,
                collision: ProjectileCollisionPolicy {
                    level_non_water: false,
                    subject_to_walls: true,
                    native_cell_collision: true,
                },
                ballistic: false,
                guidance: None,
            }
        );
    }
}

/// A cell area to reveal due to a RevealOnFire weapon firing.
pub struct RevealEvent {
    pub owner: InternedId,
    pub rx: u16,
    pub ry: u16,
    pub radius: u16,
}

/// Armor type name → Verses index mapping.
/// Matches the order defined in warhead_type.rs: none(0), flak(1), plate(2),
/// light(3), medium(4), heavy(5), wood(6), steel(7), concrete(8),
/// special_1(9), special_2(10).
const ARMOR_NAMES: &[&str] = &[
    "none",
    "flak",
    "plate",
    "light",
    "medium",
    "heavy",
    "wood",
    "steel",
    "concrete",
    "special_1",
    "special_2",
];

/// Look up the Verses array index for an armor type name.
/// Returns 0 ("none") for unrecognized armor strings.
/// Used by combat_weapon.rs for weapon selection.
pub fn armor_index(armor: &str) -> usize {
    let lower: String = armor.to_ascii_lowercase();
    ARMOR_NAMES.iter().position(|&a| a == lower).unwrap_or(0)
}

/// Combat-only target category used for projectile AA/AG legality and weapon
/// selection.
///
/// `ConsideredAircraft=yes` infantry, such as Rocketeers/JumpJets, remain
/// infantry entities for movement, selection, crush, and animation, but weapon
/// selection must treat them as air targets.
pub(crate) fn combat_target_category(
    entity: &GameEntity,
    rules: &RuleSet,
    interner: &StringInterner,
) -> EntityCategory {
    if rules
        .object(interner.resolve(entity.type_ref))
        .is_some_and(|obj| obj.considered_aircraft)
    {
        EntityCategory::Aircraft
    } else {
        entity.category
    }
}

/// Return the active wall-overlay flags at a cell, if available.
fn wall_overlay_flags_at<'a>(
    overlay_grid: Option<&OverlayGrid>,
    overlay_registry: Option<&'a OverlayTypeRegistry>,
    rx: u16,
    ry: u16,
) -> Option<&'a crate::map::overlay_types::OverlayTypeFlags> {
    let (Some(grid), Some(registry)) = (overlay_grid, overlay_registry) else {
        return None;
    };
    grid.cell(rx, ry)
        .overlay_id
        .and_then(|id| registry.flags(id))
        .filter(|flags| flags.wall)
}

fn warhead_damages_wall(
    warhead: &WarheadType,
    wall_flags: &crate::map::overlay_types::OverlayTypeFlags,
) -> bool {
    warhead.wall || warhead.wall_absolute_destroyer || (warhead.wood && wall_flags.armor_is_wood)
}

fn infantry_prone_area_raw_damage(
    target: &GameEntity,
    warhead: &WarheadType,
    damage: i32,
    ignore_defenses: bool,
) -> i32 {
    if target.category != EntityCategory::Infantry
        || !infantry::is_prone_for_damage(target)
        || damage <= 0
        || ignore_defenses
    {
        return damage;
    }

    let Ok(multiplier) =
        X87Chop53::load_f64(NativeF64Bits::from_bits(warhead.prone_damage_f64.to_bits()))
    else {
        // FISTP's native indefinite result is negative and the wrapper's
        // immediate minimum-one clamp therefore converts it to one.
        return 1;
    };
    let product = X87Chop53::mul(X87Chop53::load_i32(damage), multiplier);
    let scaled = X87Chop53::ftol_i64(product)
        .ok()
        .and_then(|value| i32::try_from(value).ok())
        // A native out-of-range FISTP yields the signed indefinite value,
        // which the immediately following minimum-one clamp replaces with 1.
        .unwrap_or(i32::MIN);
    scaled.max(1)
}

/// What an `AttackTarget` is pointing at — an entity or a ground cell.
///
/// Force-fire on empty terrain (Ctrl + click cell) sets the `Cell` variant.
/// Auto-acquired and explicit attack-on-unit orders set `Entity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TargetKind {
    /// Entity-targeted attack (normal Attack / ForceAttack on a unit/building).
    Entity(u64),
    /// Ground-targeted attack (force-fire on a cell). Cell coord in map space.
    Cell(u16, u16),
}

/// Sentinel attacker id for sourceless damage (the radiation field). Stable
/// entity ids start at 1, so 0 is never a live attacker; retaliation treats
/// it as "attacker gone" and the last-attacker bookkeeping skips it.
pub(crate) const RAD_NO_ATTACKER: u64 = 0;

/// The two receiver booleans carried by one native concrete `ReceiveDamage`
/// call. Their semantic names are intentionally limited to the verified ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReceiverCallFlags {
    pub(crate) ignore_defenses: bool,
    pub(crate) arg6: bool,
}

/// One ordered damage call. Area and direct-receiver records retain the raw
/// signed damage, native lepton distance, and concrete receiver flags until
/// dispatch; legacy direct callers retain their already-resolved amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntityDamageEvent {
    pub(crate) target_id: u64,
    pub(crate) damage: i32,
    pub(crate) attacker_id: u64,
    /// ReceiveDamage's sourceHouse ABI argument captured when the detonation
    /// enters Apply_area_damage. This remains valid if the source object is
    /// uninitialized by an earlier ordered receiver record.
    pub(crate) source_house: Option<InternedId>,
    pub(crate) warhead_ref: InternedId,
    pub(crate) distance_leptons: Option<i32>,
    pub(crate) receiver_flags: Option<ReceiverCallFlags>,
    /// This record belongs to an Apply_area_damage transaction whose captured
    /// CellSpread is at most 0.5. The receiver commit uses this transient fact
    /// to reproduce the native near-center Iron Curtain isolation scan; it is
    /// deliberately false for direct-receiver and legacy precomputed calls.
    pub(crate) near_center_ic_isolation_eligible: bool,
}

impl EntityDamageEvent {
    pub(crate) fn area(
        target_id: u64,
        raw_damage: i32,
        distance_leptons: i32,
        attacker_id: u64,
        source_house: Option<InternedId>,
        warhead_ref: InternedId,
    ) -> Self {
        Self {
            target_id,
            damage: raw_damage,
            attacker_id,
            source_house,
            warhead_ref,
            distance_leptons: Some(distance_leptons),
            receiver_flags: Some(ReceiverCallFlags {
                ignore_defenses: false,
                arg6: false,
            }),
            near_center_ic_isolation_eligible: false,
        }
    }

    pub(crate) fn direct_receiver(
        target_id: u64,
        raw_damage: i32,
        distance_leptons: i32,
        attacker_id: u64,
        source_house: Option<InternedId>,
        warhead_ref: InternedId,
        receiver_flags: ReceiverCallFlags,
    ) -> Self {
        Self {
            target_id,
            damage: raw_damage,
            attacker_id,
            source_house,
            warhead_ref,
            distance_leptons: Some(distance_leptons),
            receiver_flags: Some(receiver_flags),
            near_center_ic_isolation_eligible: false,
        }
    }

    /// `WaveClass::DamageArea` calls the concrete occupant receiver directly,
    /// at distance zero, while both the wave and firer are still represented.
    pub(crate) fn from_wave(event: WaveDamageEvent, entities: &EntityStore) -> Self {
        Self::direct_receiver(
            event.target_id,
            event.payload.base_damage,
            0,
            event.payload.firer_id,
            entities
                .get(event.payload.firer_id)
                .map(|firer| firer.owner),
            event.payload.warhead,
            ReceiverCallFlags {
                ignore_defenses: false,
                arg6: false,
            },
        )
    }
}

/// Infantry shot waiting for its current fire animation to reach the discharge frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PendingInfantryFire {
    /// Fire sequence started when the shot was accepted.
    pub sequence: SequenceKind,
    /// Animation frame index that spawns the projectile/damage.
    pub fire_frame: u16,
}

/// Component: this entity is attacking a specific target.
///
/// Attached by `issue_attack_command()` (entity targets) or
/// `issue_attack_cell_command()` (cell targets). The combat system fires the
/// attacker's weapon at the resolved target each tick. Supports burst firing:
/// multiple rapid shots per attack cycle, with ROF cooldown only after
/// the full burst completes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AttackTarget {
    /// What this attacker is firing at: an entity or a ground cell (force-fire).
    pub target: TargetKind,
    /// Simulation ticks remaining before the next shot (ROF cooldown).
    pub cooldown_ticks: u16,
    /// Shots remaining in the current burst. When this reaches 0, ROF cooldown starts.
    pub burst_remaining: u8,
    /// Ticks between individual burst shots (short inter-shot delay).
    pub burst_delay_ticks: u8,
    /// Infantry-only delayed shot latch. `None` for vehicles/buildings/aircraft.
    #[serde(default)]
    pub pending_infantry_fire: Option<PendingInfantryFire>,
}

/// Delay in simulation ticks between individual shots within a burst.
/// 1 game frame (~66ms) — fast but visible.
const BURST_INTER_SHOT_DELAY: u8 = 1;

fn infantry_fire_sequence(
    obj: &ObjectType,
    weapon_slot: WeaponSlot,
    is_prone: bool,
    is_fully_deployed: bool,
) -> SequenceKind {
    if is_fully_deployed {
        return SequenceKind::DeployedFire;
    }
    match (weapon_slot, is_prone) {
        (WeaponSlot::Primary, true) => SequenceKind::FireProne,
        (WeaponSlot::Primary, false) => SequenceKind::Attack,
        (WeaponSlot::Secondary, true) if obj.secondary_prone_frame != obj.fire_up_frame => {
            SequenceKind::SecondaryProne
        }
        (WeaponSlot::Secondary, false) if obj.secondary_fire_frame != obj.fire_up_frame => {
            SequenceKind::SecondaryFire
        }
        _ => SequenceKind::Attack,
    }
}

fn infantry_fire_frame(
    obj: &ObjectType,
    weapon_slot: WeaponSlot,
    is_prone: bool,
    is_fully_deployed: bool,
) -> u16 {
    let frame = match (weapon_slot, is_prone || is_fully_deployed) {
        (WeaponSlot::Primary, false) => obj.fire_up_frame,
        (WeaponSlot::Primary, true) => obj.fire_prone_frame,
        (WeaponSlot::Secondary, false) => obj.secondary_fire_frame,
        (WeaponSlot::Secondary, true) => obj.secondary_prone_frame,
    };
    frame as u16
}

fn infantry_idle_sequence(is_prone: bool, is_fully_deployed: bool) -> SequenceKind {
    if is_fully_deployed {
        SequenceKind::Deployed
    } else if is_prone {
        SequenceKind::Prone
    } else {
        SequenceKind::Stand
    }
}

fn uses_deploy_fire_weapon(entity: &GameEntity) -> bool {
    matches!(
        entity.deploy_state,
        Some(DeployPhase::Deploying { .. } | DeployPhase::Deployed)
    )
}

impl AttackTarget {
    /// Entity-targeted attack: fire at a specific entity by stable ID.
    pub fn new(target_stable_id: u64) -> Self {
        Self {
            target: TargetKind::Entity(target_stable_id),
            cooldown_ticks: 0,
            burst_remaining: 0,
            burst_delay_ticks: 0,
            pending_infantry_fire: None,
        }
    }

    /// Ground-targeted attack: fire at a specific cell coord (force-fire on terrain).
    pub fn for_cell(rx: u16, ry: u16) -> Self {
        Self {
            target: TargetKind::Cell(rx, ry),
            cooldown_ticks: 0,
            burst_remaining: 0,
            burst_delay_ticks: 0,
            pending_infantry_fire: None,
        }
    }
}

/// Compute the effective target coordinates for an entity.
///
/// For structures, returns the **foundation center** instead of the raw
/// position (NW corner cell center):
///   X = Location.X + (foundationWidth  - 1) * 128
///   Y = Location.Y + (foundationHeight - 1) * 128
///
/// The vanilla game has bugs where some code paths use raw Location (NW corner)
/// instead of foundation center — e.g. Destroyers mis-targeting Naval Yards
/// from certain angles (Phobos bugfix at 0x70BCE6). We fix this from the start.
fn target_coords(
    entity: &GameEntity,
    rules: Option<&RuleSet>,
    interner: &StringInterner,
) -> (u16, u16, SimFixed, SimFixed) {
    let mut rx = entity.position.rx;
    let mut ry = entity.position.ry;
    let mut sub_x = entity.position.sub_x;
    let mut sub_y = entity.position.sub_y;

    if entity.category == EntityCategory::Structure {
        if let Some(obj) = rules.and_then(|r| r.object(interner.resolve(entity.type_ref))) {
            let (fw, fh) = foundation_dimensions(&obj.foundation);
            // Shift from NW corner cell center to foundation geometric center.
            // (fw-1)*128 leptons in X, (fh-1)*128 leptons in Y.
            // sub_x/sub_y may exceed 256 — lepton_distance_sq_raw handles
            // this correctly since it computes cell*256+sub as a flat value.
            let offset_x = (fw.saturating_sub(1) as i32) * 128;
            let offset_y = (fh.saturating_sub(1) as i32) * 128;
            let full_x: i32 = rx as i32 * 256 + sub_x.to_num::<i32>() + offset_x;
            let full_y: i32 = ry as i32 * 256 + sub_y.to_num::<i32>() + offset_y;
            rx = (full_x / 256) as u16;
            ry = (full_y / 256) as u16;
            sub_x = SimFixed::from_num(full_x % 256);
            sub_y = SimFixed::from_num(full_y % 256);
        }
    }

    (rx, ry, sub_x, sub_y)
}

/// Compute lepton-precise coordinates for a cell target (force-fire on terrain).
///
/// Cell-center convention: leptons = `cell_index * 256 + 128`. Returns the
/// shape `target_coords` returns for entities (rx, ry, sub_x, sub_y) so
/// callers can branch on `TargetKind` and feed the result into the same
/// projectile-spawn pipeline.
fn cell_center_coords(rx: u16, ry: u16) -> (u16, u16, SimFixed, SimFixed) {
    (rx, ry, SimFixed::from_num(128), SimFixed::from_num(128))
}

/// Resolve target coords from a `TargetKind`, looking up entity position when
/// needed and using cell-center for `Cell` targets.
///
/// Returns `None` if the target is `Entity(id)` and the entity no longer
/// exists (despawned). `Cell` targets always resolve.
///
/// Shared by the combat tick and the pursuit pre-combat stage so range
/// decisions stay consistent.
pub(crate) fn resolve_target_coords(
    target: &TargetKind,
    entities: &EntityStore,
    rules: Option<&RuleSet>,
    interner: &StringInterner,
) -> Option<(u16, u16, SimFixed, SimFixed)> {
    match *target {
        TargetKind::Entity(id) => entities.get(id).map(|t| target_coords(t, rules, interner)),
        TargetKind::Cell(rx, ry) => Some(cell_center_coords(rx, ry)),
    }
}

/// Whether the attacker's normally selected weapon can currently reach this
/// target through the authoritative 3D `InRange` path.
///
/// gamemd-derived: SpawnManager mode 0 in `SpawnManagerClass::AI` @
/// `0x006B7230` calls the Unit owner's `TechnoClass::CanFireAtTarget` vslot,
/// which dispatches through weapon selection @ `0x006F7780`, `CanFireAt` @
/// `0x006F77B0`, and ordinary `TechnoClass::InRange` @ `0x006F7220`.
pub(crate) fn can_fire_at_target(
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
    attacker_id: u64,
    target: &TargetKind,
    terrain: &ResolvedTerrainGrid,
) -> bool {
    let Some(attacker) = entities.get(attacker_id) else {
        return false;
    };
    let Some(attacker_obj) = rules.object(interner.resolve(attacker.type_ref)) else {
        return false;
    };
    let (target_category, target_armor) = match *target {
        TargetKind::Entity(target_id) => {
            let Some(target_entity) = entities.get(target_id) else {
                return false;
            };
            let armor = rules
                .object(interner.resolve(target_entity.type_ref))
                .map(|object| object.armor.as_str())
                .unwrap_or("none");
            (
                combat_target_category(target_entity, rules, interner),
                armor,
            )
        }
        TargetKind::Cell(_, _) => (EntityCategory::Structure, attacker_obj.armor.as_str()),
    };
    let Some(selected) = select_weapon_with_override(
        rules,
        attacker_obj,
        target_category,
        target_armor,
        attacker.veterancy,
        attacker.weapon_override,
    ) else {
        return false;
    };
    let Some(source_z) = in_range::effective_z_leptons(attacker, terrain) else {
        return false;
    };
    let source = (
        i64::from(attacker.position.rx) * 256 + attacker.position.sub_x.to_num::<i64>(),
        i64::from(attacker.position.ry) * 256 + attacker.position.sub_y.to_num::<i64>(),
        source_z,
    );
    in_range::compute_in_range(
        attacker,
        source,
        target,
        selected.weapon,
        rules,
        interner,
        entities,
        terrain,
    )
}

/// Resolve the effective weapon range for an attacker against a `TargetKind`.
///
/// Uses the same weapon-select inputs as the combat tick's Phase 2 weapon
/// selection so pursuit and combat agree on "in range" at the boundary.
///
/// For `Entity` targets: uses the target's actual category and armor.
/// For `Cell` targets: synthesizes `Structure` + attacker's own armor,
/// matching the cell-target synthesis in the combat tick.
///
/// Returns `None` if no weapon engages the target (Verses 0% or projectile
/// AA/AG mismatch). Pursuit treats `None` as "skip — combat tick will drop
/// the attack on its own weapon-select fail."
pub(crate) fn pursuit_weapon_range(
    entity: &GameEntity,
    target: &TargetKind,
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
) -> Option<SimFixed> {
    use self::combat_weapon::select_weapon_with_override;
    use crate::map::entities::EntityCategory;

    let attacker_obj = rules.object(interner.resolve(entity.type_ref))?;
    let (target_cat, target_armor) = match *target {
        TargetKind::Entity(id) => {
            let target_entity = entities.get(id)?;
            let armor = rules
                .object(interner.resolve(target_entity.type_ref))
                .map(|o| o.armor.clone())
                .unwrap_or_else(|| "none".to_string());
            (
                combat_target_category(target_entity, rules, interner),
                armor,
            )
        }
        TargetKind::Cell(_, _) => {
            // Synthetic — must match the combat tick's cell-target synthesis.
            // Using attacker's own armor here is the pre-existing convention.
            let armor = rules
                .object(interner.resolve(entity.type_ref))
                .map(|o| o.armor.clone())
                .unwrap_or_else(|| "none".to_string());
            (EntityCategory::Structure, armor)
        }
    };
    let selected = if uses_deploy_fire_weapon(entity) {
        select_deploy_fire_weapon(
            rules,
            attacker_obj,
            target_cat,
            &target_armor,
            entity.veterancy,
            entity.weapon_override,
        )
    } else {
        select_weapon_with_override(
            rules,
            attacker_obj,
            target_cat,
            &target_armor,
            entity.veterancy,
            entity.weapon_override,
        )
    };

    selected.map(|sel| sel.weapon.range)
}

/// Issue an attack command: make `attacker` fire at `target`.
///
/// Replaces any existing AttackTarget on the attacker. Also updates the
/// attacker's facing to point toward the target.
pub fn issue_attack_command(
    entities: &mut EntityStore,
    attacker_id: u64,
    target_id: u64,
    rules: Option<&RuleSet>,
    interner: &StringInterner,
) -> bool {
    // Read target position first (immutable borrow, lepton-precise).
    // Use foundation center for buildings (see target_coords doc comment).
    let target_pos = entities
        .get(target_id)
        .map(|t| target_coords(t, rules, interner));
    let (trx, try_, _tsx, _tsy) = match target_pos {
        Some(p) => p,
        None => return false,
    };

    // Read attacker position before mutable borrow (needed for body-facing delta).
    let attacker_pos = entities
        .get(attacker_id)
        .map(|a| (a.position.rx, a.position.ry, a.barrel_facing.is_some()));
    let (arx, ary, has_turret) = match attacker_pos {
        Some(p) => p,
        None => return false,
    };

    // Mutate attacker.
    let attacker = match entities.get_mut(attacker_id) {
        Some(a) => a,
        None => return false,
    };

    // Body-only: instantly face the target. For turreted units, the turret
    // rotates over multiple ticks — driven by tick_turret_rotation reading
    // attack_target — so we set NO facing here. This matches gamemd: command
    // handlers set the target; Facing_Update drives the rotation.
    if !has_turret {
        let dx: i32 = trx as i32 - arx as i32;
        let dy: i32 = try_ as i32 - ary as i32;
        attacker.facing = crate::sim::movement::facing_from_delta(dx, dy);
    }

    // Remove existing movement (stop moving to attack).
    attacker.movement_target = None;

    // Attach the attack target using stable ID (fire immediately).
    attacker.attack_target = Some(AttackTarget::new(target_id));
    // An ordered target was not picked up by the passive scanner, so it is not
    // subject to the scanner's stale-target drop or the off-mission clear.
    attacker.passively_acquired_target = false;

    true
}

/// Swing an existing attack onto a different entity WITHOUT restarting the
/// weapon.
///
/// The rearm countdown, the burst counter and the inter-shot delay all live on
/// [`AttackTarget`] here, so replacing the whole record — which is what building
/// a fresh `AttackTarget` does — zeroes them and hands the attacker a free shot
/// on the spot. The original keeps its rearm timer on the OBJECT and its target
/// assignment writes nothing but the target pointer and two adjacent fields, so
/// swinging onto a new victim never shortens the reload. Mutating in place is
/// how that contract is honoured here.
///
/// This is the one owner of that operation: combat's own auto-retarget and the
/// passive scanner's re-pick both go through it. Only the pending infantry shot
/// is dropped, because it was latched against the old victim.
///
/// RESIDUAL: this does NOT perform the infantry firing-sequence and animation
/// reset that the full target setter does. That was unreachable in practice
/// before the passive scanner existed; it is now reachable on every infantry
/// re-pick, roughly every 28 frames. Deterministic and visual only — the fire
/// decision does not read the sequence — so it is recorded rather than fixed
/// here.
///
/// **Target provenance is preserved on purpose.** Swinging onto a new victim
/// continues whatever acquisition installed the target in the first place — an
/// auto-retarget after the old victim died is not a new order — so
/// `passively_acquired_target` carries over. Clearing it here would leave the
/// object holding a live target with the flag false, and that state is exactly
/// what the passive block, the pursuit skip and the release-on-range-loss path
/// all key off: the object would stop re-evaluating, start being chased across
/// the map, and never let go of a target that walked out of range. The
/// flag-clearing that the original's target ASSIGNMENT performs lives in the
/// target setter, which is the assignment's counterpart; this in-place swing has
/// no counterpart there.
pub(crate) fn retarget_preserving_rearm(entity: &mut GameEntity, new_target_sid: u64) {
    if let Some(ref mut attack) = entity.attack_target {
        attack.target = TargetKind::Entity(new_target_sid);
        attack.pending_infantry_fire = None;
    }
}

/// Issue a force-fire-on-cell command: make `attacker` fire at a ground cell.
///
/// Used by `Command::ForceAttackCell` (Ctrl + left-click on empty terrain).
/// Aborts (returns `false`) if the attacker has no weapon — caller filters
/// unarmed units client-side, but this defensive check keeps a stray command
/// from corrupting state.
pub fn issue_attack_cell_command(
    entities: &mut EntityStore,
    attacker_id: u64,
    target_rx: u16,
    target_ry: u16,
    rules: Option<&RuleSet>,
    interner: &StringInterner,
) -> bool {
    // Read attacker position + weapon presence before mutable borrow.
    let attacker_info = entities.get(attacker_id).map(|a| {
        let type_str = interner.resolve(a.type_ref);
        let has_weapon = rules
            .and_then(|r| r.object(type_str))
            .is_some_and(|obj| obj.primary.is_some() || obj.secondary.is_some());
        (
            a.position.rx,
            a.position.ry,
            a.barrel_facing.is_some(),
            has_weapon,
        )
    });
    let (arx, ary, has_turret, has_weapon) = match attacker_info {
        Some(info) => info,
        None => return false,
    };

    if !has_weapon {
        // Defensive: client-side filter should have routed this to Move.
        // Warn-log so the desync is visible rather than silent.
        log::warn!(
            "ForceAttackCell rejected for unarmed attacker {} (target cell {},{})",
            attacker_id,
            target_rx,
            target_ry
        );
        return false;
    }

    let (trx, try_, _tsx, _tsy) = cell_center_coords(target_rx, target_ry);

    let attacker = match entities.get_mut(attacker_id) {
        Some(a) => a,
        None => return false,
    };

    if !has_turret {
        let dx: i32 = trx as i32 - arx as i32;
        let dy: i32 = try_ as i32 - ary as i32;
        attacker.facing = crate::sim::movement::facing_from_delta(dx, dy);
    }

    attacker.movement_target = None;
    attacker.attack_target = Some(AttackTarget::for_cell(target_rx, target_ry));
    attacker.passively_acquired_target = false;
    true
}

/// Compute distance in cells between two entities' grid positions.
#[cfg(test)]
pub(crate) fn cell_distance(ax: u16, ay: u16, bx: u16, by: u16) -> f32 {
    let dx: f32 = ax as f32 - bx as f32;
    let dy: f32 = ay as f32 - by as f32;
    (dx * dx + dy * dy).sqrt()
}

use self::combat_targeting::{AttackerSnapshot, GarrisonSnapshot, acquire_best_target};

/// Advance combat for all entities with AttackTarget components.
pub fn tick_combat(
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    current_tick: u64,
    tick_ms: u32,
    binary_frame: u32,
    scenario_rng: &mut SimRng,
) -> CombatTickResult {
    tick_combat_with_fog(
        entities,
        occupancy,
        rules,
        interner,
        None,
        &BTreeMap::new(),
        None,
        resource_nodes,
        None,
        None,
        None,
        current_tick,
        tick_ms,
        binary_frame,
        // Convenience shim (tests only); empty live order falls back to the
        // stable-id resolution order, preserving prior behavior exactly.
        &[],
        None,
        scenario_rng,
    )
}

/// Destroyed crewed building — survivor ejection is deferred to the caller
/// (which has access to `Simulation` for spawning infantry).
pub struct DestroyedCrewedBuilding {
    pub type_id: InternedId,
    pub owner: InternedId,
    pub rx: u16,
    pub ry: u16,
    pub z: u8,
}

/// A `CanBeOccupied` building destroyed in combat with live occupants —
/// gamemd routes this through `BuildingClass::SellBuilding @ 0x00457DE0`, the
/// same occupant-eject helper used by sell. The world layer owns the deferred
/// repositioning because it has access to `Simulation` and the occupancy grid.
pub struct DestroyedGarrisonBuilding {
    pub building_id: u64,
    pub type_id: InternedId,
    /// Building's owner at time of death — ejected infantry inherit this.
    pub owner: InternedId,
    pub rx: u16,
    pub ry: u16,
    pub z: u8,
    pub foundation_w: u16,
    pub foundation_h: u16,
    /// Snapshot of `cargo.passengers` at time of death. LIFO order preserved
    /// (eject helper iterates in reverse).
    pub passenger_ids: Vec<u64>,
}

/// Explosion animation to spawn at a world position (deferred to caller
/// which has access to `Simulation` for WorldEffect spawning).
pub struct ExplosionEffect {
    pub shp_name: InternedId,
    pub rx: u16,
    pub ry: u16,
    /// Sub-cell impact X in leptons. Preserves the CoordStruct-level impact
    /// point for warhead AnimList placement.
    pub sub_x: SimFixed,
    /// Sub-cell impact Y in leptons.
    pub sub_y: SimFixed,
    pub z: u8,
}

/// One transient combat-light request emitted when active IronCurtain or
/// ForceShield rejects a positive receiver call. `FUN_0048A620` creates an
/// unowned screen-space light, not an AnimClass/ParticleSystem, so this record
/// retains the exact call inputs without inventing an attachment or house.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvulnerabilityImpactEffect {
    /// Receiver provenance only; this is not native effect ownership.
    pub target_id: u64,
    /// Post-defender-transform damage shifted left once before the helper call.
    pub doubled_damage: i32,
    pub warhead_ref: InternedId,
    pub coord: ProjectileCoord,
    /// Native helper force/create argument (literal true on this callsite).
    pub force_create: bool,
    /// Native raw draw flags: IC=1, ForceShield=6.
    pub flags: u32,
}

/// One smudge producer payload. Production commits it synchronously through
/// `CombatInlineHooks`; hookless combat fixtures retain it in their result as
/// a test adapter.
#[derive(Debug, Clone)]
pub enum SmudgeSpawnRequest {
    /// Emitted alongside ExplosionEffect when a warhead's AnimList anim spawns.
    /// Carries the anim's interned SHP name for AnimType flag lookup.
    Anim {
        anim_name: InternedId,
        rx: u16,
        ry: u16,
        sub_x: SimFixed,
        sub_y: SimFixed,
        /// Exact absolute CoordStruct Z. ExplosionEffect keeps its separate
        /// coarse presentation byte; the native smudge altitude gate is in
        /// leptons and must never reconstruct this value from that byte.
        world_z_leptons: i32,
    },
    /// Emitted once per >=2x2 building destruction (DestructionEffects path).
    BuildingCenter {
        rx: u16,
        ry: u16,
        building_z: i32,
        foundation_w: u8,
        foundation_h: u8,
    },
    /// Emitted per surviving foundation cell (SpawnSurvivors path).
    BuildingSurvivor { cell_rx: u16, cell_ry: u16 },
}

fn append_building_smudge_requests(
    requests: &mut Vec<SmudgeSpawnRequest>,
    rx: u16,
    ry: u16,
    building_z: i32,
    foundation: &str,
) {
    let (foundation_w, foundation_h) = foundation_dimensions(foundation);
    requests.push(SmudgeSpawnRequest::BuildingCenter {
        rx,
        ry,
        building_z,
        foundation_w: foundation_w as u8,
        foundation_h: foundation_h as u8,
    });
    for (dx, dy) in crate::rules::foundation::foundation_cell_offsets(foundation) {
        let cell_rx = i32::from(rx) + i32::from(dx);
        let cell_ry = i32::from(ry) + i32::from(dy);
        let (Ok(cell_rx), Ok(cell_ry)) = (u16::try_from(cell_rx), u16::try_from(cell_ry)) else {
            continue;
        };
        requests.push(SmudgeSpawnRequest::BuildingSurvivor { cell_rx, cell_ry });
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_infantry_death_anim(
    general: &crate::rules::ruleset::GeneralRules,
    inf_death: u8,
    rx: u16,
    ry: u16,
    sub_x: SimFixed,
    sub_y: SimFixed,
    z: u8,
    world_z_leptons: i32,
    interner: &mut StringInterner,
    explosion_effects: &mut Vec<ExplosionEffect>,
    smudge_spawn_requests: &mut Vec<SmudgeSpawnRequest>,
) {
    let Some(anim_name) = general.infantry_death_anim(inf_death) else {
        return;
    };
    let anim_name = interner.intern(anim_name);
    explosion_effects.push(ExplosionEffect {
        shp_name: anim_name,
        rx,
        ry,
        sub_x,
        sub_y,
        z,
    });
    smudge_spawn_requests.push(SmudgeSpawnRequest::Anim {
        anim_name,
        rx,
        ry,
        sub_x,
        sub_y,
        world_z_leptons,
    });
}

/// Emit the warhead's AnimList animation and a paired smudge spawn request
/// for one detonation at (rx, ry, z). Mirrors gamemd's WarheadType::Detonate
/// dispatch into AnimClass::Start: every detonation that spawns an anim
/// also runs the anim's first-frame smudge logic.
///
/// Pushes nothing if `warhead.anim_list` is empty.
///
/// `base_damage` is the post-modifier damage at the impact center; it
/// drives AnimList selection via `damage / 25`, clamped to `len - 1`.
/// RESIDUAL (GSI-08.11) — a dying unit does not play its own explosion. The
/// effect chosen here comes from the *warhead*'s `AnimList=` (70 stock entries)
/// indexed by damage; the TechnoType's own `Explosion=` list is authored on 487
/// stock entries and has no reader anywhere in the crate. Crew survival is the
/// other half: `Crewed=` (126 stock) only queues the building arm, so a
/// destroyed `Crewed=yes` vehicle ejects nobody, and no survivor-type resolution
/// exists — the survivor path here handles the smudge, not the unit.
/// - Trigger: every unit and building death.
/// - Player effect: deaths look wrong twice over. A Grizzly and an Apocalypse
///   die with the same warhead-derived puff instead of their authored
///   explosions, and no crew ever runs out of a wreck or a levelled structure.
/// - Frequency: continuous — this is the most-watched moment in the game.
/// - Downstream risk: crew ejection spawns entities, so it moves unit counts,
///   occupancy and the pinned replay hash; the explosion swap is comparatively
///   contained but still changes anim spawn order and its RNG draws. The debris
///   half of this row is recorded separately on `rules/warhead_type.rs`.
pub(crate) fn emit_warhead_detonation_effects(
    warhead: &WarheadType,
    base_damage: i32,
    rx: u16,
    ry: u16,
    sub_x: SimFixed,
    sub_y: SimFixed,
    z: u8,
    world_z_leptons: i32,
    interner: &mut StringInterner,
    explosion_effects: &mut Vec<ExplosionEffect>,
    smudge_spawn_requests: &mut Vec<SmudgeSpawnRequest>,
) {
    if warhead.anim_list.is_empty() {
        return;
    }
    let idx = (base_damage / ANIM_LIST_DAMAGE_STEP as i32).max(0) as usize;
    let idx = idx.min(warhead.anim_list.len() - 1);
    let interned_name = interner.intern(&warhead.anim_list[idx]);
    explosion_effects.push(ExplosionEffect {
        shp_name: interned_name,
        rx,
        ry,
        sub_x,
        sub_y,
        z,
    });
    smudge_spawn_requests.push(SmudgeSpawnRequest::Anim {
        anim_name: interned_name,
        rx,
        ry,
        sub_x,
        sub_y,
        world_z_leptons,
    });
}

/// One captured TerrainClass receiver in a fixed Apply_area_damage transaction.
///
/// Transient only: stable identity, cell, distance, and isolation scope are
/// captured during collection so dispatch never rescans a later world state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainDamageEvent {
    pub stable_id: u64,
    pub rx: u16,
    pub ry: u16,
    pub damage: i32,
    pub distance_leptons: i32,
    pub warhead_ref: InternedId,
    /// True when the parent AoE used native binary32 CellSpread <= 0.5.
    /// Terrain cannot arm IC isolation, but an armed transaction skips it.
    pub near_center_ic_isolation_eligible: bool,
}

/// Hookless compatibility record for one admitted per-cell tiberium reduction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TiberiumReductionRequest {
    pub rx: u16,
    pub ry: u16,
    pub amount: i32,
}

/// Result of a combat tick: reveal events + stable IDs of despawned entities.
pub struct CombatTickResult {
    /// Ordinary projectiles created by fire this frame, to admit after the
    /// current BulletClass pass. New bullets never advance recursively.
    pub projectile_spawns: Vec<ProjectileSpawn>,
    pub reveal_events: Vec<RevealEvent>,
    pub despawned_ids: Vec<u64>,
    /// IDs that should enter world-owned UnInit immediately this tick.
    /// `despawned_ids` also includes SHP deaths that remain in-store for their
    /// death animation; this list is the immediate structure/voxel handoff only.
    pub immediate_uninit_ids: Vec<u64>,
    /// A structure was destroyed — PathGrid needs footprint unblock.
    pub structure_destroyed: bool,
    /// Bridge impact cells that should apply terrain damage after combat resolution.
    pub bridge_damage_events: Vec<BridgeDamageEvent>,
    /// Wall writes committed inline in exact cell/recursive cleanup order.
    pub wall_mutations: Vec<WallMutation>,
    /// Scanned-cell target detach visits committed inline, descending stable ID.
    /// Runtime applies them in-place; tests retain the ledger as an order audit.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) cell_target_detaches: Vec<combat_aoe::CellTargetDetach>,
    /// Terrain cells whose inline receiver removed live spatial authority.
    /// World rebuilds cost/path/zone caches from the already-mutated resolved
    /// terrain before later same-frame consumers.
    pub terrain_navigation_changed_cells: Vec<(u16, u16)>,
    /// Tiberium cells that should be reduced through the shared cell reducer.
    pub tiberium_reduction_requests: Vec<TiberiumReductionRequest>,
    /// Fire events for render-side muzzle flash / projectile origin computation.
    pub fire_events: Vec<SimFireEvent>,
    /// Crewed buildings destroyed this tick — survivors should be ejected by the caller.
    pub destroyed_crewed_buildings: Vec<DestroyedCrewedBuilding>,
    /// Garrisoned buildings destroyed this tick — occupants should be ejected
    /// by the caller via `production::eject_destruction_garrison`.
    pub destroyed_garrison_buildings: Vec<DestroyedGarrisonBuilding>,
    /// Explosion animations to spawn at death/impact locations.
    pub explosion_effects: Vec<ExplosionEffect>,
    /// Receiver-ordered IC/ForceShield transient combat-light requests.
    pub invulnerability_impact_effects: Vec<InvulnerabilityImpactEffect>,
    /// Hookless-test adapter for smudge requests. Empty on the production world
    /// path because each producer commits before returning.
    pub smudge_spawn_requests: Vec<SmudgeSpawnRequest>,
    /// (unit_id, desired 16-bit barrel destination) — captured at Phase-2
    /// entry before current-frame attacker damage; that Unit's own explicit
    /// retarget/remove may replace it. Applied post-batch by
    /// `unit_post::apply_unit_facing`. Transient — never stored, serialized, or
    /// hashed.
    pub unit_facing: Vec<(u64, u16)>,
    /// Base-structure / harvester enemy-damage pings produced at the damage
    /// apply site. Drained by the world into BaseUnderAttack/MinerUnderAttack
    /// radar events + the local player's EVA dispatch.
    pub under_attack_events: Vec<UnderAttackEvent>,
}

/// A "your asset is being shot" ping: a Structure or harvester took damage
/// from a different house this tick.
#[derive(Debug, Clone, Copy)]
pub struct UnderAttackEvent {
    pub rx: u16,
    pub ry: u16,
    /// The VICTIM's owner — the player whose radar/EVA should react.
    pub owner: InternedId,
    /// True when the victim is a harvester (miner ping), else a base structure.
    pub miner: bool,
}

/// Exact ObjectClass-style world Z for effect and projectile coordinates.
///
/// This is deliberately distinct from [`in_range::effective_z_leptons`]: the
/// range helper applies low-flight targeting rules, while native animation and
/// bullet coordinates retain the object's actual airborne height. An explicit
/// exact coordinate is already absolute. Otherwise the base is exact sloped
/// terrain plus the object-owned bridge deck, followed by the one active
/// object/locomotor altitude source in presentation precedence order.
pub(crate) fn object_world_z_leptons(
    entity: &GameEntity,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
) -> i32 {
    if let Some(exact_z_leptons) = entity.position.exact_z_leptons {
        return exact_z_leptons;
    }

    let world_x = i32::from(entity.position.rx)
        .wrapping_mul(256)
        .wrapping_add(entity.position.sub_x.to_num::<i32>());
    let world_y = i32::from(entity.position.ry)
        .wrapping_mul(256)
        .wrapping_add(entity.position.sub_y.to_num::<i32>());
    let base_z = terrain
        .and_then(|terrain| terrain.cell(entity.position.rx, entity.position.ry))
        .and_then(|cell| ground_height_leptons(cell.level, cell.slope_type, world_x, world_y).ok())
        .map(|ground_z| {
            ground_z.wrapping_add(if entity.on_bridge {
                BRIDGE_DECK_HEIGHT_LEPTONS
            } else {
                0
            })
        })
        // Position.z is already the effective layer level (including a bridge
        // deck), so the mapless fallback must not add OnBridge a second time.
        .unwrap_or_else(|| i32::from(entity.position.z).wrapping_mul(LEPTONS_PER_LEVEL as i32));

    let altitude = entity
        .parachute_state
        .as_ref()
        .map(|state| state.altitude.to_num::<i32>())
        .or_else(|| {
            entity
                .rocket_state
                .as_ref()
                .map(|state| state.altitude.to_num::<i32>())
        })
        .or_else(|| {
            entity
                .drop_pod_state
                .as_ref()
                .filter(|state| {
                    state.phase == crate::sim::movement::drop_pod_movement::DropPodPhase::Descending
                })
                .map(|state| state.altitude.to_num::<i32>())
        })
        .or_else(|| {
            entity
                .locomotor
                .as_ref()
                .filter(|locomotor| {
                    locomotor.layer == crate::sim::movement::locomotor::MovementLayer::Air
                        && locomotor.kind != crate::rules::locomotor_type::LocomotorKind::Rocket
                })
                .map(|locomotor| locomotor.altitude.to_num::<i32>())
        })
        .unwrap_or(0);

    base_z.wrapping_add(altitude)
}

/// The **one** impact height for an attack, in tile-step level units (signed).
///
/// The original engine forms a single impact coordinate per detonation and
/// hands that same coordinate to area damage and to the animation placement —
/// there is no second Z anywhere on the path. This function is VERA's
/// equivalent single value, and every consumer reads it rather than deriving
/// its own: the AoE object-layer selector, the bridge-damage Z gate, the
/// persistent-projectile impact coordinate, the impact-animation height, and
/// (through `app::presentation::fire_effects`) the pixel the tracer ends on. A second
/// derivation could only agree with this one by coincidence.
///
/// Three native quantities sit close together here and are not the same
/// thing:
/// * a cell's **own** coordinate — cell centre on both axes, terrain floor
///   height for Z;
/// * the **aim point** for a cell target — that, plus a four-level structural
///   bridge deck offset when a span crosses the cell;
/// * the **impact** coordinate — the projectile's own location, whose Z the
///   flight step clamps to the plain cell ground-height lookup at the moment
///   of ground contact. The resolution ladder that can substitute a target's
///   bridge-aware aim point runs only when there is a live *object* target;
///   for a shot at bare ground it is skipped entirely.
///
/// VERA models the impact, so a ground cell contributes its terrain floor
/// level and nothing else. There is no branch yielding zero: zero is what a
/// level-0 cell is worth, never a stand-in for a height we failed to look up.
/// VERA carries this quantity in whole tile-step levels along the entire
/// impact path; native carries the same step count scaled into leptons.
///
/// **Residual DRIFT — the structural-bridge deck term is unmodelled here.**
/// A force-fire at a bridge cell therefore damages the ground occupant list
/// and draws its explosion at ground height rather than four levels up on the
/// deck. It is not a one-line addition, because two VERA consumers want
/// opposite values: `combat_aoe::select_object_damage_layer` picks the bridge
/// occupant list only for an impact well above the cell's ground level (it
/// wants the deck term), while `bridge_state`'s path Z gate accepts only an
/// impact within one level of the cell's *ground* level (it rejects the deck
/// term outright). With ground-only Z that gate now admits every cell target:
/// it is **disabled, not widened** — a gate that can no longer reject
/// anything is not a modelled gate. That is RNG-visible: a path that newly
/// matches consumes a bridge-strength draw from the scenario stream, so a
/// replay containing a `Wall=yes` force-fire at a bridge over ground level ≥ 2
/// diverges from one recorded before this change. Trigger frequency: needs
/// deliberate bridge-cutting over raised ground, uncommon per match but a real
/// tactic on bridge maps.
/// *Settling step:* walk the bridge block of the native area-damage routine
/// and establish which reference **its** Z comparisons use — cell ground
/// height or deck plane — before either the gate or the deck term moves. The
/// VERA gate's native equivalent is UNCHECKED and is not authority for
/// dropping a verified native term.
///
/// `terrain` is `None` only where combat runs without a loaded map (headless
/// fixtures). With no map there is no cell to read — a VERA API boundary, not
/// a game rule.
pub(crate) fn attack_impact_z(
    target: TargetKind,
    entities: &EntityStore,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
) -> i32 {
    match target {
        TargetKind::Entity(eid) => entities
            .get(eid)
            .map(|entity| i32::from(entity.position.z))
            .unwrap_or(0),
        TargetKind::Cell(rx, ry) => terrain
            .and_then(|grid| grid.cell(rx, ry))
            .map(|cell| i32::from(cell.level))
            .unwrap_or(0),
    }
}

fn attack_air_impact(
    target: TargetKind,
    impact_rx: u16,
    impact_ry: u16,
    impact_sub_x: SimFixed,
    impact_sub_y: SimFixed,
    entities: &EntityStore,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
) -> Option<combat_aoe::AoEAirImpact> {
    match target {
        TargetKind::Entity(entity_id) => {
            combat_aoe::air_impact_from_entity(entities.get(entity_id)?, terrain)
        }
        TargetKind::Cell(_, _) => combat_aoe::air_impact_from_layer_z(
            terrain,
            impact_rx,
            impact_ry,
            impact_sub_x,
            impact_sub_y,
            attack_impact_z(target, entities, terrain),
        ),
    }
}

fn attack_world_z_leptons(
    target: TargetKind,
    impact_rx: u16,
    impact_ry: u16,
    impact_sub_x: SimFixed,
    impact_sub_y: SimFixed,
    entities: &EntityStore,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
) -> i32 {
    match target {
        TargetKind::Entity(entity_id) => entities
            .get(entity_id)
            .map(|entity| object_world_z_leptons(entity, terrain))
            .unwrap_or_else(|| {
                attack_impact_z(target, entities, terrain).wrapping_mul(LEPTONS_PER_LEVEL as i32)
            }),
        TargetKind::Cell(_, _) => combat_aoe::air_impact_from_layer_z(
            terrain,
            impact_rx,
            impact_ry,
            impact_sub_x,
            impact_sub_y,
            attack_impact_z(target, entities, terrain),
        )
        .map(|impact| impact.z_leptons)
        .unwrap_or_else(|| {
            attack_impact_z(target, entities, terrain).wrapping_mul(LEPTONS_PER_LEVEL as i32)
        }),
    }
}

/// Narrow an impact z into the byte the presentation path carries it in.
///
/// The projection that turns that byte into a pixel decodes it with `as i8`
/// (`util::lepton::lepton_to_screen`), so the byte is a *signed* level count
/// and the only correct saturation is into `i8` range: clamping into `u8`
/// range instead would let 200 through, which decodes as -56 levels and throws
/// the sprite most of a screen away. One definition, so the sim's animation
/// height and the app's tracer endpoint cannot narrow the same number
/// differently.
///
/// Known mismatch, outside this file: the sprite depth key reads the same byte
/// as unsigned. The two readings agree over 0..=127, which covers every map
/// height, so it is latent — but a negative impact z would sort by one rule
/// and draw by the other.
pub(crate) fn impact_z_byte(impact_z: i32) -> u8 {
    impact_z.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8 as u8
}

fn death_weapon_ftol_product_i32_f32(value: i32, multiplier: f32) -> Option<i32> {
    let multiplier = X87Chop53::load_f32(NativeF32Bits::from_bits(multiplier.to_bits())).ok()?;
    let product = X87Chop53::mul(X87Chop53::load_i32(value), multiplier);
    i32::try_from(X87Chop53::ftol_i64(product).ok()?).ok()
}

fn death_weapon_half_strength(strength: i32) -> Option<i32> {
    let half = X87Chop53::load_f64(NativeF64Bits::HALF).ok()?;
    let product = X87Chop53::mul(X87Chop53::load_i32(strength), half);
    i32::try_from(X87Chop53::ftol_i64(product).ok()?).ok()
}

/// Resolve the active fatal-receiver death producer. The reachability gate is
/// independent of `DeathWeapon=`: native consults effective Explodes abilities
/// and the receiver's live `CurrentWeaponNumber` Suicide flag first. Once
/// admitted, selection is explicit type weapon, current weapon, then the Rules
/// default, with distinct native damage formulas for the first two vs default.
fn death_weapon_aoe(
    rules: &RuleSet,
    obj: &ObjectType,
    veterancy: u16,
    current_weapon_index: u8,
    current_weapon_ref: Option<InternedId>,
    interner: &mut StringInterner,
) -> Option<(i32, InternedId, InternedId)> {
    let selected_current_weapon =
        current_weapon_ref.and_then(|weapon_id| rules.weapon(interner.resolve(weapon_id)));
    let slot_current_weapon =
        combat_weapon::weapon_for_slot_index(obj, veterancy, i32::from(current_weapon_index))
            .and_then(|(weapon_id, _)| rules.weapon(weapon_id));
    let current_weapon = selected_current_weapon.or(slot_current_weapon);
    let effective_explodes = obj.explodes
        || (veterancy >= 100 && obj.veteran_explodes)
        || (veterancy >= 200 && obj.elite_explodes);
    if !effective_explodes && !current_weapon.is_some_and(|weapon| weapon.suicide) {
        return None;
    }

    if let Some(explicit) = obj
        .death_weapon
        .as_deref()
        .and_then(|weapon_id| rules.weapon(weapon_id))
    {
        let damage =
            death_weapon_ftol_product_i32_f32(explicit.damage, obj.death_weapon_damage_modifier)?;
        let warhead_ref = interner.intern(explicit.warhead.as_ref()?);
        let weapon_ref = interner.intern(&explicit.id);
        return Some((damage, warhead_ref, weapon_ref));
    }
    if let Some(current) = current_weapon {
        let damage =
            death_weapon_ftol_product_i32_f32(current.damage, obj.death_weapon_damage_modifier)?;
        let warhead_ref = interner.intern(current.warhead.as_ref()?);
        let weapon_ref = interner.intern(&current.id);
        return Some((damage, warhead_ref, weapon_ref));
    }
    let fallback = rules
        .combat_damage
        .death_weapon
        .as_deref()
        .and_then(|weapon_id| rules.weapon(weapon_id))?;
    let warhead_ref = interner.intern(fallback.warhead.as_ref()?);
    let weapon_ref = interner.intern(&fallback.id);
    Some((
        death_weapon_half_strength(obj.strength)?,
        warhead_ref,
        weapon_ref,
    ))
}

/// Collected side-effects from processing entity deaths in a single tick.
#[derive(Default)]
pub(crate) struct DeathEffects {
    pub(crate) despawned_ids: Vec<u64>,
    pub(crate) immediate_uninit_ids: Vec<u64>,
    pub(crate) structure_destroyed: bool,
    pub(crate) destroyed_crewed_buildings: Vec<DestroyedCrewedBuilding>,
    pub(crate) destroyed_garrison_buildings: Vec<DestroyedGarrisonBuilding>,
    pub(crate) explosion_effects: Vec<ExplosionEffect>,
    pub(crate) invulnerability_impact_effects: Vec<InvulnerabilityImpactEffect>,
    pub(crate) bridge_damage_events: Vec<BridgeDamageEvent>,
    pub(crate) wall_mutations: Vec<WallMutation>,
    pub(crate) cell_target_detaches: Vec<combat_aoe::CellTargetDetach>,
    pub(crate) tiberium_reduction_requests: Vec<TiberiumReductionRequest>,
    pub(crate) death_sounds: Vec<(InternedId, u16, u16)>,
    pub(crate) smudge_spawn_requests: Vec<SmudgeSpawnRequest>,
    pub(crate) rad_detonations: Vec<crate::sim::radiation::RadDetonation>,
    pub(crate) under_attack_events: Vec<UnderAttackEvent>,
    #[cfg(test)]
    pub(crate) receiver_stage_trace: Vec<ReceiverStageTrace>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReceiverStageTrace {
    HouseThreat { target_id: u64, delta: i32 },
    PostMortem { target_id: u64 },
    ShouldRetaliate { target_id: u64 },
}

/// World-owned lifecycle work that brackets the native death helper for a
/// concrete fatal receiver. Passenger teardown precedes the nested death
/// weapon; represented UnInit follows it before the next outer receiver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FatalLifecycleStage {
    /// Surviving Techno ReceiveDamage postlude, before Infantry scatter and
    /// synchronous retaliation. The world owns ParticleSystem storage and the
    /// shared LogicVector, so maintenance crosses the existing inline hook.
    MaintainDamageSmoke {
        state: damage::DamageState,
    },
    /// ObjectClass's exact-zero callback transaction for an eligible delayed
    /// death. It runs while the target is still represented and Health is
    /// exactly zero, before TechnoClass arms/shortens the shared C4 timer and
    /// restores Alive/Health=1.
    PostMortemExactZero {
        killer_owner: Option<InternedId>,
    },
    BeforeDeathEffects,
    AfterDeathEffects,
}

/// World bridge for synchronous side effects whose authoritative
/// storage lives outside combat's move-out transaction. One object owns both
/// methods so `Simulation` is borrowed only once while combat temporarily owns
/// entities, map grids, and the scenario RNG.
pub(crate) trait CombatInlineHooks {
    #[allow(clippy::too_many_arguments)]
    fn fatal_lifecycle(
        &mut self,
        rules: &RuleSet,
        stage: FatalLifecycleStage,
        stable_id: u64,
        category: EntityCategory,
        entities: &mut EntityStore,
        occupancy: &mut OccupancyGrid,
        interner: &mut StringInterner,
        scenario_rng: &mut SimRng,
        terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
        terrain_area_state: Option<&mut TerrainAreaState>,
        sound_sink: Option<&mut Vec<SimSoundEvent>>,
    );

    #[allow(clippy::too_many_arguments)]
    fn commit_tiberium_reduction(
        &mut self,
        rules: &RuleSet,
        request: TiberiumReductionRequest,
        scenario_rng: &mut SimRng,
        resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
        overlay_grid: Option<&mut OverlayGrid>,
        overlay_registry: Option<&OverlayTypeRegistry>,
        terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
        terrain_area_state: Option<&TerrainAreaState>,
    );

    #[allow(clippy::too_many_arguments)]
    fn commit_smudge(
        &mut self,
        rules: &RuleSet,
        request: SmudgeSpawnRequest,
        occupancy: &OccupancyGrid,
        interner: &StringInterner,
        scenario_rng: &mut SimRng,
        resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
        overlay_grid: Option<&mut OverlayGrid>,
        overlay_registry: Option<&OverlayTypeRegistry>,
        terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
        terrain_area_state: Option<&TerrainAreaState>,
    );
}

fn tiberium_reduction_amount(
    base_damage: i32,
    affect_resource: bool,
    warhead: &WarheadType,
) -> Option<i32> {
    if !affect_resource || !warhead.tiberium {
        return None;
    }
    let amount = base_damage / 10;
    (amount > 0).then_some(amount)
}

/// Apply_area_damage's Reduce_Tiberium call belongs inside the spread-cell
/// walk: it precedes that cell's wall RNG and receiver-list capture. The hook
/// owns World-only ore-growth/dirty state; combat_aoe lends the map and RNG
/// fields it already owns for the duration of one cell.
struct CombatTiberiumCellPrelude<'a, 'hook> {
    amount: i32,
    deferred: &'a mut Vec<TiberiumReductionRequest>,
    inline_hooks: &'a mut Option<&'hook mut dyn CombatInlineHooks>,
    rules: &'a RuleSet,
    resource_nodes: &'a mut BTreeMap<(u16, u16), ResourceNode>,
    terrain_area_state: Option<&'a TerrainAreaState>,
}

impl self::combat_aoe::AoECellPrelude for CombatTiberiumCellPrelude<'_, '_> {
    #[allow(clippy::too_many_arguments)]
    fn before_cell(
        &mut self,
        rx: u16,
        ry: u16,
        overlay_grid: Option<&mut OverlayGrid>,
        overlay_registry: Option<&OverlayTypeRegistry>,
        terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
        scenario_rng: Option<&mut SimRng>,
    ) {
        if !self::combat_aoe::tiberium_reduction_cell_admitted(
            overlay_grid.as_deref(),
            overlay_registry,
            rx,
            ry,
        ) {
            return;
        }
        let request = TiberiumReductionRequest {
            rx,
            ry,
            amount: self.amount,
        };
        if let Some(hooks) = self.inline_hooks.as_deref_mut() {
            let scenario_rng = scenario_rng
                .expect("production Apply_area_damage tiberium prelude requires scenario RNG");
            hooks.commit_tiberium_reduction(
                self.rules,
                request,
                scenario_rng,
                self.resource_nodes,
                overlay_grid,
                overlay_registry,
                terrain,
                self.terrain_area_state,
            );
        } else {
            self.deferred.push(request);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn commit_smudge_or_defer(
    request: SmudgeSpawnRequest,
    deferred: &mut Vec<SmudgeSpawnRequest>,
    inline_hooks: &mut Option<&mut dyn CombatInlineHooks>,
    rules: &RuleSet,
    occupancy: &OccupancyGrid,
    interner: &StringInterner,
    scenario_rng: &mut SimRng,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    terrain_area_state: Option<&TerrainAreaState>,
) {
    if let Some(hooks) = inline_hooks.as_deref_mut() {
        hooks.commit_smudge(
            rules,
            request,
            occupancy,
            interner,
            scenario_rng,
            resource_nodes,
            overlay_grid,
            overlay_registry,
            terrain,
            terrain_area_state,
        );
    } else {
        deferred.push(request);
    }
}

#[allow(clippy::too_many_arguments)]
fn commit_smudge_batch_or_defer(
    requests: Vec<SmudgeSpawnRequest>,
    deferred: &mut Vec<SmudgeSpawnRequest>,
    inline_hooks: &mut Option<&mut dyn CombatInlineHooks>,
    rules: &RuleSet,
    occupancy: &OccupancyGrid,
    interner: &StringInterner,
    scenario_rng: &mut SimRng,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    mut overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    mut terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    terrain_area_state: Option<&TerrainAreaState>,
) {
    for request in requests {
        commit_smudge_or_defer(
            request,
            deferred,
            inline_hooks,
            rules,
            occupancy,
            interner,
            scenario_rng,
            resource_nodes,
            overlay_grid.as_deref_mut(),
            overlay_registry,
            terrain.as_deref_mut(),
            terrain_area_state,
        );
    }
}

impl DeathEffects {
    fn append(&mut self, mut other: Self) {
        self.despawned_ids.append(&mut other.despawned_ids);
        self.immediate_uninit_ids
            .append(&mut other.immediate_uninit_ids);
        self.structure_destroyed |= other.structure_destroyed;
        self.destroyed_crewed_buildings
            .append(&mut other.destroyed_crewed_buildings);
        self.destroyed_garrison_buildings
            .append(&mut other.destroyed_garrison_buildings);
        self.explosion_effects.append(&mut other.explosion_effects);
        self.invulnerability_impact_effects
            .append(&mut other.invulnerability_impact_effects);
        self.bridge_damage_events
            .append(&mut other.bridge_damage_events);
        self.wall_mutations.append(&mut other.wall_mutations);
        self.cell_target_detaches
            .append(&mut other.cell_target_detaches);
        self.tiberium_reduction_requests
            .append(&mut other.tiberium_reduction_requests);
        self.death_sounds.append(&mut other.death_sounds);
        self.smudge_spawn_requests
            .append(&mut other.smudge_spawn_requests);
        self.rad_detonations.append(&mut other.rad_detonations);
        self.under_attack_events
            .append(&mut other.under_attack_events);
        #[cfg(test)]
        self.receiver_stage_trace
            .append(&mut other.receiver_stage_trace);
    }
}

fn append_selected_death_sounds(
    object_type: &ObjectType,
    owner_is_human: bool,
    main_rng: &mut SimRng,
    interner: &mut StringInterner,
    rx: u16,
    ry: u16,
    death_sounds: &mut Vec<(InternedId, u16, u16)>,
) {
    let mut append_choice = |choices: &[String]| {
        if choices.is_empty() {
            return;
        }
        let index = (main_rng.next_u32() % choices.len() as u32) as usize;
        death_sounds.push((interner.intern(&choices[index]), rx, ry));
    };

    if owner_is_human {
        append_choice(&object_type.voice_die);
    }
    append_choice(&object_type.die_sounds);
}

/// Concrete-class death work that native runs only after the shared Techno
/// death-weapon transaction has returned. Keeping the plan data-only avoids
/// consuming smudge RNG (or interning the InfDeath AnimType) too early.
enum ConcreteDeathSmudgePlan {
    Infantry {
        inf_death: u8,
        rx: u16,
        ry: u16,
        sub_x: SimFixed,
        sub_y: SimFixed,
        z: u8,
        world_z_leptons: i32,
    },
    Building {
        rx: u16,
        ry: u16,
        z: i32,
        foundation: String,
    },
}

/// Process combat-owned death effects and classify the lifecycle handoff.
///
/// Extracts death side-effects into a `DeathEffects` struct so the caller can apply them
/// (bridge damage, sound events, etc.) without the combat function growing unbounded.
fn handle_entity_deaths(
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    handles: Option<crate::sim::type_handle_table::ResolvedRuleHandles>,
    houses: &mut BTreeMap<InternedId, HouseState>,
    house_order: &[InternedId],
    alliances: &HouseAllianceMap,
    main_rng: &mut SimRng,
    scenario_rng: &mut SimRng,
    handled_deaths: &mut Vec<u64>,
    dead_entities: &[u64],
    damage_events: &[EntityDamageEvent],
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    mut overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    mut terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    terrain_area_state: &mut Option<&mut TerrainAreaState>,
    scenario_no_damage: bool,
    current_tick: u64,
    inline_hooks: &mut Option<&mut dyn CombatInlineHooks>,
    sound_sink: &mut Option<&mut Vec<SimSoundEvent>>,
) -> DeathEffects {
    debug_assert!(
        dead_entities.len() <= 1,
        "ReceiveDamage enters one concrete fatal postlude at a time"
    );
    let mut death_sounds: Vec<(InternedId, u16, u16)> = Vec::new();
    #[cfg(test)]
    let mut receiver_stage_trace = Vec::new();
    let mut tiberium_reduction_requests: Vec<TiberiumReductionRequest> = Vec::new();
    // Death-weapon detonations use the destroyed object's game-space position.
    // The cell and z still drive damage/smudge dispatch; sub-cell leptons keep
    // AnimList placement aligned with the detonation CoordStruct shape.
    let mut death_aoe: Vec<(
        u16,
        u16,
        SimFixed,
        SimFixed,
        u8,
        i32,
        Option<combat_aoe::AoEAirImpact>,
        i32,
        InternedId,
        InternedId,
        u64,
        InternedId,
    )> = Vec::new();
    let mut despawned_ids: Vec<u64> = Vec::new();
    let mut immediate_uninit_ids: Vec<u64> = Vec::new();
    let mut destroyed_crewed_buildings: Vec<DestroyedCrewedBuilding> = Vec::new();
    let mut destroyed_garrison_buildings: Vec<DestroyedGarrisonBuilding> = Vec::new();
    let mut explosion_effects: Vec<ExplosionEffect> = Vec::new();
    let mut invulnerability_impact_effects: Vec<InvulnerabilityImpactEffect> = Vec::new();
    let mut bridge_damage_events: Vec<BridgeDamageEvent> = Vec::new();
    let mut wall_mutations: Vec<WallMutation> = Vec::new();
    let mut cell_target_detaches: Vec<combat_aoe::CellTargetDetach> = Vec::new();
    let mut smudge_spawn_requests: Vec<SmudgeSpawnRequest> = Vec::new();
    let mut concrete_smudge_plans: Vec<ConcreteDeathSmudgePlan> = Vec::new();
    let mut rad_detonations: Vec<crate::sim::radiation::RadDetonation> = Vec::new();
    let mut under_attack_events: Vec<UnderAttackEvent> = Vec::new();
    let mut structure_destroyed: bool = false;
    for &dead_id in dead_entities {
        // ReceiveDamage enters the death helper exactly once at the fatal
        // transition. Non-animated objects remain in the store until the
        // world-owned UnInit handoff, so keep this tick-local guard explicit.
        if handled_deaths.contains(&dead_id) {
            continue;
        }
        handled_deaths.push(dead_id);
        let dead_info = entities.get(dead_id).map(|e| {
            if e.category == EntityCategory::Structure {
                structure_destroyed = true;
            }
            let air_impact = combat_aoe::air_impact_from_entity(e, terrain.as_deref());
            let world_z_leptons = object_world_z_leptons(e, terrain.as_deref());
            (
                e.type_ref,
                e.position.rx,
                e.position.ry,
                e.position.sub_x,
                e.position.sub_y,
                e.position.z,
                world_z_leptons,
                air_impact,
                e.owner,
                e.animation.is_some(),
                e.category,
                e.veterancy,
                e.current_weapon_index,
                e.current_weapon_ref,
            )
        });

        if let Some((
            type_id,
            rx,
            ry,
            sub_x,
            sub_y,
            z,
            world_z_leptons,
            air_impact,
            owner,
            has_animation,
            category,
            veterancy,
            current_weapon_index,
            current_weapon_ref,
        )) = dead_info
        {
            let type_id_str = interner.resolve(type_id);
            if let Some(obj) = rules.object(type_id_str) {
                append_selected_death_sounds(
                    obj,
                    houses.get(&owner).is_some_and(|house| house.is_human),
                    main_rng,
                    interner,
                    rx,
                    ry,
                    &mut death_sounds,
                );
                if let Some((dmg, wh_id, weapon_id)) = death_weapon_aoe(
                    rules,
                    obj,
                    veterancy,
                    current_weapon_index,
                    current_weapon_ref,
                    interner,
                ) {
                    death_aoe.push((
                        rx,
                        ry,
                        sub_x,
                        sub_y,
                        z,
                        world_z_leptons,
                        air_impact,
                        dmg,
                        wh_id,
                        weapon_id,
                        dead_id,
                        owner,
                    ));
                }
                // Crewed structures eject infantry survivors on destruction.
                if obj.crewed && category == EntityCategory::Structure {
                    destroyed_crewed_buildings.push(DestroyedCrewedBuilding {
                        type_id: type_id,
                        owner: owner,
                        rx,
                        ry,
                        z,
                    });
                }
            }

            // Generic transport cargo remains attached and untouched here so
            // the carrier's world-owned UnInit can recurse in cargo order. The
            // snapshot is only exported for the distinct garrison-eject path.
            let passenger_ids: Vec<u64> = entities
                .get(dead_id)
                .and_then(|e| e.passenger_role.cargo())
                .map(|c| c.passengers.clone())
                .unwrap_or_default();

            // Garrisoned CanBeOccupied buildings use the same gamemd
            // SellBuilding occupant-eject contract as sell. Generic transports
            // deliberately emit no passenger-side mutations from combat.
            //
            // Re-resolve the type string here because earlier mutable borrows
            // of `interner` (death_weapon_aoe, intern calls) ended its prior
            // immutable borrow.
            let type_id_str_for_branch = interner.resolve(type_id);
            let is_garrison_building = rules
                .object(type_id_str_for_branch)
                .map(|obj| obj.can_be_occupied)
                .unwrap_or(false)
                && category == EntityCategory::Structure
                && !passenger_ids.is_empty();

            if is_garrison_building {
                let (foundation_w, foundation_h) = rules
                    .object(type_id_str_for_branch)
                    .map(|obj| crate::sim::production::foundation_dimensions(&obj.foundation))
                    .unwrap_or((1, 1));
                destroyed_garrison_buildings.push(DestroyedGarrisonBuilding {
                    building_id: dead_id,
                    type_id,
                    owner,
                    rx,
                    ry,
                    z,
                    foundation_w,
                    foundation_h,
                    passenger_ids,
                });
            }

            // Look up the warhead that dealt the killing blow for InfDeath
            // selection below. The AnimList anim + smudge are emitted at
            // the per-shot fire site (and at the death-AoE loop), not here.
            let killing_warhead = damage_events
                .iter()
                .rfind(|event| event.target_id == dead_id)
                .and_then(|event| {
                    rules
                        .warhead(interner.resolve(event.warhead_ref))
                        .map(|wh| (wh, event.damage))
                });

            // BuildingClass runs DestructionEffects/SpawnSurvivors only after
            // TechnoClass's synchronous death weapon has returned. Capture the
            // immutable plan now; placement and all RNG stay at that postlude.
            if category == EntityCategory::Structure {
                let foundation = rules
                    .object(interner.resolve(type_id))
                    .map(|obj| obj.foundation.as_str())
                    .unwrap_or("1x1");
                concrete_smudge_plans.push(ConcreteDeathSmudgePlan::Building {
                    rx,
                    ry,
                    z: i32::from(z),
                    foundation: foundation.to_owned(),
                });
            }

            if has_animation {
                // Transitional Infantry/SHP handoff: health was already reduced
                // to zero by damage processing. Combat owns only the Rust death
                // gate and sequence selection until Mission/Foot owns cadence.
                // Select InfDeath variant from the killing warhead (default Die1).
                let inf_death: u8 = killing_warhead
                    .as_ref()
                    .map(|(wh, _)| wh.inf_death)
                    .unwrap_or(1);
                if category == EntityCategory::Infantry {
                    concrete_smudge_plans.push(ConcreteDeathSmudgePlan::Infantry {
                        inf_death,
                        rx,
                        ry,
                        sub_x,
                        sub_y,
                        z,
                        world_z_leptons,
                    });
                }
                if let Some(entity) = entities.get_mut(dead_id) {
                    entity.dying = true;
                    if let Some(ref mut anim) = entity.animation {
                        use crate::sim::animation::death_sequence_for_inf_death;
                        anim.switch_to(death_sequence_for_inf_death(inf_death));
                    }
                }
                // Still report as "despawned" for fog/path updates — entity is
                // functionally dead even though the sprite lingers for the animation.
                despawned_ids.push(dead_id);
                log::trace!("Entity {} dying (death animation)", dead_id);
            } else {
                // Structures and voxel vehicles remain otherwise intact. The
                // world consumes this request through ordered UnInit, which owns
                // deselection, targets, radio, cell/logic state, and passengers.
                immediate_uninit_ids.push(dead_id);
                despawned_ids.push(dead_id);
                log::trace!("Entity {} destroyed", dead_id);
            }
        }
    }

    // Apply death explosion AoE damage.
    for (
        rx,
        ry,
        sub_x,
        sub_y,
        z,
        world_z_leptons,
        air_impact,
        dmg,
        wh_id,
        weapon_id,
        source_id,
        owner_id,
    ) in &death_aoe
    {
        if let Some(warhead) = rules.warhead(interner.resolve(*wh_id)) {
            let routed_wall =
                wall_overlay_flags_at(overlay_grid.as_deref(), overlay_registry, *rx, *ry)
                    .is_some_and(|flags| warhead_damages_wall(warhead, flags));
            let ore_amount = if scenario_no_damage {
                None
            } else {
                tiberium_reduction_amount(*dmg, true, warhead)
            };
            let terrain_collection = terrain_area_state.as_deref().map(|state| {
                self::combat_aoe::TerrainCollectionView {
                    objects: state.terrain_objects(),
                    cells: state.terrain_object_cells(),
                }
            });
            let aoe = {
                let mut ore_prelude = ore_amount.map(|amount| CombatTiberiumCellPrelude {
                    amount,
                    deferred: &mut tiberium_reduction_requests,
                    inline_hooks,
                    rules,
                    resource_nodes,
                    terrain_area_state: terrain_area_state.as_deref(),
                });
                self::combat_aoe::apply_aoe_damage_with_terrain_and_scenario(
                    entities,
                    *rx,
                    *ry,
                    *dmg,
                    warhead,
                    rules,
                    interner,
                    handles,
                    (*source_id, Some(*owner_id), *wh_id),
                    self::combat_aoe::AoELayerContext {
                        occupancy: Some(&*occupancy),
                        terrain: terrain.as_deref_mut(),
                        overlay_grid: overlay_grid.as_deref_mut(),
                        overlay_registry,
                        scenario_rng: Some(&mut *scenario_rng),
                        air_impact: *air_impact,
                        impact_z: *z as i32,
                    },
                    terrain_collection,
                    scenario_no_damage,
                    ore_prelude
                        .as_mut()
                        .map(|prelude| prelude as &mut dyn self::combat_aoe::AoECellPrelude),
                )
            };
            wall_mutations.extend(aoe.wall_mutations);
            cell_target_detaches.extend(aoe.cell_target_detaches);
            if !scenario_no_damage && !routed_wall && warhead.wall && *dmg > 0 {
                let wh_iid = *wh_id;
                bridge_damage_events.push(BridgeDamageEvent {
                    rx: *rx,
                    ry: *ry,
                    damage: (*dmg).min(i32::from(u16::MAX)) as u16,
                    warhead_ref: wh_iid,
                    is_ion_cannon: wh_iid == handles
                        .expect("Simulation::resolve_type_handles must run before combat")
                        .ion_cannon,
                    impact_z: *z as i32,
                });
            }
            if let Some(weapon) = rules.weapon(interner.resolve(*weapon_id))
                && weapon.rad_level > 0
            {
                rad_detonations.push(crate::sim::radiation::RadDetonation {
                    rx: *rx,
                    ry: *ry,
                    rad_level: weapon.rad_level,
                    spread: warhead.cell_spread.to_num::<i32>(),
                });
            }
            // One native Apply_area_damage owns the whole fixed record vector.
            // The commit loop still enters ReceiveDamage/death effects inline
            // per record, while retaining transaction-wide IC isolation.
            let (mut nested, mut pings) = commit_area_damage_receivers_with_scenario(
                &aoe.receivers,
                entities,
                occupancy,
                rules,
                interner,
                handles,
                houses,
                house_order,
                alliances,
                main_rng,
                scenario_rng,
                handled_deaths,
                resource_nodes,
                overlay_grid.as_deref_mut(),
                overlay_registry,
                terrain.as_deref_mut(),
                terrain_area_state.as_deref_mut(),
                scenario_no_damage,
                current_tick,
                inline_hooks,
                sound_sink,
            );
            despawned_ids.append(&mut nested.despawned_ids);
            immediate_uninit_ids.append(&mut nested.immediate_uninit_ids);
            structure_destroyed |= nested.structure_destroyed;
            destroyed_crewed_buildings.append(&mut nested.destroyed_crewed_buildings);
            destroyed_garrison_buildings.append(&mut nested.destroyed_garrison_buildings);
            explosion_effects.append(&mut nested.explosion_effects);
            invulnerability_impact_effects.append(&mut nested.invulnerability_impact_effects);
            bridge_damage_events.append(&mut nested.bridge_damage_events);
            wall_mutations.append(&mut nested.wall_mutations);
            cell_target_detaches.append(&mut nested.cell_target_detaches);
            tiberium_reduction_requests.append(&mut nested.tiberium_reduction_requests);
            death_sounds.append(&mut nested.death_sounds);
            smudge_spawn_requests.append(&mut nested.smudge_spawn_requests);
            rad_detonations.append(&mut nested.rad_detonations);
            #[cfg(test)]
            receiver_stage_trace.append(&mut nested.receiver_stage_trace);
            under_attack_events.append(&mut pings);
            let outer_anim_start = smudge_spawn_requests.len();
            emit_warhead_detonation_effects(
                warhead,
                *dmg,
                *rx,
                *ry,
                *sub_x,
                *sub_y,
                *z,
                *world_z_leptons,
                interner,
                &mut explosion_effects,
                &mut smudge_spawn_requests,
            );
            let outer_anim_requests = smudge_spawn_requests.split_off(outer_anim_start);
            commit_smudge_batch_or_defer(
                outer_anim_requests,
                &mut smudge_spawn_requests,
                inline_hooks,
                rules,
                occupancy,
                interner,
                scenario_rng,
                resource_nodes,
                overlay_grid.as_deref_mut(),
                overlay_registry,
                terrain.as_deref_mut(),
                terrain_area_state.as_deref(),
            );
        }
    }

    // Concrete InfDeath AnimClass and building destruction smudges are the
    // receiver postlude. The structure is intentionally still represented and
    // its raw occupation bytes are still in TerrainAreaState during dispatch.
    for plan in concrete_smudge_plans {
        let mut requests = Vec::new();
        match plan {
            ConcreteDeathSmudgePlan::Infantry {
                inf_death,
                rx,
                ry,
                sub_x,
                sub_y,
                z,
                world_z_leptons,
            } => emit_infantry_death_anim(
                &rules.general,
                inf_death,
                rx,
                ry,
                sub_x,
                sub_y,
                z,
                world_z_leptons,
                interner,
                &mut explosion_effects,
                &mut requests,
            ),
            ConcreteDeathSmudgePlan::Building {
                rx,
                ry,
                z,
                foundation,
            } => append_building_smudge_requests(&mut requests, rx, ry, z, &foundation),
        }
        commit_smudge_batch_or_defer(
            requests,
            &mut smudge_spawn_requests,
            inline_hooks,
            rules,
            occupancy,
            interner,
            scenario_rng,
            resource_nodes,
            overlay_grid.as_deref_mut(),
            overlay_registry,
            terrain.as_deref_mut(),
            terrain_area_state.as_deref(),
        );
    }

    DeathEffects {
        despawned_ids,
        immediate_uninit_ids,
        structure_destroyed,
        destroyed_crewed_buildings,
        destroyed_garrison_buildings,
        explosion_effects,
        invulnerability_impact_effects,
        bridge_damage_events,
        wall_mutations,
        cell_target_detaches,
        tiberium_reduction_requests,
        death_sounds,
        smudge_spawn_requests,
        rad_detonations,
        under_attack_events,
        #[cfg(test)]
        receiver_stage_trace,
    }
}

/// Build the native ReceiveDamage value ABI for one ordered area or direct
/// receiver record and run the shared receiver exactly once. The returned
/// signed HP delta is the only health input consumed by `commit_damage_events`.
#[derive(Debug, Clone, Copy)]
struct ResolvedReceiveDamage {
    outcome: damage::DamageOutcome,
    invulnerability_impact: Option<InvulnerabilityImpactEffect>,
}

fn receiver_effect_coord(
    target: &GameEntity,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
) -> ProjectileCoord {
    let x = i32::from(target.position.rx)
        .wrapping_mul(256)
        .wrapping_add(target.position.sub_x.to_num::<i32>());
    let y = i32::from(target.position.ry)
        .wrapping_mul(256)
        .wrapping_add(target.position.sub_y.to_num::<i32>());
    let z = object_world_z_leptons(target, terrain);
    ProjectileCoord::new(x, y, z)
}

fn resolve_receive_damage(
    event: &EntityDamageEvent,
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
    houses: &BTreeMap<InternedId, HouseState>,
    alliances: &HouseAllianceMap,
    scenario_no_damage: bool,
    current_tick: u64,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
) -> Option<ResolvedReceiveDamage> {
    let distance_leptons = event.distance_leptons?;
    let receiver_flags = event.receiver_flags?;
    let target = entities.get(event.target_id)?;
    let warhead = rules.warhead(interner.resolve(event.warhead_ref))?;
    let target_type = rules.object(interner.resolve(target.type_ref));
    let source = (event.attacker_id != RAD_NO_ATTACKER)
        .then(|| entities.get(event.attacker_id))
        .flatten();
    let source_house = event
        .source_house
        .or_else(|| source.map(|entity| entity.owner));
    // InfantryClass mutates the positive raw i32 before forwarding to the
    // shared Techno receiver. Its sign is therefore Techno's original-sign
    // snapshot used by the IC/FS gate below.
    let receiver_input = infantry_prone_area_raw_damage(
        target,
        warhead,
        event.damage,
        receiver_flags.ignore_defenses,
    );

    let allied = |asker: InternedId, other: InternedId| {
        crate::map::houses::is_allied_with(
            alliances,
            interner.resolve(asker),
            interner.resolve(other),
        )
    };
    let attacker_is_allied = source_house.is_some_and(|owner| allied(owner, target.owner));
    let source_house_is_allied = source_house.is_some_and(|owner| allied(target.owner, owner));

    let target_is_building = target.category == EntityCategory::Structure;
    let target_view = damage::TargetDamageView {
        armor: damage::ArmorClass(
            target_type
                .map(|object| armor_index(&object.armor))
                .unwrap_or(0) as u8,
        ),
        strength: target_type
            .map(|object| object.strength)
            .filter(|&strength| strength > 0)
            .unwrap_or(i32::from(target.health.max)),
        current_hp: i32::from(target.health.current),
        object_immune: target_type.is_some_and(|object| object.immune),
        is_building: target_is_building,
        can_c4: target_type
            .map(|object| object.can_c4)
            .unwrap_or(target_is_building),
    };
    let type_immune = target_type.is_some_and(|object| object.type_immune)
        && source.is_some_and(|source| {
            source.type_ref == target.type_ref && source.owner == target.owner
        });
    let bunker_blocked = if target_is_building && target.bunker_occupant.is_some() {
        // Linked Building branch is intentionally the inverse of the installed
        // non-Building branch in TechnoClass::ReceiveDamage.
        warhead.penetrates_bunker
    } else {
        target.bunker_link.installed_in().is_some() && !warhead.penetrates_bunker
    };
    let active_invulnerability = target.invulnerability.as_ref().filter(|_| {
        crate::sim::superweapon::invulnerability::is_invulnerable(
            target.invulnerability.as_ref(),
            current_tick as u32,
        )
    });
    let gates = damage::ImmunityInputs {
        ignore_defenses: receiver_flags.ignore_defenses,
        attacker_present: event.attacker_id != RAD_NO_ATTACKER,
        type_immune,
        // IC/FS checks original sign and precedes WarpingOut. Warping does not
        // share the negative/healing exemption; both honor ignoreDefenses.
        invulnerable: !receiver_flags.ignore_defenses
            && receiver_input >= 0
            && active_invulnerability.is_some(),
        warping_out: !receiver_flags.ignore_defenses
            && target
                .teleport_state
                .as_ref()
                .is_some_and(|state| state.warp_out_active()),
        bunker_blocked,
        radiation_immune: warhead.radiation
            && target_type.is_some_and(|object| object.immune_to_radiation),
        psychic_immune: warhead.psychic_damage
            && target_type.is_some_and(|object| object.immune_to_psionic_weapons),
        poison_immune: warhead.poison && target_type.is_some_and(|object| object.immune_to_poison),
        affects_allies: warhead.affects_allies,
        attacker_is_allied,
        source_house_is_allied,
        psychedelic: warhead.psychedelic,
        psionics_immune: target_type.is_some_and(|object| object.immune_to_psionics),
        target_is_building,
    };
    let defender_country_armor = houses.get(&target.owner).map_or(1.0, |house| {
        let difficulty_armor = rules.general.difficulty_armor[house.difficulty.table_index()];
        let country_name = house
            .country
            .map(|country| interner.resolve(country))
            .unwrap_or_else(|| interner.resolve(target.owner));
        let (country_armor, category_armor) = target_type
            .map(|object| rules.country_armor_factors(country_name, object))
            .unwrap_or((1.0, 1.0));
        let house_armor = difficulty_armor * country_armor;
        house_armor * category_armor
    });
    let defender_vet_armor = target_type
        .is_some_and(|object| {
            if target.veterancy >= ELITE_VETERANCY {
                object.veteran_stronger || object.elite_stronger
            } else {
                target.veterancy >= VETERAN_VETERANCY && object.veteran_stronger
            }
        })
        .then_some(rules.general.veteran_armor)
        .unwrap_or(1.0);
    let combat_mods = damage::CombatMods {
        defender_country_armor,
        defender_unit_armor: f64::from_bits(target.armor_multiplier.bits()),
        defender_vet_armor,
        ..damage::CombatMods::default()
    };
    // `arg6` is forwarded by the concrete ABI. Its only verified Unit-class
    // consumer gates a crew-survivor branch that this lifecycle does not
    // model; periodic radiation's true value is nevertheless retained on the
    // ordered call rather than erased at collection time.
    let _receiver_arg6 = receiver_flags.arg6;
    let outcome = damage::receive::receive_damage(
        receiver_input,
        warhead.cell_spread_f64,
        warhead.percent_at_max_f64,
        &warhead.verses_f64,
        &target_view,
        &combat_mods,
        &gates,
        distance_leptons,
        scenario_no_damage,
        rules.combat_damage.max_damage,
        f64::from(rules.general.condition_red),
    );
    let invulnerability_impact = outcome.invulnerability_impact_damage.map(|doubled_damage| {
        let flags = match active_invulnerability
            .expect("receiver gate retained active state")
            .kind
        {
            crate::sim::superweapon::invulnerability::InvulnKind::IronCurtain => 1,
            crate::sim::superweapon::invulnerability::InvulnKind::ForceShield => 6,
        };
        InvulnerabilityImpactEffect {
            target_id: target.stable_id,
            doubled_damage,
            warhead_ref: event.warhead_ref,
            coord: receiver_effect_coord(target, terrain),
            force_create: true,
            flags,
        }
    });
    Some(ResolvedReceiveDamage {
        outcome,
        invulnerability_impact,
    })
}

/// TechnoClass::ReceiveDamage builds the anger-node increment from the final
/// ObjectClass damage packet, not from raw weapon damage. The x87 keeps the
/// division result live, multiplies by the type's virtual cost, then Math__ftol
/// returns an i64 whose low dword is passed to HouseClass.
fn receiver_anger_delta(final_damage: i32, strength: i32, cost: i32) -> i32 {
    if strength == 0 {
        return 0;
    }
    let ratio = X87Chop53::div(
        X87Chop53::load_i32(final_damage),
        X87Chop53::load_i32(strength),
    )
    .expect("live Techno strength is nonzero");
    X87Chop53::ftol_i64(X87Chop53::mul(ratio, X87Chop53::load_i32(cost)))
        .expect("i32 damage/cost ratio fits native ftol i64") as i32
}

/// TechnoClass::ReceiveDamage PostMortem interpolation at 0x00701ED0.
/// `CellSpread` and `DelayKillAtMax` are native binary32 inputs; every other
/// operand is signed i32 and the two ftol results use their low dword.
fn postmortem_delay_duration(warhead: &WarheadType, distance_leptons: i32) -> i32 {
    let base = X87Chop53::load_i32(warhead.delay_kill_frames);
    let at_max = X87Chop53::load_f32(NativeF32Bits::from_bits(
        (warhead.delay_kill_at_max_f64 as f32).to_bits(),
    ))
    .expect("DelayKillAtMax parser retains a finite native f32");
    let slope = X87Chop53::sub(X87Chop53::mul(at_max, base), base);
    let spread = X87Chop53::load_f32(NativeF32Bits::from_bits(
        (warhead.cell_spread_f64 as f32).to_bits(),
    ))
    .expect("CellSpread parser retains a finite native f32");
    let spread_i32 =
        X87Chop53::ftol_i64(spread).expect("finite CellSpread converts through native ftol") as i32;
    let denominator = spread_i32.wrapping_shl(8);
    let Ok(slope_per_lepton) = X87Chop53::div(slope, X87Chop53::load_i32(denominator)) else {
        // Masked x87 divide-by-zero/non-finite conversion yields the integer
        // indefinite qword; Math__ftol returns its low dword, which is zero.
        return 0;
    };
    let delay = X87Chop53::add(
        base,
        X87Chop53::mul(X87Chop53::load_i32(distance_leptons), slope_per_lepton),
    );
    X87Chop53::ftol_i64(delay).unwrap_or(i64::MIN) as i32
}

fn postmortem_duration_for_event(
    event: &EntityDamageEvent,
    target: &GameEntity,
    rules: &RuleSet,
    interner: &StringInterner,
    outcome: damage::DamageOutcome,
) -> Option<i32> {
    if outcome.state != damage::DamageState::Dead || target.category != EntityCategory::Structure {
        return None;
    }
    let warhead = rules.warhead(interner.resolve(event.warhead_ref))?;
    let object = rules.object(interner.resolve(target.type_ref))?;
    (warhead.causes_delay_kill && object.eligible_for_delay_kill)
        .then(|| postmortem_delay_duration(warhead, event.distance_leptons.unwrap_or(0)))
}

/// Type vtable `+0xAC` value used by receiver anger feedback. Unit, Infantry,
/// and Aircraft types return `Cost=` directly; BuildingType applies its active
/// bundled-pad and `FreeUnit=` deductions.
fn receiver_type_value(target: &GameEntity, object: &ObjectType, rules: &RuleSet) -> i32 {
    if target.category != EntityCategory::Structure {
        return object.cost;
    }
    rules.building_actual_cost(object)
}

/// Exact active subset of HouseClass::UpdateAngerNodes @ 0x00504790.
///
/// Native stores one node for every other HouseClass in global creation order.
/// Rust keeps touched scores keyed by identity, but scans `house_order` for the
/// strict-greater winner so equal scores preserve the same earlier house.
fn update_receiver_anger_nodes(
    houses: &mut BTreeMap<InternedId, HouseState>,
    house_order: &[InternedId],
    alliances: &HouseAllianceMap,
    interner: &StringInterner,
    victim_owner: InternedId,
    source_owner: InternedId,
    delta: i32,
) {
    let source_is_registered = source_owner != victim_owner
        && house_order.iter().any(|&owner| owner == source_owner)
        && houses.contains_key(&source_owner);
    if source_is_registered && let Some(victim) = houses.get_mut(&victim_owner) {
        // Native pre-creates every other-house node. In the sparse Rust form,
        // an absent key therefore already means native score zero: a zero
        // update must not materialize new serialized/hashed state. Once a key
        // exists it remains represented even when its score is zero.
        if delta != 0 || victim.grudge_scores.contains_key(&source_owner) {
            let score = victim.grudge_scores.entry(source_owner).or_insert(0);
            *score = score.wrapping_add(delta);
        }
    }

    let Some(victim) = houses.get(&victim_owner) else {
        return;
    };
    let mut best_score = 0;
    let mut best_house = None;
    for &candidate_id in house_order {
        if candidate_id == victim_owner {
            continue;
        }
        let Some(candidate) = houses.get(&candidate_id) else {
            continue;
        };
        let score = victim
            .grudge_scores
            .get(&candidate_id)
            .copied()
            .unwrap_or(0);
        if score > best_score
            && !candidate.is_defeated
            && !crate::map::houses::is_allied_with(
                alliances,
                interner.resolve(victim_owner),
                interner.resolve(candidate_id),
            )
        {
            best_score = score;
            best_house = Some(candidate_id);
        }
    }
    if let Some(victim) = houses.get_mut(&victim_owner) {
        victim.enemy_house = best_house;
    }
}

fn has_active_area_invulnerability(entity: &GameEntity, current_tick: u64) -> bool {
    // Every GameEntity is a TechnoClass-derived object, so the native
    // AbstractFlags +0x14 bit-0 identity test is inherent in this store. The
    // virtual +0x160 result is the existing passive IC/FS timer predicate.
    crate::sim::superweapon::invulnerability::is_invulnerable(
        entity.invulnerability.as_ref(),
        current_tick as u32,
    )
}

fn near_center_ic_isolation_armed(
    damage_events: &[EntityDamageEvent],
    entities: &EntityStore,
    current_tick: u64,
) -> bool {
    damage_events.iter().any(|event| {
        event.near_center_ic_isolation_eligible
            && event.distance_leptons.is_some_and(|distance| distance < 85)
            && entities.get(event.target_id).is_some_and(|target| {
                has_active_area_invulnerability(target, current_tick)
                    && target.invulnerability.as_ref().is_some_and(|state| {
                        state.kind
                            == crate::sim::superweapon::invulnerability::InvulnKind::IronCurtain
                    })
            })
    })
}

fn area_near_center_ic_isolation_armed(
    receivers: &[combat_aoe::AreaDamageReceiver],
    entities: &EntityStore,
    current_tick: u64,
) -> bool {
    receivers.iter().any(|receiver| {
        let combat_aoe::AreaDamageReceiver::Entity(event) = receiver else {
            return false;
        };
        event.near_center_ic_isolation_eligible
            && event.distance_leptons.is_some_and(|distance| distance < 85)
            && entities.get(event.target_id).is_some_and(|target| {
                has_active_area_invulnerability(target, current_tick)
                    && target.invulnerability.as_ref().is_some_and(|state| {
                        state.kind
                            == crate::sim::superweapon::invulnerability::InvulnKind::IronCurtain
                    })
            })
    })
}

/// Commit one complete native Apply_area_damage receiver vector. Collection is
/// finished before this function runs, so the Iron Curtain pre-scan can affect
/// records that precede the arming Techno while dispatch itself remains in the
/// captured CellClass/object-list order.
#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(crate) fn commit_area_damage_receivers(
    receivers: &[combat_aoe::AreaDamageReceiver],
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    houses: &mut BTreeMap<InternedId, HouseState>,
    house_order: &[InternedId],
    alliances: &HouseAllianceMap,
    main_rng: &mut SimRng,
    scenario_rng: &mut SimRng,
    handled_deaths: &mut Vec<u64>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    terrain_area_state: Option<&mut TerrainAreaState>,
    current_tick: u64,
    inline_hooks: &mut Option<&mut dyn CombatInlineHooks>,
    sound_sink: &mut Option<&mut Vec<SimSoundEvent>>,
) -> (DeathEffects, Vec<UnderAttackEvent>) {
    // Test-convenience wrapper: resolve rule handles the way sim init does.
    let handles = Some(crate::sim::type_handle_table::ResolvedRuleHandles::resolve(rules, interner));
    commit_area_damage_receivers_with_scenario(
        receivers,
        entities,
        occupancy,
        rules,
        interner,
        handles,
        houses,
        house_order,
        alliances,
        main_rng,
        scenario_rng,
        handled_deaths,
        resource_nodes,
        overlay_grid,
        overlay_registry,
        terrain,
        terrain_area_state,
        false,
        current_tick,
        inline_hooks,
        sound_sink,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn commit_area_damage_receivers_with_scenario(
    receivers: &[combat_aoe::AreaDamageReceiver],
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    handles: Option<crate::sim::type_handle_table::ResolvedRuleHandles>,
    houses: &mut BTreeMap<InternedId, HouseState>,
    house_order: &[InternedId],
    alliances: &HouseAllianceMap,
    main_rng: &mut SimRng,
    scenario_rng: &mut SimRng,
    handled_deaths: &mut Vec<u64>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    mut overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    mut terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    mut terrain_area_state: Option<&mut TerrainAreaState>,
    scenario_no_damage: bool,
    current_tick: u64,
    inline_hooks: &mut Option<&mut dyn CombatInlineHooks>,
    sound_sink: &mut Option<&mut Vec<SimSoundEvent>>,
) -> (DeathEffects, Vec<UnderAttackEvent>) {
    let isolation_armed = area_near_center_ic_isolation_armed(receivers, entities, current_tick);
    let mut effects = DeathEffects::default();
    let mut under_attack_events = Vec::new();

    for receiver in receivers {
        match *receiver {
            combat_aoe::AreaDamageReceiver::Entity(event) => {
                let (nested, mut pings) = commit_damage_events_with_isolation(
                    std::slice::from_ref(&event),
                    Some(isolation_armed),
                    entities,
                    occupancy,
                    rules,
                    interner,
                    handles,
                    houses,
                    house_order,
                    alliances,
                    main_rng,
                    scenario_rng,
                    handled_deaths,
                    resource_nodes,
                    overlay_grid.as_deref_mut(),
                    overlay_registry,
                    terrain.as_deref_mut(),
                    terrain_area_state.as_deref_mut(),
                    scenario_no_damage,
                    current_tick,
                    inline_hooks,
                    sound_sink,
                );
                effects.append(nested);
                under_attack_events.append(&mut pings);
            }
            combat_aoe::AreaDamageReceiver::Terrain(event) => {
                if isolation_armed && event.near_center_ic_isolation_eligible {
                    continue;
                }
                let Some(state) = terrain_area_state.as_deref_mut() else {
                    continue;
                };
                let Some(warhead) = rules.warhead(interner.resolve(event.warhead_ref)).cloned()
                else {
                    continue;
                };
                let receive = state.receive_area_damage_with_scenario(
                    event.stable_id,
                    (event.rx, event.ry),
                    event.damage,
                    event.distance_leptons,
                    &warhead,
                    rules,
                    interner,
                    scenario_no_damage,
                );
                let TerrainAreaReceiveResult::Lethal(lethal) = receive else {
                    continue;
                };

                if lethal.spawns_tiberium
                    && let Some(c4_warhead) = rules.warhead(&rules.bridge_warheads.c4_name).cloned()
                {
                    let c4_id = interner.intern(&c4_warhead.id);
                    let impact_z = terrain
                        .as_deref()
                        .and_then(|grid| grid.cell(lethal.cell.0, lethal.cell.1))
                        .map_or(0, |cell| i32::from(cell.level));
                    let ore_amount = if scenario_no_damage {
                        None
                    } else {
                        tiberium_reduction_amount(100, true, &c4_warhead)
                    };
                    let terrain_collection = combat_aoe::TerrainCollectionView {
                        objects: state.terrain_objects(),
                        cells: state.terrain_object_cells(),
                    };
                    let aoe = {
                        let mut ore_prelude = ore_amount.map(|amount| CombatTiberiumCellPrelude {
                            amount,
                            deferred: &mut effects.tiberium_reduction_requests,
                            inline_hooks,
                            rules,
                            resource_nodes,
                            terrain_area_state: Some(&*state),
                        });
                        combat_aoe::apply_aoe_damage_with_terrain_and_scenario(
                            entities,
                            lethal.cell.0,
                            lethal.cell.1,
                            100,
                            &c4_warhead,
                            rules,
                            interner,
                            handles,
                            (RAD_NO_ATTACKER, None, c4_id),
                            combat_aoe::AoELayerContext {
                                occupancy: Some(&*occupancy),
                                terrain: terrain.as_deref_mut(),
                                overlay_grid: overlay_grid.as_deref_mut(),
                                overlay_registry,
                                scenario_rng: Some(&mut *scenario_rng),
                                air_impact: None,
                                impact_z,
                            },
                            Some(terrain_collection),
                            scenario_no_damage,
                            ore_prelude
                                .as_mut()
                                .map(|prelude| prelude as &mut dyn combat_aoe::AoECellPrelude),
                        )
                    };
                    effects.wall_mutations.extend(aoe.wall_mutations);
                    effects
                        .cell_target_detaches
                        .extend(aoe.cell_target_detaches);
                    let (nested, mut pings) = commit_area_damage_receivers_with_scenario(
                        &aoe.receivers,
                        entities,
                        occupancy,
                        rules,
                        interner,
                        handles,
                        houses,
                        house_order,
                        alliances,
                        main_rng,
                        scenario_rng,
                        handled_deaths,
                        resource_nodes,
                        overlay_grid.as_deref_mut(),
                        overlay_registry,
                        terrain.as_deref_mut(),
                        Some(&mut *state),
                        scenario_no_damage,
                        current_tick,
                        inline_hooks,
                        sound_sink,
                    );
                    effects.append(nested);
                    under_attack_events.append(&mut pings);
                }

                let _ = state.finalize_lethal(lethal, terrain.as_deref_mut());
            }
        }
    }

    (effects, under_attack_events)
}

/// Commit one native-order damage slice and synchronously enter the death
/// helper at each live-to-dead transition. Object storage/removal remains the
/// world-owned deferred lifecycle handoff; only ReceiveDamage consequences are
/// recursive here.
#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(crate) fn commit_damage_events(
    damage_events: &[EntityDamageEvent],
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    houses: &mut BTreeMap<InternedId, HouseState>,
    house_order: &[InternedId],
    alliances: &HouseAllianceMap,
    main_rng: &mut SimRng,
    scenario_rng: &mut SimRng,
    handled_deaths: &mut Vec<u64>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    current_tick: u64,
    inline_hooks: &mut Option<&mut dyn CombatInlineHooks>,
    sound_sink: &mut Option<&mut Vec<SimSoundEvent>>,
) -> (DeathEffects, Vec<UnderAttackEvent>) {
    // Test-convenience wrapper: resolve rule handles the way sim init does.
    let handles = Some(crate::sim::type_handle_table::ResolvedRuleHandles::resolve(rules, interner));
    commit_damage_events_with_isolation(
        damage_events,
        None,
        entities,
        occupancy,
        rules,
        interner,
        handles,
        houses,
        house_order,
        alliances,
        main_rng,
        scenario_rng,
        handled_deaths,
        resource_nodes,
        overlay_grid,
        overlay_registry,
        terrain,
        None,
        false,
        current_tick,
        inline_hooks,
        sound_sink,
    )
}

#[allow(clippy::too_many_arguments)]
fn commit_damage_events_with_isolation(
    damage_events: &[EntityDamageEvent],
    near_center_ic_isolation_override: Option<bool>,
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    handles: Option<crate::sim::type_handle_table::ResolvedRuleHandles>,
    houses: &mut BTreeMap<InternedId, HouseState>,
    house_order: &[InternedId],
    alliances: &HouseAllianceMap,
    main_rng: &mut SimRng,
    scenario_rng: &mut SimRng,
    handled_deaths: &mut Vec<u64>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    mut overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    mut terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    mut terrain_area_state: Option<&mut TerrainAreaState>,
    scenario_no_damage: bool,
    current_tick: u64,
    inline_hooks: &mut Option<&mut dyn CombatInlineHooks>,
    sound_sink: &mut Option<&mut Vec<SimSoundEvent>>,
) -> (DeathEffects, Vec<UnderAttackEvent>) {
    let mut death = DeathEffects::default();
    let mut under_attack_events = Vec::new();
    // Apply_area_damage finishes collecting its fixed target/distance records
    // before dispatch. A qualifying near-center Iron Curtain therefore
    // isolates the entire eligible transaction, including records collected
    // before the arming record. The per-record check below remains live so an
    // earlier receiver or nested death effect can change later protection.
    let near_center_ic_isolation = near_center_ic_isolation_override
        .unwrap_or_else(|| near_center_ic_isolation_armed(damage_events, entities, current_tick));

    for event in damage_events {
        if near_center_ic_isolation
            && event.near_center_ic_isolation_eligible
            && !entities
                .get(event.target_id)
                .is_some_and(|target| has_active_area_invulnerability(target, current_tick))
        {
            continue;
        }
        let target_id = event.target_id;
        let attacker_id = event.attacker_id;
        // ReceiveDamage carries sourceHouse separately from the source object.
        // Area records snapshot it at detonation; legacy precomputed records
        // retain the former live-source lookup. Periodic radiation supplies
        // both null source object and null source house explicitly.
        let attacker_owner: Option<InternedId> = event.source_house.or_else(|| {
            (attacker_id != RAD_NO_ATTACKER)
                .then(|| entities.get(attacker_id).map(|attacker| attacker.owner))
                .flatten()
        });
        // UpdateAngerNodes reads source->Owner directly; the separately
        // captured source-house ABI argument is not used by this callback.
        let live_source_owner = (attacker_id != RAD_NO_ATTACKER)
            .then(|| entities.get(attacker_id).map(|source| source.owner))
            .flatten();
        let receiver_outcome = event.distance_leptons.map(|_| {
            resolve_receive_damage(
                event,
                entities,
                rules,
                interner,
                houses,
                alliances,
                scenario_no_damage,
                current_tick,
                terrain.as_deref(),
            )
        });
        if let Some(effect) = receiver_outcome
            .flatten()
            .and_then(|resolved| resolved.invulnerability_impact)
        {
            death.invulnerability_impact_effects.push(effect);
        }
        let postmortem_duration = receiver_outcome.flatten().and_then(|resolved| {
            let target = entities.get(target_id)?;
            postmortem_duration_for_event(event, target, rules, interner, resolved.outcome)
        });
        let mut became_fatal = false;
        let mut reached_exact_zero = false;
        let mut postmortem_candidate = None;
        let mut fatal_category = EntityCategory::Unit;
        let mut positive_postlude: Option<(u16, bool, bool)> = None;
        let mut synchronous_retaliation = false;
        let mut smoke_maintenance: Option<(EntityCategory, damage::DamageState)> = None;
        let mut healing_only = false;
        let mut latch_hostile_hit = false;
        let mut threat_feedback: Option<(InternedId, InternedId, i32, i32, i32)> = None;
        if let Some(target) = entities.get_mut(target_id) {
            if event.distance_leptons.is_none()
                && crate::sim::superweapon::invulnerability::is_invulnerable(
                    target.invulnerability.as_ref(),
                    current_tick as u32,
                )
            {
                // Damage fully nullified by IronCurtain/ForceShield.
                // Flash-effect spawn deferred (see design doc Open Questions).
                if attacker_id != RAD_NO_ATTACKER {
                    target.last_attacker_id = Some(attacker_id);
                }
                continue;
            }

            let receive_outcome = match receiver_outcome {
                Some(Some(resolved)) => Some(resolved.outcome),
                Some(None) => continue,
                None => None,
            };
            if let Some(value) = receive_outcome.and_then(|outcome| outcome.psychedelic_value) {
                // TechnoClass writes the signed kernel result first. The
                // first inactive->active transition then runs its callbacks
                // in order: optional team-member detach (not represented on
                // GameEntity), archived target clear, deferred Hunt queue.
                // Passenger cargo is unrelated and remains intact.
                target.berserk.timer = value;
                if !target.berserk.active {
                    target.berserk.active = true;
                    represented_assign_target(target, None);
                    queue_entity_mission_deferred(target, MissionId::from_known(MissionType::Hunt));
                }
                continue;
            }
            let reached_survivor_postlude =
                receive_outcome.is_some_and(|outcome| outcome.reached_survivor_postlude);
            let receive_state = receive_outcome.map(|outcome| outcome.state);
            // TechnoClass's persistent hostile-hit byte is written in the
            // shared surviving post-Object tail. The source object must be
            // non-null, and alliance direction is target owner -> captured
            // source house. This is deliberately separate from retaliation's
            // transient `last_attacker_id`.
            let hostile_source = attacker_id != RAD_NO_ATTACKER
                && attacker_owner.is_some_and(|source_owner| {
                    !crate::map::houses::is_allied_with(
                        alliances,
                        interner.resolve(target.owner),
                        interner.resolve(source_owner),
                    )
                });
            let resolved_damage = receive_outcome.map_or(event.damage, |outcome| outcome.hp_delta);
            if reached_survivor_postlude
                && let Some(source_owner) = live_source_owner
                && let Some(final_damage) =
                    receive_outcome.and_then(|outcome| outcome.post_object_damage)
                && let Some(target_type) = rules.object(interner.resolve(target.type_ref))
            {
                threat_feedback = Some((
                    target.owner,
                    source_owner,
                    final_damage,
                    target_type.strength,
                    receiver_type_value(target, target_type, rules),
                ));
            }
            if resolved_damage == 0 {
                if reached_survivor_postlude && target.health.current > 0 && hostile_source {
                    latch_hostile_hit = true;
                }
                if reached_survivor_postlude && target.health.current > 0 {
                    smoke_maintenance = receive_state.map(|state| (target.category, state));
                }
                synchronous_retaliation = event.distance_leptons.is_some()
                    && event.damage >= 0
                    && reached_survivor_postlude
                    && target.health.current > 0;
            } else if resolved_damage < 0 {
                let healing = resolved_damage.unsigned_abs().min(u32::from(u16::MAX)) as u16;
                target.health.current = target
                    .health
                    .current
                    .saturating_add(healing)
                    .min(target.health.max);
                if reached_survivor_postlude && target.health.current > 0 && hostile_source {
                    latch_hostile_hit = true;
                }
                target.refresh_building_damage_state_gate(rules.general.condition_yellow_x1000);
                if reached_survivor_postlude && target.health.current > 0 {
                    smoke_maintenance = receive_state.map(|state| (target.category, state));
                }
                healing_only = true;
            } else {
                let damage = resolved_damage.min(i32::from(u16::MAX)) as u16;
                let was_alive = target.health.current > 0;
                target.health.current = target.health.current.saturating_sub(damage);
                target.refresh_building_damage_state_gate(rules.general.condition_yellow_x1000);
                became_fatal = was_alive && target.health.current == 0;
                reached_exact_zero = became_fatal;
                if became_fatal {
                    fatal_category = target.category;
                }
                synchronous_retaliation = event.distance_leptons.is_some()
                    && event.damage >= 0
                    && reached_survivor_postlude
                    && target.health.current > 0;
                if reached_survivor_postlude && target.health.current > 0 {
                    if hostile_source {
                        latch_hostile_hit = true;
                    }
                    smoke_maintenance = receive_state.map(|state| (target.category, state));
                }
                positive_postlude = Some((damage, reached_survivor_postlude, hostile_source));
                if became_fatal && let Some(duration_frames) = postmortem_duration {
                    // Do not restore here. Native first executes ObjectClass's
                    // exact-zero kill/Destroy callbacks, then victim-house anger,
                    // and only afterward arms the timer and writes Alive/HP=1.
                    postmortem_candidate = Some(duration_frames);
                    positive_postlude = None;
                    synchronous_retaliation = false;
                    smoke_maintenance = None;
                    latch_hostile_hit = false;
                }
            }
        }

        if reached_exact_zero && let Some(target) = entities.get_mut(target_id) {
            // ObjectClass routes its kill callback while Health is exactly zero,
            // before Destroy's reference notification and before TechnoClass's
            // victim-house anger callback.
            capture_kill_credit(target, attacker_owner, rules, interner);
        }
        if postmortem_candidate.is_some()
            && let Some(hook) = inline_hooks.as_deref_mut()
        {
            hook.fatal_lifecycle(
                rules,
                FatalLifecycleStage::PostMortemExactZero {
                    killer_owner: attacker_owner,
                },
                target_id,
                fatal_category,
                entities,
                occupancy,
                interner,
                scenario_rng,
                terrain.as_deref(),
                terrain_area_state.as_deref_mut(),
                sound_sink.as_deref_mut(),
            );
        }
        if let Some((victim_owner, source_owner, final_damage, strength, cost)) = threat_feedback {
            let delta = receiver_anger_delta(final_damage, strength, cost);
            update_receiver_anger_nodes(
                houses,
                house_order,
                alliances,
                interner,
                victim_owner,
                source_owner,
                delta,
            );
            #[cfg(test)]
            death
                .receiver_stage_trace
                .push(ReceiverStageTrace::HouseThreat { target_id, delta });
        }
        if let Some(duration_frames) = postmortem_candidate {
            let current_frame = current_tick as u32 as i32;
            let target = entities
                .get_mut(target_id)
                .expect("PostMortem exact-zero callbacks retain the represented target");
            // RecordKill already consumed this exact-zero attribution in the
            // synchronous PostMortem hook. Native retains no killer on the
            // restored object: a fresh null-source timer expiry must stay
            // uncredited, while a later sourced lethal hit captures anew.
            target.killed_by = None;
            target.kill_award_points = 0;
            let replace = target
                .pending_c4_detonation
                .is_none_or(|pending| duration_frames < pending.remaining_at(current_frame));
            if replace {
                let retained_source = target
                    .pending_c4_detonation
                    .and_then(|pending| pending.source_entity_id);
                target.pending_c4_detonation = Some(crate::sim::components::PendingC4Detonation {
                    start_frame: current_frame,
                    duration_frames,
                    source_entity_id: retained_source,
                });
            }
            target.lifecycle.object_alive = true;
            target.health.current = 1;
            target.dying = false;
            target.refresh_building_damage_state_gate(rules.general.condition_yellow_x1000);
            #[cfg(test)]
            death
                .receiver_stage_trace
                .push(ReceiverStageTrace::PostMortem { target_id });
            continue;
        }
        if latch_hostile_hit && let Some(target) = entities.get_mut(target_id) {
            target.was_attacked_by_enemy = true;
        }

        if let Some((category, state)) = smoke_maintenance
            && let Some(hook) = inline_hooks.as_deref_mut()
        {
            hook.fatal_lifecycle(
                rules,
                FatalLifecycleStage::MaintainDamageSmoke { state },
                target_id,
                category,
                entities,
                occupancy,
                interner,
                scenario_rng,
                terrain.as_deref(),
                terrain_area_state.as_deref_mut(),
                sound_sink.as_deref_mut(),
            );
        }
        if healing_only {
            continue;
        }

        if let Some((damage, _reached_survivor_postlude, _hostile_source)) = positive_postlude {
            // InfantryClass's concrete receiver dispatches Scatter only for a
            // surviving result state (1..=3), after HP has changed and before
            // fear or the shared Techno postlude. The attacker coordinate is
            // read while the source is still represented, including nested
            // DeathWeapon receiver recursion.
            let surviving_infantry_result = receiver_outcome.flatten().is_some_and(|resolved| {
                matches!(
                    resolved.outcome.state,
                    damage::DamageState::Damaged
                        | damage::DamageState::Yellow
                        | damage::DamageState::Red
                )
            });
            let attacker_coord = (attacker_id != RAD_NO_ATTACKER)
                .then(|| entities.get(attacker_id))
                .flatten()
                .map(|attacker| {
                    (
                        i32::from(attacker.position.rx)
                            .wrapping_mul(256)
                            .wrapping_add(attacker.position.sub_x.to_num::<i32>()),
                        i32::from(attacker.position.ry)
                            .wrapping_mul(256)
                            .wrapping_add(attacker.position.sub_y.to_num::<i32>()),
                    )
                });
            let scatter = if surviving_infantry_result {
                attacker_coord.and_then(|attacker_coord| {
                    let target = entities.get(target_id)?;
                    let target_type = rules.object(interner.resolve(target.type_ref));
                    let infantry_is_fraidycat = target_type.is_some_and(|object| object.fraidycat);
                    let has_scatter_ability = target_type.is_some_and(|object| {
                        (target.veterancy >= VETERAN_VETERANCY && object.veteran_scatter)
                            || (target.veterancy >= ELITE_VETERANCY && object.elite_scatter)
                    });
                    crate::sim::movement::bump_crush::select_infantry_damage_scatter(
                        target,
                        attacker_coord,
                        terrain.as_deref(),
                        occupancy,
                        rules,
                        houses
                            .get(&target.owner)
                            .is_some_and(|house| house.is_human),
                        infantry_is_fraidycat,
                        has_scatter_ability,
                        scenario_rng,
                    )
                })
            } else {
                None
            };
            if let Some(scatter) = scatter {
                if let Some(target) = entities.get_mut(target_id) {
                    queue_entity_mission_deferred(target, MissionId::from_known(MissionType::Move));
                    crate::sim::mission::concrete_effects::represented_assign_destination_mode_one(
                        target,
                        Some(crate::sim::components::NavTargetRef::cell(
                            scatter.destination.0,
                            scatter.destination.1,
                        )),
                    );
                }
                let _ = crate::sim::movement::issue_direct_move(
                    entities,
                    target_id,
                    scatter.destination,
                    scatter.speed,
                );
            }

            let Some(target) = entities.get_mut(target_id) else {
                continue;
            };
            if let Some(obj) = rules.object(interner.resolve(target.type_ref)) {
                infantry::apply_fear_from_damage(
                    obj,
                    target,
                    damage,
                    true,
                    rules.general.condition_red_x1000,
                    rules.general.condition_yellow_x1000,
                );
            }
            if damage > 0 && attacker_owner.is_some_and(|ao| ao != target.owner) {
                let miner = target.miner.is_some();
                if miner || target.category == EntityCategory::Structure {
                    under_attack_events.push(UnderAttackEvent {
                        rx: target.position.rx,
                        ry: target.position.ry,
                        owner: target.owner,
                        miner,
                    });
                }
            }
            // Legacy precomputed callers have not yet entered the authoritative
            // receiver ABI, so retain their Phase-6 handoff. Ordered area/direct
            // receiver hits make their retaliation decision synchronously below.
            if attacker_id != RAD_NO_ATTACKER && event.distance_leptons.is_none() {
                target.last_attacker_id = Some(attacker_id);
            }
        }

        if synchronous_retaliation && attacker_id != RAD_NO_ATTACKER {
            #[cfg(test)]
            death
                .receiver_stage_trace
                .push(ReceiverStageTrace::ShouldRetaliate { target_id });
            if combat_targeting::should_retaliate_from_damage(
                entities,
                target_id,
                attacker_id,
                rules,
                interner,
                houses,
                alliances,
                terrain.as_deref(),
            ) {
                override_mission_on_damage_response(entities, target_id, attacker_id);
            }
        }

        if became_fatal {
            if let Some(hook) = inline_hooks.as_deref_mut() {
                hook.fatal_lifecycle(
                    rules,
                    FatalLifecycleStage::BeforeDeathEffects,
                    target_id,
                    fatal_category,
                    entities,
                    occupancy,
                    interner,
                    scenario_rng,
                    terrain.as_deref(),
                    terrain_area_state.as_deref_mut(),
                    sound_sink.as_deref_mut(),
                );
            }
            let mut nested = handle_entity_deaths(
                entities,
                occupancy,
                rules,
                interner,
                handles,
                houses,
                house_order,
                alliances,
                main_rng,
                scenario_rng,
                handled_deaths,
                &[target_id],
                std::slice::from_ref(event),
                resource_nodes,
                overlay_grid.as_deref_mut(),
                overlay_registry,
                terrain.as_deref_mut(),
                &mut terrain_area_state,
                scenario_no_damage,
                current_tick,
                inline_hooks,
                sound_sink,
            );
            under_attack_events.append(&mut nested.under_attack_events);
            if matches!(
                fatal_category,
                EntityCategory::Unit | EntityCategory::Structure
            ) {
                let mut lifecycle_handled = false;
                if let Some(hook) = inline_hooks.as_deref_mut() {
                    hook.fatal_lifecycle(
                        rules,
                        FatalLifecycleStage::AfterDeathEffects,
                        target_id,
                        fatal_category,
                        entities,
                        occupancy,
                        interner,
                        scenario_rng,
                        terrain.as_deref(),
                        terrain_area_state.as_deref_mut(),
                        sound_sink.as_deref_mut(),
                    );
                    lifecycle_handled = true;
                }
                if lifecycle_handled {
                    nested
                        .immediate_uninit_ids
                        .retain(|&dead_id| dead_id != target_id);
                }
            }
            death.append(nested);
        }
    }

    (death, under_attack_events)
}

/// Keep physical death consequences in the emission trace at the exact source
/// boundary while retaining lifecycle requests for the tick's deferred handoff.
fn absorb_inline_death_effects(
    out: &mut CombatEmit,
    lifecycle: &mut DeathEffects,
    mut death: DeathEffects,
) {
    out.bridge_damage_events
        .append(&mut death.bridge_damage_events);
    out.wall_mutations.append(&mut death.wall_mutations);
    out.cell_target_detaches
        .append(&mut death.cell_target_detaches);
    out.tiberium_reduction_requests
        .append(&mut death.tiberium_reduction_requests);
    out.explosion_effects.append(&mut death.explosion_effects);
    out.smudge_spawn_requests
        .append(&mut death.smudge_spawn_requests);
    out.rad_detonations.append(&mut death.rad_detonations);
    lifecycle.append(death);
}

/// Transient per-tick bag of the Phase-2 fire-emission outputs. Bundles the
/// emit vectors so the per-attacker fire body (`resolve_attacker_fire`) can push
/// through one `&mut` handle. Never stored on `Simulation`, never serialized,
/// never hashed — destructured back into the named locals after the Phase-2 loop.
#[derive(Default)]
pub(crate) struct CombatEmit {
    /// Persistent ordinary bullets admitted by accepted weapon fire. The world
    /// inserts them only after this frame's BulletClass pass has completed.
    pub(crate) projectile_spawns: Vec<ProjectileSpawn>,
    /// Native-order ReceiveDamage calls, including raw area records.
    pub(crate) damage_events: Vec<combat_aoe::AreaDamageReceiver>,
    /// Radiation-emitting detonations (weapon RadLevel > 0), folded into
    /// `RadiationState` before the damage-application phase.
    pub(crate) rad_detonations: Vec<crate::sim::radiation::RadDetonation>,
    pub(crate) remove_attack: Vec<u64>,
    /// (attacker_id, new_target_id)
    pub(crate) retarget_events: Vec<(u64, u64)>,
    pub(crate) fire_events: Vec<SimFireEvent>,
    pub(crate) reveal_events: Vec<RevealEvent>,
    pub(crate) bridge_damage_events: Vec<BridgeDamageEvent>,
    pub(crate) wall_mutations: Vec<WallMutation>,
    pub(crate) cell_target_detaches: Vec<combat_aoe::CellTargetDetach>,
    pub(crate) tiberium_reduction_requests: Vec<TiberiumReductionRequest>,
    pub(crate) explosion_effects: Vec<ExplosionEffect>,
    pub(crate) smudge_spawn_requests: Vec<SmudgeSpawnRequest>,
    /// (id, burst_rem, burst_delay, rof_cd)
    pub(crate) burst_updates: Vec<(u64, u8, u8, u16)>,
    /// aircraft that fired this tick
    pub(crate) ammo_deduct: Vec<u64>,
    /// building IDs to advance fire index
    pub(crate) garrison_advance: Vec<u64>,
    pub(crate) pending_infantry_updates: Vec<(u64, Option<PendingInfantryFire>)>,
    pub(crate) animation_switches: Vec<(u64, SequenceKind)>,
    /// Native `CurrentWeaponNumber` writes emitted by live weapon selection.
    /// The per-attacker host commits these before that attack's receivers run.
    pub(crate) current_weapon_updates: Vec<(u64, u8, InternedId)>,
    /// (unit_id, desired 16-bit barrel destination) — captured at Phase-2
    /// entry before current-frame attacker damage; that Unit's own explicit
    /// retarget/remove may replace it. Applied post-batch by
    /// `unit_post::apply_unit_facing`.
    pub(crate) unit_facing: Vec<(u64, u16)>,
    /// (parent_id, target) — a `Spawner=yes` weapon reached its fire point.
    /// gamemd's `Fire_At` hands the target to the parent's `SpawnManager` and
    /// returns NULL, so no bullet, damage or rearm follows.
    pub(crate) spawn_target_updates: Vec<(u64, TargetKind)>,
}

fn projectile_impact_cell(impact: ProjectileCoord) -> (u16, u16, SimFixed, SimFixed, i32) {
    let rx = impact.x.div_euclid(256).clamp(0, i32::from(u16::MAX)) as u16;
    let ry = impact.y.div_euclid(256).clamp(0, i32::from(u16::MAX)) as u16;
    (
        rx,
        ry,
        SimFixed::from_num(impact.x.rem_euclid(256)),
        SimFixed::from_num(impact.y.rem_euclid(256)),
        impact.z,
    )
}

fn shrapnel_launch_velocity(
    origin: ProjectileCoord,
    target: ProjectileCoord,
    speed: i32,
) -> ProjectileVelocity {
    let dx = f64::from(target.x - origin.x);
    let dy = f64::from(target.y - origin.y);
    let length = dx.hypot(dy);
    let angle = 0.7853262558535721_f64;
    let horizontal = f64::from(speed) * angle.cos();
    let (unit_x, unit_y) = if length == 0.0 {
        (1.0, 0.0)
    } else {
        (dx / length, dy / length)
    };
    ProjectileVelocity::new(
        (unit_x * horizontal).round_ties_even() as i32,
        (unit_y * horizontal).round_ties_even() as i32,
        (f64::from(speed) * angle.sin()).round_ties_even() as i32,
    )
}

fn emit_projectile_shrapnel(
    detonation: &ProjectileDetonation,
    entities: &EntityStore,
    occupancy: &OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
    bridge_state: Option<&BridgeRuntimeState>,
    house_alliances: &HouseAllianceMap,
    scenario_rng: &mut SimRng,
    out: &mut CombatEmit,
) {
    let Some(parent_weapon) = rules.weapon(interner.resolve(detonation.payload.weapon)) else {
        return;
    };
    let Some(parent_projectile) = parent_weapon
        .projectile
        .as_deref()
        .and_then(|name| rules.projectile(name))
    else {
        return;
    };
    let Some(child_weapon_name) = parent_projectile.shrapnel_weapon.as_deref() else {
        return;
    };
    let Some(child_weapon) = rules.weapon(child_weapon_name) else {
        return;
    };
    let Some(child_projectile) = child_weapon
        .projectile
        .as_deref()
        .and_then(|name| rules.projectile(name))
    else {
        log::debug!(
            "Projectile {} shrapnel skipped: child projectile constructor unavailable",
            detonation.projectile_id
        );
        return;
    };
    let Some(child_warhead_name) = child_weapon.warhead.as_deref() else {
        return;
    };

    let target_position = match detonation.target {
        ProjectileTarget::Entity(id) => entities.get(id).map(|entity| {
            ProjectileCoord::new(
                i32::from(entity.position.rx) * 256 + entity.position.sub_x.to_num::<i32>(),
                i32::from(entity.position.ry) * 256 + entity.position.sub_y.to_num::<i32>(),
                i32::from(entity.position.z) * crate::util::lepton::GROUND_LEVEL_HEIGHT_LEPTONS,
            )
        }),
        ProjectileTarget::Cell { rx, ry } => Some(crate::sim::projectile::cell_target_coord(
            terrain,
            bridge_state,
            rx,
            ry,
        )),
        ProjectileTarget::None => Some(ProjectileCoord::new(0, 0, 0)),
    };
    let distance_cells = target_position.map_or(0, |target| {
        let dx = i64::from(target.x - detonation.impact.x);
        let dy = i64::from(target.y - detonation.impact.y);
        let dz = i64::from(target.z - detonation.impact.z);
        (dx.saturating_mul(dx)
            .saturating_add(dy.saturating_mul(dy))
            .saturating_add(dz.saturating_mul(dz)))
        .isqrt()
        .saturating_div(256) as i32
    });
    let count = projectile_shrapnel_count(
        parent_projectile.shrapnel_count,
        entities.get(detonation.source_id).is_some(),
        distance_cells,
    );
    if count == 0 {
        return;
    }

    let center_rx = detonation.impact.x / 256;
    let center_ry = detonation.impact.y / 256;
    let source_owner = entities
        .get(detonation.source_id)
        .map(|source| source.owner);
    let mut targets = Vec::with_capacity(count as usize);
    let scan_radius = child_weapon.range.to_num::<i32>().max(0);
    for &(dx, dy) in self::cell_spread::splash_cells(SimFixed::from_num(scan_radius))
        .iter()
        .skip(1)
    {
        if targets.len() == count as usize {
            break;
        }
        let rx = center_rx + i32::from(dx);
        let ry = center_ry + i32::from(dy);
        let (Ok(rx), Ok(ry)) = (u16::try_from(rx), u16::try_from(ry)) else {
            continue;
        };
        let Some(target_id) = occupancy
            .get(rx, ry)
            .and_then(|cell| {
                cell.iter_layer(crate::sim::movement::locomotor::MovementLayer::Ground)
                    .next()
            })
            .map(|occupant| occupant.entity_id)
        else {
            continue;
        };
        if target_id == detonation.source_id {
            continue;
        }
        let Some(target) = entities.get(target_id) else {
            continue;
        };
        let allied = source_owner.is_some_and(|owner| {
            crate::map::houses::are_houses_friendly(
                house_alliances,
                interner.resolve(owner),
                interner.resolve(target.owner),
            )
        });
        if allied {
            continue;
        }
        targets.push(ProjectileTarget::Entity(target_id));
    }
    while targets.len() < count as usize {
        let (rx, ry) = projectile_random_shrapnel_cell(center_rx, center_ry, scenario_rng);
        targets.push(ProjectileTarget::Cell {
            rx: rx as u16,
            ry: ry as u16,
        });
    }

    let gravity = if child_projectile.floater {
        rules.general.gravity / 2
    } else {
        rules.general.gravity
    };
    for target in targets {
        let target_coord = match target {
            ProjectileTarget::Entity(id) => {
                let Some(entity) = entities.get(id) else {
                    continue;
                };
                ProjectileCoord::new(
                    i32::from(entity.position.rx) * 256 + entity.position.sub_x.to_num::<i32>(),
                    i32::from(entity.position.ry) * 256 + entity.position.sub_y.to_num::<i32>(),
                    i32::from(entity.position.z) * crate::util::lepton::GROUND_LEVEL_HEIGHT_LEPTONS,
                )
            }
            ProjectileTarget::Cell { rx, ry } => {
                crate::sim::projectile::cell_target_coord(terrain, bridge_state, rx, ry)
            }
            ProjectileTarget::None => ProjectileCoord::new(0, 0, 0),
        };
        out.projectile_spawns.push(ProjectileSpawn {
            source_id: detonation.source_id,
            origin: detonation.impact,
            target,
            initial_target_position: target_coord,
            payload: ProjectilePayload {
                base_damage: child_weapon.damage,
                warhead: interner.intern(child_warhead_name),
                weapon: interner.intern(child_weapon_name),
                owner: detonation.payload.owner,
            },
            speed_leptons_per_frame: child_weapon.speed.clamp(1, i32::from(u16::MAX)) as u16,
            velocity: shrapnel_launch_velocity(detonation.impact, target_coord, child_weapon.speed),
            // `BulletClass::Shrapnel @ 0x0046a310` supplies an explicit
            // 45-degree launch vector. The existing velocity/gravity flight
            // state is the represented constructor for that native handoff.
            trajectory: ProjectileTrajectory::Ballistic { gravity },
            guidance: None,
            visual: ProjectileVisualState::new(
                child_projectile.anim_low as u8,
                child_projectile.anim_high as u8,
                child_projectile.anim_rate as u8,
            ),
            arm_frames: child_projectile.arm.clamp(0, i32::from(u16::MAX)) as u16,
            fuse_frames: None,
            ranged_fuse: child_projectile.ranged,
            tracks_target: false,
            target_expiry: TargetExpiryPolicy::DetonateAtLastKnown,
            collision: ProjectileCollisionPolicy {
                level_non_water: child_projectile.level,
                subject_to_walls: child_projectile.subject_to_walls,
                native_cell_collision: child_projectile.rot <= 0,
            },
        });
    }
}

/// Reuse the ordinary combat emission paths after a persistent bullet reaches
/// `BulletClass::Detonate`; this stays before the shared damage/death phases.
#[allow(clippy::too_many_arguments)]
fn emit_one_projectile_detonation(
    detonation: &ProjectileDetonation,
    entities: &mut EntityStore,
    occupancy: &OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    handles: Option<crate::sim::type_handle_table::ResolvedRuleHandles>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    mut overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    mut terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    bridge_state: Option<&BridgeRuntimeState>,
    terrain_objects: Option<combat_aoe::TerrainCollectionView<'_>>,
    terrain_area_state: Option<&TerrainAreaState>,
    scenario_no_damage: bool,
    house_alliances: &HouseAllianceMap,
    scenario_rng: &mut SimRng,
    inline_hooks: &mut Option<&mut dyn CombatInlineHooks>,
    out: &mut CombatEmit,
) {
    let Some(warhead) = rules.warhead(interner.resolve(detonation.payload.warhead)) else {
        log::warn!(
            "Projectile {} dropped: missing serialized warhead {}",
            detonation.projectile_id,
            detonation.payload.warhead
        );
        return;
    };
    let (impact_rx, impact_ry, impact_sub_x, impact_sub_y, world_z_leptons) =
        projectile_impact_cell(detonation.impact);
    let impact_z = world_z_leptons.div_euclid(LEPTONS_PER_LEVEL as i32);
    let air_impact = Some(combat_aoe::AoEAirImpact {
        sub_x: impact_sub_x,
        sub_y: impact_sub_y,
        z_leptons: world_z_leptons,
    });

    // Named location: `BulletClass::Detonate @ 0x004690b0`. Radiation is
    // outside and before the exclusive special-effect chain.
    if let Some(weapon) = rules.weapon(interner.resolve(detonation.payload.weapon))
        && weapon.rad_level > 0
    {
        out.rad_detonations
            .push(crate::sim::radiation::RadDetonation {
                rx: impact_rx,
                ry: impact_ry,
                rad_level: weapon.rad_level,
                spread: warhead.cell_spread.to_num::<i32>(),
            });
    }

    let special_action = projectile_special_detonation_action(SpecialDetonationFlags {
        mind_control: warhead.mind_control,
        ivan_bomb: warhead.ivan_bomb,
        electric_assault: warhead.electric_assault,
        parasite: warhead.parasite,
        temporal: warhead.temporal,
        is_locomotor: warhead.is_locomotor,
        airstrike: warhead.airstrike,
        raw_335: warhead.raw_335,
        bomb_disarm: warhead.bomb_disarm,
        makes_disguise: warhead.makes_disguise,
        nuke_maker: warhead.nuke_maker,
    });
    if special_action != SpecialDetonationAction::OrdinaryDamage {
        // Effect bodies remain explicit residuals. Native else-if ownership is
        // authoritative, so an earlier unsupported predicate still shadows
        // Shrapnel and ordinary DamageArea.
        log::debug!(
            "Projectile {} selected unsupported special detonation {:?}",
            detonation.projectile_id,
            special_action
        );
        return;
    }

    emit_projectile_shrapnel(
        detonation,
        entities,
        occupancy,
        rules,
        interner,
        terrain.as_deref(),
        bridge_state,
        house_alliances,
        scenario_rng,
        out,
    );

    let routed_wall = wall_overlay_flags_at(
        overlay_grid.as_deref(),
        overlay_registry,
        impact_rx,
        impact_ry,
    )
    .is_some_and(|flags| warhead_damages_wall(warhead, flags));
    let ore_amount = if scenario_no_damage {
        None
    } else {
        tiberium_reduction_amount(detonation.payload.base_damage, true, warhead)
    };
    let aoe = {
        let mut ore_prelude = ore_amount.map(|amount| CombatTiberiumCellPrelude {
            amount,
            deferred: &mut out.tiberium_reduction_requests,
            inline_hooks,
            rules,
            resource_nodes,
            terrain_area_state,
        });
        self::combat_aoe::apply_aoe_damage_with_terrain_and_scenario(
            entities,
            impact_rx,
            impact_ry,
            detonation.payload.base_damage,
            warhead,
            rules,
            interner,
            handles,
            (
                detonation.source_id,
                Some(detonation.payload.owner),
                detonation.payload.warhead,
            ),
            self::combat_aoe::AoELayerContext {
                occupancy: Some(occupancy),
                terrain: terrain.as_deref_mut(),
                overlay_grid: overlay_grid.as_deref_mut(),
                overlay_registry,
                scenario_rng: Some(&mut *scenario_rng),
                air_impact,
                impact_z,
            },
            terrain_objects,
            scenario_no_damage,
            ore_prelude
                .as_mut()
                .map(|prelude| prelude as &mut dyn self::combat_aoe::AoECellPrelude),
        )
    };
    out.wall_mutations.extend(aoe.wall_mutations);
    out.cell_target_detaches.extend(aoe.cell_target_detaches);
    out.damage_events.extend(aoe.receivers);

    if !scenario_no_damage && detonation.payload.base_damage > 0 {
        let damage = detonation.payload.base_damage.min(i32::from(u16::MAX)) as u16;
        if !routed_wall && warhead.wall {
            out.bridge_damage_events.push(BridgeDamageEvent {
                rx: impact_rx,
                ry: impact_ry,
                damage,
                warhead_ref: detonation.payload.warhead,
                is_ion_cannon: detonation.payload.warhead == handles
                        .expect("Simulation::resolve_type_handles must run before combat")
                        .ion_cannon,
                impact_z,
            });
        }
    }
    emit_warhead_detonation_effects(
        warhead,
        detonation.payload.base_damage,
        impact_rx,
        impact_ry,
        impact_sub_x,
        impact_sub_y,
        impact_z_byte(impact_z),
        world_z_leptons,
        interner,
        &mut out.explosion_effects,
        &mut out.smudge_spawn_requests,
    );
}

/// Run `BulletClass::Explode @ 0x00468d80` in native call/RNG order.
/// Detonation work is deliberately not precomputed: ordinary Detonate may
/// consume Scenario RNG (notably Shrapnel) before the following cluster-radius
/// and angle draws.
#[allow(clippy::too_many_arguments)]
fn emit_projectile_detonations(
    detonations: &[ProjectileDetonation],
    entities: &mut EntityStore,
    occupancy: &OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    handles: Option<crate::sim::type_handle_table::ResolvedRuleHandles>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    mut overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    mut terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    bridge_state: Option<&BridgeRuntimeState>,
    terrain_objects: Option<combat_aoe::TerrainCollectionView<'_>>,
    terrain_area_state: Option<&TerrainAreaState>,
    scenario_no_damage: bool,
    house_alliances: &HouseAllianceMap,
    scenario_rng: &mut SimRng,
    inline_hooks: &mut Option<&mut dyn CombatInlineHooks>,
    out: &mut CombatEmit,
) {
    for detonation in detonations {
        let projectile_type = rules
            .weapon(interner.resolve(detonation.payload.weapon))
            .and_then(|weapon| weapon.projectile.as_deref())
            .and_then(|projectile| rules.projectile(projectile));
        let Some(projectile_type) = projectile_type else {
            emit_one_projectile_detonation(
                detonation,
                entities,
                occupancy,
                rules,
                interner,
                handles,
                resource_nodes,
                overlay_grid.as_deref_mut(),
                overlay_registry,
                terrain.as_deref_mut(),
                bridge_state,
                terrain_objects,
                terrain_area_state,
                scenario_no_damage,
                house_alliances,
                scenario_rng,
                inline_hooks,
                out,
            );
            continue;
        };

        let airburst = projectile_type.airburst;
        let cluster = projectile_type.cluster;
        if airburst {
            emit_one_projectile_detonation(
                detonation,
                entities,
                occupancy,
                rules,
                interner,
                handles,
                resource_nodes,
                overlay_grid.as_deref_mut(),
                overlay_registry,
                terrain.as_deref_mut(),
                bridge_state,
                terrain_objects,
                terrain_area_state,
                scenario_no_damage,
                house_alliances,
                scenario_rng,
                inline_hooks,
                out,
            );
            continue;
        }

        let mut coordinate = detonation.impact;
        for _ in 0..cluster.max(0) {
            let mut clustered = *detonation;
            clustered.impact = coordinate;
            emit_one_projectile_detonation(
                &clustered,
                entities,
                occupancy,
                rules,
                interner,
                handles,
                resource_nodes,
                overlay_grid.as_deref_mut(),
                overlay_registry,
                terrain.as_deref_mut(),
                bridge_state,
                terrain_objects,
                terrain_area_state,
                scenario_no_damage,
                house_alliances,
                scenario_rng,
                inline_hooks,
                out,
            );
            coordinate = projectile_next_cluster_coord(coordinate, scenario_rng);
        }
    }
}

/// Spawn-manager rockets move before the ordinary per-object fire walk too.
/// Their completed impacts therefore commit shared area-damage wall state at
/// the same pre-attacker boundary as ordinary persistent bullets.
#[allow(clippy::too_many_arguments)]
fn emit_missile_detonations(
    detonations: &[crate::sim::spawn_manager::MissileDetonation],
    entities: &mut EntityStore,
    occupancy: &OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    handles: Option<crate::sim::type_handle_table::ResolvedRuleHandles>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    mut overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    mut terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    terrain_objects: Option<combat_aoe::TerrainCollectionView<'_>>,
    terrain_area_state: Option<&TerrainAreaState>,
    scenario_no_damage: bool,
    scenario_rng: &mut SimRng,
    inline_hooks: &mut Option<&mut dyn CombatInlineHooks>,
    out: &mut CombatEmit,
) {
    for det in detonations {
        let warhead_name = interner.resolve(det.warhead).to_string();
        let Some(warhead) = rules.warhead(&warhead_name) else {
            continue;
        };
        let wh_iid = interner.intern(&warhead.id);
        let impact_z = combat_aoe::bridge_adjusted_impact_z(terrain.as_deref(), det.rx, det.ry);
        let air_impact = combat_aoe::air_impact_from_layer_z(
            terrain.as_deref(),
            det.rx,
            det.ry,
            crate::util::lepton::CELL_CENTER_LEPTON,
            crate::util::lepton::CELL_CENTER_LEPTON,
            impact_z,
        );
        let world_z_leptons = air_impact
            .map(|impact| impact.z_leptons)
            .unwrap_or_else(|| impact_z.wrapping_mul(LEPTONS_PER_LEVEL as i32));
        let mut outer_explosions = Vec::new();
        let mut outer_smudges = Vec::new();
        emit_warhead_detonation_effects(
            warhead,
            det.damage,
            det.rx,
            det.ry,
            crate::util::lepton::CELL_CENTER_LEPTON,
            crate::util::lepton::CELL_CENTER_LEPTON,
            impact_z_byte(impact_z),
            world_z_leptons,
            interner,
            &mut outer_explosions,
            &mut outer_smudges,
        );
        out.explosion_effects.extend(outer_explosions);
        commit_smudge_batch_or_defer(
            outer_smudges,
            &mut out.smudge_spawn_requests,
            inline_hooks,
            rules,
            occupancy,
            interner,
            scenario_rng,
            resource_nodes,
            overlay_grid.as_deref_mut(),
            overlay_registry,
            terrain.as_deref_mut(),
            terrain_area_state,
        );
        let ore_amount = if scenario_no_damage {
            None
        } else {
            tiberium_reduction_amount(det.damage, true, warhead)
        };
        let aoe = {
            let mut ore_prelude = ore_amount.map(|amount| CombatTiberiumCellPrelude {
                amount,
                deferred: &mut out.tiberium_reduction_requests,
                inline_hooks,
                rules,
                resource_nodes,
                terrain_area_state,
            });
            combat_aoe::apply_aoe_damage_with_terrain_and_scenario(
                entities,
                det.rx,
                det.ry,
                det.damage,
                warhead,
                rules,
                interner,
                handles,
                (det.firer_id, Some(det.owner), wh_iid),
                combat_aoe::AoELayerContext {
                    occupancy: Some(occupancy),
                    terrain: terrain.as_deref_mut(),
                    overlay_grid: overlay_grid.as_deref_mut(),
                    overlay_registry,
                    scenario_rng: Some(&mut *scenario_rng),
                    air_impact,
                    impact_z,
                },
                terrain_objects,
                scenario_no_damage,
                ore_prelude
                    .as_mut()
                    .map(|prelude| prelude as &mut dyn combat_aoe::AoECellPrelude),
            )
        };
        out.wall_mutations.extend(aoe.wall_mutations);
        out.cell_target_detaches.extend(aoe.cell_target_detaches);
        out.damage_events.extend(aoe.receivers);
    }
}

/// Advance combat with optional owner visibility gating and sound event sink.
/// Returns reveal events and stable IDs of entities despawned this tick.
///
/// `overlay_grid` and `overlay_registry` are used to discriminate wall-overlay
/// cells from bridge cells when a wall-warhead detonates (so the right event
/// path — immediate wall mutation vs BridgeDamageEvent — is selected). Pass
/// `None` to skip wall-cell discrimination (legacy bridge-only routing).
///
/// `scenario_rng` is the persistent `ScenarioClass::Random` authority used
/// inline by projectile detonation mechanisms such as the Inviso
/// impact-animation scatter.
pub fn tick_combat_with_fog(
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    fog: Option<&FogState>,
    power_states: &BTreeMap<InternedId, PowerState>,
    sound_sink: Option<&mut Vec<SimSoundEvent>>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    current_tick: u64,
    tick_ms: u32,
    binary_frame: u32,
    live_order: &[u64],
    radiation: Option<&mut crate::sim::radiation::RadiationState>,
    scenario_rng: &mut SimRng,
) -> CombatTickResult {
    // Test-convenience entry: resolve rule handles the way sim init does.
    let handles = Some(crate::sim::type_handle_table::ResolvedRuleHandles::resolve(rules, interner));
    let mut unused_main_rng = SimRng::new(0);
    let mut empty_houses = BTreeMap::new();
    tick_combat_with_fog_and_main_rng(
        entities,
        occupancy,
        rules,
        interner,
        handles,
        fog,
        power_states,
        &mut empty_houses,
        &[],
        &HouseAllianceMap::new(),
        sound_sink,
        resource_nodes,
        overlay_grid,
        overlay_registry,
        terrain,
        current_tick,
        tick_ms,
        binary_frame,
        live_order,
        &[],
        &[],
        radiation,
        &[],
        scenario_rng,
        &mut unused_main_rng,
        None,
    )
}

/// Advance combat with persistent owner state and both native RNG authorities.
///
/// Main-RNG death-sound selection is independent from Scenario-RNG projectile
/// and impact effects, so both streams are explicit at the production seam.
pub(crate) fn tick_combat_with_fog_and_main_rng(
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    handles: Option<crate::sim::type_handle_table::ResolvedRuleHandles>,
    fog: Option<&FogState>,
    power_states: &BTreeMap<InternedId, PowerState>,
    houses: &mut BTreeMap<InternedId, HouseState>,
    house_order: &[InternedId],
    alliances: &HouseAllianceMap,
    sound_sink: Option<&mut Vec<SimSoundEvent>>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    current_tick: u64,
    tick_ms: u32,
    binary_frame: u32,
    live_order: &[u64],
    projectile_detonations: &[ProjectileDetonation],
    wave_damage_events: &[WaveDamageEvent],
    radiation: Option<&mut crate::sim::radiation::RadiationState>,
    missile_detonations: &[crate::sim::spawn_manager::MissileDetonation],
    scenario_rng: &mut SimRng,
    main_rng: &mut SimRng,
    inline_hooks: Option<&mut dyn CombatInlineHooks>,
) -> CombatTickResult {
    tick_combat_with_fog_and_main_rng_with_terrain_area(
        entities,
        occupancy,
        rules,
        interner,
        handles,
        fog,
        power_states,
        houses,
        house_order,
        alliances,
        sound_sink,
        resource_nodes,
        overlay_grid,
        overlay_registry,
        terrain,
        None,
        false,
        current_tick,
        tick_ms,
        binary_frame,
        live_order,
        &BTreeSet::new(),
        projectile_detonations,
        wave_damage_events,
        radiation,
        missile_detonations,
        scenario_rng,
        main_rng,
        inline_hooks,
        None,
    )
}

/// Outputs produced by one Bullet Logic slot after its detonation receivers
/// have committed, but before the world retires the Bullet object itself.
pub(crate) struct LogicProjectileCommit {
    pub(crate) projectile_spawns: Vec<ProjectileSpawn>,
    pub(crate) effects: DeathEffects,
    pub(crate) under_attack_events: Vec<UnderAttackEvent>,
}

/// Commit completed Bullet detonations through their ordered receiver calls.
///
/// The caller owns the enclosing Logic cursor and Bullet lifetime. This helper
/// owns only the native `Detonate -> Apply_area_damage` transaction, including
/// recursive fatal lifecycle hooks, so it cannot accidentally run the broad
/// Techno fire sweep for every Bullet slot.
#[allow(clippy::too_many_arguments)]
pub(crate) fn commit_logic_projectile_detonations(
    detonations: &[ProjectileDetonation],
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    handles: Option<crate::sim::type_handle_table::ResolvedRuleHandles>,
    houses: &mut BTreeMap<InternedId, HouseState>,
    house_order: &[InternedId],
    alliances: &HouseAllianceMap,
    main_rng: &mut SimRng,
    scenario_rng: &mut SimRng,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    bridge_state: Option<&BridgeRuntimeState>,
    terrain_area_state: Option<&mut TerrainAreaState>,
    scenario_no_damage: bool,
    current_tick: u64,
    sound_sink: Option<&mut Vec<SimSoundEvent>>,
    inline_hooks: Option<&mut dyn CombatInlineHooks>,
) -> LogicProjectileCommit {
    let mut emit = CombatEmit::default();
    let mut effects = DeathEffects::default();
    let mut handled_deaths = Vec::new();
    let mut under_attack_events = Vec::new();
    let mut inline_hooks = inline_hooks;
    let mut sound_sink = sound_sink;
    commit_projectile_detonations_inline(
        detonations,
        entities,
        occupancy,
        rules,
        interner,
        handles,
        houses,
        house_order,
        alliances,
        main_rng,
        scenario_rng,
        &mut handled_deaths,
        resource_nodes,
        overlay_grid,
        overlay_registry,
        terrain,
        bridge_state,
        terrain_area_state,
        scenario_no_damage,
        current_tick,
        &mut inline_hooks,
        &mut sound_sink,
        &mut emit,
        &mut effects,
        &mut under_attack_events,
    );

    // Physical warhead/death outputs share the world's existing synchronous
    // non-combat handoff. The remaining CombatEmit fields belong exclusively
    // to Techno fire and therefore cannot be produced by this bounded path.
    effects
        .bridge_damage_events
        .append(&mut emit.bridge_damage_events);
    effects.wall_mutations.append(&mut emit.wall_mutations);
    effects
        .cell_target_detaches
        .append(&mut emit.cell_target_detaches);
    effects
        .tiberium_reduction_requests
        .append(&mut emit.tiberium_reduction_requests);
    effects
        .explosion_effects
        .append(&mut emit.explosion_effects);
    effects
        .smudge_spawn_requests
        .append(&mut emit.smudge_spawn_requests);
    effects.rad_detonations.append(&mut emit.rad_detonations);
    debug_assert!(emit.remove_attack.is_empty());
    debug_assert!(emit.retarget_events.is_empty());
    debug_assert!(emit.fire_events.is_empty());
    debug_assert!(emit.reveal_events.is_empty());
    debug_assert!(emit.burst_updates.is_empty());
    debug_assert!(emit.ammo_deduct.is_empty());
    debug_assert!(emit.garrison_advance.is_empty());
    debug_assert!(emit.pending_infantry_updates.is_empty());
    debug_assert!(emit.animation_switches.is_empty());
    debug_assert!(emit.current_weapon_updates.is_empty());
    debug_assert!(emit.unit_facing.is_empty());
    debug_assert!(emit.spawn_target_updates.is_empty());

    LogicProjectileCommit {
        projectile_spawns: emit.projectile_spawns,
        effects,
        under_attack_events,
    }
}

#[allow(clippy::too_many_arguments)]
fn commit_projectile_detonations_inline(
    projectile_detonations: &[ProjectileDetonation],
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    handles: Option<crate::sim::type_handle_table::ResolvedRuleHandles>,
    houses: &mut BTreeMap<InternedId, HouseState>,
    house_order: &[InternedId],
    alliances: &HouseAllianceMap,
    main_rng: &mut SimRng,
    scenario_rng: &mut SimRng,
    handled_deaths: &mut Vec<u64>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    mut overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    mut terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    bridge_state: Option<&BridgeRuntimeState>,
    mut terrain_area_state: Option<&mut TerrainAreaState>,
    scenario_no_damage: bool,
    current_tick: u64,
    inline_hooks: &mut Option<&mut dyn CombatInlineHooks>,
    sound_sink: &mut Option<&mut Vec<SimSoundEvent>>,
    emit: &mut CombatEmit,
    death: &mut DeathEffects,
    under_attack_events: &mut Vec<UnderAttackEvent>,
) {
    for detonation in projectile_detonations {
        let damage_start = emit.damage_events.len();
        let explosion_start = emit.explosion_effects.len();
        let smudge_start = emit.smudge_spawn_requests.len();
        let terrain_objects =
            terrain_area_state
                .as_deref()
                .map(|state| combat_aoe::TerrainCollectionView {
                    objects: state.terrain_objects(),
                    cells: state.terrain_object_cells(),
                });
        emit_projectile_detonations(
            std::slice::from_ref(detonation),
            entities,
            occupancy,
            rules,
            interner,
            handles,
            resource_nodes,
            overlay_grid.as_deref_mut(),
            overlay_registry,
            terrain.as_deref_mut(),
            bridge_state,
            terrain_objects,
            terrain_area_state.as_deref(),
            scenario_no_damage,
            alliances,
            scenario_rng,
            inline_hooks,
            emit,
        );
        let outer_explosion_effects = emit.explosion_effects.split_off(explosion_start);
        let outer_anim_requests = emit.smudge_spawn_requests.split_off(smudge_start);
        let (inline_death, mut pings) = commit_area_damage_receivers_with_scenario(
            &emit.damage_events[damage_start..],
            entities,
            occupancy,
            rules,
            interner,
            handles,
            houses,
            house_order,
            alliances,
            main_rng,
            scenario_rng,
            handled_deaths,
            resource_nodes,
            overlay_grid.as_deref_mut(),
            overlay_registry,
            terrain.as_deref_mut(),
            terrain_area_state.as_deref_mut(),
            scenario_no_damage,
            current_tick,
            inline_hooks,
            sound_sink,
        );
        absorb_inline_death_effects(emit, death, inline_death);
        emit.explosion_effects.extend(outer_explosion_effects);
        commit_smudge_batch_or_defer(
            outer_anim_requests,
            &mut emit.smudge_spawn_requests,
            inline_hooks,
            rules,
            occupancy,
            interner,
            scenario_rng,
            resource_nodes,
            overlay_grid.as_deref_mut(),
            overlay_registry,
            terrain.as_deref_mut(),
            terrain_area_state.as_deref(),
        );
        under_attack_events.append(&mut pings);
    }
}

/// World-owned combat entry that also lends the transient Terrain authority
/// required by ordered Apply_area_damage receivers and fatal callbacks.
#[allow(clippy::too_many_arguments)]
pub(crate) fn tick_combat_with_fog_and_main_rng_with_terrain_area(
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    handles: Option<crate::sim::type_handle_table::ResolvedRuleHandles>,
    fog: Option<&FogState>,
    power_states: &BTreeMap<InternedId, PowerState>,
    houses: &mut BTreeMap<InternedId, HouseState>,
    house_order: &[InternedId],
    alliances: &HouseAllianceMap,
    mut sound_sink: Option<&mut Vec<SimSoundEvent>>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    mut overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    mut terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    bridge_state: Option<&BridgeRuntimeState>,
    scenario_no_damage: bool,
    current_tick: u64,
    tick_ms: u32,
    binary_frame: u32,
    live_order: &[u64],
    fire_suppressed: &BTreeSet<u64>,
    projectile_detonations: &[ProjectileDetonation],
    wave_damage_events: &[WaveDamageEvent],
    mut radiation: Option<&mut crate::sim::radiation::RadiationState>,
    missile_detonations: &[crate::sim::spawn_manager::MissileDetonation],
    scenario_rng: &mut SimRng,
    main_rng: &mut SimRng,
    mut inline_hooks: Option<&mut dyn CombatInlineHooks>,
    mut terrain_area_state: Option<&mut TerrainAreaState>,
) -> CombatTickResult {
    if tick_ms == 0 {
        return CombatTickResult {
            projectile_spawns: Vec::new(),
            reveal_events: Vec::new(),
            despawned_ids: Vec::new(),
            immediate_uninit_ids: Vec::new(),
            structure_destroyed: false,
            bridge_damage_events: Vec::new(),
            wall_mutations: Vec::new(),
            cell_target_detaches: Vec::new(),
            terrain_navigation_changed_cells: Vec::new(),
            tiberium_reduction_requests: Vec::new(),
            fire_events: Vec::new(),
            destroyed_crewed_buildings: Vec::new(),
            destroyed_garrison_buildings: Vec::new(),
            explosion_effects: Vec::new(),
            invulnerability_impact_effects: Vec::new(),
            smudge_spawn_requests: Vec::new(),
            unit_facing: Vec::new(),
            under_attack_events: Vec::new(),
        };
    }

    // Completed prior-frame bullets physically advanced before this frame's
    // object AI/fire walk. Each detonation commits ReceiveDamage and any
    // recursive death weapon before the next detonation or attacker reads
    // wall, target, health, or RNG state.
    let mut emit = CombatEmit::default();
    let mut death = DeathEffects::default();
    let mut handled_deaths = Vec::new();
    let mut under_attack_events = Vec::new();
    commit_projectile_detonations_inline(
        projectile_detonations,
        entities,
        occupancy,
        rules,
        interner,
        handles,
        houses,
        house_order,
        alliances,
        main_rng,
        scenario_rng,
        &mut handled_deaths,
        resource_nodes,
        overlay_grid.as_deref_mut(),
        overlay_registry,
        terrain.as_deref_mut(),
        bridge_state,
        terrain_area_state.as_deref_mut(),
        scenario_no_damage,
        current_tick,
        &mut inline_hooks,
        &mut sound_sink,
        &mut emit,
        &mut death,
        &mut under_attack_events,
    );
    for detonation in missile_detonations {
        let damage_start = emit.damage_events.len();
        let terrain_objects =
            terrain_area_state
                .as_deref()
                .map(|state| combat_aoe::TerrainCollectionView {
                    objects: state.terrain_objects(),
                    cells: state.terrain_object_cells(),
                });
        emit_missile_detonations(
            std::slice::from_ref(detonation),
            entities,
            occupancy,
            rules,
            interner,
            handles,
            resource_nodes,
            overlay_grid.as_deref_mut(),
            overlay_registry,
            terrain.as_deref_mut(),
            terrain_objects,
            terrain_area_state.as_deref(),
            scenario_no_damage,
            scenario_rng,
            &mut inline_hooks,
            &mut emit,
        );
        let (inline_death, mut pings) = commit_area_damage_receivers_with_scenario(
            &emit.damage_events[damage_start..],
            entities,
            occupancy,
            rules,
            interner,
            handles,
            houses,
            house_order,
            alliances,
            main_rng,
            scenario_rng,
            &mut handled_deaths,
            resource_nodes,
            overlay_grid.as_deref_mut(),
            overlay_registry,
            terrain.as_deref_mut(),
            terrain_area_state.as_deref_mut(),
            scenario_no_damage,
            current_tick,
            &mut inline_hooks,
            &mut sound_sink,
        );
        absorb_inline_death_effects(&mut emit, &mut death, inline_death);
        under_attack_events.append(&mut pings);
    }

    // Pre-scan: collect entities blocked from firing by locomotor or power state.
    let fire_blocked = combat_fire_gate::collect_fire_blocked_entities(
        entities,
        power_states,
        Some(rules),
        interner,
    );

    let keys: Vec<u64> = entities.keys_sorted();

    // Deployed self-irradiator re-fire (Desolator): a deployed DeployFire unit
    // whose deploy weapon emits radiation — and whose own type is radiation-
    // immune — maintains the radiation field under its feet. When the site at
    // its cell is missing or its effective level has decayed below a third of
    // the weapon's RadLevel, the unit force-fires its deploy weapon at its own
    // cell (the detonation re-arms the site, which closes the gate again).
    // The synthesized self-target is cleared once the gate closes; targets the
    // player set explicitly are never touched.
    if let Some(rad) = radiation.as_deref() {
        let mut set_self_target: Vec<u64> = Vec::new();
        let mut clear_self_target: Vec<u64> = Vec::new();
        for &id in &keys {
            if fire_suppressed.contains(&id) {
                continue;
            }
            let Some(entity) = entities.get(id) else {
                continue;
            };
            if !entity.is_fully_deployed() || entity.dying || !entity.is_alive() {
                continue;
            }
            let Some(obj) = rules.object(interner.resolve(entity.type_ref)) else {
                continue;
            };
            if !obj.deploy_fire || !obj.immune_to_radiation {
                continue;
            }
            let Some(weapon) = combat_weapon::deploy_fire_weapon_id(obj, entity.veterancy)
                .and_then(|weapon_id| rules.weapon(weapon_id))
            else {
                continue;
            };
            if weapon.rad_level <= 0 {
                continue;
            }
            let own_cell = (entity.position.rx, entity.position.ry);
            let gate_open = rad.site_at(own_cell).is_none_or(|site| {
                crate::sim::radiation::RadiationState::current_site_level(site)
                    < weapon.rad_level / 3
            });
            let has_self_target = matches!(
                entity.attack_target.as_ref().map(|attack| attack.target),
                Some(TargetKind::Cell(rx, ry)) if (rx, ry) == own_cell
            );
            if gate_open && entity.attack_target.is_none() {
                set_self_target.push(id);
            } else if !gate_open && has_self_target {
                clear_self_target.push(id);
            }
        }
        for id in set_self_target {
            if let Some(entity) = entities.get_mut(id) {
                let (rx, ry) = (entity.position.rx, entity.position.ry);
                entity.attack_target = Some(AttackTarget::for_cell(rx, ry));
            }
        }
        for id in clear_self_target {
            if let Some(entity) = entities.get_mut(id) {
                entity.attack_target = None;
                entity.passively_acquired_target = false;
            }
        }
    }

    // Garrison auto-acquire: idle garrisoned buildings scan for hostile targets.
    // Runs before Phase 1 so newly-targeted buildings are included in snapshots.
    for &id in &keys {
        let (is_candidate, owner, pos_rx, pos_ry, sub_x, sub_y, type_id, _barrel_facing) = {
            let entity = match entities.get(id) {
                Some(e) => e,
                None => continue,
            };
            if entity.category != EntityCategory::Structure
                || entity.attack_target.is_some()
                || entity.dying
                || !entity.is_alive()
                || fire_blocked.contains(&id)
            {
                continue;
            }
            (
                true,
                entity.owner,
                entity.position.rx,
                entity.position.ry,
                entity.position.sub_x,
                entity.position.sub_y,
                entity.type_ref,
                entity.barrel_facing,
            )
        };
        if !is_candidate {
            continue;
        }

        let obj = match rules.object(interner.resolve(type_id)) {
            Some(o) => o,
            None => continue,
        };
        if !obj.can_be_occupied || !obj.can_occupy_fire {
            continue;
        }

        // Read cargo info (immutable borrow).
        let (occ_id, half_foundation) = {
            let entity = match entities.get(id) {
                Some(e) => e,
                None => continue,
            };
            let cargo = match entity.passenger_role.cargo() {
                Some(c) if !c.is_empty() => c,
                _ => continue,
            };
            let fi = cargo.garrison_fire_index as usize % cargo.count() as usize;
            let occ_id = cargo.passengers[fi];
            let (fw, fh) = foundation_dimensions(&obj.foundation);
            (occ_id, fw.min(fh) / 2)
        };

        // Resolve occupant type + veterancy for garrison weapon validation.
        let (occ_type, occ_vet) = match entities.get(occ_id) {
            Some(occ) => (occ.type_ref, occ.veterancy),
            None => continue,
        };

        // Scan range = half_foundation + 1 + OccupyWeaponRange (gamemd Greatest_Threat).
        let scan_cells = half_foundation as i32 + 1 + rules.garrison_rules.occupy_weapon_range;
        let scan_range = SimFixed::from_num(scan_cells.max(1));

        // Scan for best hostile target using garrison weapon for Verses/projectile checks.
        // gamemd's Greatest_Threat calls GetWeapon on the building, which returns
        // the occupant's OccupyWeapon — not the occupant's primary weapon.
        let mut best_target: Option<(i64, u8, u64)> = None;
        let owner_str = interner.resolve(owner);
        for candidate in entities.values() {
            if candidate.stable_id == id
                || candidate.health.current == 0
                || candidate.dying
                || candidate.passenger_role.is_inside_transport()
            {
                continue;
            }
            if candidate.owner == owner {
                continue;
            }
            if let Some(fog_state) = fog {
                let candidate_owner_str = interner.resolve(candidate.owner);
                if fog_state.is_friendly(owner_str, candidate_owner_str) {
                    continue;
                }
                if !fog_state.is_cell_visible(owner, candidate.position.rx, candidate.position.ry) {
                    continue;
                }
            }
            let target_cat = combat_target_category(candidate, rules, interner);
            let target_armor = rules
                .object(interner.resolve(candidate.type_ref))
                .map(|o| o.armor.as_str())
                .unwrap_or("none");
            // Use garrison weapon (OccupyWeapon) for target compatibility check.
            let occ_type_str = interner.resolve(occ_type);
            let selected = match combat_weapon::select_garrison_weapon(
                rules,
                occ_type_str,
                occ_vet,
                target_cat,
                target_armor,
            ) {
                Some(s) => s,
                None => continue,
            };
            if combat_weapon::verses_gate(selected.verses_pct)
                == combat_weapon::VersesGate::Suppressed
            {
                continue;
            }
            // Garrison passive scan_range = half_foundation + 1 + OccupyWeaponRange,
            // which never matches selected.weapon.range — same override-fallback
            // case as the scan_range_override branch in acquire_best_target. Keep
            // the 2D check until a future stage threads override-aware 3D.
            let dist_sq = lepton_distance_sq_raw(
                pos_rx,
                pos_ry,
                sub_x,
                sub_y,
                candidate.position.rx,
                candidate.position.ry,
                candidate.position.sub_x,
                candidate.position.sub_y,
            );
            if !is_within_range_leptons(dist_sq, scan_range) {
                continue;
            }
            let class = match rules.object(interner.resolve(candidate.type_ref)) {
                Some(o) if o.primary.is_some() => 0u8,
                _ => 1,
            };
            let rank = (dist_sq, class, candidate.stable_id);
            match best_target {
                Some(current) if rank >= current => {}
                _ => best_target = Some(rank),
            }
        }

        if let Some((_, _, target_id)) = best_target {
            if let Some(building) = entities.get_mut(id) {
                building.attack_target = Some(AttackTarget::new(target_id));
            }
        }
    }

    // Phase 1: snapshot all attackers and advance cooldowns / burst delays.
    let mut snapshots: Vec<AttackerSnapshot> = Vec::new();
    for &id in &keys {
        // TubeMovement owns this object's complete AI turn.  The active state
        // may already have cleared on finalization, so the world host carries
        // the entry-time suppression set into this phased combat adapter.
        if fire_suppressed.contains(&id) {
            continue;
        }
        // Mutable borrow: tick cooldowns and capture the per-attacker scalars +
        // garrison cargo info. Entity field-reads move into `build_attacker_snapshot`
        // (pure) below, after this borrow releases.
        let (
            attack_target,
            cooldown_ticks,
            burst_remaining,
            burst_delay_ticks,
            pending_infantry_fire,
            pending_building_fire,
            garrison_cargo,
        ) = {
            let entity = match entities.get_mut(id) {
                Some(e) => e,
                None => continue,
            };
            // Skip entities inside a transport — they can't fire (unless OpenTopped, deferred).
            if entity.passenger_role.is_inside_transport() {
                continue;
            }
            if entity.dying || !entity.is_alive() {
                // BuildingClass::Update no longer reaches ProcessDelayedFire
                // once the object is dead.
                continue;
            }
            let attack_state = entity.attack_target.as_mut().map(|attack| {
                attack.cooldown_ticks = attack.cooldown_ticks.saturating_sub(1);
                attack.burst_delay_ticks = attack.burst_delay_ticks.saturating_sub(1);
                (
                    attack.target,
                    attack.cooldown_ticks,
                    attack.burst_remaining,
                    attack.burst_delay_ticks,
                    attack.pending_infantry_fire,
                )
            });

            // gamemd-derived: BuildingClass::Update @ 0x0043FB20 invokes
            // ProcessDelayedFire @ 0x004503F0 after mission dispatch. The
            // signed counter is pre-decremented and values <= 0 clamp to zero
            // and expire on this visit.
            let pending_building_fire = entity.pending_building_fire.as_mut().map(|pending| {
                pending.remaining_ticks = pending.remaining_ticks.saturating_sub(1).max(0);
                *pending
            });
            if pending_building_fire.is_some_and(|pending| pending.remaining_ticks != 0) {
                // GetFireError @ 0x00447F10 blocks ordinary fire while armed.
                continue;
            }
            let Some((
                attack_target,
                cooldown_ticks,
                burst_remaining,
                burst_delay_ticks,
                pending_infantry_fire,
            )) = attack_state
            else {
                // Expiry reads only the live target. A missing target clears
                // the latch and does not acquire or drop another target.
                if pending_building_fire.is_some() {
                    entity.pending_building_fire = None;
                }
                continue;
            };
            // Skip snapshot for entities blocked by locomotor state (cooldowns still tick).
            if fire_blocked.contains(&id) {
                // Delayed expiry rechecks fire admissibility and clears on any
                // failure rather than postponing until the building is usable.
                if pending_building_fire.is_some() {
                    entity.pending_building_fire = None;
                }
                continue;
            }

            // Extract garrison cargo info while we have the entity.
            let garrison_cargo: Option<(u8, u8, u64)> =
                if entity.category == EntityCategory::Structure {
                    entity.passenger_role.cargo().and_then(|c| {
                        if c.is_empty() {
                            return None;
                        }
                        let fi = c.garrison_fire_index;
                        let count = c.count() as u8;
                        let oi = fi as usize % count as usize;
                        Some((fi, count, c.passengers[oi]))
                    })
                } else {
                    None
                };

            (
                attack_target,
                cooldown_ticks,
                burst_remaining,
                burst_delay_ticks,
                pending_infantry_fire,
                pending_building_fire,
                garrison_cargo,
            )
        }; // mutable borrow released

        // Re-fetch the attacker immutably (nothing mutated `entities` since the
        // borrow above released) and resolve any garrison occupant, then build the
        // snapshot through the shared `build_attacker_snapshot` so the field-reads
        // stay byte-identical to the per-object Fire→Facing host.
        let entity = match entities.get(id) {
            Some(e) => e,
            None => continue,
        };
        let garrison = garrison_cargo.and_then(|(fire_idx, count, occ_id)| {
            let obj = rules.object(interner.resolve(entity.type_ref))?;
            if !obj.can_be_occupied || !obj.can_occupy_fire {
                return None;
            }
            let occ = entities.get(occ_id)?;
            let (fw, fh) = foundation_dimensions(&obj.foundation);
            Some(GarrisonSnapshot {
                occupant_type_id: occ.type_ref,
                occupant_veterancy: occ.veterancy,
                fire_index: fire_idx,
                occupant_count: count,
                half_foundation: fw.min(fh) / 2,
            })
        });

        snapshots.push(build_attacker_snapshot(
            entity,
            attack_target,
            cooldown_ticks,
            burst_remaining,
            burst_delay_ticks,
            pending_infantry_fire,
            pending_building_fire,
            garrison,
        ));
    }
    // Native combat resolves each object inline during the single live-object
    // (reveal/insertion-order) AI walk, so firing/damage/kill-credit order is
    // the live-object order, not stable-id. Sort the collected attacker
    // snapshots by their position in the live order. stable_id is the
    // deterministic tiebreaker for any attacker absent from the live order
    // (limbo objects do not fire) and makes an empty live_order reproduce the
    // previous stable-id order exactly.
    let live_index: std::collections::HashMap<u64, usize> = live_order
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, i))
        .collect();
    snapshots.sort_by_key(|s| {
        (
            live_index.get(&s.stable_id).copied().unwrap_or(usize::MAX),
            s.stable_id,
        )
    });

    // UnitClass Facing_Update runs immediately after this object's Fire_At_Target,
    // before any bullet created by the fire reaches its later LogicVector slot.
    // Capture that read window for every Unit up front: VERA's unsupported-
    // projectile immediate path may enter fatal lifecycle synchronously below,
    // but that approximation must not make this frame's barrel destination
    // observe a target loss native Facing_Update cannot yet see.
    for snap in &snapshots {
        let Some(entity) = entities.get(snap.stable_id).filter(|entity| {
            entity.category == EntityCategory::Unit && entity.barrel_facing.is_some()
        }) else {
            continue;
        };
        let Some(desired) = crate::sim::movement::turret::desired_turret_facing(entity, entities)
        else {
            continue;
        };
        emit.unit_facing.push((snap.stable_id, desired));
    }

    // Phase 2: per-attacker fire decision + emission, in live-LOGIC snapshot
    // order. Each attacker is resolved through `resolve_attacker_fire` (the
    // reusable per-object fire body); emission order is identical to the prior
    // inline loop, preserving both event order and inline Scenario-RNG draws.
    // Fire is category-agnostic (Units fire through the same body here); Unit
    // FACING destinations use the preseeded native read window above, with
    // own-retarget/remove replacement below, then are applied post-batch by
    // `unit_post::apply_unit_facing`.
    for snap in &snapshots {
        let Some(live_attack) = entities
            .get(snap.stable_id)
            .filter(|entity| entity.is_alive() && !entity.dying)
            .and_then(|entity| {
                entity.attack_target.as_ref().map(|attack| {
                    (
                        attack.target,
                        attack.cooldown_ticks,
                        attack.burst_remaining,
                        attack.burst_delay_ticks,
                        attack.pending_infantry_fire,
                        entity.pending_building_fire,
                    )
                })
            })
        else {
            continue;
        };
        let mut live_snap = snap.clone();
        live_snap.target = live_attack.0;
        live_snap.cooldown_ticks = live_attack.1;
        live_snap.burst_remaining = live_attack.2;
        live_snap.burst_delay_ticks = live_attack.3;
        live_snap.pending_infantry_fire = live_attack.4;
        live_snap.pending_building_fire = live_attack.5;

        let n_retarget = emit.retarget_events.len();
        let n_remove = emit.remove_attack.len();
        let damage_start = emit.damage_events.len();
        let explosion_start = emit.explosion_effects.len();
        let smudge_start = emit.smudge_spawn_requests.len();
        let current_weapon_start = emit.current_weapon_updates.len();
        let terrain_objects =
            terrain_area_state
                .as_deref()
                .map(|state| combat_aoe::TerrainCollectionView {
                    objects: state.terrain_objects(),
                    cells: state.terrain_object_cells(),
                });
        resolve_attacker_fire(
            &live_snap,
            entities,
            rules,
            interner,
            handles,
            resource_nodes,
            fog,
            occupancy,
            overlay_grid.as_deref_mut(),
            overlay_registry,
            terrain.as_deref_mut(),
            terrain_objects,
            terrain_area_state.as_deref(),
            scenario_no_damage,
            binary_frame,
            tick_ms,
            scenario_rng,
            &mut inline_hooks,
            &mut emit,
        );
        let outer_explosion_effects = emit.explosion_effects.split_off(explosion_start);
        let outer_anim_requests = emit.smudge_spawn_requests.split_off(smudge_start);
        for &(entity_id, weapon_index, weapon_ref) in
            &emit.current_weapon_updates[current_weapon_start..]
        {
            if let Some(entity) = entities.get_mut(entity_id) {
                entity.current_weapon_index = weapon_index;
                entity.current_weapon_ref = Some(weapon_ref);
            }
        }
        let (inline_death, mut pings) = commit_area_damage_receivers_with_scenario(
            &emit.damage_events[damage_start..],
            entities,
            occupancy,
            rules,
            interner,
            handles,
            houses,
            house_order,
            alliances,
            main_rng,
            scenario_rng,
            &mut handled_deaths,
            resource_nodes,
            overlay_grid.as_deref_mut(),
            overlay_registry,
            terrain.as_deref_mut(),
            terrain_area_state.as_deref_mut(),
            scenario_no_damage,
            current_tick,
            &mut inline_hooks,
            &mut sound_sink,
        );
        absorb_inline_death_effects(&mut emit, &mut death, inline_death);
        emit.explosion_effects.extend(outer_explosion_effects);
        commit_smudge_batch_or_defer(
            outer_anim_requests,
            &mut emit.smudge_spawn_requests,
            &mut inline_hooks,
            rules,
            occupancy,
            interner,
            scenario_rng,
            resource_nodes,
            overlay_grid.as_deref_mut(),
            overlay_registry,
            terrain.as_deref_mut(),
            terrain_area_state.as_deref(),
        );
        under_attack_events.append(&mut pings);
        // S3: only this Unit's explicit retarget/remove may replace its seeded
        // destination. Synchronous target expiry from VERA's immediate-delivery
        // approximation is deliberately not visible to native Facing_Update.
        let Some(e) = entities
            .get(snap.stable_id)
            .filter(|e| e.category == EntityCategory::Unit && e.barrel_facing.is_some())
        else {
            continue;
        };
        let own_retarget = emit.retarget_events[n_retarget..]
            .iter()
            .find(|&&(aid, _)| aid == snap.stable_id)
            .map(|&(_, tid)| tid);
        let own_removed = emit.remove_attack[n_remove..].contains(&snap.stable_id);
        let replacement: Option<u16> = if let Some(tid) = own_retarget {
            Some(match entities.get(tid) {
                Some(t) => crate::sim::movement::turret::facing_toward_lepton(
                    e.position.rx,
                    e.position.ry,
                    e.position.sub_x,
                    e.position.sub_y,
                    t.position.rx,
                    t.position.ry,
                    t.position.sub_x,
                    t.position.sub_y,
                ),
                None => crate::sim::movement::turret::body_facing_to_turret(e.facing),
            })
        } else if own_removed {
            Some(crate::sim::movement::turret::body_facing_to_turret(
                e.facing,
            ))
        } else {
            None
        };
        if let Some(replacement) = replacement {
            let (_, desired) = emit
                .unit_facing
                .iter_mut()
                .find(|(id, _)| *id == snap.stable_id)
                .expect("turreted Unit attacker was seeded before fire");
            *desired = replacement;
        }
    }
    // S3 residual: every Unit not in the attacker snapshot set (target-less,
    // or in-transport holders excluded at the snapshot build). This runs after
    // all attack ReceiveDamage/death-helper calls, so its target read observes
    // the same live state the next native object window would expose.
    {
        let mut computed: Vec<u64> = emit.unit_facing.iter().map(|&(id, _)| id).collect();
        computed.sort_unstable();
        for &id in &keys {
            if fire_suppressed.contains(&id) {
                continue;
            }
            if computed.binary_search(&id).is_ok() {
                continue;
            }
            let Some(e) = entities.get(id) else { continue };
            if e.category != EntityCategory::Unit {
                continue;
            }
            let Some(desired) = crate::sim::movement::turret::desired_turret_facing(e, entities)
            else {
                continue;
            };
            emit.unit_facing.push((id, desired));
        }
    }
    // Every projectile, missile, and live-order attack damage event emitted so
    // far is already committed. WaveClass::DamageArea is consumed below in its
    // native wave -> recorded-cell -> selected Cell-list order, followed by
    // periodic radiation in live-victim order.
    let committed_damage_event_count = emit.damage_events.len();
    for event in wave_damage_events {
        emit.damage_events
            .push(combat_aoe::AreaDamageReceiver::Entity(
                EntityDamageEvent::from_wave(*event, entities),
            ));
    }
    // Destructure back into the named locals for post-fire state updates.
    let CombatEmit {
        projectile_spawns,
        mut damage_events,
        rad_detonations,
        mut remove_attack,
        retarget_events,
        fire_events,
        reveal_events,
        mut bridge_damage_events,
        mut wall_mutations,
        mut cell_target_detaches,
        mut tiberium_reduction_requests,
        mut explosion_effects,
        mut smudge_spawn_requests,
        burst_updates,
        ammo_deduct,
        garrison_advance,
        pending_infantry_updates,
        animation_switches,
        current_weapon_updates: _,
        unit_facing,
        spawn_target_updates,
    } = emit;

    // Spawner weapons: hand the fire target to the parent's spawn manager.
    // `SpawnManagerClass::SetTarget` only queues a target that differs from the
    // live one; the manager's own AI pass promotes it.
    for &(parent_id, target) in &spawn_target_updates {
        if let Some(manager) = entities
            .get_mut(parent_id)
            .and_then(|e| e.spawn_manager.as_mut())
        {
            manager.set_target(Some(target));
        }
    }

    // Phase 3: apply retargets and burst/cooldown updates.
    // Auto-retargets only ever produce Entity targets (acquire_best_target
    // scans hostile entities), so this wraps the u64 in TargetKind::Entity.
    for &(attacker_id, new_target_sid) in &retarget_events {
        if let Some(entity) = entities.get_mut(attacker_id) {
            retarget_preserving_rearm(entity, new_target_sid);
        }
    }
    for &(attacker_id, sequence) in &animation_switches {
        if let Some(entity) = entities.get_mut(attacker_id) {
            if let Some(ref mut anim) = entity.animation {
                anim.switch_to(sequence);
            }
        }
    }
    for &(attacker_id, pending) in &pending_infantry_updates {
        if let Some(entity) = entities.get_mut(attacker_id) {
            if let Some(ref mut attack) = entity.attack_target {
                attack.pending_infantry_fire = pending;
            }
        }
    }
    for &(attacker_id, burst_rem, burst_delay, rof_cd) in &burst_updates {
        if let Some(entity) = entities.get_mut(attacker_id) {
            if let Some(ref mut attack) = entity.attack_target {
                attack.burst_remaining = burst_rem;
                attack.burst_delay_ticks = burst_delay;
                attack.cooldown_ticks = rof_cd;
            }
        }
    }

    // Phase 3b: deduct ammo from aircraft that completed a burst this tick.
    for &attacker_id in &ammo_deduct {
        if let Some(entity) = entities.get_mut(attacker_id) {
            if let Some(ref mut ammo) = entity.aircraft_ammo {
                ammo.current = (ammo.current - 1).max(0);
            }
        }
    }

    // Phase 3c: advance garrison fire index for buildings that fired this tick.
    // Round-robin: (idx + 1) % count — matches gamemd Fire_At 0x006FDD50.
    for &building_id in &garrison_advance {
        if let Some(entity) = entities.get_mut(building_id) {
            if let Some(cargo) = entity.passenger_role.cargo_mut() {
                let count = cargo.count() as u8;
                if count > 0 {
                    cargo.garrison_fire_index = (cargo.garrison_fire_index + 1) % count;
                }
            }
        }
    }

    // Phase 3.5: fold radiation-emitting detonations into the field, then
    // collect the periodic radiation damage. The original applies this damage
    // inside each foot unit's own AI step, gated on the global frame counter;
    // the phased engine collects it here so deaths route through the same
    // death pipeline as weapon damage (death anim selection via the
    // RadSiteWarhead, owned-count bookkeeping, survivor ejection).
    if let Some(rad) = radiation.as_deref_mut() {
        for &det in &rad_detonations {
            rad.apply_detonation(det, binary_frame, &rules.radiation, terrain.as_deref());
        }
        if !rad.is_empty() && binary_frame.is_multiple_of(rules.radiation.application_delay as u32)
        {
            if let Some(rad_warhead) = rules.warhead(&rules.radiation.site_warhead) {
                let wh_iid = interner.intern(&rad_warhead.id);
                // Victims are walked in live-LOGIC order (the same order the
                // per-object AI would have applied this damage), stable-id
                // fallback for entities absent from the live order.
                let mut victim_ids: Vec<u64> = keys.clone();
                victim_ids
                    .sort_by_key(|&id| (live_index.get(&id).copied().unwrap_or(usize::MAX), id));
                for &id in &victim_ids {
                    let Some(entity) = entities.get(id) else {
                        continue;
                    };
                    // Buildings never take radiation damage; corpses, limbo
                    // (transported) and airborne units are exempt.
                    if entity.category == EntityCategory::Structure
                        || entity.dying
                        || !entity.is_alive()
                        || entity.immune_to_radiation
                        || entity.passenger_role.is_inside_transport()
                    {
                        continue;
                    }
                    let airborne = entity
                        .locomotor
                        .as_ref()
                        .is_some_and(|loco| loco.altitude > SIM_ZERO);
                    if airborne {
                        continue;
                    }
                    let level = rad.damaging_level(
                        (entity.position.rx, entity.position.ry),
                        rules.radiation.level_max,
                    );
                    if level <= 0 {
                        continue;
                    }
                    // FootClass::AI @ 0x004DA530 passes the signed two-stage
                    // ftol result directly to concrete ReceiveDamage at
                    // distance zero. Verses and live defender modifiers belong
                    // to that receiver, not this producer.
                    let base = (level as f64 * rules.radiation.level_factor) as i32;
                    damage_events.push(combat_aoe::AreaDamageReceiver::Entity(
                        EntityDamageEvent::direct_receiver(
                            id,
                            base,
                            0,
                            RAD_NO_ATTACKER,
                            None,
                            wh_iid,
                            ReceiverCallFlags {
                                ignore_defenses: false,
                                arg6: true,
                            },
                        ),
                    ));
                }
            }
        }
    }

    // Periodic radiation is the only damage appended after the native-order
    // projectile/missile/object windows above. Commit that late slice in its
    // existing live-victim order and enter any fatal death helper immediately.
    let (mut late_death, mut late_pings) = commit_area_damage_receivers_with_scenario(
        &damage_events[committed_damage_event_count..],
        entities,
        occupancy,
        rules,
        interner,
        handles,
        houses,
        house_order,
        alliances,
        main_rng,
        scenario_rng,
        &mut handled_deaths,
        resource_nodes,
        overlay_grid.as_deref_mut(),
        overlay_registry,
        terrain.as_deref_mut(),
        terrain_area_state.as_deref_mut(),
        scenario_no_damage,
        current_tick,
        &mut inline_hooks,
        &mut sound_sink,
    );
    if let Some(rad) = radiation.as_deref_mut() {
        for det in late_death.rad_detonations.drain(..) {
            rad.apply_detonation(det, binary_frame, &rules.radiation, terrain.as_deref());
        }
    }
    bridge_damage_events.append(&mut late_death.bridge_damage_events);
    wall_mutations.append(&mut late_death.wall_mutations);
    cell_target_detaches.append(&mut late_death.cell_target_detaches);
    tiberium_reduction_requests.append(&mut late_death.tiberium_reduction_requests);
    explosion_effects.append(&mut late_death.explosion_effects);
    smudge_spawn_requests.append(&mut late_death.smudge_spawn_requests);
    death.append(late_death);
    under_attack_events.append(&mut late_pings);

    // Phase 5: remove AttackTarget from finished attackers.
    remove_attack.sort_unstable();
    remove_attack.dedup();
    for &attacker_id in &remove_attack {
        if let Some(entity) = entities.get_mut(attacker_id) {
            entity.attack_target = None;
            // The provenance flag cannot outlive the target it describes.
            entity.passively_acquired_target = false;
        }
    }

    // Push the synchronously selected death sounds to the presentation sink;
    // entity UnInit itself remains the world-owned deferred handoff.
    if let Some(sink) = sound_sink {
        for (die_id, rx, ry) in death.death_sounds {
            sink.push(SimSoundEvent::EntityDied {
                die_sound_id: die_id,
                rx,
                ry,
            });
        }
    }

    if !damage_events.is_empty() {
        log::trace!(
            "Combat tick: {} shots fired, {} entities destroyed",
            damage_events.len(),
            handled_deaths.len(),
        );
    }

    CombatTickResult {
        projectile_spawns,
        reveal_events,
        despawned_ids: death.despawned_ids,
        immediate_uninit_ids: death.immediate_uninit_ids,
        structure_destroyed: death.structure_destroyed,
        bridge_damage_events,
        wall_mutations,
        cell_target_detaches,
        terrain_navigation_changed_cells: terrain_area_state
            .as_deref()
            .map(|state| state.navigation_changed_cells().to_vec())
            .unwrap_or_default(),
        tiberium_reduction_requests,
        fire_events,
        destroyed_crewed_buildings: death.destroyed_crewed_buildings,
        destroyed_garrison_buildings: death.destroyed_garrison_buildings,
        explosion_effects,
        invulnerability_impact_effects: death.invulnerability_impact_effects,
        smudge_spawn_requests,
        unit_facing,
        under_attack_events,
    }
}

/// Build the per-attacker fire snapshot from current entity state. PURE READ —
/// the caller has already decremented cooldown/burst-delay for this tick and
/// resolved any garrison occupant. Single source of the snapshot field-reads so
/// the legacy Phase-1 loop and the per-object Fire→Facing host build byte-identical
/// snapshots (no field-read drift between the two call sites).
pub(crate) fn build_attacker_snapshot(
    entity: &GameEntity,
    target: TargetKind,
    cooldown_ticks: u16,
    burst_remaining: u8,
    burst_delay_ticks: u8,
    pending_infantry_fire: Option<PendingInfantryFire>,
    pending_building_fire: Option<PendingBuildingFire>,
    garrison: Option<GarrisonSnapshot>,
) -> AttackerSnapshot {
    AttackerSnapshot {
        stable_id: entity.stable_id,
        owner: entity.owner,
        category: entity.category,
        target,
        pos_rx: entity.position.rx,
        pos_ry: entity.position.ry,
        pos_z: entity.position.z,
        pos_exact_z_leptons: entity.position.exact_z_leptons,
        sub_x: entity.position.sub_x,
        sub_y: entity.position.sub_y,
        type_id: entity.type_ref,
        facing: entity.facing,
        veterancy: entity.veterancy,
        cooldown_ticks,
        animation_sequence: entity.animation.as_ref().map(|a| a.sequence),
        animation_frame: entity.animation.as_ref().map(|a| a.frame_index),
        is_prone: entity
            .infantry
            .as_ref()
            .is_some_and(|infantry| infantry.is_prone),
        is_fully_deployed: entity.is_fully_deployed(),
        has_movement: entity.movement_target.is_some(),
        pending_infantry_fire,
        pending_building_fire,
        barrel_facing: entity.barrel_facing,
        burst_remaining,
        burst_delay_ticks,
        weapon_override: entity.weapon_override,
        garrison,
        scan_mission: threat_range::scan_mission_for(entity),
    }
}

/// Veterancy at or above this counts as veteran for the score award.
const VETERAN_VETERANCY: u16 = 100;
/// Veterancy at or above this counts as elite for the score award.
const ELITE_VETERANCY: u16 = 200;

/// Score value destroying `victim` is worth, before the allied-victim zeroing the
/// caller applies.
///
/// gamemd's kill-record step asks the victim's type for its value and that
/// accessor returns the type's **`Cost=`** — the same field production charges —
/// doubled at veteran and tripled at elite.
///
/// It is emphatically NOT `Points=`. `Points=` parses into its own type field
/// that nothing in the binary ever reads back: the only references are the
/// constructor zeroing it, the INI store, and the type-checksum walk. It is
/// dormant Tiberian Sun legacy in YR, so this engine does not parse it at all.
/// The two are not even proportional — a Rhino is `Cost=900 / Points=25` while a
/// GI is `200 / 10` — so using `Points=` both shrinks the column and reorders the
/// table.
///
/// gamemd passes the victim's house so its cost bonuses apply on top. This engine
/// models no per-house cost modifier anywhere — production charges the raw
/// `Cost=` too — so the award is the raw cost, consistent with what the player
/// was actually charged. UNCHECKED against the native bonus set.
pub(crate) fn score_award_for_victim(victim: Option<&ObjectType>, veterancy: u16) -> i32 {
    let cost = victim.map_or(0, |obj| obj.cost);
    if cost <= 0 {
        return 0;
    }
    let multiplier = if veterancy >= ELITE_VETERANCY {
        3
    } else if veterancy >= VETERAN_VETERANCY {
        2
    } else {
        1
    };
    cost.saturating_mul(multiplier)
}

/// Record who destroyed `victim`, at the instant it happened.
///
/// Every lethal path funnels through here so there is one capture rather than a
/// recording mechanism per death cause. Call it immediately after zeroing a
/// victim's health, with the house that should be credited.
///
/// No-ops unless the victim is actually at zero health, and the first writer
/// wins within one fatal transaction. A qualifying PostMortem callback consumes
/// and clears this deferred-UnInit latch before restoring the object, so a later
/// independent lethal transaction can attribute freshly. The award is resolved
/// here because the rules are in hand and the veterancy is still the value the
/// object died at.
///
/// Routed today: the projectile/damage-event loop, death-explosion area damage,
/// and crushing. Spawner missiles arrive through the damage loop with the firer
/// set to the launching object, so they credit the launcher's house; if the
/// launcher itself dies during the missile's flight the firer no longer resolves
/// and that kill goes uncredited.
///
/// NOT routed yet, because each site would need a `&RuleSet` threaded into a
/// function that does not take one: the Iron Curtain and Genetic Mutator infantry
/// kills, aircraft self-destruct, passengers dying with their transport
/// (`world::lifecycle`) or with a collapsing bridge, and passengers ejected by a
/// sell. Those victims book a Loss with no matching Kill.
pub(crate) fn capture_kill_credit(
    victim: &mut crate::sim::game_entity::GameEntity,
    killer_owner: Option<InternedId>,
    rules: &crate::rules::ruleset::RuleSet,
    interner: &crate::sim::intern::StringInterner,
) {
    // `DontScore=` victims are invisible to the score screen entirely. gamemd
    // returns on this byte before any of its bookkeeping, so the kill and the
    // points are both suppressed here and the loss is suppressed at the
    // lifecycle recorder — the two halves of that one native early return.
    if victim.dont_score {
        return;
    }
    if victim.health.current != 0 || victim.killed_by.is_some() {
        return;
    }
    let Some(killer_owner) = killer_owner else {
        return;
    };
    victim.killed_by = Some(killer_owner);
    victim.kill_award_points = score_award_for_victim(
        rules.object(interner.resolve(victim.type_ref)),
        victim.veterancy,
    );
}

/// Resolve one attacker's Phase-2 fire decision + emission for the current tick.
/// The caller immediately commits the newly emitted damage slice and nested
/// fatal consequences before advancing to the next live-order attacker.
/// Interns warhead/weapon/anim strings (hence `&mut StringInterner`). Pure w.r.t.
/// iteration order: the caller invokes it once per snapshot in live-LOGIC order,
/// preserving emission order exactly.
pub(crate) fn resolve_attacker_fire(
    snap: &AttackerSnapshot,
    entities: &mut EntityStore,
    rules: &RuleSet,
    interner: &mut StringInterner,
    handles: Option<crate::sim::type_handle_table::ResolvedRuleHandles>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    fog: Option<&FogState>,
    occupancy: &OccupancyGrid,
    mut overlay_grid: Option<&mut OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    mut terrain: Option<&mut crate::map::resolved_terrain::ResolvedTerrainGrid>,
    terrain_objects: Option<combat_aoe::TerrainCollectionView<'_>>,
    terrain_area_state: Option<&TerrainAreaState>,
    scenario_no_damage: bool,
    binary_frame: u32,
    _tick_ms: u32,
    scenario_rng: &mut SimRng,
    inline_hooks: &mut Option<&mut dyn CombatInlineHooks>,
    out: &mut CombatEmit,
) {
    let delayed_building_slot = snap
        .pending_building_fire
        .map(|pending| pending.weapon_slot);
    if delayed_building_slot.is_some() {
        // ProcessDelayedFire clears mode/timer regardless of whether its live
        // target and saved weapon still pass GetFireError.
        if let Some(entity) = entities.get_mut(snap.stable_id) {
            entity.pending_building_fire = None;
        }
    }
    // Pre-compute garrison scan range for retargeting (includes +1 buffer).
    let garrison_retarget_range: Option<SimFixed> = snap.garrison.as_ref().map(|gs| {
        let cells = gs.half_foundation as i32 + 1 + rules.garrison_rules.occupy_weapon_range;
        SimFixed::from_num(cells.max(1))
    });
    let obj = match rules.object(interner.resolve(snap.type_id)) {
        Some(o) => o,
        None => {
            if delayed_building_slot.is_none() {
                out.remove_attack.push(snap.stable_id);
            }
            return;
        }
    };

    // Check if target is alive and get its data.
    // For structures, target_coords returns the foundation center instead
    // of the NW corner.
    // For Cell targets (force-fire on terrain), synthesize a target_data
    // tuple: cell-center coords, "always alive" (cells don't despawn), no
    // category/type/owner — the unit fires its primary weapon and splash
    // delivers the damage.
    let target_data: Option<(
        u16,
        u16,
        SimFixed,
        SimFixed,
        u16,
        EntityCategory,
        InternedId,
        InternedId,
        bool,
    )> = match snap.target {
        TargetKind::Entity(target_id) => entities.get(target_id).map(|t| {
            let (trx, try_, tsx, tsy) = target_coords(t, Some(rules), interner);
            (
                trx,
                try_,
                tsx,
                tsy,
                t.health.current,
                combat_target_category(t, rules, interner),
                t.type_ref,
                t.owner,
                t.category == EntityCategory::Infantry && infantry::is_prone_for_damage(t),
            )
        }),
        TargetKind::Cell(rx, ry) => {
            // Synthetic target_data for force-fire-on-cell.
            // - hp = 1 so the "target dead" retarget branch never fires for cells.
            // - category = Structure so weapon-vs-armor selection picks an
            //   anti-structure weapon when one exists; otherwise falls
            //   through to primary (matches "fire your default weapon at
            //   the ground" intent).
            // - type_ref/owner = attacker's own — friendly-fire check
            //   sees self-vs-self and is short-circuited downstream.
            let (trx, try_, tsx, tsy) = cell_center_coords(rx, ry);
            Some((
                trx,
                try_,
                tsx,
                tsy,
                1u16,
                EntityCategory::Structure,
                snap.type_id,
                snap.owner,
                false,
            ))
        }
    };

    let (
        target_rx,
        target_ry,
        target_sub_x,
        target_sub_y,
        _target_hp,
        target_cat,
        target_type_ref,
        target_owner,
        _target_prone_infantry,
    ) = match target_data {
        Some((rx, ry, sx, sy, hp, cat, tr, own, prone)) if hp > 0 => {
            (rx, ry, sx, sy, hp, cat, tr, own, prone)
        }
        _ => {
            if delayed_building_slot.is_some() {
                return;
            }
            if let Some(new_target) = acquire_best_target(
                entities,
                rules,
                interner,
                snap,
                obj,
                fog,
                garrison_retarget_range,
                terrain.as_deref(),
            ) {
                out.retarget_events.push((snap.stable_id, new_target));
            } else {
                out.remove_attack.push(snap.stable_id);
            }
            return;
        }
    };

    let target_armor: String = rules
        .object(interner.resolve(target_type_ref))
        .map(|o| o.armor.clone())
        .unwrap_or_else(|| "none".to_string());

    // Weapon selection: garrison uses occupant's OccupyWeapon, standard uses IFV/Primary/Secondary.
    let (selected, is_garrison) = if let Some(saved_slot) = delayed_building_slot {
        match select_weapon_slot(
            rules,
            obj,
            target_cat,
            &target_armor,
            snap.veterancy,
            saved_slot,
        ) {
            Some(selected) => (selected, false),
            None => return,
        }
    } else if let Some(ref gs) = snap.garrison {
        match combat_weapon::select_garrison_weapon(
            rules,
            interner.resolve(gs.occupant_type_id),
            gs.occupant_veterancy,
            target_cat,
            &target_armor,
        ) {
            Some(s) => (s, true),
            None => {
                out.remove_attack.push(snap.stable_id);
                return;
            }
        }
    } else {
        let deploy_fire_weapon_active = entities
            .get(snap.stable_id)
            .is_some_and(uses_deploy_fire_weapon);
        let selected = if deploy_fire_weapon_active {
            select_deploy_fire_weapon(
                rules,
                obj,
                target_cat,
                &target_armor,
                snap.veterancy,
                snap.weapon_override,
            )
        } else {
            select_weapon_with_override(
                rules,
                obj,
                target_cat,
                &target_armor,
                snap.veterancy,
                snap.weapon_override,
            )
        };
        match selected {
            Some(s) => (s, false),
            None => {
                out.remove_attack.push(snap.stable_id);
                return;
            }
        }
    };
    let weapon = selected.weapon;
    if delayed_building_slot.is_none() {
        out.current_weapon_updates.push((
            snap.stable_id,
            match selected.slot {
                WeaponSlot::Primary => 0,
                WeaponSlot::Secondary => 1,
            },
            interner.intern(selected.weapon_id),
        ));
    }

    // Friendly-fire and visibility-driven retarget logic only applies to
    // Entity targets. Cell targets are an explicit player force-fire — the
    // player intentionally chose this cell (allies, ground, anything), so
    // never auto-retarget away from a Cell.
    let is_cell_target = matches!(snap.target, TargetKind::Cell(_, _));
    if let Some(fog_state) = fog {
        let snap_owner_str = interner.resolve(snap.owner);
        let target_owner_str = interner.resolve(target_owner);
        if !is_cell_target && fog_state.is_friendly(snap_owner_str, target_owner_str) {
            if delayed_building_slot.is_some() {
                return;
            }
            if let Some(new_target) = acquire_best_target(
                entities,
                rules,
                interner,
                snap,
                obj,
                fog,
                garrison_retarget_range,
                terrain.as_deref(),
            ) {
                out.retarget_events.push((snap.stable_id, new_target));
            } else {
                out.remove_attack.push(snap.stable_id);
            }
            return;
        }
        if !is_cell_target && !fog_state.is_cell_visible(snap.owner, target_rx, target_ry) {
            if delayed_building_slot.is_some() {
                return;
            }
            if let Some(new_target) = acquire_best_target(
                entities,
                rules,
                interner,
                snap,
                obj,
                fog,
                garrison_retarget_range,
                terrain.as_deref(),
            ) {
                out.retarget_events.push((snap.stable_id, new_target));
            } else {
                out.remove_attack.push(snap.stable_id);
            }
            return;
        }
    }

    let infantry_fire_sync =
        snap.category == EntityCategory::Infantry && !is_garrison && snap.animation_frame.is_some();
    let mut pending_at_fire_frame = false;
    if infantry_fire_sync {
        if let Some(pending) = snap.pending_infantry_fire {
            if snap.has_movement || snap.animation_sequence != Some(pending.sequence) {
                out.pending_infantry_updates.push((snap.stable_id, None));
                out.animation_switches.push((
                    snap.stable_id,
                    infantry_idle_sequence(snap.is_prone, snap.is_fully_deployed),
                ));
                return;
            }
            if snap.animation_frame != Some(pending.fire_frame) {
                return;
            }
            pending_at_fire_frame = true;
        }
    }

    // Range check (lepton-precise, sub-cell aware).
    // Garrison range: (half_foundation + OccupyWeaponRange) cells (no +1 buffer for fire).
    let effective_range = if let Some(ref gs) = snap.garrison {
        let cells = gs.half_foundation as i32 + rules.garrison_rules.occupy_weapon_range;
        SimFixed::from_num(cells.max(1))
    } else {
        weapon.range
    };
    // Range failure: range alone does not clear or retarget — the pursuit
    // pre-combat stage walks the unit into range. Combat tick just skips
    // this tick's fire attempt and lets the unit close the gap.
    let in_range_for_fire = if !is_garrison && effective_range == weapon.range {
        // Standard fire: 3D check via compute_in_range when terrain available.
        match (terrain.as_deref(), entities.get(snap.stable_id)) {
            (Some(t), Some(attacker_entity)) => {
                let Some(source_z) = in_range::effective_z_leptons(attacker_entity, t) else {
                    return;
                };
                let src = (
                    snap.pos_rx as i64 * 256 + snap.sub_x.to_num::<i64>(),
                    snap.pos_ry as i64 * 256 + snap.sub_y.to_num::<i64>(),
                    source_z,
                );
                in_range::compute_in_range(
                    attacker_entity,
                    src,
                    &snap.target,
                    weapon,
                    rules,
                    interner,
                    entities,
                    t,
                )
            }
            _ => {
                let dist_sq = lepton_distance_sq_raw(
                    snap.pos_rx,
                    snap.pos_ry,
                    snap.sub_x,
                    snap.sub_y,
                    target_rx,
                    target_ry,
                    target_sub_x,
                    target_sub_y,
                );
                is_within_range_leptons(dist_sq, effective_range)
            }
        }
    } else {
        // Garrison override path — preserve 2D until a later stage threads
        // override-aware 3D.
        let dist_sq = lepton_distance_sq_raw(
            snap.pos_rx,
            snap.pos_ry,
            snap.sub_x,
            snap.sub_y,
            target_rx,
            target_ry,
            target_sub_x,
            target_sub_y,
        );
        is_within_range_leptons(dist_sq, effective_range)
    };
    if !in_range_for_fire {
        if pending_at_fire_frame {
            out.pending_infantry_updates.push((snap.stable_id, None));
            out.animation_switches.push((
                snap.stable_id,
                infantry_idle_sequence(snap.is_prone, snap.is_fully_deployed),
            ));
        }
        return;
    }

    // `TechnoClass::GetFireError` 0x006FC0B0 refuses the shot with error 5 when
    // attacker and target disagree on OnBridge, both stand in bridge cells, and
    // the attacker is not high-flying — a unit on the deck and a unit sheltering
    // directly beneath it cannot shoot each other. `InRange` 0x006F7220 already
    // blocks the under-to-over half through
    // `attacker_under_bridge_targeting_above`; this is the over-to-under half
    // and the cases where the height test does not fire.
    //
    // Evaluated here rather than inside `compute_in_range` because the native
    // InRange has no such clause. Like the native it only suppresses the shot:
    // no retarget, no clear, no effect on the pursuit stage.
    if let (Some(t), Some(attacker_entity)) = (terrain.as_deref(), entities.get(snap.stable_id))
        && in_range::fire_error_on_bridge_mismatch(attacker_entity, &snap.target, entities, t)
    {
        if pending_at_fire_frame {
            out.pending_infantry_updates.push((snap.stable_id, None));
            out.animation_switches.push((
                snap.stable_id,
                infantry_idle_sequence(snap.is_prone, snap.is_fully_deployed),
            ));
        }
        return;
    }

    // Burst / cooldown state machine.
    if snap.cooldown_ticks > 0 || snap.burst_delay_ticks > 0 {
        if pending_at_fire_frame {
            out.pending_infantry_updates.push((snap.stable_id, None));
            out.animation_switches.push((
                snap.stable_id,
                infantry_idle_sequence(snap.is_prone, snap.is_fully_deployed),
            ));
        }
        return;
    }

    // Turret alignment check (FacingClass: destination match + not rotating).
    if let Some(ref barrel) = snap.barrel_facing {
        let desired: u16 = crate::sim::movement::turret::facing_toward_lepton(
            snap.pos_rx,
            snap.pos_ry,
            snap.sub_x,
            snap.sub_y,
            target_rx,
            target_ry,
            target_sub_x,
            target_sub_y,
        );
        // Aligned iff destination matches AND no rotation in progress.
        // Both checks needed: destination may match while interpolation
        // is still mid-arc (animated value not yet at destination).
        // RESIDUAL (GSI-08.04) — this gate exists only for entities that have a
        // `barrel_facing`. Turretless vehicles, all infantry and all structures
        // reach the fire step with no facing test at all, so they shoot
        // instantly in any direction; native gates a turretless firer on its
        // body facing.
        // - Trigger: any turretless attacker acquiring a target off its heading.
        // - Player effect: no turn-to-fire delay — an artillery piece or a rifle
        //   infantryman fires the frame it acquires instead of after swinging
        //   round, so first shots land early and units never visibly line up.
        // - Frequency: continuous; infantry alone make this most engagements.
        // - Downstream risk: adding the gate delays first shots, which moves
        //   engagement outcomes and the pinned replay hash, so it wants its own
        //   slice alongside the body-facing rate work.
        let aligned = barrel.current(binary_frame) == desired && !barrel.is_rotating(binary_frame);
        if !aligned {
            if pending_at_fire_frame {
                out.pending_infantry_updates.push((snap.stable_id, None));
                out.animation_switches.push((
                    snap.stable_id,
                    infantry_idle_sequence(snap.is_prone, snap.is_fully_deployed),
                ));
            }
            // FireDecision::Facing — drives gattling spin-up via
            // drives_gattling_spinup() == true.
            return;
        }
    }

    if delayed_building_slot.is_none()
        && snap.category == EntityCategory::Structure
        && !rules
            .general
            .prism_type
            .as_deref()
            .is_some_and(|prism_type| obj.id.eq_ignore_ascii_case(prism_type))
    {
        let delayed_fire_delay = rules
            .art_registry
            .resolve_metadata_entry(&obj.id, &obj.image)
            .filter(|art| art.is_anim_delayed_fire)
            .map(|art| art.delayed_fire_delay);
        if let Some(delay) = delayed_fire_delay {
            // gamemd-derived: the non-Prism generic arm in
            // BuildingClass::Mission_Attack @ 0x0044B630 saves the selected
            // weapon slot and signed delay without firing/rearming. This same
            // BuildingClass::Update visit then enters ProcessDelayedFire @
            // 0x004503F0, so account for its pre-decrement immediately.
            let pending = PendingBuildingFire {
                remaining_ticks: delay.saturating_sub(1).max(0),
                weapon_slot: selected.slot,
            };
            if pending.remaining_ticks != 0 {
                if let Some(entity) = entities.get_mut(snap.stable_id) {
                    entity.pending_building_fire = Some(pending);
                }
                // SpecialAnim presentation and its Report cue are app-layer
                // residuals; they do not authorize early weapon emission.
                return;
            }
        }
    }

    if infantry_fire_sync && !pending_at_fire_frame {
        let sequence =
            infantry_fire_sequence(obj, selected.slot, snap.is_prone, snap.is_fully_deployed);
        let fire_frame =
            infantry_fire_frame(obj, selected.slot, snap.is_prone, snap.is_fully_deployed);
        out.animation_switches.push((snap.stable_id, sequence));
        if fire_frame != 0 {
            out.pending_infantry_updates.push((
                snap.stable_id,
                Some(PendingInfantryFire {
                    sequence,
                    fire_frame,
                }),
            ));
            return;
        }
    }
    if pending_at_fire_frame {
        out.pending_infantry_updates.push((snap.stable_id, None));
    }

    // Spawner weapon: gamemd's Fire_At short-circuits here. It calls
    // `SpawnManagerClass::SetTarget` and returns NULL — no bullet, no damage,
    // no detonation effects, and no rearm timer write (the rearm write lives
    // further down Fire_At, past this branch). Because the branch returns above
    // the first random draw as well as above bullet allocation, a spawner fire
    // consumes zero scenario-RNG draws natively.
    //
    // `TechnoClass::GetFireError` step 18 (`disassemble_bytes 0x006FC606`) adds
    // three gates ahead of the shot, all read off the FIRER, not the target:
    //   1. `TechnoClass__IsOnBridge_ForFiring` (`0x00703B10`) → error 6.
    //      **NOT MODELLED.** It is not a plain "am I on a bridge deck" test —
    //      it is gated on the firer's own OnBridge byte being *clear*, then
    //      samples the firer's cell plus four direction-offset neighbours and
    //      tests cell flags 0x100/0x800 in four different combinations. It
    //      refuses a launch made from under or alongside a bridge span. Trigger:
    //      a V3/Dreadnought/Boomer ordered to fire while standing in a
    //      bridge-adjacent cell. Player effect: VERA launches where retail
    //      refuses; the missile flies where retail would have made the unit hold
    //      fire. Frequency: uncommon — needs the launcher parked under a span,
    //      which players avoid because it blocks line of sight anyway.
    //      Downstream risk: none — the gate only suppresses a launch, it feeds
    //      nothing.
    //
    //      Blocker updated 2026-08-19: the flags and offsets are no longer
    //      UNCHECKED. 0x100 is "cell belongs to a bridge", 0x800 is the span's
    //      axis bit, and the four offsets are `g_DirectionOffsets` 0x0089F688
    //      indices 4/0/2/6 — S, N, E, W — with N and S requiring 0x800 SET and
    //      E and W requiring it CLEAR (table filled at runtime from 0x0049F2F0).
    //      The predicate itself is now ported, as
    //      `app::presentation::instances::helpers::is_on_bridge_for_firing`.
    //      What blocks this gate is placement, not evidence: that function sits
    //      above the sim boundary and `sim/` must never depend on `app/`, so
    //      wiring it here means moving the predicate down to `map/bridge_facts`
    //      first and having both callers read it there.
    //   2. `this->vtable+0x380` non-zero → error 6. **NOT MODELLED**; the slot's
    //      identity is UNCHECKED.
    //   3. `SpawnManagerClass::CountAliveSpawns == 0` → error 3. MODELLED below.
    if weapon.spawner {
        let alive = entities
            .get(snap.stable_id)
            .and_then(|e| e.spawn_manager.as_ref())
            .map(|m| m.count_alive_spawns())
            .unwrap_or(0);
        if alive > 0 {
            out.spawn_target_updates.push((snap.stable_id, snap.target));
        }
        return;
    }

    // Fire one shot!
    let warhead = selected.warhead;
    // Garrison damage: apply OccupyDamageMultiplier to base damage before AoE or
    // single-target paths. Matches gamemd Fire_At which modifies damage before bullet
    // creation, so AoE splash uses the modified value.
    let base_damage = if is_garrison {
        sim_to_i32(
            SimFixed::from_num(weapon.damage) * rules.garrison_rules.occupy_damage_multiplier,
        )
    } else {
        weapon.damage
    };
    let persistent_delivery = classify_projectile_delivery(weapon, rules);
    if let ProjectileDelivery::Persistent {
        arm_frames,
        tracks_target,
        collision,
        ballistic,
        guidance,
    } = persistent_delivery
    {
        let impact_world_z_leptons = attack_world_z_leptons(
            snap.target,
            target_rx,
            target_ry,
            target_sub_x,
            target_sub_y,
            entities,
            terrain.as_deref(),
        );
        let origin_world_z_leptons = entities
            .get(snap.stable_id)
            .map(|entity| object_world_z_leptons(entity, terrain.as_deref()))
            .or(snap.pos_exact_z_leptons)
            .unwrap_or_else(|| i32::from(snap.pos_z).wrapping_mul(LEPTONS_PER_LEVEL as i32));
        let impact = ProjectileCoord::new(
            i32::from(target_rx) * 256 + target_sub_x.to_num::<i32>(),
            i32::from(target_ry) * 256 + target_sub_y.to_num::<i32>(),
            impact_world_z_leptons,
        );
        let target = match snap.target {
            TargetKind::Entity(id) => ProjectileTarget::Entity(id),
            TargetKind::Cell(rx, ry) => ProjectileTarget::Cell { rx, ry },
        };
        // RESIDUAL (GSI-08.04) — the shot leaves the unit's centre, not its
        // barrel. `PrimaryFireFLH=`/`SecondaryFireFLH=` and their elite variants
        // are parsed and transformed (`rules/flh.rs`, `util/flh_transform.rs`),
        // but the only consumer is `app/presentation/fire_effects.rs`, which
        // places the muzzle flash and the report sound. Native's fire location
        // is weapon-slot and barrel-facing dependent and is what the projectile
        // is launched from, what range is measured from, and what line of fire
        // is traced from.
        // - Trigger: every shot from a unit whose FLH is not the origin, which
        //   in stock artmd is most of them.
        // - Player effect: the muzzle flash and the projectile disagree — the
        //   flash sits at the barrel and the shot starts at the hull centre, so
        //   at short range the tracer visibly begins in the wrong place. Range
        //   is also measured from the centre, so a long-barrelled unit reaches
        //   marginally less far than it should.
        // - Frequency: continuous, every shot.
        // - Downstream risk: high. Moving the origin changes ballistic launch
        //   vectors and therefore impact frames, so it moves the pinned replay
        //   hash and the closed ballistic vector tests; it also wants the
        //   turret/barrel facing split recorded on `movement/turret.rs`, since
        //   the native fire location reads the barrel facing rather than the
        //   receiver's.
        let origin = ProjectileCoord::new(
            i32::from(snap.pos_rx) * 256 + snap.sub_x.to_num::<i32>(),
            i32::from(snap.pos_ry) * 256 + snap.sub_y.to_num::<i32>(),
            origin_world_z_leptons,
        );
        let projectile_type = weapon
            .projectile
            .as_deref()
            .and_then(|projectile_id| rules.projectile(projectile_id));
        let gravity = projectile_type
            .map(|projectile| {
                if projectile.floater {
                    rules.general.gravity / 2
                } else {
                    rules.general.gravity
                }
            })
            .unwrap_or(0);
        // The root-selector argument remains ABI-ambiguous in the closed RE;
        // use the proved ordinary (+root) path rather than inferring Lobber.
        let velocity = if ballistic {
            ballistic_launch_velocity(origin, impact, weapon.speed, gravity, false)
                .unwrap_or(ProjectileVelocity::new(0, 0, 0))
        } else {
            ProjectileVelocity::new(0, 0, 0)
        };
        let visual = projectile_type
            .map(|projectile| {
                ProjectileVisualState::new(
                    projectile.anim_low as u8,
                    projectile.anim_high as u8,
                    projectile.anim_rate as u8,
                )
            })
            .unwrap_or_else(|| ProjectileVisualState::new(0, 0, 0));
        out.projectile_spawns.push(ProjectileSpawn {
            source_id: snap.stable_id,
            origin,
            target,
            initial_target_position: impact,
            payload: ProjectilePayload {
                base_damage,
                warhead: interner.intern(&warhead.id),
                weapon: interner.intern(selected.weapon_id),
                owner: snap.owner,
            },
            speed_leptons_per_frame: weapon.speed.clamp(1, i32::from(u16::MAX)) as u16,
            velocity,
            trajectory: if ballistic {
                ProjectileTrajectory::Ballistic { gravity }
            } else {
                ProjectileTrajectory::Straight
            },
            guidance,
            visual,
            arm_frames,
            fuse_frames: None,
            ranged_fuse: tracks_target
                || projectile_type.is_some_and(|projectile| projectile.ranged),
            tracks_target,
            target_expiry: TargetExpiryPolicy::DetonateAtLastKnown,
            collision,
        });
    } else {
        let impact_z = attack_impact_z(snap.target, entities, terrain.as_deref());
        let air_impact = attack_air_impact(
            snap.target,
            target_rx,
            target_ry,
            target_sub_x,
            target_sub_y,
            entities,
            terrain.as_deref(),
        );
        let world_z_leptons = attack_world_z_leptons(
            snap.target,
            target_rx,
            target_ry,
            target_sub_x,
            target_sub_y,
            entities,
            terrain.as_deref(),
        );
        let routed_wall = wall_overlay_flags_at(
            overlay_grid.as_deref(),
            overlay_registry,
            target_rx,
            target_ry,
        )
        .is_some_and(|flags| warhead_damages_wall(warhead, flags));
        let wh_iid = interner.intern(&warhead.id);
        let ore_amount = if scenario_no_damage {
            None
        } else {
            tiberium_reduction_amount(base_damage, true, warhead)
        };
        let aoe = {
            let mut ore_prelude = ore_amount.map(|amount| CombatTiberiumCellPrelude {
                amount,
                deferred: &mut out.tiberium_reduction_requests,
                inline_hooks,
                rules,
                resource_nodes,
                terrain_area_state,
            });
            self::combat_aoe::apply_aoe_damage_with_terrain_and_scenario(
                entities,
                target_rx,
                target_ry,
                base_damage,
                warhead,
                rules,
                interner,
                handles,
                (snap.stable_id, Some(snap.owner), wh_iid),
                self::combat_aoe::AoELayerContext {
                    occupancy: Some(&*occupancy),
                    terrain: terrain.as_deref_mut(),
                    overlay_grid: overlay_grid.as_deref_mut(),
                    overlay_registry,
                    scenario_rng: Some(&mut *scenario_rng),
                    air_impact,
                    impact_z,
                },
                terrain_objects,
                scenario_no_damage,
                ore_prelude
                    .as_mut()
                    .map(|prelude| prelude as &mut dyn self::combat_aoe::AoECellPrelude),
            )
        };
        out.wall_mutations.extend(aoe.wall_mutations);
        out.cell_target_detaches.extend(aoe.cell_target_detaches);

        out.damage_events.extend(aoe.receivers);
        if !scenario_no_damage && base_damage > 0 && !routed_wall && warhead.wall {
            let wh_iid = interner.intern(&warhead.id);
            out.bridge_damage_events.push(BridgeDamageEvent {
                rx: target_rx,
                ry: target_ry,
                damage: base_damage.min(i32::from(u16::MAX)) as u16,
                warhead_ref: wh_iid,
                is_ion_cannon: wh_iid == handles
                        .expect("Simulation::resolve_type_handles must run before combat")
                        .ion_cannon,
                impact_z,
            });
        }
        // Radiation-emitting detonation: one site request per shot at the impact
        // cell. Spread is the warhead's CellSpread truncated to whole cells.
        if weapon.rad_level > 0 {
            out.rad_detonations
                .push(crate::sim::radiation::RadDetonation {
                    rx: target_rx,
                    ry: target_ry,
                    rad_level: weapon.rad_level,
                    spread: warhead.cell_spread.to_num::<i32>(),
                });
        }

        // One impact coordinate: the original engine hands the SAME resolved
        // coord to the area-damage call and to the AnimList placement, so the
        // animation height is the impact height that fed the damage above,
        // never a separately derived value. (Only the smudge dispatcher
        // re-derives a ground reference of its own; the animation is drawn at
        // this z.)
        let effect_z: u8 = impact_z_byte(impact_z);
        // BulletClass::Detonate randomizes only the visible CoordStruct for an
        // Inviso projectile. The draw happens before AnimList selection, so this
        // must run even when the warhead has no animation to emit.
        let (effect_rx, effect_ry, effect_sub_x, effect_sub_y) = if weapon
            .projectile
            .as_deref()
            .and_then(|projectile_id| rules.projectile(projectile_id))
            .is_some_and(|projectile| projectile.inviso)
        {
            inviso_scatter::scatter_inviso_effect_coord(
                scenario_rng,
                target_rx,
                target_ry,
                target_sub_x,
                target_sub_y,
            )
        } else {
            (target_rx, target_ry, target_sub_x, target_sub_y)
        };
        emit_warhead_detonation_effects(
            warhead,
            base_damage,
            effect_rx,
            effect_ry,
            effect_sub_x,
            effect_sub_y,
            effect_z,
            world_z_leptons,
            interner,
            &mut out.explosion_effects,
            &mut out.smudge_spawn_requests,
        );
    }

    let report_sound_id = weapon
        .report
        .as_ref()
        .map(|report_id| interner.intern(report_id));
    let weapon_burst: u8 = weapon.burst.max(1) as u8;
    let burst_index = if weapon_burst <= 1 || snap.burst_remaining == 0 {
        0
    } else {
        weapon_burst
            .saturating_sub(snap.burst_remaining)
            .min(weapon_burst.saturating_sub(1))
    };
    out.fire_events.push(SimFireEvent {
        attacker_id: snap.stable_id,
        attacker_type_ref: snap.type_id,
        weapon_slot: selected.slot,
        weapon_id: interner.intern(selected.weapon_id),
        facing: snap.facing,
        veterancy: snap.veterancy,
        origin_snapshot: FireOriginSnapshot {
            rx: snap.pos_rx,
            ry: snap.pos_ry,
            z: snap.pos_z,
            sub_x: snap.sub_x,
            sub_y: snap.sub_y,
            facing: snap.facing,
            category: snap.category,
            burst_index,
        },
        target: snap.target,
        report_sound_id,
        garrison_muzzle_index: snap.garrison.as_ref().map(|gs| gs.fire_index),
        occupant_anim: if is_garrison {
            weapon.occupant_anim.as_ref().map(|s| interner.intern(s))
        } else {
            None
        },
    });
    if weapon.reveal_on_fire {
        out.reveal_events.push(RevealEvent {
            owner: snap.owner,
            rx: snap.pos_rx,
            ry: snap.pos_ry,
            radius: REVEAL_ON_FIRE_RADIUS,
        });
    }

    let current_remaining: u8 = if snap.burst_remaining == 0 {
        weapon_burst.saturating_sub(1)
    } else {
        snap.burst_remaining.saturating_sub(1)
    };
    if current_remaining > 0 {
        out.burst_updates
            .push((snap.stable_id, current_remaining, BURST_INTER_SHOT_DELAY, 0));
    } else {
        let mut rof_ticks = rof_to_cooldown_frames(weapon.rof);
        // Garrison ROF: divide by occupant count, then by multiplier.
        // More occupants = proportionally faster fire (gamemd GetROF 0x006FCFA0).
        if let Some(ref gs) = snap.garrison {
            let count = (gs.occupant_count as u16).max(1);
            rof_ticks /= count;
            if rules.garrison_rules.occupy_rof_multiplier > SIM_ZERO {
                rof_ticks = sim_to_i32(
                    SimFixed::from_num(rof_ticks) / rules.garrison_rules.occupy_rof_multiplier,
                ) as u16;
            }
            rof_ticks = rof_ticks.max(1);
        }
        out.burst_updates.push((snap.stable_id, 0, 0, rof_ticks));
    }

    // Aircraft ammo deduction: one ammo per burst completion (not per shot).
    if current_remaining == 0 {
        out.ammo_deduct.push(snap.stable_id);
    }

    // Track garrison buildings that fired for round-robin advancement.
    if is_garrison {
        out.garrison_advance.push(snap.stable_id);
    }
}

/// Squared distance in leptons between two positions (sub-cell precise).
///
/// Uses i64 arithmetic to avoid overflow on large maps — a 200-cell lepton
/// delta squared is ~2.6 billion, which exceeds i32 max (2.1 billion).
/// 256 leptons = 1 cell.
#[allow(dead_code)] // Convenience API — callers currently use lepton_distance_sq_raw.
pub(crate) fn lepton_distance_sq(
    a: &crate::sim::components::Position,
    b: &crate::sim::components::Position,
) -> i64 {
    lepton_distance_sq_raw(a.rx, a.ry, a.sub_x, a.sub_y, b.rx, b.ry, b.sub_x, b.sub_y)
}

/// Squared distance in leptons from raw coordinates.
///
/// Same as `lepton_distance_sq` but takes individual fields instead of
/// `&Position`, for use with snapshots where positions are destructured.
pub(crate) fn lepton_distance_sq_raw(
    ax_cell: u16,
    ay_cell: u16,
    ax_sub: SimFixed,
    ay_sub: SimFixed,
    bx_cell: u16,
    by_cell: u16,
    bx_sub: SimFixed,
    by_sub: SimFixed,
) -> i64 {
    let ax: i64 = ax_cell as i64 * 256 + ax_sub.to_num::<i64>();
    let ay: i64 = ay_cell as i64 * 256 + ay_sub.to_num::<i64>();
    let bx: i64 = bx_cell as i64 * 256 + bx_sub.to_num::<i64>();
    let by: i64 = by_cell as i64 * 256 + by_sub.to_num::<i64>();
    let dx: i64 = ax - bx;
    let dy: i64 = ay - by;
    dx * dx + dy * dy
}

/// Check if a squared lepton distance is within weapon range.
///
/// Converts weapon range from cells to leptons (×256) before squaring.
/// Uses i64 to match `lepton_distance_sq()` output.
pub(crate) fn is_within_range_leptons(dist_sq_leptons: i64, range_cells: SimFixed) -> bool {
    let range_leptons: i64 = range_cells.to_num::<i64>() * 256;
    let range_sq: i64 = range_leptons * range_leptons;
    dist_sq_leptons <= range_sq
}

/// `ROF=` is already a native frame count, so the cooldown is the raw value.
///
/// RESIDUAL (GSI-08.05) — nothing modifies it. `[General] VeteranROF=` is
/// present in stock and has no consumer, so a veteran or elite unit reloads at
/// exactly its rookie cadence; the only thing veterancy changes about firing is
/// which weapon an elite selects. `RadialFireSegments=` is not parsed at all
/// (one stock entry), and `BurstDelay=` is not parsed either, so the inter-shot
/// gap inside a burst is the fixed `BURST_INTER_SHOT_DELAY` constant rather
/// than an authored one.
/// - Trigger: any promoted unit firing, and the one stock radial-fire type.
/// - Player effect: promoted units feel slower than they should — a veteran
///   Grizzly gains damage but not rate — and the radial type fires as an
///   ordinary single-target weapon.
/// - Frequency: the veterancy arm is continuous once promotion exists; today it
///   is zero, because nothing promotes (see the GSI-08.12 residual). The
///   `BurstDelay=` arm never fires in stock, which authors the key nowhere.
/// - Downstream risk: `VeteranROF` becomes live the moment promotion lands, so
///   the two want sequencing; changing cadence also moves every combat timing
///   test and the pinned replay hash.
fn rof_to_cooldown_frames(rof_frames: i32) -> u16 {
    rof_frames.clamp(1, u16::MAX as i32) as u16
}

pub use self::combat_targeting::{acquire_best_target_for_entity, tick_retaliation};

/// Impact-height tests: a shot that lands on a ground cell must take that
/// cell's terrain floor height, not a constant. Kept inline because they pin
/// `attack_impact_z` and the single-impact-coordinate wiring that lives in
/// this file.
#[cfg(test)]
mod impact_height_tests {
    use super::*;
    use crate::map::bridge_facts::{BRIDGE_FLAG_STRUCTURAL, BridgeCellFacts};
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::ini_parser::IniFile;
    use crate::sim::intern::test_interner;

    const TEST_GRID: u16 = 16;
    /// Terrain floor used by every raised-ground case here. Chosen because two
    /// levels is what the reported screenshot showed: one whole tile of
    /// vertical error.
    const RAISED_LEVEL: u8 = 2;

    #[test]
    fn gsi_04_11_refinery_survivor_requests_follow_native_sentinel_offsets() {
        let mut requests = Vec::new();
        append_building_smudge_requests(&mut requests, 10, 20, 3, "3x3Refinery");
        let SmudgeSpawnRequest::BuildingCenter {
            foundation_w,
            foundation_h,
            ..
        } = &requests[0]
        else {
            panic!("first request must be the destruction-center mark");
        };
        assert_eq!((*foundation_w, *foundation_h), (3, 3));
        let survivor_cells: Vec<_> = requests[1..]
            .iter()
            .map(|request| match request {
                SmudgeSpawnRequest::BuildingSurvivor { cell_rx, cell_ry } => (*cell_rx, *cell_ry),
                _ => panic!("remaining requests must be survivor marks"),
            })
            .collect();
        assert_eq!(
            survivor_cells,
            vec![
                (10, 20),
                (11, 20),
                (12, 20),
                (10, 21),
                (11, 21),
                (10, 22),
                (11, 22),
                (12, 22),
            ]
        );
        assert!(!survivor_cells.contains(&(12, 21)));
    }

    #[test]
    fn gsi_04_11_fatal_infantry_special_anim_emits_effect_and_smudge_request() {
        let mut interner = test_interner();
        let general = crate::rules::ruleset::GeneralRules::default();
        let cases = [
            (1, None),
            (2, None),
            (3, Some("S_BANG34")),
            (4, Some("FLAMEGUY")),
            (5, Some("ELECTRO")),
            (6, Some("YURIDIE")),
            (7, Some("NUKEDIE")),
            (8, Some("VIRUSD")),
            (9, Some("GENDEATH")),
            (10, Some("BRUTDIE")),
        ];
        for (inf_death, expected_name) in cases {
            let mut effects = Vec::new();
            let mut smudges = Vec::new();
            emit_infantry_death_anim(
                &general,
                inf_death,
                7,
                8,
                SimFixed::from_num(64),
                SimFixed::from_num(192),
                2,
                208,
                &mut interner,
                &mut effects,
                &mut smudges,
            );
            let Some(expected_name) = expected_name else {
                assert!(effects.is_empty(), "InfDeath {inf_death}");
                assert!(smudges.is_empty(), "InfDeath {inf_death}");
                continue;
            };
            assert_eq!(effects.len(), 1, "InfDeath {inf_death}");
            assert_eq!(smudges.len(), 1, "InfDeath {inf_death}");
            assert_eq!(interner.resolve(effects[0].shp_name), expected_name);
            assert_eq!((effects[0].rx, effects[0].ry, effects[0].z), (7, 8, 2));
            let SmudgeSpawnRequest::Anim {
                anim_name,
                rx,
                ry,
                sub_x,
                sub_y,
                world_z_leptons,
            } = &smudges[0]
            else {
                panic!("special death effect must run the Anim smudge start path");
            };
            assert_eq!(interner.resolve(*anim_name), expected_name);
            assert_eq!(
                (
                    *rx,
                    *ry,
                    sub_x.to_num::<i32>(),
                    sub_y.to_num::<i32>(),
                    *world_z_leptons,
                ),
                (7, 8, 64, 192, 208)
            );
        }
    }

    fn terrain_cell(rx: u16, ry: u16, level: u8) -> ResolvedTerrainCell {
        ResolvedTerrainCell {
            rx,
            ry,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level,
            filled_clear: true,
            tileset_index: Some(0),
            land_type: 0,
            yr_cell_land_type: 0,
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: Default::default(),
            speed_costs: Default::default(),
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            height_in_pixels: 0,
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
            bridge_facts: BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0; 3],
            radar_right: [0; 3],
            accepts_smudge: true,
            allows_tiberium: false,
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    fn terrain_at_level(level: u8) -> ResolvedTerrainGrid {
        let cells: Vec<ResolvedTerrainCell> = (0..TEST_GRID)
            .flat_map(|ry| (0..TEST_GRID).map(move |rx| terrain_cell(rx, ry, level)))
            .collect();
        ResolvedTerrainGrid::from_cells(TEST_GRID, TEST_GRID, cells)
    }

    /// Armed tank plus a warhead that emits an impact animation, so a
    /// force-fire produces an observable `ExplosionEffect`.
    fn impact_rules() -> RuleSet {
        let ini = IniFile::from_str(
            "\
[VehicleTypes]\n0=MTNK\n\n\
[InfantryTypes]\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n\n\
[Warheads]\n0=AP\n\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=105mm\n\n\
[105mm]\nDamage=65\nROF=50\nRange=6\nWarhead=AP\n\n\
[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\nAnimList=TWLT070\n",
        );
        RuleSet::from_ini(&ini).expect("impact rules should parse")
    }

    #[test]
    fn cell_target_impact_z_is_the_cells_terrain_floor() {
        let entities = EntityStore::new();
        let flat = terrain_at_level(0);
        let raised = terrain_at_level(RAISED_LEVEL);

        assert_eq!(
            attack_impact_z(TargetKind::Cell(7, 9), &entities, Some(&raised)),
            i32::from(RAISED_LEVEL),
            "a ground-cell impact takes the cell's terrain floor height; a \
             constant 0 here renders the impact one whole tile below the ground \
             it landed on"
        );
        assert_eq!(
            attack_impact_z(TargetKind::Cell(7, 9), &entities, Some(&flat)),
            0,
            "level-0 ground is still zero — that is the value, not the fallback"
        );
        assert_eq!(
            attack_impact_z(TargetKind::Cell(7, 9), &entities, None),
            0,
            "no loaded map means no cell to read"
        );
        assert_eq!(
            attack_impact_z(
                TargetKind::Cell(TEST_GRID + 5, TEST_GRID + 5),
                &entities,
                Some(&raised)
            ),
            0,
            "VERA API boundary, no native equivalent: an off-map cell is not a \
             targetable cell in the first place, so this pins only that the \
             helper stays total, not a game rule about off-map heights"
        );
    }

    #[test]
    fn cell_target_impact_z_is_the_ground_floor_not_the_bridge_aim_point() {
        // A structural bridge cell whose ground floor is RAISED_LEVEL and whose
        // deck sits a full deck height above it.
        let mut cells: Vec<ResolvedTerrainCell> = (0..TEST_GRID)
            .flat_map(|ry| (0..TEST_GRID).map(move |rx| terrain_cell(rx, ry, RAISED_LEVEL)))
            .collect();
        let idx = 9 * TEST_GRID as usize + 7;
        cells[idx].bridge_facts = BridgeCellFacts {
            raw_flags: BRIDGE_FLAG_STRUCTURAL,
            ..BridgeCellFacts::default()
        };
        cells[idx].has_bridge_deck = true;
        cells[idx].bridge_walkable = true;
        cells[idx].bridge_deck_level = RAISED_LEVEL + 4;
        let terrain = ResolvedTerrainGrid::from_cells(TEST_GRID, TEST_GRID, cells);

        let entities = EntityStore::new();
        assert_eq!(
            attack_impact_z(TargetKind::Cell(7, 9), &entities, Some(&terrain)),
            i32::from(RAISED_LEVEL),
            "the impact coordinate is the projectile's own location clamped to \
             the cell's ground height, not the bridge-aware aim point — the \
             deck-adding accessor is reached only for a live object target. The \
             deck term is a recorded residual on `attack_impact_z`, not an \
             oversight"
        );
        assert_ne!(
            attack_impact_z(TargetKind::Cell(7, 9), &entities, Some(&terrain)),
            combat_aoe::bridge_adjusted_impact_z(Some(&terrain), 7, 9),
            "the aim-point helper is a different quantity; if these two ever \
             agree, the deck residual was closed and the bridge-damage Z gate \
             has to be settled in the same change"
        );
    }

    /// The impact byte is a signed level count on both sides of the sim/app
    /// boundary, because the projection decodes it with `as i8`.
    ///
    /// Catches the two-narrowings shape error: clamping into `u8` range lets
    /// 200 through, which the projection reads back as -56 levels and draws
    /// 840 px away, while clamping into `i8` range saturates at the top of the
    /// domain the reader actually decodes.
    #[test]
    fn impact_z_byte_saturates_in_the_signed_domain_the_projection_decodes() {
        for level in [0_i32, 1, 2, 14, 127] {
            assert_eq!(
                i32::from(impact_z_byte(level) as i8),
                level,
                "every reachable map height must survive the round trip"
            );
        }
        assert_eq!(
            impact_z_byte(200) as i8,
            i8::MAX,
            "an over-range height saturates at the top of the signed domain, it \
             does not wrap to a large negative one"
        );
        assert_eq!(impact_z_byte(-40) as i8, -40, "below-ground z stays signed");
        assert_eq!(impact_z_byte(-9000) as i8, i8::MIN);
    }

    #[test]
    fn entity_target_impact_z_still_reads_the_entity_height() {
        let mut entities = EntityStore::new();
        let mut on_deck = GameEntity::test_default(1, "MTNK", "Americans", 7, 9);
        on_deck.position.z = 6;
        entities.insert(on_deck);
        let terrain = terrain_at_level(RAISED_LEVEL);

        assert_eq!(
            attack_impact_z(TargetKind::Entity(1), &entities, Some(&terrain)),
            6,
            "an object target still contributes its own height, terrain or not"
        );
        assert_eq!(
            attack_impact_z(TargetKind::Entity(1), &entities, None),
            6,
            "entity height does not depend on the terrain grid"
        );
        assert_eq!(
            attack_impact_z(TargetKind::Entity(404), &entities, Some(&terrain)),
            0,
            "a vanished target contributes nothing"
        );
    }

    #[test]
    fn force_fire_on_raised_ground_places_the_explosion_at_the_terrain_height() {
        let rules = impact_rules();
        let mut terrain = terrain_at_level(RAISED_LEVEL);
        let mut store = EntityStore::new();
        // `test_interner` snapshots the thread-local, so the entity's type and
        // owner strings must be interned before the snapshot is taken.
        store.insert(GameEntity::test_default(1, "MTNK", "Americans", 5, 5));
        let mut interner = test_interner();
        assert!(
            issue_attack_cell_command(&mut store, 1, 5, 6, Some(&rules), &interner),
            "armed tank should accept a force-fire order on an adjacent cell"
        );

        let mut scenario_rng = SimRng::new(1);
        let result = tick_combat_with_fog(
            &mut store,
            &mut OccupancyGrid::new(),
            &rules,
            &mut interner,
            None,
            &BTreeMap::<InternedId, PowerState>::new(),
            None,
            &mut BTreeMap::new(),
            None,
            None,
            Some(&mut terrain),
            0,
            100,
            0,
            &[1],
            None,
            &mut scenario_rng,
        );

        let effect = result
            .explosion_effects
            .first()
            .expect("force-fire should emit the warhead's impact animation");
        assert_eq!((effect.rx, effect.ry), (5, 6));
        assert_eq!(
            effect.z, RAISED_LEVEL,
            "the impact animation is placed at the impact height; a constant 0 \
             draws it 15 screen pixels per level below the ground it hit"
        );
    }
}
