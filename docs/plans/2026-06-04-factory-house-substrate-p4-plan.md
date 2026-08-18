<!--
Provenance: authored 2026-06-04 from the APPROVED design
  docs/plans/2026-06-04-factory-house-substrate-p4-design.md
  (D2 substrate-fit winner, grafted with D3 structural-no-hash + D1 tiny-detail ledger;
   the three P4 DECISIONS: (a) queued-tail-first / active-fallback precedence + NoMatch,
   (b) start_next_queued PROVES front-pop + held-object guard + the C7/C12 negative invariant,
   DEFERS the delivery binding to P5, (c) refund = original_balance - balance against an ORACLE),
  grounded in the v2-verified study
  docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md (C6 line 421, C7 line 423,
  C8 line 425, C12 line 433, C15 line 439; §6.2 line 593-596; §8 P4 line 735-741; §9.1).
House style mirrored from docs/plans/2026-06-04-factory-house-substrate-p3-plan.md.
Status: DRAFTED, not approved or executed. Review (/review-plan) before implementing.
Scope: P4 ONLY — `enum CancelOutcome`, `FactoryRegistry::cancel_one` (C6 first-match queued
  removal / C8 active AbandonProduction partial refund), `Factory::cancel_active` (the refund +
  reset primitive), `Factory::start_next_queued` (FIFO front-pop with the C7/C12 held-object
  guard, PROVEN in isolation, NOT wired to delivery), and the cancel-conservation shadow-assert.
  HASH-NEUTRAL: cancel/refund/advance run against an ORACLE (clone) economy + clone registry,
  never the hashed wallet. The legacy `cancel_by_type_for_owner` stays AUTHORITATIVE (its
  `.rev()` last-match + full-cost refund are the verified DRIFTs P4 models CORRECTLY in the
  shadow). world_hash.rs UNTOUCHED; SNAPSHOT_VERSION STAYS 17.
  OUT OF SCOPE (seams only): authority flip + fixing legacy cancel + 17->18 (P5), the delivery
  command that drives start_next_queued (P5+), the post-AbandonProduction auto-StartNextQueued
  (P5), per-step charge (DONE P3), prereq revalidation (P6), purifier/IncomeMult (P7).
-->

# Factory/House Substrate — P4 Plan (FIFO queue + cancel + partial refund, hash-neutral oracle)

> Linear path: **P4-T1 → P4-T2 → P4-T3 → P4-T4 → P4-T5 → P4-T6 → P4-T7**.
> Every task builds green (`cargo check -p vera20k`) before the next. The hash-neutrality test
> (`factory_cancel_one_does_not_change_state_hash`) + the version pin
> (`snapshot_version_is_17_in_shadow_phase`, snapshot.rs:374) are the contract gate: if either
> fails after a task, STOP — the oracle leaked into the hashed wallet or a serde derive crept in.
>
> **#1 invariant preserved:** `sim/production/factory.rs` depends only on `std` + `sim/` (intern,
> production_types, economy, rules data through `&RuleSet`); NEVER on render/ui/sidebar/audio/net.
>
> **No-hash contract (the whole point of P4):** `cancel_one`/`cancel_active`/`start_next_queued`
> mutate only a `Factory`/`FactoryRegistry` + an `Economy`, NONE of which `state_hash()` visits
> (no serde derive on `Factory`/`FactoryRegistry`/`Economy`/`CancelOutcome`; the registry lives
> in the `#[serde(skip)]` `factory_shadow`). The new methods have **NO authoritative `advance_tick`
> call site** — P4 only ever passes a `clone()` of the registry + a `clone()` of an economy (or a
> test-local one). The legacy `cancel_by_type_for_owner` (production_queue.rs:794, the `.rev()`
> last-match + full-cost-refund DRIFT) stays authoritative, untouched. `world_hash.rs` is NOT
> touched. `SNAPSHOT_VERSION` stays **17** (snapshot.rs:24). The 17→18 authority flip is P5.

---

## A. Verified preconditions (live reads this session — quote file:TEXT)

The tree shifts (a concurrent session edits miner/combat/movement/unit_post); anchor on the quoted
TEXT, not the line number.

