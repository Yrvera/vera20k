//! The fire-range gate: `TechnoClass::CanFireAt` 0x006F77B0 builds the world
//! point a shot is measured FROM, `TechnoClass::InRange` 0x006F7220 decides
//! whether the target sits inside the weapon's reach from there.
//!
//! Replaces the 2D `lepton_distance_sq_raw` + `is_within_range_leptons` pair
//! at the four targeting/cursor sites. Implements 3D distance, IsLowFlying
//! ground-snap, AirRange bonus, arcing-weapon 2D fallthrough, foundation
//! bonus, the InRange bridge gate (0x006F75FB), the verified boundary
//! semantics (<= max inclusive, < min strict, -512 lep sentinel), and the two
//! caller-side source substitutions — `CellRangefinding=` and the high-flying
//! attacker's target-Z swap — that let a Kirov reach the ground below it.
//!
//! Ranges are compared in LEPTONS throughout: `CCINIClass::ReadRange`
//! 0x00474620 scales `Range=` by 256 before truncating, so the fraction on 61
//! stock weapons is real reach, not rounding noise.
//!
//! Stages 2-N add the remaining range-VALUE chain (Garrison / Bunker /
//! OpenTopped / Veteran). Stage Arcing adds the full Branch B slope-arc check.
//!
//! Depends on: rules (ObjectType, Weapon, ProjectileType), map (terrain
//! height + bridge), util/lepton (constants), util/fixed_math (isqrt_i64).
//! Does NOT depend on render/ui/sidebar/audio/net.

use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::ruleset::RuleSet;
use crate::rules::weapon_type::WeaponType;
use crate::sim::combat::TargetKind;
use crate::sim::entity_store::EntityStore;
use crate::sim::game_entity::GameEntity;
use crate::sim::intern::StringInterner;
use crate::sim::map::bridge_topology::BRIDGE_DECK_HEIGHT_LEPTONS;
use crate::sim::production::foundation_dimensions;
use crate::util::fixed_math::{SimFixed, isqrt_i64};
use crate::util::lepton::{
    BRIDGE_HEIGHT_DELTA_LEPTONS, HIGH_FLIGHT_THRESHOLD_LEPTONS,
    WEAPON_RANGE_ALWAYS_IN_RANGE_LEPTONS, ground_height_leptons,
};

fn terrain_ground_z_at(
    terrain: &ResolvedTerrainGrid,
    rx: u16,
    ry: u16,
    world_x: i32,
    world_y: i32,
) -> Option<i64> {
    let cell = terrain.cell(rx, ry)?;
    ground_height_leptons(cell.level, cell.slope_type, world_x, world_y)
        .ok()
        .map(i64::from)
}

fn entity_ground_z_leptons(entity: &GameEntity, terrain: &ResolvedTerrainGrid) -> Option<i64> {
    let world_x = i32::from(entity.position.rx)
        .wrapping_mul(256)
        .wrapping_add(entity.position.sub_x.to_num::<i32>());
    let world_y = i32::from(entity.position.ry)
        .wrapping_mul(256)
        .wrapping_add(entity.position.sub_y.to_num::<i32>());
    let ground = terrain_ground_z_at(
        terrain,
        entity.position.rx,
        entity.position.ry,
        world_x,
        world_y,
    )?;
    Some(
        ground
            + if entity.on_bridge {
                i64::from(BRIDGE_DECK_HEIGHT_LEPTONS)
            } else {
                0
            },
    )
}

/// Absolute world-coordinate Z of an entity. An object-owned exact coordinate
/// is authoritative; otherwise this reconstructs exact sloped terrain ground,
/// the entity-owned OnBridge deck offset, and locomotor altitude.
///
/// Droppod and parachute altitudes are intentionally NOT added — those
/// entities are always IsLowFlying-equivalent during descent and get
/// ground-snapped by the InRange caller.
pub(crate) fn effective_z_leptons(
    entity: &GameEntity,
    terrain: &ResolvedTerrainGrid,
) -> Option<i64> {
    if let Some(exact_z_leptons) = entity.position.exact_z_leptons {
        return Some(i64::from(exact_z_leptons));
    }

    let base = entity_ground_z_leptons(entity, terrain)?;
    Some(
        base + entity
            .locomotor
            .as_ref()
            .map(|loco| loco.altitude.to_num::<i64>())
            .unwrap_or(0),
    )
}

/// Current altitude above the object's own ground, in leptons — the VERA
/// stand-in for the height `ObjectClass::IsLowFlying` 0x005F6B60 and
/// `ObjectClass::IsHighFlying` 0x004DE620 fetch through `vtable+0x1C8`.
fn airborne_height_leptons(entity: &GameEntity) -> i64 {
    entity
        .locomotor
        .as_ref()
        .map(|l| l.altitude.to_num::<i64>())
        .unwrap_or(0)
}

/// `ObjectClass::IsLowFlying` 0x005F6B60 — `this->+0x74 != 0 &&
/// this->vtable+0x1C8() < 2 * g_nFootLevelHeightLeptons`.
///
/// Used by the InRange caller to decide whether to ground-snap the target's
/// Z before the distance computation: low-flying targets are ranged at the
/// ground beneath them, not at their actual altitude.
///
/// **Not scoped to aircraft.** The native predicate lives on `ObjectClass`, so
/// it answers for a `UnitClass` too — and the stock Kirov `[ZEP]` is listed in
/// `[VehicleTypes]`, not `[AircraftTypes]`, so a category gate here silently
/// grounded every Jumpjet vehicle. The `+0x74` byte's INI identity is
/// UNCHECKED; VERA reads a positive locomotor altitude as its analogue, which
/// differs from the native only for an in-air object at height exactly 0.
pub(crate) fn is_low_flying(entity: &GameEntity) -> bool {
    let alt = airborne_height_leptons(entity);
    alt > 0 && alt < HIGH_FLIGHT_THRESHOLD_LEPTONS
}

/// `ObjectClass::IsHighFlying` 0x004DE620 — `this->+0x74 != 0 &&
/// this->vtable+0x1C8() >= 2 * g_nFootLevelHeightLeptons`. Mutually exclusive
/// with `is_low_flying`; see that function for the scope note.
///
/// Read on the TARGET at 0x006F7263 to enable the AirRange bonus, and on the
/// ATTACKER at 0x006F7895 (in `TechnoClass::CanFireAt`) to decide whether the
/// shot is measured from the target's own Z.
pub(crate) fn is_high_flying(entity: &GameEntity) -> bool {
    airborne_height_leptons(entity) >= HIGH_FLIGHT_THRESHOLD_LEPTONS
}

/// `AirRangeBonus=` reaches `TechnoTypeClass+0x68C` through
/// `CCINIClass::ReadRange` 0x00474620 (the call at 0x007147A9), so the native
/// field already holds LEPTONS and `TechnoClass::InRange` adds it raw at
/// 0x006F7274 with no cell scaling. VERA keeps the key as cells, so the scale
/// happens here, on the fixed-point bits rather than through a truncating
/// `to_num` — `AirRangeBonus=1.5` is 384 leptons, not 256.
///
/// RESIDUAL: VERA parses the key through `f32`, so a fractional value picks up
/// an I16F16 rounding the native `ReadRange` double does not have, and this
/// floors where `Math__ftol` chops. Both are inert on stock data — the one
/// stock `AirRangeBonus=4` is an exact non-negative integer.
fn cells_fixed_to_leptons(cells: SimFixed) -> i64 {
    (i64::from(cells.to_bits()) * 256) >> 16
}

/// Effective max range in leptons for an attacker firing at a target with
/// `weapon`: weapon base range plus AirRange bonus (target high-flying) plus
/// foundation bonus (target is a building) plus height-fire bonus (Stage 1
/// stub returns 0).
///
/// Stages 2-N add: Garrison REPLACES, Bunker, OpenTopped, Veteran. Each is a
/// branch added to this function — call sites stay unchanged.
pub(crate) fn compute_effective_max_range_leptons(
    attacker: &GameEntity,
    target: &TargetKind,
    weapon: &WeaponType,
    rules: &RuleSet,
    interner: &StringInterner,
    entities: &EntityStore,
) -> i64 {
    let mut range_lep: i64 = i64::from(weapon.range_leptons);

    if let TargetKind::Entity(target_id) = *target {
        if let Some(target_entity) = entities.get(target_id) {
            // AirRange bonus when target is high-flying.
            if is_high_flying(target_entity) {
                if let Some(attacker_obj) = rules.object(interner.resolve(attacker.type_ref)) {
                    if let Some(air_bonus) = attacker_obj.air_range_bonus {
                        range_lep += cells_fixed_to_leptons(air_bonus);
                    }
                }
            }
            // Foundation bonus when target is a building: (FoundationW + FoundationH) * 64 lep.
            if target_entity.category == EntityCategory::Structure {
                if let Some(target_obj) = rules.object(interner.resolve(target_entity.type_ref)) {
                    let (fw, fh) = foundation_dimensions(&target_obj.foundation);
                    range_lep += (fw as i64 + fh as i64) * 0x40;
                }
            }
        }
    }

    // Height-fire bonus (gated by weapon.projectile.subject_to_elevation).
    // Stage 1 stub: always returns 0. The full bonus only fires when both
    // attacker AND target are low-flying aircraft AND the projectile sets
    // SubjectToElevation=yes — rare in standard play. Stage 2+ implements
    // the formula.
    let subject_to_elevation = weapon
        .projectile
        .as_deref()
        .and_then(|name| rules.projectile(name))
        .map(|p| p.subject_to_elevation)
        .unwrap_or(false);
    if subject_to_elevation {
        range_lep += height_fire_bonus_leptons(attacker, target, entities, rules);
    }

    range_lep
}

/// Stage 1 stub — returns 0.
fn height_fire_bonus_leptons(
    _attacker: &GameEntity,
    _target: &TargetKind,
    _entities: &EntityStore,
    _rules: &RuleSet,
) -> i64 {
    0
}

