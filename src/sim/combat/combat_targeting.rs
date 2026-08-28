//! Target acquisition and retaliation for the combat system.
//!
//! Handles two responsibilities:
//! 1. **Target acquisition** — finding the best hostile target for an idle or
//!    attack-moving unit within its guard/weapon range.
//! 2. **Retaliation** — idle units automatically attack the entity that hit them.
//!
//! ## Target priority
//! When multiple valid targets exist, the nearest is preferred. Ties are broken
//! by threat class (armed units > unarmed > buildings) and stable entity ID
//! (for deterministic replay).
//!
//! ## Scan radius
//! How far the scan reaches is a property of the attacker and the mission it is
//! on, not of the candidate — see [`super::threat_range`]. A unit on Area Guard
//! acquires roughly twice as far out as the same unit on plain Guard.
//!
//! ## Auto-deploy on target acquisition
//! Targeting NEVER initiates a deploy transition. A walking GGI that acquires
//! an air target uses its Secondary weapon in place — it does not auto-deploy.
//! This matches the original's behavior: deploy is a player-driven command,
//! never triggered by AI target acquisition. Verified by grepping every writer
//! of `deploy_state` — only the player command handler and the deploy tick
//! advance set it.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/ (RuleSet) and sim/components.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeMap;

use super::combat_weapon::{VersesGate, select_weapon_with_override, verses_gate};
use super::threat_range::{ScanMission, ScanRange, scan_mission_for, scan_range};
use super::{armor_index, combat_target_category, is_within_range_leptons, lepton_distance_sq_raw};
use crate::map::entities::EntityCategory;
use crate::map::houses::{HouseAllianceMap, is_allied_with};
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::object_type::ObjectCategory;
use crate::rules::ruleset::RuleSet;
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::house_state::HouseState;
use crate::sim::intern::{InternedId, StringInterner};
use crate::sim::vision::FogState;
use crate::util::fixed_math::SimFixed;
use crate::util::lepton::LEPTONS_PER_LEVEL;
use crate::util::native_x87::{NativeF64Bits, X87Chop53, X87Ordering, X87Value, sqrt_approx_f32};

/// Snapshot of garrison state for a garrisoned building attacker.
/// Extracted during Phase 1 to avoid borrow conflicts in Phase 2.
#[derive(Clone)]
pub(crate) struct GarrisonSnapshot {
    /// Type ID of the occupant that will fire this tick.
    pub occupant_type_id: InternedId,
    /// Veterancy of the firing occupant (for elite weapon selection).
    pub occupant_veterancy: u16,
    /// Current round-robin fire index.
    pub fire_index: u8,
    /// Total occupant count (for ROF division).
    pub occupant_count: u8,
    /// Half foundation size: `min(width, height) / 2` (for range formula).
    pub half_foundation: u16,
}

/// Snapshot of an attacker's state for target scanning.
/// Extracted to avoid borrow conflicts during entity iteration.
#[derive(Clone)]
pub(crate) struct AttackerSnapshot {
    pub stable_id: u64,
    pub owner: InternedId,
    pub category: EntityCategory,
    /// What the attacker is firing at — entity ID or cell coord.
    /// Cell targets skip auto-retarget and friendly-fire checks (the player
    /// explicitly chose this cell).
    pub target: super::TargetKind,
    pub pos_rx: u16,
    pub pos_ry: u16,
    pub pos_z: u8,
    pub pos_exact_z_leptons: Option<i32>,
    pub sub_x: SimFixed,
    pub sub_y: SimFixed,
    pub type_id: InternedId,
    pub facing: u8,
    pub veterancy: u16,
    pub cooldown_ticks: u16,
    pub animation_sequence: Option<crate::sim::animation::SequenceKind>,
    pub animation_frame: Option<u16>,
    pub is_prone: bool,
    pub is_fully_deployed: bool,
    pub has_movement: bool,
    pub pending_infantry_fire: Option<super::PendingInfantryFire>,
    pub pending_building_fire: Option<crate::sim::game_entity::PendingBuildingFire>,
    pub barrel_facing: Option<crate::sim::movement::FacingClass>,
    pub burst_remaining: u8,
    pub burst_delay_ticks: u8,
    /// Weapon-selection override (Gunner-IFV slot OR open-topped passenger weapon).
    pub weapon_override: Option<super::combat_weapon::WeaponOverride>,
    /// Garrison state — present only for garrisoned buildings (IsOccupied).
    pub garrison: Option<GarrisonSnapshot>,
    /// Which acquisition mission the attacker is on. Selects the scan radius
    /// formula — Area Guard reaches roughly twice as far as plain Guard.
    pub scan_mission: ScanMission,
}

