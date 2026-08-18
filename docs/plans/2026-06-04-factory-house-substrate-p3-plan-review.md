<!--
Provenance: consolidated /review-plan output for the P3 plan
  docs/plans/2026-06-04-factory-house-substrate-p3-plan.md
  and design docs/plans/2026-06-04-factory-house-substrate-p3-design.md,
  reconciling four reviewer reports (RV1:codebase, RV2:parity, RV3:fit-determinism,
  RV4:consistency) and re-verifying every load-bearing finding against the committed tree.
Source of truth: docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md
  (C2-C5, C12, C15; §6.2; §8 P3; §9.1). P0 charge math VERIFIED-LIVE v2 (NO x0.9).
Status: REVIEW. Not approved. Fix the BLOCKERs before executing.
-->

# Factory/House Substrate — P3 Plan Review (consolidated)

## Metadata

| Field | Value |
|---|---|
| Plan under review | `docs/plans/2026-06-04-factory-house-substrate-p3-plan.md` |
| Design under review | `docs/plans/2026-06-04-factory-house-substrate-p3-design.md` |
| Reviewers consolidated | RV1 codebase, RV2 parity, RV3 fit/determinism, RV4 consistency |
| Scope | P3 ONLY — `Factory::advance_one_step` + `Factory::set_rate`, hash-neutral oracle |
| Re-verification | every BLOCKER/CONCERN re-read this session against the committed tree (file:line cited) |
| Date | 2026-06-04 |

---

## Verdict: **READY-WITH-FIXES**

The design is sound and parity-grounded: the charge algorithm, SetRate scope (NO ×0.9), oracle/clone hash-neutrality, and the substrate-fit (probe beside the no-op arm) are all VERIFIED CLEAN against the v2 study contract and the committed P1/P2 code. **No design rework is needed.**

But the plan as written **does not compile** — 2 BLOCKERs are real and confirmed by direct re-read (wrong `advance_tick` arg positions; a `RuleSet` import path that does not resolve). Both are mechanical, one-line fixes. After they land, 3 CONCERNs (one vacuous test, two misleading comments — one of which the study itself propagates) and 2 NITs are cleanup that should be folded in during the same pass.

**BLOCKER count: 2.**

---

## Top must-fix items (the punch-list summary)

1. **BLOCKER — fix the `advance_tick` arg order in P3-T6** (`rules` is slot 2, not slot 4).
2. **BLOCKER — `crate::rules::RuleSet` does not resolve; use `crate::rules::ruleset::RuleSet`** in all three `factory.rs` signatures.
3. **CONCERN — `factory_last_step_charges_full_remainder` is VACUOUS for every cost** (the final `steps_left==0` step always charges 0; the whole remainder drains on the `steps_left==1` step at progress 53). Re-target the assertion.
4. **CONCERN — correct the "last step charges the remainder" narrative** in the plan comments AND the design (§2.2 / §7) — it is the `steps_left==1` step (progress 52→53) that drains the balance; the `steps_left==0` step is the div-by-zero guard and charges 0.
5. **CONCERN — `StepOutcome` in the `factory_oracle_step_trace` return type needs a module-level `use` in techno_ai.rs**, not a function-body `use` (a body `use` cannot bring a name into scope for the signature).
6. **NIT (promote to a task step) — add `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` to `StepOutcome`** (factory.rs:98 is a bare enum); it is currently only flagged in prose.

---

## Ranked findings (de-duplicated; severity = compile/parity impact × player-visibility)

