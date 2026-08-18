# High Bridge Forward2 Traversal Parity Fix Implementation Plan

> **For implementer:** Execute this plan task-by-task. This is a planning artifact only; do not write Rust code until this plan is approved.

**Goal:** Stop normal vehicle A* from routing onto the direction-6 high-bridge Forward2 edge lane while preserving authoritative `SetBridgeDirection` stamping and non-anchor bridge traversal cells.

**Architecture:** This is a `sim/pathfinding` legality fix. `map/bridge_facts` and `map/resolved_terrain` remain the source of stamped bridge facts; A* must consume those facts through the existing `check_bridge_traversal` gate instead of widening movement from `bridge_walkable` alone. No render, UI, sidebar, audio, or net dependency is introduced.

**Design Input:** User-approved scope plus `docs/research/HIGH_BRIDGE_EDGE_LANE_TRAVERSAL_REINVESTIGATION_GHIDRA_REPORT.md`. No separate `*-design.md` exists for this narrow follow-up; the reinvestigation report supplies the architecture and impact analysis for this plan.

---

## Grounding Summary

- The reinvestigation report says the BayOPigs symptom is primarily movement legality, not raw overlay shrinkage.
- `SetBridgeDirection` stamping is authoritative and must stay as-is: direction-6 `BRIDGE2` anchors stamp Forward1, Forward2, Anchor, Opposite, and side-marker cells.
- Direction-6 BayOPigs anchors at `x=112` and `x=160` currently yield Rust path columns `x=110..113` and `x=158..161`.
- The suspect min-X columns are Forward2: component 1 `x=110`, component 2 `x=158`.
- Forward2 has structural flag `0x100`, but lacks transition/bridgehead flag `0x200`.
- `CheckBridgeTraversal @ 0x004D9C60` uses path height plus `0x200` in the diff-0 gate. A vehicle already on the bridge deck carries `path_height = Level + 4`, so deck-height movement into Forward2 is blocked.
- `UnitClass::Can_Enter_Cell @ 0x0073F0A0` calls the bridge traversal validator before later occupancy checks; a rejected Forward2 candidate cannot be rescued by bridge occupancy bits.
- Current Rust already has a close `check_bridge_traversal` analogue in `src/sim/pathfinding/core.rs`, including the diff-0 `{candidate structural, candidate transition, parent structural}` check.
- Current Rust A* bypasses that analogue for structural-to-structural moves when the neighbor is not a transition, then accepts `bridge_walkable`.
- Live Ghidra MCP was available, but the currently loaded program/address space did not resolve the report addresses during planning. This plan therefore relies on the verified Ghidra reports rather than fresh MCP output.
- INI files do not drive the Forward2 legality rule. Relevant INI data is overlay taxonomy only: `ini/rulesmd.ini` `[OverlayTypes] 26=BRIDGE2`, `[BRIDGE2] Image=BRIDGE`, `[BRIDGE2] Name = Bridge 2`; base `rules.ini` has the same fallback entries.

## Key Technical Decisions

- Preserve `BridgeCellFacts` stamping and `PathGrid.bridge_walkable` for Forward2. The player-visible pathing fix belongs in A* edge legality, not map fact generation. **Confidence:** high. **Source:** reinvestigation report, `BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md`, current `src/map/bridge_facts.rs`.
- Apply `check_bridge_traversal` to bridge-deck structural-to-structural A* moves, including non-transition candidates. The current local structural bypass is too permissive for Forward2. **Confidence:** high. **Source:** reinvestigation report, `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`, current `src/sim/pathfinding/core.rs`.
- Keep renderer and railing changes out of this fix. Visual artifacts still show separate rendering/railing questions, but they are not needed to stop the illegal min-X path lane. **Confidence:** high. **Source:** `BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md`, `docs/visual-checks/bridge-unit-edge-footprint/investigation.md`.
- Treat existing "body-to-body non-transition is always OK" tests as stale if they contradict the corrected diff-0 gate. Replace them with tests for stamped transition cells and explicit Forward2 rejection. **Confidence:** medium-high. **Source:** reinvestigation report and current tests around `astar_allows_body_to_body_diagonal`.