| # | Fact the plan relies on | Verified at (text anchor) |
|---|---|---|
| A1 | `Factory` carries `progress: u16`, `step_rate_frames: u16`, `step_timer: u16`, `balance: i32`, `original_balance: i32`, `object: Option<PendingObject>`, `on_hold/suspended/manual: bool`, `special: SpecialItem`, `queue: VecDeque<InternedId>`, `insertion_seq: u64` — exactly the fields `cancel_active`/`start_next_queued` mutate | factory.rs `pub struct Factory {` … `pub insertion_seq: u64,` |
| A2 | `Factory` derives `#[derive(Debug, Clone, Default, PartialEq, Eq)] // NO serde in P1-P3` — `clone()` is available; no serde | factory.rs `#[derive(Debug, Clone, Default, PartialEq, Eq)] // NO serde in P1-P3` above `pub struct Factory` |
| A3 | `PendingObject { pub type_id: InternedId, pub entity_id: Option<u64> }`, derives `Debug, Clone, Default, PartialEq, Eq` (no serde); in P2-P4 shadow `entity_id` is always `None` | factory.rs `pub struct PendingObject {` |
| A4 | `SpecialItem::NoneNeg1` is the canonical "none" (`Default`); the 0/-1 collapse is forbidden | factory.rs `pub enum SpecialItem {` + `impl Default for SpecialItem` |
| A5 | `StepOutcome { Idle, Stepped, Stalled, Completed }` derives `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` (serde-free) — the exact derive line `CancelOutcome` copies | factory.rs `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` above `pub enum StepOutcome` |
| A6 | `PRODUCTION_STEPS: u16 = 54`, `STEP_RATE_MIN/MAX` already declared | factory.rs `pub const PRODUCTION_STEPS: u16 = 54;` |
| A7 | the module is `#![allow(dead_code)]` — the new dormant methods (no authoritative caller in P4) raise no unused-warning | factory.rs `#![allow(dead_code)]` |
| A8 | `Factory::advance_one_step(&mut self, economy: &mut Economy) -> StepOutcome` exists (P3) — reused verbatim by the conservation/round-trip tests | factory.rs `pub fn advance_one_step(&mut self, economy: &mut Economy) -> StepOutcome {` |
| A9 | `Economy::add_credits(&mut self, amount: i32)` saturating-adds; `spend`/`available` exist — REUSE for the oracle refund, do not add methods | economy.rs `pub fn add_credits(&mut self, amount: i32) {` |
| A10 | `FactoryRegistry { factories: BTreeMap<(InternedId, ProductionCategory), Factory>, next_insertion_seq, seq_carry }` — `factories` is PRIVATE; `cancel_one` is an `impl FactoryRegistry` method so it can `self.factories.get_mut(&key)` | factory.rs `pub struct FactoryRegistry {` + `factories: BTreeMap<...>` |
| A11 | `FactoryRegistry::iter_insertion_ordered(&self) -> Vec<&Factory>`, `view(owner, category) -> Option<FactoryView>` — read-only accessors the assert/tests reuse | factory.rs `pub fn iter_insertion_ordered(&self) -> Vec<&Factory> {` |
| A12 | `rebuild_shadow` puts the active object in `Factory.object` and the TAIL behind it (`queue.iter().skip(1)`) in `Factory.queue` — so the active object is NOT an element of `queue` (the C6 split) | factory.rs `let tail: VecDeque<InternedId> = queue.iter().skip(1).map(|item| item.type_id).collect();` |
| A13 | `production/mod.rs` re-exports `StepOutcome`/`PRODUCTION_STEPS` etc. via `pub use self::factory::{...}` — `CancelOutcome` is added to that list | production/mod.rs `pub use self::factory::{ … StepOutcome, PRODUCTION_STEPS, …};` |
| A14 | `debug_assert_production_shadow(&self)` (#[cfg(debug_assertions)]) calls `debug_assert_economy_shadow()` → `debug_assert_factory_shell_trace()` → `debug_assert_factory_conservation()` (P3); the cancel assert slots in beside the P3 one | world/mod.rs `pub(crate) fn debug_assert_production_shadow(&self) {` |
| A15 | the P3 `debug_assert_factory_conservation` is the template: clone factory + clone economy seeded with `original_balance`, step, SURFACE with `tick + owner + category`, NEVER write back | world/mod.rs `pub(crate) fn debug_assert_factory_conservation(&self) {` |
| A16 | the P4 test fixtures (`empty_rules()`, `queued_item(..)`, `insert_queue(..)`) already exist; `Economy` imported at `use crate::sim::economy::Economy;`; the production import line imports `BuildQueueItem, BuildQueueState, ProductionCategory, PRODUCTION_STEPS` | production_shadow_tests.rs `fn empty_rules() -> RuleSet {` / `use crate::sim::production::{BuildQueueItem, BuildQueueState, ProductionCategory, PRODUCTION_STEPS};` |
| A17 | the P3 no-hash acceptance test `factory_advance_step_does_not_change_state_hash` (clone-step-against-clone, assert `before == sim.state_hash()` + legacy wallet untouched) is the structural template the P4 acceptance test mirrors | production_shadow_tests.rs `fn factory_advance_step_does_not_change_state_hash() {` |
| A18 | `HouseState::new(name, side_index, country, is_human, credits, tech_level)` is the test fixture ctor; `sim.houses[&owner].economy.clone()` is the oracle source; `sim.houses[&owner].credits` is the legacy wallet | production_shadow_tests.rs `sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));` |
| A19 | `SNAPSHOT_VERSION == 17` and the version-pin test `snapshot_version_is_17_in_shadow_phase` exist — P4 must NOT bump | snapshot.rs `const SNAPSHOT_VERSION: u32 = 17;` / `fn snapshot_version_is_17_in_shadow_phase() {` |
| A20 | the legacy `cancel_by_type_for_owner` uses `.rev()` (last-match, DRIFT) and refunds `obj.cost.max(0)` (FULL cost, DRIFT); it stays authoritative through P5 — P4 does NOT touch it | production_queue.rs `.rev()` in `cancel_by_type_for_owner` + `*credits_entry_for_owner(sim, owner) += obj.cost.max(0);` |
| A21 | `BuildQueueState { Queued, Building, NoFunds, Paused, Done }`; `rebuild_shadow` sets `object = Some` for `Building | NoFunds | Done`, `None` for `Queued | Paused` | production_types.rs `pub enum BuildQueueState {` + factory.rs `let has_object = matches!(front.state, S::Building | S::NoFunds | S::Done);` |

**Facts the design pins from the study (no re-decompile in P4 — VERIFIED-LIVE v2):**
- **C6 (study line 421):** the queue is FIFO; `StartNextQueued` (0x004CA5A0) pops the FRONT;
  `RemoveFromQueue` (0x004CA620, cancel-one) removes the **FIRST** matching type FRONT-TO-BACK.
- **C7 (study line 423):** a queued item starts only after a successful **delivery command**
  (`CompletedProduction` 0x004CA1A0 has no begin/next call; the advance is `FUN_004FAA10`'s
  post-delivery `StartNextQueued`). Delivery is command-bound (P5+).
- **C8 (study line 425):** cancel refunds only the already-paid amount (`GetCost − Balance`), NOT
  the full cost. In the Rust model `balance` is the remaining-unpaid, so the refund =
  `original_balance − balance` (the spent portion).
- **C12 (study line 433):** completion sets the factory **suspended with the object still
  attached** (`balance = 0`); the object stays pending until a separate delivery succeeds.
  `advance_one_step` (P3) ALREADY implements the completion half (sets `suspended`, leaves
  `object = Some`, returns `Completed`).
- **C15 (study line 439):** with a mid-build cancel, total removed equals the spent portion and
  the refund returns it (`Σ per-step charge + refund = original_balance`, exact telescoping).
- **§6.2 (study line 593-596):** `cancel_one(&mut self, owner, category, type_id, &mut Economy)`
  removes "the FIRST matching queued type (front-to-back), **OR** abandon the active object with
  partial refund" — an OR with the queued path named FIRST (the precedence DECISION a).
- **§8 P4 (study line 739-741):** the three named tests — `cancel_one_removes_first_matching`
  (`[A,B,A,C]` cancel A → `[B,A,C]`), `cancel_active_refunds_spent_only` (progress 20, refund the
  spent portion, credits return to pre-build), `queue_advances_only_after_delivery`.

---

## B. Files touched (summary)

| File | Change | Task |
|---|---|---|
| `src/sim/production/factory.rs` | `enum CancelOutcome` (serde-free, mirror `StepOutcome`); `Factory::cancel_active(&mut self, &mut Economy) -> Option<i32>` (private, the C8 refund+reset primitive); `Factory::start_next_queued(&mut self) -> Option<InternedId>` (FIFO front-pop + held-object guard); `FactoryRegistry::cancel_one(owner, category, type_id, &mut Economy) -> CancelOutcome` (C6 queued-first / C8 active-fallback); the §7 unit tests | P4-T1, P4-T2, P4-T3 |
| `src/sim/production/mod.rs` | add `CancelOutcome` to the `pub use self::factory::{...}` re-export | P4-T4 |
| `src/sim/world/mod.rs` | add `debug_assert_factory_cancel_refund`; one call line into `debug_assert_production_shadow` (beside the P3 `debug_assert_factory_conservation`) | P4-T5 |
| `src/sim/world/production_shadow_tests.rs` | `factory_cancel_one_does_not_change_state_hash` (acceptance), `queue_advances_only_after_delivery`, `production_shadow_with_cancel_is_deterministic` (reuse `empty_rules`/`queued_item`/`insert_queue`) | P4-T6 |

`world_hash.rs` and `snapshot.rs` are **NOT** in this list — that is the no-hash contract.
No miner/combat/movement/unit_post file is touched (concurrent session owns those). The legacy
`production_queue.rs` cancel is **NOT** touched (stays authoritative + wrong, fixed P5).

---

## C. P4 — the cancel/refund primitives + the FIFO front-pop

### P4-T1 — `Factory::cancel_active` (C8 partial refund + reset)

**File (EDIT):** `src/sim/production/factory.rs` — add to the existing `impl Factory` block
(the one that already holds `set_rate` + `advance_one_step`), after `advance_one_step`. Integer
math only; no float, no RNG; no engine addresses in comments. `Economy` is already imported
(`use crate::sim::economy::Economy;` at the top of the file, A9-adjacent).

```rust
    /// AbandonProduction the ACTIVE object (C8): refund the ALREADY-PAID portion
    /// (`original_balance - balance`, the spent credits) to the (oracle) economy,
    /// then reset to the empty-but-registered idle state (the partial object is
    /// destroyed). Returns `Some(refund)` when it ACTED (refund may be 0 for a
    /// not-yet-charged build) and `None` when it was a NO-OP — no active object, OR
    /// a complete-but-held object (the "no-op after completion" rule: a finished but
    /// undelivered build is cancelled through the ready-queue path, a later slice).
    /// Leaves the queue tail INTACT — the next-queue advance (`start_next_queued`) is
    /// command-bound and is NOT auto-invoked here.
    ///
    /// `&mut Economy` is an ORACLE (clone) in P4; hash-neutrality is enforced at the
    /// CALL SITE, never in this body. The authority-flip slice flips WHO is passed.
    fn cancel_active(&mut self, economy: &mut Economy) -> Option<i32> {
        // No active object -> no-op.
        self.object.as_ref()?;

        // No-op after completion: a complete-but-held object (progress 54, suspended,
        // object attached) is NOT abandoned via this path — it is awaiting delivery,
        // and cancelling a completed build goes through the ready-queue path (a later
        // slice). Returning None here keeps the completed object + its state intact.
        if self.progress >= PRODUCTION_STEPS {
            return None;
        }

        // C8: refund the already-paid (spent) portion. `balance` is the remaining
        // unpaid amount, charged down per step; `original_balance` is the full-cost
        // snapshot. `original_balance - balance` is therefore exactly what the per-step
        // ladder removed (NOT the full cost — that is the legacy DRIFT). `.max(0)`
        // documents intent and guards a malformed shadow; the invariant
        // `balance <= original_balance` holds (the stepper only decrements balance), so
        // it never fires in a well-formed shadow.
        let refund = (self.original_balance - self.balance).max(0);
        economy.add_credits(refund); // ORACLE economy in P4 (saturating add)

        // Reset to the empty-but-registered idle state; the partial object is destroyed.
        // In the P4 shadow `object.entity_id` is always None (the legacy path owns the
        // produced entity), so "destroy the partial object" is exactly `object = None`;
        // the real partial-object despawn hooks in at the authority-flip slice.
        self.object = None;
        self.progress = 0;
        self.balance = 0;
        self.original_balance = 0;
        self.step_rate_frames = 0; // no-object => rate-0 sentinel (matches set_rate)
        self.step_timer = 0;
        self.on_hold = false;
        self.suspended = false;
        self.manual = false;
        self.special = SpecialItem::NoneNeg1; // canonical "none"; do NOT collapse 0/-1
        // `self.queue` is LEFT INTACT — StartNextQueued is command-bound (C7), a later slice.
        Some(refund)
    }
```

**Why `Option<i32>` and not a bare `i32`:** `cancel_active` must distinguish "acted (refunded +
reset)" from "no-op (completed / no object)" WITHOUT overloading the `0` refund — a progress-0
active object legitimately refunds 0 but DID act (it reset the factory + destroyed the partial
object). `Some(0)` = acted-with-zero-refund; `None` = did-not-act. `cancel_one` (P4-T2) maps
`Some(r) -> AbandonedActive { refund: r }`, `None -> NoMatch`.

