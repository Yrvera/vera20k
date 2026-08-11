//! Live map terrain object state and lifecycle helpers.
//!
//! This module owns deterministic sim state for `TerrainClass`-style objects
//! loaded from map `[Terrain]`. TIBTRE ore spawners are a derived index of this
//! live state, not the lifecycle owner.

use std::collections::{BTreeMap, BTreeSet};

use crate::map::resolved_terrain::{ResolvedTerrainGrid, recalc_zone_type};
use crate::rules::ruleset::RuleSet;
use crate::rules::terrain_object_type::TerrainObjectType;
use crate::rules::warhead_type::WarheadType;
use crate::sim::combat::{armor_index, damage};
use crate::sim::intern::{InternedId, StringInterner};
use crate::sim::occupancy::RawCellOccupationGrid;
use crate::sim::production::ProductionState;
use crate::sim::terrain_spawn::TerrainSpawnerState;

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

/// Captured lethal Terrain receiver state needed by the synchronous finalize tail.
///
/// The object remains represented with exact-zero health between receive and
/// finalize so a caller can run the verified nested C4 transaction first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerrainLethalDamage {
    pub(crate) stable_id: u64,
    pub(crate) cell: (u16, u16),
    pub(crate) spawns_tiberium: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TerrainAreaReceiveResult {
    Ignored,
    Damaged { remaining: i32 },
    Lethal(TerrainLethalDamage),
}

/// Transient owner for Terrain authority while an ordered area-damage transaction runs.
///
/// None of these fields add snapshot or hash state: the persisted maps/sets and raw
/// occupation grid are moved out of their existing owners, then restored after commit.
/// `navigation_changed_cells` and `finalizing_terrain` exist only for the duration of
/// the outer transaction and are shared by nested receiver commits.
#[derive(Debug, Default)]
pub(crate) struct TerrainAreaState {
    terrain_spawners: BTreeMap<(u16, u16), TerrainSpawnerState>,
    terrain_objects: BTreeMap<u64, TerrainObjectState>,
    terrain_object_cells: BTreeMap<(u16, u16), u64>,
    terrain_occupation_bits: BTreeMap<(u16, u16), u8>,
    tiberium_spawning_terrain_cells: BTreeSet<(u16, u16)>,
    raw_occupation: RawCellOccupationGrid,
    navigation_changed_cells: Vec<(u16, u16)>,
    finalizing_terrain: BTreeSet<u64>,
}

impl TerrainAreaState {
    pub(crate) fn take_from(
        production: &mut ProductionState,
        raw_occupation: &mut RawCellOccupationGrid,
    ) -> Self {
        Self {
            terrain_spawners: std::mem::take(&mut production.terrain_spawners),
            terrain_objects: std::mem::take(&mut production.terrain_objects),
            terrain_object_cells: std::mem::take(&mut production.terrain_object_cells),
            terrain_occupation_bits: std::mem::take(&mut production.terrain_occupation_bits),
            tiberium_spawning_terrain_cells: std::mem::take(
                &mut production.tiberium_spawning_terrain_cells,
            ),
            raw_occupation: std::mem::take(raw_occupation),
            navigation_changed_cells: Vec::new(),
            finalizing_terrain: BTreeSet::new(),
        }
    }

    /// Temporarily expose the exact moved authority through `Simulation` during a
    /// fatal lifecycle callback, then call this again to reclaim its mutations.
    pub(crate) fn swap_authority(
        &mut self,
        production: &mut ProductionState,
        raw_occupation: &mut RawCellOccupationGrid,
    ) {
        std::mem::swap(&mut self.terrain_spawners, &mut production.terrain_spawners);
        std::mem::swap(&mut self.terrain_objects, &mut production.terrain_objects);
        std::mem::swap(
            &mut self.terrain_object_cells,
            &mut production.terrain_object_cells,
        );
        std::mem::swap(
            &mut self.terrain_occupation_bits,
            &mut production.terrain_occupation_bits,
        );
        std::mem::swap(
            &mut self.tiberium_spawning_terrain_cells,
            &mut production.tiberium_spawning_terrain_cells,
        );
        std::mem::swap(&mut self.raw_occupation, raw_occupation);
    }

