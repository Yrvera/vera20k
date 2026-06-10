//! gamemd ApplyWarheadDamage kernel: distance falloff -> Verses -> MaxDamage cap.
//! Pure. The ONE copy that both the AoE per-target loop and the direct-hit path
//! call after cutover (folds the inline AoE/direct-hit formulas).
//!
//! Verified against gamemd.exe (ApplyWarheadDamage; three Math__ftol calls).
//! Corrections from the 2026-06-04 adversarial pass are noted inline.

use super::ArmorClass;

/// Leptons per cell INSIDE the kernel's CellSpread->lepton conversion.
/// Bit-read `read_memory 0x007e2224 = 0x43800000 = 256.0` (verified this run).
/// The earlier "128" was a hex->decimal mis-conversion; the live AoE collection
/// radius also uses 256, so the kernel and AoE distance units agree.
const KERNEL_LEPTONS_PER_CELL: f64 = 256.0;

/// Truncate toward zero, saturating — the gamemd `Math__ftol` (round-to-zero
/// control word) analog. NOT `util::sim_to_i32` (a `SimFixed` conversion); the
/// kernel operates on `f64`, and `f64 as i32` is the unambiguous
/// truncate-toward-zero. Falloff can floor at 0 and healing is negative, so the
/// toward-zero direction is load-bearing.
#[inline]
fn ftol(v: f64) -> i32 {
    v as i32
}

