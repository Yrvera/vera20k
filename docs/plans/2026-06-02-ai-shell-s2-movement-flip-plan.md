# Plan — AI-shell Slice S2: absorb the ground-movement phase into the per-object stage (hash-neutral)

**Status:** DRAFT for review (revised). Implements design §9 Slice S2 + §8 ledger #1
(`TECHNOCLASS_FOOTCLASS_SUBSTRATE_SERVICE_DESIGN.md`), with the scope/golden corrections in §0.1.
**Predecessors landed on `dev`:** S0 (`eff7fc09`), S1 (`1241206b`), FIX (`dfd9f7a4`).
**Date:** 2026-06-02.

Two sub-slices, each committed to `dev` separately: **S2a** (relocate the no-op stage) and **S2b**
(absorb the ground-movement loop into it, faithfully).

---

## 0. The binary finding that drives this plan (verified 2026-06-02)

S2 is the first slice that makes the per-object AI stage own real work (the locomotor). Where the stage
sits in `advance_tick`, and whether the absorb changes observable output, was verified against the
binary before committing.

**gamemd runs one per-object pass per tick, in LogicVector order, with no global pre-movement vision
pass.** `Main_Tick` (`0x0055D360`) calls the per-tick logic update once
(`LogicClassPerTickUpdateLiveVector`, decompiled this session); that update iterates the live object
vector and calls each object's `AI` (`vtable+0x5C`) in a single loop. The locomotor `Process` runs
**inside** each object's `AI` pass (design §7.3 / lane D6). There is **no global "recompute all sight"
call before that loop** — sight is maintained per-object as the object moves. So an object's vision
always reflects its **post-move** position the same frame. (HIGH confidence: the absence of a global
sight pass ahead of the loop is read directly from the decompile; per-object reveal-on-move is the
established Foot mechanism carried from D6.)

**VERA splits this into ordered global phases** ([`world/mod.rs::advance_tick`](src/sim/world/mod.rs:1751)):
- Phase 1 ground movement — [`:1783`](src/sim/world/mod.rs:1783) (`tick_movement_with_grids`,
  processes movers in **live order** with occupancy feedback — confirmed at
  [`:1786`](src/sim/world/mod.rs:1786) / [`movement_tick.rs:963-1009`](src/sim/movement/movement_tick.rs:963)).
- Phase 2 air/special movement (teleport, tunnel, rocket, droppod, parachute) — [`:1820`](src/sim/world/mod.rs:1820).
- Phase 2.5 rocking (render); aircraft mission state machines — [`:1914`](src/sim/world/mod.rs:1914)
  (behavior; reads positions); wake (render).
- Phase 3 **global** vision recompute — [`:1965`](src/sim/world/mod.rs:1965) (`refresh_fog`).
- Phase 4 power; Phase 5 combat; … ; AI stage parked at **end-of-tick** ([`:2403`](src/sim/world/mod.rs:2403)).

Critically: **ground movement (P1) already runs before the global vision recompute (P3)** — so VERA's
vision is correct today. The only thing wrong for the migration is that the per-object AI stage is
parked at end-of-tick, *after* vision and combat, with S0's comment committing to absorb behavior
"without moving this call site" ([`:2397`](src/sim/world/mod.rs:2397)).

---

## 0.1 Scope & golden corrections vs the design (and vs an earlier draft of this plan)

The design's §9 S2 text ("scoped UnitClass path only … `SNAPSHOT_VERSION` bump and a fresh golden")
rests on a tick model that does not match VERA's actual `advance_tick`. Two corrections, both
load-bearing:

1. **Scope: whole ground-movement phase, not a scoped subset.** The per-object stage must own the
   *per-object* locomotor `Process`, because Slice S3 appends Fire→Facing **after** each unit's move
   within the same pass — a global movement call cannot host that. Moving only a subset (the
   `is_s1_scoped` set, or only `UnitClass`) into the stage while the rest stay in Phase 1 **splits the
   single live-order movement pass**, changing cell-contention arbitration vs gamemd (which moves all
   objects in one pass). That is a real drift, and it would persist across every slice until the last
   mover migrates. Absorbing the **entire** ground-movement loop (vehicles + infantry + ships, every
   ground mover) in one faithful step keeps the single live-order pass intact → **zero contention
   drift**. `is_s1_scoped_move_unit` ([`techno_ai.rs:127`](src/sim/world/techno_ai.rs:127)) stays as
   the **S1 shadow predicate** it already is; it is not used to gate S2's absorb.

