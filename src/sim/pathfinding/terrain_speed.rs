//! Runtime per-cell speed modifiers applied during movement execution.
//!
//! The original engine applies terrain speed as a runtime modifier (not an
//! A* cost weight). Two multiplicative factors are combined each tick:
//!
//! 1. **Terrain type** — from rules.ini land-type sections ([Clear] Foot=100%, etc.)
//! 2. **Slope** — vehicles moving up/down a grade, chosen by SpeedType (Track vs
//!    other) and travel direction. Vanilla: uphill ×1.0 (no change), downhill
//!    ×1.2 (faster).
//!
//! The original engine has no crowd/density speed term: congestion is resolved by
//! blocking and re-pathing, never by scaling a mover's speed. A former synthetic
//! crowd-jam factor (radius-2 occupancy scan → 0.7×) was removed — it had no
//! source in the original engine and no INI key driving it (invented behavior).
//!
//! Depends on: `ResolvedTerrainGrid` (cell height + land type),
//! `SpeedCostProfile` (INI-parsed terrain percentages).

use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::rules::locomotor_type::{LocomotorKind, SpeedType};
use crate::util::fixed_math::{SIM_HALF, SIM_ONE, SimFixed};

// --- Constants from the original engine ---

/// Original engine boosts 0% terrain speed to 50% so passable terrain never
/// fully immobilizes a unit. Applied when the INI percentage is exactly 0.
const TERRAIN_SPEED_MIN: SimFixed = SIM_HALF;

/// Original engine clamps terrain speed multipliers above 1.0 to exactly 1.0.
const TERRAIN_SPEED_MAX: SimFixed = SIM_ONE;

/// Maximum allowed combined speed modifier (downhill on road can exceed 1.0).
const COMBINED_MAX: SimFixed = SimFixed::lit("1.2");

/// Minimum combined speed modifier — prevents near-zero crawl.
const COMBINED_MIN: SimFixed = SimFixed::lit("0.3");

/// Cliff/slope speed coefficients from `[General]` in rules.ini.
///
/// The original engine keeps four: tracked vs wheeled vehicles, each with an
/// uphill and a downhill coefficient, selected by the mover's SpeedType (Track
/// uses the tracked pair, every other SpeedType uses the wheeled pair) and by
/// travel direction. Vanilla values are 1.0 uphill / 1.2 downhill for both.
#[derive(Debug, Clone)]
pub struct TerrainSpeedConfig {
    /// Tracked vehicle moving uphill (`TrackedUphill=`; vanilla 1.0).
    pub tracked_uphill: SimFixed,
    /// Tracked vehicle moving downhill (`TrackedDownhill=`; vanilla 1.2).
    pub tracked_downhill: SimFixed,
    /// Non-tracked (wheeled and other) vehicle moving uphill (`WheeledUphill=`; vanilla 1.0).
    pub wheeled_uphill: SimFixed,
    /// Non-tracked vehicle moving downhill (`WheeledDownhill=`; vanilla 1.2).
    pub wheeled_downhill: SimFixed,
}

impl Default for TerrainSpeedConfig {
    fn default() -> Self {
        // Vanilla rulesmd.ini [General]: 1.0 uphill / 1.2 downhill for both pairs.
        Self {
            tracked_uphill: SIM_ONE,
            tracked_downhill: SimFixed::lit("1.2"),
            wheeled_uphill: SIM_ONE,
            wheeled_downhill: SimFixed::lit("1.2"),
        }
    }
}

impl TerrainSpeedConfig {
    /// Build config from the four parsed `[General]` slope coefficients.
    pub fn from_general(
        tracked_uphill: SimFixed,
        tracked_downhill: SimFixed,
        wheeled_uphill: SimFixed,
        wheeled_downhill: SimFixed,
    ) -> Self {
        Self {
            tracked_uphill,
            tracked_downhill,
            wheeled_uphill,
            wheeled_downhill,
        }
    }
}