/// Acquire the best currently valid target for one attacker entity.
/// Returns the target's stable entity ID.
///
/// `terrain` is threaded through for the 3D InRange check; when `None`
/// (headless tests, no map loaded), the range check falls back to the
/// existing 2D behavior.
pub fn acquire_best_target_for_entity(
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
    attacker_id: u64,
    fog: Option<&FogState>,
    terrain: Option<&ResolvedTerrainGrid>,
    require_playfield_membership: bool,
) -> Option<u64> {
    let entity = entities.get(attacker_id)?;
    // Aircraft with 0 ammo should not acquire new targets — need to reload.
    if let Some(ref ammo) = entity.aircraft_ammo {
        if ammo.current <= 0 {
            return None;
        }
    }
    let obj = rules.object(interner.resolve(entity.type_ref))?;
    // Need at least one weapon to acquire targets.
    if obj.primary.is_none() && obj.secondary.is_none() {
        return None;
    }

    let snapshot = AttackerSnapshot {
        stable_id: entity.stable_id,
        owner: entity.owner,
        category: entity.category,
        target: super::TargetKind::Entity(0), // Dummy — no current target when acquiring fresh
        pos_rx: entity.position.rx,
        pos_ry: entity.position.ry,
        pos_z: entity.position.z,
        pos_exact_z_leptons: entity.position.exact_z_leptons,
        sub_x: entity.position.sub_x,
        sub_y: entity.position.sub_y,
        type_id: entity.type_ref,
        facing: entity.facing,
        veterancy: entity.veterancy,
        cooldown_ticks: 0,
        animation_sequence: entity.animation.as_ref().map(|a| a.sequence),
        animation_frame: entity.animation.as_ref().map(|a| a.frame_index),
        is_prone: entity
            .infantry
            .as_ref()
            .is_some_and(|infantry| infantry.is_prone),
        is_fully_deployed: entity.is_fully_deployed(),
        has_movement: entity.movement_target.is_some(),
        pending_infantry_fire: None,
        pending_building_fire: None,
        barrel_facing: entity.barrel_facing,
        burst_remaining: 0,
        burst_delay_ticks: 0,
        weapon_override: entity.weapon_override,
        garrison: None,
        scan_mission: scan_mission_for(entity),
    };
    acquire_best_target(
        entities, rules, interner, &snapshot, obj, fog, None, terrain,
        require_playfield_membership,
    )
}

fn threat_class(rules: &RuleSet, interner: &StringInterner, type_id: InternedId) -> u8 {
    match rules.object(interner.resolve(type_id)) {
        Some(obj) if obj.primary.is_some() => 0,
        Some(obj) => match obj.category {
            ObjectCategory::Vehicle | ObjectCategory::Aircraft | ObjectCategory::Infantry => 1,
            ObjectCategory::Building => 2,
        },
        None => 3,
    }
}