2. **Golden: S2 is hash-neutral; no `SNAPSHOT_VERSION` bump.** If the stage is placed at **Phase 1's
   current slot** (~[`:1786`](src/sim/world/mod.rs:1786)) and the ground loop is relocated *faithfully*
   — same mover population, same live order, same occupancy feedback, same per-mover `scenario_rng`
   draw order — the per-tick `state_hash` is **bit-identical**. Ground still moves before air (P2) and
   before vision (P3), exactly as today; only the *call structure* changes (driven per-object from the
   stage instead of one global call). The observable behavior changes the design attributed to S2 do
   not actually occur until **S3** (Fire/Facing per-object coupling) and **S5** (mission dispatch
   becomes authoritative — until then the stage's "dispatch" is an inert marker, so dispatch-before-
   process has no output effect). **The golden staying bit-identical IS the acceptance proof** that the
   relocation is faithful; a moved hash means the refactor has a bug, not that S2 "correctly changed
   behavior."

   (An earlier draft of this plan proposed a scoped subset with a golden rebaseline and a documented
   transient contention drift; a wider-scope variant with a golden bump was also considered. Both are
   superseded: the faithful whole-phase relocation is strictly better — zero drift, hash-neutral, and
   it sets up S3 without a placement contradiction.)

---

## 1. Scope and non-goals

**In scope (S2b):** the entire current Phase-1 ground-movement loop
(`tick_movement_with_grids` → the per-mover step at [`movement_tick.rs:978`](src/sim/movement/movement_tick.rs:978)),
relocated to run per-object from the stage, for **all ground movers** (the movers the loop builds at
[`:963-975`](src/sim/movement/movement_tick.rs:963): `movement_target.is_some()`, not air, not
underground — vehicles, infantry, ships). Plus the per-object `+0xC4` counter for `UnitClass`.

**Out of scope / NOT this slice:**
- Air/special movement (Phase 2) — stays where it is, after the stage, unchanged. (Ledger #2, later.)
- Combat Fire/Facing per-object coupling → **S3** (the stage will be ready to host it — §7).
- Mission-dispatch **authority** (`MissionCom` authoritative, `ready_to_commence`) → **S5**. The stage
  reads the **shadow** `mission.current` only; it does not make dispatch authoritative, and adds no
  parallel mission field.
- Per-leaf **behavior** (infantry fear/prone/sequencer, harvest brain, spawn, turret) → S3/S4/S6/S7/S8.
  S2b relocates only the **basic locomotor move** for non-Unit ground categories; their leaf behavior
  is untouched and stays in its current phase. (Relocating infantry/ship *basic move* ahead of S6 is
  content-neutral — the move-step is shared locomotor logic, not leaf behavior — and is required to
  keep the single-pass contention; see §6.)

---

## 2. S2a — relocate the no-op stage to immediately before Phase-1 ground movement (hash-neutral)

**Goal.** Move `self.object_ai_stage()` (and, in debug builds, `self.debug_assert_s1_shadow()`) from
end-of-tick ([`:2403`](src/sim/world/mod.rs:2403)/[`:2415`](src/sim/world/mod.rs:2415)) to **immediately
before** the Phase-1 ground-movement call — i.e. just before `let movement_order = …` at
[`:1786`](src/sim/world/mod.rs:1786), after the command-region `flush_pending_delete`
([`:1779`](src/sim/world/mod.rs:1779)). The stage is still the S0 no-op, so this changes nothing the
hash observes.

**Why before Phase 1 (not :1898).** Placing the stage just before the ground loop is the slot the loop
will be absorbed *into* (S2b), and it keeps movement in its current relative position (before air,
before vision) → zero downstream reordering. (An earlier draft suggested ~:1898, between Phase 2 and
vision; that would have moved ground after air. Phase-1's slot is strictly better — no ground-vs-air
change.)

**Mission-shadow read.** S1's shadow reads `entity.mission.current` (set by `refresh_mission_shadow`,
currently end-of-tick [`:2407`](src/sim/world/mod.rs:2407)). With the stage now at tick-start it reads
**last tick's** `mission.current`. For an in-scope S1 mover that value is stable (`Move` stays `Move`
while it keeps moving), so the debug `MissionType::Move` assert still holds. Leave
`refresh_mission_shadow` and `debug_assert_mission_shadow_consistent` at end-of-tick (they validate
against the legacy machines' end-of-tick state). Document in the S1 shadow that its read is one tick
stale by construction at this position — acceptable because the read is an inert marker until S5.

**Becomes authoritative.** Nothing. Still a no-op walk; movement still runs as the Phase-1 global call
immediately after.

**Files/surfaces.**
- [`src/sim/world/mod.rs`](src/sim/world/mod.rs) — move the `object_ai_stage()` call (and the debug S1
  assert) to before [`:1786`](src/sim/world/mod.rs:1786). Rewrite the S0 doc-comment that says "without
  moving this call site" to the S2a rationale (the stage MUST precede movement to host the per-object
  spine; movement stays pre-vision so sight is unaffected).