    /// Restore Terrain authority and return ordered, first-occurrence-only navigation
    /// cells for the World-owned cache publication tail.
    pub(crate) fn restore_into(
        mut self,
        production: &mut ProductionState,
        raw_occupation: &mut RawCellOccupationGrid,
    ) -> Vec<(u16, u16)> {
        debug_assert!(
            self.finalizing_terrain.is_empty(),
            "every lethal Terrain receiver must finalize before authority restoration"
        );
        self.swap_authority(production, raw_occupation);
        self.navigation_changed_cells
    }

    pub(crate) fn terrain_objects(&self) -> &BTreeMap<u64, TerrainObjectState> {
        &self.terrain_objects
    }

    pub(crate) fn terrain_object_cells(&self) -> &BTreeMap<(u16, u16), u64> {
        &self.terrain_object_cells
    }

    pub(crate) fn navigation_changed_cells(&self) -> &[(u16, u16)] {
        &self.navigation_changed_cells
    }

    /// Raw CellClass occupation bytes moved into this receiver transaction.
    /// Inline building-survivor smudges must read them before world authority
    /// is restored, while the destroyed building still occupies its cells.
    pub(crate) fn raw_occupation(&self) -> &RawCellOccupationGrid {
        &self.raw_occupation
    }

    /// Terrain-object cells that remain authoritative tiberium sources while
    /// combat has the production maps moved into this transaction.
    pub(crate) fn tiberium_spawning_terrain_cells(&self) -> &BTreeSet<(u16, u16)> {
        &self.tiberium_spawning_terrain_cells
    }

    pub(crate) fn is_finalizing(&self, stable_id: u64) -> bool {
        self.finalizing_terrain.contains(&stable_id)
    }

