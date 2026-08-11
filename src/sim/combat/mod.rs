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

use std::collections::BTreeMap;

use crate::sim::miner::ResourceNode;

use self::combat_weapon::{WeaponSlot, select_deploy_fire_weapon, select_weapon_with_override};
use crate::map::entities::EntityCategory;
use crate::map::overlay_types::OverlayTypeRegistry;
use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::RuleSet;
use crate::rules::warhead_type::WarheadType;
use crate::sim::bridge_state::BridgeDamageEvent;
use crate::sim::entity_store::EntityStore;
use crate::sim::house_state::HouseState;
use crate::sim::infantry;
use crate::sim::intern::{InternedId, StringInterner};
use crate::sim::overlay_grid::{OverlayGrid, WallDamageEvent};
use crate::sim::power_system::PowerState;
use crate::sim::projectile::{
    ProjectileCollisionPolicy, ProjectileCoord, ProjectileDetonation, ProjectilePayload,
    ProjectileSpawn, ProjectileTarget, TargetExpiryPolicy,
};
use crate::sim::rng::SimRng;
use crate::sim::vision::FogState;
use crate::sim::world::{FireOriginSnapshot, SimFireEvent, SimSoundEvent};
use crate::util::fixed_math::{SIM_ZERO, SimFixed, sim_to_i32};

use super::animation::SequenceKind;
use super::deploy::DeployPhase;
use super::game_entity::GameEntity;
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
    Ballistic,
    ObstacleTrajectory,
    SpecialTrajectory,
}

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
    if projectile.arcing || weapon.lobber {
        return ProjectileDelivery::Immediate(ImmediateProjectileReason::Ballistic);
    }
    // YR BulletClass::AI linkage: Level's current-cell water predicate and
    // wall entry are now owned by the world collision rung. Cliff/elevation
    // kernels still need their native coordinate contracts.
    if projectile.subject_to_cliffs || projectile.subject_to_elevation {
        return ProjectileDelivery::Immediate(ImmediateProjectileReason::ObstacleTrajectory);
    }
    if projectile.airburst
        || projectile.dropping
        || projectile.very_high
        || projectile.proximity
        || projectile.flak_scatter
        || projectile.inaccurate
        || projectile.degenerates
        || projectile.bouncy
        // BulletTypeClass defaults Cluster to one ordinary impact. Only a
        // value other than that baseline requires the cluster kernel.
        || projectile.cluster != 1
        || projectile.shrapnel_count != 0
    {
        return ProjectileDelivery::Immediate(ImmediateProjectileReason::SpecialTrajectory);
    }
    ProjectileDelivery::Persistent {
        arm_frames: projectile.arm.max(0).min(u16::MAX as i32) as u16,
        tracks_target: projectile.rot > 0,
        collision: ProjectileCollisionPolicy {
            level_non_water: projectile.level,
            subject_to_walls: projectile.subject_to_walls,
        },
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
                },
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

pub(crate) fn apply_prone_damage_modifier(
    target_prone_infantry: bool,
    warhead: &WarheadType,
    damage: i32,
) -> u16 {
    if damage <= 0 {
        return 0;
    }

    let scaled = if target_prone_infantry {
        (damage as u64 * warhead.prone_damage_basis_points as u64 / 10_000) as i32
    } else {
        damage
    };

    scaled.clamp(0, u16::MAX as i32) as u16
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

/// A deferred smudge spawn request emitted from combat death-handling.
/// Drained in `Simulation::advance_tick` after combat resolves but before
/// the ore-growth tick stage so that crater-path `Reduce_Tiberium(6)`
/// land before ore-growth reads tiberium density.
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
        z: i32,
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

/// Emit the warhead's AnimList animation and a paired smudge spawn request
/// for one detonation at (rx, ry, z). Mirrors gamemd's WarheadType::Detonate
/// dispatch into AnimClass::Start: every detonation that spawns an anim
/// also runs the anim's first-frame smudge logic.
///
/// Pushes nothing if `warhead.anim_list` is empty.
///
/// `base_damage` is the post-modifier damage at the impact center; it
/// drives AnimList selection via `damage / 25`, clamped to `len - 1`.
pub(crate) fn emit_warhead_detonation_effects(
    warhead: &WarheadType,
    base_damage: i32,
    rx: u16,
    ry: u16,
    sub_x: SimFixed,
    sub_y: SimFixed,
    z: u8,
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
        z: z as i32,
    });
}

/// Terrain object impact cells emitted by combat and applied by `World`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainDamageEvent {
    pub rx: u16,
    pub ry: u16,
    pub damage: i32,
    pub warhead_ref: InternedId,
}

/// Tiberium reduction request emitted by combat and applied by `World`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TiberiumReductionRequest {
    pub rx: u16,
    pub ry: u16,
    pub amount: u16,
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
    /// Owners who lost their last SpySat building — need full reshroud.
    pub spy_sat_reshroud_owners: Vec<InternedId>,
    /// Bridge impact cells that should apply terrain damage after combat resolution.
    pub bridge_damage_events: Vec<BridgeDamageEvent>,
    /// Wall overlay impact cells that should apply wall damage after combat resolution.
    pub wall_damage_events: Vec<WallDamageEvent>,
    /// Terrain object impact cells that should apply Wood/Immune terrain damage.
    pub terrain_damage_events: Vec<TerrainDamageEvent>,
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
    /// Smudge spawn requests collected during death-handling. Drained by
    /// `Simulation::advance_tick` between combat and ore-growth.
    pub smudge_spawn_requests: Vec<SmudgeSpawnRequest>,
    /// (unit_id, desired 16-bit barrel destination) — computed in the Phase-2
    /// per-object window (pre-death state; own-retarget visible), applied
    /// post-batch by `unit_post::apply_unit_facing`. Transient — never stored,
    /// serialized, or hashed.
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

