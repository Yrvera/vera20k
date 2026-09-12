//! Navigation cache rebuilds read one explicit live map and bridge view.
//!
//! The cache borrows never move gameplay authority out of Simulation. Resident
//! world rebuilds and synchronous receiver rebuilds share the same policy.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::map::entities::EntityCategory;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::SpeedType;
use crate::rules::ruleset::RuleSet;
use crate::sim::bridge_state::BridgeRuntimeState;
use crate::sim::entity_store::EntityStore;
use crate::sim::intern::StringInterner;
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::terrain_cost::{TerrainCostGrid, build_canonical_terrain_cost_grids};
use crate::sim::pathfinding::zone_map::ZoneGrid;

/// Cell marking owns structure presence; shared by full and touched-cell views.
fn visit_structure_movement_cells(
    entities: &EntityStore,
    interner: &StringInterner,
    rules: &RuleSet,
    mut visit: impl FnMut((u16, u16)),
) {
    // Native: Techno enter/exit (0x005683C0 / 0x005687F0) call CellClass
    // AddContent/RemoveContent (0x0047E8A0 / 0x0047EA90), which mark/clear
    // occupation; see docs/research/bridges/02-cell-state-layering-zones/
    // BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md. Held factory objects
    // and retained attached upgrades have no independent marked footprint.
    // A dying structure still blocks until the lifecycle owner unmarks it.
    let mut structures: Vec<(u16, u16, String)> = entities
        .values()
        .filter_map(|entity| {
            (entity.category == EntityCategory::Structure && entity.lifecycle.cell_marked)
                .then_some((
                    entity.position.rx,
                    entity.position.ry,
                    interner.resolve(entity.type_ref()).to_string(),
                ))
        })
        .collect();
    structures.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    for (rx, ry, type_id) in structures {
        let object_type = rules.object(&type_id);
        let foundation = object_type
            .map(|object| object.foundation.as_str())
            .unwrap_or("1x1");
        let has_bib = object_type.is_some_and(|object| object.bib);
        let foundation_cells =
            crate::sim::production::building_base_foundation_cells(rx, ry, foundation);
        for cell in
            crate::sim::production::building_movement_blocking_cells(&foundation_cells, has_bib)
        {
            visit(cell);
        }
    }
}

pub(super) struct NavigationCaches<'a> {
    pub(super) terrain_costs: &'a mut BTreeMap<SpeedType, TerrainCostGrid>,
    pub(super) zones: &'a mut Option<ZoneGrid>,
    pub(super) path: &'a mut Option<Arc<PathGrid>>,
    /// Current operation's world bounds; this borrowed view is never retained.
    pub(super) playfield_bounds: Option<crate::map::playfield::PlayfieldBounds>,
}

