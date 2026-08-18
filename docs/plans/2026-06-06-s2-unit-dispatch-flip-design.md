# Slice S2 — UnitClass dispatch→Process authority flip (scoped) Design

**Status:** DESIGN v2 — all REVISE findings resolved in-doc (2026-06-10; resolutions inline in
the findings section + new §Save/load authority). Awaiting re-review → `/write-plan`. No Rust written.
**Date:** 2026-06-06 (v2 revision 2026-06-10)
**Rule:** Rust-native structure, gamemd-native semantics. First hash-affecting object-AI slice.
**Ladder position:** Slice **S2** in `docs/research/TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md` §9
(3rd of S0–S8). Builds on the landed read-only host (S2a / "S1.5", branch
`unit-mission-dispatch-host`). Successor: S3 (Fire→Facing→…→Spawn).

## Goal

Promote the per-object dispatch→locomotor-`Process` ordering to **authoritative for the
scoped UnitClass path only**: per object, in live order — increment `tick_counter` (`+0xC4`
analogue) → `Mission_Dispatch` (dispatch-time mission becomes the authoritative
`mission.current`) → run the locomotor `Process`. Achieve it by **interleaving the dispatch
decision into the existing Phase-1 mover loop** (which is already the per-object locomotor
pass), not by relocating movement. Hash-affecting → `SNAPSHOT_VERSION` bump + gamemd-cited
golden re-baseline.

## Architecture Context

- `object_ai_stage` runs at top-of-tick (post-command, pre-movement,
  [src/sim/world/mod.rs:1968](../../src/sim/world/mod.rs)); the `Unit` arm is the landed
  read-only dispatch host (S2a) — records host-time routing, mutates nothing.
- Phase-1 `tick_movement_with_grids` ([src/sim/movement/movement_tick.rs:820](../../src/sim/movement/movement_tick.rs))
  moves all units **in live object order with incremental occupancy** (a mover's committed
  cell is visible to later movers — already gamemd-faithful; doc-comment "gamemd processes
  movers in live object order"). It has cross-entity shared state: per-owner block sets
  refreshed on occupancy generation change, scatter dedup, blocker-neighbor counts.
- `mission.current` is hashed (Slice 8) but written at the **tail** by `refresh_mission_shadow`
  ([mod.rs:910/921](../../src/sim/world/mod.rs)) as a projection of `derived_mission()`
  ([game_entity.rs:523](../../src/sim/game_entity.rs)) of the legacy machines. `tick_counter`
  (hashed, [world_hash.rs:55](../../src/sim/world/world_hash.rs)) is also incremented there.
- S1 shadow (`unit_move_dispatch_then_process_shadow_agrees`) already proved dispatch-then-
  Process matches the phase-split movement result for scoped Move Units, every tick.

## Impact Analysis — what S2 actually changes in the hash

Three candidate deltas, analyzed:

1. **Movement timing — NEUTRAL, by read-set argument (P2-corrected; not by S1).** The landed
   S1 shadow proves dispatch-precedes-Process *marker* ordering + scope only
   (`process_drive_locomotion_shell` is a read-only marker, not a position advance), so it
   cannot carry movement-output equality. The argument that does hold: **no behavior system
   reads `mission.current` or `tick_counter`** — the only non-test readers are `world_hash`
   (the hash fold) and the debug S1 shadow; movement is driven by `movement_target`/`locomotor`
   state, which the dispatch step never writes. The mover-loop body and whole-set setup are
   untouched, so every unit's `Process` runs with identical inputs in identical order.
   **No position delta, by construction** — and additionally checked by a direct
   position-sequence test (see Testing Strategy).
2. **`tick_counter` (`+0xC4`) relocation — VALUE-NEUTRAL.** Incremented once per tick either
   way; relocating top↔tail leaves the end-of-tick value unchanged (must remove the tail
   increment for scoped units to avoid double-count). **No hash delta from the counter value.**
3. **`mission.current` authority point — THE REAL DELTA.** Today it is the *tail* projection
   (end-of-tick `derived_mission`); S2 makes it the *host-time* (top-of-tick, post-command)
   value. The difference is exactly the **measured churn** (rare, arrival-driven Move→Sleep;
   dense scenario: 2 of 300 ticks, whole-column-at-once). The golden re-baseline is driven by
   these churn ticks only — small and bursty, not pervasive.

**Faithfulness of the delta (binary-verified, `decompile 0x004D4200`):** on the arrival tick
gamemd's `CurrentMission` stays `Move` (Mission_Move calls OnArrival/→Guard only when the
locomotor is done AND NavCom is already null, which — because `Process` runs after dispatch —
is first true the *next* tick). Host-time = `Move` on the arrival tick → **matches gamemd**;
the current tail = `Sleep` → does not. So S2 is a **fidelity improvement**, and the new golden
is justified by gamemd evidence, not a tolerated drift.

