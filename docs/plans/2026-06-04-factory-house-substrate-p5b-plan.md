<!--
Provenance: authored 2026-06-04 from the APPROVED design
  docs/plans/2026-06-04-factory-house-substrate-p5b-design.md
  (WINNER D-PARITY-MIN, minimum-hash-delta, unanimous 24/22.33/21 across three judges;
   grafted: D-RISK 4-micro-step authoring order + stale-test inversion reconciliation;
   D-SUBSTRATE §3.3 double-hash audit + P5d retirement seam).
House style mirrored from docs/plans/2026-06-04-factory-house-substrate-p5a-plan.md.
Status: DRAFTED, not approved or executed. Review (/review-plan) before implementing.
Scope: P5b ONLY — the atomic authority flip + SNAPSHOT_VERSION 17->18 + the C1 ordering fold.
  The FIRST hashed-state change in the program; the milestone; the riskiest slice. IN:
  serde + un-skip the five shadow types; the hash fold (ADD Factory+Economy+registry fields,
  REMOVE remaining_base_frames+progress_carry from the per-item fold, DROP next_insertion_seq+
  seq_carry FIELDS); flip the per-step charge to the REAL wallet (house.credits) via
  FactoryRegistry::step_all at Phase-7 head BEFORE the house tail (C1 fold); persist Factory
  progress/balance across ticks (reconcile-not-rebuild); bind start_next_queued at the C7
  delivery/cancel commit; swap set_rate's input to the build_step_time producer; retire the
  upfront charge / .rev()-full-refund cancel / frames timer / the credits_entry_for_owner
  auto-create-house hazard / the legacy build-time family.
  OUT (clean seams, NOT implemented): P6 prereq 3-way; P7 purifier-count/IncomeMult/
  HarvestedCredits economy fix; P5c (P9) replay/parity acceptance gate; the Ship category
  (D2 deferred follow-up); full queues_by_owner retirement into Factory.queue (P5d seam).
Locked decisions designed-within (NOT relitigated): D1 drop next_insertion_seq+seq_carry
  (REVISES STUDY §6.4, resolves P5a-review CONCERN-2); D2 defer Ship (naval stays collapsed
  into Vehicle, pinned by category_for_object_naval_collapses_to_vehicle_documented); STEP
  ORDER = registry sweep in insertion_seq (= temporal enqueue_order) order; FOLD C1 into the
  one 17->18 bump; the EntityCategory::Structure arm stays a no-op (sweep is a standalone
  Phase-7 step, NOT per-building dispatch). §3.4 OVERRIDE: KEEP active_producer_by_owner hashed
  (verified-live authoritative producer-focus binding, not a throwaway) — the scope-F
  legacy_active_producer_removed_from_hash test is DROPPED.
-->

# Factory/House Substrate — P5b Plan (the authority flip: serde + hash fold + real-wallet charge + C1 fold + 17->18)

> Linear path: **P5b-T1 -> P5b-T2 -> ... -> P5b-T12**, authored as the four design micro-steps:
> **M1 INVERT** (T1) -> **M2 ROUND-TRIP** (T2..T5) -> **M3 CHARGE-FLIP** (T6..T9) -> **M4 DELIVERY+C1** (T10..T11) -> verify (T12).
> Every task builds green (`cargo check -p vera20k`) before the next. cargo is a SEPARATE
> FOREGROUND PASS the human runs AFTER the workflow (T12) — do NOT run cargo inside the workflow.
>
> **#1 invariant preserved:** `sim/production/factory.rs` depends only on `std` + `sim/` (intern,
> production_types, economy, rules data through `&RuleSet`) + `sim/world::Simulation` (read-only for
> the reconcile/step); NEVER on render/ui/sidebar/audio/net.
>
> **The flip INTENTIONALLY breaks the no-hash contract — that IS this slice.** The near-term guard is
> `factory_flip_determinism_over_scripted_commands` (two sims, scripted stream, identical per-tick
> `state_hash` sequence); the global replay/parity proof is P5c (a clean seam, NOT implemented here).
>
> **Safe-order invariant (the never-uncharged rule):** within the commit, the REAL charge
> (`step_all` live, M3/T6) goes in BEFORE the legacy charge is removed (T7), so no built revision is
> ever double-charged or uncharged.
>
> **V2 corrections honored (NEVER reintroduce):** (a) NO ×0.9 in the build-step base — the
> `build_step_time` producer is already x0.9-free (shipped P5a); (b) Primary_For* Aircraft / Infantry
> binding (the inverse is REFUTED) — `category_for_object` delegates to `production_category_for_object`,
> Aircraft->Aircraft / Infantry->Infantry; (c) `set_rate` takes the build-step TOTAL (the producer
> returns the total, `set_rate` owns `/54 + clamp[1,255]`); (d) `SpecialItem` 0-vs-(-1)-vs-`Item` stays
> distinct (the hash folds the three states distinctly; serde derives on the enum directly); (e) the
> purifier base is the OrePurifier building COUNT (`refresh_economy_shadow`'s purifier pass is
> UNTOUCHED — P7 wires its use); (f) Ship stays collapsed into Vehicle (D2; pinned by the regression
> test) — NOT fixed here.

---

## A. Verified preconditions (live reads THIS session — quote file:TEXT)

The tree shifts (a concurrent session edits miner/combat/movement/unit_post AND world/mod.rs); anchor
on the quoted TEXT, never the line number. Every row below was Read/Grep'd against the live tree this
session; **two drifts vs the context's LIVE-TREE ANCHOR MAP are corrected in A11 and A30.**

