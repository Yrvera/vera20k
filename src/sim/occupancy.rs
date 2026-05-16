//! Persistent per-cell occupancy grid — tracks which entities occupy each map cell.
//!
//! Replaces the ephemeral `build_occupancy_maps()` approach with an incrementally
//! maintained grid. Entities are added on spawn/move-in, removed on death/move-out.
//! Structures occupy all their foundation cells.
//!
//! Unified single grid with layer-tagged occupants (no separate ground/bridge maps).
//! Equivalent to the original engine's CellClass::FirstObject/AltObject linked lists.
//!
//! ## Dependency rules
//! - Part of sim/ — depends on sim/movement/locomotor (MovementLayer).
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

use std::collections::BTreeMap;

use crate::map::entities::EntityCategory;
use crate::sim::movement::locomotor::MovementLayer;

/// Single occupant entry in a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellOccupant {
    pub entity_id: u64,
    pub layer: MovementLayer,
    /// Infantry sub-cell (2, 3, or 4). None for vehicles/structures.
    pub sub_cell: Option<u8>,
}

/// Requested insertion order for a cell's selected gamemd object list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellListInsertion {
    PrependNonBuilding,
    AppendBuilding,
}

impl CellListInsertion {
    pub fn from_category(category: EntityCategory) -> Self {
        if category == EntityCategory::Structure {
            Self::AppendBuilding
        } else {
            Self::PrependNonBuilding
        }
    }
}

/// All occupants of a single cell.
#[derive(Debug, Clone, Default)]
pub struct CellOccupancy {
    /// Occupant list. Common case is 0-3 infantry or 1 vehicle per cell.
    pub occupants: Vec<CellOccupant>,
}

impl CellOccupancy {
    /// All occupants on a selected movement layer in gamemd list order.
    pub fn iter_layer(&self, layer: MovementLayer) -> impl Iterator<Item = &CellOccupant> + '_ {
        self.occupants.iter().filter(move |o| o.layer == layer)
    }

    /// Non-infantry occupants on a given layer, preserving layer-list order.
    pub fn blockers(&self, layer: MovementLayer) -> impl Iterator<Item = u64> + '_ {
        self.occupants
            .iter()
            .filter(move |o| o.layer == layer && o.sub_cell.is_none())
            .map(|o| o.entity_id)
    }

    /// Infantry occupants on a given layer, preserving layer-list order.
    pub fn infantry(&self, layer: MovementLayer) -> impl Iterator<Item = (u64, u8)> + '_ {
        self.occupants
            .iter()
            .filter(move |o| o.layer == layer && o.sub_cell.is_some())
            .map(|o| (o.entity_id, o.sub_cell.unwrap()))
    }

    /// Whether this cell has any occupants on the given layer.
    pub fn is_empty_on(&self, layer: MovementLayer) -> bool {
        !self.occupants.iter().any(|o| o.layer == layer)
    }

    /// Whether this cell has any non-infantry occupants on the given layer.
    pub fn has_blockers_on(&self, layer: MovementLayer) -> bool {
        self.occupants
            .iter()
            .any(|o| o.layer == layer && o.sub_cell.is_none())
    }

    /// Count occupants on a given layer.
    pub fn count_on(&self, layer: MovementLayer) -> usize {
        self.occupants.iter().filter(|o| o.layer == layer).count()
    }
}

/// Persistent per-cell occupancy index, stored on `Simulation`.
///
/// Mirrors entity positions: every entity that occupies a map cell has an entry.
/// Structures occupy all their foundation cells. Maintained incrementally — add
/// on spawn/move-in, remove on death/move-out.
pub struct OccupancyGrid {
    cells: BTreeMap<(u16, u16), CellOccupancy>,
}

impl Default for OccupancyGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl OccupancyGrid {
    /// Rebuild occupancy from scratch by scanning all entities.
    /// Used at map load (deserialization) and for debug validation.
    pub fn rebuild(entities: &crate::sim::entity_store::EntityStore) -> Self {
        let mut grid = Self::new();
        for entity in entities.values() {
            // Entities inside transports don't occupy cells.
            if entity.passenger_role.is_inside_transport() {
                continue;
            }
            let Some(layer) = entity.occupancy_list_layer() else {
                continue;
            };
            let rx = entity.position.rx;
            let ry = entity.position.ry;
            let sid = entity.stable_id;
            let sub = if entity.category == EntityCategory::Infantry {
                entity.sub_cell
            } else {
                None
            };
            let insertion = CellListInsertion::from_category(entity.category);
            grid.add(rx, ry, sid, layer, sub, insertion);
        }
        grid
    }
}

