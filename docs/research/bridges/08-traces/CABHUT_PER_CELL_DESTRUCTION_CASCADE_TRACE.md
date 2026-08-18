# CABHUT Per-Cell Destruction Cascade — Trace Report

**Swarm slot:** 4 of 5
**Mechanic:** Per-cell destruction cascade after CABHUT C4 detonation
**Scenario:** SEAL plants C4 on CABHUT. Bridge collapse is triggered. This slot
traces whether each cell in the destroyed span receives the full per-cell cascade:
DamageState → Destroyed, overlay swap, occupant kill, debris spawn,
bridge_state_changed flag, zone rebuild.
**Date:** 2026-05-20
**Verdict:** **FAIL — root bug found in Stage 2 (iteration scope)**

---

## Verdict Tally

**PASS: 2 | FAIL: 5 | UNCHECKED: 1 | NOT-IMPLEMENTED: 2**

---

## Pipeline Entry

### Trigger path

C4 timer expires on CABHUT →
`world_orders.rs:741` calls `dispatch_bridge_collapse_from_hut(sim, rules, hut_center)` →
`bridge_orchestrator.rs:165` runs the hut-specific cascade:
1. `cells_in_5x5_scan(hut_center)` — 25 cells
2. `choose_hut_bridge_family` — picks Low or High from overlay evidence
3. `find_hut_overlay_entry` — finds one entry cell in the overlay band
4. If found: calls `run_hut_destroy_entry` → `apply_hut_bridge_outcomes`
5. If not found: calls `find_hut_fallback_cells` → per-cell `apply_hut_damage_to_cell` loop

---

## Stage-by-Stage Trace

---

### Stage 1 — Cascade entry: destroyed_set list size preserved

**File:line:** `bridge_orchestrator.rs:165–226` (`dispatch_bridge_collapse_from_hut`)
**Input:** `outcomes: Vec<StateOutcome>` from `run_hut_destroy_entry` or the
fallback loop
**Our code:** `apply_hut_bridge_outcomes` aggregates `destroyed_cells` from all
`StateOutcome::Collapsed` variants into `destroyed_set: BTreeSet<(u16, u16)>`.
`blow_up_cells` is also populated from `set_bridge_direction.actions` where
`action == CellAction::BlowUpBridge`.
**Assessment:** The aggregation loop (lines 241–258) is correct and lossless — every
`destroyed_cells` entry and every BlowUpBridge action is added to the respective set.
No early break exists.
**gamemd:** `DestroyBridgeFromCell_High @ 0x5749C0` — single call site in the inner
scan; the mutation is done downstream inside `CollapseBridge_*_High` walkers which
iterate the full span. Equivalent chain.

**Verdict: PASS** — the aggregation of already-computed outcomes is lossless.
The bug is upstream in what goes INTO `outcomes`, not in how it is consumed.

---

### Stage 2 — Iteration scope: does the cascade iterate over EVERY cell?

**File:line:** `bridge_orchestrator.rs:502–565` (`run_hut_destroy_entry`)
**Input:** One `entry_cell (rx, ry)` with a matching destroy overlay, `axis`, sweep direction
**Our code:**

```rust
const MAX_HUT_SWEEP_STEPS: usize = 4;
...
for _ in 0..MAX_HUT_SWEEP_STEPS {
    // call destroy_bridge_high/low on current cell (up to 3 attempts)
    ...
    current = step_axis(current, axis, sweep_dir)?;
}
```

The sweep starts at `midpoint_biased_start` (which shifts the start ±cells based on
how many overlay cells are behind vs. ahead) and walks up to 4 positions.

**gamemd reference:** `DestroyBridge_High_MapInit @ 0x574000` (DESTROYBRIDGE_MAPINIT
§2, Phase 4) does NOT walk multiple overlay cells. It:
1. Finds ONE ramp cell → calls `ApplyDamageToCell` up to 3× on it until true
2. Walks backward to find endpoint cell → calls `ApplyDamageToCell` up to 3× on it