| # | Fact the plan relies on | Verified at (text anchor) |
|---|---|---|
| A1 | `factory.rs` is `#![allow(dead_code)]` — new `step_all`/`reconcile_from_queues` raise no unused-warning during the bisectable micro-steps | factory.rs `#![allow(dead_code)]` |
| A2 | `Factory` derives `#[derive(Debug, Clone, Default, PartialEq, Eq)] // NO serde in P1-P3` — serde is ADDED here; the derive line is the anchor | factory.rs `#[derive(Debug, Clone, Default, PartialEq, Eq)] // NO serde in P1-P3` immediately above `pub struct Factory {` |
| A3 | `Factory` fields (the full hash-fold set): `owner, category, progress: u16, step_rate_frames: u16, step_timer: u16, balance: i32, original_balance: i32, object: Option<PendingObject>, on_hold: bool, suspended: bool, manual: bool, special: SpecialItem, queue: VecDeque<InternedId>, insertion_seq: u64` | factory.rs `pub owner: InternedId,` … `pub insertion_seq: u64,` inside `pub struct Factory` |
| A4 | `PendingObject { pub type_id: InternedId, pub entity_id: Option<u64> }` derives `#[derive(Debug, Clone, Default, PartialEq, Eq)] // NO serde in P1-P3` — serde ADDED; the hash folds `type_id` + the `entity_id` Option presence tag | factory.rs `pub struct PendingObject {` + `pub entity_id: Option<u64>,` |
| A5 | `SpecialItem` is a 3-variant enum `NoneNeg1`/`NoneZero`/`Item(u32)` with `#[derive(Debug, Clone, Copy, PartialEq, Eq)] // NO serde in P1-P3` + a manual `Default` => `NoneNeg1` — serde derived directly; 0/-1/Item fold distinctly (NEVER collapse) | factory.rs `pub enum SpecialItem {` + `NoneNeg1,` `NoneZero,` `Item(u32),` |
| A6 | `FactoryRegistry { factories: BTreeMap<(InternedId, ProductionCategory), Factory> (PRIVATE), next_insertion_seq: u64, seq_carry: BTreeMap<...,u64> }` with `#[derive(Debug, Clone, Default, PartialEq, Eq)] // NO serde in P1-P3` — serde ADDED, `next_insertion_seq` + `seq_carry` FIELDS REMOVED (D1) | factory.rs `pub struct FactoryRegistry {` + `next_insertion_seq: u64,` + `seq_carry: BTreeMap<(InternedId, ProductionCategory), u64>,` |
| A7 | `iter_insertion_ordered(&self) -> Vec<&Factory>` sorts `self.factories.values()` by `insertion_seq` (STABLE; the hash fold + the sweep both ride this) | factory.rs `pub fn iter_insertion_ordered(&self) -> Vec<&Factory> {` + `all.sort_by_key(\|f\| f.insertion_seq);` |
| A8 | `advance_one_step(&mut self, economy: &mut Economy) -> StepOutcome` — per-step `balance/steps_left` charge, strict-`<` stall rewind (`if economy.available() < charge { self.progress -= 1; self.on_hold = true; return Stalled }`), completion suspends with object held + balance 0. BODY UNCHANGED by P5b | factory.rs `pub fn advance_one_step(&mut self, economy: &mut Economy) -> StepOutcome {` + `if economy.available() < charge {` |
| A9 | `set_rate(&mut self, build_step_time: i32)` — no-object -> `step_rate_frames = 0`; else `per_step = build_step_time / (PRODUCTION_STEPS as i32)` then `.clamp(STEP_RATE_MIN as i32, STEP_RATE_MAX as i32)`. Takes the TOTAL; owns `/54`. BODY UNCHANGED | factory.rs `pub fn set_rate(&mut self, build_step_time: i32) {` + `let per_step = build_step_time / (PRODUCTION_STEPS as i32);` |
| A10 | `start_next_queued(&mut self) -> Option<InternedId>` is `pub(crate)`, front-pop + held-object guard (`if self.object.is_some() { return None; }`), seeds `progress=0`, leaves balance for the next reconcile to seed. BODY UNCHANGED; bound to the delivery/cancel commit here | factory.rs `pub(crate) fn start_next_queued(&mut self) -> Option<InternedId> {` |
| A11 | **DRIFT-CORRECTED:** `rebuild_shadow_inner` ALREADY mints the temporal seq `let seq = front.enqueue_order; new_carry.insert(key, seq);` (the P5a mint shipped). It still WRITES `new_carry`/`self.seq_carry = new_carry` + reads `total_base_frames`/`remaining_base_frames` for the progress bridge. P5b REPLACES the whole `rebuild_shadow_inner` body with `reconcile_from_queues` (§2.2) and DELETES `seq_carry`/`new_carry` | factory.rs `let seq = front.enqueue_order;` + `new_carry.insert(key, seq);` + `self.seq_carry = new_carry;` |
| A12 | `rebuild_shadow` / `rebuild_shadow_no_rules` / `rebuild_shadow_inner` / `remaining_balance_after(cost, progress)` are the RETIRED rebuild family; `rebuild_shadow` is `pub(crate)` and called only from `refresh_production_shadow` | factory.rs `pub(crate) fn rebuild_shadow(` + `fn rebuild_shadow_inner(` + `fn remaining_balance_after(cost: i32, progress: u16) -> i32 {` |
| A13 | `cancel_one(&mut self, owner, category, type_id, economy: &mut Economy) -> CancelOutcome` — FIRST-match queued removal (`position()` front-to-back) else active-abandon partial refund (`original_balance - balance`). BODY UNCHANGED; the cancel COMMAND routes here | factory.rs `pub fn cancel_one(` + `if let Some(idx) = f.queue.iter().position(\|&t\| t == type_id) {` |
| A14 | `PRODUCTION_STEPS: u16 = 54`, `STEP_RATE_MIN: u16 = 1`, `STEP_RATE_MAX: u16 = 255` | factory.rs `pub const PRODUCTION_STEPS: u16 = 54;` |
| A15 | `build_step_time(&BuildStepTimeInputs) -> i32` (x0.9-free producer) + `BuildStepTimeInputs` (transient, `#[derive(Debug, Clone)]`, NO serde) + `category_for_object(&ObjectType) -> ProductionCategory` are SHIPPED (P5a) | factory.rs `pub fn build_step_time(inp: &BuildStepTimeInputs) -> i32 {` + `pub struct BuildStepTimeInputs {` + `pub fn category_for_object(obj: &ObjectType) -> ProductionCategory {` |
| A16 | `HouseState` derives `#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]` and carries `pub credits: i32` + `#[serde(skip)] pub economy: Economy`. The un-skip anchor is the `#[serde(skip)]` line | house_state.rs `#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]` + `#[serde(skip)]` immediately above `pub economy: Economy,` |
| A17 | `HouseState::new(name: InternedId, side_index: u8, country: Option<InternedId>, is_human: bool, credits: i32, tech_level: i32)` — the 6-arg test ctor | house_state.rs `pub fn new(` … `tech_level: i32,` |
| A18 | `Economy { credits, spent_credits, harvested_credits, purifier_count }` derives `#[derive(Debug, Clone, Default, PartialEq, Eq)]` (NO serde) — serde ADDED. `add_credits`/`spend`/`available` exist; `spend` drains `credits` + tracks `spent_credits`; `available()` returns `credits` | economy.rs `#[derive(Debug, Clone, Default, PartialEq, Eq)]` above `pub struct Economy {` + `pub fn spend(&mut self, amount: i32) -> i32 {` |
| A19 | `ProductionState` derives Serialize/Deserialize; `#[serde(skip)] pub factory_shadow: FactoryRegistry`; `pub active_producer_by_owner: BTreeMap<InternedId, BTreeMap<ProductionCategory, u64>>`; `pub next_enqueue_order: u64`. The un-skip anchor is the `#[serde(skip)]` line | production_types.rs `#[serde(skip)]` immediately above `pub factory_shadow: FactoryRegistry,` |
| A20 | `BuildQueueItem` (`#[derive(Debug, Clone, Serialize, Deserialize)]`) carries `owner, type_id, queue_category, state, total_base_frames: u32, remaining_base_frames: u32, progress_carry: u64, enqueue_order: u64` — `remaining_base_frames`/`progress_carry` are REMOVED from the hash; `total_base_frames`/`enqueue_order` STAY | production_types.rs `pub remaining_base_frames: u32,` + `pub progress_carry: u64,` + `pub enqueue_order: u64,` |
| A21 | `ProductionCategory` is `Building < Defense < Infantry < Vehicle < Aircraft`, `#[default] Building`, derives `Ord, Hash, Serialize, Deserialize` (NO Ship — D2) | production_types.rs `pub enum ProductionCategory {` + `#[default]` + `Building, Defense, Infantry, Vehicle, Aircraft,` |
| A22 | `PRODUCTION_RATE_SCALE: u64 = 1_000_000` (the PPM scale 1.0) | production_types.rs `pub(super) const PRODUCTION_RATE_SCALE: u64 = 1_000_000;` |
| A23 | `hash_production` per-item fold hashes `owner, type_id, queue_category, state, total_base_frames, remaining_base_frames, progress_carry, enqueue_order`; then `ready_by_owner`; then `active_producer_by_owner`; then `next_enqueue_order`; then resources/ore/terrain | world_hash.rs `item.remaining_base_frames.hash(hasher);` + `item.progress_carry.hash(hasher);` + the `active_producer_by_owner` block `for (owner, categories) in &self.production.active_producer_by_owner {` |
| A24 | `active_producer_by_owner` IS hashed (the §3.4 KEEP block) — verified-live a still-written authoritative producer-focus field; NOT removed by P5b | world_hash.rs `for (owner, categories) in &self.production.active_producer_by_owner {` + `sid.hash(hasher);` |
| A25 | `hash_houses` folds `house.credits` (+ side_index/is_human/defeat/win/loss/counts/tech_level/rally/base) but NOT `economy`. KEEP `house.credits`; ADD the economy statistics sub-fold | world_hash.rs `fn hash_houses(&self, hasher: &mut impl Hasher) {` + `house.credits.hash(hasher);` |
| A26 | `SNAPSHOT_VERSION == 17` + the pin test `snapshot_version_is_17_in_shadow_phase` (`assert_eq!(super::SNAPSHOT_VERSION, 17)`) — bumped to 18, the test flipped to `snapshot_version_is_18` | snapshot.rs `const SNAPSHOT_VERSION: u32 = 17;` + `fn snapshot_version_is_17_in_shadow_phase() {` |
| A27 | `refresh_production_shadow(&mut self, rules: Option<&RuleSet>)` does `refresh_economy_shadow(rules)` then `let mut registry = std::mem::take(&mut self.production.factory_shadow);` then `rebuild_shadow`/`rebuild_shadow_no_rules` then swap back. P5b replaces the rebuild with `reconcile_from_queues` (POSITION unchanged: tick tail) | world/mod.rs `pub(crate) fn refresh_production_shadow(&mut self, rules: Option<&RuleSet>) {` + `let mut registry = std::mem::take(&mut self.production.factory_shadow);` |
| A28 | `refresh_economy_shadow` writes `house.economy.credits = house.credits;` (line ~979) — the mirror line DELETED (§3.3); its purifier-count pass STAYS untouched | world/mod.rs `house.economy.credits = house.credits;` |
| A29 | Phase-7 production block head is `spawned_entities \|= production::tick_production_with_overlay_registry(self, rules, height_map, path_grid, overlay_registry, tick_ms);` — `step_all` is inserted at the head, BEFORE this call | world/mod.rs `spawned_entities \|= production::tick_production_with_overlay_registry(` |
| A30 | **DRIFT-CORRECTED:** `debug_assert_production_shadow(&self)` takes NO `rules` param (the P5a `None`-fallback form shipped, NOT the threaded form). It calls `self.debug_assert_factory_step_matches_legacy(None); // P5a`. P5b retires/repurposes that P5a call (§T11) | world/mod.rs `pub(crate) fn debug_assert_production_shadow(&self) {` + `self.debug_assert_factory_step_matches_legacy(None); // P5a` |
| A31 | the house tail is `self.run_late_region(rules, path_grid, height_map, tick_ms, execute_tick, &mut spawned_entities,);`; the tick tail then runs `refresh_mission_shadow()` -> `refresh_production_shadow(rules)` -> asserts -> `let state_hash = self.state_hash();` (C1: `step_all` must run BEFORE `run_late_region`) | world/mod.rs `self.run_late_region(` … `self.refresh_production_shadow(rules);` … `let state_hash = self.state_hash();` |
| A32 | upfront charge `*credits_entry_for_owner(sim, owner) -= obj.cost;` in `enqueue_by_type`, gated by `if obj.cost <= 0 \|\| owner_credits < obj.cost { ... }` (the affordability gate STAYS; the `-=` is RETIRED) | production_queue.rs `*credits_entry_for_owner(sim, owner) -= obj.cost;` + `if obj.cost <= 0 \|\| owner_credits < obj.cost {` |
| A33 | `cancel_by_type_for_owner` uses `.rev()` last-match + `*credits_entry_for_owner(sim, owner) += obj.cost.max(0);` full refund; `cancel_completed_building_from_ready` also `.rev()` + full refund. RETIRED -> route to `cancel_one` (partial refund) | production_queue.rs `.rev()` (in `cancel_by_type_for_owner`) + `*credits_entry_for_owner(sim, owner) += obj.cost.max(0);` |
| A34 | `credits_entry_for_owner` auto-creates `HouseState::new(key, 0, None, true, STARTING_CREDITS, 10)` if the house is missing (the hashed-state fabrication hazard) — made non-fabricating after both charge paths are off it | production_queue.rs `crate::sim::house_state::HouseState::new(key, 0, None, true, STARTING_CREDITS, 10),` |
| A35 | the frames timer half: `advance_queue_item(front, tick_ms, progress_rate);` + `front.remaining_base_frames` decrement + completion -> `ready_by_owner` push + `pop_completed_front(...)`. RETIRE the frames/charge half; the placement/spawn geometry STAYS | production_queue.rs `advance_queue_item(front, tick_ms, progress_rate);` + `pop_completed_front(sim, owner_id, queue_category, done.enqueue_order);` |
| A36 | `effective_time_to_build_frames_for_type(sim, rules, owner, type_id, base_frames: u32) -> u32` is a LIVE reader of `base_frames` (the sidebar ETA basis = `total_base_frames`) — so `total_base_frames` STAYS hashed (U-QFRAMES: KEEP) | production_tech.rs `pub(in crate::sim) fn effective_time_to_build_frames_for_type(` + `base_frames: u32,` |
| A37 | `production_category_for_object(obj: &ObjectType) -> ProductionCategory` is `pub(super)` in production_tech.rs (the routing source `category_for_object` delegates to) | production_tech.rs `pub(super) fn production_category_for_object(` |
| A38 | `matching_factory_count_for_owner(entities, rules, owner, category, interner) -> u32` is the full-store rescan; `owner_power_percentage_ppm(sim, owner)` is the live power-ratio source `owner_effective_production_speed_ppm` already uses (the producer's `power_ratio_ppm` input) | production_tech.rs `fn matching_factory_count_for_owner(` + `let power_pct_ppm = owner_power_percentage_ppm(sim, owner);` |
| A39 | the legacy build-time DRIFT family bakes `let base_value = (cost * speed_x1000 * 9 / 10000) as i32;` (the REFUTED ×0.9) — NOT reused; retired once the producer + registry-key count replace it | production_tech.rs `let base_value = (cost * speed_x1000 * 9 / 10000) as i32;` |
| A40 | `pub use self::factory::{ build_step_time, category_for_object, BuildEligibility, BuildStepTimeInputs, CancelOutcome, Factory, FactoryRegistry, FactoryView, PendingObject, SpecialItem, StepOutcome, PRODUCTION_STEPS, STEP_RATE_MAX, STEP_RATE_MIN };` — `step_all` added if it needs a pub re-export (it is a method on the re-exported `FactoryRegistry`, so no new name needed) | production/mod.rs `pub use self::factory::{` |
| A41 | `Simulation::object_type(&self, type_ref: InternedId, rules: &RuleSet) -> Option<&ObjectType>`; `ObjectType` carries `pub cost: i32`, `pub build_time_multiplier_x1000: u64`, `pub wall: bool`, `pub category: ObjectCategory` (the producer inputs) | factory.rs (existing) `sim.object_type(front.type_id, r)` + object_type.rs `pub cost: i32,` |
| A42 | the world-test helpers `empty_rules()`, `queued_item(owner, ty, cat, state, total, remaining, order)`, `insert_queue(..)`, plus `sim.advance_tick(&[], Some(&rules), &heights, None, None, 67)` exist (the P5a tests use them) | production_shadow_tests.rs `fn empty_rules() -> RuleSet {` + `sim.advance_tick(&[], Some(&rules), &heights, None, None, 67)` |

**Facts pinned from the v2-verified study (no re-decompile this slice — cited, not re-decoded):**
- **C1:** the engine's per-tick factory loop (registration order) precedes the house loop; the Rust
  analog is `step_all` (registry sweep in `insertion_seq` = temporal `enqueue_order` order) BEFORE
  `run_late_region`. Folded into the one 17->18 bump.
- **C3/C15:** per-step charge telescopes to exact cost; the upfront charge is the DRIFT removed here.
- **C4:** under-funded -> the factory sets `on_hold`, spends nothing that step (strict-`<` stall).
- **C5:** `set_rate` consumes the build-step TOTAL; the per-step CDTimer cadence is `step_rate_frames`.
- **C6:** FIFO queue-of-record; first-match cancel (the `.rev()` last-match is DRIFT).
- **C7:** queue advance is bound to the successful DELIVERY command (post-delivery StartNextQueued),
  AND the post-AbandonProduction auto-StartNextQueued binds at the same seam.
- **C8:** active-abandon refunds the SPENT portion (`original_balance - balance`), not full cost.
- **C12:** completion suspends with the object STILL attached; delivery clears it.

---

## B. Files touched (summary) — world_hash.rs + snapshot.rs ARE in this list now (the flip)

| File | Change | Task |
|---|---|---|
| `src/sim/production/factory.rs` | M1: replace `rebuild_shadow*`/`rebuild_shadow_inner`/`remaining_balance_after` with `reconcile_from_queues` (§2.2). M2: add `Serialize, Deserialize` to `Factory`/`FactoryRegistry`/`PendingObject`/`SpecialItem`; REMOVE the `next_insertion_seq` + `seq_carry` FIELDS (D1). M3: add `step_all(&mut self, houses, rules)` + the `step_timer` cadence + the `set_rate`-from-producer call + the cancel route helper. M4: the delivery/cancel `start_next_queued` bind helper. `advance_one_step`/`cancel_one`/`start_next_queued`/`set_rate`/`build_step_time` BODIES UNCHANGED. | T1, T2, T6, T8, T10 |
| `src/sim/economy.rs` | add `Serialize, Deserialize` to the `Economy` derive (keep `credits` but it is no longer the wallet/hashed — re-pointed to the per-sweep shim) | T3 |
| `src/sim/house_state.rs` | remove `#[serde(skip)]` from `economy` | T4 |
| `src/sim/production/production_types.rs` | remove `#[serde(skip)]` from `factory_shadow`; NO `active_producer_by_owner` change (§3.4 KEEP) | T4 |
| `src/sim/world/world_hash.rs` | `hash_production`: REMOVE `item.remaining_base_frames` + `item.progress_carry`; KEEP `item.total_base_frames` (A36) + the `active_producer_by_owner` block (§3.4); ADD `hash_factory_registry` (iter_insertion_ordered fold). `hash_houses`: ADD the `economy.{spent_credits, harvested_credits, purifier_count}` sub-fold; KEEP `house.credits` | T5 |
| `src/sim/world/mod.rs` (CO-EDITED — minimal, text-anchored) | M1: replace `refresh_production_shadow` body (reconcile-not-rebuild; POSITION unchanged). M3: DELETE the mirror line `house.economy.credits = house.credits;`; add `self.production.factory_shadow.step_all(&mut self.houses, rules);` at Phase-7 head before `tick_production_with_overlay_registry` (C1 fold). T11: retire/repurpose the P5a `debug_assert_factory_step_matches_legacy(None)` call | T1, T7, T9, T11 |
| `src/sim/production/production_queue.rs` | M3: retire upfront `-= obj.cost` (keep the affordability gate); route `.rev()`+full-refund cancel to `cancel_one`; make `credits_entry_for_owner` non-fabricating. M4: retire the frames-timer/charge + completion->`ready_by_owner`/`pop_completed_front` half (keep spawn/placement) | T7, T8, T10 |
| `src/sim/production/production_tech.rs` | retire the legacy build-time family + `matching_factory_count_for_owner` rescan once the producer + registry-key count replace them | T8 |
| `src/sim/snapshot.rs` | `SNAPSHOT_VERSION` 17->18 + history comment; flip `snapshot_version_is_17_in_shadow_phase` -> `snapshot_version_is_18` | T5 |
| `src/sim/world/production_shadow_tests.rs` (+ factory.rs `mod tests`) | the §D test list + the §D.7 stale-test inversions | T2..T11 (per-task) |

**NOT touched:** miner/combat/movement/unit_post (concurrent session owns those);
`place_ready_building` placement geometry; the P5c replay harness.
**world/mod.rs is CO-EDITED — anchor every edit on the quoted TEXT, never a line number; keep each edit
to the minimal hunk (replace one body, delete one line, add one call line, retire one call).**

---

## C. The serde / un-skip / field-removal / hash-delta ledger (exact, for cross-check)

**serde derives ADDED (exactly five types):**
- `economy.rs`: `Economy` — `#[derive(Debug, Clone, Default, PartialEq, Eq)]` -> `#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]`.
- `factory.rs`: `Factory`, `FactoryRegistry`, `PendingObject` — append `serde::Serialize, serde::Deserialize` to each `// NO serde in P1-P3` derive line.
- `factory.rs`: `SpecialItem` — `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` -> add `serde::Serialize, serde::Deserialize` (the 3 variants serialize distinctly; manual `Default` stays).

**serde derives NOT added (transient return/input types — NO serde):** `StepOutcome`, `CancelOutcome`,
`BuildEligibility`, `BuildStepTimeInputs`, `FactoryView` (borrow projection). None is stored.

**`#[serde(skip)]` REMOVED (exactly two):**
- `house_state.rs`: the `#[serde(skip)]` above `pub economy: Economy,` (A16).
- `production_types.rs`: the `#[serde(skip)]` above `pub factory_shadow: FactoryRegistry,` (A19).

**FIELDS REMOVED (D1 — exactly two, on `FactoryRegistry`):** `next_insertion_seq: u64,` +
`seq_carry: BTreeMap<(InternedId, ProductionCategory), u64>,`. Removing them also requires deleting the
`new_carry`/`self.seq_carry = new_carry` writes — which vanish anyway when `rebuild_shadow_inner` is
replaced by `reconcile_from_queues` (T1). After removal, `FactoryRegistry` serializes as just the
`factories` BTreeMap.

**HASH DELTA (`world_hash.rs`):**
- `hash_production` per-item fold: **REMOVE** `item.remaining_base_frames` + `item.progress_carry`.
  **KEEP** `item.total_base_frames` (A36 reader) + `owner, type_id, queue_category, state, enqueue_order`.
- `hash_production`: **KEEP** `ready_by_owner`, the `active_producer_by_owner` block (§3.4), `next_enqueue_order`, and all resource/ore/terrain folds. **ADD** a `hash_factory_registry` call.
- `hash_houses`: **ADD** `economy.{spent_credits, harvested_credits, purifier_count}`; **do NOT** hash `economy.credits` (§3.3); **KEEP** `house.credits`.

The double-hash audit (design §3.3) proves the only value-redundancy is registry `object.type_id` /
`insertion_seq` vs the queue front's `type_id` / `enqueue_order` — DETERMINISTIC (the registry is
reconciled one-way from the queue-of-record) and HASH-SAFE; no single authority's field is hashed twice.

