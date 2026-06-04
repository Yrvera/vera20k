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

use crate::rules::object_type::ObjectType;
use crate::rules::ruleset::RuleSet;
use crate::sim::economy::Economy;
use crate::sim::intern::InternedId;
use crate::sim::production::production_tech::production_category_for_object;
use crate::sim::production::production_types::{PRODUCTION_RATE_SCALE, ProductionCategory};

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

    /// AbandonProduction the ACTIVE object (C8): refund the ALREADY-PAID portion
    /// (`original_balance - balance`, the spent credits) to the (oracle) economy, then
    /// reset to the empty-but-registered idle state (the partial object is destroyed).
    /// Returns `Some(refund)` when it ACTED (refund may be 0 for a not-yet-charged
    /// build) and `None` on a NO-OP — no active object, OR a complete-but-held object
    /// (the "no-op after completion" rule: a finished-but-undelivered build is
    /// cancelled through the ready-queue path, a later slice). Leaves the queue tail
    /// INTACT — `start_next_queued` is command-bound (C7) and is NOT auto-invoked here.
    ///
    /// `&mut Economy` is an ORACLE (clone) in P4; hash-neutrality is enforced at the
    /// CALL SITE, never in this body. The authority-flip slice flips WHO is passed.
    fn cancel_active(&mut self, economy: &mut Economy) -> Option<i32> {
        // No active object -> no-op.
        self.object.as_ref()?;

        // No-op after completion: a complete-but-held object (progress 54, suspended,
        // object attached) is NOT abandoned via this path — it awaits delivery, and
        // cancelling a completed build goes through the ready-queue path (a later
        // slice). Returning None keeps the completed object + its state intact.
        if self.progress >= PRODUCTION_STEPS {
            return None;
        }

        // C8: refund the already-paid (spent) portion. `balance` is the remaining
        // unpaid amount, charged down per step; `original_balance` is the full-cost
        // snapshot. `original_balance - balance` is exactly what the per-step ladder
        // removed (NOT the full cost — that is the legacy DRIFT). `.max(0)` documents
        // intent; the invariant `balance <= original_balance` holds (the stepper only
        // decrements balance), so it never fires in a well-formed shadow.
        let refund = (self.original_balance - self.balance).max(0);
        economy.add_credits(refund); // ORACLE economy in P4 (saturating add)

        // Reset to the empty-but-registered idle state; the partial object is
        // destroyed. In the P4 shadow `object.entity_id` is always None (the legacy
        // path owns the produced entity), so "destroy the partial object" is exactly
        // `object = None`; the real partial-object despawn hooks in at P5.
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

    /// Pop the FRONT of the queue into a fresh active object (FIFO StartNextQueued,
    /// C6). Returns the popped `type_id`, or `None` when blocked/empty. PROVEN-but-
    /// DORMANT in P4: no `advance_tick`/command path calls this — the queue advance is
    /// command-bound to a successful delivery (C7), wired in a later slice. P4 only
    /// proves the pure pop mechanics + the gating guard.
    ///
    /// GUARD (C7/C12): a held object blocks the advance. A completed-but-held factory
    /// (progress 54, suspended, object attached) is a NO-OP here — the queue does not
    /// advance on completion alone; the delivery commit clears the object first.
    pub(crate) fn start_next_queued(&mut self) -> Option<InternedId> {
        // "Object null required": an in-flight OR completed-held object is never displaced.
        if self.object.is_some() {
            return None;
        }
        let next = self.queue.pop_front()?; // FIFO FRONT pop; None on an empty queue
        self.object = Some(PendingObject {
            type_id: next,
            entity_id: None,
        });
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
}

/// Resolved inputs for the build-step TOTAL producer. A transient param struct (NO
/// serde, NO storage, only `Debug`/`Clone`) so the producer is a pure function of
/// explicit inputs — testable in isolation, no `Simulation` handle. The caller (the
/// authority-flip begin path / the inversion-readiness assert) gathers these from rules
/// + the owner's power + the per-category factory count. PPM scale =
/// `PRODUCTION_RATE_SCALE` (1_000_000 = 1.0), so the parsed `*_ppm` rules fields feed it
/// directly.
#[derive(Debug, Clone)]
pub struct BuildStepTimeInputs {
    /// GetCost of the object under construction.
    pub cost: i32,
    /// Per-CATEGORY build-time bonus (side multiplier), default 1.0 =
    /// `PRODUCTION_RATE_SCALE`. NOT a generic build-speed and NOT a single house scalar.
    /// Stock YR (no per-side bonus) passes 1.0 — no rules field backs it yet.
    pub build_time_bonus_ppm: u64,
    /// Per-TYPE BuildTimeMultiplier, pre-scaled to PPM by the caller.
    pub build_time_multiplier_ppm: u64,
    /// Owner power ratio, clamped to `[0, SCALE]`; `SCALE` (1.0) when not under-powered.
    pub power_ratio_ppm: u64,
    /// LowPowerPenaltyModifier (already PPM-parsed).
    pub low_power_penalty_modifier_ppm: u64,
    /// MinLowPowerProductionSpeed (Min divisor clamp, applied ALWAYS).
    pub min_clamp_ppm: u64,
    /// MaxLowPowerProductionSpeed (Max divisor clamp, applied ONLY when ratio < 1.0).
    pub max_clamp_ppm: u64,
    /// MultipleFactory (loop gate, strict `> 0`).
    pub multiple_factory_ppm: u64,
    /// Per-category matching factory count (the `(n - 1)` loop count).
    pub factory_count: u32,
    /// True only for a wall building (building category AND the wall flag).
    pub is_wall: bool,
    /// BuildSpeed wall coefficient, pre-converted to PPM by the caller (used only when
    /// `is_wall`).
    pub wall_build_speed_ppm: u64,
}

/// Produce the build-step TOTAL — the per-step build-time pipeline's return BEFORE the
/// caller's `/54 + clamp[1,255]` (`set_rate` owns that). PURE: integer/i128 throughout,
/// no `&mut`, no RNG, no hashed-state read, no float in the committed math. The legacy
/// `production_tech` build-time family is a verified DRIFT (it bakes a REFUTED ×0.9 via
/// `* 9 / 10000`, models build time as a rate-domain single-truncate division, and uses
/// a generic build-speed instead of the per-category bonus) and is NOT reused.
///
/// Pipeline (every multiply/divide truncates toward zero = floor for non-negatives):
///   T1  base = trunc(BuildTimeBonus × Cost)                 (NO ×0.9)
///   T2  × per-type BuildTimeMultiplier, trunc
///   T3  ÷ divisor d = 1 − (1 − ratio) × LPPM, clamped:
///         Min clamp ALWAYS; Max clamp ONLY when ratio < 1.0; d ≤ 0 floors to 0.01
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

    // T3: low-power divide. divisor d = 1 − (1 − ratio) × LPPM, clamped.
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

    // T4: MultipleFactory loop — (count − 1) iters, PER-ITERATION trunc, gated MF > 0.
    if inp.multiple_factory_ppm > 0 && inp.factory_count > 1 {
        for _ in 0..(inp.factory_count - 1) {
            acc = acc * inp.multiple_factory_ppm as i128 / SCALE; // trunc EACH iter
        }
    }

    // T5: wall branch — wall building only, trunc(acc × BuildSpeed).
    if inp.is_wall {
        acc = acc * inp.wall_build_speed_ppm as i128 / SCALE; // trunc
    }

    acc.clamp(0, i32::MAX as i128) as i32 // the TOTAL; set_rate does /54 + clamp[1,255]
}

