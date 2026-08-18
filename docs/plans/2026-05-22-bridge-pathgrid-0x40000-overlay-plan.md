# Bridge Pathgrid 0x40000 Overlay Implementation Plan

> For Codex: Execute this plan task-by-task. Each task is self-contained.

**Goal:** Add gamemd.exe's temporary `CellClass+0x140 & 0x40000` bridge/pathgrid
cost-marker behavior as a search-local A* overlay. The marker must affect path
choice near bridge traffic by multiplying marked destination-cell costs by 4,
without becoming persistent `PathGrid`, `ZoneGrid`, bridge state, occupancy, or
save data.

**Architecture:** Add a pathfinding-internal `bridge_markers` module. Movement
builds deterministic peer snapshots; `zone_search` only forwards the marker
context; `astar_search` builds a local `BridgeCostOverlay` after normal A*
entry gates and applies the 4x destination cost during edge-cost calculation.

**Design Doc:** [docs/plans/2026-05-22-bridge-pathgrid-0x40000-overlay-design.md](2026-05-22-bridge-pathgrid-0x40000-overlay-design.md)

---

## Grounding Summary

- **Verified marker lifecycle:** `PathfinderClass::UpdateBridgePassability @
  0x0042ACF0` toggles `0x40000` before A* and toggles it back on ordinary
  success/failure. Source:
  `docs/research/ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md`.
- **Verified cost consumer:** `AStar_compute_edge_cost @ 0x00429830` multiplies
  destination-cell cost by `4.0` when `0x40000` is present. Source:
  `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`.
- **Verified negative boundary:** `0x40000` is not `0x400`; `0x400` has
  placement/render/bridge-fallback uses and is not the A* cost marker. Source:
  `CELLCLASS_0X140_0X400_GLOBAL_XREF_CENSUS_GHIDRA_REPORT.md`.
- **Verified peer gate:** normal peer path marking skips same type and requires
  `searcher.TechnoType.Speed > peer.TechnoType.Speed`; urgency `2` bypasses the
  same-type, speed, and playfield gate. Source:
  `PATHFINDER_0042ACF0_OBJECT_0X678_GATE_GHIDRA_REPORT.md`.
- **Verified direction replay:** peer path entries `0..7` use N, NE, E, SE, S,
  SW, W, NW offsets; entry `8` uses tube metadata and `TubeClass+0x28` / Rust
  `TubeFact.exit`. Source:
  `GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`.
- **Repo fit:** `AStarOptions` already carries transient per-search inputs.
  `zone_search` already forwards options into final A* calls. `movement_tick`
  already uses `MoverSnapshot` to cross borrow boundaries deterministically.

## Key Technical Decisions

- **Search-local overlay, not `PathGrid` mutation** - confidence high. It
  matches gamemd's ordinary pre/post toggle lifecycle and keeps the static
  terrain grid clean.
- **Overlay built inside `astar_search`, not in `zone_search`** - confidence
  high. Zone precheck aborts must not create markers; corridor retries must get
  fresh overlays per cell-A* attempt.
- **Movement supplies snapshots, not live callbacks** - confidence high. This
  preserves deterministic iteration order and avoids coupling A* to `EntityStore`
  borrow rules.
- **Peer path work lands before fallback probe work** - confidence high. Peer
  path replay and cost application are fully sourced. The pseudo-random fallback
  probe still needs an explicit Rust equivalent for `RateTimer__Current`.
- **Do not use `SimRng` for the probe** - confidence high. Consuming gameplay RNG
  would perturb unrelated deterministic systems.

## Open Questions

### Resolved During Planning

- **Where does `0x40000` live in Rust?** Search-local `BridgeCostOverlay` inside
  pathfinding. It does not live in `PathGrid`.
- **Should zone search own cleanup?** No. Rust cleanup is dropping the local
  overlay. `zone_search` only forwards context.
- **Does smoothing need the overlay?** No. The marker influences A* route choice
  only. Smoothing continues to avoid known entity soft-block cells via
  `entity_block_map`, but it does not need historical bridge marker state.
- **Should `entity_block_map` be reused?** No. It models Can_Enter_Cell codes
  2/5/6, not the temporary bridge cost bit.

### Deferred to Implementation

- **Exact `RateTimer__Current` equivalent:** Required before enabling the 5x5
  fallback probe in live play. Implement the API with `timer_current`, but gate
  live fallback use behind a named TODO unless an equivalent existing tick/timer
  value is proven.
- **Exact `0x0042B080` subobject predicate:** If current Rust entity data cannot
  reproduce the helper exactly, isolate the approximation in a named helper with
  a report citation. Do not broaden it into "all nearby peers".
