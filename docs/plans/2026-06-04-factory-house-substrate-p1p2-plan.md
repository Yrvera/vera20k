<!--
Provenance: authored 2026-06-04 from the APPROVED design
  docs/plans/2026-06-04-factory-house-substrate-p1p2-design.md (D2 substrate-fit winner,
  FIT §6.3 = option (a)), grounded in the v2-verified study
  docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md (C1-C20, §6, §8 P1/P2).
House style mirrored from docs/plans/2026-06-01-mission-radio-substrate-implementation-plan.md.
Status: DRAFTED, not approved or executed. Review (/review-plan) before implementing.
Scope: P1 + P2 ONLY — additive, #[serde(skip)] shadow, ZERO state_hash change, NO authority
  flip, NO SNAPSHOT_VERSION bump (stays 17). P3-P9 are seams only.
-->

# Factory/House Substrate — P1+P2 Shadow Implementation Plan

> Linear path: **P1-T1 → P1-T2 → P1-T3 → P1-T4 → P2-T1 → P2-T2 → P2-T3 → P2-T4 → P2-T5 → P2-T6 → P2-T7**.
> Every task builds green (`cargo check -p vera20k`) before the next. The hash-neutrality
> tests (`*_does_not_change_state_hash`, `snapshot_version_is_17_*`) are the contract gate:
> if any of them fails after a task, STOP — a `#[serde(skip)]` or no-derive discipline broke.
>
> **#1 invariant preserved:** `sim/economy.rs` and `sim/production/factory.rs` depend only on
> `std` + `sim/` (intern, house_state, entity store, rules data through `&Simulation`/`&RuleSet`);
> NEVER on render/ui/sidebar/audio/net.
>
> **No-hash contract (the whole point of P1+P2):** both new fields are `#[serde(skip)]` AND
> their value-types carry **no `Serialize`/`Deserialize` derive** in P1+P2. `world_hash.rs` is
> NOT touched. `SNAPSHOT_VERSION` stays **17** (`snapshot.rs:24`). The 17→18 bump is P5, out of scope.

---

## A. Verified preconditions (live reads this session — quote file:line)

| # | Fact the plan relies on | Verified at |
|---|---|---|
| A1 | `HouseState` derives `Default` (so a new `economy` field defaults with no `new()` change needed for the field itself) | `house_state.rs:17` `#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]` |
| A2 | `HouseState::new` is a hand-written ctor enumerating every field — a new field on `HouseState` MUST be added here too (Default-derive does not cover `new()`) | `house_state.rs:51-77` |
| A3 | `hash_houses` hashes `credits, side_index, is_human, is_defeated, has_won, has_lost, owned_building_count, owned_unit_count, tech_level, rally_point, base_center` — **`economy` is not and must not be referenced** | `world_hash.rs:157-184` |
| A4 | `ProductionState` is `#[derive(Debug, Clone, Serialize, Deserialize)]` with a **hand-written `Default`** — a new field MUST be added to both the struct and the `Default` impl | `production_types.rs:196-263` |
| A5 | `hash_production` hashes `queues_by_owner` items, `ready_by_owner`, `active_producer_by_owner`, `next_enqueue_order`, resources, ore growth, terrain, dock contacts — **`factory_shadow` is not and must not be referenced** | `world_hash.rs:187-271` |
| A6 | `BuildQueueItem` fields: `owner, type_id, queue_category, state, total_base_frames, remaining_base_frames, progress_carry, enqueue_order` — **there is NO `cost` field on the item** (cost lives in `rules.object(type_id).cost`) | `production_types.rs:22-34` |
| A7 | `queues_by_owner: BTreeMap<InternedId, BTreeMap<ProductionCategory, VecDeque<BuildQueueItem>>>` — the authoritative source the shadow derives from | `production_types.rs:198-199` |
| A8 | `BuildQueueState` variants `{Queued, Building, NoFunds, Paused, Done}`; "On Hold" is the `NoFunds` label | `production_types.rs:144-163` |
| A9 | `ProductionCategory` `{Building, Defense, Infantry, Vehicle, Aircraft}` derives `Ord` (tuple-key `Ord` for the registry `BTreeMap`) | `production_types.rs:135-142` |
| A10 | `refresh_mission_shadow()` is called at `world/mod.rs:2426`, `debug_assert_s1_shadow()` at `2432`, `let state_hash = self.state_hash();` at `2433` — the new shadow build + assert slot in **between 2426 and 2433**, mirroring the mission shadow | `world/mod.rs:2426/2432/2433` |
| A11 | `rules: &RuleSet` is in scope at the advance_tick tail (used at `world/mod.rs:2402`) — so `refresh_production_shadow(rules)` can take it | `world/mod.rs:2402` |
| A12 | `object_ai_stage()` is called at `world/mod.rs:1788` and dispatches `EntityCategory::Structure => {}` no-op arm at `techno_ai.rs:107`; the no-hash guarantee is the test `techno_ai_shell_is_passthrough_no_hash_change` at `techno_ai.rs:243-270` | `world/mod.rs:1788`, `techno_ai.rs:107/244` |
| A13 | `count_purifiers_for_owner(sim, rules, owner) -> i32` already counts owned OrePurifier structures (entity → `object_type` → `obj.ore_purifier`); REUSE it for `economy.purifier_count` (do NOT duplicate the logic) | `miner_system.rs:1460-1471` |
| A14 | `effective_purifier_count` (= real + AIVirtualPurifiers) is the *deposit-bonus* count and is a P7 concern — P1 `purifier_count` is the **real building count only** (`count_purifiers_for_owner`), NOT the effective count | `miner_system.rs:1479-1499` |
| A15 | The shadow pattern to mirror verbatim: `unit_ai_shadow_step` returns the OBSERVED value and never equalizes (`techno_ai.rs:160-181`); `debug_assert_s1_shadow` surfaces divergence with `tick + id` (`techno_ai.rs:192-222`) | `techno_ai.rs:160-222` |
| A16 | `Simulation::new()` + `state_hash()` + `set_logic_order_for_test` + `GameEntity::test_default` are the test fixtures used by the existing no-hash tests | `techno_ai.rs:248-270` |

**Two assumptions that need the design-lead's confirmation (see §E) — both about the P2
shadow's `balance`/`step_rate_frames` derive, because `BuildQueueItem` has no `cost` field (A6)
and `rebuild_shadow` is `&Simulation`-shaped.** They do not block P1 and are isolated to P2-T3.

---

## B. Files touched (summary)

| File | P1 | P2 | Change |
|---|---|---|---|
| `src/sim/economy.rs` | NEW | — | `Economy` value-type (no serde derive), methods, tests (P1-T1, P1-T4) |
| `src/sim/mod.rs` | EDIT | — | `pub mod economy;` declaration (P1-T1) |
| `src/sim/house_state.rs` | EDIT | — | `#[serde(skip)] pub economy: Economy` field + `new()` init (P1-T2) |
| `src/sim/world/mod.rs` | EDIT | EDIT | `refresh_economy_shadow` + `debug_assert_economy_shadow` (P1-T3); `refresh_production_shadow` + `debug_assert_production_shadow` wiring at the tail (P2-T5) |
| `src/sim/production/factory.rs` | — | NEW | `Factory`/`PendingObject`/`SpecialItem`/`StepOutcome`/`BuildEligibility`/`FactoryRegistry`/`FactoryView` + `rebuild_shadow` + tests (P2-T1, P2-T3, P2-T6) |
| `src/sim/production/mod.rs` | — | EDIT | `mod factory;` + re-exports (P2-T1) |
| `src/sim/production/production_types.rs` | — | EDIT | `#[serde(skip)] pub factory_shadow: FactoryRegistry` field + `Default` init (P2-T2) |
| `src/sim/world/techno_ai.rs` | — | EDIT | Structure arm read-only `factory_shadow_step` trace (P2-T4) |

