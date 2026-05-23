//! Map lighting — parses [Lighting] section and computes per-cell RGB tint.
//!
//! RA2 maps define global lighting parameters (Ambient, Red, Green, Blue, Ground,
//! Level) in their INI [Lighting] section. These determine a per-cell color
//! multiplier that tints terrain tiles and entity sprites. Ordinary map
//! lighting is app/render-derived state; simulation does not own this data.
//!
//! Point light sources (lamp posts, buildings with LightVisibility) add localized
//! brightness using linear falloff: `contribution = ((range - distance) / range) * intensity`.
//! This matches the original engine's point light calculation.
//!
//! ## Dependency rules
//! - Part of map/ — depends on rules/ini_parser for IniFile, map/entities for MapEntity.

use std::collections::HashMap;

use crate::map::entities::{EntityCategory, MapEntity};
use crate::map::map_file::MapCell;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;

/// Maximum combined lighting value per channel.
pub const TOTAL_AMBIENT_CAP: f32 = 2.0;

/// Binary light unit scale for Ambient/Red/Green/Blue INI values.
pub const AMBIENT_RGB_UNIT_SCALE: i32 = 100;

/// Binary light unit scale for Ground/Level INI values.
pub const GROUND_LEVEL_UNIT_SCALE: i32 = 250;

/// Binary light unit scale for point-light intensity/tint values.
pub const POINT_LIGHT_UNIT_SCALE: i32 = 1000;

/// Leptons per cell in RA2's coordinate system.
pub const LEPTONS_PER_CELL: i32 = 256;

const HALF_CELL_LEPTONS: i32 = LEPTONS_PER_CELL / 2;

/// Default white tint (no lighting effect).
pub const DEFAULT_TINT: [f32; 3] = [1.0, 1.0, 1.0];

/// Global lighting parameters from the map's [Lighting] INI section.
#[derive(Debug, Clone)]
pub struct LightingConfig {
    /// Base brightness level (default 1.0).
    pub ambient: f32,
    /// Red channel multiplier (default 1.0).
    pub red: f32,
    /// Green channel multiplier (default 1.0).
    pub green: f32,
    /// Blue channel multiplier (default 1.0).
    pub blue: f32,
    /// Ground-level darkening subtracted from ambient. Default 0.20.
    pub ground: f32,
    /// Height-based ambient boost per elevation level. Default 0.032.
    pub level: f32,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            ambient: 1.0,
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            ground: 0.20,
            level: 0.032,
        }
    }
}

/// Integer RGB identity used by the light profile cache.
pub type LightRgbKey = [i32; 3];

/// Stable id for a cached light profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LightProfileId(pub usize);

/// Cached RGB profile shared by cells with the same channel multipliers.
#[derive(Debug, Clone, PartialEq)]
pub struct LightProfile {
    pub id: LightProfileId,
    pub rgb_key: LightRgbKey,
    pub rgb: [f32; 3],
}

/// LightConvert-style profile identity cache for render-facing cell light.
#[derive(Debug, Clone)]
pub struct LightProfileCache {
    profiles: Vec<LightProfile>,
    by_key: HashMap<LightRgbKey, LightProfileId>,
}

impl LightProfileCache {
    pub fn new() -> Self {
        let mut cache = Self {
            profiles: Vec::new(),
            by_key: HashMap::new(),
        };
        cache.profile_id_for_rgb(DEFAULT_TINT);
        cache
    }

    pub fn default_profile_id(&self) -> LightProfileId {
        LightProfileId(0)
    }

    pub fn profile_id_for_rgb(&mut self, rgb: [f32; 3]) -> LightProfileId {
        let key = rgb_to_key(rgb);
        if let Some(id) = self.by_key.get(&key).copied() {
            return id;
        }
        let id = LightProfileId(self.profiles.len());
        self.profiles.push(LightProfile {
            id,
            rgb_key: key,
            rgb,
        });
        self.by_key.insert(key, id);
        id
    }

