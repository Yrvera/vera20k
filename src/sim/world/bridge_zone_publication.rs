//! Live56DB70 repair callback. Keep the record owner and navigation's retained
//! input mirror synchronized without replacing the ordered hierarchy graph.
use super::*;

impl LivePublication<'_> {
    pub(super) fn validate_bridge_zones(&mut self, query: CellCoord) -> Result<bool, String> {
        let sim = &mut self.sim;
        let terrain = sim
            .resolved_terrain
            .as_ref()
            .ok_or("repair has no live terrain")?;
        let bridges = sim
            .bridge_state
            .as_mut()
            .ok_or("repair has no bridge record owner")?;
        let zones = sim
            .zone_grid
            .as_mut()
            .ok_or("repair has no live zone owner")?;
        let path = sim
            .path_grid
            .as_deref()
            .ok_or("repair has no live path owner")?;
        let bounds = sim.playfield_bounds;
        let result = bridges.validate_repaired_zones(terrain, query, |record| {
            zones.activate_repaired_bridge(terrain, record, bounds)
        });
        // Even a partially completed callback has already changed its active
        // byte. Preserve the same record state in later navigation consumers.
        zones.retain_repaired_bridge_records(path, terrain, bridges.endpoint_records());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::locomotor_type::MovementZone;
    use crate::sim::pathfinding::{PathGrid, zone_map::ZoneGrid};
    use std::sync::Arc;

    fn edge_state(sim: &Simulation) -> Vec<Vec<Vec<(u16, u8)>>> {
        let hierarchy = sim
            .zone_grid
            .as_ref()
            .unwrap()
            .hierarchy_for(MovementZone::Normal)
            .unwrap();
        (0..3)
            .map(|level| {
                let graph = hierarchy.level(level).unwrap();
                (0..graph.record_slot_count())
                    .map(|zone| {
                        graph
                            .edges(zone as u16)
                            .iter()
                            .map(|edge| (edge.neighbor, edge.flag))
                            .collect()
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn live_bridge_validation_recomputes_missing_records_and_survives_cache_reuse() {
        let rules = RuleSet::from_ini(&IniFile::from_str("")).unwrap();
        let mut terrain = ResolvedTerrainGrid::from_cells(
            16,
            16,
            (0..16)
                .flat_map(|y| {
                    (0..16).map(move |x| {
                        crate::sim::world::lifecycle_tests::common_raw_terrain_cell(x, y, 0, false)
                    })
                })
                .collect(),
        );
        terrain.test_set_high_bridge_set_starts(Some(100), Some(200));
        for (x, y) in [(5, 5), (10, 5)] {
            let cell = terrain.cell_mut(x, y).unwrap();
            cell.final_tile_index = 106;
            cell.final_sub_tile = 4;
        }
        let mut bridges =
            BridgeRuntimeState::from_resolved_terrain_with_map_size(&terrain, true, 1, (8, 8));
        // A bridge loaded already broken has no runtime damage group; native
        // kind0/active remains independent of the Rust group sentinel.
        assert_eq!(bridges.endpoint_records().len(), 1);
        assert_eq!(bridges.endpoint_records()[0].group_id, 0);
        assert!(!bridges.endpoint_records()[0].active);
        bridges.test_set_endpoint_records(Vec::new());
        let mut path = PathGrid::from_resolved_terrain_with_bridges(&terrain, Some(&bridges));
        path.set_blocked(7, 7, true); // Retained structure blocking.
        let mut sim = Simulation::with_seed(31);
        sim.install_resolved_terrain_for_new_map(terrain);
        sim.playfield_bounds = Some(crate::sim::cell_rect::PlayfieldBounds {
            base: 8,
            off_fc: 0,
            off_100: 0,
            off_104: 8,
            off_108: 8,
        });
        sim.zone_grid = Some(ZoneGrid::build_with_native_bridge_geometry(
            &path,
            &sim.terrain_costs,
            sim.resolved_terrain.as_ref(),
            bridges.endpoint_records(),
            16,
            16,
            Some((8, 8)),
        ));
        sim.bridge_state = Some(bridges);
        sim.path_grid = Some(Arc::new(path.clone()));
        let before = edge_state(&sim);
        let mut host = LivePublication {
            sim: &mut sim,
            rules: &rules,
            registry: None,
            collapsed: false,
        };
        assert!(!host.validate_bridge_zones((5, 5)).unwrap());
        assert!(sim.bridge_state.as_ref().unwrap().endpoint_records()[0].active);
        let after = edge_state(&sim);
        let count =
            |state: &Vec<Vec<Vec<(u16, u8)>>>| state.iter().flatten().map(Vec::len).sum::<usize>();
        assert_eq!(count(&after), count(&before) + 18);
        let mut host = LivePublication {
            sim: &mut sim,
            rules: &rules,
            registry: None,
            collapsed: false,
        };
        assert!(!host.validate_bridge_zones((5, 5)).unwrap());
        assert_eq!(edge_state(&sim), after);
        sim.rebuild_zone_grid(&path);
        assert_eq!(
            edge_state(&sim),
            after,
            "navigation reuse erased native repair edge history"
        );
        assert_eq!(sim.path_grid().unwrap().cell(7, 7), path.cell(7, 7));
    }
}