## Open Questions

### Resolved During Planning

- **Should PathGrid shrink to raw anchors?** No. Raw `BRIDGE2` anchors are too narrow; `SetBridgeDirection` stamps multiple cells.
- **Is Forward2 missing from stamping?** No. Forward2 is correctly stamped structural and non-transition; the problem is A* treating that as freely bridge-walkable.
- **Do INI keys decide Forward2 legality?** No. INI identifies overlay types, but the movement rule is binary cell flag/path-height logic.

### Deferred to Implementation

- **Does any existing synthetic bridge test encode an old, overbroad invariant?** Implementation must run the focused test set after changing A* and update only tests whose assertions conflict with the verified diff-0 rule.
- **Do BayOPigs visual tools need new A* reachability columns?** Existing visual tools report `PathGrid.bridge_walkable`, which should remain unchanged. If the implementer needs a BayOPigs route-footprint artifact, add a small reachability diagnostic as a verification-only follow-up, not as the engine fix.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/sim/pathfinding/core.rs` | Use `check_bridge_traversal` for bridge-deck structural-to-structural A* expansion instead of bypassing it through `bridge_walkable`. |
| Modify | `src/sim/pathfinding/core_tests.rs` | Add focused regression coverage for direction-6 stamp shape, Forward2 rejection, and non-anchor transition cells staying usable. |
| Read only | `src/map/bridge_facts.rs` | Confirm direction-6 Forward2 remains structural without transition. No implementation change expected. |
| Read only | `src/map/resolved_terrain.rs` | Confirm bridge facts still derive `bridge_walkable` and `bridge_transition` from stamped facts. No implementation change expected. |
| Read only | `src/sim/movement/movement_bridge.rs` | Confirm runtime render/on-bridge state is not part of this A* route legality fix. |

## Interface Changes

No public API change is expected. Keep `PathCell`, `PathGrid`, `BridgeTraversalInput`, and `BridgeTraversalResult` signatures unchanged unless a test exposes an unavoidable need.

If a helper is introduced, keep it private to `src/sim/pathfinding/core.rs`, for example a predicate that decides when A* must call `check_bridge_traversal`. It must not expose bridge internals outside `sim/pathfinding`.

## Sim Checklist

- [ ] No `f32` or `f64` in game logic.
- [ ] No new deterministic state.
- [ ] No dependency from `sim/` to `render/`, `ui/`, `sidebar/`, `audio/`, or `net/`.
- [ ] Tick ordering unchanged; this affects path selection before movement ticks consume a path.
- [ ] `EntityStore` iteration order not relevant; A* cell expansion order stays the existing `NEIGHBORS` order.

## Risk Areas

- A* may reject too many bridge cells if the traversal gate is applied without preserving valid Anchor, Forward1, and Opposite transitions.
- Existing synthetic tests may use "bridge body" to mean any non-transition bridge cell. The corrected binary evidence says non-transition Forward2 is not a free deck destination in the diff-0 deck-height case.
- Existing visual diagnostics named `path_bridge_walkable` are not route legality diagnostics. They should remain broad after this fix because stamping and PathGrid facts are preserved.
- Diagonal bridge corner-cutting still uses `grid.is_walkable_on_layer`; this plan does not change diagonal corner checks unless a focused regression proves they reintroduce Forward2 reachability.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| 2 | Forward2 rejected for bridge-deck movement | Vehicles should not path/render on the BayOPigs min-X edge lane outside the visible railing/deck envelope. | Unit test with direction-6 stamp and BayOPigs-style Forward2, plus A* route check. |
| 2 | Anchor, Forward1, and Opposite remain usable | Raw-anchor-only shrink would make bridge lanes too narrow compared with `gamemd.exe` stamping. | Unit tests prove non-anchor transition cells are accepted. |
| 5 | BayOPigs footprint does not collapse to raw anchors | Player pathing should lose the illegal min-X lane without destroying legitimate stamped bridge width. | Re-run bridge diagnostic tools and inspect pathing-focused test output. |

---

## Tasks

### Task 1: Pin Forward2 Stamping Facts In Tests

**Why:** Establish that the map facts are not being changed and that the fix targets movement legality only.

**Files:**
- Modify: `src/map/bridge_facts.rs` only if existing tests are not explicit enough.
- Prefer modifying: `src/sim/pathfinding/core_tests.rs` to avoid touching map stamping code.

**Pattern:** Existing `stamp_dir6_intact_sets_west_slots_and_two_east_slots` test in `src/map/bridge_facts.rs`.

**Steps:**
1. Add or strengthen a test that stamps direction `6` at anchor `(2, 1)` on a small grid.
2. Assert slot coordinates:
   - Forward2 at `(0, 1)` is structural.
   - Forward2 at `(0, 1)` is not transition.
   - Forward1 at `(1, 1)`, Anchor at `(2, 1)`, and Opposite at `(3, 1)` are structural and transition.
   - ExtraDir6 at `(4, 1)` is not structural.
3. Do not change `stamp_set_bridge_direction`, flag constants, or resolved terrain derivation.

**Verify:**
Run: `cargo test stamp_dir6 -- --nocapture`

Expected: direction-6 stamp tests pass and Forward2 remains structural without transition.

### Task 2: Add A* Regression For Direction-6 Forward2 Rejection

**Why:** Capture the exact BayOPigs disparity before changing A* expansion.

**Files:**
- Modify: `src/sim/pathfinding/core_tests.rs`

**Pattern:** Existing bridge A* tests near `astar_blocks_height_diff_4_without_bridgehead` and `astar_blocks_structural_body_to_body_bad_height_jump`.

**Steps:**
1. Add a helper that builds a 5-column direction-6 high-bridge row:
   - `(0, 1)` Forward2: `ground_level=0`, `bridge_walkable=true`, `bridge_structural=true`, `transition=false`, `bridge_deck_level=4`.
   - `(1, 1)` Forward1: same but `transition=true`.
   - `(2, 1)` Anchor: same but `transition=true`.
   - `(3, 1)` Opposite: same but `transition=true`.
   - `(4, 1)` ExtraDir6/side marker: not bridge-walkable.
2. Seed A* from a valid bridge-deck structural transition cell using `MovementLayer::Bridge`.
3. Assert a path from Anchor/Opposite to Forward1 succeeds.
4. Assert a path whose only bridge-deck destination is Forward2 is rejected, or that any successful route does not include `(0, 1)` on `MovementLayer::Bridge`.
5. Add a direct `check_bridge_traversal` unit test for parent structural deck-height to candidate structural non-transition:
   - `path_height = candidate.Level + 4`.
   - Parent structural true.
   - Candidate structural true, transition false.
   - Expected `allowed=false`.
6. Add the paired direct test for candidate structural transition true and expected `allowed=true`.

**Verify:**
Run: `cargo test forward2 -- --nocapture`

Expected before implementation: at least the A* Forward2 regression should fail, proving it covers the current bug. Expected after implementation: all Forward2 tests pass.

### Task 3: Route Structural Deck Moves Through `check_bridge_traversal`

**Why:** Fix the actual bug: Rust skips the verified diff-0/path-height/`0x200` gate for structural-to-structural bridge moves.

**Files:**
- Modify: `src/sim/pathfinding/core.rs`

**Pattern:** Existing `needs_bridge_traversal` branch in A* neighbor expansion.

**Steps:**
1. Replace the current `needs_bridge_traversal` predicate:
   - Current shape: call traversal only for transition candidates or non-structural edges.
   - New shape: also call traversal when the current node's carried height is on the bridge deck and both current and neighbor cells are structural bridge cells.
   - Define "current node is on the bridge deck" from height, not from `AStarNode.on_bridge`: use `current.height == cur_cell.bridge_deck_level`, or equivalently `current.height as i16 == cur_cell.signed_level() + 4`.
   - Do not use the local `on_bridge` flag for this predicate. It is the push-time closed-list selector and can diverge from the node height at bridgehead transition cells.
2. Use the existing `check_bridge_traversal` call; do not duplicate the diff-0 formula in A*.
3. Keep `BridgeTraversalInput.path_height` as the current A* node height.
4. Keep the explicit parent as the current cell.
5. Preserve `force_bridge_list` handling exactly as today.
6. Leave the local non-bridge height legality fallback in place only for edges that still do not call `check_bridge_traversal`.
7. Add a regression that proves the new predicate follows `current.height`, not `on_bridge`, for a bridgehead/layer-divergent case.
8. Do not change `compute_neighbor_height` unless a failing test proves the current height propagation is inconsistent with the verified A* create-node behavior.

**Verify:**
Run: `cargo test forward2 -- --nocapture`

Expected: Forward2 deck destination is blocked; Forward1/Anchor/Opposite transition destinations remain accepted.

### Task 4: Audit And Update Existing Bridge A* Tests

**Why:** The corrected binary evidence may invalidate older synthetic "body-to-body non-transition is always OK" expectations.

**Files:**
- Modify: `src/sim/pathfinding/core_tests.rs`

**Pattern:** Existing `astar_allows_body_to_body_diagonal` and height-diff legality tests.

**Steps:**
1. Run the bridge-focused tests after Task 3.
2. If `astar_allows_body_to_body_diagonal` fails because it routes deck-height into a non-transition structural candidate, replace its assertion with a verified invariant:
   - Transition structural cells remain traversable.
   - Non-transition Forward2-style structural cells are not traversable as bridge-deck destinations.
3. Keep the existing bad-height-jump test; it remains relevant because structural bridge cells must still obey height legality.
4. Do not weaken tests that protect ground-to-bridge bridgehead entry, diff-2/diff-3 blocking, or diff-1 slope gates.

**Verify:**
Run:
- `cargo test bridge_traversal -- --nocapture`
- `cargo test astar_ -- --nocapture`

Expected: all bridge traversal and A* bridge regression tests pass.

### Task 5: Guard Against Raw-Anchor-Only Shrinkage

**Why:** The fix must remove Forward2 from normal deck pathing without reducing bridge pathing to raw `BRIDGE2` anchors only.

**Files:**
- Modify: `src/sim/pathfinding/core_tests.rs`
- Read only: `src/map/bridge_facts.rs`

**Pattern:** Same direction-6 bridge row fixture from Task 2.

**Steps:**
1. Add a test whose raw anchor is `(2, 1)` and whose successful route must include at least one non-anchor transition cell, either Forward1 `(1, 1)` or Opposite `(3, 1)`.
2. Assert ExtraDir6 `(4, 1)` remains unusable as a bridge-deck destination.
3. Assert Forward2 `(0, 1)` remains present in `PathGrid` as `bridge_walkable=true` if the fixture models stamped PathGrid facts directly, but is not accepted by A* deck movement.

**Verify:**
Run:
- `cargo test raw_anchor -- --nocapture`
- `cargo test forward2 -- --nocapture`

Expected: pathing uses stamped non-anchor transition cells but rejects Forward2.

### Task 6: Run BayOPigs Diagnostics With Correct Interpretation

**Why:** Confirm the player-visible BayOPigs lane no longer appears in normal vehicle route legality while preserving map stamping.

**Files:**
- Read only unless a missing diagnostic column blocks verification:
  - `docs/visual-checks/bridge-terrain-overlay-dump-tool/src/main.rs`
  - `docs/visual-checks/bridge-unit-edge-footprint-tool/src/main.rs`
  - `docs/visual-checks/bridge-render-footprint-tool/src/main.rs`

**Pattern:** Existing visual-check tools under `docs/visual-checks`.

**Steps:**
1. Re-run the terrain overlay dump:
   - Command:
     `powershell -NoProfile -Command '$env:RA2_DIR="<ra2-install>"; cargo run --manifest-path docs/visual-checks/bridge-terrain-overlay-dump-tool/Cargo.toml -- BayOPigs.mmx docs/visual-checks/bridge-terrain-overlay-mismatch'`
   - Expected: `path_bridge_walkable` still includes component 1 `x=110..113` and component 2 `x=158..161`. This confirms stamping was not shrunk.
2. Re-run the unit edge footprint tool:
   - Command:
     `powershell -NoProfile -Command '$env:RA2_DIR="<ra2-install>"; cargo run --manifest-path docs/visual-checks/bridge-unit-edge-footprint-tool/Cargo.toml -- BayOPigs.mmx docs/visual-checks/bridge-unit-edge-footprint'`
   - Expected: existing unit-center/render envelope findings may remain; renderer/railing questions are out of scope.
3. If no existing diagnostic reports A* reachability, rely on the new A* regression tests for movement legality and record in the implementation notes that `path_bridge_walkable` is a stamp footprint, not a route footprint.
4. If the implementer adds an A* reachability column to the dump tool, expected BayOPigs result is:
   - component 1 normal bridge-deck route footprint excludes `x=110`;
   - component 2 normal bridge-deck route footprint excludes `x=158`;
   - both components still include more than the raw anchor columns `x=112` and `x=160`.

**Verify:**
Inspect:
- `docs/visual-checks/bridge-terrain-overlay-mismatch/bayopigs-mmx-summary.md`
- `docs/visual-checks/bridge-unit-edge-footprint/bayopigs-mmx-edge-unit-footprint.md`
- the new test output from Tasks 2 and 5.

### Task 7: Full Local Verification

**Why:** This is a hot path in movement. Run both focused and broader checks before handoff.

**Files:** No edits.

**Steps:**
1. Run focused tests:
   - `cargo test forward2 -- --nocapture`
   - `cargo test bridge_traversal -- --nocapture`
2. Run the pathfinding suite:
   - `cargo test sim::pathfinding -- --nocapture`
3. Run formatting/checks:
   - `cargo fmt --check`
   - `cargo check`
4. If unrelated dirty-tree changes outside pathfinding break broad `cargo check`, do not fix unrelated work. Record the exact failing command and first unrelated error in the handoff.

**Expected:** Focused pathfinding tests pass. Formatting passes for touched files. Broad check either passes or fails only for unrelated pre-existing dirty-tree work.

## Sources & References

- `docs/research/HIGH_BRIDGE_EDGE_LANE_TRAVERSAL_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_SETBRIDGEDIRECTION_STAMPING_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md`
- `docs/research/UNIT_CAN_ENTER_CELL_BRIDGE_TUNNEL_GHIDRA_REPORT.md`
- `docs/research/BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md`
- `src/sim/pathfinding/core.rs`
- `src/sim/pathfinding/core_tests.rs`
- `src/map/bridge_facts.rs`
- `src/map/resolved_terrain.rs`
- `src/sim/movement/movement_bridge.rs`
- `docs/visual-checks/bridge-terrain-overlay-mismatch/investigation.md`
- `docs/visual-checks/bridge-unit-edge-footprint/investigation.md`
- `docs/visual-checks/bridge-render-footprint/investigation.md`
- Ghidra functions cited by reports: `CheckBridgeTraversal @ 0x004D9C60`, `UnitClass::Can_Enter_Cell @ 0x0073F0A0`, `AStar_main_loop @ 0x00429A90`, `AStar_create_node @ 0x0042A4A0`, `SetBridgeDirection_NESW @ 0x0047E040`, `SetBridgeDirection_NWSE @ 0x0047E470`.
- INI references: `ini/rulesmd.ini` `[OverlayTypes] 26=BRIDGE2`, `[BRIDGE2] Image=BRIDGE`, `[BRIDGE2] Name = Bridge 2`; `ini/rules.ini` fallback has the same base entries.
