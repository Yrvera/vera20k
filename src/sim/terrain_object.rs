//! Live map terrain object state and lifecycle helpers.
//!
//! This module owns deterministic sim state for `TerrainClass`-style objects
//! loaded from map `[Terrain]`. TIBTRE ore spawners are a derived index of this
//! live state, not the lifecycle owner.

use std::collections::BTreeSet;

use crate::map::resolved_terrain::{ResolvedTerrainGrid, recalc_zone_type};
use crate::rules::ruleset::RuleSet;
use crate::rules::terrain_object_type::TerrainObjectType;
use crate::rules::warhead_type::WarheadType;
use crate::sim::combat::armor_index;
use crate::sim::intern::{InternedId, StringInterner};
use crate::sim::occupancy::RawCellOccupationGrid;
use crate::sim::production::ProductionState;

const TERRAIN_LIMBO_CLEAR_BIT: u8 = 0x40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum TerrainObjectLifecycle {
    Live,
    Limbo,
    Destroyed,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TerrainObjectState {
    pub stable_id: u64,
    pub type_ref: InternedId,
    pub rx: u16,
    pub ry: u16,
    pub health: i32,
    pub max_health: i32,
    pub occupation_bits: u8,
    pub lifecycle: TerrainObjectLifecycle,
}

impl TerrainObjectState {
    pub fn new(
        stable_id: u64,
        type_ref: InternedId,
        rx: u16,
        ry: u16,
        terrain_type: &TerrainObjectType,
        snow_theater: bool,
    ) -> Self {
        Self {
            stable_id,
            type_ref,
            rx,
            ry,
            health: terrain_type.strength,
            max_health: terrain_type.strength,
            occupation_bits: occupation_bits_for(terrain_type, snow_theater),
            lifecycle: TerrainObjectLifecycle::Live,
        }
    }

    pub fn cell(&self) -> (u16, u16) {
        (self.rx, self.ry)
    }

    pub fn is_live(&self) -> bool {
        self.lifecycle == TerrainObjectLifecycle::Live
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainDamageResult {
    Ignored,
    Damaged { remaining: i32 },
    Destroyed,
}

pub fn occupation_bits_for(terrain_type: &TerrainObjectType, snow_theater: bool) -> u8 {
    (if snow_theater {
        terrain_type.snow_occupation_bits
    } else {
        terrain_type.temperate_occupation_bits
    }) & 0x07
}

pub(crate) fn terrain_raw_occupation_mask(source_mask: u8) -> u8 {
    (source_mask & 0x07) << 2
}

pub(crate) fn mark_terrain_raw_occupation(
    raw_occupation: &mut RawCellOccupationGrid,
    cell: (u16, u16),
    source_mask: u8,
) {
    raw_occupation.mark_ground(
        cell.0,
        cell.1,
        terrain_raw_occupation_mask(source_mask),
    );
}

fn unmark_terrain_raw_occupation(
    raw_occupation: &mut RawCellOccupationGrid,
    cell: (u16, u16),
    source_mask: u8,
) {
    raw_occupation.clear_ground(
        cell.0,
        cell.1,
        terrain_raw_occupation_mask(source_mask),
    );
}

pub fn sync_spawner_indices_from_live_terrain(production: &mut ProductionState) {
    let live_spawning_cells: BTreeSet<(u16, u16)> = production
        .terrain_objects
        .values()
        .filter(|terrain| terrain.is_live())
        .filter_map(|terrain| {
            let cell = terrain.cell();
            production
                .tiberium_spawning_terrain_cells
                .contains(&cell)
                .then_some(cell)
        })
        .collect();
    production
        .terrain_spawners
        .retain(|cell, _| live_spawning_cells.contains(cell));
    production.tiberium_spawning_terrain_cells = live_spawning_cells;
}

pub fn mark_terrain_occupation(
    production: &mut ProductionState,
    terrain: &TerrainObjectState,
    resolved_terrain: Option<&mut ResolvedTerrainGrid>,
) {
    let cell = terrain.cell();
    if terrain.occupation_bits != 0 {
        production
            .terrain_occupation_bits
            .insert(cell, terrain.occupation_bits);
    } else {
        production.terrain_occupation_bits.remove(&cell);
    }
    if let Some(grid) = resolved_terrain {
        set_resolved_terrain_object_occupation(grid, cell, Some(terrain.occupation_bits));
    }
}

pub fn unmark_terrain_occupation(
    production: &mut ProductionState,
    terrain: &TerrainObjectState,
    resolved_terrain: Option<&mut ResolvedTerrainGrid>,
) {
    let cell = terrain.cell();
    production.terrain_occupation_bits.remove(&cell);
    if let Some(grid) = resolved_terrain {
        set_resolved_terrain_object_occupation(grid, cell, None);
    }
}

pub(crate) fn limbo_terrain_object_at_cell(
    production: &mut ProductionState,
    cell: (u16, u16),
    raw_occupation: &mut RawCellOccupationGrid,
    resolved_terrain: Option<&mut ResolvedTerrainGrid>,
) -> bool {
    let Some(&stable_id) = production.terrain_object_cells.get(&cell) else {
        return false;
    };
    let Some(snapshot) = production.terrain_objects.get(&stable_id).cloned() else {
        return false;
    };
    if !snapshot.is_live() {
        return false;
    }

    let source_cell = snapshot.cell();
    raw_occupation.clear_ground(
        source_cell.0,
        source_cell.1,
        TERRAIN_LIMBO_CLEAR_BIT,
    );
    production.terrain_object_cells.remove(&cell);
    unmark_terrain_raw_occupation(raw_occupation, source_cell, snapshot.occupation_bits);
    unmark_terrain_occupation(production, &snapshot, resolved_terrain);
    if let Some(terrain) = production.terrain_objects.get_mut(&stable_id) {
        terrain.lifecycle = TerrainObjectLifecycle::Limbo;
    }
    production.terrain_spawners.remove(&source_cell);
    production
        .tiberium_spawning_terrain_cells
        .remove(&source_cell);
    true
}

pub(crate) fn damage_terrain_object_at_cell(
    production: &mut ProductionState,
    raw_occupation: &mut RawCellOccupationGrid,
    rules: &RuleSet,
    interner: &StringInterner,
    cell: (u16, u16),
    base_damage: i32,
    warhead: &WarheadType,
    resolved_terrain: Option<&mut ResolvedTerrainGrid>,
) -> TerrainDamageResult {
    if base_damage <= 0 || !warhead.wood {
        return TerrainDamageResult::Ignored;
    }
    let Some(&stable_id) = production.terrain_object_cells.get(&cell) else {
        return TerrainDamageResult::Ignored;
    };
    let Some(terrain) = production.terrain_objects.get(&stable_id) else {
        return TerrainDamageResult::Ignored;
    };
    if !terrain.is_live() {
        return TerrainDamageResult::Ignored;
    }
    let type_name = interner.resolve(terrain.type_ref);
    let Some(terrain_type) = rules.terrain_object_type_case_insensitive(type_name) else {
        return TerrainDamageResult::Ignored;
    };
    if terrain_type.immune {
        return TerrainDamageResult::Ignored;
    }

    let armor_idx = armor_index(&terrain_type.armor);
    let verses_pct = warhead.verses.get(armor_idx).copied().unwrap_or(100) as i32;
    let damage = base_damage.saturating_mul(verses_pct) / 100;
    if damage <= 0 {
        return TerrainDamageResult::Ignored;
    }

    let (remaining, destroyed) =
        if let Some(terrain) = production.terrain_objects.get_mut(&stable_id) {
            terrain.health = terrain.health.saturating_sub(damage);
            (terrain.health, terrain.health <= 0)
        } else {
            return TerrainDamageResult::Ignored;
        };

    if destroyed {
        let did_limbo =
            limbo_terrain_object_at_cell(production, cell, raw_occupation, resolved_terrain);
        if did_limbo {
            if let Some(terrain) = production.terrain_objects.get_mut(&stable_id) {
                terrain.lifecycle = TerrainObjectLifecycle::Destroyed;
            }
        }
        TerrainDamageResult::Destroyed
    } else {
        TerrainDamageResult::Damaged { remaining }
    }
}

fn set_resolved_terrain_object_occupation(
    grid: &mut ResolvedTerrainGrid,
    cell: (u16, u16),
    occupation: Option<u8>,
) {
    let Some(terrain_cell) = grid.cell_mut(cell.0, cell.1) else {
        return;
    };
    let blocked = occupation.is_some_and(|occupation| occupation != 0);
    terrain_cell.terrain_object_occupation = occupation;
    terrain_cell.terrain_object_blocks = blocked;
    terrain_cell.ground_walk_blocked =
        terrain_cell.base_ground_walk_blocked || terrain_cell.overlay_blocks || blocked;
    terrain_cell.build_blocked = terrain_cell.base_build_blocked
        || terrain_cell.overlay_blocks
        || terrain_cell.has_bridge_deck
        || blocked;
    terrain_cell.zone_type = recalc_zone_type(
        terrain_cell.outside_playfield,
        terrain_cell.overlay_zone_type,
        terrain_cell.land_type,
        terrain_cell.speed_costs.wheel,
        terrain_cell.terrain_object_occupation,
    );
}

pub fn next_terrain_object_id(production: &mut ProductionState) -> u64 {
    let id = production.next_terrain_object_id;
    production.next_terrain_object_id = production.next_terrain_object_id.saturating_add(1);
    id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::overlay::TerrainObject;
    use crate::map::resolved_terrain::{ResolvedTerrainCell, zone_class};
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;
    use crate::rules::terrain_rules::{LandType, SpeedCostProfile, TerrainClass};
    use crate::sim::terrain_spawn::seed_terrain_spawners;
    use crate::sim::world::Simulation;
    use std::collections::BTreeMap;

    fn terrain_rules(type_name: &str, wood: bool, type_section: &str) -> RuleSet {
        let wood = if wood { "yes" } else { "no" };
        let ini = IniFile::from_str(&format!(
            "[General]\nTreeStrength=10\n\
              [InfantryTypes]\n\
              [VehicleTypes]\n0=DUMMY\n\
              [AircraftTypes]\n\
              [BuildingTypes]\n\
              [TerrainTypes]\n0={}\n\
              [DUMMY]\nPrimary=Gun\nStrength=100\nArmor=heavy\n\
              [Gun]\nDamage=10\nWarhead=WH\n\
              [WH]\nWood={}\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n\
              [{}]\n{}",
            type_name, wood, type_name, type_section
        ));
        RuleSet::from_ini(&ini).expect("rules")
    }

    fn rules(tib_section: &str) -> RuleSet {
        terrain_rules(
            "TIBTRE01",
            true,
            &format!(
                "SpawnsTiberium=yes\nIsAnimated=yes\nAnimationRate=3\nAnimationProbability=1\n{}",
                tib_section
            ),
        )
    }

    fn seed_one(sim: &mut Simulation, rules: &RuleSet) {
        seed_one_at(sim, rules, "TIBTRE01", (10, 11));
    }

    fn seed_one_at(
        sim: &mut Simulation,
        rules: &RuleSet,
        type_name: &str,
        cell: (u16, u16),
    ) {
        seed_terrain_spawners(
            sim,
            &[TerrainObject {
                rx: cell.0,
                ry: cell.1,
                name: type_name.to_string(),
            }],
            rules,
            &BTreeMap::new(),
            &BTreeMap::new(),
            false,
        );
    }

    fn resolved_clear_grid() -> ResolvedTerrainGrid {
        let mut speed_costs = SpeedCostProfile::default();
        speed_costs.wheel = Some(100);
        ResolvedTerrainGrid::from_cells(
            1,
            1,
            vec![ResolvedTerrainCell {
                rx: 0,
                ry: 0,
                source_tile_index: 0,
                source_sub_tile: 0,
                final_tile_index: 0,
                final_sub_tile: 0,
                is_wood_bridge_repair_tile: false,
                level: 0,
                filled_clear: false,
                tileset_index: None,
                land_type: LandType::Clear.as_index(),
                yr_cell_land_type: LandType::Clear.as_index(),
                slope_type: 0,
                template_height: 0,
                render_offset_x: 0,
                render_offset_y: 0,
                terrain_class: TerrainClass::Clear,
                speed_costs,
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
                terrain_object_occupation: None,
                overlay_blocks: false,
                overlay_zone_type: None,
                outside_playfield: false,
                zone_type: zone_class::GROUND,
                base_ground_walk_blocked: false,
                base_build_blocked: false,
                base_land_type: LandType::Clear.as_index(),
                base_yr_cell_land_type: LandType::Clear.as_index(),
                base_terrain_class: TerrainClass::Clear,
                base_speed_costs: speed_costs,
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
                has_damaged_data: false,
                bridgehead_anchor_class_at_load: None,
            }],
        )
    }

    #[test]
    fn gsi_04_04_runtime_terrain_occupation_uses_shared_zone_writer() {
        let rules = rules("TemperateOccupationBits=7\n");
        let terrain_type = rules
            .terrain_object_type_case_insensitive("TIBTRE01")
            .expect("terrain type");
        let mut interner = StringInterner::default();
        let type_ref = interner.intern("TIBTRE01");
        let mut terrain = TerrainObjectState::new(1, type_ref, 0, 0, terrain_type, false);
        let mut production = ProductionState::default();
        let mut grid = resolved_clear_grid();

        mark_terrain_occupation(&mut production, &terrain, Some(&mut grid));
        let cell = grid.cell(0, 0).unwrap();
        assert_eq!(cell.terrain_object_occupation, Some(7));
        assert!(cell.terrain_object_blocks);
        assert_eq!(cell.zone_type, zone_class::WALL);

        unmark_terrain_occupation(&mut production, &terrain, Some(&mut grid));
        let cell = grid.cell(0, 0).unwrap();
        assert_eq!(cell.terrain_object_occupation, None);
        assert!(!cell.terrain_object_blocks);
        assert_eq!(cell.zone_type, zone_class::GROUND);
        assert_eq!(cell.land_type, LandType::Clear.as_index());

        terrain.occupation_bits = 4;
        mark_terrain_occupation(&mut production, &terrain, Some(&mut grid));
        assert_eq!(grid.cell(0, 0).unwrap().zone_type, zone_class::BUILDING);
        unmark_terrain_occupation(&mut production, &terrain, Some(&mut grid));

        terrain.occupation_bits = 0;
        mark_terrain_occupation(&mut production, &terrain, Some(&mut grid));
        let cell = grid.cell(0, 0).unwrap();
        assert_eq!(cell.terrain_object_occupation, Some(0));
        assert!(!cell.terrain_object_blocks);
        assert_eq!(cell.zone_type, zone_class::BUILDING);

        let cell = grid.cell_mut(0, 0).unwrap();
        cell.overlay_zone_type = Some(zone_class::GROUND);
        terrain.occupation_bits = 7;
        mark_terrain_occupation(&mut production, &terrain, Some(&mut grid));
        assert_eq!(
            grid.cell(0, 0).unwrap().zone_type,
            zone_class::GROUND,
            "terminal rubble result outranks terrain occupation"
        );
    }

    #[test]
    fn gsi_04_10_stock_immune_tibtree_ignores_wood_damage_and_keeps_spawner() {
        let rules = rules("Immune=yes\n");
        let mut sim = Simulation::new();
        seed_one(&mut sim, &rules);
        let warhead = rules.warhead("WH").expect("warhead");

        let result = damage_terrain_object_at_cell(
            &mut sim.production,
            &mut sim.substrate.raw_cell_occupation,
            &rules,
            &sim.interner,
            (10, 11),
            10,
            warhead,
            None,
        );

        assert_eq!(result, TerrainDamageResult::Ignored);
        assert!(sim.production.terrain_spawners.contains_key(&(10, 11)));
        let terrain = sim.production.terrain_objects.values().next().unwrap();
        assert_eq!(terrain.lifecycle, TerrainObjectLifecycle::Live);
        assert_eq!(terrain.health, 10);
    }

    #[test]
    fn gsi_04_10_nonimmune_tibtree_death_limbos_object_and_removes_spawner_indices() {
        let rules = rules("Immune=no\n");
        let mut sim = Simulation::new();
        seed_one(&mut sim, &rules);
        let stable_id = *sim.production.terrain_object_cells.get(&(10, 11)).unwrap();
        let warhead = rules.warhead("WH").expect("warhead");

        let result = damage_terrain_object_at_cell(
            &mut sim.production,
            &mut sim.substrate.raw_cell_occupation,
            &rules,
            &sim.interner,
            (10, 11),
            10,
            warhead,
            None,
        );

        assert_eq!(result, TerrainDamageResult::Destroyed);
        assert!(!sim.production.terrain_object_cells.contains_key(&(10, 11)));
        assert!(!sim.production.terrain_spawners.contains_key(&(10, 11)));
        assert!(
            !sim.production
                .tiberium_spawning_terrain_cells
                .contains(&(10, 11))
        );
        assert!(
            !sim.production
                .terrain_occupation_bits
                .contains_key(&(10, 11))
        );
        assert_eq!(
            sim.production.terrain_objects[&stable_id].lifecycle,
            TerrainObjectLifecycle::Destroyed
        );
    }

    #[test]
    fn gsi_04_10_non_wood_and_immune_damage_do_not_mutate_ordinary_tree() {
        for (wood, type_section) in [(false, ""), (true, "Immune=yes\n")] {
            let rules = terrain_rules("TREE01", wood, type_section);
            let mut sim = Simulation::new();
            seed_one_at(&mut sim, &rules, "TREE01", (0, 0));
            let stable_id = sim.production.terrain_object_cells[&(0, 0)];
            let terrain = sim.production.terrain_objects[&stable_id].clone();
            let mut grid = resolved_clear_grid();
            mark_terrain_occupation(&mut sim.production, &terrain, Some(&mut grid));
            let before = sim.production.terrain_objects[&stable_id].clone();
            let warhead = rules.warhead("WH").expect("warhead");

            let result = damage_terrain_object_at_cell(
                &mut sim.production,
                &mut sim.substrate.raw_cell_occupation,
                &rules,
                &sim.interner,
                (0, 0),
                5,
                warhead,
                Some(&mut grid),
            );

            assert_eq!(result, TerrainDamageResult::Ignored);
            assert_eq!(sim.production.terrain_objects[&stable_id], before);
            assert_eq!(sim.production.terrain_object_cells[&(0, 0)], stable_id);
            assert_eq!(sim.production.terrain_occupation_bits[&(0, 0)], 7);
            let cell = grid.cell(0, 0).unwrap();
            assert_eq!(cell.terrain_object_occupation, Some(7));
            assert!(cell.terrain_object_blocks);
        }
    }

    #[test]
    fn gsi_04_10_wood_sublethal_damage_changes_only_health() {
        let rules = terrain_rules("TREE01", true, "");
        let mut sim = Simulation::new();
        seed_one_at(&mut sim, &rules, "TREE01", (0, 0));
        let stable_id = sim.production.terrain_object_cells[&(0, 0)];
        let terrain = sim.production.terrain_objects[&stable_id].clone();
        let mut grid = resolved_clear_grid();
        mark_terrain_occupation(&mut sim.production, &terrain, Some(&mut grid));
        let mut expected = sim.production.terrain_objects[&stable_id].clone();
        expected.health = 6;
        let warhead = rules.warhead("WH").expect("warhead");

        let result = damage_terrain_object_at_cell(
            &mut sim.production,
            &mut sim.substrate.raw_cell_occupation,
            &rules,
            &sim.interner,
            (0, 0),
            4,
            warhead,
            Some(&mut grid),
        );

        assert_eq!(result, TerrainDamageResult::Damaged { remaining: 6 });
        assert_eq!(sim.production.terrain_objects[&stable_id], expected);
        assert_eq!(sim.production.terrain_object_cells[&(0, 0)], stable_id);
        assert_eq!(sim.production.terrain_occupation_bits[&(0, 0)], 7);
        let cell = grid.cell(0, 0).unwrap();
        assert_eq!(cell.terrain_object_occupation, Some(7));
        assert!(cell.terrain_object_blocks);
    }

    #[test]
    fn gsi_04_10_lethal_ordinary_tree_uninits_and_clears_spatial_authority_same_call() {
        let rules = terrain_rules("TREE01", true, "");
        let mut sim = Simulation::new();
        seed_one_at(&mut sim, &rules, "TREE01", (0, 0));
        let stable_id = sim.production.terrain_object_cells[&(0, 0)];
        let terrain = sim.production.terrain_objects[&stable_id].clone();
        let mut grid = resolved_clear_grid();
        mark_terrain_occupation(&mut sim.production, &terrain, Some(&mut grid));
        assert!(grid.cell(0, 0).unwrap().terrain_object_blocks);
        let warhead = rules.warhead("WH").expect("warhead");

        let result = damage_terrain_object_at_cell(
            &mut sim.production,
            &mut sim.substrate.raw_cell_occupation,
            &rules,
            &sim.interner,
            (0, 0),
            10,
            warhead,
            Some(&mut grid),
        );

        assert_eq!(result, TerrainDamageResult::Destroyed);
        assert!(!sim.production.terrain_object_cells.contains_key(&(0, 0)));
        assert!(!sim.production.terrain_occupation_bits.contains_key(&(0, 0)));
        assert!(!sim.production.terrain_spawners.contains_key(&(0, 0)));
        assert!(
            !sim.production
                .tiberium_spawning_terrain_cells
                .contains(&(0, 0))
        );
        assert_eq!(
            sim.production.terrain_objects[&stable_id].lifecycle,
            TerrainObjectLifecycle::Destroyed
        );
        let cell = grid.cell(0, 0).unwrap();
        assert_eq!(cell.terrain_object_occupation, None);
        assert!(!cell.terrain_object_blocks);
        assert!(!cell.ground_walk_blocked);
        assert!(!cell.build_blocked);
        assert_eq!(cell.zone_type, zone_class::GROUND);
    }

    #[test]
    fn gsi_04_12_terrain_raw_occupation_live_limbo_clears_native_bits_only() {
        let rules = terrain_rules("TREE01", true, "TemperateOccupationBits=7\n");
        let mut sim = Simulation::new();
        sim.substrate.raw_cell_occupation.mark_ground(0, 0, 0xE0);
        sim.substrate.raw_cell_occupation.mark_deck(0, 0, 0xA5);
        seed_one_at(&mut sim, &rules, "TREE01", (0, 0));
        let stable_id = sim.production.terrain_object_cells[&(0, 0)];

        assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(0, 0), 0xFC);
        let did_limbo = limbo_terrain_object_at_cell(
            &mut sim.production,
            (0, 0),
            &mut sim.substrate.raw_cell_occupation,
            None,
        );

        assert!(did_limbo);
        assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(0, 0), 0xA0);
        assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(0, 0), 0xA5);
        assert!(!sim.production.terrain_object_cells.contains_key(&(0, 0)));
        assert!(!sim.production.terrain_occupation_bits.contains_key(&(0, 0)));
        assert_eq!(
            sim.production.terrain_objects[&stable_id].lifecycle,
            TerrainObjectLifecycle::Limbo
        );
    }

    #[test]
    fn gsi_04_12_terrain_raw_occupation_damage_preserves_until_lethal_limbo() {
        let rules = terrain_rules("TREE01", true, "TemperateOccupationBits=7\n");
        let mut sim = Simulation::new();
        sim.substrate.raw_cell_occupation.mark_ground(0, 0, 0xE0);
        sim.substrate.raw_cell_occupation.mark_deck(0, 0, 0x5A);
        seed_one_at(&mut sim, &rules, "TREE01", (0, 0));
        let stable_id = sim.production.terrain_object_cells[&(0, 0)];
        let warhead = rules.warhead("WH").expect("warhead");

        let ignored = damage_terrain_object_at_cell(
            &mut sim.production,
            &mut sim.substrate.raw_cell_occupation,
            &rules,
            &sim.interner,
            (0, 0),
            0,
            warhead,
            None,
        );
        assert_eq!(ignored, TerrainDamageResult::Ignored);
        assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(0, 0), 0xFC);
        assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(0, 0), 0x5A);

        let damaged = damage_terrain_object_at_cell(
            &mut sim.production,
            &mut sim.substrate.raw_cell_occupation,
            &rules,
            &sim.interner,
            (0, 0),
            4,
            warhead,
            None,
        );
        assert_eq!(damaged, TerrainDamageResult::Damaged { remaining: 6 });
        assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(0, 0), 0xFC);
        assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(0, 0), 0x5A);
        assert_eq!(sim.production.terrain_occupation_bits[&(0, 0)], 7);

        let destroyed = damage_terrain_object_at_cell(
            &mut sim.production,
            &mut sim.substrate.raw_cell_occupation,
            &rules,
            &sim.interner,
            (0, 0),
            6,
            warhead,
            None,
        );
        assert_eq!(destroyed, TerrainDamageResult::Destroyed);
        assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(0, 0), 0xA0);
        assert_eq!(sim.substrate.raw_cell_occupation.deck_bits(0, 0), 0x5A);
        assert_eq!(
            sim.production.terrain_objects[&stable_id].lifecycle,
            TerrainObjectLifecycle::Destroyed
        );
    }
}