`world_hash.rs` and `snapshot.rs` are **NOT** in this list — that is the no-hash contract.

---

## C. P1 — `Economy` value-type (shadow)

### P1-T1 — Create `src/sim/economy.rs`; declare the module

**File (NEW):** `src/sim/economy.rs`

Complete file:

```rust
//! Per-house wallet/storage/statistics value-type. Shadow-first: introduced as a
//! non-serialized field on `HouseState` that mirrors the authoritative `credits`.
//!
//! The purifier-bonus base is the per-house OrePurifier *building count* (NOT silo
//! storage capacity, and NOT the deposit-time effective count that folds in the AI
//! virtual term). `IncomeMult` is NOT stored here — it is read per-deposit from the
//! house's country type at a later slice. Depends only on `std`; NEVER on
//! render/ui/sidebar/audio/net (sim invariant #1).
//!
//! P1 scope: this type is `#[serde(skip)]` shadow state on `HouseState` and carries
//! NO `Serialize`/`Deserialize` derive — so the bincode layout is provably
//! byte-identical and the lockstep hash is untouched. The serde derive + hash fold
//! land at the authority-flip slice, not here.

/// Per-house wallet + storage + statistics, mirrored from the authoritative
/// `HouseState.credits` each tick. Shadow-only in P1.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Economy {
    /// Spendable balance. Tracks the legacy `HouseState.credits` exactly in P1
    /// (same `i32` scale — P1 introduces no rescale).
    pub credits: i32,
    /// Running total spent (statistics). No legacy mirror exists in P1 — exercised
    /// only by the isolated method unit tests, never accumulated from a live path.
    pub spent_credits: i32,
    /// Ore-deposit x5.0 statistics accumulator. No legacy mirror exists in P1 —
    /// isolated-method-tested only.
    pub harvested_credits: i32,
    /// OrePurifier building count; the purifier-bonus base. NEVER silo storage
    /// capacity, and NEVER the AI-virtual-inclusive effective count.
    pub purifier_count: i32,
}

impl Economy {
    /// Add credits to the balance (deposit, refund, grant).
    pub fn add_credits(&mut self, amount: i32) {
        self.credits = self.credits.saturating_add(amount);
    }

    /// Accumulate the statistics x5.0 figure for `bales` deposited. Integer `*5`
    /// because bales are integral (the engine's deposit x5.0 truncates to integer).
    /// Statistics only — does NOT touch `credits`.
    pub fn add_harvested(&mut self, bales: i32) {
        self.harvested_credits = self
            .harvested_credits
            .saturating_add(bales.saturating_mul(5));
    }

    /// Spend up to `amount`; returns the amount actually paid. In P1 the body is
    /// the trivial `min(credits, amount)` deduction so the type unit-tests in
    /// isolation; the silo-drain fallback is a later slice. `advance_tick` NEVER
    /// calls this on a real economy in P1+P2 — the legacy charge stays authoritative.
    pub fn spend(&mut self, amount: i32) -> i32 {
        let paid = amount.max(0).min(self.credits.max(0));
        self.credits -= paid;
        self.spent_credits = self.spent_credits.saturating_add(paid);
        paid
    }

    /// Spendable balance.
    pub fn available(&self) -> i32 {
        self.credits
    }
}
```

**File (EDIT):** `src/sim/mod.rs` — add the module declaration. Re-read the module block
before inserting; place `pub mod economy;` in alphabetical position among the existing
`pub mod` lines (it sorts before `pub mod entity_store;` / after `pub mod docking;`).

```rust
pub mod economy;
```

**Verification:**
- `cargo check -p vera20k`

---

### P1-T2 — Add the shadow field to `HouseState`

**File (EDIT):** `src/sim/house_state.rs`

Add the import (top of file, after the existing `use crate::sim::intern::InternedId;` at line 10):

```rust
use crate::sim::economy::Economy;
```

Add the field at the end of the `HouseState` struct, after `waypoint_edge: u8,` (line 48,
inside the struct that closes at line 49):

```rust
    /// Per-house wallet/storage/statistics shadow. Mirrors the authoritative
    /// `credits` field each tick; non-serialized and non-hashed until the
    /// authority flip. `Economy` carries no serde derive in P1+P2, so this
    /// `#[serde(skip)]` field cannot change the bincode layout or the state hash.
    #[serde(skip)]
    pub economy: Economy,
```

Add the init to the hand-written `HouseState::new` (A2), after `waypoint_edge: 0,` (line 74):

```rust
            economy: Economy::default(),
```

> NOTE: `HouseState` derives `Default` (A1), so `Economy: Default` makes the derived
> `Default` impl compile unchanged. The explicit `new()` init is required because `new()`
> enumerates every field by hand (it does not delegate to `Default`).

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k house_state` (existing `waypoint_edge_tests` must still pass —
  proves the field addition broke nothing)

---

### P1-T3 — `refresh_economy_shadow` + `debug_assert_economy_shadow`

**File (EDIT):** `src/sim/world/mod.rs`

Add these two methods to the `impl Simulation` block near `refresh_mission_shadow` (it begins
at line 909 — place the new methods in the same impl block; re-read the surrounding context
to land them adjacent, not inside another method). Mirror the mission/S1 shadow rhythm (A15):
derive direction is **legacy → shadow** (legacy `credits` authoritative through P4), divergence
is **surfaced with tick + owner**, never written back.

```rust
    /// P1 SHADOW BUILD: mirror each existing house's authoritative `credits` (and
    /// recompute its OrePurifier building count) into the non-hashed `economy`
    /// shadow. Derive direction is legacy -> shadow. READ-ONLY w.r.t. all hashed
    /// state: it iterates the existing `houses` map only and NEVER inserts a house
    /// (the §4.3 auto-create hazard guard — no call to `credits_entry_for_owner`).
    /// A missing owner simply has no economy update.
    pub(crate) fn refresh_economy_shadow(&mut self, rules: &crate::rules::RuleSet) {
        // Purifier counts are computed first (an immutable borrow of self), then
        // applied — `count_purifiers_for_owner` needs `&Simulation`, so we cannot
        // hold a `&mut self.houses` borrow across the call. BTreeMap key order is
        // deterministic; the temporary vec preserves it.
        let owner_ids: Vec<crate::sim::intern::InternedId> = self.houses.keys().copied().collect();
        let counts: Vec<(crate::sim::intern::InternedId, i32)> = owner_ids
            .iter()
            .map(|&id| {
                let owner = self.interner.resolve(id).to_string();
                (
                    id,
                    crate::sim::miner::miner_system::count_purifiers_for_owner(self, rules, &owner),
                )
            })
            .collect();
        for (id, count) in counts {
            if let Some(house) = self.houses.get_mut(&id) {
                // Mirror the authoritative wallet verbatim (same i32 scale, P1).
                house.economy.credits = house.credits;
                // Purifier-bonus base = real OrePurifier building COUNT (not silo
                // capacity, not the AI-virtual effective count).
                house.economy.purifier_count = count;
                // spent_credits / harvested_credits have NO legacy mirror — they are
                // NOT touched here (isolated-method-tested only, study §4.3 MISSING).
            }
        }
    }

    /// Debug-only P1 assert: every house's economy shadow must track the
    /// authoritative `credits`. Divergence is surfaced with tick + owner and
    /// asserted — never written back (the surface-not-equalize discipline).
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_economy_shadow(&self) {
        for (owner, house) in &self.houses {
            debug_assert_eq!(
                house.economy.credits, house.credits,
                "economy shadow: tick {} owner {:?}: economy.credits {} must track credits {}",
                self.tick, owner, house.economy.credits, house.credits,
            );
        }
    }
```