impl OccupancyGrid {
    /// Create an empty occupancy grid.
    pub fn new() -> Self {
        Self {
            cells: BTreeMap::new(),
        }
    }

    /// Add an entity to a cell. For structures, caller must invoke once per
    /// foundation cell.
    pub fn add(
        &mut self,
        rx: u16,
        ry: u16,
        entity_id: u64,
        layer: MovementLayer,
        sub_cell: Option<u8>,
        insertion: CellListInsertion,
    ) {
        let new_occupant = CellOccupant {
            entity_id,
            layer,
            sub_cell,
        };
        let occ = self.cells.entry((rx, ry)).or_default();
        match insertion {
            CellListInsertion::PrependNonBuilding => {
                let index = occ
                    .occupants
                    .iter()
                    .position(|o| o.layer == layer)
                    .unwrap_or(0);
                occ.occupants.insert(index, new_occupant);
            }
            CellListInsertion::AppendBuilding => {
                let index = occ
                    .occupants
                    .iter()
                    .rposition(|o| o.layer == layer)
                    .map_or(occ.occupants.len(), |i| i + 1);
                occ.occupants.insert(index, new_occupant);
            }
        }
    }

    /// Remove an entity from a cell. No-op if entity not found.
    /// For structures, caller must invoke once per foundation cell.
    pub fn remove(&mut self, rx: u16, ry: u16, entity_id: u64) {
        if let Some(occ) = self.cells.get_mut(&(rx, ry)) {
            occ.occupants.retain(|o| o.entity_id != entity_id);
            if occ.occupants.is_empty() {
                self.cells.remove(&(rx, ry));
            }
        }
    }

    /// Move an entity from one cell to another (remove + add).
    pub fn move_entity(
        &mut self,
        old_rx: u16,
        old_ry: u16,
        new_rx: u16,
        new_ry: u16,
        entity_id: u64,
        layer: MovementLayer,
        sub_cell: Option<u8>,
        insertion: CellListInsertion,
    ) {
        self.remove(old_rx, old_ry, entity_id);
        self.add(new_rx, new_ry, entity_id, layer, sub_cell, insertion);
    }

    /// Update an entity's sub-cell within the same cell.
    pub fn update_sub_cell(&mut self, rx: u16, ry: u16, entity_id: u64, new_sub_cell: Option<u8>) {
        if let Some(occ) = self.cells.get_mut(&(rx, ry)) {
            if let Some(o) = occ.occupants.iter_mut().find(|o| o.entity_id == entity_id) {
                o.sub_cell = new_sub_cell;
            }
        }
    }

    /// Get occupancy for a cell (all layers).
    pub fn get(&self, rx: u16, ry: u16) -> Option<&CellOccupancy> {
        self.cells.get(&(rx, ry))
    }

    /// Check if a cell has no occupants on a given layer.
    pub fn is_empty_on_layer(&self, rx: u16, ry: u16, layer: MovementLayer) -> bool {
        self.cells
            .get(&(rx, ry))
            .map_or(true, |occ| occ.is_empty_on(layer))
    }

    /// Count total occupants on a layer in a cell.
    pub fn count_on_layer(&self, rx: u16, ry: u16, layer: MovementLayer) -> usize {
        self.cells
            .get(&(rx, ry))
            .map_or(0, |occ| occ.count_on(layer))
    }

    /// Check if a specific entity is in a specific cell.
    pub fn contains_entity(&self, rx: u16, ry: u16, entity_id: u64) -> bool {
        self.cells
            .get(&(rx, ry))
            .is_some_and(|occ| occ.occupants.iter().any(|o| o.entity_id == entity_id))
    }