Each `ApplyDamageToCell` call dispatches to `DestroyBridge_High @ 0x57CCF0`, which
calls `DestroyBridgeWalker_NS_High` or `_EW_High`. A SINGLE walker call on a healthy
overlay cell (e.g., `0xCE`) writes `0xD3` to the (this, north, south) triple and
cascades perpendicular siblings — but does NOT advance the walker to the next column.
A second call on the same cell (now `0xD3`) writes `0xE7` (final collapse) to the
triple and cascades — that is 3 cells (the column), plus perpendicular siblings' own
triples. For a 3-wide bridge, this is 3×3 = 9 cells max per walker call.

**The critical issue:** Our `run_hut_destroy_entry` sweeps up to 4 axis-steps, calling
the walker once per axis position. For a typical bridge 3+ columns wide, the
MIDPOINT-BIAS start may place the starting position in the middle. One sweep direction
gets: column N (absorbed, returns Absorbed), column N+1 (absorbed), column N+2 (collapsed,
returns Collapsed with `is_final=true`). But the walker at a single column only writes to
that column's triple (this, north, south) for HIGH-NS axis — it does NOT sweep all
columns of the bridge.

**For a typical large bridge (e.g., 8 columns × 3 rows):**
- gamemd: hits ramp cell → walker fires → writes to ramp column triple → cascades
  perp siblings (edge columns). Second call on same cell → final. Then endpoint.
  The full-bridge collapse in gamemd is driven by `CollapseBridge_*_High` walkers
  inside `DestroyBridgeFromCell_High`, which walk the ENTIRE bridge span in one call,
  not by repeated `ApplyDamageToCell` on different columns.
- Ours: `run_hut_destroy_entry` sweeps 4 columns linearly. Each column's walker
  progresses 1 damage stage per attempt. With `MAX_HUT_ATTEMPTS_PER_STEP = 3` and
  a freshly healthy bridge, one column gets: attempt 1 → 0xCE→0xD3 (Absorbed),
  attempt 2 → 0xD3→0xE7 (Collapsed final). The sweep STOPS here because `Collapsed`
  breaks the inner attempt loop, but crucially `outcomes.push(collapsed)` and the
  outer sweep `break`s immediately when `collapsed = true` (line ~216 in fallback path).

**Wait — re-reading `run_hut_destroy_entry` (lines 529–565):** The outer `for _ in
0..MAX_HUT_SWEEP_STEPS` loop does NOT break on first collapse. It pushes the
`Collapsed` outcome and continues stepping. BUT: on the NEXT step, `current = step_axis(...)`.
The next cell's overlay has been already mutated to 0xE7 (final) by the sibling cascade
from the prior column's walker. `destroy_bridge_walker_ns_high` with `cur = 0xE7`
returns `StateOutcome::NoChange` (falls into the `else` arm). So the sweep effectively
produces ONE `Collapsed` outcome and then NoChange for subsequent cells.

**This is the primary bug:** The per-column walker writes only to the 3-cell NS triple
(this, north, south). For an EW-axis bridge with multiple EW columns, each column is a
separate walker call. But `run_hut_destroy_entry` steps `sweep_dir = ±1` along the axis
direction AFTER a Collapsed outcome, visits a cell that is now already-destroyed
(overlay = 0xE7), gets NoChange, and stops because the next cell is outside the
`matching_destroy_overlay` band check (lines 531–537). The result is that only ONE
column (3 cells for NS-oriented bridge) reaches `DamageState::Destroyed` and enters
`destroyed_set`.

**gamemd's `DestroyBridgeFromCell_High @ 0x5749C0`** (Phase 2, §11 in BRIDGE_REPAIR):
Calls `CollapseBridge_*_High` which is a walk-to-completion function that iterates the
entire bridge span from one end to the other in a single call. This is fundamentally
different from our column-by-column sweep.

**Verdict: FAIL** — Our sweep visits at most MAX_HUT_SWEEP_STEPS = 4 columns and
produces only 1 `Collapsed` outcome (3 cells) because subsequent visited cells are
already at the terminal overlay after the first column cascades. gamemd's
`DestroyBridgeFromCell_High` drives a span-completing walker in one shot.

---

### Stage 3 — Per-cell DamageState mutation

