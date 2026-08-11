//! Overlay type registry — maps overlay ID to name and SHP filename.
//!
//! Overlay IDs in [OverlayPack] are indices into the [OverlayTypes] list
//! in rules.ini. Each overlay type has a name (e.g., "GASAND", "GEM01")
//! which is also the SHP filename prefix.
//!
//! For terrain objects from the [Terrain] section, the name IS the SHP prefix
//! (e.g., "INTREE01" → try `intree01.tem`, `intree01.shp`).
//!
//! ## Dependency rules
//! - Part of map/ — depends on rules/ (ini_parser).

use crate::rules::ini_parser::IniFile;
use crate::rules::terrain_rules::{LandType, SpeedCostProfile, TerrainRules};
use crate::rules::tiberium_type::{TiberiumType, TiberiumTypeId, TiberiumTypeRegistry};
use std::borrow::Cow;
use std::sync::OnceLock;

const STOCK_FLAT_RIPARIUS_VARIANT_COUNT: usize = 12;
const TIBERIUM_FLAT_VARIANT_COUNT: usize = 12;
const NATIVE_TIBERIUM_PRIMARY_IMAGE_COUNT: usize = 12;
const NATIVE_TIBERIUM_EXTRA_IMAGE_COUNT: usize = 8;
const NATIVE_CRUENTUS_OVERLAY_BASE: usize = 27;
const NATIVE_RIPARIUS_OVERLAY_BASE: usize = 102;
const NATIVE_VINIFERA_OVERLAY_BASE: usize = 127;
const NATIVE_ABOREUS_OVERLAY_BASE: usize = 147;

/// Per-overlay-type vertical draw bias, in screen pixels.
///
/// Applied by the native overlay draw-offset helper to Tiberium, Wall and Crate
/// overlays; every other overlay type gets 0.
const OVERLAY_TYPE_Y_DRAW_BIAS_PX: f32 = -12.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeTiberiumOverlayRange {
    base: usize,
    primary_count: usize,
    extra_count: usize,
}

impl NativeTiberiumOverlayRange {
    const fn new(base: usize, primary_count: usize, extra_count: usize) -> Self {
        Self {
            base,
            primary_count,
            extra_count,
        }
    }

    fn contains(self, overlay_id: usize) -> bool {
        let primary_end = self.base + self.primary_count;
        (self.base..primary_end).contains(&overlay_id)
            || (primary_end..primary_end + self.extra_count).contains(&overlay_id)
    }
}

fn native_tiberium_overlay_range(image: u8) -> NativeTiberiumOverlayRange {
    match image {
        2 => NativeTiberiumOverlayRange::new(
            NATIVE_CRUENTUS_OVERLAY_BASE,
            NATIVE_TIBERIUM_PRIMARY_IMAGE_COUNT,
            0,
        ),
        3 => NativeTiberiumOverlayRange::new(
            NATIVE_VINIFERA_OVERLAY_BASE,
            NATIVE_TIBERIUM_PRIMARY_IMAGE_COUNT,
            NATIVE_TIBERIUM_EXTRA_IMAGE_COUNT,
        ),
        4 => NativeTiberiumOverlayRange::new(
            NATIVE_ABOREUS_OVERLAY_BASE,
            NATIVE_TIBERIUM_PRIMARY_IMAGE_COUNT,
            NATIVE_TIBERIUM_EXTRA_IMAGE_COUNT,
        ),
        _ => NativeTiberiumOverlayRange::new(
            NATIVE_RIPARIUS_OVERLAY_BASE,
            NATIVE_TIBERIUM_PRIMARY_IMAGE_COUNT,
            NATIVE_TIBERIUM_EXTRA_IMAGE_COUNT,
        ),
    }
}

/// Check if an overlay index is a bridge overlay. The original engine identifies
/// bridges by hardcoded index position in `[OverlayTypes]`, not by INI flags.
/// These indices must not be reordered without breaking bridge logic.
pub fn is_bridge_overlay_index(id: u8) -> bool {
    matches!(
        id,
        24 | 25             // BRIDGE1, BRIDGE2 (high concrete)
        | 237 | 238         // BRIDGEB1, BRIDGEB2 (high wood)
        | 74..=101          // LOBRDG01-28 (low wood)
        | 122..=125         // LOBRDGE1-4 (low wood ends, TS)
        | 205..=232         // LOBRDB01-28 (low urban)
        | 233..=236         // LOBRDGB1-4 (low urban ends)
    )
}

/// Check if a bridge overlay index is a HIGH bridge (elevated, 3-cell-wide).
pub fn is_high_bridge_index(id: u8) -> bool {
    matches!(id, 24 | 25 | 237 | 238)
}

/// Check if an overlay index is one of the four high-bridge map-load anchors
/// that dispatch through `SetBridgeDirection`.
pub fn is_high_bridge_anchor_overlay_index(id: u8) -> bool {
    crate::map::bridge_facts::high_bridge_stamp_for_overlay(id).is_some()
}

/// Get the binary `SetBridgeDirection` direction for a high-bridge anchor.
pub fn high_bridge_stamp_direction(id: u8) -> Option<u8> {
    crate::map::bridge_facts::high_bridge_stamp_for_overlay(id).map(|(_, dir)| dir)
}

/// Get the bridge direction from a high bridge overlay index.
/// Returns None for low bridges or non-bridge indices.
pub fn high_bridge_direction(id: u8) -> Option<u8> {
    match id {
        24 | 237 => Some(1), // Direction 1 (EW / NE-SW)
        25 | 238 => Some(2), // Direction 2 (NS / NW-SE)
        _ => None,
    }
}

