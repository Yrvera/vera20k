//! Per-(house, category) factory shadow + deterministic registry.
//!
//! P2 introduces these as DERIVED, non-serialized shadow state on `ProductionState`,
//! rebuilt each tick from the authoritative `queues_by_owner`. They mirror the
//! engine's per-(house, category) production state machine on the relevant fields,
//! but the legacy queue stays authoritative through the authority flip (out of
//! scope). Divergence is SURFACED, never equalized — the unit-AI shadow discipline.
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
//! Depends on: `sim/intern`, `sim/production/production_types` (ProductionCategory,
//! BuildQueueState), and `sim/world::Simulation` (read-only) for the derive.
//! NEVER on render/ui/sidebar/audio/net (sim invariant #1).
//!
//! P1+P2 shadow scaffold: several types/methods (`StepOutcome`, `BuildEligibility`,
//! the step-rate clamps, some `Factory` fields) are forward-declared seams consumed
//! by later slices (P3 per-step charge, P4 cancel, P6 prereq revalidation) and are
//! intentionally unused here, so dead-code is allowed module-wide.
#![allow(dead_code)]

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

/// Outcome of a single factory step (consumer is the per-step charge slice, P3).
/// Defined now so the registry surface is stable.
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

    /// P2 SHADOW BUILD: (re)derive the whole registry from the legacy queues each
    /// tick. READ-ONLY w.r.t. all hashed state. Reuses `seq_carry` to keep
    /// `insertion_seq` stable for surviving (owner, category) factories.
    ///
    /// E1 decision: balance/step_rate are derived from the legacy base-frame counts
    /// (a cost-free monotone projection) — `BuildQueueItem` carries no cost and the
    /// derive has no `&RuleSet`. The P2 asserts are monotone-tracking only, so this
    /// satisfies them; the exact `cost - progress*cost/54` shape + real step rate
    /// are owned by the authoritative per-step charge slice (P3).
    pub(crate) fn rebuild_shadow(&mut self, sim: &crate::sim::world::Simulation) {
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
                        entity_id: None, // legacy path owns the produced entity in P2
                    })
                } else {
                    None
                };

                // Tail items become the FIFO queue (order preserved).
                let tail: VecDeque<InternedId> =
                    queue.iter().skip(1).map(|item| item.type_id).collect();

                let factory = Factory {
                    owner,
                    category,
                    progress,
                    // No-object => rate 0 (the contract); with-object stays 0 in P2
                    // (exact rate needs the build-time path / rules — P3).
                    step_rate_frames: 0,
                    step_timer: 0,
                    // Frames-based shadow balance (E1): cost-free monotone projection.
                    balance: front.remaining_base_frames as i32,
                    original_balance: front.total_base_frames as i32,
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
}