> The `count_purifiers_for_owner` re-export path: it is `pub(crate)` in
> `miner_system.rs:1460`. Confirm at impl time that `crate::sim::miner::miner_system` is the
> reachable path from `world/mod.rs` (the module is `pub(crate)`/`pub` along the chain); if the
> path differs, adjust the `use`/fully-qualified call — do NOT duplicate the counting logic
> (A13).

**Verification:**
- `cargo check -p vera20k`
- (asserts exercised by P1-T4 tests)

---

### P1-T4 — P1 tests (study §8 P1 set)

**File (EDIT):** `src/sim/economy.rs` — append a `#[cfg(test)] mod tests`. These are the
isolated-method unit tests plus the cross-system hash-neutrality / no-house-create guards.
The hash-neutrality + no-house-create tests live where the fixtures are easiest — put them in
`economy.rs` if `Simulation::new()`/`state_hash` are reachable, else mirror them into the
`world/mod.rs` test module (re-check reachability at impl time; the existing no-hash tests live
in `techno_ai.rs`, so a `world`-level test module is the proven home).

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn economy_default_is_zeroed() {
        let e = Economy::default();
        assert_eq!(
            (e.credits, e.spent_credits, e.harvested_credits, e.purifier_count),
            (0, 0, 0, 0)
        );
    }

    #[test]
    fn economy_add_credits_accumulates() {
        let mut e = Economy::default();
        e.add_credits(500);
        e.add_credits(250);
        assert_eq!(e.credits, 750);
        assert_eq!(e.available(), 750);
    }

    /// Isolated method test (NOT a shadow-track assert): the x5.0 statistics
    /// accumulator truncates to integer `*5` and never touches credits.
    #[test]
    fn economy_add_harvest_truncates_x5() {
        let mut e = Economy::default();
        e.add_harvested(7);
        assert_eq!(e.harvested_credits, 35);
        assert_eq!(e.credits, 0, "harvested stat must not move credits");
    }

    /// Isolated method test: spend deducts up to the balance, returns the paid
    /// amount, never goes negative, and tracks spent_credits. The silo-drain
    /// fallback is a later slice; this is the trivial P1 body.
    #[test]
    fn economy_spend_caps_at_balance_and_tracks_spent() {
        let mut e = Economy::default();
        e.add_credits(100);
        assert_eq!(e.spend(30), 30);
        assert_eq!(e.credits, 70);
        assert_eq!(e.spent_credits, 30);
        // Over-spend is capped at the balance; never negative.
        assert_eq!(e.spend(1000), 70);
        assert_eq!(e.credits, 0);
        assert_eq!(e.spent_credits, 100);
    }
}
```

**File (EDIT):** the `world`-level test module that hosts the existing no-hash tests — add the
shadow-track / hash-neutrality / no-house-create tests. (If `techno_ai.rs`'s `mod tests` is the
chosen host, mirror the `Simulation::new()` + `state_hash()` fixture shape from
`techno_ai.rs:248-270`.)

```rust
    /// P1 shadow track: after refresh, every house's economy.credits equals the
    /// authoritative credits.
    #[test]
    fn economy_shadow_tracks_legacy_credits() {
        let mut sim = Simulation::new();
        let rules = crate::rules::RuleSet::default(); // empty rules: 0 purifiers
        let a = sim.interner.intern("Americans");
        let b = sim.interner.intern("Russians");
        sim.houses.insert(
            a,
            crate::sim::house_state::HouseState::new(a, 0, None, true, 5000, 10),
        );
        sim.houses.insert(
            b,
            crate::sim::house_state::HouseState::new(b, 1, None, true, 1234, 10),
        );
        sim.refresh_economy_shadow(&rules);
        assert_eq!(sim.houses[&a].economy.credits, 5000);
        assert_eq!(sim.houses[&b].economy.credits, 1234);
        assert_eq!(sim.houses[&a].economy.purifier_count, 0);
    }

    /// The §4.3 hazard guard: the shadow refresh must NOT auto-create a house for
    /// an owner that has none, and must not perturb the hash.
    #[test]
    fn economy_shadow_does_not_create_houses() {
        let mut sim = Simulation::new();
        let rules = crate::rules::RuleSet::default();
        // No houses inserted; queue/owner-less sim.
        let before_len = sim.houses.len();
        let before = sim.state_hash();
        sim.refresh_economy_shadow(&rules);
        assert_eq!(sim.houses.len(), before_len, "shadow must not create houses");
        assert_eq!(before, sim.state_hash(), "shadow must not perturb the hash");
    }

    /// The no-hash contract (mirror of techno_ai_shell_is_passthrough_no_hash_change).
    #[test]
    fn economy_shadow_does_not_change_state_hash() {
        let mut sim = Simulation::new();
        let rules = crate::rules::RuleSet::default();
        let a = sim.interner.intern("Americans");
        sim.houses.insert(
            a,
            crate::sim::house_state::HouseState::new(a, 0, None, true, 5000, 10),
        );
        let before = sim.state_hash();
        sim.refresh_economy_shadow(&rules);
        let after = sim.state_hash();
        assert_eq!(before, after, "economy shadow must not perturb the state hash");
    }
```

> If `crate::rules::RuleSet::default()` is not a thing, use the smallest available rules
> constructor (the miner tests build one via `RuleSet::from_ini`); re-check at impl time. The
> purifier-count path needs *some* `&RuleSet`; an empty/minimal one yields 0 purifiers, which is
> what these P1 tests assert.

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k economy` — runs: `economy_default_is_zeroed`,
  `economy_add_credits_accumulates`, `economy_add_harvest_truncates_x5`,
  `economy_spend_caps_at_balance_and_tracks_spent`, `economy_shadow_tracks_legacy_credits`,
  `economy_shadow_does_not_create_houses`, `economy_shadow_does_not_change_state_hash`

---

## D. P2 — `Factory` + `FactoryRegistry` (derived shadow) + FIT option (a) trace

### P2-T1 — Create `src/sim/production/factory.rs`; declare + re-export

**File (NEW):** `src/sim/production/factory.rs` — types only this task (the `rebuild_shadow`
body + the Structure-arm trace land in P2-T3/P2-T4). No serde derives anywhere in this file
in P1+P2 (graft from D3: provably byte-identical bincode layout).