impl NavigationCaches<'_> {
    pub(super) fn rebuild_dynamic(
        &mut self,
        terrain: &ResolvedTerrainGrid,
        bridges: Option<&BridgeRuntimeState>,
        entities: &EntityStore,
        interner: &StringInterner,
        rules: &RuleSet,
    ) {
        let mut grid = PathGrid::from_resolved_terrain_with_bridges(terrain, bridges);
        *self.terrain_costs = build_canonical_terrain_cost_grids(terrain);

        visit_structure_movement_cells(entities, interner, rules, |(x, y)| {
            grid.block_structure_cell(x, y);
        });

        self.rebuild_zones(&grid, terrain, bridges);
    }

    /// Publish the path/cost views of one completed47D2B0 Recalc before another
    /// bridge callback reads them. Zone IDs and hierarchy remain owned by the
    /// separate56C510/586990 callbacks. Height/slope changes require publication
    /// even when the narrower overlay passability receipt reports no change.
    pub(super) fn publish_recalculated_cell(
        &mut self,
        terrain: &ResolvedTerrainGrid,
        bridges: Option<&BridgeRuntimeState>,
        entities: &EntityStore,
        interner: &StringInterner,
        rules: &RuleSet,
        coord: (u16, u16),
    ) -> Result<(), String> {
        let cell = terrain
            .cell(coord.0, coord.1)
            .ok_or("Recalc cell is outside terrain")?;
        if let Some(zones) = self.zones.as_mut() {
            zones.refresh_base_cell_attributes_at(terrain, coord.0, coord.1);
        }
        self.publish_current_path_cell(terrain, bridges, entities, interner, rules, coord)?;
        for (&speed_type, costs) in self.terrain_costs.iter_mut() {
            if costs.width() != terrain.width()
                || costs.height() != terrain.height()
                || !costs.refresh_resolved_cell(cell, speed_type)
            {
                return Err("Recalc terrain cost cell could not be published".into());
            }
        }
        Ok(())
    }

    /// Current Cell/path view only. Native ramp repair's raw +11B writes can
    /// precede a count30 abort, with no Recalc or cached class/height publication.
    /// Keep the same path projection as Recalc without changing costs or zones.
    pub(super) fn publish_current_path_cell(
        &mut self,
        terrain: &ResolvedTerrainGrid,
        bridges: Option<&BridgeRuntimeState>,
        entities: &EntityStore,
        interner: &StringInterner,
        rules: &RuleSet,
        coord: (u16, u16),
    ) -> Result<(), String> {
        let cell = terrain
            .cell(coord.0, coord.1)
            .ok_or("path cell is outside terrain")?;
        // Cache owners are optional during loading/headless execution. Refresh
        // installed views without inventing a zone rebuild or eager map load.
        if let Some(path) = self.path.as_mut() {
            if path.width() != terrain.width() || path.height() != terrain.height() {
                return Err("path dimensions differ from terrain".into());
            }
            let mut structure_blocked = false;
            visit_structure_movement_cells(entities, interner, rules, |marked| {
                structure_blocked |= marked == coord;
            });
            if !Arc::make_mut(path).refresh_resolved_cell(cell, bridges, structure_blocked) {
                return Err("current path cell could not be published".into());
            }
        }
        Ok(())
    }

    pub(super) fn rebuild_zones(
        &mut self,
        path_grid: &PathGrid,
        terrain: &ResolvedTerrainGrid,
        bridges: Option<&BridgeRuntimeState>,
    ) {
        let records = bridges
            .map(BridgeRuntimeState::endpoint_records)
            .unwrap_or(&[]);
        let geometry = bridges.and_then(BridgeRuntimeState::native_zone_source_size);
        if self
            .zones
            .as_ref()
            .is_some_and(|zones| !zones.bridge_inputs_match(records, geometry))
        {
            self.rebuild_zones_full(path_grid, terrain, bridges);
            return;
        }
        if let (Some(prev), Some(zones)) = (self.path.as_deref(), self.zones.as_mut()) {
            if let Some(changed) = prev.diff_cells(path_grid) {
                if changed.is_empty() && zones.movement_classes_match(terrain) {
                    // PathGrid does not carry CellClass reduced zone type.
                    // Both path state and retained base classes must match.
                    *self.path = Some(Arc::new(path_grid.clone()));
                    return;
                }
                if !changed.is_empty()
                    && crate::sim::pathfinding::zone_incremental::try_incremental_update(
                        zones,
                        &changed,
                        path_grid,
                        self.terrain_costs,
                        Some(terrain),
                        bridges
                            .map(BridgeRuntimeState::endpoint_records)
                            .unwrap_or(&[]),
                    )
                {
                    log::trace!("zone: incremental update ({} cells changed)", changed.len());
                    *self.path = Some(Arc::new(path_grid.clone()));
                    return;
                }
            }
        }
        self.rebuild_zones_full(path_grid, terrain, bridges);
    }

    /// Bypass reuse when a changed reduced zone type needs a full rebuild even
    /// though boolean walkability remains identical (e.g. OccupationBits=0).
    pub(super) fn rebuild_zones_full(
        &mut self,
        path_grid: &PathGrid,
        terrain: &ResolvedTerrainGrid,
        bridges: Option<&BridgeRuntimeState>,
    ) {
        *self.zones = Some(ZoneGrid::build_with_native_map_context(
            path_grid,
            self.terrain_costs,
            terrain,
            bridges
                .map(BridgeRuntimeState::endpoint_records)
                .unwrap_or(&[]),
            bridges.and_then(BridgeRuntimeState::native_zone_source_size),
            self.playfield_bounds,
        ));
        *self.path = Some(Arc::new(path_grid.clone()));
    }
}
