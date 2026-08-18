<!--
Provenance: CONSOLIDATED review of
  docs/plans/2026-06-04-factory-house-substrate-p5b-plan.md (DRAFTED)
  + docs/plans/2026-06-04-factory-house-substrate-p5b-design.md (APPROVED design)
  from four adversarial reviewer reports (RV1 codebase, RV2 parity-grounding,
  RV3 fit + hash + determinism, RV4 consistency + buildability).
  De-duplicated, false-positives dropped, EVERY load-bearing finding re-verified
  against the LIVE tree this session (file:TEXT anchors, not line numbers — the tree
  shifts; world/mod.rs is co-edited by a concurrent session).
House style mirrored from 2026-06-04-factory-house-substrate-p5a-plan-review.md.
Status: REVIEW. Verdict below. Fold the BLOCKERs before executing P5b.
-->

# Factory/House Substrate — P5b Plan Review (consolidated, the authority flip)

| | |
|---|---|
| **Reviewed** | `2026-06-04-factory-house-substrate-p5b-plan.md` + `…-p5b-design.md` |
| **Reviewers consolidated** | RV1 codebase, RV2 parity-grounding, RV3 fit + hash + determinism, RV4 consistency + buildability |
| **Verdict** | **NOT-READY** (the flip is sound; 4 BLOCKERs break the test/debug build or open a hash hole as literally written) |
| **BLOCKER count** | **4** (B1 debug-assert panic · B2 `remaining_base_frames` hash-hole/symmetry · B3 cost-0 fixture · B4 deleted-fn-with-surviving-test) |
| **CONCERN count** | **5** (C1 delivery trigger re-point · C2 snapshot API · C3 `StepInputs` re-export · C4 `factory_count` type · C5 borrow-prepass signature) |
| **NIT count** | 4 |
| **Scope (unchanged by review)** | the atomic authority flip: serde + un-skip the five shadow types; hash fold (ADD Factory+Economy, REMOVE `remaining_base_frames`+`progress_carry`, DROP `next_insertion_seq`+`seq_carry` FIELDS); real-wallet `step_all` at Phase-7 head before the house tail (C1 fold); persist progress (reconcile-not-rebuild); retire upfront-charge / `.rev()`-full-refund / frames-timer / auto-create-house; bump 17→18 |

## Verdict

**NOT-READY** — the *architecture* is parity-sound and honors every locked decision, but the plan as **literally written** will not reach a green test build: it panics one debug-build invariant (B1), ships a hash hole on `remaining_base_frames` (B2), wires five economy/charge tests through a cost-0 fixture that makes them inert or fail (B3), and deletes a function while keeping a test that calls it (B4). All four are bounded, one-task scope additions — none invalidates the D-PARITY-MIN design (one wallet, `queues_by_owner` stays the hashed queue-of-record + temporal source, `active_producer_by_owner` KEPT hashed, x0.9-free producer, D1/D2 honored, the §3.4 override correct, the §3.3 double-hash audit airtight). Fix B1–B4 + re-point the C1 delivery trigger, and the plan moves to READY-WITH-FIXES.

**What is verified GOOD (re-checked live, not re-litigated):**
- **No refuted-v2 claim reappears.** `build_step_time` is x0.9-free (the legacy `(cost * speed_x1000 * 9 / 10000)` at `production_tech.rs` `let base_value =` is named DRIFT, NOT reused); `category_for_object` delegates to `production_category_for_object` (Aircraft→Aircraft / Infantry→Infantry, the inverse never returns); `set_rate` takes the TOTAL and owns `/PRODUCTION_STEPS + clamp`; `SpecialItem` 0/-1/Item folds the three states distinctly; the purifier base stays the OrePurifier building COUNT (`refresh_economy_shadow` `house.economy.purifier_count = purifiers.get(id)…` untouched); Ship stays collapsed (D2).
- **D1/D2 honored, CONCERN-2 revision stated.** Both `next_insertion_seq` + `seq_carry` FIELDS removed; the swap of `registry_next_insertion_seq_is_serialized_and_hashed` → `factory_insertion_seq_equals_front_enqueue_order` is in §D #5; design §3.5 + plan T2-note state the STUDY §6.4 / P5b-seam revision explicitly.
- **`active_producer_by_owner` KEPT hashed (§3.4 override).** Verified-live: written by `find_spawn_selection_for_owner_with_type` (`active_producer_by_owner.entry(owner_id)…`), read by the placement focus path, hashed in `hash_production` (`for (owner, categories) in &self.production.active_producer_by_owner`); NOT retired by P5b. Dropping `legacy_active_producer_removed_from_hash` is correct.
- **Hash field set has no double-hash of any single authority.** The §3.3 audit (registry `object.type_id`/`insertion_seq` vs front `type_id`/`enqueue_order`) is the only value-redundancy, deterministic + one-way-derived. `house.credits` hashed once; `economy.credits` correctly NOT hashed.
- **Determinism preserved across the bump.** `iter_insertion_ordered` rides strictly-monotonic `enqueue_order` (no ties → total order), BTreeMap/VecDeque only, integer/i128 only, no RNG in the charge path. The §D #6 determinism guard is the right near-term proxy; P5c is the acceptance gate.