/// Compute the combined per-cell speed multiplier for a unit moving between cells.
///
/// Returns a SimFixed in [`COMBINED_MIN`, `COMBINED_MAX`] that should be multiplied
/// with the unit's base speed each movement tick.
pub fn compute_cell_speed_modifier(
    speed_type: SpeedType,
    locomotor_kind: LocomotorKind,
    current_cell: (u16, u16),
    next_cell: (u16, u16),
    terrain: &ResolvedTerrainGrid,
    config: &TerrainSpeedConfig,
) -> SimFixed {
    let terrain_factor = terrain_speed_factor(speed_type, next_cell, terrain);
    let slope_factor = slope_speed_factor(
        speed_type,
        locomotor_kind,
        current_cell,
        next_cell,
        terrain,
        config,
    );

    let combined = terrain_factor * slope_factor;
    combined.clamp(COMBINED_MIN, COMBINED_MAX)
}

/// Factor 1: terrain type speed from INI land-type percentages.
///
/// Looks up the *destination* cell's terrain speed for the unit's SpeedType.
/// Matches original engine: 0% → 50%, >100% → clamped to 100%, missing → 100%.
fn terrain_speed_factor(
    speed_type: SpeedType,
    next_cell: (u16, u16),
    terrain: &ResolvedTerrainGrid,
) -> SimFixed {
    let Some(cell) = terrain.cell(next_cell.0, next_cell.1) else {
        return SIM_ONE;
    };
    let multiplier = cell.speed_costs.speed_multiplier_for(speed_type);
    multiplier.clamp(TERRAIN_SPEED_MIN, TERRAIN_SPEED_MAX)
}

/// Factor 2: slope speed coefficient from the current→next cell grade.
///
/// Only ground movers interact with terrain grade — Hover/Fly/Jumpjet/Rocket
/// float above the surface and ignore slopes. The coefficient is picked by
/// SpeedType and direction in [`slope_factor_for`].
fn slope_speed_factor(
    speed_type: SpeedType,
    locomotor_kind: LocomotorKind,
    current_cell: (u16, u16),
    next_cell: (u16, u16),
    terrain: &ResolvedTerrainGrid,
    config: &TerrainSpeedConfig,
) -> SimFixed {
    // Airborne and hovering locomotors don't interact with terrain grade.
    if !is_slope_affected(locomotor_kind) {
        return SIM_ONE;
    }

    let cur_level = terrain
        .cell(current_cell.0, current_cell.1)
        .map(|c| c.level)
        .unwrap_or(0);
    let next_level = terrain
        .cell(next_cell.0, next_cell.1)
        .map(|c| c.level)
        .unwrap_or(0);

    slope_factor_for(speed_type, cur_level, next_level, config)
}

/// Pick the slope coefficient for a mover stepping from `cur_level` to `next_level`.
///
/// Destination higher than current = uphill; lower = downhill; equal = no change.
/// Track SpeedType uses the tracked pair, every other SpeedType the wheeled pair —
/// matching the original engine's `SpeedType == Track` test (infantry are handled
/// by a separate precomputed-foot mechanism and don't reach this vehicle path).
fn slope_factor_for(
    speed_type: SpeedType,
    cur_level: u8,
    next_level: u8,
    config: &TerrainSpeedConfig,
) -> SimFixed {
    let tracked = speed_type == SpeedType::Track;
    if next_level > cur_level {
        // Uphill.
        if tracked {
            config.tracked_uphill
        } else {
            config.wheeled_uphill
        }
    } else if next_level < cur_level {
        // Downhill.
        if tracked {
            config.tracked_downhill
        } else {
            config.wheeled_downhill
        }
    } else {
        SIM_ONE
    }
}