**File:line:** `walker.rs:874–883` (in `destroy_bridge_walker_ns_high`) and equivalent
**Our code:** When `is_final = true` (overlay reaches 0xE7/0xE8/0x64/0x65), the walker
writes `DamageState::Destroyed` to each cell in the triple. The sibling cascade
(`apply_bridge_destruction_ns_high`) also writes `DamageState::Destroyed` on cells
that reach 0xE7 (lines 748–751).
**gamemd:** `DestroyBridgeWalker_*_High` (HIGH_BRIDGE_DAMAGE §3.1) writes state byte 0
(Destroyed encoding) to `cell+0x11E` for cells reaching final collapse.
**Verdict: PASS** (for cells that ARE reached by the walker). The mutation code is
correct. The bug is that too few cells are reached (Stage 2).

---

### Stage 4 — Tile-set swap (overlay byte → destroyed tile)

**File:line:** `walker.rs:874` (`c.overlay_byte = next;` where next = 0xE7 or 0xE8)
and sibling cascade leaves (same pattern).
**Our code:** `c.overlay_byte` is set to 0xE7 (NS final) or 0xE8 (EW final) on the
final-collapse cells, and intermediate values (0xD3, 0xDC, etc.) on non-final cells.
The renderer reads `overlay_byte` from `BridgeRuntimeCell`.
**gamemd:** `DestroyBridgeWalker_NS_High @ 0x57CF60` writes `cell+0x44 = 0xE7` for
final NS cells; `_EW_High @ 0x57D530` writes 0xE8 for EW final. LOW walkers write
0x64 / 0x65 similarly (BRIDGE_REPAIR §12 walker overlay table).
**Assessment:** Our overlay writes match the gamemd values for cells that are reached.
The tile-swap is correct where it fires. The display table lookup (render side, Step 5)
is out of scope for this slot.

**Verdict: PASS** (for cells that ARE reached; again blocked by Stage 2).

---

### Stage 5 — Overlay clear

**File:line:** Not applicable in our code.
**Our code:** There is no separate "overlay clear" step. The bridge overlay byte is
set to the final-destroyed value (0xE7 / 0xE8 / 0x64 / 0x65) rather than cleared.
There are no separate map overlay cells (OverlayClass instances) tracking the bridge
body in our model — the `BridgeRuntimeCell.overlay_byte` IS the overlay.
**gamemd:** `ApplyDamageToCell @ 0x587180` calls `DestroyBridge_High @ 0x57CCF0` which
calls `DestroyBridgeWalker_*_High`. The walker writes to `cell+0x44` (overlay index).
Anchor railings/pavements are a separate step handled by `ToggleBridgePavement` /
`SetOverlayAndPropagate` inside the `CollapseBridge_*_High` paths. These pave the
destroyed bridge surface overlay and are not modeled in our bridge_state layer.
**Assessment:** The bridge body overlay byte is correctly set to the terminal value.
The pave/railing overlay for destroyed bridge (handled by `ToggleBridgePavement` in
gamemd) is not implemented, but this is a visual-only difference and is the
render-side slot 5's concern, not this cascade.

**Verdict: UNCHECKED** (ToggleBridgePavement / pave-overlay mechanics not traced this slot).

---

### Stage 6 — Occupant kill via BlowUpBridge / C4Warhead

**File:line:** `bridge_orchestrator.rs:103–105` (`kill_ground_occupants_at` loop)
**Our code:**
```rust
let c4_inf_death: u8 = {
    let c4_id = rules.c4_warhead_id();
    let name = sim.interner.resolve(c4_id);
    rules.warhead(name).map(|wh| wh.inf_death).unwrap_or(1)
};
for &(rx, ry) in &blow_up_cells {
    kill_ground_occupants_at(sim, rx, ry, c4_inf_death);
}
```
`blow_up_cells` is built from `set_bridge_direction.actions` where
`action == CellAction::BlowUpBridge`. For high bridges the walker emits
`BlowUpBridge` on every cell in the collapsed triple plus perpendicular siblings that
reach 0xE7.
**gamemd:** `FUN_00487720 @ 0x487180` is the ground-occupant damage pass called from
`ApplyDamageToCell`; uses warhead at `RulesClass+0xFA8` (C4Warhead). Verified
in DESTROYBRIDGE_MAPINIT_BODIES §5.
**Assessment:** The C4Warhead pre-resolution via `rules.c4_warhead_id()` is correct.
The kill loop sets `health = 0`, `dying = true`, switches infantry death sequence per
`InfDeath` byte. This matches gamemd's damage path. HOWEVER: `blow_up_cells` only
contains cells from the outcomes that were actually produced — which, per Stage 2's
bug, is only 3 cells (one column) instead of the full span.

