//! 3D weapon range check matching the original engine's TechnoClass::InRange.
//!
//! Replaces the 2D `lepton_distance_sq_raw` + `is_within_range_leptons` pair
//! at the four targeting/cursor sites. Stage 1 implements 3D distance,
//! IsLowFlying ground-snap, AirRange bonus, arcing-weapon 2D fallthrough,
//! foundation bonus, bridge LOS gate, and the verified boundary semantics
//! (<= max inclusive, < min strict, -512 lep sentinel).
//!
//! Stages 2-N add the remaining range-VALUE chain (Bunker / OpenTopped /
//! Veteran). Stage Arcing adds the full Branch B slope-arc check.
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
use crate::sim::production::foundation_dimensions;
use crate::util::fixed_math::{SIM_ZERO, SimFixed, isqrt_i64};
use crate::util::lepton::{
    BRIDGE_HEIGHT_DELTA_LEPTONS, HIGH_FLIGHT_THRESHOLD_LEPTONS, LEPTONS_PER_LEVEL,
    WEAPON_RANGE_ALWAYS_IN_RANGE_LEPTONS,
};

/// Combined absolute Z of an entity in leptons (cell elevation × 104 +
/// locomotor altitude for airborne entities). Returns a single absolute-leptons
/// value rather than separate level + altitude fields.
///
/// Droppod and parachute altitudes are intentionally NOT added — those
/// entities are always IsLowFlying-equivalent during descent and get
/// ground-snapped by the InRange caller.
pub(crate) fn effective_z_leptons(entity: &GameEntity) -> i64 {
    let base = entity.position.z as i64 * LEPTONS_PER_LEVEL;
    match entity.locomotor.as_ref() {
        Some(loco) => base + loco.altitude.to_num::<i64>(),
        None => base,
    }
}

/// Entity is currently airborne and below the high-flight threshold.
///
/// Used by the InRange caller to decide whether to ground-snap the target's
/// Z before the distance computation: low-flying targets are ranged at the
/// ground beneath them, not at their actual altitude.
pub(crate) fn is_low_flying(entity: &GameEntity) -> bool {
    if entity.category != EntityCategory::Aircraft {
        return false;
    }
    let alt = entity
        .locomotor
        .as_ref()
        .map(|l| l.altitude.to_num::<i64>())
        .unwrap_or(0);
    alt > 0 && alt < HIGH_FLIGHT_THRESHOLD_LEPTONS
}