### BLOCKER-1 — P3-T6 determinism test passes `advance_tick` args in the wrong positions (won't compile; also defeats the test)
- **Source:** RV4 (BLOCKER-1). RV1/RV3 verified the same signature but did not catch the misuse.
- **Re-verified this session.** Real signature: `advance_tick(commands, rules: Option<&RuleSet>, height_map, path_grid: Option<&PathGrid>, overlay_registry, tick_ms)` — `src/sim/world/mod.rs:1822-1830`. The existing P2 call is `advance_tick(&[], None, &heights, None, None, 67)` — `src/sim/world/production_shadow_tests.rs:343` — so slot 2 = `rules` (`None`), slot 4 = `path_grid` (`None`).
- **The defect:** plan P3-T6 line 863 writes `sim.advance_tick(&[], None, &heights, Some(&rules), None, 67)` and the note at lines 872-876 claims "the 4th arg is the `Option<&RuleSet>` tail." That is false: it passes `None` for `rules` (slot 2) and `Some(&rules)` for `path_grid` (slot 4) — a type error (`&RuleSet` ≠ `&PathGrid`), and even if it compiled it would run the cost-0 `None` rebuild, the opposite of the test's stated goal.
- **Fix:** `sim.advance_tick(&[], Some(&rules), &heights, None, None, 67);` — rules in slot 2, path_grid/overlay `None` in slots 4/5. Rewrite the note (lines 872-876) accordingly: "`rules` is the 2nd positional arg; pass `Some(&rules)` there. Slots 4/5 (`path_grid`, `overlay_registry`) stay `None`."
- **Player-visibility:** none directly (test-only), but it blocks the build and silently weakens the determinism guard — high priority because it gates everything after T6.

### BLOCKER-2 — `crate::rules::RuleSet` does not resolve (canonical path is `crate::rules::ruleset::RuleSet`)
- **Source:** RV4 (BLOCKER-2). RV1 wrote signatures using `&RuleSet` (assuming an import), so did not flag the fully-qualified mis-path; RV4 caught it.
- **Re-verified this session.** `src/rules/mod.rs:37` declares `pub mod ruleset;` with **no** `pub use ruleset::RuleSet` re-export (grep for `pub use ruleset|ruleset::RuleSet` in `src/rules/` returned no match). `RuleSet` is reached as `crate::rules::ruleset::RuleSet` — exactly how `world/mod.rs:40` imports it (`use crate::rules::ruleset::RuleSet;`).
- **The defect:** plan P3-T3 lines 470, 514, 516 write `rules: &crate::rules::RuleSet`. That path is unresolved → compile error.
- **Fix:** use `crate::rules::ruleset::RuleSet` in all three `factory.rs` signatures, or add `use crate::rules::ruleset::RuleSet;` at the top of factory.rs and write `&RuleSet`. (The latter matches the world/mod.rs convention.)
- **Player-visibility:** none (compile-time), but blocks the build.

### CONCERN-1 — `factory_last_step_charges_full_remainder` is VACUOUS for every cost, not just 700
- **Source:** RV4 (CONCERN-1) flagged it for cost=700; RV2 (NIT) correctly identified the mechanism. **This review sharpens both:** the test is vacuous for ALL costs, not a 700-specific accident.
- **Re-verified this session by replaying the exact ladder** (`steps_left = 54 - value`, increment-first): for value=53 the divisor is `54-53 = 1`, so the charge is `balance/1 = the whole remaining balance` and the balance drains to 0 at progress 53. For value=54 (the final step) the divisor is `54-54 = 0`, the div-by-zero guard fires, and `charge = balance = 0`. Confirmed for costs {1, 25, 53, 55, 700, 99991}: in EVERY case `bal_after_progress53 == 0` and `final_step_charge == 0`. There is no cost for which the `steps_left==0` step charges a nonzero remainder.
- **The defect:** plan P3-T2 lines 338-359. `remainder = f.balance` reads 0 at progress 53, the final step charges 0, the test asserts `0 == 0`. It passes but exercises nothing it is named for. (The mechanism it intends to prove — "charge the whole remainder, once, with the div-by-zero guard" — lives on the `steps_left==1` step at progress 53, which is a `Stepped`, not the `Completed` step the test inspects.)
- **Fix:** re-target the assertion to the `steps_left==1` step. Drive to progress 52 (`while f.progress < PRODUCTION_STEPS - 2`), capture `remainder = f.balance`, step once (the value=53 step), assert the charge `== remainder` and `f.balance == 0` and the outcome is `Stepped`; then step once more (value=54) and assert it is `Completed` and charges 0. Rename to something like `factory_steps_left_one_charges_full_remainder` and keep a one-line note that the `steps_left==0` final step is the guard and charges 0.
- **Player-visibility:** none (test correctness only), but it is a false safety signal for the load-bearing div-by-zero / full-remainder behavior — fix before relying on the suite.

