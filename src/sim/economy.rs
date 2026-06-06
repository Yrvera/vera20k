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
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

    /// Accumulate a PRE-COMPUTED HarvestedCredits figure (already the x5.0 product). The
    /// OrePurifier-bonus stat term is `trunc(count * 0.25 * amount * 5.0)` computed in ONE
    /// step (NOT floor-the-bonus-bales-then-x5), so the caller passes the finished value.
    /// Statistics only — never touches `credits`.
    pub fn add_harvested_raw(&mut self, harvested: i32) {
        self.harvested_credits = self.harvested_credits.saturating_add(harvested);
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

/// Parts-per-million scale for `IncomeMult` (1_000_000 = 1.0×). MUST equal
/// `rules::ruleset::INCOME_PPM_SCALE` (kept local so this module stays `std`-only).
pub const INCOME_PPM_SCALE: i64 = 1_000_000;

/// Apply an `IncomeMult` (parts-per-million; `INCOME_PPM_SCALE` = 1.0×) to a non-negative
/// credit amount, truncating toward zero — matching gamemd's single `ftol` per deposit
/// call. The `i64` intermediate avoids overflow; the result saturates into `i32`. With
/// `income_ppm == INCOME_PPM_SCALE` this is the identity (so stock YR, where every country
/// is 1.0, is hash-neutral).
pub fn apply_income_mult(amount: i32, income_ppm: i64) -> i32 {
    let scaled = (amount as i64).saturating_mul(income_ppm) / INCOME_PPM_SCALE;
    scaled.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// Per-slot OrePurifier BONUS credits — gamemd's second `Add_Tiberium_Credits` call:
/// `trunc(base_value × IncomeMult × purifier_count × (bonus_pct/100))` as ONE i64
/// truncation. IncomeMult AND the `×count×bonus_pct/100` are folded inside a single floor
/// (matching gamemd's one `ftol`); a separate `/100` truncation first would drift ±1 when
/// `IncomeMult != 1.0`. `base_value` is the slot's total ore/gem credit value (Σ bale
/// values). Returns 0 for ≤0 purifiers.
pub fn purifier_bonus_credits(
    base_value: i32,
    purifier_count: i32,
    bonus_pct: i32,
    income_ppm: i64,
) -> i32 {
    if purifier_count <= 0 {
        return 0;
    }
    let v = (base_value as i64) * (purifier_count as i64) * (bonus_pct as i64) * income_ppm
        / (100 * INCOME_PPM_SCALE);
    v.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// Per-slot OrePurifier BONUS HarvestedCredits stat: `trunc(purifier_count × 0.25 × bales
/// × 5.0)` as ONE i64 expr (`bonus_pct/100 == 0.25`, the `×5` baked in — NOT
/// floor-bonus-bales-then-×5). Statistics only; 0 for ≤0 purifiers.
pub fn purifier_bonus_harvested(bales: i32, purifier_count: i32, bonus_pct: i32) -> i32 {
    if purifier_count <= 0 {
        return 0;
    }
    let v = (bales as i64) * (purifier_count as i64) * (bonus_pct as i64) * 5 / 100;
    v.clamp(i32::MIN as i64, i32::MAX as i64) as i32
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

    // ===== P7 — income wiring =====

    /// IncomeMult identity at 1.0× (stock): apply_income_mult is the identity, so stock YR
    /// (every country 1.0) is hash-neutral.
    #[test]
    fn apply_income_mult_identity_at_one() {
        assert_eq!(apply_income_mult(123, INCOME_PPM_SCALE), 123);
        assert_eq!(apply_income_mult(0, INCOME_PPM_SCALE), 0);
    }

    /// IncomeMult truncates toward zero (one ftol), matching gamemd.
    #[test]
    fn apply_income_mult_truncates_toward_zero() {
        assert_eq!(apply_income_mult(25, 1_200_000), 30); // 30.0
        assert_eq!(apply_income_mult(7, 1_200_000), 8); // trunc(8.4)
        assert_eq!(apply_income_mult(1000, 1_200_000), 1200);
    }

    /// The BLOCKER parity case: the purifier bonus is ONE truncation folding IncomeMult
    /// inside. gem slot_value 50, 3 purifiers @25%, IncomeMult 1.2 -> 45 (gamemd), NOT 44
    /// (the double-truncation `trunc(1.2 × trunc(50×3×25/100=37)) = 44` the design's first
    /// draft produced).
    #[test]
    fn purifier_bonus_credits_single_truncation_not_double() {
        assert_eq!(purifier_bonus_credits(50, 3, 25, 1_200_000), 45);
        // Sanity: the buggy double-trunc would have been 44.
        let double_trunc = apply_income_mult((50 * 3 * 25) / 100, 1_200_000);
        assert_eq!(double_trunc, 44, "documents the bug this guards against");
        assert_ne!(purifier_bonus_credits(50, 3, 25, 1_200_000), double_trunc);
    }

    /// At IncomeMult 1.0 the bonus equals the legacy `slot_value×count×pct/100` (so stock
    /// is hash-neutral), and 0 purifiers -> 0.
    #[test]
    fn purifier_bonus_credits_stock_and_zero() {
        assert_eq!(purifier_bonus_credits(100, 2, 25, INCOME_PPM_SCALE), 50);
        assert_eq!(purifier_bonus_credits(100, 0, 25, INCOME_PPM_SCALE), 0);
        assert_eq!(purifier_bonus_credits(100, -1, 25, 1_200_000), 0);
    }

    /// The bonus HarvestedCredits stat is trunc(count × 0.25 × bales × 5) in one step:
    /// count 1, 1 bale -> trunc(1.25) = 1 (NOT floor(0.25)×5 = 0).
    #[test]
    fn purifier_bonus_harvested_single_truncation() {
        assert_eq!(purifier_bonus_harvested(1, 1, 25), 1);
        assert_eq!(purifier_bonus_harvested(4, 1, 25), 5); // trunc(0.25×4×5=5.0)
        assert_eq!(purifier_bonus_harvested(10, 2, 25), 25); // 10×2×25×5/100
        assert_eq!(purifier_bonus_harvested(10, 0, 25), 0);
    }

    /// add_harvested_raw adds a pre-computed figure without the ×5 (used for the bonus
    /// stat, which is already the ×5 product); never touches credits.
    #[test]
    fn add_harvested_raw_adds_without_x5() {
        let mut e = Economy::default();
        e.add_harvested_raw(7);
        assert_eq!(e.harvested_credits, 7);
        assert_eq!(e.credits, 0);
    }
}
