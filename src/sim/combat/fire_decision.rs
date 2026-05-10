//! Per-tick fire-decision outcomes for one attacker.
//!
//! Behavioral subset of gamemd's GetFireError codes (see
//! ra2-rust-game-docs/UNITCLASS_TURRET_TRACKING_AND_FIRE_TIMING_GHIDRA_REPORT.md
//! §4.2). Code 5 (Generic) collapses ~30 binary sub-reasons since they all
//! map to "no fire this tick"; threading sub-reason complexity buys zero
//! observable difference.
//!
//! ## Dependency rules
//! - Part of sim/ — depends only on standard library.
//! - sim/ NEVER depends on render/, ui/, sidebar/, audio/, net/.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FireDecision {
    Fire,
    Cooldown,
    Facing,
    Range,
    NoAmmo,
    CloakedTarget,
    ForceFire,
    Generic,
}

impl FireDecision {
    /// Whether this decision drives gattling-weapon spin-up (gamemd codes
    /// {0, 2, 3, 4} per research doc §4.8). Code 4 is unmapped in our enum;
    /// we approximate with Generic since it covers "rotation/cooldown-related
    /// no-fire" cases.
    pub fn drives_gattling_spinup(self) -> bool {
        matches!(
            self,
            Self::Fire | Self::Facing | Self::Cooldown | Self::Generic
        )
    }

    /// Whether this decision means "fire happens this tick".
    pub fn is_fire(self) -> bool {
        matches!(self, Self::Fire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drives_gattling_spinup_truth_table() {
        assert!(FireDecision::Fire.drives_gattling_spinup());
        assert!(FireDecision::Facing.drives_gattling_spinup());
        assert!(FireDecision::Cooldown.drives_gattling_spinup());
        assert!(FireDecision::Generic.drives_gattling_spinup());

        assert!(!FireDecision::Range.drives_gattling_spinup());
        assert!(!FireDecision::NoAmmo.drives_gattling_spinup());
        assert!(!FireDecision::CloakedTarget.drives_gattling_spinup());
        assert!(!FireDecision::ForceFire.drives_gattling_spinup());
    }

    #[test]
    fn is_fire_only_for_fire_variant() {
        assert!(FireDecision::Fire.is_fire());
        assert!(!FireDecision::Facing.is_fire());
        assert!(!FireDecision::ForceFire.is_fire());
        assert!(!FireDecision::Cooldown.is_fire());
    }
}
