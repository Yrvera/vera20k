//! House color definitions — palette ramps for player team colors.
//!
//! RA2 reserves palette indices 16–31 for "house colors" — 16 shades that get
//! swapped per player to distinguish units visually. Allied units appear blue,
//! Soviet units appear red, etc.
//!
//! Each color scheme is a 16-entry gradient from lightest (index 0) to darkest
//! (index 15). When rendering a unit, the base palette's indices 16–31 are
//! replaced with the owning player's color scheme before pixel conversion.
//!
//! ## Standard RA2 Schemes
//! Gold (default/neutral), DarkBlue (Allied), DarkRed (Soviet), Green,
//! Orange, Purple, LightBlue, Brown.
//!
//! ## Dependency rules
//! - Part of rules/ — depends only on assets/pal_file (Color type).

use crate::assets::pal_file::Color;
use crate::rules::color_scheme::{ColorSchemeEntry, hsv_to_rgb};

/// Index into the standard color scheme table.
///
/// Stored as u8 for cheap hashing in atlas keys (used in HashMap lookups every frame).
/// Default (0) = Gold, the neutral/unassigned color.
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

/// Number of standard house color schemes.
const SCHEME_COUNT: usize = 9;

/// Number of shades per house color ramp (matches palette indices 16–31).
const RAMP_SIZE: usize = 16;

/// Standard scheme names for lookup. Order matches SCHEMES array indices.
const SCHEME_NAMES: [&str; SCHEME_COUNT] = [
    "gold",
    "darkblue",
    "darkred",
    "green",
    "orange",
    "purple",
    "lightblue",
    "brown",
    "grey",
];

/// Base RGB values for each scheme. Ramps are generated from these.
const SCHEME_BASES: [(u8, u8, u8); SCHEME_COUNT] = [
    (200, 180, 60),  // Gold
    (40, 60, 200),   // DarkBlue
    (200, 40, 40),   // DarkRed
    (40, 180, 40),   // Green
    (220, 140, 20),  // Orange
    (160, 40, 180),  // Purple
    (80, 160, 220),  // LightBlue
    (140, 90, 40),   // Brown
    (140, 140, 130), // Grey — civilian/neutral buildings
];

/// Pre-computed color ramps for all standard schemes.
/// Each scheme has 16 shades from brightest (index 0) to darkest (index 15).
static SCHEMES: [[Color; RAMP_SIZE]; SCHEME_COUNT] = {
    let mut result: [[Color; RAMP_SIZE]; SCHEME_COUNT] = [[Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    }; RAMP_SIZE]; SCHEME_COUNT];

    let mut scheme_idx: usize = 0;
    while scheme_idx < SCHEME_COUNT {
        let (base_r, base_g, base_b) = SCHEME_BASES[scheme_idx];
        result[scheme_idx] = generate_ramp(base_r, base_g, base_b);
        scheme_idx += 1;
    }
    result
};

/// Get the 16-color ramp for a house color index.
///
/// Returns the Gold ramp for out-of-range indices (defensive fallback).
pub fn house_color_ramp(index: HouseColorIndex) -> &'static [Color; RAMP_SIZE] {
    let idx: usize = index.0 as usize;
    if idx < SCHEME_COUNT {
        &SCHEMES[idx]
    } else {
        &SCHEMES[0] // Gold fallback
    }
}

/// Map a color scheme name to its index.
///
/// Case-insensitive lookup. Returns Gold (index 0) for unknown names.
/// Accepts both RA2 names ("DarkBlue") and bare color words ("Blue").
pub fn color_index_for_name(name: &str) -> HouseColorIndex {
    let lower: String = name.to_lowercase();

    // Exact match against standard names.
    for (i, &scheme_name) in SCHEME_NAMES.iter().enumerate() {
        if lower == scheme_name {
            return HouseColorIndex(i as u8);
        }
    }

    // Partial match for common aliases.
    if lower.contains("blue") && lower.contains("light") {
        return HouseColorIndex(6); // LightBlue
    }
    if lower.contains("blue") {
        return HouseColorIndex(1); // DarkBlue
    }
    if lower.contains("red") {
        return HouseColorIndex(2); // DarkRed
    }
    if lower.contains("green") {
        return HouseColorIndex(3); // Green
    }
    if lower.contains("orange") {
        return HouseColorIndex(4); // Orange
    }
    if lower.contains("purple") || lower.contains("magenta") {
        return HouseColorIndex(5); // Purple
    }
    if lower.contains("brown") {
        return HouseColorIndex(7); // Brown
    }
    if lower.contains("grey") || lower.contains("gray") {
        return HouseColorIndex(8); // Grey (civilian/neutral)
    }
    if lower.contains("gold") || lower.contains("yellow") {
        return HouseColorIndex(0); // Gold
    }

    // Default fallback for unknown color names.
    HouseColorIndex(0)
}

