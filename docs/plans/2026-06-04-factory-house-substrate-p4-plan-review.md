<!--
Provenance: consolidated review of
  docs/plans/2026-06-04-factory-house-substrate-p4-plan.md (DRAFTED)
  + docs/plans/2026-06-04-factory-house-substrate-p4-design.md (APPROVED design)
  from four reviewer reports (RV1 codebase, RV2 parity, RV3 fit-determinism, RV4 consistency).
  De-duplicated, false-positives dropped, load-bearing findings re-verified against the
  committed tree this session (file:text anchors, not line numbers — the tree shifts).
Status: REVIEW. Verdict below. Address the BLOCKER before executing P4.
-->

# Factory/House Substrate — P4 Plan Review (consolidated)

| | |
|---|---|
| **Reviewed** | `2026-06-04-factory-house-substrate-p4-plan.md` + `…-p4-design.md` |
| **Reviewers consolidated** | RV1 codebase, RV2 parity-grounding, RV3 fit + hash-neutrality, RV4 internal-consistency + buildability |
| **Verdict** | **READY-WITH-FIXES** |
| **BLOCKER count** | **1** (one root defect, surfaced by 3 of 4 reviewers; RV4 split it into 2 — they share one fix site, P4-T5) |
| **CONCERN count** | 2 |
| **NIT count** | 4 |
| **Scope (unchanged by review)** | shadow-only, hash-neutral oracle; legacy `cancel_by_type_for_owner` stays authoritative; `world_hash.rs` untouched; `SNAPSHOT_VERSION` stays 17 |

## Verdict

**READY-WITH-FIXES.** The P4 design is parity-sound (RV2 PASS, no findings) and structurally
hash-neutral by construction (RV3 PASS: serde-free + `#[serde(skip)]` shadow + clone-only oracle +
no mutating tick call site + `world_hash.rs`/`SNAPSHOT_VERSION` untouched). The core primitives
(P4-T1 `cancel_active`, P4-T2 `cancel_one`/`CancelOutcome`, P4-T3 `start_next_queued`, P4-T4
re-export) and the world-level tests (P4-T6) are internally consistent and compile-plausible — the
`[A,B,A,C]` cancel-A → `[B,A,C]` first-match assertion is correct, the refund formula
`original_balance − balance` is correctly grounded in C8, and the legacy DRIFTs (`.rev()` last-match,
full-cost refund) are left authoritative and modeled correctly in the shadow, never equalized.

The one BLOCKER is entirely contained in **P4-T5** (the live `debug_assert_factory_cancel_refund`):
as written it is **not buildable** (calls a method that no task step defines) and would **panic in
debug builds during normal play** (its unconditional `AbandonedActive` expectation is wrong whenever
two of the same unit are queued). Both defects share one fix site. Fix P4-T5 and the slice is ready.

All codebase assumptions A1–A21 were spot-verified against the committed tree this session and hold.

---

## Ranked findings

### BLOCKER-1 — P4-T5 live assert: nonexistent helper + tail-retention precedence panic (one fix site)

This is one defect zone with two failure modes; RV4 raised both as BLOCKERs, RV1/RV3 raised the
precedence half as CONCERN. Verified against the tree; both are real.

**(a) `insert_for_assert` is referenced but never added by any concrete task step.**
P4-T5's assert body (`reg.insert_for_assert(key, f.clone());`) calls a `FactoryRegistry` method that
does not exist and is described only in prose (the "Helper note", plan lines ~765-776). The plan's
"Files touched" for `factory.rs` lists only `cancel_one`/`cancel_active`/`start_next_queued`/
`CancelOutcome` — not `insert_for_assert`. Verified: `factory.rs` has no such method today
(`fn insert_for_assert` → no matches); the only inserts into the private `factories` map are the
in-module `#[cfg(test)]` tests. **As written, P4-T5 does not compile.**

**(b) The cloned `f` retains the live `queue` tail, so a duplicate-of-active type returns
`QueuedRemoved` and the unconditional `debug_assert!` panics in normal play.**
Verified: `rebuild_shadow` builds the tail as `queue.iter().skip(1)` (factory.rs, confirmed this
session) — so the active object lives in `Factory.object` AND a second copy of the same unit sits in
`Factory.queue`. P4-T5 does `let mut f = factory.clone();` (resets progress/balance/flags) but
**never clears `f.queue`**, then routes through `cancel_one`, which scans the queued tail FIRST
(R1). When a player queues two Grizzlies (the front is the active build, the second is a tail copy of
the same `type_id`), `cancel_one(…, obj.type_id, …)` hits R1 → `QueuedRemoved` (refund 0), and the
assert `debug_assert!(matches!(outcome, AbandonedActive { refund } if refund == spent))` FAILS.
Because `debug_assert_production_shadow` runs every tick in debug (verified at world/mod.rs:2589,
inside `advance_tick`), this is a **debug-build panic in ordinary play** (queue-2-of-a-kind is
common), not a test-only edge. The design's own §4 caveat flags exactly this case ("the assert must
NOT pick a type that also sits in the tail … else `cancel_one` returns `QueuedRemoved`"), but the
plan's concrete P4-T5 code does not implement the branch — it only carries the caveat in prose.

