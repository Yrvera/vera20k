<!--
Provenance: consolidated review of
  docs/plans/2026-06-04-factory-house-substrate-p1p2-plan.md
  (design: docs/plans/2026-06-04-factory-house-substrate-p1p2-design.md),
reconciling 4 reviewer reports (RV1 codebase, RV2 parity, RV3 fit-determinism,
RV4 consistency) against the v2-verified study
  docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md.
All load-bearing claims spot-checked live against the current tree this session.
-->

# Factory/House Substrate P1+P2 Plan — Consolidated Review

| | |
|---|---|
| **Plan reviewed** | `docs/plans/2026-06-04-factory-house-substrate-p1p2-plan.md` |
| **Design reviewed** | `docs/plans/2026-06-04-factory-house-substrate-p1p2-design.md` |
| **Reviewers reconciled** | RV1 codebase, RV2 parity, RV3 fit-determinism, RV4 consistency |
| **Spot-checks this session** | `world/mod.rs:1747-2434`, `production_types.rs:135-151`, `miner_system.rs:1455-1471`, `house_state.rs:15-77`, `techno_ai.rs:100-223`, `snapshot.rs:24,92,115,321-325`, `rules/ruleset.rs:1314-1315` (no `Default`) |
| **Scope** | P1 (`Economy` shadow) + P2 (`Factory`/`FactoryRegistry` shadow) ONLY — additive `#[serde(skip)]`, zero `state_hash` change, no `SNAPSHOT_VERSION` bump |

## Verdict: **READY-WITH-FIXES**

4 BLOCKERs, all mechanical and fixable in-place; no architectural rework. The design's
winner (D2 substrate-fit, FIT option (a)), the no-hash discipline, BTreeMap/InternedId
determinism, and the §4.3 auto-create-hazard guard are all sound and verified. None of the
BLOCKERs touch the architecture — they are (1) a wrong borrow-scope claim that makes the wiring
not compile, (2) two non-existent constructor calls repeated across ~17 test bodies, (3) a
self-comparing tautology assertion that defeats the FIT-(a) proof, and (4) a dropped
anti-REFUTED-claim acceptance test. Fix all four and the plan is executable.

---

## Reconciliation notes (de-dup + dropped false positives)

- **`RuleSet::default()` does not exist** — raised by RV1 (CONCERN-1), RV2 (implicitly via renamed tests), and RV4 (BLOCKER-2). **Merged into BLOCKER-2.** Verified: `ruleset.rs:1314` is `#[derive(Debug)]` only; no `impl Default`, no `fn default()` in `src/rules`. RV4 correctly counts it across BOTH P1-T4 and P2-T6 (~17 call sites), where RV1 saw it as one CONCERN — RV4's framing (compile error, repeated) is the accurate one, so it is ranked BLOCKER.
- **Snapshot round-trip placeholder API** — raised by RV1 (CONCERN-2) and RV4 (NIT-1). **Merged into CONCERN-3.** Verified the real API: `GameSnapshot::save(&sim, 0, 0, "test_map", 0)` then `GameSnapshot::load(&bytes).expect(...).sim` (`snapshot.rs:321-325`); `serialize_for_test`/`deserialize_for_test` do not exist. The plan already flags it (E4) but the test body must bind `.sim` and the assertions become `restored.houses[&owner].economy` / `restored.production.factory_shadow.is_empty()`.
- **Line-citation drift (±2)** — RV1 NIT-1, RV3 N1, RV4 (clean). **Mostly dropped as cosmetic, with one correction folded into BLOCKER-1:** the tail anchors the plan cites as `2426/2432/2433` are actually `2427/2433/2434` in the current tree, and — more importantly — line `2402` is NOT where an unwrapped `&RuleSet` lives (it is inside the `Some(rules)` block). The drift itself is harmless; the *scope* claim built on it is not (BLOCKER-1).
- **`step_all` vs `iter_insertion_ordered` naming** — RV3 C3. **Kept as CONCERN-4** (design/plan surface mismatch), confirmed the plan's `iter_insertion_ordered` is the better minimal P2 surface and supersedes the design's `step_all`.
- **DROPPED false positive:** none of the reviewers raised a claim I could disprove outright. RV3's N2 (`mem::take` soundness) and RV4's borrow-check / derive-satisfaction / type-flow / task-ordering checks were independently consistent with the tree and are recorded as **confirmed-clean** below, not as findings.
- **Confirmed-clean (no action):** `HouseState` derives `Default` + 6-arg `new()` (`house_state.rs:17,51`); `hash_houses`/`hash_production` reference neither new field (no-hash contract holds); `BuildQueueItem` has no `cost` field (the E1 fork is correctly grounded); `ProductionCategory` derives `Ord` (registry key OK); `SpecialItem` 3-state guard preserved; no `storage_capacity` / `IncomeMult` on `Economy`; no ×0.9 anywhere; `SNAPSHOT_VERSION == 17` pin correct; `count_purifiers_for_owner` reused (not duplicated).