```rust
//! Per-(house, category) factory shadow + deterministic registry.
//!
//! P2 introduces these as DERIVED, non-serialized shadow state on `ProductionState`,
//! rebuilt each tick from the authoritative `queues_by_owner`. They mirror the
//! engine's per-(house,category) production state machine on the hash-relevant
//! fields, but the legacy queue stays authoritative through the authority flip
//! (out of scope). Divergence is SURFACED (tick + owner + category), never
//! equalized — the same discipline as the unit-AI shadow.
//!
//! P2 scope: NO `Serialize`/`Deserialize` derive on any type here, so the registry
//! field is provably hash-neutral and `SNAPSHOT_VERSION` stays put. The serde
//! derive + hash fold + the `next_insertion_seq`-is-serialized obligation are P5.
//!
//! Determinism: `BTreeMap<(InternedId, ProductionCategory), Factory>` (both key
//! components derive `Ord`) gives sorted iteration for replay/lockstep; no
//! `HashMap`, no fixed-size player array, no `1<<idx` bitmask — satisfies the
//! 30-player scale target. Integer math only; no float, no RNG.
//!
//! Depends on: `sim/intern`, `sim/production/production_types` (ProductionCategory),
//! and `sim/world::Simulation` (read-only) for the derive. NEVER on render/ui/etc.

use std::collections::{BTreeMap, VecDeque};

use crate::sim::intern::InternedId;
use crate::sim::production::production_types::ProductionCategory;

/// Build completes at exactly this many progress steps (the engine's step count).
pub const PRODUCTION_STEPS: u16 = 54;
/// Per-step frame-rate clamp (the engine clamps `total/54` into `[1, 255]`).
pub const STEP_RATE_MIN: u16 = 1;
pub const STEP_RATE_MAX: u16 = 255;

/// The object a factory holds from start through delivery. In P2 shadow `entity_id`
/// is always `None` (the produced entity is created by the legacy path); the field
/// is held distinct so the complete-but-not-delivered state is representable now.
#[derive(Debug, Clone, Default, PartialEq, Eq)] // NO serde in P1+P2
pub struct PendingObject {
    pub type_id: InternedId,
    pub entity_id: Option<u64>,
}

/// Engine special/superweapon discriminator. The study proves the writer of the
/// engine's special-item field was never located, so value `0` cannot be proven
/// unreachable and `0`-vs-`(-1)` MUST NOT be collapsed. Three states keep them
/// distinct. In P1+P2 (normal builds) this is always `NoneNeg1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)] // NO serde in P1+P2
pub enum SpecialItem {
    NoneNeg1,
    NoneZero,
    Item(u32),
}

impl Default for SpecialItem {
    fn default() -> Self {
        SpecialItem::NoneNeg1
    }
}

/// One production state machine per (house, category). Value-type owned by the
/// `FactoryRegistry`. In P2 it is DERIVED shadow — the per-step charge/stepping is
/// a later slice; here the fields mirror the legacy queue item.
#[derive(Debug, Clone, Default, PartialEq, Eq)] // NO serde in P1+P2
pub struct Factory {
    pub owner: InternedId,
    pub category: ProductionCategory,
    /// `0..=54`; completion at `PRODUCTION_STEPS`.
    pub progress: u16,
    /// Per-step frame rate = `clamp(GetBuildStepTime()/54, 1, 255)`; `0` when no object.
    pub step_rate_frames: u16,
    /// Frames remaining in the current step (engine CDTimer). Shadow best-effort in P2.
    pub step_timer: u16,
    /// Remaining cost still owed (charged down per step at a later slice). Shadow value.
    pub balance: i32,
    /// Full-cost snapshot at start, for exact-cost conservation (later slice).
    pub original_balance: i32,
    pub object: Option<PendingObject>,
    /// Set when a step could not be afforded (UI "On Hold"); does not advance.
    pub on_hold: bool,
    /// Complete-but-not-delivered, or paused: not stepping.
    pub suspended: bool,
    /// User-vs-system pause distinction.
    pub manual: bool,
    pub special: SpecialItem,
    /// FIFO type ids waiting behind the active object.
    pub queue: VecDeque<InternedId>,
    /// Deterministic registration order for same-frame completion sequencing.
    pub insertion_seq: u64,
}

/// Outcome of a single factory step (consumer is the per-step charge slice).
/// Defined now so the registry surface is stable.
pub enum StepOutcome {
    Idle,
    Stepped,
    Stalled,
    Completed,
}

/// 3-way prerequisite eligibility (later-slice consumer; defined now so the
/// registry surface is stable). The active object runs BOTH `(1,0,1)` and `(1,1,1)`
/// gates; queued items only `(1,0,1)`.
pub enum BuildEligibility {
    Buildable,
    TemporarilyBlocked,
    PermanentlyBlocked,
}

/// Borrow-only sidebar projection (render seam). Never mutates; never hashed.
pub struct FactoryView<'a> {
    pub progress: u16,
    pub on_hold: bool,
    pub suspended: bool,
    pub object: Option<&'a PendingObject>,
    pub queue: &'a VecDeque<InternedId>,
    /// `true` when the active object has reached `PRODUCTION_STEPS`.
    pub ready: bool,
}

/// Deterministic registry of all factories — the derived shadow analog of the
/// engine's global factory array, keyed (no fixed-size player array) for scale.
#[derive(Debug, Clone, Default, PartialEq, Eq)] // NO serde in P1+P2
pub struct FactoryRegistry {
    factories: BTreeMap<(InternedId, ProductionCategory), Factory>,
    next_insertion_seq: u64,
    /// Carried across the per-tick rebuild so a surviving (owner, category) keeps a
    /// stable `insertion_seq` (same-frame ordering identity). Skipped + unhashed.
    seq_carry: BTreeMap<(InternedId, ProductionCategory), u64>,
}

impl FactoryRegistry {
    /// Read-only sidebar projection. Never mutates.
    pub fn view(
        &self,
        owner: InternedId,
        category: ProductionCategory,
    ) -> Option<FactoryView<'_>> {
        let f = self.factories.get(&(owner, category))?;
        Some(FactoryView {
            progress: f.progress,
            on_hold: f.on_hold,
            suspended: f.suspended,
            object: f.object.as_ref(),
            queue: &f.queue,
            ready: f.progress >= PRODUCTION_STEPS,
        })
    }

    /// Number of registered factories (test/observation helper).
    pub fn len(&self) -> usize {
        self.factories.len()
    }

    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }

    /// Iterate factories in deterministic `insertion_seq` order — reproduces the
    /// native registration order for same-frame completion sequencing (NOT the
    /// BTreeMap key order). P2 exercises this only via the iteration-order test;
    /// it charges no economy.
    pub fn iter_insertion_ordered(&self) -> Vec<&Factory> {
        let mut all: Vec<&Factory> = self.factories.values().collect();
        all.sort_by_key(|f| f.insertion_seq);
        all
    }

    // ---- declared for later slices; documented seams, NOT called in P1-P2 ----
    // `begin` (AI seam), `cancel_one` (P4), `revalidate` (P6), `step_all` /
    // `advance_one_step` (P3) are intentionally NOT defined yet: an empty fn body
    // would be dead code and clippy noise. They are added in their owning slice.
}
```

**File (EDIT):** `src/sim/production/mod.rs` — add the module + re-exports. Place `mod factory;`
with the other `mod` lines (after `war_factory_exit;` at line 18 or in alpha order — re-read):

```rust
mod factory;
```

Add the re-export with the other `pub use self::...` lines (near line 46):

```rust
pub use self::factory::{
    BuildEligibility, Factory, FactoryRegistry, FactoryView, PendingObject, SpecialItem,
    StepOutcome, PRODUCTION_STEPS, STEP_RATE_MAX, STEP_RATE_MIN,
};
```

**Verification:**
- `cargo check -p vera20k` (will warn on unused `StepOutcome`/`BuildEligibility`/consts until
  consumed; mark with `#[allow(dead_code)]` on the enums/consts if clippy is run as part of the
  gate, or accept the warnings as documented seams — re-check the project's warning policy)

---

### P2-T2 — Add the shadow field to `ProductionState`

**File (EDIT):** `src/sim/production/production_types.rs`

Add to the import group (the type is re-exported from `production/mod.rs` but within the same
crate the direct path is cleanest):