    pub fn get(&self, id: LightProfileId) -> Option<&LightProfile> {
        self.profiles.get(id.0)
    }

    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
}

impl Default for LightProfileCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-cell light state backed by a cached RGB profile.
#[derive(Debug, Clone, PartialEq)]
pub struct CellLight {
    pub profile_id: LightProfileId,
    pub top_scalar: f32,
    pub common_scalar: f32,
    pub bottom_scalar: f32,
    pub rgb_key: LightRgbKey,
}

impl CellLight {
    pub fn new(profile_id: LightProfileId, rgb_key: LightRgbKey, common_scalar: f32) -> Self {
        Self {
            profile_id,
            top_scalar: common_scalar,
            common_scalar,
            bottom_scalar: common_scalar,
            rgb_key,
        }
    }
}

/// Map-level cell lighting container with compatibility tint accessors.
#[derive(Debug, Clone)]
pub struct CellLightGrid {
    cells: HashMap<(u16, u16), CellLight>,
    profiles: LightProfileCache,
}

impl CellLightGrid {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            profiles: LightProfileCache::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cells: HashMap::with_capacity(capacity),
            profiles: LightProfileCache::new(),
        }
    }

    pub fn profiles(&self) -> &LightProfileCache {
        &self.profiles
    }

    pub fn insert_light(&mut self, cell: (u16, u16), light: CellLight) {
        self.cells.insert(cell, light);
    }

    pub fn insert_profiled_light(&mut self, cell: (u16, u16), rgb: [f32; 3], common_scalar: f32) {
        let profile_id = self.profiles.profile_id_for_rgb(rgb);
        let rgb_key = rgb_to_key(rgb);
        self.insert_light(cell, CellLight::new(profile_id, rgb_key, common_scalar));
    }

    pub fn set_compat_tint(&mut self, cell: (u16, u16), tint: [f32; 3]) {
        self.insert_profiled_light(cell, tint, 1.0);
    }

    pub fn cells(&self) -> impl Iterator<Item = ((u16, u16), &CellLight)> {
        self.cells.iter().map(|(cell, light)| (*cell, light))
    }

    pub fn cell_light_at(&self, cell: (u16, u16)) -> Option<&CellLight> {
        self.cells.get(&cell)
    }

    pub fn tint_at(&self, cell: (u16, u16)) -> Option<[f32; 3]> {
        let light = self.cells.get(&cell)?;
        Some(self.tint_for_light(light))
    }

    pub fn tint_or_default(&self, cell: (u16, u16)) -> [f32; 3] {
        self.tint_at(cell).unwrap_or(DEFAULT_TINT)
    }

    pub fn techno_tint_at(&self, cell: (u16, u16)) -> [f32; 3] {
        self.tint_or_default(cell)
    }

    pub fn unit_tint_at(&self, cell: (u16, u16)) -> [f32; 3] {
        self.techno_tint_at(cell)
    }

    pub fn infantry_tint_at(&self, cell: (u16, u16)) -> [f32; 3] {
        self.techno_tint_at(cell)
    }

    pub fn aircraft_tint_at(&self, cell: (u16, u16)) -> [f32; 3] {
        self.techno_tint_at(cell)
    }

    pub fn building_body_tint_at(&self, cell: (u16, u16)) -> [f32; 3] {
        self.techno_tint_at(cell)
    }

    pub fn overlay_tint_at(&self, cell: (u16, u16)) -> [f32; 3] {
        self.tint_or_default(cell)
    }

    pub fn terrain_object_tint_at(&self, cell: (u16, u16)) -> [f32; 3] {
        self.tint_or_default(cell)
    }

    pub fn anim_tint_at(&self, cell: (u16, u16)) -> [f32; 3] {
        self.tint_or_default(cell)
    }

    pub fn bridge_body_tint_at(&self, cell: (u16, u16)) -> [f32; 3] {
        self.tint_or_default(cell)
    }

    fn tint_for_light(&self, light: &CellLight) -> [f32; 3] {
        let profile = self
            .profiles
            .get(light.profile_id)
            .or_else(|| self.profiles.get(self.profiles.default_profile_id()))
            .expect("default light profile is always present");
        [
            profile.rgb[0] * light.common_scalar,
            profile.rgb[1] * light.common_scalar,
            profile.rgb[2] * light.common_scalar,
        ]
    }
}

