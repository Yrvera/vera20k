# Bridge Crossing Path Oracle - Implementation Plan

> **For Codex:** Execute this plan task-by-task. Keep patches small. Do not change bridge movement/pathfinding behavior until the oracle produces a concrete FAIL that a later fix plan targets.

**Goal:** Build a single-fixture gamemd-vs-Rust oracle for one high-bridge Grizzly crossing, capturing map-load bridge facts, A* / `Can_Enter_Cell` step data, Rust equivalents, and runtime movement layer state.

**Design Doc:** [docs/plans/2026-05-26-bridge-crossing-path-oracle-design.md](2026-05-26-bridge-crossing-path-oracle-design.md)

---

## Grounding Summary

- `docs/research/traces/BRIDGE_CROSSING_PATH_ORACLE_REQUIREMENTS_TRACE.md` defines the missing evidence and says the current bridge route slot must remain UNCHECKED until same-cell gamemd and Rust values exist.
- `BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md` verifies high-bridge map-load flags, state byte, anchor relation, family, and direction. The oracle must dump these as independent fields.
- `BRIDGE_TWO_PASS_CAN_ENTER_CELL_SPLIT_GHIDRA_REPORT.md` verifies the two-pass split: pre-`CheckBridgeTraversal` object-list layer and post-`CheckBridgeTraversal` occupancy-bit layer can disagree.
- `UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md` verifies `UnitClass::Can_Enter_Cell @ 0x0073F0A0` is the live YR vehicle A* legality function and `CheckBridgeTraversal @ 0x004D9C60` is the bridge sub-check.
- `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md` verifies signed `Level`, `SlopeIndex`, explicit parent-cell A* binding, and bridge-relevant cell offsets.
- `BRIDGE_DUAL_LAYER_ASTAR_SYSTEM_MODEL_SYNTHESIS.md` verifies dual ground/bridge A* closed/cost arrays and implementation-safe pathfinding facts, while deferring exact stock route assertions.
- Current Rust touchpoints are `src/map/bridge_facts.rs`, `src/map/resolved_terrain.rs`, `src/sim/pathfinding/core.rs`, `src/sim/pathfinding/cell_entry.rs`, `src/sim/pathfinding/terrain_cost.rs`, `src/sim/movement/movement_bridge.rs`, `src/sim/movement/movement_step.rs`, and `src/sim/movement/movement_tick.rs`.

## Non-Goals

- Do not modify bridge behavior, pathfinding legality, terrain classification, or movement state transitions.
- Do not reopen the committed theater numeric cliff/ramp classification fix (`c6bee17`).
- Do not claim general bridge parity from one fixture.
- Do not include low bridge/tube behavior.
- Do not include bridge collapse, repair, CABHUT, or damaged bridge variants.
- Do not make Ghidra mutations: no renames, labels, comments, saves, or writes.
- Do not let missing gamemd values become inferred expected values from Rust.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Create | `docs/plans/2026-05-26-bridge-crossing-path-oracle-design.md` | Approved design reference |
| Create | `docs/plans/2026-05-26-bridge-crossing-path-oracle-plan.md` | This task plan |
| Create/modify | likely `tools/bridge_oracle/` or similar | Trace schema and comparator |
| Modify | `src/map/resolved_terrain.rs` / `src/map/bridge_facts.rs` | Diagnostic Rust cell-facts extraction only |
| Modify | `src/sim/pathfinding/core.rs` | Diagnostic A* candidate-step extraction only |
| Modify | `src/sim/pathfinding/cell_entry.rs` | Diagnostic cell-entry layer/result extraction only |
| Modify | `src/sim/movement/movement_step.rs`, `movement_tick.rs`, `movement_bridge.rs` | Diagnostic runtime tick extraction only |
| Create | `tests/fixtures/bridge_oracle/` or `docs/research/traces/bridge_oracle/` | Scenario manifest and captured traces |
| Create | `docs/research/traces/*BRIDGE_CROSSING*ORACLE*.md` | Final PASS/FAIL/UNCHECKED report |