/// Per-overlay-type rendering flags parsed from each type's rules.ini section.
///
/// These flags select the correct palette and Y-offset for rendering.
#[derive(Debug, Clone)]
pub struct OverlayTypeFlags {
    /// Tiberium=yes — rendered with unit palette, gets -12px Y offset.
    pub tiberium: bool,
    /// ChainReaction=yes permits Apply_area_damage resource reduction.
    pub chain_reaction: bool,
    /// Wall=yes — rendered with unit palette, gets -12px Y offset.
    pub wall: bool,
    /// Overlay `Armor=wood`, used by the Wood warhead wall-damage route.
    pub armor_is_wood: bool,
    /// IsVeins=yes — rendered with unit palette, gets -12px Y offset.
    pub is_veins: bool,
    /// IsVeinholeMonster=yes — rendered with unit palette.
    pub is_veinhole_monster: bool,
    /// Gate=yes — retained for gate-specific consumers; RecalcZoneType does not read it.
    pub is_gate: bool,
    /// Crushable=yes inherited from ObjectTypeClass; RecalcZoneType column 1.
    pub crushable: bool,
    /// Crate=yes — gets -12px Y offset.
    pub crate_type: bool,
    /// `Overrides=yes` protects an existing overlay from ordinary runtime placement.
    pub overrides: bool,
    /// A non-empty `CellAnim=` supplies valid overlay art even when no SHP resolves.
    pub has_cell_anim: bool,
    /// IsRubble=yes — explicitly returns reduced ZoneType 0 after earlier overlay checks.
    pub is_rubble: bool,
    /// IsARock=yes — reduced ZoneType 6.
    pub is_a_rock: bool,
    /// True when the overlay Land= section has Wheel speed exactly 0%.
    pub land_wheel_speed_zero: bool,
    /// Overlay name identifies a bridge deck/high-bridge overlay.
    pub bridge_deck: bool,
    /// `RadarColor=R,G,B` from the type's rules section.
    ///
    /// The engine prefers the growth stage's colour out of the overlay SHP and
    /// falls back to this when that comes back essentially black, so an overlay
    /// whose art is missing still paints the right colour on radar and previews.
    pub radar_color: Option<[u8; 3]>,
    /// Railroad track overlay (TRACKS01..TRACKS16). FA2 renders these +15px lower.
    pub track: bool,
    /// Canonical Land= value. Constructor default is Clear.
    pub land: LandType,
    /// Whether CellClass retains the overlay's land instead of restoring tile land.
    /// Constructor default is true and ReadINI uses the existing field as default.
    pub no_use_tile_land_type: bool,
    /// Final Land row's speed profile, when that canonical rules section exists.
    pub land_speed_costs: Option<SpeedCostProfile>,
    /// Strength= from rules.ini — hit points for destructible overlays.
    /// Only meaningful when wall=true. Default 1.
    pub strength: u16,
    /// DamageLevels= from art.ini — number of damage stages for walls.
    /// Only meaningful when wall=true. Default 1.
    pub damage_levels: u16,
}

impl Default for OverlayTypeFlags {
    fn default() -> Self {
        Self {
            tiberium: false,
            chain_reaction: false,
            wall: false,
            armor_is_wood: false,
            is_veins: false,
            is_veinhole_monster: false,
            is_gate: false,
            crushable: false,
            crate_type: false,
            overrides: false,
            has_cell_anim: false,
            is_rubble: false,
            is_a_rock: false,
            land_wheel_speed_zero: false,
            bridge_deck: false,
            track: false,
            land: LandType::Clear,
            no_use_tile_land_type: true,
            land_speed_costs: None,
            radar_color: None,
            strength: 1,
            damage_levels: 1,
        }
    }
}

impl OverlayTypeFlags {
    /// Whether this overlay type should use the unit palette instead of theater palette.
    pub fn uses_unit_palette(&self) -> bool {
        self.tiberium || self.wall || self.is_veins || self.is_veinhole_monster
    }

    /// Y pixel offset applied before the sprite is centred on its cell.
    ///
    /// The active YR overlay draw path biases Tiberium, Wall and Crate overlays
    /// by -12px and leaves everything else at 0. It is NOT the 15px half-cell
    /// height: the half-cell shift is already carried by the cell-centre term
    /// the caller adds, and both engines apply it identically, so this constant
    /// is the whole remaining vertical delta. `IsVeins=` is deliberately absent
    /// — it is not in the binary's predicate (and is TS-legacy dead data in YR).
    ///
    /// Not modelled: the `-1` the binary adds for `Land=Railroad` overlays and
    /// for one specific overlay slot. The railroad case is entangled with a
    /// separate FA2-sourced track offset in the instance builder; see the
    /// GSI-13.05 report.
    pub fn y_draw_offset(&self) -> f32 {
        if self.tiberium || self.wall || self.crate_type {
            OVERLAY_TYPE_Y_DRAW_BIAS_PX
        } else {
            0.0
        }
    }
}

/// Registry of overlay type names indexed by overlay ID.
///
/// Built from the [OverlayTypes] section of rules.ini.
/// Overlay IDs 0..N map to the names listed in order.
pub struct OverlayTypeRegistry {
    /// Overlay ID -> name (indexed by preserved internal [OverlayTypes] IDs).
    names: Vec<String>,
    /// Per-type rendering flags (same indexing as names).
    flags: Vec<OverlayTypeFlags>,
}