/// The **one** impact height for an attack, in tile-step level units (signed).
///
/// The original engine forms a single impact coordinate per detonation and
/// hands that same coordinate to area damage and to the animation placement —
/// there is no second Z anywhere on the path. This function is VERA's
/// equivalent single value, and every consumer reads it rather than deriving
/// its own: the AoE object-layer selector, the bridge-damage Z gate, the
/// persistent-projectile impact coordinate, the impact-animation height, and
/// (through `app_fire_effects`) the pixel the tracer ends on. A second
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

/// Look up death weapon AoE data from an ObjectType.
/// Returns (damage, warhead_id) if the entity should deal AoE damage on death.
/// Checks DeathWeapon first, then falls back to primary weapon if Explodes=yes.
fn death_weapon_aoe(
    rules: &RuleSet,
    obj: &ObjectType,
    interner: &mut StringInterner,
) -> Option<(i32, InternedId)> {
    if let Some(ref dw_id) = obj.death_weapon {
        let dw = rules.weapon(dw_id)?;
        let wh_id = dw.warhead.as_ref()?;
        return Some((dw.damage, interner.intern(wh_id)));
    }
    if obj.explodes {
        let pri = rules.weapon(obj.primary.as_ref()?)?;
        let wh_id = pri.warhead.as_ref()?;
        return Some((pri.damage, interner.intern(wh_id)));
    }
    None
}

