# PathGrid / Cell Passability Disparity Scan

**Date:** 2026-05-27  
**Investigation Mode:** disparity-scan  
**Claimed Scope:** shared Rust cell/path legality surfaces that can let ground units path or drive onto water-looking/pier-adjacent cells: `ResolvedTerrainCell`, `PathGrid`, `TerrainCostGrid`, zone precheck, A*, path smoothing, movement issue helpers, scatter, and miner staging helpers.  
**Non-Scope:** fresh Ghidra decompilation, exact bridge/pier tile family audit, implementation patches, runtime capture.  
**Confidence:** High for current Rust source mismatches; High for verified binary zone/passability contracts cited from existing research; Medium for player-visible pier ranking because no map-specific repro cell was traced in this pass.  
**Active in YR:** Yes for standard ground units using `MovementZone=Normal/Crusher/Destroyer/...`, water/shore cells, bridge/pier cells, and standard movement/pathing.

## 1. Overview

The most likely shared source of "units drive outside the pier / onto water" is not Chrono Miner-specific logic. Current Rust has a split legality model:

1. `PathGrid` marks water as `ground_walkable`.
2. Some movement paths rely on `TerrainCostGrid` to reject water for non-water SpeedTypes.
3. Other paths still consult `PathGrid::is_walkable()` or `is_any_layer_walkable()` without a mover-specific passability/cost predicate.
4. Path smoothing also uses `PathGrid`-only walkability for non-water movers.

This means any caller that omits `TerrainCostGrid`, omits `ResolvedTerrainGrid`, or uses `PathGrid` for candidate selection can accept a water or visually-water/pier cell before the full movement legality gate has a chance to reject it.

In gamemd, the core pathing legality is not a global "water is walkable, cost fixes it later" boolean. Cells are classified by `CellClass::RecalcZoneType`, checked through `ZonePassabilityMatrix[MovementZone][ZoneType]`, and candidate passability paths use caller-specific `Can_Enter_Cell` / `CellRect__CheckPassability` inputs.

## 2. Verified Binary Baseline

| Behavior | Evidence | Active in YR |
|---|---|---|
| `CellClass::RecalcZoneType` classifies water as reduced `ZoneType=4`, beach as `3`, impassable as `6`, and default land as `0`. | `docs/research/CELLCLASS_RECALCZONE_TYPE_00483C80_GHIDRA_REPORT.md` sections 1 and 3. | Yes |
| Movement legality is matrix-based: `ZonePassabilityMatrix[MovementZone][ZoneType]` must equal `1`. Normal/Crusher/Destroyer rows block Water. Water row only passes Water. WaterBeach passes Beach and Water. | `docs/research/NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md` sections 1-2. | Yes |
| `SpeedType` controls terrain speed table lookup; `MovementZone` controls zone passability. | `docs/research/NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md` sections 3-4. | Yes |
| `CellRect__CheckPassability` threads SpeedType, required zone, MovementZone, height/layer, bridge-aware flag, and overlay rejection through `CellClass__CheckCellPassability`; it is not equivalent to `PathGrid::is_walkable`. | `docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md` sections 3.1 and 6. | Yes |
| Sea TMP terrain byte `9` becomes binary LandType `2` Water; Rust's local 8-column `LandType::Water = 4` is a compatibility remap, not the binary LandType value. | `docs/research/SEA_TILES_GHIDRA_REPORT.md` sections 2-3. | Yes |

## 3. Current Rust Surfaces