/// Find the best hostile target within scan range for a given attacker snapshot.
///
/// Filters by: alive, hostile, visible (fog), weapon compatibility (projectile
/// flags + Verses > 0%), and range. Ranks by distance, threat class, stable ID.
/// Returns the target's stable entity ID.
///
/// RESIDUAL (GSI-08.01) — the ranking is VERA's own, not `Greatest_Threat`'s.
/// `threat_class` above is a four-bucket ordering invented here (armed, unarmed
/// mobile, building, unknown) and the key is `(distance², class, stable id)`.
/// Native scores each candidate with a weighted distance/value/`ThreatPosed`
/// evaluation and selects the maximum. `calculate_ai_threat_score` in this file
/// does model that score — from
/// `TechnoClass::Calculate_Threat_Score @ 0x0070CD10` — but it is wired only to
/// retaliation, never to acquisition.
/// - Trigger: any unit or defence picking among two or more legal targets.
/// - Player effect: the wrong pick. Nearest-first ignores value, so a Grizzly
///   that should shoot the engineer walking past shoots the closer wall
///   segment, and a base defence spreads onto whatever is nearest instead of
///   concentrating on the biggest threat.
/// - Frequency: continuous. Every engagement with more than one candidate in
///   range resolves through this key.
/// - Downstream risk: high, and that is why it is not a drive-by change. The
///   key decides which target every attacker holds, so switching it moves the
///   pinned replay hash and every combat test that depends on selection order;
///   it needs its own slice with a re-baseline. `ThreatPosed=` is also not
///   parsed anywhere despite 232 stock entries (141 of them `0`, i.e. "never
///   pick me" — engineers, spies and the like).
///
///   **Correction to an earlier draft of this note**, which said `ThreatPosed=`
///   was "the input the native score weights most heavily". It is not an input
///   to the score at all. `ThreatPosed` reaches `TechnoTypeClass+0x670`
///   (`TechnoTypeClass::ReadINI @ 0x007149CE`, via `ReadInteger @ 0x005276D0`,
///   its only xref), whereas `TechnoClass::Calculate_Threat_Score @ 0x0070CD10`
///   reads `TechnoType+0x2C0` for its special-threat term — the field this file
///   already models as `special_threat_value`. What `+0x670` actually feeds is
///   UNCHECKED; it is somewhere in candidate selection or the AI's own target
///   picking, NOT in this score. Implementing it into the C term on the
///   strength of the old note would have put a real key in the wrong place.
///
///   **A second gap in the score itself, and pass 2 settles which set is live.**
///   Native picks its five coefficients from one of two sources, branching on
///   the scorer's owning HOUSE byte `+0x1FB` (`0x0070CD4E` reads
///   `param_1[0x87]`, the owner pointer): clear takes `Rules+0x1068`..`+0x108C`
///   — the `[General] Dumb*Coefficient` set this file reads — and set takes the
///   scorer TYPE's own `+0x2C8`..`+0x2EC`, seeded from `[General]
///   *CoefficientDefault` by `TechnoTypeClass::ReadINI @ 0x007156D8` reading
///   `Rules+0x1060`. That byte is latched to 1 by the setter at `0x00509130`,
///   whose only callers are `BuildingClass::Unlimbo`/`Limbo`, and it is never
///   cleared — so EVERY house flips it the moment its MCV deploys, at frame
///   zero of an ordinary skirmish, and the per-type set is the live one. The
///   `Dumb*` set VERA reads is effectively dead, and the two differ by two sign
///   flips plus a 10x on the distance term. There is also a `Rules+0x1090`
///   bonus added when the scorer's `+0x5600` field matches a candidate field,
///   which this file does not model. The identity of type `+0x2C8`..`+0x2EC`
///   and the `+0x5600` pair are UNCHECKED — recorded as read, not as named.
///
///   **Why this row stays deferred after a second pass.** The native passive
///   path is not a re-keyed sort over a candidate list at all:
///   `Retaliate_And_Scan @ 0x00709820` calls vtable `+0x3C4` with `flags & 3`,
///   so acquisition always takes the ring topology — expanding square rings,
///   the occupant list walked per cell, the best kept only on a STRICTLY
///   greater score, and an early return at ring `cells/4` and again at
///   `cells/2` once anything has been found. Ties break on scan order, never on
///   stable id. `EntityStore` has no cell index and no native-insertion
///   occupant order, so swapping in a score-max over `values()` would trade a
///   named DRIFT for an unnamed one: VERA would evaluate candidates native
///   never reaches. The prerequisite is the cell index, not the scoring
///   function.
///   `OmniFire=`
///   (18 stock) and `DistributedWeaponFire=` are parsed and read by nothing,
///   and spread-fire types are explicitly refused a target by the passive scan,
///   so an Aegis Cruiser never acquires at all. `OpportunityFire=` (14) is the
///   exception in that list — it is read by `passive_acquire_gate` in
///   `world/techno_ai.rs` and covered by named tests there.
///
/// `scan_range_override`: when `Some`, replaces the mission-derived radius with
/// a hard cutoff. Used by garrisoned buildings whose scan range is derived from
/// foundation size + OccupyWeaponRange.
pub(crate) fn acquire_best_target(
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
    attacker: &AttackerSnapshot,
    attacker_obj: &crate::rules::object_type::ObjectType,
    fog: Option<&FogState>,
    scan_range_override: Option<SimFixed>,
    terrain: Option<&ResolvedTerrainGrid>,
    require_playfield_membership: bool,
) -> Option<u64> {
    let mut best: Option<(i64, u8, u64)> = None;

    // The radius is a property of the scanning object and its mission, not of
    // any one candidate, so it is resolved once outside the loop.
    let effective_scan_range = match scan_range_override {
        Some(cells) => ScanRange::Hard(cells),
        None => scan_range(
            rules,
            attacker_obj,
            attacker.veterancy,
            attacker.scan_mission,
        ),
    };

    for candidate in entities.values() {
        if candidate.stable_id == attacker.stable_id {
            continue;
        }
        if candidate.health.current == 0 || candidate.dying {
            continue;
        }
        // `TechnoClass::Evaluate_Candidate @ 0x006F7DB0` rejects a candidate
        // whose stored TechnoClass+0x3D5 byte is false. A live MapClass caller
        // enables this explicitly; headless fixtures retain their old behavior.
        if require_playfield_membership && !candidate.in_playfield {
            continue;
        }
        // Skip entities inside a transport — they are hidden from the battlefield.
        if candidate.passenger_role.is_inside_transport() {
            continue;
        }
        let attacker_owner_str = interner.resolve(attacker.owner);
        let candidate_owner_str = interner.resolve(candidate.owner);
        if fog
            .is_some_and(|fog_state| fog_state.is_friendly(attacker_owner_str, candidate_owner_str))
            || candidate.owner == attacker.owner
        {
            continue;
        }
        if let Some(fog_state) = fog {
            if !fog_state.is_cell_visible(
                attacker.owner,
                candidate.position.rx,
                candidate.position.ry,
            ) {
                continue;
            }
        }

        // Check if any weapon can engage this target (projectile flags + Verses > 0%).
        let target_cat: EntityCategory = combat_target_category(candidate, rules, interner);
        let target_armor: &str = rules
            .object(interner.resolve(candidate.type_ref))
            .map(|o| o.armor.as_str())
            .unwrap_or("none");
        let selected = match select_weapon_with_override(
            rules,
            attacker_obj,
            target_cat,
            target_armor,
            attacker.veterancy,
            attacker.weapon_override,
        ) {
            Some(s) => s,
            None => continue, // No weapon can engage this target.
        };

        // For passive acquisition, skip targets where Verses is Suppressed (1%).
        if verses_gate(selected.verses_pct) == VersesGate::Suppressed {
            continue;
        }

        // 2D dist_sq still feeds the ranking key below; the in-range boolean
        // is computed separately via 3D when possible.
        let dist_sq = lepton_distance_sq_raw(
            attacker.pos_rx,
            attacker.pos_ry,
            attacker.sub_x,
            attacker.sub_y,
            candidate.position.rx,
            candidate.position.ry,
            candidate.position.sub_x,
            candidate.position.sub_y,
        );
        let in_range = match effective_scan_range {
            // A hard cutoff — garrison override, `GuardRange=` on plain Guard,
            // or the doubled Area Guard radius. The retail acceptance test
            // applies this distance test and does NOT then ask whether the
            // weapon can reach, so neither does this. Kept 2D until later
            // stages thread an explicit radius through `compute_in_range`.
            ScanRange::Hard(cells) => is_within_range_leptons(dist_sq, cells),
            // Radius zero — acceptance defers to the attacker's own
            // can-fire-at-this-target query, which is the range of the weapon
            // selected against this very candidate. 3D when terrain and the
            // attacker entity are available.
            ScanRange::CanFireAt => match (terrain, entities.get(attacker.stable_id)) {
                (Some(t), Some(attacker_entity)) => {
                    let Some(source_z) = super::in_range::effective_z_leptons(attacker_entity, t)
                    else {
                        continue;
                    };
                    let src = (
                        attacker.pos_rx as i64 * 256 + attacker.sub_x.to_num::<i64>(),
                        attacker.pos_ry as i64 * 256 + attacker.sub_y.to_num::<i64>(),
                        source_z,
                    );
                    super::in_range::compute_in_range(
                        attacker_entity,
                        src,
                        &super::TargetKind::Entity(candidate.stable_id),
                        selected.weapon,
                        rules,
                        interner,
                        entities,
                        t,
                    )
                }
                _ => is_within_range_leptons(dist_sq, selected.weapon.range),
            },
        };
        if !in_range {
            continue;
        }

        let class = threat_class(rules, interner, candidate.type_ref);
        let rank = (dist_sq, class, candidate.stable_id);
        match best {
            Some(current) if rank >= current => {}
            _ => best = Some(rank),
        }
    }

    best.map(|(_, _, sid)| sid)
}