**Why `original_balance − balance` and NOT the full cost:** `original_balance` is the full-cost
snapshot set in `rebuild_shadow`; `balance` is the remaining-unpaid, charged down per step in
`advance_one_step`. Their difference is the spent portion = `GetCost − Balance` (C8). The refund
is the exact arithmetic complement of `Σ per-step charge`, so `Σ spent + refund = original_balance`
exactly (telescoping, C15) — no rounding, both terms are exact `i32`. The legacy
`cancel_by_type_for_owner` refunds the FULL `obj.cost` (A20) — the DRIFT P4 models correctly here.

**Unit tests** (append to the `factory.rs` `mod tests`, reusing the existing `armed_factory(cost)`
helper that sets `object = Some, balance = cost, original_balance = cost`):

```rust
    #[test]
    fn cancel_active_refunds_spent_only() {
        // Step an armed cost-700 build to progress 20, then cancel the active object.
        // The refund equals the SPENT portion (original_balance - balance), and the
        // oracle returns to its pre-build credits (C8/C15). The factory resets to idle.
        let mut f = armed_factory(700);
        let mut econ = Economy { credits: 700, ..Economy::default() };
        while f.progress < 20 {
            assert!(matches!(f.advance_one_step(&mut econ), StepOutcome::Stepped));
        }
        let spent = econ.spent_credits; // what the ladder removed by progress 20
        let expected_refund = f.original_balance - f.balance;
        assert_eq!(expected_refund, spent, "spent portion == original_balance - balance");
        let refund = f.cancel_active(&mut econ).expect("active build is abandonable");
        assert_eq!(refund, spent, "C8: refund the already-paid spent portion only");
        assert_eq!(econ.credits, 700, "C15: oracle returns to pre-build credits");
        // Factory reset to idle: object destroyed, progress/balance zeroed.
        assert!(f.object.is_none(), "the partial object is destroyed");
        assert_eq!(f.progress, 0);
        assert_eq!(f.balance, 0);
        assert_eq!(f.original_balance, 0);
        assert_eq!(f.step_rate_frames, 0, "no-object => rate-0 sentinel");
        assert!(!f.suspended && !f.on_hold && !f.manual);
    }

    #[test]
    fn cancel_active_at_progress_zero_refunds_nothing() {
        // A never-stepped active object (progress 0, balance == original_balance) ACTED
        // but refunds 0 (the spent portion is 0) — Some(0), NOT None.
        let mut f = armed_factory(700);
        let mut econ = Economy { credits: 0, ..Economy::default() };
        let refund = f.cancel_active(&mut econ);
        assert_eq!(refund, Some(0), "acted, refund 0 (spent nothing yet)");
        assert_eq!(econ.credits, 0, "no credits added for a zero refund");
        assert!(f.object.is_none(), "factory reset even on a zero-refund cancel");
        assert_eq!(f.progress, 0);
    }

    #[test]
    fn cancel_active_no_object_is_noop() {
        // No active object -> None, nothing touched.
        let mut f = Factory::default();
        let mut econ = Economy { credits: 500, ..Economy::default() };
        assert_eq!(f.cancel_active(&mut econ), None);
        assert_eq!(econ.credits, 500, "no-op leaves the oracle untouched");
    }

    #[test]
    fn cancel_active_completed_is_noop() {
        // A complete-but-held object (progress 54, suspended, object attached) is NOT
        // abandoned via this path -> None; the completed object + state stay intact
        // (the "no-op after completion" rule; the ready-queue cancel is a later slice).
        let mut f = armed_factory(700);
        let mut econ = Economy { credits: 700, ..Economy::default() };
        loop {
            if matches!(f.advance_one_step(&mut econ), StepOutcome::Completed) {
                break;
            }
        }
        assert_eq!(f.progress, PRODUCTION_STEPS);
        assert!(f.suspended && f.object.is_some(), "completed-but-held");
        let credits_before = econ.credits;
        assert_eq!(f.cancel_active(&mut econ), None, "no-op after completion");
        assert_eq!(econ.credits, credits_before, "no refund on a completed build");
        assert!(f.object.is_some(), "the completed object is NOT destroyed");
        assert_eq!(f.progress, PRODUCTION_STEPS, "progress unchanged");
    }

    #[test]
    fn cancel_active_round_trip_conserves() {
        // C15 cancel-side telescoping: for each cost x mid-build progress, step k times
        // against an oracle seeded with exactly the cost, then cancel — the oracle must
        // return to its starting credits regardless of where the cancel lands.
        for cost in [1i32, 25, 700, 99991] {
            for stop_at in [0u16, 1, 20, 53] {
                let mut f = armed_factory(cost);
                let mut econ = Economy { credits: cost, ..Economy::default() };
                while f.progress < stop_at {
                    if !matches!(f.advance_one_step(&mut econ), StepOutcome::Stepped) {
                        break; // a free (cost-0-ish) build may Complete early; harmless
                    }
                }
                // Only an in-progress (not-completed) build is abandonable here.
                if f.object.is_some() && f.progress < PRODUCTION_STEPS {
                    let _ = f.cancel_active(&mut econ);
                    assert_eq!(
                        econ.credits, cost,
                        "cost {cost} stop {stop_at}: cancel returns the oracle to start"
                    );
                }
            }
        }
    }
```

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k cancel_active_refunds_spent_only cancel_active_at_progress_zero_refunds_nothing cancel_active_no_object_is_noop cancel_active_completed_is_noop cancel_active_round_trip_conserves`

---

### P4-T2 — `enum CancelOutcome` + `FactoryRegistry::cancel_one` (C6/C8)

**File (EDIT):** `src/sim/production/factory.rs`

**(i)** Add the `CancelOutcome` enum next to `StepOutcome` (serde-free, the same derive line):

```rust
/// Outcome of a `FactoryRegistry::cancel_one` (consumer: tests + the P4
/// cancel-conservation assert). Serde-free — the same no-hash discipline as
/// `StepOutcome`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)] // NO serde
pub enum CancelOutcome {
    /// No factory for (owner, category), OR the type matched neither a queued tail
    /// copy nor an abandonable active object (including the complete-but-held case).
    /// A true no-op: zero economy mutation, zero state change.
    NoMatch,
    /// A queued tail copy of `type_id` was removed (FIRST match, front-to-back). No
    /// refund — a queued item was never charged (its spent portion is 0).
    QueuedRemoved,
    /// The active object was AbandonProduction'd; `refund` credits returned (C8).
    AbandonedActive { refund: i32 },
}
```

**(ii)** Add `cancel_one` to the `impl FactoryRegistry` block (after `rebuild_shadow_no_rules`, or
anywhere in the block — it needs private access to `self.factories`):

```rust
    /// Cancel one production of `type_id` for (owner, category) — the substrate
    /// analog of the engine's cancel-one command. PURE on the registry + an ORACLE
    /// (clone) economy in P4 (never the hashed wallet; the legacy
    /// `cancel_by_type_for_owner` stays authoritative through the authority-flip
    /// slice). Precedence (C6/§6.2 OR-signature, queued path named first): a QUEUED
    /// tail copy is removed FIRST (front-to-back, FIRST match — the RemoveFromQueue
    /// path); ONLY when no queued copy of `type_id` matches AND the ACTIVE object is
    /// `type_id` is the active build abandoned (refund = original_balance - balance,
    /// the AbandonProduction path). No match -> NoMatch.
    ///
    /// `&mut Economy` is an ORACLE (clone) in P4; the authority-flip slice flips WHO
    /// is passed, not this body.
    pub fn cancel_one(
        &mut self,
        owner: InternedId,
        category: ProductionCategory,
        type_id: InternedId,
        economy: &mut Economy,
    ) -> CancelOutcome {
        // (R0) the one factory for this (owner, category). None -> NoMatch.
        let Some(f) = self.factories.get_mut(&(owner, category)) else {
            return CancelOutcome::NoMatch;
        };

        // (R1) QUEUED TAIL FIRST — RemoveFromQueue (C6): the FIRST front-to-back match.
        //   `VecDeque::iter().position()` scans front-to-back and returns the FIRST
        //   index; `VecDeque::remove(idx)` removes it and shifts survivors down
        //   (relative order preserved). This is the DRIFT fix vs the legacy `.rev()`
        //   last-match: `[A,B,A,C]` cancel A -> remove index 0 -> `[B,A,C]`.
        if let Some(idx) = f.queue.iter().position(|&t| t == type_id) {
            f.queue.remove(idx);
            return CancelOutcome::QueuedRemoved; // no refund: a queued item is uncharged
        }

        // (R2) ELSE the ACTIVE object, if it is this type AND abandonable: AbandonProduction.
        //   `cancel_active` no-ops (returns None) on a complete-but-held object, in
        //   which case we report NoMatch (the "no-op after completion" rule).
        if f.object.as_ref().map(|o| o.type_id) == Some(type_id) {
            return match f.cancel_active(economy) {
                Some(refund) => CancelOutcome::AbandonedActive { refund },
                None => CancelOutcome::NoMatch, // completed-held / nothing to abandon
            };
        }

        // (R3) no queued copy, active object is a different type (or none) -> no-op.
        CancelOutcome::NoMatch
    }
