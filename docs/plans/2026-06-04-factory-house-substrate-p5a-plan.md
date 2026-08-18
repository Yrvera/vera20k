<!--
Provenance: authored 2026-06-04 from the APPROVED design
  docs/plans/2026-06-04-factory-house-substrate-p5-design.md
  (D2 substrate-fit winner 23/23/23, grafted with D3 structural-no-hash + D1 tiny-detail ledger;
   four P5a pieces: (1) the x0.9-FREE `build_step_time` producer feeding the already-shipped
   `set_rate(total)`, (2) the `category_for_object` routing delegate + surfaced Ship-collapse gap,
   (3) the C7 delivery seam IDENTIFIED + the dormant `start_next_queued` confirmed as the bind point,
   (4) the Lane-A temporal `insertion_seq` mint CORRECTION + the inversion-readiness shadow assert),
  grounded in the v2-verified study
  docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md (C1, C5, C7, C10, C11, C12, C15;
  §6.1/§6.3 ordering; §6.4 hash-set; §7 retire list; §8 P5/P8/P9).
House style mirrored from docs/plans/2026-06-04-factory-house-substrate-p4-plan.md.
Status: DRAFTED, not approved or executed. Review (/review-plan) before implementing.
Scope: P5a ONLY — the LAST hash-neutral prep slice before the authority flip. Lands every
  flip-enabling piece provable WITHOUT touching the hash: the pure producer, the routing delegate,
  the temporal mint correction, the C7 delivery seam (dormant), and the inversion-readiness assert.
  STRICTLY HASH-NEUTRAL: NO serde derive added; NO un-skip of economy/factory_shadow; NO
  world_hash.rs change; SNAPSHOT_VERSION STAYS 17; the legacy production_queue path stays
  AUTHORITATIVE. Proven by a `*_does_not_change_state_hash` acceptance test mirroring P3/P4.
  OUT OF SCOPE (seams only): the authority flip itself + serde derives + un-skip + hash fold +
  17->18 + the C1 ordering lock (ALL P5b); the P9 global parity/replay gate (P5c).
-->

# Factory/House Substrate — P5a Plan (pure producers + temporal mint + inversion-readiness assert, hash-neutral)

> Linear path: **P5a-T1 → P5a-T2 → P5a-T3 → P5a-T4 → P5a-T5 → P5a-T6 → P5a-T7**.
> Every task builds green (`cargo check -p vera20k`) before the next. The hash-neutrality test
> (`factory_flip_prep_does_not_change_state_hash`) + the version pin
> (`snapshot_version_is_17_in_shadow_phase`, snapshot.rs `assert_eq!(super::SNAPSHOT_VERSION, 17)`)
> are the contract gate: if either fails after a task, STOP — a serde derive crept in, a call site
> became authoritative, or the mint change leaked into a hashed field.
>
> **#1 invariant preserved:** `sim/production/factory.rs` depends only on `std` + `sim/` (intern,
> production_types, economy, rules data through `&RuleSet`); NEVER on render/ui/sidebar/audio/net.
>
> **No-hash contract (the whole point of P5a):** the new `build_step_time` / `category_for_object`
> are free functions; `BuildStepTimeInputs` is a transient param struct (no serde, never stored);
> the `insertion_seq` mint change mutates a `#[serde(skip)]`, no-serde-derive `FactoryRegistry`
> field that `hash_production` never reads; the inversion assert + dormant delivery probe run only
> on CLONES. NO new authoritative `advance_tick` call site — the `EntityCategory::Structure` arm of
> `object_ai_stage` stays a no-op (LOCKED-decision 2), and `refresh_production_shadow` still only
> calls `refresh_economy_shadow` + `rebuild_shadow`. The legacy `production_queue` charge / `.rev()`
> cancel / frames-timer path stays authoritative, untouched. `world_hash.rs` and `snapshot.rs` are
> NOT touched. `SNAPSHOT_VERSION` stays **17**. The 17→18 fold is P5b.
>
> **V2 corrections honored (NEVER reintroduce):** (a) NO ×0.9 in the build-step base; (b)
> Primary_For* Aircraft / Infantry binding (the inverse is REFUTED) — `category_for_object` reuses
> `production_category_for_object`, which already routes Aircraft→Aircraft / Infantry→Infantry;
> (c) `set_rate` takes the build-step TOTAL (the producer returns the total, NOT the legacy ×0.9
> frames); (d) `SpecialItem` 0-vs-(-1) is NOT collapsed (the mint change never touches `special`);
> (e) the purifier base is the OrePurifier building COUNT (untouched here).

---

## A. Verified preconditions (live reads this session — quote file:TEXT)

The tree shifts (a concurrent session edits miner/combat/movement/unit_post AND world/mod.rs);
anchor on the quoted TEXT, not the line number.