/// Check if an entity can retaliate against an attacker (weapon + Verses gate).
fn can_retaliate(
    entity: &GameEntity,
    attacker: &GameEntity,
    rules: &RuleSet,
    interner: &StringInterner,
) -> bool {
    let obj = match rules.object(interner.resolve(entity.type_ref)) {
        Some(o) => o,
        None => return false,
    };
    let target_cat: EntityCategory = combat_target_category(attacker, rules, interner);
    let target_armor: &str = rules
        .object(interner.resolve(attacker.type_ref))
        .map(|o| o.armor.as_str())
        .unwrap_or("none");
    let selected = match select_weapon_with_override(
        rules,
        obj,
        target_cat,
        target_armor,
        entity.veterancy,
        entity.weapon_override,
    ) {
        Some(s) => s,
        None => return false,
    };
    // 0% is already filtered by select_weapon_with_override (returns None).
    // 1% (Suppressed) also blocks retaliation.
    verses_gate(selected.verses_pct) != VersesGate::Suppressed
}

fn load_threat_double(value: f64) -> Option<X87Value> {
    X87Chop53::load_f64(NativeF64Bits::from_bits(value.to_bits())).ok()
}

fn threat_coord(entity: &GameEntity, terrain: Option<&ResolvedTerrainGrid>) -> (i32, i32, i32) {
    let x = i32::from(entity.position.rx)
        .wrapping_mul(256)
        .wrapping_add(entity.position.sub_x.to_num::<i32>());
    let y = i32::from(entity.position.ry)
        .wrapping_mul(256)
        .wrapping_add(entity.position.sub_y.to_num::<i32>());
    let z = terrain
        .and_then(|terrain| super::in_range::effective_z_leptons(entity, terrain))
        .and_then(|z| i32::try_from(z).ok())
        .unwrap_or_else(|| {
            i32::from(entity.position.z)
                .wrapping_mul(LEPTONS_PER_LEVEL as i32)
                .wrapping_add(
                    entity
                        .locomotor
                        .as_ref()
                        .map(|locomotor| locomotor.altitude.to_num::<i32>())
                        .unwrap_or(0),
                )
        });
    (x, y, z)
}