    /// Enter the shared Object damage kernel for one captured Terrain receiver.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn receive_area_damage(
        &mut self,
        stable_id: u64,
        cell: (u16, u16),
        raw_damage: i32,
        distance_leptons: i32,
        warhead: &WarheadType,
        rules: &RuleSet,
        interner: &StringInterner,
    ) -> TerrainAreaReceiveResult {
        self.receive_area_damage_with_scenario(
            stable_id,
            cell,
            raw_damage,
            distance_leptons,
            warhead,
            rules,
            interner,
            false,
        )
    }

    /// Scenario-aware shared Object-kernel entry used by production damage
    /// transactions. The compatibility wrapper above keeps isolated fixtures
    /// on the ordinary stock-skirmish false path.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn receive_area_damage_with_scenario(
        &mut self,
        stable_id: u64,
        cell: (u16, u16),
        raw_damage: i32,
        distance_leptons: i32,
        warhead: &WarheadType,
        rules: &RuleSet,
        interner: &StringInterner,
        scenario_no_damage: bool,
    ) -> TerrainAreaReceiveResult {
        if self.finalizing_terrain.contains(&stable_id)
            || self.terrain_object_cells.get(&cell) != Some(&stable_id)
        {
            return TerrainAreaReceiveResult::Ignored;
        }

        let Some(snapshot) = self.terrain_objects.get(&stable_id) else {
            return TerrainAreaReceiveResult::Ignored;
        };
        if !snapshot.is_live() || snapshot.cell() != cell {
            return TerrainAreaReceiveResult::Ignored;
        }
        let Some(terrain_type) =
            rules.terrain_object_type_case_insensitive(interner.resolve(snapshot.type_ref))
        else {
            return TerrainAreaReceiveResult::Ignored;
        };
        if !warhead.wood || terrain_type.immune {
            return TerrainAreaReceiveResult::Ignored;
        }

        let resolved_damage = damage::kernel::apply_warhead_damage(
            raw_damage,
            warhead.cell_spread_f64,
            warhead.percent_at_max_f64,
            &warhead.verses_f64,
            damage::ArmorClass(armor_index(&terrain_type.armor) as u8),
            distance_leptons,
            scenario_no_damage,
            rules.combat_damage.max_damage,
        );
        if resolved_damage == 0 {
            return TerrainAreaReceiveResult::Ignored;
        }

        let terrain = self
            .terrain_objects
            .get_mut(&stable_id)
            .expect("captured Terrain receiver remains represented");
        if resolved_damage < 0 {
            terrain.health = terrain
                .health
                .wrapping_sub(resolved_damage)
                .min(terrain.max_health);
            return TerrainAreaReceiveResult::Damaged {
                remaining: terrain.health,
            };
        }

        let remaining = terrain.health.wrapping_sub(resolved_damage);
        if remaining > 0 {
            terrain.health = remaining;
            return TerrainAreaReceiveResult::Damaged { remaining };
        }

        terrain.health = 0;
        self.finalizing_terrain.insert(stable_id);
        TerrainAreaReceiveResult::Lethal(TerrainLethalDamage {
            stable_id,
            cell,
            spawns_tiberium: terrain_type.spawns_tiberium,
        })
    }

    /// Complete a lethal receiver after any nested C4 transaction has returned.
    pub(crate) fn finalize_lethal(
        &mut self,
        lethal: TerrainLethalDamage,
        resolved_terrain: Option<&mut ResolvedTerrainGrid>,
    ) -> bool {
        if !self.finalizing_terrain.contains(&lethal.stable_id) {
            return false;
        }
        if self.terrain_object_cells.get(&lethal.cell) != Some(&lethal.stable_id) {
            self.finalizing_terrain.remove(&lethal.stable_id);
            return false;
        }

        let removed_id = limbo_terrain_object_at_cell_parts(
            self.authority_parts(),
            lethal.cell,
            resolved_terrain,
        );
        let finalized = removed_id == Some(lethal.stable_id);
        if finalized {
            if let Some(terrain) = self.terrain_objects.get_mut(&lethal.stable_id) {
                terrain.lifecycle = TerrainObjectLifecycle::Destroyed;
            }
            if !self.navigation_changed_cells.contains(&lethal.cell) {
                self.navigation_changed_cells.push(lethal.cell);
            }
        }
        self.finalizing_terrain.remove(&lethal.stable_id);
        finalized
    }

    fn authority_parts(&mut self) -> TerrainAuthorityParts<'_> {
        TerrainAuthorityParts {
            terrain_spawners: &mut self.terrain_spawners,
            terrain_objects: &mut self.terrain_objects,
            terrain_object_cells: &mut self.terrain_object_cells,
            terrain_occupation_bits: &mut self.terrain_occupation_bits,
            tiberium_spawning_terrain_cells: &mut self.tiberium_spawning_terrain_cells,
            raw_occupation: &mut self.raw_occupation,
        }
    }
}

struct TerrainAuthorityParts<'a> {
    terrain_spawners: &'a mut BTreeMap<(u16, u16), TerrainSpawnerState>,
    terrain_objects: &'a mut BTreeMap<u64, TerrainObjectState>,
    terrain_object_cells: &'a mut BTreeMap<(u16, u16), u64>,
    terrain_occupation_bits: &'a mut BTreeMap<(u16, u16), u8>,
    tiberium_spawning_terrain_cells: &'a mut BTreeSet<(u16, u16)>,
    raw_occupation: &'a mut RawCellOccupationGrid,
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
    raw_occupation.mark_ground(cell.0, cell.1, terrain_raw_occupation_mask(source_mask));
}

fn unmark_terrain_raw_occupation(
    raw_occupation: &mut RawCellOccupationGrid,
    cell: (u16, u16),
    source_mask: u8,
) {
    raw_occupation.clear_ground(cell.0, cell.1, terrain_raw_occupation_mask(source_mask));
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
    limbo_terrain_object_at_cell_parts(
        production_authority_parts(production, raw_occupation),
        cell,
        resolved_terrain,
    )
    .is_some()
}

