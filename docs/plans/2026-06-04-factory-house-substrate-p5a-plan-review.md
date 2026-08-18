<!--
Provenance: consolidated review of
  docs/plans/2026-06-04-factory-house-substrate-p5a-plan.md (DRAFTED)
  + docs/plans/2026-06-04-factory-house-substrate-p5-design.md (APPROVED design)
  from four reviewer reports (RV1 codebase, RV2 parity-grounding, RV3 fit-determinism,
  RV4 consistency-buildability).
  De-duplicated, false-positives dropped, load-bearing findings re-verified against the
  committed tree this session (file:text anchors, not line numbers — the tree shifts;
  world/mod.rs is co-edited by a concurrent session).
Status: REVIEW. Verdict below. Fold the BLOCKER into P5a-T1 before executing.
-->

# Factory/House Substrate — P5a Plan Review (consolidated)

| | |
|---|---|
| **Reviewed** | `2026-06-04-factory-house-substrate-p5a-plan.md` + `…-p5-design.md` |
| **Reviewers consolidated** | RV1 codebase, RV2 parity-grounding, RV3 fit + hash-neutrality, RV4 consistency + buildability |
| **Verdict** | **READY-WITH-FIXES** |
| **BLOCKER count** | **1** (missing import; surfaced by RV4 only — RV1/RV2/RV3 missed it; verified real this run) |
| **CONCERN count** | 2 |
| **NIT count** | 6 |
| **Scope (unchanged by review)** | hash-neutral prep only; legacy `production_queue` charge/cancel/frames stays AUTHORITATIVE; `world_hash.rs`/`snapshot.rs` untouched; `SNAPSHOT_VERSION` stays 17; no serde derive; no un-skip |

## Verdict

**READY-WITH-FIXES.** The P5a design and plan are parity-sound and structurally hash-neutral by
construction. The producer pipeline (T1–T5: `cost × bonus` → BuildTimeMultiplier → C10 low-power
divide → C11 per-iteration MultipleFactory → wall) is x0.9-free, integer/i128 throughout, and matches
the C5/C10/C11 contract — every walked fixture (700→700, 67×1.15→77, count3/MF0.8→6 vs single-trunc 7,
50000/0.01→5_000_000, MTNK total→rate 12) reproduces the expected value. `category_for_object` delegates
to the verified Aircraft/Infantry binding (the refuted inverse can never return). The Lane-A temporal
`insertion_seq` mint (`front.enqueue_order`) is correctly grounded and provably hash-neutral — verified
HARD: a grep of `world_hash.rs` for `insertion_seq|next_insertion_seq|factory_shadow|seq_carry|economy`
returns **zero** matches, so the mint swap touches only `#[serde(skip)]`, no-serde-derive state the hash
never reads. No refuted v2 claim is reintroduced.

The one BLOCKER is entirely contained in **P5a-T1**: the producer and its `bst()` test helper reference
`PRODUCTION_RATE_SCALE` ~14×, but no task step adds the import — `factory.rs` imports only
`ProductionCategory` from `production_types`. As written, T1 fails `cargo check`. Add the one-line import
and the slice lands green.

All A-row preconditions (A1–A28) were spot-verified against the committed tree this session and hold.

---

## Ranked findings

### BLOCKER-1 — `PRODUCTION_RATE_SCALE` is referenced in T1 but never imported into `factory.rs` (`cargo check` fails at T1)

**Failure mode.** The T1 producer opens with `const SCALE: i128 = PRODUCTION_RATE_SCALE as i128;` and the
T1 `bst()` test helper references `PRODUCTION_RATE_SCALE` repeatedly (~14 uses across producer + helper +
the eight build_step_time unit tests). Verified: `factory.rs`'s only `production_types` import is
`use crate::sim::production::production_types::ProductionCategory;` — there is no `PRODUCTION_RATE_SCALE`
import, and `PRODUCTION_RATE_SCALE` is `pub(super) const PRODUCTION_RATE_SCALE: u64 = 1_000_000;` in
`production_types.rs` (not re-exported into scope). No task in the plan adds the import. As written, T1
does not compile, and every producer test depends on it.