The exact tool/test paths can be chosen during implementation after checking existing test-fixture conventions.

## Interface Decisions

- Use a versioned trace schema with explicit field names.
- Prefer JSON for nested per-step/tick data.
- Keep gamemd trace and Rust trace separate.
- Comparator output must distinguish PASS, FAIL, and UNCHECKED per field group.
- Missing required data is UNCHECKED, not PASS. A trace that omits a required field
  must not be treated as a complete oracle artifact.
- Numeric values should be emitted in both decimal and hex only where readability helps; comparison uses numeric values.
- Diagnostics must be opt-in through test/tool code, not active during normal gameplay.

## Sim Checklist

- [x] No render/UI/sidebar/audio/net dependency.
- [x] No persistent sim state added by diagnostics.
- [x] No behavior changes before oracle results exist.
- [x] Deterministic capture ordering: sort or emit by route/step/tick order explicitly.
- [x] No floating point in sim logic changes. Comparator can parse numbers as data.
- [x] No ECS or new runtime framework.

## Risks

- **Instrumentation drift:** capture code could accidentally change pathfinding timing/order. Keep capture read-only and behind explicit diagnostics.
- **Field collapse:** object-list layer and occupancy-bit layer must be separate fields.
- **Route mismatch ambiguity:** if gamemd and Rust choose different candidate expansions, comparator must report where the first mismatch occurs instead of summarizing only final route.
- **Incomplete gamemd capture:** missing map-load bridge facts, activation proof,
  or A* step values leave the oracle UNCHECKED.
- **Fixture instability:** stock maps can include surrounding path choices; a minimal map may be better if it can be loaded identically by both engines.

## Parity-Critical Fields

| Group | Required values |
|---|---|
| Scenario | map/fixture, theater, unit `[MTNK]`, house, start, target, selected route cells |
| Tile facts | tile id, subtile, slope byte, level byte, land type, `BridgeSet` / `WoodBridgeSet` membership |
| Bridge stamp | flags `0x80`, `0x100`, `0x200`, `0x400`, state byte, anchor identity, family, direction, deck level |
| gamemd A* | current cell, candidate cell, direction, incoming path height, candidate closed-list layer, edge cost, carried height |
| gamemd cell entry | `CheckBridgeTraversal` result, `Can_Enter_Cell` code, object-list layer, occupancy-bit layer, selected object list, occupancy bits |
| Rust terrain/path | `ResolvedTerrainCell`, `PathGrid`, `TerrainCostGrid`, `compute_neighbor_height`, `check_bridge_traversal`, `CanEnterLayerContext` |
| Runtime | tick, current/next cell, `loco.layer`, `on_bridge`, bridge occupancy, occupancy layer before/after, visible Z |
| Activation proof | gamemd unit pointer/id, `[MTNK]` type identity, house, issued order id/tick, pathfinder/search id, and call-site category tying captured rows to the selected crossing |

---

## Tasks

### Task 1: Select the concrete fixture route

**Why:** The oracle only works if both engines run the same map, theater, unit, start, target, and intended crossing.

**Files:**
- Read: stock maps / minimal map fixtures
- Read: `docs/research/traces/BRIDGE_CROSSING_PATH_ORACLE_REQUIREMENTS_TRACE.md`
- Create later: scenario manifest

**Steps:**

1. Identify one high-bridge crossing with an intact deck and clear Grizzly route.
2. Prefer a minimal fixture if it can be loaded by gamemd and Rust without extra path-choice noise. Use a stock map only if the gamemd capture tooling cannot practically run the minimal fixture.
3. If using a stock map, choose a route with minimal competing detours and stable start/target coordinates.
4. Record theater, map name, start cell, target cell, expected bridge route cell window, and bridge overlay anchors.
5. Include adjacent cells required by `SetBridgeDirection` stamping, not only the visible route cells.
6. Stop if no route can be selected without new research; write the blocker before instrumenting.