```

**Precedence grounding (DECISION a):** the active object lives in `Factory.object`; the tail
behind it lives in `Factory.queue` (A12 — the active object is NOT a `queue` element). So R1
scans only the queued tail, R2 only the active slot — they are mutually exclusive call paths
(the engine's `RemoveFromQueue` vs `AbandonProduction`), not a fallthrough that could double-act.
The §6.2 OR-signature names the queued path first; the faithful single-entry reproduction prefers
the queued removal and abandons the active only when no queued copy of that type remains. This
reproduces the observable cadence — a right-click on a cameo with a count badge drops the queue
count before the in-progress build's progress bar resets.

**Unit tests** (append to `factory.rs` `mod tests`; a small registry helper keeps them readable —
the `factories` map is private but in-module so the test can insert directly):

```rust
    /// Insert a factory at (owner, category) into a registry (test helper; the
    /// `factories` map is private but in-module).
    fn reg_with(owner: InternedId, category: ProductionCategory, f: Factory) -> FactoryRegistry {
        let mut reg = FactoryRegistry::default();
        reg.factories.insert((owner, category), f);
        reg
    }

    #[test]
    fn cancel_one_removes_first_matching() {
        // queue [A,B,A,C] (all queued, no active object), cancel A -> [B,A,C]:
        // the FIRST (front-most) A is removed, NOT the last (the legacy .rev() DRIFT).
        let owner = InternedId::default();
        let a = InternedId::from_index(1);
        let b = InternedId::from_index(2);
        let c = InternedId::from_index(3);
        let f = Factory {
            owner,
            category: ProductionCategory::Vehicle,
            queue: VecDeque::from(vec![a, b, a, c]),
            object: None,
            ..Factory::default()
        };
        let mut reg = reg_with(owner, ProductionCategory::Vehicle, f);
        let mut econ = Economy::default();
        let outcome = reg.cancel_one(owner, ProductionCategory::Vehicle, a, &mut econ);
        assert_eq!(outcome, CancelOutcome::QueuedRemoved);
        assert_eq!(econ.credits, 0, "a queued removal refunds nothing");
        let q: Vec<InternedId> = reg
            .view(owner, ProductionCategory::Vehicle)
            .unwrap()
            .queue
            .iter()
            .copied()
            .collect();
        assert_eq!(q, vec![b, a, c], "first A removed -> [B,A,C]");
    }

    #[test]
    fn cancel_one_queued_preferred_over_active_same_type() {
        // active = A (mid-build), tail = [A]; cancel A removes the TAIL copy
        // (QueuedRemoved), the active build is UNTOUCHED (queued-first precedence).
        let owner = InternedId::default();
        let a = InternedId::from_index(1);
        let mut f = Factory {
            owner,
            category: ProductionCategory::Vehicle,
            object: Some(PendingObject { type_id: a, entity_id: None }),
            balance: 300,
            original_balance: 700,
            progress: 20,
            queue: VecDeque::from(vec![a]),
            ..Factory::default()
        };
        f.suspended = false;
        let mut reg = reg_with(owner, ProductionCategory::Vehicle, f);
        let mut econ = Economy { credits: 1000, ..Economy::default() };
        let outcome = reg.cancel_one(owner, ProductionCategory::Vehicle, a, &mut econ);
        assert_eq!(outcome, CancelOutcome::QueuedRemoved, "tail copy removed first");
        assert_eq!(econ.credits, 1000, "no refund (queued removal)");
        let view = reg.view(owner, ProductionCategory::Vehicle).unwrap();
        assert!(view.queue.is_empty(), "the one tail copy is gone");
        assert!(view.object.is_some(), "the active build is untouched");
        assert_eq!(view.progress, 20, "active progress unchanged");
    }

    #[test]
    fn cancel_one_active_when_no_queued_copy() {
        // active = A (mid-build), tail = [B]; cancel A abandons the ACTIVE (no queued A).
        let owner = InternedId::default();
        let a = InternedId::from_index(1);
        let b = InternedId::from_index(2);
        let f = Factory {
            owner,
            category: ProductionCategory::Vehicle,
            object: Some(PendingObject { type_id: a, entity_id: None }),
            balance: 300,
            original_balance: 700,
            progress: 20,
            queue: VecDeque::from(vec![b]),
            ..Factory::default()
        };
        let mut reg = reg_with(owner, ProductionCategory::Vehicle, f);
        let mut econ = Economy { credits: 0, ..Economy::default() };
        let outcome = reg.cancel_one(owner, ProductionCategory::Vehicle, a, &mut econ);
        assert_eq!(
            outcome,
            CancelOutcome::AbandonedActive { refund: 400 },
            "spent portion = original_balance 700 - balance 300 = 400"
        );
        assert_eq!(econ.credits, 400, "the spent portion is refunded to the oracle");
        let view = reg.view(owner, ProductionCategory::Vehicle).unwrap();
        assert!(view.object.is_none(), "active object abandoned");
        let q: Vec<InternedId> = view.queue.iter().copied().collect();
        assert_eq!(q, vec![b], "the tail is left intact (no auto-advance in P4)");
    }

    #[test]
    fn cancel_one_completed_active_is_noop() {
        // active object completed-but-held (progress 54, suspended), no queued copy:
        // cancel the active type -> NoMatch, factory unchanged.
        let owner = InternedId::default();
        let a = InternedId::from_index(1);
        let f = Factory {
            owner,
            category: ProductionCategory::Vehicle,
            object: Some(PendingObject { type_id: a, entity_id: None }),
            progress: PRODUCTION_STEPS,
            suspended: true,
            balance: 0,
            original_balance: 700,
            ..Factory::default()
        };
        let mut reg = reg_with(owner, ProductionCategory::Vehicle, f);
        let mut econ = Economy { credits: 100, ..Economy::default() };
        let outcome = reg.cancel_one(owner, ProductionCategory::Vehicle, a, &mut econ);
        assert_eq!(outcome, CancelOutcome::NoMatch, "no-op after completion");
        assert_eq!(econ.credits, 100, "no refund on a completed build");
        let view = reg.view(owner, ProductionCategory::Vehicle).unwrap();
        assert!(view.object.is_some(), "completed object NOT destroyed");
        assert_eq!(view.progress, PRODUCTION_STEPS);
    }

    #[test]
    fn cancel_one_no_match_is_noop() {
        // (1) no factory for the key -> NoMatch; (2) type absent from both active and
        // tail -> NoMatch. Zero economy/state change either way.
        let owner = InternedId::default();
        let a = InternedId::from_index(1);
        let z = InternedId::from_index(9);
        let mut empty = FactoryRegistry::default();
        let mut econ = Economy { credits: 50, ..Economy::default() };
        assert_eq!(
            empty.cancel_one(owner, ProductionCategory::Vehicle, a, &mut econ),
            CancelOutcome::NoMatch,
            "no factory -> NoMatch"
        );
        // present factory, but z is neither active nor queued.
        let f = Factory {
            owner,
            category: ProductionCategory::Vehicle,
            object: Some(PendingObject { type_id: a, entity_id: None }),
            balance: 300,
            original_balance: 700,
            queue: VecDeque::from(vec![a]),
            ..Factory::default()
        };
        let mut reg = reg_with(owner, ProductionCategory::Vehicle, f);
        assert_eq!(
            reg.cancel_one(owner, ProductionCategory::Vehicle, z, &mut econ),
            CancelOutcome::NoMatch,
            "type absent -> NoMatch"
        );
        assert_eq!(econ.credits, 50, "a no-op cancel never touches credits");
    }