impl OverlayTypeRegistry {
    /// Parse [OverlayTypes] from rules.ini into an indexed registry.
    ///
    /// The section lists types with numeric keys: `0=GASAND\n1=GEM01\n...`.
    /// RA2/YR may skip some numeric keys, but internal overlay ids still follow
    /// the ordered list rather than reserving holes for every missing raw key.
    /// Returns an empty registry if the section is missing.
    pub fn from_ini(ini: &IniFile, art_ini: Option<&IniFile>) -> Self {
        let section = match ini.section("OverlayTypes") {
            Some(s) => s,
            None => {
                log::warn!("[OverlayTypes] section not found in rules.ini");
                return OverlayTypeRegistry {
                    names: Vec::new(),
                    flags: Vec::new(),
                };
            }
        };

        let names: Vec<String> = section
            .get_values()
            .into_iter()
            .map(str::to_string)
            .collect();
        if names.is_empty() {
            log::warn!("[OverlayTypes] present but empty");
            return OverlayTypeRegistry {
                names: Vec::new(),
                flags: Vec::new(),
            };
        }

        // Native registry allocation follows declaration order; numeric key
        // text is not a sorting authority.
        let terrain_rules = TerrainRules::from_ini(ini);
        let clear_speed_costs = terrain_rules
            .semantics_for_land_type(LandType::Clear.as_index())
            .map(|semantics| semantics.speed_costs);
        let mut flags: Vec<OverlayTypeFlags> = Vec::with_capacity(names.len());
        for (idx, name) in names.iter().enumerate() {
            let upper_name = name.to_ascii_uppercase();
            // Bridge overlays are identified by hardcoded index position in
            // [OverlayTypes], matching the original engine's direct index checks.
            let bridge_deck = is_bridge_overlay_index(idx as u8);
            let track = upper_name.starts_with("TRACKS");
            if let Some(type_section) = ini.section(name) {
                let tiberium = type_section.get_bool("Tiberium").unwrap_or(false);
                let mut land = type_section
                    .get("Land")
                    .and_then(parse_land_type)
                    .unwrap_or(LandType::Clear);
                if tiberium && land == LandType::Clear {
                    land = LandType::Tiberium;
                }
                let land_speed_costs = terrain_rules
                    .semantics_for_land_type(land.as_index())
                    .map(|semantics| semantics.speed_costs);
                let land_wheel_speed_zero =
                    land_speed_costs.is_some_and(|speed_costs| speed_costs.wheel == Some(0));
                // Strength from rules section (e.g., [GAWALL] Strength=300).
                let strength = type_section
                    .get("Strength")
                    .and_then(|v| v.parse::<u16>().ok())
                    .unwrap_or(1);
                let radar_color = type_section.get("RadarColor").and_then(parse_radar_color);
                // DamageLevels from art section (e.g., [GASAND] DamageLevels=2 in art.ini).
                let damage_levels = art_ini
                    .and_then(|art| art.section(name))
                    .and_then(|s| s.get("DamageLevels"))
                    .and_then(|v| v.parse::<u16>().ok())
                    .unwrap_or(1);
                flags.push(OverlayTypeFlags {
                    tiberium,
                    chain_reaction: type_section.get_bool("ChainReaction").unwrap_or(false),
                    wall: type_section.get_bool("Wall").unwrap_or(false),
                    armor_is_wood: type_section
                        .get("Armor")
                        .is_some_and(|armor| armor.eq_ignore_ascii_case("wood")),
                    is_veins: type_section.get_bool("IsVeins").unwrap_or(false),
                    is_veinhole_monster: type_section
                        .get_bool("IsVeinholeMonster")
                        .unwrap_or(false),
                    is_gate: type_section.get_bool("Gate").unwrap_or(false),
                    crushable: type_section.get_bool("Crushable").unwrap_or(false),
                    crate_type: type_section.get_bool("Crate").unwrap_or(false),
                    overrides: type_section.get_bool("Overrides").unwrap_or(false),
                    has_cell_anim: type_section
                        .get("CellAnim")
                        .is_some_and(|value| !value.trim().is_empty()),
                    is_rubble: type_section.get_bool("IsRubble").unwrap_or(false),
                    is_a_rock: type_section.get_bool("IsARock").unwrap_or(false),
                    land_wheel_speed_zero,
                    bridge_deck,
                    track,
                    land,
                    no_use_tile_land_type: type_section
                        .get_bool("NoUseTileLandType")
                        .unwrap_or(true),
                    land_speed_costs,
                    radar_color,
                    strength,
                    damage_levels,
                });
            } else {
                flags.push(OverlayTypeFlags {
                    bridge_deck,
                    track,
                    land_speed_costs: clear_speed_costs,
                    land_wheel_speed_zero: clear_speed_costs
                        .is_some_and(|speed_costs| speed_costs.wheel == Some(0)),
                    ..OverlayTypeFlags::default()
                });
            }
        }
        let max_index: usize = names.len().saturating_sub(1);

        log::info!(
            "OverlayTypeRegistry: {} types loaded (max_id={})",
            names.len(),
            max_index,
        );
        OverlayTypeRegistry { names, flags }
    }

    /// Create an empty registry (used as fallback when map loading fails).
    pub fn empty() -> Self {
        OverlayTypeRegistry {
            names: Vec::new(),
            flags: Vec::new(),
        }
    }

    /// Look up the name for an overlay ID. Returns None if out of range.
    pub fn name(&self, overlay_id: u8) -> Option<&str> {
        self.names
            .get(overlay_id as usize)
            .filter(|s| !s.is_empty())
            .map(|s| s.as_str())
    }

    /// Look up the rendering flags for an overlay ID.
    pub fn flags(&self, overlay_id: u8) -> Option<&OverlayTypeFlags> {
        self.flags.get(overlay_id as usize)
    }

    /// Look up flags by overlay name (case-sensitive).
    pub fn flags_by_name(&self, name: &str) -> Option<&OverlayTypeFlags> {
        self.names
            .iter()
            .position(|n| n == name)
            .and_then(|i| self.flags.get(i))
    }

    /// Look up overlay_id by name (case-insensitive). Returns None if not found.
    pub fn id_for_name(&self, name: &str) -> Option<u8> {
        self.names
            .iter()
            .position(|n| n.eq_ignore_ascii_case(name))
            .and_then(|i| u8::try_from(i).ok())
    }

    /// Stock flat Riparius overlay variants (`TIB01..TIB12`) in registry order.
    ///
    /// gamemd's no-overlay TIBTRE placement picks one of these 12 entries using
    /// the Riparius image index plus `RandomRanged(0, 0xB)`. The returned IDs
    /// come from the parsed registry positions and require `Tiberium=yes`, so no
    /// internal overlay IDs are baked into Rust.
    pub fn stock_flat_riparius_variant_ids(&self) -> Option<[u8; 12]> {
        let mut ids: Vec<u8> = Vec::with_capacity(STOCK_FLAT_RIPARIUS_VARIANT_COUNT);
        let mut found = [false; STOCK_FLAT_RIPARIUS_VARIANT_COUNT];

        for (idx, name) in self.names.iter().enumerate() {
            let Some(variant) = stock_flat_riparius_variant_index(name) else {
                continue;
            };
            if found[variant] || !self.flags.get(idx).is_some_and(|flags| flags.tiberium) {
                return None;
            }
            found[variant] = true;
            ids.push(u8::try_from(idx).ok()?);
        }

        if !found.iter().all(|present| *present) {
            return None;
        }

        ids.try_into().ok()
    }