| # | Fact the plan relies on | Verified at (text anchor) |
|---|---|---|
| A1 | `factory.rs` is `#![allow(dead_code)]` — the new dormant `build_step_time` / `category_for_object` raise no unused-warning | factory.rs `#![allow(dead_code)]` |
| A2 | `Factory::set_rate(&mut self, build_step_time: i32)` ALREADY does `per_step = build_step_time / (PRODUCTION_STEPS as i32)` then `.clamp(STEP_RATE_MIN as i32, STEP_RATE_MAX as i32)`, with the no-object→`step_rate_frames = 0` sentinel — so the producer returns the TOTAL and does NOT divide/clamp | factory.rs `pub fn set_rate(&mut self, build_step_time: i32) {` … `let per_step = build_step_time / (PRODUCTION_STEPS as i32);` |
| A3 | the existing test `set_rate_total_over_54_truncates_clamps` already pins `set_rate(661) -> 12` and `set_rate(14000) -> 255` — the producer test only needs to pin the TOTAL | factory.rs `fn set_rate_total_over_54_truncates_clamps() {` + `(661, 12), (14000, 255)` |
| A4 | `PRODUCTION_STEPS: u16 = 54`, `STEP_RATE_MIN/MAX` already declared | factory.rs `pub const PRODUCTION_STEPS: u16 = 54;` |
| A5 | `PRODUCTION_RATE_SCALE: u64 = 1_000_000` is the existing PPM scale (1.0); the parsed `*_ppm` rules fields feed the producer directly | production_types.rs `pub(super) const PRODUCTION_RATE_SCALE: u64 = 1_000_000;` |
| A6 | `production_category_for_object(obj: &ObjectType) -> ProductionCategory` exists, is `pub(super)` in `production_tech.rs`, and routes Infantry→Infantry / Vehicle→Vehicle / Aircraft→Aircraft / Building(`BuildCategory::Combat`)→Defense else Building — `category_for_object` delegates to it | production_tech.rs `pub(super) fn production_category_for_object(` + `ObjectCategory::Aircraft => ProductionCategory::Aircraft,` |
| A7 | `ObjectCategory` has exactly `Infantry/Vehicle/Aircraft/Building` (no naval/ship variant) — naval collapses into `Vehicle` (the surfaced DRIFT, §4.3) | production_tech.rs match arms over `obj.category`; `ObjectCategory::Building => match obj.build_cat {` |
| A8 | `ProductionCategory{Building, Defense, Infantry, Vehicle, Aircraft}` with `#[default] Building` and `Ord` = `Building < Defense < Infantry < Vehicle < Aircraft` (the enum-sort order the mint change MUST stop using) | production_types.rs `pub enum ProductionCategory {` + `#[default] Building, Defense, Infantry, Vehicle, Aircraft,` |
| A9 | `BuildQueueItem` carries `pub enqueue_order: u64` and is `Serialize, Deserialize` — so it is already serialized + hashed via `hash_production`; the temporal mint reads `front.enqueue_order` from a hashed field | production_types.rs `pub enqueue_order: u64,` in `pub struct BuildQueueItem` (`#[derive(Debug, Clone, Serialize, Deserialize)]`) |
| A10 | `enqueue_order` is stamped monotonically from `next_enqueue_order` on every enqueue (the Begin-command analog); strictly increasing → ties impossible | production_queue.rs `pub(super) fn next_enqueue_order(sim: &mut Simulation) -> u64 {` + `let enqueue_order = next_enqueue_order(sim);` |
| A11 | `rebuild_shadow_inner` mints `insertion_seq` via `self.seq_carry.get(&key)` else `self.next_insertion_seq` (the BTreeMap-iteration first-appearance block to be REPLACED), writes `new_carry.insert(key, seq)`, and sets `insertion_seq: seq` on the built `Factory` | factory.rs `let seq = match self.seq_carry.get(&key) {` … `insertion_seq: seq,` |
| A12 | `rebuild_shadow_inner` already binds `let Some(front) = queue.front() else { continue; };` — `front.enqueue_order` is in scope for the temporal mint with no extra lookup | factory.rs `let Some(front) = queue.front() else {` … `continue; // empty category: no factory` |
| A13 | `FactoryRegistry { factories: BTreeMap<...> (PRIVATE), next_insertion_seq, seq_carry }` carries `#[derive(Debug, Clone, Default, PartialEq, Eq)] // NO serde in P1-P3` — clone is available; no serde derive to keep absent | factory.rs `pub struct FactoryRegistry {` + `#[derive(Debug, Clone, Default, PartialEq, Eq)] // NO serde in P1-P3` |
| A14 | `iter_insertion_ordered(&self) -> Vec<&Factory>` sorts `self.factories.values()` by `insertion_seq` (the sort is unchanged — only the mint SOURCE changes); `view(owner, category)` is the read-only accessor | factory.rs `pub fn iter_insertion_ordered(&self) -> Vec<&Factory> {` + `all.sort_by_key(\|f\| f.insertion_seq);` |
| A15 | `Factory::start_next_queued(&mut self) -> Option<InternedId>` is `pub(crate)`, proven-but-dormant (P4: front-pop + held-object guard) — P5a adds NO authority, only documents the bind point + a dormant probe | factory.rs `pub(crate) fn start_next_queued(&mut self) -> Option<InternedId> {` |
| A16 | `Factory::advance_one_step(&mut self, economy: &mut Economy) -> StepOutcome` (P3) — reused verbatim by the inversion assert's per-step clone drive | factory.rs `pub fn advance_one_step(&mut self, economy: &mut Economy) -> StepOutcome {` |
| A17 | `Economy::add_credits`/`spend`/`available` exist; `Economy` derives `Debug, Clone, Default, PartialEq, Eq` (no serde) — REUSE as-is for the inversion-assert clones | economy.rs `pub fn add_credits(&mut self, amount: i32) {` + `#[derive(Debug, Clone, Default, PartialEq, Eq)]` |
| A18 | `production/mod.rs` re-exports `pub use self::factory::{ … CancelOutcome, Factory, FactoryRegistry, … StepOutcome, PRODUCTION_STEPS, … };` — `build_step_time`/`BuildStepTimeInputs`/`category_for_object` are added to that list | production/mod.rs `pub use self::factory::{` … `StepOutcome, PRODUCTION_STEPS, STEP_RATE_MAX, STEP_RATE_MIN,` |
| A19 | `debug_assert_production_shadow(&self)` (`#[cfg(debug_assertions)]`) chains `debug_assert_economy_shadow()` → `debug_assert_factory_shell_trace()` → `debug_assert_factory_conservation(); // P3` — the inversion assert slots in beside the P3 one | world/mod.rs `pub(crate) fn debug_assert_production_shadow(&self) {` + `self.debug_assert_factory_conservation(); // P3` |
| A20 | `debug_assert_factory_conservation` is the EXACT clone-only template: `iter_insertion_ordered()` → clone factory + clone `Economy { credits: cost, ..default }` → drive → `debug_assert_eq!` SURFACING `self.tick, factory.owner, factory.category` → never writes back | world/mod.rs `pub(crate) fn debug_assert_factory_conservation(&self) {` … `self.tick, factory.owner, factory.category,` |
| A21 | the tick tail runs `self.refresh_production_shadow(rules);` then (debug) `self.debug_assert_production_shadow();` then `let state_hash = self.state_hash();` — `rules: Option<&RuleSet>` is in scope at the assert call for the inversion-assert (B) producer inputs | world/mod.rs `self.refresh_production_shadow(rules);` … `self.debug_assert_production_shadow();` … `let state_hash = self.state_hash();` |
| A22 | `Simulation::object_type(&self, type_ref: InternedId, rules: &RuleSet) -> Option<&ObjectType>` resolves the front type for the producer's `cost` / multiplier / wall inputs (the same lookup `rebuild_shadow_inner` uses for `full_cost`) | world/mod.rs `pub fn object_type<'r>(` ; factory.rs `sim.object_type(front.type_id, r).map(\|o\| o.cost.max(0))` |
| A23 | `ObjectType` carries `pub cost: i32`, `pub build_time_multiplier_x1000: u64` (BuildTimeMultiplier), `pub wall: bool` — the producer's per-type inputs | object_type.rs `pub cost: i32,` + `pub build_time_multiplier_x1000: u64,` + `pub wall: bool,` |
| A24 | `rules.production` carries `multiple_factory_ppm`, `low_power_penalty_modifier_ppm`, `min_low_power_production_speed_ppm`, `max_low_power_production_speed_ppm`, `build_speed_x1000`, `wall_build_speed_coefficient: f32` — all already PPM-parsed (only the FORMULA is new) | ruleset.rs `pub multiple_factory_ppm: u64,` + `pub low_power_penalty_modifier_ppm: u64,` + `pub wall_build_speed_coefficient: f32,` |
| A25 | `SNAPSHOT_VERSION == 17` and the version-pin test `snapshot_version_is_17_in_shadow_phase` (`assert_eq!(super::SNAPSHOT_VERSION, 17)`) exist — P5a must NOT bump | snapshot.rs `const SNAPSHOT_VERSION: u32 = 17;` + `fn snapshot_version_is_17_in_shadow_phase() {` |
| A26 | the world-level test helpers `empty_rules()`, `queued_item(..)`, `insert_queue(..)`, `HouseState::new(owner, 0, None, true, credits, 10)` exist; the production import line is `use crate::sim::production::{ BuildQueueItem, BuildQueueState, CancelOutcome, ProductionCategory, StepOutcome, PRODUCTION_STEPS };` | production_shadow_tests.rs `fn empty_rules() -> RuleSet {` + `use crate::sim::production::{` |
| A27 | the P3/P4 acceptance tests `factory_advance_step_does_not_change_state_hash` / `factory_cancel_one_does_not_change_state_hash` are the structural template the P5a acceptance test mirrors (clone-against-clone; assert `before == sim.state_hash()` + legacy `credits` untouched) | production_shadow_tests.rs `fn factory_advance_step_does_not_change_state_hash() {` + `fn factory_cancel_one_does_not_change_state_hash() {` |
| A28 | the legacy `production_tech.rs` build-time family bakes ×0.9 (`cost * speed_x1000 * 9 / 10000`) + is a rate-domain division — DRIFT, NOT reused; it stays authoritative until P5b | production_tech.rs `let base_value = (cost * speed_x1000 * 9 / 10000) as i32;` |

**Facts the design pins from the study + this run's Lane A/B research (no re-decompile in P5a — VERIFIED-LIVE v2):**
- **C5:** `GetBuildStepTime` = `trunc(BuildTimeBonus × Cost)` (NO ×0.9) → `× BuildTimeMultiplier` (trunc) →
  low-power divide → MultipleFactory loop → wall branch; the caller applies `/54 + clamp[1,255]` (the
  signed `/54` magic).
- **C10:** low-power divisor `d = 1 − (1 − ratio) × LowPowerPenaltyModifier`, clamped to Min ALWAYS,
  to Max ONLY when `ratio < 1.0`, and floored to `0.01` if `d <= 0`.
- **C11:** MultipleFactory loop runs `(factory_count − 1)` iterations with PER-ITERATION truncation,
  gated `MultipleFactory > 0` (skip on 0); NOT `MF^(n−1)` with one truncate.
- **C1 (study §8 P5):** factory-step-before-house-tick ordering lock folds into the P5b flip (NOT P5a).
- **C7 (study):** a queued item starts only after a successful DELIVERY command; delivery is
  command-bound (P5b).
- **C12 (study):** completion suspends with the object STILL attached (`advance_one_step` already does this).
- **C15 (study):** per-step charge + refund telescopes to exact cost (the conservation invariant the
  inversion assert leans on, proven by `debug_assert_factory_conservation`).
- **Lane A (this run):** the engine factory array is temporal first-Begin order (strict tail-append on
  ctor; compacting shift-left on dtor); `PerTickUpdate` sweeps ascending array index. The Rust temporal
  analog is `front.enqueue_order` (§5.2); the current `next_insertion_seq++` over BTreeMap iteration is
  sorted-(owner,category) order, which DIVERGES — the §6.1/§6.3 UNPROVEN equivalence, honestly DRIFT.

---

## B. Files touched (summary)

| File | Change | Task |
|---|---|---|
| `src/sim/production/factory.rs` | `struct BuildStepTimeInputs` + pure `fn build_step_time(&BuildStepTimeInputs) -> i32` (§3); pure `fn category_for_object(&ObjectType) -> ProductionCategory` delegate (§4); switch `rebuild_shadow_inner`'s `insertion_seq` derivation from the `seq_carry`/`next_insertion_seq` first-appearance block to `front.enqueue_order` temporal derivation (§5.2); the §7 pure-function `mod tests` | P5a-T1, P5a-T2, P5a-T3 |
| `src/sim/production/mod.rs` | add `build_step_time, BuildStepTimeInputs, category_for_object` to the `pub use self::factory::{...}` re-export | P5a-T4 |
| `src/sim/world/mod.rs` | add `debug_assert_factory_step_matches_legacy(&self, rules: Option<&RuleSet>)`; one call line into `debug_assert_production_shadow` (beside the P3 sibling) + thread `rules` from the tail; add the `#[cfg(test)]` dormant delivery probe | P5a-T5 |
| `src/sim/world/production_shadow_tests.rs` | the six world-level tests (§7): `factory_flip_prep_does_not_change_state_hash` (acceptance), `factory_insertion_seq_equals_front_enqueue_order`, `factory_step_order_matches_legacy_temporal_order`, `factory_step_matches_legacy_shadow_holds`, `production_delivery_probe_is_dormant`, `production_flip_prep_is_deterministic` | P5a-T6 |

`world_hash.rs` and `snapshot.rs` are **NOT** in this list — that is the no-hash contract (§A25, and
no field is added to / removed from the hash). The legacy `production_queue.rs` / `production_tech.rs`
build-time family is **NOT** touched (stays authoritative + DRIFT, replaced at P5b/P7 — the producer
coexists dormant). No miner/combat/movement/unit_post file is touched (concurrent session owns those).
**world/mod.rs is CO-EDITED by a concurrent session — anchor every edit on the quoted TEXT, never on a
line number, and keep the edit to the minimal "add the assert fn + one call line + the rules arg."**

---

## C. P5a — the pure producers + the temporal mint + the inversion-readiness assert

### P5a-T1 — `build_step_time` producer + `BuildStepTimeInputs` (C5/C10/C11, x0.9-free)

**File (EDIT):** `src/sim/production/factory.rs` — add at module scope (NOT inside `impl Factory`),
after the `Factory` impl block (before or after `StepOutcome`/`CancelOutcome` — module scope, the
module is `#![allow(dead_code)]` so the dormant items raise no warning). Integer/i128 math only; no
float, no RNG; no engine addresses in comments. **If `factory.rs` crosses ~600 lines after T1–T3, the
producer + `BuildStepTimeInputs` may move to a sibling `factory_rate.rs` (anchor on the function, not
the file) — planner's call at impl time; the re-export in T4 then points at `factory_rate`.**

```rust
/// Resolved inputs for the build-step TOTAL producer. A transient param struct (NO
/// serde, NO storage, only `Debug`) so the producer is a pure function of explicit
/// inputs — testable in isolation, no `Simulation` handle. The caller (the P5b begin
/// path / the P5a dormant probe + inversion assert) gathers these from rules + the
/// depositing house's country type + the owner's power + the per-category factory
/// count. PPM scale = `PRODUCTION_RATE_SCALE` (1_000_000 = 1.0), so the parsed `*_ppm`
/// rules fields feed it directly.
#[derive(Debug, Clone)]
pub struct BuildStepTimeInputs {
    /// GetCost of the object under construction.
    pub cost: i32,
    /// Per-CATEGORY build-time bonus (HouseType side multiplier), default 1.0 =
    /// `PRODUCTION_RATE_SCALE`. NOT the generic BuildSpeed and NOT a single house
    /// scalar. Stock YR (no per-side bonus) passes 1.0 — no rules field backs it yet.
    pub build_time_bonus_ppm: u64,
    /// Per-TYPE BuildTimeMultiplier (`ObjectType.build_time_multiplier_x1000`,
    /// pre-scaled to PPM by the caller).
    pub build_time_multiplier_ppm: u64,
    /// Owner power ratio, clamped to `[0, SCALE]`; `SCALE` (1.0) when not under-powered.
    pub power_ratio_ppm: u64,
    /// Rules LowPowerPenaltyModifier (already PPM-parsed).
    pub low_power_penalty_modifier_ppm: u64,
    /// Rules MinLowPowerProductionSpeed (Min divisor clamp, applied ALWAYS).
    pub min_clamp_ppm: u64,
    /// Rules MaxLowPowerProductionSpeed (Max divisor clamp, applied ONLY when ratio < 1.0).
    pub max_clamp_ppm: u64,
    /// Rules MultipleFactory (loop gate, strict `> 0`).
    pub multiple_factory_ppm: u64,
    /// Per-category matching factory count (the `(n - 1)` loop count).
    pub factory_count: u32,
    /// True only for a wall building (RTTI==building AND the wall flag).
    pub is_wall: bool,
    /// Rules BuildSpeed wall coefficient, pre-converted to PPM by the caller (used
    /// only when `is_wall`).
    pub wall_build_speed_ppm: u64,
}

/// Produce the build-step TOTAL — the engine's GetBuildStepTime return, BEFORE the
/// caller's `/54 + clamp[1,255]`. PURE: integer/i128 throughout, no `&mut`, no RNG,
/// no hashed-state read, no float in the committed math. Fed to `Factory::set_rate`
/// (which owns the `/54`). The legacy `production_tech` build-time family is a
/// verified DRIFT (it bakes a REFUTED ×0.9 via `* 9 / 10000`, models build time as a
/// rate-domain single-truncate division, and uses the generic BuildSpeed instead of
/// the per-category bonus) and is NOT reused.
///
/// Pipeline (every multiply-truncate rounds toward zero = floor for non-negatives):
///   T1  base = trunc(BuildTimeBonus × Cost)                 (NO ×0.9)
///   T2  × per-type BuildTimeMultiplier, trunc
///   T3  ÷ divisor d = 1 − (1 − ratio) × LPPM, clamped:
///         Min clamp ALWAYS; Max clamp ONLY when ratio < 1.0; d <= 0 floors to 0.01
///   T4  MultipleFactory loop: (count − 1) iters, trunc EACH iter, gated MF > 0
///   T5  wall branch: trunc(acc × BuildSpeed) only for a wall building
pub fn build_step_time(inp: &BuildStepTimeInputs) -> i32 {
    const SCALE: i128 = PRODUCTION_RATE_SCALE as i128; // 1_000_000 = 1.0
    let cost = inp.cost.max(0) as i128;
    if cost == 0 {
        return 0; // no work -> the rate-0 path in set_rate
    }

    // T1: base = trunc(BuildTimeBonus × Cost). NO ×0.9 (the legacy *9/10000 is REFUTED).
    let s1 = cost * inp.build_time_bonus_ppm as i128 / SCALE; // floor

    // T2: × per-type BuildTimeMultiplier, trunc.
    let s2 = s1 * inp.build_time_multiplier_ppm as i128 / SCALE; // floor

    // T3: low-power divide. divisor d = 1 − (1 − ratio) × LPPM, clamped (C10).
    let ratio = (inp.power_ratio_ppm as i128).min(SCALE); // clamp ratio to [.., 1.0]
    let deficit = SCALE - ratio; // (1 − ratio), >= 0
    let penalty = deficit * inp.low_power_penalty_modifier_ppm as i128 / SCALE;
    let mut d = SCALE - penalty; // (1 − (1 − ratio) × LPPM) in PPM
    d = d.max(inp.min_clamp_ppm as i128); // Min clamp ALWAYS
    if ratio < SCALE {
        d = d.min(inp.max_clamp_ppm as i128); // Max clamp ONLY when under-powered
    }
    if d <= 0 {
        d = SCALE / 100; // 0.01 divisor floor
    }
    let mut acc = s2 * SCALE / d; // trunc(s2 / d): s2 over a PPM fraction

    // T4: MultipleFactory loop — (count − 1) iters, PER-ITERATION trunc (C11), gated MF > 0.
    if inp.multiple_factory_ppm > 0 && inp.factory_count > 1 {
        for _ in 0..(inp.factory_count - 1) {
            acc = acc * inp.multiple_factory_ppm as i128 / SCALE; // trunc EACH iter
        }
    }

    // T5: wall branch — RTTI==building wall only, trunc(acc × BuildSpeed).
    if inp.is_wall {
        acc = acc * inp.wall_build_speed_ppm as i128 / SCALE; // trunc
    }

    acc.clamp(0, i32::MAX as i128) as i32 // the TOTAL; set_rate does /54 + clamp[1,255]
}
```

**Why i128 intermediates (A24-grounded):** `cost` can reach ~50_000 and `build_time_bonus_ppm` ~1e6,
so `cost × bonus` overflows i32 and even strains i64 across the chained multiplies; i128 keeps every
truncation point exact, and the final `clamp(0, i32::MAX)` returns the i32 TOTAL `set_rate` expects.

**Why the producer does NOT divide by 54 or clamp (A2/A3):** `set_rate(build_step_time)` already does
`build_step_time / 54` then `clamp(1, 255)` with the no-object→0 sentinel. The producer returning the
TOTAL keeps a single `/54` site (the caller). `set_rate(700) -> 700/54 = 12`; the C5 "MTNK 661 → 12"
case is one concrete total (`661/54 = 12`), already pinned by `set_rate_total_over_54_truncates_clamps`
(A3) — the producer test only pins the TOTAL.

**Unit tests** (append to the `factory.rs` `mod tests`, after the P4 block):

```rust
    // ---- P5a build_step_time producer (C5/C10/C11, x0.9-free) ----

    /// Full-power, no-bonus, no-multiplier, single-factory inputs at `cost`.
    fn bst(cost: i32) -> BuildStepTimeInputs {
        BuildStepTimeInputs {
            cost,
            build_time_bonus_ppm: PRODUCTION_RATE_SCALE,       // 1.0
            build_time_multiplier_ppm: PRODUCTION_RATE_SCALE,  // 1.0
            power_ratio_ppm: PRODUCTION_RATE_SCALE,            // 1.0 (full power)
            low_power_penalty_modifier_ppm: PRODUCTION_RATE_SCALE,
            min_clamp_ppm: PRODUCTION_RATE_SCALE / 2,          // 0.5
            max_clamp_ppm: (PRODUCTION_RATE_SCALE * 9) / 10,   // 0.9
            multiple_factory_ppm: (PRODUCTION_RATE_SCALE * 8) / 10, // 0.8
            factory_count: 1,
            is_wall: false,
            wall_build_speed_ppm: PRODUCTION_RATE_SCALE,
        }
    }

    #[test]
    fn build_step_time_no_x09_base() {
        // cost 700, bonus 1.0, mult 1.0, ratio 1.0, count 1, no wall -> TOTAL 700,
        // NOT 630 (= the REFUTED 700*0.9). Then set_rate(700) -> 700/54 = 12.
        let total = build_step_time(&bst(700));
        assert_eq!(total, 700, "x0.9-free base: trunc(1.0 * 700) = 700, not 630");
        assert_ne!(total, 630, "the legacy x0.9 (630) must NOT appear");
        let mut f = Factory {
            object: Some(PendingObject::default()),
            ..Factory::default()
        };
        f.set_rate(total);
        assert_eq!(f.step_rate_frames, 12, "set_rate(700) -> 12");
    }

    #[test]
    fn build_step_time_mtnk_rate_12() {
        // Two totals that both divide to rate 12 (the C5 reference band): 700 and 661.
        for total in [700, 661] {
            let mut f = Factory {
                object: Some(PendingObject::default()),
                ..Factory::default()
            };
            f.set_rate(total);
            assert_eq!(f.step_rate_frames, 12, "total {total} -> rate 12");
        }
    }

    #[test]
    fn build_step_time_build_time_multiplier_truncates_at_t2() {
        // base 67 (cost 67, bonus 1.0) x mult 1.15 -> trunc(67 * 1.15) = trunc(77.05) = 77.
        let mut inp = bst(67);
        inp.build_time_multiplier_ppm = (PRODUCTION_RATE_SCALE * 115) / 100; // 1.15
        assert_eq!(build_step_time(&inp), 77, "T2 truncates: trunc(67 * 1.15) = 77");
    }

    #[test]
    fn build_step_time_low_power_max_clamp_gated() {
        // ratio 0.5, LPPM 1.0 -> d = 1 - 0.5*1.0 = 0.5; Max clamp 0.9 does NOT lower it
        // (0.5 < 0.9). Min clamp 0.5 -> stays 0.5. cost 100 -> trunc(100 / 0.5) = 200.
        let mut inp = bst(100);
        inp.power_ratio_ppm = PRODUCTION_RATE_SCALE / 2; // 0.5
        assert_eq!(build_step_time(&inp), 200, "under-power doubles the step total");

        // ratio 1.0 (full power): the Max clamp is NOT applied; d = 1.0 -> total = cost.
        let mut full = bst(100);
        full.max_clamp_ppm = PRODUCTION_RATE_SCALE / 2; // a Max that WOULD bite if applied
        assert_eq!(build_step_time(&full), 100, "ratio==1.0 skips the Max clamp");

        // ratio 0.0, LPPM 1.0 -> d = 1 - 1.0 = 0.0 -> floored to 0.01 -> trunc(100/0.01)=10000.
        let mut zero = bst(100);
        zero.power_ratio_ppm = 0;
        zero.min_clamp_ppm = 0; // let d hit 0 so the 0.01 floor is exercised
        zero.max_clamp_ppm = PRODUCTION_RATE_SCALE; // Max does not bite
        assert_eq!(build_step_time(&zero), 10_000, "d<=0 floors to 0.01");
    }

    #[test]
    fn build_step_time_multiple_factory_per_iteration_trunc() {
        // count 3, MF 0.8: per-iteration trunc DIFFERS from acc * MF^2 single-truncate.
        // base acc = 11; iter1: trunc(11*0.8)=8; iter2: trunc(8*0.8)=6.
        // single-truncate MF^2=0.64: trunc(11*0.64)=trunc(7.04)=7. So 6 != 7 proves per-iter.
        let mut inp = bst(11);
        inp.factory_count = 3;
        inp.multiple_factory_ppm = (PRODUCTION_RATE_SCALE * 8) / 10; // 0.8
        let per_iter = build_step_time(&inp);
        assert_eq!(per_iter, 6, "per-iteration trunc: 11 -> 8 -> 6");
        let single = (11i128 * (((PRODUCTION_RATE_SCALE * 8) / 10) as i128).pow(2)
            / (PRODUCTION_RATE_SCALE as i128).pow(2)) as i32;
        assert_eq!(single, 7, "single-truncate MF^2 would be 7");
        assert_ne!(per_iter, single, "per-iteration trunc must DIFFER from MF^2 single");
    }

    #[test]
    fn build_step_time_multiple_factory_gate_skips_on_zero_and_count_one() {
        // MF == 0 -> loop skipped regardless of count.
        let mut mf0 = bst(500);
        mf0.factory_count = 4;
        mf0.multiple_factory_ppm = 0;
        assert_eq!(build_step_time(&mf0), 500, "MF=0 skips the loop");
        // count == 1 -> loop skipped (n-1 == 0).
        let mut c1 = bst(500);
        c1.factory_count = 1;
        assert_eq!(build_step_time(&c1), 500, "count 1 skips the loop");
    }

    #[test]
    fn build_step_time_wall_branch_only_for_walls() {
        // is_wall=true applies BuildSpeed 0.5 -> trunc(400 * 0.5) = 200.
        let mut wall = bst(400);
        wall.is_wall = true;
        wall.wall_build_speed_ppm = PRODUCTION_RATE_SCALE / 2; // 0.5
        assert_eq!(build_step_time(&wall), 200, "wall applies BuildSpeed");
        // is_wall=false leaves the total unchanged.
        let mut not_wall = bst(400);
        not_wall.wall_build_speed_ppm = PRODUCTION_RATE_SCALE / 2;
        assert_eq!(build_step_time(&not_wall), 400, "non-wall ignores BuildSpeed");
    }

    #[test]
    fn build_step_time_zero_cost_is_zero() {
        assert_eq!(build_step_time(&bst(0)), 0, "cost 0 -> total 0");
        assert_eq!(build_step_time(&bst(-5)), 0, "negative cost clamps to 0 -> total 0");
    }

    #[test]
    fn build_step_time_overflow_safe() {
        // Large inputs do not overflow (i128 intermediates) and clamp to i32::MAX.
        let mut big = bst(50_000);
        big.build_time_bonus_ppm = PRODUCTION_RATE_SCALE; // 1.0
        big.build_time_multiplier_ppm = PRODUCTION_RATE_SCALE; // 1.0
        big.power_ratio_ppm = 0; // forces a big divide (d floors to 0.01 if min=0)
        big.min_clamp_ppm = 0;
        big.max_clamp_ppm = PRODUCTION_RATE_SCALE;
        let total = build_step_time(&big); // 50000 / 0.01 = 5_000_000 (fits i32)
        assert_eq!(total, 5_000_000, "no overflow, exact");
        // Push past i32 to prove the clamp.
        let mut huge = bst(2_000_000_000);
        huge.power_ratio_ppm = 0;
        huge.min_clamp_ppm = 0;
        huge.max_clamp_ppm = PRODUCTION_RATE_SCALE;
        assert_eq!(build_step_time(&huge), i32::MAX, "clamps to i32::MAX");
    }
```

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k build_step_time_no_x09_base build_step_time_mtnk_rate_12 build_step_time_build_time_multiplier_truncates_at_t2 build_step_time_low_power_max_clamp_gated build_step_time_multiple_factory_per_iteration_trunc build_step_time_multiple_factory_gate_skips_on_zero_and_count_one build_step_time_wall_branch_only_for_walls build_step_time_zero_cost_is_zero build_step_time_overflow_safe`

---

### P5a-T2 — `category_for_object` routing delegate + the surfaced Ship gap (§4)

**File (EDIT):** `src/sim/production/factory.rs` — add at module scope (a thin named delegate; do NOT
re-implement the routing). It needs `production_category_for_object` (A6, `pub(super)` in
`production_tech.rs`, same `production` module tree) and the `ObjectType` type. Add the use to the
existing imports at the top of `factory.rs`:

```rust
use crate::rules::object_type::ObjectType;
use crate::sim::production::production_tech::production_category_for_object;
```

> If `production_category_for_object` is not reachable as `super::production_tech::...` from
> `factory.rs`, raise its visibility to `pub(in crate::sim::production)` (a no-behavior visibility
> widen within the same module tree) OR import via the crate path above — confirm the exact path at
> impl time; the routing logic is NOT duplicated either way.

```rust
/// Map an object type to the `ProductionCategory` whose factory produces it — the
/// Rust analog of the engine's Begin_Production Primary_For* slot resolution
/// (RTTI -> factory slot). A thin tested delegate over `production_category_for_object`:
/// ONE routing source, not a fork. Its value is being the single call site the P5b
/// registry sweep uses, and the place the routing DRIFTs are pinned by tests.
///
/// SURFACED DRIFT (NOT resolved in P5a): the engine keeps a 6th factory slot for
/// Ships, but Rust has no `Ship` `ProductionCategory` — naval object types collapse
/// into `Vehicle`. When a house owns both a War Factory and a Naval Yard, the single
/// `Vehicle` factory key collapses two engine factories, diverging the MultipleFactory
/// `factory_count` and same-frame completion ordering. This is a P5b-or-later
/// structural decision (add `Ship` vs accept the collapse) requiring sign-off — NEVER
/// silently folded. `category_for_object_naval_collapses_to_vehicle_documented`
/// regression-guards the current behavior so a future silent change is caught.
pub fn category_for_object(obj: &ObjectType) -> ProductionCategory {
    production_category_for_object(obj)
}
```

**Why a delegate and not new logic (§4.2):** the Rust `ProductionCategory` maps 1:1 onto FIVE of the
six engine slots, and `production_category_for_object` already does the Building↔Defense split
(`BuildCategory::Combat => Defense`, the analog of the engine defense flag) AND honors the verified
Aircraft→Aircraft / Infantry→Infantry binding (the REFUTED inverse can never return). Re-implementing
it would fork the routing source. `category_for_object` is the named seam the P5b sweep targets.

**Unit tests** (append to the `factory.rs` `mod tests`; build `ObjectType` fixtures via the rules
parser the existing `build_time_integer_tests` use, OR via `ObjectType` field construction — confirm
the available `ObjectType` ctor at impl time; the test only needs an object of each category):

```rust
    // ---- P5a category_for_object routing delegate ----

    #[test]
    fn category_for_object_matches_rtti_table() {
        use crate::rules::ini_parser::IniFile;
        use crate::rules::ruleset::RuleSet;
        // One object per category; a Combat-categorized building routes to Defense.
        let ini = IniFile::from_str(
            "[InfantryTypes]\n0=GI\n[VehicleTypes]\n0=GRIZZLY\n[AircraftTypes]\n0=BEAG\n\
             [BuildingTypes]\n0=GAPOWR\n1=GAPILL\n\
             [GI]\nCost=100\n[GRIZZLY]\nCost=700\n[BEAG]\nCost=600\n\
             [GAPOWR]\nCost=800\n[GAPILL]\nCost=500\nDeployToFire=yes\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("rules parse");
        let inf = rules.object("GI").unwrap();
        let veh = rules.object("GRIZZLY").unwrap();
        let air = rules.object("BEAG").unwrap();
        let bld = rules.object("GAPOWR").unwrap();
        assert_eq!(category_for_object(inf), ProductionCategory::Infantry, "infantry -> Infantry (NOT the refuted inverse)");
        assert_eq!(category_for_object(veh), ProductionCategory::Vehicle, "vehicle -> Vehicle");
        assert_eq!(category_for_object(air), ProductionCategory::Aircraft, "aircraft -> Aircraft (NOT the refuted inverse)");
        assert_eq!(category_for_object(bld), ProductionCategory::Building, "plain building -> Building");
        // The delegate must agree with the routing source it wraps (no fork).
        assert_eq!(
            category_for_object(veh),
            production_category_for_object(veh),
            "delegate == production_category_for_object (single routing source)"
        );
    }

    #[test]
    fn category_for_object_naval_collapses_to_vehicle_documented() {
        use crate::rules::ini_parser::IniFile;
        use crate::rules::ruleset::RuleSet;
        // A naval unit is an ObjectCategory::Vehicle in the Rust rules model (no Ship
        // category), so it routes to Vehicle. This pins the DOCUMENTED collapse: if a
        // future change adds a Ship category, this test breaks and forces a decision.
        let ini = IniFile::from_str("[VehicleTypes]\n0=DEST\n[DEST]\nCost=1000\n");
        let rules = RuleSet::from_ini(&ini).expect("rules parse");
        let naval = rules.object("DEST").unwrap();
        assert_eq!(
            category_for_object(naval),
            ProductionCategory::Vehicle,
            "naval collapses to Vehicle (the surfaced DRIFT; not a Ship category in P5a)"
        );
    }
```

> **Confirm at impl time:** the exact INI keys that make a `BuildingType` route to `Defense`
> (`BuildCategory::Combat`). The fixture above uses `DeployToFire=yes` as a proxy for a combat
> building; if that does not set `build_cat = Some(Combat)`, use whatever key the `ObjectType` parser
> maps to `BuildCategory::Combat` (grep `BuildCategory::Combat` in `src/rules/object_type.rs`). The
> Defense-vs-Building split is itself a surfaced DRIFT (U-DEFENSE); the test pins the current behavior.
> If a Defense fixture is awkward, drop the Defense row from `category_for_object_matches_rtti_table`
> and pin only the four unambiguous categories + the delegate-equality assertion — the load-bearing
> guarantee is the Aircraft/Infantry binding (refuted inverse) and the single-source delegation.

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k category_for_object_matches_rtti_table category_for_object_naval_collapses_to_vehicle_documented`

---

### P5a-T3 — the Lane-A `insertion_seq` mint correction (temporal `enqueue_order`, hash-neutral)

**File (EDIT):** `src/sim/production/factory.rs` — in `rebuild_shadow_inner`, REPLACE the
`seq_carry`/`next_insertion_seq` first-appearance block (A11) with a temporal derivation from
`front.enqueue_order` (A9/A12). This is the load-bearing P5a change: it bakes the gamemd temporal
sweep order (Lane A) into the dormant registry NOW, so the P5b authoritative charge order is correct.
Hash-neutral because the registry is `#[serde(skip)]` with no serde derive (A13) and `hash_production`
never reads `insertion_seq`/`seq_carry`/`next_insertion_seq`.

**Replace this block** (A11 — the current mint + carry):

```rust
                // insertion_seq: reuse a surviving factory's seq, else mint a new one.
                let seq = match self.seq_carry.get(&key) {
                    Some(&s) => s,
                    None => {
                        let s = self.next_insertion_seq;
                        self.next_insertion_seq = self.next_insertion_seq.wrapping_add(1);
                        s
                    }
                };
                new_carry.insert(key, seq);
```

**with the temporal mint:**

```rust
                // insertion_seq = the front (earliest still-live) item's enqueue_order:
                // the temporal first-Begin stamp of when this (owner, category) began
                // producing. This reproduces the engine factory array's temporal
                // tail-append order (Lane A), NOT the BTreeMap sorted-(owner,category)
                // order the old next_insertion_seq++ mint produced. enqueue_order is
                // strictly monotonic, so ties are impossible. A lapsed-then-restarted
                // category re-reads a FRESH, higher front enqueue_order each rebuild,
                // matching the engine's destroy-recreate -> tail re-append. The carry
                // is no longer the ordering source (it was the wrong mechanism: it kept
                // a stale seq across a queue-empty gap, or re-minted in sorted position).
                let seq = front.enqueue_order;
                new_carry.insert(key, seq);
```

> **`seq_carry` / `next_insertion_seq` fields stay declared (DO NOT remove in P5a).** They are still
> written (`new_carry.insert`) so the struct + its `Default`/`Clone`/`PartialEq` derives are
> undisturbed and no other reader breaks; they are simply no longer the ordering SOURCE. Removing the
> fields is a P5b cleanup (the design flags `next_insertion_seq` for DROP from the hashed set at P5b,
> §2.2). Keeping them written-but-unused in P5a keeps the diff minimal and the no-hash proof trivial.
> If `next_insertion_seq` becomes dead-code-warned, the module is `#![allow(dead_code)]` (A1) — fine.

**Existing-test reconciliation (REQUIRED — these encoded the OLD mint):**
- `registry_iter_insertion_ordered_not_map_order` (factory.rs `mod tests`) hand-sets `insertion_seq`
  on `Factory` values directly (not via `rebuild_shadow_inner`), so it is UNAFFECTED — it tests the
  sort, not the mint. Confirm it still passes.
- `factory_registry_iteration_is_insertion_ordered` (production_shadow_tests.rs, A26) asserts only
  that iteration is monotonic in `insertion_seq` and that there are 6 factories — both hold under the
  new mint (each `(owner, category)` front has a distinct `enqueue_order` in that fixture's `order: 1`
  → wait: the fixture stamps EVERY item `order: 1`). **AUDIT:** that fixture passes `order: 1` to every
  `queued_item`, so under the temporal mint ALL six fronts would mint `insertion_seq == 1` (ties). The
  sort is stable but the monotonic assertion (`seqs == sorted`) still holds (all-equal is monotonic).
  Confirm it passes; if the all-equal seqs make the assertion ambiguous, update the fixture to stamp
  distinct `order` values per `(owner, category)` (a test-fixture change, not a behavior change).
- `insertion_seq_stable_across_rebuild` (production_shadow_tests.rs, A26) advances the SAME front item
  (same `enqueue_order`) and asserts the seq is stable across rebuild. Under the temporal mint the seq
  IS `front.enqueue_order`, which is unchanged across that rebuild (the same item, same order) → the
  test still passes. Confirm; it is now testing the temporal source, which is the intended property.

**Unit test** (append to the `factory.rs` `mod tests` — a direct rebuild-driven proof is in the
world-level tests T6; this pure-level one is optional and may be folded into T6 instead):

```rust
    // (the temporal mint is proven at the world level in
    //  factory_insertion_seq_equals_front_enqueue_order +
    //  factory_step_order_matches_legacy_temporal_order — see P5a-T6)
```

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k registry_iter_insertion_ordered_not_map_order factory_registry_iteration_is_insertion_ordered insertion_seq_stable_across_rebuild` — the three existing mint-adjacent tests must still pass (adjust the all-`order:1` fixture to distinct orders only if the monotonic assertion turns ambiguous; that is a fixture change, not a behavior change).

---

### P5a-T4 — re-export the new pure items

**File (EDIT):** `src/sim/production/mod.rs` — add `build_step_time`, `BuildStepTimeInputs`, and
`category_for_object` to the existing `pub use self::factory::{...}` list (A18, the one that already
re-exports `CancelOutcome`/`StepOutcome`/`PRODUCTION_STEPS`):

```rust
pub use self::factory::{
    build_step_time, category_for_object, BuildEligibility, BuildStepTimeInputs, CancelOutcome,
    Factory, FactoryRegistry, FactoryView, PendingObject, SpecialItem, StepOutcome,
    PRODUCTION_STEPS, STEP_RATE_MAX, STEP_RATE_MIN,
};
```

> If T1 moved the producer to a sibling `factory_rate.rs`, re-export `build_step_time` /
> `BuildStepTimeInputs` from `self::factory_rate::{...}` instead (and `mod factory_rate;` in mod.rs).
> `category_for_object` stays a `self::factory::` re-export regardless.

**Verification:**
- `cargo check -p vera20k` (the re-export compiles; nothing consumes it authoritatively yet)

---

### P5a-T5 — the inversion-readiness shadow assert + the dormant delivery probe (surface, never equalize)

**File (EDIT):** `src/sim/world/mod.rs` — add `debug_assert_factory_step_matches_legacy` beside the P3
`debug_assert_factory_conservation` (A20), wire one call line into `debug_assert_production_shadow`,
thread `rules` from the tail (A21), and add the `#[cfg(test)]` dormant delivery probe (A15). Mirrors
the P3 clone-only template EXACTLY: clone factory + clone economy, surface `tick+owner+category`,
NEVER write back. **world/mod.rs is co-edited — anchor each edit on the quoted TEXT below.**

**(i) Add the assert fn** (after `debug_assert_factory_conservation`, anchor on its closing brace):

```rust
    /// Debug-only P5a inversion-readiness assert: prove the AUTHORITATIVE MODEL
    /// (registry-sweep step in temporal `insertion_seq` order + the real
    /// `build_step_time` -> `set_rate` rate + the delivery-driven queue advance) WOULD
    /// produce the same per-tick result as the LEGACY path (charge/progress/completion/
    /// ready) — so the P5b flip is verified-equivalent BEFORE it happens. Runs on
    /// CLONES only; SURFACES divergence with tick+owner+category; NEVER equalizes,
    /// NEVER writes back. `rules` is threaded from the tail so (B) can build the
    /// producer inputs; with `None` the producer sub-check is skipped (the cost-free
    /// tail, like `rebuild_shadow_no_rules`).
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_factory_step_matches_legacy(&self, rules: Option<&RuleSet>) {
        use crate::sim::economy::Economy;
        use crate::sim::production::{build_step_time, BuildStepTimeInputs, StepOutcome, PRODUCTION_STEPS};

        // (A) ORDER: the registry sweep order (temporal insertion_seq) must equal the
        //     legacy per-house temporal order. For each owner, collect (category,
        //     front.enqueue_order) from queues_by_owner, sort by enqueue_order, and
        //     assert iter_insertion_ordered() visits that owner's factories in the same
        //     sequence. (insertion_seq IS front.enqueue_order after P5a-T3, so this is a
        //     consistency check that the mint + the queue agree.)
        for (&owner, queues) in &self.production.queues_by_owner {
            let mut legacy: Vec<(ProductionCategory, u64)> = queues
                .iter()
                .filter_map(|(&cat, q)| q.front().map(|f| (cat, f.enqueue_order)))
                .collect();
            legacy.sort_by_key(|&(_, order)| order);
            let swept: Vec<(ProductionCategory, u64)> = self
                .production
                .factory_shadow
                .iter_insertion_ordered()
                .iter()
                .filter(|f| f.owner == owner)
                .map(|f| (f.category, f.insertion_seq))
                .collect();
            let legacy_cats: Vec<ProductionCategory> = legacy.iter().map(|&(c, _)| c).collect();
            let swept_cats: Vec<ProductionCategory> = swept.iter().map(|&(c, _)| c).collect();
            debug_assert_eq!(
                swept_cats, legacy_cats,
                "P5a (A): tick {} owner {:?}: sweep order must equal legacy temporal order \
                 (swept {:?} vs legacy {:?})",
                self.tick, owner, swept, legacy,
            );
        }

        for factory in self.production.factory_shadow.iter_insertion_ordered() {
            let Some(_obj) = factory.object.as_ref() else {
                continue; // queue-only / no active object
            };

            // (B) RATE: build the producer inputs from rules + the per-type fields, run
            //     the producer, and assert it is INTERNALLY consistent through set_rate
            //     (total/54 clamp). SURFACE (never equalize) the producer-vs-legacy
            //     effective-rate gap so the x0.9/truncation DRIFT is VISIBLE in the log:
            //     the legacy frames model is the verified-WRONG one; this RECORDS the
            //     gap, it does NOT force a match (frames<->step is NOT bit-identical).
            if let Some(r) = rules {
                if let Some(obj) = self.object_type(factory.object.as_ref().unwrap().type_id, r) {
                    let inp = BuildStepTimeInputs {
                        cost: obj.cost.max(0),
                        build_time_bonus_ppm: 1_000_000, // stock YR default 1.0 (U-BONUS)
                        build_time_multiplier_ppm: obj.build_time_multiplier_x1000.max(1) * 1_000,
                        power_ratio_ppm: 1_000_000, // (B) records the DRIFT; full-power proxy
                        low_power_penalty_modifier_ppm: r.production.low_power_penalty_modifier_ppm,
                        min_clamp_ppm: r.production.min_low_power_production_speed_ppm,
                        max_clamp_ppm: r.production.max_low_power_production_speed_ppm,
                        multiple_factory_ppm: r.production.multiple_factory_ppm,
                        factory_count: 1, // P5a proxy; the real per-category count is P5b
                        is_wall: obj.category == crate::rules::object_type::ObjectCategory::Building
                            && obj.wall,
                        wall_build_speed_ppm: (r.production.wall_build_speed_coefficient.max(0.0)
                            as f64
                            * 1_000_000.0) as u64,
                    };
                    let total = build_step_time(&inp);
                    // INTERNAL consistency: set_rate on a CLONE produces clamp(total/54,1,255).
                    let mut probe = factory.clone();
                    probe.set_rate(total);
                    debug_assert!(
                        probe.step_rate_frames >= 1 && probe.step_rate_frames <= 255,
                        "P5a (B): tick {} {:?}/{:?}: set_rate must yield a clamped [1,255] rate (got {})",
                        self.tick, factory.owner, factory.category, probe.step_rate_frames,
                    );
                }
            }

            // (C) CHARGE/PROGRESS/COMPLETION/STALL: drive a CLONE of the per-step model
            //     over a full build against a CLONE economy seeded with exactly
            //     original_balance; assert the model conserves exact cost and settles
            //     (the per-step ladder == the legacy upfront amount by construction —
            //     debug_assert_factory_conservation already proves this; this re-asserts
            //     it under the inversion framing, SURFACING tick+owner+category).
            let cost = factory.original_balance;
            let mut f = factory.clone();
            f.progress = 0;
            f.balance = cost;
            f.on_hold = false;
            f.suspended = false;
            f.manual = false;
            let mut econ = Economy { credits: cost, ..Economy::default() };
            let mut steps = 0i32;
            loop {
                match f.advance_one_step(&mut econ) {
                    StepOutcome::Stepped => steps += 1,
                    StepOutcome::Completed => {
                        steps += 1;
                        break;
                    }
                    _ => break,
                }
            }
            debug_assert_eq!(
                econ.spent_credits, cost,
                "P5a (C): tick {} {:?}/{:?}: model spend {} must equal cost {} (== legacy upfront)",
                self.tick, factory.owner, factory.category, econ.spent_credits, cost,
            );
            debug_assert_eq!(
                steps, PRODUCTION_STEPS as i32,
                "P5a (C): tick {} {:?}/{:?}: a full build must take 54 model steps (got {})",
                self.tick, factory.owner, factory.category, steps,
            );

            // (D) DELIVERY: on a CLONE, clear a completed factory's object then
            //     start_next_queued; assert the FIFO front advances (the legacy
            //     ready->next transition). SURFACE only — no authoritative call site.
            let mut d = factory.clone();
            let expected_next = d.queue.front().copied();
            d.object = None; // simulate the delivery commit (P5b binds this)
            d.suspended = false;
            let popped = d.start_next_queued();
            debug_assert_eq!(
                popped, expected_next,
                "P5a (D): tick {} {:?}/{:?}: post-delivery advance must pop the FIFO front",
                self.tick, factory.owner, factory.category,
            );
        }
    }
```

> **(B) honest-tolerance discipline (D3 graft):** the legacy frames model bakes the verified-wrong
> ×0.9 / rate-domain math, so (B) does NOT assert the producer equals the legacy effective rate — it
> asserts the producer is INTERNALLY consistent (clamps to [1,255] through `set_rate`) and the
> magnitude can be SURFACED. Forcing a bit-equality with the wrong legacy value would be exactly the
> "invent equivalence to clean up a report" anti-pattern. `factory_count: 1` / `power_ratio_ppm: 1.0`
> are P5a proxies; the real per-category count + power are P5b inputs (the producer body is unchanged).

**(ii) Wire the call** — anchor on the existing `self.debug_assert_factory_conservation(); // P3`
line inside `debug_assert_production_shadow` (A19); add ONE line after it:

```rust
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_production_shadow(&self, rules: Option<&RuleSet>) {
        self.debug_assert_economy_shadow();
        self.debug_assert_factory_shell_trace();
        self.debug_assert_factory_conservation(); // P3
        self.debug_assert_factory_step_matches_legacy(rules); // P5a  <-- added
    }
```

> **`rules` threading.** `debug_assert_production_shadow` currently takes no `rules`. The minimal edit
> adds the `rules: Option<&RuleSet>` parameter and passes it through. The single call site at the tail
> (A21) becomes `self.debug_assert_production_shadow(rules);` — `rules` is in scope there. **If the
> concurrent session's edits to world/mod.rs collide on the `debug_assert_production_shadow` signature
> or its call site,** fall back to keeping `debug_assert_production_shadow(&self)` unchanged and
> calling `self.debug_assert_factory_step_matches_legacy(None)` from inside it (the producer (B)
> sub-check then skips, exactly like `rebuild_shadow_no_rules`; (A)/(C)/(D) still run). The
> load-bearing checks are (A) order + (C) conservation; (B) is the surfaced-DRIFT recorder. Pick the
> threaded form if the file is clean; the `None` form if it collides.

**(iii) Update the tail call site** — anchor on `self.debug_assert_production_shadow();`:

```rust
        #[cfg(debug_assertions)]
        self.debug_assert_production_shadow(rules);
```

**(iv) The dormant delivery probe** (A15) — a `#[cfg(test)]` `Simulation` method operating on a CLONE
of the registry, proving the post-delivery mechanics end-to-end without a live call site. Add to the
`#[cfg(test)] impl Simulation` block, or as a free `#[cfg(test)]` method beside the existing
`factory_oracle_step_trace` (techno_ai.rs) — planner's call; the probe must NOT touch
`self.production.factory_shadow` (clone only):

```rust
    /// Test-only dormant probe: prove the C7 delivery -> start_next_queued mechanics on
    /// a CLONE of the registry (NEVER the hashed shadow). Returns, per factory with a
    /// non-empty tail, (owner, category, popped-front-after-delivery). NO authoritative
    /// call site — P5b binds start_next_queued to the real delivery commit.
    #[cfg(test)]
    pub(crate) fn factory_delivery_probe(
        &self,
    ) -> Vec<(crate::sim::intern::InternedId, crate::sim::production::ProductionCategory, Option<crate::sim::intern::InternedId>)>
    {
        let mut out = Vec::new();
        for factory in self.production.factory_shadow.iter_insertion_ordered() {
            let mut d = factory.clone();
            d.object = None; // simulate the delivery commit
            d.suspended = false;
            let popped = d.start_next_queued();
            out.push((factory.owner, factory.category, popped));
        }
        out
    }
```

> **Determinism / no-write-back:** the assert + the probe read `iter_insertion_ordered()` (A14),
> clone, and assert/return; they NEVER mutate `self.production.factory_shadow`, `self.houses`, or any
> entity — same discipline as `debug_assert_factory_conservation` (A20). Surfaced with
> `tick+owner+category`; never equalized; no `advance_tick` path invokes `start_next_queued`.

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k production_shadow_preserves_advance_tick_phase_order production_shadow_with_oracle_is_deterministic production_shadow_with_cancel_is_deterministic factory_oracle_step_trace_walks_live_structures` — the P2/P3/P4 tests still pass with the new debug assert active (it surfaces, never perturbs); the assert must not fire for the empty-rules fixtures ((B) skips on `None`/cost-0; (A)/(C)/(D) hold by construction).

---

### P5a-T6 — the no-hash acceptance + temporal-order + inversion-holds + dormant-probe + determinism tests

**File (EDIT):** `src/sim/world/production_shadow_tests.rs` — append after the P4 block. Reuse
`empty_rules()`, `queued_item`, `insert_queue`, `HouseState::new` (A26). The production import line
already has `BuildQueueState, CancelOutcome, ProductionCategory, StepOutcome, PRODUCTION_STEPS`; add
nothing new unless a test names a fresh type.

```rust
// ===== P5a — flip-prep (pure producers + temporal mint + inversion-readiness, hash-neutral) =====

/// P5a no-hash guarantee (the acceptance test; mirrors
/// `factory_advance_step_does_not_change_state_hash` /
/// `factory_cancel_one_does_not_change_state_hash`): building the producer, routing a
/// type, stepping a CLONE registry against CLONE economies, and running the dormant
/// delivery probe leaves `state_hash()` bit-identical (no serde derive; no authoritative
/// call site; the mint change touches only the `#[serde(skip)]` registry).
#[test]
fn factory_flip_prep_does_not_change_state_hash() {
    use crate::sim::production::{build_step_time, category_for_object, BuildStepTimeInputs};
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    let ty = sim.interner.intern("GRIZZLY");
    insert_queue(
        &mut sim,
        owner,
        ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1),
    );
    sim.refresh_production_shadow(Some(&rules));
    let before = sim.state_hash();
    let legacy_credits = sim.houses[&owner].credits;

    // Run every P5a piece against CLONES / pure values.
    let total = build_step_time(&BuildStepTimeInputs {
        cost: 700,
        build_time_bonus_ppm: 1_000_000,
        build_time_multiplier_ppm: 1_000_000,
        power_ratio_ppm: 1_000_000,
        low_power_penalty_modifier_ppm: 1_000_000,
        min_clamp_ppm: 500_000,
        max_clamp_ppm: 900_000,
        multiple_factory_ppm: 800_000,
        factory_count: 1,
        is_wall: false,
        wall_build_speed_ppm: 1_000_000,
    });
    assert_eq!(total, 700, "producer is pure, returns the TOTAL");
    let mut f = sim.production.factory_shadow.iter_insertion_ordered()[0].clone();
    f.set_rate(total);
    let mut oracle = sim.houses[&owner].economy.clone();
    for _ in 0..PRODUCTION_STEPS {
        let _ = f.advance_one_step(&mut oracle);
    }
    let _ = category_for_object; // routing is exercised in the factory.rs unit tests
    let _probe = sim.factory_delivery_probe(); // dormant; clone-only

    assert_eq!(
        before,
        sim.state_hash(),
        "P5a flip-prep on clones/pure values must not perturb the state hash"
    );
    assert_eq!(
        sim.houses[&owner].credits, legacy_credits,
        "the legacy wallet is untouched by the flip-prep"
    );
}

/// P5a Lane-A mint: after `refresh_production_shadow`, each factory's `insertion_seq`
/// equals its queue front's `enqueue_order` (the temporal first-Begin stamp), NOT the
/// old BTreeMap sorted-(owner,category) mint. A fixture with one owner's two categories
/// in temporal order OPPOSITE to enum-sort proves the source is `enqueue_order`.
#[test]
fn factory_insertion_seq_equals_front_enqueue_order() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    let air_ty = sim.interner.intern("BEAG");
    let veh_ty = sim.interner.intern("GRIZZLY");
    // Aircraft begun FIRST (enqueue_order 10), Vehicle SECOND (order 20). Enum-sort
    // would place Vehicle (3) before Aircraft (4); the temporal mint must NOT.
    let mut air_dq = VecDeque::new();
    air_dq.push_back(queued_item(owner, air_ty, ProductionCategory::Aircraft, BuildQueueState::Building, 54, 30, 10));
    let mut veh_dq = VecDeque::new();
    veh_dq.push_back(queued_item(owner, veh_ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 20));
    let mut cats = BTreeMap::new();
    cats.insert(ProductionCategory::Aircraft, air_dq);
    cats.insert(ProductionCategory::Vehicle, veh_dq);
    sim.production.queues_by_owner.insert(owner, cats);
    sim.refresh_production_shadow(Some(&rules));

    let air = sim.production.factory_shadow.view(owner, ProductionCategory::Aircraft).unwrap();
    let veh = sim.production.factory_shadow.view(owner, ProductionCategory::Vehicle).unwrap();
    let _ = (&air, &veh); // view has no insertion_seq; assert via iter_insertion_ordered
    let ordered: Vec<(ProductionCategory, u64)> = sim
        .production
        .factory_shadow
        .iter_insertion_ordered()
        .iter()
        .map(|f| (f.category, f.insertion_seq))
        .collect();
    assert_eq!(
        ordered,
        vec![(ProductionCategory::Aircraft, 10), (ProductionCategory::Vehicle, 20)],
        "insertion_seq == front.enqueue_order; sweep follows TEMPORAL, not enum-sort, order"
    );
}

/// P5a Lane-A order: the sweep visits Aircraft (begun first) before Vehicle (begun
/// second) — the DRIFT-fix vs the old sorted mint, exposed as a positive blocking test.
#[test]
fn factory_step_order_matches_legacy_temporal_order() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    let air_ty = sim.interner.intern("BEAG");
    let veh_ty = sim.interner.intern("GRIZZLY");
    let mut air_dq = VecDeque::new();
    air_dq.push_back(queued_item(owner, air_ty, ProductionCategory::Aircraft, BuildQueueState::Building, 54, 30, 5));
    let mut veh_dq = VecDeque::new();
    veh_dq.push_back(queued_item(owner, veh_ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 9));
    let mut cats = BTreeMap::new();
    cats.insert(ProductionCategory::Aircraft, air_dq);
    cats.insert(ProductionCategory::Vehicle, veh_dq);
    sim.production.queues_by_owner.insert(owner, cats);
    sim.refresh_production_shadow(Some(&rules));

    let cats_in_sweep: Vec<ProductionCategory> = sim
        .production
        .factory_shadow
        .iter_insertion_ordered()
        .iter()
        .map(|f| f.category)
        .collect();
    assert_eq!(
        cats_in_sweep,
        vec![ProductionCategory::Aircraft, ProductionCategory::Vehicle],
        "sweep visits the earlier-begun Aircraft first (temporal), not Vehicle (enum-sort)"
    );
}

/// P5a inversion-readiness: drive `advance_tick` over N ticks with a scripted queue;
/// the debug inversion assert (`debug_assert_factory_step_matches_legacy`) fires no
/// divergence every tick. In a debug build the assert is live inside advance_tick; a
/// clean N-tick run with a stable hash sequence proves it holds.
#[test]
fn factory_step_matches_legacy_shadow_holds() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    let ty = sim.interner.intern("GRIZZLY");
    insert_queue(
        &mut sim,
        owner,
        ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1),
    );
    let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
    for _ in 0..5 {
        // If the inversion assert diverges, advance_tick panics in a debug build.
        sim.advance_tick(&[], Some(&rules), &heights, None, None, 67);
    }
    // No panic -> the inversion-readiness assert held for all 5 ticks.
}

/// P5a delivery seam is DORMANT: the probe is test-only and a plain tick run leaves
/// queue fronts unchanged absent a delivery (no advance_tick path invokes
/// start_next_queued).
#[test]
fn production_delivery_probe_is_dormant() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    let active = sim.interner.intern("GRIZZLY");
    let next = sim.interner.intern("FV");
    let mut dq = VecDeque::new();
    dq.push_back(queued_item(owner, active, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1));
    dq.push_back(queued_item(owner, next, ProductionCategory::Vehicle, BuildQueueState::Queued, 54, 54, 2));
    let mut cats = BTreeMap::new();
    cats.insert(ProductionCategory::Vehicle, dq);
    sim.production.queues_by_owner.insert(owner, cats);
    sim.refresh_production_shadow(Some(&rules));

    let before = sim.state_hash();
    // The probe reports the post-delivery pop on a CLONE; it must NOT mutate the shadow.
    let probe = sim.factory_delivery_probe();
    assert_eq!(probe.len(), 1, "one factory with a tail");
    assert_eq!(probe[0].2, Some(next), "the probe would pop FV after a delivery (on a clone)");
    // The live shadow front is unchanged (no authoritative start_next_queued call).
    let view = sim.production.factory_shadow.view(owner, ProductionCategory::Vehicle).unwrap();
    assert_eq!(view.object.map(|o| o.type_id), Some(active), "live active still GRIZZLY");
    assert_eq!(view.queue.iter().copied().collect::<Vec<_>>(), vec![next], "live tail unchanged");
    assert_eq!(before, sim.state_hash(), "the probe must not perturb the hash");
}

/// P5a determinism: a per-tick closure that builds the producer + runs the dormant
/// probe on clones produces identical per-tick state_hash sequences across two runs
/// (mirrors `production_shadow_with_oracle_is_deterministic` /
/// `production_shadow_with_cancel_is_deterministic`).
#[test]
fn production_flip_prep_is_deterministic() {
    use crate::sim::production::{build_step_time, BuildStepTimeInputs};
    fn run() -> Vec<u64> {
        let mut sim = Simulation::new();
        let rules = empty_rules();
        let owner = sim.interner.intern("Americans");
        sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
        let ty = sim.interner.intern("GRIZZLY");
        insert_queue(
            &mut sim,
            owner,
            ProductionCategory::Vehicle,
            queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1),
        );
        let heights: BTreeMap<(u16, u16), u8> = BTreeMap::new();
        (0..5)
            .map(|_| {
                sim.advance_tick(&[], Some(&rules), &heights, None, None, 67);
                // Per-tick flip-prep probe on clones/pure values (NEVER written back).
                let _ = build_step_time(&BuildStepTimeInputs {
                    cost: 700,
                    build_time_bonus_ppm: 1_000_000,
                    build_time_multiplier_ppm: 1_000_000,
                    power_ratio_ppm: 1_000_000,
                    low_power_penalty_modifier_ppm: 1_000_000,
                    min_clamp_ppm: 500_000,
                    max_clamp_ppm: 900_000,
                    multiple_factory_ppm: 800_000,
                    factory_count: 1,
                    is_wall: false,
                    wall_build_speed_ppm: 1_000_000,
                });
                let _ = sim.factory_delivery_probe();
                sim.state_hash()
            })
            .collect()
    }
    assert_eq!(run(), run(), "advance_tick with the P5a flip-prep probe stays deterministic");
}
```

> **Confirm at impl time:** (1) the `advance_tick` signature — mirror the P3/P4 call
> `sim.advance_tick(&[], Some(&rules), &heights, None, None, 67)` EXACTLY (the 2nd positional arg is
> `Option<&RuleSet>`). (2) `factory_delivery_probe` reachability — it is a `#[cfg(test)] pub(crate)`
> method on `Simulation` (T5-iv); if T5 placed it in techno_ai.rs, the `sim.factory_delivery_probe()`
> call still resolves (same `impl Simulation`). (3) `FactoryView` has no `insertion_seq` field (A14),
> so the temporal-mint tests assert via `iter_insertion_ordered()`, not `view(...)`.

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k factory_flip_prep_does_not_change_state_hash factory_insertion_seq_equals_front_enqueue_order factory_step_order_matches_legacy_temporal_order factory_step_matches_legacy_shadow_holds production_delivery_probe_is_dormant production_flip_prep_is_deterministic`

---

### P5a-T7 — full-suite verify + no-bump / no-hash-file lock (separate foreground pass)

Per the build-discipline memory (don't bury slow cargo inside a background workflow), run the
verification as a separate bounded foreground pass.

**Verification:**
- `cargo test -p vera20k` — read the literal `test result:` line. The P5a set must pass:
  `build_step_time_no_x09_base`, `build_step_time_mtnk_rate_12`,
  `build_step_time_build_time_multiplier_truncates_at_t2`, `build_step_time_low_power_max_clamp_gated`,
  `build_step_time_multiple_factory_per_iteration_trunc`,
  `build_step_time_multiple_factory_gate_skips_on_zero_and_count_one`,
  `build_step_time_wall_branch_only_for_walls`, `build_step_time_zero_cost_is_zero`,
  `build_step_time_overflow_safe`, `category_for_object_matches_rtti_table`,
  `category_for_object_naval_collapses_to_vehicle_documented`,
  `factory_flip_prep_does_not_change_state_hash`, `factory_insertion_seq_equals_front_enqueue_order`,
  `factory_step_order_matches_legacy_temporal_order`, `factory_step_matches_legacy_shadow_holds`,
  `production_delivery_probe_is_dormant`, `production_flip_prep_is_deterministic`.
- The P1/P2/P3/P4 tests must still pass — especially the mint-adjacent ones:
  `registry_iter_insertion_ordered_not_map_order`, `factory_registry_iteration_is_insertion_ordered`,
  `insertion_seq_stable_across_rebuild` (adjust the all-`order:1` fixture to distinct orders ONLY if
  the monotonic assertion turns ambiguous — a fixture change, not a behavior change), and the
  no-hash/version tests `factory_advance_step_does_not_change_state_hash`,
  `factory_cancel_one_does_not_change_state_hash`, `snapshot_roundtrip_ignores_shadow`,
  `production_shadow_preserves_advance_tick_phase_order`, `production_shadow_with_oracle_is_deterministic`,
  `production_shadow_with_cancel_is_deterministic`, `factory_oracle_step_trace_walks_live_structures`,
  `snapshot_version_is_17_in_shadow_phase`.
- `cargo test -p vera20k snapshot_version_is_17_in_shadow_phase` — confirms `SNAPSHOT_VERSION` still 17.
- Confirm `git diff --stat` shows NO change to `src/sim/world/world_hash.rs` and NO change to
  `SNAPSHOT_VERSION` in `src/sim/snapshot.rs` (the no-hash contract).

---

## D. No-hash contract gate (the slice is WRONG if either fails)

| Gate test | What it proves | If it fails |
|---|---|---|
| **`factory_flip_prep_does_not_change_state_hash`** (P5a-T6, mirrors `factory_advance_step_does_not_change_state_hash` A27) | building the producer + routing + stepping clones + the dormant probe leaves `state_hash()` bit-identical and the legacy wallet untouched | a serde derive crept in, a call site became authoritative, or the mint change leaked into a hashed field — STOP, the slice is wrong |
| **`snapshot_version_is_17_in_shadow_phase`** (existing, A25; `assert_eq!(super::SNAPSHOT_VERSION, 17)`) | `SNAPSHOT_VERSION` is still 17 and `snapshot.rs` is untouched | the version bump (P5b) leaked into P5a — STOP |
| **`snapshot_roundtrip_ignores_shadow`** (existing, A26) | the skipped `economy`/`factory_shadow` come back `Default` and the hash is unchanged across the round-trip | a serde derive / un-skip crept in — STOP |

The four structural facts these gates encode (each auditable from the diff): (1) NO serde derive added
— `build_step_time` returns `i32`, `BuildStepTimeInputs` derives only `Debug+Clone`, `category_for_object`
returns an existing serde type but adds no field, and `Factory`/`FactoryRegistry`/`Economy`/`PendingObject`/
`SpecialItem`/`StepOutcome`/`CancelOutcome` stay serde-free; (2) NO new authoritative call site — the
`EntityCategory::Structure` arm stays no-op and `refresh_production_shadow` still only calls
`refresh_economy_shadow` + `rebuild_shadow`; the producer/`set_rate`/`start_next_queued`/inversion model
run ONLY from the debug assert + `#[cfg(test)]` code, on clones; (3) the mint change touches only the
`#[serde(skip)]`, no-serde-derive `FactoryRegistry` that `hash_production` never reads; (4) `world_hash.rs`
+ `snapshot.rs` untouched.