/// Whether a locomotor type is affected by terrain slope.
/// Ground movers interact with hills; airborne and hovering ones don't.
fn is_slope_affected(kind: LocomotorKind) -> bool {
    matches!(
        kind,
        LocomotorKind::Drive
            | LocomotorKind::Walk
            | LocomotorKind::Mech
            | LocomotorKind::Ship
            | LocomotorKind::Tunnel
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::terrain_rules::SpeedCostProfile;

    #[test]
    fn speed_multiplier_for_normal_terrain() {
        let profile = SpeedCostProfile {
            foot: Some(100),
            track: Some(100),
            ..Default::default()
        };
        assert_eq!(profile.speed_multiplier_for(SpeedType::Foot), SIM_ONE);
    }

    #[test]
    fn speed_multiplier_for_rough_terrain() {
        let profile = SpeedCostProfile {
            track: Some(75),
            ..Default::default()
        };
        let mult = profile.speed_multiplier_for(SpeedType::Track);
        assert_eq!(mult, SimFixed::lit("0.75"));
    }

    #[test]
    fn speed_multiplier_zero_boosted_to_half() {
        let profile = SpeedCostProfile {
            foot: Some(0),
            ..Default::default()
        };
        assert_eq!(profile.speed_multiplier_for(SpeedType::Foot), SIM_HALF);
    }

    #[test]
    fn speed_multiplier_none_defaults_to_one() {
        let profile = SpeedCostProfile::default();
        assert_eq!(profile.speed_multiplier_for(SpeedType::Foot), SIM_ONE);
    }

    /// RC-7 / AT-13: percentages above 100 clamp to full speed (1.0); a sub-100
    /// percentage passes through as its fraction. The original never lets a
    /// terrain speed bonus push a unit faster than its base speed.
    #[test]
    fn speed_multiplier_clamps_at_one() {
        let fast = SpeedCostProfile {
            foot: Some(120),
            ..Default::default()
        };
        assert_eq!(fast.speed_multiplier_for(SpeedType::Foot), SIM_ONE);

        // 80% is below the cap, so it passes through as the fraction 80/100
        // (no clamp). Compare against the same fixed-point division the
        // implementation performs — a `lit("0.8")` literal differs by 1 ULP
        // because 0.8 is not exactly representable in binary fixed-point.
        let slow = SpeedCostProfile {
            foot: Some(80),
            ..Default::default()
        };
        let pass_through = SimFixed::from_num(80u8) / SimFixed::from_num(100u8);
        assert_eq!(slow.speed_multiplier_for(SpeedType::Foot), pass_through);
        assert!(pass_through < SIM_ONE, "80% must stay below the 1.0 cap");
    }

    #[test]
    fn default_config_values() {
        let config = TerrainSpeedConfig::default();
        assert_eq!(config.tracked_uphill, SIM_ONE);
        assert_eq!(config.tracked_downhill, SimFixed::lit("1.2"));
        assert_eq!(config.wheeled_uphill, SIM_ONE);
        assert_eq!(config.wheeled_downhill, SimFixed::lit("1.2"));
    }

    #[test]
    fn slope_uphill_no_change_downhill_boost() {
        let config = TerrainSpeedConfig::default();
        // Uphill (next higher) → 1.0; downhill → 1.2; flat → 1.0. Track and Wheel
        // share vanilla values but exercise both selection arms.
        for st in [SpeedType::Track, SpeedType::Wheel] {
            assert_eq!(
                slope_factor_for(st, 0, 1, &config),
                SIM_ONE,
                "uphill {st:?}"
            );
            let down = slope_factor_for(st, 1, 0, &config);
            assert_eq!(down, SimFixed::lit("1.2"), "downhill {st:?}");
            assert_eq!(slope_factor_for(st, 2, 2, &config), SIM_ONE, "flat {st:?}");
        }
    }

    #[test]
    fn slope_selects_tracked_vs_wheeled_pair() {
        // Distinct values per pair to prove the SpeedType arm is honoured.
        let config = TerrainSpeedConfig {
            tracked_uphill: SimFixed::lit("0.5"),
            tracked_downhill: SimFixed::lit("1.5"),
            wheeled_uphill: SimFixed::lit("0.7"),
            wheeled_downhill: SimFixed::lit("1.1"),
        };
        assert_eq!(
            slope_factor_for(SpeedType::Track, 0, 1, &config),
            SimFixed::lit("0.5")
        );
        assert_eq!(
            slope_factor_for(SpeedType::Track, 1, 0, &config),
            SimFixed::lit("1.5")
        );
        // Foot and Wheel both take the wheeled (non-Track) pair.
        assert_eq!(
            slope_factor_for(SpeedType::Wheel, 0, 1, &config),
            SimFixed::lit("0.7")
        );
        assert_eq!(
            slope_factor_for(SpeedType::Foot, 1, 0, &config),
            SimFixed::lit("1.1")
        );
    }
}
