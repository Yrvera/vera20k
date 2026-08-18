# PathGrid Water/Pier Cell Legality Re-Swarm Reconciliation

Date: 2026-05-27

Scope: reconcile the five-slot re-swarm on units driving onto water / outside pier-like cells. This is research only; no Rust code was changed.

## Verdict

The issue area is confirmed: this is a shared pathfinding and cell-entry legality drift, not a Chrono Miner-only mission bug.

The strongest verified mismatch is that current Rust still lets multiple call paths treat `PathGrid::is_walkable()` / `is_any_layer_walkable()` as final ground-unit passability, while active YR rechecks mover-specific cell-entry legality through `Can_Enter_Cell` or `CellRect::CheckPassability`.

Because Rust `PathGrid::from_resolved_terrain_with_bridges` marks water as ground-walkable, any helper, goal redirect, A* fallback, or smoother that consumes the coarse grid can choose water/pier-adjacent cells that gamemd rejects for ordinary ground units.

## Parent Spot Checks

The parent rechecked the key Ghidra anchors after the subagent reports:

- `Path_smooth_single_segment @ 0x0042B420` calls mover vtable `+0x1AC` with candidate `CellClass`, direction, height, `0`, `1`; nonzero return rejects the shortcut. It also rejects `CellClass+0x140 & 0x40000`.
- `Path_Reroute_Straight_Line @ 0x0042BE20` calls mover vtable `+0x1AC` for each reroute candidate, rejects nonzero return, rejects `CellClass+0x140 & 0x40000`, and retries once with the two direction runs swapped.
- `AStar_main_loop @ 0x00429A90` calls mover vtable `+0x1AC` during neighbor expansion before accepting normal cells, and calls smoothing/optimization after path reconstruction.
- `FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20` calls `CellRect__CheckPassability` for candidates and optional `CellRect__CheckOccupancy`, rather than a boolean pathgrid.
- `UnitClass__Can_Enter_Cell @ 0x0073F0A0` includes bridge/tube, locomotor passability, overlay/object, and final `SpeedType x LandType` zero-speed rejection.
- `CellClass__RecalcZoneType @ 0x00483C80` writes water `LandType == 2 -> ZoneType 4` and beach `LandType == 6 -> ZoneType 3`.

These checks support the shared claim that a coarse pathgrid boolean is not a valid replacement for native cell-entry legality.

## Slot Results

| Slot | Report | Status | Key result |
|---|---|---|---|
| 1 | `CAN_ENTER_CELL_WATER_PIER_LEGALITY_RESWARM_20260527.md` | Complete from prior verified docs; fresh Ghidra was unavailable inside slot | Non-water ground entry into water is rejected by MovementZone/ZoneType and `Can_Enter_Cell`; Rust has coarse `PathGrid` consumers. |
| 2 | `ASTAR_WATER_PIER_NEIGHBOR_LEGALITY_RESWARM_20260527.md` | Complete from prior verified docs/source; fresh Ghidra was unavailable inside slot | A* legality/cost is not `PathGrid` plus terrain speed weighting. Rust skips reduced-zone precheck for live rows such as `Crusher` and can run without terrain costs. |
| 3 | `PATH_SMOOTHING_WATER_PIER_LEGALITY_RESWARM_20260527.md` | Complete | Active YR smoothing and reroute validate candidate shortcut cells through mover `Can_Enter_Cell`; Rust smoothing uses `PathGrid` closures. |
| 4 | `PIER_BRIDGE_WATER_CLASSIFICATION_RESWARM_20260527.md` | Complete for mechanism; per-TMP WaterBridge bytes remain unchecked | No separate binary "pier" movement class was found. Cells classify as water, beach, high bridge, low bridge/tube, or ordinary terrain/object state. |
| 5 | `HELPER_PASSABLE_CELL_CONTRACTS_RESWARM_20260527.md` | Partial only for one exact `Find_Path -> FNPC` argument row | Native helper selection uses `Can_Enter_Cell` or `FNPC -> CellRect::CheckPassability`; Rust helper sites use coarse `PathGrid` in miner staging, scatter, and goal redirection. |

## Agreed Findings

### P0 DRIFT - Water is ground-walkable in `PathGrid`

Rust currently lets water cells be `ground_walkable` in `PathGrid`, relying on later mover-specific layers to reject them. That split is unsafe because many current consumers do not apply those later layers.