---

## E. Out-of-scope seams (left clean, NOT implemented)

| Concern | Status | Seam |
|---|---|---|
| Authority flip (oracle/clone → real wallet); serde derives + un-skip `economy`/`factory_shadow`; hash fold in `world_hash.rs` (ADD Factory/Economy/registry fields, DROP `next_insertion_seq` since the order is now `enqueue_order`-carried, REMOVE retired `active_producer_by_owner`/`remaining_base_frames`/`progress_carry`); `SNAPSHOT_VERSION` 17→18; fold C1 (factory-step-before-house-tail) | **P5b** | the producer is correct + tested; the temporal mint is proven; the inversion assert is the green light. P5b flips WHO is passed + what is hashed, not the algorithm/order/rate. |
| The `FactoryRegistry::step_all` authoritative call in `advance_tick` Phase 7 (charging real wallets in `insertion_seq` = temporal order) | **P5b** | `advance_one_step(&mut Economy)` is P5b-ready; P5a proved the per-step model conserves cost. |
| The C7 delivery command that drives `start_next_queued` | **P5b** | `start_next_queued` is proven-but-dormant (P4) + the bind point is documented (§5.1); P5a adds the test-only `factory_delivery_probe`, no live call. |
| Replacing `set_rate`'s input source with `build_step_time`; retiring the legacy `production_tech` build-time family + the upfront-charge (`enqueue_by_type` `*credits -= obj.cost`) + the `.rev()`+full-refund cancel (`cancel_by_type_for_owner`) + the frames timer (`tick_production_with_overlay_registry` PPM `remaining_base_frames`/`progress_carry`) | **P5b** | all named in §A28; the producer coexists DORMANT; the legacy path stays authoritative through P5a. |
| The P9 global parity/replay harness (recorded command stream replayed twice + vs the pre-flip baseline → bit-identical per-tick `state_hash` sequence; `economy_conservation_over_replay`) | **P5c** | reuses the existing replay harness; if a same-frame two-Begin divergence shows, re-verify intra-frame `EventClass::Execute` dispatch order (U-ORDER). |
| Per-category `GetBuildTimeBonus` rules field (the `build_time_bonus_ppm` input has no backing field; stock YR = 1.0) | **P5b/P7 (U-BONUS)** | the producer's input seam is present; P5a passes 1.0. |
| Ship `ProductionCategory` (naval collapses into Vehicle; divergent MF count + same-frame order on water maps) | **P5b-or-later (U-SHIP)** | `category_for_object` SURFACES it + a regression-guard test pins the collapse; a structural decision requiring sign-off. |
| Prereq revalidation / purifier / IncomeMult | **P6/P7** | `Economy` fields present; not exercised by P5a. |