/// `TechnoClass::Calculate_Threat_Score @ 0x0070CD10`, narrowed to the
/// non-human (Rules `Dumb*`) coefficient branch consumed synchronously by
/// `ShouldRetaliate`. The caller uses the native NullCoord sentinel path, so
/// Sqrt_Approx/ftol distance is converted from leptons to whole cells here.
pub(crate) fn calculate_ai_threat_score(
    entities: &EntityStore,
    scorer_id: u64,
    candidate_id: u64,
    rules: &RuleSet,
    interner: &StringInterner,
    terrain: Option<&ResolvedTerrainGrid>,
) -> Option<X87Value> {
    let scorer = entities.get(scorer_id)?;
    let candidate = entities.get(candidate_id)?;
    let scorer_type = rules.object(interner.resolve(scorer.type_ref))?;
    let candidate_type = rules.object(interner.resolve(candidate.type_ref))?;
    let general = &rules.general;
    let coeff_a = load_threat_double(general.dumb_my_effectiveness_coefficient)?;
    let coeff_b = load_threat_double(general.dumb_target_effectiveness_coefficient)?;
    let coeff_c = load_threat_double(general.dumb_target_special_threat_coefficient)?;
    let coeff_d = load_threat_double(general.dumb_target_strength_coefficient)?;
    let coeff_e = load_threat_double(general.dumb_target_distance_coefficient)?;
    let mut score = X87Chop53::load_i32(0);

    // B: the candidate's selected weapon against the scorer. A candidate
    // already targeting the scorer contributes the negated term.
    let scorer_category = combat_target_category(scorer, rules, interner);
    if let Some(selected) = select_weapon_with_override(
        rules,
        candidate_type,
        scorer_category,
        &scorer_type.armor,
        candidate.veterancy,
        candidate.weapon_override,
    ) {
        let verses =
            load_threat_double(selected.warhead.verses_f64[armor_index(&scorer_type.armor)])?;
        let mut term = X87Chop53::mul(coeff_b, verses);
        if candidate
            .attack_target
            .as_ref()
            .is_some_and(|target| target.target == super::TargetKind::Entity(scorer.stable_id))
        {
            term = X87Chop53::neg(term);
        }
        score = X87Chop53::add(score, term);
    }

    // C: candidate type SpecialThreatValue.
    score = X87Chop53::add(
        score,
        X87Chop53::mul(
            coeff_c,
            load_threat_double(candidate_type.special_threat_value)?,
        ),
    );

    // A: the scorer's selected weapon against the candidate. Retain the
    // selected weapon for the native range term below.
    let candidate_category = combat_target_category(candidate, rules, interner);
    let selected_scorer_weapon = select_weapon_with_override(
        rules,
        scorer_type,
        candidate_category,
        &candidate_type.armor,
        scorer.veterancy,
        scorer.weapon_override,
    );
    if let Some(selected) = selected_scorer_weapon.as_ref() {
        let verses =
            load_threat_double(selected.warhead.verses_f64[armor_index(&candidate_type.armor)])?;
        score = X87Chop53::add(score, X87Chop53::mul(coeff_a, verses));
    }

    // D: live candidate health ratio.
    let health_ratio = if candidate.health.max == 0 {
        X87Chop53::load_i32(0)
    } else {
        X87Chop53::div(
            X87Chop53::load_i32(i32::from(candidate.health.current)),
            X87Chop53::load_i32(i32::from(candidate.health.max)),
        )
        .ok()?
    };
    score = X87Chop53::add(score, X87Chop53::mul(coeff_d, health_ratio));

    // E: whole cells beyond the scorer's selected weapon range. Weapon Range
    // is represented in cells in Rust, so its toward-zero conversion is the
    // native `(range + sign_adjust) >> 8` result.
    let scorer_coord = threat_coord(scorer, terrain);
    let candidate_coord = threat_coord(candidate, terrain);
    let dx = X87Chop53::load_i32(candidate_coord.0.wrapping_sub(scorer_coord.0));
    let dy = X87Chop53::load_i32(candidate_coord.1.wrapping_sub(scorer_coord.1));
    let dz = X87Chop53::load_i32(candidate_coord.2.wrapping_sub(scorer_coord.2));
    let distance_sq = X87Chop53::add(
        X87Chop53::add(X87Chop53::mul(dx, dx), X87Chop53::mul(dy, dy)),
        X87Chop53::mul(dz, dz),
    );
    let distance_root = X87Chop53::load_f32(sqrt_approx_f32(distance_sq).ok()?).ok()?;
    let distance_leptons = i32::try_from(X87Chop53::ftol_i64(distance_root).ok()?).ok()?;
    let distance_cells = crate::util::direction_tables::lepton_to_cell(distance_leptons);
    let range_cells = selected_scorer_weapon
        .as_ref()
        .map_or(scorer_type.sight, |selected| {
            selected.weapon.range.to_num::<i32>()
        });
    let beyond_range = distance_cells.wrapping_sub(range_cells).max(0);
    score = X87Chop53::add(
        X87Chop53::mul(X87Chop53::load_i32(beyond_range), coeff_e),
        score,
    );
    Some(X87Chop53::add(score, load_threat_double(100_000.0)?))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
enum RetaliationPeekFireError {
    Clear = 0,
    Illegal = 5,
}

/// Represented structural part of the read-only `GetFireError` peek issued by
/// `ShouldRetaliate`. The target is the still-represented damage source; a
/// source in ObjectClass limbo returns native `FIRE_ILLEGAL` without consuming
/// ammo, RNG, or weapon state.
fn retaliation_peek_fire_error(target: &GameEntity) -> RetaliationPeekFireError {
    if target.lifecycle.in_limbo {
        RetaliationPeekFireError::Illegal
    } else {
        RetaliationPeekFireError::Clear
    }
}

/// Evaluate the live, receiver-synchronous part of
/// `TechnoClass::ShouldRetaliate @ 0x007087C0` using represented authority.
///
/// The native call consumes the still-represented source object, including a
/// dying DeathWeapon producer, so source health is deliberately not an
/// admission gate here. Represented gates are read afresh for every ordered
/// receiver.
pub(crate) fn should_retaliate_from_damage(
    entities: &EntityStore,
    victim_id: u64,
    attacker_id: u64,
    rules: &RuleSet,
    interner: &StringInterner,
    houses: &BTreeMap<InternedId, HouseState>,
    alliances: &HouseAllianceMap,
    terrain: Option<&ResolvedTerrainGrid>,
) -> bool {
    let (Some(victim), Some(attacker)) = (entities.get(victim_id), entities.get(attacker_id))
    else {
        return false;
    };
    if !victim.is_alive()
        || victim.dying
        || !victim.lifecycle.object_alive
        || victim.lifecycle.in_limbo
    {
        return false;
    }

    let Some(victim_type) = rules.object(interner.resolve(victim.type_ref)) else {
        return false;
    };
    if !victim_type.can_retaliate
        || victim.bunker_link.installed_in().is_some()
        || victim.is_mind_controlled()
        || victim
            .capture_manager
            .as_ref()
            .is_some_and(|manager| manager.blocks_retaliation())
        || victim.spawn_manager.is_some()
        || victim_type
            .enslaves
            .as_deref()
            .is_some_and(|slave_type| rules.object_case_insensitive(slave_type).is_some())
    {
        return false;
    }
    if victim
        .mission
        .current()
        .known()
        .and_then(|mission| rules.mission_control.entry(mission))
        .is_some_and(|entry| !entry.retaliate)
    {
        return false;
    }

    let victim_owner = interner.resolve(victim.owner);
    let attacker_owner = interner.resolve(attacker.owner);
    if is_allied_with(alliances, victim_owner, attacker_owner)
        || is_allied_with(alliances, attacker_owner, victim_owner)
    {
        return false;
    }
    // The active human-control branch refuses to replace an existing TarCom.
    // A computer-owned receiver compares raw float10 threat scores at its own
    // coordinate and keeps its current target only when that score is strictly
    // greater. Equal or lower permits the normal retaliation path.
    let is_human = houses
        .get(&victim.owner)
        .is_some_and(|house| house.is_human);
    if is_human && victim.attack_target.is_some() {
        return false;
    }
    let target_category = combat_target_category(attacker, rules, interner);
    let target_armor = rules
        .object(interner.resolve(attacker.type_ref))
        .map(|object| object.armor.as_str())
        .unwrap_or("none");
    let Some(selected) = select_weapon_with_override(
        rules,
        victim_type,
        target_category,
        target_armor,
        victim.veterancy,
        victim.weapon_override,
    ) else {
        return false;
    };
    if retaliation_peek_fire_error(attacker) == RetaliationPeekFireError::Illegal {
        return false;
    }

    if !is_human
        && let Some(super::TargetKind::Entity(current_id)) =
            victim.attack_target.as_ref().map(|target| target.target)
        && let (Some(current_score), Some(attacker_score)) = (
            calculate_ai_threat_score(entities, victim_id, current_id, rules, interner, terrain),
            calculate_ai_threat_score(entities, victim_id, attacker_id, rules, interner, terrain),
        )
        // This comparison changed behaviour in the GSI-05.13 slice, and the
        // change is a correction rather than a side effect: `X87Chop53::compare`
        // ordered on the exponent before settling zero operands, so any score in
        // `(0, 1)` compared against an exactly-zero score came back as the
        // SMALLER one. An AI defender holding a zero-scored current target used
        // to refuse retaliation against a positive-scored attacker; it no longer
        // does. Reachability needs an exactly-zero `calculate_ai_threat_score`,
        // which the term structure makes uncommon but not impossible.
        && X87Chop53::compare(current_score, attacker_score) == X87Ordering::Greater
    {
        return false;
    }

    selected.weapon.range > SimFixed::ZERO && verses_gate(selected.verses_pct) == VersesGate::Normal
}

/// Retaliation system: idle units that were recently hit auto-attack their attacker.
///
/// Called after `tick_combat_with_fog()` in the game loop. Iterates entities
/// that have a `last_attacker_id` but no `attack_target` and no `order_intent`.
/// Skips retaliation if the weapon has 0% or 1% Verses against the attacker's armor.
pub fn tick_retaliation(
    entities: &mut EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
    live_order: &[u64],
) {
    // Collect retaliation candidates: (retaliator_id, attacker_id).
    let mut retaliators: Vec<(u64, u64)> = Vec::new();
    // Native retaliation is resolved during the same live-object (reveal/insertion
    // order) AI walk as the rest of combat, so scan victims in live order rather
    // than stable-id order.
    for &id in live_order {
        let entity = match entities.get(id) {
            Some(e) => e,
            None => continue,
        };
        // Must have last_attacker, no current attack target, no order intent.
        let attacker_sid = match entity.last_attacker_id {
            Some(sid) => sid,
            None => continue,
        };
        // The `order_intent.is_some()` suppression is the conceptual mission-busy
        // gate (`mission::verb::get_current_mission`), kept LITERAL on purpose: an
        // `is_busy`-only gate would let a Guarding unit (order_intent = Guard,
        // mission idle) begin retaliating — a proven DRIFT. Retiring the
        // `order_intent` predicate in favour of a mission goal field is a later
        // slice; the runtime check stays byte-identical here.
        if entity.attack_target.is_some() || entity.order_intent.is_some() {
            continue;
        }
        // Verify attacker is still alive. A sold/captured attacker keeps health
        // but is `dying` (a corpse awaiting the end-of-tick drain) — exclude it
        // so the victim doesn't retaliate against a dead object.
        let attacker_alive = entities
            .get(attacker_sid)
            .is_some_and(|a| a.health.current > 0 && !a.dying);
        if !attacker_alive {
            continue;
        }
        retaliators.push((id, attacker_sid));
    }

    // Process retaliation — issue attack commands.
    for (entity_id, attacker_sid) in retaliators {
        let retaliate = {
            let entity = match entities.get(entity_id) {
                Some(e) => e,
                None => continue,
            };
            let attacker = match entities.get(attacker_sid) {
                Some(a) => a,
                None => {
                    // Attacker gone — clear last_attacker.
                    if let Some(e) = entities.get_mut(entity_id) {
                        e.last_attacker_id = None;
                    }
                    continue;
                }
            };
            can_retaliate(entity, attacker, rules, interner)
        };

        if retaliate {
            // Read attacker rx/ry (only needed for body-only retaliators).
            let attacker_pos = match entities.get(attacker_sid) {
                Some(a) => (a.position.rx, a.position.ry),
                None => continue,
            };
            if let Some(entity) = entities.get_mut(entity_id) {
                if entity.barrel_facing.is_none() {
                    // Body-only retaliator — instantly face the attacker. Turreted
                    // retaliators get their turret rotation driven by
                    // tick_turret_rotation in subsequent ticks (matches gamemd).
                    let dx: i32 = attacker_pos.0 as i32 - entity.position.rx as i32;
                    let dy: i32 = attacker_pos.1 as i32 - entity.position.ry as i32;
                    entity.facing = crate::sim::movement::facing_from_delta(dx, dy);
                }
                entity.movement_target = None;
                entity.attack_target = Some(crate::sim::combat::AttackTarget::new(attacker_sid));
                // Retaliation is a damage-driven target, not a scanner pick.
                entity.passively_acquired_target = false;
            }
        }
        // Clear last_attacker regardless (prevent repeated attempts).
        if let Some(entity) = entities.get_mut(entity_id) {
            entity.last_attacker_id = None;
        }
    }
}