Affected surfaces:

- `src/sim/pathfinding/core.rs`
- `src/sim/movement/movement_path.rs`
- `src/sim/miner/miner_dock_sequence.rs`
- `src/sim/miner/miner_system.rs`
- `src/sim/movement/scatter.rs`
- `src/sim/movement/bump_crush.rs`

### P0 DRIFT - Smoothing uses the wrong legality boundary

Gamemd smoothing calls `Can_Enter_Cell` for candidate shortcut cells. Rust smoothing receives closures based on `PathGrid::is_walkable` / `is_walkable_on_layer`, so it can create water shortcuts after A*.

### P0 DRIFT - Helper and goal redirection use the wrong contract

Native nearby-passable helpers use caller-specific `CheckPassability` / `Can_Enter_Cell` inputs. Rust uses generic nearest-walkable / any-layer logic for non-water ground movers in several places.

### P1 DRIFT - Reduced-zone precheck coverage is too narrow

Rust currently disables reduced-zone precheck for several live `MovementZone` rows, including `Crusher`, which is relevant for miners. Existing verified docs indicate the native matrix is rowed by `MovementZone`, not a small subset of rows.

### P1 DRIFT - A* cost semantics differ

Gamemd A* edge cost is based on `Can_Enter_Cell` return code and bridge/marker/direction costs. Rust also uses terrain speed percentage as route weight when a `TerrainCostGrid` is present. This may not directly cause water driving, but it is exact-mechanism drift and can alter pier/shore routing choices.

### P1/UNCHECKED - WaterBridge and exact pier repro cell

No separate binary pier movement class was found. `WaterBridge` is verified as a LAT/visual exemption, not as a movement class by name. The exact symptom still needs a concrete map/cell trace to know whether the offending cell is true water, beach, high bridge, low bridge/tube, WaterBridge TMP data, or a Rust classification mistake.

## Contradictions

No cross-slot contradiction was found on the core issue. The slots agree that native behavior is mover-specific legality, not coarse pathgrid passability.

The only evidence-quality split is that slots 1 and 2 lacked fresh Ghidra MCP access inside their subagent sessions. Parent-side Ghidra spot checks and slots 3-5 cover the key anchors well enough to keep the main verdict as DRIFT, while still marking exact `Find_Path -> FNPC` stack arguments and concrete pier-cell classification as open.

## Implementation Handoff

Do not fix this in miner logic alone.

The next implementation contract should require a native-shaped cell-entry/passability layer above `PathGrid` and route these consumers through it:

1. A* neighbor legality and blocked-goal fallback.
2. Move goal validation and redirection.
3. Path smoothing and straight-line reroute.
4. Scatter and bump/crush candidate selection.
5. Miner/refinery staging and exit helpers.

The evaluator must keep these mechanisms separate:

- `MovementZone x reduced ZoneType` for zone reachability.
- `SpeedType x LandType` zero/nonzero terrain entry legality.
- Bridge/tube height and layer semantics.
- Dynamic occupant/object-list return codes.
- Caller-specific FNPC flags for rectangle size, bridge filtering, overlay rejection, object safety, and final occupancy.

## Acceptance Targets

- A `MovementZone::Crusher` ground unit cannot path, redirect, smooth, scatter, or stage onto `ZoneType 4` water even when `PathGrid` stores water metadata.
- A pier-edge path that A* keeps on land cannot be smoothed across adjacent water.
- Generic move-goal redirection does not select an "any layer walkable" water/bridge-adjacent cell for a non-water mover.
- Unit scatter skips adjacent water if `Can_Enter_Cell` would return impassable.
- Chrono Miner far-return staging uses FNPC-style passability rather than `PathGrid::is_walkable`.
- A concrete repro cell logs final tile/subtile, TMP land byte, `LandType`, reduced `ZoneType`, bridge flags, tube index, overlay, `PathGrid` fields, MovementZone, SpeedType, and entry result.

## Follow-Up Research

1. Dump retail TMP terrain bytes for WaterBridge tiles in relevant theaters.
2. Capture one concrete map/cell repro trace for the player-visible pier symptom.
3. Walk the exact assembly push row for `FootClass::Find_Path @ 0x004D3920 -> FootClass__Find_Nearby_Passable_Cell @ 0x0056DC20`.
4. Convert the confirmed deltas into an implementation contract before patching shared movement code.
