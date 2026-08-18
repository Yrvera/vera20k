# Slice S4a — TechnoClass common-body bracket + unit IsAlive early-returns — DESIGN

**Status:** ✅ **COMPLETE.** Option B shadow shell landed 2026-06-10 (`981ae795`); authoritative
flip landed 2026-06-11 (`684d9d22`, `SNAPSHOT_VERSION` 24). **Date:** 2026-06-10

## Progress (2026-06-10)

- **Shadow shell landed** (`src/sim/world/techno_ai.rs`): `techno_common_pre`/`techno_common_post`
  no-op stubs + `unit_techno_bracket` (pre → IsAlive guard B → dispatch MARKER → IsAlive guard E →
  post) wired into the Unit arm of the live-object walk; `BracketReach` enum; 3 tests. The
  authoritative `+0xC4`/mission-commit **stays in `movement_tick`** this step (shadow). Verified
  **hash-neutral**: full `sim::world` suite 273/273 incl. `global_skirmish_replay_is_deterministic_
  and_baseline_stable` (committed baseline unshifted).
- **Next (Option B authoritative flip — Ghidra-gated):** relocate the `+0xC4`/mission-commit out
  of `movement_tick:1045-1051` into the bracket's dispatch point, prove on the S2 arrival/idle
  goldens (re-baseline with reason if any tick shifts), after decode **U6** (contiguous order) +
  **U3** (post-dispatch IsAlive address). Until then guard E (`DiedInDispatch`) is unreachable
  (the dispatch is a marker) — documented in the enum.

---

**Parent:** `2026-06-10-s4-techno-common-prepost-design.md` (S4 split S4a/S4b/S4c, user-confirmed).
**Body map:** `docs/research/TECHNOCLASS_AI_UPDATE_BODY_SYNTHESIS.md` (docs-grounded; live decode
verifies U1–U6). S4a is the **structural, no-RNG** sub-slice — it does NOT need U2 (the
damage-particle draw count); it needs U3 (post-dispatch IsAlive addr) and U6 (contiguous order),
which are design-around-able plan-time gates (the same pattern S3 used for its G4 facing gate).

## 1. Goal (narrowed)

gamemd `TechnoClass::AI_Update` runs, contiguously, per live object, once per tick in LogicVector
order: **pre-mission block → `+0xC4`++ → `Mission_Dispatch` → post-mission block**, with two
IsAlive early-returns (synthesis §2: **B** pre-dispatch `+0x90==0` after the lethal pre-block;
**E** post-dispatch step-27). S4a reproduces the **bracket ordering + the two early-returns for
UnitClass**. It does **not** add the damage-particle RNG (S4b), the passive scanner (S4c), or any
new subsystem. Per the §10 inventory most pre/post steps are separate existing services or absent
(EMP/self-heal not modeled), so S4a's authoritative content is the **ordering shell**, not a
relocation of every step.

## 2. The central decision — host-siting (REQUIRES A CHOICE)

**Problem:** S2 left per-Unit dispatch *fragmented* across two tick phases:
- **Scoped movers** dispatch in-loop in `movement/movement_tick.rs:1045-1051` (Phase 1):
  `tick_counter++`, commit `derived_mission()`, before that unit's locomotor `Process`.
- **Idle / non-mover Units** project in the tail `refresh_mission_shadow_except`
  (`world/mod.rs:2692`, Phase ~9); their `tick_counter++` is the tail path.

gamemd has **one** contiguous per-object bracket for **every** Unit. A contiguous `pre → dispatch
→ post` therefore cannot simply wrap the two fragmented S2 sites. Two ways forward:

**Option A — Wrap-in-place (lowest risk, less faithful).** Add `techno_common_pre/post` calls
around each existing dispatch site (movement_tick for movers; the tail for idle). The bracket is
*logically* per-object but *physically* split across phases. Cheap, hash-neutral, no movement
restructure. Cost: not contiguous — post-block work for a mover runs in Phase 1, for an idle Unit
in Phase 9; any post-block step that must observe cross-phase state (none today, since EMP/
self-heal/particle are absent/deferred) would be mis-placed later.