    /// Flat overlay variants for one parsed tiberium type image family.
    pub fn flat_tiberium_variant_ids(&self, ty: &TiberiumType) -> Option<[u8; 12]> {
        let mut ids: Vec<u8> = Vec::with_capacity(TIBERIUM_FLAT_VARIANT_COUNT);
        for variant in 1..=TIBERIUM_FLAT_VARIANT_COUNT {
            let name = tiberium_flat_overlay_name(ty.image, variant as u8)?;
            let id = self.id_for_name(&name)?;
            if !self.flags(id).is_some_and(|flags| flags.tiberium) {
                return None;
            }
            ids.push(id);
        }
        ids.try_into().ok()
    }

    /// Reproduce the native overlay flag gate and ordered tiberium range lookup.
    pub fn tiberium_type_for_overlay(
        &self,
        tiberium_types: &TiberiumTypeRegistry,
        overlay_id: u8,
    ) -> Option<TiberiumTypeId> {
        if !self.flags(overlay_id).is_some_and(|flags| flags.tiberium) {
            return None;
        }

        for ty in tiberium_types.types() {
            if native_tiberium_overlay_range(ty.image).contains(usize::from(overlay_id)) {
                return Some(ty.id);
            }
        }

        tiberium_types.types().first().map(|ty| ty.id)
    }

    /// Total number of registered overlay types.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

fn stock_flat_riparius_variant_index(name: &str) -> Option<usize> {
    let prefix = name.get(..3)?;
    if !prefix.eq_ignore_ascii_case("TIB") {
        return None;
    }
    let suffix = name.get(3..)?;
    if suffix.len() != 2 {
        return None;
    }
    let variant = suffix.parse::<usize>().ok()?;
    (1..=STOCK_FLAT_RIPARIUS_VARIANT_COUNT)
        .contains(&variant)
        .then_some(variant - 1)
}

fn tiberium_flat_overlay_name(image: u8, variant: u8) -> Option<String> {
    if variant == 0 || variant as usize > TIBERIUM_FLAT_VARIANT_COUNT {
        return None;
    }
    match image {
        1 => Some(format!("TIB{:02}", variant)),
        2 => Some(format!("GEM{:02}", variant)),
        image => image
            .checked_sub(1)
            .map(|suffix| format!("TIB{}_{:02}", suffix, variant)),
    }
}

fn parse_land_type(value: &str) -> Option<LandType> {
    LandType::ALL
        .into_iter()
        .find(|land_type| value.eq_ignore_ascii_case(land_type.section_name()))
}

/// Exact predicate for RecalcAttributes' early overlay-Land branch.
pub(crate) fn uses_early_recalc_land_branch(flags: &OverlayTypeFlags) -> bool {
    flags.land == LandType::Wall || flags.land == LandType::Railroad || flags.no_use_tile_land_type
}

/// Whether RecalcAttributes removes this resource overlay object/index/data.
pub(crate) fn clears_tiberium_on_slope(flags: &OverlayTypeFlags, slope_type: u8) -> bool {
    flags.tiberium && slope_type != 0 && (uses_early_recalc_land_branch(flags) || slope_type >= 5)
}

/// Cell land retained by the current RecalcAttributes invocation.
///
/// The early branch copies overlay Land before it removes a sloped resource,
/// so that invocation keeps the copied land even though the overlay pointer is
/// gone. A normal/nonclaiming resource removed on slope 5+ restores tile land.
pub(crate) fn retained_overlay_land(flags: &OverlayTypeFlags, slope_type: u8) -> Option<LandType> {
    let uses_early_branch = uses_early_recalc_land_branch(flags);
    if clears_tiberium_on_slope(flags, slope_type) && !uses_early_branch {
        return None;
    }
    (uses_early_branch || flags.tiberium).then_some(flags.land)
}

/// Generate candidate SHP filenames for an overlay name.
///
/// Returns a list of filenames to try in order, using the theater extension first
/// (e.g., for temperate: `.tem`), then the generic `.shp` extension.
/// Both lowercase and original case are tried.
/// Parse a `RadarColor=R,G,B` value. Anything malformed yields `None` so the
/// caller falls back to the overlay's art rather than to a wrong colour.
fn parse_radar_color(value: &str) -> Option<[u8; 3]> {
    let mut channels = value.split(',').map(|part| part.trim().parse::<u8>());
    let red = channels.next()?.ok()?;
    let green = channels.next()?.ok()?;
    let blue = channels.next()?.ok()?;
    if channels.next().is_some() {
        return None;
    }
    Some([red, green, blue])
}

pub fn overlay_shp_candidates(name: &str, theater_ext: &str) -> Vec<String> {
    let lower: String = name.to_lowercase();
    vec![
        format!("{}.{}", lower, theater_ext),
        format!("{}.shp", lower),
        format!("{}.{}", name, theater_ext),
        format!("{}.shp", name),
    ]
}

/// Generate candidate SHP filenames for a terrain object (from [Terrain] section).
///
/// Terrain objects like "INTREE01" may have theater-specific variants.
pub fn terrain_shp_candidates(name: &str, theater_ext: &str) -> Vec<String> {
    overlay_shp_candidates(name, theater_ext)
}

/// Optional debug remap for problematic resource overlays.
///
/// When `RA2_FORCE_TIB3_TO_TIB01=1`, remap `TIB3_20` to `TIB01`.
/// This is a temporary diagnostic switch to isolate rules/mapping issues.
pub fn remap_overlay_name_for_debug<'a>(name: &'a str) -> Cow<'a, str> {
    static FORCE_TIB3_TO_TIB01: OnceLock<bool> = OnceLock::new();
    let enabled: bool = *FORCE_TIB3_TO_TIB01.get_or_init(|| {
        std::env::var("RA2_FORCE_TIB3_TO_TIB01")
            .ok()
            .map(|v| {
                let n = v.trim().to_ascii_lowercase();
                n == "1" || n == "true" || n == "yes" || n == "on"
            })
            .unwrap_or(false)
    });
    if enabled && name.eq_ignore_ascii_case("TIB3_20") {
        Cow::Borrowed("TIB01")
    } else {
        Cow::Borrowed(name)
    }
}

