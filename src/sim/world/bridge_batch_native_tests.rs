// Actual LivePublication callbacks against original586990 +47D2B0 snapshots.
// Supplied retained planes/Cells match the native corpus; no Recalc substitution.
use crate::sim::pathfinding::zone_build::{
    assert_native_hierarchy_graphs, build_zone_hierarchy_with_query, hierarchy_native_bounds,
    hierarchy_native_fixture,
};

struct BatchTrace<'a, 'world> {
    live: &'a mut LivePublication<'world>,
    queries: Vec<(i16, i16)>,
    recalcs: Vec<(i16, i16)>,
    patches: Vec<(i16, i16)>,
}

impl ZoneBatchHost for BatchTrace<'_, '_> {
    type Error = String;
    fn admitted(&mut self, coord: CellCoord) -> Result<bool, String> {
        self.queries.push(coord);
        ZoneBatchHost::admitted(self.live, coord)
    }
    fn clear_fine_zone(&mut self, coord: CellCoord) -> Result<(), String> {
        ZoneBatchHost::clear_fine_zone(self.live, coord)
    }
    fn recalc_at(&mut self, coord: CellCoord) -> Result<(), String> {
        assert_eq!(self.live.fine_zone(coord)?, 0, "clear must precede Recalc");
        let terrain = self.live.terrain();
        let actual = terrain
            .native_fixed_cell_index(coord.0, coord.1)
            .map(|i| {
                let cell = &terrain.cells()[i];
                (cell.rx as i16, cell.ry as i16)
            })
            .unwrap_or(coord);
        self.recalcs.push(actual);
        ZoneBatchHost::recalc_at(self.live, coord)
    }
    fn fine_zone(&self, coord: CellCoord) -> Result<u16, String> {
        ZoneBatchHost::fine_zone(self.live, coord)
    }
    fn patch_hierarchy(&mut self, coord: CellCoord) -> Result<(), String> {
        self.patches.push(coord);
        ZoneBatchHost::patch_hierarchy(self.live, coord)
    }
}

fn batch_coord(value: &serde_json::Value) -> CellCoord {
    (
        value[0].as_i64().unwrap() as i16,
        value[1].as_i64().unwrap() as i16,
    )
}

fn bridge_batch_simulation(case: &serde_json::Value) -> Simulation {
    let input = &case["input"];
    let label = input["name"].as_str().unwrap();
    let (mut base, mut terrain, records) = hierarchy_native_fixture(input);
    assert!(
        records.is_empty(),
        "current native batch fixtures have no bridge records"
    );
    let width = terrain.width();
    let bounds = hierarchy_native_bounds(input);
    for (row, values) in base
        .raw_zone_ids_by_row
        .iter_mut()
        .zip(case["initial"]["raw_rows"].as_array().unwrap())
    {
        *row = values
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u16)
            .collect();
    }
    crate::map::resolved_terrain::install_bridge_batch_test_catalog(
        &mut terrain,
        input["slope_recalc"].as_bool().unwrap_or(false),
    );
    let path = PathGrid::from_resolved_terrain(&terrain);
    let size = (
        input["size"][0].as_i64().unwrap() as i32,
        input["size"][1].as_i64().unwrap() as i32,
    );
    let mut zones = ZoneGrid::build_with_native_map_context(
        &path,
        &BTreeMap::new(),
        &terrain,
        &records,
        Some(size),
        Some(bounds),
    );
    *zones.base_topology_mut().unwrap() = base.clone();
    zones.replace_hierarchy(build_zone_hierarchy_with_query(
        &base,
        Some(&terrain),
        &records,
        width,
        width,
        &mut |x, y| {
            crate::sim::cell_rect::cell_is_in_playfield_height_aware(
                (x, y),
                Some(bounds),
                Some(&terrain),
            )
        },
    ));
    assert_native_hierarchy_graphs(
        zones.hierarchy_for(MovementZone::Normal).unwrap(),
        &terrain,
        &case["initial"],
        label,
    );
    let mut sim = Simulation::with_seed(0x586990);
    sim.playfield_bounds = Some(bounds);
    sim.bridge_state = Some(BridgeRuntimeState::from_resolved_terrain_with_map_size(
        &terrain, true, 300, size,
    ));
    sim.overlay_grid = Some(crate::sim::overlay_grid::OverlayGrid::new(width, width));
    sim.path_grid = Some(Arc::new(path));
    sim.zone_grid = Some(zones);
    sim.install_resolved_terrain_for_new_map(terrain);
    sim
}