**Blast radius:** with the dispatch-into-the-loop approach, the movement loop and its whole-set
setup are untouched, so **occupancy/scatter interleaving is byte-identical for every unit** —
the cross-interaction concern the ladder flags (a scoped vs. unscoped unit contending for one
cell, doc §9 S2 parity risk) is structurally avoided, not just mitigated. Still test it
explicitly. The only behavioral change is the per-scoped-unit dispatch (`tick_counter` point +
`mission.current` authority), whose hash effect is the measured churn.

**Why not hoist movement up (Option A, rejected):** the per-mover step depends on a whole-set
setup that runs once at Phase-1 time (blocker-neighbor counts, drive re-aims, pending drive
arrivals, per-owner block sets, occupancy generation; [movement_tick.rs:845-976](../../src/sim/movement/movement_tick.rs)).
Calling one unit's `Process` at top-of-tick would run it without that setup → divergence;
reproducing the setup at top-of-tick changes movement for *all* units. The dispatch-into-the-
loop approach sidesteps this entirely.

## Chosen Approach

**Interleave the per-object dispatch INTO the Phase-1 mover loop for scoped Units — push
dispatch DOWN into the loop; do NOT hoist movement up into `object_ai_stage`.** The Phase-1
mover loop ([movement_tick.rs:978](../../src/sim/movement/movement_tick.rs)) already iterates
live object order and runs each unit's locomotor `Process` after a whole-set setup (blocker
counts, drive re-aims, pending arrivals, per-owner block sets + occupancy-generation
tracking). That loop **is** the Rust equivalent of gamemd's per-object locomotor pass. So the
faithful, low-blast-radius S2 is: inside that loop, immediately **before** a scoped unit's
`Process` step:

1. `tick_counter += 1` — the `+0xC4`-before-dispatch point, per object.
2. Commit `mission.current`/`substate` = the unit's `derived_mission()` read right here
   (post-command, pre-`Process`) — authoritative now, not re-projected at the tail for these
   units. This is the gamemd dispatch-time value (movement_target still set on the arrival
   tick → `Move`, matching gamemd).
3. The existing per-mover `Process` step runs next, **unchanged**.
4. Mark the unit so the tail `refresh_mission_shadow` skips its `tick_counter`/`current`
   rewrite (no double-count, no authority clobber). Movement is **not** skipped or relocated —
   it stays exactly where it is in the loop; only the dispatch *decision* is interleaved.

