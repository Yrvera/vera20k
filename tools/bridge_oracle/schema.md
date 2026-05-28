# Bridge Crossing Oracle Trace Schema

Schema version: `1`

The oracle compares two separate JSON traces: one captured from active
`gamemd.exe`, one captured from Rust. Missing required fields are `UNCHECKED`;
they are never inferred from the other side.

Top-level fields:

- `schema_version`: integer, currently `1`.
- `scenario`: fixture identity and activation proof.
- `cell_facts`: route/window map-load cell facts.
- `astar_steps`: A* neighbor candidate rows, including rejected candidates.
- `runtime_ticks`: runtime movement layer/on-bridge/occupancy rows.

Comparator verdict rules:

- `PASS`: both values are present and literally equal.
- `FAIL`: both values are present and unequal.
- `UNCHECKED`: either value is missing.

A* row matching contract:

- Primary key: `(search_id, expansion_index)`.
- Validation tuple: `(current_cell, candidate_cell, direction, incoming_path_height)`.
- If expansion order diverges, report the first divergent expansion before using
  overlapping validation tuples as secondary evidence.
- If either side lacks `astar_steps`, the expansion-order group is `UNCHECKED`.

Candidate coverage:

- Emit every A* neighbor candidate inside the selected route window, including
  rejected candidates.
- Do not reduce the trace to the final accepted route.

Required scenario fields:

- `id`, `map`, `theater`, `unit`, `house`, `start_cell`, `target_cell`,
  `route_window`, `bridge_overlay_anchors`.
- Activation proof on gamemd traces: `unit_pointer`, `unit_type`, `house`,
  `issued_order_id`, `issued_order_tick`, `pathfinder_search_id`,
  `callsite_category`.

Required cell fact fields:

- `rx`, `ry`, source/final tile id and subtile, `level`, `slope_type`,
  `land_type`, `yr_cell_land_type`, `bridge_set_member`,
  `wood_bridge_set_member`, bridge raw flags, booleans for `0x80`, `0x100`,
  `0x200`, `0x400`, `state_byte`, `overlay_id`, `family`, `direction`,
  `anchor`, `bridge_deck_level`, `has_bridge_deck`, `bridge_walkable`,
  `bridge_transition`.

Required runtime tick fields:

- `tick`, `current_cell`, `next_cell`, `loco_layer_before`,
  `next_path_layer`, `on_bridge_before`, `on_bridge_after`, `bridge_update`,
  `bridge_occupancy_before`, `bridge_occupancy_after`, `visible_z_after`.