- **Recovering direction `8` from current movement paths:** If existing
  `MovementTarget.path` cannot distinguish explicit tube path entries from
  arbitrary non-adjacent jumps, add a deterministic path-direction cache or defer
  direction-8 live generation until the movement surface can represent it.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Add | [src/sim/pathfinding/bridge_markers.rs](../../src/sim/pathfinding/bridge_markers.rs) | Overlay data types, direction replay, peer eligibility, overlay builder, unit tests |
| Modify | [src/sim/pathfinding/mod.rs](../../src/sim/pathfinding/mod.rs) | Register/re-export pathfinding-internal marker module/types |
| Modify | [src/sim/pathfinding/core.rs](../../src/sim/pathfinding/core.rs) | Extend `AStarOptions`; build overlay in `astar_search`; apply 4x destination multiplier; thread wrappers |
| Modify | [src/sim/pathfinding/zone_search.rs](../../src/sim/pathfinding/zone_search.rs) | Forward optional bridge marker context through every final A* call |
| Modify | [src/sim/movement/mod.rs](../../src/sim/movement/mod.rs) | Extend `MoverSnapshot` / pathfinding request context with marker inputs |
| Modify | [src/sim/movement/movement_tick.rs](../../src/sim/movement/movement_tick.rs) | Build deterministic mover/peer snapshots and context for normal path requests |
| Modify | [src/sim/movement/movement_blocked.rs](../../src/sim/movement/movement_blocked.rs) | Pass marker context through blocked repaths with urgency 1/2 |
| Modify | [src/sim/movement/movement_path.rs](../../src/sim/movement/movement_path.rs) | Forward marker context into flat/layered zone search |
| Modify | [src/sim/pathfinding/core_tests.rs](../../src/sim/pathfinding/core_tests.rs) | Overlay, replay, cost, cleanup tests |
| Modify | [src/sim/pathfinding/zone_search_tests.rs](../../src/sim/pathfinding/zone_search_tests.rs) | Zone/corridor lifetime tests |
| Modify | [src/sim/movement/movement_tests.rs](../../src/sim/movement/movement_tests.rs) | Snapshot/plumbing tests |

## Interface Changes

- `AStarOptions<'a>` gains:

```rust
pub(crate) bridge_marker_context: Option<&'a BridgeMarkerContext<'a>>,
```

- These pathfinding entry points gain a final optional context parameter:
  `find_path_with_costs`, `find_path_with_costs_corridor`, `find_layered_path`,
  `find_path_zoned`, and `find_layered_path_zoned`.
- `movement_path::find_move_path` gains:

```rust
bridge_marker_context: Option<&BridgeMarkerContext<'_>>
```

- Non-movement callers pass `None`.
- No save format or public app API changes unless implementation discovers that
  peer path direction bytes must be cached on entities. If that cache is added,
  it must be serialized and included in deterministic state hashing.

## Sim Checklist

- [x] No render/ui/sidebar/audio/net dependency.
- [x] No persistent `PathGrid` / `ZoneGrid` mutation for `0x40000`.
- [x] No floating point in sim logic; use integer `* 4` cost multiplier.
- [x] Deterministic iteration required: peers from `EntityStore` order or sorted
      vectors.
- [x] No `SimRng` consumption for probe selection.
- [x] Tick ordering unchanged; overlay is built during existing path requests.
- [ ] If new entity state is added for path direction caching, include it in
      serialization and deterministic hash.

## Risk Areas

- **Signature churn:** `find_path_with_costs` and zoned wrappers have many call
  sites. Mitigation: add the parameter last and pass `None` everywhere first.
- **Wrong overlay lifetime:** Building markers in movement or `zone_search` can
  create markers for paths gamemd would abort before A*. Mitigation: construct
  overlay inside `astar_search` after normal entry gates.
- **Peer path direction mismatch:** Existing Rust paths are cell lists; gamemd
  replays direction bytes. Mitigation: convert only adjacent deltas with the
  verified table; handle direction `8` only when explicit tube state is known.
- **Timer uncertainty:** Probe/fallback parity depends on `RateTimer__Current`.
  Mitigation: implement the field and tests, but do not silently substitute RNG
  or an unverified timer.
- **Duplicate marker semantics:** Using a set as "mark only" is wrong. Mitigation:
  overlay API exposes `toggle`, and tests assert duplicate writes cancel.
- **Movement smoothing:** Smoothing could erase A* cost decisions if it shortcuts
  through marked cells. The initial design does not feed overlay into smoothing
  because gamemd marker is an A* cost input. If tests show obvious reintroduction
  of marked-cell shortcuts, pause and re-check binary smoothing order before
  adding overlay awareness there.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|---|---|---|---|
