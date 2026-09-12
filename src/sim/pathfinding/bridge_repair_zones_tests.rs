use super::*;
use crate::sim::bridge_state::{BridgeRecordKind, BridgeRuntimeState};
use crate::sim::pathfinding::PathGrid;
use crate::sim::pathfinding::zone_hierarchy::ZoneLevelGraph;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn coord(value: &Value) -> (u16, u16) {
    (
        value[0].as_i64().unwrap() as u16,
        value[1].as_i64().unwrap() as u16,
    )
}

fn record(value: &Value) -> BridgeEndpointRecord {
    BridgeEndpointRecord {
        endpoint_a: coord(&value[0]),
        endpoint_b: coord(&value[1]),
        active: value[2].as_bool().unwrap(),
        group_id: 0,
        bridge_kind: if value[3] == 0 {
            BridgeRecordKind::High
        } else {
            BridgeRecordKind::Low
        },
    }
}

fn hierarchy(original: &Value) -> ZoneHierarchy {
    let width = original["width"].as_u64().unwrap() as u16;
    let mut levels = std::array::from_fn::<_, 3, _>(|level| {
        let ids = serde_json::from_value(original["zones"][level].clone()).unwrap();
        ZoneLevelGraph::new(31).with_cell_zone_ids(ids, width, width)
    });
    for graph in &mut levels {
        for zone in 0..32 {
            graph.push_edge(zone, ZoneEdgeRecord::new(zone, 1));
        }
    }
    let [a, b, c] = levels;
    ZoneHierarchy::new(a, b, c)
}

fn edges(hierarchy: &ZoneHierarchy) -> Value {
    json!(
        (0..3)
            .map(|level| (0..32)
                .map(|zone| hierarchy
                    .level(level)
                    .unwrap()
                    .edges(zone)
                    .iter()
                    .map(|edge| [u32::from(edge.neighbor), u32::from(edge.flag)])
                    .collect::<Vec<_>>())
                .collect::<Vec<_>>())
            .collect::<Vec<_>>()
    )
}

#[test]
fn direct_repair_edges_match_original_all_theater_offsets_and_boundaries() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../tools/spatial_oracle/bridge_repair_zones.json"
    ))
    .unwrap();
    let mut count = 0;
    for original in corpus["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["input"]["validate"] != true)
    {
        let input = &original["input"];
        let mut graph = hierarchy(original);
        let tile = input["tile"].as_u64().unwrap_or(106);
        let offset = (tile - if tile >= 200 { 200 } else { 100 }) as usize;
        let bridge = BridgeEndpointRecord {
            endpoint_a: coord(&input["a"]),
            endpoint_b: coord(&input["b"]),
            active: true,
            group_id: 0,
            bridge_kind: BridgeRecordKind::High,
        };
        append_repaired_bridge_edges(
            &mut graph,
            &bridge,
            HIGH_BRIDGE_HIERARCHY_DIRECTIONS[offset] as u8,
            16,
            Some((8, 8)),
        )
        .unwrap();
        assert_eq!(edges(&graph), original["edges"], "{}", input["name"]);
        count += 1;
    }
    assert_eq!(count, 37);
}