### CONCERN-2 — the "last step charges the remainder" narrative is misattributed in BOTH the plan and the design
- **Source:** RV2 (NIT) and RV4 (CONCERN-2). Consolidated and broadened to the design.
- **Re-verified this session (same ladder replay).** The balance is fully drained on the `steps_left==1` step (`value 53`, `charge = balance/1`); the `steps_left==0` step (`value 54`) charges 0; completion runs `spend(0)`. For cost=1 the lone credit is charged at progress 53 (a `Stepped`), never on the final step.
- **The defects (narrative only — the algorithm and assertions are correct):**
  - Plan `factory_exact_cost_conservation_cost1_corner` comment, lines 316-318 ("1/1 == 1 on step 53->54's predecessor framing, and the final remainder charges the lone credit once") — implies the final/remainder step charges the 1; it does not.
  - Design §2.2 line 153-157 and §7 lines 474, 483-484 ("the LAST step charges the entire remaining balance"; "`1/1=1` on the last step") — same misattribution.
  - The contract's own §8-P3 wording is the upstream source of the imprecision; the plan need not patch the study, but should not propagate the phrasing.
- **Fix:** reword to: "the balance drains on the `steps_left==1` step (`value 53`, `charge = balance/1`); the final `steps_left==0` step is the div-by-zero guard and charges 0; completion's spend runs as `spend(0)`. Conservation depends on the `/1` step, not the guard step." Apply to plan lines 316-318 and design §2.2 / §7. (NOTE: the algorithm itself is bit-faithful — RV2 verified it against the binary `FactoryClass::AI` this session; only the prose is wrong.)
- **Player-visibility:** none, but a misleading rationale can send the implementer chasing a nonexistent "last step charges the 1" behavior.

### CONCERN-3 — `StepOutcome` in the probe's return type needs a module-level `use`, not a body `use`
- **Source:** RV4 (CONCERN-3). RV1/RV2 noted `StepOutcome` lacks derives but not the import-scope error.
- **Re-verified this session.** `techno_ai.rs:15-25` imports `super::Simulation`, `EntityCategory`, and the S1 helpers — but NOT `StepOutcome`. `StepOutcome` is re-exported at `crate::sim::production::StepOutcome` (`src/sim/production/mod.rs:50`). A `use` inside `factory_oracle_step_trace`'s body cannot bring a name into scope for the `-> Vec<(u64, StepOutcome)>` signature.
- **The defect:** plan P3-T5 line 728 return type + note lines 768-770 ("add a `use` at the call site or fully-qualify").
- **Fix:** add `#[cfg(any(test, debug_assertions))] use crate::sim::production::StepOutcome;` at module level in techno_ai.rs (gated to match the fn), OR write the return type as `Vec<(u64, crate::sim::production::StepOutcome)>`. Correct the note.
- **Player-visibility:** none (compile-time, debug/test-gated).

