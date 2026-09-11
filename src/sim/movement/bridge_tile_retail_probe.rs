//! Production-loader input witness for native middle-ramp terrain replacement.
//! Loading is not emulated here; a native harness must establish damage parity.

#[test]
#[ignore = "requires active retail assets; writes a local native-oracle input receipt"]
fn retail_high_bridge_middle_tile_inputs() {
    use serde_json::json;
    let retail = super::retail_dir().expect("configured active retail install");
    let map_name =
        std::env::var("VERA20K_BRIDGE_TILE_MAP").unwrap_or_else(|_| "xmp34u4.map".into());
    let scenario = crate::headless_scenario::load(&retail, &map_name, super::SEED)
        .unwrap_or_else(|error| panic!("load {map_name}: {error}"));
    let sim = scenario.sim();
    let terrain = sim.resolved_terrain.as_ref().expect("resolved terrain");
    let bridges = sim.bridge_state.as_ref().expect("bridge state");
    let keys = terrain
        .high_bridge_rim_tiles()
        .expect("retail theater keys");
    let cells: Vec<_> = terrain
        .iter()
        .map(|cell| {
            json!([
                cell.rx,
                cell.ry,
                cell.final_tile_index,
                cell.final_sub_tile,
                cell.bridge_facts.raw_flags,
                cell.bridge_facts.overlay_id,
                cell.bridge_facts.state_byte,
                cell.bridge_facts
                    .native_anchor
                    .map(|anchor| terrain.native_cell_coord(anchor)),
                cell.level,
                cell.yr_cell_land_type
            ])
        })
        .collect();
    let middle: Vec<_> = terrain
        .iter()
        .filter(|cell| {
            let relative = cell
                .final_tile_index
                .wrapping_sub(keys.base)
                .wrapping_add(1);
            keys.middle
                .iter()
                .any(|middle| (0..=4).any(|variant| relative == middle + variant))
        })
        .map(|cell| {
            json!({"coord": [cell.rx, cell.ry], "tile": cell.final_tile_index,
        "subtile": cell.final_sub_tile, "level": cell.level, "slope": cell.slope_type,
        "land": cell.yr_cell_land_type, "zone": cell.zone_type,
        "has_deck": cell.has_bridge_deck, "walkable": cell.bridge_walkable,
        "runtime": bridges.cell(cell.rx, cell.ry)})
        })
        .collect();
    assert!(
        !middle.is_empty(),
        "selected map must establish a live middle-ramp witness"
    );
    let result = json!({"map": map_name, "theater": scenario.map.header.theater,
        "seed": super::SEED,
        "size": [scenario.map.header.width, scenario.map.header.height],
        "local_size": [scenario.map.header.local_left, scenario.map.header.local_top,
            scenario.map.header.local_width, scenario.map.header.local_height],
        "bridge_base": keys.base,
        "rim_keys": {"BridgeTopLeft1": keys.top_left[0], "BridgeTopLeft2": keys.top_left[1],
            "BridgeBottomRight1": keys.bottom_right[0], "BridgeBottomRight2": keys.bottom_right[1],
            "BridgeTopRight1": keys.top_right[0], "BridgeTopRight2": keys.top_right[1],
            "BridgeBottomLeft1": keys.bottom_left[0], "BridgeBottomLeft2": keys.bottom_left[1],
            "BridgeMiddle1": keys.middle[0], "BridgeMiddle2": keys.middle[1]},
        "records": bridges.endpoint_records(), "cells": cells, "middle_cells": middle,
        "cell_fields": ["x", "y", "tile", "subtile", "full_bridge_flags", "overlay_id",
            "overlay_state", "literal_anchor_current_coord", "level", "land"],
        "scope": "Current Rust retail-loader scalars and runtime middle-ramp projection; native loading and damage excluded"});
    let output = std::env::var("VERA20K_BRIDGE_TILE_OUTPUT")
        .unwrap_or_else(|_| ".local/bridge-middle-retail-inputs.json".into());
    std::fs::write(&output, serde_json::to_vec(&result).unwrap()).unwrap();
    println!(
        "{map_name}: {} cells, {} middle cells, saved {output}",
        cells.len(),
        middle.len()
    );
}