The A1–A42 anchor map is overwhelmingly accurate against the live tree (derives, fields, signatures, hash-fold lines, the Phase-7 head/tail order, the mirror line, the retirement targets, the A11/A30 self-corrections). The defects are in **two debug-assert interactions**, the **hash-symmetry of the frames-timer pair**, and the **test bodies** the plan ships uncompilable while flagging them only as "confirm at impl time."

---

## Ranked findings

### BLOCKER-1 — deleting the credits-mirror line (T7) makes `debug_assert_economy_shadow` panic for every factory-less house
*(RV1 — confirmed live; RV2/RV3/RV4 missed it)*

**Failure mode.** `debug_assert_economy_shadow` (world/mod.rs, anchor `pub(crate) fn debug_assert_economy_shadow(&self) {`) asserts `debug_assert_eq!(house.economy.credits, house.credits, …)` for **every** house in `&self.houses`, and it runs every tick via `debug_assert_production_shadow` (anchor `self.debug_assert_economy_shadow();`). The ONLY thing that keeps `economy.credits == house.credits` for ALL houses today is the mirror line `house.economy.credits = house.credits;` in `refresh_economy_shadow` (anchor that exact line). T7 deletes that mirror line (correct per §3.3 — `economy.credits` is demoted to a per-sweep shim). But `step_all` only writes `house.economy.credits` for owners with an **active factory** (it `continue`s on `houses.get_mut == None` and on `f.object.is_none()`). Any house whose `credits` moves through a non-factory path — a deposit, a refund, a `HouseState::new(owner, .., 1_000_000, ..)` test seed with no queue — now has `economy.credits != credits`, and the assert **panics in every debug build and every `cargo test` run**. The plan never mentions `debug_assert_economy_shadow` in T7/T11.

**Single fix site.** **T7** (or **T11**, with the other P5a-assert repurpose): retire `debug_assert_economy_shadow` from the `debug_assert_production_shadow` chain (anchor `self.debug_assert_economy_shadow();`), OR rewrite its body — it asserts a now-INVALID contract (`economy.credits` is no longer required to track `house.credits`; that is the whole point of the demotion, §3.3). This is a hard debug-build panic, not cosmetic.

Player-visibility: none directly (debug-only) — but it crashes every test/debug run, so it blocks the slice from green.

### BLOCKER-2 — `remaining_base_frames` removed from the hash while still serialized AND still a live sidebar reader: a hash hole + a frozen-ETA DRIFT
*(RV3 — confirmed live; the symmetry argument is decisive)*