```

> **Confirm at impl time:** the `InternedId` constructors used by the tests
> (`InternedId::default()`, `InternedId::from_index(n)`). If `from_index` is not the public ctor,
> mint ids via a `StringInterner`/`Simulation` as the existing `registry_iter_insertion_ordered_not_map_order`
> test does with `InternedId::default()` — or intern distinct names through a local interner. Use
> whatever the current `InternedId` API exposes; the test only needs DISTINCT ids for A/B/C.

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k cancel_one_removes_first_matching cancel_one_queued_preferred_over_active_same_type cancel_one_active_when_no_queued_copy cancel_one_completed_active_is_noop cancel_one_no_match_is_noop`

---

### P4-T3 — `Factory::start_next_queued` (FIFO front-pop + held-object guard, C6/C7/C12)

**File (EDIT):** `src/sim/production/factory.rs` — add to the `impl Factory` block, after
`cancel_active`.

```rust
    /// Pop the FRONT of the queue into a fresh active object (FIFO StartNextQueued,
    /// C6). Returns the popped `type_id`, or `None` when blocked/empty. PROVEN-but-
    /// DORMANT in P4: no `advance_tick`/command path calls this — the queue advance is
    /// command-bound to a successful delivery (C7), wired in a later slice. P4 only
    /// proves the pure pop mechanics + the gating guard.
    ///
    /// GUARD (C7/C12): a held object blocks the advance. A completed-but-held factory
    /// (progress 54, suspended, object attached) is a NO-OP here — the queue does not
    /// advance on completion alone; the delivery commit clears the object first.
    fn start_next_queued(&mut self) -> Option<InternedId> {
        // "Object null required" precondition: an in-flight OR completed-held object is
        // never displaced.
        if self.object.is_some() {
            return None;
        }
        let next = self.queue.pop_front()?; // FIFO FRONT pop; None on an empty queue
        self.object = Some(PendingObject { type_id: next, entity_id: None });
        self.progress = 0;
        // balance/original_balance/step_rate are LEFT for the next rebuild_shadow to
        // seed from the type cost (the single source of the cost-based balance in the
        // shadow). The authoritative begin path (a later slice) decides whether the pop
        // seeds the cost inline — that is a wiring choice, not this algorithm.
        self.balance = 0;
        self.original_balance = 0;
        self.step_rate_frames = 0;
        self.step_timer = 0;
        self.suspended = false;
        self.on_hold = false;
        self.manual = false;
        Some(next)
    }
```

**What P4 proves vs defers (DECISION b):** P4 PROVES (1) the FRONT pop (FIFO) and (2) the
held-object guard, in isolation against hand-seeded factories. It DEFERS (a) the **delivery
command binding** (no `advance_tick` call site — the method is dormant), (b) the
**post-AbandonProduction auto-StartNextQueued** (`cancel_active` leaves the queue intact and does
NOT auto-call this — that auto-advance is the same command-bound path, a later slice), and (c)
**inline balance/rate seeding** of the popped front (left to the next `rebuild_shadow`). The C7/C12
negative invariant (completion holds the object; the queue does not advance until the object is
cleared) is proven end-to-end in P4-T6's `queue_advances_only_after_delivery`.

**Unit tests** (append to `factory.rs` `mod tests`):