---

## D. The reconcile (M1) — the load-bearing PERSIST/SEED design

`reconcile_from_queues` REPLACES the per-tick `std::mem::take` + `rebuild_shadow` clobber. Two arms:

- **PERSIST arm** — a factory whose front `(type_id, enqueue_order)` is unchanged keeps its
  authoritative `progress`/`balance`/`step_timer`/`on_hold`/`suspended` UNTOUCHED; only the FIFO tail +
  the `manual` pause bridge are refreshed.
- **SEED arm** — no factory, or the front changed identity (delivery/cancel advanced the FIFO front to a
  higher `enqueue_order`) -> seed `progress=0, balance=original_balance=full_cost` ONCE.

Identity test = `(object.type_id == front.type_id) && (insertion_seq == front.enqueue_order)`. Because
`enqueue_order` is strictly monotonic, a delivered-then-restarted `(owner, category)` gets a higher
front -> the test fails -> SEED re-arms (the C7 destroy-recreate analog). `manual = (front.state ==
Paused)` is the ONE `front.state` read kept (the §2.3 pause bridge). The reconcile NEVER reads
`front.state` for progress/hold/suspend (those are authoritative-in-registry now). The reconcile stays
at the TICK TAIL (POSITION unchanged); `step_all` runs at the NEXT tick's Phase-7 head.

---

## E. Linear tasks (M1 -> M2 -> M3 -> M4)

### P5b-T1 (M1 INVERT) — reconcile-not-rebuild; registry persists; PROVEN hash-neutral first

**Goal:** kill the `std::mem::take` rebuild clobber and persist progress across ticks, with serde STILL
SKIPPED so the hash does not move yet. This isolates the single riskiest mechanical change (losing the
rebuild) from the hash move.

**File (EDIT):** `src/sim/production/factory.rs` — REPLACE `rebuild_shadow` / `rebuild_shadow_no_rules` /
`rebuild_shadow_inner` / `remaining_balance_after` (A12) with `reconcile_from_queues`. Keep the same
`pub(crate)` visibility on the new entry points so `refresh_production_shadow` can call them.

Replace the four functions (anchor on `pub(crate) fn rebuild_shadow(` … through the close of
`rebuild_shadow_inner`, and the standalone `fn remaining_balance_after`) with:

```rust
    /// Reconcile the registry from the legacy `queues_by_owner` (the queue-of-record),
    /// PRESERVING the authoritative progress of an unchanged active build (the PERSIST
    /// arm) and seeding a fresh one ONCE when the front changes identity (the SEED arm).
    /// This REPLACES the per-tick rebuild-from-scratch clobber: once the registry is
    /// authoritative its `progress`/`balance`/`step_timer`/`on_hold`/`suspended` must
    /// survive across ticks. READ-ONLY w.r.t. all hashed state except the registry it owns.
    ///
    /// `rules == None` (the cost-free tail) resolves `full_cost` to 0, exactly like the
    /// retired `rebuild_shadow_no_rules`.
    pub(crate) fn reconcile_from_queues(
        &mut self,
        sim: &crate::sim::world::Simulation,
        rules: Option<&RuleSet>,
    ) {
        use crate::sim::production::production_types::BuildQueueState as S;

        // Track which keys still have a non-empty queue this tick (membership).
        let mut live_keys: std::collections::BTreeSet<(InternedId, ProductionCategory)> =
            std::collections::BTreeSet::new();

        for (&owner, queues) in &sim.production.queues_by_owner {
            for (&category, queue) in queues {
                let Some(front) = queue.front() else {
                    continue; // empty category: no factory
                };
                let key = (owner, category);
                live_keys.insert(key);
                let seq = front.enqueue_order; // temporal insertion_seq (D1; the P5a mint)

                // The tail (FIFO behind the active object) + the pause bridge are refreshed
                // every reconcile regardless of arm.
                let tail: VecDeque<InternedId> =
                    queue.iter().skip(1).map(|item| item.type_id).collect();
                let paused = matches!(front.state, S::Paused);

                // The front item is the active object when Building/NoFunds/Done; a Paused
                // front is still the active (held) object; a Queued-only front has none.
                let has_object =
                    matches!(front.state, S::Building | S::NoFunds | S::Done | S::Paused);

                match self.factories.get_mut(&key) {
                    Some(f)
                        if has_object
                            && f.object.as_ref().map(|o| o.type_id) == Some(front.type_id)
                            && f.insertion_seq == seq =>
                    {
                        // PERSIST arm: same active build as last tick. Do NOT touch
                        // progress/balance/step_timer/on_hold/suspended. Refresh only the
                        // FIFO tail + the pause bridge.
                        f.queue = tail;
                        f.manual = paused;
                    }
                    _ => {
                        // SEED arm: a new/changed active build begins. Seed ONCE from cost.
                        let full_cost = match rules {
                            Some(r) => sim
                                .object_type(front.type_id, r)
                                .map(|o| o.cost.max(0))
                                .unwrap_or(0),
                            None => 0,
                        };
                        let object = if has_object {
                            Some(PendingObject { type_id: front.type_id, entity_id: None })
                        } else {
                            None
                        };
                        let factory = Factory {
                            owner,
                            category,
                            progress: 0,
                            step_rate_frames: 0,
                            step_timer: 0,
                            balance: full_cost,
                            original_balance: full_cost,
                            object,
                            on_hold: false,
                            suspended: false,
                            manual: paused,
                            special: SpecialItem::NoneNeg1,
                            queue: tail,
                            insertion_seq: seq,
                        };
                        self.factories.insert(key, factory);
                    }
                }
            }
        }

        // Drop factories whose (owner, category) no longer has a non-empty queue.
        self.factories.retain(|key, _| live_keys.contains(key));
    }
```