**Acceptance:**

- A scenario manifest draft exists with map/theater/unit/start/target/route-window data.
- The route is a high bridge, not a low bridge/tube crossing.

### Task 2: Define the trace schema and comparator contract

**Why:** Both captures must write the same field names before either side starts dumping data.

**Files:**
- Create: likely `tools/bridge_oracle/schema.md` or equivalent
- Create: likely comparator skeleton path, no behavior changes

**Steps:**

1. Define top-level trace schema: `scenario`, `cell_facts`, `astar_steps`, `runtime_ticks`.
2. Add schema version field.
3. Define required fields for gamemd and Rust sides.
4. Define comparator verdict rules:
   - PASS: both present and equal;
   - FAIL: both present and unequal;
   - UNCHECKED: either missing.
5. Define route mismatch reporting: first differing step/tick must be shown.
6. Define the A* row comparison key:
   - primary key: `(search_id, expansion_index)`;
   - validation tuple: `(current_cell, candidate_cell, direction, incoming_path_height)`;
   - if expansion indexes diverge, report the first divergent expansion and then
     compare any overlapping `(current_cell, candidate_cell, direction,
     incoming_path_height)` rows as secondary evidence only.
7. Define candidate coverage as every A* neighbor candidate popped/expanded inside
   the selected route window, including rejected candidates, not just accepted path
   cells.
8. Add fixtures with tiny hand-written sample traces to test PASS, FAIL, UNCHECKED,
   divergent expansion order, and missing activation-proof reporting.

**Acceptance:**

- Comparator can compare sample traces without needing Rust or gamemd capture.
- Missing fields produce UNCHECKED, not PASS.
- Divergent expansion order reports the first divergence instead of silently
  joining unrelated rows.

**Run:**

```powershell
cargo check -q
```

If the comparator is implemented outside Rust, run its focused test command instead.

### Task 3: Add Rust map-load cell-facts capture

**Why:** The first parity gate is whether Rust and gamemd stamp/load the same bridge facts for the same cells.

**Files:**
- Modify: `src/map/resolved_terrain.rs`
- Modify/read: `src/map/bridge_facts.rs`
- Add tests/tool glue as needed

**Steps:**

1. Add an opt-in diagnostic extractor for selected coordinates.
2. Emit:
   - source/final tile id and subtile;
   - `level`, `slope_type`, `land_type`, `yr_cell_land_type`;
   - `BridgeSet` / `WoodBridgeSet` membership from the theater lookup;
   - `bridge_facts.raw_flags`;
   - booleans for `0x80`, `0x100`, `0x200`, `0x400`;
   - `state_byte`, `overlay_id`, `family`, `direction`, `anchor`, `bridge_deck_level`;
   - `has_bridge_deck`, `bridge_walkable`, `bridge_transition`.
3. If theater lookup is unavailable, fail the Rust cell-facts capture or mark the
   tile-facts group UNCHECKED; do not emit a complete trace with this field absent.
4. Keep extraction read-only and route-scoped.
5. Add a focused test using existing synthetic bridge facts if practical.

**Acceptance:**

- Rust can dump `cell_facts` for the selected route/window.
- Existing terrain tests still pass.

**Run:**

```powershell
cargo test -q resolved_terrain
cargo test -q theater
```

### Task 4: Add Rust A* candidate-step capture

**Why:** The oracle must compare layer, height, traversal, cost, and carried height per candidate, not just final path cells.

**Files:**
- Modify: `src/sim/pathfinding/core.rs`
- Modify tests as needed

**Steps:**