```rust
    #[test]
    fn start_next_queued_pops_front() {
        // queue [X,Y,Z], no active object -> active = X, queue [Y,Z] (FIFO front pop).
        let x = InternedId::from_index(1);
        let y = InternedId::from_index(2);
        let z = InternedId::from_index(3);
        let mut f = Factory {
            object: None,
            queue: VecDeque::from(vec![x, y, z]),
            ..Factory::default()
        };
        let popped = f.start_next_queued();
        assert_eq!(popped, Some(x), "the FRONT is popped");
        assert_eq!(f.object.as_ref().map(|o| o.type_id), Some(x), "active = X");
        assert_eq!(f.progress, 0, "fresh active object starts at progress 0");
        let q: Vec<InternedId> = f.queue.iter().copied().collect();
        assert_eq!(q, vec![y, z], "queue advanced to [Y,Z]");
    }

    #[test]
    fn start_next_queued_blocked_while_object_held() {
        // object Some -> None, queue unchanged (the "Object null required" guard). True
        // for both an in-flight object and a completed-held one.
        let x = InternedId::from_index(1);
        let mut f = Factory {
            object: Some(PendingObject::default()),
            queue: VecDeque::from(vec![x]),
            progress: 30,
            ..Factory::default()
        };
        assert_eq!(f.start_next_queued(), None, "a held object blocks the advance");
        let q: Vec<InternedId> = f.queue.iter().copied().collect();
        assert_eq!(q, vec![x], "queue unchanged while blocked");
        assert_eq!(f.progress, 30, "the held object's progress is untouched");
    }

    #[test]
    fn start_next_queued_empty_queue_is_noop() {
        // No object, empty queue -> None, no object created.
        let mut f = Factory::default();
        assert_eq!(f.start_next_queued(), None);
        assert!(f.object.is_none(), "no object created from an empty queue");
    }
```

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k start_next_queued_pops_front start_next_queued_blocked_while_object_held start_next_queued_empty_queue_is_noop`

---

### P4-T4 — re-export `CancelOutcome`

**File (EDIT):** `src/sim/production/mod.rs` — add `CancelOutcome` to the existing
`pub use self::factory::{...}` list (the one that already re-exports `StepOutcome`,
`PRODUCTION_STEPS`, etc., A13):

```rust
pub use self::factory::{
    BuildEligibility, CancelOutcome, Factory, FactoryRegistry, FactoryView, PendingObject,
    SpecialItem, StepOutcome, PRODUCTION_STEPS, STEP_RATE_MAX, STEP_RATE_MIN,
};
```

> `cancel_one`/`cancel_active`/`start_next_queued` are reachable without further re-exports:
> `cancel_one` is `pub` on `FactoryRegistry` (already re-exported); `cancel_active`/
> `start_next_queued` are PRIVATE `Factory` methods (the registry drives them, and the
> `factory.rs` unit tests are in-module). Only the public `CancelOutcome` type needs re-exporting
> so the world-level tests in `production_shadow_tests.rs` can name it.

**Verification:**
- `cargo check -p vera20k` (the re-export compiles; nothing consumes it yet outside tests)

---

### P4-T5 — the cancel-conservation shadow-assert (surface, never equalize)

**File (EDIT):** `src/sim/world/mod.rs` — add `debug_assert_factory_cancel_refund` beside the P3
`debug_assert_factory_conservation`, and wire one call line into `debug_assert_production_shadow`.
Mirrors the P3 template EXACTLY: clone factory + clone economy, drive forward, cancel, assert,
NEVER write back. To avoid the queued-vs-active precedence ambiguity in the assert (a registry-
level `cancel_one` on a type that also sits in the tail would report `QueuedRemoved`, refund 0),
the assert drives `cancel_active` DIRECTLY on the `f` clone — the registry-level `cancel_one` is
covered by the P4-T2 unit tests. (`cancel_active` is private to `factory.rs`, so the assert calls
`cancel_one` on a throwaway single-factory registry clone instead — see the body.)

```rust
    /// Debug-only P4 assert: each live shadow factory's active build, when cancelled
    /// mid-build, refunds EXACTLY the spent portion (C8) and returns the oracle to its
    /// starting credits (C15). Steps a CLONE forward ~half the build against a CLONE
    /// economy seeded with exactly `original_balance`, then cancels via a throwaway
    /// single-factory registry CLONE; SURFACES divergence with tick + owner + category,
    /// NEVER writes back to the shadow or the wallet.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_factory_cancel_refund(&self) {
        use crate::sim::economy::Economy;
        use crate::sim::production::{CancelOutcome, FactoryRegistry, StepOutcome, PRODUCTION_STEPS};
        for factory in self.production.factory_shadow.iter_insertion_ordered() {
            let Some(obj) = factory.object.as_ref() else {
                continue; // no active object: nothing to cancel
            };
            let cost = factory.original_balance;
            // A fresh, armed clone driven from progress 0 to ~half the build with exact
            // funds. (`original_balance` is the full-cost snapshot; a freshly-armed
            // clone with balance == cost mirrors the start-of-build state.)
            let mut f = factory.clone();
            f.progress = 0;
            f.balance = cost;
            f.on_hold = false;
            f.suspended = false;
            f.manual = false;
            let mut econ = Economy { credits: cost, ..Economy::default() };
            let target = (PRODUCTION_STEPS / 2).max(1);
            while f.progress < target {
                if !matches!(f.advance_one_step(&mut econ), StepOutcome::Stepped) {
                    break; // a free build may Complete early; the asserts below handle it
                }
            }
            let spent = econ.spent_credits; // what the ladder removed by the cancel point
            // Cancel via a throwaway single-factory registry clone so the assert exercises
            // the real `cancel_one` path. The clone holds only this `f` (the active type
            // is NOT in any tail here -> the active-abandon branch, not QueuedRemoved).
            let mut reg = FactoryRegistry::default();
            let key = (factory.owner, factory.category);
            reg.insert_for_assert(key, f.clone()); // see the helper note below
            let type_id = obj.type_id;
            let outcome = reg.cancel_one(factory.owner, factory.category, type_id, &mut econ);
            // A free build that Completed before the cancel point is a NoMatch (no-op
            // after completion); skip the spent/refund assert for that degenerate case.
            if f.progress >= PRODUCTION_STEPS {
                continue;
            }
            debug_assert!(
                matches!(outcome, CancelOutcome::AbandonedActive { refund } if refund == spent),
                "C8: tick {} {:?}/{:?}: active cancel must refund the spent portion {}",
                self.tick, factory.owner, factory.category, spent,
            );
            debug_assert_eq!(
                econ.credits, cost,
                "C15: tick {} {:?}/{:?}: post-cancel oracle balance must equal start {}",
                self.tick, factory.owner, factory.category, cost,
            );
        }
    }
```

Wire the call into `debug_assert_production_shadow` (one added line, beside the P3 sibling):

```rust
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_production_shadow(&self) {
        self.debug_assert_economy_shadow();
        self.debug_assert_factory_shell_trace();
        self.debug_assert_factory_conservation(); // P3
        self.debug_assert_factory_cancel_refund(); // P4  <-- added
    }
```

> **Helper note (`insert_for_assert`):** the assert needs to put a factory into a fresh registry,
> but `FactoryRegistry.factories` is PRIVATE and there is no public insert. Two options, pick the
> lower-surface one at impl time:
> - **(preferred)** add a tiny `#[cfg(debug_assertions)] pub(crate) fn insert_for_assert(&mut self, key: (InternedId, ProductionCategory), f: Factory)` to `impl FactoryRegistry` in `factory.rs` that does `self.factories.insert(key, f);`. It is debug-only and crate-private — no production surface. Mirror the existing in-module test inserts (`reg.factories.insert(...)`).
> - **(alternative, zero new API)** make the assert call the PRIVATE `cancel_active` indirectly by
>   NOT going through the registry at all: assert directly on the `f` clone with a registry built
>   the same way the P4-T2 `reg_with` test helper does — but that helper is `#[cfg(test)]`, not
>   available in a debug-build assert. So the `insert_for_assert` helper is the clean path.
>
> If neither is desired, the assert MAY be dropped to a `#[cfg(test)]` world-test that builds the
> registry via the `reg_with`-style path; the load-bearing P4 guarantees are the unit tests +
> the no-hash acceptance test, not this live assert. Flag the choice for the design-lead (E1).