    /// Total number of occupied cells (for diagnostics).
    pub fn occupied_cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Assert that this grid matches an expected grid. Panics with a diff on mismatch.
    /// Only compiled in debug builds — used as a safety net after each tick.
    #[cfg(debug_assertions)]
    pub fn debug_assert_matches(&self, expected: &OccupancyGrid) {
        let self_cells: std::collections::BTreeSet<(u16, u16)> =
            self.cells.keys().copied().collect();
        let expected_cells: std::collections::BTreeSet<(u16, u16)> =
            expected.cells.keys().copied().collect();
        let missing: Vec<_> = expected_cells.difference(&self_cells).collect();
        let extra: Vec<_> = self_cells.difference(&expected_cells).collect();
        if !missing.is_empty() || !extra.is_empty() {
            panic!(
                "OccupancyGrid mismatch: {} missing cells, {} extra cells.\n\
                 Missing (expected but not in grid): {:?}\n\
                 Extra (in grid but not expected): {:?}",
                missing.len(),
                extra.len(),
                &missing[..missing.len().min(10)],
                &extra[..extra.len().min(10)],
            );
        }
        for (&cell, expected_occ) in &expected.cells {
            let actual_occ = self.cells.get(&cell).unwrap();
            let mut expected_ids: Vec<u64> =
                expected_occ.occupants.iter().map(|o| o.entity_id).collect();
            let mut actual_ids: Vec<u64> =
                actual_occ.occupants.iter().map(|o| o.entity_id).collect();
            expected_ids.sort();
            actual_ids.sort();
            if expected_ids != actual_ids {
                panic!(
                    "OccupancyGrid mismatch at cell ({},{}): expected {:?}, got {:?}",
                    cell.0, cell.1, expected_ids, actual_ids,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_get() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let occ = grid.get(5, 5).unwrap();
        assert_eq!(occ.occupants.len(), 1);
        assert_eq!(occ.occupants[0].entity_id, 1);
        assert_eq!(occ.occupants[0].layer, MovementLayer::Ground);
        assert!(occ.occupants[0].sub_cell.is_none());
    }

    #[test]
    fn remove_cleans_up_empty_cell() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.remove(5, 5, 1);
        assert!(grid.get(5, 5).is_none());
        assert_eq!(grid.occupied_cell_count(), 0);
    }

    #[test]
    fn remove_nonexistent_is_noop() {
        let mut grid = OccupancyGrid::new();
        grid.remove(5, 5, 99);
        assert!(grid.get(5, 5).is_none());
    }

    #[test]
    fn move_entity_transfers_between_cells() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.move_entity(
            5,
            5,
            6,
            6,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        assert!(grid.get(5, 5).is_none());
        let occ = grid.get(6, 6).unwrap();
        assert_eq!(occ.occupants.len(), 1);
        assert_eq!(occ.occupants[0].entity_id, 1);
    }

    #[test]
    fn layer_filtering() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            2,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            3,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );

        let occ = grid.get(5, 5).unwrap();
        let ground_ids: Vec<u64> = occ
            .iter_layer(MovementLayer::Ground)
            .map(|o| o.entity_id)
            .collect();
        assert_eq!(ground_ids, vec![3, 1]);
        let ground_blockers: Vec<u64> = occ.blockers(MovementLayer::Ground).collect();
        assert_eq!(ground_blockers, vec![1]);
        let bridge_blockers: Vec<u64> = occ.blockers(MovementLayer::Bridge).collect();
        assert_eq!(bridge_blockers, vec![2]);
        let ground_inf: Vec<(u64, u8)> = occ.infantry(MovementLayer::Ground).collect();
        assert_eq!(ground_inf, vec![(3, 2)]);
        assert_eq!(occ.infantry(MovementLayer::Bridge).count(), 0);
    }

    #[test]
    fn is_empty_on_layer() {
        let mut grid = OccupancyGrid::new();
        assert!(grid.is_empty_on_layer(5, 5, MovementLayer::Ground));
        grid.add(
            5,
            5,
            1,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        assert!(grid.is_empty_on_layer(5, 5, MovementLayer::Ground));
        assert!(!grid.is_empty_on_layer(5, 5, MovementLayer::Bridge));
    }

    #[test]
    fn infantry_subcells() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            10,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            11,
            MovementLayer::Ground,
            Some(3),
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            12,
            MovementLayer::Ground,
            Some(4),
            CellListInsertion::PrependNonBuilding,
        );

