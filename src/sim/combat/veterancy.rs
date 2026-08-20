//! Veterancy accumulation and rank crossing.
//!
//! gamemd keeps a single running `float` per object and derives every rank from
//! it. `TechnoClass::Record_The_Kill @ 0x00702D40` computes the award and hands
//! it to the accumulator `VeterancyClass::Add @ 0x0074FF50`; the readers
//! (`IsRookie @ 0x0074FFF0`, `IsVeteran @ 0x0074FF90`, `IsElite @ 0x00750010`)
//! only sample it.
//!
//! Dependencies: `rules` for the two `[General]` constants, `util::native_x87`
//! for the native float substrate. No `sim` state is stored as a float — the
//! accumulator lives as its `f32` bit pattern, so hashing, snapshots and replay
//! stay integer-exact.

use crate::util::native_x87::NativeF32Bits;

/// Rank ids, in the order the native tests run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VeterancyRank {
    Rookie,
    Veteran,
    Elite,
}

/// `Veterancy(u16)` value for a rookie.
pub const RANK_ROOKIE_U16: u16 = 0;
/// `Veterancy(u16)` value for a veteran — matches `VETERAN_VETERANCY`.
pub const RANK_VETERAN_U16: u16 = 100;
/// `Veterancy(u16)` value for an elite — matches `ELITE_VETERANCY`.
pub const RANK_ELITE_U16: u16 = 200;

/// `VeterancyClass::IsVeteran @ 0x0074FF90` compares against 1.0f.
const VETERAN_THRESHOLD_BITS: u32 = 0x3F80_0000;
/// `VeterancyClass::IsElite @ 0x00750010` compares against 2.0f.
const ELITE_THRESHOLD_BITS: u32 = 0x4000_0000;

/// Sample the rank from a raw accumulator.
///
/// gamemd-derived: elite is tested before veteran, and both are `>=`
/// comparisons on the float — a value of exactly 1.0 is already a veteran.
pub fn rank_of(raw: NativeF32Bits) -> VeterancyRank {
    let value = f32::from_bits(raw.bits());
    if value >= f32::from_bits(ELITE_THRESHOLD_BITS) {
        VeterancyRank::Elite
    } else if value >= f32::from_bits(VETERAN_THRESHOLD_BITS) {
        VeterancyRank::Veteran
    } else {
        VeterancyRank::Rookie
    }
}

/// Project the raw accumulator onto the `Veterancy(u16)` rank every existing
/// reader consumes.
pub fn rank_u16(raw: NativeF32Bits) -> u16 {
    match rank_of(raw) {
        VeterancyRank::Rookie => RANK_ROOKIE_U16,
        VeterancyRank::Veteran => RANK_VETERAN_U16,
        VeterancyRank::Elite => RANK_ELITE_U16,
    }
}

/// The raw accumulator a scenario-authored rank starts at.
///
/// A map or scenario can place an already-veteran or already-elite object;
/// native seeds the float directly (`VeterancyStruct::SetVeteran @ 0x00750090`
/// writes 1.0f, `SetElite @ 0x007500B0` writes 2.0f), so the projection and the
/// accumulator agree from the first tick.
pub fn raw_for_rank(rank_u16: u16) -> NativeF32Bits {
    if rank_u16 >= RANK_ELITE_U16 {
        NativeF32Bits::from_bits(ELITE_THRESHOLD_BITS)
    } else if rank_u16 >= RANK_VETERAN_U16 {
        NativeF32Bits::from_bits(VETERAN_THRESHOLD_BITS)
    } else {
        NativeF32Bits::POSITIVE_ZERO
    }
}

/// The award a kill is worth, before the recipient's own cost divides it.
///
/// gamemd-derived: `TechnoClass::Record_The_Kill @ 0x00702D40` takes the
/// victim's cost, zeroes it when the killer is an ally of the victim, and
/// otherwise doubles it for a veteran victim or triples it for an elite one.
/// The ally test runs FIRST, which is observationally identical to running it
/// last because `0 * 2 == 0 * 3 == 0`.
pub fn kill_award_points(
    victim_cost: i32,
    victim_rank: VeterancyRank,
    killer_is_ally: bool,
) -> i32 {
    if killer_is_ally || victim_cost <= 0 {
        return 0;
    }
    match victim_rank {
        VeterancyRank::Rookie => victim_cost,
        VeterancyRank::Veteran => victim_cost.saturating_mul(2),
        VeterancyRank::Elite => victim_cost.saturating_mul(3),
    }
}