1. Add an opt-in trace sink or collector passed through `AStarOptions` or a test-only wrapper.
2. For every A* neighbor candidate popped/expanded inside the selected route
   window, including rejected candidates and the first divergence window, emit:
   - current node cell/layer/height;
   - candidate cell;
   - direction;
   - initial candidate closed-list layer;
   - `compute_neighbor_height` result;
   - whether `check_bridge_traversal` ran;
   - traversal allowed/resulting path height/forced bridge-list flag;
   - final candidate layer;
   - walkability and terrain cost;
   - soft-block/object-list layer if available;
   - edge cost and tentative carried height.
3. Emit `search_id` and `expansion_index` so the comparator can match rows
   without guessing.
4. Keep full-map A* debug off by default.
5. Do not change ordering, costs, or bridge predicates.

**Acceptance:**

- Rust A* trace can be generated for the selected scenario.
- Rust A* trace includes rejected candidates in the selected route window.
- Existing pathfinding tests still pass.

**Run:**

```powershell
cargo test -q pathfinding::core
```

### Task 5: Add Rust cell-entry layer/result capture

**Why:** gamemd's object-list layer and occupancy-bit layer can disagree. Rust must report its equivalent split explicitly.

**Files:**
- Modify: `src/sim/pathfinding/cell_entry.rs`
- Modify callers/tests as needed

**Steps:**

1. Emit `CanEnterLayerContext` for traced checks.
2. Emit terrain layer, object-list layer, occupancy-bit layer separately.
3. Emit final `CellEntryResult::yr_code()`.
4. Emit occupancy bits/list source equivalent where Rust has it.
5. Keep capture optional and read-only.

**Acceptance:**

- Rust trace has separate object-list and occupancy-bit layer fields.
- Existing cell-entry tests pass.

**Run:**

```powershell
cargo test -q cell_entry
```

### Task 6: Add Rust runtime movement capture

**Why:** Even if A* matches, runtime layer/on-bridge/occupancy transitions can still diverge at bridgeheads.

**Files:**
- Modify: `src/sim/movement/movement_step.rs`
- Modify: `src/sim/movement/movement_tick.rs`
- Modify/read: `src/sim/movement/movement_bridge.rs`

**Steps:**

1. Add opt-in route/tick capture for the selected Grizzly.
2. Emit:
   - tick number;
   - current cell and next cell;
   - active `loco.layer`;
   - current/projected `on_bridge`;
   - `BridgeStateUpdate`;
   - bridge occupancy before/after;
   - occupancy layer before/after;
   - visible `position.z`;
   - `path_layers` next layer.
3. Keep diagnostics off in normal gameplay.
4. Avoid changing movement order or state writes.

**Acceptance:**

- Rust can dump runtime ticks for the fixture crossing.
- Existing movement bridge tests pass.

**Run:**

```powershell
cargo test -q movement_bridge
cargo test -q movement_step
```

### Task 7: Capture gamemd map-load and A* / Can_Enter_Cell values

**Why:** Rust traces are not an oracle by themselves. The gamemd side is the spec.

**Files:**
- Create: gamemd trace artifact under chosen fixture/capture directory
- Read-only Ghidra/debugger usage only

**Steps:**

1. Capture gamemd map-load cell facts for every selected route/window cell:
   - tile id, subtile, slope byte, level byte, land type;
   - flags `0x80`, `0x100`, `0x200`, `0x400`;
   - state byte;
   - anchor identity;
   - direction/family;
   - deck level.
2. Capture activation proof tying rows to the selected live Grizzly crossing:
   - unit pointer/id;
   - object type `[MTNK]`;
   - house;
   - issued move order id/tick;
   - pathfinder/search id;
   - whether the row came from A* `UnitClass::Can_Enter_Cell`, runtime
     locomotor probing, or another verified call-site category.
3. Capture gamemd A* candidate-step values for every neighbor candidate
   popped/expanded inside the selected route window, including rejected candidates:
   - current cell;
   - candidate cell;
   - direction;
   - search id and expansion index;
   - incoming path height;
   - candidate closed-list layer;
   - `CheckBridgeTraversal` result and resulting height;
   - `UnitClass::Can_Enter_Cell` code;
   - selected object list and occupancy bits;
   - edge cost and carried path height.