---

## Ranked findings

### BLOCKER-1 — `rules` at the wiring point is `Option<&RuleSet>`, not `&RuleSet`; A11 is factually wrong and the P2-T5 call will not compile
*(RV1 BLOCKER-1; player-visibility: none directly — but it blocks the whole slice from building)*

- **Evidence (verified this session):** `advance_tick` is declared `rules: Option<&RuleSet>` (`world/mod.rs:1750`). The unwrapped `&RuleSet` exists ONLY inside `if let Some(rules) = rules {` which opens at `world/mod.rs:1994` and **closes at `2405`** — *before* the LATE region and before `refresh_mission_shadow()` (`2427`). At the insert point (between `refresh_mission_shadow` @2427 and `state_hash` @2434), the in-scope `rules` is the original `Option<&RuleSet>`. The plan's A11 cites line `2402` as proof an unwrapped `&RuleSet` is in scope at the tail — but `2402` (`if spawned_entities`) is *inside* the `Some(rules)` block, not at the tail. A11 is wrong.
- **Transitive impact:** `refresh_production_shadow(&mut self, rules: &RuleSet)` (P2-T5) and `refresh_economy_shadow(&mut self, rules: &RuleSet)` (P1-T3) both take `&RuleSet`; the call `self.refresh_production_shadow(rules)` at the tail passes an `Option<&RuleSet>` → type error.
- **Fix:** change both new method signatures to take `rules: Option<&crate::rules::RuleSet>`. In `refresh_economy_shadow`, when `None`, still mirror `house.economy.credits = house.credits` for every existing house but set `purifier_count = 0` (the count needs rules; `None` ⇒ 0, which matches the empty-rules behavior every P1 test already assumes). This preserves the documented "runs beside the mission shadow, after all authoritative systems, before the hash" placement (the alternative — wiring inside the `Some(rules)` block at 2405 — would move the shadow *before* `refresh_mission_shadow` and break the §6 hook claim, so it is rejected). Update A11 to state the truth: `rules` at the tail is `Option<&RuleSet>`; the shadow takes the option and treats `None` as zero-purifier.
- **Also fix:** correct the cited tail anchors `2426/2432/2433` → `2427/2433/2434` (cosmetic, but the plan should not carry wrong line numbers into the insert instructions).

### BLOCKER-2 — `Factory: Default` will not compile: `ProductionCategory` has no `Default`
*(RV4 BLOCKER-1; player-visibility: none — compile error)*

- **Evidence (verified):** `Factory` (plan §D P2-T1, line 475) derives `Default` and has field `category: ProductionCategory`. `ProductionCategory` (`production_types.rs:135`) derives `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize` — **no `Default`**, and there is no manual `impl`. A derived `Default` requires every field type to be `Default`, so `Factory::default()` fails to compile. It is consumed by P2-T6 tests `factory_default_progress_zero_no_object` (line 958) and `registry_iter_insertion_ordered_not_map_order` (`..Factory::default()`, lines 972-973).
- **Fix (pick one, plan must state which):** (a) add `Default` to `ProductionCategory`'s derive with `#[default] Building` on the first variant — hash/serde-neutral (changes no existing value, the enum is already `Serialize`), and lets `Factory` keep `..Factory::default()` struct-update; or (b) drop `Default` from `Factory` and construct it explicitly in the two tests. Option (a) is cleaner and is recommended. Update P2-T1 and the two affected P2-T6 tests accordingly.