**Fix (resolves both (a) and (b) — pick one, the design already offers the right one):**
- **Preferred (lowest surface):** demote the assert to a `#[cfg(test)]` world-test, OR keep it live
  but `f.queue.clear();` immediately after the clone (the assert only exercises the active-abandon
  path) so R1 cannot fire — then the unconditional `AbandonedActive` expectation is correct. The
  registry-level queued-vs-active precedence is already covered by the P4-T2 unit tests
  (`cancel_one_removes_first_matching`, `cancel_one_queued_preferred_over_active_same_type`).
- **If keeping the registry-driven live assert:** add `insert_for_assert` as a concrete
  `#[cfg(debug_assertions)] pub(crate) fn insert_for_assert(&mut self, key, f: Factory)` step in
  P4-T5's task body (not prose), AND either `f.queue.clear()` before insert OR branch the
  expectation (`QueuedRemoved` if `factory.queue.contains(&obj.type_id)`, else `AbandonedActive`).
- **Do NOT** adopt the design §4 "simplest robust form: drive `f.cancel_active(&mut econ)` directly"
  as written — `cancel_active` is **private to `factory.rs`** and the assert lives in
  `world/mod.rs` (a different module), so that call does not compile unless `cancel_active` is
  promoted to `pub(crate)`. See CONCERN-1.

Player-visibility: a debug-build crash during a routine action (queue two of a unit, in any
skirmish). Highest priority.

---

### CONCERN-1 — design §4 recommends a cross-module call to the private `cancel_active`

The design's §4 "simplest robust form" suggests the assert call `f.cancel_active(&mut econ)`
directly. But P4-T1/E4 keep `cancel_active` **private**, and the assert is in `world/mod.rs`, a
different module — a module-private `fn` on `Factory` in `factory.rs` is not callable from there. The
plan's actual P4-T5 body sidesteps this by going through `cancel_one` (which is `pub`), so the *plan*
is internally consistent; the *design doc* recommends a non-compilable form. Keep `cancel_active`
private (the in-module `factory.rs mod tests` reach it fine) and either drop the design §4
"simplest robust form" line or annotate that it requires `pub(crate)`. No code change to the plan if
the BLOCKER-1 `cancel_one`-based (or `#[cfg(test)]`) path is chosen.

### CONCERN-2 — design §2.1/§2.3 contradict each other on `cancel_active`'s return type

Design §0 (line ~36) and the §2.1 signature box (line ~146) say `-> i32`; design §2.3 (line ~233),
ledger #14, and §11 P4-T1 say `-> Option<i32>`. The **plan is consistent and correct** throughout
(`-> Option<i32>`): the `cancel_one` R2 branch requires the `Option` to distinguish "acted" (`Some`)
from "completed no-op" (`None`); a bare `i32` would not compile against the plan's `match
f.cancel_active(economy) { Some(refund) => …, None => … }`. Fix the two stale `-> i32` lines in the
**design** (§0, §2.1) to `-> Option<i32>` so the two docs do not contradict. Plan needs no change.

---

### NIT-1 — P4-T6 import line over-adds `VecDeque` (duplicate) and `PendingObject` (unused)

The plan's proposed production import note says to add `CancelOutcome, StepOutcome` and
"`VecDeque`/`PendingObject` if a test seeds them directly." Verified against the tree:
`production_shadow_tests.rs` already imports `use std::collections::{BTreeMap, VecDeque};` (line 18),
so re-adding `VecDeque` is a duplicate-import error; and the P4 world tests seed via `queued_item`
(not `PendingObject`), so `PendingObject` is unused at world level. The current production import is
`{BuildQueueItem, BuildQueueState, ProductionCategory, PRODUCTION_STEPS}` — `StepOutcome` is genuinely
missing and IS used by the P4 tests (`queue_advances_only_after_delivery` matches
`StepOutcome::Completed`). **Add exactly `CancelOutcome, StepOutcome` to the existing production import
set; add nothing from `std::collections`; do not import `PendingObject`.**

### NIT-2 — P4-T6 no-hash test carries a dead `{ let f = reg.view(...); let _ = f; }` block

Plan lines ~834-842. The block borrows `reg` read-only, does nothing, and has a self-contradicting
comment. `view` returns a read-only `FactoryView`, so it cannot re-seed the clone; the test's
contract (hash + legacy wallet unchanged) holds with the cost-0 clone regardless. **Delete the block
entirely** — the cost-0 acceptance form (E2's default) is sufficient and the block only muddies it.

### NIT-3 — P4-T6 no-hash test `matches!` arm includes an unreachable `| QueuedRemoved`

The fixture has a single `insert_queue` item → empty `queue` after rebuild, so only `AbandonedActive`
can occur; the `| CancelOutcome::QueuedRemoved` arm is unreachable for this fixture. Harmless (the
test asserts the hash regardless). Drop the arm or comment it as defensive. Pure tidiness.

### NIT-4 — design §5.1 wording: "no new call site" overstates; and `#[allow]` vs `#![allow]`