/// Map an object type to the `ProductionCategory` whose factory produces it — the Rust
/// analog of the engine's begin-production factory-slot resolution. A thin tested
/// delegate over `production_category_for_object`: ONE routing source, not a fork. Its
/// value is being the single call site the authority-flip registry sweep will use, and
/// the place the routing DRIFTs are pinned by tests.
///
/// SURFACED DRIFT (NOT resolved here): the engine keeps a 6th factory slot for Ships,
/// but Rust has no `Ship` `ProductionCategory` — naval object types collapse into
/// `Vehicle`. When a house owns both a War Factory and a Naval Yard, the single
/// `Vehicle` factory key collapses two engine factories, diverging the MultipleFactory
/// count and same-frame completion ordering. That is a later structural decision (add
/// `Ship` vs accept the collapse) requiring sign-off — NEVER silently folded.
pub fn category_for_object(obj: &ObjectType) -> ProductionCategory {
    production_category_for_object(obj)
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

/// Outcome of a `FactoryRegistry::cancel_one` (consumer: tests). Serde-free — the
/// same no-hash discipline as `StepOutcome`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)] // NO serde
pub enum CancelOutcome {
    /// No factory for (owner, category), OR the type matched neither a queued tail
    /// copy nor an abandonable active object (incl. the complete-but-held case).
    /// A true no-op: zero economy mutation, zero state change.
    NoMatch,
    /// A queued tail copy of `type_id` was removed (FIRST match, front-to-back). No
    /// refund — a queued item was never charged (its spent portion is 0).
    QueuedRemoved,
    /// The active object was AbandonProduction'd; `refund` credits returned (C8).
    AbandonedActive { refund: i32 },
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

    /// Cancel one production of `type_id` for (owner, category) — the substrate analog
    /// of the engine's cancel-one command. PURE on the registry + an ORACLE (clone)
    /// economy in P4 (never the hashed wallet; the legacy `cancel_by_type_for_owner`
    /// stays authoritative through the authority-flip slice). Precedence (C6 / §6.2 OR,
    /// queued path named first): a QUEUED tail copy is removed FIRST (front-to-back,
    /// FIRST match — RemoveFromQueue); ONLY when no queued copy of `type_id` matches AND
    /// the ACTIVE object is `type_id` is the active build abandoned (refund =
    /// original_balance - balance, AbandonProduction). No match -> NoMatch.
    ///
    /// `&mut Economy` is an ORACLE (clone) in P4; the authority-flip slice flips WHO is
    /// passed, not this body.
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
        // `position()` scans front-to-back returning the FIRST index; `remove(idx)`
        // shifts survivors down (relative order preserved). DRIFT fix vs the legacy
        // `.rev()` last-match: [A,B,A,C] cancel A -> remove index 0 -> [B,A,C].
        if let Some(idx) = f.queue.iter().position(|&t| t == type_id) {
            f.queue.remove(idx);
            return CancelOutcome::QueuedRemoved; // no refund: a queued item is uncharged
        }

        // (R2) ELSE the ACTIVE object, if it is this type AND abandonable.
        // `cancel_active` no-ops (None) on a complete-but-held object -> NoMatch.
        if f.object.as_ref().map(|o| o.type_id) == Some(type_id) {
            return match f.cancel_active(economy) {
                Some(refund) => CancelOutcome::AbandonedActive { refund },
                None => CancelOutcome::NoMatch,
            };
        }

        // (R3) no queued copy, active is a different type (or none) -> no-op.
        CancelOutcome::NoMatch
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

                // insertion_seq = the front (earliest still-live) item's enqueue_order:
                // the temporal first-Begin stamp of when this (owner, category) began
                // producing. Reproduces the native factory array's temporal tail-append
                // order, NOT the BTreeMap sorted-(owner, category) order the old
                // next_insertion_seq++ mint produced. enqueue_order is strictly
                // monotonic, so ties are impossible; a lapsed-then-restarted category
                // re-reads a fresh, higher front enqueue_order each rebuild, matching the
                // native destroy-recreate -> tail re-append. `seq_carry` /
                // `next_insertion_seq` are no longer the ordering SOURCE (the carry kept
                // a stale seq across a queue-empty gap, or re-minted in sorted position);
                // they stay written for a minimal diff and are retired at the flip.
                let seq = front.enqueue_order;
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

    // ---- P4 cancel / refund / FIFO tests ----

    /// Insert a factory at (owner, category) into a registry (test helper; the
    /// `factories` map is private but in-module).
    fn reg_with(owner: InternedId, category: ProductionCategory, f: Factory) -> FactoryRegistry {
        let mut reg = FactoryRegistry::default();
        reg.factories.insert((owner, category), f);
        reg
    }

    #[test]
    fn cancel_active_refunds_spent_only() {
        // Step an armed cost-700 build to progress 20, then cancel the active object:
        // the refund equals the SPENT portion (original_balance - balance) and the
        // oracle returns to its pre-build credits (C8/C15). The factory resets to idle.
        let mut f = armed_factory(700);
        let mut econ = Economy { credits: 700, ..Economy::default() };
        while f.progress < 20 {
            assert!(matches!(f.advance_one_step(&mut econ), StepOutcome::Stepped));
        }
        let spent = econ.spent_credits;
        let expected_refund = f.original_balance - f.balance;
        assert_eq!(expected_refund, spent, "spent portion == original_balance - balance");
        let refund = f.cancel_active(&mut econ).expect("active build is abandonable");
        assert_eq!(refund, spent, "C8: refund the already-paid spent portion only");
        assert_eq!(econ.credits, 700, "C15: oracle returns to pre-build credits");
        assert!(f.object.is_none(), "the partial object is destroyed");
        assert_eq!(f.progress, 0);
        assert_eq!(f.balance, 0);
        assert_eq!(f.original_balance, 0);
        assert_eq!(f.step_rate_frames, 0, "no-object => rate-0 sentinel");
        assert!(!f.suspended && !f.on_hold && !f.manual);
    }

    #[test]
    fn cancel_active_at_progress_zero_refunds_nothing() {
        // A never-stepped active object ACTED but refunds 0 (spent nothing) — Some(0), not None.
        let mut f = armed_factory(700);
        let mut econ = Economy { credits: 0, ..Economy::default() };
        assert_eq!(f.cancel_active(&mut econ), Some(0), "acted, refund 0 (spent nothing yet)");
        assert_eq!(econ.credits, 0, "no credits added for a zero refund");
        assert!(f.object.is_none(), "factory reset even on a zero-refund cancel");
        assert_eq!(f.progress, 0);
    }

    #[test]
    fn cancel_active_no_object_is_noop() {
        let mut f = Factory::default();
        let mut econ = Economy { credits: 500, ..Economy::default() };
        assert_eq!(f.cancel_active(&mut econ), None);
        assert_eq!(econ.credits, 500, "no-op leaves the oracle untouched");
    }

    #[test]
    fn cancel_active_completed_is_noop() {
        // A complete-but-held object (progress 54, suspended) is NOT abandoned here.
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
        // C15 cancel-side telescoping: stepping k times then cancelling returns the
        // oracle to its starting credits regardless of where the cancel lands.
        for cost in [1i32, 25, 700, 99991] {
            for stop_at in [0u16, 1, 20, 53] {
                let mut f = armed_factory(cost);
                let mut econ = Economy { credits: cost, ..Economy::default() };
                while f.progress < stop_at {
                    if !matches!(f.advance_one_step(&mut econ), StepOutcome::Stepped) {
                        break; // a free build may Complete early; harmless
                    }
                }
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

    #[test]
    fn cancel_one_removes_first_matching() {
        // queue [A,B,A,C] (all queued, no active), cancel A -> [B,A,C]: the FIRST
        // (front-most) A is removed, NOT the last (the legacy .rev() DRIFT).
        let owner = InternedId::default();
        let a = InternedId::from_index(1);
        let b = InternedId::from_index(2);
        let c = InternedId::from_index(3);
        let f = Factory {
            owner,
            category: ProductionCategory::Vehicle,
            queue: std::collections::VecDeque::from(vec![a, b, a, c]),
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
        let f = Factory {
            owner,
            category: ProductionCategory::Vehicle,
            object: Some(PendingObject { type_id: a, entity_id: None }),
            balance: 300,
            original_balance: 700,
            progress: 20,
            queue: std::collections::VecDeque::from(vec![a]),
            ..Factory::default()
        };
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
            queue: std::collections::VecDeque::from(vec![b]),
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
        // (1) no factory for the key -> NoMatch; (2) type absent from active+tail -> NoMatch.
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
        let f = Factory {
            owner,
            category: ProductionCategory::Vehicle,
            object: Some(PendingObject { type_id: a, entity_id: None }),
            balance: 300,
            original_balance: 700,
            queue: std::collections::VecDeque::from(vec![a]),
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

    #[test]
    fn start_next_queued_pops_front() {
        // queue [X,Y,Z], no active object -> active = X, queue [Y,Z] (FIFO front pop).
        let x = InternedId::from_index(1);
        let y = InternedId::from_index(2);
        let z = InternedId::from_index(3);
        let mut f = Factory {
            object: None,
            queue: std::collections::VecDeque::from(vec![x, y, z]),
            ..Factory::default()
        };
        assert_eq!(f.start_next_queued(), Some(x), "the FRONT is popped");
        assert_eq!(f.object.as_ref().map(|o| o.type_id), Some(x), "active = X");
        assert_eq!(f.progress, 0, "fresh active object starts at progress 0");
        let q: Vec<InternedId> = f.queue.iter().copied().collect();
        assert_eq!(q, vec![y, z], "queue advanced to [Y,Z]");
    }

    #[test]
    fn start_next_queued_blocked_while_object_held() {
        // object Some -> None, queue unchanged (the "Object null required" guard).
        let x = InternedId::from_index(1);
        let mut f = Factory {
            object: Some(PendingObject::default()),
            queue: std::collections::VecDeque::from(vec![x]),
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
        let mut f = Factory::default();
        assert_eq!(f.start_next_queued(), None);
        assert!(f.object.is_none(), "no object created from an empty queue");
    }

    // ---- P5a build_step_time producer (C5/C10/C11, x0.9-free) ----

    /// Full-power, no-bonus, no-multiplier, single-factory inputs at `cost`.
    fn bst(cost: i32) -> BuildStepTimeInputs {
        BuildStepTimeInputs {
            cost,
            build_time_bonus_ppm: PRODUCTION_RATE_SCALE,      // 1.0
            build_time_multiplier_ppm: PRODUCTION_RATE_SCALE, // 1.0
            power_ratio_ppm: PRODUCTION_RATE_SCALE,           // 1.0 (full power)
            low_power_penalty_modifier_ppm: PRODUCTION_RATE_SCALE,
            min_clamp_ppm: PRODUCTION_RATE_SCALE / 2,             // 0.5
            max_clamp_ppm: (PRODUCTION_RATE_SCALE * 9) / 10,      // 0.9
            multiple_factory_ppm: (PRODUCTION_RATE_SCALE * 8) / 10, // 0.8
            factory_count: 1,
            is_wall: false,
            wall_build_speed_ppm: PRODUCTION_RATE_SCALE,
        }
    }

    #[test]
    fn build_step_time_no_x09_base() {
        // cost 700, all-1.0, count 1, no wall -> TOTAL 700, NOT 630 (the REFUTED ×0.9).
        // Then set_rate(700) -> 700/54 = 12.
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
        // base 67 x mult 1.15 -> trunc(67 * 1.15) = trunc(77.05) = 77.
        let mut inp = bst(67);
        inp.build_time_multiplier_ppm = (PRODUCTION_RATE_SCALE * 115) / 100; // 1.15
        assert_eq!(build_step_time(&inp), 77, "T2 truncates: trunc(67 * 1.15) = 77");
    }

    #[test]
    fn build_step_time_low_power_max_clamp_gated() {
        // ratio 0.5, LPPM 1.0 -> d = 1 - 0.5 = 0.5; Max clamp 0.9 does NOT lower it; Min
        // 0.5 keeps it. cost 100 -> trunc(100 / 0.5) = 200.
        let mut inp = bst(100);
        inp.power_ratio_ppm = PRODUCTION_RATE_SCALE / 2; // 0.5
        assert_eq!(build_step_time(&inp), 200, "under-power doubles the step total");

        // ratio 1.0 (full power): the Max clamp is NOT applied; d = 1.0 -> total = cost.
        let mut full = bst(100);
        full.max_clamp_ppm = PRODUCTION_RATE_SCALE / 2; // a Max that WOULD bite if applied
        assert_eq!(build_step_time(&full), 100, "ratio==1.0 skips the Max clamp");

        // ratio 0.0, LPPM 1.0 -> d = 0.0 -> floored to 0.01 -> trunc(100 / 0.01) = 10000.
        let mut zero = bst(100);
        zero.power_ratio_ppm = 0;
        zero.min_clamp_ppm = 0; // let d hit 0 so the 0.01 floor is exercised
        zero.max_clamp_ppm = PRODUCTION_RATE_SCALE; // Max does not bite
        assert_eq!(build_step_time(&zero), 10_000, "d<=0 floors to 0.01");
    }

    #[test]
    fn build_step_time_multiple_factory_per_iteration_trunc() {
        // count 3, MF 0.8: per-iteration trunc DIFFERS from acc * MF^2 single-truncate.
        // base acc 11; iter1 trunc(11*0.8)=8; iter2 trunc(8*0.8)=6. Single MF^2=0.64 ->
        // trunc(11*0.64)=7. So 6 != 7 proves per-iteration truncation.
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
        big.power_ratio_ppm = 0; // forces a big divide (d floors to 0.01 when min=0)
        big.min_clamp_ppm = 0;
        big.max_clamp_ppm = PRODUCTION_RATE_SCALE;
        assert_eq!(build_step_time(&big), 5_000_000, "no overflow, exact (50000 / 0.01)");
        // Push past i32 to prove the clamp.
        let mut huge = bst(2_000_000_000);
        huge.power_ratio_ppm = 0;
        huge.min_clamp_ppm = 0;
        huge.max_clamp_ppm = PRODUCTION_RATE_SCALE;
        assert_eq!(build_step_time(&huge), i32::MAX, "clamps to i32::MAX");
    }

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
             [GAPOWR]\nCost=800\n[GAPILL]\nCost=500\nBuildCat=Combat\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("rules parse");
        let inf = rules.object("GI").unwrap();
        let veh = rules.object("GRIZZLY").unwrap();
        let air = rules.object("BEAG").unwrap();
        let bld = rules.object("GAPOWR").unwrap();
        let def = rules.object("GAPILL").unwrap();
        assert_eq!(category_for_object(inf), ProductionCategory::Infantry, "infantry -> Infantry (NOT the refuted inverse)");
        assert_eq!(category_for_object(veh), ProductionCategory::Vehicle, "vehicle -> Vehicle");
        assert_eq!(category_for_object(air), ProductionCategory::Aircraft, "aircraft -> Aircraft (NOT the refuted inverse)");
        assert_eq!(category_for_object(bld), ProductionCategory::Building, "plain building -> Building");
        assert_eq!(category_for_object(def), ProductionCategory::Defense, "BuildCat=Combat building -> Defense");
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
        // category), so it routes to Vehicle. This PINS the documented collapse: if a
        // future change adds a Ship category, this test breaks and forces a decision.
        let ini = IniFile::from_str("[VehicleTypes]\n0=DEST\n[DEST]\nCost=1000\n");
        let rules = RuleSet::from_ini(&ini).expect("rules parse");
        let naval = rules.object("DEST").unwrap();
        assert_eq!(
            category_for_object(naval),
            ProductionCategory::Vehicle,
            "naval collapses to Vehicle (the surfaced DRIFT; no Ship category in this slice)"
        );
    }
}