**Acceptance tests.**
- `s2a_relocation_is_hash_neutral` — full-replay golden over a fixed skirmish seed; per-tick
  `state_hash` **bit-identical** to the pre-S2a (post-S1) baseline. (Load-bearing: a no-op relocation
  that moves the hash is a bug.)
- `s2a_stage_runs_before_ground_movement` — structural guard: the stage call precedes the Phase-1
  movement call in `advance_tick`.
- Existing S0/S1 tests pass unchanged (the stage body is untouched; S1 shadow now reads last-tick's
  stable mission).

---

## 3. S2b — absorb the ground-movement loop into the stage, per-object, faithfully

**Goal.** Relocate the ground-movement work from the standalone Phase-1 call into the per-object stage
walk: for each live object in live order, the stage runs (Unit) `+0xC4` increment → (shadow) mission
read → locomotor move-step; (other ground mover) → move-step. The standalone `tick_movement_with_grids`
Phase-1 call is **retired**. Because the relocation is faithful (same population, order, occupancy
feedback, RNG draw order), the golden is **bit-identical** — no `SNAPSHOT_VERSION` bump.

**The shared move-step extraction.** S1's `process_drive_locomotion_shell` is a read-only marker — it
does not move the unit. S2b needs the **real** per-mover step: the body of the loop at
[`movement_tick.rs:978`](src/sim/movement/movement_tick.rs:978) (snapshot → stale-block-set refresh →
inner move/crush/bump → commit). Extract it into a callable single-mover function the stage invokes per
object. **The loop *prelude* state must move with it** — the movers-list build
([`:963-975`](src/sim/movement/movement_tick.rs:963)), A*/grid setup, `entity_block_sets` +
`block_set_built_at_gen` caches, `forced_drive_processed`, occupancy generation tracking, and the
`MovementStats` accumulator. Options for hosting that per-tick prelude state so the per-object stage can
use it (decide in implementation, document the choice):
  1. A short-lived `GroundMoveContext` struct built once at the top of the stage pass (before the
     walk) and threaded into each per-mover call — mirrors the current loop-local state exactly.
  2. Run the existing `tick_movement_with_grids` prelude once, then drive its per-mover step from the
     walk. (Same effect; pick whichever keeps the borrow structure cleanest.)
This is the same extraction the mission/radio Plan-2 L1 targets at this anchor — **do it once**;
whichever slice lands first owns it, the other consumes it (mirror the S0/L1.0 ownership
reconciliation).