```rust
use crate::sim::production::factory::FactoryRegistry;
```

Add the field at the end of the `ProductionState` struct, after `airfield_docks: ...` (line 237,
inside the struct closing at 238):

```rust
    /// Per-(house, category) factory shadow, rebuilt each tick from
    /// `queues_by_owner`. Derived; non-serialized and non-hashed until the
    /// authority flip. `FactoryRegistry` carries no serde derive in P1+P2, so this
    /// `#[serde(skip)]` field cannot change the bincode layout or the state hash.
    #[serde(skip)]
    pub factory_shadow: FactoryRegistry,
```

Add to the hand-written `Default` impl (A4), after `airfield_docks: ...` (line 260):

```rust
            factory_shadow: FactoryRegistry::default(),
```

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k production` (existing production tests must still pass — proves the
  field addition + Default broke nothing)

---

### P2-T3 — `FactoryRegistry::rebuild_shadow` (derive from `queues_by_owner`)

**File (EDIT):** `src/sim/production/factory.rs` — add the derive body to `impl FactoryRegistry`.

This walks `sim.production.queues_by_owner` (A7) and derives one `Factory` per
(owner, category) with a non-empty queue, per the design §4.2 mapping. `insertion_seq` reuses
`seq_carry` for survivors (stable identity), minting new seqs only for newly-appearing
factories. READ-ONLY w.r.t. all hashed state; writes only the shadow.

> **DESIGN-LEAD CONFIRMATION NEEDED (see §E1):** `BuildQueueItem` has no `cost` field (A6) and
> `rebuild_shadow(&Simulation)` has no `&RuleSet`. So `balance`/`original_balance`/
> `step_rate_frames` cannot be derived from cost here. The plan derives them from the **frame
> counts** (`total_base_frames`/`remaining_base_frames`) as a cost-free monotone projection:
> `original_balance = total_base_frames as i32`, `balance = remaining_base_frames as i32`,
> `step_rate_frames` left `0` in P2 (no object→no rate is the contract; with an object it is a
> proportional stand-in). The P2 asserts are **monotone-tracking only** (§4.2 / U2), so a
> frames-based projection satisfies them without a cost lookup. If the design-lead wants the
> exact `cost - progress*cost/54` shape, `rebuild_shadow` must take `(&Simulation, &RuleSet)` —
> a one-line signature change wired at P2-T5. Frames-based is the lower-surface default.

```rust
impl FactoryRegistry {
    /// P2 SHADOW BUILD: (re)derive the whole registry from the legacy queues each
    /// tick. READ-ONLY w.r.t. all hashed state. Reuses `seq_carry` to keep
    /// `insertion_seq` stable for surviving (owner, category) factories.
    pub(crate) fn rebuild_shadow(&mut self, sim: &crate::sim::world::Simulation) {
        let mut new_factories: BTreeMap<(InternedId, ProductionCategory), Factory> =
            BTreeMap::new();
        let mut new_carry: BTreeMap<(InternedId, ProductionCategory), u64> = BTreeMap::new();

        for (&owner, queues) in &sim.production.queues_by_owner {
            for (&category, queue) in queues {
                let Some(front) = queue.front() else {
                    continue; // empty category: no factory
                };
                let key = (owner, category);

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

                // progress 0..=54: monotone bridge from base-frame remaining.
                // Guard division by zero on total==0.
                let progress = if front.total_base_frames == 0 {
                    0u16
                } else {
                    let done = front
                        .total_base_frames
                        .saturating_sub(front.remaining_base_frames);
                    let p = (u64::from(done) * u64::from(PRODUCTION_STEPS))
                        / u64::from(front.total_base_frames);
                    (p as u16).min(PRODUCTION_STEPS)
                };

                // The front item is the active object when it is Building/NoFunds/
                // Done; a Paused front is suspended; a Queued front is queue-only.
                use crate::sim::production::production_types::BuildQueueState as S;
                let has_object = matches!(front.state, S::Building | S::NoFunds | S::Done);
                let object = if has_object {
                    Some(PendingObject {
                        type_id: front.type_id,
                        entity_id: None, // legacy path owns the produced entity in P2
                    })
                } else {
                    None
                };

                // Tail items become the FIFO queue (order preserved).
                let tail: VecDeque<InternedId> =
                    queue.iter().skip(1).map(|item| item.type_id).collect();

                // Frames-based shadow balance (see §E1): cost-free monotone projection.
                let original_balance = front.total_base_frames as i32;
                let balance = front.remaining_base_frames as i32;

                let factory = Factory {
                    owner,
                    category,
                    progress,
                    // No-object => rate 0 (the contract); with-object stays 0 in P2
                    // (exact rate needs the build-time path / rules — later slice).
                    step_rate_frames: 0,
                    step_timer: 0,
                    balance,
                    original_balance,
                    object,
                    on_hold: matches!(front.state, S::NoFunds),
                    // Paused (user) or Done (awaiting placement) => not stepping.
                    suspended: matches!(front.state, S::Paused | S::Done),
                    manual: matches!(front.state, S::Paused),
                    special: SpecialItem::NoneNeg1, // normal builds
                    queue: tail,
                    insertion_seq: seq,
                };
                new_factories.insert(key, factory);
            }
        }

        self.factories = new_factories;
        self.seq_carry = new_carry;
    }
}
```

> If `crate::sim::world::Simulation` is not re-exported at that path, use the canonical path the
> rest of `sim/` uses (`crate::sim::world::Simulation` is the path `techno_ai.rs:15` imports via
> `super::Simulation`); re-check at impl time.

**Verification:**
- `cargo check -p vera20k`
- (behavior exercised by P2-T6 tests)

---

### P2-T4 — Structure arm read-only trace (FIT option a)

**File (EDIT):** `src/sim/world/techno_ai.rs`

Fill the `EntityCategory::Structure => {}` no-op arm (line 107) with a read-only shadow
observation that records a `FactoryShellTrace` in LogicVector order — exactly as
`unit_ai_shadow_step` records a `ShellTrace`. It performs `&Simulation`-shaped reads only:
no hashed-state mutation, no RNG — so `techno_ai_shell_is_passthrough_no_hash_change` (A12)
still passes.

> SCOPE NOTE: the P2 Structure arm records a trace; it does NOT step or charge (no authoritative
> step exists until P5). The same slot flips from "record trace" to "step" by a body swap at the
> authority flip — the iteration source (LogicVector order) is unchanged. Because (a) makes the
> order LogicVector-by-construction, the P2 trace-order test is a TRUE assertion, never an
> UNPROVEN-equivalence guard.

Because `techno_ai_shell` is a free fn with no trace accumulator, add a debug-only
`debug_assert_factory_shell_trace` Simulation method (mirroring `debug_assert_s1_shadow`) that
walks live order and builds the trace itself — this keeps the no-op `techno_ai_shell` arm a
no-op for the release hot path (the trace is debug/test observation only, like S1). Add the
trace struct + step fn gated `#[cfg(any(test, debug_assertions))]`, beside the S1 shadow block:

```rust
// ===== P2 (factory substrate) — Structure-arm read-only shadow trace =====
//
// FIT option (a): the per-(house,category) factory step is driven from the
// Structure arm of object_ai_stage() in LogicVector order; the FactoryRegistry is
// a LOOKUP. In P1+P2 there is no authoritative step, so the arm records a
// read-only trace of each live Structure that is its category's active producer,
// in LogicVector order. The trace-order test asserts the order IS LogicVector
// order BY CONSTRUCTION — a true assertion, not the option-(b) guard. Read-only,
// debug-only, never hashed; the authority flip is a later slice.

#[cfg(any(test, debug_assertions))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FactoryShellTrace {
    /// Live structure id, in LogicVector visit order.
    structure_id: u64,
    /// Ordinal at which this structure was visited (LogicVector order index).
    visit_seq: u32,
}

impl Simulation {
    /// Debug-only P2 trace: walk live order; for each live Structure record a
    /// FactoryShellTrace in LogicVector order. Read-only; never hashed, never
    /// serialized. Asserts the recorded visit_seq is strictly increasing in
    /// LogicVector order (true by construction under FIT option a) and that the
    /// trace order equals the live Structure order.
    #[cfg(any(test, debug_assertions))]
    pub(crate) fn debug_assert_factory_shell_trace(&self) {
        let mut seq = 0u32;
        let mut traces: Vec<FactoryShellTrace> = Vec::new();
        for id in self.live_object_order_snapshot() {
            let Some(entity) = self.substrate.entities.get(id) else {
                continue;
            };
            if entity.dying || entity.category != EntityCategory::Structure {
                continue;
            }
            traces.push(FactoryShellTrace {
                structure_id: id,
                visit_seq: seq,
            });
            seq += 1;
        }
        // Trace order == LogicVector Structure order, by construction.
        let logic_structures: Vec<u64> = self
            .live_object_order_snapshot()
            .into_iter()
            .filter(|&id| {
                self.substrate
                    .entities
                    .get(id)
                    .is_some_and(|e| !e.dying && e.category == EntityCategory::Structure)
            })
            .collect();
        let traced: Vec<u64> = traces.iter().map(|t| t.structure_id).collect();
        debug_assert_eq!(
            traced, logic_structures,
            "P2: tick {}: factory shell trace order must equal LogicVector Structure order",
            self.tick,
        );
        // visit_seq strictly increasing.
        for w in traces.windows(2) {
            debug_assert!(
                w[0].visit_seq < w[1].visit_seq,
                "P2: tick {}: factory shell trace visit_seq must strictly increase",
                self.tick,
            );
        }
    }
}
```

> The `EntityCategory::Structure => {}` arm stays a no-op (the trace is built by the debug-only
> Simulation method above, exactly as the S1 dispatch→process proof lives in
> `debug_assert_s1_shadow`, not in the `techno_ai_shell` arm). This keeps the release walk a
> strict no-op and `techno_ai_shell_is_passthrough_no_hash_change` unchanged. Add a one-line
> comment on the Structure arm pointing to `debug_assert_factory_shell_trace` so the intent is
> discoverable.

**Verification:**
- `cargo check -p vera20k`
- (exercised by `factory_shadow_trace_order_matches_logic_vector` in P2-T6)

---

### P2-T5 — Wire `refresh_production_shadow` into `advance_tick`

**File (EDIT):** `src/sim/world/mod.rs`

Add the umbrella refresh + assert methods to `impl Simulation` (near `refresh_economy_shadow`
from P1-T3):

```rust
    /// P2 SHADOW BUILD umbrella: (i) refresh the per-house economy shadow, then
    /// (ii) rebuild the factory registry from the legacy queues. Runs at the
    /// advance_tick tail, AFTER all authoritative systems, so the derive sees
    /// settled legacy state. Both sub-steps write ONLY the non-hashed shadow
    /// fields; the legacy production path is completely untouched.
    pub(crate) fn refresh_production_shadow(&mut self, rules: &crate::rules::RuleSet) {
        self.refresh_economy_shadow(rules);
        // Take the registry out to satisfy the borrow checker (rebuild_shadow needs
        // &Simulation while writing &mut the registry). Swap back after.
        let mut registry = std::mem::take(&mut self.production.factory_shadow);
        registry.rebuild_shadow(self);
        self.production.factory_shadow = registry;
    }

    /// Debug-only P2 asserts: (a) economy tracks credits; (b) the factory shell
    /// trace order equals LogicVector order. Divergence is surfaced, never
    /// equalized.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_production_shadow(&self) {
        self.debug_assert_economy_shadow();
        self.debug_assert_factory_shell_trace();
    }
```

> `std::mem::take` requires `FactoryRegistry: Default` (it derives `Default`, A4-analog) — the
> taken-out registry is empty during `rebuild_shadow`, which is fine because `rebuild_shadow`
> reads `sim.production.queues_by_owner` (the legacy source), not `factory_shadow`. This is the
> idiomatic Rust way to thread `&Simulation` + `&mut registry`; it allocates nothing beyond the
> BTreeMap swaps already inherent to a from-scratch rebuild.

Wire the calls at the advance_tick tail. Edit the block at `world/mod.rs:2426-2433` (A10):

```rust
        self.refresh_mission_shadow();                 // existing (2426)
        // P1+P2 production+economy shadow: mirror credits + purifier_count, rebuild
        // the factory registry from the legacy queues. Runs after all authoritative
        // systems, before the hash; writes only non-hashed shadow fields.
        self.refresh_production_shadow(rules);          // NEW
        #[cfg(debug_assertions)]
        self.debug_assert_s1_shadow();                  // existing (2432)
        #[cfg(debug_assertions)]
        self.debug_assert_production_shadow();          // NEW
        let state_hash = self.state_hash();             // existing (2433) — MUST be unchanged
```

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k production_shadow_preserves_advance_tick_phase_order` (P2-T6)

---

### P2-T6 — P2 tests (study §8 P2 set + the design §5.2 proving tests)

**File (EDIT):** `src/sim/production/factory.rs` — append a `#[cfg(test)] mod tests` for the
pure-type / derive / iteration-order / special-item tests; put the `state_hash`-bearing and
`advance_tick`-bearing tests in the `world`-level test module (where `Simulation::new()`,
`advance_tick`, snapshot round-trip, and `state_hash` fixtures live).

Pure-type / derive tests in `factory.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_item_none_zero_and_neg1_distinct() {
        // The 0/-1 collapse the study forbids: the three states must compare unequal.
        assert_ne!(SpecialItem::NoneNeg1, SpecialItem::NoneZero);
        assert_ne!(SpecialItem::NoneNeg1, SpecialItem::Item(0));
        assert_ne!(SpecialItem::NoneZero, SpecialItem::Item(0));
        assert_eq!(SpecialItem::default(), SpecialItem::NoneNeg1);
    }

    #[test]
    fn factory_default_progress_zero_no_object() {
        let f = Factory::default();
        assert_eq!(f.progress, 0);
        assert!(f.object.is_none());
        assert_eq!(f.step_rate_frames, 0);
    }

    #[test]
    fn registry_iter_insertion_ordered_not_map_order() {
        // Hand-build two factories with insertion_seq reversed vs key order, prove
        // iter_insertion_ordered sorts by seq (not BTreeMap key order).
        let mut reg = FactoryRegistry::default();
        // (direct field access is in-module) — keys A<B but seqs 1,0.
        let a = InternedId::default();
        // NOTE: a real fixture interns distinct ids; this is a structural sort proof.
        let mut fa = Factory { insertion_seq: 1, ..Factory::default() };
        let mut fb = Factory { insertion_seq: 0, ..Factory::default() };
        fa.category = ProductionCategory::Building;
        fb.category = ProductionCategory::Infantry;
        fa.owner = a;
        fb.owner = a;
        reg.factories.insert((a, ProductionCategory::Building), fa);
        reg.factories.insert((a, ProductionCategory::Infantry), fb);
        let ordered: Vec<u64> = reg.iter_insertion_ordered().iter().map(|f| f.insertion_seq).collect();
        assert_eq!(ordered, vec![0, 1], "iteration is insertion_seq order, not map order");
    }
}
```

