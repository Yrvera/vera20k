//! Area-of-effect (AoE) damage logic for warheads with CellSpread > 0.
//!
//! When a warhead detonates with CellSpread > 0, it damages all entities
//! within the blast radius. Damage falls off linearly from 100% at the
//! epicenter to `PercentAtMax` at the edge of the radius.
//!
//! ## Damage formula
//! ```text
//! damage_at_distance(d) = base_damage * verses[armor] * lerp(1.0, percent_at_max, d / cell_spread)
//! ```
//!
//! ## Dependency rules
//! - Part of sim/ — depends on rules/, map terrain, and sim occupancy/components.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeSet;

use super::{apply_prone_damage_modifier, armor_index, cell_spread, lepton_distance_sq_raw};
use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::ruleset::RuleSet;
use crate::rules::warhead_type::WarheadType;
use crate::sim::entity_store::EntityStore;
use crate::sim::infantry;
use crate::sim::intern::StringInterner;
use crate::sim::map::bridge_topology::{BRIDGE_DECK_HEIGHT_LEVELS, CellBridgeView, ListLayer};
use crate::sim::movement::locomotor::MovementLayer;
use crate::sim::occupancy::OccupancyGrid;
use crate::util::fixed_math::{SIM_ZERO, SimFixed, isqrt_i64};
use crate::util::lepton::CELL_CENTER_LEPTON;

// GATE A1/A2 CUTOVER (authoritative): the bridge-AoE deck height is now the single
// gamemd-verified value sourced from `bridge_topology` — the full deck offset is
// `2 × per_level` (208 leptons = 2 Level units), so the half-deck term used by the
// object-layer selector is `2 / 2 = 1`, NOT the prior ad-hoc `4`/`2`. The layer
// boundary math lives in `CellBridgeView::aoe_object_layer` (strict `>` against
// `ground_z + half_deck`); this module routes through it so there is ONE source of
// truth for the deck height. This replaces the old `BRIDGE_AOE_SELECTOR_HEIGHT_LEVELS
// = 4` const and its per-cell `(deck_level - level).max(4)` floor.
// (Source: GATE_BRIDGE_DECK_HEIGHT_RESOLUTION_GHIDRA_REPORT.md §3/§5;
//  GATE_BRIDGE_ONBRIDGE_OCCUPANCY_RESOLUTION_GHIDRA_REPORT.md §a.)

/// Optional map context for gamemd bridge object-list selection.
#[derive(Clone, Copy, Default)]
pub(crate) struct AoELayerContext<'a> {
    pub occupancy: Option<&'a OccupancyGrid>,
    pub terrain: Option<&'a ResolvedTerrainGrid>,
    pub impact_z: i32,
}

/// Build the caller-owned impact Z used by bridge-aware AoE call sites.
///
/// Generic cell-center helpers stay ground-only; verified superweapon callers
/// add the structural-bridge deck height before entering Apply_area_damage.
pub(crate) fn bridge_adjusted_impact_z(
    terrain: Option<&ResolvedTerrainGrid>,
    impact_rx: u16,
    impact_ry: u16,
) -> i32 {
    let Some(cell) = terrain.and_then(|terrain| terrain.cell(impact_rx, impact_ry)) else {
        return 0;
    };

    let mut impact_z = cell.level as i32;
    if cell.bridge_facts.has_structural_bridge() {
        // Authoritative deck offset = full deck height (2 levels), not a per-cell
        // span. Same const the layer selector below compares against, so the
        // synthesized impact Z and the layer threshold stay consistent.
        impact_z += BRIDGE_DECK_HEIGHT_LEVELS;
    }
    impact_z
}

