//! House color definitions — runtime per-`[Colors]`-entry team-color ramps.
//!
//! RA2 reserves palette indices 16–31 for "house colors" — 16 shades swapped per
//! player to distinguish units visually (Allied blue, Soviet red, etc.). Each
//! band runs from brightest (index 0) to darkest (index 15); when rendering a
//! unit the base palette's indices 16–31 are replaced with the owning player's
//! band before pixel conversion.
//!
//! Bands are built at load from the rules `[Colors]` H,S,V list via gamemd's
//! fixed-hue trig Saturation/Value sweep ([`build_scheme_ramp`]) and held in a
//! runtime [`HouseColorRamps`] table on the `RuleSet`. A `HouseColorIndex` is a
//! `[Colors]` entry index into that table.
//!
//! ## Dependency rules
//! - Part of rules/ — depends on assets/pal_file (Color) + rules/color_scheme.

use crate::assets::pal_file::Color;
use crate::rules::color_scheme::{ColorSchemeEntry, hsv_to_rgb};

/// A `[Colors]` entry index — selects a player's team-color band.
///
/// Stored as u8 for cheap hashing in atlas keys (HashMap lookups every frame) and
/// reused as the GPU ramp-texture row key (`row = index + 1`). Default (0) is the
/// first `[Colors]` entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct HouseColorIndex(pub u8);

/// Sentinel value meaning "do not apply house color remap — use raw palette."
/// Used for Neutral, Special, Civilian buildings that have no player color.
pub const NO_REMAP: HouseColorIndex = HouseColorIndex(255);

/// Returns true if the given owner is a non-player house that should NOT receive
/// player color remapping. These buildings render with their original palette.
pub fn is_non_player_house(owner: &str) -> bool {
    let up = owner.to_ascii_uppercase();
    matches!(
        up.as_str(),
        "NEUTRAL" | "SPECIAL" | "CIVILIAN" | "GOODGUY" | "BADGUY"
    )
}

/// Number of shades per house color band (matches palette indices 16–31).
const RAMP_SIZE: usize = 16;

/// gamemd's per-scheme 16-shade team band (palette indices 16..31): fixed hue H; V rides a cosine
/// 50°→90°, S rides a sine 20°→90° (shade 0 sine = π/16); each (modS, modV) goes through the
/// 6-sextant integer HSV→RGB. Shade 0 is the brightest (the radar/UI/target-line color). `f64` trig
/// is acceptable here (rules/render, not lockstep sim); bit-exact values come from the emulation
/// reference (gamemd uses table/x87 trig, so an `f64` port can drift ±1 per channel).
pub fn build_scheme_ramp(hsv: [u8; 3]) -> [Color; RAMP_SIZE] {
    use std::f64::consts::PI;
    let h = hsv[0];
    let s = hsv[1] as f64;
    let v = hsv[2] as f64;
    let cos_base = 50.0_f64.to_radians();
    let cos_step = (40.0_f64 / 15.0).to_radians();
    let sin_base = 20.0_f64.to_radians();
    let sin_step = (70.0_f64 / 15.0).to_radians();
    let mut ramp = [Color { r: 0, g: 0, b: 0, a: 255 }; RAMP_SIZE];
    for (i, slot) in ramp.iter_mut().enumerate() {
        let cos_angle = cos_base + (i as f64) * cos_step;
        let sin_angle = if i == 0 { PI / 16.0 } else { sin_base + (i as f64) * sin_step };
        // gamemd ftol = truncate toward zero, then clamp into a palette byte.
        let mod_v = (cos_angle.cos() * v).trunc().clamp(0.0, 255.0) as u8;
        let mod_s = (sin_angle.sin() * s).trunc().clamp(0.0, 255.0) as u8;
        let [r, g, b] = hsv_to_rgb([h, mod_s, mod_v]);
        *slot = Color { r, g, b, a: 255 };
    }
    ramp
}

/// `[Colors]` entry index used when a house has no resolvable color. gamemd `InitColor` forces a
/// negative ColorSchemeIndex to 5 (runtime scheme 5 → `[Colors]` entry 2 = LightGrey / white-ish).
pub const DEFAULT_SCHEME_ENTRY: usize = 2;

/// Flat fallback ramp used only when the `[Colors]` list is empty (rules not yet loaded); a real
/// skirmish always has a populated scheme list.
static FALLBACK_RAMP: [Color; RAMP_SIZE] = [Color { r: 180, g: 180, b: 180, a: 255 }; RAMP_SIZE];

/// Runtime per-`[Colors]`-entry house-color ramp table. Index = `[Colors]` entry index =
/// `HouseColorIndex.0`. Built once at load from the parsed `[Colors]` schemes and held on the
/// `RuleSet`. `Default` (empty) is used only when rules are unavailable (headless tests, missing
/// assets); `ramp()` then yields the flat fallback.
#[derive(Debug, Default)]
pub struct HouseColorRamps {
    ramps: Vec<[Color; RAMP_SIZE]>,
}

impl HouseColorRamps {
    /// Build one trig ramp per `[Colors]` scheme, in declaration order.
    pub fn from_schemes(schemes: &[ColorSchemeEntry]) -> Self {
        Self {
            ramps: schemes.iter().map(|s| build_scheme_ramp(s.hsv)).collect(),
        }
    }

    /// Ramp for a house color index. `NO_REMAP` or an out-of-range index falls back to the default
    /// scheme (or a flat ramp if the scheme list is empty).
    pub fn ramp(&self, index: HouseColorIndex) -> &[Color; RAMP_SIZE] {
        if index != NO_REMAP
            && let Some(r) = self.ramps.get(index.0 as usize)
        {
            return r;
        }
        self.ramps.get(DEFAULT_SCHEME_ENTRY).unwrap_or(&FALLBACK_RAMP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_scheme_ramp_16_opaque_brightest_first() {
        let r = build_scheme_ramp([153, 214, 212]); // DarkBlue HSV
        assert_eq!(r.len(), 16);
        assert!(r.iter().all(|c| c.a == 255));
        let lum = |c: &Color| c.r as u32 + c.g as u32 + c.b as u32;
        assert!(lum(&r[0]) > lum(&r[15]), "shade 0 must be brighter than shade 15");
        assert!(r[0].b >= r[0].r && r[0].b >= r[0].g, "blue hue preserved: {:?}", r[0]);
    }

    #[test]
    fn house_color_ramps_indexes_by_entry_and_falls_back() {
        let mk = |name: &str, hsv: [u8; 3]| ColorSchemeEntry {
            name: name.into(),
            hsv,
        };
        let schemes = vec![
            mk("A", [0, 230, 255]),
            mk("B", [153, 214, 212]),
            mk("C", [0, 0, 240]),
        ];
        let table = HouseColorRamps::from_schemes(&schemes);
        assert_eq!(table.ramp(HouseColorIndex(1)), &build_scheme_ramp([153, 214, 212]));
        // NO_REMAP and out-of-range fall back to DEFAULT_SCHEME_ENTRY (2).
        assert_eq!(table.ramp(NO_REMAP), table.ramp(HouseColorIndex(2)));
        assert_eq!(table.ramp(HouseColorIndex(99)), table.ramp(HouseColorIndex(2)));
    }
}