        let occ = grid.get(5, 5).unwrap();
        let inf: Vec<(u64, u8)> = occ.infantry(MovementLayer::Ground).collect();
        assert_eq!(inf.len(), 3);
        assert!(!occ.has_blockers_on(MovementLayer::Ground));
    }

    #[test]
    fn multi_cell_building() {
        let mut grid = OccupancyGrid::new();
        for dy in 0..2u16 {
            for dx in 0..2u16 {
                grid.add(
                    10 + dx,
                    10 + dy,
                    100,
                    MovementLayer::Ground,
                    None,
                    CellListInsertion::AppendBuilding,
                );
            }
        }
        assert!(grid.contains_entity(10, 10, 100));
        assert!(grid.contains_entity(11, 10, 100));
        assert!(grid.contains_entity(10, 11, 100));
        assert!(grid.contains_entity(11, 11, 100));
        assert!(!grid.contains_entity(12, 12, 100));

        for dy in 0..2u16 {
            for dx in 0..2u16 {
                grid.remove(10 + dx, 10 + dy, 100);
            }
        }
        assert_eq!(grid.occupied_cell_count(), 0);
    }

    #[test]
    fn update_sub_cell() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );
        grid.update_sub_cell(5, 5, 1, Some(4));
        let occ = grid.get(5, 5).unwrap();
        let inf: Vec<(u64, u8)> = occ.infantry(MovementLayer::Ground).collect();
        assert_eq!(inf, vec![(1, 4)]);
    }

    #[test]
    fn count_on_layer() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            2,
            MovementLayer::Ground,
            Some(2),
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            3,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        assert_eq!(grid.count_on_layer(5, 5, MovementLayer::Ground), 2);
        assert_eq!(grid.count_on_layer(5, 5, MovementLayer::Bridge), 1);
        assert_eq!(grid.count_on_layer(5, 5, MovementLayer::Air), 0);
    }

    #[test]
    fn non_buildings_prepend_on_same_layer() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            2,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let ids: Vec<u64> = grid
            .get(5, 5)
            .unwrap()
            .iter_layer(MovementLayer::Ground)
            .map(|o| o.entity_id)
            .collect();
        assert_eq!(ids, vec![2, 1]);
    }

    #[test]
    fn buildings_append_on_same_layer() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            100,
            MovementLayer::Ground,
            None,
            CellListInsertion::AppendBuilding,
        );
        grid.add(
            5,
            5,
            2,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let ids: Vec<u64> = grid
            .get(5, 5)
            .unwrap()
            .iter_layer(MovementLayer::Ground)
            .map(|o| o.entity_id)
            .collect();
        assert_eq!(ids, vec![2, 1, 100]);
    }

    #[test]
    fn layers_have_independent_order() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            10,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            2,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            20,
            MovementLayer::Bridge,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let ground: Vec<u64> = grid
            .get(5, 5)
            .unwrap()
            .iter_layer(MovementLayer::Ground)
            .map(|o| o.entity_id)
            .collect();
        let bridge: Vec<u64> = grid
            .get(5, 5)
            .unwrap()
            .iter_layer(MovementLayer::Bridge)
            .map(|o| o.entity_id)
            .collect();
        assert_eq!(ground, vec![2, 1]);
        assert_eq!(bridge, vec![20, 10]);
    }

    #[test]
    fn remove_preserves_remaining_order() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            5,
            5,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            2,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            5,
            5,
            3,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.remove(5, 5, 2);
        let ids: Vec<u64> = grid
            .get(5, 5)
            .unwrap()
            .iter_layer(MovementLayer::Ground)
            .map(|o| o.entity_id)
            .collect();
        assert_eq!(ids, vec![3, 1]);
    }

    #[test]
    fn move_entity_reinserts_with_requested_order() {
        let mut grid = OccupancyGrid::new();
        grid.add(
            1,
            1,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.add(
            2,
            2,
            2,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        grid.move_entity(
            1,
            1,
            2,
            2,
            1,
            MovementLayer::Ground,
            None,
            CellListInsertion::PrependNonBuilding,
        );
        let ids: Vec<u64> = grid
            .get(2, 2)
            .unwrap()
            .iter_layer(MovementLayer::Ground)
            .map(|o| o.entity_id)
            .collect();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn rebuild_uses_category_insertion() {
        let mut entities = crate::sim::entity_store::EntityStore::new();
        let mut first = crate::sim::game_entity::GameEntity::test_default(1, "E1", "Allies", 5, 5);
        first.category = EntityCategory::Infantry;
        first.sub_cell = Some(2);
        let mut second =
            crate::sim::game_entity::GameEntity::test_default(2, "HTNK", "Allies", 5, 5);
        second.category = EntityCategory::Unit;
        let mut structure =
            crate::sim::game_entity::GameEntity::test_default(100, "GAPOWR", "Allies", 5, 5);
        structure.category = EntityCategory::Structure;
        entities.insert(first);
        entities.insert(second);
        entities.insert(structure);
        let grid = OccupancyGrid::rebuild(&entities);
        let ids: Vec<u64> = grid
            .get(5, 5)
            .unwrap()
            .iter_layer(MovementLayer::Ground)
            .map(|o| o.entity_id)
            .collect();
        assert_eq!(ids, vec![2, 1, 100]);
    }
}