/// Generate a 16-shade gradient ramp from a base color.
///
/// Shade 0 is the brightest (base color tinted toward white).
/// Shade 15 is the darkest (base color shaded toward black).
/// This produces a smooth gradient suitable for house color remapping.
const fn generate_ramp(base_r: u8, base_g: u8, base_b: u8) -> [Color; RAMP_SIZE] {
    let mut ramp: [Color; RAMP_SIZE] = [Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    }; RAMP_SIZE];
    let mut i: usize = 0;
    while i < RAMP_SIZE {
        // t ranges from 0.0 (brightest) to 1.0 (darkest).
        // We use integer math to stay const-compatible.
        // Brightness range: 1.4 (lightest) down to 0.3 (darkest).
        // Formula: brightness = 1.4 - (i * 1.1 / 15)
        // In fixed point (x100): brightness_100 = 140 - (i * 110 / 15)
        let brightness_100: u32 = 140 - (i as u32 * 110 / 15);

        let r_raw: u32 = base_r as u32 * brightness_100 / 100;
        let g_raw: u32 = base_g as u32 * brightness_100 / 100;
        let b_raw: u32 = base_b as u32 * brightness_100 / 100;
        let r: u32 = if r_raw > 255 { 255 } else { r_raw };
        let g: u32 = if g_raw > 255 { 255 } else { g_raw };
        let b: u32 = if b_raw > 255 { 255 } else { b_raw };

        ramp[i] = Color {
            r: r as u8,
            g: g as u8,
            b: b as u8,
            a: 255,
        };
        i += 1;
    }
    ramp
}

/// Public wrapper for generate_ramp — creates a 16-shade gradient from an RGB base.
/// Used for tiberium color remapping (same algorithm as house colors).
pub fn generate_ramp_from_base(r: u8, g: u8, b: u8) -> [Color; RAMP_SIZE] {
    generate_ramp(r, g, b)
}

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
const DEFAULT_SCHEME_ENTRY: usize = 2;

/// Flat fallback ramp used only when the `[Colors]` list is empty (rules not yet loaded); a real
/// skirmish always has a populated scheme list.
static FALLBACK_RAMP: [Color; RAMP_SIZE] = [Color { r: 180, g: 180, b: 180, a: 255 }; RAMP_SIZE];

/// Runtime per-`[Colors]`-entry house-color ramp table. Index = `[Colors]` entry index =
/// `HouseColorIndex.0`. Built once at load from the parsed `[Colors]` schemes; replaces the legacy
/// compile-time invented `SCHEMES` once consumers are migrated.
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
    fn test_all_schemes_have_correct_size() {
        for (i, scheme) in SCHEMES.iter().enumerate() {
            assert_eq!(scheme.len(), 16, "Scheme {} has wrong size", i);
            // All colors should be fully opaque.
            for color in scheme {
                assert_eq!(color.a, 255, "Scheme {} has non-opaque color", i);
            }
        }
    }

    #[test]
    fn test_ramp_brightest_to_darkest() {
        // For each scheme, shade 0 should be brighter than shade 15.
        for (i, scheme) in SCHEMES.iter().enumerate() {
            let bright: u32 = scheme[0].r as u32 + scheme[0].g as u32 + scheme[0].b as u32;
            let dark: u32 = scheme[15].r as u32 + scheme[15].g as u32 + scheme[15].b as u32;
            assert!(
                bright > dark,
                "Scheme {} not bright→dark: {} vs {}",
                i,
                bright,
                dark
            );
        }
    }

    #[test]
    fn test_color_index_exact_match() {
        assert_eq!(color_index_for_name("DarkBlue"), HouseColorIndex(1));
        assert_eq!(color_index_for_name("darkred"), HouseColorIndex(2));
        assert_eq!(color_index_for_name("GOLD"), HouseColorIndex(0));
        assert_eq!(color_index_for_name("Green"), HouseColorIndex(3));
    }

    #[test]
    fn test_color_index_partial_match() {
        assert_eq!(color_index_for_name("Blue"), HouseColorIndex(1));
        assert_eq!(color_index_for_name("Red"), HouseColorIndex(2));
        assert_eq!(color_index_for_name("LightBlue"), HouseColorIndex(6));
    }

    #[test]
    fn test_unknown_color_returns_gold() {
        assert_eq!(color_index_for_name("PinkPolkaDot"), HouseColorIndex(0));
        assert_eq!(color_index_for_name(""), HouseColorIndex(0));
    }

    #[test]
    fn test_house_color_ramp_valid() {
        let ramp: &[Color; 16] = house_color_ramp(HouseColorIndex(1)); // DarkBlue
        // Blue should dominate in the DarkBlue scheme.
        assert!(
            ramp[0].b > ramp[0].r,
            "DarkBlue shade 0 should have more blue than red"
        );
    }

    #[test]
    fn test_out_of_range_returns_gold() {
        let gold: &[Color; 16] = house_color_ramp(HouseColorIndex(0));
        let oob: &[Color; 16] = house_color_ramp(HouseColorIndex(255));
        assert_eq!(gold[0], oob[0], "Out-of-range should return Gold ramp");
    }

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