fn production_authority_parts<'a>(
    production: &'a mut ProductionState,
    raw_occupation: &'a mut RawCellOccupationGrid,
) -> TerrainAuthorityParts<'a> {
    TerrainAuthorityParts {
        terrain_spawners: &mut production.terrain_spawners,
        terrain_objects: &mut production.terrain_objects,
        terrain_object_cells: &mut production.terrain_object_cells,
        terrain_occupation_bits: &mut production.terrain_occupation_bits,
        tiberium_spawning_terrain_cells: &mut production.tiberium_spawning_terrain_cells,
        raw_occupation,
    }
}

fn limbo_terrain_object_at_cell_parts(
    authority: TerrainAuthorityParts<'_>,
    cell: (u16, u16),
    resolved_terrain: Option<&mut ResolvedTerrainGrid>,
) -> Option<u64> {
    let stable_id = *authority.terrain_object_cells.get(&cell)?;
    let snapshot = authority.terrain_objects.get(&stable_id)?.clone();
    if !snapshot.is_live() {
        return None;
    }

    let source_cell = snapshot.cell();
    authority
        .raw_occupation
        .clear_ground(source_cell.0, source_cell.1, TERRAIN_LIMBO_CLEAR_BIT);
    authority.terrain_object_cells.remove(&cell);
    unmark_terrain_raw_occupation(
        authority.raw_occupation,
        source_cell,
        snapshot.occupation_bits,
    );
    authority.terrain_occupation_bits.remove(&source_cell);
    if let Some(grid) = resolved_terrain {
        set_resolved_terrain_object_occupation(grid, source_cell, None);
    }
    if let Some(terrain) = authority.terrain_objects.get_mut(&stable_id) {
        terrain.lifecycle = TerrainObjectLifecycle::Limbo;
    }
    authority.terrain_spawners.remove(&source_cell);
    authority
        .tiberium_spawning_terrain_cells
        .remove(&source_cell);
    Some(stable_id)
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
    let mut area_state = TerrainAreaState::take_from(production, raw_occupation);
    let receive_result = area_state.terrain_object_cells.get(&cell).copied().map_or(
        TerrainAreaReceiveResult::Ignored,
        |stable_id| {
            area_state.receive_area_damage(
                stable_id,
                cell,
                base_damage,
                0,
                warhead,
                rules,
                interner,
            )
        },
    );
    let result = match receive_result {
        TerrainAreaReceiveResult::Ignored => TerrainDamageResult::Ignored,
        TerrainAreaReceiveResult::Damaged { remaining } => {
            TerrainDamageResult::Damaged { remaining }
        }
        TerrainAreaReceiveResult::Lethal(lethal) => {
            if area_state.finalize_lethal(lethal, resolved_terrain) {
                TerrainDamageResult::Destroyed
            } else {
                TerrainDamageResult::Ignored
            }
        }
    };
    let _ = area_state.restore_into(production, raw_occupation);
    result
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
    use crate::rules::locomotor_type::MovementZone;
    use crate::rules::ruleset::RuleSet;
    use crate::rules::terrain_rules::{LandType, SpeedCostProfile, TerrainClass};
    use crate::sim::movement::bump_crush::{
        CrushCapability, build_blocker_neighbor_counts, collect_crush_victims,
    };
    use crate::sim::movement::locomotor::MovementLayer;
    use crate::sim::pathfinding::PathGrid;
    use crate::sim::pathfinding::cell_entry::{
        CanEnterCellContext, CanEnterCellResult, TerrainEntryMode, evaluate_can_enter_cell,
    };
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
              [{}]\nFixtureOnly=1\n{}",
            type_name, wood, type_name, type_section
        ));
        RuleSet::from_ini(&ini).expect("rules")
    }

    fn terrain_kernel_rules(warhead_section: &str, max_damage: i32) -> RuleSet {
        let ini = IniFile::from_str(&format!(
            "[General]\nTreeStrength=100\n\
              [CombatDamage]\nMaxDamage={}\n\
              [InfantryTypes]\n\
              [VehicleTypes]\n0=DUMMY\n\
              [AircraftTypes]\n\
              [BuildingTypes]\n\
              [TerrainTypes]\n0=TREE01\n\
              [DUMMY]\nPrimary=Gun\nStrength=100\nArmor=heavy\n\
              [Gun]\nDamage=10\nWarhead=WH\n\
              [WH]\nWood=yes\n{}\n\
              [TREE01]\nStrength=100\nArmor=wood\nTemperateOccupationBits=7\n",
            max_damage, warhead_section
        ));
        RuleSet::from_ini(&ini).expect("kernel rules")
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

    fn seed_one_at(sim: &mut Simulation, rules: &RuleSet, type_name: &str, cell: (u16, u16)) {
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

    fn resolved_clear_grid_3x3() -> ResolvedTerrainGrid {
        let template = resolved_clear_grid()
            .cell(0, 0)
            .expect("clear template")
            .clone();
        let mut cells = Vec::with_capacity(9);
        for ry in 0..3 {
            for rx in 0..3 {
                cells.push(ResolvedTerrainCell {
                    rx,
                    ry,
                    ..template.clone()
                });
            }
        }
        ResolvedTerrainGrid::from_cells(3, 3, cells)
    }

    #[test]
    fn gsi_04_10_crushers_do_not_admit_or_remove_terrain_even_when_crushable_yes() {
        let rules = terrain_rules("TREE01", true, "Crushable=yes\nTemperateOccupationBits=4\n");
        let mut sim = Simulation::new();
        let source_cell = (1, 1);
        seed_one_at(&mut sim, &rules, "TREE01", source_cell);
        let stable_id = sim.production.terrain_object_cells[&source_cell];
        let before = sim.production.terrain_objects[&stable_id].clone();

        let mut resolved = resolved_clear_grid_3x3();
        mark_terrain_occupation(&mut sim.production, &before, Some(&mut resolved));
        let path_grid = PathGrid::from_resolved_terrain(&resolved);
        let blocker_counts = build_blocker_neighbor_counts(
            &sim.substrate.entities,
            3,
            3,
            Some(&resolved),
            &sim.interner,
            None,
        );
        let blocker_counts_ref = &blocker_counts;
        let before_neighbor_total = (0..3)
            .flat_map(|ry| (0..3).map(move |rx| blocker_counts_ref.count_at(rx, ry) as u32))
            .sum::<u32>();
        let before_zone = resolved
            .cell(source_cell.0, source_cell.1)
            .expect("Terrain source")
            .zone_type;

        for movement_zone in [MovementZone::Crusher, MovementZone::CrusherAll] {
            assert_eq!(
                evaluate_can_enter_cell(CanEnterCellContext {
                    target: source_cell,
                    terrain_layer: MovementLayer::Ground,
                    movement_zone: Some(movement_zone),
                    speed_type: None,
                    path_grid: Some(&path_grid),
                    resolved_terrain: Some(&resolved),
                    terrain_costs: None,
                    bypass_grid: false,
                    mode: TerrainEntryMode::RuntimeTransition,
                    is_infantry: false,
                }),
                CanEnterCellResult::HardBlocked,
                "Terrain's ObjectClass identity blocks entry even when custom rules spell Crushable=yes"
            );
        }

        for capability in [
            CrushCapability::new(true, false),
            CrushCapability::new(false, true),
        ] {
            assert!(
                collect_crush_victims(
                    source_cell,
                    &sim.substrate.occupancy,
                    MovementLayer::Ground,
                    capability,
                    &sim.substrate.entities,
                )
                .is_empty(),
                "Terrain never enters the Techno crush-victim list"
            );
        }

        let after = &sim.production.terrain_objects[&stable_id];
        assert_eq!(sim.production.terrain_object_cells[&source_cell], stable_id);
        assert_eq!(after.lifecycle, TerrainObjectLifecycle::Live);
        assert_eq!(after.health, before.health);
        assert_eq!(
            resolved
                .cell(source_cell.0, source_cell.1)
                .unwrap()
                .terrain_object_occupation,
            Some(4)
        );
        assert_eq!(before_neighbor_total, 8);
        assert_eq!(before_zone, zone_class::BUILDING);
        assert_eq!(
            resolved
                .cell(source_cell.0, source_cell.1)
                .expect("unchanged Terrain source")
                .zone_type,
            before_zone
        );
    }

    #[test]
    fn gsi_04_10_terrain_area_state_swaps_one_authoritative_runtime_state() {
        let rules = terrain_rules("TREE01", true, "TemperateOccupationBits=7\n");
        let mut sim = Simulation::new();
        sim.substrate.raw_cell_occupation.mark_ground(0, 0, 0x80);
        seed_one_at(&mut sim, &rules, "TREE01", (0, 0));
        let stable_id = sim.production.terrain_object_cells[&(0, 0)];

        let mut area = TerrainAreaState::take_from(
            &mut sim.production,
            &mut sim.substrate.raw_cell_occupation,
        );
        assert!(sim.production.terrain_objects.is_empty());
        assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(0, 0), 0);
        assert_eq!(area.terrain_object_cells[&(0, 0)], stable_id);
        assert_eq!(area.raw_occupation.ground_bits(0, 0), 0x9C);

        area.swap_authority(&mut sim.production, &mut sim.substrate.raw_cell_occupation);
        assert_eq!(sim.production.terrain_object_cells[&(0, 0)], stable_id);
        assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(0, 0), 0x9C);
        sim.substrate.raw_cell_occupation.clear_ground(0, 0, 0x80);
        area.swap_authority(&mut sim.production, &mut sim.substrate.raw_cell_occupation);

        let changed =
            area.restore_into(&mut sim.production, &mut sim.substrate.raw_cell_occupation);
        assert!(changed.is_empty());
        assert_eq!(sim.production.terrain_object_cells[&(0, 0)], stable_id);
        assert_eq!(sim.substrate.raw_cell_occupation.ground_bits(0, 0), 0x1C);
    }

    #[test]
    fn gsi_04_10_terrain_area_receive_uses_shared_fractional_kernel_and_max_damage() {
        let fractional_rules = terrain_kernel_rules(
            "CellSpread=1\n\
             PercentAtMax=0.5\n\
             Verses=100%,100%,100%,100%,100%,100%,50.5%,100%,100%,0%,0%",
            10_000,
        );
        let mut fractional = Simulation::new();
        seed_one_at(&mut fractional, &fractional_rules, "TREE01", (0, 0));
        let stable_id = fractional.production.terrain_object_cells[&(0, 0)];
        let warhead = fractional_rules.warhead("WH").expect("warhead");
        let mut area = TerrainAreaState::take_from(
            &mut fractional.production,
            &mut fractional.substrate.raw_cell_occupation,
        );
        assert_eq!(
            area.receive_area_damage(
                stable_id,
                (0, 0),
                99,
                128,
                warhead,
                &fractional_rules,
                &fractional.interner,
            ),
            TerrainAreaReceiveResult::Damaged { remaining: 63 }
        );
        let _ = area.restore_into(
            &mut fractional.production,
            &mut fractional.substrate.raw_cell_occupation,
        );

        let capped_rules = terrain_kernel_rules(
            "CellSpread=0\n\
             PercentAtMax=1\n\
             Verses=100%,100%,100%,100%,100%,100%,200%,100%,100%,0%,0%",
            25,
        );
        let mut capped = Simulation::new();
        seed_one_at(&mut capped, &capped_rules, "TREE01", (0, 0));
        let stable_id = capped.production.terrain_object_cells[&(0, 0)];
        let warhead = capped_rules.warhead("WH").expect("warhead");
        let mut area = TerrainAreaState::take_from(
            &mut capped.production,
            &mut capped.substrate.raw_cell_occupation,
        );
        assert_eq!(
            area.receive_area_damage(
                stable_id,
                (0, 0),
                8_000,
                0,
                warhead,
                &capped_rules,
                &capped.interner,
            ),
            TerrainAreaReceiveResult::Damaged { remaining: 75 }
        );
        let _ = area.restore_into(
            &mut capped.production,
            &mut capped.substrate.raw_cell_occupation,
        );
    }

    #[test]
    fn gsi_04_10_terrain_area_receive_heals_near_and_clamps_to_max() {
        let rules = terrain_rules("TREE01", true, "TemperateOccupationBits=7\n");
        let mut sim = Simulation::new();
        seed_one_at(&mut sim, &rules, "TREE01", (0, 0));
        let stable_id = sim.production.terrain_object_cells[&(0, 0)];
        let warhead = rules.warhead("WH").expect("warhead");
        let mut area = TerrainAreaState::take_from(
            &mut sim.production,
            &mut sim.substrate.raw_cell_occupation,
        );
        area.terrain_objects.get_mut(&stable_id).unwrap().health = 4;

        assert_eq!(
            area.receive_area_damage(stable_id, (0, 0), -50, 7, warhead, &rules, &sim.interner,),
            TerrainAreaReceiveResult::Damaged { remaining: 10 }
        );
        area.terrain_objects.get_mut(&stable_id).unwrap().health = 4;
        assert_eq!(
            area.receive_area_damage(stable_id, (0, 0), -50, 8, warhead, &rules, &sim.interner,),
            TerrainAreaReceiveResult::Ignored
        );
        assert_eq!(area.terrain_objects[&stable_id].health, 4);
        let _ = area.restore_into(&mut sim.production, &mut sim.substrate.raw_cell_occupation);
    }

    #[test]
    fn gsi_04_10_terrain_area_lethal_is_exact_zero_guarded_then_finalized_once() {
        let rules = rules("Immune=no\nTemperateOccupationBits=7\n");
        let mut sim = Simulation::new();
        seed_one_at(&mut sim, &rules, "TIBTRE01", (0, 0));
        let stable_id = sim.production.terrain_object_cells[&(0, 0)];
        let terrain = sim.production.terrain_objects[&stable_id].clone();
        let mut grid = resolved_clear_grid();
        mark_terrain_occupation(&mut sim.production, &terrain, Some(&mut grid));
        let warhead = rules.warhead("WH").expect("warhead");
        let mut area = TerrainAreaState::take_from(
            &mut sim.production,
            &mut sim.substrate.raw_cell_occupation,
        );

        let TerrainAreaReceiveResult::Lethal(lethal) =
            area.receive_area_damage(stable_id, (0, 0), 100, 0, warhead, &rules, &sim.interner)
        else {
            panic!("expected lethal Terrain receiver");
        };
        assert!(lethal.spawns_tiberium);
        assert_eq!(area.terrain_objects[&stable_id].health, 0);
        assert_eq!(
            area.terrain_objects[&stable_id].lifecycle,
            TerrainObjectLifecycle::Live
        );
        assert!(area.is_finalizing(stable_id));
        assert_eq!(
            area.receive_area_damage(stable_id, (0, 0), 100, 0, warhead, &rules, &sim.interner,),
            TerrainAreaReceiveResult::Ignored,
            "nested Wood=yes C4 reentry must not receive or finalize twice"
        );

        assert!(area.finalize_lethal(lethal, Some(&mut grid)));
        assert!(!area.finalize_lethal(lethal, Some(&mut grid)));
        assert!(!area.is_finalizing(stable_id));
        assert_eq!(area.navigation_changed_cells(), &[(0, 0)]);
        assert_eq!(
            area.terrain_objects[&stable_id].lifecycle,
            TerrainObjectLifecycle::Destroyed
        );
        assert_eq!(area.raw_occupation.ground_bits(0, 0), 0);
        assert!(!area.terrain_object_cells.contains_key(&(0, 0)));
        assert!(!area.terrain_spawners.contains_key(&(0, 0)));
        assert!(!area.tiberium_spawning_terrain_cells.contains(&(0, 0)));
        assert!(!area.terrain_occupation_bits.contains_key(&(0, 0)));
        let cell = grid.cell(0, 0).unwrap();
        assert_eq!(cell.terrain_object_occupation, None);
        assert!(!cell.terrain_object_blocks);

        let changed =
            area.restore_into(&mut sim.production, &mut sim.substrate.raw_cell_occupation);
        assert_eq!(changed, vec![(0, 0)]);
        assert_eq!(
            sim.production.terrain_objects[&stable_id].lifecycle,
            TerrainObjectLifecycle::Destroyed
        );
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