| Surface | Current behavior | Evidence |
|---|---|---|
| `PathGrid::from_resolved_terrain_with_bridges` | Sets `ground_walkable` true for water via `!cell.ground_walk_blocked || cell.is_water`. | `src/sim/pathfinding/core.rs:1811` |
| `PathGrid` test suite | Explicitly asserts "Water is PathGrid-walkable". | `src/sim/pathfinding/core_tests.rs:948` |
| `TerrainCostGrid` | Blocks or slows per `SpeedType`, and gives bridge decks normal cost. | `src/sim/pathfinding/terrain_cost.rs:48` |
| A* terrain cost gate | For non-water movers, zero terrain cost rejects neighbors only when a cost grid is provided. Without a cost grid, terrain cost defaults to `100`. | `src/sim/pathfinding/core.rs:1221` |
| Goal resolution | For non-water movers, `is_move_goal_walkable` uses `grid.is_any_layer_walkable`, not TerrainCostGrid or MovementZone matrix. | `src/sim/movement/movement_path.rs:60` |
| Nearest-goal fallback | For non-water movers, `nearest_move_goal` delegates to `nearest_walkable_any_layer`, again without terrain-cost or MovementZone passability. | `src/sim/movement/movement_path.rs:87` |
| Path smoothing | For non-water movers, `smooth_walkable` uses `grid.is_walkable`; layered smoothing uses `grid.is_walkable_on_layer`. Neither checks terrain cost. | `src/sim/movement/movement_path.rs:242`, `src/sim/movement/movement_path.rs:304` |
| Miner queue/staging helper | `issue_move_if_idle` calls `movement::issue_move_command` with no terrain costs, no resolved terrain, no entity blocks. | `src/sim/miner/miner_system.rs:1447` |
| Miner nearby passable helper | `is_exit_cell_passable` uses `grid.is_walkable` and optional occupancy only. | `src/sim/miner/miner_dock_sequence.rs:268` |
| Scatter candidate selection | Both idle scatter and spiral scatter candidate selection use `PathGrid::is_walkable` before issuing movement. | `src/sim/movement/scatter.rs:134`, `src/sim/movement/scatter.rs:334` |
| Zone precheck | Reduced-zone precheck is enabled only for `Normal`, `Amphibious`, `Infantry`, and `Fly`; many ground movement zones skip it. | `src/sim/pathfinding/zone_search.rs:62` |

## 4. Disparities

### D1 - `PathGrid` encodes water as ground-walkable

**Verdict:** DRIFT  
**Priority:** P0  
**Frequency:** Any map with water/pier-adjacent cells; triggered when a caller consults `PathGrid` directly or omits cost data.  
**Player-visible effect:** Ground units can accept, smooth through, scatter toward, or stage on a water-looking cell if another gate does not reject it later.

Verified gamemd separates cell classification and mover legality. Water is a reduced zone column and normal ground movement rows block it. Current Rust encodes water as `ground_walkable` and relies on later SpeedType cost checks. That is not an exact mechanism and leaks through direct `PathGrid` callers.

**Current Rust evidence:** `src/sim/pathfinding/core.rs:1811`; test expectation at `src/sim/pathfinding/core_tests.rs:948`.  
**Binary evidence:** `CELLCLASS_RECALCZONE_TYPE_00483C80_GHIDRA_REPORT.md`; `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md`.

### D2 - Goal redirection ignores mover-specific passability for non-water movers

**Verdict:** DRIFT  
**Priority:** P0  
**Frequency:** Every move order whose clicked/desired goal is blocked and searches for a nearby cell; especially near water, piers, bridges, and refinery/structure footprints.  
**Player-visible effect:** A blocked destination can be redirected to a water-looking or bridge-layer cell before A* runs.

`resolve_requested_move_goal` should not decide passability from `is_any_layer_walkable` for ground units. A candidate can be "any-layer walkable" because water is ground-walkable or because the bridge layer is walkable, even if the mover's `MovementZone`/height context would not accept that cell in gamemd.

**Current Rust evidence:** `src/sim/movement/movement_path.rs:60`, `src/sim/movement/movement_path.rs:87`, `src/sim/pathfinding/core.rs:1668`.  
**Binary evidence:** `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md` section 3.1; `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md` sections 1-2.

### D3 - A* can become grid-only when cost data is absent

**Verdict:** DRIFT  
**Priority:** P0  
**Frequency:** Internal/helper moves that call movement without TerrainCostGrid; likely less common than player move orders, but high visibility when triggered.  
**Player-visible effect:** A unit path can be computed through water because A* treats terrain cost as `100` when no cost grid is supplied.

The A* neighbor check has two gates: `is_cell_passable_for_mover` and terrain cost. For non-water movers, `is_cell_passable_for_mover` falls back to `grid.is_walkable`. Since water is grid-walkable, the only water rejection for ordinary ground units is `TerrainCostGrid::cost_at == 0`. When callers omit the cost grid, terrain cost defaults to `100`.

**Current Rust evidence:** `src/sim/pathfinding/core.rs:1392`, `src/sim/pathfinding/core.rs:1221`; omitted-cost call at `src/sim/miner/miner_system.rs:1447`.  
**Binary evidence:** `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md` sections 1-4.

### D4 - Path smoothing can reintroduce water/invalid shortcuts