**Coverage (P3):** the in-loop dispatch step fires only for scoped units that are *collected as
active movers* that tick. The mover-collection guards
([movement_tick.rs:909-914](../../src/sim/movement/movement_tick.rs)) skip
`forced_drive_processed` / target-less / `low_bridge_tube_state` units. Whether a forced-drive
or low-bridge unit can simultaneously satisfy the scope predicate is **UNVERIFIED** — the
predicate ([techno_ai.rs:285-293](../../src/sim/world/techno_ai.rs)) does not test those two
fields — but the mechanism is safe either way: a scoped unit that is not collected on a given tick never enters the
`dispatched` set and falls to the tail `refresh_mission_shadow`, which performs its single
`tick_counter` increment and `current`/`substate` write exactly as today — tail authority for
that unit-tick, exactly one increment (the `dispatched` set is the single guard; absence from
it means the tail path runs, presence means it doesn't — no path does both or neither). Tested
explicitly (`guard_skipped_scoped_unit_single_count`).

`object_ai_stage` stays the read-only host (its top-of-tick shadow record is unchanged and
still proves agreement; nothing mutates a scoped unit's machines between the host record and
this loop, so the values match). The whole-set movement setup is untouched, so occupancy /
scatter / block-set interleaving stays **bit-identical to today for every unit** — the
property Option A could only buy by hoisting the entire setup. The remaining work is small
and local: a dispatch step added at the top of the scoped branch of the existing mover loop,
plus the tail-skip bookkeeping.

## Tiny-Detail Ledger

- **Per-object order:** `tick_counter++` → `Mission_Dispatch` → locomotor `Process`, same
  pass. `[doc line 21 "VERIFIED to the byte": +0xC4 store 0x006fa64f → Mission_Dispatch
  0x006FA655 → Process vtable+0x40 0x004DA877 AFTER]`
- **Arrival tick keeps `Move`; →`Guard` next tick.** OnArrival fires only when loco done AND
  NavCom null; `Process` runs after dispatch, so arrival is seen one tick later.
  `[decompile 0x004D4200 this session; FOOTCLASS_MISSION_MOVE_GHIDRA_REPORT.md §4/§9;
  NAVCOM_LIFECYCLE_GHIDRA_REPORT.md §5]`
- **Host-time mission is the authoritative selector** (post-command, pre-Process), NOT the
  tail re-derivation. `[decompile 0x006F9E50: dispatch precedes Process/acquire]`
- **`tick_counter` incremented exactly once/tick** — at host time for scoped units, at tail
  for the rest; never both. Hashed. `[world_hash.rs:55; mod.rs:921]`
- **Movement is unchanged** — the mover loop and its whole-set setup stay byte-identical; the
  dispatch step is interleaved before each scoped unit's existing `Process`, so occupancy /
  block-set / scatter interleaving is preserved by construction. `[movement_tick.rs:945/998]`
- **Scope = `is_s1_scoped_move_unit`** (moving drive Unit, pure Move/Guard, no combat/dock/
  miner/aircraft). `[techno_ai.rs:285-293]`
- **Out-of-scope idle→`Guard` (not `Sleep`) and passive acquire are S3**, not S2 — do not
  "fix" `derived_mission` idle mapping here. `[S2_MISSION_DISPATCH_VS_PASSIVE_ACQUIRE_ORDERING.md]`
- **Frame-anchored dispatch gate** (IsActive→timer→Health) still NOT enforced this slice
  (S4 owns the gate). `[doc §2.2]`
- **Load trusts serialized `MissionCom`** — the post-load `current`/`substate` re-derive in
  `rebuild_logic_membership` is deleted; the `presence` reconcile stays (`#[serde(skip)]`).
  `[mod.rs:1394-1402; snapshot.rs:469/521; §Save/load authority]`

## Design

### Components
- `movement_tick.rs` mover loop: in the **scoped branch only**, immediately before the
  existing `Process` step, run the dispatch step (`tick_counter++`; commit `mission.current`/
  `substate` from the `derived_mission()` read at that point) and record the id into a
  per-tick `dispatched` set. The per-mover body is otherwise **unchanged** — no extraction.
- `techno_ai.rs`: reuse `is_s1_scoped_move_unit` (already exists) as the scope predicate;
  `object_ai_stage`'s Unit arm stays the read-only host (no authority moves there).
- `mod.rs`: thread the `dispatched` set from the movement call to the tail;
  `refresh_mission_shadow` skips `current`/`tick_counter` for ids in it; `SNAPSHOT_VERSION`
  bump; golden re-baseline.
- `mod.rs` `rebuild_logic_membership`: delete the post-load `mission.current`/`substate`
  re-derive — load trusts serialized `MissionCom` (see §Save/load authority; P1 fix).

### Interfaces / Contracts
- The dispatch step is added **inline** at the top of the scoped branch of the existing mover
  loop — no new movement entry point, no extraction of the per-mover body (Option A's risky
  refactor is avoided).
- A per-tick `dispatched` id set is threaded from the movement call to the tail
  `refresh_mission_shadow`; it is the single guard against double-count / authority clobber.

### Data Flow
```
object_ai_stage (top):  read-only dispatch host record (unchanged)
Phase-1 mover loop (whole-set setup unchanged): for id in live_order:
  if is_s1_scoped_move_unit(id):
     tick_counter++ ; mission.current/substate = derived_mission(id)   // pre-Process = dispatch-time value
     dispatched.insert(id)
  <existing per-mover Process step for id — unchanged>
tail refresh_mission_shadow: for id: if !dispatched(id) { current/substate=… ; tick_counter++ }
state_hash
```

### Save/load authority (P1 resolution)
Confirmed on current code (2026-06-10): the re-derive wins on load today. Both production load
paths run the rebuild *after* serde restore — the `GameSnapshot` restore path
(`snapshot.rs:259`) and the app-layer load (`app_input.rs:755`), both via
`rebuild_caches_after_load` ([mod.rs:1370](../../src/sim/world/mod.rs)) — and
`rebuild_logic_membership` ([mod.rs:1394-1402](../../src/sim/world/mod.rs)) unconditionally
overwrites the serialized `mission.current`/`substate` with `derived_mission()`. (The two
test call sites, `snapshot.rs:469/521`, reproduce the same post-deserialize sequence.)

**Fix (in S2): delete the mission re-derive from `rebuild_logic_membership`; load trusts the
serialized `MissionCom` for all units.** Why this is safe and right:

- `MissionCom` fully round-trips via serde — no skipped fields
  ([mission/mod.rs:186-202](../../src/sim/mission/mod.rs)) — and is canonical hashed lockstep
  state since Slice 8 ([world_hash.rs:28-55](../../src/sim/world/world_hash.rs)). `queued`/
  `suspended`/`timer`/`tick_counter` are *already* trusted from serde today; only
  `current`/`substate` get the load-time overwrite.
- **Non-S2 units:** the `current == derived_mission()` invariant holds at every save point
  (the tail projection wrote it), so serialized == re-derived — removing the overwrite is
  value-identical for them.
- **S2 units:** the serialized value is the only correct source — the arrival-tick `Move`
  cannot be re-derived after the after-loop `movement_target` clear.
- The `presence` reconcile in the same function **stays** (`presence` is `#[serde(skip)]` and
  genuinely needs re-derivation).
- A conditional variant (re-derive only non-S2 units) was rejected: value-identical to full
  deletion, but preserves a load-time writer that contradicts MissionCom's Slice-8 authority.

Guard test: a save/load round-trip **on an arrival tick** must restore an identical
`state_hash` (golden replay cannot catch this; only the round-trip test can).

Implementation-time cleanup alongside: the `MissionCom` doc comment
([mission/mod.rs:181-185](../../src/sim/mission/mod.rs)) still says "NOT yet folded into
`world_hash`" — stale since Slice 8; and the rebuild's own comment ("re-derived ... so a
save/load round-trip restores identical derived state") describes the pre-S2 invariant.

### Error Handling
Absent/dying unit skipped (inherits S0 guards). The per-mover step is the existing fallible-
free body; no new failure modes.

### Testing Strategy
- `unit_move_start_slip_matches_dispatch_then_process` — freshly-ordered Move advances on the
  dispatch-then-Process tick (unchanged from phase-split, per S1).
- `unit_c4_counter_increments_before_dispatch` — scoped unit's `tick_counter` increments at
  host time, exactly once/tick (not double via the tail).
- `arrival_tick_mission_is_move_not_sleep` — on the arrival tick the scoped unit's hashed
  `mission.current` is `Move` (gamemd-faithful), transitioning away only next tick.
- `scoped_and_unscoped_unit_same_cell_contention` — a scoped and a phase-split unit racing for
  one cell resolve identically to all-phase-split (occupancy interleaving preserved).
- `save_load_round_trip_on_arrival_tick` — save on the arrival tick (where the hashed
  `mission.current = Move` diverges from `derived_mission() = None`), reload, and require an
  identical `state_hash` (P1 guard; golden replay cannot catch this).
- `guard_skipped_scoped_unit_single_count` — a scoped unit NOT collected as a mover that tick
  gets tail authority with exactly one `tick_counter` increment (no zero/double count; P3 guard).
- `position_sequence_unchanged_direct` — per-tick position sequences for a mixed
  scoped/unscoped scenario are identical to pre-S2 (movement neutrality measured on outputs,
  not only argued; P2 guard).
- Golden: re-baseline `global_skirmish_replay_is_deterministic_and_baseline_stable` with the
  documented churn-driven reason; determinism (replay==record) must still hold bit-exact.

## Architectural Decisions
- **No movement-body extraction and no second movement entry point** — the dispatch step is
  added inline in the existing mover loop, so movement keeps a single owner. Lower-risk than
  Option A's refactor.
- `match`/scoped predicate; no `dyn`. Out-of-scope units unchanged → minimal blast.
- The dispatch *decision* now lives in the mover loop for scoped Units while `object_ai_stage`
  remains the read-only host — a deliberate transitional split. Later slices (S3+) fold combat/
  facing/etc. into the same per-object pass; S5 unifies authority across all categories.
  Acceptable, explicitly transitional (doc §9).

## Alternatives Considered
- **Option A — hoist scoped movement up into `object_ai_stage`** (the original draft). Rejected
  after a feasibility spike: the per-mover step depends on a whole-set setup that runs at
  Phase-1 time (blocker counts, drive re-aims, pending arrivals, block sets, occupancy
  generation); calling one unit's `Process` at top-of-tick runs it *without* that setup
  (divergence), and reproducing the setup at top-of-tick changes movement for all units.
  Dispatch-into-the-loop achieves the same per-object dispatch→Process ordering with no setup
  disturbance and no body extraction.
- **Absorb ALL Unit movement at once.** Rejected: forces reworking occupancy/scatter/block-set
  timing for the whole set in one hash-affecting slice — exactly the blast radius the ladder
  splits across S2→S5.
- **Keep tail authority, only relocate `+0xC4`.** Rejected: value-neutral, so it neither
  improves fidelity (arrival tick stays wrongly `Sleep`) nor advances the ladder.
- **Set `mission.current` from the post-handler (tail) value at host time.** Rejected: that is
  the current behavior; the verified gamemd arrival-tick value is the *dispatch-time* `Move`.

## Open / Pre-implementation gates
- **Borrow compatibility:** the dispatch step needs a `&mut` entity (to bump `tick_counter` and
  write `mission.current`) plus a `derived_mission()` read; the mover loop already does a careful
  scoped `get_mut`/release dance ([movement_tick.rs:1034](../../src/sim/movement/movement_tick.rs)).
  Confirm the dispatch step slots cleanly before/into that scoped block without fighting the
  later immutable crush/bump lookups. This is the one real implementation risk.
- **No double-count:** confirm `tick_counter` is incremented exactly once (loop dispatch for
  scoped ids; tail for the rest) and `mission.current` authority isn't clobbered by the tail.
- The post-arrival idle=`Guard`-vs-`Sleep` + passive-acquire gap is **S3**, recorded here so it
  is not silently absorbed into S2.
- **Dock↔move boundary:** `mission/retask.rs:97` writes `mission.current` (DockTeardown). Scope
  excludes dock, but confirm at plan time that a unit crossing the scope boundary mid-tick
  (e.g., retasked into dock the same tick it was dispatched as a scoped mover) cannot get two
  `mission.current` writers in one tick.

## Design-Review Findings (2026-06-06) — verdict REVISE — ALL RESOLVED in v2 (2026-06-10)

Verified against the actual code (not just cited docs). Approach is sound and the parity
reasoning is binary-confirmed; resolve these before `/write-plan`:

**v2 resolution summary** (details inline below and in the body sections they amended):
- **[P1] RESOLVED** — confirmed the rebuild re-derive wins on load (runs after serde in both
  production load paths: `snapshot.rs:259` and `app_input.rs:755`, via
  `rebuild_caches_after_load` → `mod.rs:1394-1402`). Fix
  chosen: delete the mission re-derive; load trusts serialized `MissionCom` (value-identical
  for non-S2 units by the save-point invariant, the only correct source for S2 units). New
  §Save/load authority section; `save_load_round_trip_on_arrival_tick` test added.
- **[P2] RESOLVED** — Impact Analysis item 1 rewritten on the read-set basis (no behavior
  system reads `mission.current`/`tick_counter`; S1 demoted to ordering/scope evidence only);
  direct `position_sequence_unchanged_direct` test added.
- **[P3] RESOLVED** — explicit Coverage paragraph added to Chosen Approach (guard-skipped
  scoped unit → tail authority, exactly one increment via the `dispatched`-set guard);
  `guard_skipped_scoped_unit_single_count` test added.
- **Test additions** — all three review-requested tests added to Testing Strategy.
- **Non-interaction (`mission/retask.rs:97` DockTeardown)** — NOT yet verified; carried as an
  explicit pre-implementation gate (scope excludes dock; confirm no dock↔move boundary
  interaction at plan time).

- **[P1] BLOCKER — save/load divergence on arrival ticks.** S2 makes `mission.current` diverge
  from `derived_mission()` on the arrival tick (`Move` committed pre-clear; `derived_mission()`
  becomes `None` after the after-loop `movement_target` clear). But `rebuild_logic_membership`
  ([mod.rs:1402-1404](../../src/sim/world/mod.rs)) — the post-deserialize reconciliation —
  **unconditionally re-derives** `mission.current` from the machines. A save taken on an arrival
  tick reloads with `mission.current = None` → different `state_hash` → **save/load desync** (a
  hard lockstep requirement). Today this is invisible because `mission.current == derived_mission()`
  always holds; S2 breaks that invariant, making `mission.current` *stateful authority that can't
  be re-derived on load*. **Fix:** make load trust the serialized `mission.current` for
  S2-authoritative units (don't re-derive), or otherwise reconcile; add a **save/load round-trip
  test on an arrival tick** (golden replay won't catch it). Confirm whether serde or the rebuild
  re-derive currently wins on load before choosing the fix.
- **[P2] Fix the movement-neutrality justification.** The doc leans on S1, but the landed S1
  shadow proves dispatch-precedes-Process-**marker** ordering + scope, NOT movement-output
  equality (`process_drive_locomotion_shell` is a read-only marker, not a position advance). The
  conclusion still holds, for a stronger verified reason: **no behavior system reads
  `mission.current` or `tick_counter`** — the only non-test reads are `world_hash` (the hash) and
  the debug S1 shadow; movement is driven by `movement_target`/`locomotor`. Re-justify on that.
- **[P3] Coverage note.** In-loop dispatch only fires for scoped units that are *active movers*
  that tick; the mover-collection guards ([movement_tick.rs:910-915](../../src/sim/movement/movement_tick.rs))
  skip `forced_drive_processed`/`low_bridge_tube_state` units (outside pure-Move scope). State this
  explicitly; confirm a guard-skipped scoped unit falls to tail authority with **exactly one**
  `tick_counter` increment (no double/zero count); add a test.
- **Test additions:** save/load-on-arrival round-trip; direct position-sequence-unchanged check;
  no-double-count for a guard-skipped scoped unit.
- **Non-interaction noted:** `mission/retask.rs:97` writes `mission.current` (DockTeardown) — out
  of S2 scope (scoped excludes dock); confirm no dock↔move boundary interaction.