### NIT-1 — `StepOutcome` has NO derives; promote the fix from prose to an explicit task sub-step
- **Source:** RV1 (NIT 1), RV2, RV3, RV4 (NIT-1) all agree.
- **Re-verified this session.** `factory.rs:98` is a bare `pub enum StepOutcome { Idle, Stepped, Stalled, Completed }` — no `#[derive]`. Tests use `{other:?}` (needs `Debug`); the probe collects into a `Vec` and `matches!`/`PartialEq` are used in tests (need `PartialEq`, and `Copy` is convenient for the collected vec). The required `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` is serde-free, so it holds the no-hash contract.
- **The defect:** the plan handles it only in prose (P3-T2 lines 412-416, P3-T5 line 766); an implementer following the task list mechanically could skip it.
- **Fix:** add an explicit numbered sub-step in P3-T2: "Add `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` to `StepOutcome` (factory.rs:98) — no serde derive." (The plan already correctly says to verify the existing derive before editing; it just needs to be a step, not a note.)
- **Player-visibility:** none.

### NIT-2 — P3-T6 over-specifies imports
- **Source:** RV4 (NIT-2), RV1 (confirmed the existing imports).
- **Re-verified this session.** `production_shadow_tests.rs:13-18` already imports `Economy`, `HouseState`, `BuildQueueState`, `ProductionCategory`, `PRODUCTION_STEPS`, `BuildQueueItem`. The P3-T6 test bodies as written reference neither `PendingObject` nor `StepOutcome` by name (returns go into `let _`).
- **The defect:** plan note lines 796-798 ("Import `PendingObject`/`StepOutcome`/`Economy` as needed") implies a change that is not required for the tests as written.
- **Fix:** soften to "no new imports required for the tests as written; `PRODUCTION_STEPS`/`Economy`/`HouseState`/`BuildQueueState` are already imported (production_shadow_tests.rs:13-18)."
- **Player-visibility:** none.

---

## Dropped / downgraded findings (false positives or non-issues)

- **RV2 "binary omits the explicit `min(charge, balance)` clamp" (raised as CONCERN):** DOWNGRADED to a comment-only nit folded into CONCERN-2. RV2 itself proves it is a mathematical no-op for every input here (`balance/k <= balance` for k>=1; `min(balance,balance)=balance` for k==0), so the output is bit-identical with or without it. The only actionable part is the justification comment, which CONCERN-2's rewording already covers ("`charge <= balance` in every case"). Mirroring the explicit `charge = charge.min(self.balance)` for byte-faithfulness is OPTIONAL (the project's "model the primitive" default favors it, but output is identical either way). Not a blocker, not a standalone CONCERN.
- **RV3 CONCERN-1 (set_rate in the probe sets `suspended=false` on the clone) and CONCERN-2 (reset clone keeps `step_rate_frames=0`):** NOT raised as findings — both are explicitly hash-neutral (clone-only) and already surfaced as design open questions E2 (plan lines 933-939). `advance_one_step` does not read `step_rate_frames` in P3, so the rate value is inert. No action beyond confirming E2 with the design-lead.
- **RV3 / RV4 E1 framing ("54 Stepped" vs C12 `Completed`-on-54th):** NOT a defect. The plan already resolves it correctly (E1, plan lines 926-931): 53 `Stepped` + 1 `Completed` = 54 total step calls. C12 is authoritative over the study's test-shorthand wording. Confirm with the design-lead (E1), no code change.
- **All RV1 "verified correct" items (A1-A17, field/method/const/path spot-checks):** confirmed; not re-listed. Spot-re-verified this session: `Economy::spend` caps at `min(credits, amount)` and `available()` returns `credits` (economy.rs:52-62) → strict-`<` precheck + `debug_assert_eq!(paid, charge)` holds; `self.tick` exists (mod.rs:290); `live_object_order_snapshot` (mod.rs:924) + `self.substrate.entities.get` exist for the probe; `EntityCategory` is imported at module scope in techno_ai.rs:16.

---

## Design-doc assumptions also needing correction

The design is approved and needs no rework, but two prose items should be corrected in the same pass so the plan and design stay consistent:

1. **Design §2.2 (lines 153-157) and §7 (lines 474, 483-484): the "last step charges the entire remaining balance" / "`1/1=1` on the last step" framing is misattributed** (see CONCERN-2). The drain happens on the `steps_left==1` step (progress 52→53), and the `steps_left==0` step charges 0. The design's algorithm pseudocode is correct; only the surrounding prose mislabels which step drains. Reword to match CONCERN-2's fix.
2. **Design §4.2 / §10 cite `production_queue.rs:~218` with a `~`** — RV3 verified it is EXACT (`*credits_entry_for_owner(sim, owner) -= obj.cost` at production_queue.rs:218). Drop the `~` (cosmetic).

No other design assumption is wrong: the SetRate-source finding (legacy ×0.9 is REFUTED, `production_tech.rs:334` `* 9 / 10000`), the structural no-hash proof (no serde derive; no authoritative call site; oracle-always-a-clone), the cost-based-shadow E1 decision, and the FIT-(a) probe-beside-the-no-op-arm are all verified clean.

---

## Required-revisions punch-list (by task)

**P3-T2 (`factory.rs`):**
- [ ] Add `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` to `StepOutcome` (factory.rs:98) as an explicit numbered sub-step (NIT-1).
- [ ] Re-target `factory_last_step_charges_full_remainder`: drive to progress 52, assert the `steps_left==1` step (value 53) charges the whole remainder and zeroes the balance and is `Stepped`; then assert the final step is `Completed` and charges 0. Rename accordingly (CONCERN-1).
- [ ] Reword the `factory_exact_cost_conservation_cost1_corner` comment (lines 316-318): the lone credit is charged on the `steps_left==1` step (progress 53, a `Stepped`); the final step charges 0 (CONCERN-2).
- [ ] (Optional) Mirror the engine's explicit `charge = charge.min(self.balance)` for byte-faithfulness, OR just fix the safety comment at line 238 to "`charge <= balance` in every case (k>=1: `balance/k`; k==0: the whole balance), matching the engine's unconditional `min(charge, balance)` guard" (RV2, folded).

**P3-T3 (`factory.rs`):**
- [ ] Replace `&crate::rules::RuleSet` with `&crate::rules::ruleset::RuleSet` in all three signatures (lines 470, 514, 516), or add `use crate::rules::ruleset::RuleSet;` to factory.rs (BLOCKER-2).

**P3-T5 (`techno_ai.rs`):**
- [ ] Add `#[cfg(any(test, debug_assertions))] use crate::sim::production::StepOutcome;` at module level (or fully-qualify the return type) — the body `use` in the note is insufficient (CONCERN-3).

**P3-T6 (`production_shadow_tests.rs`):**
- [ ] Fix the `advance_tick` call: `sim.advance_tick(&[], Some(&rules), &heights, None, None, 67);` (rules in slot 2). Rewrite the note at lines 872-876 (BLOCKER-1).
- [ ] Soften the import note (lines 796-798): no new imports required for the tests as written (NIT-2).

**Design doc (same pass):**
- [ ] Reword §2.2 / §7 last-step narrative (CONCERN-2 / design item 1).
- [ ] Drop the `~` on `production_queue.rs:218` (design item 2).

**Confirm with the design-lead (open questions, no code change):**
- [ ] E1 — 53 `Stepped` + 1 `Completed` reading (already correct in the plan).
- [ ] E2 — `set_rate` stand-in input in the probe (hash-neutral; inert rate in P3).
- [ ] E3 — optional `debug_assert_factory_oracle_probe` (plan defaults to NOT adding it).
- [ ] E4 — `rebuild_shadow_inner` delegate shape (lower-drift default).

---

*End of review. Re-run `cargo check -p vera20k` after the two BLOCKER fixes; the CONCERN/NIT cleanup should land in the same task passes. The no-hash contract (`world_hash.rs` untouched, `SNAPSHOT_VERSION` 17, no serde derive, oracle-always-a-clone) is verified intact and must stay so through every task.*