#[test]
fn live_bridge_batch_matches_original_order_recalc_cache_and_hierarchy() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tools/spatial_oracle/bridge_hierarchy.json"
    ))
    .unwrap();
    let rules = RuleSet::from_ini(&IniFile::from_str("")).unwrap();
    let registry =
        crate::map::overlay_types::OverlayTypeRegistry::from_ini(&IniFile::from_str(""), None);
    let mut checked = 0;
    for case in corpus["cases"].as_array().unwrap() {
        let input = &case["input"];
        if !input["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a.get("cells").is_some())
        {
            continue;
        }
        let label = input["name"].as_str().unwrap();
        let mut sim = bridge_batch_simulation(case);
        let width = sim.resolved_terrain.as_ref().unwrap().width();
        let size = (
            input["size"][0].as_i64().unwrap() as i32,
            input["size"][1].as_i64().unwrap() as i32,
        );
        for (action, expected) in input["actions"]
            .as_array()
            .unwrap()
            .iter()
            .zip(case["states"].as_array().unwrap())
        {
            if let Some(bounds) = action.get("bounds") {
                sim.playfield_bounds = Some(crate::map::playfield::PlayfieldBounds {
                    base: size.0,
                    off_fc: bounds[0].as_i64().unwrap() as i32,
                    off_100: bounds[1].as_i64().unwrap() as i32,
                    off_104: bounds[2].as_i64().unwrap() as i32,
                    off_108: bounds[3].as_i64().unwrap() as i32,
                });
            }
            for change in action["changes"].as_array().into_iter().flatten() {
                let coord = batch_coord(&change["cell"]);
                let i = coord.1 as usize * usize::from(width) + coord.0 as usize;
                let terrain = sim.resolved_terrain.as_mut().unwrap();
                if let Some(value) = change.get("cell_level") {
                    terrain.cells[i].level = value.as_u64().unwrap() as u8;
                }
                if let Some(value) = change.get("tile") {
                    terrain.cells[i].final_tile_index = value.as_i64().unwrap() as i32;
                }
                if change["missing"] == true {
                    let allocated: Vec<_> = (0..width)
                        .flat_map(|y| (0..width).map(move |x| (x, y)))
                        .filter(|&(x, y)| {
                            (x as i16, y as i16) != coord
                                && terrain
                                    .native_fixed_cell_index(x as i16, y as i16)
                                    .is_some()
                        })
                        .collect();
                    terrain.test_set_native_allocated_cells(&allocated);
                }
                let zones = sim.zone_grid.as_mut().unwrap();
                let base = zones.base_topology_mut().unwrap();
                if let Some(value) = change.get("class") {
                    base.movement_classes[i] = value.as_u64().unwrap() as u8;
                }
                if let Some(value) = change.get("level") {
                    base.levels[i] = value.as_u64().unwrap() as u8;
                }
                if let Some(value) = change.get("base_id") {
                    base.zone_ids[i] = value.as_u64().unwrap() as u16;
                }
                if change["clear_level0"] == true {
                    let (_, hierarchy) = zones.base_and_hierarchy_mut().unwrap();
                    hierarchy.levels_mut()[0]
                        .set_native_zone_at(coord, Some(size), 0)
                        .unwrap();
                }
            }
            let cells: Vec<_> = action["cells"]
                .as_array()
                .unwrap()
                .iter()
                .map(batch_coord)
                .collect();
            let mut live = LivePublication {
                sim: &mut sim,
                rules: &rules,
                registry: Some(&registry),
                collapsed: false,
            };
            let mut trace = BatchTrace {
                live: &mut live,
                queries: Vec::new(),
                recalcs: Vec::new(),
                patches: Vec::new(),
            };
            recalculate_zone_batch(&mut trace, &cells)
                .unwrap_or_else(|error| panic!("{label}: {error}"));
            assert_eq!(
                serde_json::json!(trace.queries),
                expected["batch_queries"],
                "{label}: both outer passes"
            );
            assert_eq!(
                serde_json::json!(trace.recalcs),
                expected["recalcs"],
                "{label}: original Recalc order"
            );
            assert_eq!(
                serde_json::json!(trace.patches),
                expected["patch_calls"],
                "{label}: current fine IDs select patches"
            );
            let recalcs = trace.recalcs;
            let terrain = sim.resolved_terrain.as_ref().unwrap();
            let zones = sim.zone_grid.as_mut().unwrap();
            assert_native_hierarchy_graphs(
                zones.hierarchy_for(MovementZone::Normal).unwrap(),
                terrain,
                expected,
                label,
            );
            let base = zones.base_topology_mut().unwrap();
            assert_eq!(
                serde_json::json!(base.movement_classes),
                expected["classes"],
                "{label}: retained class"
            );
            assert_eq!(
                serde_json::json!(base.levels),
                expected["levels"],
                "{label}: retained height"
            );
            assert_eq!(
                serde_json::json!(base.zone_ids),
                expected["base_ids"],
                "{label}: base IDs conserved"
            );
            assert_eq!(
                serde_json::json!(base.raw_zone_ids_by_row),
                expected["raw_rows"],
                "{label}: all raw rows conserved"
            );
            for (coord, expected) in recalcs
                .iter()
                .zip(expected["recalculated_cells"].as_array().unwrap())
            {
                let (level, slope) =
                    if let Some(i) = terrain.native_fixed_cell_index(coord.0, coord.1) {
                        let cell = &terrain.cells()[i];
                        assert_eq!(
                            serde_json::json!(cell.zone_type),
                            expected["zone"],
                            "{label}: current Cell class"
                        );
                        (cell.level, cell.slope_type)
                    } else {
                        let dummy = terrain.shared_cell_dummy().snapshot();
                        (dummy.level as u8, dummy.slope_type)
                    };
                assert_eq!(
                    serde_json::json!(level),
                    expected["level"],
                    "{label}: current Cell height"
                );
                assert_eq!(
                    serde_json::json!(slope),
                    expected["slope"],
                    "{label}: actual resident TMP slope"
                );
            }
            assert_eq!(
                serde_json::json!(terrain.shared_cell_dummy().snapshot().coord),
                expected["dummy"],
                "{label}: final dummy"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 8);
}

#[test]
fn live_bridge_batch_recovers_rust_append_capacity_without_replacing_base_connectivity() {
    use crate::sim::pathfinding::zone_hierarchy::ZoneRecord;

    // Rust storage recovery regression, not a native threshold parity claim.
    // Exhaust only the fine level so coarse levels have already been patched
    // when the append fails; recovery must replace that partially changed graph.
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tools/spatial_oracle/bridge_hierarchy.json"
    ))
    .unwrap();
    let case = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["input"]["name"] == "clear_local")
        .unwrap();
    let mut sim = bridge_batch_simulation(case);
    let zones = sim.zone_grid.as_mut().unwrap();
    let before = zones.base_topology_mut().unwrap().clone();
    let (_, hierarchy) = zones.base_and_hierarchy_mut().unwrap();
    let fine = &mut hierarchy.levels_mut()[0];
    while let Ok(id) = u16::try_from(fine.record_slot_count()) {
        assert!(fine.append_record(ZoneRecord::new(id, 0, 0)));
    }
    assert_eq!(fine.record_slot_count(), usize::from(u16::MAX) + 1);
    let rules = RuleSet::from_ini(&IniFile::from_str("")).unwrap();
    let registry =
        crate::map::overlay_types::OverlayTypeRegistry::from_ini(&IniFile::from_str(""), None);
    LivePublication {
        sim: &mut sim,
        rules: &rules,
        registry: Some(&registry),
        collapsed: false,
    }
    .recalculate_bridge_zones(&[(10, 10)])
    .unwrap();
    let terrain = sim.resolved_terrain.as_ref().unwrap();
    let zones = sim.zone_grid.as_mut().unwrap();
    // The supplied invalid TMP leaves class/height unchanged, so complete
    // reconstruction must reproduce the original native full-build snapshot.
    assert_native_hierarchy_graphs(
        zones.hierarchy_for(MovementZone::Normal).unwrap(),
        terrain,
        &case["initial"],
        "Rust capacity recovery",
    );
    let after = zones.base_topology_mut().unwrap();
    assert_eq!(after.zone_ids, before.zone_ids);
    assert_eq!(after.raw_zone_ids_by_row, before.raw_zone_ids_by_row);
    assert_eq!(after.movement_classes, before.movement_classes);
    assert_eq!(after.levels, before.levels);
}