/// gamemd ApplyWarheadDamage. Pure. Reproduces the double-ftol contract
/// `ftol( ftol(lerp) x Verses )` plus the CellSpread->lepton ftol.
///
/// `cell_spread` and `percent_at_max` are the warhead's decoded f64 values
/// (CellSpread in cells; PercentAtMax 0..1, where 1.0 = flat). `verses_f64` is
/// the warhead's full-precision Verses[11] (the single float exception).
/// `distance_leptons` is the impact-to-target distance in the kernel's lepton
/// unit (256 leptons/cell). `scenario_no_damage` = ScenarioFlags & 0x20.
/// `max_damage` = the running Rules MaxDamage (stock YR = 10000).
///
/// NB the caller must pass a real (non-null) warhead: gamemd has a third
/// `warhead == NULL -> 0` early-out folded into the same OR as the two below;
/// it is the caller's concern since this kernel takes decoded f64 inputs.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_warhead_damage(
    damage: i32,
    cell_spread: f64,
    percent_at_max: f64,
    verses_f64: &[f64; 11],
    armor: ArmorClass,
    distance_leptons: i32,
    scenario_no_damage: bool,
    max_damage: i32,
) -> i32 {
    // D1 early-outs.
    if damage == 0 || scenario_no_damage {
        return 0;
    }
    // D2 healing: negative bypasses falloff+Verses; armor index >= 8 (concrete,
    // special_1, special_2) cannot heal (verified `CMP EDI,0x8; SETGE; DEC; AND`).
    if damage < 0 {
        return if armor.0 >= 8 { 0 } else { damage };
    }

    // D3 distance falloff. cs_leptons = ftol(CellSpread * 256.0) (interior ftol #1).
    let cs_leptons: i32 = ftol(cell_spread * KERNEL_LEPTONS_PER_CELL);
    let damage_f = damage as f64;
    // Branch guard: damage*PAM != damage (PAM==1.0 => flat) AND cs_leptons != 0.
    let damage_pam = damage_f * percent_at_max;
    let falloff: i32 = if damage_pam != damage_f && cs_leptons != 0 {
        // gamemd grouping: D*PAM + (D - D*PAM) * (csL - dist) / csL.
        // (FIMUL by the integer (csL - dist), FIDIV by the integer csL.)
        let lerped = damage_pam
            + (damage_f - damage_pam) * (cs_leptons - distance_leptons) as f64 / cs_leptons as f64;
        ftol(lerped) // interior ftol #2
    } else {
        damage
    };
    let falloff = falloff.max(0); // zero-crossing floor (verified mask-to-0)

    // D4 Verses multiply (the single f64 multiply) + interior ftol #3.
    let scaled: i32 = ftol(falloff as f64 * verses_f64[armor.0 as usize]);

    // D6 MaxDamage cap (signed, inclusive-on-equal). Only strictly-greater is cut.
    scaled.min(max_damage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::combat::damage::ArmorClass;

    /// Stock YR running MaxDamage (`ini/rulesmd.ini` MaxDamage=10000, overriding
    /// the legacy 1000). The cap field is `[Rules+0x16C8]`.
    const MAXD: i32 = 10000;

    fn verses(v: f64) -> [f64; 11] {
        let mut t = [1.0; 11];
        // index 5 = heavy; set the value under test, leave others 1.0.
        t[5] = v;
        t
    }

    #[test]
    fn kernel_matches_worked_example() {
        // 100 dmg, Verses 0.5 (Heavy), CellSpread 1.0, PAM 0.25, dist 128 leptons.
        // cs_leptons = ftol(1.0*256) = 256; t = (256-128)/256 = 0.5;
        // lerped = 0.25*100 + 0.75*100*0.5 = 62.5 => ftol = 62;
        // scaled = ftol(62*0.5) = ftol(31) = 31.
        // (Kernel = AoE = 256 leptons/cell, so Q1 is resolved and this is no
        //  longer #[ignore]'d; value is 31, NOT the mis-converted-128 "12".)
        let d = apply_warhead_damage(100, 1.0, 0.25, &verses(0.5), ArmorClass(5), 128, false, MAXD);
        assert_eq!(d, 31);
    }

    #[test]
    fn kernel_double_ftol_order() {
        // 99 dmg, PAM 0.5, dist 128 (cs_leptons=256 => t=0.5).
        // lerped = 0.5*99 + 0.5*99*0.5 = 74.25 => ftol #2 = 74;
        // scaled = ftol(74 * 0.5) = ftol(37) = 37. Exercises both interior ftols.
        let d = apply_warhead_damage(99, 1.0, 0.5, &verses(0.5), ArmorClass(5), 128, false, MAXD);
        assert_eq!(d, 37);
    }

    #[test]
    fn kernel_healing_blocked_special_armor() {
        // armor 9 (special_1) cannot heal; armor 5 heals by the full negative.
        let nine = apply_warhead_damage(-40, 0.0, 1.0, &[1.0; 11], ArmorClass(9), 0, false, MAXD);
        let eight = apply_warhead_damage(-40, 0.0, 1.0, &[1.0; 11], ArmorClass(8), 0, false, MAXD);
        let five = apply_warhead_damage(-40, 0.0, 1.0, &[1.0; 11], ArmorClass(5), 0, false, MAXD);
        assert_eq!(nine, 0);
        assert_eq!(eight, 0); // index 8 = concrete is ALSO blocked (>= 8)
        assert_eq!(five, -40);
    }

    #[test]
    fn kernel_pam_one_is_flat() {
        // PAM==1.0 => branch guard false => flat damage at any distance.
        let near = apply_warhead_damage(100, 5.0, 1.0, &verses(1.0), ArmorClass(5), 0, false, MAXD);
        let far = apply_warhead_damage(100, 5.0, 1.0, &verses(1.0), ArmorClass(5), 600, false, MAXD);
        assert_eq!(near, 100);
        assert_eq!(far, 100);
    }

    #[test]
    fn kernel_maxdamage_cap() {
        // Verses 2.0 x large flat base => clamps to 10000.
        // base 8000, flat => ftol(8000*2.0)=16000 => min(16000,10000)=10000.
        let d = apply_warhead_damage(8000, 0.0, 1.0, &verses(2.0), ArmorClass(5), 0, false, MAXD);
        assert_eq!(d, 10000);
    }

    #[test]
    fn kernel_maxdamage_cap_inclusive_on_equal() {
        // scaled == cap is kept (inclusive); only strictly-greater is reduced.
        let d = apply_warhead_damage(5000, 0.0, 1.0, &verses(2.0), ArmorClass(5), 0, false, MAXD);
        assert_eq!(d, 10000); // 5000*2 == 10000, kept
    }

    #[test]
    fn kernel_scenario_no_damage_zero() {
        let d = apply_warhead_damage(100, 0.0, 1.0, &verses(1.0), ArmorClass(5), 0, true, MAXD);
        assert_eq!(d, 0);
    }
}