**Mover order equivalence (the faithfulness invariant).** The stage walks `for_each_live_object` (live
order). The current loop's movers = `entity_order` (= `live_object_order_snapshot`, same source)
filtered to `movement_target.is_some()` + non-air/underground. So the stage, calling the move-step only
for objects passing that same filter, visits movers in the **identical order**. Occupancy feedback
(each mover sees prior movers' committed moves) is preserved because the walk commits in order, exactly
as the loop does. RNG: `scenario_rng` is consumed per-mover in that same order → identical draw
sequence.

**The `+0xC4` counter.** Add `ai_tick: u32` to `GameEntity`, incremented in the stage's `Unit` arm
immediately **before** the shadow mission read (the verified increment-before-dispatch point, design
§7.3 step 21→22). **Do NOT hash `ai_tick`** — its only consumer (the mission-dispatch gate) is not
authoritative until S5; hashing a free-running counter now would move the golden for a behaviorally
inert reason. Record the deferred-hashing decision in `state_hash`'s field list and the field's
doc-comment. The increment **point** is pinned by a test regardless of hashing.

**Becomes authoritative.** The per-object structure of ground movement (driven from the stage in live
order) and the `+0xC4` increment point. **No observable behavior changes** — movement output is
bit-identical; dispatch stays inert (S5) and combat coupling is untouched (S3).

**Parity risk.** The risk is entirely in the **refactor faithfulness**, not in an intended behavior
change. A mistake in prelude-state hosting, mover ordering, occupancy commit order, or RNG draw
position would move the golden — and the `s2b_*` hash-identity tests reject exactly that. There is **no
contention drift** (whole phase relocated intact) and **no vision lag** (movement stays before P3
vision). Ground-vs-air order is unchanged (stage at Phase-1's slot, air still P2 after).

**Acceptance tests.**
- `s2b_ground_absorb_is_hash_identical` — full-replay golden over a fixed multi-unit skirmish
  (vehicles + infantry + ships moving, with cell contention) is **bit-identical** to the pre-S2b
  baseline. This is the central proof of faithful relocation.
- `s2b_c4_counter_increments_before_dispatch` — `ai_tick` is incremented immediately before the shadow
  mission read in the `Unit` arm (pins the increment point even though unhashed).
- `s2b_c4_counter_unhashed` — adding/incrementing `ai_tick` leaves `state_hash` unchanged.
- `s2b_mover_order_matches_legacy_loop` — the per-object move dispatch order equals the legacy movers
  order for a fixture with interleaved categories (guards the faithfulness invariant directly).
- `s2b_scenario_rng_draws_unchanged` — `scenario_rng` draw count/position over a bump/scatter scenario
  is identical pre/post relocation.
- `s2b_phase1_call_retired` — the standalone `tick_movement_with_grids` Phase-1 call site is gone
  (movement is driven solely from the stage); guards against double-processing.

---

## 4. Open items / implementation checks

- **Verify `tick_movement_with_grids` consumes `entity_order` *only* to build the movers list** (and
  any per-mover lookups), with no other dependency on the full order — so driving the same per-mover
  step from the stage walk is behavior-equivalent. (Read the full fn before extracting.)
- **Prelude-state hosting** (§3) is the main mechanical risk — confirm the chosen `GroundMoveContext`
  carries every piece of loop-local state the per-mover step reads/writes, with the same lifetimes.
- **`gate_runtime::tick_gate_runtimes`** runs right after Phase-1 movement ([`:1811`](src/sim/world/mod.rs:1811)).
  With movement now driven from the stage (immediately before its old slot), confirm gate runtimes
  still see post-move positions (they should — the stage runs just before, same as P1 did).
- **Infantry/ship basic-move relocated ahead of S6/S7** — content-neutral (locomotor move only, no leaf
  behavior), but note it so S6/S7 authors know the basic move already lives in the stage and only leaf
  behavior remains to absorb.
- **Mid-tick-spawned movers** — the stage walks the start-of-tick live order; objects added later in
  the tick (e.g. production) are not walked this tick, same as Phase-1 movement only moved
  start-of-tick movers. No change vs today; revisit for the same-pass re-read model at S5 (design C9).
- **`ai_tick` unhashed until S5** — record where `state_hash` lists fields so a future reader does not
  "fix" the omission.

---

## 5. How this sets up S3 and S5

- **S3 (Fire→Facing):** the stage now owns the per-object move, so S3 appends `Fire_At_Target` →
  `Facing_Update` in the `Unit` arm **after** the move-step, retiring the global combat/turret sweeps
  for stage-handled units. *That* is where the first observable, hash-moving flip happens (fire-before-
  facing coupling) — with its own `SNAPSHOT_VERSION` bump and gamemd-evidence-backed golden. S2
  deliberately leaves the hash untouched so S3's golden delta is attributable purely to combat
  coupling.
- **S5 (mission authority):** `refresh_mission_shadow`/dispatch becomes authoritative and moves into
  the stage *before* the move-step — at which point dispatch-before-move stops being an inert marker
  and the `+0xC4` counter gains its consumer (and becomes hashed).

---

## 6. Rollback

S2a and S2b are independent commits. If S2b's hash-identity test fails and the cause is not quickly
found, revert S2b alone — S2a (the no-op relocation) stands on its own and leaves the stage correctly
positioned for a re-attempt. S2a reverts cleanly to the S1 state (call-site move only).

---

## 7. Invariant check (design §9 hard constraints)

- **sim/ purity** — no render/ui/audio/net touched. ✓
- **No class tree / dyn / vtable** — dispatch stays `match category`. ✓
- **advance_tick phase order** — S2a moves the AI-stage call site (design-sanctioned for S2) and S2b
  folds the ground-movement phase into it; no phase is collapsed or reordered relative to vision/
  combat, and the fold is proven hash-identical. The one explicit structural change (movement driven
  from the stage) is documented and behavior-preserving. ✓
- **Shadow-first** — the whole slice is hash-neutral: S2a no-op relocation; S2b faithful relocation
  (golden-identical); `ai_tick` unhashed until its S5 consumer. No authority flips ahead of S3/S5. ✓
- **No new presence/lifecycle owner** — the move-step routes through the existing movement/occupancy
  APIs; the stage adds no `store.insert`/`logic.push`/`occupancy.add`. ✓
- **Frame-anchored timers** — `ai_tick` is a counter, not a timer; no per-tick decrement introduced. ✓
- **State-hash is a determinism oracle** — S2 introduces no new gamemd-matching behavior, so no golden
  re-baseline and no new evidence artifact is required; the hash-identity tests assert faithfulness.
  (The gamemd-evidence-backed golden changes arrive at S3/S5.) ✓