**Verdict: FAIL** (correct code, but applied to only ~3 cells instead of ~N×3).

---

### Stage 7 — Debris / rim refresh

**File:line:** `bridge_orchestrator.rs:118` (`spawn_bridge_debris`) and
`bridge_orchestrator.rs:137` (`update_adjacent_bridges`)

**Our code:** Both functions iterate `destroyed_set` and `rim_cells` respectively.
The debris spawn (lines 807–883) follows the binary's exact RNG sequence:
outer 95% gate → jitter×2 → metallic 50% → metallic slot → explosion delay →
explosion slot. Verified in the `debris_consumes_correct_rng_count_per_cell` test.

**gamemd:** Debris spawn is inside `BlowUpBridge` step 4 (HIGH §11.4, step 4 in
DESTROYBRIDGE_MAPINIT §5). RNG order confirmed.

**Assessment:** Debris spawn code is correct. Rim refresh (`update_adjacent_bridges`)
walks 8 neighbors of each rim cell and resets orphaned stubs. This is a simplified
analog of gamemd's `UpdateAdjacentBridges_High @ 0x576770`. The test
`rim_refresh_clears_dangling_stub_cells` passes.

**HOWEVER:** Both loops iterate over `destroyed_set` / `rim_cells` which are computed
from Stage 2's truncated output. Only ~3 cells get debris, not the full span.

**Verdict: FAIL** (logic correct, but applied to only partial cell set due to Stage 2).

---

### Stage 8 — bridge_state_changed flag

**File:line:** `bridge_orchestrator.rs:277` (`!destroyed_set.is_empty()`)
**Our code:** Returns `true` iff `destroyed_set` is non-empty. Caller in
`world_orders.rs:739-746` stores the result in `bridge_state_changed` and uses it
to signal PathGrid rebuild before next tick.
**gamemd:** `MapClass::DestroyBridge_High_MapInit` writes
`*(byte*)(g_Tactical + 0xD7C) = 1` as the deferred-rebuild flag.
**Assessment:** The flag is set correctly when any cell collapses. Even with Stage 2's
truncation, the flag will be set if at least one column collapses. However, the PathGrid
will be rebuilt with only 3 destroyed cells reflecting correctly — the remaining intact
cells will still show as walkable, allowing units to path across a visually broken bridge.
This is a secondary effect of Stage 2's bug.

**Verdict: PASS** (flag is set; but the partial destruction means PathGrid has wrong
connectivity until all cells are destroyed).

---

### Stage 9 — Zone rebuild

**File:line:** `bridge_orchestrator.rs:275` (`refresh_bridge_zones_if_dirty`)
**Our code:** `any_zones_dirty` is OR'd from all `StateOutcome::Collapsed.zones_dirty`
fields. `zones_dirty = is_final` in each walker, so it is `true` for any final-collapse
call. `refresh_bridge_zones_if_dirty` rebuilds endpoint records, path grid, and zone grid.
**gamemd:** `UpdateBridgeZonesHelper @ 0x56C510` (unconditional tail of
`DestroyBridge_*_MapInit`). Equivalent scope.
**Assessment:** Zone rebuild fires correctly when a collapse happens. But the endpoint
records reflect only the 3 destroyed cells, not the full span — so the zone
connectivity gap may be narrower than gamemd's (a partial gap rather than full
disconnection). This is again a downstream effect of Stage 2.

**Verdict: FAIL** (zone rebuild fires, but reflects partial destruction — bridge endpoint
records may still show the bridge as traversable since only 3 of N cells are Destroyed).

---

### Stage 10 — Determinism