**Failure mode.** The plan REMOVES `item.remaining_base_frames` from `hash_production` (§3.2, T5) on the premise it is "a dead, no-longer-advanced field." But `remaining_base_frames` is read by the **same** sidebar `QueueItemView` builder the plan uses to JUSTIFY keeping `total_base_frames` hashed (U-QFRAMES / A36). Verified live, both feed one view, symmetric:
```
production_queue.rs:701   q.remaining_base_frames,   // -> effective_time_to_build_frames_for_type -> the "time remaining" ETA
production_queue.rs:708   q.total_base_frames.max(1),// -> the same fn -> the total build time
production_queue.rs:713   let frames = q.remaining_base_frames;   // the no-rules fallback ETA
```
The plan is internally inconsistent: it keeps `total_base_frames` hashed citing this exact reader (A36) and drops `remaining_base_frames` though it has the SAME live reader. Two real consequences under the burden-of-proof DRIFT default:
1. **Hash hole.** `BuildQueueItem` still derives `Serialize` and `remaining_base_frames` is still a mutable serialized field; dropping it from the hash while keeping it serialized+reachable means a divergence in `remaining_base_frames` between two clients is NOT caught by `state_hash()` — a desync the lockstep contract must catch.
2. **Frozen-ETA DRIFT (player-visible).** Once T10 retires `advance_queue_item` (the only writer that decrements `remaining_base_frames`), the field freezes at its enqueue value (`= total_base_frames`, production_queue.rs:228), so the sidebar `remaining_ms` (line 701) becomes a constant — the build bar drains but the "time remaining" never moves. Frequency: every build, every match.

**Single fix site.** **T5 / §3.2.** Pick one and state it: **(a)** KEEP `remaining_base_frames` hashed too (symmetric with `total_base_frames`) and re-scope the "frames timer retired" claim to the PROGRESS/CHARGE role only, NOT the field — but then you must ALSO re-point the sidebar `remaining_ms` (production_queue.rs:701) at the registry `Factory.balance`/`progress` so the ETA tracks the authoritative build (new sidebar wiring the plan does not scope); OR **(b)** prove `remaining_base_frames` is frozen-and-provably-equal-to-`total_base_frames` post-flip AND re-point the sidebar reader, then a `legacy_remaining_base_frames_removed_from_hash` test is honest. Today §D #4 (`legacy_progress_carry_removed_from_hash`) CODIFIES the hole for both fields. Note: `progress_carry` IS genuinely dead (no reader — grep confirms only the queue-builder + hash touch it), so removing `progress_carry` from the hash is fine; the defect is specifically `remaining_base_frames`.

### BLOCKER-3 — `empty_rules()` has no object types → cost-0 GRIZZLY makes five charge/cancel/stall/enqueue tests inert or failing
*(RV1 C1 + RV2 B1 + RV3 B2 + RV4 C2 — all four reviewers, confirmed live)*

**Failure mode.** `empty_rules()` is `RuleSet::from_ini(&IniFile::from_str(""))` (production_shadow_tests.rs, anchor `fn empty_rules() -> RuleSet {`) — an empty INI with NO `GRIZZLY` type. So `sim.object_type("GRIZZLY", &rules)` → `None` → `full_cost = 0`, and `enqueue_by_type`'s gate `if obj.cost <= 0 || owner_credits < obj.cost { return false; }` (production_queue.rs:214, after `let Some(obj) = rules.object(type_id) else { return false; }` at :206) returns FALSE for cost 0. Per test (all ship literal `empty_rules()` bodies that assert against a real cost):
- **#16 `no_upfront_charge_at_enqueue`** — `enqueue_by_type(.., "GRIZZLY")` returns **false** (no type) → `assert!(ok, …)` **FAILS**.
- **#11 `single_wallet_charged_once_no_double_debit`** — `full_cost = 0`; `assert_eq!(debited, full_cost)` and `spent_credits == full_cost` are `0 == 0`, **vacuous** (proves nothing about the wallet).
- **#12 `cancel_one_partial_refund_to_house_credits`** — `assert!(refunded > 0 && refunded < full_cost)` is `0 > 0` → **FAILS**.
- **#17 `stall_on_no_funds_holds`** — cost 0 → first charge is 0, never `< available`, `on_hold` never sets; the stall path is never exercised (the test note admits this).
- **#6 `factory_flip_determinism_over_scripted_commands`** — the cancel/charge command stream is inert (no real costs move) → the run is a weak guard.

**Single fix site.** **T6/T8 + §D** test bodies. Add a shared costed-vehicle fixture mirroring the in-tree pattern at `factory.rs` (`category_for_object_matches_rtti_table`, INI `[VehicleTypes]\n0=GRIZZLY\n[GRIZZLY]\nCost=700`) and route #6/#11/#12/#16/#17 through it; recompute the expected numbers. The plan flags this inline ("confirm the empty-rules cost or use a fixture") but ships every literal body on `empty_rules()` — they must be CHANGED, not annotated.