impl Default for CellLightGrid {
    fn default() -> Self {
        Self::new()
    }
}

fn rgb_to_key(rgb: [f32; 3]) -> LightRgbKey {
    [
        (rgb[0] * POINT_LIGHT_UNIT_SCALE as f32).round() as i32,
        (rgb[1] * POINT_LIGHT_UNIT_SCALE as f32).round() as i32,
        (rgb[2] * POINT_LIGHT_UNIT_SCALE as f32).round() as i32,
    ]
}

/// Return signed building body draw-depth adjustment from art.ini ExtraLight.
pub fn building_body_depth_adjustment(extra_light: i32) -> i32 {
    extra_light
}

/// Parse [Lighting] section from a map INI file.
pub fn parse_lighting(ini: &IniFile) -> LightingConfig {
    let section = match ini.section("Lighting") {
        Some(s) => s,
        None => return LightingConfig::default(),
    };
    LightingConfig {
        ambient: section.get_f32("Ambient").unwrap_or(1.0),
        red: section.get_f32("Red").unwrap_or(1.0),
        green: section.get_f32("Green").unwrap_or(1.0),
        blue: section.get_f32("Blue").unwrap_or(1.0),
        ground: section.get_f32("Ground").unwrap_or(0.20),
        level: section.get_f32("Level").unwrap_or(0.032),
    }
}

/// Compute the RGB tint for a single cell given its elevation.
pub fn cell_tint(config: &LightingConfig, z: u8) -> [f32; 3] {
    let cell_ambient: f32 = cell_light_scalar(config, z);
    let mut r: f32 = config.red * cell_ambient;
    let mut g: f32 = config.green * cell_ambient;
    let mut b: f32 = config.blue * cell_ambient;

    // Cap: if any channel exceeds TOTAL_AMBIENT_CAP, scale all down proportionally.
    let max_val: f32 = r.max(g).max(b);
    if max_val > TOTAL_AMBIENT_CAP {
        let scale: f32 = TOTAL_AMBIENT_CAP / max_val;
        r *= scale;
        g *= scale;
        b *= scale;
    }

    [r, g, b]
}

/// Compute the shared ambient scalar for a terrain elevation level.
pub fn cell_light_scalar(config: &LightingConfig, z: u8) -> f32 {
    config.ambient + config.level * z as f32 - config.ground
}

/// Compute the uniform terrain tint for the map.
///
/// Terrain uses the ground-level lighting value across all cells so repeating
/// tile textures do not expose the map grid through per-cell tint boundaries.
pub fn terrain_tint(config: &LightingConfig) -> [f32; 3] {
    cell_tint(config, 0)
}

/// Build the profile-backed base cell light grid from map INI and cell data.
pub fn build_cell_light_grid(ini: &IniFile, cells: &[MapCell]) -> CellLightGrid {
    let config = parse_lighting(ini);
    build_cell_light_grid_from_heights(
        cells.iter().map(|cell| ((cell.rx, cell.ry), cell.z)),
        &config,
    )
}

/// Build the profile-backed base cell light grid from known cell heights.
pub fn build_cell_light_grid_from_heights<I>(heights: I, config: &LightingConfig) -> CellLightGrid
where
    I: IntoIterator<Item = ((u16, u16), u8)>,
{
    let mut grid = CellLightGrid::new();
    let rgb = [config.red, config.green, config.blue];
    for (cell, z) in heights {
        grid.insert_profiled_light(cell, rgb, cell_light_scalar(config, z));
    }
    grid
}