> **Determinism / no-write-back:** the assert reads `iter_insertion_ordered()` (sorted, A11),
> clones, and asserts; it NEVER mutates `self.production.factory_shadow`, `self.houses`, or any
> entity. Same discipline as `debug_assert_factory_conservation` (A15). Surfaced with
> `tick + owner + category`; never equalized.

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k production_shadow_preserves_advance_tick_phase_order production_shadow_with_oracle_is_deterministic factory_oracle_step_trace_walks_live_structures` — the P2/P3 tests still pass with the new debug assert active (it surfaces, never perturbs); the assert must not fire for the empty-rules fixtures (cost 0 -> the free-build early-`continue` covers it).

---

### P4-T6 — the no-hash acceptance + queue-advance + determinism tests

**File (EDIT):** `src/sim/world/production_shadow_tests.rs` — append after the P3 block. Reuse
`empty_rules()`, `queued_item`, `insert_queue` (A16). Add `CancelOutcome` and `StepOutcome` to the
production import line, and `VecDeque`/`PendingObject` if a test seeds them directly:

```rust
use crate::sim::production::{
    BuildQueueItem, BuildQueueState, CancelOutcome, ProductionCategory, StepOutcome,
    PRODUCTION_STEPS,
};
```

```rust
// ===== P4 — FIFO queue + cancel + partial refund (hash-neutral oracle) =====

/// P4 no-hash guarantee (the acceptance test; mirrors
/// `factory_advance_step_does_not_change_state_hash`): cancelling a mid-build active
/// object on a CLONE of the registry against a CLONE of the wallet leaves
/// `state_hash()` bit-identical (the oracle never touches the hashed wallet;
/// `Factory`/`FactoryRegistry`/`Economy`/`CancelOutcome` carry no serde derive, and
/// the registry lives in the `#[serde(skip)]` `factory_shadow`).
#[test]
fn factory_cancel_one_does_not_change_state_hash() {
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
    sim.refresh_production_shadow(Some(&rules)); // cost-based shadow built
    let before = sim.state_hash();
    let legacy_credits = sim.houses[&owner].credits;

    // Cancel (active abandon, mid-build) against a CLONE of the registry + a CLONE of
    // the wallet; prove the real hash + the legacy wallet are bit-identical.
    let mut reg = sim.production.factory_shadow.clone();
    let mut oracle = sim.houses[&owner].economy.clone();
    // empty_rules -> cost 0; seed a mid-build cost on the clone so the refund is
    // nonzero (mutate the clone's factory directly — it is a value-type clone).
    {
        let f = reg
            .view(owner, ProductionCategory::Vehicle)
            .expect("factory exists");
        // `view` is read-only; to mutate the clone we re-seed via cancel-then-assert.
        // Instead, drive the cancel through the registry: seed a real cost first by
        // rebuilding the clone's factory through a helper, or accept the cost-0 refund.
        let _ = f; // (see the seeding note below)
    }
    let outcome = reg.cancel_one(owner, ProductionCategory::Vehicle, ty, &mut oracle);
    // With empty_rules cost is 0, so the active object refunds 0 (still AbandonedActive,
    // since the active type matches and progress < 54). The point of THIS test is the
    // HASH, not the refund value — both must be unchanged regardless of refund.
    assert!(
        matches!(outcome, CancelOutcome::AbandonedActive { .. } | CancelOutcome::QueuedRemoved),
        "the clone cancel acted on the cloned registry"
    );

    assert_eq!(
        before,
        sim.state_hash(),
        "P4 cancel on a clone must not perturb the state hash (serde-skip + clone)"
    );
    assert_eq!(
        sim.houses[&owner].credits, legacy_credits,
        "the legacy wallet is untouched by the oracle cancel"
    );
}

/// P4 C7/C12: completion suspends with the object attached; `start_next_queued` does
/// NOT advance while the object is held; only after the object is CLEARED (simulating
/// the delivery commit, a later slice) does the queue front advance. Proves the
/// negative invariant end-to-end WITHOUT wiring delivery. Driven on a CLONE.
#[test]
fn queue_advances_only_after_delivery() {
    let mut sim = Simulation::new();
    let rules = empty_rules();
    let owner = sim.interner.intern("Americans");
    sim.houses.insert(owner, HouseState::new(owner, 0, None, true, 1_000_000, 10));
    let active = sim.interner.intern("GRIZZLY");
    let next = sim.interner.intern("FV"); // the queued tail item
    // Front Building (active object) with a tail item behind it.
    let mut dq = VecDeque::new();
    dq.push_back(queued_item(owner, active, ProductionCategory::Vehicle, BuildQueueState::Building, 54, 30, 1));
    dq.push_back(queued_item(owner, next, ProductionCategory::Vehicle, BuildQueueState::Queued, 54, 54, 2));
    let mut cats = BTreeMap::new();
    cats.insert(ProductionCategory::Vehicle, dq);
    sim.production.queues_by_owner.insert(owner, cats);
    sim.refresh_production_shadow(Some(&rules));

    // Drive a CLONE of the shadow factory to completion against a CLONE wallet.
    let before = sim.state_hash();
    let mut f = sim.production.factory_shadow.iter_insertion_ordered()[0].clone();
    assert_eq!(f.object.as_ref().map(|o| o.type_id), Some(active), "active = GRIZZLY");
    let tail: Vec<_> = f.queue.iter().copied().collect();
    assert_eq!(tail, vec![next], "tail = [FV]");
    // empty_rules -> cost 0; seed a real cost so completion takes the full ladder.
    f.progress = 0;
    f.balance = 700;
    f.original_balance = 700;
    let mut oracle = sim.houses[&owner].economy.clone();
    loop {
        if matches!(f.advance_one_step(&mut oracle), StepOutcome::Completed) {
            break;
        }
    }
    assert!(f.suspended && f.object.is_some(), "C12: completion holds the object, suspended");
    // The queue does NOT advance on completion alone.
    assert_eq!(f.start_next_queued(), None, "C7: held object blocks the advance");
    assert_eq!(
        f.queue.iter().copied().collect::<Vec<_>>(),
        vec![next],
        "queue front unchanged while the object is held"
    );
    // Simulate the delivery commit: clear the object, THEN the queue advances.
    f.object = None;
    f.suspended = false;
    assert_eq!(f.start_next_queued(), Some(next), "after delivery the front pops");
    assert_eq!(f.object.as_ref().map(|o| o.type_id), Some(next), "active = FV");
    assert!(f.queue.is_empty(), "tail consumed");

    assert_eq!(before, sim.state_hash(), "the clone drive must not perturb the hash");
}

/// P4 determinism: identical fixtures over N ticks with a per-tick cancel/advance
/// closure on CLONES produce identical per-tick state_hash sequences. Guards against
/// the cancel/advance methods introducing nondeterminism (mirrors
/// `production_shadow_with_oracle_is_deterministic`).
#[test]
fn production_shadow_with_cancel_is_deterministic() {
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
                // Per-tick clone cancel/advance probe (NEVER written back).
                let mut reg = sim.production.factory_shadow.clone();
                let mut oracle = sim
                    .houses
                    .get(&owner)
                    .map(|h| h.economy.clone())
                    .unwrap_or_default();
                let _ = reg.cancel_one(owner, ProductionCategory::Vehicle, ty, &mut oracle);
                sim.state_hash()
            })
            .collect()
    }
    assert_eq!(run(), run(), "advance_tick with the P4 clone cancel probe stays deterministic");
}
```

> **Seeding note (the no-hash test):** `FactoryView` is read-only (it borrows; A11), so the clone's
> factory cannot be re-seeded through `view`. With `empty_rules()` the cost is 0, so the active
> cancel refunds 0 — that is FINE for the no-hash test, whose contract is "the hash + the legacy
> wallet are unchanged regardless of refund." If a NONZERO-refund clone is wanted for clarity, add a
> debug-only `#[cfg(debug_assertions)] pub(crate)` mutable accessor on `FactoryRegistry` (e.g. the
> `insert_for_assert` helper from P4-T5 reused to overwrite the clone's factory with a hand-seeded
> mid-build one), OR assert the refund separately in the pure `factory.rs` tests (P4-T2 already does:
> `cancel_one_active_when_no_queued_copy` asserts refund 400). The simplest hash-neutral form keeps
> the cost-0 refund and asserts only the hash + legacy-wallet invariants — that is the version
> written above. Confirm with the design-lead (E2).