| 1-3 | `0x40000` overlay is search-local | Persistent markers would poison later paths and diverge from gamemd cleanup tails | Core tests for success/failure no residue |
| 2 | Destination cost multiplier is exactly 4x | Wrong multiplier changes bridge traffic route choices | Cost route-choice fixture |
| 4 | Zone precheck aborts do not create overlays | gamemd early returns before A* have no toggles to clean | Zone precheck abort test |
| 5 | Peer same-type/strict speed/urgency-2 gate | Determines which moving units influence bridge congestion paths | Peer gate unit tests |
| 6 | Direction replay order and direction 8 tube behavior | Wrong direction table marks wrong cells, visibly changing path choice | Direction replay tests |
| 6 | Toggle semantics | Duplicate path writes can cancel; set-only behavior is wrong | Duplicate toggle test |
| 7 | Urgency-1 no-peer early return and 5x5 center rules | Fallback marks are small but path-choice visible near bridge approaches | Fallback tests |
| 8 | Snapshot order deterministic | Same save must replay same path decisions | Movement snapshot order test |

---

## Tasks

### Task 1: Add `bridge_markers` module skeleton

**Why:** Establish the isolated home for the legacy marker model before touching
A* signatures.

**Files:**
- Add: [src/sim/pathfinding/bridge_markers.rs](../../src/sim/pathfinding/bridge_markers.rs)
- Modify: [src/sim/pathfinding/mod.rs](../../src/sim/pathfinding/mod.rs)

**Steps:**

1. Add a module doc comment explaining that this models the search-local
   `0x40000` A* cost marker, not persistent bridge/pathgrid state.
2. Define:
   - `BridgeCostOverlay`
   - `BridgeMarkerContext`
   - `BridgeMarkerMover`
   - `BridgeMarkerPeer`
   - `BridgeMarkerSnapshot`
   - `SmallPathDirs`
   - `BridgeMarkerOccupancy`
3. Implement `BridgeCostOverlay::toggle` and `is_marked`.
4. Add the verified direction table:

```rust
pub(crate) const BRIDGE_MARKER_DIRECTIONS: [(i16, i16); 8] = [
    (0, -1), (1, -1), (1, 0), (1, 1),
    (0, 1), (-1, 1), (-1, 0), (-1, -1),
];
```

5. Register the module from `pathfinding/mod.rs` as `pub(crate) mod
   bridge_markers;`.
6. Add unit tests for `toggle` insert/remove and `SmallPathDirs` max-24 cap.

**Verification:**

```text
cargo test bridge_marker_overlay_toggle
```

Do not proceed until the module tests compile.

### Task 2: Add overlay cost hook to A*

**Why:** The cost consumer is independent of peer snapshot construction. Getting
the 4x cost hook correct first gives later tasks a simple target.

**Files:**
- Modify: [src/sim/pathfinding/core.rs](../../src/sim/pathfinding/core.rs)
- Modify: [src/sim/pathfinding/core_tests.rs](../../src/sim/pathfinding/core_tests.rs)

**Steps:**

1. Import `BridgeMarkerContext` / `BridgeCostOverlay`.
2. Extend `AStarOptions` with `bridge_marker_context: Option<&BridgeMarkerContext<'_>>`.
3. Temporarily support an already-built overlay in tests if useful, but keep the
   production path as "context -> overlay inside `astar_search`".
4. In `astar_search`, initialize `let bridge_overlay = ...` after start/goal
   trivial gates and before the main loop.
5. For this task, allow `BridgeMarkerContext` to carry a test-only explicit
   overlay or builder stub so edge-cost behavior can be tested without movement
   snapshots.
6. Apply `step_cost *= 4` when `(nx, ny)` is marked. Apply before
   `DIR_TIEBREAK[dir_index]`.
7. Add tests:
   - `bridge_overlay_multiplies_destination_cost_by_four`
   - `bridge_overlay_is_search_scoped_after_success`
   - `bridge_overlay_is_search_scoped_after_no_path`
   - `bridge_overlay_does_not_hard_block_marked_cells`

**Verification:**

```text
cargo test bridge_overlay_
```

### Task 3: Thread marker context through all pathfinding wrappers

**Why:** Mechanical API threading should be separated from marker generation, so
compile errors are easier to isolate.

**Files:**
- Modify: [src/sim/pathfinding/core.rs](../../src/sim/pathfinding/core.rs)
- Modify: [src/sim/pathfinding/zone_search.rs](../../src/sim/pathfinding/zone_search.rs)
- Modify: current call sites found by `rg "find_path_with_costs|find_path_zoned|find_layered_path_zoned"`