/// Apply area-of-effect damage from a warhead detonation at a specific cell.
///
/// Returns a list of (stable_id, damage) pairs for all entities within the blast
/// radius. Friendly fire IS applied — CellSpread does not discriminate by owner,
/// matching RA2 behavior (e.g., V3 rockets can damage your own units).
///
/// `base_damage` is the weapon's raw damage value (before Verses scaling).
pub(crate) fn apply_aoe_damage(
    entities: &EntityStore,
    impact_rx: u16,
    impact_ry: u16,
    base_damage: i32,
    warhead: &WarheadType,
    rules: &RuleSet,
    interner: &StringInterner,
    _attacker_owner: &str,
    layer_context: AoELayerContext<'_>,
) -> Vec<(u64, u16)> {
    let cell_spread: SimFixed = warhead.cell_spread;
    if cell_spread <= SIM_ZERO {
        return Vec::new();
    }

    // Pre-compute squared radius in lepton space (i64) for quick rejection.
    let spread_leptons: i64 = cell_spread.to_num::<i64>() * 256;
    let spread_sq: i64 = spread_leptons * spread_leptons;
    let mut damage_list: Vec<(u64, u16)> = Vec::new();

    if let (Some(occupancy), Some(terrain)) = (layer_context.occupancy, layer_context.terrain) {
        let selected_layer =
            select_object_damage_layer(terrain, impact_rx, impact_ry, layer_context.impact_z);
        let mut seen: BTreeSet<u64> = BTreeSet::new();
        let spread_radius = cell_spread.to_num::<u32>();

        for &(dx, dy) in cell_spread::cells_in_spread(spread_radius) {
            let Some(rx) = offset_cell_coord(impact_rx, dx) else {
                continue;
            };
            let Some(ry) = offset_cell_coord(impact_ry, dy) else {
                continue;
            };
            let Some(cell_occ) = occupancy.get(rx, ry) else {
                continue;
            };
            for occupant in cell_occ.iter_layer(selected_layer) {
                if seen.insert(occupant.entity_id) {
                    push_entity_aoe_damage(
                        &mut damage_list,
                        entities,
                        occupant.entity_id,
                        impact_rx,
                        impact_ry,
                        spread_sq,
                        cell_spread,
                        base_damage,
                        warhead,
                        rules,
                        interner,
                    );
                }
            }
        }

        // The CellClass ground/bridge lists do not cover airborne objects.
        // Keep prior behavior for non-ground/non-bridge entities until the
        // separate airborne splash path is fully ported.
        for entity in entities.values() {
            if entity.occupancy_list_layer().is_none() {
                push_entity_aoe_damage(
                    &mut damage_list,
                    entities,
                    entity.stable_id,
                    impact_rx,
                    impact_ry,
                    spread_sq,
                    cell_spread,
                    base_damage,
                    warhead,
                    rules,
                    interner,
                );
            }
        }

        return damage_list;
    }

    for entity in entities.values() {
        if entity.health.current == 0 {
            continue;
        }

        // Impact point detonates at cell center (sub = 128,128).
        let dist_sq_leptons: i64 = lepton_distance_sq_raw(
            impact_rx,
            impact_ry,
            CELL_CENTER_LEPTON,
            CELL_CENTER_LEPTON,
            entity.position.rx,
            entity.position.ry,
            entity.position.sub_x,
            entity.position.sub_y,
        );

        // Quick reject in lepton space.
        if dist_sq_leptons > spread_sq {
            continue;
        }

        // Convert lepton distance to cell distance for falloff formula.
        // sqrt(dist_sq_leptons) / 256 = distance in cells.
        // Uses integer sqrt to avoid platform-dependent f64 rounding.
        let dist_leptons: i64 = isqrt_i64(dist_sq_leptons);
        let distance: SimFixed = SimFixed::from_num(dist_leptons / 256);
        if distance > cell_spread {
            continue;
        }

        // Look up target armor for Verses scaling.
        let armor_str: &str = rules
            .object(interner.resolve(entity.type_ref))
            .map(|o| o.armor.as_str())
            .unwrap_or("none");
        let idx: usize = armor_index(armor_str);
        let verses_pct: u8 = warhead.verses.get(idx).copied().unwrap_or(100);

        let raw_damage: u16 = aoe_damage_at_distance(
            base_damage,
            distance,
            cell_spread,
            warhead.percent_at_max,
            verses_pct,
        );
        let prone_infantry =
            entity.category == EntityCategory::Infantry && infantry::is_prone_for_damage(entity);
        let dmg: u16 = apply_prone_damage_modifier(prone_infantry, warhead, raw_damage as i32);

        if dmg > 0 {
            damage_list.push((entity.stable_id, dmg));
        }
    }

    damage_list
}

