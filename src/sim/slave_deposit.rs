//! Shared SlaveManager6B0880 query used by Infantry51C2BC and Walk75C4B0.
//! Reads existing master_id/slave_bindings authority; no harvest-state gate.
//! Ordered map lookups retain real/dummy Cell identity. This is a spatial
//! predicate, not a replacement for slave harvesting or manager AI.
//! Evidence: docs/research/bridges/05-damage-collapse-repair-cabhut/
//! LIVE_REPAIR_SLAVE_PREDICATE.md (original bodies, callers and retail limits).
use crate::map::{
    cell_index::NativeCellIdentity as Cell, entities::EntityCategory,
    resolved_terrain::ResolvedTerrainGrid,
};
use crate::rules::ruleset::RuleSet;
use crate::sim::movement::{ground_pose, locomotor::MovementLayer};
use crate::sim::{entity_store::EntityStore, intern::StringInterner, occupancy::OccupancyGrid};
use std::collections::BTreeMap;

pub(crate) struct SlaveDepositQuery<'a> {
    pub entities: &'a EntityStore,
    pub bindings: &'a BTreeMap<u64, Vec<u64>>,
    pub occupancy: &'a OccupancyGrid,
    pub terrain: &'a ResolvedTerrainGrid,
    pub rules: &'a RuleSet,
    pub interner: &'a StringInterner,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::sim::{
        game_entity::GameEntity, occupancy::CellListInsertion, slave_miner::SlaveHarvester,
    };
    use crate::util::fixed_math::SimFixed;

    fn setup() -> (
        EntityStore,
        StringInterner,
        RuleSet,
        OccupancyGrid,
        BTreeMap<u64, Vec<u64>>,
    ) {
        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[BuildingTypes]\n0=MASTER\n[MASTER]\nFoundation=2x2\n[InfantryTypes]\n0=SLAV\n[VehicleTypes]\n0=OTHER\n",
        )).unwrap();
        let mut interner = StringInterner::new();
        let mut entities = EntityStore::new();
        for (id, name, category) in [
            (1, "MASTER", EntityCategory::Structure),
            (2, "SLAV", EntityCategory::Infantry),
            (3, "OTHER", EntityCategory::Unit),
        ] {
            let mut e = GameEntity::test_default(id, name, "Owner", 4, 4);
            e.owner = interner.intern("Owner");
            e.type_ref = interner.intern(name);
            e.category = category;
            if id == 2 {
                e.slave_harvester = Some(SlaveHarvester::new(1, 4));
            }
            if id == 3 {
                e.position.sub_x = SimFixed::from_num(32);
                e.position.sub_y = SimFixed::from_num(32);
            }
            entities.insert(e);
        }
        let mut occupancy = OccupancyGrid::default();
        for p in [(4, 4), (5, 4), (4, 5), (5, 5)] {
            occupancy.add(
                p.0,
                p.1,
                1,
                MovementLayer::Ground,
                None,
                CellListInsertion::AppendBuilding,
            );
        }
        occupancy.add(
            5,
            4,
            3,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        (
            entities,
            interner,
            rules,
            occupancy,
            BTreeMap::from([(1, vec![2])]),
        )
    }

    #[test]
    fn slave_deposit_uses_foundation_center_and_live_manager_membership() {
        let (mut entities, interner, rules, occupancy, mut bindings) = setup();
        let terrain = ResolvedTerrainGrid::from_cells(
            8,
            8,
            (0..8)
                .flat_map(|y| {
                    (0..8).map(move |x| {
                        crate::sim::world::common_raw_test_terrain_cell(x, y, 0, false)
                    })
                })
                .collect(),
        );
        macro_rules! query {
            () => {
                SlaveDepositQuery {
                    entities: &entities,
                    bindings: &bindings,
                    occupancy: &occupancy,
                    terrain: &terrain,
                    rules: &rules,
                    interner: &interner,
                }
            };
        }
        let primary = terrain.native_cell_identity((5, 5));
        let secondary = terrain.native_cell_identity((5, 4));
        assert!(query!().admits(2, 1, primary));
        assert!(query!().admits(2, 1, secondary));
        // +48 turns the 2x2 master's low bytes from (128,128) into(0,0),
        // beating the earlier Unit(32,32). Raw anchor scoring would fail.
        let input = crate::sim::components::DriveCoord {
            x: 5 * 256 + 192,
            y: 4 * 256 + 64,
            z: 0,
        };
        assert!(query!().walk_priority(2, input));
        entities
            .get_mut(2)
            .unwrap()
            .slave_harvester
            .as_mut()
            .unwrap()
            .state = crate::sim::slave_miner::SlaveHarvestState::Idle;
        assert!(query!().walk_priority(2, input));
        bindings.get_mut(&1).unwrap().clear();
        assert_eq!(query!().master(2), Some(1));
        assert!(!query!().walk_priority(2, input));
        bindings.insert(1, vec![2]);
        entities
            .get_mut(2)
            .unwrap()
            .slave_harvester
            .as_mut()
            .unwrap()
            .master_id = 3;
        assert!(!query!().walk_priority(2, input));
    }

    #[test]
    fn slave_deposit_keeps_dummy_identity_and_lookup_before_membership() {
        let (entities, interner, rules, occupancy, mut bindings) = setup();
        let terrain = ResolvedTerrainGrid::from_cells(
            8,
            8,
            vec![crate::sim::world::common_raw_test_terrain_cell(
                4, 4, 0, false,
            )],
        );
        let queried = terrain.native_cell_identity((7, 7));
        assert_eq!(queried, Cell::Dummy);
        macro_rules! query {
            () => {
                SlaveDepositQuery {
                    entities: &entities,
                    bindings: &bindings,
                    occupancy: &occupancy,
                    terrain: &terrain,
                    rules: &rules,
                    interner: &interner,
                }
            };
        }
        assert!(
            query!().admits(2, 1, queried),
            "distinct absent coordinates alias one native Cell"
        );
        assert_eq!(
            terrain.dummy_cell_requested_coord(),
            (5, 5),
            "secondary(5,4) is queried before primary(5,5)"
        );
        bindings.insert(1, vec![]);
        terrain.stamp_dummy_cell_requested_coord(7, 7);
        assert!(!query!().admits(2, 1, queried));
        assert_eq!(
            terrain.dummy_cell_requested_coord(),
            (5, 5),
            "empty manager still performs geometry"
        );
        bindings.remove(&1);
        terrain.stamp_dummy_cell_requested_coord(7, 7);
        assert!(!query!().admits(2, 1, queried));
        assert_eq!(
            terrain.dummy_cell_requested_coord(),
            (7, 7),
            "absent manager fails the caller gate"
        );
    }
}