**File (EDIT):** `src/sim/world/mod.rs` — replace the `refresh_production_shadow` BODY (A27; POSITION at
the tick tail unchanged). Anchor on `let mut registry = std::mem::take(&mut self.production.factory_shadow);`:

```rust
    pub(crate) fn refresh_production_shadow(&mut self, rules: Option<&RuleSet>) {
        self.refresh_economy_shadow(rules);
        // Take the registry out so reconcile can borrow `&self` while writing the
        // registry; swap it back after. Reconcile reads `queues_by_owner` (the
        // queue-of-record), PERSISTING an unchanged active build's authoritative
        // progress (NOT re-deriving it from cost every tick — the P5b inversion).
        let mut registry = std::mem::take(&mut self.production.factory_shadow);
        registry.reconcile_from_queues(self, rules);
        self.production.factory_shadow = registry;
    }
```

**Note (mint-adjacent factory.rs unit tests):** the `rebuild_shadow_inner`-driven tests are gone with
the function; the surviving `registry_iter_insertion_ordered_not_map_order` hand-sets `insertion_seq`
(unaffected). Any `mod tests` reference to `rebuild_shadow`/`remaining_balance_after` must be deleted or
ported to `reconcile_from_queues` — grep `remaining_balance_after` + `rebuild_shadow` in `factory.rs`
`mod tests` at impl time and remove the now-dangling tests (`remaining_balance_ladder_matches_stepper`,
`cost25_ladder_sums_to_exactly_25` exercise the stepper directly and stay; only the rebuild-named ones go).

**Add the M1 guard test** to `production_shadow_tests.rs`:

```rust
/// M1 (C2): the registry persists across ticks WITHOUT a rebuild-from-scratch, AND
/// (serde still skipped) the state_hash is STILL bit-identical — proving the load-bearing
/// PERSIST arm in isolation, BEFORE any serde/hash move.
#[test]
fn factory_registry_persists_across_ticks_hash_neutral() {
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

    // Manually advance the registry progress (the authoritative path is not live yet),
    // then reconcile again with the SAME front: the PERSIST arm must NOT reset it.
    {
        let mut reg = std::mem::take(&mut sim.production.factory_shadow);
        if let Some(f) = reg.iter_insertion_ordered().first() {
            let key = (f.owner, f.category);
            let _ = key; // (registry map is private; mutate via a fresh reconcile-preserving probe)
        }
        sim.production.factory_shadow = reg;
    }
    // A second reconcile with an unchanged front leaves progress untouched (PERSIST arm).
    sim.refresh_production_shadow(Some(&rules));
    assert_eq!(
        before,
        sim.state_hash(),
        "M1: reconcile (registry still serde-skip) must not move the hash"
    );
}
```

> **Confirm at impl time:** the registry map is private, so the test cannot poke `progress` directly.
> The load-bearing assertion is the hash-neutral round-of-reconcile; if a white-box progress-persist
> assertion is wanted, add a `#[cfg(test)] pub(crate) fn debug_progress(&self, owner, category) ->
> Option<u16>` accessor on `FactoryRegistry` (read-only, test-only) and assert progress is unchanged
> across two reconciles. Keep it read-only.

**Verification (cargo is the T12 foreground pass):**
`cargo test -p vera20k factory_registry_persists_across_ticks_hash_neutral`
+ the P1-P5a no-hash family must STILL pass at this point (`snapshot_version_is_17_in_shadow_phase`
still 17 — serde not yet added).

---

### P5b-T2 (M2 ROUND-TRIP) — serde on the factory types + DROP next_insertion_seq/seq_carry

**File (EDIT):** `src/sim/production/factory.rs`.