### BLOCKER-4 — T1 deletes `remaining_balance_after` but `remaining_balance_ladder_matches_stepper` still calls it → won't compile
*(RV1 B1 — confirmed live)*

**Failure mode.** T1 (and §6.1 retirement table, A12) deletes `remaining_balance_after` with the rebuild family. But the T1 note says *"`remaining_balance_ladder_matches_stepper` … exercise the stepper directly and stay; only the rebuild-named ones go."* That test is NOT rebuild-named, yet it hard-depends on the deleted fn (factory.rs `mod tests`, anchor `remaining_balance_after(cost, k),` and `assert_eq!(remaining_balance_after(cost, PRODUCTION_STEPS), 0);`). Deleting `remaining_balance_after` while keeping the test = `error[E0425]: cannot find function remaining_balance_after` → the crate test build fails. The plan is self-contradictory as written. Confirmed: the only non-test caller is `rebuild_shadow_inner` (factory.rs, anchor `let balance = remaining_balance_after(full_cost, progress);`), which T1 replaces; the only remaining callers are the two test lines.

**Single fix site.** **T1.** Pick one explicitly: **(a)** KEEP `remaining_balance_after` as a `#[cfg(test)]` helper (it has no non-test caller once `rebuild_shadow_inner` is gone), so `remaining_balance_ladder_matches_stepper` + `cost25_ladder_sums_to_exactly_25` still compile; OR **(b)** ALSO delete `remaining_balance_ladder_matches_stepper`. The plan's current text ("the stepper-direct ones stay") implies (a) but the retirement table deletes the fn — resolve the contradiction.

---

### CONCERN-1 — the delivery completion→spawn trigger is not re-pointed at the registry; T10 risks a total production stall
*(RV2 C3 + RV3 C1 — the highest-risk gap after the BLOCKERs)*

The live `tick_production_with_overlay_registry` drives delivery off `front.remaining_base_frames` reaching 0 → `front.state = Done` → the spawn/placement geometry (`find_spawn_selection_*`, `spawn_object`, helipad reserve, `place_ready_building`), each gated on that completion. Once the frames timer is retired (T10/M3), nothing in that function knows the build completed — completion now lives in the registry (`Factory` `suspended` + `progress == PRODUCTION_STEPS`). T10 says "replace the completion→ready half with the C7 bind" but never states the NEW trigger. Without re-pointing it, deliveries never fire (units never spawn, buildings never reach `ready_by_owner`) — a total production stall, and B3 hides it (the cost-0 tests never complete a build). **Fix site: T10.** Specify explicitly: "delivery now triggers off the registry factory reaching `suspended` + `progress >= PRODUCTION_STEPS`, replacing the `front.state == Done` / `remaining_base_frames == 0` gate; the existing spawn/placement geometry is driven from that registry state." Also resolve the two completion-path full-refunds at production_queue.rs:528/554 (`+= obj.cost.max(0)` on a spawn-block) — post-flip the build already paid full cost via `step_all`, so a full refund there is correct ONLY if the registry factory is also cleared/reset; T10 must define that reconciliation or the credit and the registry diverge.

### CONCERN-2 — snapshot round-trip test #3 calls a non-existent API and the wrong `rebuild_caches_after_load` arity
*(RV1 C2 + RV2 B2 + RV3 (impl) + RV4 C3 — confirmed live)*

The live snapshot API is `GameSnapshot::save(&sim, map_hash, rules_hash, name, ts) -> Vec<u8>` and `GameSnapshot::load(&bytes) -> Result<GameSnapshot, _>` consumed as `GameSnapshot::load(&bytes).expect("…").sim` (snapshot.rs, anchors `pub fn save(` / `pub fn load(` / the in-tree `snapshot.sim`). Test #3 `snapshot_roundtrip_factory_registry` calls `sim.to_snapshot_bytes()` and `Simulation::from_snapshot_bytes(&blob)` — **neither exists** → won't compile. Second layer: the test calls `loaded.rebuild_caches_after_load()` with **zero args**, but the live signature takes **7** (world/mod.rs, anchor `pub fn rebuild_caches_after_load(` — `resolved_terrain, terrain_speed_config, bridge_explosions, metallic_debris, bridge_anim_sounds, effect_frame_counts, terrain_costs`). **Fix site: T5 / §D #3.** Rewrite on `GameSnapshot::save`/`::load(..).sim`; DROP the `rebuild_caches_after_load()` call (the existing `snapshot_roundtrip_ignores_shadow` omits it — that is the precedent, and the round-trip stays honest without it).