> The `iter_insertion_ordered` test reaches `reg.factories` / private fields — it is an
> in-module `#[cfg(test)] mod tests` so that is allowed. If `InternedId::default()` is not a
> thing, build ids via a throwaway `StringInterner` (`intern("A")`/`intern("B")`); re-check at
> impl time.

`world`-level tests (derive-from-legacy, monotone tracking, hash-neutrality, no-create,
snapshot round-trip, trace order, phase order, version-pin). Build the fixture by inserting
`BuildQueueItem`s into `sim.production.queues_by_owner` directly:

```rust
    use crate::sim::production::production_types::{BuildQueueItem, BuildQueueState, ProductionCategory};
    use std::collections::VecDeque;

    fn queued_item(owner: crate::sim::intern::InternedId, ty: crate::sim::intern::InternedId,
                   cat: ProductionCategory, state: BuildQueueState,
                   total: u32, remaining: u32, order: u64) -> BuildQueueItem {
        BuildQueueItem {
            owner, type_id: ty, queue_category: cat, state,
            total_base_frames: total, remaining_base_frames: remaining,
            progress_carry: 0, enqueue_order: order,
        }
    }

    /// Shadow progress maps monotonically to legacy remaining frames and reaches 54
    /// at completion; divergence is surfaced, never equalized.
    #[test]
    fn factory_shadow_progress_tracks_legacy_remaining() {
        let mut sim = Simulation::new();
        let rules = crate::rules::RuleSet::default();
        let owner = sim.interner.intern("Americans");
        let ty = sim.interner.intern("GRIZZLY");
        // Half-built: 54 total frames, 27 remaining -> progress 27.
        let mut q = std::collections::BTreeMap::new();
        let mut dq = VecDeque::new();
        dq.push_back(queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 27, 1));
        q.insert(ProductionCategory::Vehicle, dq);
        sim.production.queues_by_owner.insert(owner, q);

        sim.refresh_production_shadow(&rules);
        let view = sim.production.factory_shadow.view(owner, ProductionCategory::Vehicle).unwrap();
        assert_eq!(view.progress, 27, "half-remaining -> half progress");
        assert!(view.object.is_some(), "Building front => active object");

        // Drive remaining to 0 -> progress 54 (completion coincidence).
        sim.production.queues_by_owner.get_mut(&owner).unwrap()
            .get_mut(&ProductionCategory::Vehicle).unwrap()
            .front_mut().unwrap().remaining_base_frames = 0;
        sim.refresh_production_shadow(&rules);
        let view = sim.production.factory_shadow.view(owner, ProductionCategory::Vehicle).unwrap();
        assert_eq!(view.progress, super::super::production::PRODUCTION_STEPS,
                   "remaining 0 -> progress reaches 54");
    }

    /// 3 owners x 2 categories: step iteration visits factories in insertion_seq
    /// order, stable across a rebuild.
    #[test]
    fn factory_registry_iteration_is_insertion_ordered() {
        let mut sim = Simulation::new();
        let rules = crate::rules::RuleSet::default();
        for (i, name) in ["A", "B", "C"].iter().enumerate() {
            let owner = sim.interner.intern(name);
            let ty = sim.interner.intern(&format!("U{i}"));
            let mut q = std::collections::BTreeMap::new();
            for cat in [ProductionCategory::Vehicle, ProductionCategory::Infantry] {
                let mut dq = VecDeque::new();
                dq.push_back(queued_item(owner, ty, cat, BuildQueueState::Building, 54, 10, 1));
                q.insert(cat, dq);
            }
            sim.production.queues_by_owner.insert(owner, q);
        }
        sim.refresh_production_shadow(&rules);
        let seqs: Vec<u64> = sim.production.factory_shadow.iter_insertion_ordered()
            .iter().map(|f| f.insertion_seq).collect();
        let mut sorted = seqs.clone();
        sorted.sort();
        assert_eq!(seqs, sorted, "iteration is monotonic in insertion_seq");
        assert_eq!(seqs.len(), 6, "3 owners x 2 categories = 6 factories");
    }

    /// insertion_seq is stable across a rebuild for a surviving (owner,category).
    #[test]
    fn insertion_seq_stable_across_rebuild() {
        let mut sim = Simulation::new();
        let rules = crate::rules::RuleSet::default();
        let owner = sim.interner.intern("Americans");
        let ty = sim.interner.intern("GRIZZLY");
        let mut q = std::collections::BTreeMap::new();
        let mut dq = VecDeque::new();
        dq.push_back(queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1));
        q.insert(ProductionCategory::Vehicle, dq);
        sim.production.queues_by_owner.insert(owner, q);

        sim.refresh_production_shadow(&rules);
        let seq1 = sim.production.factory_shadow.view(owner, ProductionCategory::Vehicle).unwrap().progress; // touch
        let _ = seq1;
        let seq_a = sim.production.factory_shadow.iter_insertion_ordered()[0].insertion_seq;
        // Advance the build, rebuild — same factory survives, same seq.
        sim.production.queues_by_owner.get_mut(&owner).unwrap()
            .get_mut(&ProductionCategory::Vehicle).unwrap()
            .front_mut().unwrap().remaining_base_frames = 10;
        sim.refresh_production_shadow(&rules);
        let seq_b = sim.production.factory_shadow.iter_insertion_ordered()[0].insertion_seq;
        assert_eq!(seq_a, seq_b, "surviving factory keeps a stable insertion_seq");
    }

    /// The no-hash contract: building the factory shadow leaves state_hash() bit-identical.
    #[test]
    fn factory_registry_shadow_no_hash_change() {
        let mut sim = Simulation::new();
        let rules = crate::rules::RuleSet::default();
        let owner = sim.interner.intern("Americans");
        let ty = sim.interner.intern("GRIZZLY");
        let mut q = std::collections::BTreeMap::new();
        let mut dq = VecDeque::new();
        dq.push_back(queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1));
        q.insert(ProductionCategory::Vehicle, dq);
        sim.production.queues_by_owner.insert(owner, q);

        let before = sim.state_hash();
        sim.refresh_production_shadow(&rules);
        let after = sim.state_hash();
        assert_eq!(before, after, "factory shadow rebuild must not perturb the state hash");
    }

    /// The shadow must NOT create a house for a queued owner with no house entry.
    #[test]
    fn production_shadow_does_not_create_houses() {
        let mut sim = Simulation::new();
        let rules = crate::rules::RuleSet::default();
        let owner = sim.interner.intern("Ghost"); // no HouseState inserted
        let ty = sim.interner.intern("GRIZZLY");
        let mut q = std::collections::BTreeMap::new();
        let mut dq = VecDeque::new();
        dq.push_back(queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1));
        q.insert(ProductionCategory::Vehicle, dq);
        sim.production.queues_by_owner.insert(owner, q);

        let before_houses = sim.houses.len();
        let before_queues = sim.production.queues_by_owner.len();
        let before = sim.state_hash();
        sim.refresh_production_shadow(&rules);
        assert_eq!(sim.houses.len(), before_houses, "shadow must not create houses");
        assert_eq!(sim.production.queues_by_owner.len(), before_queues, "queues unchanged");
        assert_eq!(before, sim.state_hash(), "hash unchanged");
    }

    /// FIT (a): the Structure-arm trace order equals LogicVector Structure order
    /// (true by construction). A bare run of the debug assert must not panic.
    #[test]
    fn factory_shadow_trace_order_matches_logic_vector() {
        let mut sim = Simulation::new();
        // Insert structures in a deliberately non-sorted live order.
        for id in [3u64, 1, 2] {
            let mut e = crate::sim::game_entity::GameEntity::test_default(id, "B", "Americans", 5, 5);
            e.category = crate::map::entities::EntityCategory::Structure;
            sim.substrate.entities.insert(e);
        }
        sim.set_logic_order_for_test(vec![3, 1, 2]);
        sim.debug_assert_factory_shell_trace(); // must not panic — order is LogicVector order
    }

    /// Snapshot round-trip: the skipped economy/factory_shadow come back Default and
    /// the hash is unchanged across the serialize->deserialize boundary.
    #[test]
    fn snapshot_roundtrip_ignores_shadow() {
        let mut sim = Simulation::new();
        let rules = crate::rules::RuleSet::default();
        let owner = sim.interner.intern("Americans");
        sim.houses.insert(owner, crate::sim::house_state::HouseState::new(owner, 0, None, true, 5000, 10));
        let ty = sim.interner.intern("GRIZZLY");
        let mut q = std::collections::BTreeMap::new();
        let mut dq = VecDeque::new();
        dq.push_back(queued_item(owner, ty, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1));
        q.insert(ProductionCategory::Vehicle, dq);
        sim.production.queues_by_owner.insert(owner, q);
        sim.refresh_production_shadow(&rules);
        let hash_before = sim.state_hash();

        // Serialize -> deserialize via the snapshot path (re-check the exact API at
        // impl time; the project has GameSnapshot::serialize/deserialize or a
        // bincode round-trip helper in snapshot.rs).
        let blob = crate::sim::snapshot::serialize_for_test(&sim);
        let restored = crate::sim::snapshot::deserialize_for_test(&blob);

        assert_eq!(restored.houses[&owner].economy, crate::sim::economy::Economy::default(),
                   "skipped economy comes back Default");
        assert!(restored.production.factory_shadow.is_empty(),
                "skipped factory_shadow comes back Default (empty)");
        assert_eq!(restored.state_hash(), hash_before,
                   "hash unchanged across the round-trip (shadow not load-bearing)");
    }

    /// Identical fixtures over N ticks produce identical per-tick state_hash
    /// sequences (mirror of techno_ai_shell_preserves_advance_tick_phase_order).
    #[test]
    fn production_shadow_preserves_advance_tick_phase_order() {
        fn run() -> Vec<u64> {
            let mut sim = Simulation::new();
            let heights = std::collections::BTreeMap::new();
            (0..5).map(|_| {
                sim.advance_tick(&[], None, &heights, None, None, 67);
                sim.state_hash()
            }).collect()
        }
        assert_eq!(run(), run(), "advance_tick with the production shadow stays deterministic");
    }
```