4. Capture runtime movement ticks:
   - cell/layer/on-bridge/occupancy/visible height.
5. Do not mutate Ghidra project metadata.
6. Mark any uncaptured required field explicitly missing.

**Acceptance:**

- Gamemd trace file exists for the selected fixture.
- Every required field is either present or explicitly marked missing.
- Captured gamemd rows include activation proof for the selected `[MTNK]` move
  order; otherwise the oracle remains UNCHECKED.

### Task 8: Run Rust capture for the same fixture

**Why:** Comparator input must be generated from the same scenario, not a synthetic approximation.

**Files:**
- Create: Rust trace artifact under chosen fixture/capture directory

**Steps:**

1. Load the same fixture/map/theater.
2. Spawn/order the same `[MTNK]` route.
3. Generate Rust `cell_facts`, `astar_steps`, and `runtime_ticks`.
4. Confirm route window coordinates match the manifest.
5. Do not hand-edit trace fields.

**Acceptance:**

- Rust trace file exists and references the same scenario id/schema version as gamemd trace.
- Comparator can parse both traces.

### Task 9: Compare and write the parity report

**Why:** The deliverable is a PASS/FAIL/UNCHECKED verdict with concrete field evidence.

**Files:**
- Create: `docs/research/traces/<SCENARIO>_BRIDGE_CROSSING_PATH_ORACLE_TRACE.md`

**Steps:**

1. Run comparator on gamemd and Rust traces.
2. Produce summary counts: PASS, FAIL, UNCHECKED.
3. Report first route/step/tick mismatch if any.
4. Include tables for:
   - map-load cell facts;
   - A* candidate steps;
   - cell-entry layer/result split;
   - runtime movement ticks.
5. If missing fields remain, keep final verdict UNCHECKED and name the missing fields.
6. If fields differ, mark FAIL and list exact values.
7. If all required fields match, mark PASS for this one scenario only.

**Acceptance:**

- Report exists and clearly states scoped verdict.
- No bridge implementation fix is included in the same patch.

### Task 10: Focused verification bundle

**Why:** Diagnostics touch pathfinding and movement surfaces; focused checks must prove they did not regress current behavior.

**Run:**

```powershell
cargo test -q theater
cargo test -q resolved_terrain
cargo test -q terrain_cost
cargo test -q pathfinding::core
cargo test -q movement_bridge
cargo test -q movement_step
cargo test -q movement_tick
cargo check -q
```

If a test target is noisy for unrelated local changes, record the unrelated failure and run the narrow new tests plus the closest existing passing target.

## Acceptance Criteria

- One concrete high-bridge Grizzly route is selected and documented.
- Trace schema is versioned and comparator-tested with PASS/FAIL/UNCHECKED samples.
- Rust can dump route-scoped map-load bridge facts.
- Rust can dump route-scoped A* candidate-step facts.
- Rust can dump separate cell-entry object-list and occupancy-bit layer choices.
- Rust can dump runtime movement layer/on-bridge/occupancy/visible-Z ticks.
- Gamemd trace exists for the same route and required fields are present or explicitly missing.
- Comparator emits literal PASS/FAIL/UNCHECKED verdicts by field group.
- Final trace report is saved under `docs/research/traces/`.
- No bridge behavior fix is mixed into the oracle patch.

## Follow-Up Queue After This Plan

1. If the oracle reports FAIL, write a separate implementation contract or fix plan for the first concrete mismatch.
2. Add blocker-on-ground and blocker-on-bridge variants if empty-route bridge movement passes but object-list/occupancy split remains untested.
3. Generalize to a second high-bridge family/direction only after the first route is complete.
4. Use a later trace swarm only after this single oracle exists, so bridge route checks can produce PASS/FAIL instead of UNCHECKED.
