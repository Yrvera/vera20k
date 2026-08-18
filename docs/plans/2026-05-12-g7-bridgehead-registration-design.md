# G7 Bridgehead Registration — Design

## Goal

Eliminate the asymmetry where bridgehead cells silently lose `bridge_walkable`
after every `rebuild_dynamic_path_grid` by registering them in
`BridgeRuntimeState` as `BridgeCellRole::Bridgehead` cells.

## Architecture Context

Three structures own "bridge-walkable" facts today:

1. **`ResolvedTerrainCell` (static, set at map load)** — owns `bridge_walkable`,
   `has_bridge_deck`, `bridge_transition`, `bridge_deck_level`. Built once in
   [`resolved_terrain.rs`](../../src/map/resolved_terrain.rs). Bridgeheads here
   have `bridge_walkable=true`, `bridge_transition=true`,
   `has_bridge_deck=false` (set in the bridgehead-detection pass at lines
   538–599).

2. **`BridgeRuntimeState` (live, in `sim/bridge_state/`)** — built from
   resolved terrain at sim init. Currently registers ONLY cells with
   `has_bridge_deck=true` as `BridgeRuntimeCell` records (pass 1, lines
   460/477). Owns runtime `damage_state`, `deck_present`, anchor spans.
   `is_bridge_walkable(rx, ry)` returns `cell_record.exists && deck_present
   && !destroyed` — false for any cell not in `cells[]`.

