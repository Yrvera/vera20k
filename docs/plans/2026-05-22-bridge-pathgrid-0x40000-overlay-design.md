# Bridge Pathgrid 0x40000 Overlay - Full Parity Design

## Goal

Implement the gamemd.exe temporary bridge/pathgrid cost-marker behavior around
`PathfinderClass::UpdateBridgePassability @ 0x0042ACF0`.

The player-visible target is movement path choice near occupied bridge approaches:
units should treat specific peer path cells and fallback probe cells as expensive
during one A* search, causing traffic to route around bridge congestion the same
way Yuri's Revenge does. This is not a walkability, occupancy, render, or bridge
damage flag.

## Architecture Context

### Current Rust pathfinding shape

- `src/sim/pathfinding/core.rs` owns `PathGrid`, A*, entity soft-block costs,
  bridge-layer routing, tube edges, and the public `find_path_with_costs` /
  `find_layered_path` wrappers.
- `AStarOptions` already carries transient search inputs:
  `terrain_costs`, hard `entity_blocks`, `entity_block_map`, `urgency`,
  `mover_is_crusher`, `movement_zone`, `resolved_terrain`, and optional corridor
  state.
- `src/sim/pathfinding/zone_search.rs` wraps the cell-level A* in zone prechecks
  and corridor retries. It repeatedly calls `find_path_with_costs`,
  `find_path_with_costs_corridor`, or `find_layered_path`.
- `src/sim/movement/movement_path.rs` is the main movement-facing path request
  surface. It receives `entity_block_map`, `urgency`, mover movement zone, and
  terrain context, then forwards them into zone search.
- `src/sim/movement/mod.rs` and `movement_tick.rs` already use a
  `MoverSnapshot` pattern to preserve mover data across borrow boundaries.
- `src/rules/object_type.rs` already carries parsed `Speed=`, which is the field
  gamemd uses for the peer marker priority gate.

### Current mismatch

Rust has entity soft-block costs matching the `AStar_compute_edge_cost` code-2,
code-5, and code-6 paths, but it does not have an equivalent for
`CellClass+0x140 & 0x40000`. In gamemd this marker is toggled before the A*
body, read during cost evaluation as a 4x destination-cell multiplier, and toggled
back off on normal success or failure.

The Rust `PathGrid` is persistent terrain/bridge state. That is the correct base
model, but `0x40000` must not be stored there because gamemd's ordinary lifecycle
is search-local.

### Synthesis outputs

No relevant `*_SYSTEM_MODEL_SYNTHESIS.md` file exists for this pathgrid system.
The design therefore cites the underlying Ghidra reports directly.

## Impact Analysis

Touches:

- `src/sim/pathfinding/core.rs` - add the search-local overlay type, cost
  multiplier hook, and A* option plumbing.
- `src/sim/pathfinding/mod.rs` - expose the new pathfinding-internal marker
  types if they are split into a new module.
- `src/sim/pathfinding/zone_search.rs` - thread an optional marker context through
  every fallback, direct, and corridor A* call.
- `src/sim/movement/movement_path.rs` - pass the per-mover marker context from
  movement into zone search.
- `src/sim/movement/movement_tick.rs` / `movement_blocked.rs` - construct the
  marker context for normal pathfinds and blocked repaths with the correct
  urgency.
- `src/sim/movement/mod.rs` - likely extend pathfinding context or add a
  per-request argument, plus expand `MoverSnapshot` with mover type identity and
  type speed.
- `src/sim/pathfinding/core_tests.rs` and `zone_search_tests.rs` - focused A*
  parity fixtures.
- `src/sim/movement/movement_tests.rs` - movement-level snapshot/plumbing tests.

Does not touch:

- `PathGrid` persistent cell storage, save data, bridge damage state, rendering,
  sidebar, audio, UI, or network code.
- `ZoneGrid` persistent connectivity. Zone search may route the A* call, but it
  must not own or persist bridge marker state.

Blast radius:

- All movement path requests that currently pass `urgency > 0` can start seeing
  extra 4x destination costs near bridge approaches.