fn is_resource_overlay_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    upper.starts_with("TIB") || upper.starts_with("GEM")
}

fn tiberium_id_offset() -> isize {
    static TIB_ID_OFFSET: OnceLock<isize> = OnceLock::new();
    *TIB_ID_OFFSET.get_or_init(|| {
        std::env::var("RA2_TIB_ID_OFFSET")
            .ok()
            .and_then(|s| s.parse::<isize>().ok())
            .unwrap_or(0)
    })
}

/// Resolve overlay name for rendering/debug display with optional resource-only ID offset.
///
/// `RA2_TIB_ID_OFFSET=N` applies only when the base ID resolves to TIB*/GEM* and
/// the shifted target also resolves to TIB*/GEM*. This avoids shifting bridges/rocks.
pub fn resolve_overlay_name_for_render(
    reg: &OverlayTypeRegistry,
    overlay_id: u8,
) -> Option<String> {
    let base_name = reg.name(overlay_id)?;
    let mut resolved_id: u8 = overlay_id;
    let offset = tiberium_id_offset();
    if offset != 0 && is_resource_overlay_name(base_name) {
        let shifted = overlay_id as isize + offset;
        if (0..=u8::MAX as isize).contains(&shifted) {
            let shifted_id = shifted as u8;
            if let Some(candidate) = reg.name(shifted_id) {
                if is_resource_overlay_name(candidate) {
                    resolved_id = shifted_id;
                }
            }
        }
    }
    reg.name(resolved_id)
        .map(|n| remap_overlay_name_for_debug(n).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_overlay_types() {
        let text: &str = "\
[OverlayTypes]
0=GASAND
1=INTREE01
2=GAWALL
3=GEM01
";
        let ini: IniFile = IniFile::from_str(text);
        let reg: OverlayTypeRegistry = OverlayTypeRegistry::from_ini(&ini, None);
        assert_eq!(reg.len(), 4);
        assert_eq!(reg.name(0), Some("GASAND"));
        assert_eq!(reg.name(1), Some("INTREE01"));
        assert_eq!(reg.name(3), Some("GEM01"));
        assert_eq!(reg.name(255), None);
    }

    #[test]
    fn test_shp_candidates() {
        let names: Vec<String> = overlay_shp_candidates("GEM01", "tem");
        assert_eq!(names[0], "gem01.tem");
        assert_eq!(names[1], "gem01.shp");
        assert_eq!(names[2], "GEM01.tem");
        assert_eq!(names[3], "GEM01.shp");
    }

    #[test]
    fn test_sparse_overlay_types() {
        let text: &str = "\
[OverlayTypes]
0=GASAND
1=GEM01
5=BRIDGE
10=BIGFENCE
";
        let ini: IniFile = IniFile::from_str(text);
        let reg: OverlayTypeRegistry = OverlayTypeRegistry::from_ini(&ini, None);
        // Keys sorted by numeric value, compacted to 0-based sequential indices.
        assert_eq!(reg.len(), 4);
        assert_eq!(reg.name(0), Some("GASAND"));
        assert_eq!(reg.name(1), Some("GEM01"));
        assert_eq!(reg.name(2), Some("BRIDGE"));
        assert_eq!(reg.name(3), Some("BIGFENCE"));
        assert_eq!(reg.name(4), None);
    }

    #[test]
    fn test_empty_registry() {
        let text: &str = "[General]\nKey=Value\n";
        let ini: IniFile = IniFile::from_str(text);
        let reg: OverlayTypeRegistry = OverlayTypeRegistry::from_ini(&ini, None);
        assert!(reg.is_empty());
        assert_eq!(reg.name(0), None);
    }

    #[test]
    fn test_parse_reduced_zone_overlay_flags() {
        let text: &str = "\
[OverlayTypes]
0=SANDBAG
1=ROCKOVL
2=RUBBLE
[Clear]
Wheel=100%
[Rock]
Wheel=0%
[SANDBAG]
Crushable=yes
Wall=yes
Land=Clear
[ROCKOVL]
Land=Rock
IsARock=yes
[RUBBLE]
IsRubble=yes
";
        let ini: IniFile = IniFile::from_str(text);
        let reg: OverlayTypeRegistry = OverlayTypeRegistry::from_ini(&ini, None);

        let sandbag = reg.flags(0).expect("sandbag flags");
        assert!(sandbag.crushable);
        assert!(sandbag.wall);
        assert!(!sandbag.land_wheel_speed_zero);

        let rock = reg.flags(1).expect("rock flags");
        assert!(rock.is_a_rock);
        assert!(rock.land_wheel_speed_zero);

        let rubble = reg.flags(2).expect("rubble flags");
        assert!(rubble.is_rubble);
    }

    #[test]
    fn gsi_04_04_overlay_land_defaults_and_exact_names() {
        let mut text = String::from("[OverlayTypes]\n0=MISSING_SECTION\n");
        for (index, _land) in LandType::ALL.into_iter().enumerate() {
            text.push_str(&format!("{}=LAND_{index}\n", index + 1));
        }
        text.push_str("13=LOWERCASE\n");
        for (index, land) in LandType::ALL.into_iter().enumerate() {
            text.push_str(&format!(
                "[LAND_{index}]\nLand={}\nNoUseTileLandType=no\n",
                land.section_name()
            ));
        }
        text.push_str("[LOWERCASE]\nLand=road\n");

        let ini = IniFile::from_str(&text);
        let reg = OverlayTypeRegistry::from_ini(&ini, None);

        let missing = reg.flags(0).expect("missing-section defaults");
        assert_eq!(missing.land, LandType::Clear);
        assert!(missing.no_use_tile_land_type);
        for (index, expected) in LandType::ALL.into_iter().enumerate() {
            let flags = reg.flags((index + 1) as u8).expect("canonical land flags");
            assert_eq!(flags.land, expected, "{}", expected.section_name());
            assert!(!flags.no_use_tile_land_type);
        }
        let lowercase = reg.flags(13).expect("lowercase land flags");
        assert_eq!(lowercase.land, LandType::Road);
        assert!(lowercase.no_use_tile_land_type);
    }

    #[test]
    fn gsi_04_04_tiberium_forces_land_before_final_wheel_lookup() {
        let ini = IniFile::from_str(
            "\
[OverlayTypes]
0=ORE
[Clear]
Wheel=100%
[Tiberium]
Wheel=0%
[ORE]
Tiberium=yes
Land=Clear
NoUseTileLandType=no
",
        );
        let reg = OverlayTypeRegistry::from_ini(&ini, None);
        let flags = reg.flags(0).expect("ore flags");

        assert!(flags.tiberium);
        assert_eq!(flags.land, LandType::Tiberium);
        assert!(!flags.no_use_tile_land_type);
        assert!(flags.land_wheel_speed_zero);
        assert_eq!(
            flags.land_speed_costs.expect("Tiberium speed row").wheel,
            Some(0)
        );
    }

    #[test]
    fn gsi_04_11_chain_reaction_ctor_default_is_false_and_ini_key_overrides() {
        let ini = IniFile::from_str(
            "[OverlayTypes]\n0=GREEN\n1=CHAIN\n2=BLUE\n\
             [GREEN]\nTiberium=yes\n\
             [CHAIN]\nTiberium=yes\nChainReaction=yes\n\
             [BLUE]\nTiberium=yes\nChainReaction=no\n",
        );
        let reg = OverlayTypeRegistry::from_ini(&ini, None);

        assert!(!reg.flags(0).unwrap().chain_reaction);
        assert!(reg.flags(1).unwrap().chain_reaction);
        assert!(!reg.flags(2).unwrap().chain_reaction);
    }

    #[test]
    fn gsi_04_04_overlay_land_retention_matches_recalc_attributes() {
        let mut ordinary = OverlayTypeFlags {
            land: LandType::Road,
            no_use_tile_land_type: false,
            ..OverlayTypeFlags::default()
        };
        assert_eq!(retained_overlay_land(&ordinary, 0), None);

        ordinary.no_use_tile_land_type = true;
        assert!(uses_early_recalc_land_branch(&ordinary));
        assert_eq!(retained_overlay_land(&ordinary, 0), Some(LandType::Road));

        for land in [LandType::Wall, LandType::Railroad] {
            let flags = OverlayTypeFlags {
                land,
                no_use_tile_land_type: false,
                ..OverlayTypeFlags::default()
            };
            assert!(uses_early_recalc_land_branch(&flags));
            assert_eq!(retained_overlay_land(&flags, 0), Some(land));
        }

        let resource = OverlayTypeFlags {
            tiberium: true,
            land: LandType::Tiberium,
            no_use_tile_land_type: false,
            ..OverlayTypeFlags::default()
        };
        assert_eq!(
            retained_overlay_land(&resource, 4),
            Some(LandType::Tiberium)
        );
        assert_eq!(
            retained_overlay_land(&resource, 1),
            Some(LandType::Tiberium)
        );
        assert_eq!(retained_overlay_land(&resource, 5), None);

        let stock_default = OverlayTypeFlags {
            tiberium: true,
            land: LandType::Tiberium,
            ..OverlayTypeFlags::default()
        };
        assert_eq!(
            [0, 1, 4, 5].map(|slope| retained_overlay_land(&stock_default, slope)),
            [Some(LandType::Tiberium); 4]
        );

        for land in [LandType::Wall, LandType::Railroad] {
            let authoritative = OverlayTypeFlags {
                tiberium: true,
                land,
                no_use_tile_land_type: false,
                ..OverlayTypeFlags::default()
            };
            assert_eq!(retained_overlay_land(&authoritative, 0), Some(land));
            assert_eq!(retained_overlay_land(&authoritative, 1), Some(land));
        }
    }

    #[test]
    fn test_high_bridge_anchor_overlay_helpers_are_narrow() {
        for id in [0x18, 0x19, 0xED, 0xEE] {
            assert!(is_high_bridge_anchor_overlay_index(id));
            assert!(high_bridge_stamp_direction(id).is_some());
            assert!(is_bridge_overlay_index(id));
        }
        for id in [0x4A, 0x7A, 0xCD, 0xE9] {
            assert!(!is_high_bridge_anchor_overlay_index(id));
            assert_eq!(high_bridge_stamp_direction(id), None);
        }
        assert!(is_bridge_overlay_index(0x4A));
        assert!(is_bridge_overlay_index(0xCD));
    }

    #[test]
    fn test_one_based_overlay_types_compacted() {
        let text: &str = "\
[OverlayTypes]
1=GASAND
2=GEM01
";
        let ini: IniFile = IniFile::from_str(text);
        let reg: OverlayTypeRegistry = OverlayTypeRegistry::from_ini(&ini, None);
        // Keys sorted and compacted to 0-based — key numbers are ordering hints only.
        assert_eq!(reg.len(), 2);
        assert_eq!(reg.name(0), Some("GASAND"));
        assert_eq!(reg.name(1), Some("GEM01"));
    }

    #[test]
    fn test_sparse_keys_compacted() {
        let text: &str = "\
[OverlayTypes]
1=GASAND
2=GEM01
6=BRIDGE
11=BIGFENCE
";
        let ini: IniFile = IniFile::from_str(text);
        let reg: OverlayTypeRegistry = OverlayTypeRegistry::from_ini(&ini, None);
        assert_eq!(reg.len(), 4);
        assert_eq!(reg.name(0), Some("GASAND"));
        assert_eq!(reg.name(1), Some("GEM01"));
        assert_eq!(reg.name(2), Some("BRIDGE"));
        assert_eq!(reg.name(3), Some("BIGFENCE"));
    }

    #[test]
    fn stock_flat_riparius_variant_ids_use_registry_order() {
        let text = "\
[OverlayTypes]
1=GASAND
2=TIB01
3=TIB02
4=TIB03
5=TIB04
6=TIB05
7=TIB06
8=TIB07
9=TIB08
10=TIB09
11=TIB10
12=TIB11
13=TIB12
";
        let mut sections = String::from(text);
        for i in 1..=12 {
            sections.push_str(&format!("[TIB{:02}]\nTiberium=yes\n", i));
        }
        let ini = IniFile::from_str(&sections);
        let reg = OverlayTypeRegistry::from_ini(&ini, None);

        assert_eq!(
            reg.stock_flat_riparius_variant_ids(),
            Some([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12])
        );
    }

    #[test]
    fn stock_flat_riparius_variant_ids_require_tiberium_flag() {
        let mut text = String::from("[OverlayTypes]\n");
        for i in 1..=12 {
            text.push_str(&format!("{}=TIB{:02}\n", i, i));
        }
        for i in 1..=11 {
            text.push_str(&format!("[TIB{:02}]\nTiberium=yes\n", i));
        }
        text.push_str("[TIB12]\nTiberium=no\n");

        let ini = IniFile::from_str(&text);
        let reg = OverlayTypeRegistry::from_ini(&ini, None);

        assert_eq!(reg.stock_flat_riparius_variant_ids(), None);
    }

    fn stock_shaped_tiberium_ini(false_flag_name: Option<&str>) -> IniFile {
        let mut text = String::from(
            "\
[Tiberiums]
0=Riparius
1=Cruentus
2=Vinifera
3=Aboreus

[Riparius]
Image=1
[Cruentus]
Image=2
[Vinifera]
Image=3
[Aboreus]
Image=4

[OverlayTypes]
",
        );

        let mut names = Vec::new();
        for raw_key in (1..=170).filter(|key| *key != 40 && *key != 41) {
            let name = match raw_key {
                28..=39 => format!("GEM{:02}", raw_key - 27),
                105..=124 => format!("TIB{:02}", raw_key - 104),
                130..=149 => format!("TIB2_{:02}", raw_key - 129),
                150..=169 => format!("TIB3_{:02}", raw_key - 149),
                170 => "STRAY".to_owned(),
                _ => format!("FILL{raw_key:03}"),
            };
            text.push_str(&format!("{raw_key}={name}\n"));
            names.push(name);
        }
        for name in names {
            let enabled = false_flag_name != Some(name.as_str());
            text.push_str(&format!(
                "[{name}]\nTiberium={}\n",
                if enabled { "yes" } else { "no" }
            ));
        }

        IniFile::from_str(&text)
    }

    fn stock_shaped_tiberium_registries(
        false_flag_name: Option<&str>,
    ) -> (OverlayTypeRegistry, TiberiumTypeRegistry) {
        let ini = stock_shaped_tiberium_ini(false_flag_name);
        (
            OverlayTypeRegistry::from_ini(&ini, None),
            TiberiumTypeRegistry::from_ini(&ini),
        )
    }

    #[test]
    fn native_tiberium_overlay_range_covers_every_u8_selector() {
        let cruentus = NativeTiberiumOverlayRange::new(27, 12, 0);
        let riparius = NativeTiberiumOverlayRange::new(102, 12, 8);
        let vinifera = NativeTiberiumOverlayRange::new(127, 12, 8);
        let aboreus = NativeTiberiumOverlayRange::new(147, 12, 8);

        for image in u8::MIN..=u8::MAX {
            let expected = match image {
                2 => cruentus,
                3 => vinifera,
                4 => aboreus,
                _ => riparius,
            };
            assert_eq!(native_tiberium_overlay_range(image), expected);
        }
        for image in [0, 1, 5, 255] {
            assert_eq!(native_tiberium_overlay_range(image), riparius);
        }
    }

    #[test]
    fn overlay_to_tiberium_index_covers_stock_primary_and_extra_boundaries() {
        let (overlays, tiberiums) = stock_shaped_tiberium_registries(None);
        let cases = [
            ("GEM01", 27, 1),
            ("GEM12", 38, 1),
            ("TIB01", 102, 0),
            ("TIB12", 113, 0),
            ("TIB13", 114, 0),
            ("TIB20", 121, 0),
            ("TIB2_01", 127, 2),
            ("TIB2_12", 138, 2),
            ("TIB2_13", 139, 2),
            ("TIB2_20", 146, 2),
            ("TIB3_01", 147, 3),
            ("TIB3_12", 158, 3),
            ("TIB3_13", 159, 3),
            ("TIB3_20", 166, 3),
        ];

        for (name, expected_overlay_id, expected_type_id) in cases {
            let overlay_id = overlays
                .id_for_name(name)
                .unwrap_or_else(|| panic!("{name} id"));
            assert_eq!(overlay_id, expected_overlay_id, "{name} compact id");
            assert_eq!(
                overlays.tiberium_type_for_overlay(&tiberiums, overlay_id),
                Some(TiberiumTypeId(expected_type_id)),
                "{name} type"
            );
        }

        let after_gem = overlays.id_for_name("FILL042").expect("flagged slot 39");
        assert_eq!(after_gem, 39);
        assert_eq!(
            overlays.tiberium_type_for_overlay(&tiberiums, after_gem),
            Some(TiberiumTypeId(0)),
            "Cruentus has no extra range; a flagged miss falls back to type zero"
        );
    }

    #[test]
    fn overlay_to_tiberium_index_uses_compact_slots_across_numeric_key_gaps() {
        let (overlays, tiberiums) = stock_shaped_tiberium_registries(None);
        for (name, compact_id, type_id) in
            [("GEM01", 27, 1), ("TIB2_01", 127, 2), ("TIB3_01", 147, 3)]
        {
            let overlay_id = overlays
                .id_for_name(name)
                .unwrap_or_else(|| panic!("{name} id"));
            assert_eq!(overlay_id, compact_id);
            assert_eq!(
                overlays.tiberium_type_for_overlay(&tiberiums, overlay_id),
                Some(TiberiumTypeId(type_id))
            );
        }
    }

    #[test]
    fn overlay_to_tiberium_index_rejects_false_flag_before_range_lookup() {
        let (overlays, tiberiums) = stock_shaped_tiberium_registries(Some("TIB2_01"));
        let overlay_id = overlays.id_for_name("TIB2_01").expect("TIB2_01 id");
        assert_eq!(
            overlays.tiberium_type_for_overlay(&tiberiums, overlay_id),
            None
        );
    }

    #[test]
    fn overlay_to_tiberium_index_flagged_range_miss_falls_back_to_type_zero() {
        let (overlays, tiberiums) = stock_shaped_tiberium_registries(None);
        let stray_id = overlays.id_for_name("STRAY").expect("STRAY id");
        assert_eq!(stray_id, 167);
        assert_eq!(
            overlays.tiberium_type_for_overlay(&tiberiums, stray_id),
            Some(TiberiumTypeId(0))
        );

        let (unflagged, tiberiums) = stock_shaped_tiberium_registries(Some("STRAY"));
        let stray_id = unflagged.id_for_name("STRAY").expect("STRAY id");
        assert_eq!(
            unflagged.tiberium_type_for_overlay(&tiberiums, stray_id),
            None
        );
    }

    #[test]
    fn overlay_to_tiberium_index_overlapping_ranges_keep_first_tiberium_order() {
        let mut text = String::from(
            "\
[Tiberiums]
0=Nonmatching
1=FirstDefault
2=SecondDefault

[Nonmatching]
Image=2
[FirstDefault]
Image=1
[SecondDefault]
Image=5

[OverlayTypes]
",
        );
        for key in 0..=102 {
            let name = if key == 102 {
                "TARGET".to_owned()
            } else {
                format!("FILL{key:03}")
            };
            text.push_str(&format!("{key}={name}\n"));
        }
        text.push_str("[TARGET]\nTiberium=yes\n");
        let ini = IniFile::from_str(&text);
        let overlays = OverlayTypeRegistry::from_ini(&ini, None);
        let tiberiums = TiberiumTypeRegistry::from_ini(&ini);
        let target = overlays.id_for_name("TARGET").expect("TARGET id");

        assert_eq!(target, 102);
        assert_eq!(
            overlays.tiberium_type_for_overlay(&tiberiums, target),
            Some(TiberiumTypeId(1))
        );
    }

    #[test]
    fn overlay_to_tiberium_index_handles_unknown_and_empty_registries() {
        let (overlays, tiberiums) = stock_shaped_tiberium_registries(None);
        assert_eq!(
            overlays.tiberium_type_for_overlay(&tiberiums, u8::MAX),
            None
        );

        let empty_types = TiberiumTypeRegistry::from_ini(&IniFile::from_str(""));
        let stray_id = overlays.id_for_name("STRAY").expect("STRAY id");
        assert_eq!(
            overlays.tiberium_type_for_overlay(&empty_types, stray_id),
            None
        );
    }

    #[test]
    fn flat_tiberium_variants_remain_twelve_primary_images() {
        let (overlays, tiberiums) = stock_shaped_tiberium_registries(None);
        let expected = [
            ("TIB01", "TIB12", "TIB13"),
            ("GEM01", "GEM12", "FILL042"),
            ("TIB2_01", "TIB2_12", "TIB2_13"),
            ("TIB3_01", "TIB3_12", "TIB3_13"),
        ];

        for (ty, (first, last, first_extra)) in tiberiums.types().iter().zip(expected) {
            let variants = overlays
                .flat_tiberium_variant_ids(ty)
                .unwrap_or_else(|| panic!("flat variants for {}", ty.section));
            assert_eq!(variants.len(), 12);
            assert_eq!(
                variants[0],
                overlays.id_for_name(first).expect("first primary")
            );
            assert_eq!(
                variants[11],
                overlays.id_for_name(last).expect("last primary")
            );
            assert!(
                !variants.contains(
                    &overlays
                        .id_for_name(first_extra)
                        .expect("first extra or post-family slot")
                )
            );
        }
    }

    #[test]
    fn overlay_y_draw_bias_matches_the_native_predicate() {
        // The binary biases Tiberium / Wall / Crate overlays by -12 and nothing
        // else. -15 (half a cell height) is the caller's cell-centre term, not
        // this one, so folding it in here double-counts by 3px on every ore
        // cell, wall and crate on screen.
        for flags in [
            OverlayTypeFlags {
                tiberium: true,
                ..OverlayTypeFlags::default()
            },
            OverlayTypeFlags {
                wall: true,
                ..OverlayTypeFlags::default()
            },
            OverlayTypeFlags {
                crate_type: true,
                ..OverlayTypeFlags::default()
            },
        ] {
            assert_eq!(flags.y_draw_offset(), -12.0);
        }

        // Bridges, tracks, rocks and the rest sit flat on the cell.
        assert_eq!(OverlayTypeFlags::default().y_draw_offset(), 0.0);
        assert_eq!(
            OverlayTypeFlags {
                bridge_deck: true,
                ..OverlayTypeFlags::default()
            }
            .y_draw_offset(),
            0.0
        );
        // IsVeins= is not part of the native predicate.
        assert_eq!(
            OverlayTypeFlags {
                is_veins: true,
                ..OverlayTypeFlags::default()
            }
            .y_draw_offset(),
            0.0
        );
    }

    #[test]
    fn radar_color_parses_only_a_well_formed_triple() {
        assert_eq!(parse_radar_color("220,200,0"), Some([220, 200, 0]));
        assert_eq!(parse_radar_color(" 220 , 200 , 0 "), Some([220, 200, 0]));
        // Malformed values yield None so the caller falls back to the art
        // rather than painting a confidently wrong colour.
        assert_eq!(parse_radar_color("220,200"), None);
        assert_eq!(parse_radar_color("220,200,0,5"), None);
        assert_eq!(parse_radar_color("220,200,300"), None);
        assert_eq!(parse_radar_color(""), None);
    }
}