(i) Add serde to four types (anchor each `// NO serde in P1-P3` derive line):

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PendingObject {
```
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SpecialItem {
```
```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Factory {
```
```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FactoryRegistry {
    factories: BTreeMap<(InternedId, ProductionCategory), Factory>,
}
```

(ii) REMOVE the `next_insertion_seq` + `seq_carry` fields (D1) — the new `FactoryRegistry` body is the
single `factories` map shown above. Grep `next_insertion_seq` + `seq_carry` across `factory.rs` and
delete every remaining reference (there are none left after T1 replaced `rebuild_shadow_inner`; confirm).

> **D1 note (REVISES STUDY §6.4 / resolves CONCERN-2):** after the P5a temporal mint, `insertion_seq ==
> front.enqueue_order`, so the counter is never the ordering source and is dead. Dropping the FIELDS (not
> just un-hashing) is the locked call. The hashed/serialized temporal-ordering source is the per-queue
> `enqueue_order` (already hashed via `hash_production`), which P5a proved hash-neutral.

**Verification:** `cargo check -p vera20k` (serde compiles; nothing un-skipped yet, so the hash has not
moved — the un-skip is T4).

---

### P5b-T3 (M2) — serde on Economy

**File (EDIT):** `src/sim/economy.rs` — anchor on the `#[derive(Debug, Clone, Default, PartialEq, Eq)]`
above `pub struct Economy`:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Economy {
```

**Verification:** `cargo check -p vera20k`.

---

### P5b-T4 (M2) — un-skip economy + factory_shadow (the round-trip goes live)

**File (EDIT):** `src/sim/house_state.rs` — remove the `#[serde(skip)]` above `pub economy: Economy,`
(A16). Update the doc-comment on the field from "non-serialized and non-hashed until the authority flip"
to "serialized + hashed (statistics) as of the authority flip".

**File (EDIT):** `src/sim/production/production_types.rs` — remove the `#[serde(skip)]` above
`pub factory_shadow: FactoryRegistry,` (A19). Do NOT touch `active_producer_by_owner` (§3.4 KEEP).

**Verification:** `cargo check -p vera20k`. At this point `economy` + `factory_shadow` serialize but are
NOT yet in the hash (T5 folds them) — so the snapshot round-trips them but the state_hash is unchanged
until T5. (The P3/P4 `*_does_not_change_state_hash` tests still pass; `snapshot_roundtrip_ignores_shadow`,
if present, now FAILS — it is intentionally inverted in T5/§D.7.)

---

### P5b-T5 (M2) — the hash fold + the 17->18 bump

**File (EDIT):** `src/sim/world/world_hash.rs`.

(i) `hash_production` per-item fold — REMOVE two lines (anchor on them), KEEP `total_base_frames`:

```rust
                    item.owner.hash(hasher);
                    item.type_id.hash(hasher);
                    item.queue_category.hash(hasher);
                    item.state.hash(hasher);
                    item.total_base_frames.hash(hasher); // KEEP: sidebar ETA basis (effective_time_to_build_frames_for_type)
                    item.enqueue_order.hash(hasher);
```
(the `item.remaining_base_frames.hash(hasher);` and `item.progress_carry.hash(hasher);` lines are
DELETED — the frames timer is retired; progress lives in `Factory`.)

(ii) ADD the registry fold — insert a `self.hash_factory_registry(hasher);` call into `hash_production`
(anchor right after `self.production.next_enqueue_order.hash(hasher);`, BEFORE the resource folds), and
add the helper to the same `impl` block:

```rust
        self.production.next_enqueue_order.hash(hasher);
        self.hash_factory_registry(hasher); // P5b: the authoritative factory registry
```
```rust
    /// Hash the authoritative factory registry in the deterministic temporal sweep
    /// order (`iter_insertion_ordered`, by `insertion_seq` = front `enqueue_order`) — the
    /// SAME order `step_all` charges in, so the fold order is part of the hash contract.
    /// Explicit-field folding (NOT `#[derive(Hash)]`) so `SpecialItem`'s three states +
    /// the Option presence tags fold distinctly, consistent with the rest of this file.
    fn hash_factory_registry(&self, hasher: &mut impl Hasher) {
        for f in self.production.factory_shadow.iter_insertion_ordered() {
            f.owner.hash(hasher);
            (f.category as u8).hash(hasher);
            f.insertion_seq.hash(hasher);
            f.progress.hash(hasher);
            f.step_rate_frames.hash(hasher);
            f.step_timer.hash(hasher);
            f.balance.hash(hasher);
            f.original_balance.hash(hasher);
            match &f.object {
                Some(o) => {
                    1u8.hash(hasher);
                    o.type_id.hash(hasher);
                    match o.entity_id {
                        Some(e) => {
                            1u8.hash(hasher);
                            e.hash(hasher);
                        }
                        None => 0u8.hash(hasher),
                    }
                }
                None => 0u8.hash(hasher),
            }
            f.on_hold.hash(hasher);
            f.suspended.hash(hasher);
            f.manual.hash(hasher);
            match f.special {
                crate::sim::production::SpecialItem::NoneNeg1 => 0u8.hash(hasher),
                crate::sim::production::SpecialItem::NoneZero => 1u8.hash(hasher),
                crate::sim::production::SpecialItem::Item(v) => {
                    2u8.hash(hasher);
                    v.hash(hasher);
                }
            }
            (f.queue.len() as u64).hash(hasher);
            for t in &f.queue {
                t.hash(hasher);
            }
        }
    }
```

> **Confirm at impl time:** the `Factory` fields are private only at the registry-map level;
> `iter_insertion_ordered` returns `&Factory` and all the fields above are `pub` (A3), so the fold reads
> them directly. `category` is `Copy` (`ProductionCategory` derives `Copy`), so `f.category as u8` is fine.

(iii) `hash_houses` — ADD the economy statistics sub-fold (anchor right after `house.credits.hash(hasher);`),
do NOT hash `economy.credits`:

```rust
            house.credits.hash(hasher);
            house.economy.spent_credits.hash(hasher);
            house.economy.harvested_credits.hash(hasher);
            house.economy.purifier_count.hash(hasher);
```

**File (EDIT):** `src/sim/snapshot.rs` — bump the version + add a history comment (anchor on
`const SNAPSHOT_VERSION: u32 = 17;`):

```rust
// Bumped 17 -> 18: Factory/Economy authority flip — the factory registry + the per-house
// economy statistics are now serialized + hashed; the frames-timer per-item fields
// (remaining_base_frames / progress_carry) are removed from the hash (progress lives in
// Factory); next_insertion_seq + seq_carry fields removed (insertion_seq == front
// enqueue_order); the C1 factory-step-before-house-tail ordering lock is folded in.
const SNAPSHOT_VERSION: u32 = 18;
```

Flip the pin test (anchor on `fn snapshot_version_is_17_in_shadow_phase() {`):

```rust
    /// The authority flip (P5b) is the FIRST hashed-state change: the version bumped
    /// 17 -> 18. This pins it so a later accidental bump is caught.
    #[test]
    fn snapshot_version_is_18() {
        assert_eq!(super::SNAPSHOT_VERSION, 18);
    }
```

**Add the M2 hash/round-trip tests** to `production_shadow_tests.rs` (full bodies in §D #1/#3/#4/#5).

**Verification:** `cargo test -p vera20k snapshot_version_is_18 production_authoritative_hash_includes_factory_fields legacy_progress_carry_removed_from_hash snapshot_roundtrip_factory_registry factory_insertion_seq_equals_front_enqueue_order`.

---

### P5b-T6 (M3 CHARGE-FLIP) — step_all (the authoritative per-step charge) + the cadence + the rate swap

**File (EDIT):** `src/sim/production/factory.rs` — add `step_all` as a method on `FactoryRegistry`
(impl block), driving `advance_one_step` against the REAL `house.credits` (via a per-sweep `Economy`
shim) in `insertion_seq` (temporal) order, gating the cadence on `step_timer`, and setting the rate from
the `build_step_time` producer. `advance_one_step`/`set_rate`/`build_step_time` BODIES UNCHANGED.

```rust
    /// The authoritative per-tick factory sweep (the P5b charge flip). Walks the
    /// registry in `iter_insertion_ordered` (temporal `insertion_seq`) order — the same
    /// order the hash folds in — and, for each factory whose per-step cadence timer has
    /// expired this tick, (re)computes the rate from the `build_step_time` producer and
    /// charges ONE step against the owner's REAL wallet (`house.credits`). Reproduces the
    /// engine's per-tick factory loop (C1) walked before the house tail.
    ///
    /// The wallet: `house.credits` is THE single wallet (one debit per step). The
    /// per-sweep `Economy` shim is loaded from `house.credits` at entry and stored back
    /// after, so `advance_one_step`'s `&mut Economy` contract is honored unchanged and
    /// `economy.spent_credits` accumulates. `economy.credits` is a transient shim, never
    /// the authority, never hashed.
    pub fn step_all(
        &mut self,
        houses: &mut std::collections::BTreeMap<InternedId, crate::sim::house_state::HouseState>,
        rules: Option<&RuleSet>,
        sim_inputs: &StepInputs<'_>,
    ) {
        // Sweep order = temporal insertion_seq (strictly monotonic enqueue_order -> no
        // ties -> total order -> deterministic).
        let mut order: Vec<(u64, InternedId, ProductionCategory)> = self
            .factories
            .iter()
            .map(|(&(o, c), f)| (f.insertion_seq, o, c))
            .collect();
        order.sort_by_key(|&(seq, _, _)| seq);

        for (_, owner, category) in order {
            let Some(f) = self.factories.get_mut(&(owner, category)) else {
                continue;
            };
            // No active object -> nothing to step (queue-only factory).
            if f.object.is_none() {
                continue;
            }
            let Some(house) = houses.get_mut(&owner) else {
                continue; // defensive: a vanished house is skipped (NO auto-create)
            };

            // (Rate) recompute the per-step rate from the producer each cadence the
            // factory could step (the producer is pure + cheap; a superset of the
            // engine's per-change RecalcAllRates -> the VALUE matches whenever inputs do).
            if let (Some(r), Some(obj_id)) =
                (rules, f.object.as_ref().map(|o| o.type_id))
            {
                if let Some(inputs) = sim_inputs.build_step_time_inputs(r, owner, category, obj_id) {
                    f.set_rate(build_step_time(&inputs));
                }
            }

            // (Cadence) one step per `step_rate_frames` frames (the engine CDTimer). A
            // rate of 0 (no object / not-yet-rated) is treated as "step this tick" so a
            // freshly-armed build is not stuck; the next reconcile/step rates it.
            if f.step_timer > 0 {
                f.step_timer -= 1;
                continue; // not this tick
            }

            // (Charge) one authoritative step against the real wallet via the shim.
            let mut wallet = std::mem::take(&mut house.economy);
            wallet.credits = house.credits; // load the authoritative balance
            let _ = f.advance_one_step(&mut wallet);
            house.credits = wallet.credits; // store the debited balance back (ONE wallet)
            house.economy = wallet; // keep spent_credits / etc.

            // Reset the cadence timer to the freshly-computed rate (reset-to-rate; see
            // U-STEPRATE). A completed factory zeroed its own step_timer already.
            f.step_timer = f.step_rate_frames.saturating_sub(1);
        }
    }
```

The producer inputs are gathered through a small read-only adapter so `factory.rs` does not duplicate the
power/factory-count plumbing (which lives in `production_tech.rs`). Add the adapter as a borrow over the
data the producer needs:

```rust
    /// Read-only inputs adapter for the build-step producer, so `step_all` can build
    /// `BuildStepTimeInputs` without `factory.rs` re-implementing the power/factory-count
    /// plumbing (that lives in `production_tech`). A borrow over `Simulation`.
    pub struct StepInputs<'a> {
        pub sim: &'a crate::sim::world::Simulation,
    }

    impl<'a> StepInputs<'a> {
        pub fn build_step_time_inputs(
            &self,
            rules: &RuleSet,
            owner: InternedId,
            category: ProductionCategory,
            type_id: InternedId,
        ) -> Option<BuildStepTimeInputs> {
            let obj = self.sim.object_type(type_id, rules)?;
            let owner_name = self.sim.interner.resolve(owner).to_string();
            Some(BuildStepTimeInputs {
                cost: obj.cost.max(0),
                build_time_bonus_ppm: PRODUCTION_RATE_SCALE, // stock YR 1.0 (U-BONUS)
                build_time_multiplier_ppm: obj.build_time_multiplier_x1000.max(1) * 1_000,
                power_ratio_ppm: crate::sim::production::production_tech::owner_power_percentage_ppm(
                    self.sim, &owner_name,
                ),
                low_power_penalty_modifier_ppm: rules.production.low_power_penalty_modifier_ppm,
                min_clamp_ppm: rules.production.min_low_power_production_speed_ppm,
                max_clamp_ppm: rules.production.max_low_power_production_speed_ppm,
                multiple_factory_ppm: rules.production.multiple_factory_ppm,
                factory_count: self.factory_count_for(rules, &owner_name, category, obj),
                is_wall: obj.category == crate::rules::object_type::ObjectCategory::Building
                    && obj.wall,
                wall_build_speed_ppm: (rules.production.wall_build_speed_coefficient.max(0.0) as f64
                    * PRODUCTION_RATE_SCALE as f64) as u64,
            })
        }
        fn factory_count_for(/* ... */) -> u32 { /* registry-key count or the rescan; see below */ }
    }
```

> **Confirm at impl time (three plumbing points):**
> 1. **`owner_power_percentage_ppm` visibility.** It is a private `fn` in `production_tech.rs` (A38). Widen
>    it to `pub(in crate::sim::production)` (a no-behavior visibility widen) so `StepInputs` can call it,
>    OR add a thin `pub(in crate::sim::production) fn owner_power_ratio_ppm(sim, owner) -> u64` wrapper.
>    Do NOT fork the power math.
> 2. **`factory_count`.** The design retires `matching_factory_count_for_owner`'s full-store rescan in
>    favor of "the registry key count" — but the registry only holds ONE key per (owner, category) (it
>    collapses physical factories), so the registry-key count is always 1 and is NOT the engine's
>    per-category BUILDING count. The faithful `factory_count` is the number of matching factory BUILDINGS
>    the owner has (the `(n-1)` MultipleFactory loop count). KEEP calling `matching_factory_count_for_owner`
>    (A38) here (widen it to `pub(in crate::sim::production)`); the "retire the rescan" line in the design
>    is DEFERRED to P5d (when the registry tracks building counts). FLAG this as U-FACTORYCOUNT.
> 3. **`StepInputs` placement / borrow.** `step_all` takes `&mut self.houses` AND the reconcile already
>    `std::mem::take`s the registry, so `step_all` must run on a taken registry (see T7 call site) to
>    avoid double-borrowing `&self.production.factory_shadow` while holding `&mut self.houses`. `StepInputs
>    { sim: &*self }` borrows `&Simulation` immutably — which CONFLICTS with `&mut self.houses` if `houses`
>    is reached through `self`. Resolve by passing `&mut self.houses` and `&self.production` SEPARATELY, or
>    by gathering the producer inputs into an owned `Vec<(key, BuildStepTimeInputs)>` BEFORE the `&mut
>    houses` loop (split-borrow). The owned-prepass is the cleaner borrow story; prefer it. The plan's
>    `step_all` signature is illustrative — the implementer picks the split-borrow shape that compiles, with
>    the SAME observable behavior (rate-then-cadence-then-charge in insertion_seq order).

**Add the cadence test** (§D #14) + the no-double-charge test (§D #11) here; full bodies in §D.

**Verification:** `cargo test -p vera20k step_cadence_respects_step_rate_frames single_wallet_charged_once_no_double_debit stall_on_no_funds_holds` (note `step_all` is not yet CALLED from advance_tick — these test `step_all` directly on a constructed registry/houses, mirroring the factory.rs charge-stepper tests).

---

### P5b-T7 (M3) — wire step_all at Phase-7 head (C1 fold) + delete the mirror line

**File (EDIT):** `src/sim/world/mod.rs`.

(i) DELETE the mirror line (A28) in `refresh_economy_shadow` — anchor on it:

```rust
        for (id, house) in self.houses.iter_mut() {
            // (the `house.economy.credits = house.credits;` mirror line is DELETED — the
            //  economy wallet is no longer mirrored; house.credits is the one wallet.)
            house.economy.purifier_count = purifiers.get(id).copied().unwrap_or(0);
        }
```

(ii) ADD the `step_all` call at the Phase-7 production head, BEFORE
`tick_production_with_overlay_registry` (A29) and unconditionally before `run_late_region` (C1). Use the
`std::mem::take` borrow pattern (mirror `refresh_production_shadow`):

```rust
            // Phase 7, FIRST production step — the authoritative factory sweep (C1:
            // factories step before the house tail run_late_region). The tail reconcile
            // (refresh_production_shadow) at tick N prepared the registry; step_all at
            // tick N+1 charges it; the spawn/placement pass below then advances queues.
            {
                let mut registry = std::mem::take(&mut self.production.factory_shadow);
                let inputs = crate::sim::production::StepInputs { sim: &*self };
                registry.step_all(&mut self.houses, rules, &inputs);
                self.production.factory_shadow = registry;
            }
            spawned_entities |= production::tick_production_with_overlay_registry(
                self,
                rules,
                height_map,
                path_grid,
                overlay_registry,
                tick_ms,
            );
```

> **Borrow-check confirm at impl time:** `StepInputs { sim: &*self }` borrows `&self` while
> `registry.step_all(&mut self.houses, ...)` needs `&mut self.houses`. The registry is already TAKEN OUT
> (so `factory_shadow` is not double-borrowed), but `&*self` (immutable) and `&mut self.houses` (mutable,
> through `self`) still conflict. Resolve with the split-borrow / owned-input-prepass noted in T6: gather
> the per-factory `BuildStepTimeInputs` into an owned `Vec` from `&self` FIRST, drop the `&self` borrow,
> THEN run the `&mut self.houses` charge loop. The plan's literal `StepInputs { sim: &*self }` is
> illustrative; the implementer makes it compile via the owned-prepass with identical behavior.

(iii) Add the C1 ordering test (§D #15).

**Verification:** `cargo test -p vera20k c1_factories_step_before_house_tail`.

---

### P5b-T8 (M3) — retire the upfront charge + route cancel to cancel_one + non-fabricating getter + retire the build-time family

**File (EDIT):** `src/sim/production/production_queue.rs`.

(i) RETIRE the upfront charge (A32) — KEEP the affordability gate; remove the `-=`:

```rust
    if obj.cost <= 0 || owner_credits < obj.cost {
        return false; // the can-afford-to-START gate stays (C20 begin precondition)
    }
    // (the `*credits_entry_for_owner(sim, owner) -= obj.cost;` upfront debit is RETIRED;
    //  the per-step advance_one_step charges over the build against house.credits.)
```

(ii) Route the cancel COMMAND to the registry `cancel_one` (A13/A33). `cancel_by_type_for_owner` and
`cancel_completed_building_from_ready` STOP doing the `.rev()` full refund; instead resolve the
(owner, category), call `factory_shadow.cancel_one(owner_id, category, type_id, &mut wallet)` against the
`house.credits` shim (the partial refund, C8), AND remove the matching `queues_by_owner` item to keep the
queue-of-record in sync, then `start_next_queued` if a tail remains (the C7 abandon seam, wired in T10).

> **Confirm at impl time:** `cancel_by_type_for_owner` currently owns BOTH the queued-tail removal AND the
> active-refund. Since the registry `cancel_one` now owns refund + first-match precedence, the cleanest
> shape is: (a) compute `category` from the type, (b) take the registry out, (c) `cancel_one(...)` against
> a `house.credits` shim, (d) mirror the resulting queue mutation into `queues_by_owner` (remove the
> first-match tail item OR pop the active front on abandon), (e) swap the registry back. Keep the
> player-facing command entry point (`cancel_by_type_for_owner`'s callers) unchanged; only its body flips.
> This is the largest single behavioral hunk — review carefully against the §D #12 partial-refund test.

(iii) Make `credits_entry_for_owner` non-fabricating (A34) — after both charge paths are off it, the
remaining callers are spawn-fail refunds. Replace the auto-create with a `houses.get_mut(&owner)` that
NO-OPs (or returns `Option`) if the house is absent:

```rust
    // (the auto-create `HouseState::new(key, 0, None, true, STARTING_CREDITS, 10)` is
    //  RETIRED — fabricating an is_human=true house mutates HASHED state. Return the
    //  existing house's credits slot, or None/no-op when the house is absent.)
```

> **Confirm at impl time:** `credits_entry_for_owner` returns `&mut i32`. A non-fabricating variant must
> change the signature to `Option<&mut i32>` (and update its callers to `if let Some(slot) = ...`), OR
> the callers move to `houses.get_mut(&owner).map(|h| &mut h.credits)`. Trace every caller (grep
> `credits_entry_for_owner`) and confirm none still relies on the fabrication. This is a CORRECTNESS fix
> (the fabrication mutates `hash_houses`), not cleanup.

**File (EDIT):** `src/sim/production/production_tech.rs` — retire the legacy build-time DRIFT family
(A39) + the `matching_factory_count_for_owner` rescan ONLY once nothing reads them. Per U-FACTORYCOUNT
(T6 note 2), `matching_factory_count_for_owner` is STILL needed by `step_all`'s `factory_count`; so it
STAYS (widened visibility). The `(cost * speed_x1000 * 9 / 10000)` x0.9 base (A39) and any rate-domain
build-time fn no longer read by the sidebar (confirm `effective_time_to_build_frames_for_type` and its
helpers are the surviving readers; A36) are retired. **If a sidebar reader still depends on the legacy
family, KEEP it and FLAG U-BUILDTIME-READER** — do not break the sidebar ETA to satisfy a retirement.

**Verification:** `cargo test -p vera20k no_upfront_charge_at_enqueue cancel_one_partial_refund_to_house_credits`.

---

### P5b-T9 (M3) — pause bridge guard test (no code change beyond the reconcile bridge from T1)

The pause bridge (`f.manual = (front.state == Paused)`) is already in `reconcile_from_queues` (T1) and
`advance_one_step`'s ARMED GATE already idles a `manual` factory (A8). T9 is the GUARD test only (§D #13).

**Verification:** `cargo test -p vera20k pause_front_maps_to_manual_idle`.

---

### P5b-T10 (M4 DELIVERY+C1) — bind start_next_queued at the delivery + cancel commit

**File (EDIT):** `src/sim/production/production_queue.rs` — in
`tick_production_with_overlay_registry`, REPLACE the completion->`ready_by_owner`/`pop_completed_front`
queue-advance half (A35) with the C7 delivery bind. The PLACEMENT/SPAWN geometry STAYS.

On a successful delivery (a unit spawned, or a building placed via `place_ready_building`):
1. clear the producing factory's `object` (`f.object = None`) — leave `entity_id` None (U-ENTITYID; the
   hash folds it as the 0u8 presence tag);
2. call `f.start_next_queued()` (A10) — front-pop the next FIFO type into a fresh `object`, `progress=0`;
   the next reconcile's SEED arm seeds the new `balance` from cost (the front `enqueue_order` is higher,
   so the identity test fails -> SEED);
3. for BUILDINGS, completion->`ready_by_owner` push STAYS as the "awaiting placement" signal (C12); the
   delivery bind fires on `place_ready_building` success. For UNITS, delivery is the spawn (immediate);
   the bind fires at spawn success, replacing `pop_completed_front`.

The post-AbandonProduction auto-`start_next_queued` (C7) binds at the SAME seam — the cancel path (T8 ii)
calls `start_next_queued` after the refund if a queue tail remains.

> **Confirm at impl time:** the registry is the authority for `object`/`progress` now, so the delivery
> bind must mutate the registry factory (take it out, find `(owner, category)`, clear object +
> `start_next_queued`, swap back) AND remove/advance the matching `queues_by_owner` front so the next
> reconcile sees the advanced queue-of-record. Keep the spawn geometry (`find_spawn_selection_*`,
> `spawn_object`, helipad reserve, rally-move, `place_ready_building`) untouched. This is the second-largest
> behavioral hunk; review against §D #10 (`reconcile_seed_arm_re_arms_on_new_front`).

**Verification:** `cargo test -p vera20k reconcile_seed_arm_re_arms_on_new_front`.

---

### P5b-T11 (M4) — retire/repurpose the P5a inversion assert + the determinism guard

**File (EDIT):** `src/sim/world/mod.rs` — the P5a `debug_assert_factory_step_matches_legacy(None)` call
(A30) compared the model vs the legacy charge. Post-flip there is no legacy charge to compare, so the
comparison is vacuous. Two clean options (confirm at impl time):
- **(preferred) repurpose** it into an authoritative-invariant assert: drop the (B)/(C) model-vs-legacy
  drive (the model IS the authority now), KEEP (A) the sweep-order-equals-temporal-order check (still a
  valid invariant on the registry) + a new "balance never exceeds original_balance, progress in 0..=54"
  sanity assert. Rename to `debug_assert_factory_invariants`.
- **(fallback) retire** the call line from `debug_assert_production_shadow` and delete the fn, if the
  concurrent session's edits make the repurpose collide.

Either way, `debug_assert_production_shadow(&self)` stays `(&self)` (A30 — it never took `rules`).

**Add the determinism guard** (§D #6) — the flip's near-term lockstep proof.

**Verification:** `cargo test -p vera20k factory_flip_determinism_over_scripted_commands`.

---

### P5b-T12 — full-suite verify (separate FOREGROUND pass the human runs)

Per the build-discipline memory (don't bury slow cargo inside a background workflow), the human runs:
- `cargo test -p vera20k` — read the literal `test result:` line. The P5b set (§D #1-#17) must pass.
- `cargo test -p vera20k snapshot_version_is_18` — confirms `SNAPSHOT_VERSION == 18`.
- Confirm the stale-test inversions (§D.7) are PRESENT (replaced, not silently deleted) and the P1-P4
  shadow-no-hash tests that ASSUMED serde-skip are gone/inverted (they would now fail by design).
- `git diff --stat` shows `world_hash.rs` + `snapshot.rs` CHANGED (the flip) and miner/combat/movement/
  unit_post UNCHANGED (concurrent session).

---

## D. The P5b test list (full bodies) — each tied to a contract Cn

> Append to `src/sim/world/production_shadow_tests.rs` unless a `factory.rs` `mod tests` placement is noted.
> Reuse `empty_rules()`, `queued_item`, `insert_queue`, `HouseState::new` (A42). Confirm the import line
> carries `BuildQueueState, ProductionCategory` (it does, per the P5a tests).

**#1 `production_authoritative_hash_includes_factory_fields` (C12/C15) [T5]** — mutating each newly-hashed
field moves the hash:

```rust
#[test]
fn production_authoritative_hash_includes_factory_fields() {
    fn sim_with_midbuild() -> Simulation {
        let mut sim = Simulation::new();
        let rules = empty_rules();
        let owner = sim.interner.intern("Americans");
        sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
        let ty = sim.interner.intern("GRIZZLY");
        insert_queue(
            &mut sim, owner, ProductionCategory::Vehicle,
            queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1),
        );
        sim.refresh_production_shadow(Some(&rules));
        sim
    }
    // A white-box mutator over the registry (add a #[cfg(test)] pub(crate) accessor that
    // returns &mut Factory for (owner, category), or rebuild the registry with the field
    // pre-set). Each mutation must change state_hash().
    let base = sim_with_midbuild().state_hash();
    for mutate in [
        |f: &mut crate::sim::production::Factory| f.progress += 1,
        |f: &mut crate::sim::production::Factory| f.balance += 1,
        |f: &mut crate::sim::production::Factory| f.step_timer += 1,
        |f: &mut crate::sim::production::Factory| f.on_hold = !f.on_hold,
        |f: &mut crate::sim::production::Factory| f.suspended = !f.suspended,
        |f: &mut crate::sim::production::Factory| f.original_balance += 1,
        |f: &mut crate::sim::production::Factory| f.step_rate_frames += 1,
        |f: &mut crate::sim::production::Factory| f.manual = !f.manual,
        |f: &mut crate::sim::production::Factory| f.special = crate::sim::production::SpecialItem::NoneZero,
    ] {
        let mut sim = sim_with_midbuild();
        sim.production.factory_shadow.test_mutate_first(&mutate); // #[cfg(test)] accessor
        assert_ne!(base, sim.state_hash(), "a newly-hashed Factory field must move the hash");
    }
    // economy statistics:
    for mutate in [
        |e: &mut crate::sim::economy::Economy| e.spent_credits += 1,
        |e: &mut crate::sim::economy::Economy| e.harvested_credits += 1,
        |e: &mut crate::sim::economy::Economy| e.purifier_count += 1,
    ] {
        let mut sim = sim_with_midbuild();
        let owner = sim.interner.intern("Americans");
        mutate(&mut sim.houses.get_mut(&owner).unwrap().economy);
        assert_ne!(base, sim.state_hash(), "a hashed economy statistic must move the hash");
    }
}
```

> **Confirm at impl time:** add a `#[cfg(test)] pub(crate) fn test_mutate_first(&mut self, f: &dyn Fn(&mut
> Factory))` to `FactoryRegistry` that applies `f` to the first `iter_insertion_ordered` factory (the
> registry map is private). Read-only-elsewhere; test-only.

**#2 `snapshot_version_is_18` (versioning) [T5]** — in `snapshot.rs` tests (body shown in T5).

**#3 `snapshot_roundtrip_factory_registry` (C15) [T5]** — a mid-build registry survives save->load AND a
post-load reconcile (PERSIST arm):

```rust
#[test]
fn snapshot_roundtrip_factory_registry() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    let ty = sim.interner.intern("GRIZZLY");
    insert_queue(
        &mut sim, owner, ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1),
    );
    sim.refresh_production_shadow(Some(&rules));
    // Advance a few authoritative steps so progress/balance are non-trivial.
    let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
    for _ in 0..3 { sim.advance_tick(&[], Some(&rules), &heights, None, None, 67); }
    let before = sim.state_hash();
    let blob = sim.to_snapshot_bytes().expect("serialize"); // confirm the exact snapshot API at impl time
    let mut loaded = Simulation::from_snapshot_bytes(&blob).expect("deserialize");
    loaded.rebuild_caches_after_load(); // confirm this does NOT re-derive the authoritative registry
    assert_eq!(before, loaded.state_hash(), "registry round-trips bit-identically");
    // The first post-load reconcile (PERSIST arm) must NOT perturb the loaded progress.
    loaded.refresh_production_shadow(Some(&rules));
    assert_eq!(before, loaded.state_hash(), "post-load reconcile leaves the loaded build untouched");
}
```

> **Confirm at impl time:** the exact serialize/deserialize entry points (grep `GameSnapshot` /
> `to_snapshot` / `from_snapshot` in snapshot.rs) and that `rebuild_caches_after_load` does NOT call the
> retired rebuild (it cannot — `rebuild_shadow` is gone; confirm it does not call `reconcile_from_queues`
> in a way that re-seeds, which it would not for an unchanged front).

**#4 `legacy_progress_carry_removed_from_hash` (frames timer retired) [T5]** — mutating
`remaining_base_frames`/`progress_carry` on a queue item does NOT move the hash:

```rust
#[test]
fn legacy_progress_carry_removed_from_hash() {
    let mut sim = Simulation::new();
    let owner = sim.interner.intern("Americans");
    let ty = sim.interner.intern("GRIZZLY");
    insert_queue(
        &mut sim, owner, ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1),
    );
    let before = sim.state_hash();
    {
        let q = sim.production.queues_by_owner.get_mut(&owner).unwrap()
            .get_mut(&ProductionCategory::Vehicle).unwrap();
        let item = q.front_mut().unwrap();
        item.remaining_base_frames = item.remaining_base_frames.wrapping_add(7);
        item.progress_carry = item.progress_carry.wrapping_add(99);
    }
    assert_eq!(before, sim.state_hash(), "retired frames-timer fields are out of the hash");
}
```

**#5 `factory_insertion_seq_equals_front_enqueue_order` (D1/C6/C1) [T5]** — REPLACES the dropped
`registry_next_insertion_seq_is_serialized_and_hashed`; the already-passing P5a test (keep it verbatim
from the P5a suite — `insertion_seq == front.enqueue_order` after reconcile; the temporal-vs-enum-sort
fixture). It already lives in `production_shadow_tests.rs`; confirm it still passes under
`reconcile_from_queues` (the seq source is identical: `front.enqueue_order`).

**#6 `factory_flip_determinism_over_scripted_commands` (lockstep determinism) [T11]** — two sims, same
scripted stream, identical per-tick hash sequence:

```rust
#[test]
fn factory_flip_determinism_over_scripted_commands() {
    fn run() -> Vec<u64> {
        let mut sim = Simulation::new();
        let rules = empty_rules();
        let a = sim.interner.intern("Americans");
        let b = sim.interner.intern("Russians");
        sim.houses.insert(a, HouseState::new(a, 0, None, true, 1_000_000, 10));
        sim.houses.insert(b, HouseState::new(b, 1, None, true, 1_000_000, 10));
        // Two owners, two categories, a same-tick two-Begin, and a cancel-one.
        for (owner, ty, cat, order) in [
            (a, "GRIZZLY", ProductionCategory::Vehicle, 1u64),
            (a, "BEAG", ProductionCategory::Aircraft, 2),
            (b, "GRIZZLY", ProductionCategory::Vehicle, 3),
        ] {
            let t = sim.interner.intern(ty);
            insert_queue(&mut sim, owner, cat,
                queued_item(owner, t, cat, BuildQueueState::Building, 54, 54, order));
        }
        let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
        (0..120).map(|i| {
            // cancel one of A's builds partway, then let the rest run to completion+delivery.
            if i == 10 {
                crate::sim::production::production_queue::cancel_by_type_for_owner(
                    &mut sim, &rules, "Americans", "BEAG"); // confirm the exact cancel entry point
            }
            sim.advance_tick(&[], Some(&rules), &heights, None, None, 67);
            sim.state_hash()
        }).collect()
    }
    assert_eq!(run(), run(), "the authority flip preserves lockstep determinism across the bump");
}
```

> **Confirm at impl time:** the exact cancel command entry point + that 120 ticks is enough for ≥1
> completion+delivery at the test rate (the empty-rules producer rate). Adjust the tick count if the
> build does not complete; the load-bearing assertion is run==run.

**Lane guards (full bodies):**

**#11 `single_wallet_charged_once_no_double_debit` (§3.3/C15) [T6]** — over a full build, total debit to
`house.credits` == full cost; `economy.spent_credits == full cost`; `economy.credits` never the source:

```rust
#[test]
fn single_wallet_charged_once_no_double_debit() {
    use crate::sim::production::{FactoryRegistry, Factory, PendingObject, ProductionCategory, StepInputs};
    // Build a registry with one armed factory + a funded house; drive step_all to completion.
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    let ty = sim.interner.intern("GRIZZLY");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    insert_queue(&mut sim, owner, ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 54, 1));
    sim.refresh_production_shadow(Some(&rules)); // SEED arm: balance = full_cost
    let start = sim.houses[&owner].credits;
    let full_cost = sim.object_type(ty, &rules).map(|o| o.cost.max(0)).unwrap_or(0);
    // Drive many ticks of advance_tick (step_all is wired at Phase-7 head as of T7).
    let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
    for _ in 0..(54 * 60) { sim.advance_tick(&[], Some(&rules), &heights, None, None, 67); }
    let debited = start - sim.houses[&owner].credits;
    assert_eq!(debited, full_cost, "exactly one full-cost debit to house.credits over the build");
    assert_eq!(sim.houses[&owner].economy.spent_credits, full_cost, "spent_credits accumulates the cost");
}
```

> **Confirm at impl time:** the empty-rules `GRIZZLY` cost (use a known-cost type, or read it via
> `object_type`). The tick budget `54*60` covers the cadence; reduce if the rate is fast.

**#12 `cancel_one_partial_refund_to_house_credits` (C8) [T8]** — mid-build cancel refunds
`original_balance - balance` to `house.credits` (NOT full cost), first-match, removes the queue item:

```rust
#[test]
fn cancel_one_partial_refund_to_house_credits() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    let ty = sim.interner.intern("GRIZZLY");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    insert_queue(&mut sim, owner, ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 54, 1));
    sim.refresh_production_shadow(Some(&rules));
    let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
    for _ in 0..200 { sim.advance_tick(&[], Some(&rules), &heights, None, None, 67); } // partway
    let credits_before = sim.houses[&owner].credits;
    crate::sim::production::production_queue::cancel_by_type_for_owner(&mut sim, &rules, "Americans", "GRIZZLY");
    let refunded = sim.houses[&owner].credits - credits_before;
    let full_cost = sim.object_type(ty, &rules).map(|o| o.cost.max(0)).unwrap_or(0);
    assert!(refunded > 0 && refunded < full_cost, "partial refund (spent portion), NOT full cost (the .rev() DRIFT)");
    // the queue-of-record item is gone too.
    assert!(sim.production.queues_by_owner.get(&owner)
        .and_then(|c| c.get(&ProductionCategory::Vehicle))
        .map_or(true, |q| q.is_empty()), "the cancelled build left the queue-of-record");
}
```

**#13 `pause_front_maps_to_manual_idle` (§2.3) [T9]** — a `Paused` front -> `manual` -> no step, progress
held; unpause resumes:

```rust
#[test]
fn pause_front_maps_to_manual_idle() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    let ty = sim.interner.intern("GRIZZLY");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    insert_queue(&mut sim, owner, ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Paused, 54, 54, 1));
    sim.refresh_production_shadow(Some(&rules));
    let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
    let credits_before = sim.houses[&owner].credits;
    for _ in 0..120 { sim.advance_tick(&[], Some(&rules), &heights, None, None, 67); }
    assert_eq!(sim.houses[&owner].credits, credits_before, "a paused build does not charge");
    // Unpause: flip the front to Building, reconcile, and confirm it now charges.
    {
        let q = sim.production.queues_by_owner.get_mut(&owner).unwrap()
            .get_mut(&ProductionCategory::Vehicle).unwrap();
        q.front_mut().unwrap().state = BuildQueueState::Building;
    }
    for _ in 0..120 { sim.advance_tick(&[], Some(&rules), &heights, None, None, 67); }
    assert!(sim.houses[&owner].credits < credits_before, "unpause resumes charging");
}
```

**#14 `step_cadence_respects_step_rate_frames` (C5) [T6]** — a build with `step_rate_frames > 1` does NOT
advance every tick:

```rust
#[test]
fn step_cadence_respects_step_rate_frames() {
    // Drive step_all directly with a constructed registry whose factory has a known
    // step_rate_frames > 1, and assert progress advances once per `step_rate_frames` calls.
    // (Construct via a #[cfg(test)] registry builder OR a sequence of refresh+advance.)
    // Load-bearing: over R*K calls, progress advances ~K times for rate R, NOT R*K.
    // Full body parameterized at impl time on the empty-rules producer rate; assert the
    // ratio, not an exact count (the rate depends on cost/power inputs).
}
```

> **Confirm at impl time:** the cleanest construction is a `#[cfg(test)] pub(crate)` registry builder on
> `FactoryRegistry` (one armed factory with a forced `step_rate_frames`) + a funded `HouseState`, then
> count `progress` deltas over N `step_all` calls. Assert progress advanced strictly fewer than N times
> for a rate > 1. Keep it integer/deterministic.