**Option B — Unify into the `object_ai_stage` host (gamemd-faithful, higher blast radius).** The
host (`world/techno_ai.rs::object_ai_stage`, `world/mod.rs:2017`) already walks **all** live
objects in LogicVector order — the natural home for one contiguous per-Unit bracket. Move the
authoritative `+0xC4`++ + mission-commit out of movement_tick into the host, and have the host run
`pre → +0xC4 → dispatch → post` per Unit. Cost: the host runs at top-of-tick but the locomotor
`Process` (the movement_tick loop body) stays in Phase 1 — so "dispatch then Process" (S1/S2's
proven order) is preserved only if dispatch stays before Phase 1. The host at `:2017` is *before*
Phase 1 movement, so dispatch-in-host **does** precede Process — good. But moving the commit out of
the mover loop changes the arrival-tick interplay (mover commits in host, then moves in Phase 1) —
needs the S2 arrival-tick tests re-proven, and a `SNAPSHOT_VERSION` bump if any tick's hash shifts.

**Recommendation: Option B**, but **only if** the decode's U6 confirms the bracket→Process order
matches the host-at-:2017 placement, AND the move proves hash-neutral on the S2 arrival/idle
goldens. Rationale: B is the migration's actual target (one per-object AI pass; the migration
boundary §3.4 first-safe-slice is "a mobile Techno object-AI shell … mission dispatch before
locomotor Process"), and the host already exists with the right iteration order. A is a stopgap
that entrenches the fragmentation the ladder exists to remove. **Fallback to A** if B cannot be
shown hash-neutral within S4a's budget — then unification becomes its own slice. **User: pick A or
B before plan.**

## 3. The early-returns

- **B (pre-dispatch IsAlive, `0x006FA23C`, `+0x90==0`):** in Rust, "the unit died during the
  pre-block." Today no pre-block step kills a Unit (rocking/self-heal absent), so B is a **named
  guard that never fires yet** — implement it as `if !sim.is_alive(id) { return; }` after the
  (currently empty) pre-block, so it is correct when a lethal pre-block step lands. Hash-neutral.
- **E (post-dispatch IsAlive, step 27, addr U3):** "the unit died during mission dispatch." This
  one **can** fire today (a dispatch handler / commanded self-destruct). Implement as
  `if !sim.is_alive(id) { return; }` immediately after the dispatch commit, **before** any
  post-block work. Whether this changes the hash depends on whether any current post-dispatch Unit
  work runs on a just-died Unit — must be checked (likely hash-neutral; assert it).

## 4. Hash & versioning

Target: **hash-neutral** (Option A) or **minimal, justified shift** (Option B, if the commit move
perturbs an arrival/idle tick). Shadow-first: land the bracket recording its order in debug, prove
`state_hash` bit-identical (A) or re-baseline with the cited reason (B), then flip. No new RNG.

## 5. Plan-time Ghidra gates (design-around-able now; verify at plan)

- **U3** — exact post-dispatch IsAlive address (confirms E's placement is right after dispatch).
- **U6** — contiguous order B→C→D→dispatch→E (confirms the bracket shape; informs A-vs-B).
- **U1** — pre-block step list (confirms nothing lethal/RNG-bearing is silently omitted from the
  S4a pre-block; if a pre-block step DOES draw RNG — U4 cloak/spawn — it escalates to S4b scope).

## 6. Acceptance tests

- `techno_ai_pre_then_dispatch_then_post_order` — per Unit, the bracket runs pre → `+0xC4` →
  dispatch → post in that order (debug visit trace, like the S1 shadow).
- `unit_died_in_dispatch_early_returns_no_post` (E) — a Unit whose IsAlive is false after dispatch
  skips the post-block.
- `pre_block_death_guard_present_no_fire` (B) — the pre-block IsAlive guard exists and, with no
  lethal pre-block step today, never fires (no behavior change).
- `s4a_bracket_hash_neutral` (Option A) / `s4a_commit_move_golden` (Option B) — the chosen siting's
  hash contract holds on the S2 arrival/idle goldens.
- `tick_counter_still_exactly_once` — re-prove S2's exactly-once `+0xC4` survives the bracket
  (no double-count across the mover/idle paths).

## 7. Open question for the user

**Host-siting: Option A (wrap-in-place, stopgap) or Option B (unify into `object_ai_stage`, the
ladder's target)?** Recommendation B, contingent on hash-neutrality + decode U6. This is the one
decision that shapes the S4a plan; everything else follows from the synthesis + the inventory.