### BLOCKER-3 — `crate::rules::RuleSet::default()` does not exist; ~17 test bodies will not compile
*(RV4 BLOCKER-2; RV1 CONCERN-1; RV2 implicit; player-visibility: none — compile error)*

- **Evidence (verified):** `RuleSet` is `#[derive(Debug)]` only (`ruleset.rs:1314`); no `Default` derive, no `impl Default`, no `fn default()`/`empty()` anywhere in `src/rules`. The constructor the codebase uses is `RuleSet::from_ini(&IniFile)` (e.g. `miner_tests.rs:73`). Every `let rules = crate::rules::RuleSet::default();` is a compile error — appearing in P1-T4 (3×) and P2-T6 (~6×, plus the helper-shaped reuses). The plan flags it once in a P1-T4 parenthetical (lines 391-394) but presents all bodies verbatim with the non-compiling call, and P2-T6 reuses it with no caveat.
- **Fix:** add one test helper in the chosen test module — `fn empty_rules() -> RuleSet { RuleSet::from_ini(&IniFile::from_str("[BuildingTypes]\n[VehicleTypes]\n[InfantryTypes]\n[AircraftTypes]\n")).expect("empty rules") }` — and replace every `RuleSet::default()` with `empty_rules()`. Since the tests insert zero (or non-purifier) Structure entities, `count_purifiers_for_owner` returns 0 under empty rules, so the `purifier_count == 0` / credits-mirror assertions hold. Confirm the exact `IniFile` constructor at impl time (mirror `miner_tests.rs`).

### BLOCKER-4 — P2-T4's FIT-(a) trace assertion is a tautology, and the trace is not actually driven from the Structure arm
*(RV3 B1; player-visibility: none in P1+P2 — but it voids the FIT-(a) proof the design commits to and sets the wrong trajectory for P3)*

- **Evidence (verified against the S1 pattern):** the real shadow pattern (`debug_assert_s1_shadow`, `techno_ai.rs:193-222`) walks live order, calls `unit_ai_shadow_step` per id, and asserts an *intrinsic* per-trace property (`dispatch_seq < process_seq`, mission is `Move`, `is_drive`). It never compares a filtered list to a copy of itself. The plan's `debug_assert_factory_shell_trace` (plan lines 816-857) instead builds `traced` by filtering `live_object_order_snapshot()` for live Structures, then builds `logic_structures` by filtering the **same** `live_object_order_snapshot()` for live Structures **the same way**, and does `debug_assert_eq!(traced, logic_structures, ...)`. That compares a list to a copy of itself — a tautology that passes regardless of order and proves nothing about LogicVector order. Separately, the design (§2, §6.1, L-FIT-1, lines 422-429) commits to option (a) = "fill the `EntityCategory::Structure => {}` arm with the read-only step"; the plan leaves the arm a literal no-op and re-walks live order in a standalone method, so the "order IS LogicVector order **by construction**" claim is unsupported — the construction never goes through the Structure arm.
- **Fix (pick one, make plan self-consistent with design L-FIT-1):**
  - (i) **Route through the Structure arm (matches the design and the S1 dispatch intent):** have the `EntityCategory::Structure` arm of `techno_ai_shell` record a `FactoryShellTrace` per visited Structure into a pass-scoped accumulator, exactly as S1 conceptually dispatches from the Unit arm. Then the trace order IS LogicVector order by construction, and the debug assert checks an *intrinsic* property (monotone `visit_seq`, and each traced id resolves to a live Structure) — not a self-comparison.
  - (ii) **Keep the debug-only standalone method (the literal S1 mechanism)** but drop the tautological `debug_assert_eq!(traced, logic_structures)`. Replace it with: assert `visit_seq` strictly increases (already present and meaningful), assert each `structure_id` resolves to a live, non-dying Structure, and — if a LogicVector-order claim is wanted — compare against an *independently sourced* order (e.g. the raw `live_object_order_snapshot()` BEFORE the Structure filter, asserting the traced ids are a subsequence), never against a re-derived copy of the same filter.
- Whichever is chosen, the plan's prose claim "trace order == LogicVector order by construction" must be made literally true by the code, and `factory_shadow_trace_order_matches_logic_vector` (P2-T6) must exercise a non-trivial order (it already inserts `[3,1,2]` in non-sorted live order — keep that, but ensure the assertion would actually FAIL if the trace were mis-ordered).