- A* tie outcomes can change where the overlay is active. This is intended.
- Incorrect peer iteration order would cause deterministic but wrong paths. Peer
  snapshots must be built from the deterministic `EntityStore` order or another
  explicitly ordered source.
- Building the overlay outside A* would risk applying it to precheck/early-return
  paths where gamemd never toggled it. The overlay should be constructed inside
  the A* body after the same trivial-return gates have passed.

Determinism:

- Use fixed integer costs only. The 4x multiplier is integer `* 4`, matching the
  existing `STEP_COST` scaling style.
- Do not consume `SimRng`. The initial probe direction comes from gamemd's
  `RateTimer__Current` expression, so Rust needs a deterministic timer-equivalent
  input. If no existing field is already proven equivalent, implementation must
  add a named `bridge_marker_timer_current` input and document the mapping before
  enabling the fallback probe in live play.
- Use `BTreeMap` / `BTreeSet` or sorted vectors for overlay and snapshot order.

## Tiny-Detail Ledger

- `0x40000` is the active A* temporary cost bit; `0x400` is a separate bridge
  marker/render/placement bit and is not the A* cost marker.
  Source: `CELLCLASS_0X140_0X400_GLOBAL_XREF_CENSUS_GHIDRA_REPORT.md`,
  `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`.
- `0x40000` is toggled around an ordinary A* search and cleaned on both normal
  success and normal failure. It is not persistent `PathGrid` state.
  Source: `ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md`.
- The cost consumer multiplies the destination cell edge cost by `4.0` when the
  destination cell has `0x40000`.
  Source: `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`,
  `AStar_compute_edge_cost @ 0x00429830`.
- The pre-toggle path is gated by nontrivial path search and
  `PathfinderClass+0x3C != 0`.
  Source: `ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md`.
- Normal peer eligibility skips same `TechnoType` and requires
  `searcher.TechnoType.Speed > peer.TechnoType.Speed`; equality skips.
  Source: `PATHFINDER_0042ACF0_OBJECT_0X678_GATE_GHIDRA_REPORT.md`.
- `TechnoTypeClass+0x678` is parsed/scaled `Speed=`, not owner, veterancy,
  runtime speed, or object id.
  Source: `PATHFINDER_0042ACF0_OBJECT_0X678_GATE_GHIDRA_REPORT.md`.
- Urgency `2` bypasses the same-type, speed, and playfield gate.
  Source: `PATHFINDER_0042ACF0_OBJECT_0X678_GATE_GHIDRA_REPORT.md`.
- Peer path replay processes at most 24 entries and stops at `-1`.
  Source: `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`.
- Direction entries `0..7` use table order `N, NE, E, SE, S, SW, W, NW` with
  signed offsets `(0,-1), (1,-1), (1,0), (1,1), (0,1), (-1,1), (-1,0),
  (-1,-1)`.
  Source: `GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`.
- Direction `8` is a tube jump. It reads current `Cell+0x116`; `-1` maps to
  `(0,0)`, otherwise `g_TubeArray[idx]+0x28` supplies the next coordinate.
  Source: `GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`.
- The direction-8 consumer has only a `-1` guard, not a positive upper-bound
  check. Rust may validate tube data earlier for safety, but that is an input
  sanitation boundary, not gamemd consumer behavior.
  Source: `GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`.
- Marker writes toggle, not set. Duplicate writes to the same cell can cancel.
  Source: `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`,
  `GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`.
- If no peer path was processed and urgency is `1`, gamemd clears the urgency
  field and returns before writing fallback markers. Rust should model this as
  "no overlay produced for that path" rather than as cleanup by reset.
  Source: `ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md`.
- The fallback phase scans a 5x5 square around the pseudo-random probe cell,
  toggles occupied non-self candidates, skips the searching unit's current cell,
  then toggles the probe center unconditionally.
  Source: `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`.
- An occupied probe-center cell is toggled once by the candidate loop and once
  by the final center toggle, so its net marker state is unchanged. An unoccupied
  center is toggled once.
  Source: `PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`.