/// A point light source placed on the map (lamp post, lit building, etc.).
///
/// Created from map entity data during map load. Each light contributes
/// localized brightness to nearby cells using linear distance falloff.
#[derive(Debug, Clone)]
pub struct PointLight {
    /// Cell position of the light source.
    pub rx: u16,
    pub ry: u16,
    /// Light center X in leptons.
    pub center_x: i32,
    /// Light center Y in leptons.
    pub center_y: i32,
    /// Inclusive visibility radius in leptons.
    pub radius_leptons: i32,
    /// Brightness intensity in `1000 == 1.0` units. Can be negative.
    pub intensity: i32,
    /// RGB tint components in `1000 == 1.0` units.
    pub tint: [i32; 3],
    /// Whether the light source currently contributes.
    pub active: bool,
    /// Detail-level gate placeholder for ordinary light sources.
    pub detail: bool,
}

/// Collect point light sources from map-placed buildings with nonzero LightIntensity.
///
/// Iterates all structure entities on the map and checks their ObjectType
/// for light emission properties parsed from rules.ini.
pub fn collect_building_lights(entities: &[MapEntity], rules: Option<&RuleSet>) -> Vec<PointLight> {
    let Some(rules) = rules else {
        return Vec::new();
    };
    let mut lights = Vec::new();
    for ent in entities {
        if ent.category != EntityCategory::Structure {
            continue;
        }
        let Some(obj) = rules.object(&ent.type_id) else {
            continue;
        };
        if let Some(light) = point_light_from_object(
            ent.cell_x,
            ent.cell_y,
            obj.light_visibility,
            obj.light_intensity,
            [
                obj.light_red_tint,
                obj.light_green_tint,
                obj.light_blue_tint,
            ],
        ) {
            lights.push(light);
        }
    }
    lights
}

/// Build a point light from object light fields at a cell origin.
pub fn point_light_from_object(
    rx: u16,
    ry: u16,
    visibility_leptons: i32,
    intensity: f32,
    tint: [f32; 3],
) -> Option<PointLight> {
    let intensity = light_value_to_units(intensity);
    if intensity == 0 {
        return None;
    }
    let radius_leptons = visibility_leptons.max(0);
    if radius_leptons == 0 {
        return None;
    }
    Some(PointLight {
        rx,
        ry,
        center_x: i32::from(rx) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
        center_y: i32::from(ry) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
        radius_leptons,
        intensity,
        tint: [
            light_value_to_units(tint[0]),
            light_value_to_units(tint[1]),
            light_value_to_units(tint[2]),
        ],
        active: true,
        detail: true,
    })
}

/// Accumulate point light contributions into an existing CellLightGrid.
///
/// Contributions are signed and summed before one final clamp per channel.
pub fn accumulate_point_lights(grid: &mut CellLightGrid, lights: &[PointLight]) {
    if lights.is_empty() {
        return;
    }
    let cells: Vec<((u16, u16), [f32; 3])> = grid
        .cells()
        .map(|(cell, _)| (cell, grid.tint_or_default(cell)))
        .collect();
    for (cell, base_tint) in cells {
        let cell_center_x = i32::from(cell.0) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS;
        let cell_center_y = i32::from(cell.1) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS;
        let mut sums = [
            light_float_to_units(base_tint[0]),
            light_float_to_units(base_tint[1]),
            light_float_to_units(base_tint[2]),
        ];
        for light in lights {
            if !light.active || !light.detail || light.radius_leptons <= 0 {
                continue;
            }
            let dx = i64::from(cell_center_x - light.center_x);
            let dy = i64::from(cell_center_y - light.center_y);
            let distance_sq = dx * dx + dy * dy;
            let radius = i64::from(light.radius_leptons);
            if distance_sq > radius * radius {
                continue;
            }
            let distance = integer_sqrt(distance_sq);
            let falloff = radius - distance;
            for (channel, sum) in sums.iter_mut().enumerate() {
                let numerator =
                    falloff * i64::from(light.intensity) * i64::from(light.tint[channel]);
                let contribution = numerator / radius / i64::from(POINT_LIGHT_UNIT_SCALE);
                *sum += contribution as i32;
            }
        }
        let cap = light_float_to_units(TOTAL_AMBIENT_CAP);
        grid.set_compat_tint(
            cell,
            [
                sums[0].clamp(0, cap) as f32 / POINT_LIGHT_UNIT_SCALE as f32,
                sums[1].clamp(0, cap) as f32 / POINT_LIGHT_UNIT_SCALE as f32,
                sums[2].clamp(0, cap) as f32 / POINT_LIGHT_UNIT_SCALE as f32,
            ],
        );
    }
}

