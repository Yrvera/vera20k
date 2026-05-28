//! Live map terrain object state and lifecycle helpers.
//!
//! This module owns deterministic sim state for `TerrainClass`-style objects
//! loaded from map `[Terrain]`. TIBTRE ore spawners are a derived index of this
//! live state, not the lifecycle owner.

use std::collections::BTreeSet;

use crate::map::resolved_terrain::{ResolvedTerrainGrid, zone_class};
use crate::rules::ruleset::RuleSet;
use crate::rules::terrain_object_type::TerrainObjectType;
use crate::rules::warhead_type::WarheadType;
use crate::sim::combat::armor_index;
use crate::sim::intern::{InternedId, StringInterner};
use crate::sim::production::ProductionState;

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
    }
    if let Some(grid) = resolved_terrain {
        set_resolved_terrain_object_block(grid, cell, terrain.occupation_bits != 0);
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
        set_resolved_terrain_object_block(grid, cell, false);
    }
}

pub fn limbo_terrain_object_at_cell(
    production: &mut ProductionState,
    cell: (u16, u16),
    resolved_terrain: Option<&mut ResolvedTerrainGrid>,
) -> bool {
    let Some(stable_id) = production.terrain_object_cells.remove(&cell) else {
        return false;
    };
    let Some(snapshot) = production.terrain_objects.get(&stable_id).cloned() else {
        return false;
    };
    if !snapshot.is_live() {
        return false;
    }

    unmark_terrain_occupation(production, &snapshot, resolved_terrain);
    if let Some(terrain) = production.terrain_objects.get_mut(&stable_id) {
        terrain.lifecycle = TerrainObjectLifecycle::Limbo;
    }
    production.terrain_spawners.remove(&cell);
    production.tiberium_spawning_terrain_cells.remove(&cell);
    true
}

pub fn damage_terrain_object_at_cell(
    production: &mut ProductionState,
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
        let did_limbo = limbo_terrain_object_at_cell(production, cell, resolved_terrain);
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

fn set_resolved_terrain_object_block(
    grid: &mut ResolvedTerrainGrid,
    cell: (u16, u16),
    blocked: bool,
) {
    let Some(terrain_cell) = grid.cell_mut(cell.0, cell.1) else {
        return;
    };
    terrain_cell.terrain_object_blocks = blocked;
    terrain_cell.ground_walk_blocked =
        terrain_cell.base_ground_walk_blocked || terrain_cell.overlay_blocks || blocked;
    terrain_cell.build_blocked =
        terrain_cell.base_build_blocked || terrain_cell.overlay_blocks || blocked;
    terrain_cell.zone_type = if terrain_cell.overlay_blocks {
        terrain_cell.zone_type
    } else if blocked {
        zone_class::BUILDING
    } else if terrain_cell.is_water {
        zone_class::WATER
    } else if terrain_cell.base_ground_walk_blocked {
        zone_class::IMPASSABLE
    } else {
        zone_class::GROUND
    };
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
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;
    use crate::sim::terrain_spawn::seed_terrain_spawners;
    use crate::sim::world::Simulation;
    use std::collections::BTreeMap;

    fn rules(tib_section: &str) -> RuleSet {
        let ini = IniFile::from_str(&format!(
            "[General]\nTreeStrength=10\n\
             [InfantryTypes]\n\
             [VehicleTypes]\n0=DUMMY\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [TerrainTypes]\n46=TIBTRE01\n\
             [DUMMY]\nPrimary=Gun\nStrength=100\nArmor=heavy\n\
             [Gun]\nDamage=10\nWarhead=WH\n\
             [WH]\nWood=yes\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,0%,0%\n\
             [TIBTRE01]\nSpawnsTiberium=yes\nIsAnimated=yes\nAnimationRate=3\nAnimationProbability=1\n{}",
            tib_section
        ));
        RuleSet::from_ini(&ini).expect("rules")
    }

    fn seed_one(sim: &mut Simulation, rules: &RuleSet) {
        seed_terrain_spawners(
            sim,
            &[TerrainObject {
                rx: 10,
                ry: 11,
                name: "TIBTRE01".to_string(),
            }],
            rules,
            &BTreeMap::new(),
            &BTreeMap::new(),
            false,
        );
    }

    #[test]
    fn stock_immune_tibtree_ignores_wood_damage_and_keeps_spawner() {
        let rules = rules("Immune=yes\n");
        let mut sim = Simulation::new();
        seed_one(&mut sim, &rules);
        let warhead = rules.warhead("WH").expect("warhead");

        let result = damage_terrain_object_at_cell(
            &mut sim.production,
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
    fn nonimmune_tibtree_death_limbos_object_and_removes_spawner_indices() {
        let rules = rules("Immune=no\n");
        let mut sim = Simulation::new();
        seed_one(&mut sim, &rules);
        let stable_id = *sim.production.terrain_object_cells.get(&(10, 11)).unwrap();
        let warhead = rules.warhead("WH").expect("warhead");

        let result = damage_terrain_object_at_cell(
            &mut sim.production,
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
}