/// `VeterancyClass::Add @ 0x0074FF50`, literally.
///
/// ```text
/// vet = f32( points / (cost * Rules->VeteranRatio) + vet )
/// if (unrounded sum >= Rules->VeteranCap) vet = f32(VeteranCap)
/// ```
///
/// The clamp compares the UNROUNDED sum, before the store narrows it to `f32` —
/// that ordering is why the accumulate and the compare cannot be folded.
///
/// This is the project's documented native-float substrate rather than
/// `SimFixed` deliberately: the native state is a running `f32` that carries its
/// own rounding forward, so an exact-rational accumulator diverges from it in
/// general. Reproducing the float exactly is the only form that matches; the
/// value is stored as bits, so no float reaches sim state.
///
/// RESIDUAL (GSI-08.12) — the x87 precision control word is UNCHECKED. Under
/// the MSVC CRT default of 53-bit precision this is bit-exact; under 64-bit
/// precision the two can differ by one ulp of `f32`, and only when the
/// unrounded sum sits within half an ulp of a rounding tie. Neither stock
/// promotion boundary is near a tie (see the tests), so no stock kill count
/// moves either way.
pub fn accumulate(
    raw: NativeF32Bits,
    recipient_cost: i32,
    points: i32,
    veteran_ratio: f64,
    veteran_cap: f64,
) -> NativeF32Bits {
    if points <= 0 {
        return raw;
    }
    // A zero-cost recipient is NOT an early return in native: the divide yields
    // +INF, the compare sends it to the clamp, and the object stores
    // `VeteranCap` — instant elite on its first kill. A negative cost yields
    // -INF, which stores as a rookie value. Reproduce both rather than guarding.
    if recipient_cost == 0 {
        return NativeF32Bits::from_bits((veteran_cap as f32).to_bits());
    }
    // `FDIV`/`FADD` at x87 working precision, then one `FCOMP` against the cap
    // on the UNROUNDED sum, then a single `FSTP float` store. `X87Chop53` is
    // deliberately NOT used here: it models the truncating mode gamemd sets for
    // `ftol`, and this store rounds to nearest-even, which is one ulp of `f32`
    // apart at every step.
    let delta = f64::from(points) / (f64::from(recipient_cost) * veteran_ratio);
    let sum = delta + f64::from(f32::from_bits(raw.bits()));
    let clamped = if sum >= veteran_cap { veteran_cap } else { sum };
    NativeF32Bits::from_bits((clamped as f32).to_bits())
}