/// Full 3D range check. Returns true if `attacker` (firing from `src`) can
/// hit `target` with `weapon`, accounting for all Stage 1 gates.
///
/// `src` is caller-supplied as `(attacker_x_lep, attacker_y_lep,
/// effective_z_leptons(attacker, terrain))`.
pub(crate) fn compute_in_range(
    attacker: &GameEntity,
    src: (i64, i64, i64),
    target: &TargetKind,
    weapon: &WeaponType,
    rules: &RuleSet,
    interner: &StringInterner,
    entities: &EntityStore,
    terrain: &ResolvedTerrainGrid,
) -> bool {
    let weapon_range_lep: i64 = i64::from(weapon.range_leptons);

    // Sentinel — always-in-range short-circuit (`CMP EDI,0xFFFFFE00` at
    // 0x006F724E, before every other read).
    if weapon_range_lep == WEAPON_RANGE_ALWAYS_IN_RANGE_LEPTONS {
        return true;
    }

    let Some((tx, ty, tz)) = resolve_target_coords_3d(target, entities, rules, interner, terrain)
    else {
        return false;
    };

    let (sx, sy, sz) = src;

    // MinimumRange, 0x006F737F–0x006F73E8. Native runs it BEFORE the arcing
    // split, always in 3-D, and gates on `!= 0` rather than `> 0`; a strict
    // `JL` at 0x006F73E8 makes the boundary itself legal.
    if weapon.minimum_range_leptons != 0 {
        let dx = sx - tx;
        let dy = sy - ty;
        let dz = sz - tz;
        if isqrt_i64(dx * dx + dy * dy + dz * dz) < i64::from(weapon.minimum_range_leptons) {
            return false;
        }
    }

    // Arcing-weapon 2D fallthrough — preserves V3/Prism/etc. current behavior.
    let arcing = weapon
        .projectile
        .as_deref()
        .and_then(|name| rules.projectile(name))
        .map(|p| p.arcing)
        .unwrap_or(false);
    if arcing {
        return compute_in_range_arcing_2d(src, (tx, ty), weapon_range_lep);
    }

    let max_range_lep =
        compute_effective_max_range_leptons(attacker, target, weapon, rules, interner, entities);

    let dx = sx - tx;
    let dy = sy - ty;
    let dz = sz - tz;
    let dist_sq: i64 = dx * dx + dy * dy + dz * dz;
    let dist_lep = isqrt_i64(dist_sq);

    if dist_lep > max_range_lep {
        return false;
    }

    if attacker_under_bridge_targeting_above(src, tz, terrain) {
        return false;
    }

    true
}

/// Stage 1 arcing-weapon path: 2D distance only, base weapon range only
/// (no AirRange / Foundation / height-fire bonuses), preserving the current
/// 2D behavior for V3 / Prism / Dreadnought / Apocalypse Rocket / etc.
/// Stage Arcing replaces this with the full slope-arc check.
///
/// The MinimumRange test that used to live here has moved to the caller: the
/// native runs it once, in 3-D, before the `MOV CL,[EDX+0x29B]` arcing split
/// at 0x006F73F6.
fn compute_in_range_arcing_2d(
    src: (i64, i64, i64),
    target_xy: (i64, i64),
    weapon_range_lep: i64,
) -> bool {
    let (sx, sy, _sz) = src;
    let dx = sx - target_xy.0;
    let dy = sy - target_xy.1;
    let dist_sq: i64 = dx * dx + dy * dy;
    isqrt_i64(dist_sq) <= weapon_range_lep
}

/// Resolve target coords for the 3D path. Applies LowFlying ground-snap on
/// entity targets; cell targets get cell-center XY and ground-Z from the
/// terrain (with bridge deck offset if present).
fn resolve_target_coords_3d(
    target: &TargetKind,
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
    terrain: &ResolvedTerrainGrid,
) -> Option<(i64, i64, i64)> {
    match *target {
        TargetKind::Entity(id) => {
            let Some(t) = entities.get(id) else {
                return Some((i64::MAX / 4, i64::MAX / 4, 0));
            };
            let (rx, ry, sub_x, sub_y) = resolve_entity_target_coords(t, rules, interner);
            let tx = rx as i64 * 256 + sub_x.to_num::<i64>();
            let ty = ry as i64 * 256 + sub_y.to_num::<i64>();
            let tz = if is_low_flying(t) {
                let world_x = i32::from(t.position.rx)
                    .wrapping_mul(256)
                    .wrapping_add(t.position.sub_x.to_num::<i32>());
                let world_y = i32::from(t.position.ry)
                    .wrapping_mul(256)
                    .wrapping_add(t.position.sub_y.to_num::<i32>());
                let mut ground =
                    terrain_ground_z_at(terrain, t.position.rx, t.position.ry, world_x, world_y)?;
                if terrain
                    .cell(t.position.rx, t.position.ry)
                    .is_some_and(|cell| cell.bridge_facts.has_structural_bridge())
                {
                    ground += BRIDGE_HEIGHT_DELTA_LEPTONS;
                }
                ground
            } else {
                effective_z_leptons(t, terrain)?
            };
            Some((tx, ty, tz))
        }
        TargetKind::Cell(rx, ry) => {
            let tx = rx as i64 * 256 + 128;
            let ty = ry as i64 * 256 + 128;
            let tz = ground_z_with_bridge_offset(rx, ry, terrain)?;
            Some((tx, ty, tz))
        }
    }
}

/// `CellClass::GetCoords` 0x00486840 — a cell's own world point is its centre
/// (`MapCoord * 0x100 + 0x80` on both axes) with Z from
/// `CellClass::ComputeGroundHeightAtCoord` 0x0047B3A0. **No bridge-deck term:**
/// the deck offset belongs to whoever is standing on the deck, and
/// `TechnoClass::CanFireAt` adds it separately from the object's own OnBridge
/// byte at 0x006F7887.
fn cell_own_coords(rx: u16, ry: u16, terrain: &ResolvedTerrainGrid) -> Option<(i64, i64, i64)> {
    let world_x = i32::from(rx).wrapping_mul(256).wrapping_add(128);
    let world_y = i32::from(ry).wrapping_mul(256).wrapping_add(128);
    let z = terrain_ground_z_at(terrain, rx, ry, world_x, world_y)?;
    Some((i64::from(world_x), i64::from(world_y), z))
}

/// The target's own `GetCoords` Z (`vtable+0x48`), with NO low-flying snap —
/// the snap belongs to `InRange` 0x006F7332, not to this read.
fn target_own_z_leptons(
    target: &TargetKind,
    entities: &EntityStore,
    terrain: &ResolvedTerrainGrid,
) -> Option<i64> {
    match *target {
        TargetKind::Entity(id) => effective_z_leptons(entities.get(id)?, terrain),
        TargetKind::Cell(rx, ry) => cell_own_coords(rx, ry, terrain).map(|(_, _, z)| z),
    }
}

/// `TechnoClass::CanFireAt` 0x006F77B0 — the world point the range gate
/// measures FROM. Native builds it in three steps and hands the result to
/// `TechnoClass::InRange` 0x006F7220 at 0x006F78B8:
///
/// 1. `this->GetCoords()` at 0x006F77D3 — the attacker's own world point,
///    altitude included.
/// 2. `CellRangefinding=` (`WeaponType+0x134`, read at 0x006F7803): the point
///    is replaced by the attacker's OWN CELL — cell index from
///    `(coord + (coord < 0 ? 0xFF : 0)) >> 8` at 0x006F7821–0x006F7845, then
///    `CellClass::GetCoords` at 0x006F7866, so X/Y become the cell centre and Z
///    the ground under it. `+0x8C` (OnBridge) adds the deck offset at
///    0x006F7887. This is the key `[BlimpBomb]` carries and the reason the
///    Kirov's `Range=1.5` is measured from the ground, not from 750 leptons up.
/// 3. `this->IsHighFlying()` at 0x006F7895: the source Z is replaced by the
///    TARGET's own `GetCoords` Z, which cancels the height term for a
///    high-flying attacker. This is why a Kirov, a MiG or a Jumpjet never pays
///    its own altitude in the range check.
///
/// Both steps write the same Z slot, so a high-flying attacker with
/// `CellRangefinding=` ends on the target's Z — step 3 runs last.
pub(crate) fn fire_source_coords(
    attacker: &GameEntity,
    target: &TargetKind,
    weapon: &WeaponType,
    entities: &EntityStore,
    terrain: &ResolvedTerrainGrid,
) -> Option<(i64, i64, i64)> {
    let mut x = i64::from(attacker.position.rx) * 256 + attacker.position.sub_x.to_num::<i64>();
    let mut y = i64::from(attacker.position.ry) * 256 + attacker.position.sub_y.to_num::<i64>();
    let mut z = effective_z_leptons(attacker, terrain)?;

    if weapon.cell_rangefinding {
        // `SAR` after the sign-bias add is a truncate-toward-zero cell index;
        // VERA cell coordinates are unsigned, so the bias term is always 0.
        let (cx, cy) = ((x >> 8) as u16, (y >> 8) as u16);
        let (cell_x, cell_y, cell_z) = cell_own_coords(cx, cy, terrain)?;
        x = cell_x;
        y = cell_y;
        z = cell_z
            + if attacker.on_bridge {
                BRIDGE_HEIGHT_DELTA_LEPTONS
            } else {
                0
            };
    }

    if is_high_flying(attacker) {
        z = target_own_z_leptons(target, entities, terrain)?;
    }

    Some((x, y, z))
}

/// Resolve an entity's target coords (rx, ry, sub_x, sub_y) — buildings shift
/// from NW corner cell center to foundation geometric center, others use the
/// entity's raw position.
fn resolve_entity_target_coords(
    t: &GameEntity,
    rules: &RuleSet,
    interner: &StringInterner,
) -> (u16, u16, SimFixed, SimFixed) {
    if t.category == EntityCategory::Structure {
        if let Some(obj) = rules.object(interner.resolve(t.type_ref)) {
            let (fw, fh) = foundation_dimensions(&obj.foundation);
            let offset_x = (fw.saturating_sub(1) as i32) * 128;
            let offset_y = (fh.saturating_sub(1) as i32) * 128;
            let full_x: i32 =
                t.position.rx as i32 * 256 + t.position.sub_x.to_num::<i32>() + offset_x;
            let full_y: i32 =
                t.position.ry as i32 * 256 + t.position.sub_y.to_num::<i32>() + offset_y;
            return (
                (full_x / 256) as u16,
                (full_y / 256) as u16,
                SimFixed::from_num(full_x % 256),
                SimFixed::from_num(full_y % 256),
            );
        }
    }
    (
        t.position.rx,
        t.position.ry,
        t.position.sub_x,
        t.position.sub_y,
    )
}

/// Ground Z in leptons for a cell, plus bridge deck offset if a bridge deck
/// is present on the cell.
fn ground_z_with_bridge_offset(rx: u16, ry: u16, terrain: &ResolvedTerrainGrid) -> Option<i64> {
    let cell = terrain.cell(rx, ry)?;
    let world_x = i32::from(rx).wrapping_mul(256).wrapping_add(128);
    let world_y = i32::from(ry).wrapping_mul(256).wrapping_add(128);
    let mut z = terrain_ground_z_at(terrain, rx, ry, world_x, world_y)?;
    if cell.has_bridge_deck {
        z += BRIDGE_HEIGHT_DELTA_LEPTONS;
    }
    Some(z)
}