**Steps:**

1. Add the optional context as the last parameter to:
   - `find_path_with_costs`
   - `find_path_with_costs_corridor`
   - `find_layered_path`
   - `find_path_zoned`
   - `find_layered_path_zoned`
2. Forward the context into every `AStarOptions` literal.
3. In `zone_search.rs`, forward the same context through every direct,
   fallback, and corridor A* call.
4. Pass `None` at all non-movement call sites.
5. Keep this task behavior-neutral except for tests that explicitly pass an
   overlay/context.

**Verification:**

```text
cargo check
cargo test bridge_overlay_
```

### Task 4: Add zone-search lifetime tests

**Why:** These tests guard the most likely architectural regression: marker state
being owned by the wrong layer.

**Files:**
- Modify: [src/sim/pathfinding/zone_search_tests.rs](../../src/sim/pathfinding/zone_search_tests.rs)

**Steps:**

1. Add `bridge_overlay_applies_inside_corridor_astar`.
2. Add `bridge_overlay_not_created_for_zone_precheck_abort`.
3. Add `bridge_overlay_rebuilt_per_corridor_retry`.
4. Use a small fixture grid and a test marker context with deterministic marks.

**Verification:**

```text
cargo test bridge_overlay_ --test '*'
```

Use the repo's actual test invocation pattern if integration/unit tests require
a different command.

### Task 5: Implement peer eligibility and replay in `bridge_markers`

**Why:** This is the fully sourced core parity behavior and should land before
movement plumbing.

**Files:**
- Modify: [src/sim/pathfinding/bridge_markers.rs](../../src/sim/pathfinding/bridge_markers.rs)
- Modify: [src/sim/pathfinding/core_tests.rs](../../src/sim/pathfinding/core_tests.rs)

**Steps:**

1. Implement `build_bridge_cost_overlay` for peer path replay, excluding the
   fallback 5x5 phase for now.
2. Apply peer gate:
   - urgency `0`: no overlay.
   - urgency `1`: same type skipped; require strict `mover.type_speed >
     peer.type_speed`; require playfield-valid path start.
   - urgency `2`: bypass same-type, speed, and playfield checks.
3. Respect kind prerequisites:
   - kind `1`: path entries `[0]` and `[1]` must be valid before replay.
   - kind `0xF`: entries `[0]`, `[1]`, and `[2]` must be valid before replay.
4. Replay from `path[0]`.
5. Stop at 24 processed entries or `-1`.
6. Implement direction `0..7` using the verified table.
7. Implement direction `8` only where the context/grid can provide valid tube
   exit metadata. Add the no-tube `(0,0)` behavior in fixtures.
8. Toggle destination cells.
9. Add tests:
   - `bridge_peer_markers_require_strict_type_speed_priority`
   - `bridge_peer_markers_urgency2_bypasses_speed_priority_gate`
   - `bridge_peer_path_replay_caps_at_24_entries`
   - `bridge_peer_path_uses_gamemd_direction_order`
   - `bridge_peer_path_direction8_marks_tube_exit`
   - `bridge_peer_path_direction8_without_tube_marks_origin`
   - `bridge_overlay_toggle_duplicate_marks_cancel`

**Verification:**

```text
cargo test bridge_peer_
cargo test bridge_overlay_toggle_duplicate_marks_cancel
```

### Task 6: Wire movement snapshot construction

**Why:** A* now understands the overlay; movement must supply deterministic,
borrow-safe inputs from actual entities.

**Files:**
- Modify: [src/sim/movement/mod.rs](../../src/sim/movement/mod.rs)
- Modify: [src/sim/movement/movement_tick.rs](../../src/sim/movement/movement_tick.rs)
- Modify: [src/sim/movement/movement_blocked.rs](../../src/sim/movement/movement_blocked.rs)
- Modify: [src/sim/movement/movement_path.rs](../../src/sim/movement/movement_path.rs)
- Modify: [src/sim/movement/movement_tests.rs](../../src/sim/movement/movement_tests.rs)

**Steps:**

1. Extend `MoverSnapshot` with mover entity id, type identity, type speed,
   current cell, current layer, and on-bridge state.
2. Add a helper that builds `BridgeMarkerSnapshot` by iterating `EntityStore`
   deterministically.
3. Include peer type identity, type speed, layer, path start, and path directions.
4. Convert adjacent path cell deltas to direction ids using the verified table.
5. Do not infer direction `8` from arbitrary non-adjacent path cells. Only emit
   `8` when explicit tube path state exists; otherwise leave a named TODO and
   omit that direction from live snapshots.
