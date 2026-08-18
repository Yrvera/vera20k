# Bridge Crossing Path Oracle Design

## Goal

Design the smallest reliable gamemd-vs-Rust oracle for one concrete high-bridge Grizzly crossing so bridge movement/pathfinding parity can move from UNCHECKED to literal PASS or FAIL.

## Architecture Context

The bridge crossing evidence spans map loading, pathfinding, cell-entry legality, and runtime movement. Rust already has the right broad surfaces, but no same-cell oracle ties them to active gamemd values.

Map-load bridge facts live in `src/map/bridge_facts.rs` and `src/map/resolved_terrain.rs`. `BridgeCellFacts` preserves raw stamp flags, state byte, overlay id, family, direction, and anchor relation. `ResolvedTerrainCell` carries `tile id/subtile`, `level`, `slope_type`, bridge fields, and derived deck level.

Pathfinding uses `src/sim/pathfinding/core.rs`. `PathGrid` stores ground and bridge walkability, structural bridge flag, transition flag, ground level, deck level, and slope byte. `astar_search` carries a node height, chooses ground/bridge arrays per cell, calls `check_bridge_traversal`, builds `CanEnterLayerContext`, applies terrain cost, soft blocker cost, search marker cost, and direction tie-break.

Cell-entry layer selection is represented in `src/sim/pathfinding/cell_entry.rs` by `CanEnterLayerContext`, but current module wording still marks bridge legality as an approximation of gamemd's two-pass split.

Runtime movement uses `src/sim/movement/movement_bridge.rs`, `movement_step.rs`, and `movement_tick.rs`. `loco.layer`, `on_bridge`, and `bridge_occupancy` are intentionally decoupled, which is required for bridge ramp boundary ticks.

The committed theater numeric cliff/ramp classification fix (`c6bee17`) is a prerequisite source of correct underlying tile facts. This oracle must not reopen or block that commit.

## Impact Analysis

The oracle should add diagnostics and comparison surfaces only. It should not change bridge movement behavior, pathfinding semantics, terrain classification, or runtime occupancy rules until a later fix plan consumes concrete FAIL output.

Likely implementation touchpoints:

| Area | Path | Role |
|---|---|---|
| Map-load dump | `src/map/resolved_terrain.rs`, `src/map/bridge_facts.rs` | Export Rust cell facts for selected route cells |
| Pathfinding dump | `src/sim/pathfinding/core.rs` | Export A* candidate-step layer/height/cost decisions |
| Cell-entry dump | `src/sim/pathfinding/cell_entry.rs` | Export terrain/object-list/occupancy-bit layer selections and return code |
| Runtime movement dump | `src/sim/movement/movement_step.rs`, `movement_tick.rs`, `movement_bridge.rs` | Export per-boundary/tick layer, on-bridge, occupancy, and visible Z |
| Comparator | likely `tools/` or test support | Compare gamemd JSON/CSV against Rust JSON/CSV |
| Fixture data | `docs/research/traces/` or `tests/fixtures/` | Store selected route metadata and captured oracle files |

Risk areas:

- Instrumentation changing deterministic behavior or allocation/order in sim code.
- Treating a Rust-only trace as parity evidence.
- Comparing route summaries instead of per-candidate values.
- Letting object-list and occupancy-bit layer choices collapse into one field.
- Accidental broad bridge refactors while adding capture points.

## Chosen Approach

Use a fixture-driven dual-trace oracle:

1. Select one exact stock or minimal high-bridge crossing with a Grizzly Tank.
2. Capture gamemd map-load cell facts and per-step A*/`Can_Enter_Cell` values for that route.
3. Capture Rust map-load, pathfinding, cell-entry, and runtime movement values for the same route.
4. Compare literal numeric fields by cell, step, and tick.

This is the smallest reliable route because it proves or disproves one concrete bridge crossing without requiring a general bridge rewrite. It also prevents overclaiming: missing fields stay UNCHECKED, mismatched fields become FAIL, and exact matches become PASS for only the scoped route.

## Tiny-Detail Ledger

- `SetBridgeDirection` writes independent bridge flags and metadata; `0x80`, `0x100`, `0x200`, `0x400`, state byte, anchor relation, family, and direction must remain separate. Source: `docs/research/bridges/01-assets-map-load-overlay/BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md`.
- Intact high bridge slots differ. Forward2 can have `0x100` without `0x200`, so structural bridge is not enough to allow every deck move. Source: `docs/research/bridges/03-traversal-pathfinding-entry/HIGH_BRIDGE_EDGE_LANE_TRAVERSAL_REINVESTIGATION_GHIDRA_REPORT.md`.
- `UnitClass::Can_Enter_Cell @ 0x0073F0A0` is the live YR vehicle/unit A* legality function through vtable `+0x1AC`. Source: `docs/research/bridges/03-traversal-pathfinding-entry/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`.
- `CheckBridgeTraversal @ 0x004D9C60` is the bridge sub-check through vtable `+0x1B0`; it returns only `0` or `7` and may update path height and bridge-list state. Source: `docs/research/bridges/03-traversal-pathfinding-entry/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`.
- Pre-`CheckBridgeTraversal` object-list selection uses candidate `Flags&0x100`, incoming `path_height`, signed `Level`, and `abs(path_height - Level) <= 1`; `path_height == -1` selects the bridge list for bridge cells. Source: `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_TWO_PASS_CAN_ENTER_CELL_SPLIT_GHIDRA_REPORT.md`.
- Initial occupancy snapshot always reads ground occupancy `+0x124` / `+0x54`; bridge occupancy `+0x128` / `+0x58` can overwrite only after `CheckBridgeTraversal` when `path_height == cell.Level + 4`. Source: `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_TWO_PASS_CAN_ENTER_CELL_SPLIT_GHIDRA_REPORT.md`.
- Object-list layer and occupancy-bit layer can disagree because the object-list byte is not recomputed after traversal. Source: `docs/research/bridges/02-cell-state-layering-zones/BRIDGE_OCCUPANCY_OBJECT_LISTS_GHIDRA_REPORT.md`.
- `CellClass+0x11B` `Level` is signed-byte math; `CellClass+0x11C` `SlopeIndex` gates diff-1 traversal. Source: `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`.
- Normal A* passes explicit current-node cell to `Can_Enter_Cell`; predecessor fallback is for callers that pass no parent. Source: `docs/research/bridges/03-traversal-pathfinding-entry/BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`.
- A* uses separate ground and bridge closed/cost arrays; selected layer is tied to current path height and candidate bridge/level facts. Source: `docs/research/bridges/00-system-models/BRIDGE_DUAL_LAYER_ASTAR_SYSTEM_MODEL_SYNTHESIS.md`.
- Runtime `loco.layer`, `on_bridge`, `bridge_occupancy`, and visible Z must be compared separately; Rust already decouples them. Source: `src/sim/movement/movement_bridge.rs`.

