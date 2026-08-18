# Miner Short-Scan Continuation Before Return — Design

## Goal

When a harvester's current ore cell depletes with partial cargo, scan
within `TiberiumShortScan` radius (default 6 cells) for more ore and
keep harvesting if found — only return to refinery if the short scan
fails. Match gamemd's State 1 cell-depletion behavior.

## Architecture Context

The miner lifecycle is a single-file state machine in
[src/sim/miner/miner_system.rs](../../src/sim/miner/miner_system.rs)
(`tick_miners` → `process_miner` → one `handle_*` per state). The
relevant pieces:

- **`handle_harvest`** (lines 394-441) — the on-cell tick. Extract one
  bale via `extract_bale`. If full → `begin_return`. If cell still has
  ore → continue. If cell depleted with cargo → `begin_return`
  immediately (the bug). If cell depleted without cargo → `SearchOre`.
- **`handle_search_ore`** (lines 216-311) — runs a four-stage cascade
  (short → archive → long → global) for fresh ore search. Used at State
  0 entry. Has the existing zone-grid + locomotor + anchor reachability
  filter setup inline (lines 228-249, ~20 lines).
- **`search_local_ore(nodes, center, radius, filter)`** — the shared
  scan helper. Already used at line 252 for the State 0 short scan.
  Takes an optional reachability predicate.
- **`MinerState::MoveToOre`** — pathfinds toward `target_ore_cell` and
  transitions to `Harvest` on arrival. The continuation case reuses
  this same transition.

`tick_miners` Phase 4/4b gates harvest VFX (voxel anim + oregath SHP)
on `state == Harvest`. Transitioning out to `MoveToOre` correctly
clears those VFX without extra wiring (verified at lines 138-168).

Reachability filter setup is the only piece of duplication this fix
needs to avoid: `handle_search_ore` builds it inline, and the new
short-scan in `handle_harvest` needs the same one.

## Impact Analysis

**Touched files:**
- `src/sim/miner/miner_system.rs` — extract `build_reachable_filter`
  helper, modify `handle_harvest`, refactor `handle_search_ore` to
  call the helper.
- `src/sim/miner/miner_tests.rs` — add tests for the new behavior.

**Blast radius:** small. `handle_harvest` is called only from
`process_miner`. No struct, public API, or tick-ordering change.
Determinism preserved (BTreeMap iteration, deterministic zone grid,
no RNG).

**Risk:**
- The transition `Harvest → MoveToOre` clears `voxel_animation.playing`
  and `harvest_overlay.visible` in Phase 4/4b. Already wired off
  `state == Harvest`, so VFX cadence is correct without extra changes.
- `forced_return` is handled via `MinerState::ForcedReturn`, set
  outside `handle_harvest`. No interaction.
- AI (`src/sim/ai.rs`) doesn't consult miner state for refinery
  tracking. No interaction.

**Determinism:** unchanged. `search_local_ore` is deterministic.
Reachability filter reads `sim.zone_grid` and entity locomotor, both
already part of the deterministic sim state.

## Chosen Approach

Approach B from the brainstorm — extract `build_reachable_filter` as a
private helper, reuse from both `handle_search_ore` and the new
short-scan branch in `handle_harvest`.

Rationale: the same inline 20-line filter setup would otherwise be
duplicated across two sites, in a file already 1017 lines (over the
600-line guideline). Pure deduplication, no behavior change to the
existing State 0 cascade.

## Tiny-Detail Ledger

The implementation must preserve each of these:

- **Scan center** = unit's current cell `(snap.rx, snap.ry)`, NOT
  `last_harvest_cell`. State 1 differs from State 0 here. [GHIDRA
  0x0073E5E0 case 1, after `Harvest_Ore_Tick`]
- **Scan radius** = `TiberiumShortScan` (default 6 cells, stored as
  leptons internally and shifted by 8). Our `config.local_continuation_radius`
  is already wired to this via `from_general_rules`. [GHIDRA + doc
  HARVESTER_MISSION_HARVEST §7]
- **Scan algorithm** = diamond/rhombus spiral, expanding outward, early
  exit at first ring with ore, pick highest-value cell in that ring.
  Our `search_local_ore` is the canonical helper used identically in
  State 0 today. Whether it matches gamemd's exact algorithm is a
  separate concern not introduced or worsened by this fix. [doc §4.1]