**Verdict:** DRIFT  
**Priority:** P0  
**Frequency:** Any path that passes near water/pier cells after A* found a valid route.  
**Player-visible effect:** Even if A* initially avoids water through terrain cost, smoothing can delete intermediate points and create a straightened path through a `PathGrid`-walkable water cell.

For non-water movers, the smoothing predicate uses `grid.is_walkable` only. That predicate does not consult `TerrainCostGrid`, reduced `zone_type`, or `MovementZone`. Layered smoothing similarly checks only the selected layer walkability.

**Current Rust evidence:** `src/sim/movement/movement_path.rs:242`, `src/sim/movement/movement_path.rs:304`.  
**Binary evidence:** Existing docs prove movement legality is not plain grid walkability; exact gamemd smoothing/line-of-sight path shortcut parity was not re-opened in this scan, so the precise smoothing mechanism remains UNCHECKED. The current Rust mechanism still DRIFTs because it can admit cells A* would reject by cost/matrix.

### D5 - Reduced-zone precheck is disabled for many live ground movement zones

**Verdict:** DRIFT  
**Priority:** P1  
**Frequency:** Units with `MovementZone=Crusher`, `Destroyer`, `AmphibiousDestroyer`, `AmphibiousCrusher`, `Subterranean`, `InfantryDestroyer`, `Water`, `WaterBeach`, `CrusherAll`. Chrono Miner is `Crusher`, but this is broader than miners.  
**Player-visible effect:** Large-scale zone separation is skipped for many ground movers, so the fallback becomes local A* legality only. Combined with water-as-walkable, this increases the chance of bad pier/water routes.

The verified matrix has rows for all 13 movement zones. Current Rust only allows reduced-zone precheck for a subset, with a TODO for water movers. That is a broad mechanism mismatch, not just an optimization difference.

**Current Rust evidence:** `src/sim/pathfinding/zone_search.rs:62`.  
**Binary evidence:** `NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md` sections 2-3; `CELLCLASS_RECALCZONE_TYPE_00483C80_GHIDRA_REPORT.md` section 5.

### D6 - Candidate helpers use `PathGrid::is_walkable` as "passable"

**Verdict:** DRIFT  
**Priority:** P1  
**Frequency:** Scatter, miner/refinery staging, production/spawn helpers, and any direct use of `PathGrid::is_walkable` for gameplay placement or movement candidate selection.  
**Player-visible effect:** Units can be ordered, scattered, or staged toward cells that are only grid-walkable, not mover-passable.

The concrete surfaces found in this scan are miner staging and scatter. Other `rg` hits should be audited before patching because some are render/effect or building-placement contexts with different rules.

**Current Rust evidence:** `src/sim/miner/miner_dock_sequence.rs:268`, `src/sim/movement/scatter.rs:134`, `src/sim/movement/scatter.rs:334`.  
**Binary evidence:** `CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md` section 9: do not replace caller-specific passability with static `PathGrid::is_walkable`.

### D7 - Bridge/pier layer acceptance is mixed with ground goal acceptance

**Verdict:** UNCHECKED -> likely DRIFT  
**Priority:** P1  
**Frequency:** Maps with bridge/pier/water bridge tiles or overlay bridge metadata near intended ground destinations.  
**Player-visible effect:** A cell that visually belongs to a pier/water edge may be accepted because it is bridge-layer walkable or because a bridge deck gives normal terrain cost.

Current Rust uses `is_any_layer_walkable` for non-water goal redirection and treats bridge decks as normal cost in `TerrainCostGrid`. That is probably correct for many high-bridge deck cells, but the exact gamemd distinction between high bridge, low bridge/tunnel, WaterBridge, shore/pier-like tiles, and ground entry requires a focused bridge/pier tile audit.

**Current Rust evidence:** `src/sim/movement/movement_path.rs:60`, `src/sim/pathfinding/terrain_cost.rs:48`, `src/map/resolved_terrain.rs:600`.  
**Existing related evidence:** `docs/research/WATER_SHORE_EDGE_TRANSITIONS_GHIDRA_REPORT.md`; bridge research under `docs/research/bridges/`.

## 5. Highest-Risk Call Chains

1. **Player or AI move near water**
   `Command::Move` -> `issue_move_command_with_layered` -> `resolve_requested_move_goal` -> `is_any_layer_walkable` -> possible water/bridge-layer target -> A* with cost if available -> smoothing with `grid.is_walkable`.