6. Thread `Option<&BridgeMarkerContext>` through `find_move_path`.
7. Build contexts for normal path requests and blocked repaths using the existing
   urgency value.
8. Add tests:
   - `movement_bridge_marker_snapshot_order_is_entity_id_stable`
   - `movement_bridge_marker_uses_type_speed_not_runtime_speed`
   - `movement_bridge_marker_does_not_hard_block_marked_cells`

**Verification:**

```text
cargo test movement_bridge_marker_
cargo check
```

### Task 7: Implement fallback probe and 5x5 marker phase

**Why:** This completes the full `0x0042ACF0` marker model, but it depends on
timer/probe clarity and occupancy-layer data.

**Files:**
- Modify: [src/sim/pathfinding/bridge_markers.rs](../../src/sim/pathfinding/bridge_markers.rs)
- Modify: [src/sim/movement/movement_tick.rs](../../src/sim/movement/movement_tick.rs)
- Modify: [src/sim/pathfinding/core_tests.rs](../../src/sim/pathfinding/core_tests.rs)

**Steps:**

1. Add `timer_current: u32` to the live `BridgeMarkerContext` if not already
   present.
2. Prove and document the Rust source for `timer_current`. If not proven, keep
   live fallback disabled and add a test-only timer input for fixture coverage.
3. Compute probe direction:

```rust
let dir = (((timer_current >> 12) + 1) >> 1) & 7;
```

4. Implement probe cell and selected layer/list logic using structural bridge
   metadata, level difference `> 3`, and mover `on_bridge`.
5. Implement the fallback object lookup as a deterministic helper. If the exact
   `0x0042B080` subobject predicate is not representable, name the gap in the
   helper and constrain the approximation to available normal entity data.
6. Implement no-peer urgency-1 early return before fallback markers.
7. Implement 5x5 occupation toggles:
   - scan offsets `-2..=2`;
   - skip unoccupied cells;
   - skip the searcher's current cell;
   - toggle occupied candidates;
   - toggle probe center unconditionally afterward.
8. Add tests:
   - `bridge_fallback_urgency1_no_peer_produces_no_overlay`
   - `bridge_fallback_5x5_center_toggle_rules`
   - `bridge_fallback_uses_timer_probe_direction`
   - `bridge_fallback_selected_layer_respects_bridge_level_gap`

**Verification:**

```text
cargo test bridge_fallback_
```

### Task 8: Run focused and broad checks

**Why:** The change cuts across core A*, zone wrappers, and movement plumbing.

**Commands:**

```text
cargo fmt
cargo test bridge_overlay_
cargo test bridge_peer_
cargo test bridge_fallback_
cargo test movement_bridge_marker_
cargo test zone_search
cargo test pathfinding
cargo check
```

If package names are required, use the names from `Cargo.toml`. If unrelated
pre-existing failures appear, record them and do not fix unrelated files.

### Task 9: Manual parity scenario notes

**Why:** This is movement behavior; unit tests prove mechanics, but the player
sees route choice.

**Scenario to document after implementation:**

1. Create or use a map with a narrow bridge approach.
2. Put a slower peer unit on a queued bridge approach path.
3. Path a faster unit nearby with normal urgency and observe it prefers the
   unmarked alternative when costs tie closely.
4. Repeat with equal-speed/same-type peer and confirm the extra avoidance does
   not trigger at normal urgency.
5. Repeat with blocked repath urgency `2` and confirm the bypassed gate can mark
   same-type/equal-speed peer paths.

Record the result in the implementation PR or final work summary.

## Definition of Done

- [ ] `BridgeCostOverlay` exists and toggles cells rather than set-only marking.
- [ ] `astar_search` applies a 4x destination-cell multiplier from the overlay.
- [ ] `PathGrid` and `ZoneGrid` remain unchanged by marker generation.
- [ ] Path wrappers thread `Option<&BridgeMarkerContext>` without changing
      non-movement callers.
- [ ] Peer marker generation matches strict type speed gate, same-type skip,
      urgency-2 bypass, 24-entry cap, `-1` terminator, direction table, direction
      `8`, and duplicate toggle behavior.
- [ ] Movement snapshot construction is deterministic and uses type `Speed=`,
      not runtime speed.
- [ ] Fallback probe/5x5 behavior is implemented or explicitly gated with the
      unresolved `RateTimer__Current` mapping named.
- [ ] Focused pathfinding, zone, and movement tests pass.
- [ ] `cargo fmt` and `cargo check` complete, or unrelated pre-existing failures
      are documented.
