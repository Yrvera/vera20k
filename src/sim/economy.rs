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