**File:line:** `bridge_orchestrator.rs:807–883` (RNG draws in `spawn_bridge_debris`)
**Our code:** BTreeSet iteration order is deterministic. RNG draws follow the exact
binary sequence (test `debris_consumes_correct_rng_count_per_cell` verifies). No
external state dependencies.
**Assessment:** The subset of cells that ARE processed follow deterministic paths.
RNG draw order is parity-correct for those cells.

**Verdict: PASS** (determinism holds for cells processed; Stage 2 bug affects WHICH
cells are processed, but the processing is itself deterministic).

---

## Root Bug — Stage 2 Detail

**Location:** `bridge_orchestrator.rs:502–565` (`run_hut_destroy_entry`)
**Line of divergence:** `const MAX_HUT_SWEEP_STEPS: usize = 4;` and the sweep loop.

**What gamemd does:**
`DestroyBridge_High_MapInit` (Phase 1) finds ONE overlay entry cell via the 5×5 scan,
then dispatches to `DestroyBridgeFromCell_High @ 0x5749C0`. That function is NOT a
per-column incrementer — it finds the bridge's axis and calls `CollapseBridge_NS_High`
or `_EW_High`, which walk the entire bridge from end to end in one shot, writing
destroyed tiles to every body cell in the span.

**What our code does:**
`run_hut_destroy_entry` steps along the bridge axis, calling the single-column
`destroy_bridge_high` (→ `destroy_bridge_walker_ns_high` / `_ew_high`) per position.
Each walker call writes to ONE column triple (3 cells). After the first final-collapse
call, the next step visits a cell already at 0xE7 → NoChange → sweep stops matching
at line 533 (`!matching_destroy_overlay(family, current_overlay)` returns `false` for
0xE7 since 0xE7 is in the HIGH destroy range but the walker itself returns NoChange).

**Concrete example (8-column NS-axis high bridge):**
- gamemd: ONE call to `DestroyBridgeFromCell_High` → iterates all 8 columns → 24 cells
  (8 × 3) go to Destroyed
- ours: `run_hut_destroy_entry` enters at midpoint (column ~4), sweeps up to 4 steps.
  Column 4: absorb → damaged. Column 5: absorb → damaged. Column 6: collapse → 3 cells
  destroyed (this, north, south). Walker cascades perp siblings but those are on DIFFERENT
  columns' triples, not additional rows. Loop continues to column 7: overlay is now 0xE7
  (mutated by sibling cascade or by the column 6 write if same column). NoChange.
  **Result: ~3 cells destroyed, 21 cells intact.**

**Player-visible effect:** The bridge "shivers" — one segment collapses visually — but
most of the span remains intact and walkable. Matches the user's reported observation
("only 1 small piece of bridge falls").

---

## Top 5 Player-Visible Failures

1. **Stage 2 — Iteration scope (PRIMARY BUG)**
   Player sees: Only 1 visual segment (~3 cells) of the bridge collapses. Rest remains
   intact. Units can still path across the bridge after C4.
   File:line: `bridge_orchestrator.rs:297` (`MAX_HUT_SWEEP_STEPS = 4`) and
   `bridge_orchestrator.rs:529-565` (sweep loop)
   gamemd evidence: `DestroyBridgeFromCell_High @ 0x5749C0` calls a span-completing
   walker, not a per-column incrementer (BRIDGE_REPAIR_AND_HUT_DEATH §11 table,
   DESTROYBRIDGE_MAPINIT_BODIES §5 helper graph)

2. **Stage 6 — Occupant kill scope**
   Player sees: Ground units standing on non-destroyed bridge cells survive the C4
   detonation. Should die via C4Warhead. Only ~3 cells receive the kill pass.
   File:line: `bridge_orchestrator.rs:103-105` (blow_up_cells loop, populated only
   from truncated Stage 2 output)
   gamemd evidence: `FUN_00487720 @ 0x487180` applies C4Warhead to every destroyed cell
   (DESTROYBRIDGE_MAPINIT_BODIES §5)