- **Cell validity** = in playfield + reachable by unit's locomotor +
  LandType == Tiberium. Reachability filter covers movement-zone gating;
  resource_nodes map gates LandType (only ore/gem cells are present).
  [doc §4.1]
- **Hit behavior** = set `target_ore_cell`, set `state = MoveToOre`.
  Harvest VFX clears via the existing Phase 4 gate. [GHIDRA case 1
  "stay in state 1, IsHarvesting=1" — semantically equivalent: we
  move to the new cell via MoveToOre, then re-enter Harvest on arrival]
- **Miss + cargo > 0** = `begin_return` → `MinerState::ReturnToRefinery`.
  [GHIDRA case 1]
- **Miss + cargo == 0** = fall through to `MinerState::SearchOre` (user
  choice: keep current 4-stage cascade rather than strict gamemd
  return-then-rescan-from-refinery). Observably equivalent in normal
  play. [user-accepted minor drift]
- **`is_full` gate before short scan** = full-cargo miners must NOT
  enter the short-scan path. The existing `if snap.miner.is_full() {
  begin_return; return; }` at lines 415-419 already enforces this.
  [GHIDRA case 1 harvester+full branch]
- **`IsHarvesting` visual flag (`+0x6D2` in gamemd)** = cleared during
  the short-scan window, set again after a successful scan hit. Our
  equivalent: `state == Harvest` (which is currently true during the
  scan, then transitions to MoveToOre). The visual cadence may differ
  by one tick (gamemd clears the flag during scan; we don't until the
  state transitions). Acceptable: one-tick VFX difference is
  imperceptible. [GHIDRA + doc §10]
- **Existing-destination guard (`param_1[0x169]`)** = gamemd stays in
  state 1 if scan misses but a movement_target is already set. Our
  Harvest state implies the miner has arrived, so `movement_target`
  is `None` in this branch. Not introduced; flag as a known minor
  edge case if a player queues a Move during harvest. [GHIDRA case 1]
- **Determinism** = `search_local_ore` iterates BTreeMap; reachability
  filter is a pure function of `sim.zone_grid` + entity locomotor.
  No new RNG, no new wall-clock reads. [implementation invariant]

## Design

### Components

1. **`build_reachable_filter` (new private fn in miner_system.rs)**
   - Returns `Option<Box<dyn Fn((u16, u16)) -> bool + 'a>>` (same
     lifetime/box pattern the existing inline code uses).
   - Reads: `sim.zone_grid`, entity's `locomotor.movement_zone`,
     entity's `movement_layer_or_ground()`, anchor cell via
     `effective_zone_cell`.
   - If any input is missing, returns `None` (unfiltered scan — same
     fallback as today).
   - No state mutation. Pure read.

2. **`handle_harvest` — modified cell-depletion branch**
   - Existing lines 433-441 replaced. New flow:
     1. Build reachable filter via the helper.
     2. Call `search_local_ore((snap.rx, snap.ry),
        config.local_continuation_radius, filter)`.
     3. If `Some(cell)`: `target_ore_cell = Some(cell);
        state = MoveToOre;` return.
     4. Else if `!cargo.is_empty()`: `begin_return(...)`.
     5. Else: `state = SearchOre`.

3. **`handle_search_ore` — refactored**
   - Inline reachability filter setup (lines 228-249) replaced with a
     single `build_reachable_filter(sim, snap)` call.
   - All four scan stages (short / archive / long / global) keep the
     same behavior; only the filter construction site changes.

### Interfaces / Contracts

```rust
fn build_reachable_filter<'a>(
    sim: &'a Simulation,
    snap: &MinerSnapshot,
) -> Option<Box<dyn Fn((u16, u16)) -> bool + 'a>>;
```

Returns `None` when zone-grid or harvester-anchor is unavailable, in
which case callers proceed with an unfiltered scan (matches today's
fallback at line 248-249).

### Data Flow

```
process_miner(MinerState::Harvest)
  → handle_harvest
    → timer countdown
    → extract_bale
       ├ Some(bale) + is_full   → begin_return
       ├ Some(bale) + cell-has  → reset harvest_timer, stay in Harvest
       └ None OR cell-depleted  → [NEW] short-scan path:
           build_reachable_filter
           search_local_ore(current_pos, local_continuation_radius)
             ├ Some(cell)          → target_ore_cell, state = MoveToOre
             ├ None & cargo > 0    → begin_return
             └ None & cargo == 0   → state = SearchOre
```

Subsequent ticks:

- `MoveToOre` pathfinds to the new cell, transitions to `Harvest` on
  arrival, and the cycle continues — exactly matching gamemd's "stay
  in state 1, IsHarvesting=1" semantics on the output side.
- `ReturnToRefinery` runs the existing return logic (chrono teleport
  vs drive).

### Error Handling

No new error paths. Same fallback as `handle_search_ore` if
zone-grid/locomotor data is missing: unfiltered scan. `search_local_ore`
returns `None` for empty resource_nodes or no-match; both already handled.

### Testing Strategy

Three tests in `miner_tests.rs`:

1. **`harvest_continues_to_nearby_ore_when_cell_depletes_partial_cargo`**
   - Spawn a chrono miner with `cargo = [1 bale]`, position on a
     depletable ore cell, place a fresh ore cell 3 cells away.
   - Tick until the current cell depletes (drain via `extract_bale` or
     pre-seed `remaining = 0`).
   - Assert: `state == MoveToOre` AND `target_ore_cell == Some(nearby)`.
   - Assert NOT: `state == ReturnToRefinery`.

2. **`harvest_returns_when_no_ore_within_short_scan`**
   - Spawn a miner with partial cargo on a depletable cell, with the
     nearest other ore beyond `local_continuation_radius` cells.
   - Drain cell, tick.
   - Assert: `state == ReturnToRefinery`.

3. **`empty_cargo_falls_back_to_full_search`** (existing-behavior
   regression guard)
   - Spawn a miner with empty cargo on a cell that's already empty
     when Harvest state is entered. Nearest ore is OUTSIDE
     `local_continuation_radius` but INSIDE `long_scan_radius`.
   - Tick.
   - Assert: miner finds the further ore via `SearchOre` (state ends
     at `MoveToOre` with target == the far cell).

Tests use the existing `Simulation::new` + manual entity insertion
pattern already used by miner_tests.rs.

### Determinism Considerations

- `search_local_ore` iterates `BTreeMap<(u16,u16), ResourceNode>` —
  deterministic ordering.
- `build_reachable_filter` is a closure over `sim.zone_grid` and a
  copy of `MovementZone` + `MovementLayer` — no mutable shared state.
- No RNG, no Instant::now, no IO. State hash unaffected.

## Architectural Decisions

- **Pattern followed:** existing handler style (one `handle_<state>`
  per state, all state transitions explicit, all returns from match
  arms). The fix lives entirely inside `handle_harvest`.
- **Pattern followed:** `search_local_ore` + filter pattern from
  `handle_search_ore`. Identical call shape.
- **Pattern followed:** `MoveToOre` as the transition target for "got
  a new ore cell to head to" — same as State 0's cascade.
- **Deviation:** extracted `build_reachable_filter` rather than
  duplicating the 20-line setup. Justification: the file is at 1017
  lines (over the 600-line guideline); duplication would push it
  further. Pure cleanup, behavior identical.
- **Tech debt introduced:** none.

## Alternatives Considered

- **Approach A — inline duplicate filter setup.** Same behavior as B,
  but with the reachability filter inlined twice. Rejected: pushes a
  large file further over the size guideline and creates a drift
  hazard if one site changes and the other doesn't.

- **Approach C — new `MinerState::HarvestContinuation` transient
  state.** Add an explicit FSM state for "looking for nearby ore
  mid-cycle." Rejected: our states correspond to multi-tick activities
  (driving, harvesting, unloading), not single-tick decisions. Adds a
  state arm, an extra tick, and a debug event with no observable
  benefit — an anti-pattern from the brainstorm checklist
  ("new pattern for no reason").

## Open Follow-ups (out of scope)

- **Doc correction**: `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md §2
  State 0` says chrono harvesters use `TiberiumShortScan`. Binary
  verification (0x0073E5E0 case 0) shows both normal and chrono
  harvesters use `TiberiumLongScan` in State 0 — only weeders use the
  short variant. Same doc §2 State 1 says "If chrono harvester and
  full"; binary shows "If harvester (any) and full". Fix the doc, no
  code change.
- **`search_local_ore` algorithm verification**: not introduced by this
  fix, but worth comparing against gamemd's `FootClass::Scan_For_Tiberium`
  diamond-spiral + best-value-in-ring algorithm. Out of scope here.
- **Existing-destination guard**: gamemd's `param_1[0x169] == 0`
  check is not mirrored. Only fires when a player queues a Move
  during harvest, which today already routes via the command pipeline.
  Track if it surfaces as a visible bug.