/// Award one kill's experience to its killer, if the killer can hold it.
///
/// gamemd-derived: the recipient arm of `TechnoClass::Record_The_Kill @
/// 0x00702D40`. `cost` is the RECIPIENT's own cost — an expensive unit needs a
/// proportionally larger haul to promote — and the award is the victim's cost
/// after the ally gate and the victim's own rank multiplier.
///
/// RESIDUAL (GSI-08.12) — only the killer itself receives experience here.
/// Native redirects the award through three earlier paths: a garrisoned
/// occupant, a spawner's owner and a transport's owner all take the experience
/// their platform earned, and the platform itself gets nothing. Those fields'
/// identities are UNCHECKED. Trigger: any kill by a garrisoned infantryman, an
/// Aircraft Carrier's Hornets or an IFV. Player effect: the occupants and
/// spawns never promote, and the platform promotes where retail does not.
/// Frequency: routine in ordinary play wherever garrisons and carriers appear.
pub fn award_kill(
    killer: &mut crate::sim::game_entity::GameEntity,
    killer_cost: i32,
    points: i32,
    trainable: bool,
    veteran_ratio: f64,
    veteran_cap: f64,
) {
    if !trainable || points <= 0 || killer_cost <= 0 {
        return;
    }
    killer.veterancy_raw = accumulate(
        killer.veterancy_raw,
        killer_cost,
        points,
        veteran_ratio,
        veteran_cap,
    );
    killer.veterancy = rank_u16(killer.veterancy_raw);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stock `[General] VeteranRatio=` and `VeteranCap=`.
    const RATIO: f64 = 3.0;
    const CAP: f64 = 2.0;

    fn run(cost: i32, points: i32, kills: usize) -> Vec<NativeF32Bits> {
        let mut raw = NativeF32Bits::POSITIVE_ZERO;
        (0..kills)
            .map(|_| {
                raw = accumulate(raw, cost, points, RATIO, CAP);
                raw
            })
            .collect()
    }

    /// A Grizzly (Cost=700) killing rookie Rhinos (Cost=900) earns 3/7 per kill:
    /// veteran on kill 3, elite on kill 5. Both crossings sit far from a
    /// rounding tie, so the kill counts do not depend on the precision-control
    /// question recorded on `accumulate`.
    #[test]
    fn gsi_08_12_grizzly_promotes_on_the_third_and_fifth_rhino() {
        let steps = run(700, 900, 5);
        let ranks: Vec<u16> = steps.iter().map(|raw| rank_u16(*raw)).collect();
        assert_eq!(ranks, vec![0, 0, 100, 100, 200]);
    }

    /// A GI killing GIs is the knife edge: the delta is exactly 1/3, so the
    /// running `f32` reaches 1.0000000199 on kill 3 rather than exactly 1.0.
    #[test]
    fn gsi_08_12_gi_promotes_on_the_third_and_sixth_gi() {
        let steps = run(200, 200, 6);
        // The `f32` running sum reaches 1.0000000199 on kill 3, which stores as
        // exactly 1.0 — still a promotion.
        assert_eq!(steps[0].bits(), 0x3EAA_AAAB);
        assert_eq!(steps[2].bits(), 0x3F80_0000);
        let ranks: Vec<u16> = steps.iter().map(|raw| rank_u16(*raw)).collect();
        assert_eq!(ranks, vec![0, 0, 100, 100, 100, 200]);
    }

    /// The victim's own rank multiplies the award before it is divided.
    #[test]
    fn gsi_08_12_victim_rank_multiplies_the_award() {
        assert_eq!(kill_award_points(900, VeterancyRank::Rookie, false), 900);
        assert_eq!(kill_award_points(900, VeterancyRank::Veteran, false), 1800);
        assert_eq!(kill_award_points(900, VeterancyRank::Elite, false), 2700);
        assert_eq!(kill_award_points(900, VeterancyRank::Elite, true), 0);
    }

    /// `VeteranCap=2` is exactly the elite threshold, so elite is terminal in
    /// stock: the accumulator saturates and never climbs past it.
    #[test]
    fn gsi_08_12_accumulator_saturates_at_the_veteran_cap() {
        let steps = run(700, 900, 20);
        let last = *steps.last().expect("20 kills");
        assert_eq!(last.bits(), ELITE_THRESHOLD_BITS);
        assert_eq!(rank_u16(last), 200);
    }

    /// An award of zero — an allied kill, or a victim with no cost — leaves the
    /// accumulator untouched. A zero-cost RECIPIENT is a different case: native
    /// divides by zero, gets `+INF`, and the clamp stores `VeteranCap`.
    #[test]
    fn gsi_08_12_zero_award_does_not_move_the_accumulator() {
        let start = accumulate(NativeF32Bits::POSITIVE_ZERO, 700, 900, RATIO, CAP);
        assert_eq!(accumulate(start, 700, 0, RATIO, CAP).bits(), start.bits());
    }

    #[test]
    fn gsi_08_12_zero_cost_recipient_saturates_to_the_cap() {
        let out = accumulate(NativeF32Bits::POSITIVE_ZERO, 0, 900, RATIO, CAP);
        assert_eq!(out.bits(), ELITE_THRESHOLD_BITS);
        assert_eq!(rank_u16(out), 200);
    }
}