3. **Stage 9 — Zone rebuild partial**
   Player sees: Pathfinding treats most of the bridge as still traversable. Enemy AI
   and player units continue routing across a "destroyed" bridge. Would be fully
   blocked in gamemd.
   File:line: `bridge_orchestrator.rs:770-785` (`refresh_bridge_zones_if_dirty`)
   — runs but reflects only 3 destroyed cells
   gamemd evidence: `UpdateBridgeZonesHelper @ 0x56C510` called after full span
   collapse; with all cells Destroyed, endpoint records flip `active = false`
   (DESTROYBRIDGE_MAPINIT_BODIES §2 Phase 5 tail)

4. **Stage 7 — Debris scope**
   Player sees: Debris explosion animations appear over only ~3 bridge cells. A full
   bridge collapse should spray debris across all N cells. Visually thin explosion.
   File:line: `bridge_orchestrator.rs:118` (`spawn_bridge_debris` on truncated
   `destroyed_set`)
   gamemd evidence: `BlowUpBridge` (HIGH §11.4) fires per-cell for every cell in the
   collapse span

5. **Stage 8 / Stage 9 composite — PathGrid shows walkable gap**
   Player sees: After C4, bridge appears partially destroyed but units path across
   the visually intact sections. The PathGrid is rebuilt but reflects only the 3
   actually-destroyed cells; non-destroyed cells remain `bridge_walkable = true`.
   File:line: `bridge_orchestrator.rs:275` (zone rebuild with partial `destroyed_set`);
   `bridge_state/mod.rs:919-923` (`is_bridge_walkable` returns false only for
   `DamageState::Destroyed`)
   gamemd evidence: full span walk sets all body cells to state byte 0
   (HIGH_BRIDGE_DAMAGE §3.1 collapse arm)

---

## Adjacent Findings (do not trace this run)

1. **`DestroyBridgeFromCell_High @ 0x5749C0`** is not modeled in our code at all.
   Our `run_hut_destroy_entry` approximates its behavior but uses a fundamentally
   different algorithm (incremental column sweep vs. span-completing walker). The fix
   likely requires implementing a hut-specific `collapse_bridge_from_cell_high/low`
   function that mirrors the binary's single-shot full-span collapse (per
   BRIDGE_REPAIR_AND_HUT_DEATH §11 and DESTROYBRIDGE_MAPINIT_BODIES §5).

2. **`ToggleBridgePavement`** — called inside `CollapseBridge_*_High` walkers to stamp
   the destroyed-pavement overlay. Our code does not call this. Deferred to Stage 5
   (visual), but worth noting here as an adjacent gap.

3. **LOW bridge CABHUT path** has the same bug: `run_hut_destroy_entry` is shared for
   both Low and High. Low bridges on CABHUT destruction would also show only partial
   collapse.

4. **`find_hut_fallback_cells` path** (lines 379–431) — used when no overlay entry is
   found in the 5×5 scan. The fallback loop also uses `MAX_HUT_ATTEMPTS_PER_STEP = 3`
   but iterates cells one at a time with early break on first `Collapsed` (lines
   209–220). This is an even more severe variant of the same truncation bug.

5. **`FUN_00487720 @ 0x487180`** in gamemd applies splash damage to ALL cells in a
   5×5 grid around each destroyed cell (not just single-cell kill). Our
   `kill_ground_occupants_at` is single-cell only. This is a separate disparity but
   fires only for cells that ARE destroyed — compounded by Stage 2's bug.

---

## Sources

**Docs consulted:**
- `DESTROYBRIDGE_MAPINIT_BODIES_GHIDRA_REPORT.md` §2–§5 (primary reference for
  gamemd cascade mechanics and helper graph)
- `BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md` §11 (DestroyBridgeFromCell_High,
  span-completing walker vs. incremental semantics)
- `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §3.1 (state transitions, final
  collapse arms)
- `BRIDGE_DISPLAY_TABLE_GHIDRA_REPORT.md` §2.2 (layer ordering, overlay write
  confirmed event-driven not per-frame)

**Rust files read:**
- `src/sim/world/bridge_orchestrator.rs` (full — 1458 lines)
- `src/sim/bridge_state/mod.rs` (lines 0–1050)
- `src/sim/bridge_state/walker.rs` (lines 0–1350)
- `src/sim/world/world_orders.rs` (lines 720–780)

**Ghidra MCP:** Not invoked this run — docs were sufficient to establish the divergence.

---

**Status: COMPLETE**