### CONCERN-3 — `StepInputs` is never re-exported, but T7 + test #11 reference `crate::sim::production::StepInputs`
*(RV2 B3 + RV3 C2 — confirmed live)*

The re-export `pub use self::factory::{ build_step_time, category_for_object, BuildEligibility, BuildStepTimeInputs, … STEP_RATE_MIN };` (production/mod.rs) lists no `StepInputs`. T7's `crate::sim::production::StepInputs { sim: &*self }` and test #11's `use crate::sim::production::{…, StepInputs}` won't resolve. The Files-touched row for `production/mod.rs` says only "re-export any new public surface (`step_all` if pub)" — but `step_all` is a method on the already-re-exported `FactoryRegistry` (needs nothing), while `StepInputs` (a new pub struct) is the type that actually needs adding. **Fix site: T6** (+ the mod.rs row): add `StepInputs` to the `pub use self::factory::{…}` list.

### CONCERN-4 — the `factory_count` adapter passes `ProductionCategory` but `matching_factory_count_for_owner` takes `ObjectCategory`; won't compile, and the design's "registry-key count" is a wrong-rate trap
*(RV2 C1 + RV3 B3 + RV4 C5 — confirmed live)*

`matching_factory_count_for_owner(entities, rules, owner, category: ObjectCategory, interner) -> u32` takes `ObjectCategory` (rules enum), NOT `ProductionCategory` (the registry key) — verified live (production_tech.rs, anchor `category: ObjectCategory,` at the fn). The plan's `StepInputs::build_step_time_inputs` adapter receives `category: ProductionCategory` and the stub `factory_count_for(.., category, obj)` passes it; the correct arg is `obj.category` (the `ObjectCategory` on `ObjectType`). They are distinct enums — won't compile as wired. Separately, the **design** §4.3/§6.1/§12 still say "factory count from the registry key count, retiring `matching_factory_count_for_owner`" — but the registry collapses physical factories to ONE key per (owner, category), so the key count is always 1; following the design there silently breaks the MultipleFactory `(n-1)` speedup (player-visible: a 2nd War Factory gives no speed bonus; every base with ≥2 same-category factories). The plan's U-FACTORYCOUNT (E-section) correctly catches this and KEEPS the rescan — but the design body is uncorrected. **Fix site: T6.** (a) Pin `obj.category` (ObjectCategory) as the count arg; (b) note the plan's U-FACTORYCOUNT OVERRIDES the design's retirement-table row (keep `matching_factory_count_for_owner`, widen its visibility to `pub(in crate::sim::production)` — it is currently a private `fn`; also widen `owner_power_percentage_ppm`, likewise private). The executor follows the PLAN, not the design's §4.3.

### CONCERN-5 — the `StepInputs { sim: &*self }` borrow conflict: the canonical signature shown can't be the one wired
*(RV1 C4 — confirmed live; the plan flags it but presents the conflicting signature primary)*

T7's literal `let inputs = StepInputs { sim: &*self }; registry.step_all(&mut self.houses, rules, &inputs);` borrows `&*self` immutably and `&mut self.houses` mutably at once → `error[E0502]`. The plan's own borrow-confirm note prescribes the owned-prepass (gather `Vec<BuildStepTimeInputs>` from `&self` first, drop the borrow, then the `&mut self.houses` loop), but it presents `step_all(&mut self, houses, rules, sim_inputs: &StepInputs)` as the primary signature across T6/§4.2 and the prepass as an aside. **Fix site: T6 signature + T7 call.** Lock the prepass shape as the REAL signature (e.g. `step_all(&mut self, houses, prepared: &BTreeMap<(InternedId,ProductionCategory), BuildStepTimeInputs>)`), so the implementer is not left reconciling two conflicting signatures. The observable behavior (rate→cadence→charge in insertion_seq order) is unchanged.

---

