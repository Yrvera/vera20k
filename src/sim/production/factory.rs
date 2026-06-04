//! Per-(house, category) factory shadow + deterministic registry.
//!
//! P2 introduced these as DERIVED, non-serialized shadow state on `ProductionState`,
//! rebuilt each tick from the authoritative `queues_by_owner`. P3 adds the per-step
//! charge state machine (`advance_one_step`) and the rate (`set_rate`), exercised
//! against an ORACLE (clone) economy — the legacy queue + upfront-charge stay
//! authoritative through the authority flip (P5, out of scope). Divergence is
//! SURFACED, never equalized — the unit-AI shadow discipline.
//!
//! P2/P3 scope: NO `Serialize`/`Deserialize` derive on any type here, so the registry
//! field is provably hash-neutral and `SNAPSHOT_VERSION` stays put. The serde derive
//! + hash fold + the authority flip (oracle -> real wallet) are P5.
//!
//! Determinism: `BTreeMap<(InternedId, ProductionCategory), Factory>` (both key
//! components derive `Ord`) gives sorted iteration for replay/lockstep; no
//! `HashMap`, no fixed-size player array, no `1<<idx` bitmask — satisfies the
//! 30-player scale target. Integer math only; no float, no RNG.
//!
//! Depends on: `sim/intern`, `sim/production/production_types` (ProductionCategory,
//! BuildQueueState), `sim/economy` (the oracle wallet), `rules` (type cost), and
//! `sim/world::Simulation` (read-only) for the derive. NEVER on
//! render/ui/sidebar/audio/net (sim invariant #1).
//!
//! P2/P3 shadow scaffold: several types/methods (`BuildEligibility`, the step-rate
//! clamps, some `Factory` fields) are forward-declared seams consumed by later
//! slices (P4 cancel, P6 prereq revalidation) and are intentionally unused here, so
//! dead-code is allowed module-wide.
#![allow(dead_code)]

use std::collections::{BTreeMap, VecDeque};

use crate::rules::ruleset::RuleSet;
use crate::sim::economy::Economy;
use crate::sim::intern::InternedId;
use crate::sim::production::production_types::ProductionCategory;

/// Build completes at exactly this many progress steps (the engine's step count).
pub const PRODUCTION_STEPS: u16 = 54;
/// Per-step frame-rate clamp (the engine clamps `total/54` into `[1, 255]`).
pub const STEP_RATE_MIN: u16 = 1;
pub const STEP_RATE_MAX: u16 = 255;

/// Replay the per-step charge ladder for `progress` steps to recover the exact
/// running balance an authoritative stepper would hold at that progress. Used to
/// seed the cost-based shadow balance so a freshly-stepped factory and the rebuilt
/// shadow agree (the conservation assert is then meaningful). At most 54 integer
/// iterations; `cost` clamped non-negative; mirrors `advance_one_step`'s charge.
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

/// The object a factory holds from start through delivery. In P2/P3 shadow
/// `entity_id` is always `None` (the produced entity is created by the legacy path);
/// the field is held distinct so the complete-but-not-delivered state is
/// representable now.
#[derive(Debug, Clone, Default, PartialEq, Eq)] // NO serde in P1-P3
pub struct PendingObject {
    pub type_id: InternedId,
    pub entity_id: Option<u64>,
}

/// Engine special/superweapon discriminator. The study proves the writer of the
/// engine's special-item field was never located, so value `0` cannot be proven
/// unreachable and `0`-vs-`(-1)` MUST NOT be collapsed. Three states keep them
/// distinct. In P1-P3 (normal builds) this is always `NoneNeg1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)] // NO serde in P1-P3
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
/// `FactoryRegistry`. In P2/P3 it is DERIVED shadow — the per-step charge stepping
/// (P3) runs against an ORACLE clone, never the hashed wallet.
#[derive(Debug, Clone, Default, PartialEq, Eq)] // NO serde in P1-P3
pub struct Factory {
    pub owner: InternedId,
    pub category: ProductionCategory,
    /// `0..=54`; completion at `PRODUCTION_STEPS`.
    pub progress: u16,
    /// Per-step frame rate = `clamp(GetBuildStepTime()/54, 1, 255)`; `0` when no object.
    pub step_rate_frames: u16,
    /// Frames remaining in the current step (engine CDTimer). Shadow best-effort.
    pub step_timer: u16,
    /// Remaining cost still owed (charged down per step). Cost-based (credits) in P3.
    pub balance: i32,
    /// Full-cost snapshot at start, for exact-cost conservation + cancel refund.
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

impl Factory {
    /// Resume + (re)compute the per-step frame rate from a GIVEN build-step total (C5).
    ///   no object  -> step_rate_frames = 0  (sentinel; the clamp does NOT apply)
    ///   else        -> step_rate_frames = clamp(build_step_time / 54, 1, 255)
    /// `build_step_time` is the already-resolved total (no hidden 0.9 scaling — the
    /// legacy base-frame total bakes a verified-REFUTED x0.9, so it is NOT used here);
    /// the `/54` is signed integer division (truncates toward zero). The full
    /// low-power / multiple-factory pipeline that PRODUCES `build_step_time` is a
    /// later slice. SetRate resumes a system-suspend; a manual (user) pause is left.
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

