<!--
Provenance: authored 2026-06-04 from the APPROVED design
  docs/plans/2026-06-04-factory-house-substrate-p3-design.md
  (D2 substrate-fit winner; (a) cost-based oracle via &RuleSet, (b) SetRate takes the
  build-step total as input, (c) oracle step runs as a debug probe beside the no-op arm),
  grounded in the v2-verified study
  docs/research/FACTORY_HOUSE_ENGINE_SUBSTRATE_SERVICE_STUDY.md (C2-C5, C12, C15; §6.2; §8 P3; §9.1).
House style mirrored from docs/plans/2026-06-04-factory-house-substrate-p1p2-plan.md.
Status: DRAFTED, not approved or executed. Review (/review-plan) before implementing.
Scope: P3 ONLY — `Factory::advance_one_step` + `Factory::set_rate` (the per-step charge
  state machine + rate), HASH-NEUTRAL: charges an ORACLE (clone) economy, never the hashed
  wallet. The legacy upfront-charge stays AUTHORITATIVE. NO authority flip (P5), NO
  cancel/refund (P4), NO prereq revalidation (P6), NO purifier/IncomeMult (P7),
  NO delivery/exit (P5+). world_hash.rs UNTOUCHED; SNAPSHOT_VERSION STAYS 17.
-->

# Factory/House Substrate — P3 Plan (per-step charge + SetRate, hash-neutral oracle)

> Linear path: **P3-T1 → P3-T2 → P3-T3 → P3-T4 → P3-T5 → P3-T6 → P3-T7**.
> Every task builds green (`cargo check -p vera20k`) before the next. The hash-neutrality
> test (`factory_advance_step_does_not_change_state_hash`) + the version pin
> (`snapshot_version_is_17_in_shadow_phase`, snapshot.rs:374) are the contract gate: if either
> fails after a task, STOP — the oracle leaked into the hashed wallet or a serde derive crept in.
>
> **#1 invariant preserved:** `sim/production/factory.rs` and `sim/economy.rs` depend only on
> `std` + `sim/` (intern, production_types, rules data through `&Simulation`/`&RuleSet`);
> NEVER on render/ui/sidebar/audio/net.
>
> **No-hash contract (the whole point of P3):** `advance_one_step`/`set_rate` mutate only a
> `Factory` + an `Economy`, NEITHER of which `state_hash()` visits (no serde derive on
> `Factory`/`FactoryRegistry`/`Economy`, factory.rs:42/52/68/127, economy.rs:17). The new
> methods have **NO authoritative `advance_tick` call site** — they are reached only from the
> debug-only oracle probe, always against a CLONE. `world_hash.rs` is NOT touched.
> `SNAPSHOT_VERSION` stays **17** (snapshot.rs:24). The 17→18 authority flip is P5, out of scope.

---

## A. Verified preconditions (live reads this session — quote file:line)