3. **`PathCell` (rebuilt on refresh signals)** — built by
   `PathGrid::from_resolved_terrain_with_bridges`
   ([`core.rs:976-1025`](../../src/sim/pathfinding/core.rs#L976)).
   `bridge_walkable` is unconditionally overridden from
   `bridge_state.is_bridge_walkable()` if `bridge_state` is `Some`.
   `rebuild_dynamic_path_grid` fires on `bridge_state_changed |
   destroyed_structure | ownership_changed | spawned_entities`
   ([`app_sim_tick.rs:549-557`](../../src/app_sim_tick.rs#L549)).

The asymmetry: bridgeheads live in #1 with `bridge_walkable=true`, but are
absent from #2. So #3's rebuild flips their `bridge_walkable` from
`true → false` on every refresh.

All A* / movement-tick consumers read `PathCell.bridge_walkable` — so the
bug manifests as soon as the first refresh fires (which happens very early
in a match, since unit spawns trigger it).

## Impact Analysis

**Touched modules:**

- [`src/sim/bridge_state/mod.rs`](../../src/sim/bridge_state/mod.rs) —
  new pass in `from_resolved_terrain` (~30 lines).
- [`src/sim/pathfinding/core_tests.rs`](../../src/sim/pathfinding/core_tests.rs) —
  fix existing test fixtures that paper over the bug with unrealistic
  `has_bridge_deck=true` on bridgeheads.
- New test cases (in `bridge_state/mod.rs` and possibly `world_tests.rs`)
  asserting the cross-rebuild invariant.

**Depends on what we're changing:**

- All downstream A* / movement-tick consumers of `PathCell.bridge_walkable`
  benefit. No interface changes.
- `bridge_state` consumers (walkers, damage state machines, repair routines)
  must not accidentally process bridgehead cells as body cells. The role
  enum already has `Bridgehead`; consumers must respect it. This is the
  primary risk and requires a walker-safety audit at /write-plan time.

**Risk areas:**

1. Damage walkers iterating `cells.iter()` may unintentionally process
   bridgehead records. Mitigation: explicit role-skip checks at iteration
   sites, plus a "bridgeheads never enter Damaged" regression test.
2. `endpoint_records` / `group_cells` semantics — bridgeheads are NOT in a
   bridge group, but they ARE bridge-related. Need to confirm consumers
   that iterate endpoint_records don't expect bridgeheads to be present
   (and vice versa).
3. `state_hash` (determinism) — adding cells to the registry changes
   serialization order if any hashing iterates `cells`. Need to verify.

**Determinism:** index-based `Vec<Option<BridgeRuntimeCell>>` storage, no
new iteration patterns, no float math. Safe assuming the walker audit comes
clean.

**Migration:** none — this is a pure data fix, no save-format change, no
public API change.

## Chosen Approach

Register bridgeheads in `BridgeRuntimeState` via a new pass that uses
`ResolvedTerrainCell.bridge_walkable && !has_bridge_deck` as the
detection signal. Picked over the one-line "preserve-static in rebuild"
shape because both produce identical observable output, but registration
mirrors gamemd's internal data model (cell.Flags carries both bit 0x80 and
the BridgeHead flag — unified per-cell bridge metadata) and centralizes
ownership of bridge facts in `bridge_state/`.

## Tiny-Detail Ledger

Parity-relevant details the implementation must preserve. Sourced from
`BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md` and prior audit-log entries
(2026-05-11, 2026-05-12).

1. **`cell.Flags & 0x80`** = "this cell is part of a walkable bridge
   structure (deck)". Set ONLY on body/anchor cells; bridgeheads do NOT
   carry bit 0x80. [GHIDRA 0x47E040; audit 2026-05-12]

2. **Bridgeheads carry a SEPARATE `CellFlags::BridgeHead` flag** (not bit
   0x80). It is a static map-load flag and is NOT touched by
   `SetBridgeDirection`. [resolved_terrain.rs:540 comment; audit
   2026-05-12 confirms SetBridgeDirection's mask 0xFFFEE8FF preserves
   non-0x80 bits]

3. **`SetBridgeDirection` mutates bit 0x80 only on `param_1`** (the anchor
   it's called on), via `(param_3 & 1) << 7`. Neighbor cells' bit 0x80
   is preserved. [GHIDRA 0x47E040 NESW / 0x47E470 NWSE; audit 2026-05-11]

4. **`SetBridgeDirection` caller inventory (18 sites)**: 12 damage/repair
   (UpdateRamp_*_Collapse, UpdateBridgeEdgeTiles,
   ProcessBridgeDamageStateMachine), 3 map-load (`OverlayClass::Mark`
   for overlays 0x18/0x19/0xED/0xEE), 2 map-resize refresh. Bridgeheads
   never appear as `SetBridgeDirection` callees in damage/repair paths.
   [audit 2026-05-12 BRIDGE_DEFERRED_MECHANICS]

5. **Bridgehead walkability persists across bridge collapse in gamemd.**
   A bridgehead remains a walkable ramp tile after its bridge's deck
   collapses — it just leads to nothing. Derived from #2 + #3 + #4: no
   damage path touches the BridgeHead flag or bridgehead cells.

6. **Ground→Bridge entry** in A* requires `parent_height -
   neighbor.ground_level == 4` AND `neighbor.transition == true`.
   [`core.rs:147-148`; GHIDRA AStar_create_node 0x42A460]

7. **Bridge→Ground exit predicate**: `dst_h == src_h - 4 &&
   !dst.bridge_walkable && src.bridge_walkable`.
   [`movement_bridge.rs:67-68`; GHIDRA 0x4B0F20 audited 2026-05-11]

8. **`bridge_walkable` and `bridge_transition` are read together** for
   layer-transition gating. `can_enter_bridge_layer_from_ground` is
   `bridge_walkable && transition`. Splitting them silently changes A*
   behavior. [`core.rs:697`]

9. **`is_bridge_walkable(rx, ry)` semantics today** (must be preserved
   for body cells): `cell_record exists && deck_present && !destroyed`.
   Bridgeheads must return true for the same predicate.
   [`bridge_state/mod.rs:756-760`]

10. **`rebuild_dynamic_path_grid` fires on multiple non-bridge events**
    (building death, unit spawn, ownership change). Each currently flips
    bridgeheads' `bridge_walkable` true→false. Frequency = many times per
    minute in a normal skirmish. [`app_sim_tick.rs:549-557`]

11. **Bridgehead `deck_level`** at map load is taken from an adjacent
    bridge span cell so the bridgehead matches the normalized bridge
    height; falls back to `level + 4` if no neighbor.
    [`resolved_terrain.rs:556-571`]

12. **The detection signal `bw=true && has_deck=false` is
    bridgehead-exclusive.** Verified by tracing every write to
    `bridge_walkable` in `resolved_terrain.rs`: initial assignment ties
    bw to has_deck; side extrapolation sets both; gap fill sets both;
    only the bridgehead pass at line 583 sets bw without has_deck.

`UNKNOWN — needs RE` items (out of scope, tracked as follow-ups):

- Whether gamemd allows naval-shipyard placement under destroyed high
  bridge gaps (for the deferred `production_placement.rs:391` migration).
- Whether `TooBigToFitUnderBridge` naval units can path through destroyed
  high bridge gaps in gamemd (for the deferred `movement_path.rs:23-24`
  migration). gamemd's intact-bridge baseline is also not yet RE'd.

## Design

### Components

Single touch point: `BridgeRuntimeState::from_resolved_terrain` in
[`src/sim/bridge_state/mod.rs`](../../src/sim/bridge_state/mod.rs).

Add a new pass (after pass 3 today) that registers bridgehead cells as
`BridgeRuntimeCell` records with `role = BridgeCellRole::Bridgehead`.

Pass 3 today only RE-roles existing cells. Bridgeheads have no existing
cell at pass 3 (they were never created in pass 1), so pass 3 is a no-op
for them. The new pass 4 owns their creation end-to-end.

### Interfaces / Contracts

No public API change. `BridgeRuntimeState::is_bridge_walkable(rx, ry)`
keeps the same signature; its returned value for bridgeheads simply
flips from `false` (today) to `true` (post-fix).

The contract that becomes load-bearing after the fix:

> A `BridgeRuntimeCell` with `role == Bridgehead`:
> - has `deck_present = true` permanently
> - has `damage_state = Healthy { variant: 0 }` permanently
> - has `bridge_group_id = None`
> - has `anchor_span_id = None`
> - is NEVER mutated by damage walkers, repair walkers, state machines,
>   or `SetBridgeDirection` analogs
> - MUST be skipped by any code that iterates `cells.iter()` for
>   damage-related work

This contract is captured by the role enum value and enforced at consumer
sites (walker-safety audit at /write-plan time).

### Data Flow

```
map load
  → ResolvedTerrainGrid (static, bridgeheads: bw=true, has_deck=false)
  → BridgeRuntimeState::from_resolved_terrain:
      pass 1: register has_bridge_deck cells (body + anchor + gap-fill)
      pass 2: walk anchor patterns; tag body/tail/anchor roles
      pass 3: re-role non-anchor-overlay cells (existing cells only)
      pass 4 (NEW): register bridgeheads (bw && !has_deck) as Bridgehead
  → PathGrid::from_resolved_terrain_with_bridges:
      bridge_walkable = state.is_bridge_walkable(rx, ry)
      // now returns true for bridgeheads → PathCell.bridge_walkable = true

tick → bridge collapse:
  → damage event hits body cell
  → walker / state machine transitions body cell to Destroyed
  → bridgehead cells UNTOUCHED (skipped by role check)
  → rebuild_dynamic_path_grid:
      body cells: bw=false (Destroyed)
      bridgeheads: bw=true (Healthy, deck_present)

tick → unrelated event (building death, unit spawn):
  → rebuild_dynamic_path_grid (NO bridge state change):
      bridgeheads: bw=true (unchanged)
```

### Error Handling

None — this is pure data construction. Bridgeheads that fail to resolve
a `deck_level` from a neighbor fall back to `level + 4` (existing
behavior in `resolved_terrain.rs:571`); pass 4 inherits this via the
already-resolved `cell.bridge_deck_level`.

### Testing Strategy

**Unit tests in `bridge_state/mod.rs`:**

1. `test_bridgeheads_registered_with_bridgehead_role` — build a small
   terrain with one bridgehead cell (`bw=true, has_deck=false,
   transition=true`); assert `BridgeRuntimeState.cell(rx, ry)` returns
   `Some` with `role == Bridgehead`, `deck_present == true`,
   `damage_state == Healthy`.

2. `test_bridgehead_is_bridge_walkable_returns_true` — same fixture;
   assert `state.is_bridge_walkable(rx, ry) == true`.

3. `test_bridgehead_survives_body_cell_collapse` — terrain with one
   bridgehead and one body cell; transition body to Destroyed; assert
   bridgehead's `damage_state` is still Healthy and `is_bridge_walkable`
   returns true.

**Integration tests in `pathfinding/core_tests.rs`:**

4. New: `test_pathcell_bridge_walkable_preserved_for_bridgeheads_across_rebuild`
   — build a realistic bridgehead fixture; rebuild PathGrid via
   `from_resolved_terrain_with_bridges`; assert PathCell.bridge_walkable
   for bridgeheads is true.

5. **Fix existing**: `test_layered_path_rebuild_blocks_destroyed_bridge_deck`
   ([`core_tests.rs:655`](../../src/sim/pathfinding/core_tests.rs#L655))
   currently sets `has_bridge_deck: true` on cells at indices 1 and 3
   (the "bridgeheads"). Switch them to realistic semantics
   (`has_bridge_deck: false, bw=true, transition=true`); test should
   still pass after the fix.

**Integration tests in `world_tests.rs`:**

6. New: `test_bridgehead_walkability_invariant_across_non_bridge_rebuild_triggers`
   — set up sim with a high bridge; fire `destroyed_structure`,
   `ownership_changed`, `spawned_entities` refresh triggers in sequence;
   assert PathCell.bridge_walkable for bridgeheads is true at every step.

7. New: `test_layered_astar_can_traverse_bridge_after_unrelated_building_death`
   — set up sim with a high bridge and an unrelated building; verify
   A* finds a Ground→Bridge→Ground path; destroy the unrelated building;
   re-verify A* still finds the same path.

### Determinism Considerations

- New `BridgeRuntimeCell` records inserted into `cells: Vec<Option<...>>`
  at deterministic indices (`index_of(width, height, rx, ry)`). No
  iteration-order change.
- `state_hash`: if hashing iterates `cells`, the hash changes. This is
  a one-time hash bump — replays from before this fix won't be valid
  with the post-fix engine. Acceptable; no replay protocol exists yet.
- No new `BTreeMap` insertions, no float math, no `HashMap`/`HashSet`
  introduced.

## Architectural Decisions

**Patterns followed:**

- Pre-existing multi-pass structure in `from_resolved_terrain` (3 passes
  today, adding 4th). Same style (per-cell `for cell in terrain.iter()`
  loop with explicit `index_of` and `cells.get_mut`).
- Role enum already enumerates `Bridgehead`; just attaching it to a real
  cell instead of leaving it as a dead branch.

**Patterns deviated from:**

- Bridgeheads are the first cell type with `bridge_group_id: None` and
  `damage_state == Healthy` permanently. Document this in the pass-4
  comment so future readers don't infer "bridgehead death" semantics.

**Tech debt introduced:**

- None. This removes tech debt: the `static.bridge_walkable` override
  surprise goes away. After this fix, `PathCell.bridge_walkable` is a
  pure projection of `BridgeRuntimeState.is_bridge_walkable()` — no
  special cases.

**Tech debt deferred:**

- `production_placement.rs:391` still reads `ResolvedTerrainCell.
  bridge_walkable` directly. Symptom: extremely rare (placing naval yard
  under destroyed bridge gap). Will be addressed by a follow-up PR after
  RE clarifies the gamemd baseline.
- `movement_path.rs:23-24` (`is_under_bridge_blocked_cell`) still reads
  `ResolvedTerrainCell.is_elevated_bridge_cell()` directly. Symptom:
  `TooBigToFitUnderBridge` naval units (Dreadnought, Boomer, Destroyer)
  cannot path through destroyed high bridge gaps in our port; unknown
  whether gamemd allows this. Will be addressed by a follow-up PR after
  `/re-investigate` on the TooBigToFitUnderBridge rule.

## Alternatives Considered

**Option B — Preserve-static in rebuild.** One-OR patch:
`bridge_walkable = state.is_bridge_walkable() || (static.bridge_walkable
&& !has_bridge_deck)` at
[`core.rs:1006-1008`](../../src/sim/pathfinding/core.rs#L1006).

Rejected. Same observable output as the chosen approach, but leaves
bridge metadata fragmented across two structures (`ResolvedTerrainGrid`
for bridgeheads, `BridgeRuntimeState` for bodies). The fragmentation is
exactly what allowed this bug to slip through originally; preserving
it invites the same class of bug for future bridgehead-related state.
One-line cleanness traded against architectural coherence.

**Option C — Migrate stale-source consumers to PathCell (bundled).**
Migrate `production_placement.rs:391` and `movement_path.rs:23-24` to
read from `PathCell` instead of `ResolvedTerrainCell` in the same PR.

Rejected for this PR (deferred). The `movement_path.rs` case has a
`TODO(RE)` for gamemd's `TooBigToFitUnderBridge` baseline — bundling
would commit us before the RE work is done. Splitting keeps the
bridgehead fix shippable now and gates the consumer migration on
verifiable evidence.

**Option Hybrid — Option A + cross-rebuild invariant test.** Same as
chosen approach but with a stronger regression test asserting bridgehead
walkability across all rebuild triggers.

Folded into the chosen approach via test #6. Not a separate option.