#[test]
fn record_activation_matches_original_and_keeps_raw_connectivity_rows() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../tools/spatial_oracle/bridge_repair_zones.json"
    ))
    .unwrap();
    let mut count = 0;
    for original in
        corpus["edges"].as_array().unwrap().iter().filter(|row| {
            row["input"]["validate"] == true && row["input"].get("recomputed").is_none()
        })
    {
        let input = &original["input"];
        let mut terrain = crate::sim::pathfinding::zone_map_tests::terrain_from_zone_classes(
            16, 16, &[0; 256], &[0; 256],
        );
        terrain.test_set_high_bridge_set_starts(Some(100), Some(200));
        terrain.cell_mut(5, 5).unwrap().final_tile_index = 106;
        let redirect_case = input["name"] == "source_fringe";
        if redirect_case {
            // Extra compatibility consumer regression. Native Validate's
            // bridge flags are false, so these structural flags do not alter
            // its captured result. A's fringe shortcut avoids a connectivity
            // rebuild despite distinct raw endpoint labels.
            for x in [7, 8] {
                terrain.cell_mut(x, 5).unwrap().bridge_facts.raw_flags = 0x100;
            }
            terrain.cell_mut(9, 5).unwrap().final_tile_index = 106;
        }
        let path = PathGrid::from_resolved_terrain(&terrain);
        let mut owner =
            BridgeRuntimeState::from_resolved_terrain_with_map_size(&terrain, true, 1, (8, 8));
        let records = input
            .get("records")
            .map(|v| v.as_array().unwrap().iter().map(record).collect())
            .unwrap_or_else(|| {
                vec![BridgeEndpointRecord {
                    endpoint_a: coord(&input["a"]),
                    endpoint_b: coord(&input["b"]),
                    active: false,
                    group_id: 0,
                    bridge_kind: BridgeRecordKind::High,
                }]
            });
        owner.test_set_endpoint_records(records);
        let mut zones = ZoneGrid::build_with_native_bridge_geometry(
            &path,
            &BTreeMap::new(),
            Some(&terrain),
            owner.endpoint_records(),
            16,
            16,
            Some((8, 8)),
        );
        zones.hierarchy = Some(hierarchy(original));
        let base = zones.base_topology.as_mut().unwrap();
        base.zone_ids.fill(1);
        let b = coord(&input["b"]);
        let native_index = (i32::from(b.1 as i16) * 17 + i32::from(b.0 as i16)).clamp(0, 288);
        base.zone_ids[(native_index / 17 * 16 + native_index % 17) as usize] = 2;
        let raw: Vec<u16> = input
            .get("raw")
            .map(|v| serde_json::from_value(v.clone()).unwrap())
            .unwrap_or(vec![2, 3]);
        base.raw_zone_ids_by_row[0] = vec![0, raw[0], raw[1]];
        let before = (base.zone_ids.clone(), base.raw_zone_ids_by_row.clone());
        if redirect_case {
            assert_eq!(
                zones.get_zone_id_native((7, 5), MovementZone::Normal, true),
                Some(3)
            );
        }
        let bounds: Vec<i32> = input
            .get("bounds")
            .map(|v| serde_json::from_value(v.clone()).unwrap())
            .unwrap_or(vec![0, 0, 8, 8]);
        let bounds = Some(PlayfieldBounds {
            base: 8,
            off_fc: bounds[0],
            off_100: bounds[1],
            off_104: bounds[2],
            off_108: bounds[3],
        });
        terrain.native_cell_identity((1234, -2345));
        let returned = owner
            .validate_repaired_zones(&terrain, (5, 5), |bridge| {
                zones.activate_repaired_bridge(&terrain, bridge, bounds)
            })
            .unwrap();
        zones.retain_repaired_bridge_records(&path, &terrain, owner.endpoint_records());
        assert_eq!(json!(returned), original["returned"], "{}", input["name"]);
        assert_eq!(
            edges(zones.hierarchy.as_ref().unwrap()),
            original["edges"],
            "{}",
            input["name"]
        );
        assert_eq!(
            json!(
                owner
                    .endpoint_records()
                    .iter()
                    .map(|r| r.active)
                    .collect::<Vec<_>>()
            ),
            original["active"]
        );
        let base = zones.base_topology.as_ref().unwrap();
        assert_eq!(
            (base.zone_ids.clone(), base.raw_zone_ids_by_row.clone()),
            before
        );
        assert_eq!(
            json!(terrain.native_cell_coord(crate::map::cell_index::NativeCellIdentity::Dummy)),
            original["dummy"]
        );
        if redirect_case {
            assert!(!returned);
            assert_eq!(
                zones.get_zone_id_native((7, 5), MovementZone::Normal, true),
                Some(2)
            );
            assert_eq!(
                zones.get_path_zone_id_native(&terrain, (7, 5), MovementZone::Normal, true),
                Some(2)
            );
        }
        count += 1;
    }
    assert_eq!(count, 9);
}