| # | Fact the plan relies on | Verified at |
|---|---|---|
| A1 | `Factory` carries `progress: u16`, `step_rate_frames: u16`, `step_timer: u16`, `balance: i32`, `original_balance: i32`, `object: Option<PendingObject>`, `on_hold/suspended/manual: bool` — exactly the fields `advance_one_step`/`set_rate` mutate | `factory.rs:69-94` |
| A2 | `StepOutcome { Idle, Stepped, Stalled, Completed }` is already declared (the P3 seam) — P3 only adds the producing methods, not the enum | `factory.rs:98-103` |
| A3 | consts `PRODUCTION_STEPS: u16 = 54`, `STEP_RATE_MIN: u16 = 1`, `STEP_RATE_MAX: u16 = 255` already declared | `factory.rs:34-37` |
| A4 | the module is `#![allow(dead_code)]` — the new methods (no authoritative caller in P3) raise no unused-warning | `factory.rs:26` |
| A5 | `Economy` has `available(&self) -> i32` (returns `credits`) and `spend(&mut self, amount: i32) -> i32` (caps at balance, tracks `spent_credits`); both are exactly what the oracle needs — REUSE, do not add new methods | `economy.rs:52-62` |
| A6 | `rebuild_shadow(&mut self, sim: &Simulation)` currently sets `balance = front.remaining_base_frames as i32`, `original_balance = front.total_base_frames as i32`, `step_rate_frames = 0` (E1 frames-based placeholder) — P3 changes the signature to add `&RuleSet` and makes it cost-based | `factory.rs:182, 242-246` |
| A7 | `Simulation::object_type(type_ref: InternedId, rules: &RuleSet) -> Option<&ObjectType>` resolves a type handle to its `ObjectType`; `ObjectType.cost: i32` is the full credit cost | `world/mod.rs:496`, `object_type.rs:155` |
| A8 | `refresh_production_shadow(&mut self, rules: Option<&RuleSet>)` calls `refresh_economy_shadow(rules)` then `registry.rebuild_shadow(self)` via `mem::take` — the tail `rules` is an `Option<&RuleSet>`, so the cost-based rebuild needs a `Some`/`None` split | `world/mod.rs:1006-1015` |
| A9 | `debug_assert_production_shadow(&self)` (#[cfg(debug_assertions)]) calls `debug_assert_economy_shadow()` then `debug_assert_factory_shell_trace()` — the conservation assert slots in here, beside them | `world/mod.rs:1020-1024` |
| A10 | the `EntityCategory::Structure => {}` arm in `techno_ai_shell` is a literal no-op (S8 absorb bracket); the no-hash guarantee is the test `techno_ai_shell_is_passthrough_no_hash_change` | `techno_ai.rs:107` |
| A11 | the P2 factory shell trace lives in a debug-only `factory_shell_trace()` (#[cfg(any(test, debug_assertions))], walks `live_object_order_snapshot()`, READ-ONLY `&Simulation`) — the P3 oracle probe extends BESIDE it, same shape | `techno_ai.rs:252-271` |
| A12 | the divergence-surfacing template: `debug_assert_s1_shadow` walks live order, asserts with `tick + id`, NEVER writes back; `unit_ai_shadow_step` returns the OBSERVED value and never equalizes | `techno_ai.rs:162-222` |
| A13 | the P3 test fixtures (`empty_rules()`, `queued_item(..)`, `insert_queue(..)`) already exist in `production_shadow_tests.rs`; `PRODUCTION_STEPS` is re-exported via `crate::sim::production` | `production_shadow_tests.rs:17, 20-61` |
| A14 | `HouseState::new(name, side_index, country, is_human, credits, tech_level)` — the test fixture ctor | `house_state.rs:59-66` |
| A15 | `iter_insertion_ordered(&self) -> Vec<&Factory>` (sorted by `insertion_seq`); `view(owner, category) -> Option<FactoryView>` — read-only registry accessors the probe/tests reuse | `factory.rs:138-171` |
| A16 | `SNAPSHOT_VERSION == 17` and the version-pin test `snapshot_version_is_17_in_shadow_phase` already exist — P3 must not bump | `snapshot.rs:24, 374` |
| A17 | the legacy `build_time_base_frames` total bakes a verified-REFUTED ×0.9 (`cost * speed_x1000 * 9 / 10000`) — so SetRate MUST NOT derive its total from the frames balance (decisive design finding (b)) | `production_tech.rs:334` |

**Two facts the design pins from the study (no re-decompile in P3 — VERIFIED-LIVE v2):**
- charge = `⌊Balance/(54−Value)⌋` after `Value` is incremented; the final step skips the IDIV
  (div-by-zero guard) and charges the **entire remaining Balance, once**; completion runs
  `Spend_Money(0)` — charging the remainder twice is a bug (study C3, line 415).
- SetRate = `clamp(GetBuildStepTime()/54, 1, 255)`, signed-truncate; no-object → 0; 661 is one
  example total (`12 × 54`), not a constant divisor (study C5, line 419; 661→12 in §8 P3 line 731).

---

## B. Files touched (summary)

| File | Change | Task |
|---|---|---|
| `src/sim/production/factory.rs` | `Factory::set_rate(&mut self, build_step_time: i32)`; `Factory::advance_one_step(&mut self, &mut Economy) -> StepOutcome`; `remaining_balance_after(cost, progress)` free fn; cost-based `rebuild_shadow(&mut self, sim, rules: &RuleSet)` + `rebuild_shadow_no_rules(&mut self, sim)` fallback; `original_balance()` test accessor; the §8-P3 unit tests + boundary sub-cases | P3-T1, P3-T2, P3-T3 |
| `src/sim/world/mod.rs` | thread `rules` into `rebuild_shadow` (Some/None split, `refresh_production_shadow`, :1006); add `debug_assert_factory_conservation` to `debug_assert_production_shadow` (:1021) | P3-T4 |
| `src/sim/world/techno_ai.rs` | add debug-only `factory_oracle_step_trace` BESIDE `factory_shell_trace` (:252); the `EntityCategory::Structure` arm STAYS a literal no-op | P3-T5 |
| `src/sim/world/production_shadow_tests.rs` | `factory_advance_step_does_not_change_state_hash` + a per-tick determinism variant (reuse `empty_rules`/`queued_item`/`insert_queue`) | P3-T6 |

`world_hash.rs` and `snapshot.rs` are **NOT** in this list — that is the no-hash contract.
No miner/combat/movement file is touched (concurrent session owns those).

---

## C. P3 — the per-step charge state machine + rate

### P3-T1 — `Factory::set_rate` (C5)

**File (EDIT):** `src/sim/production/factory.rs` — add to `impl FactoryRegistry`'s sibling
`impl Factory` block. There is no `impl Factory` block yet; create one immediately after the
`Factory` struct definition (after line 94, before the `StepOutcome` enum) so the methods live
with their type. Integer-only; no float, no RNG; no engine addresses in comments.

```rust
impl Factory {
    /// Resume + (re)compute the per-step frame rate from a GIVEN build-step total.
    ///   no object  -> step_rate_frames = 0  (sentinel; the clamp does NOT apply)
    ///   else        -> step_rate_frames = clamp(build_step_time / 54, 1, 255)
    /// `build_step_time` is the already-resolved total (no hidden 0.9 scaling); the
    /// `/54` is signed integer division (truncates toward zero). The full
    /// low-power / multiple-factory pipeline that PRODUCES `build_step_time` is a
    /// later slice — here SetRate owns only the verified divide/clamp/sentinel shape.
    ///
    /// SetRate resumes a system-suspend; a manual (user) pause is left untouched.
    pub fn set_rate(&mut self, build_step_time: i32) {
        if !self.manual {
            self.suspended = false;
        }
        // Rate-0-no-object sentinel: (Object ? total : 0) / 54. With no object the
        // rate is the literal 0 (NOT clamped up to 1).
        if self.object.is_none() {
            self.step_rate_frames = 0;
            return;
        }
        let per_step = build_step_time / (PRODUCTION_STEPS as i32); // i32/54, truncate toward zero
        let clamped = per_step.clamp(STEP_RATE_MIN as i32, STEP_RATE_MAX as i32); // [1, 255]
        self.step_rate_frames = clamped as u16;
    }
}
```

**Why `build_step_time` is a parameter (not derived from `balance`/the legacy frames):** the
legacy base-frame total carries a verified-REFUTED ×0.9 (A17, `production_tech.rs:334`). Feeding
that into `/54` would bake the drift into the rate. SetRate takes the build-step total as input;
the pipeline producer plugs in at its own slice.

**Unit tests** (append to the `factory.rs` `mod tests`, end of file):

```rust
    #[test]
    fn set_rate_total_over_54_truncates_clamps() {
        // With an object, rate = clamp(total/54, 1, 255):
        //   0/54   = 0   -> clamp 1
        //   53/54  = 0   -> clamp 1
        //   54/54  = 1   -> 1
        //   661/54 = 12  -> 12   (661 = 12 x 54 + 13; the study's MTNK example)
        //   14000/54 = 259 -> clamp 255
        let cases = [(0, 1u16), (53, 1), (54, 1), (661, 12), (14000, 255)];
        for (total, expected) in cases {
            let mut f = Factory {
                object: Some(PendingObject::default()),
                ..Factory::default()
            };
            f.set_rate(total);
            assert_eq!(
                f.step_rate_frames, expected,
                "set_rate({total}) with object must be {expected}"
            );
        }
    }

    #[test]
    fn set_rate_zero_when_no_object() {
        // No object -> rate 0 (the sentinel, NOT clamped up to 1), even for a large total.
        let mut f = Factory::default();
        assert!(f.object.is_none());
        f.set_rate(14000);
        assert_eq!(f.step_rate_frames, 0, "no-object factory yields the rate-0 sentinel");
        // A suspended/queued-only (no-object) factory does not step.
        f.suspended = true;
        assert!(matches!(f.advance_one_step(&mut Economy::default()), StepOutcome::Idle));
    }
```

> `set_rate_zero_when_no_object` calls `advance_one_step` (P3-T2) — keep this test, but it only
> compiles after P3-T2 lands. If running P3-T1's check before P3-T2, comment out the last two
> lines, then restore them in P3-T2. (The plan's task order builds T1 then T2; the final tree
> has both.) `Economy` is in-module via `use crate::sim::economy::Economy;` added in P3-T2.

**Verification:**
- `cargo check -p vera20k`
- (the `set_rate_*` tests run green after P3-T2 supplies `advance_one_step`)

---

### P3-T2 — `Factory::advance_one_step` (C2/C3/C4/C12/C15)

**File (EDIT):** `src/sim/production/factory.rs` — add to the `impl Factory` block created in
P3-T1, and add the `Economy` import at the top of the file (after the existing
`use crate::sim::production::production_types::ProductionCategory;` at line 31):

```rust
use crate::sim::economy::Economy;
```

Add the method:

```rust
impl Factory {
    /// Advance one step against an ORACLE economy (a clone / throwaway), NOT the
    /// hashed wallet. Hash-neutrality is enforced at the CALL SITE (P3 only ever
    /// passes a clone); the method body is wallet-agnostic. The `&mut Economy`
    /// param (kept distinct from `&mut self`) is the exact shape the authority-flip
    /// slice makes real — that slice flips WHO is passed, not this algorithm.
    ///
    /// One step per call (the per-tick `step_timer` countdown is a separate concern,
    /// not wired authoritative in P3). The step:
    ///   * increments `progress` first, then reads stepsLeft = 54 - progress;
    ///   * charges `balance / stepsLeft` (the LAST step, stepsLeft == 0, skips the
    ///     divide and charges the whole remaining balance once);
    ///   * on a shortfall: rewinds the step, sets `on_hold`, spends nothing;
    ///   * on reaching 54: suspends with the object STILL attached and balance 0
    ///     (delivery, a later slice, clears the object and advances the queue).
    pub fn advance_one_step(&mut self, economy: &mut Economy) -> StepOutcome {
        // ARMED GATE: not stepping this call -> Idle. No object, or suspended
        // (complete-held / paused), or a latched on_hold, or a manual pause.
        if self.object.is_none() || self.suspended || self.on_hold || self.manual {
            return StepOutcome::Idle;
        }
        // Defensive: a settled factory is suspended (caught above); guard anyway.
        if self.progress >= PRODUCTION_STEPS {
            return StepOutcome::Idle;
        }

        // Take one tentative step; the charge reads stepsLeft = 54 - the NEW value.
        self.progress += 1;
        let steps_left = PRODUCTION_STEPS - self.progress; // 54 - new progress

        // Per-step charge, signed-truncate toward zero (= floor for a non-negative
        // balance). The final step (steps_left == 0) skips the divide (div-by-zero
        // guard) and charges the entire remaining balance, once.
        let charge = if steps_left == 0 {
            self.balance // whole remainder (may be 0 for a cost-0 type)
        } else {
            self.balance / (steps_left as i32)
        };

        // Affordability PRE-CHECK (no spend on a stall, so the oracle's spent total
        // stays clean). Exactly-affordable (available == charge) PROCEEDS (strict <).
        if economy.available() < charge {
            self.progress -= 1; // rewind the tentative step (net-zero advance)
            self.on_hold = true; // UI "On Hold"
            return StepOutcome::Stalled; // nothing spent, balance unchanged
        }

        // Pay-as-you-go: spend exactly `charge`, decrement balance by the same.
        self.on_hold = false; // a successful step clears a prior hold
        let paid = economy.spend(charge);
        debug_assert_eq!(paid, charge, "an afforded charge must be paid in full");
        self.balance -= charge; // charge <= balance always (stepsLeft >= 1) -> no underflow

        // Completion settlement on reaching 54. The last-step charge already zeroed
        // the balance, so there is NO second charge here (the engine's completion
        // spend runs as spend(0); charging the remainder twice would double-spend).
        if self.progress >= PRODUCTION_STEPS {
            debug_assert_eq!(self.balance, 0, "the last-step charge must zero the balance");
            self.balance = 0; // idempotent; the contract value
            self.suspended = true; // complete-but-not-delivered
            self.step_timer = 0; // the engine zeroes the per-step timer on completion
            // `object` STAYS Some(..); delivery (a later slice) clears it + advances the queue.
            return StepOutcome::Completed;
        }

        StepOutcome::Stepped
    }
}
```

**Unit tests** (append to the `factory.rs` `mod tests`):

```rust
    /// A small helper: a fresh armed factory holding `cost` credits of work.
    #[cfg(test)]
    fn armed_factory(cost: i32) -> Factory {
        Factory {
            object: Some(PendingObject::default()),
            balance: cost,
            original_balance: cost,
            ..Factory::default()
        }
    }

    #[test]
    fn factory_54_steps_to_complete() {
        // From a fresh start with funds, the factory takes exactly 54 steps: the
        // first 53 are `Stepped`, the 54th is `Completed` (C2). progress reaches 54.
        let mut f = armed_factory(700);
        let mut econ = Economy { credits: 700, ..Economy::default() };
        let mut stepped = 0;
        let mut completed = 0;
        for _ in 0..PRODUCTION_STEPS {
            match f.advance_one_step(&mut econ) {
                StepOutcome::Stepped => stepped += 1,
                StepOutcome::Completed => completed += 1,
                other => panic!("unexpected outcome {other:?} before completion"),
            }
        }
        assert_eq!(stepped, 53, "exactly 53 Stepped before the final Completed");
        assert_eq!(completed, 1, "exactly one Completed at step 54");
        assert_eq!(f.progress, PRODUCTION_STEPS, "progress reaches 54");
        assert!(f.suspended && f.object.is_some(), "complete-but-not-delivered");
        // A 55th call is Idle (the settled factory is suspended).
        assert!(matches!(f.advance_one_step(&mut econ), StepOutcome::Idle));
    }

    #[test]
    fn factory_exact_cost_conservation() {
        // Sum of oracle spend over a full build == the full type cost; balance ends 0
        // (C3/C15). Boundary set {1, 25, 700, 99991}: tiny, mid, round, and a large
        // near-100k prime-ish value that exercises per-step truncation accumulation.
        for cost in [1i32, 25, 700, 99991] {
            let mut f = armed_factory(cost);
            let mut econ = Economy { credits: cost, ..Economy::default() };
            loop {
                match f.advance_one_step(&mut econ) {
                    StepOutcome::Stepped => {}
                    StepOutcome::Completed => break,
                    other => panic!("cost {cost}: unexpected {other:?} with exact funds"),
                }
            }
            assert_eq!(econ.spent_credits, cost, "cost {cost}: total spent == full cost");
            assert_eq!(econ.credits, 0, "cost {cost}: oracle drained to exactly 0");
            assert_eq!(f.balance, 0, "cost {cost}: balance ends 0");
        }
    }

    #[test]
    fn factory_exact_cost_conservation_cost1_corner() {
        // The cost-1 corner proves conservation DEPENDS on the full-remainder last
        // step: 1/k == 0 for k>=2 (steps 1..=52 charge 0), 1/1 == 1 on step 53->54's
        // predecessor framing, and the final remainder charges the lone credit once.
        let mut f = armed_factory(1);
        let mut econ = Economy { credits: 1, ..Economy::default() };
        let mut total = 0;
        loop {
            let before = econ.spent_credits;
            match f.advance_one_step(&mut econ) {
                StepOutcome::Stepped => total += econ.spent_credits - before,
                StepOutcome::Completed => {
                    total += econ.spent_credits - before;
                    break;
                }
                other => panic!("unexpected {other:?}"),
            }
        }
        assert_eq!(total, 1, "the single credit is charged exactly once across the build");
        assert_eq!(f.balance, 0);
    }

    #[test]
    fn factory_last_step_charges_full_remainder() {
        // Drive to progress 53 (one before completion), then assert the final step
        // charges the WHOLE remaining balance, once, with no div-by-zero at
        // steps_left == 0, and completion does NOT charge again (C3/C5/C12).
        let mut f = armed_factory(700);
        let mut econ = Economy { credits: 700, ..Economy::default() };
        while f.progress < PRODUCTION_STEPS - 1 {
            assert!(matches!(f.advance_one_step(&mut econ), StepOutcome::Stepped));
        }
        assert_eq!(f.progress, PRODUCTION_STEPS - 1, "stopped one before completion");
        let remainder = f.balance;
        let spent_before = econ.spent_credits;
        let outcome = f.advance_one_step(&mut econ); // the 54th step
        assert!(matches!(outcome, StepOutcome::Completed));
        assert_eq!(
            econ.spent_credits - spent_before,
            remainder,
            "the final step charges the entire remaining balance, once"
        );
        assert_eq!(f.balance, 0, "completion leaves balance 0 (no second remainder charge)");
    }

    #[test]
    fn factory_stall_on_no_funds_rewinds() {
        // Oracle one credit below the first step's charge -> Stalled: on_hold set,
        // progress unchanged, NOTHING spent (C4). With cost 700 the first step's
        // charge is 700/53 = 13; fund the oracle with 12.
        let mut f = armed_factory(700);
        let first_charge = 700 / (PRODUCTION_STEPS as i32 - 1); // 700/53 = 13
        let mut econ = Economy { credits: first_charge - 1, ..Economy::default() };
        let outcome = f.advance_one_step(&mut econ);
        assert!(matches!(outcome, StepOutcome::Stalled));
        assert!(f.on_hold, "a shortfall latches on_hold");
        assert_eq!(f.progress, 0, "the tentative step is rewound (net-zero advance)");
        assert_eq!(econ.spent_credits, 0, "a stall spends nothing");
        assert_eq!(econ.credits, first_charge - 1, "the oracle wallet is untouched");
    }

    #[test]
    fn factory_exactly_affordable_step_proceeds() {
        // available == charge PROCEEDS (the strict-< boundary): fund the oracle with
        // exactly the first step's charge and assert it steps, not stalls.
        let mut f = armed_factory(700);
        let first_charge = 700 / (PRODUCTION_STEPS as i32 - 1); // 13
        let mut econ = Economy { credits: first_charge, ..Economy::default() };
        assert!(matches!(f.advance_one_step(&mut econ), StepOutcome::Stepped));
        assert_eq!(f.progress, 1);
        assert_eq!(econ.spent_credits, first_charge);
    }

    #[test]
    fn factory_cost_zero_completes_free() {
        // A cost-0 type: every charge is 0, so it completes with zero spend and
        // conservation holds trivially (sum 0 == original_balance 0).
        let mut f = armed_factory(0);
        let mut econ = Economy::default(); // 0 credits, but every charge is 0
        let mut steps = 0;
        loop {
            match f.advance_one_step(&mut econ) {
                StepOutcome::Stepped => steps += 1,
                StepOutcome::Completed => {
                    steps += 1;
                    break;
                }
                other => panic!("unexpected {other:?} for a free build"),
            }
        }
        assert_eq!(steps, PRODUCTION_STEPS as i32, "a free build still takes 54 steps");
        assert_eq!(econ.spent_credits, 0, "free build spends nothing");
        assert_eq!(f.balance, 0);
    }
```

> The `factory_54_steps_to_complete` panic arm prints `StepOutcome` via `{other:?}`. Add
> `#[derive(Debug)]` to `StepOutcome` (factory.rs:98) if it is not already derived — it is a
> pure unit enum, so `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` is the right set and stays
> serde-free (the no-hash contract). Verify the existing derive before editing; add only the
> missing traits.

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k factory_54_steps_to_complete factory_exact_cost_conservation factory_exact_cost_conservation_cost1_corner factory_last_step_charges_full_remainder factory_stall_on_no_funds_rewinds factory_exactly_affordable_step_proceeds factory_cost_zero_completes_free set_rate_total_over_54_truncates_clamps set_rate_zero_when_no_object`

---

### P3-T3 — cost-based `rebuild_shadow` (E1) + the ladder-replay helper

**File (EDIT):** `src/sim/production/factory.rs`

**(i)** Add the ladder-replay free fn near the consts (after line 37), integer-only:

```rust
/// Replay the per-step charge ladder for `progress` steps to recover the exact
/// running balance an authoritative stepper would hold at that progress. Used to
/// seed the cost-based shadow balance so a freshly-stepped factory and the rebuilt
/// shadow agree (the conservation assert is then meaningful). At most 54 integer
/// iterations; `cost` clamped to non-negative; mirrors `advance_one_step`'s charge.
fn remaining_balance_after(cost: i32, progress: u16) -> i32 {
    let mut balance = cost.max(0);
    let steps = progress.min(PRODUCTION_STEPS);
    for value in 1..=steps {
        let steps_left = PRODUCTION_STEPS - value;
        let charge = if steps_left == 0 {
            balance
        } else {
            balance / (steps_left as i32)
        };
        balance -= charge;
    }
    balance
}
```

**(ii)** Change `rebuild_shadow` to take `&RuleSet` and seed a cost-based balance. Replace the
current signature + the frames-based balance lines (factory.rs:182, 244-246). The new signature:

```rust
    /// P3 SHADOW BUILD: (re)derive the whole registry from the legacy queues each
    /// tick, with a COST-based oracle balance. READ-ONLY w.r.t. all hashed state.
    /// Reuses `seq_carry` to keep `insertion_seq` stable for surviving factories.
    ///
    /// E1 resolved (P3): `original_balance` = the front type's full credit cost
    /// (from `rules`); `balance` = the not-yet-charged remainder, recovered by
    /// replaying the exact per-step charge ladder for `progress` steps (NOT a
    /// one-shot proportion — that drifts from the per-step floor-division running
    /// balance). The per-step charge is in CREDITS, so a cost-based balance is what
    /// the oracle (and the conservation assert) require; the frames-based P2
    /// placeholder is retired here.
    pub(crate) fn rebuild_shadow(
        &mut self,
        sim: &crate::sim::world::Simulation,
        rules: &crate::rules::RuleSet,
    ) {
```

Inside the per-(owner, category) loop, replace the frames-based balance assignment
(factory.rs:244-246) with a cost-based one. The `progress` derivation, `object`/`tail`/state
flags, and `insertion_seq` are unchanged. The replacement block:

```rust
                // E1 (P3): cost-based oracle balance. original_balance = the full
                // type cost (snapshot, for conservation); balance = the remainder
                // after `progress` steps of the exact charge ladder.
                let full_cost = sim
                    .object_type(front.type_id, rules)
                    .map(|o| o.cost.max(0))
                    .unwrap_or(0);
                let original_balance = full_cost;
                let balance = remaining_balance_after(full_cost, progress);
```

and in the `Factory { .. }` literal, set `balance` and `original_balance` to these locals (drop
the two `front.*_base_frames as i32` lines). `step_rate_frames` stays `0` in the rebuild — the
probe (P3-T5) calls `set_rate` separately; the rebuild does not have a build-step total.

**(iii)** Add the cost-free fallback for the `None`-rules tail callers (P2's
`production_shadow_preserves_advance_tick_phase_order` calls `advance_tick` with `None`):

```rust
    /// Cost-free fallback when no `RuleSet` is available (the advance_tick `None`
    /// tail). Same derive as `rebuild_shadow` but with `balance`/`original_balance`
    /// = 0 (no type data to resolve cost). Hash-neutral either way (the registry is
    /// `#[serde(skip)]` + no serde derive). Implemented by delegating to a shared
    /// inner builder so the two paths cannot drift; see the impl note below.
    pub(crate) fn rebuild_shadow_no_rules(&mut self, sim: &crate::sim::world::Simulation) {
        self.rebuild_shadow_inner(sim, None);
    }
```

To avoid duplicating the loop, refactor the body into a private `rebuild_shadow_inner(&mut self,
sim, rules: Option<&RuleSet>)` and have both public methods delegate:

```rust
    pub(crate) fn rebuild_shadow(
        &mut self,
        sim: &crate::sim::world::Simulation,
        rules: &crate::rules::RuleSet,
    ) {
        self.rebuild_shadow_inner(sim, Some(rules));
    }
```

The inner method holds the existing loop; the only `rules`-dependent line is the cost lookup:

```rust
                let full_cost = match rules {
                    Some(r) => sim
                        .object_type(front.type_id, r)
                        .map(|o| o.cost.max(0))
                        .unwrap_or(0),
                    None => 0,
                };
```

**(iv)** Add a tiny test accessor so the unit tests can read `original_balance` off a registry
factory without making the field path public (it is already `pub`, but the registry's
`factories` map is private; `iter_insertion_ordered()` returns `&Factory`, so `.original_balance`
is reachable — no new accessor needed). Confirm at impl time; if the field is reachable via
`iter_insertion_ordered()[0].original_balance`, skip this sub-step.

**Unit test** (append to `factory.rs` `mod tests`) — the ladder-replay correctness:

```rust
    #[test]
    fn remaining_balance_ladder_matches_stepper() {
        // remaining_balance_after must equal the balance the stepper actually holds.
        // For each cost in the boundary set, step the factory k times and compare.
        for cost in [1i32, 25, 700, 99991] {
            let mut f = armed_factory(cost);
            let mut econ = Economy { credits: cost, ..Economy::default() };
            for k in 0..PRODUCTION_STEPS {
                assert_eq!(
                    f.balance,
                    remaining_balance_after(cost, k),
                    "cost {cost}: ladder replay must match the stepper at progress {k}"
                );
                let _ = f.advance_one_step(&mut econ);
            }
            assert_eq!(remaining_balance_after(cost, PRODUCTION_STEPS), 0);
        }
    }

    #[test]
    fn cost25_ladder_sums_to_exactly_25() {
        // The cost-25 ladder: the sum of per-step charges equals 25 exactly (the
        // truncation-direction proof — floor division never loses or gains a credit
        // because the last step charges the whole remainder).
        let mut f = armed_factory(25);
        let mut econ = Economy { credits: 25, ..Economy::default() };
        loop {
            if matches!(f.advance_one_step(&mut econ), StepOutcome::Completed) {
                break;
            }
        }
        assert_eq!(econ.spent_credits, 25);
    }
```

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k remaining_balance_ladder_matches_stepper cost25_ladder_sums_to_exactly_25`
- (the `rebuild_shadow` signature change cascades to world/mod.rs — done in P3-T4; the tree will
  not compile between this task and P3-T4. Do P3-T3 and P3-T4 together before re-checking.)

---

### P3-T4 — thread `rules` into `rebuild_shadow`; add the conservation assert

**File (EDIT):** `src/sim/world/mod.rs`

**(i)** `refresh_production_shadow` (:1006) now splits on the `Option`:

```rust
    pub(crate) fn refresh_production_shadow(&mut self, rules: Option<&RuleSet>) {
        self.refresh_economy_shadow(rules);
        // Take the registry out so `rebuild_shadow` can borrow `&self` while writing
        // the registry; swap it back after. `rebuild_shadow` reads
        // `queues_by_owner` (the legacy source), not `factory_shadow`, so the
        // temporarily-defaulted field is fine.
        let mut registry = std::mem::take(&mut self.production.factory_shadow);
        match rules {
            Some(r) => registry.rebuild_shadow(self, r), // cost-based oracle balance (P3)
            None => registry.rebuild_shadow_no_rules(self), // cost-0 fallback (None tail)
        }
        self.production.factory_shadow = registry;
    }
```

**(ii)** Add the conservation assert and wire it into `debug_assert_production_shadow` (:1020).
The assert checks the INTRINSIC conservation invariant of `advance_one_step` on each live shadow
factory — it steps a CLONE of the factory against a CLONE economy seeded with exactly
`original_balance`, and SURFACES any drift (tick + owner + category), never writes back. Mirrors
the `debug_assert_s1_shadow` surface-not-equalize discipline (A12).

```rust
    /// Debug-only P3 assert: each live shadow factory's `advance_one_step` conserves
    /// exact cost (C15) and settles correctly (C2/C12). Steps a CLONE against a CLONE
    /// economy seeded with exactly `original_balance`; SURFACES divergence with
    /// tick + owner + category, NEVER writes back to the shadow or the wallet.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_factory_conservation(&self) {
        use crate::sim::economy::Economy;
        use crate::sim::production::{StepOutcome, PRODUCTION_STEPS};
        for factory in self.production.factory_shadow.iter_insertion_ordered() {
            if factory.object.is_none() {
                continue; // queue-only / no active object: nothing to conserve
            }
            let cost = factory.original_balance;
            // A fresh, armed clone driven from progress 0 with exact funds.
            let mut f = factory.clone();
            f.progress = 0;
            f.balance = cost;
            f.on_hold = false;
            f.suspended = false;
            f.manual = false;
            let mut econ = Economy {
                credits: cost,
                ..Economy::default()
            };
            let mut steps = 0i32;
            loop {
                match f.advance_one_step(&mut econ) {
                    StepOutcome::Stepped => steps += 1,
                    StepOutcome::Completed => {
                        steps += 1;
                        break;
                    }
                    // Stalled/Idle cannot happen with exact funds + a fresh arm; the
                    // asserts below fire (steps != 54) and surface the divergence.
                    _ => break,
                }
            }
            debug_assert_eq!(
                steps, PRODUCTION_STEPS as i32,
                "C2: tick {} {:?}/{:?}: a full build must take 54 steps (got {})",
                self.tick, factory.owner, factory.category, steps,
            );
            debug_assert_eq!(
                econ.spent_credits, cost,
                "C15: tick {} {:?}/{:?}: total spent {} must equal full cost {}",
                self.tick, factory.owner, factory.category, econ.spent_credits, cost,
            );
            debug_assert_eq!(
                f.balance, 0,
                "C12: tick {} {:?}/{:?}: completion must zero the balance",
                self.tick, factory.owner, factory.category,
            );
            debug_assert!(
                f.suspended && f.object.is_some(),
                "C12: tick {} {:?}/{:?}: completion must suspend with the object attached",
                self.tick, factory.owner, factory.category,
            );
        }
    }
```

Add the call to `debug_assert_production_shadow` (:1021):

```rust
    #[cfg(debug_assertions)]
    pub(crate) fn debug_assert_production_shadow(&self) {
        self.debug_assert_economy_shadow();
        self.debug_assert_factory_shell_trace();
        self.debug_assert_factory_conservation(); // NEW (P3)
    }
```

> `Factory`/`category`/`owner` are `pub`; `iter_insertion_ordered()` returns `&Factory`, so
> `factory.clone()` + field access compile. `ProductionCategory` derives `Debug` (it must, for
> the `{:?}` in the message — verify; it derives `Ord`, which on this enum implies the standard
> derive set includes `Debug`. If not, add `Debug`).

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k production_shadow_preserves_advance_tick_phase_order factory_shadow_progress_tracks_legacy_remaining factory_registry_shadow_no_hash_change snapshot_roundtrip_ignores_shadow` — the P2 tests still pass with the cost-based shadow (they assert progress/hash/round-trip, not balance, so cost-based seeding does not perturb them; the conservation assert runs in debug and must not fire)

> NOTE on P2 test `factory_shadow_progress_tracks_legacy_remaining`: it uses `empty_rules()`, so
> `object_type("GRIZZLY")` resolves to `None` → cost 0 → `original_balance = 0`,
> `balance = 0`. The conservation assert skips/handles cost-0 cleanly (a 0-cost full build is 54
> free steps, `spent == 0 == cost`). Confirm the assert does not fire for the empty-rules
> fixtures; if a cost-0 active object makes the `Completed`-with-object assert awkward, the assert
> already handles it (cost 0 still completes in 54 steps with the object attached). No change
> needed, but re-run with `--debug` to confirm.

---

### P3-T5 — the oracle-step probe BESIDE the no-op arm (FIT-a, hash-neutral)

**File (EDIT):** `src/sim/world/techno_ai.rs`

The `EntityCategory::Structure => {}` arm (techno_ai.rs:107) STAYS a literal no-op. The oracle
step runs as a debug-only probe extending the existing `factory_shell_trace` block (:252), the
"proof lives beside, not inside, the no-op arm" shape. It walks live Structures in LogicVector
order, clones each owner's factories + economy, exercises `set_rate` + `advance_one_step` on the
clones, and records the outcomes locally — NEVER writing back (the S1 template, A12).

Add after `factory_shell_trace` (after line 271), inside the `impl Simulation` block:

```rust
    /// Debug-only P3 oracle probe: walk live Structures in LogicVector order and,
    /// for each, step a CLONE of its owner's factories against a CLONE of the
    /// owner's economy — exercising `set_rate` + `advance_one_step` on throwaways.
    /// READ-ONLY w.r.t. all hashed state: it writes only local clones, NEVER the
    /// registry, the wallet, or any entity. The arm stays a no-op; this is the
    /// "proof beside the no-op" shape (FIT option a). The full per-building
    /// Primary_For* routing is a later slice — the probe uses a bounded per-owner
    /// scope (every factory the visited Structure's owner holds), which is
    /// hash-neutral regardless of routing precision.
    #[cfg(any(test, debug_assertions))]
    pub(crate) fn factory_oracle_step_trace(&self) -> Vec<(u64, StepOutcome)> {
        use crate::sim::economy::Economy;
        let mut out: Vec<(u64, StepOutcome)> = Vec::new();
        for id in self.live_object_order_snapshot() {
            let Some(entity) = self.substrate.entities.get(id) else {
                continue;
            };
            if entity.dying || entity.category != EntityCategory::Structure {
                continue;
            }
            let owner = entity.owner;
            // Clone the owner's economy (the oracle wallet); default if no house.
            let mut oracle_econ = self
                .houses
                .get(&owner)
                .map(|h| h.economy.clone())
                .unwrap_or_default();
            // Bounded scope: step a clone of each of this owner's factories. The
            // registry is a LOOKUP (FIT a); we read it, never mutate it.
            for factory in self.production.factory_shadow.iter_insertion_ordered() {
                if factory.owner != owner || factory.object.is_none() {
                    continue;
                }
                let mut oracle_factory = factory.clone();
                // Exercise SetRate so the rate is non-stale (the build-step total is
                // a placeholder until the GetBuildStepTime pipeline lands; use the
                // factory's own original_balance frames as a stand-in input — the
                // probe only proves the step machine runs, not the rate value).
                oracle_factory.set_rate(oracle_factory.original_balance);
                let outcome = oracle_factory.advance_one_step(&mut oracle_econ);
                out.push((id, outcome));
                // local clones dropped here; nothing written back.
            }
        }
        out
    }
```

> `StepOutcome` must be in scope and `Clone`/`Debug` to be collected into a `Vec` and printed in
> a panic message. It is a unit enum; ensure `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` on it
> (factory.rs:98) — added in P3-T2's note. Import path: `StepOutcome` is re-exported at
> `crate::sim::production::StepOutcome` (production/mod.rs:48-51); add a `use` at the call site or
> fully-qualify.

The arm stays no-op; add a one-line discoverability comment on it:

```rust
        EntityCategory::Structure => {} // S8 absorb bracket; P3 oracle probe lives in factory_oracle_step_trace
```

> Do NOT wire `factory_oracle_step_trace` into `debug_assert_production_shadow` as an authoritative
> driver — it is exercised by the P3-T6 determinism/no-hash tests only. If a debug-time
> well-formedness assert is wanted, a `debug_assert_factory_oracle_probe` mirroring
> `debug_assert_factory_shell_trace` may call it and assert the outcome vec is well-formed (every
> id resolves to a live Structure), but that is OPTIONAL — the load-bearing guarantee is the
> no-hash test, not a runtime assert. Mark it deferred if not added.

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k techno_ai_shell_is_passthrough_no_hash_change` — the arm is still a
  no-op, so the P2 passthrough test is unchanged

---

### P3-T6 — the no-hash + determinism acceptance tests

**File (EDIT):** `src/sim/world/production_shadow_tests.rs` — append. Reuse `empty_rules()`,
`queued_item`, `insert_queue` (A13). Import `PendingObject`/`StepOutcome`/`Economy` as needed
(the `use crate::sim::production::{...}` line at :17 already imports `PRODUCTION_STEPS`; extend
it; `Economy` is imported at :13).

```rust
/// P3 no-hash guarantee: stepping a CLONE of a shadow factory against a CLONE of the
/// wallet 54 times leaves `state_hash()` bit-identical (the oracle never touches the
/// hashed wallet; `Factory`/`Economy` carry no serde derive). The acceptance test.
#[test]
fn factory_advance_step_does_not_change_state_hash() {
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

    // Step a CLONE of the shadow factory against a CLONE of the wallet, 54 times.
    let mut f = sim.production.factory_shadow.iter_insertion_ordered()[0].clone();
    // empty_rules -> cost 0 -> seed a real cost so the step machine actually charges.
    f.progress = 0;
    f.balance = 700;
    f.original_balance = 700;
    let mut oracle = sim.houses[&owner].economy.clone();
    for _ in 0..PRODUCTION_STEPS {
        let _ = f.advance_one_step(&mut oracle);
    }
    sim.refresh_production_shadow(Some(&rules)); // rebuild again

    assert_eq!(
        before,
        sim.state_hash(),
        "P3 oracle stepping must not perturb the state hash (serde-skip + clone)"
    );
    // And the real wallet is untouched (the oracle was a clone).
    assert_eq!(
        sim.houses[&owner].credits, 1_000_000,
        "the legacy wallet is untouched by oracle stepping"
    );
}

/// P3 determinism: identical fixtures over N ticks (with the oracle probe + the
/// conservation assert active in debug) produce identical per-tick state_hash
/// sequences. Guards against the probe introducing nondeterminism.
#[test]
fn production_shadow_with_oracle_is_deterministic() {
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
                sim.advance_tick(&[], None, &heights, Some(&rules), None, 67);
                sim.state_hash()
            })
            .collect()
    }
    assert_eq!(run(), run(), "advance_tick with the P3 oracle probe stays deterministic");
}
```

> Verify the exact `advance_tick` signature at impl time. The P2 determinism test calls
> `sim.advance_tick(&[], None, &heights, None, None, 67)` (production_shadow_tests.rs:343) — the
> 4th arg is the `Option<&RuleSet>` tail. Pass `Some(&rules)` here so the cost-based rebuild +
> conservation assert run. If the signature differs in the current tree, mirror the existing P2
> call exactly and only swap the rules arg.

**Verification:**
- `cargo check -p vera20k`
- `cargo test -p vera20k factory_advance_step_does_not_change_state_hash production_shadow_with_oracle_is_deterministic`

---

### P3-T7 — full-suite verify + no-bump / no-hash-file lock (separate foreground pass)

Per the build-discipline memory (don't bury slow cargo inside a background workflow), run the
verification as a separate bounded foreground pass.

**Verification:**
- `cargo test -p vera20k` — read the literal `test result:` line. The P3 set must pass:
  `factory_54_steps_to_complete`, `factory_exact_cost_conservation`,
  `factory_exact_cost_conservation_cost1_corner`, `factory_last_step_charges_full_remainder`,
  `factory_stall_on_no_funds_rewinds`, `factory_exactly_affordable_step_proceeds`,
  `factory_cost_zero_completes_free`, `remaining_balance_ladder_matches_stepper`,
  `cost25_ladder_sums_to_exactly_25`, `set_rate_total_over_54_truncates_clamps`,
  `set_rate_zero_when_no_object`, `factory_advance_step_does_not_change_state_hash`,
  `production_shadow_with_oracle_is_deterministic`.
- The P1+P2 tests must still pass: `economy_*`, `factory_shadow_*`, `insertion_seq_*`,
  `snapshot_roundtrip_ignores_shadow`, `production_shadow_preserves_advance_tick_phase_order`,
  `snapshot_version_is_17_in_shadow_phase` (snapshot.rs:374),
  `techno_ai_shell_is_passthrough_no_hash_change`.
- `cargo test -p vera20k snapshot_version_is_17_in_shadow_phase` — confirms SNAPSHOT_VERSION
  still 17.
- Confirm `git diff --stat` shows NO change to `src/sim/world/world_hash.rs` and NO change to
  `SNAPSHOT_VERSION` in `src/sim/snapshot.rs` (the no-hash contract).

---

## D. Out-of-scope seams (left clean, NOT implemented)

| Concern | Status | Seam |
|---|---|---|
| Authority flip (oracle → real wallet), SNAPSHOT_VERSION 17→18 | P5 | `advance_one_step(&mut Economy)` signature is P5-ready; flip the call site (arm body) + add the timer driver, not the method. |
| Cancel / partial refund (`original_balance − balance`) | P4 | `original_balance` + `balance` hold the spent split (C8). |
| Prereq revalidation (3-way) / `on_hold` auto-unstick on resume | P6 | `BuildEligibility` declared (factory.rs:108); P3 does NOT auto-clear `on_hold` (a stalled factory stays stalled). |
| Full `GetBuildStepTime` pipeline (low-power C10, MultipleFactory C11) | P-later | `set_rate(build_step_time: i32)` takes the total as input; the pipeline producer plugs in. |
| Purifier / IncomeMult / HarvestedCredits | P7 | `Economy` fields present; not exercised by P3. |
| Delivery / queue advance / object clear | P5+ | completion leaves `object: Some(..)`, `suspended=true`; delivery is command-bound (C7). |
| `step_timer` per-tick authoritative driver | P3 stepper is timer-free | the per-tick countdown is a P5 timing concern (U2). |
| Building-type → ProductionCategory routing (Primary_For*) | P5 | the probe uses a bounded per-owner scope; full routing is P5. |

---

## E. Open questions for the design-lead (confirm before / during implementing)

**E1 — `factory_54_steps_to_complete` Stepped-count framing.** The study §8-P3 says "exactly 54
`Stepped` outcomes" (line 727), but C12 makes the 54th step return `Completed` (not `Stepped`).
The plan asserts **53 `Stepped` + 1 `Completed` = 54 total step calls** (the mechanically-correct
reading). Confirm this is the intended interpretation (it must be — completion can't also be a
plain `Stepped`), or whether the design wants the 54th outcome relabeled. The conservation /
total-step-count guarantee (C2/C15) holds either way.

**E2 — SetRate `build_step_time` input in the probe (P3-T5).** The probe feeds
`oracle_factory.original_balance` (a CREDIT cost) as the `build_step_time` (a FRAME total) just to
exercise `set_rate` non-trivially. These are different quantities (U3 in the design); the probe
does not assert the resulting rate value, only that the step machine runs. Confirm this stand-in
is acceptable for the probe, or whether `set_rate` should be left out of the probe entirely until
the GetBuildStepTime pipeline (C10/C11) lands and supplies a real total. (Leaving it out is the
zero-risk option; including it exercises the C5 path on a live walk.)

**E3 — optional `debug_assert_factory_oracle_probe`.** P3-T5 leaves the probe's runtime
well-formedness assert OPTIONAL (the load-bearing guarantee is the no-hash test, not a runtime
assert). Confirm whether to add it (mirroring `debug_assert_factory_shell_trace`) or leave the
probe as a test-exercised accessor only. The plan defaults to NOT adding it (smaller surface).

**E4 — `rebuild_shadow` two-arity vs inner-delegate.** P3-T3 refactors the body into a private
`rebuild_shadow_inner(sim, Option<&RuleSet>)` with `rebuild_shadow(sim, &RuleSet)` and
`rebuild_shadow_no_rules(sim)` delegating, to avoid duplicating the loop. Confirm the
inner-delegate shape (vs two full bodies). The inner-delegate is the lower-drift default.

---

*End of P3 plan. The slice is additive and oracle-only: `advance_one_step`/`set_rate` are pure
`Factory` methods exercised against a CLONED economy; the legacy upfront-charge stays
authoritative; `world_hash.rs`/`snapshot.rs` are untouched and `SNAPSHOT_VERSION` stays 17. The
authority flip (oracle → real wallet), the timer-gated per-tick driver, and the 17→18 bump land
at P5 (out of scope). SetRate deliberately takes the build-step total as an input rather than
deriving it from the legacy `build_time_base_frames`, which carries the verified-REFUTED ×0.9 —
the decisive parity choice this slice makes.*