### NIT-1 — `production_delivery_probe_is_dormant`'s docstring becomes a lie after T10 *(RV4 C1)*
Live: `production_delivery_probe_is_dormant` (production_shadow_tests.rs, anchor `fn production_delivery_probe_is_dormant()`) calls `sim.factory_delivery_probe()` DIRECTLY (a clone-based probe), NOT via `advance_tick`, and its docstring claims "no advance_tick path invokes start_next_queued." T10 binds `start_next_queued` at the delivery commit. The test itself likely still passes (the probe is clone-only and the live front is mutated by the delivery bind only on a real delivery, which this test doesn't drive) — but the docstring "no advance_tick path invokes start_next_queued" is then false. §D.7 lists `factory_flip_prep_does_not_change_state_hash` (which also calls the probe) for inversion but NOT this one. **Fix site: §D.7** — add `production_delivery_probe_is_dormant` to the reconciliation list: re-verify it passes (probe is clone-only) and invert its docstring (state the probe-vs-live-delivery distinction).

### NIT-2 — `factory_flip_prep_does_not_change_state_hash` must be inverted, and it is NOT cost-0-blocked *(RV4 C4-adjacent)*
Live: `factory_flip_prep_does_not_change_state_hash` (production_shadow_tests.rs:611) drives the producer with a hardcoded `cost: 700` `BuildStepTimeInputs`, so it does NOT depend on GRIZZLY's cost (immune to B3). But after T4 un-skips `factory_shadow`, `refresh_production_shadow` populating the now-hashed registry MOVES the hash relative to baseline → this test FAILS by design. It is exactly a §D.7 inversion. **Fix site: §D.7** — the plan lists this one (good); just confirm its replacement (#1 `production_authoritative_hash_includes_factory_fields`) covers the inverse, and that `economy_shadow_does_not_change_state_hash` (production_shadow_tests.rs:94) ALSO survives (it inserts no purifier + touches no spend, so the economy stats stay 0 and the recompute writes identical zeros — likely passes, but state it so green-landing has no surprise).

### NIT-3 — `economy.purifier_count` (and statistics) are `i32`, not `u32` *(RV1 N2 + RV2 N1 — confirmed live)*
`economy.rs` (anchor `pub purifier_count: i32,`, `pub spent_credits: i32,`, `pub harvested_credits: i32,`). The hash-fold `house.economy.purifier_count.hash(hasher)` and the test `e.purifier_count += 1` are type-correct with `i32` (consistent with `house.credits: i32`); just don't write a `u32` cast. `purifier_count` is recomputed-then-overwritten every `refresh_economy_shadow` (not a true accumulator) — hashing it is still correct (it round-trips as derived-then-overwritten). No action.

### NIT-4 — A40 re-export anchor is a paraphrase *(RV1 N1)*
A40 quotes the `pub use self::factory::{ … }` block as one line; it is multi-line (production/mod.rs:48-52). No edit hinges on it (CONCERN-3 is the real re-export action — add `StepInputs`). Note only.

---

## Required-revisions punch-list (by task)

| Task | Severity | Action |
|---|---|---|
| **T7** (or T11) | **BLOCKER-1** | Retire or rewrite `debug_assert_economy_shadow` (anchor `self.debug_assert_economy_shadow();` in `debug_assert_production_shadow`): after the mirror line is deleted, `economy.credits == house.credits` is no longer an invariant → it panics for every factory-less house. Must be addressed in the same hunk as the mirror-line delete. |
| **T5 / §3.2** | **BLOCKER-2** | Resolve the `remaining_base_frames` asymmetry: either KEEP it hashed (symmetric with `total_base_frames`, A36) AND re-point the sidebar `remaining_ms` (production_queue.rs:701) at the registry, OR prove-it-frozen + re-point the reader. Do NOT drop it from the hash while it stays serialized + read. (`progress_carry` removal is fine — it has no live reader.) |
| **T6/T8 + §D** | **BLOCKER-3** | Add a shared costed-vehicle rules fixture (e.g. `[VehicleTypes]\n0=GRIZZLY\n[GRIZZLY]\nCost=700`) and route tests #6/#11/#12/#16/#17 through it instead of `empty_rules()`; recompute expected debit/refund/stall values. |
| **T1** | **BLOCKER-4** | Either keep `remaining_balance_after` as a `#[cfg(test)]` helper (so `remaining_balance_ladder_matches_stepper` + `cost25_ladder_sums_to_exactly_25` compile) OR also delete `remaining_balance_ladder_matches_stepper`. Resolve the self-contradiction. |
| **T10** | CONCERN-1 | Specify the new delivery trigger: completion fires off the registry factory (`suspended` + `progress >= PRODUCTION_STEPS`), replacing the `front.state == Done` gate, and DEFINE the spawn-block path (production_queue.rs:528/554 full-refund) so it clears the registry factory — else credit and registry diverge. Re-point the existing spawn/placement geometry from registry state. |
| **T5 / §D #3** | CONCERN-2 | Rewrite #3 on `GameSnapshot::save(&sim, 0,0,"m",0)` / `GameSnapshot::load(&bytes).expect("…").sim`; DROP `rebuild_caches_after_load()` (7-arg; the existing roundtrip test omits it). |
| **T6 + mod.rs row** | CONCERN-3 | Add `StepInputs` to the `pub use self::factory::{…}` re-export. |
| **T6** | CONCERN-4 | Pass `obj.category` (ObjectCategory) to `matching_factory_count_for_owner`; widen `matching_factory_count_for_owner` + `owner_power_percentage_ppm` to `pub(in crate::sim::production)`; follow the plan's U-FACTORYCOUNT (keep the rescan), NOT the design's §4.3 "registry-key count." |
| **T6 sig + T7 call** | CONCERN-5 | Lock the owned-prepass `step_all` signature as primary (gather `BuildStepTimeInputs` from `&self` first, then the `&mut self.houses` loop); do not present `StepInputs { sim: &*self }` as the wired form. |
| **§D.7** | NIT-1/NIT-2 | Add `production_delivery_probe_is_dormant` (re-verify + invert docstring) and confirm `economy_shadow_does_not_change_state_hash` survives (stats stay 0). |
| **T6 hash-fold** | NIT-3 | Hash the statistics as `i32` (no `u32` cast). |
| **A40** | NIT-4 | Multi-line anchor; cosmetic. |

---

## Dropped findings (false positives / de-escalated — re-verified against the tree this run)

- **"`active_producer_by_owner` should be removed from the hash" (literal scope-B / the `legacy_active_producer_removed_from_hash` test).** DROPPED as a defect — the plan correctly does NOT do this. Verified live: the field is written by `find_spawn_selection_for_owner_with_type`, read by the placement focus path, hashed in `hash_production`, and NOT retired by P5b. KEEPING it hashed and DROPPING the scope-F test is the correct §3.4 override; all four reviewers concur.
- **"`insertion_seq` is double-hashed with `front.enqueue_order`" (RV2 N2 / RV3 C4).** DROPPED. The §3.3 audit is correct: they are equal-by-construction but distinct ROLES (per-factory sweep key vs per-item temporal stamp), one-way-derived, deterministic, hash-safe; KEEPING `insertion_seq` as a regression guard is fine. RV3's seq-0/default-overlap sub-concern is also benign: `next_enqueue_order` is a single GLOBAL counter (not per-owner), so two owners' first builds get distinct `enqueue_order` (0 and 1) → no tie.
- **"`saturating_add` on `next_enqueue_order` admits a u64-ceiling tie" (RV3 C5).** DROPPED to a documented non-reachable boundary (2^64 enqueues). Surface-only; no action.
- **"A30 says `debug_assert_factory_step_matches_legacy` takes no rules param" (RV3 N3/C3a).** Clarified, not a plan defect: A30's "takes NO rules" refers to `debug_assert_production_shadow(&self)` (correct — verified `pub(crate) fn debug_assert_production_shadow(&self)`); `debug_assert_factory_step_matches_legacy(&self, rules: Option<&RuleSet>)` DOES take `rules: Option<&RuleSet>`, and the CALLER passes `None`. T11 edits the call/fn either way; just keep the `#[cfg(debug_assertions)]` gate on the repurposed assert (it is gated live).

---

## Cross-cutting confirmations (not defects — recorded so they are not re-litigated)

- **Parity (PASS — no refuted-v2 reintroduction).** x0.9-free producer; Aircraft/Infantry binding (no inverse); `set_rate` takes the TOTAL; `SpecialItem` 0/-1/Item distinct; purifier = building COUNT; Ship collapsed (D2, pinned by `category_for_object_naval_collapses_to_vehicle_documented`). `advance_one_step`/`cancel_one`/`set_rate`/`start_next_queued` BODIES unchanged — the flip changes WHO is passed (the real wallet shim), not the algorithm. C3/C4/C5/C8/C12/C15 charge mechanics faithful.
- **Hash-correctness (PASS except B2).** The field set is complete and double-hash-free per §3.3 EXCEPT the `remaining_base_frames` hole (B2). `house.credits` hashed once; `economy.credits` correctly NOT hashed; `active_producer_by_owner` KEPT (§3.4); `next_insertion_seq`/`seq_carry` dropped (D1, safe — zero readers outside the replaced `rebuild_shadow_inner`).
- **Progress-persist (PASS).** The reconcile PERSIST/SEED arm replaces the `std::mem::take` + `rebuild_shadow` clobber; the `(type_id, enqueue_order)` identity test is correct (strictly-monotonic `enqueue_order` → SEED re-arms on a new front). M1 isolates it hash-neutral before the hash move.
- **Economy/credits authority (UNAMBIGUOUS).** One wallet (`house.credits`), charged once by `step_all`; `economy.credits` demoted to a per-sweep shim loaded from / stored to `house.credits`; the mirror line deleted. The ONLY snag is BLOCKER-1 (the debug assert that policed the now-deleted mirror).
- **Legacy charge/refund/fabricate retirement (COMPLETE in scope, ordering safe).** Upfront `-=` (production_queue.rs:218), `.rev()`+full-refund cancel (783/837/876), frames timer, and the `credits_entry_for_owner` auto-create-house hazard (the `HouseState::new(key, .., is_human=true ..)` fabrication, production_queue.rs:74) are all enumerated with the safe-order invariant (real `step_all` charge live T6 BEFORE the upfront `-=` removed T8). CONCERN-1 adds the two completion-path refunds (production_queue.rs:528/554) that T10 must reconcile with the registry.
- **Determinism (PASS).** BTreeMap/VecDeque iteration only; integer/i128; no RNG in the charge path; the sweep rides strictly-monotonic `enqueue_order` (no ties). P5b intentionally breaks the no-hash contract (that IS the flip) while preserving lockstep; §D #6 is the near-term guard, P5c the acceptance gate.
- **Codebase (sound).** A1–A42 spot-verified live (derives, fields, signatures, hash lines, Phase-7 head 2559 < `run_late_region` < tail `refresh_production_shadow` < `state_hash`, the A11/A30 self-corrections). Drift to watch: world/mod.rs is co-edited — anchor every edit on quoted TEXT and hunk-stage only the four P5b hunks.
- **Buildability.** Aside from B1–B4 + C2/C3 (test/adapter compile surfaces), the structure is compile-plausible. The largest unverified compile surface is the `StepInputs` adapter body (CONCERN-3/4/5) — the implementer must wire it per the punch-list before it checks.
- **Scope (clean).** No P6/P7 creep (`purifier_count`/`harvested_credits` PLACED + hashed, WIRED later); no Ship (D2); the P5c (acceptance gate) and P5d (queues_by_owner retirement) seams left clean. CONCERN-2 (STUDY §6.4 revision) from P5a is stated in-doc.

---

*End of P5b plan review. The D-PARITY-MIN authority flip is architecturally ready — one wallet, the queue-of-record stays the hashed temporal source, the live `active_producer_by_owner` is correctly KEPT, no refuted-v2 claim returns, D1/D2 honored, the §3.3 double-hash audit airtight, determinism preserved. But the plan as literally written is **NOT-READY**: it panics `debug_assert_economy_shadow` after deleting the mirror (B1), ships a `remaining_base_frames` hash hole + frozen-ETA DRIFT (B2), runs five economy/charge tests through a cost-0 `empty_rules()` fixture (B3), and deletes `remaining_balance_after` while keeping a test that calls it (B4) — and CONCERN-1 (the un-re-pointed delivery trigger) risks a silent total production stall that B3 would mask. Fix B1–B4, specify the C1 delivery trigger, and land C2–C5 (the test/adapter compile substitutions) at their named tasks; none touches the architecture.*