    /// Advance one step against an ORACLE economy (a clone / throwaway), NOT the
    /// hashed wallet (C2/C3/C4/C12/C15). Hash-neutrality is enforced at the CALL SITE
    /// (P3 only ever passes a clone); the body is wallet-agnostic. The `&mut Economy`
    /// param (distinct from `&mut self`) is the exact shape the authority-flip slice
    /// (P5) makes real — that slice flips WHO is passed, not this algorithm.
    ///
    /// One step per call. The step:
    ///   * increments `progress` first, then reads stepsLeft = 54 - progress;
    ///   * charges `balance / stepsLeft` (the `stepsLeft == 1` step at value 53 thus
    ///     charges `balance/1` = the whole remaining balance; the final
    ///     `stepsLeft == 0` step at value 54 skips the divide (div-by-zero guard) and
    ///     charges 0 — conservation depends on the `/1` step, not the guard step);
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
        // guard) and charges 0; the balance is already drained on the steps_left == 1
        // step (value 53, charge = balance/1).
        let charge = if steps_left == 0 {
            self.balance // 0 here: the balance was drained on the steps_left==1 step
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

        // Completion settlement on reaching 54. The steps_left==1 charge already
        // zeroed the balance, so there is NO second charge here (the engine's
        // completion spend runs as spend(0); charging the remainder twice double-spends).
        if self.progress >= PRODUCTION_STEPS {
            debug_assert_eq!(self.balance, 0, "the steps_left==1 step must have zeroed the balance");
            self.balance = 0; // idempotent; the contract value
            self.suspended = true; // complete-but-not-delivered
            self.step_timer = 0; // the engine zeroes the per-step timer on completion
            // `object` STAYS Some(..); delivery (a later slice) clears it + advances the queue.
            return StepOutcome::Completed;
        }

        StepOutcome::Stepped
    }
}

/// Outcome of a single factory step (consumer is the P3 charge stepper + the
/// conservation assert). Derives are serde-free (the no-hash contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome {
    Idle,
    Stepped,
    Stalled,
    Completed,
}