- The marker is cost-only. It must not block movement, alter reachability zones,
  or change occupancy.
  Source: all three `0x40000` pathfinder reports above.

## Chosen Approach

Use an explicit per-search `BridgeCostOverlay`, built lazily inside `astar_search`
from a deterministic `BridgeMarkerContext`.

This is the best fit because it matches the observed gamemd lifecycle without
mutating persistent pathgrid state. Movement supplies snapshots of the searcher
and nearby peers; A* decides whether the current search actually needs the
overlay and applies it only to edge-cost calculation.

### Why not the alternatives

- Cloning and mutating `PathGrid` would make the marker look like terrain or
  bridge state. It also risks accidental persistence through retries and costs
  more memory on large maps.
- Computing markers lazily through callbacks from the A* edge-cost loop would
  tangle A* with `EntityStore`, movement internals, and borrow-heavy peer lookup.
  It is harder to make deterministic and harder to test.
- Reusing `entity_block_map` would conflate different binary mechanisms.
  Entity soft-block codes 2/5/6 and the `0x40000` bridge marker are read by the
  same broad cost function, but they have different producers, gates, and
  multipliers.

## Design

### Components

Add a new pathfinding-internal module:

```rust
// src/sim/pathfinding/bridge_markers.rs
pub(crate) struct BridgeCostOverlay {
    marked: BTreeSet<(u16, u16)>,
}

pub(crate) struct BridgeMarkerContext<'a> {
    pub urgency: u8,
    pub timer_current: u32,
    pub mover: BridgeMarkerMover,
    pub peer_snapshot: &'a BridgeMarkerSnapshot,
}

pub(crate) struct BridgeMarkerMover {
    pub entity_id: u64,
    pub cell: (u16, u16),
    pub layer: MovementLayer,
    pub on_bridge: bool,
    pub type_id: InternedId,
    pub type_speed: i32,
}

pub(crate) struct BridgeMarkerPeer {
    pub entity_id: u64,
    pub cell: (u16, u16),
    pub layer: MovementLayer,
    pub on_bridge: bool,
    pub kind_code: u8,
    pub type_id: InternedId,
    pub type_speed: i32,
    pub path_start: (u16, u16),
    pub path_dirs: SmallPathDirs,
}

pub(crate) struct BridgeMarkerSnapshot {
    pub peers: Vec<BridgeMarkerPeer>,
    pub occupied_by_layer: BridgeMarkerOccupancy,
}
```

`SmallPathDirs` can be a fixed `[i8; 24]` plus length, or a `Vec<i8>` capped at
24 during snapshot construction. The fixed array better expresses the gamemd
limit and avoids allocation, but either is acceptable if tests enforce the cap.

`BridgeCostOverlay` should provide:

```rust
impl BridgeCostOverlay {
    pub(crate) fn is_marked(&self, cell: (u16, u16)) -> bool;
    fn toggle(&mut self, cell: (u16, u16));
}
```

`toggle` removes an already-marked cell and inserts an unmarked cell. This is
required for duplicate marker parity.

### Data flow

```text
movement_tick
  builds MoverSnapshot
  builds deterministic BridgeMarkerSnapshot from EntityStore / occupancy
  computes BridgeMarkerContext for this path request
  calls movement_path::find_move_path(..., bridge_marker_context)

movement_path::find_move_path
  forwards bridge_marker_context into zone_search

zone_search
  forwards bridge_marker_context into every final cell-level A* call
  does not create, clear, or store overlays

core::astar_search
  performs normal start/goal/trivial gates
  if urgency != 0 and the search reaches the main A* body:
      build BridgeCostOverlay from BridgeMarkerContext
  during edge-cost calculation:
      if destination cell is in overlay:
          step_cost *= 4
  overlay drops at return
```

The overlay is local stack-owned state inside `astar_search`. There is no cleanup
API because Rust drops the overlay when the search returns. Tests must still prove
that no persistent pathgrid or zone state changes.

### AStarOptions

Extend `AStarOptions`:

```rust
pub struct AStarOptions<'a> {
    // existing fields...
    pub(crate) bridge_marker_context: Option<&'a BridgeMarkerContext<'a>>,
}
```

Every public wrapper receives an optional marker context and forwards it into the
`AStarOptions` literal:

- `find_path_with_costs`
- `find_path_with_costs_corridor`
- `find_layered_path`
- any layered/corridor wrapper that constructs `AStarOptions`

Non-movement callers pass `None`.

### Overlay construction

`build_bridge_cost_overlay` receives:

- `grid: &PathGrid`
- `resolved_terrain: Option<&ResolvedTerrainGrid>` for tube exits
- current start cell, start layer, start height, and mover bridge state
- `BridgeMarkerContext`

It returns `None` or an empty overlay when gamemd would not write markers.

Required behavior:

1. If `context.urgency == 0`, return no overlay.
2. Compute the pseudo-random probe direction from `timer_current`:

   ```rust
   let dir = (((timer_current >> 12) + 1) >> 1) & 7;
   ```

3. Probe `mover.cell + DIRECTION_OFFSETS[dir]`.
4. Select the probe object-list layer using bridge bit/level/mover-on-bridge
   semantics:
   - Ground list when the probe is not structural bridge.
   - Ground list when the absolute level difference is `<= 3` and the mover is
     not already on a bridge.
   - Bridge list otherwise.
5. Find peers in the selected layer/list at the probe cell.
6. If that list is empty, run the 5x5 fallback object lookup equivalent. Rust
   does not need to copy the legacy vtable shape, but it must produce the same
   candidate choice for normal entity data: scan the same 5x5 area, respect the
   requested height/layer choice, and choose the first deterministic matching
   object.
7. For each candidate peer of kind `1` or `0xF`, apply the peer eligibility gate:
   - urgency `2`: bypass same-type, speed, and playfield gate.
   - otherwise: skip same type; require `mover.type_speed > peer.type_speed`;
     require peer path start in playfield.
8. For kind `1`, require at least two valid path entries before replay.
9. For kind `0xF`, require at least three valid path entries before replay.
10. Replay from `path[0]`, not from `path[1]`.
11. Process at most 24 entries and stop on `-1`.
12. For directions `0..7`, add the signed direction offset.
13. For direction `8`, use current cell tube metadata:
    - no tube (`-1` / `None`) maps to `(0,0)`;
    - valid tube id maps to `TubeFact.exit`;
    - invalid positive tube id should be impossible after Rust data validation,
      but should be logged/asserted as invalid fixture data rather than silently
      described as gamemd behavior.
14. Toggle the destination cell in the overlay after each replayed coordinate.
15. Track whether any peer path marker was processed.
16. If no peer path was processed and urgency is `1`, return the overlay as empty
    without running the fallback marker phase.
17. Otherwise run the 5x5 occupation fallback:
    - offsets `-2..=2` in both axes around the probe cell;
    - toggle occupied candidate cells;
    - skip the searcher's own current cell;
    - toggle the probe center unconditionally after the loop.

The initial implementation should keep the object-selection helper small and
testable. If the exact `0x0042B080` height/subobject predicate cannot be matched
from current Rust entity data, the implementation should isolate that gap behind
a clearly named helper and add a TODO citing the report. It should not replace
the whole behavior with "all nearby peers".

### Edge-cost integration

In `astar_search`, after terrain, cliff, and entity soft-block multipliers have
computed `step_cost`, apply the overlay multiplier:

```rust
if bridge_overlay
    .as_ref()
    .is_some_and(|overlay| overlay.is_marked((nx, ny)))
{
    step_cost *= 4;
}
```

The marker is read on the destination cell. It applies before the direction
tie-breaker is added.

The marker must not bypass or modify:

- walkability checks;
- hard entity blocks;
- zone corridor filtering;
- bridge layer selection;
- tube expansion legality;
- path smoothing.

Path smoothing should continue to avoid soft-blocked shortcuts via
`entity_block_map`. The bridge marker is only a cost input to A*, so smoothing
does not need to inspect it after the path is chosen.

### Movement snapshot construction