impl SlaveDepositQuery<'_> {
    /// The caller tests both child+2DC and master+2D8 before6B0880. An
    /// existing empty manager remains admitted to the geometry/lookup body.
    pub(crate) fn master(&self, slave: u64) -> Option<u64> {
        let master = self
            .entities
            .get(slave)?
            .slave_harvester
            .as_ref()?
            .master_id;
        self.bindings.contains_key(&master).then_some(master)
    }

    fn first_building(&self, cell: Cell) -> Option<u64> {
        let Cell::Real(index) = cell else {
            return None;
        };
        let c = &self.terrain.cells()[index];
        self.occupancy
            .first_building_on_layer(c.rx, c.ry, MovementLayer::Ground)
    }

    /// Native6B0690/6B0880 use current master+1B8 and Type foundation(false).
    /// The first secondary lookup precedes the primary lookup, even when the
    /// primary will match. A second secondary lookup is conditional afterward.
    pub(crate) fn admits(&self, slave: u64, master: u64, queried: Cell) -> bool {
        if self.master(slave) != Some(master) {
            return false;
        }
        let Some(owner) = self.entities.get(master) else {
            return false;
        };
        let coord = ground_pose::position_world_coord(&owner.position);
        let base = ((coord.x / 256) as i16, (coord.y / 256) as i16);
        let building = owner.category == EntityCategory::Structure;
        let primary = if building {
            let Some(kind) = self.rules.object(self.interner.resolve(owner.type_ref())) else {
                return false;
            };
            let (width, height) = crate::rules::foundation::foundation_dimensions(&kind.foundation);
            (
                base.0.wrapping_add((i32::from(width) - 1) as i16),
                base.1.wrapping_add((i32::from(height) / 2) as i16),
            )
        } else {
            base
        };
        let secondary = if building {
            //6B0934 recomputes6B0690; no callback changes its inputs here.
            let candidate = (primary.0, primary.1.wrapping_sub(1));
            let cell = self.terrain.native_cell_identity(candidate);
            (self.first_building(cell) == Some(master)).then_some(candidate)
        } else {
            None
        };
        let primary_cell = self.terrain.native_cell_identity(primary);
        let matched = queried == primary_cell
            || secondary
                .filter(|&c| c != (0, 0))
                .is_some_and(|c| self.terrain.native_cell_identity(c) == queried);
        //6B09E0 only touches the manager list after geometry has matched.
        matched
            && self
                .bindings
                .get(&master)
                .is_some_and(|members| members.iter().rev().any(|&id| id == slave))
    }

    /// Walk75C46A ->47C3D0(0,0,ground,null), then repeated input lookup at
    ///75C49A before6B0880. Raw clear is owned by the caller before entry here.
    pub(crate) fn walk_priority(
        &self,
        slave: u64,
        input: crate::sim::components::DriveCoord,
    ) -> bool {
        let Some(master) = self.master(slave) else {
            return false;
        };
        let coord = ((input.x / 256) as i16, (input.y / 256) as i16);
        let selected = self.terrain.native_cell_identity(coord);
        let nearest = match selected {
            Cell::Dummy => None,
            Cell::Real(index) => {
                let c = &self.terrain.cells()[index];
                let candidates = self
                    .occupancy
                    .get(c.rx, c.ry)
                    .into_iter()
                    .flat_map(|list| list.iter_layer(MovementLayer::Ground))
                    .filter_map(|entry| self.entities.get(entry.entity_id))
                    .filter_map(|entity| {
                        let kind = self
                            .rules
                            .object(self.interner.resolve(entity.type_ref()))?;
                        let c = ground_pose::object_center_coord(entity, kind);
                        Some((
                            entity.stable_id(),
                            true,
                            crate::sim::cell_kernel::CellQueryPoint { x: c.x, y: c.y },
                        ))
                    });
                crate::sim::cell_kernel::nearest_eligible_in_order(
                    crate::sim::cell_kernel::CellQueryPoint { x: 0, y: 0 },
                    candidates,
                )
            }
        };
        if nearest != Some(master) {
            return false;
        }
        let selected = self.terrain.native_cell_identity(coord);
        self.admits(slave, master, selected)
    }
}