/// Collected side-effects from processing entity deaths in a single tick.
struct DeathEffects {
    despawned_ids: Vec<u64>,
    immediate_uninit_ids: Vec<u64>,
    structure_destroyed: bool,
    spy_sat_reshroud_owners: Vec<InternedId>,
    destroyed_crewed_buildings: Vec<DestroyedCrewedBuilding>,
    destroyed_garrison_buildings: Vec<DestroyedGarrisonBuilding>,
    explosion_effects: Vec<ExplosionEffect>,
    bridge_damage_events: Vec<BridgeDamageEvent>,
    wall_damage_events: Vec<WallDamageEvent>,
    terrain_damage_events: Vec<TerrainDamageEvent>,
    tiberium_reduction_requests: Vec<TiberiumReductionRequest>,
    death_sounds: Vec<(InternedId, u16, u16)>,
    smudge_spawn_requests: Vec<SmudgeSpawnRequest>,
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

/// Process combat-owned death effects and classify the lifecycle handoff.
///
/// Extracts death side-effects into a `DeathEffects` struct so the caller can apply them
/// (bridge damage, sound events, etc.) without the combat function growing unbounded.
fn handle_entity_deaths(
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    houses: &BTreeMap<InternedId, HouseState>,
    main_rng: &mut SimRng,
    dead_entities: &[u64],
    damage_events: &[(u64, u16, u64, InternedId)],
    _resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: Option<&OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
    current_tick: u64,
) -> DeathEffects {
    let mut death_sounds: Vec<(InternedId, u16, u16)> = Vec::new();
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
        InternedId,
        InternedId,
    )> = Vec::new();
    let mut despawned_ids: Vec<u64> = Vec::new();
    let mut immediate_uninit_ids: Vec<u64> = Vec::new();
    let mut spy_sat_reshroud_owners: Vec<InternedId> = Vec::new();
    let mut destroyed_crewed_buildings: Vec<DestroyedCrewedBuilding> = Vec::new();
    let mut destroyed_garrison_buildings: Vec<DestroyedGarrisonBuilding> = Vec::new();
    let mut explosion_effects: Vec<ExplosionEffect> = Vec::new();
    let mut bridge_damage_events: Vec<BridgeDamageEvent> = Vec::new();
    let mut wall_damage_events: Vec<WallDamageEvent> = Vec::new();
    let mut smudge_spawn_requests: Vec<SmudgeSpawnRequest> = Vec::new();
    let mut structure_destroyed: bool = false;
    for &dead_id in dead_entities {
        let dead_info = entities.get(dead_id).map(|e| {
            if e.category == EntityCategory::Structure {
                structure_destroyed = true;
            }
            (
                e.type_ref,
                e.position.rx,
                e.position.ry,
                e.position.sub_x,
                e.position.sub_y,
                e.position.z,
                e.owner,
                e.animation.is_some(),
                e.category,
            )
        });

        if let Some((type_id, rx, ry, sub_x, sub_y, z, owner, has_animation, category)) = dead_info
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
                if let Some((dmg, wh_id)) = death_weapon_aoe(rules, obj, interner) {
                    death_aoe.push((rx, ry, sub_x, sub_y, z, dmg, wh_id, owner));
                }
                if obj.spy_sat {
                    spy_sat_reshroud_owners.push(owner);
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
                .rfind(|(tid, _, _, _)| *tid == dead_id)
                .and_then(|(_, dmg, _, wh_id)| {
                    rules.warhead(interner.resolve(*wh_id)).map(|wh| (wh, *dmg))
                });

            // Building-destruction smudges: emit one BuildingCenter event plus
            // one BuildingSurvivor event per foundation cell. Drained by
            // `Simulation::advance_tick` (Task 13.5) to write into SmudgeGrid.
            if category == EntityCategory::Structure {
                let foundation = rules
                    .object(interner.resolve(type_id))
                    .map(|obj| foundation_dimensions(&obj.foundation))
                    .unwrap_or((1, 1));
                let foundation_w = foundation.0 as u8;
                let foundation_h = foundation.1 as u8;
                smudge_spawn_requests.push(SmudgeSpawnRequest::BuildingCenter {
                    rx,
                    ry,
                    building_z: z as i32,
                    foundation_w,
                    foundation_h,
                });
                for dy in 0..foundation_h as u16 {
                    for dx in 0..foundation_w as u16 {
                        smudge_spawn_requests.push(SmudgeSpawnRequest::BuildingSurvivor {
                            cell_rx: rx + dx,
                            cell_ry: ry + dy,
                        });
                    }
                }
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
    for (rx, ry, sub_x, sub_y, z, dmg, wh_id, owner_id) in &death_aoe {
        if let Some(warhead) = rules.warhead(interner.resolve(*wh_id)) {
            if *dmg > 0 {
                let damage_u16 = (*dmg).max(0) as u16;
                let wall_flags = wall_overlay_flags_at(overlay_grid, overlay_registry, *rx, *ry);
                if wall_flags.is_some_and(|flags| warhead_damages_wall(warhead, flags)) {
                    wall_damage_events.push(WallDamageEvent {
                        rx: *rx,
                        ry: *ry,
                        damage: damage_u16,
                    });
                } else if wall_flags.is_none() && warhead.wall {
                    let wh_iid = *wh_id;
                    bridge_damage_events.push(BridgeDamageEvent {
                        rx: *rx,
                        ry: *ry,
                        damage: damage_u16,
                        warhead_ref: wh_iid,
                        is_ion_cannon: wh_iid == rules.ion_cannon_warhead_id(),
                        impact_z: *z as i32,
                    });
                }
            }
            let aoe_hits = self::combat_aoe::apply_aoe_damage(
                entities,
                *rx,
                *ry,
                *dmg,
                warhead,
                rules,
                interner,
                interner.resolve(*owner_id),
                self::combat_aoe::AoELayerContext {
                    occupancy: Some(&*occupancy),
                    terrain,
                    impact_z: *z as i32,
                },
            );
            for (target_id, aoe_dmg) in aoe_hits {
                if let Some(target) = entities.get_mut(target_id) {
                    if crate::sim::superweapon::invulnerability::is_invulnerable(
                        target.invulnerability.as_ref(),
                        current_tick as u32,
                    ) {
                        continue;
                    }
                    target.health.current = target.health.current.saturating_sub(aoe_dmg);
                    target.refresh_building_damage_state_gate(rules.general.condition_yellow_x1000);
                    if let Some(obj) = rules.object(interner.resolve(target.type_ref)) {
                        infantry::apply_fear_from_damage(
                            obj,
                            target,
                            aoe_dmg,
                            false,
                            rules.general.condition_red_x1000,
                            rules.general.condition_yellow_x1000,
                        );
                    }
                    // A death explosion kills on behalf of the object that
                    // carried it — a Demo Truck's owner is credited with what its
                    // detonation destroys, and so is the owner of anything caught
                    // in a chain reaction. This path subtracts damage inline
                    // rather than going through the damage-event loop, so the
                    // capture has to happen here too.
                    capture_kill_credit(target, Some(*owner_id), rules, interner);
                }
            }
            // Ore destruction from death explosion.
            destroy_ore_at_impact(
                &mut tiberium_reduction_requests,
                *rx,
                *ry,
                *dmg,
                warhead.cell_spread,
            );
            emit_warhead_detonation_effects(
                warhead,
                *dmg,
                *rx,
                *ry,
                *sub_x,
                *sub_y,
                *z,
                interner,
                &mut explosion_effects,
                &mut smudge_spawn_requests,
            );
        }
    }

    DeathEffects {
        despawned_ids,
        immediate_uninit_ids,
        structure_destroyed,
        spy_sat_reshroud_owners,
        destroyed_crewed_buildings,
        destroyed_garrison_buildings,
        explosion_effects,
        bridge_damage_events,
        wall_damage_events,
        terrain_damage_events: Vec::new(),
        tiberium_reduction_requests,
        death_sounds,
        smudge_spawn_requests,
    }
}

/// Destroy ore/gem resources at cells affected by a warhead detonation.
///
/// Iterates cells in the warhead's CellSpread radius and reduces ore density
/// by `base_damage / 10` at each cell. Matches gamemd's `Apply_area_damage`
/// ore destruction logic (0x00489280).
///
/// ALL warheads destroy ore unconditionally — the `Tiberium=` INI flag only
/// gates vein destruction (not implemented).
fn destroy_ore_at_impact(
    requests: &mut Vec<TiberiumReductionRequest>,
    impact_rx: u16,
    impact_ry: u16,
    base_damage: i32,
    cell_spread: SimFixed,
) {
    let ore_damage = (base_damage / 10).max(0) as u16;
    if ore_damage == 0 {
        return;
    }
    // gamemd cell sweep: count_table[ftol(CellSpread + 0.99)] entries, exact order.
    for &(dx, dy) in self::cell_spread::splash_cells(cell_spread) {
        let cx = impact_rx as i32 + dx as i32;
        let cy = impact_ry as i32 + dy as i32;
        if cx >= 0 && cy >= 0 {
            requests.push(TiberiumReductionRequest {
                rx: cx as u16,
                ry: cy as u16,
                amount: ore_damage,
            });
        }
    }
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
    /// (target_id, damage, attacker_id, warhead_id)
    pub(crate) damage_events: Vec<(u64, u16, u64, InternedId)>,
    /// Radiation-emitting detonations (weapon RadLevel > 0), folded into
    /// `RadiationState` before the damage-application phase.
    pub(crate) rad_detonations: Vec<crate::sim::radiation::RadDetonation>,
    pub(crate) remove_attack: Vec<u64>,
    /// (attacker_id, new_target_id)
    pub(crate) retarget_events: Vec<(u64, u64)>,
    pub(crate) fire_events: Vec<SimFireEvent>,
    pub(crate) reveal_events: Vec<RevealEvent>,
    pub(crate) bridge_damage_events: Vec<BridgeDamageEvent>,
    pub(crate) wall_damage_events: Vec<WallDamageEvent>,
    pub(crate) terrain_damage_events: Vec<TerrainDamageEvent>,
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
    /// (unit_id, desired 16-bit barrel destination) — computed in the Phase-2
    /// per-object window (pre-death state; own-retarget visible), applied
    /// post-batch by `unit_post::apply_unit_facing`.
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

/// Reuse the ordinary combat emission paths after a persistent bullet reaches
/// `BulletClass::Detonate`; this stays before the shared damage/death phases.
fn emit_projectile_detonations(
    detonations: &[ProjectileDetonation],
    entities: &mut EntityStore,
    occupancy: &OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    overlay_grid: Option<&OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
    out: &mut CombatEmit,
) {
    for detonation in detonations {
        let Some(warhead) = rules.warhead(interner.resolve(detonation.payload.warhead)) else {
            log::warn!(
                "Projectile {} dropped: missing serialized warhead {}",
                detonation.projectile_id,
                detonation.payload.warhead
            );
            continue;
        };
        let (impact_rx, impact_ry, impact_sub_x, impact_sub_y, impact_z) =
            projectile_impact_cell(detonation.impact);
        let direct_target = match detonation.target {
            ProjectileTarget::Entity(id) => entities
                .get(id)
                .filter(|entity| entity.is_alive() && !entity.dying),
            ProjectileTarget::Cell(_) => None,
        };

        if warhead.cell_spread > SIM_ZERO {
            let aoe_hits = self::combat_aoe::apply_aoe_damage(
                entities,
                impact_rx,
                impact_ry,
                detonation.payload.base_damage,
                warhead,
                rules,
                interner,
                interner.resolve(detonation.payload.owner),
                self::combat_aoe::AoELayerContext {
                    occupancy: Some(occupancy),
                    terrain,
                    impact_z,
                },
            );
            for (target_id, damage) in aoe_hits {
                out.damage_events.push((
                    target_id,
                    damage,
                    detonation.source_id,
                    detonation.payload.warhead,
                ));
            }
        } else if let Some(target) = direct_target {
            let armor = rules
                .object(interner.resolve(target.type_ref))
                .map(|object| object.armor.as_str())
                .unwrap_or("none");
            let raw_damage =
                detonation.payload.base_damage * warhead.verses[armor_index(armor)] as i32 / 100;
            let prone = target.category == EntityCategory::Infantry
                && infantry::is_prone_for_damage(target);
            let actual_damage = apply_prone_damage_modifier(prone, warhead, raw_damage);
            if actual_damage > 0 {
                out.damage_events.push((
                    target.stable_id,
                    actual_damage,
                    detonation.source_id,
                    detonation.payload.warhead,
                ));
            }
        }

        if detonation.payload.base_damage > 0 {
            let damage = detonation.payload.base_damage.min(i32::from(u16::MAX)) as u16;
            let wall_flags =
                wall_overlay_flags_at(overlay_grid, overlay_registry, impact_rx, impact_ry);
            if wall_flags.is_some_and(|flags| warhead_damages_wall(warhead, flags)) {
                out.wall_damage_events.push(WallDamageEvent {
                    rx: impact_rx,
                    ry: impact_ry,
                    damage,
                });
            } else if wall_flags.is_none() && warhead.wall {
                out.bridge_damage_events.push(BridgeDamageEvent {
                    rx: impact_rx,
                    ry: impact_ry,
                    damage,
                    warhead_ref: detonation.payload.warhead,
                    is_ion_cannon: detonation.payload.warhead == rules.ion_cannon_warhead_id(),
                    impact_z,
                });
            }
        }
        if warhead.wood && detonation.payload.base_damage > 0 {
            out.terrain_damage_events.push(TerrainDamageEvent {
                rx: impact_rx,
                ry: impact_ry,
                damage: detonation.payload.base_damage,
                warhead_ref: detonation.payload.warhead,
            });
        }
        destroy_ore_at_impact(
            &mut out.tiberium_reduction_requests,
            impact_rx,
            impact_ry,
            detonation.payload.base_damage,
            warhead.cell_spread,
        );
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
        emit_warhead_detonation_effects(
            warhead,
            detonation.payload.base_damage,
            impact_rx,
            impact_ry,
            impact_sub_x,
            impact_sub_y,
            impact_z_byte(impact_z),
            interner,
            &mut out.explosion_effects,
            &mut out.smudge_spawn_requests,
        );
    }
}

/// Advance combat with optional owner visibility gating and sound event sink.
/// Returns reveal events and stable IDs of entities despawned this tick.
///
/// `overlay_grid` and `overlay_registry` are used to discriminate wall-overlay
/// cells from bridge cells when a wall-warhead detonates (so the right event
/// stream — WallDamageEvent vs BridgeDamageEvent — gets populated). Pass
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
    overlay_grid: Option<&OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
    current_tick: u64,
    tick_ms: u32,
    binary_frame: u32,
    live_order: &[u64],
    radiation: Option<&mut crate::sim::radiation::RadiationState>,
    scenario_rng: &mut SimRng,
) -> CombatTickResult {
    let mut unused_main_rng = SimRng::new(0);
    tick_combat_with_fog_and_main_rng(
        entities,
        occupancy,
        rules,
        interner,
        fog,
        power_states,
        &BTreeMap::new(),
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
        radiation,
        &[],
        scenario_rng,
        &mut unused_main_rng,
    )
}

/// Advance combat with persistent owner state and both native RNG authorities.
///
/// Main-RNG death-sound selection is independent from Scenario-RNG projectile
/// and impact effects, so both streams are explicit at the production seam.
pub fn tick_combat_with_fog_and_main_rng(
    entities: &mut EntityStore,
    occupancy: &mut OccupancyGrid,
    rules: &RuleSet,
    interner: &mut StringInterner,
    fog: Option<&FogState>,
    power_states: &BTreeMap<InternedId, PowerState>,
    houses: &BTreeMap<InternedId, HouseState>,
    sound_sink: Option<&mut Vec<SimSoundEvent>>,
    resource_nodes: &mut BTreeMap<(u16, u16), ResourceNode>,
    overlay_grid: Option<&OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
    current_tick: u64,
    tick_ms: u32,
    binary_frame: u32,
    live_order: &[u64],
    projectile_detonations: &[ProjectileDetonation],
    radiation: Option<&mut crate::sim::radiation::RadiationState>,
    missile_detonations: &[crate::sim::spawn_manager::MissileDetonation],
    scenario_rng: &mut SimRng,
    main_rng: &mut SimRng,
) -> CombatTickResult {
    if tick_ms == 0 {
        return CombatTickResult {
            projectile_spawns: Vec::new(),
            reveal_events: Vec::new(),
            despawned_ids: Vec::new(),
            immediate_uninit_ids: Vec::new(),
            structure_destroyed: false,
            spy_sat_reshroud_owners: Vec::new(),
            bridge_damage_events: Vec::new(),
            wall_damage_events: Vec::new(),
            terrain_damage_events: Vec::new(),
            tiberium_reduction_requests: Vec::new(),
            fire_events: Vec::new(),
            destroyed_crewed_buildings: Vec::new(),
            destroyed_garrison_buildings: Vec::new(),
            explosion_effects: Vec::new(),
            smudge_spawn_requests: Vec::new(),
            unit_facing: Vec::new(),
            under_attack_events: Vec::new(),
        };
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
        // Mutable borrow: tick cooldowns and capture the per-attacker scalars +
        // garrison cargo info. Entity field-reads move into `build_attacker_snapshot`
        // (pure) below, after this borrow releases.
        let (
            attack_target,
            cooldown_ticks,
            burst_remaining,
            burst_delay_ticks,
            pending_infantry_fire,
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
            let attack = match entity.attack_target.as_mut() {
                Some(a) => a,
                None => continue,
            };
            attack.cooldown_ticks = attack.cooldown_ticks.saturating_sub(1);
            attack.burst_delay_ticks = attack.burst_delay_ticks.saturating_sub(1);
            let attack_target = attack.target;
            let cooldown_ticks = attack.cooldown_ticks;
            let burst_remaining = attack.burst_remaining;
            let burst_delay_ticks = attack.burst_delay_ticks;
            let pending_infantry_fire = attack.pending_infantry_fire;
            // Skip snapshot for entities blocked by locomotor state (cooldowns still tick).
            if fire_blocked.contains(&id) {
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

    // Phase 2: per-attacker fire decision + emission, in live-LOGIC snapshot
    // order. Each attacker is resolved through `resolve_attacker_fire` (the
    // reusable per-object fire body); emission order is identical to the prior
    // inline loop, preserving both event order and inline Scenario-RNG draws.
    // Fire is category-agnostic (Units fire through the same body here); Unit
    // FACING destinations are computed per-object right after each Unit's own
    // resolution (S3 post-Foot Fire→Facing order) and applied post-batch by
    // `unit_post::apply_unit_facing`.
    let mut emit = CombatEmit::default();
    for snap in &snapshots {
        let n_retarget = emit.retarget_events.len();
        let n_remove = emit.remove_attack.len();
        resolve_attacker_fire(
            snap,
            entities,
            rules,
            interner,
            fog,
            occupancy,
            overlay_grid,
            overlay_registry,
            terrain,
            binary_frame,
            tick_ms,
            scenario_rng,
            &mut emit,
        );
        // S3: per-object barrel destination for Unit attackers, read in the
        // per-object window — deaths/clears (Phases 3-6) are not yet applied,
        // so a unit whose target dies this tick still aims at it (idle-return
        // begins next tick); a unit whose own resolution retargeted aims at
        // the new target now; one whose own resolution cleared returns to body.
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
        let desired: u16 = if let Some(tid) = own_retarget {
            match entities.get(tid) {
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
            }
        } else if own_removed {
            crate::sim::movement::turret::body_facing_to_turret(e.facing)
        } else {
            match crate::sim::movement::turret::desired_turret_facing(e, entities) {
                Some(d) => d,
                // Unreachable: barrel_facing presence checked above.
                None => continue,
            }
        };
        emit.unit_facing.push((snap.stable_id, desired));
    }
    // S3 residual: every Unit not in the attacker snapshot set (target-less,
    // or in-transport holders excluded at the snapshot build). Iterates the
    // SAME keys_sorted() coverage the legacy tick_unit_facing pass had —
    // including limbo/dying Units — so the only output delta vs. the legacy
    // pass is the pre-death read window (placement before Phases 3-6 is
    // semantic: those phases clear attack_target on finished attackers and
    // dead targets). Per-entity independent → id order is output-neutral.
    {
        let mut computed: Vec<u64> = emit.unit_facing.iter().map(|&(id, _)| id).collect();
        computed.sort_unstable();
        for &id in &keys {
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
    // YR BulletClass::Detonate: completed prior-frame bullets enter the same
    // combat damage/death pipeline as all other authoritative detonations.
    emit_projectile_detonations(
        projectile_detonations,
        entities,
        occupancy,
        rules,
        interner,
        overlay_grid,
        overlay_registry,
        terrain,
        &mut emit,
    );
    // Destructure back into the named locals so Phases 3-6 are untouched.
    let CombatEmit {
        projectile_spawns,
        mut damage_events,
        rad_detonations,
        mut remove_attack,
        retarget_events,
        fire_events,
        reveal_events,
        mut bridge_damage_events,
        mut wall_damage_events,
        mut terrain_damage_events,
        mut tiberium_reduction_requests,
        mut explosion_effects,
        mut smudge_spawn_requests,
        burst_updates,
        ammo_deduct,
        garrison_advance,
        pending_infantry_updates,
        animation_switches,
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
    if let Some(rad) = radiation {
        for &det in &rad_detonations {
            rad.apply_detonation(det, binary_frame, &rules.radiation, terrain);
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
                    // trunc(level × RadLevelFactor), then the live-path Verses
                    // scaling against the victim's armor.
                    let base = (level as f64 * rules.radiation.level_factor) as i32;
                    if base <= 0 {
                        continue;
                    }
                    let armor = rules
                        .object(interner.resolve(entity.type_ref))
                        .map(|o| o.armor.as_str())
                        .unwrap_or("none");
                    let verses_pct = rad_warhead
                        .verses
                        .get(armor_index(armor))
                        .copied()
                        .unwrap_or(100);
                    let dmg = base * verses_pct as i32 / 100;
                    if dmg > 0 {
                        damage_events.push((
                            id,
                            dmg.min(u16::MAX as i32) as u16,
                            RAD_NO_ATTACKER,
                            wh_iid,
                        ));
                    }
                }
            }
        }
    }

    // Phase 3.9: spawn-manager missile impacts recorded during this tick's
    // movement pass. gamemd's `RocketLocomotion::Detonate` calls the engine's
    // shared area-damage routine, so the impact must reach the same
    // damage → death → despawn pipeline as any other detonation — including
    // death weapons, crew ejection and kill credit. Folded in here, ahead of
    // Phase 4, in the order the missiles landed.
    for det in missile_detonations {
        let warhead_name = interner.resolve(det.warhead).to_string();
        let Some(warhead) = rules.warhead(&warhead_name) else {
            continue;
        };
        let owner_house = interner.resolve(det.owner).to_string();
        let impact_z = combat_aoe::bridge_adjusted_impact_z(terrain, det.rx, det.ry);
        let hits = combat_aoe::apply_aoe_damage(
            entities,
            det.rx,
            det.ry,
            det.damage,
            warhead,
            rules,
            interner,
            &owner_house,
            combat_aoe::AoELayerContext {
                occupancy: Some(&*occupancy),
                terrain,
                impact_z,
            },
        );
        let wh_iid = interner.intern(&warhead.id);
        for (target_id, damage) in hits {
            damage_events.push((target_id, damage, det.firer_id, wh_iid));
        }
        // Same single impact coordinate the area damage above consumed — the
        // missile's detonation animation sits at the impact height, not at
        // sea level.
        emit_warhead_detonation_effects(
            warhead,
            det.damage,
            det.rx,
            det.ry,
            crate::util::lepton::CELL_CENTER_LEPTON,
            crate::util::lepton::CELL_CENTER_LEPTON,
            impact_z_byte(impact_z),
            interner,
            &mut explosion_effects,
            &mut smudge_spawn_requests,
        );
    }

    // Phase 4: apply damage to targets and track last attacker for retaliation.
    let mut dead_entities: Vec<u64> = Vec::new();
    let mut under_attack_events: Vec<UnderAttackEvent> = Vec::new();
    for (target_id, damage, attacker_id, _wh_id) in &damage_events {
        // Attacker owner read before the target's mutable borrow. None for
        // sourceless damage (radiation) or an already-despawned attacker.
        let attacker_owner: Option<InternedId> = if *attacker_id != RAD_NO_ATTACKER {
            entities.get(*attacker_id).map(|a| a.owner)
        } else {
            None
        };
        if let Some(target) = entities.get_mut(*target_id) {
            if crate::sim::superweapon::invulnerability::is_invulnerable(
                target.invulnerability.as_ref(),
                current_tick as u32,
            ) {
                // Damage fully nullified by IronCurtain/ForceShield.
                // Flash-effect spawn deferred (see design doc Open Questions).
                if *attacker_id != RAD_NO_ATTACKER {
                    target.last_attacker_id = Some(*attacker_id);
                }
                continue;
            }
            target.health.current = target.health.current.saturating_sub(*damage);
            target.refresh_building_damage_state_gate(rules.general.condition_yellow_x1000);
            if let Some(obj) = rules.object(interner.resolve(target.type_ref)) {
                infantry::apply_fear_from_damage(
                    obj,
                    target,
                    *damage,
                    true,
                    rules.general.condition_red_x1000,
                    rules.general.condition_yellow_x1000,
                );
            }
            if target.health.current == 0 {
                dead_entities.push(*target_id);
                // Score-screen kill record, taken here because this is the
                // instant of destruction — the same point gamemd records a kill.
                // Reading it later would be unsafe: the retaliation pass clears
                // `last_attacker_id` unconditionally later this tick, and dying
                // infantry stay in the logic vector through their death
                // animation, so they would reach removal with no attacker left.
                capture_kill_credit(target, attacker_owner, rules, interner);
            }
            // Under-attack ping: another house damaged a base structure or a
            // harvester. Owner-differs is the hostility gate — alliances are
            // not in scope in this pass; allied splash is rare and self-damage
            // never pings, which matches the observable contract.
            if *damage > 0 && attacker_owner.is_some_and(|ao| ao != target.owner) {
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
            // Sourceless damage (radiation field) never arms retaliation and
            // must not overwrite a real attacker recorded this tick.
            if *attacker_id != RAD_NO_ATTACKER {
                target.last_attacker_id = Some(*attacker_id);
            }
        }
    }

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

    // Phase 6: handle death effects — death weapons, passengers, explosions, despawn.
    let death = handle_entity_deaths(
        entities,
        occupancy,
        rules,
        interner,
        houses,
        main_rng,
        &dead_entities,
        &damage_events,
        resource_nodes,
        overlay_grid,
        overlay_registry,
        terrain,
        current_tick,
    );
    bridge_damage_events.extend(death.bridge_damage_events);
    wall_damage_events.extend(death.wall_damage_events);
    terrain_damage_events.extend(death.terrain_damage_events);
    tiberium_reduction_requests.extend(death.tiberium_reduction_requests);
    explosion_effects.extend(death.explosion_effects);
    smudge_spawn_requests.extend(death.smudge_spawn_requests);

    // Phase 7: push sound events to the sink.
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
            dead_entities.len(),
        );
    }

    CombatTickResult {
        projectile_spawns,
        reveal_events,
        despawned_ids: death.despawned_ids,
        immediate_uninit_ids: death.immediate_uninit_ids,
        structure_destroyed: death.structure_destroyed,
        spy_sat_reshroud_owners: death.spy_sat_reshroud_owners,
        bridge_damage_events,
        wall_damage_events,
        terrain_damage_events,
        tiberium_reduction_requests,
        fire_events,
        destroyed_crewed_buildings: death.destroyed_crewed_buildings,
        destroyed_garrison_buildings: death.destroyed_garrison_buildings,
        explosion_effects,
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
/// wins — a second lethal hit in the same tick cannot re-credit the kill to a
/// different house. The award is resolved here because the rules are in hand and
/// the veterancy is still the value the object died at.
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
/// READ-ONLY w.r.t. entities/occupancy (HP/death are applied later in the batched
/// Phase 4/6); it reads target/rules/occupancy/fog and pushes events into `out`.
/// Interns warhead/weapon/anim strings (hence `&mut StringInterner`). Pure w.r.t.
/// iteration order: the caller invokes it once per snapshot in live-LOGIC order,
/// preserving emission order exactly.
pub(crate) fn resolve_attacker_fire(
    snap: &AttackerSnapshot,
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &mut StringInterner,
    fog: Option<&FogState>,
    occupancy: &OccupancyGrid,
    overlay_grid: Option<&OverlayGrid>,
    overlay_registry: Option<&OverlayTypeRegistry>,
    terrain: Option<&crate::map::resolved_terrain::ResolvedTerrainGrid>,
    binary_frame: u32,
    tick_ms: u32,
    scenario_rng: &mut SimRng,
    out: &mut CombatEmit,
) {
    // Pre-compute garrison scan range for retargeting (includes +1 buffer).
    let garrison_retarget_range: Option<SimFixed> = snap.garrison.as_ref().map(|gs| {
        let cells = gs.half_foundation as i32 + 1 + rules.garrison_rules.occupy_weapon_range;
        SimFixed::from_num(cells.max(1))
    });
    let obj = match rules.object(interner.resolve(snap.type_id)) {
        Some(o) => o,
        None => {
            out.remove_attack.push(snap.stable_id);
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
        target_prone_infantry,
    ) = match target_data {
        Some((rx, ry, sx, sy, hp, cat, tr, own, prone)) if hp > 0 => {
            (rx, ry, sx, sy, hp, cat, tr, own, prone)
        }
        _ => {
            if let Some(new_target) = acquire_best_target(
                entities,
                rules,
                interner,
                snap,
                obj,
                fog,
                garrison_retarget_range,
                terrain,
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
    let (selected, is_garrison) = if let Some(ref gs) = snap.garrison {
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

    // Friendly-fire and visibility-driven retarget logic only applies to
    // Entity targets. Cell targets are an explicit player force-fire — the
    // player intentionally chose this cell (allies, ground, anything), so
    // never auto-retarget away from a Cell.
    let is_cell_target = matches!(snap.target, TargetKind::Cell(_, _));
    if let Some(fog_state) = fog {
        let snap_owner_str = interner.resolve(snap.owner);
        let target_owner_str = interner.resolve(target_owner);
        if !is_cell_target && fog_state.is_friendly(snap_owner_str, target_owner_str) {
            if let Some(new_target) = acquire_best_target(
                entities,
                rules,
                interner,
                snap,
                obj,
                fog,
                garrison_retarget_range,
                terrain,
            ) {
                out.retarget_events.push((snap.stable_id, new_target));
            } else {
                out.remove_attack.push(snap.stable_id);
            }
            return;
        }
        if !is_cell_target && !fog_state.is_cell_visible(snap.owner, target_rx, target_ry) {
            if let Some(new_target) = acquire_best_target(
                entities,
                rules,
                interner,
                snap,
                obj,
                fog,
                garrison_retarget_range,
                terrain,
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
        match (terrain, entities.get(snap.stable_id)) {
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
    //      nothing. The exact 0x100/0x800 flag meanings and the four direction
    //      offsets are UNCHECKED and must be traced before implementing.
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
    } = persistent_delivery
    {
        let impact = ProjectileCoord::new(
            i32::from(target_rx) * 256 + target_sub_x.to_num::<i32>(),
            i32::from(target_ry) * 256 + target_sub_y.to_num::<i32>(),
            attack_impact_z(snap.target, entities, terrain),
        );
        let target = match snap.target {
            TargetKind::Entity(id) => ProjectileTarget::Entity(id),
            TargetKind::Cell(_, _) => ProjectileTarget::Cell(impact),
        };
        out.projectile_spawns.push(ProjectileSpawn {
            source_id: snap.stable_id,
            origin: ProjectileCoord::new(
                i32::from(snap.pos_rx) * 256 + snap.sub_x.to_num::<i32>(),
                i32::from(snap.pos_ry) * 256 + snap.sub_y.to_num::<i32>(),
                i32::from(snap.pos_z),
            ),
            target,
            initial_target_position: impact,
            payload: ProjectilePayload {
                base_damage,
                warhead: interner.intern(&warhead.id),
                weapon: interner.intern(selected.weapon_id),
                owner: snap.owner,
            },
            speed_leptons_per_frame: weapon.speed.clamp(1, i32::from(u16::MAX)) as u16,
            arm_frames,
            fuse_frames: None,
            tracks_target,
            target_expiry: TargetExpiryPolicy::DetonateAtLastKnown,
            collision,
        });
    } else {
        let impact_z = attack_impact_z(snap.target, entities, terrain);
        if warhead.cell_spread > SIM_ZERO {
            let aoe_hits = self::combat_aoe::apply_aoe_damage(
                entities,
                target_rx,
                target_ry,
                base_damage,
                warhead,
                rules,
                interner,
                interner.resolve(snap.owner),
                self::combat_aoe::AoELayerContext {
                    occupancy: Some(&*occupancy),
                    terrain,
                    impact_z,
                },
            );
            for (target_id, dmg) in aoe_hits {
                let wh_iid = interner.intern(&warhead.id);
                out.damage_events
                    .push((target_id, dmg, snap.stable_id, wh_iid));
            }
            if base_damage > 0 {
                let damage_u16 = base_damage as u16;
                let wall_flags =
                    wall_overlay_flags_at(overlay_grid, overlay_registry, target_rx, target_ry);
                if wall_flags.is_some_and(|flags| warhead_damages_wall(warhead, flags)) {
                    out.wall_damage_events.push(WallDamageEvent {
                        rx: target_rx,
                        ry: target_ry,
                        damage: damage_u16,
                    });
                } else if wall_flags.is_none() && warhead.wall {
                    let wh_iid = interner.intern(&warhead.id);
                    out.bridge_damage_events.push(BridgeDamageEvent {
                        rx: target_rx,
                        ry: target_ry,
                        damage: damage_u16,
                        warhead_ref: wh_iid,
                        is_ion_cannon: wh_iid == rules.ion_cannon_warhead_id(),
                        impact_z,
                    });
                }
            }
            if warhead.wood && base_damage > 0 {
                out.terrain_damage_events.push(TerrainDamageEvent {
                    rx: target_rx,
                    ry: target_ry,
                    damage: base_damage,
                    warhead_ref: interner.intern(&warhead.id),
                });
            }
        } else {
            // Integer damage: base_damage * verses_pct / 100.
            // base_damage already includes OccupyDamageMultiplier for garrison.
            let raw_damage: i32 = base_damage * selected.verses_pct as i32 / 100;
            let actual_damage: u16 =
                apply_prone_damage_modifier(target_prone_infantry, warhead, raw_damage);
            // Direct-hit damage only applies to Entity targets. For Cell
            // targets (force-fire on terrain), splash logic via warhead
            // CellSpread handles AoE damage at the impact cell — there's no
            // primary target entity to damage.
            if actual_damage > 0 {
                if let TargetKind::Entity(target_id) = snap.target {
                    let wh_iid = interner.intern(&warhead.id);
                    out.damage_events
                        .push((target_id, actual_damage, snap.stable_id, wh_iid));
                }
            }
            if base_damage > 0 {
                let damage_u16 = base_damage as u16;
                let wall_flags =
                    wall_overlay_flags_at(overlay_grid, overlay_registry, target_rx, target_ry);
                if wall_flags.is_some_and(|flags| warhead_damages_wall(warhead, flags)) {
                    out.wall_damage_events.push(WallDamageEvent {
                        rx: target_rx,
                        ry: target_ry,
                        damage: damage_u16,
                    });
                } else if wall_flags.is_none() && warhead.wall {
                    let wh_iid = interner.intern(&warhead.id);
                    out.bridge_damage_events.push(BridgeDamageEvent {
                        rx: target_rx,
                        ry: target_ry,
                        damage: damage_u16,
                        warhead_ref: wh_iid,
                        is_ion_cannon: wh_iid == rules.ion_cannon_warhead_id(),
                        impact_z,
                    });
                }
            }
            if warhead.wood && base_damage > 0 {
                out.terrain_damage_events.push(TerrainDamageEvent {
                    rx: target_rx,
                    ry: target_ry,
                    damage: base_damage,
                    warhead_ref: interner.intern(&warhead.id),
                });
            }
        }

        // Ore destruction: all warheads unconditionally destroy ore at impact cells.
        // CellSpreadTable[0] = 1, so even CellSpread=0 weapons check the center cell.
        destroy_ore_at_impact(
            &mut out.tiberium_reduction_requests,
            target_rx,
            target_ry,
            base_damage,
            warhead.cell_spread,
        );

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

/// Check if a squared cell distance is within weapon range.
/// Compares entirely in u32 to avoid I16F16 overflow on large maps
/// (dist_sq can exceed SimFixed max of 32,767 for distant entities).
pub(crate) fn is_within_weapon_range_sq(dist_sq_cells: u32, range_cells: SimFixed) -> bool {
    let range_i64: i64 = sim_to_i32(range_cells) as i64;
    let range_sq: u32 = (range_i64 * range_i64) as u32;
    dist_sq_cells <= range_sq
}

pub(crate) fn cell_distance_sq(ax: u16, ay: u16, bx: u16, by: u16) -> u32 {
    let dx = ax as i64 - bx as i64;
    let dy = ay as i64 - by as i64;
    (dx * dx + dy * dy) as u32
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
        let terrain = terrain_at_level(RAISED_LEVEL);
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
            Some(&terrain),
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