2. **Internal miner/refinery staging**
   `issue_move_if_idle` -> `issue_move_command` -> no terrain costs/resolved terrain -> A* defaults terrain cost to `100` -> water accepted if `PathGrid` says walkable.

3. **Scatter**
   Candidate selection checks `PathGrid::is_walkable` before issuing movement. The issued move may have a cost grid, but the selected destination can already be a water/pier cell and some calls pass `resolved_terrain=None`.

4. **Blocked repath**
   Repath uses the same goal redirection and smoothing predicates. If a unit is blocked near a pier or refinery edge, the recovery path can select a bad nearby cell.

## 6. Recommended Fix Direction

Do not patch individual miner or scatter symptoms first. The correct boundary should be a shared mover-specific legality predicate that takes:

- `MovementZone`
- `SpeedType`
- current/target layer
- resolved `zone_type` / land type
- terrain speed table result
- bridge transition context
- overlay/object blockers
- occupancy/soft blocker context where the caller needs it

Then use that predicate consistently in:

- goal acceptance and nearest-goal fallback
- A* neighbor expansion
- path smoothing
- scatter/staging candidate selection
- per-step movement validation

Short-term containment options:

1. Stop making water `PathGrid::ground_walkable` for ground layer, and add a separate water/ship legality path.
2. Or keep the current broad grid, but make `PathGrid::is_walkable` impossible to call for mover legality; require a mover profile API.

The second option is less likely to break bridge/naval code abruptly, but it is a larger API cleanup.

## 7. Acceptance Tests To Add

| Test | Expected result |
|---|---|
| `normal_vehicle_cannot_goal_redirect_to_water_cell` | A Normal/Crusher/Destroyer unit near a water cell does not accept water as direct or nearest fallback goal. |
| `issue_move_without_cost_grid_does_not_cross_water_for_ground_mover` | Internal move helpers cannot path a Track/Crusher unit through water even when no `TerrainCostGrid` is supplied. |
| `path_smoothing_preserves_zero_cost_water_detour` | A* path around a water cell is not smoothed into a line crossing that water cell. |
| `scatter_does_not_pick_water_for_ground_unit` | Scatter candidate search rejects water for normal ground units. |
| `bridge_layer_goal_not_accepted_without_valid_transition` | A ground unit does not redirect to a bridge-layer-only cell unless bridge transition rules admit it. |
| `water_mover_still_paths_on_water_after_pathgrid_fix` | Ships continue to path through ZoneType Water and reject land. |
| `amphibious_units_accept_beach_and_water_but_normal_units_do_not` | Matrix rows for Amphibious/WaterBeach vs Normal are preserved. |

## 8. Suggested Follow-Up Research

1. `/re-swarm pathgrid CellClass Can_Enter_Cell water pier bridge movement legality`
   - Split by `Can_Enter_Cell`, A* neighbor legality, path smoothing/straight-line reroute, bridge/waterbridge tile classification, and movement-zone precheck.
2. Focused bridge/pier disparity scan:
   - Determine whether the reported "outside pier" cell is a high bridge deck, low bridge/tunnel, water bridge, shore piece, or plain water TMP cell in `ResolvedTerrainCell`.
3. Implementation contract:
   - "Ground mover cannot enter water/pier-adjacent cells unless gamemd matrix/bridge rules allow it."

## 9. Sources

- `docs/research/CELLCLASS_RECALCZONE_TYPE_00483C80_GHIDRA_REPORT.md`
- `docs/research/NAVAL_ZONE_LEGALITY_GHIDRA_REPORT.md`
- `docs/research/CELLRECT_PASSABILITY_OCCUPANCY_VALIDATORS_GHIDRA_REPORT.md`
- `docs/research/SEA_TILES_GHIDRA_REPORT.md`
- `docs/research/WATER_SHORE_EDGE_TRANSITIONS_GHIDRA_REPORT.md`
- `src/sim/pathfinding/core.rs`
- `src/sim/pathfinding/terrain_cost.rs`
- `src/sim/pathfinding/zone_search.rs`
- `src/sim/movement/movement_path.rs`
- `src/sim/movement/movement_step.rs`
- `src/sim/movement/scatter.rs`
- `src/sim/miner/miner_dock_sequence.rs`
- `src/sim/miner/miner_system.rs`