/// Convert INI light values using the verified `value * 1000 + 0.1` shape.
pub fn light_value_to_units(value: f32) -> i32 {
    (value * POINT_LIGHT_UNIT_SCALE as f32 + 0.1) as i32
}

fn light_float_to_units(value: f32) -> i32 {
    (value * POINT_LIGHT_UNIT_SCALE as f32).round() as i32
}

fn integer_sqrt(value: i64) -> i64 {
    (value as f64).sqrt() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_lighting_uses_yr_ground_subtraction() {
        let config: LightingConfig = LightingConfig::default();
        assert!((config.ground - 0.20).abs() < 0.001);
        let tint: [f32; 3] = cell_tint(&config, 0);
        assert!((tint[0] - 0.8).abs() < 0.001);
        assert!((tint[1] - 0.8).abs() < 0.001);
        assert!((tint[2] - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_parse_lighting_uses_yr_defaults() {
        let ini = IniFile::from_str("[Lighting]\n");
        let config = parse_lighting(&ini);
        assert!((config.ambient - 1.0).abs() < 0.001);
        assert!((config.red - 1.0).abs() < 0.001);
        assert!((config.green - 1.0).abs() < 0.001);
        assert!((config.blue - 1.0).abs() < 0.001);
        assert!((config.ground - 0.20).abs() < 0.001);
        assert!((config.level - 0.032).abs() < 0.001);
    }

    #[test]
    fn test_elevation_boost() {
        let config: LightingConfig = LightingConfig::default();
        let tint_z0: [f32; 3] = cell_tint(&config, 0);
        let tint_z4: [f32; 3] = cell_tint(&config, 4);
        // 1.0 + Level(0.032) * z(4) - Ground(0.20) = 0.928
        assert!(tint_z4[0] > tint_z0[0]);
        assert!((tint_z4[0] - 0.928).abs() < 0.01);
    }

    #[test]
    fn test_dark_map() {
        let config: LightingConfig = LightingConfig {
            ambient: 0.5,
            red: 1.0,
            green: 0.8,
            blue: 0.6,
            ground: 0.0,
            level: 0.0,
        };
        let tint: [f32; 3] = cell_tint(&config, 0);
        assert!((tint[0] - 0.5).abs() < 0.001);
        assert!((tint[1] - 0.4).abs() < 0.001);
        assert!((tint[2] - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_cap_at_two() {
        let config: LightingConfig = LightingConfig {
            ambient: 3.0,
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            ground: 0.0,
            level: 0.0,
        };
        let tint: [f32; 3] = cell_tint(&config, 0);
        // 3.0 > 2.0 cap → scaled to 2.0
        assert!((tint[0] - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_ground_darkening() {
        let config: LightingConfig = LightingConfig {
            ambient: 1.0,
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            ground: 0.5,
            level: 0.0,
        };
        let tint: [f32; 3] = cell_tint(&config, 0);
        // ambient - ground = 0.5
        assert!((tint[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_terrain_tint_matches_ground_level_cell_tint() {
        let config: LightingConfig = LightingConfig {
            ambient: 1.0,
            red: 1.0,
            green: 0.88,
            blue: 0.88,
            ground: 0.0,
            level: 0.039,
        };
        let terrain: [f32; 3] = terrain_tint(&config);
        let ground_cell: [f32; 3] = cell_tint(&config, 0);
        assert_eq!(terrain, ground_cell);
    }

    #[test]
    fn test_light_profile_cache_reuses_identical_rgb() {
        let mut cache = LightProfileCache::new();
        let neutral = cache.default_profile_id();
        let first = cache.profile_id_for_rgb([1.0, 1.0, 1.0]);
        let second = cache.profile_id_for_rgb([1.0, 1.0, 1.0]);

        assert_eq!(neutral, first);
        assert_eq!(first, second);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cell_light_grid_default_cell_uses_neutral_tint() {
        let grid = CellLightGrid::new();
        assert_eq!(grid.tint_or_default((99, 99)), DEFAULT_TINT);
        assert_eq!(grid.profiles().default_profile_id(), LightProfileId(0));
    }

    #[test]
    fn test_cell_light_grid_compatibility_tint() {
        let mut grid = CellLightGrid::new();
        grid.insert_profiled_light((1, 2), [0.8, 0.7, 0.6], 0.5);

        let tint = grid.tint_or_default((1, 2));
        assert!((tint[0] - 0.4).abs() < 0.001);
        assert!((tint[1] - 0.35).abs() < 0.001);
        assert!((tint[2] - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_consumer_accessors_return_compatibility_tint() {
        let mut grid = CellLightGrid::new();
        grid.set_compat_tint((4, 5), [0.7, 0.8, 0.9]);

        assert_eq!(grid.techno_tint_at((4, 5)), [0.7, 0.8, 0.9]);
        assert_eq!(grid.unit_tint_at((4, 5)), [0.7, 0.8, 0.9]);
        assert_eq!(grid.infantry_tint_at((4, 5)), [0.7, 0.8, 0.9]);
        assert_eq!(grid.aircraft_tint_at((4, 5)), [0.7, 0.8, 0.9]);
        assert_eq!(grid.overlay_tint_at((4, 5)), [0.7, 0.8, 0.9]);
        assert_eq!(grid.terrain_object_tint_at((4, 5)), [0.7, 0.8, 0.9]);
        assert_eq!(grid.anim_tint_at((4, 5)), [0.7, 0.8, 0.9]);
        assert_eq!(grid.bridge_body_tint_at((4, 5)), [0.7, 0.8, 0.9]);
    }

    #[test]
    fn test_building_body_depth_adjustment_keeps_signed_extra_light() {
        assert_eq!(building_body_depth_adjustment(350), 350);
        assert_eq!(building_body_depth_adjustment(-100), -100);
    }

    #[test]
    fn test_base_cell_light_grid_matches_cell_tint() {
        let config = LightingConfig {
            ambient: 1.0,
            red: 0.8,
            green: 0.9,
            blue: 1.0,
            ground: 0.20,
            level: 0.032,
        };
        let grid = build_cell_light_grid_from_heights([((10, 10), 0), ((10, 11), 4)], &config);

        for (cell, z) in [((10, 10), 0), ((10, 11), 4)] {
            let expected = cell_tint(&config, z);
            let actual = grid.tint_or_default(cell);
            assert!((actual[0] - expected[0]).abs() < 0.001);
            assert!((actual[1] - expected[1]).abs() < 0.001);
            assert!((actual[2] - expected[2]).abs() < 0.001);
        }
    }

    #[test]
    fn test_base_cell_light_grid_reuses_neutral_profile() {
        let config = LightingConfig::default();
        let grid = build_cell_light_grid_from_heights([((0, 0), 0), ((1, 0), 3)], &config);

        assert_eq!(grid.profiles().len(), 1);
        assert_eq!(
            grid.cell_light_at((0, 0)).map(|light| light.profile_id),
            Some(LightProfileId(0))
        );
        assert_eq!(
            grid.cell_light_at((1, 0)).map(|light| light.profile_id),
            Some(LightProfileId(0))
        );
    }

    /// Helper: build a small grid with uniform tint for testing point lights.
    fn test_grid(size: u16, base_tint: [f32; 3]) -> CellLightGrid {
        let mut grid = CellLightGrid::with_capacity(usize::from(size) * usize::from(size));
        for y in 0..size {
            for x in 0..size {
                grid.set_compat_tint((x, y), base_tint);
            }
        }
        grid
    }

    fn test_point_light(
        rx: u16,
        ry: u16,
        radius_leptons: i32,
        intensity: f32,
        tint: [f32; 3],
    ) -> PointLight {
        PointLight {
            rx,
            ry,
            center_x: i32::from(rx) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
            center_y: i32::from(ry) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
            radius_leptons,
            intensity: light_value_to_units(intensity),
            tint: [
                light_value_to_units(tint[0]),
                light_value_to_units(tint[1]),
                light_value_to_units(tint[2]),
            ],
            active: true,
            detail: true,
        }
    }

    #[test]
    fn test_point_light_linear_falloff() {
        let mut grid = test_grid(20, [0.5, 0.5, 0.5]);
        let light = test_point_light(10, 10, 5 * LEPTONS_PER_CELL, 1.0, [1.0, 1.0, 1.0]);
        accumulate_point_lights(&mut grid, &[light]);

        // Center cell gets full intensity: 0.5 + 1.0 = 1.5
        let center = grid.tint_or_default((10, 10));
        assert!((center[0] - 1.5).abs() < 0.01, "center={:.3}", center[0]);

        // Cell at distance 2.5 gets (5-2.5)/5 = 0.5 intensity: 0.5 + 0.5 = 1.0
        // (2, 1.5 diagonal ≈ 2.5 cells — use (12, 10) which is distance 2.0)
        let d2 = grid.tint_or_default((12, 10));
        let expected_d2 = 0.5 + (5.0 - 2.0) / 5.0;
        assert!(
            (d2[0] - expected_d2).abs() < 0.01,
            "d2={:.3} expected={:.3}",
            d2[0],
            expected_d2
        );

        // Cell at distance 5+ is unchanged: still 0.5
        let far = grid.tint_or_default((15, 10));
        assert!((far[0] - 0.5).abs() < 0.01, "far={:.3}", far[0]);
    }

    #[test]
    fn test_point_light_boundary_is_inclusive_with_zero_edge_contribution() {
        let mut grid = test_grid(8, [0.5, 0.5, 0.5]);
        let light = test_point_light(2, 2, 2 * LEPTONS_PER_CELL, 1.0, [1.0, 1.0, 1.0]);
        accumulate_point_lights(&mut grid, &[light]);

        let edge = grid.tint_or_default((4, 2));
        assert_eq!(edge, [0.5, 0.5, 0.5]);
        let inside = grid.tint_or_default((3, 2));
        assert!((inside[0] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_negative_point_light_darkens_cell() {
        let mut grid = test_grid(5, [0.8, 0.8, 0.8]);
        let light = test_point_light(2, 2, 2 * LEPTONS_PER_CELL, -0.2, [1.0, 1.0, 1.0]);
        accumulate_point_lights(&mut grid, &[light]);

        let center = grid.tint_or_default((2, 2));
        assert!((center[0] - 0.6).abs() < 0.001);
        assert!((center[1] - 0.6).abs() < 0.001);
        assert!((center[2] - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_point_lights_sum_before_clamp() {
        let mut grid = test_grid(5, [1.9, 1.9, 1.9]);
        let brighten = test_point_light(2, 2, 2 * LEPTONS_PER_CELL, 0.3, [1.0, 1.0, 1.0]);
        let darken = test_point_light(2, 2, 2 * LEPTONS_PER_CELL, -0.3002, [1.0, 1.0, 1.0]);
        accumulate_point_lights(&mut grid, &[brighten, darken]);

        let center = grid.tint_or_default((2, 2));
        assert!((center[0] - 1.9).abs() < 0.001);
    }

    #[test]
    fn test_point_light_division_truncates_toward_zero() {
        let mut grid = test_grid(1, [0.0, 0.0, 0.0]);
        let mut light = test_point_light(0, 0, 3, 1.0, [1.0, 1.0, 1.0]);
        light.center_x -= 1;
        accumulate_point_lights(&mut grid, &[light]);

        let center = grid.tint_or_default((0, 0));
        assert!((center[0] - 0.666).abs() < 0.001);
    }

    #[test]
    fn test_point_light_colored_tint() {
        let mut grid = test_grid(10, [0.3, 0.3, 0.3]);
        let light = test_point_light(5, 5, 3 * LEPTONS_PER_CELL, 0.6, [1.0, 0.0, 0.5]);
        accumulate_point_lights(&mut grid, &[light]);

        let center = grid.tint_or_default((5, 5));
        // Red: 0.3 + 0.6*1.0 = 0.9
        assert!((center[0] - 0.9).abs() < 0.01, "r={:.3}", center[0]);
        // Green: 0.3 + 0.6*0.0 = 0.3 (unchanged)
        assert!((center[1] - 0.3).abs() < 0.01, "g={:.3}", center[1]);
        // Blue: 0.3 + 0.6*0.5 = 0.6
        assert!((center[2] - 0.6).abs() < 0.01, "b={:.3}", center[2]);
    }

    #[test]
    fn test_accumulation_clamps_at_cap() {
        let mut grid = test_grid(5, [1.8, 1.8, 1.8]);
        let light = test_point_light(2, 2, 3 * LEPTONS_PER_CELL, 1.0, [1.0, 1.0, 1.0]);
        accumulate_point_lights(&mut grid, &[light]);

        let center = grid.tint_or_default((2, 2));
        // 1.8 + 1.0 = 2.8 → clamped to 2.0
        assert!((center[0] - TOTAL_AMBIENT_CAP).abs() < 0.001);
    }

    #[test]
    fn test_light_value_to_units_uses_verified_bias() {
        assert_eq!(light_value_to_units(0.0), 0);
        assert_eq!(light_value_to_units(0.01), 10);
        assert_eq!(light_value_to_units(0.5), 500);
        assert_eq!(light_value_to_units(-0.5), -499);
    }

    #[test]
    fn test_collect_building_lights_uses_intensity_gate_and_default_radius() {
        let ini = IniFile::from_str(
            "[BuildingTypes]\n1=LAMP\n2=DARK\n3=ZERO\n\
             [LAMP]\nLightIntensity=0.5\n\
             [DARK]\nLightVisibility=2048\nLightIntensity=-0.25\n\
             [ZERO]\nLightVisibility=4096\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("rules");
        let entities = vec![
            test_map_structure("LAMP", 3, 4),
            test_map_structure("DARK", 5, 6),
            test_map_structure("ZERO", 7, 8),
        ];

        let lights = collect_building_lights(&entities, Some(&rules));
        assert_eq!(lights.len(), 2);
        assert_eq!(lights[0].radius_leptons, 5000);
        assert_eq!(lights[0].intensity, 500);
        assert_eq!(lights[0].center_x, 3 * LEPTONS_PER_CELL + HALF_CELL_LEPTONS);
        assert_eq!(lights[1].radius_leptons, 2048);
        assert_eq!(lights[1].intensity, -249);
    }

    fn test_map_structure(type_id: &str, cell_x: u16, cell_y: u16) -> MapEntity {
        MapEntity {
            owner: "Neutral".to_string(),
            type_id: type_id.to_string(),
            health: 256,
            cell_x,
            cell_y,
            facing: 0,
            category: EntityCategory::Structure,
            sub_cell: 0,
            veterancy: 0,
            high: false,
        }
    }

    #[test]
    fn test_extra_light_is_not_rgb_cell_light() {
        let grid = test_grid(5, [0.4, 0.4, 0.4]);
        let key = (2u16, 2u16);
        // art.ini ExtraLight affects building body depth, not map RGB tint.
        let result = grid.tint_or_default(key);
        assert_eq!(result, [0.4, 0.4, 0.4]);
    }
}