Two small design-doc wording fixes (plan is correct):
- §5.1 fact #2 says hash-neutrality rests on "no new authoritative call site," but P4-T5 DOES add a
  call into the tick-adjacent `debug_assert_production_shadow` (runs every debug tick). It is not a
  hash leak (clones + `debug_assert!` only, identical discipline to the proven
  `debug_assert_factory_conservation`), but the prose should read "no new *mutating/authoritative*
  call site; the debug-assert is a read-only clone-prober."
- §5.1 cites `#[allow(dead_code)]` (outer attr) for `factory.rs`; the file actually has
  `#![allow(dead_code)]` (module-level inner attr). The plan (A7) is correct; align the design.

---

## Required-revisions punch-list (by task)

| Task | Severity | Action |
|---|---|---|
| **P4-T5** | **BLOCKER** | Resolve the live-assert defect. Recommended: demote `debug_assert_factory_cancel_refund` to a `#[cfg(test)]` world-test (drops the per-tick risk; load-bearing guarantees are the unit tests + the no-hash acceptance test). If kept live: (1) add `insert_for_assert` as a concrete `#[cfg(debug_assertions)] pub(crate)` step in the task body (not prose), and (2) `f.queue.clear()` after the clone OR branch the expectation on `factory.queue.contains(&obj.type_id)`. |
| **P4-T6** | NIT-1 | Import line: add only `CancelOutcome, StepOutcome` to the existing production import; do NOT re-add `VecDeque` (already imported, line 18); do NOT add `PendingObject` (unused). |
| **P4-T6** | NIT-2 | Delete the dead `{ let f = reg.view(...); let _ = f; }` block in `factory_cancel_one_does_not_change_state_hash`. |
| **P4-T6** | NIT-3 | Drop (or comment as defensive) the unreachable `| CancelOutcome::QueuedRemoved` arm in the no-hash test's `matches!`. |
| **(no code change)** | — | Keep `cancel_active` private; route the assert via `cancel_one` (or `#[cfg(test)]`). |

## Design-doc assumptions also needing correction

These are doc-consistency fixes (the plan is the authoritative artifact and is correct on all four):

| Design loc | Fix |
|---|---|
| §0, §2.1 | `cancel_active -> i32` → `-> Option<i32>` (CONCERN-2); plan already uses `Option<i32>`. |
| §4 caveat | Drop or annotate "drive the assert via `f.cancel_active(&mut econ)` directly" — `cancel_active` is private to `factory.rs`, not callable from `world/mod.rs` without `pub(crate)` (CONCERN-1). |
| §5.1 fact #2 | "no new *mutating/authoritative* call site; the debug-assert is a read-only clone-prober" (NIT-4). |
| §5.1 fact #2 | `#[allow(dead_code)]` → `#![allow(dead_code)]` (module-level) (NIT-4). |

## Cross-cutting confirmations (not defects — recorded so they are not re-litigated)

- **Parity (RV2, PASS):** every P4 rule traces to a verified study contract (C6 first-match / C7
  delivery-bound advance / C8 spent-only refund / C12 completion-holds / C15 telescoping); both
  legacy DRIFTs are left authoritative and modeled correctly in the shadow, never equalized. The one
  inferred decision (queued-first precedence, design U3) and the completed-build no-op (U1) are
  honestly surfaced as UNCHECKED per the DRIFT-default. No reintroduced refuted claim.
- **Hash-neutrality / fit (RV3, PASS):** serde-free `Factory`/`FactoryRegistry`/`Economy`/
  `CancelOutcome`; `factory_shadow` is `#[serde(skip)]` (verified production_types.rs:248-249);
  oracle is always a clone; legacy cancel untouched (`.rev()` + `obj.cost.max(0)` verified at
  production_queue.rs:811/:837); `world_hash.rs` and `SNAPSHOT_VERSION` (=17, snapshot.rs:24)
  untouched; VecDeque/BTreeMap/integer-only, no float/RNG/HashMap. #1 invariant held.
- **Codebase (RV1, sound):** A1–A21 spot-verified — `Factory` fields/derives, `PendingObject`,
  `SpecialItem`, `StepOutcome` derive, `PRODUCTION_STEPS=54`, `advance_one_step`, `add_credits`,
  private `factories` map, `iter_insertion_ordered`/`view`, `rebuild_shadow` tail = `skip(1)`,
  re-export list, `debug_assert_production_shadow` chain (runs at world/mod.rs:2589),
  `empty_rules`/`queued_item`/`insert_queue`, `HouseState::new`, `factory_advance_step_does_not_
  change_state_hash` template, `armed_factory` (factory.rs:468), `InternedId::from_index` public
  (intern.rs:31), `advance_tick(&[], Some(&rules), &heights, None, None, 67)` signature.