fn select_object_damage_layer(
    terrain: &ResolvedTerrainGrid,
    impact_rx: u16,
    impact_ry: u16,
    impact_z: i32,
) -> MovementLayer {
    let Some(cell) = terrain.cell(impact_rx, impact_ry) else {
        return MovementLayer::Ground;
    };

    // Authoritative: delegate the ground-vs-deck choice to the single verified
    // selector in bridge_topology. It applies the strict-`>` half-deck boundary
    // (`impact_z > ground_z + DECK/2`, half = 1 level) on the structural-bridge gate.
    // `ground_z` is `cell.level` in the same Level domain as `impact_z` (P0b: the
    // generic cell-center callers add the fixed deck offset, never a routed
    // GetGroundHeight, so both operands are cell-Level units here).
    let view = CellBridgeView::from_resolved(cell);
    match view.aoe_object_layer(impact_z, cell.level as i32) {
        ListLayer::Bridge => MovementLayer::Bridge,
        ListLayer::Ground => MovementLayer::Ground,
    }
}

fn offset_cell_coord(origin: u16, delta: i16) -> Option<u16> {
    let value = origin as i32 + delta as i32;
    if (0..=u16::MAX as i32).contains(&value) {
        Some(value as u16)
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn push_entity_aoe_damage(
    damage_list: &mut Vec<(u64, u16)>,
    entities: &EntityStore,
    entity_id: u64,
    impact_rx: u16,
    impact_ry: u16,
    spread_sq: i64,
    cell_spread: SimFixed,
    base_damage: i32,
    warhead: &WarheadType,
    rules: &RuleSet,
    interner: &StringInterner,
) {
    let Some(entity) = entities.get(entity_id) else {
        return;
    };
    if entity.health.current == 0 {
        return;
    }

    // Impact point detonates at cell center (sub = 128,128).
    let dist_sq_leptons: i64 = lepton_distance_sq_raw(
        impact_rx,
        impact_ry,
        CELL_CENTER_LEPTON,
        CELL_CENTER_LEPTON,
        entity.position.rx,
        entity.position.ry,
        entity.position.sub_x,
        entity.position.sub_y,
    );

    // Quick reject in lepton space.
    if dist_sq_leptons > spread_sq {
        return;
    }

    // Convert lepton distance to cell distance for falloff formula.
    // sqrt(dist_sq_leptons) / 256 = distance in cells.
    // Uses integer sqrt to avoid platform-dependent f64 rounding.
    let dist_leptons: i64 = isqrt_i64(dist_sq_leptons);
    let distance: SimFixed = SimFixed::from_num(dist_leptons / 256);
    if distance > cell_spread {
        return;
    }

    // Look up target armor for Verses scaling.
    let armor_str: &str = rules
        .object(interner.resolve(entity.type_ref))
        .map(|o| o.armor.as_str())
        .unwrap_or("none");
    let idx: usize = armor_index(armor_str);
    let verses_pct: u8 = warhead.verses.get(idx).copied().unwrap_or(100);

    let raw_damage: u16 = aoe_damage_at_distance(
        base_damage,
        distance,
        cell_spread,
        warhead.percent_at_max,
        verses_pct,
    );
    let prone_infantry =
        entity.category == EntityCategory::Infantry && infantry::is_prone_for_damage(entity);
    let dmg: u16 = apply_prone_damage_modifier(prone_infantry, warhead, raw_damage as i32);

    if dmg > 0 {
        damage_list.push((entity.stable_id, dmg));
    }
}

/// Compute distance-scaled AoE damage using integer/fixed-point math.
///
/// At distance 0 (epicenter): full `base_damage * verses_pct / 100`.
/// At distance == cell_spread (edge): `base_damage * verses_pct * percent_at_max_pct / 10000`.
/// Linear interpolation between those extremes.
fn aoe_damage_at_distance(
    base_damage: i32,
    distance: SimFixed,
    cell_spread: SimFixed,
    percent_at_max_pct: u8,
    verses_pct: u8,
) -> u16 {
    // t = distance / cell_spread, clamped [0, 1] — how far from center (SimFixed).
    let t: SimFixed = if cell_spread > SIM_ZERO {
        (distance / cell_spread).clamp(SIM_ZERO, SimFixed::from_num(1))
    } else {
        SIM_ZERO
    };
    // falloff_pct = lerp(100, percent_at_max_pct, t) in integer.
    // = 100 + (percent_at_max_pct - 100) * t
    let pam: i32 = percent_at_max_pct as i32;
    let falloff_fixed: SimFixed = SimFixed::from_num(100) + SimFixed::from_num(pam - 100) * t;
    let falloff_pct: i32 = falloff_fixed.to_num::<i32>();

    // raw = base_damage * verses_pct * falloff_pct / 10000
    // Compute in i64 and clamp to i32 range to prevent silent narrowing overflow.
    let wide = base_damage as i64 * verses_pct as i64 * falloff_pct as i64 / 10000;
    wide.clamp(0, u16::MAX as i64) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::bridge_facts::{BRIDGE_FLAG_STRUCTURAL, BridgeCellFacts};
    use crate::map::resolved_terrain::{ResolvedTerrainCell, ResolvedTerrainGrid};
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;
    use crate::rules::terrain_rules::{SpeedCostProfile, TerrainClass};
    use crate::sim::entity_store::EntityStore;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::intern::{StringInterner, test_intern, test_interner};
    use crate::sim::movement::locomotor::MovementLayer;
    use crate::sim::occupancy::{CellListInsertion, OccupancyGrid};
    use crate::util::fixed_math::sim_from_f32;

    #[test]
    fn test_aoe_damage_at_center() {
        // At distance 0, full damage: 100 * 100 * 100 / 10000 = 100.
        let dmg = aoe_damage_at_distance(100, SIM_ZERO, sim_from_f32(3.0), 25, 100);
        assert_eq!(dmg, 100);
    }

    #[test]
    fn test_aoe_damage_at_edge() {
        // At distance == cell_spread, damage = base * percent_at_max / 100 = 100 * 25 / 100 = 25.
        let dmg = aoe_damage_at_distance(100, sim_from_f32(3.0), sim_from_f32(3.0), 25, 100);
        assert_eq!(dmg, 25);
    }

    #[test]
    fn test_aoe_damage_at_midpoint() {
        // At half distance, falloff_pct = lerp(100, 25, 0.5) = 62.
        // damage = 100 * 100 * 62 / 10000 = 62.
        let dmg = aoe_damage_at_distance(100, sim_from_f32(1.5), sim_from_f32(3.0), 25, 100);
        assert_eq!(dmg, 62);
    }

    #[test]
    fn test_aoe_damage_with_verses() {
        // 50% verses at center: 100 * 50 * 100 / 10000 = 50.
        let dmg = aoe_damage_at_distance(100, SIM_ZERO, sim_from_f32(3.0), 25, 50);
        assert_eq!(dmg, 50);
    }

    #[test]
    fn test_aoe_damage_zero_verses() {
        let dmg = aoe_damage_at_distance(100, SIM_ZERO, sim_from_f32(3.0), 25, 0);
        assert_eq!(dmg, 0);
    }

    #[test]
    fn test_aoe_beyond_radius() {
        // Beyond radius clamped to t=1 → percent_at_max.
        let dmg = aoe_damage_at_distance(100, sim_from_f32(5.0), sim_from_f32(3.0), 25, 100);
        assert_eq!(dmg, 25);
    }

    #[test]
    fn bridge_impact_above_threshold_damages_only_bridge_layer() {
        let (entities, occupancy, terrain, rules, warhead, interner) = bridge_layer_test_fixture();
        let hits = apply_aoe_damage(
            &entities,
            5,
            5,
            100,
            &warhead,
            &rules,
            &interner,
            "Americans",
            AoELayerContext {
                occupancy: Some(&occupancy),
                terrain: Some(&terrain),
                impact_z: 4,
            },
        );

        let hit_ids: Vec<u64> = hits.into_iter().map(|(id, _)| id).collect();
        assert_eq!(hit_ids, vec![2]);
    }

    #[test]
    fn bridge_impact_at_threshold_stays_on_ground_layer() {
        let (entities, occupancy, terrain, rules, warhead, interner) = bridge_layer_test_fixture();
        let hits = apply_aoe_damage(
            &entities,
            5,
            5,
            100,
            &warhead,
            &rules,
            &interner,
            "Americans",
            AoELayerContext {
                occupancy: Some(&occupancy),
                terrain: Some(&terrain),
                // Exactly at the half-deck mid-height (ground_z 0 + DECK/2 = 1).
                // Verified deck = 2 levels → half-deck = 1; strict `>` keeps the
                // boundary on the ground list (was tested at impact_z 2 under the
                // proven-wrong deck = 4 / half = 2 selector).
                impact_z: 1,
            },
        );

        let hit_ids: Vec<u64> = hits.into_iter().map(|(id, _)| id).collect();
        assert_eq!(hit_ids, vec![1]);
    }

    #[test]
    fn bridge_adjusted_impact_z_adds_height_only_at_call_site() {
        let (_, _, terrain, _, _, _) = bridge_layer_test_fixture();
        assert_eq!(bridge_adjusted_impact_z(Some(&terrain), 4, 4), 0);
        // Structural cell (5,5): ground level 0 + verified full deck height
        // (BRIDGE_DECK_HEIGHT_LEVELS = 2). Was 4 under the proven-wrong const.
        assert_eq!(bridge_adjusted_impact_z(Some(&terrain), 5, 5), 2);
        assert_eq!(bridge_adjusted_impact_z(None, 5, 5), 0);
    }

    fn bridge_layer_test_fixture() -> (
        EntityStore,
        OccupancyGrid,
        ResolvedTerrainGrid,
        RuleSet,
        WarheadType,
        StringInterner,
    ) {
        let ini = IniFile::from_str(
            "\
[VehicleTypes]\n0=MTNK\n\n\
[InfantryTypes]\n\n\
[AircraftTypes]\n\n\
[BuildingTypes]\n\n\
[Warheads]\n0=BridgeSplash\n\n\
[MTNK]\nStrength=300\nArmor=heavy\nSpeed=6\n\n\
[BridgeSplash]\nCellSpread=1\nPercentAtMax=1\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("rules parse");
        let warhead =
            WarheadType::from_ini_section("BridgeSplash", ini.section("BridgeSplash").unwrap());

        let ground = GameEntity::test_default(1, "MTNK", "Soviet", 5, 5);
        let mut bridge = GameEntity::test_default(2, "MTNK", "Soviet", 5, 5);
        bridge.on_bridge = true;
        bridge.position.z = 4;

        let mut entities = EntityStore::new();
        entities.insert(ground);
        entities.insert(bridge);

        let mut occupancy = OccupancyGrid::new();
        occupancy.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        occupancy.add(
            5,
            5,
            2,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );

        let mut cells = Vec::new();
        for ry in 0..10 {
            for rx in 0..10 {
                cells.push(test_terrain_cell(rx, ry));
            }
        }
        let idx = 5 * 10 + 5;
        cells[idx].bridge_facts = BridgeCellFacts {
            raw_flags: BRIDGE_FLAG_STRUCTURAL,
            ..BridgeCellFacts::default()
        };
        cells[idx].has_bridge_deck = true;
        cells[idx].bridge_walkable = true;
        cells[idx].bridge_deck_level = 4;
        let terrain = ResolvedTerrainGrid::from_cells(10, 10, cells);

        test_intern("Americans");
        let interner = test_interner();

        (entities, occupancy, terrain, rules, warhead, interner)
    }

    fn test_terrain_cell(rx: u16, ry: u16) -> ResolvedTerrainCell {
        ResolvedTerrainCell {
            rx,
            ry,
            source_tile_index: 0,
            source_sub_tile: 0,
            final_tile_index: 0,
            final_sub_tile: 0,
            is_wood_bridge_repair_tile: false,
            level: 0,
            filled_clear: false,
            tileset_index: Some(0),
            land_type: 0,
            yr_cell_land_type: 0,
            slope_type: 0,
            template_height: 0,
            render_offset_x: 0,
            render_offset_y: 0,
            terrain_class: TerrainClass::Clear,
            speed_costs: SpeedCostProfile::default(),
            is_water: false,
            is_cliff_like: false,
            is_rough: false,
            is_road: false,
            accepts_smudge: false,
            allows_tiberium: false,
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
            radar_left: [0, 0, 0],
            radar_right: [0, 0, 0],
            has_damaged_data: false,
            bridgehead_anchor_class_at_load: None,
        }
    }
}