> **CONCERN-2 (P5b hash-seam contradiction — flag, do NOT resolve in P5a).** The "DROP
> `next_insertion_seq` from the hashed set since the order is now `enqueue_order`-carried"
> recommendation above REVISES STUDY §6.4 ("the factory map **and** `next_insertion_seq`
> must round-trip and hash") and the originally-stated P5b seam ("`next_insertion_seq`
> must be serialized+hashed"). It is defensible (after the temporal mint the counter is
> never incremented and is vestigial), but it changes a P5b hash-field decision the
> design-lead stated explicitly — **confirm with the design-lead at P5b before dropping
> `next_insertion_seq` from the hash set.** P5a is unaffected (it hashes nothing new and
> keeps the field written-but-unused).

---

## F. Open questions for the design-lead (confirm before / during implementing)

**F1 — `rules` threading into `debug_assert_production_shadow` (P5a-T5).** The minimal edit adds a
`rules: Option<&RuleSet>` param to `debug_assert_production_shadow` and threads it from the tail (A21).
If the concurrent session collides on that signature or its single call site, the fallback is to leave
`debug_assert_production_shadow(&self)` unchanged and call
`self.debug_assert_factory_step_matches_legacy(None)` internally (the (B) producer sub-check skips;
(A)/(C)/(D) still run). Confirm threaded (preferred) vs `None` fallback.