**Single fix site.** **P5a-T1** — add to the `factory.rs` import block:
`use crate::sim::production::production_types::PRODUCTION_RATE_SCALE;` (resolves: sibling module within
`crate::sim::production`, `pub(super)` is visible). If T1 moves the producer to a sibling `factory_rate.rs`
(F2), the import goes there instead.

Player-visibility: none (build-time only) — but it blocks the entire slice from reaching green, so it is
the top priority.

---

### CONCERN-1 — `debug_assert_production_shadow` signature + call-site change is a guaranteed collision with the co-editing session

Verified: the committed `debug_assert_production_shadow(&self)` (world/mod.rs, anchor
`pub(crate) fn debug_assert_production_shadow(&self) {`) takes no args, and its sole tail call site
(`self.debug_assert_production_shadow();`, immediately before `let state_hash = self.state_hash();`) passes
none. T5 (ii)/(iii) change BOTH the signature (`+ rules: Option<&RuleSet>`) and that call line. The prompt
and the plan both flag world/mod.rs as co-edited; a signature+call-site edit on the one entangled file is
exactly the collision class to avoid.

**Recommendation (make the plan's fallback the default).** F1 already offers the `None`-arm fallback: keep
`debug_assert_production_shadow(&self)` unchanged and call `self.debug_assert_factory_step_matches_legacy(None)`
internally. Adopt that as the DEFAULT for P5a. The only thing needing `rules` is the (B) producer sub-check,
which is explicitly the "surface-only, never equalize" recorder — losing it in P5a costs nothing
load-bearing ((A) order + (C) conservation + (D) delivery still run). This shrinks the world/mod.rs diff to
"add one fn + one call line" and leaves the `advance_tick` tail untouched. Fix site: **P5a-T5 (ii)/(iii)
and F1** — pick the `None` form, not the threaded form.

### CONCERN-2 — the design's P5b recommendation to DROP `next_insertion_seq` from the hash contradicts STUDY §6.4 and the locked P5b seam; flag the contradiction in-doc

The design (and the plan's §E out-of-scope table: "DROP `next_insertion_seq` since the order is now
`enqueue_order`-carried") recommends P5b drop `next_insertion_seq` from the hashed/serialized set. But
STUDY §6.4 explicitly says `FactoryRegistry` hashes "the factory map **and** `next_insertion_seq` (the
counter must round-trip and hash)," and the prompt's LOCKED P5b seam lists adding
`FactoryRegistry next_insertion_seq` to the hash. The recommendation is defensible (after the T3 mint swap,
`next_insertion_seq` is never incremented and becomes dead state), but a P5a prep doc is making a P5b
hash-field call that contradicts two authorities **without surfacing the contradiction** — which is exactly
how the wrong hash-field list ships at P5b. Not a P5a-correctness defect (P5a hashes nothing new). Fix site:
**design §2.2/§5.2/ledger + plan §E** — add one sentence: "this REVISES STUDY §6.4 and the original P5b
seam; confirm with the design-lead at P5b before dropping `next_insertion_seq` from the hash set."

---

### NIT-1 — T2 Defense fixture uses the wrong INI key (`DeployToFire`, not `BuildCat=Combat`); test still passes but the fixture misleads

Verified: `build_cat` is parsed solely from `section.get("BuildCat")` (object_type.rs, anchor
`build_cat: section.get("BuildCat").and_then(BuildCategory::from_ini)`), and `BuildCategory::from_ini`
maps the string `"combat"` → `Combat`. `DeployToFire` is never consulted for `build_cat`, so the T2
fixture's `GAPILL ... DeployToFire=yes` does NOT route `GAPILL` to `Defense`. This does **not** break
`category_for_object_matches_rtti_table` — that test only asserts inf/veh/air/`GAPOWR`→Building and never
asserts the Defense row — so it is a misleading fixture, not a failure. Fix site: **P5a-T2 fixture + the
inline §496-503 note** — if a Defense assert is ever added, use `[GAPILL]\nBuildCat=Combat\n`. The plan's
own note already offers "drop the Defense row" as the fallback; this records the concrete correct key.

### NIT-2 — `factory_registry_iteration_is_insertion_ordered` all-`order:1` fixture makes the ordering assertion vacuous under the new mint (but it passes, and there is no determinism risk)

Verified (production_shadow_tests.rs, anchor `fn factory_registry_iteration_is_insertion_ordered`): every
`queued_item(...)` in that fixture passes `order: 1` (the trailing `1`), so under the temporal mint
(`seq = front.enqueue_order`) all six factories mint `insertion_seq == 1`. The assertion `seqs == sorted`
(all-equal is monotonic) and `seqs.len() == 6` both still **pass**. The assertion no longer proves distinct
ordering, only non-decreasing.

**De-escalation note (RV2 raised this to CONCERN citing non-determinism — that half is dropped as a false
positive).** The ties do NOT reintroduce sweep non-determinism: the registry stores factories in a
`BTreeMap<(InternedId, ProductionCategory), Factory>` (deterministic value-iteration by key), and
`iter_insertion_ordered`'s `Vec::sort_by_key` is a **stable** sort, so equal `insertion_seq` keys preserve
the deterministic BTreeMap input order. The only real cost is a weakened (vacuous-ordering) guard. Fix site:
**P5a-T3 / F5** — recommend stamping distinct `order` values per `(owner, category)` so the test keeps
guarding ordering after the mint; optional (it passes as-is), but a vacuous ordering assertion in a test
whose purpose is ordering is worth one fixture edit. (`insertion_seq_stable_across_rebuild` mutates only
`remaining_base_frames`, never `enqueue_order`, so its seq stays 1 across rebuild — passes unchanged.)

### NIT-3 — design vs plan producer visibility mismatch (`pub(crate)` vs `pub`)

Design §3.2/§4.1 declare `pub(crate) fn build_step_time` / `pub(crate) struct BuildStepTimeInputs` /
`pub(crate) fn category_for_object`; the plan T1/T2 declare all three as bare `pub`. Both compile under the
`pub use self::factory::{...}` re-export (a `pub`-or-`pub(crate)` item re-exported via `pub use` resolves),
but the two docs disagree. Fix site: **T1/T2 + design §3.2/§4.1** — pick one; recommend `pub(crate)` to
match the existing `start_next_queued` discipline (the producer has no cross-crate consumer). Cosmetic.

### NIT-4 — `production_category_for_object` reachability is over-hedged (F3); the `pub(super)` already suffices

Verified: `production_category_for_object` is `pub(super)` in `production_tech.rs` (anchor
`pub(super) fn production_category_for_object(`). `factory.rs` is a sibling submodule of
`crate::sim::production`, so `super::production_tech::production_category_for_object` is reachable with NO
visibility widen. The plan's F3 "widen to `pub(in crate::sim::production)` OR import via crate path —
confirm at impl time" is unnecessary caution. Fix site: **P5a-T2 import note / F3** — state the
no-widen `super::production_tech::...` (or the equivalent crate path) as the primary path; drop the
"widen" suggestion. Cosmetic.

### NIT-5 — inversion-assert (B) `build_time_multiplier_x1000.max(1) * 1_000` guard is intentional and safe (record so it is not re-litigated)

Verified: `ObjectType.build_time_multiplier_x1000` is clamped at parse to `(btm_f32.max(0.01) * 1000)` ≥ 10
(object_type.rs, anchor `build_time_multiplier_x1000: (btm_f32.max(0.01) as f64 * 1000.0).round() as u64`),
so the `.max(1)` in the (B) input construction can never fire on a real type — it is a harmless debug guard.
Even if a 0 reached it, (B) is a debug-only SURFACE assert that only checks the rate clamps to [1,255]; it
cannot corrupt authoritative state or the hash. No fix needed; flagged so the implementer knows it is
intentional, not a real multiplier path.

### NIT-6 — assert-name overpromise + minor T5 tidiness (record, no action required)

(a) The assert is named `debug_assert_factory_step_matches_legacy`, but its load-bearing checks are (A)
order (a same-source consistency check, since `insertion_seq` IS `front.enqueue_order` after T3) and (C)
conservation (which duplicates the existing `debug_assert_factory_conservation`); (B) deliberately does NOT
bit-compare against the legacy effective rate (frames↔step is not bit-identical — forcing equality would be
the "invent equivalence" anti-pattern). The design correctly scopes (B) as a DRIFT recorder; just be aware
the assert is lighter-weight de-risk than the "strongest possible de-risk" prose implies. (b) T5 (B)
double-binds the object (`let Some(_obj) = factory.object.as_ref() else {continue}` then
`factory.object.as_ref().unwrap().type_id`); reuse `obj.type_id`. Readability only. Fix site: **P5a-T5** —
optional.

---

## Required-revisions punch-list (by task)

| Task | Severity | Action |
|---|---|---|
| **P5a-T1** | **BLOCKER** | Add `use crate::sim::production::production_types::PRODUCTION_RATE_SCALE;` to the `factory.rs` import block (or to `factory_rate.rs` if the producer moves per F2). Without it T1 fails `cargo check`. |
| **P5a-T5 (ii)/(iii)** | CONCERN-1 | Adopt the F1 `None`-arm fallback as the DEFAULT: keep `debug_assert_production_shadow(&self)` unchanged, call `self.debug_assert_factory_step_matches_legacy(None)` internally, do NOT touch the `advance_tick` tail. Minimizes the world/mod.rs collision surface; only the (B) magnitude log is lost. |
| **design §2.2/§5.2 + plan §E** | CONCERN-2 | Add one sentence noting the "DROP `next_insertion_seq` at P5b" recommendation REVISES STUDY §6.4 and the locked P5b hash seam — confirm with the design-lead at P5b. |
| **P5a-T2** | NIT-1 | If a Defense assert is added, use `[GAPILL]\nBuildCat=Combat\n` (not `DeployToFire=yes`); otherwise the fixture is misleading-but-harmless. |
| **P5a-T3 / F5** | NIT-2 | Stamp distinct `order` per `(owner, category)` in `factory_registry_iteration_is_insertion_ordered` so the ordering assertion stays meaningful post-mint (passes as-is; recommended, not blocking). |
| **T1/T2 + design §3.2/§4.1** | NIT-3 | Reconcile `pub` vs `pub(crate)` on the producer/inputs/delegate (recommend `pub(crate)`). |
| **P5a-T2 / F3** | NIT-4 | State `super::production_tech::production_category_for_object` (no visibility widen) as the primary path. |
| **P5a-T5** | NIT-6 | Reuse `obj.type_id` after the `let Some(obj) = …` guard instead of re-`unwrap()`. |

## Dropped findings (false positives — re-verified against the tree this run)

- **"T4 re-export drops `FactoryView` / reorders" (RV1 NIT-1):** FALSE POSITIVE. The committed re-export
  block is `BuildEligibility, CancelOutcome, Factory, FactoryRegistry, FactoryView, PendingObject,
  SpecialItem, StepOutcome, PRODUCTION_STEPS, STEP_RATE_MAX, STEP_RATE_MIN` (production/mod.rs), and the
  plan's T4 block keeps `FactoryView` and adds the three new names. The advisory "ADD to the existing list,
  don't rewrite it" is sound housekeeping but not a defect.
- **"all-`order:1` ties reintroduce sweep non-determinism" (RV2 CONCERN-2, the determinism half):**
  DROPPED. The BTreeMap value order is deterministic and `sort_by_key` is stable; ties weaken the *guard*,
  they do not threaten determinism (kept as NIT-2 above for the weakened-guard half only).

## Cross-cutting confirmations (not defects — recorded so they are not re-litigated)

- **Parity (PASS).** Every producer rule traces to a verified contract: T1 `cost × bonus` is x0.9-free
  (the legacy `cost * speed_x1000 * 9 / 10000` at production_tech.rs is the REFUTED model, left
  authoritative + DRIFT, NOT reused — A28 verified); the C10 Max-clamp gate fires only when `ratio < 1.0`;
  the `d <= 0` → 0.01 divisor floor; the C11 per-iteration MultipleFactory truncation gated `> 0`; the wall
  branch resolved as `category == Building && wall`. `set_rate` owns the `/54` + `clamp[1,255]` (verified:
  `set_rate(661) → 12`, `(14000) → 255` already pinned; no-object → 0 sentinel). `category_for_object`
  honors the verified Aircraft@Aircraft / Infantry@Infantry binding (the refuted inverse cannot return);
  `SpecialItem` 0/-1 and the purifier base are untouched.
- **Hash-neutrality / determinism (PASS).** No serde derive added — `Factory`/`FactoryRegistry`/`Economy`/
  `PendingObject`/`SpecialItem`/`StepOutcome`/`CancelOutcome` stay serde-free; `factory_shadow` is
  `#[serde(skip)]` (production_types.rs:248-249) and `economy` is serde-skip on `HouseState`. `world_hash.rs`
  never reads the mint or the shadow (grep returned zero matches), so the T3 `front.enqueue_order` mint swap
  is provably hash-neutral. No new authoritative call site — the `EntityCategory::Structure` arm stays
  no-op, `refresh_production_shadow` still only calls `refresh_economy_shadow` + `rebuild_shadow`, and the
  producer/`set_rate`/`start_next_queued`/inversion model run only from `#[cfg(debug_assertions)]` /
  `#[cfg(test)]` clone paths. `SNAPSHOT_VERSION` stays 17 (snapshot.rs:24, pinned by
  `snapshot_version_is_17_in_shadow_phase`). Integer/i128-only, no RNG, BTreeMap+VecDeque; `enqueue_order`
  is strictly monotonic from `next_enqueue_order` (starts at 1, saturating_add) → no ties → deterministic.
- **Codebase (sound).** A1–A28 spot-verified: `#![allow(dead_code)]` (factory.rs); `set_rate(build_step_time:
  i32)` does `/54` + `clamp(1,255)` with no-object→0; `PRODUCTION_STEPS = 54`; `front` bound by
  `let Some(front) = queue.front()` BEFORE the mint block, `BuildQueueItem.enqueue_order: u64` exists;
  `FactoryRegistry.factories` PRIVATE; `start_next_queued` `pub(crate)`; `advance_one_step(&mut Economy)`;
  the `debug_assert_production_shadow` chain runs at the tick tail before `state_hash()`; `object_type`,
  ObjectType `cost`/`build_time_multiplier_x1000`/`wall`, all six `rules.production.*_ppm` fields, the test
  helpers (`empty_rules`/`queued_item`/`insert_queue`/`HouseState::new`), and the P3/P4 no-hash acceptance
  templates all present as the plan claims.
- **Buildability.** Aside from BLOCKER-1, the four pieces are compile-plausible: the (B)/(C)/(D) clone loops
  hold only shared `&self` borrows (the `Vec<&Factory>` from `iter_insertion_ordered()` coexists with the
  disjoint `self.object_type(...)` / `self.interner` shared borrows); the dormant probe returns
  `Copy` tuples; `start_next_queued` is `pub(crate)` and reachable; the world tests call
  `advance_tick(&[], Some(&rules), &heights, None, None, 67)` matching the existing P3/P4 call shape (2nd
  positional arg `Option<&RuleSet>`).

---

*End of P5a plan review. P5a is READY once BLOCKER-1 (the `PRODUCTION_RATE_SCALE` import in T1) is folded
in. Adopt the F1 `None`-arm fallback as the default (CONCERN-1) to keep the world/mod.rs diff minimal on
the co-edited file, and add the STUDY §6.4 / P5b hash-seam contradiction note (CONCERN-2). The remaining
six items are cosmetic/test-fixture polish. No refuted v2 claim is reintroduced; the no-hash contract holds
by construction, gated by `factory_flip_prep_does_not_change_state_hash` +
`snapshot_version_is_17_in_shadow_phase`.*