**#15 `c1_factories_step_before_house_tail` (C1 fold) [T7]** — `step_all`'s charge is applied before
`run_late_region`:

```rust
#[test]
fn c1_factories_step_before_house_tail() {
    // run_late_region runs defeat detection; a factory that just charged must have done so
    // BEFORE the tail. Simplest observable: seed a build, run one tick, and assert the
    // charge (house.credits delta) is reflected in the same tick's final state_hash — i.e.
    // step_all ran within Phase 7, ahead of the tail + the hash. A structural check:
    // assert the credits debit occurred on the tick step_all ran (not deferred to the tail).
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    let ty = sim.interner.intern("GRIZZLY");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    insert_queue(&mut sim, owner, ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 54, 1));
    sim.refresh_production_shadow(Some(&rules)); // SEED
    let before = sim.houses[&owner].credits;
    let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
    sim.advance_tick(&[], Some(&rules), &heights, None, None, 67);
    assert!(sim.houses[&owner].credits <= before, "the first step charged within Phase 7 (before the tail)");
}
```

> **Confirm at impl time:** a stronger ordering assert (step_all strictly before run_late_region) needs a
> debug observation hook; if one exists (a tick-ordered trace), assert the relative order. Otherwise the
> charge-within-the-tick observable above is the available proof; the §D #6 determinism guard covers the
> ordering's stability.