> The snapshot round-trip test names `serialize_for_test`/`deserialize_for_test` as placeholders
> — at impl time use the project's real snapshot API (`snapshot.rs` defines `GameSnapshot`; find
> the existing `round_trip`/serialize test helper the mission slice used and reuse it). The
> assertion content (skipped fields return Default, hash unchanged) is the load-bearing part.

---

### P2-T7 — Pin `SNAPSHOT_VERSION == 17` (no-bump lock)

**File (EDIT):** the `world`-level (or `snapshot.rs`) test module — add a test that locks "no
version bump in the shadow phase". `SNAPSHOT_VERSION` is private (`snapshot.rs:24`); add the
test inside `snapshot.rs`'s own `#[cfg(test)] mod tests` (or expose a `pub(crate) const` /
`#[cfg(test)]` getter if no test module exists there — prefer the in-file test module).

```rust
    #[test]
    fn snapshot_version_is_17_in_shadow_phase() {
        // P1+P2 are additive #[serde(skip)] shadow with no serde derive on the new
        // types — the bincode layout is byte-identical, so the version must NOT bump.
        // The 17->18 bump lands at the authority flip (P5), out of P1+P2 scope.
        assert_eq!(super::SNAPSHOT_VERSION, 17);
    }
```

**Verification (P2-T6 + P2-T7 combined):**
- `cargo check -p vera20k`
- `cargo test -p vera20k factory` — runs the `factory.rs` pure-type tests:
  `special_item_none_zero_and_neg1_distinct`, `factory_default_progress_zero_no_object`,
  `registry_iter_insertion_ordered_not_map_order`
- `cargo test -p vera20k factory_shadow` — runs:
  `factory_shadow_progress_tracks_legacy_remaining`,
  `factory_registry_iteration_is_insertion_ordered`, `insertion_seq_stable_across_rebuild`,
  `factory_registry_shadow_no_hash_change`, `production_shadow_does_not_create_houses`,
  `factory_shadow_trace_order_matches_logic_vector`, `snapshot_roundtrip_ignores_shadow`,
  `production_shadow_preserves_advance_tick_phase_order`
- `cargo test -p vera20k snapshot_version_is_17_in_shadow_phase`

---

## E. Open questions for the design-lead (confirm before implementing P2-T3)

**E1 — `balance`/`step_rate_frames` derive shape (BLOCKS only P2-T3's field math).**
`BuildQueueItem` has no `cost` field (A6) and `rebuild_shadow(&Simulation)` carries no
`&RuleSet`. The plan defaults to a **frames-based** projection (`original_balance =
total_base_frames`, `balance = remaining_base_frames`, `step_rate_frames = 0`), which satisfies
the design's monotone-tracking asserts (§4.2 / U2) with the lowest surface. If you want the
exact `cost - progress*cost/54` shape and a real `step_rate_frames = clamp(total/54, 1, 255)`,
`rebuild_shadow` must take `(&Simulation, &RuleSet)` (one-line signature + call-site change at
P2-T5, and `refresh_production_shadow` already has `rules` in hand). **Pick:** frames-based
(default) or cost-based-with-rules.

**E2 — Where the `state_hash`/`advance_tick`/snapshot tests live.** The pure-type tests go in
`factory.rs`/`economy.rs`. The hash-bearing tests need `Simulation::new()` + `state_hash` +
`set_logic_order_for_test` + the snapshot API; the proven home is a `world`-level `#[cfg(test)]`
module (where `techno_ai.rs`'s no-hash tests live). Confirm whether to (a) add them to the
existing `techno_ai.rs mod tests`, or (b) create a small `src/sim/world/production_shadow_tests.rs`
wired via `#[cfg(test)] mod production_shadow_tests;` in `world/mod.rs` (cleaner separation,
matches the mission slice's `mission_authoritative_tests.rs`). Plan assumes (b).

**E3 — `StepOutcome`/`BuildEligibility`/consts unused-warning policy.** They are documented P3/P6
seams with no caller in P1+P2. Confirm whether to `#[allow(dead_code)]` them now or accept the
warnings until their slice lands.

**E4 — snapshot round-trip test API.** The plan references the snapshot serialize/deserialize
helper by placeholder name. Confirm the exact helper the mission slice's
`snapshot_roundtrip_ignores_shadow`-equivalent used so P2-T6 calls the real API.

---

*End of P1+P2 plan. Strictly shadow, `#[serde(skip)]` + no serde derive on the new types,
zero hashed bits: `world_hash.rs`/`snapshot.rs` are untouched and `SNAPSHOT_VERSION` stays 17.
The authority flip, per-step charge, cancel/refund, prereq revalidation, purifier fix, ordering
lock, and parity harness are P3-P9, out of scope — declared as seams only.*