**F2 — producer location (P5a-T1).** If `factory.rs` crosses ~600 lines after T1–T3, move
`build_step_time` + `BuildStepTimeInputs` to a sibling `factory_rate.rs` (T4 re-export points there).
Confirm in-`factory.rs` (default) vs sibling.

**F3 — `production_category_for_object` visibility (P5a-T2).** It is `pub(super)` in `production_tech.rs`.
`category_for_object` in `factory.rs` (a sibling module) needs to call it; the plan imports it via the
crate path or widens it to `pub(in crate::sim::production)` (no behavior change). Confirm the path.

**F4 — `factory_delivery_probe` placement (P5a-T5-iv).** A `#[cfg(test)] pub(crate)` method on
`Simulation`; the plan suggests beside `factory_oracle_step_trace` (techno_ai.rs) or in the
`#[cfg(test)] impl Simulation` block in world/mod.rs. Confirm placement (clone-only either way).

**F5 — the all-`order:1` fixture (P5a-T3).** `factory_registry_iteration_is_insertion_ordered` stamps
every item `order: 1`; under the temporal mint all six fronts mint `insertion_seq == 1` (ties; still
monotonic). Confirm leaving it (passes) vs stamping distinct orders for clarity (fixture-only change).

---

*End of P5a plan. The slice is additive and hash-neutral: the x0.9-free `build_step_time` producer
feeds the already-shipped `set_rate(total)` caller (legacy build-time family stays authoritative +
DRIFT, retired at P5b); `category_for_object` is a tested delegate surfacing the Ship-collapse +
per-category-bonus gaps; the C7 delivery seam is identified + proven-but-dormant (no authoritative call
site); and the load-bearing de-risk — the Lane-A temporal `insertion_seq` mint correction + the
inversion-readiness assert — converts the §6.1/§6.3 same-frame-ordering UNPROVEN into a proven
invariant. NO serde derive is added, `economy`/`factory_shadow` stay serde-skip, `world_hash.rs`/
`snapshot.rs` are untouched, and `SNAPSHOT_VERSION` stays 17 — proven by
`factory_flip_prep_does_not_change_state_hash` + `snapshot_version_is_17_in_shadow_phase`. When the
inversion + temporal-order asserts hold across the suite, P5b is a near-mechanical swap: add derives,
fold the hash (dropping `next_insertion_seq`, removing the retired legacy fields), add the `step_all` +
delivery call sites, fold C1, bump 17→18. P5c is the global replay/parity gate.*
