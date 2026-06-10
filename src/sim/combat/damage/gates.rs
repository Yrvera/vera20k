//! Ordered receiver immunity gates. Pure over ImmunityInputs. Returns the gate
//! decision; the caller short-circuits on Nullified/MindControlled.
//!
//! Order verified against TechnoClass::ReceiveDamage (0x00701900): the armor
//! divides + min-1 run FIRST (in the caller), then these gates in the order
//! below. TypeImmune -> WarpingOut -> ForceShield -> Bunker -> Radiation ->
//! Psychic -> Poison -> AffectsAllies -> Psychedelic (each short-circuits).

use super::{DamageGate, ImmunityInputs};

/// Evaluate the receiver immunity gates in gamemd's verified order. Each gate
/// short-circuits. Runs AFTER the armor divides + min-1 (see the receiver
/// pipeline), matching the binary's gate placement.
pub(crate) fn evaluate_gates(g: &ImmunityInputs) -> DamageGate {
    // 1. TypeImmune: attacker present + same WhatAmI + same owner.
    if g.attacker_present && g.type_immune {
        return DamageGate::Nullified;
    }
    // 2. WarpingOut.
    if g.warping_out {
        return DamageGate::Nullified;
    }
    // 3. ForceShield / invuln (IronCurtain/ForceShield).
    if g.force_shield {
        return DamageGate::Nullified;
    }
    // 4. Bunker/garrison-link block (target bunkered AND warhead lacks
    //    PenetratesBunker). NOT a wall check.
    if g.bunker_blocked {
        return DamageGate::Nullified;
    }
    // 5. Radiation immune.
    if g.radiation_immune {
        return DamageGate::Nullified;
    }
    // 6. PsychicDamage immune.
    if g.psychic_immune {
        return DamageGate::Nullified;
    }
    // 7. Poison immune.
    if g.poison_immune {
        return DamageGate::Nullified;
    }
    // 8. !AffectsAllies && attacker present && allied (AffectsAllies default true).
    if !g.affects_allies && g.attacker_present && g.is_allied {
        return DamageGate::Nullified;
    }
    // 9. Psychedelic/MindControl: allied -> 0; psionics-immune -> 0; building ->
    //    0; else MindControlled (0 HP, return-code-1 marker).
    if g.psychedelic {
        if g.is_allied || g.psionics_immune || g.target_is_building {
            return DamageGate::Nullified;
        }
        return DamageGate::MindControlled;
    }
    DamageGate::Pass
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::combat::damage::ImmunityInputs;

    fn base() -> ImmunityInputs {
        ImmunityInputs { affects_allies: true, ..Default::default() }
    }

    #[test]
    fn type_immune_same_owner_zeroes() {
        let g = ImmunityInputs { attacker_present: true, type_immune: true, ..base() };
        assert_eq!(evaluate_gates(&g), DamageGate::Nullified);
    }

    #[test]
    fn type_immune_needs_attacker_present() {
        // type_immune set but no attacker -> not gated by rule 1.
        let g = ImmunityInputs { attacker_present: false, type_immune: true, ..base() };
        assert_eq!(evaluate_gates(&g), DamageGate::Pass);
    }

    #[test]
    fn affects_allies_default_hits_ally() {
        // AffectsAllies default true: an allied hit still passes.
        let g = ImmunityInputs { attacker_present: true, is_allied: true, ..base() };
        assert_eq!(evaluate_gates(&g), DamageGate::Pass);
    }

    #[test]
    fn affects_allies_off_blocks_ally() {
        let g = ImmunityInputs {
            attacker_present: true,
            is_allied: true,
            affects_allies: false,
            ..base()
        };
        assert_eq!(evaluate_gates(&g), DamageGate::Nullified);
    }

    #[test]
    fn force_shield_zeroes() {
        assert_eq!(
            evaluate_gates(&ImmunityInputs { force_shield: true, ..base() }),
            DamageGate::Nullified
        );
    }

    #[test]
    fn bunker_blocked_zeroes() {
        assert_eq!(
            evaluate_gates(&ImmunityInputs { bunker_blocked: true, ..base() }),
            DamageGate::Nullified
        );
    }

    #[test]
    fn radiation_immune_zeroes() {
        assert_eq!(
            evaluate_gates(&ImmunityInputs { radiation_immune: true, ..base() }),
            DamageGate::Nullified
        );
    }

    #[test]
    fn poison_immune_zeroes() {
        assert_eq!(
            evaluate_gates(&ImmunityInputs { poison_immune: true, ..base() }),
            DamageGate::Nullified
        );
    }

    #[test]
    fn psionic_immune_zeroes() {
        assert_eq!(
            evaluate_gates(&ImmunityInputs { psychic_immune: true, ..base() }),
            DamageGate::Nullified
        );
    }

    #[test]
    fn mindcontrol_warhead_applies_marker() {
        let g = ImmunityInputs { attacker_present: true, psychedelic: true, ..base() };
        assert_eq!(evaluate_gates(&g), DamageGate::MindControlled);
    }

    #[test]
    fn mindcontrol_on_building_nullifies() {
        let g = ImmunityInputs {
            attacker_present: true,
            psychedelic: true,
            target_is_building: true,
            ..base()
        };
        assert_eq!(evaluate_gates(&g), DamageGate::Nullified);
    }

    #[test]
    fn mindcontrol_on_psionics_immune_nullifies() {
        let g = ImmunityInputs {
            attacker_present: true,
            psychedelic: true,
            psionics_immune: true,
            ..base()
        };
        assert_eq!(evaluate_gates(&g), DamageGate::Nullified);
    }
}