**#16 `no_upfront_charge_at_enqueue` (C3) [T8]** — enqueuing does NOT debit `house.credits`:

```rust
#[test]
fn no_upfront_charge_at_enqueue() {
    let mut sim = Simulation::new();
    let rules = empty_rules(); // confirm empty_rules has a buildable GRIZZLY, else use a rules fixture with cost
    let owner_name = "Americans";
    let owner = sim.interner.intern(owner_name);
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    let before = sim.houses[&owner].credits;
    let ok = crate::sim::production::production_queue::enqueue_by_type(&mut sim, &rules, owner_name, "GRIZZLY");
    assert!(ok, "the affordability gate still permits an affordable START");
    assert_eq!(sim.houses[&owner].credits, before, "enqueue does NOT debit upfront (the per-step charge does)");
}
```

> **Confirm at impl time:** `enqueue_by_type` needs a rules fixture where `GRIZZLY` is buildable + has a
> cost ≤ credits. If `empty_rules()` lacks it, build a minimal `RuleSet` with one buildable vehicle (the
> P5a `category_for_object_matches_rtti_table` test shows the INI-fixture pattern).

**#17 `stall_on_no_funds_holds` (C4) [T6]** — an underfunded house sets `on_hold`, spends nothing that step:

```rust
#[test]
fn stall_on_no_funds_holds() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    let ty = sim.interner.intern("GRIZZLY");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 0, 10)); // 0 credits
    insert_queue(&mut sim, owner, ProductionCategory::Vehicle,
        queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 54, 1));
    sim.refresh_production_shadow(Some(&rules)); // SEED: balance = full_cost, 0 funds
    let heights: std::collections::BTreeMap<(u16, u16), u8> = std::collections::BTreeMap::new();
    sim.advance_tick(&[], Some(&rules), &heights, None, None, 67);
    assert_eq!(sim.houses[&owner].credits, 0, "a stalled step spends nothing");
    assert_eq!(sim.houses[&owner].economy.spent_credits, 0, "nothing accumulated");
    // (the factory's on_hold is set; assert via a #[cfg(test)] read accessor if available)
}
```