/// 3-way prerequisite eligibility (P6 consumer; defined now so the registry
/// surface is stable). The active object runs BOTH `(1,0,1)` and `(1,1,1)` gates;
/// queued items only `(1,0,1)`.
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
#[derive(Debug, Clone, Default, PartialEq, Eq)] // NO serde in P1-P3
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
    /// BTreeMap key order).
    pub fn iter_insertion_ordered(&self) -> Vec<&Factory> {
        let mut all: Vec<&Factory> = self.factories.values().collect();
        all.sort_by_key(|f| f.insertion_seq);
        all
    }

    /// P3 SHADOW BUILD: (re)derive the whole registry from the legacy queues each
    /// tick, with a COST-based oracle balance. READ-ONLY w.r.t. all hashed state.
    /// Reuses `seq_carry` to keep `insertion_seq` stable for surviving factories.
    ///
    /// E1 resolved (P3): `original_balance` = the front type's full credit cost (from
    /// `rules`); `balance` = the not-yet-charged remainder, recovered by replaying the
    /// exact per-step charge ladder for `progress` steps (NOT a one-shot proportion).
    /// The per-step charge is in CREDITS, so a cost-based balance is what the oracle
    /// (and the conservation assert) require; the frames-based P2 placeholder is gone.
    pub(crate) fn rebuild_shadow(
        &mut self,
        sim: &crate::sim::world::Simulation,
        rules: &RuleSet,
    ) {
        self.rebuild_shadow_inner(sim, Some(rules));
    }

    /// Cost-free fallback when no `RuleSet` is available (the advance_tick `None`
    /// tail). Same derive as `rebuild_shadow` but cost resolves to 0. Hash-neutral
    /// either way (the registry is `#[serde(skip)]` + no serde derive).
    pub(crate) fn rebuild_shadow_no_rules(&mut self, sim: &crate::sim::world::Simulation) {
        self.rebuild_shadow_inner(sim, None);
    }

    fn rebuild_shadow_inner(
        &mut self,
        sim: &crate::sim::world::Simulation,
        rules: Option<&RuleSet>,
    ) {
        use crate::sim::production::production_types::BuildQueueState as S;

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
                // Guard division by zero on total == 0.
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

                // The front item is the active object when Building/NoFunds/Done; a
                // Paused front is suspended; a Queued front is queue-only.
                let has_object = matches!(front.state, S::Building | S::NoFunds | S::Done);
                let object = if has_object {
                    Some(PendingObject {
                        type_id: front.type_id,
                        entity_id: None, // legacy path owns the produced entity in P2/P3
                    })
                } else {
                    None
                };

                // Tail items become the FIFO queue (order preserved).
                let tail: VecDeque<InternedId> =
                    queue.iter().skip(1).map(|item| item.type_id).collect();

                // E1 (P3): cost-based oracle balance. original_balance = full type
                // cost (snapshot); balance = remainder after `progress` steps of the
                // exact charge ladder. None rules (cost-free tail) => cost 0.
                let full_cost = match rules {
                    Some(r) => sim
                        .object_type(front.type_id, r)
                        .map(|o| o.cost.max(0))
                        .unwrap_or(0),
                    None => 0,
                };
                let original_balance = full_cost;
                let balance = remaining_balance_after(full_cost, progress);

                let factory = Factory {
                    owner,
                    category,
                    progress,
                    // No-object => rate 0 (the contract). The probe (P3) calls
                    // set_rate separately; the rebuild has no build-step total.
                    step_rate_frames: 0,
                    step_timer: 0,
                    balance,
                    original_balance,
                    object,
                    on_hold: matches!(front.state, S::NoFunds),
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

#[cfg(test)]
mod tests {
    use super::*;

    // ---- P2 pure-type tests ----

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
        // Keys order Building < Infantry, but seqs are 1, 0 — iteration must follow
        // insertion_seq (=> [0, 1]), not the BTreeMap key order (=> [1, 0]).
        let mut reg = FactoryRegistry::default();
        let owner = InternedId::default();
        let fa = Factory {
            owner,
            category: ProductionCategory::Building,
            insertion_seq: 1,
            ..Factory::default()
        };
        let fb = Factory {
            owner,
            category: ProductionCategory::Infantry,
            insertion_seq: 0,
            ..Factory::default()
        };
        reg.factories.insert((owner, ProductionCategory::Building), fa);
        reg.factories.insert((owner, ProductionCategory::Infantry), fb);
        let ordered: Vec<u64> = reg
            .iter_insertion_ordered()
            .iter()
            .map(|f| f.insertion_seq)
            .collect();
        assert_eq!(ordered, vec![0, 1], "iteration is insertion_seq order, not map key order");
    }

    // ---- P3 set_rate / charge tests ----

    /// A fresh armed factory holding `cost` credits of work.
    fn armed_factory(cost: i32) -> Factory {
        Factory {
            object: Some(PendingObject::default()),
            balance: cost,
            original_balance: cost,
            ..Factory::default()
        }
    }

    #[test]
    fn set_rate_total_over_54_truncates_clamps() {
        // With an object, rate = clamp(total/54, 1, 255):
        //   0/54=0->clamp 1, 53/54=0->1, 54/54=1, 661/54=12 (MTNK example), 14000/54=259->255
        let cases = [(0, 1u16), (53, 1), (54, 1), (661, 12), (14000, 255)];
        for (total, expected) in cases {
            let mut f = Factory {
                object: Some(PendingObject::default()),
                ..Factory::default()
            };
            f.set_rate(total);
            assert_eq!(f.step_rate_frames, expected, "set_rate({total}) with object must be {expected}");
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

    #[test]
    fn factory_54_steps_to_complete() {
        // From a fresh armed start with funds: 53 `Stepped` then 1 `Completed` (C2);
        // progress reaches 54 (E1: the 54th call is Completed, not a plain Stepped).
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
        assert!(matches!(f.advance_one_step(&mut econ), StepOutcome::Idle), "a settled factory is Idle");
    }

    #[test]
    fn factory_exact_cost_conservation() {
        // Sum of oracle spend over a full build == the full type cost; balance ends 0
        // (C3/C15). Boundary set {1, 25, 700, 99991}.
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
        // The cost-1 corner: 1/k == 0 for k>=2 (the value-1..52 steps charge 0); the
        // lone credit is charged on the steps_left==1 step (value 53, 1/1==1, a
        // Stepped); the final value-54 step charges 0. Conservation depends on the
        // /1 step, not the guard step.
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
    fn factory_steps_left_one_charges_full_remainder() {
        // The balance drains on the steps_left==1 step (value 53, charge=balance/1),
        // NOT the final value-54 step (the div-by-zero guard, which charges 0).
        let mut f = armed_factory(700);
        let mut econ = Economy { credits: 700, ..Economy::default() };
        while f.progress < PRODUCTION_STEPS - 2 {
            assert!(matches!(f.advance_one_step(&mut econ), StepOutcome::Stepped));
        }
        assert_eq!(f.progress, PRODUCTION_STEPS - 2, "stopped two before completion (progress 52)");
        let remainder = f.balance;
        assert!(remainder > 0, "balance is nonzero at progress 52");
        // The steps_left==1 step (value 53) charges the WHOLE remainder, once.
        let spent_before = econ.spent_credits;
        assert!(matches!(f.advance_one_step(&mut econ), StepOutcome::Stepped), "value-53 is a Stepped");
        assert_eq!(econ.spent_credits - spent_before, remainder, "drains the whole remainder once");
        assert_eq!(f.balance, 0, "balance zeroed on the steps_left==1 step");
        // The final value-54 step is the div-by-zero guard: charges 0, Completed.
        let spent_before2 = econ.spent_credits;
        assert!(matches!(f.advance_one_step(&mut econ), StepOutcome::Completed));
        assert_eq!(econ.spent_credits - spent_before2, 0, "the final step charges 0 (guard)");
        assert_eq!(f.balance, 0, "completion leaves balance 0 (no second remainder charge)");
    }

    #[test]
    fn factory_stall_on_no_funds_rewinds() {
        // Oracle one credit below the first step's charge -> Stalled: on_hold set,
        // progress unchanged, NOTHING spent (C4). cost 700 -> first charge 700/53 = 13.
        let mut f = armed_factory(700);
        let first_charge = 700 / (PRODUCTION_STEPS as i32 - 1); // 700/53 = 13
        let mut econ = Economy { credits: first_charge - 1, ..Economy::default() };
        assert!(matches!(f.advance_one_step(&mut econ), StepOutcome::Stalled));
        assert!(f.on_hold, "a shortfall latches on_hold");
        assert_eq!(f.progress, 0, "the tentative step is rewound (net-zero advance)");
        assert_eq!(econ.spent_credits, 0, "a stall spends nothing");
        assert_eq!(econ.credits, first_charge - 1, "the oracle wallet is untouched");
    }

    #[test]
    fn factory_exactly_affordable_step_proceeds() {
        // available == charge PROCEEDS (the strict-< boundary).
        let mut f = armed_factory(700);
        let first_charge = 700 / (PRODUCTION_STEPS as i32 - 1); // 13
        let mut econ = Economy { credits: first_charge, ..Economy::default() };
        assert!(matches!(f.advance_one_step(&mut econ), StepOutcome::Stepped));
        assert_eq!(f.progress, 1);
        assert_eq!(econ.spent_credits, first_charge);
    }

    #[test]
    fn factory_cost_zero_completes_free() {
        // A cost-0 type: every charge is 0, completes with zero spend; conservation
        // holds trivially (sum 0 == original_balance 0).
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

    #[test]
    fn remaining_balance_ladder_matches_stepper() {
        // remaining_balance_after must equal the balance the stepper actually holds.
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
        // floor division never loses/gains a credit: the last charging step takes the
        // whole remainder, so the per-step charges sum to exactly the cost.
        let mut f = armed_factory(25);
        let mut econ = Economy { credits: 25, ..Economy::default() };
        loop {
            if matches!(f.advance_one_step(&mut econ), StepOutcome::Completed) {
                break;
            }
        }
        assert_eq!(econ.spent_credits, 25);
    }
}