### CONCERN-1 — P1-T4 drops the study/design C14 purifier-count-is-building-count guard (the direct anti-REFUTED-claim test)
*(RV2 BLOCKER; ranked CONCERN here; player-visibility: none in P1+P2, but it is the regression guard for the one v2 correction the task brief names)*

- **Evidence (verified):** the task brief says verbatim "the Economy type must NOT model storage_capacity as a purifier base." The study's whole v2 correction (C14; ledger L-ECON-2, design line 512) is "purifier base = OrePurifier building COUNT, NOT silo StorageCapacity." The design's own P1-T4 list names `economy_purifier_count_is_building_count` (design line 547). The plan omits it. Its only purifier assertion is `purifier_count == 0` under empty rules (plan line 357) — which passes whether the implementation counts buildings, counts storage capacity, or always returns 0. It is not a guard against the REFUTED model.
- **Why CONCERN not BLOCKER:** the implementation correctly *reuses* `count_purifiers_for_owner` (`miner_system.rs:1460`, the building count), so the behavior is right; this is a missing acceptance test, not wrong code. But per the project's burden-of-proof rule a dropped anti-REFUTED-claim test must be restored, so this is the top must-fix among non-blockers.
- **Fix:** add a `world`-level test that inserts ≥1 (then 2) OrePurifier Structure entities for an owner with non-empty rules that mark the type `ore_purifier=true`, runs `refresh_economy_shadow`, and asserts `economy.purifier_count == <building count>` (1, then 2) — proving it tracks the building count and would fail if it ever modeled storage capacity. This needs a non-empty `RuleSet` (an OrePurifier type with the flag set), so coordinate with the BLOCKER-3 helper.

### CONCERN-2 — `refresh_economy_shadow` is O(houses × all-entities) every tick; the one new per-tick scale cost
*(RV3 C1; player-visibility: none functional — a frame-time cost at the 30-player/20k-unit target)*