## Design

### Components

**Scenario manifest**

One small manifest names:

- fixture/map name;
- theater;
- `[MTNK]` Grizzly identity;
- owning house;
- start cell;
- target cell;
- required route cell list;
- adjacent cells needed for bridge stamping and flank/transition context.

**Gamemd trace**

The gamemd capture must be read-only. It can come from debugger/runtime logging or an external capture process, but it must not require Ghidra mutations. The trace must record raw values exactly as gamemd stores and passes them.

**Rust trace**

Rust trace output should be gated by an explicit fixture/test/diagnostic path and should not run in normal gameplay. It should dump only the selected route and relevant candidate expansions, not the full map or every path search.

**Comparator**

The comparator reads gamemd and Rust traces and emits a field-by-field verdict:

- PASS when both sides have the field and the literal value matches;
- FAIL when both sides have the field and values differ;
- UNCHECKED when either side is missing the field.

### Interfaces / Contracts

Trace output should be structured JSON or CSV with stable numeric fields. JSON is preferred for nested per-step details.

Top-level groups:

- `scenario`
- `cell_facts`
- `astar_steps`
- `runtime_ticks`

Required `cell_facts` fields:

- `rx`, `ry`
- tile id, subtile, slope byte, level byte, land type
- `bridge_set_member`, `wood_bridge_set_member`
- flags `0x80`, `0x100`, `0x200`, `0x400`
- raw bridge state byte
- anchor identity
- stamp family
- direction
- deck level

Required A* / cell-entry fields:

- current cell, candidate cell, direction
- incoming path height
- candidate closed-list layer
- `CheckBridgeTraversal` result and resulting path height
- `UnitClass::Can_Enter_Cell` return code
- selected object-list layer
- selected occupancy-bit layer
- selected object list / occupancy bits
- edge cost
- carried path height

Required Rust-equivalent fields:

- `ResolvedTerrainCell` bridge facts
- `PathGrid` ground/bridge walkability, structural, transition, levels
- `TerrainCostGrid` output
- A* node layer, candidate layer, `compute_neighbor_height`
- Rust `check_bridge_traversal` result
- `CanEnterLayerContext`
- final path cells/layers/heights

Required runtime fields:

- tick number
- current cell
- next cell
- active `loco.layer`
- `on_bridge`
- bridge occupancy before/after
- occupancy layer before/after
- visible Z / deck level

### Data Flow

1. Select fixture and route.
2. Capture gamemd cell facts at map load.
3. Capture gamemd A* and `Can_Enter_Cell` values for the route.
4. Capture gamemd runtime movement ticks for the same order.
5. Run Rust with the same fixture and route.
6. Dump Rust map/path/runtime trace.
7. Compare literal fields.
8. Save verdict report under `docs/research/traces/`.

### Error Handling

- Missing scenario identity is a hard comparator error.
- Missing field inside a valid trace is UNCHECKED for that field group, not FAIL.
- Extra fields are ignored unless a schema version marks them required.
- Route mismatch is FAIL if both sides produce a route and the route differs; it is UNCHECKED if one side lacks route data.

### Testing Strategy

Start with an empty-occupancy crossing. This isolates bridge legality from object classification.

After the empty crossing is complete, add optional blocker variants only if needed:

- one ground occupant under the bridge;
- one bridge occupant on the deck.

Focused Rust checks after implementation should include:

```powershell
cargo test -q theater
cargo test -q resolved_terrain
cargo test -q terrain_cost
cargo test -q pathfinding::core
```

The final oracle test should run only after gamemd fixture data exists. It must compare captured values, not regenerate expected gamemd behavior from Rust logic.

## Architectural Decisions

- Use existing Rust structures and capture points instead of adding a parallel oracle path.
- Keep gamemd evidence and Rust evidence in separate files so the comparator cannot silently derive one side from the other.
- Keep the first fixture to one high-bridge crossing and one Grizzly Tank.
- Do not include low bridges, collapse/repair, CABHUT, bridge flank costs, zone-precheck retry producer, or stock Carville detour assertions in this first oracle unless the selected route requires a field already in the ledger.

## Alternatives Considered

**Rust-only synthetic tests**

Rejected as the primary oracle because they cannot move parity from UNCHECKED to PASS/FAIL without gamemd same-cell values.

**Broad trace swarm**

Rejected because prior trace work already found the blocker: no concrete oracle values exist. A swarm without a fixture oracle would likely return UNCHECKED again.

**Full bridge pathfinding rewrite first**

Rejected because this task is evidentiary. Implementation should follow concrete FAILs, not precede them.