/// `cell+0x140 & 0x100` — "this cell belongs to a bridge". Written by
/// `CellClass::SetBridgeDirection_NESW` 0x0047E040 / `_NWSE` 0x0047E470, and
/// the exact word `TechnoClass::InRange` and `GetFireError` both test.
fn cell_is_bridge(terrain: &ResolvedTerrainGrid, rx: u16, ry: u16) -> bool {
    terrain
        .cell(rx, ry)
        .is_some_and(|cell| cell.bridge_facts.has_structural_bridge())
}

/// `TechnoClass::GetFireError` 0x006FC0B0, the block at 0x006FCBE6.
///
/// ```text
/// MOV CL,[ESI+0x8C]        ; attacker OnBridge
/// MOV AL,[EBP+0x8C]        ; target   OnBridge
/// CMP CL,AL ; JZ           ; only runs when they DISAGREE
/// ...  both objects' own cells (vtable+0x1BC = ObjectClass::GetOccupiedCell
///      0x005F6960) must carry cell+0x140 & 0x100
/// ...  and the attacker must NOT satisfy vtable+0x54
/// -> FireError 5, the shot does not happen
/// ```
///
/// `EBP` is built at 0x006FC177 as `(target->flags_0x14 & 1) ? target : 0` and
/// the tail is guarded by `TEST EBP,EBP` at 0x006FCAFA, so the block is reached
/// only when the target narrows to a TechnoClass — a cell target never gets
/// here.
///
/// **vtable+0x54, pinned 2026-08-19.** For `FootClass` (so every infantry and
/// vehicle) the slot holds `ObjectClass::IsHighFlying` 0x004DE620:
/// `this->+0x74 != 0 && this->vtable+0x1C8() >= 2 * g_nFootLevelHeightLeptons`,
/// i.e. height at or above 208 leptons. `AircraftClass` overrides it at
/// 0x0041B920 and forwards to the same body EXCEPT for two Rules-designated
/// types, which answer from the spawn manager's `vtable+0x80` instead:
/// `RulesClass+0x4E0` = `[General] V3RocketType` (key string 0x0083BA88,
/// written at 0x006713B0) and `RulesClass+0x514` = `[General] DMislType`
/// (key string 0x0083B9B0, written at 0x0067156A). Both are missile bodies
/// that are airborne whenever they are alive, so the branch cannot decide this
/// gate for anything a player commands.
///
/// `is_high_flying` is scoped to aircraft where the native predicate is
/// universal. They agree on every input this gate can reach: a ground object's
/// height is 0, which fails the native's `>= 208` just as VERA's category test
/// fails. Recorded rather than glossed.
///
/// This is deliberately NOT folded into `compute_in_range`. The native
/// `InRange` 0x006F7220 has no such clause; putting it there would be a gate
/// gamemd's InRange lacks. It is evaluated beside the range test at the fire
/// site, as its own refusal.
///
/// What a refused attacker does NEXT is a separate question. Native consumers
/// of the FireError code outside 0x006FC0B0 have not been traced — UNCHECKED —
/// so this only suppresses the shot and leaves target selection and pursuit
/// exactly as they were.
pub(crate) fn fire_error_on_bridge_mismatch(
    attacker: &GameEntity,
    target: &crate::sim::combat::TargetKind,
    entities: &EntityStore,
    terrain: &ResolvedTerrainGrid,
) -> bool {
    let crate::sim::combat::TargetKind::Entity(target_id) = target else {
        return false;
    };
    let Some(target_entity) = entities.get(*target_id) else {
        return false;
    };
    if attacker.bridge_occupancy.is_some() == target_entity.bridge_occupancy.is_some() {
        return false;
    }
    if !cell_is_bridge(terrain, attacker.position.rx, attacker.position.ry) {
        return false;
    }
    if !cell_is_bridge(
        terrain,
        target_entity.position.rx,
        target_entity.position.ry,
    ) {
        return false;
    }
    !is_high_flying(attacker)
}