Movement must build a marker snapshot without introducing nondeterminism or
borrow conflicts.

Recommended structure:

1. Extend `MoverSnapshot` with:
   - `entity_id`;
   - type identity;
   - type `Speed=`;
   - current cell;
   - current movement layer / on-bridge state.
2. Build `BridgeMarkerSnapshot` from immutable `EntityStore` access before
   taking a mutable reference to the mover, or after releasing it, following the
   existing snapshot pattern in `movement_tick.rs`.
3. Iterate `EntityStore` in its natural `BTreeMap<u64, GameEntity>` order.
4. Include only entities with the path/locomotor state needed to expose a queued
   direction path. If current Rust stores path cells rather than direction bytes,
   convert adjacent cell deltas to gamemd direction ids using the verified table.
   Direction `8` must come from explicit tube path state, not from "non-adjacent
   cell happened" heuristics unless that mapping is separately verified.
5. Build `occupied_by_layer` from the same layer split used for
   `LayeredEntityBlockMap`, so stacked ground/bridge occupants do not collapse
   into one coordinate.

If current movement state cannot recover peer path directions faithfully, add a
small movement-facing cache of the last planned segment as direction ids. That
cache is deterministic state and must be included in serialization/hash only if
it is stored on entities. Prefer deriving it from existing `MovementTarget.path`
when possible.

### Timer input

The fallback probe direction depends on `RateTimer__Current`, not random state.
The design requires a single named input:

```rust
pub(crate) struct BridgeMarkerContext<'a> {
    pub timer_current: u32,
    // ...
}
```

Implementation must either:

- prove an existing Rust tick/time value is the intended equivalent and document
  that in code/tests; or
- temporarily gate only the probe/fallback branch behind an `UNKNOWN - needs RE`
  TODO while still implementing peer path overlays.

Do not consume `SimRng` for this. It would perturb unrelated deterministic systems.

## Interfaces

Likely signature changes:

```rust
pub fn find_path_with_costs(
    grid: &PathGrid,
    start: (u16, u16),
    goal: (u16, u16),
    costs: Option<&TerrainCostGrid>,
    entity_blocks: Option<&BTreeSet<(u16, u16)>>,
    movement_zone: Option<MovementZone>,
    resolved_terrain: Option<&ResolvedTerrainGrid>,
    entity_block_map: Option<&LayeredEntityBlockMap>,
    urgency: u8,
    mover_is_crusher: bool,
    bridge_marker_context: Option<&BridgeMarkerContext<'_>>,
) -> Option<Vec<(u16, u16)>>;
```

`find_path_with_costs_corridor`, `find_layered_path`, `find_path_zoned`, and
`find_layered_path_zoned` receive the same final optional parameter.

`movement_path::find_move_path` receives:

```rust
bridge_marker_context: Option<&BridgeMarkerContext<'_>>
```

and forwards it to both layered and flat path calls.

## Testing Strategy

Add low-level pathfinding tests first, then movement plumbing tests.

Core tests:

- `bridge_overlay_is_search_scoped_after_success` - path search with overlay
  succeeds; a second search over the same `PathGrid` sees no marker residue.
- `bridge_overlay_is_search_scoped_after_no_path` - no-path failure drops the
  overlay and leaves `PathGrid` unchanged.
- `bridge_overlay_multiplies_destination_cost_by_four` - a marked destination
  cell changes route choice where an unmarked equal path would be chosen.
- `bridge_overlay_toggle_duplicate_marks_cancel` - two writes to the same cell
  remove the mark.
- `bridge_peer_markers_require_strict_type_speed_priority` - same type, equal
  speed, and slower searcher do not mark under normal urgency.
- `bridge_peer_markers_urgency2_bypasses_speed_priority_gate` - same-type peer
  can mark under urgency `2`.
- `bridge_peer_path_replay_caps_at_24_entries` - entry 25 is ignored.
- `bridge_peer_path_uses_gamemd_direction_order` - `0..7` replay maps to the
  verified N, NE, E, SE, S, SW, W, NW offsets.