> **Confirm at impl time:** the exact `advance_tick` signature. The P3 determinism test calls
> `sim.advance_tick(&[], Some(&rules), &heights, None, None, 67)` (the 2nd positional arg is
> `Option<&RuleSet>`). Mirror the existing P3 call EXACTLY; only the cancel probe is new.

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k factory_cancel_one_does_not_change_state_hash queue_advances_only_after_delivery production_shadow_with_cancel_is_deterministic`

---

### P4-T7 — full-suite verify + no-bump / no-hash-file lock (separate foreground pass)

Per the build-discipline memory (don't bury slow cargo inside a background workflow), run the
verification as a separate bounded foreground pass.

**Verification:**
- `cargo test -p vera20k` — read the literal `test result:` line. The P4 set must pass:
  `cancel_active_refunds_spent_only`, `cancel_active_at_progress_zero_refunds_nothing`,
  `cancel_active_no_object_is_noop`, `cancel_active_completed_is_noop`,
  `cancel_active_round_trip_conserves`, `cancel_one_removes_first_matching`,
  `cancel_one_queued_preferred_over_active_same_type`, `cancel_one_active_when_no_queued_copy`,
  `cancel_one_completed_active_is_noop`, `cancel_one_no_match_is_noop`,
  `start_next_queued_pops_front`, `start_next_queued_blocked_while_object_held`,
  `start_next_queued_empty_queue_is_noop`, `factory_cancel_one_does_not_change_state_hash`,
  `queue_advances_only_after_delivery`, `production_shadow_with_cancel_is_deterministic`.
- The P1/P2/P3 tests must still pass: `economy_*`, `factory_shadow_*`, `insertion_seq_*`,
  `factory_54_steps_to_complete`, `factory_exact_cost_conservation`,
  `factory_advance_step_does_not_change_state_hash`,
  `production_shadow_with_oracle_is_deterministic`, `factory_oracle_step_trace_walks_live_structures`,
  `snapshot_roundtrip_ignores_shadow`, `production_shadow_preserves_advance_tick_phase_order`,
  `snapshot_version_is_17_in_shadow_phase`, `techno_ai_shell_is_passthrough_no_hash_change`.
- `cargo test -p vera20k snapshot_version_is_17_in_shadow_phase` — confirms SNAPSHOT_VERSION
  still 17.
- Confirm `git diff --stat` shows NO change to `src/sim/world/world_hash.rs` and NO change to
  `SNAPSHOT_VERSION` in `src/sim/snapshot.rs` (the no-hash contract).

---

## D. Out-of-scope seams (left clean, NOT implemented)

| Concern | Status | Seam |
|---|---|---|
| Authority flip (oracle → real wallet), fixing legacy `cancel_by_type_for_owner` (`.rev()` last-match + full-cost refund), SNAPSHOT_VERSION 17→18 | P5 | `cancel_one(&mut Economy)` signature is P5-ready; flip WHO is passed + replace the legacy cancel call, not this body. |
| The delivery command that drives `start_next_queued` (C7) | P5+ | `start_next_queued` shipped proven-but-dormant; P5 binds it to the delivery commit. |
| Post-AbandonProduction auto-StartNextQueued (`heapId = −1` path) | P5 | `cancel_active` leaves the queue tail intact; P5 binds abandon + advance in the same command. |
| Completed-build cancel (ready-queue path) | P5+ | `cancel_active` no-ops on a completed factory (the "no-op after completion" rule); the ready-queue cancel is P5's. |
| Inline balance/rate seed of the popped front | P5 | `start_next_queued` leaves the seed to the next rebuild; P5's begin path decides. |
| Per-step charge / SetRate | DONE P3 | `advance_one_step` / `set_rate`. |
| Empty-factory self-delete (C9) | P6 | `cancel_active` leaves the empty factory; the next rebuild drops empty-queue categories (the `queue.front()` continue). |
| Prereq revalidation's abandon-active path (C9/C19) | P6 | reuses `cancel_active` + the refund formula, wired to the building-lifecycle. |
| Purifier / IncomeMult | P7 | `Economy` fields present; not exercised by P4. |

---

## E. Open questions for the design-lead (confirm before / during implementing)

**E1 — `debug_assert_factory_cancel_refund` registry-insert helper (P4-T5).** The assert builds a
throwaway single-factory `FactoryRegistry` to exercise the real `cancel_one`, but `factories` is
private and there is no public insert. The plan's preferred path adds a debug-only
`#[cfg(debug_assertions)] pub(crate) fn insert_for_assert(...)` to `FactoryRegistry`. Alternative:
drop the live assert to a `#[cfg(test)]` world-test (the load-bearing guarantees are the unit
tests + the no-hash acceptance test). Confirm the helper (default) vs the test-only assert.

**E2 — no-hash test refund value (P4-T6).** `factory_cancel_one_does_not_change_state_hash` runs
on `empty_rules()` (cost 0 → refund 0). The test's contract is the HASH + legacy-wallet
invariants, which hold regardless of refund; the nonzero-refund assertion lives in the pure
`factory.rs` test `cancel_one_active_when_no_queued_copy` (refund 400). Confirm leaving the
acceptance test at a cost-0 clone (zero new API) vs adding a debug-only seed accessor to make the
clone's refund nonzero (more surface, same guarantee).

**E3 — `InternedId` test ctor (P4-T1/T2/T3).** The pure tests need DISTINCT ids for A/B/C/X/Y/Z.
The plan uses `InternedId::from_index(n)` + `InternedId::default()`. Confirm `from_index` is the
public ctor; if not, mint via a local `StringInterner`/`Simulation::intern` (as the world-level
tests do). Purely a test-fixture mechanism; no behavior depends on it.

**E4 — `cancel_active` privacy.** The plan keeps `cancel_active` PRIVATE (the registry drives it;
`factory.rs` tests are in-module). If a later slice (P6 revalidation) needs it cross-module, it
promotes to `pub(crate)` then. Confirm private-now is acceptable (it is the lower-surface default).

---

*End of P4 plan. The slice is additive and oracle-only: `cancel_one`/`cancel_active`/
`start_next_queued` are pure `Factory`/`FactoryRegistry` methods exercised against cloned
registries + economies; the legacy `cancel_by_type_for_owner` stays authoritative (its `.rev()`
last-match + full-cost refund are the verified DRIFTs P4 models CORRECTLY in the shadow so P5 can
adopt them); `world_hash.rs`/`snapshot.rs` are untouched and `SNAPSHOT_VERSION` stays 17. P4
PROVES the first-match queued removal (C6), the spent-only refund (C8/C15), and the C7/C12
held-object queue-advance guard; it DEFERS the delivery command that drives the advance and the
authority flip to P5. The completed-build cancel is modeled as a no-op (not a full refund) and
surfaced as the one open behavioral position, per the burden-of-proof default.*