/// `TechnoClass::InRange` 0x006F7220, the block at 0x006F75FB reached once the
/// distance test has already passed:
///
/// ```text
/// cell = MapClass::Get_CellClass_At_Coord(source);
/// if (cell->flags_140 & 0x100) {                       // 0x006F760C  TEST CH,0x1
///     top = CellClass::GetGroundHeight(source) + g_nTechnoInRangeStructuralDeckOffsetLeptons;
///     if (source.Z < top && target.Z >= top)           // 0x006F7627 / 0x006F762B
///         return false;
/// }
/// ```
///
/// The attacker is standing in a bridge cell underneath the deck and the target
/// sits at or above the deck top, so the shot would pass through the deck.
/// Only the attacker's cell is consulted — the target may be anywhere.
/// Boundary semantics are asymmetric on purpose: the source test is strict
/// (`JGE` allows equality), the target test is inclusive.
fn attacker_under_bridge_targeting_above(
    src: (i64, i64, i64),
    target_z_lep: i64,
    terrain: &ResolvedTerrainGrid,
) -> bool {
    let (sx, sy, sz) = src;
    let rx = (sx / 256) as u16;
    let ry = (sy / 256) as u16;
    if !cell_is_bridge(terrain, rx, ry) {
        return false;
    }
    let Some(ground_z) = terrain_ground_z_at(terrain, rx, ry, sx as i32, sy as i32) else {
        return false;
    };
    let bridge_top = ground_z + BRIDGE_HEIGHT_DELTA_LEPTONS;
    sz < bridge_top && target_z_lep >= bridge_top
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::ini_parser::IniFile;
    use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
    use crate::rules::ruleset::RuleSet;
    use crate::sim::entity_store::EntityStore;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::intern::test_interner;
    use crate::sim::movement::locomotion::LocomotorSlot;
    use crate::sim::movement::locomotor::{
        AirMovePhase, GroundMovePhase, LocomotorState, MovementLayer,
    };
    use crate::util::fixed_math::{SIM_ONE, SIM_ZERO, SimFixed};
    use crate::util::lepton::LEPTONS_PER_LEVEL;

    fn ground_entity_at_level(level: u8) -> GameEntity {
        let mut e = GameEntity::test_default(1, "MTNK", "Test", 10, 10);
        e.position.z = level;
        e.category = EntityCategory::Unit;
        e
    }

    fn aircraft_at_altitude(altitude_lep: i64) -> GameEntity {
        let mut e = GameEntity::test_default(2, "ORCA", "Test", 10, 10);
        e.category = EntityCategory::Aircraft;
        e.locomotor = Some(LocomotorState {
            kind: LocomotorKind::Fly,
            slot: LocomotorSlot::from_kind(LocomotorKind::Fly),
            powered: true,
            piggyback: None,
            runtime_payload: crate::sim::movement::locomotion::LocomotorRuntimePayload::for_kind(
                LocomotorKind::Fly,
                0,
            ),
            layer: MovementLayer::Air,
            phase: GroundMovePhase::Idle,
            air_phase: AirMovePhase::Cruising,
            speed_multiplier: SIM_ONE,
            speed_fraction: SIM_ONE,
            fly_current_speed: SIM_ZERO,
            altitude: SimFixed::from_num(altitude_lep as i32),
            target_altitude: SimFixed::from_num(altitude_lep as i32),
            climb_rate: SIM_ZERO,
            jumpjet_speed: SIM_ZERO,
            jumpjet_accel: SIM_ZERO,
            jumpjet_current_speed: SIM_ZERO,
            jumpjet_deviation: 0,
            jumpjet_crash_speed: SIM_ZERO,
            jumpjet_turn_rate: 0,
            balloon_hover: false,
            hover_attack: false,
            speed_type: SpeedType::Winged,
            movement_zone: MovementZone::Fly,
            rot: 0,
            air_progress: SIM_ZERO,
            infantry_wobble_phase: 0.0,
            subcell_dest: None,
            hover_throttle: crate::util::fixed_math::SIM_ZERO,
            hover_speed_request: crate::util::fixed_math::SIM_ZERO,
            hover_bob_offset: crate::util::fixed_math::SIM_ZERO,
        });
        e
    }

    #[test]
    fn effective_z_ground_unit() {
        let e = ground_entity_at_level(5);
        let mut terrain = flat_terrain(16, 16);
        terrain.cells[10 * 16 + 10].level = 5;
        assert_eq!(effective_z_leptons(&e, &terrain), Some(520));
    }

    #[test]
    fn effective_z_airborne_aircraft_adds_altitude() {
        let mut e = aircraft_at_altitude(1500);
        e.position.z = 0;
        let terrain = flat_terrain(16, 16);
        assert_eq!(effective_z_leptons(&e, &terrain), Some(1500));

        let mut e2 = aircraft_at_altitude(800);
        e2.position.z = 2;
        let mut elevated = flat_terrain(16, 16);
        elevated.cells[10 * 16 + 10].level = 2;
        assert_eq!(effective_z_leptons(&e2, &elevated), Some(1008));
    }

    #[test]
    fn gsi_04_03b_effective_z_uses_exact_sloped_subcell_and_air_altitude() {
        let mut entity = aircraft_at_altitude(500);
        entity.position.sub_x = SimFixed::from_num(64);
        entity.position.sub_y = SimFixed::from_num(192);
        let mut terrain = flat_terrain(16, 16);
        let cell = &mut terrain.cells[10 * 16 + 10];
        cell.level = 2;
        cell.slope_type = 1;
        assert_eq!(effective_z_leptons(&entity, &terrain), Some(734));
    }

    #[test]
    fn gsi_04_03b_effective_z_uses_entity_on_bridge_not_cell_deck() {
        let mut entity = ground_entity_at_level(0);
        let mut terrain = flat_terrain(16, 16);
        terrain.cells[10 * 16 + 10].has_bridge_deck = true;

        assert_eq!(effective_z_leptons(&entity, &terrain), Some(0));
        entity.on_bridge = true;
        assert_eq!(
            effective_z_leptons(&entity, &terrain),
            Some(i64::from(BRIDGE_DECK_HEIGHT_LEPTONS))
        );
    }

    #[test]
    fn gsi_04_15_effective_z_prefers_exact_signed_world_coordinate() {
        let mut entity = aircraft_at_altitude(1500);
        entity.position.rx = 600;
        entity.position.ry = 600;
        entity.position.z = 9;
        entity.position.exact_z_leptons = Some(-137);
        entity.on_bridge = true;

        let terrain = flat_terrain(1, 1);
        assert_eq!(effective_z_leptons(&entity, &terrain), Some(-137));
    }

    #[test]
    fn is_low_flying_only_for_airborne_objects() {
        let ground = ground_entity_at_level(5);
        assert!(!is_low_flying(&ground));

        let grounded_air = aircraft_at_altitude(0);
        assert!(!is_low_flying(&grounded_air));

        let low = aircraft_at_altitude(HIGH_FLIGHT_THRESHOLD_LEPTONS - 1);
        assert!(is_low_flying(&low));

        let high = aircraft_at_altitude(1500);
        assert!(!is_low_flying(&high));
    }

    #[test]
    fn is_high_flying_inverse_threshold() {
        let just_below = aircraft_at_altitude(HIGH_FLIGHT_THRESHOLD_LEPTONS - 1);
        assert!(!is_high_flying(&just_below));

        let at_threshold = aircraft_at_altitude(HIGH_FLIGHT_THRESHOLD_LEPTONS);
        assert!(is_high_flying(&at_threshold));

        let cruise = aircraft_at_altitude(1500);
        assert!(is_high_flying(&cruise));

        let ground = ground_entity_at_level(5);
        assert!(!is_high_flying(&ground));
    }

    /// `ObjectClass::IsHighFlying` 0x004DE620 and `ObjectClass::IsLowFlying`
    /// 0x005F6B60 live on `ObjectClass`, so they answer for a `UnitClass` too.
    /// The stock Kirov `[ZEP]` is a `[VehicleTypes]` entry, so an aircraft-only
    /// gate here would have called a Jumpjet at 750 leptons "grounded".
    #[test]
    fn flight_predicates_are_not_scoped_to_the_aircraft_category() {
        let cruising = jumpjet_unit_at_altitude(KIROV_JUMPJET_HEIGHT_LEPTONS);
        assert_eq!(cruising.category, EntityCategory::Unit);
        assert!(is_high_flying(&cruising));
        assert!(!is_low_flying(&cruising));

        let climbing = jumpjet_unit_at_altitude(HIGH_FLIGHT_THRESHOLD_LEPTONS - 1);
        assert!(is_low_flying(&climbing));
        assert!(!is_high_flying(&climbing));

        let landed = jumpjet_unit_at_altitude(0);
        assert!(!is_low_flying(&landed));
        assert!(!is_high_flying(&landed));
    }

    // ─── Fixtures for compute_in_range tests ────────────────────────────

    fn flat_terrain(w: u16, h: u16) -> ResolvedTerrainGrid {
        let cells: Vec<ResolvedTerrainCell> = (0..h)
            .flat_map(|ry| (0..w).map(move |rx| default_cell(rx, ry)))
            .collect();
        ResolvedTerrainGrid::from_cells(w, h, cells)
    }

    fn default_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
        ResolvedTerrainCell {
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
            bridge_facts: crate::map::bridge_facts::BridgeCellFacts::default(),
            tube_index: None,
            radar_left: [0; 3],
            radar_right: [0; 3],
            accepts_smudge: true,
            allows_tiberium: false,
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }

    fn rules_with_weapon(weapon_ini: &str, attacker_ini: &str, target_ini: &str) -> RuleSet {
        let ini_str = format!(
            "[InfantryTypes]\n\n\
             [VehicleTypes]\n0=ATKR\n1=TGT\n\n\
             [AircraftTypes]\n\n\
             [BuildingTypes]\n\n\
             [ATKR]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=GUN\n{attacker_ini}\n\n\
             [TGT]\nStrength=300\nArmor=heavy\nSpeed=6\n{target_ini}\n\n\
             [GUN]\n{weapon_ini}\n\n\
             [WH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        );
        let ini = IniFile::from_str(&ini_str);
        RuleSet::from_ini(&ini).expect("rules parse")
    }

    fn ground_attacker(rx: u16, ry: u16, level: u8, type_ref: &str) -> GameEntity {
        let mut e = GameEntity::test_default(100, type_ref, "Attackers", rx, ry);
        e.position.z = level;
        e.category = EntityCategory::Unit;
        e
    }

    fn ground_target(rx: u16, ry: u16, level: u8, type_ref: &str) -> GameEntity {
        let mut e = GameEntity::test_default(200, type_ref, "Defenders", rx, ry);
        e.position.z = level;
        e.category = EntityCategory::Unit;
        e
    }

    fn building_target(rx: u16, ry: u16, type_ref: &str) -> GameEntity {
        let mut e = GameEntity::test_default(300, type_ref, "Defenders", rx, ry);
        e.position.z = 0;
        e.category = EntityCategory::Structure;
        e
    }

    fn aircraft_target(
        rx: u16,
        ry: u16,
        level: u8,
        altitude_lep: i64,
        type_ref: &str,
    ) -> GameEntity {
        let mut e = aircraft_at_altitude(altitude_lep);
        e.stable_id = 200;
        e.position.rx = rx;
        e.position.ry = ry;
        e.position.z = level;
        e.type_ref = crate::sim::intern::test_intern(type_ref);
        e
    }

    /// Stock `[ZEP] JumpjetHeight=750`.
    const KIROV_JUMPJET_HEIGHT_LEPTONS: i64 = 750;

    /// A Jumpjet VEHICLE — the shape the stock Kirov actually has. `[ZEP]` is
    /// listed under `[VehicleTypes]`, so it is a `UnitClass` that flies.
    fn jumpjet_unit_at_altitude(altitude_lep: i64) -> GameEntity {
        let mut e = aircraft_at_altitude(altitude_lep);
        e.category = EntityCategory::Unit;
        e
    }

    fn src_at_cell(rx: u16, ry: u16, level: u8) -> (i64, i64, i64) {
        (
            rx as i64 * 256 + 128,
            ry as i64 * 256 + 128,
            level as i64 * LEPTONS_PER_LEVEL,
        )
    }

    // Test 1: Sentinel always-in-range
    #[test]
    fn sentinel_always_in_range() {
        let rules = rules_with_weapon("Damage=1\nROF=20\nRange=-2\nWarhead=WH", "", "");
        let weapon = rules.weapon("GUN").expect("weapon");
        let attacker = ground_attacker(5, 5, 0, "ATKR");
        let target = ground_target(50, 50, 0, "TGT");
        let mut entities = EntityStore::new();
        entities.insert(target);
        let interner = test_interner();
        let terrain = flat_terrain(64, 64);

        let in_range = compute_in_range(
            &attacker,
            src_at_cell(5, 5, 0),
            &TargetKind::Entity(200),
            weapon,
            &rules,
            &interner,
            &entities,
            &terrain,
        );
        assert!(in_range, "sentinel range should always be in range");
    }

    // Test 2: Boundary inclusive max
    #[test]
    fn max_range_inclusive_at_exact_boundary() {
        let rules = rules_with_weapon("Damage=1\nROF=20\nRange=4\nWarhead=WH", "", "");
        let weapon = rules.weapon("GUN").expect("weapon");
        let attacker = ground_attacker(0, 0, 0, "ATKR");
        // dx = 4 cells = 1024 lep exactly.
        let target_at = ground_target(4, 0, 0, "TGT");
        let mut entities = EntityStore::new();
        entities.insert(target_at);
        let interner = test_interner();
        let terrain = flat_terrain(64, 64);

        let at_boundary = compute_in_range(
            &attacker,
            src_at_cell(0, 0, 0),
            &TargetKind::Entity(200),
            weapon,
            &rules,
            &interner,
            &entities,
            &terrain,
        );
        assert!(at_boundary, "exact-range boundary should be inclusive");

        // Now move target 1 lepton further (rx=4, sub_x=129 → total = 1024+1 lep horizontal).
        let mut over = ground_target(4, 0, 0, "TGT");
        over.position.sub_x = SimFixed::from_num(129);
        let mut entities2 = EntityStore::new();
        entities2.insert(over);
        let one_past = compute_in_range(
            &attacker,
            src_at_cell(0, 0, 0),
            &TargetKind::Entity(200),
            weapon,
            &rules,
            &interner,
            &entities2,
            &terrain,
        );
        assert!(
            !one_past,
            "one lepton past max range should be out of range"
        );
    }

    // Test 3: Boundary strict min
    #[test]
    fn min_range_strict_at_exact_boundary() {
        let rules = rules_with_weapon(
            "Damage=1\nROF=20\nRange=10\nMinimumRange=2\nWarhead=WH",
            "",
            "",
        );
        let weapon = rules.weapon("GUN").expect("weapon");
        let attacker = ground_attacker(0, 0, 0, "ATKR");
        let terrain = flat_terrain(64, 64);
        let interner = test_interner();

        // At exactly min range (2 cells = 512 lep): inclusive — true.
        let target = ground_target(2, 0, 0, "TGT");
        let mut entities = EntityStore::new();
        entities.insert(target);
        let at_min = compute_in_range(
            &attacker,
            src_at_cell(0, 0, 0),
            &TargetKind::Entity(200),
            weapon,
            &rules,
            &interner,
            &entities,
            &terrain,
        );
        assert!(
            at_min,
            "at min-range boundary should be in range (inclusive)"
        );

        // 1 lepton inside min (rx=1, sub_x=128+255=383 → 1*256+255=511 lep): strict — false.
        let mut inside = ground_target(1, 0, 0, "TGT");
        inside.position.sub_x = SimFixed::from_num(255);
        let mut entities2 = EntityStore::new();
        entities2.insert(inside);
        let inside_min = compute_in_range(
            &attacker,
            src_at_cell(0, 0, 0),
            &TargetKind::Entity(200),
            weapon,
            &rules,
            &interner,
            &entities2,
            &terrain,
        );
        assert!(
            !inside_min,
            "1 lep inside min-range should be rejected (strict <)"
        );
    }

    // Test 4: 3D vs 2D divergence
    #[test]
    fn three_d_distance_rejects_high_z_delta() {
        // Target ground is 12 terrain levels = 1248 lep, one cell away.
        // 3D distance truncates to 1273 leptons: outside 4 cells, inside 5 cells.
        let rules = rules_with_weapon("Damage=1\nROF=20\nRange=4\nWarhead=WH", "", "");
        let weapon = rules.weapon("GUN").expect("weapon");
        let attacker = ground_attacker(5, 5, 0, "ATKR");
        let target = ground_target(6, 5, 12, "TGT");
        let mut entities = EntityStore::new();
        entities.insert(target);
        let interner = test_interner();
        let mut terrain = flat_terrain(64, 64);
        terrain.cells[5 * 64 + 6].level = 12;

        let r4 = compute_in_range(
            &attacker,
            src_at_cell(5, 5, 0),
            &TargetKind::Entity(200),
            weapon,
            &rules,
            &interner,
            &entities,
            &terrain,
        );
        assert!(!r4, "1273-lepton 3D distance exceeds 1024 leptons");

        // Same setup, range=5 cells (=1280 lep) → true.
        let rules2 = rules_with_weapon("Damage=1\nROF=20\nRange=5\nWarhead=WH", "", "");
        let weapon2 = rules2.weapon("GUN").expect("weapon");
        let r5 = compute_in_range(
            &attacker,
            src_at_cell(5, 5, 0),
            &TargetKind::Entity(200),
            weapon2,
            &rules2,
            &interner,
            &entities,
            &terrain,
        );
        assert!(r5, "1273-lepton 3D distance is within 1280 leptons");
    }

    // Test 5: LowFlying ground-snap
    #[test]
    fn low_flying_target_z_snapped_to_ground() {
        let rules = rules_with_weapon("Damage=1\nROF=20\nRange=4\nWarhead=WH", "", "");
        let weapon = rules.weapon("GUN").expect("weapon");
        let attacker = ground_attacker(0, 0, 0, "ATKR");
        // Aircraft one lepton below the verified high-flight split, 4 cells away.
        let target = aircraft_target(4, 0, 0, HIGH_FLIGHT_THRESHOLD_LEPTONS - 1, "TGT");
        let mut entities = EntityStore::new();
        entities.insert(target);
        let interner = test_interner();
        let terrain = flat_terrain(64, 64);

        let in_range = compute_in_range(
            &attacker,
            src_at_cell(0, 0, 0),
            &TargetKind::Entity(200),
            weapon,
            &rules,
            &interner,
            &entities,
            &terrain,
        );
        assert!(
            in_range,
            "low-flying target should snap to ground for range check"
        );
    }

    #[test]
    fn gsi_04_03b_low_flight_snap_uses_exact_target_subcell() {
        let rules = rules_with_weapon("Damage=1\nROF=20\nRange=4\nWarhead=WH", "", "");
        let interner = test_interner();
        let mut target = aircraft_target(4, 0, 0, HIGH_FLIGHT_THRESHOLD_LEPTONS - 1, "TGT");
        target.position.sub_x = SimFixed::from_num(3);
        target.position.sub_y = SIM_ZERO;
        let mut entities = EntityStore::new();
        entities.insert(target);
        let mut terrain = flat_terrain(8, 1);
        terrain.cells[4].slope_type = 1;

        let (_, _, target_z) = resolve_target_coords_3d(
            &TargetKind::Entity(200),
            &entities,
            &rules,
            &interner,
            &terrain,
        )
        .expect("supported target terrain");
        assert_eq!(target_z, 1, "slope 1 at local X=3 is one lepton high");
    }

    #[test]
    fn gsi_04_03b_low_flight_bridge_snap_requires_structural_flag() {
        let rules = rules_with_weapon("Damage=1\nROF=20\nRange=4\nWarhead=WH", "", "");
        let interner = test_interner();
        let target = aircraft_target(0, 0, 0, HIGH_FLIGHT_THRESHOLD_LEPTONS - 1, "TGT");
        let mut entities = EntityStore::new();
        entities.insert(target);
        let mut terrain = flat_terrain(1, 1);
        terrain.cells[0].has_bridge_deck = true;

        let non_structural_z = resolve_target_coords_3d(
            &TargetKind::Entity(200),
            &entities,
            &rules,
            &interner,
            &terrain,
        )
        .expect("supported non-structural terrain")
        .2;
        assert_eq!(
            non_structural_z, 0,
            "generic or low deck state does not satisfy Cell+0x140 bit 0x100"
        );

        terrain.cells[0].bridge_facts.raw_flags = crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL;
        let structural_z = resolve_target_coords_3d(
            &TargetKind::Entity(200),
            &entities,
            &rules,
            &interner,
            &terrain,
        )
        .expect("supported structural terrain")
        .2;
        assert_eq!(structural_z, BRIDGE_HEIGHT_DELTA_LEPTONS);
    }

    #[test]
    fn gsi_04_03b_cell_ground_z_uses_center_and_range_bridge_delta() {
        let mut terrain = flat_terrain(1, 1);
        terrain.cells[0].slope_type = 1;
        assert_eq!(ground_z_with_bridge_offset(0, 0, &terrain), Some(52));
        terrain.cells[0].has_bridge_deck = true;
        assert_eq!(
            ground_z_with_bridge_offset(0, 0, &terrain),
            Some(52 + BRIDGE_HEIGHT_DELTA_LEPTONS)
        );
    }

    // Test 6: HighFlying does NOT snap, AirRange bonus applies
    #[test]
    fn high_flying_target_uses_actual_z_with_air_range_bonus() {
        // Attacker has AirRangeBonus=2 cells. weapon.range = 4 cells.
        // Effective max = 6 cells = 1536 lep.
        // dist = sqrt(1024² + 1500²) ≈ 1816 lep > 1536 → false.
        let rules = rules_with_weapon(
            "Damage=1\nROF=20\nRange=4\nWarhead=WH",
            "AirRangeBonus=2",
            "",
        );
        let weapon = rules.weapon("GUN").expect("weapon");
        let attacker = ground_attacker(0, 0, 0, "ATKR");
        let target = aircraft_target(4, 0, 0, 1500, "TGT");
        let mut entities = EntityStore::new();
        entities.insert(target);
        let interner = test_interner();
        let terrain = flat_terrain(64, 64);

        let in_range = compute_in_range(
            &attacker,
            src_at_cell(0, 0, 0),
            &TargetKind::Entity(200),
            weapon,
            &rules,
            &interner,
            &entities,
            &terrain,
        );
        assert!(
            !in_range,
            "high-flying z-delta should still exceed weapon+AirRange budget"
        );
    }

    // Test 7: Foundation bonus on building target
    #[test]
    fn foundation_bonus_extends_range_for_building_target() {
        // Target = 4x2 building at NW corner (0, 0). weapon.range = 4 cells = 1024 lep.
        // Foundation bonus = (4+2) * 64 = 384 lep → effective = 1408 lep.
        //
        // resolve_target_coords_3d shifts the target to the foundation center:
        //   tx = 0*256 + 128 + 3*128 = 512 lep, ty = 0*256 + 128 + 1*128 = 256 lep.
        // Attacker at (6, 1): sx = 6*256 + 128 = 1664 lep, sy = 256 lep.
        // dx = 1152, dy = 0. dist = 1152 lep.
        // 1152 > 1024 (would reject without bonus) and 1152 < 1408 (passes with bonus).
        let ini_str = "[InfantryTypes]\n\n\
                       [VehicleTypes]\n0=ATKR\n\n\
                       [AircraftTypes]\n\n\
                       [BuildingTypes]\n0=BLDG\n\n\
                       [ATKR]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=GUN\n\n\
                       [BLDG]\nStrength=750\nArmor=wood\nFoundation=4x2\n\n\
                       [GUN]\nDamage=1\nROF=20\nRange=4\nWarhead=WH\n\n\
                       [WH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
        let ini = IniFile::from_str(ini_str);
        let rules = RuleSet::from_ini(&ini).expect("rules parse");
        let weapon = rules.weapon("GUN").expect("weapon");

        let attacker = ground_attacker(6, 1, 0, "ATKR");
        let target = building_target(0, 0, "BLDG");
        let mut entities = EntityStore::new();
        entities.insert(target);
        let interner = test_interner();
        let terrain = flat_terrain(64, 64);

        let in_range = compute_in_range(
            &attacker,
            src_at_cell(6, 1, 0),
            &TargetKind::Entity(300),
            weapon,
            &rules,
            &interner,
            &entities,
            &terrain,
        );
        assert!(
            in_range,
            "foundation bonus should extend max range past base 1024 lep"
        );
    }

    // Test 8: Sentinel beats min-range
    #[test]
    fn sentinel_overrides_min_range() {
        let rules = rules_with_weapon(
            "Damage=1\nROF=20\nRange=-2\nMinimumRange=10\nWarhead=WH",
            "",
            "",
        );
        let weapon = rules.weapon("GUN").expect("weapon");
        let attacker = ground_attacker(5, 5, 0, "ATKR");
        let target = ground_target(5, 5, 0, "TGT");
        let mut entities = EntityStore::new();
        entities.insert(target);
        let interner = test_interner();
        let terrain = flat_terrain(64, 64);

        let in_range = compute_in_range(
            &attacker,
            src_at_cell(5, 5, 0),
            &TargetKind::Entity(200),
            weapon,
            &rules,
            &interner,
            &entities,
            &terrain,
        );
        assert!(in_range, "sentinel should bypass min-range gate");
    }

    // Test 9: Cell target uses 3D distance, no bonuses
    #[test]
    fn cell_target_uses_3d_distance_no_bonuses() {
        let rules = rules_with_weapon("Damage=1\nROF=20\nRange=2\nWarhead=WH", "", "");
        let weapon = rules.weapon("GUN").expect("weapon");
        // Attacker on level 5 (= 520 lep up). Target = cell at same XY, level 0.
        // Range = 2 cells = 512 lep. dz = 520 lep > 512 → false.
        let attacker = ground_attacker(5, 5, 5, "ATKR");
        let entities = EntityStore::new();
        let interner = test_interner();
        let terrain = flat_terrain(64, 64);
        let src = (5i64 * 256 + 128, 5i64 * 256 + 128, 5i64 * LEPTONS_PER_LEVEL);

        let in_range = compute_in_range(
            &attacker,
            src,
            &TargetKind::Cell(5, 5),
            weapon,
            &rules,
            &interner,
            &entities,
            &terrain,
        );
        assert!(
            !in_range,
            "5-level z-delta should reject cell-target via 3D dist"
        );
    }

    // Test 10: Arcing weapon falls through to 2D
    #[test]
    fn arcing_weapon_uses_2d_distance() {
        // Weapon with arcing projectile. 4 cells horizontal, 5 levels up.
        // 2D dist = 4 cells = 1024 lep == range → true.
        // 3D dist would be sqrt(1024² + 520²) ≈ 1149 lep > 1024 → would reject.
        let ini_str = "[InfantryTypes]\n\n\
                       [VehicleTypes]\n0=ATKR\n1=TGT\n\n\
                       [AircraftTypes]\n\n\
                       [BuildingTypes]\n\n\
                       [ATKR]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=GUN\n\n\
                       [TGT]\nStrength=300\nArmor=heavy\nSpeed=6\n\n\
                       [GUN]\nDamage=1\nROF=20\nRange=4\nWarhead=WH\nProjectile=ARC\n\n\
                       [ARC]\nArcing=yes\n\n\
                       [WH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
        let ini = IniFile::from_str(ini_str);
        let rules = RuleSet::from_ini(&ini).expect("rules parse");
        let weapon = rules.weapon("GUN").expect("weapon");

        let attacker = ground_attacker(0, 0, 0, "ATKR");
        let target = ground_target(4, 0, 5, "TGT");
        let mut entities = EntityStore::new();
        entities.insert(target);
        let interner = test_interner();
        let terrain = flat_terrain(64, 64);

        let in_range = compute_in_range(
            &attacker,
            src_at_cell(0, 0, 0),
            &TargetKind::Entity(200),
            weapon,
            &rules,
            &interner,
            &entities,
            &terrain,
        );
        assert!(
            in_range,
            "arcing weapon should ignore z-delta and use 2D distance"
        );
    }

    // ─── `TechnoClass::CanFireAt` 0x006F77B0 source-coordinate construction ──

    /// Stock `[ZEP]` / `[BlimpBomb]` as `ini/rulesmd.ini` carries them, with
    /// the two keys this group exercises left exactly as authored.
    fn kirov_rules(weapon_extra: &str) -> RuleSet {
        let ini_str = format!(
            "[InfantryTypes]\n\n\
             [VehicleTypes]\n0=ZEP\n1=TGT\n\n\
             [AircraftTypes]\n\n\
             [BuildingTypes]\n\n\
             [ZEP]\nStrength=2000\nArmor=medium\nSpeed=5\nPrimary=BlimpBomb\n\
             JumpjetHeight=750\nBalloonHover=yes\n\n\
             [TGT]\nStrength=300\nArmor=heavy\nSpeed=6\n\n\
             [BlimpBomb]\nDamage=250\nBurst=1\nROF=50\nRange=1.5\n{weapon_extra}\n\
             Speed=20\nWarhead=BlimpHE\nOmniFire=yes\n\n\
             [BlimpHE]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
        );
        let ini = IniFile::from_str(&ini_str);
        RuleSet::from_ini(&ini).expect("rules parse")
    }

    fn kirov_at(rx: u16, ry: u16, altitude_lep: i64) -> GameEntity {
        let mut e = jumpjet_unit_at_altitude(altitude_lep);
        e.stable_id = 100;
        e.position.rx = rx;
        e.position.ry = ry;
        e.type_ref = crate::sim::intern::test_intern("ZEP");
        e
    }

    fn kirov_can_engage(rules: &RuleSet, kirov: &GameEntity, target: GameEntity) -> bool {
        let interner = test_interner();
        let terrain = flat_terrain(64, 64);
        let mut entities = EntityStore::new();
        entities.insert(target);
        let weapon = rules.weapon("BlimpBomb").expect("weapon");
        let target = TargetKind::Entity(200);
        let src =
            fire_source_coords(kirov, &target, weapon, &entities, &terrain).expect("source coords");
        compute_in_range(
            kirov, src, &target, weapon, rules, &interner, &entities, &terrain,
        )
    }

    /// The headline symptom this mechanism exists to remove: a stock Kirov
    /// could not drop its bomb on anything. `Range=1.5` was truncated to one
    /// cell and the airship's own 750-lepton altitude was charged to the
    /// distance, so 0x006F75F2's `CMP EAX,EBX` could never pass.
    #[test]
    fn stock_kirov_engages_a_ground_target_at_its_authored_range() {
        let rules = kirov_rules("CellRangefinding=yes");
        assert_eq!(
            rules.weapon("BlimpBomb").unwrap().range_leptons,
            384,
            "Range=1.5 is 384 leptons"
        );

        let kirov = kirov_at(5, 5, KIROV_JUMPJET_HEIGHT_LEPTONS);
        assert!(
            kirov_can_engage(&rules, &kirov, ground_target(6, 5, 0, "TGT")),
            "a cruising Kirov must be able to bomb the cell next to it"
        );

        // The fraction itself, pinned at the boundary: the Kirov fires from the
        // centre of (5,5) = 1408 leptons, so a target at x = 1792 is exactly
        // 384 away and one lepton further is out. Truncating `Range=1.5` to a
        // whole cell refuses both.
        let mut at_reach = ground_target(7, 5, 0, "TGT");
        at_reach.position.sub_x = SIM_ZERO;
        assert!(
            kirov_can_engage(&rules, &kirov, at_reach),
            "384 leptons of separation is inside 384 leptons of reach"
        );

        let mut past_reach = ground_target(7, 5, 0, "TGT");
        past_reach.position.sub_x = SimFixed::from_num(1);
        assert!(
            !kirov_can_engage(&rules, &kirov, past_reach),
            "385 leptons is out — Range=1.5 must not become an unbounded reach"
        );
    }

    /// `MOV AL,byte ptr [EDI + 0x134]` at 0x006F7803, then
    /// `CellClass::GetCoords` at 0x006F7866: the attacker fires from its own
    /// cell CENTRE at the ground under it, whatever its sub-cell offset and
    /// altitude. Isolated from the high-flying substitution by keeping the
    /// attacker under the 208-lepton split.
    #[test]
    fn cell_rangefinding_moves_the_shot_to_the_attackers_cell_centre() {
        let terrain = flat_terrain(64, 64);
        let entities = EntityStore::new();
        let target = TargetKind::Cell(9, 9);

        let mut low = jumpjet_unit_at_altitude(HIGH_FLIGHT_THRESHOLD_LEPTONS - 8);
        low.position.rx = 5;
        low.position.ry = 5;
        low.position.sub_x = SimFixed::from_num(3);
        low.position.sub_y = SimFixed::from_num(250);

        let without = kirov_rules("");
        assert_eq!(
            fire_source_coords(
                &low,
                &target,
                without.weapon("BlimpBomb").unwrap(),
                &entities,
                &terrain,
            ),
            Some((
                5 * 256 + 3,
                5 * 256 + 250,
                HIGH_FLIGHT_THRESHOLD_LEPTONS - 8
            )),
            "without the key the attacker's own world point is used"
        );

        let with = kirov_rules("CellRangefinding=yes");
        assert_eq!(
            fire_source_coords(
                &low,
                &target,
                with.weapon("BlimpBomb").unwrap(),
                &entities,
                &terrain,
            ),
            Some((5 * 256 + 128, 5 * 256 + 128, 0)),
            "the key replaces X/Y with the cell centre and Z with the ground"
        );
    }

    /// `MOV CL,byte ptr [ESI + 0x8C]` at 0x006F7876 — the deck offset the
    /// CellRangefinding branch adds comes from the ATTACKER's OnBridge byte,
    /// not from the cell (`CellClass::GetCoords` carries no deck term).
    #[test]
    fn cell_rangefinding_adds_the_deck_offset_from_the_attackers_own_flag() {
        let rules = kirov_rules("CellRangefinding=yes");
        let weapon = rules.weapon("BlimpBomb").unwrap();
        let entities = EntityStore::new();
        let terrain = terrain_with_bridge_cells(&[(5, 5)]);
        let target = TargetKind::Cell(9, 9);

        let mut ground = jumpjet_unit_at_altitude(0);
        ground.position.rx = 5;
        ground.position.ry = 5;
        assert_eq!(
            fire_source_coords(&ground, &target, weapon, &entities, &terrain).map(|(_, _, z)| z),
            Some(0),
            "standing under the deck keeps ground Z"
        );

        ground.on_bridge = true;
        assert_eq!(
            fire_source_coords(&ground, &target, weapon, &entities, &terrain).map(|(_, _, z)| z),
            Some(BRIDGE_HEIGHT_DELTA_LEPTONS)
        );
    }

    /// `CALL dword ptr [EDX + 0x54]` at 0x006F7895: a high-flying attacker
    /// measures from the TARGET's own Z, so its altitude never enters the
    /// distance. Isolated from CellRangefinding — no `CellRangefinding=` here.
    #[test]
    fn a_high_flying_attacker_measures_from_the_targets_own_z() {
        let rules = kirov_rules("");
        let weapon = rules.weapon("BlimpBomb").unwrap();
        let interner = test_interner();
        // The airship hovers over a level-5 plateau and the target stands on
        // level-0 ground one cell away, so the height term is what decides.
        let mut terrain = flat_terrain(64, 64);
        terrain.cells[5 * 64 + 5].level = 5;
        let mut entities = EntityStore::new();
        entities.insert(ground_target(6, 5, 0, "TGT"));
        let target = TargetKind::Entity(200);

        let cruising = kirov_at(5, 5, KIROV_JUMPJET_HEIGHT_LEPTONS);
        let src = fire_source_coords(&cruising, &target, weapon, &entities, &terrain).unwrap();
        assert_eq!(
            src,
            (5 * 256 + 128, 5 * 256 + 128, 0),
            "Z becomes the target's own coordinate, not the 520 + 750 it stands at"
        );
        assert!(compute_in_range(
            &cruising, src, &target, weapon, &rules, &interner, &entities, &terrain,
        ));

        // The same airship one lepton below the split is NOT high-flying, so
        // native charges its altitude and the shot is refused.
        let climbing = kirov_at(5, 5, HIGH_FLIGHT_THRESHOLD_LEPTONS - 1);
        let climbing_src =
            fire_source_coords(&climbing, &target, weapon, &entities, &terrain).unwrap();
        assert_eq!(climbing_src.2, 5 * 104 + HIGH_FLIGHT_THRESHOLD_LEPTONS - 1);
        assert!(
            !compute_in_range(
                &climbing,
                climbing_src,
                &target,
                weapon,
                &rules,
                &interner,
                &entities,
                &terrain,
            ),
            "below the split the attacker's own height is still charged"
        );
    }

    /// `MOV ECX,[EDX+0xB8]` at 0x006F737F runs the MinimumRange test in 3-D and
    /// BEFORE the `MOV CL,[EDX+0x29B]` arcing split at 0x006F73F6, so an arcing
    /// weapon's minimum reach counts the height difference too.
    #[test]
    fn min_range_is_3d_and_runs_before_the_arcing_split() {
        let ini_str = "[InfantryTypes]\n\n\
                       [VehicleTypes]\n0=ATKR\n1=TGT\n\n\
                       [AircraftTypes]\n\n\
                       [BuildingTypes]\n\n\
                       [ATKR]\nStrength=300\nArmor=heavy\nSpeed=6\nPrimary=GUN\n\n\
                       [TGT]\nStrength=300\nArmor=heavy\nSpeed=6\n\n\
                       [GUN]\nDamage=1\nROF=20\nRange=10\nMinimumRange=2.34\n\
                       Warhead=WH\nProjectile=ARC\n\n\
                       [ARC]\nArcing=yes\n\n\
                       [WH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
        let ini = IniFile::from_str(ini_str);
        let rules = RuleSet::from_ini(&ini).expect("rules parse");
        let weapon = rules.weapon("GUN").expect("weapon");
        assert_eq!(weapon.minimum_range_leptons, 599, "2.34 cells chopped");

        let attacker = ground_attacker(0, 0, 0, "ATKR");
        let interner = test_interner();
        let mut terrain = flat_terrain(64, 64);
        terrain.cells[5].level = 5;
        let mut entities = EntityStore::new();
        entities.insert(ground_target(5, 0, 5, "TGT"));

        // Flat distance is 5 cells; that alone clears the minimum. The point of
        // the test is the pairing: the 3-D leg is the one native measures, and
        // it is measured for an arcing weapon too.
        assert!(compute_in_range(
            &attacker,
            src_at_cell(0, 0, 0),
            &TargetKind::Entity(200),
            weapon,
            &rules,
            &interner,
            &entities,
            &terrain,
        ));

        // Two cells out (512 leptons flat) is inside a 599-lepton minimum, but
        // five terrain levels of climb push the 3-D distance to 730 — legal.
        let mut close = flat_terrain(64, 64);
        close.cells[2].level = 5;
        let mut close_entities = EntityStore::new();
        close_entities.insert(ground_target(2, 0, 5, "TGT"));
        assert!(
            compute_in_range(
                &attacker,
                src_at_cell(0, 0, 0),
                &TargetKind::Entity(200),
                weapon,
                &rules,
                &interner,
                &close_entities,
                &close,
            ),
            "a 2-D minimum-range test would have refused this arcing shot"
        );
    }

    // Test 11: the `TechnoClass::InRange` 0x006F7220 bridge gate (block at
    // 0x006F75FB), plus recorded residuals for the separate bridge gates that
    // live in `TechnoClass::GetFireError` 0x006FC0B0 and are not ported.

    /// 16x16 grid whose listed cells carry `CellClass+0x140` bit 0x100.
    fn terrain_with_bridge_cells(cells_on_bridge: &[(u16, u16)]) -> ResolvedTerrainGrid {
        let mut cells: Vec<ResolvedTerrainCell> = (0..16)
            .flat_map(|ry| (0..16).map(move |rx| default_cell(rx, ry)))
            .collect();
        for &(rx, ry) in cells_on_bridge {
            let idx = ry as usize * 16 + rx as usize;
            cells[idx].has_bridge_deck = true;
            cells[idx].bridge_deck_level = 4;
            cells[idx].bridge_facts.raw_flags |= crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL;
        }
        ResolvedTerrainGrid::from_cells(16, 16, cells)
    }

    fn in_range_at(
        attacker: &GameEntity,
        src: (i64, i64, i64),
        target: &TargetKind,
        terrain: &ResolvedTerrainGrid,
        entities: &EntityStore,
    ) -> bool {
        let rules = rules_with_weapon("Damage=1\nROF=20\nRange=6\nWarhead=WH", "", "");
        let weapon = rules.weapon("GUN").expect("weapon");
        let interner = test_interner();
        compute_in_range(
            attacker, src, target, weapon, &rules, &interner, entities, terrain,
        )
    }

    /// The gate fires: attacker in a bridge cell, below the deck top, target at
    /// the deck top.
    #[test]
    fn inrange_bridge_gate_blocks_under_bridge_attacker_firing_up() {
        let terrain = terrain_with_bridge_cells(&[(5, 5)]);
        let attacker = ground_attacker(5, 5, 0, "ATKR");
        let entities = EntityStore::new();

        let under = (5i64 * 256 + 128, 5i64 * 256 + 128, 0i64);
        assert!(
            !in_range_at(
                &attacker,
                under,
                &TargetKind::Cell(5, 5),
                &terrain,
                &entities
            ),
            "under-bridge attacker firing at the deck must be blocked"
        );
    }

    /// `JGE` at 0x006F7629: an attacker whose Z has reached the deck top is
    /// already allowed, so a deck-standing attacker never trips the gate.
    #[test]
    fn inrange_bridge_gate_allows_attacker_at_or_above_the_deck_top() {
        let terrain = terrain_with_bridge_cells(&[(5, 5)]);
        let attacker = ground_attacker(5, 5, 0, "ATKR");
        let entities = EntityStore::new();

        let on_deck = (
            5i64 * 256 + 128,
            5i64 * 256 + 128,
            BRIDGE_HEIGHT_DELTA_LEPTONS,
        );
        assert!(
            in_range_at(
                &attacker,
                on_deck,
                &TargetKind::Cell(5, 5),
                &terrain,
                &entities
            ),
            "attacker standing on the deck is above the strict source test"
        );
    }

    /// gamemd tests only the SOURCE cell's flag word. A bridge cell under the
    /// target with none under the attacker does not gate anything.
    #[test]
    fn inrange_bridge_gate_reads_only_the_attacker_cell() {
        let terrain = terrain_with_bridge_cells(&[(5, 6)]);
        let attacker = ground_attacker(5, 5, 0, "ATKR");
        let entities = EntityStore::new();

        let src = (5i64 * 256 + 128, 5i64 * 256 + 128, 0i64);
        assert!(
            in_range_at(&attacker, src, &TargetKind::Cell(5, 6), &terrain, &entities),
            "only the attacker's own cell is consulted at 0x006F7601"
        );
    }

    /// A cell carrying `has_bridge_deck` but not flag 0x100 is not a bridge
    /// cell as far as 0x006F760C is concerned. `has_bridge_deck` is derived
    /// from overlay effects in resolved terrain; the binary tests the flag
    /// word, so the gate must follow `BRIDGE_FLAG_STRUCTURAL`.
    #[test]
    fn inrange_bridge_gate_follows_flag_0x100_not_the_overlay_deck() {
        let mut cells: Vec<ResolvedTerrainCell> = (0..16)
            .flat_map(|ry| (0..16).map(move |rx| default_cell(rx, ry)))
            .collect();
        let idx = 5 * 16 + 5;
        cells[idx].has_bridge_deck = true;
        cells[idx].bridge_deck_level = 4;
        let terrain = ResolvedTerrainGrid::from_cells(16, 16, cells);

        let attacker = ground_attacker(5, 5, 0, "ATKR");
        let entities = EntityStore::new();
        let under = (5i64 * 256 + 128, 5i64 * 256 + 128, 0i64);
        assert!(
            in_range_at(
                &attacker,
                under,
                &TargetKind::Cell(5, 5),
                &terrain,
                &entities
            ),
            "no 0x100 flag means no gate"
        );
    }

    /// A target below the deck top clears the inclusive target test.
    #[test]
    fn inrange_bridge_gate_allows_target_below_the_deck_top() {
        let terrain = terrain_with_bridge_cells(&[(5, 5)]);
        let attacker = ground_attacker(5, 5, 0, "ATKR");
        let mut entities = EntityStore::new();
        entities.insert(ground_target(6, 5, 0, "TGT"));

        let under = (5i64 * 256 + 128, 5i64 * 256 + 128, 0i64);
        assert!(
            in_range_at(
                &attacker,
                under,
                &TargetKind::Entity(200),
                &terrain,
                &entities
            ),
            "ground target below the deck top is reachable"
        );
    }

    /// `TechnoClass::GetFireError` 0x006FC0B0 block at 0x006FCBE6, ported as
    /// `fire_error_on_bridge_mismatch`. One test per term of the conjunction.
    fn bridge_pair_terrain() -> ResolvedTerrainGrid {
        let mut terrain = flat_terrain(16, 16);
        for (rx, ry) in [(10u16, 10u16), (11, 10)] {
            let idx = ry as usize * 16 + rx as usize;
            terrain.cells[idx].bridge_facts.raw_flags |=
                crate::map::bridge_facts::BRIDGE_FLAG_STRUCTURAL;
        }
        terrain
    }

    /// Attacker at (10,10), target at (11,10); `on_deck` picks which of the
    /// two carries the OnBridge byte.
    fn mismatched_pair(attacker_on_deck: bool, target_on_deck: bool) -> EntityStore {
        let mut store = EntityStore::new();
        let mut attacker = GameEntity::test_default(1, "MTNK", "Test", 10, 10);
        attacker.category = EntityCategory::Unit;
        if attacker_on_deck {
            attacker.bridge_occupancy =
                Some(crate::sim::components::BridgeOccupancy { deck_level: 4 });
        }
        let mut target = GameEntity::test_default(2, "MTNK", "Test", 11, 10);
        target.category = EntityCategory::Unit;
        if target_on_deck {
            target.bridge_occupancy =
                Some(crate::sim::components::BridgeOccupancy { deck_level: 4 });
        }
        store.insert(attacker);
        store.insert(target);
        store
    }

    #[test]
    fn getfireerror_refuses_a_shot_from_the_deck_down_at_a_unit_beneath_it() {
        // The half `InRange` 0x006F75FB does NOT cover: the attacker is on the
        // deck, so its own under-bridge height clause never fires.
        let terrain = bridge_pair_terrain();
        let store = mismatched_pair(true, false);
        let attacker = store.get(1).unwrap();
        assert!(fire_error_on_bridge_mismatch(
            attacker,
            &crate::sim::combat::TargetKind::Entity(2),
            &store,
            &terrain,
        ));
    }

    #[test]
    fn getfireerror_refuses_the_shot_from_underneath_as_well() {
        let terrain = bridge_pair_terrain();
        let store = mismatched_pair(false, true);
        let attacker = store.get(1).unwrap();
        assert!(fire_error_on_bridge_mismatch(
            attacker,
            &crate::sim::combat::TargetKind::Entity(2),
            &store,
            &terrain,
        ));
    }

    #[test]
    fn getfireerror_allows_the_shot_when_both_agree_on_onbridge() {
        // CMP CL,AL / JZ — equal OnBridge bytes skip the whole block.
        let terrain = bridge_pair_terrain();
        for both in [false, true] {
            let store = mismatched_pair(both, both);
            let attacker = store.get(1).unwrap();
            assert!(
                !fire_error_on_bridge_mismatch(
                    attacker,
                    &crate::sim::combat::TargetKind::Entity(2),
                    &store,
                    &terrain,
                ),
                "both on_bridge={both} must not refuse"
            );
        }
    }

    #[test]
    fn getfireerror_needs_both_cells_to_carry_the_structural_flag() {
        // Same mismatch, but one of the two cells is ordinary ground.
        let store = mismatched_pair(true, false);
        let attacker = store.get(1).unwrap();
        for drop_idx in [10usize * 16 + 10, 10 * 16 + 11] {
            let mut terrain = bridge_pair_terrain();
            terrain.cells[drop_idx].bridge_facts.raw_flags = 0;
            assert!(
                !fire_error_on_bridge_mismatch(
                    attacker,
                    &crate::sim::combat::TargetKind::Entity(2),
                    &store,
                    &terrain,
                ),
                "cell {drop_idx} without 0x100 must not refuse"
            );
        }
    }

    #[test]
    fn getfireerror_exempts_a_high_flying_attacker() {
        // vtable+0x54. An aircraft at or above 2 level heights is exempt; the
        // same aircraft sitting on the deck is not.
        let terrain = bridge_pair_terrain();
        let mut store = EntityStore::new();
        let mut flyer = aircraft_at_altitude(HIGH_FLIGHT_THRESHOLD_LEPTONS);
        flyer.position.rx = 10;
        flyer.position.ry = 10;
        let mut target = GameEntity::test_default(3, "MTNK", "Test", 11, 10);
        target.category = EntityCategory::Unit;
        target.bridge_occupancy = Some(crate::sim::components::BridgeOccupancy { deck_level: 4 });
        store.insert(flyer);
        store.insert(target);
        let flyer_ref = store.get(2).unwrap();
        assert!(!fire_error_on_bridge_mismatch(
            flyer_ref,
            &crate::sim::combat::TargetKind::Entity(3),
            &store,
            &terrain,
        ));
    }

    #[test]
    fn getfireerror_skips_a_cell_target() {
        // EBP is null unless the target narrows to a TechnoClass, and the tail
        // is guarded by TEST EBP,EBP at 0x006FCAFA.
        let terrain = bridge_pair_terrain();
        let store = mismatched_pair(true, false);
        let attacker = store.get(1).unwrap();
        assert!(!fire_error_on_bridge_mismatch(
            attacker,
            &crate::sim::combat::TargetKind::Cell(11, 10),
            &store,
            &terrain,
        ));
    }

    /// RESIDUAL — gamemd address 0x006FC0B0, fall-through of the same block at
    /// 0x006FCC5D–0x006FCCBA.
    ///
    /// Mechanism: when the warhead byte at `WeaponType+0xAC` `+0x159` is set
    /// and `abs(attacker.Z - target.Z)` exceeds
    /// `2 * g_nTechnoInRangeLevelHeightLeptons` (`LEA EDX,[ECX+ECX]` at
    /// 0x006FCCA7), the call returns FireError 5 at 0x006FCCAE. Reached under
    /// the same `OnBridge`-mismatch guard as the gate above.
    ///
    /// Trigger: a warhead whose `Parasite=` flag is set, fired across a
    /// deck at a height difference over two level heights. The byte is
    /// written by `WarheadTypeClass::ReadINI` at 0x0075D84E from the key
    /// string at 0x0081717C, which `read_memory` gives as `Parasite`.
    ///
    /// Effect: unmodelled; VERA allows shots gamemd refuses.
    ///
    /// Frequency: bounded to the three stock warheads carrying
    /// `Parasite=yes` in `ini/rulesmd.ini` - `[Parasite]` (Terror Drone),
    /// `[ParasiteDog]` (attack dog) and `[ParasitePlus]` (Giant Squid) -
    /// each attacking across a bridge deck. Uncommon, but all three are
    /// ordinary skirmish units rather than edge cases.
    #[test]
    #[ignore = "gamemd 0x006FCCAE Parasite-warhead height clause across a deck is unported"]
    fn getfireerror_bridge_height_clause_is_unported() {
        panic!("unimplemented: GetFireError 0x006FCCAE warhead +0x159 height clause");
    }

    /// RESIDUAL — gamemd address 0x006FC612, calling
    /// `TechnoClass::IsOnBridge_ForFiring` 0x00703B10.
    ///
    /// Mechanism: `MOV AL,[EDI+0x131]` at 0x006FC606 gates on the weapon-type
    /// byte that `WeaponTypeClass::ReadINI` 0x007720FA fills from the INI key
    /// `Spawner=` (key string at 0x00849538); the call to
    /// `SpawnManagerClass::CountAliveSpawns` 0x006B7D30 just after confirms it.
    /// When set, `IsOnBridge_ForFiring` non-zero returns FireError 6 at
    /// 0x006FCD29. That predicate is NOT the mismatch gate: it early-outs to 0
    /// when the object's own `OnBridge` byte +0x8C is set, then tests the
    /// object's own cell for flag 0x100 plus four neighbour cells, each
    /// qualified by that neighbour's orientation bit 0x800 matching the axis it
    /// lies on.
    ///
    /// Trigger: a `Spawner=` unit (aircraft carrier, Boomer sub) standing
    /// UNDER or immediately beside a bridge deck cell. Not on the deck — the
    /// +0x8C early-out exempts a unit that is actually on it.
    ///
    /// Effect: gamemd refuses to launch spawns; VERA launches them.
    ///
    /// Frequency: occasional — needs a naval or amphibious map with a bridge
    /// over water and a spawner unit parked at it.
    #[test]
    #[ignore = "gamemd 0x006FC612 blocks spawner weapons on/beside a bridge; VERA does not"]
    fn getfireerror_spawner_bridge_block_is_unported() {
        panic!("unimplemented: GetFireError 0x006FC612 IsOnBridge_ForFiring spawner gate");
    }

    /// RESIDUAL — gamemd address 0x006F7220, the arcing branch of
    /// `TechnoClass::InRange`.
    ///
    /// Branch selector: `0x006F73F6 MOV CL,[EDX+0x29B]` with `EDX` = the
    /// projectile at `WeaponType+0xA0`. `BulletTypeClass::ReadINI` 0x0046BFC4
    /// fills +0x29B from the INI key `Arcing=` (key string at 0x0081B130). It
    /// is NOT `WeaponType+0xB8`, which is read at 0x006F737F and gates only the
    /// MinimumRange test.
    ///
    /// Clause, at 0x006F74D7–0x006F7504: the arc/slope test at 0x0048ABC0 must
    /// pass in every case, and when it does, the shot is additionally refused
    /// unless the TARGET's cell has flag 0x100 clear, or
    /// `target.Z - source.Z < 3 * g_nTechnoInRangeLevelHeightLeptons`. A bridge
    /// cell under the target therefore TIGHTENS the check with an extra height
    /// ceiling; it does not relax it. Note this reads the target's cell, the
    /// opposite of the gate at 0x006F75FB in the same function, which reads the
    /// source's.
    ///
    /// Trigger: arcing weapons (V3, Dreadnought, artillery, Apocalypse rocket)
    /// firing at something standing on a bridge deck.
    ///
    /// Effect: `compute_in_range_arcing_2d` is a documented 2D fallthrough stub
    /// with neither the slope test nor this ceiling, so VERA allows arcing
    /// shots at deck targets that gamemd refuses.
    ///
    /// Frequency: every arcing shot at a unit on a bridge. Bounded by the fact
    /// that the whole arc check is stubbed, so this clause is downstream of a
    /// larger unported mechanism and cannot be fixed on its own.
    #[test]
    #[ignore = "gamemd 0x006F74D7 adds a height ceiling for arcing shots at bridge-cell targets; VERA's arcing path is a 2D stub"]
    fn inrange_arcing_branch_bridge_ceiling_is_unported() {
        panic!("unimplemented: InRange 0x006F74D7 arcing bridge height ceiling");
    }

    /// RESIDUAL — gamemd address 0x006F7642, the `CALL 0x004CC310` whose
    /// result decides `InRange`'s final `return`.
    ///
    /// Mechanism: after the distance test passes, `InRange` calls
    /// `FUN_004CC310(weapon, this->+0x21C)` with the source and target
    /// coordinates and returns `result == 0`. That function delegates to
    /// `FUN_004CC100`, which — only when the projectile at `WeaponType+0xA0`
    /// sets `SubjectToCliffs` (`BulletType+0x296`, key string 0x0081B118) or
    /// `SubjectToWalls` (`+0x298`, key string 0x0081B0F4) — walks the line
    /// source→target one cell at a time (Chebyshev step count, per-axis
    /// integer division for the increments) and asks `FUN_004CC360` whether
    /// each cell blocks. A nonzero answer means blocked, and
    /// `CellClass::IsWallConnectableInDirection` 0x00480510 combined with the
    /// warhead byte at `WeaponType+0xAC` `+0x144` re-admits the shot when the
    /// warhead may break the wall itself.
    ///
    /// Trigger: any shot whose projectile is `SubjectToWalls=` or
    /// `SubjectToCliffs=` and that has a wall or cliff on the line.
    ///
    /// Effect: gamemd reports "not in range" and the attacker never fires;
    /// VERA passes the gate, fires, and leaves the outcome to the projectile's
    /// own in-flight wall handling in `sim/projectile.rs`. Target selection and
    /// pursuit also disagree, since both run through this same predicate.
    ///
    /// Frequency: 10 of the 30 stock projectiles carry `SubjectToWalls=yes`
    /// and 9 `SubjectToCliffs=yes`, covering most direct-fire ground weapons —
    /// so this fires whenever such a unit shoots across a wall or cliff, which
    /// is ordinary skirmish, not an edge case. Not ported here because the
    /// cell-blocking predicate `FUN_004CC360` is its own mechanism.
    #[test]
    #[ignore = "gamemd 0x006F7642 refuses the shot when a wall or cliff blocks the line; VERA's range gate has no line walk"]
    fn inrange_line_of_fire_block_is_unported() {
        panic!("unimplemented: InRange 0x006F7642 SubjectToWalls/SubjectToCliffs line walk");
    }

    /// RESIDUAL — gamemd address 0x006F7314, `CALL dword ptr [EDX + 0x48]` on
    /// the target.
    ///
    /// Mechanism: `InRange` reads the target's coordinate through
    /// `AbstractClass::GetCoords` (`vtable+0x48`). For a force-fire cell
    /// target that resolves to `CellClass::GetCoords` 0x00486840, which
    /// carries NO bridge-deck term — the bridge-aware aim point is a separate
    /// slot, `CellClass::GetTargetCoords` 0x00486890 at `vtable+0x58`, and
    /// this callsite does not use it.
    ///
    /// `resolve_target_coords_3d` adds `BRIDGE_HEIGHT_DELTA_LEPTONS` for a
    /// `TargetKind::Cell` whose terrain carries a deck.
    ///
    /// Trigger: force-firing (Ctrl-click) at a cell that has a bridge deck.
    ///
    /// Effect: VERA measures to the deck top where gamemd measures to the
    /// ground under it, so the range verdict differs by up to 416 leptons of
    /// height — about 1.6 cells of reach on a shot straight up or down.
    ///
    /// Frequency: uncommon — needs a deliberate force-fire on a bridge cell.
    /// Left recorded rather than changed because it belongs to the cell-target
    /// aim-point question (which slot each consumer reads), not to this
    /// mechanism.
    #[test]
    #[ignore = "gamemd 0x006F7314 reads CellClass::GetCoords (no deck term) for a cell target; VERA adds the deck offset"]
    fn inrange_cell_target_deck_offset_is_drift() {
        panic!("unimplemented: InRange 0x006F7314 cell-target coordinate has no bridge deck term");
    }

    /// Guards the INCLUSIVE half of the gate's boundary pair (`JGE` at
    /// 0x006F762F): a target exactly on the deck top is refused. Without this
    /// the target comparison could be inverted to `>` and every other test in
    /// this group would still pass.
    #[test]
    fn inrange_bridge_gate_target_boundary_is_inclusive() {
        let terrain = terrain_with_bridge_cells(&[(5, 5)]);
        let attacker = ground_attacker(5, 5, 0, "ATKR");
        let mut entities = EntityStore::new();
        let mut target = ground_target(6, 5, 0, "TGT");
        target.on_bridge = true;
        entities.insert(target);

        let under = (5i64 * 256 + 128, 5i64 * 256 + 128, 0i64);
        assert!(
            !in_range_at(
                &attacker,
                under,
                &TargetKind::Entity(200),
                &terrain,
                &entities
            ),
            "a target sitting exactly at the deck top must be refused"
        );
    }
}