> Note: with a cost-free type the first charge is 0 and never stalls; use a non-zero-cost `GRIZZLY` so
> `full_cost > 0` and the strict-`<` stall fires at 0 funds. Confirm the empty-rules cost or use a fixture.

**Determinism / extra guards #8 (`factory_registry_persists_across_ticks_hash_neutral`, M1, T1 body),
#9 (`progress_persists_across_ticks_not_re_derived`), #10 (`reconcile_seed_arm_re_arms_on_new_front`)** —
#8 body is in T1; #9/#10 bodies parameterized on the registry accessor (confirm at impl time), each
asserting the PERSIST/SEED arm behavior described in §D of the design.

### D.7 STALE tests — intentional INVERSIONS (list explicitly so they don't read as regressions)

The P3/P4 `*_does_not_change_state_hash` family + `snapshot_roundtrip_ignores_shadow` (if present)
ASSUMED serde-skip. Under the flip they INVERT — the registry now DOES change the hash and DOES
round-trip. They are REPLACED by #1 / #3 (the inverse assertions). At impl time, grep
`does_not_change_state_hash` + `roundtrip_ignores_shadow` + `factory_flip_prep_does_not_change_state_hash`
in the test files and REMOVE/REPLACE each with the noted inverse (do NOT leave them asserting the old
no-hash contract — they would fail by design). List the removed names in the commit message as intentional
inversions: `factory_advance_step_does_not_change_state_hash`,
`factory_cancel_one_does_not_change_state_hash`, `factory_flip_prep_does_not_change_state_hash`,
`snapshot_roundtrip_ignores_shadow`, `snapshot_version_is_17_in_shadow_phase` (-> `snapshot_version_is_18`).

### DROPPED (per §3.4 override)

`legacy_active_producer_removed_from_hash` — DROPPED. `active_producer_by_owner` is a still-written
authoritative producer-focus binding (A24); removing it from the hash is a hash hole. Deferred to the
producer-focus retirement slice. All three judges concur.

---

## E. UNKNOWN / UNCHECKED (DRIFT-default) — what P5b leaves open

- **U-FACTORYCOUNT (NEW, surfaced at plan time).** The design's "retire `matching_factory_count_for_owner`'s
  full-store rescan, use the registry key count" is WRONG as stated: the registry collapses physical
  factories to ONE key per (owner, category), so its key count is always 1 — NOT the engine's per-category
  BUILDING count that drives the MultipleFactory `(n-1)` loop. **P5b KEEPS `matching_factory_count_for_owner`
  (widened visibility) as the `factory_count` source;** the rescan retirement is DEFERRED to P5d (when the
  registry tracks building counts). DEFAULT: keep the rescan; do not break the MF count.
- **active_producer_by_owner removal — DEFERRED / DRIFT-resolved (§3.4).** Still-written authoritative
  field; KEPT hashed; its removal + the `legacy_active_producer_removed_from_hash` test deferred.
- **U-SHIP (D2 deferred) — KNOWN DRIFT, NOT fixed.** No `Ship` ProductionCategory; naval collapses into
  `Vehicle` (A21). A house owning a War Factory + Naval Yard collapses two engine factories into one
  Vehicle key -> diverges the MF `factory_count` + same-frame completion order on water maps. **Frequency:
  every naval water-map match.** PINNED by `category_for_object_naval_collapses_to_vehicle_documented`; Ship
  is a focused follow-up slice (its own hash-key change + version bump).
- **U-STEPRATE — per-step cadence reset semantics.** The `step_timer` countdown (decrement, step at 0,
  reset to `step_rate_frames`) is the faithful cadence, but the exact reset (reset-to-rate vs
  reset-to-rate-minus-overshoot) was not re-decoded. DEFAULT: reset-to-rate (`step_timer =
  step_rate_frames - 1`); flag for a C5-adjacent spot-check if the §D #6 determinism test surfaces a
  cadence drift.
- **U-QFRAMES — `total_base_frames` KEPT hashed.** VERIFIED this session: `effective_time_to_build_frames_for_type`
  (A36) is a live reader (sidebar ETA basis). So `total_base_frames` STAYS hashed (REJECTS D-SUBSTRATE's
  removal). DEFAULT: keep hashed.
- **U-BUILDTIME-READER — legacy build-time family retirement.** The `(cost * speed_x1000 * 9 / 10000)`
  x0.9 base (A39) is retired ONLY if no surviving reader depends on it. Confirm
  `effective_time_to_build_frames_for_type` + its helpers (A36) are the surviving sidebar path; if any
  legacy build-time fn still backs the sidebar, KEEP it and flag. DEFAULT: do not break the sidebar ETA.
- **U-ENTITYID — `object.entity_id` at delivery — left None in P5b.** Whether the factory remembers its
  delivered child is a P5b choice with no current consumer; left None (hashed as the 0u8 presence tag).
  Flag if a later slice (radio link to the exiting vehicle) wants the back-reference.
- **U-ORDER — same-frame two-Begin command-dispatch order — UNCHECKED.** `enqueue_order` makes two
  same-tick Begins distinct (total SWEEP order) -> deterministic charge. Whether the command-execute
  dispatch assigns `enqueue_order` in the SAME relative order the engine does for two same-frame Begins
  from different players is the P5c U-ORDER spot-check. DEFAULT: surface, not asserted-equal.
- **U-AFFORD — affordability read == write wallet.** `economy.available() == credits` and `spend()` both
  touch the one `credits` shim loaded from `house.credits` -> holds by construction in Rust. The
  engine-side proof was asserted in the study but not decompiled. DEFAULT: holds in Rust; engine-side proof
  incomplete.

---

## F. SHARED-CHECKOUT note + the P5c clean seam

- **Stage only your hunks.** `world/mod.rs` is CO-EDITED by a concurrent session — `git add -p` (hunk-stage)
  the four P5b hunks (the `refresh_production_shadow` body, the deleted mirror line, the Phase-7 `step_all`
  call, the P5a-assert retire/repurpose); do NOT stage unrelated miner/combat/movement/unit_post hunks.
  Anchor every edit on the quoted TEXT (the file shifts).
- **If `cargo check` fails in files you did NOT modify**, assume it is the concurrent session's
  in-progress work — do NOT fix/revert/stash it; continue your own hunks or wait (CLAUDE.md parallel-sessions
  rule).
- **P5c (the replay/parity acceptance gate) is a SEPARATE later slice — leave the seam, do NOT implement
  it.** It replays a recorded command stream twice AND against a pre-flip baseline for bit-identical
  per-tick `state_hash`, plus `economy_conservation_over_replay` (C15) + the pre-flip-vs-post-flip
  observable-output equivalence (the x0.9-free producer correction documented as the ONE intended
  difference). Reuses the existing replay harness. The §D #6 determinism guard is the near-term proxy; P5c
  ratifies the flip.
- **P5d (full `queues_by_owner` retirement into `Factory.queue`) — clean seam, NOT implemented.** Moves
  `enqueue_order` storage into the registry, retires the `BuildQueueItem` mirror, erases the registry-key
  count limitation (U-FACTORYCOUNT) + the registry<->mirror value-redundancy. Its own hash-key change +
  version bump (18->19).

---

*End of P5b plan. Twelve linear tasks authored as the four design micro-steps (M1 INVERT / M2 ROUND-TRIP /
M3 CHARGE-FLIP / M4 DELIVERY+C1), each building green before the next. The riskiest task is T8 (retire the
upfront charge + route cancel to `cancel_one` + the non-fabricating getter): it is the largest behavioral
hunk, it touches the player-facing cancel command, and it must keep the safe-order invariant (the real
`step_all` charge is live from T6 BEFORE the upfront `-=` is removed). Two live-tree DRIFTS corrected vs the
context anchor map: A11 (`rebuild_shadow_inner` ALREADY mints `front.enqueue_order` — the P5a mint shipped;
P5b replaces the whole body) and A30 (`debug_assert_production_shadow(&self)` takes NO `rules` param — the
P5a `None`-fallback form shipped). One NEW unknown surfaced: U-FACTORYCOUNT (the registry key count is NOT
the engine per-category building count; keep the rescan, defer to P5d). All v2 corrections honored; D1/D2
designed-within; `active_producer_by_owner` KEPT hashed (§3.4 override). P5c left a clean seam.*