- `bridge_peer_path_direction8_marks_tube_exit` - direction `8` marks
  `TubeFact.exit`.
- `bridge_peer_path_direction8_without_tube_marks_origin` - no tube marks
  `(0,0)` instead of doing nothing.
- `bridge_fallback_urgency1_no_peer_produces_no_overlay` - no peer path and
  urgency `1` returns before fallback marking.
- `bridge_fallback_5x5_center_toggle_rules` - occupied center nets unchanged,
  unoccupied center toggles once, own current cell is skipped.

Zone/corridor tests:

- `bridge_overlay_applies_inside_corridor_astar` - corridor A* still sees the
  cost marker.
- `bridge_overlay_not_created_for_zone_precheck_abort` - cross-zone precheck
  failure returns before any overlay is built.
- `bridge_overlay_rebuilt_per_corridor_retry` - retry attempts do not reuse stale
  overlay state from a failed cell A*.

Movement tests:

- `movement_bridge_marker_snapshot_order_is_entity_id_stable` - peer snapshot
  order follows deterministic entity order.
- `movement_bridge_marker_uses_type_speed_not_runtime_speed` - changing runtime
  speed does not affect the gate; changing type speed does.
- `movement_bridge_marker_does_not_hard_block_marked_cells` - marked cells remain
  enterable if A* chooses them.

Run:

```text
cargo test -p ra2-rust-game pathfinding::core_tests::bridge_
cargo test -p ra2-rust-game pathfinding::zone_search_tests::bridge_
cargo test -p ra2-rust-game movement::movement_tests::movement_bridge_marker_
cargo fmt
```

Use the actual package name from `Cargo.toml` if it differs.

## Implementation Plan

1. Add `bridge_markers.rs` with `BridgeCostOverlay`, marker context types, the
   verified direction table, and pure unit tests for toggle/replay behavior.
2. Extend `AStarOptions` and apply the 4x destination multiplier in `astar_search`.
3. Thread `Option<&BridgeMarkerContext>` through flat, layered, zoned, and
   corridor path wrappers with `None` at all non-movement call sites.
4. Build movement-side `BridgeMarkerSnapshot` from deterministic entity snapshots.
5. Implement peer eligibility and path replay.
6. Implement probe object-list selection and the 5x5 fallback. If the exact timer
   equivalent is not yet proven, keep this branch behind a named disabled gap
   until the timer mapping is resolved.
7. Add the focused tests above.
8. Run formatting and targeted tests.

## Definition of Done

- [ ] A* has a search-local bridge marker overlay and never mutates `PathGrid` for
      `0x40000`.
- [ ] Destination edge costs are multiplied by `4` when the overlay marks the
      destination cell.
- [ ] Peer marker generation preserves same-type skip, strict `Speed=` ordering,
      and urgency `2` bypass.
- [ ] Peer path replay preserves the 24-entry cap, `-1` terminator, direction
      table order, direction `8` tube behavior, and toggle semantics.
- [ ] Fallback marking preserves the urgency-1 no-peer early return, 5x5
      occupation gates, own-cell skip, and unconditional center toggle.
- [ ] Zone search forwards the marker context but does not persist or own marker
      lifetime.
- [ ] Movement builds deterministic peer snapshots without consuming RNG.
- [ ] Focused pathfinding, zone, and movement tests pass.
- [ ] `cargo fmt` has been run.

## Sources

- `docs/research/PATHFINDER_UPDATE_BRIDGE_PASSABILITY_0042ACF0_GHIDRA_REPORT.md`
- `docs/research/ASTAR_0X40000_CLEANUP_TAILS_GHIDRA_REPORT.md`
- `docs/research/GDIRECTIONOFFSETS_0089F688_BRIDGE_MARKER_PATH_GHIDRA_REPORT.md`
- `docs/research/PATHFINDER_0042ACF0_OBJECT_0X678_GATE_GHIDRA_REPORT.md`
- `docs/research/CELLCLASS_0X140_0X400_GLOBAL_XREF_CENSUS_GHIDRA_REPORT.md`