- **Evidence (verified):** `count_purifiers_for_owner` is a full `sim.substrate.entities.values()` linear scan (`miner_system.rs:1461-1471`); the plan calls it once per house in `refresh_economy_shadow`. At 30 houses × 20k entities that is ~600k entity-visits/tick, every tick, to refresh a shadow nothing reads in P1+P2. Deterministic (BTreeMap iteration), so not a correctness bug — but it contradicts the "Scale is the parity exception / replace the O(N²) structure" directive and sits in the hot `advance_tick` tail.
- **Fix (pick one):** (a) single-pass — scan `substrate.entities` once, accumulating purifier counts into a `BTreeMap<InternedId, i32>` keyed by `e.owner`, then apply per house (one scan, no per-house re-scan, and the InternedId-keyed accumulation also removes CONCERN-5's string round-trip); or (b) since the count has no consumer in P1+P2, gate the purifier refresh behind `cfg!(debug_assertions)` (the asserts are its only exerciser). Recommend (a). Add as a 5th open question for the design-lead if (b) is preferred.

### CONCERN-3 — snapshot round-trip test uses a placeholder API; real API is `GameSnapshot::save/load(...).sim`
*(RV1 CONCERN-2 + RV4 NIT-1, merged; player-visibility: none — the test won't compile as written)*

- **Evidence (verified):** `serialize_for_test`/`deserialize_for_test` do not exist. Real round-trip (matching `snapshot.rs:321-325`): `let bytes = GameSnapshot::save(&sim, 0, 0, "test_map", 0); let restored = GameSnapshot::load(&bytes).expect("load").sim;`. The restored `Simulation` is the **`.sim` field** of the loaded snapshot.
- **Fix:** inline the real API in `snapshot_roundtrip_ignores_shadow` (P2-T6) — the plan already knows it (E4), so promote it from open-question to test body. Assertions become `restored.houses[&owner].economy == Economy::default()`, `restored.production.factory_shadow.is_empty()`, and `restored.state_hash() == hash_before`. (`factory_shadow` and `economy` must be `pub` — already specified.)

### CONCERN-4 — design/plan name the registry tick surface three ways; design's `step_all` is never used
*(RV3 C3; player-visibility: none — internal consistency)*

- **Evidence:** design §3.5 lists `step_all(&mut self, economies: &mut BTreeMap<InternedId, Economy>)` and a `rebuild_factory_shadow` free fn among the "first three needed"; the plan uses a `rebuild_shadow(&mut self, sim)` method and `iter_insertion_ordered()` (read-only `Vec<&Factory>`) instead, and never defines `step_all`. The plan's surface is the better minimal P2 choice (charges no economy, which the design itself warns `step_all` must not).
- **Fix:** add one line to the plan (P2-T1 or §E) noting `iter_insertion_ordered` supersedes the design's `step_all`/`rebuild_factory_shadow` naming, so an implementer following the design verbatim does not build the unused `step_all`. See design-doc corrections below.

### NIT-1 — `super::super::production::PRODUCTION_STEPS` path is fragile; use the absolute path
*(RV1 NIT-2)* — In `factory_shadow_progress_tracks_legacy_remaining` (plan line 1035) the relative `super::super::production::PRODUCTION_STEPS` depends on the still-undecided (E2) test-module home. **Fix:** use `crate::sim::production::PRODUCTION_STEPS` (the canonical re-export the plan itself adds).

### NIT-2 — P1-T1 `sim/mod.rs` placement instruction is internally contradictory
*(RV4 CONCERN-1; ranked NIT — module order does not affect compilation)* — The plan says place `pub mod economy;` "before `entity_store` / after `docking`," but `sim/mod.rs` is grouped by section, not alpha-sorted (`entity_store` is early, `docking` later), so both cannot hold. **Fix:** reword to "add `pub mod economy;` among the early `pub mod` group (order is by section, not alphabetical)."

### NIT-3 — `economy_spend_*` and C16 study tests renamed/dropped without an explicit deferral note
*(RV2 CONCERN ×2; ranked NIT — surfacing hygiene, no wrong code)* — The study's P1 `economy_spend_silo_drain_matches_engine` is descoped to a trivial `min(credits, amount)` cap (correct for P1 — no ore storage yet) but renamed `economy_spend_caps_at_balance_and_tracks_spent` with no note tying it to the study test; and the study's `economy_ore_deposit_has_no_credit_cap` (C16) is silently absent (defensible — no deposit path in P1). **Fix:** add a one-line comment on the spend test mapping it to the descoped study test, and one sentence in §E recording C16's no-cap test as P7-deferred (no deposit path exists in P1). Per the burden-of-proof rule a dropped study test must be a recorded decision, not a silent gap.

### NIT-4 — C5 "rate 0 ⇔ no object" sentinel is collapsed by the frames-based default; name it in E1
*(RV2 NIT)* — The frames-based default sets `step_rate_frames: 0` even with an active object, so `0` no longer means only "no object" (study C5 / L-FAC-5: `0` only when no object). No P2 test fails (asserts are monotone-only) and P5 owns the authoritative rate, so this is not a parity violation in pure shadow — but E1 should name C5/L-FAC-5 explicitly and note the sentinel is temporarily collapsed (restored at P5), OR pick the cost-based-with-rules E1 option which preserves it.

---

## Required revisions — punch-list (by task)

**P1-T3** (`refresh_economy_shadow`):
1. **[BLOCKER-1]** Change signature to `rules: Option<&crate::rules::RuleSet>`; on `None`, mirror credits and set `purifier_count = 0`.
2. **[CONCERN-2]** Rewrite the purifier scan as a single pass over `substrate.entities` accumulating into `BTreeMap<InternedId, i32>` keyed by `e.owner` (removes the per-house re-scan AND the per-tick `.to_string()` allocation).

**P1-T4** (P1 tests):
3. **[BLOCKER-3]** Replace every `RuleSet::default()` with an `empty_rules()` helper built via `RuleSet::from_ini`.
4. **[CONCERN-1]** ADD `economy_purifier_count_is_building_count`: insert 1 then 2 OrePurifier structures (rules marking the type `ore_purifier=true`), assert `purifier_count` equals the building count (1, then 2).
5. **[NIT-3]** Add a one-line comment on `economy_spend_caps_at_balance_and_tracks_spent` mapping it to the descoped study `economy_spend_silo_drain_matches_engine`.

**P2-T1** (`factory.rs` types + `production/mod.rs`):
6. **[BLOCKER-2]** Resolve `Factory: Default` — either add `#[derive(... Default)]` + `#[default] Building` to `ProductionCategory` (recommended), or drop `Default` from `Factory` and construct explicitly in tests.
7. **[CONCERN-4]** Add a note that `iter_insertion_ordered` supersedes the design's `step_all`/`rebuild_factory_shadow` naming.

**P2-T3** (`rebuild_shadow`):
8. **[NIT-4]** In E1 (or a code comment), name C5/L-FAC-5 and note the frames-based default temporarily collapses the "rate 0 ⇔ no object" sentinel (restored at P5) — or select the cost-based-with-rules E1 option.

**P2-T4** (Structure-arm trace):
9. **[BLOCKER-4]** Make the FIT-(a) proof real: either route the trace through the `EntityCategory::Structure` arm (matches design L-FIT-1) and assert an intrinsic per-trace property, OR keep the standalone debug method but DELETE the tautological `debug_assert_eq!(traced, logic_structures)` and replace it with monotone-`visit_seq` + each-id-is-a-live-Structure (and, if a LogicVector claim is wanted, a subsequence check against the unfiltered snapshot). Update the prose so "by construction" is literally true.

**P2-T5** (wiring):
10. **[BLOCKER-1]** `refresh_production_shadow` takes `rules: Option<&RuleSet>` and forwards it; the tail call `self.refresh_production_shadow(rules)` now type-checks. Correct the cited anchors `2426/2432/2433` → `2427/2433/2434`.

**P2-T6** (P2 tests):
11. **[BLOCKER-3]** Same `empty_rules()` substitution as P1-T4.
12. **[CONCERN-3]** Replace `serialize_for_test`/`deserialize_for_test` with `GameSnapshot::save(&sim, 0, 0, "test_map", 0)` → `GameSnapshot::load(&bytes).expect(...).sim`; assert `restored.houses[&owner].economy == Economy::default()`, `restored.production.factory_shadow.is_empty()`, `restored.state_hash() == hash_before`.
13. **[NIT-1]** Use `crate::sim::production::PRODUCTION_STEPS` (absolute), not `super::super::...`.

**P1-T1** (module decl):
14. **[NIT-2]** Reword the `sim/mod.rs` placement to "early `pub mod` group; order is by section, not alphabetical."

**§E open questions:**
15. **[CONCERN-2]** Add E5: single-pass vs debug-gated purifier scan.
16. **[NIT-3]** Record C16 `economy_ore_deposit_has_no_credit_cap` as P7-deferred (no deposit path in P1).

---

## Design-doc assumptions that also need correcting

- **§6 / §3.5 hook claim (drives BLOCKER-1):** the design states the shadow "takes `&Simulation` (or iterates `&mut self.houses`)" and runs at the tail with rules in hand. It must be amended to reflect that the only `rules` in scope at the tail is `Option<&RuleSet>` — the shadow API takes the option and treats `None` as zero purifiers. (The design's broader placement is correct; only the rules-availability assumption is wrong.)
- **§3.5 registry surface (CONCERN-4):** the design lists `step_all` and a `rebuild_factory_shadow` free fn as needed-in-P2. The implemented surface is `rebuild_shadow` (method) + `iter_insertion_ordered` (read-only). Update §3.5/§10 so the design and plan agree; `step_all` is a later-slice seam, not a P2 surface.
- **§2 / §6.1 / L-FIT-1 (BLOCKER-4):** the design's "fill the Structure arm with the read-only step" + "trace order is LogicVector order by construction" is only honored if the plan actually routes through the arm (fix option (i)) OR drops the self-comparison and asserts an intrinsic property (fix option (ii)). If the plan adopts (ii), note in L-FIT-1 that the proof is the S1-style intrinsic assertion, not a literal arm dispatch.
- **§9 ledger L-ECON-2 / §10 P1-T4 (CONCERN-1):** the design already names `economy_purifier_count_is_building_count`; the plan dropped it. No design change needed — the plan must be brought up to the design here.
- **Otherwise the design is sound:** D2 winner, FIT option (a) rationale, SpecialItem 3-state guard, no `storage_capacity`/`IncomeMult`/×0.9, `Defense` distinct key, and the no-hash discipline all verified consistent with the study and the tree.