/// Entity is currently airborne and at or above the high-flight threshold.
/// Mutually exclusive with `is_low_flying` for airborne units.
///
/// Used by the InRange caller to enable the AirRange bonus on the attacker's
/// weapon when the target is high-flying.
pub(crate) fn is_high_flying(entity: &GameEntity) -> bool {
    if entity.category != EntityCategory::Aircraft {
        return false;
    }
    let alt = entity
        .locomotor
        .as_ref()
        .map(|l| l.altitude.to_num::<i64>())
        .unwrap_or(0);
    alt >= HIGH_FLIGHT_THRESHOLD_LEPTONS
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
    let mut range_lep: i64 = weapon.range.to_num::<i64>() * 256;

    if let TargetKind::Entity(target_id) = *target {
        if let Some(target_entity) = entities.get(target_id) {
            // AirRange bonus when target is high-flying.
            if is_high_flying(target_entity) {
                if let Some(attacker_obj) = rules.object(interner.resolve(attacker.type_ref)) {
                    if let Some(air_bonus) = attacker_obj.air_range_bonus {
                        range_lep += air_bonus.to_num::<i64>() * 256;
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
/// effective_z_leptons(attacker))`.
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
    let weapon_range_lep: i64 = weapon.range.to_num::<i64>() * 256;

    // Sentinel — always-in-range short-circuit.
    if weapon_range_lep == WEAPON_RANGE_ALWAYS_IN_RANGE_LEPTONS {
        return true;
    }

    // Arcing-weapon 2D fallthrough — preserves V3/Prism/etc. current behavior.
    let arcing = weapon
        .projectile
        .as_deref()
        .and_then(|name| rules.projectile(name))
        .map(|p| p.arcing)
        .unwrap_or(false);
    if arcing {
        return compute_in_range_arcing_2d(src, target, weapon, rules, interner, entities);
    }

    let max_range_lep =
        compute_effective_max_range_leptons(attacker, target, weapon, rules, interner, entities);

    let (tx, ty, tz) = resolve_target_coords_3d(target, entities, rules, interner, terrain);

    let (sx, sy, sz) = src;
    let dx = sx - tx;
    let dy = sy - ty;
    let dz = sz - tz;
    let dist_sq: i64 = dx * dx + dy * dy + dz * dz;
    let dist_lep = isqrt_i64(dist_sq);

    if weapon.minimum_range > SIM_ZERO {
        let min_range_lep = weapon.minimum_range.to_num::<i64>() * 256;
        if dist_lep < min_range_lep {
            return false;
        }
    }

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
fn compute_in_range_arcing_2d(
    src: (i64, i64, i64),
    target: &TargetKind,
    weapon: &WeaponType,
    rules: &RuleSet,
    interner: &StringInterner,
    entities: &EntityStore,
) -> bool {
    let weapon_range_lep: i64 = weapon.range.to_num::<i64>() * 256;
    if weapon_range_lep == WEAPON_RANGE_ALWAYS_IN_RANGE_LEPTONS {
        return true;
    }

    let (tx, ty) = resolve_target_xy_2d(target, entities, rules, interner);

    let (sx, sy, _sz) = src;
    let dx = sx - tx;
    let dy = sy - ty;
    let dist_sq: i64 = dx * dx + dy * dy;
    let dist_lep = isqrt_i64(dist_sq);

    if weapon.minimum_range > SIM_ZERO {
        let min_range_lep = weapon.minimum_range.to_num::<i64>() * 256;
        if dist_lep < min_range_lep {
            return false;
        }
    }
    dist_lep <= weapon_range_lep
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
) -> (i64, i64, i64) {
    match *target {
        TargetKind::Entity(id) => {
            let Some(t) = entities.get(id) else {
                return (i64::MAX / 4, i64::MAX / 4, 0);
            };
            let (rx, ry, sub_x, sub_y) = resolve_entity_target_coords(t, rules, interner);
            let tx = rx as i64 * 256 + sub_x.to_num::<i64>();
            let ty = ry as i64 * 256 + sub_y.to_num::<i64>();
            let tz = if is_low_flying(t) {
                ground_z_with_bridge_offset(rx, ry, terrain)
            } else {
                effective_z_leptons(t)
            };
            (tx, ty, tz)
        }
        TargetKind::Cell(rx, ry) => {
            let tx = rx as i64 * 256 + 128;
            let ty = ry as i64 * 256 + 128;
            let tz = ground_z_with_bridge_offset(rx, ry, terrain);
            (tx, ty, tz)
        }
    }
}

/// 2D-only target XY for arcing weapons (no LowFlying snap, no terrain).
/// Mirrors the foundation-center adjustment for buildings.
fn resolve_target_xy_2d(
    target: &TargetKind,
    entities: &EntityStore,
    rules: &RuleSet,
    interner: &StringInterner,
) -> (i64, i64) {
    match *target {
        TargetKind::Entity(id) => {
            let Some(t) = entities.get(id) else {
                return (i64::MAX / 4, i64::MAX / 4);
            };
            let (rx, ry, sub_x, sub_y) = resolve_entity_target_coords(t, rules, interner);
            (
                rx as i64 * 256 + sub_x.to_num::<i64>(),
                ry as i64 * 256 + sub_y.to_num::<i64>(),
            )
        }
        TargetKind::Cell(rx, ry) => (rx as i64 * 256 + 128, ry as i64 * 256 + 128),
    }
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
fn ground_z_with_bridge_offset(rx: u16, ry: u16, terrain: &ResolvedTerrainGrid) -> i64 {
    let cell_idx = ry as usize * terrain.width() as usize + rx as usize;
    let cell = match terrain.cells.get(cell_idx) {
        Some(c) => c,
        None => return 0,
    };
    let mut z = cell.level as i64 * LEPTONS_PER_LEVEL;
    if cell.has_bridge_deck {
        z += BRIDGE_HEIGHT_DELTA_LEPTONS;
    }
    z
}

/// Bridge LOS gate: returns true when the attacker is in a bridge cell, at a
/// Z below the bridge deck top, and the target Z is at or above the deck top
/// — meaning the attacker would have to fire through the deck.
fn attacker_under_bridge_targeting_above(
    src: (i64, i64, i64),
    target_z_lep: i64,
    terrain: &ResolvedTerrainGrid,
) -> bool {
    let (sx, sy, sz) = src;
    let rx = (sx / 256) as u16;
    let ry = (sy / 256) as u16;
    let cell_idx = ry as usize * terrain.width() as usize + rx as usize;
    let cell = match terrain.cells.get(cell_idx) {
        Some(c) => c,
        None => return false,
    };
    if !cell.has_bridge_deck {
        return false;
    }
    let bridge_top = cell.level as i64 * LEPTONS_PER_LEVEL + BRIDGE_HEIGHT_DELTA_LEPTONS;
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
    use crate::sim::movement::locomotor::{
        AirMovePhase, GroundMovePhase, LocomotorState, MovementLayer,
    };
    use crate::util::fixed_math::{SIM_ONE, SIM_ZERO, SimFixed};

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
            jumpjet_wobbles: 0.0,
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
            override_state: None,
            air_progress: SIM_ZERO,
            infantry_wobble_phase: 0.0,
            subcell_dest: None,
        });
        e
    }

    #[test]
    fn effective_z_ground_unit() {
        let e = ground_entity_at_level(5);
        assert_eq!(effective_z_leptons(&e), 5 * LEPTONS_PER_LEVEL);
    }

    #[test]
    fn effective_z_airborne_aircraft_adds_altitude() {
        let mut e = aircraft_at_altitude(1500);
        e.position.z = 0;
        assert_eq!(effective_z_leptons(&e), 1500);

        let mut e2 = aircraft_at_altitude(800);
        e2.position.z = 2;
        assert_eq!(effective_z_leptons(&e2), 2 * LEPTONS_PER_LEVEL + 800);
    }

    #[test]
    fn is_low_flying_only_for_airborne_aircraft() {
        let ground = ground_entity_at_level(5);
        assert!(!is_low_flying(&ground));

        let grounded_air = aircraft_at_altitude(0);
        assert!(!is_low_flying(&grounded_air));

        let low = aircraft_at_altitude(500);
        assert!(is_low_flying(&low));

        let high = aircraft_at_altitude(1500);
        assert!(!is_low_flying(&high));
    }

    #[test]
    fn is_high_flying_inverse_threshold() {
        let just_below = aircraft_at_altitude(999);
        assert!(!is_high_flying(&just_below));

        let at_threshold = aircraft_at_altitude(HIGH_FLIGHT_THRESHOLD_LEPTONS);
        assert!(is_high_flying(&at_threshold));

        let cruise = aircraft_at_altitude(1500);
        assert!(is_high_flying(&cruise));

        let ground = ground_entity_at_level(5);
        assert!(!is_high_flying(&ground));
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
            level: 0,
            filled_clear: true,
            tileset_index: Some(0),
            land_type: 0,
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
            is_cliff_redraw: false,
            variant: 0,
            has_ramp: false,
            canonical_ramp: None,
            ground_walk_blocked: false,
            terrain_object_blocks: false,
            overlay_blocks: false,
            zone_type: 0,
            base_ground_walk_blocked: false,
            base_build_blocked: false,
            build_blocked: false,
            has_bridge_deck: false,
            bridge_walkable: false,
            bridge_transition: false,
            bridge_deck_level: 0,
            bridge_layer: None,
            radar_left: [0; 3],
            radar_right: [0; 3],
            accepts_smudge: true,
            has_damaged_data: false,
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
        // dz = 10 levels = 1040 lep, dx=dy=0. Range 4 cells (=1024 lep) → false.
        let rules = rules_with_weapon("Damage=1\nROF=20\nRange=4\nWarhead=WH", "", "");
        let weapon = rules.weapon("GUN").expect("weapon");
        let attacker = ground_attacker(5, 5, 0, "ATKR");
        let target = ground_target(5, 5, 10, "TGT");
        let mut entities = EntityStore::new();
        entities.insert(target);
        let interner = test_interner();
        let terrain = flat_terrain(64, 64);

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
        assert!(!r4, "1040 lep z-delta exceeds 1024 lep 2D range");

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
        assert!(r5, "1040 lep z-delta within 1280 lep range");
    }

    // Test 5: LowFlying ground-snap
    #[test]
    fn low_flying_target_z_snapped_to_ground() {
        let rules = rules_with_weapon("Damage=1\nROF=20\nRange=4\nWarhead=WH", "", "");
        let weapon = rules.weapon("GUN").expect("weapon");
        let attacker = ground_attacker(0, 0, 0, "ATKR");
        // Aircraft at altitude 500 lep (low-flying), 4 cells away horizontally.
        let target = aircraft_target(4, 0, 0, 500, "TGT");
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

    // Test 11: Bridge LOS gate
    #[test]
    fn bridge_los_gate_blocks_under_bridge_to_deck() {
        let rules = rules_with_weapon("Damage=1\nROF=20\nRange=2\nWarhead=WH", "", "");
        let weapon = rules.weapon("GUN").expect("weapon");
        let interner = test_interner();

        // Build a 16x16 grid with a bridge deck on cell (5, 5), ground level 0.
        // bridge_top = 0*104 + 416 = 416 lep.
        let mut cells: Vec<ResolvedTerrainCell> = (0..16)
            .flat_map(|ry| (0..16).map(move |rx| default_cell(rx, ry)))
            .collect();
        let idx = 5 * 16 + 5;
        cells[idx].has_bridge_deck = true;
        cells[idx].bridge_deck_level = 4;
        let bridge_terrain = ResolvedTerrainGrid::from_cells(16, 16, cells);

        let attacker = ground_attacker(5, 5, 0, "ATKR");
        let entities = EntityStore::new();

        // Attacker beneath deck (Z=0), target = cell on bridge cell (Z snaps to 416).
        let under = (5i64 * 256 + 128, 5i64 * 256 + 128, 0i64);
        let blocked = compute_in_range(
            &attacker,
            under,
            &TargetKind::Cell(5, 5),
            weapon,
            &rules,
            &interner,
            &entities,
            &bridge_terrain,
        );
        assert!(
            !blocked,
            "under-bridge attacker firing up at deck must be blocked"
        );

        // Attacker on the deck (Z=416). Same cell. Gate should NOT trigger.
        let on_deck = (5i64 * 256 + 128, 5i64 * 256 + 128, 416i64);
        let allowed = compute_in_range(
            &attacker,
            on_deck,
            &TargetKind::Cell(5, 5),
            weapon,
            &rules,
            &interner,
            &entities,
            &bridge_terrain,
        );
        assert!(
            allowed,
            "attacker on the deck targeting the deck should not trip gate"
        );
    }
}
