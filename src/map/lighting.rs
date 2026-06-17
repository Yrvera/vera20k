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

/// Maximum combined lighting value per channel in current compatibility tint output.
pub const TOTAL_AMBIENT_CAP: f32 = 2.0;

/// Internal light unit scale. `1000 == 1.0`.
pub const LIGHT_UNIT: i32 = 1000;

/// Minimum stored scalar light value.
pub const LIGHT_CLAMP_MIN: i32 = 0;

/// Maximum stored scalar/RGB light value.
pub const LIGHT_CLAMP_MAX: i32 = 2000;

/// Identity 16.16 scale used by the LightConvert normalization path.
pub const LIGHT_SCALE16_IDENTITY: i32 = 0x10000;

/// Binary light unit scale for Ambient/Red/Green/Blue INI values parsed as Rust ratios.
pub const AMBIENT_RGB_UNIT_SCALE: i32 = LIGHT_UNIT;

/// Binary light unit scale for Ground/Level INI values.
pub const GROUND_LEVEL_UNIT_SCALE: i32 = 250;

/// Binary light unit scale for point-light intensity/tint values.
pub const POINT_LIGHT_UNIT_SCALE: i32 = LIGHT_UNIT;

/// Leptons per cell in RA2's coordinate system.
pub const LEPTONS_PER_CELL: i32 = 256;

const HALF_CELL_LEPTONS: i32 = LEPTONS_PER_CELL / 2;
const BOTTOM_LEVEL_OFFSET: i32 = 4;
const DETAIL_HIGH_RGB_MASK: i32 = !31;

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
        self.profile_id_for_key(key)
    }

    pub fn profile_id_for_key(&mut self, key: LightRgbKey) -> LightProfileId {
        if let Some(id) = self.by_key.get(&key).copied() {
            return id;
        }
        let id = LightProfileId(self.profiles.len());
        self.profiles.push(LightProfile {
            id,
            rgb_key: key,
            rgb: key_to_rgb(key),
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
    /// Quantized normalized RGB key used for the profile/cache bridge.
    pub rgb_key: LightRgbKey,
    /// Raw RGB accumulators before normalization.
    pub raw_rgb: LightRgbKey,
    /// 16.16 scale computed by the verified RGB normalization helper.
    pub scale16: i32,
    /// Raw additive source intensity before RGB normalization.
    pub raw_additive_intensity: i32,
    /// Additive source intensity after RGB normalization.
    pub additive_intensity: i32,
    /// Raw top/common scalar before final clamp.
    pub raw_top_scalar: i32,
    /// Raw bottom scalar before `scale16` multiplication and final clamp.
    pub raw_bottom_scalar: i32,
    pub top_scalar: i32,
    pub common_scalar: i32,
    pub bottom_scalar: i32,
}

impl CellLight {
    pub fn new(
        profile_id: LightProfileId,
        rgb_key: LightRgbKey,
        raw_rgb: LightRgbKey,
        scale16: i32,
        raw_additive_intensity: i32,
        additive_intensity: i32,
        raw_top_scalar: i32,
        raw_bottom_scalar: i32,
        top_scalar: i32,
        common_scalar: i32,
        bottom_scalar: i32,
    ) -> Self {
        Self {
            profile_id,
            rgb_key,
            raw_rgb,
            scale16,
            raw_additive_intensity,
            additive_intensity,
            raw_top_scalar,
            raw_bottom_scalar,
            top_scalar,
            common_scalar,
            bottom_scalar,
        }
    }

    pub fn compatibility(
        profile_id: LightProfileId,
        rgb_key: LightRgbKey,
        common_scalar: i32,
    ) -> Self {
        let scalar = common_scalar.clamp(LIGHT_CLAMP_MIN, LIGHT_CLAMP_MAX);
        Self::new(
            profile_id,
            rgb_key,
            rgb_key,
            LIGHT_SCALE16_IDENTITY,
            0,
            0,
            common_scalar,
            common_scalar,
            scalar,
            scalar,
            scalar,
        )
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
        self.insert_light(
            cell,
            CellLight::compatibility(profile_id, rgb_key, light_float_to_units(common_scalar)),
        );
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
        self.tint_for_common_scalar_or_default(cell)
    }

    pub fn terrain_object_tint_for_type(
        &self,
        cell: (u16, u16),
        spawns_tiberium: bool,
    ) -> [f32; 3] {
        if spawns_tiberium {
            self.tint_for_top_scalar_or_default(cell)
        } else {
            self.tint_for_common_scalar_or_default(cell)
        }
    }

    pub fn terrain_tile_tint_at(&self, cell: (u16, u16)) -> [f32; 3] {
        self.tint_for_common_scalar_or_default(cell)
    }

    pub fn anim_tint_at(&self, cell: (u16, u16)) -> [f32; 3] {
        self.tint_or_default(cell)
    }

    pub fn bridge_body_tint_at(&self, cell: (u16, u16)) -> [f32; 3] {
        self.tint_or_default(cell)
    }

    fn tint_for_light(&self, light: &CellLight) -> [f32; 3] {
        self.tint_for_light_scalar(light, light.common_scalar)
    }

    fn tint_for_top_scalar_or_default(&self, cell: (u16, u16)) -> [f32; 3] {
        let Some(light) = self.cells.get(&cell) else {
            return DEFAULT_TINT;
        };
        self.tint_for_light_scalar(light, light.top_scalar)
    }

    fn tint_for_common_scalar_or_default(&self, cell: (u16, u16)) -> [f32; 3] {
        let Some(light) = self.cells.get(&cell) else {
            return DEFAULT_TINT;
        };
        self.tint_for_light_scalar(light, light.common_scalar)
    }

    fn tint_for_light_scalar(&self, light: &CellLight, scalar: i32) -> [f32; 3] {
        let profile = self
            .profiles
            .get(light.profile_id)
            .or_else(|| self.profiles.get(self.profiles.default_profile_id()))
            .expect("default light profile is always present");
        let scalar = scalar as f32 / LIGHT_UNIT as f32;
        [
            profile.rgb[0] * scalar,
            profile.rgb[1] * scalar,
            profile.rgb[2] * scalar,
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
        (rgb[0] * LIGHT_UNIT as f32).round() as i32,
        (rgb[1] * LIGHT_UNIT as f32).round() as i32,
        (rgb[2] * LIGHT_UNIT as f32).round() as i32,
    ]
}

fn key_to_rgb(key: LightRgbKey) -> [f32; 3] {
    [
        key[0] as f32 / LIGHT_UNIT as f32,
        key[1] as f32 / LIGHT_UNIT as f32,
        key[2] as f32 / LIGHT_UNIT as f32,
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
    let grid = build_cell_light_grid_from_heights([((1, 1), z)], config);
    grid.tint_or_default((1, 1))
}

/// Compute the shared ambient scalar for a terrain elevation level.
pub fn cell_light_scalar(config: &LightingConfig, z: u8) -> f32 {
    let units = scenario_units(config);
    let scalar = units.ambient + units.level * i32::from(z) - units.ground;
    clamp_light_scalar(scalar) as f32 / LIGHT_UNIT as f32
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
    let units = scenario_units(config);
    for (cell, z) in heights {
        if cell == (0, 0) {
            let light = neutral_cell_light(&mut grid.profiles);
            grid.insert_light(cell, light);
            continue;
        }
        let raw_top = units.ambient + units.level * i32::from(z) - units.ground;
        let raw_bottom =
            units.ambient + units.level * (i32::from(z) + BOTTOM_LEVEL_OFFSET) - units.ground;
        let light = build_cell_light_from_raw(
            &mut grid.profiles,
            [units.red, units.green, units.blue],
            0,
            raw_top,
            raw_bottom,
        );
        grid.insert_light(cell, light);
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

/// Build a radiation-glow point light at a cell center.
///
/// `intensity` and `tint` are already in `1000 == 1.0` units (the radiation
/// light math is done by the caller). Detail is forced on: radiation is never
/// culled by the detail level, unlike ordinary lamps.
pub fn radiation_point_light(
    rx: u16,
    ry: u16,
    radius_leptons: i32,
    intensity: i32,
    tint: [i32; 3],
) -> PointLight {
    PointLight {
        rx,
        ry,
        center_x: i32::from(rx) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
        center_y: i32::from(ry) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS,
        radius_leptons: radius_leptons.max(0),
        intensity,
        tint,
        active: true,
        detail: true,
    }
}

/// Accumulate point light contributions into an existing CellLightGrid.
///
/// Contributions are signed and summed before one final clamp per channel.
pub fn accumulate_point_lights(grid: &mut CellLightGrid, lights: &[PointLight]) {
    if lights.is_empty() {
        return;
    }
    let cells: Vec<((u16, u16), CellLight)> = grid
        .cells()
        .map(|(cell, light)| (cell, light.clone()))
        .collect();
    for (cell, base_light) in cells {
        let cell_center_x = i32::from(cell.0) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS;
        let cell_center_y = i32::from(cell.1) * LEPTONS_PER_CELL + HALF_CELL_LEPTONS;
        let mut raw_rgb = base_light.raw_rgb;
        let mut raw_additive = base_light.raw_additive_intensity;
        let mut raw_top = base_light.raw_top_scalar;
        let mut raw_bottom = base_light.raw_bottom_scalar;
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
            let factor = (falloff * i64::from(LIGHT_UNIT)) / radius;
            let intensity = signed_div_1000(i64::from(light.intensity) * factor);
            raw_additive += intensity;
            raw_top += intensity;
            raw_bottom += intensity;
            for (channel, raw) in raw_rgb.iter_mut().enumerate() {
                *raw += signed_div_1000(i64::from(light.tint[channel]) * factor);
            }
        }
        let updated = build_cell_light_from_raw(
            &mut grid.profiles,
            raw_rgb,
            raw_additive,
            raw_top,
            raw_bottom,
        );
        grid.insert_light(cell, updated);
    }
}

/// Convert INI light values using the verified `value * 1000 + 0.1` shape.
pub fn light_value_to_units(value: f32) -> i32 {
    (value * POINT_LIGHT_UNIT_SCALE as f32 + 0.1) as i32
}

fn light_float_to_units(value: f32) -> i32 {
    (value * LIGHT_UNIT as f32).round() as i32
}

fn integer_sqrt(value: i64) -> i64 {
    (value as f64).sqrt() as i64
}

#[derive(Debug, Clone, Copy)]
struct ScenarioLightUnits {
    ambient: i32,
    red: i32,
    green: i32,
    blue: i32,
    ground: i32,
    level: i32,
}

#[derive(Debug, Clone, Copy)]
struct NormalizedLight {
    scale16: i32,
    additive_intensity: i32,
    rgb_key: LightRgbKey,
}

fn scenario_units(config: &LightingConfig) -> ScenarioLightUnits {
    ScenarioLightUnits {
        ambient: light_float_to_units(config.ambient),
        red: light_float_to_units(config.red),
        green: light_float_to_units(config.green),
        blue: light_float_to_units(config.blue),
        ground: light_ground_level_to_units(config.ground),
        level: light_ground_level_to_units(config.level),
    }
}

fn light_ground_level_to_units(value: f32) -> i32 {
    (value * GROUND_LEVEL_UNIT_SCALE as f32 + 0.1) as i32
}

fn build_cell_light_from_raw(
    profiles: &mut LightProfileCache,
    raw_rgb: LightRgbKey,
    raw_additive_intensity: i32,
    raw_top_scalar: i32,
    raw_bottom_scalar: i32,
) -> CellLight {
    let normalized = normalize_light(raw_rgb, raw_additive_intensity);
    let top_high = raw_top_scalar.min(LIGHT_CLAMP_MAX);
    let common_high = top_high;
    let scaled_bottom = ((i64::from(raw_bottom_scalar) * i64::from(normalized.scale16)) >> 16)
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    let bottom_high = scaled_bottom.min(LIGHT_CLAMP_MAX);
    let top_scalar = clamp_light_scalar(top_high);
    let common_scalar = clamp_light_scalar(common_high);
    let bottom_scalar = clamp_light_scalar(bottom_high);
    let profile_id = profiles.profile_id_for_key(normalized.rgb_key);
    CellLight::new(
        profile_id,
        normalized.rgb_key,
        raw_rgb,
        normalized.scale16,
        raw_additive_intensity,
        normalized.additive_intensity,
        raw_top_scalar,
        raw_bottom_scalar,
        top_scalar,
        common_scalar,
        bottom_scalar,
    )
}

fn neutral_cell_light(profiles: &mut LightProfileCache) -> CellLight {
    let rgb_key = [LIGHT_UNIT, LIGHT_UNIT, LIGHT_UNIT];
    let profile_id = profiles.profile_id_for_key(rgb_key);
    CellLight::new(
        profile_id,
        rgb_key,
        rgb_key,
        LIGHT_SCALE16_IDENTITY,
        0,
        0,
        LIGHT_UNIT,
        LIGHT_UNIT,
        LIGHT_UNIT,
        LIGHT_UNIT,
        LIGHT_UNIT,
    )
}

fn normalize_light(raw_rgb: LightRgbKey, additive_intensity: i32) -> NormalizedLight {
    let mut rgb = [
        raw_rgb[0].clamp(0, LIGHT_CLAMP_MAX),
        raw_rgb[1].clamp(0, LIGHT_CLAMP_MAX),
        raw_rgb[2].clamp(0, LIGHT_CLAMP_MAX),
    ];
    let mut additive = additive_intensity;
    let mut scale16 = LIGHT_SCALE16_IDENTITY;

    if rgb != [LIGHT_UNIT, LIGHT_UNIT, LIGHT_UNIT] {
        let max_channel = rgb[0].max(rgb[1]).max(rgb[2]);
        scale16 = ((i64::from(max_channel) * i64::from(LIGHT_SCALE16_IDENTITY))
            / i64::from(LIGHT_UNIT)) as i32;
        if scale16 < 66 {
            scale16 = LIGHT_SCALE16_IDENTITY;
            rgb = [LIGHT_UNIT, LIGHT_UNIT, LIGHT_UNIT];
            additive = 0;
        } else {
            if rgb[0] >= rgb[1] && rgb[0] >= rgb[2] {
                let max = rgb[0].max(1);
                rgb[1] = normalize_channel(rgb[1], max);
                rgb[2] = normalize_channel(rgb[2], max);
                rgb[0] = LIGHT_UNIT;
            } else if rgb[1] >= rgb[2] {
                let scale = scale16.max(1);
                rgb[0] = normalize_channel_by_scale(rgb[0], scale);
                rgb[2] = normalize_channel_by_scale(rgb[2], scale);
                rgb[1] = LIGHT_UNIT;
            } else {
                let scale = scale16.max(1);
                rgb[0] = normalize_channel_by_scale(rgb[0], scale);
                rgb[1] = normalize_channel_by_scale(rgb[1], scale);
                rgb[2] = LIGHT_UNIT;
            }
            additive = ((i64::from(scale16) * i64::from(additive)) >> 16)
                .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        }
    }
    if additive > LIGHT_CLAMP_MAX {
        additive = LIGHT_CLAMP_MAX;
    }

    NormalizedLight {
        scale16,
        additive_intensity: additive,
        rgb_key: quantize_rgb_key(rgb),
    }
}

fn normalize_channel(channel: i32, max_channel: i32) -> i32 {
    ((i64::from(channel) * i64::from(LIGHT_UNIT)) / i64::from(max_channel)) as i32
}

fn normalize_channel_by_scale(channel: i32, scale16: i32) -> i32 {
    ((i64::from(channel) * i64::from(LIGHT_SCALE16_IDENTITY)) / i64::from(scale16)) as i32
}

fn quantize_rgb_key(rgb: LightRgbKey) -> LightRgbKey {
    if rgb == [LIGHT_UNIT, LIGHT_UNIT, LIGHT_UNIT] {
        return rgb;
    }
    [
        (rgb[0].clamp(0, LIGHT_UNIT)) & DETAIL_HIGH_RGB_MASK,
        (rgb[1].clamp(0, LIGHT_UNIT)) & DETAIL_HIGH_RGB_MASK,
        (rgb[2].clamp(0, LIGHT_UNIT)) & DETAIL_HIGH_RGB_MASK,
    ]
}

fn clamp_light_scalar(value: i32) -> i32 {
    value.clamp(LIGHT_CLAMP_MIN, LIGHT_CLAMP_MAX)
}

fn signed_div_1000(value: i64) -> i32 {
    (value / i64::from(LIGHT_UNIT)) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_lighting_uses_yr_ground_subtraction() {
        let config: LightingConfig = LightingConfig::default();
        assert!((config.ground - 0.20).abs() < 0.001);
        let tint: [f32; 3] = cell_tint(&config, 0);
        assert!((tint[0] - 0.95).abs() < 0.001);
        assert!((tint[1] - 0.95).abs() < 0.001);
        assert!((tint[2] - 0.95).abs() < 0.001);
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
    fn default_flat_no_lamps_ground_scalar_is_950() {
        let grid = build_cell_light_grid_from_heights([((3, 4), 0)], &LightingConfig::default());
        let light = grid.cell_light_at((3, 4)).expect("light");

        assert_eq!(light.common_scalar, 950);
        assert_eq!(light.top_scalar, 950);
        assert_eq!(light.bottom_scalar, 982);
        assert_eq!(grid.terrain_tile_tint_at((3, 4)), [0.95, 0.95, 0.95]);
    }

    #[test]
    fn sentinel_cell_zero_zero_uses_neutral_light() {
        let grid = build_cell_light_grid_from_heights([((0, 0), 0)], &LightingConfig::default());
        let light = grid.cell_light_at((0, 0)).expect("light");

        assert_eq!(light.scale16, LIGHT_SCALE16_IDENTITY);
        assert_eq!(light.additive_intensity, 0);
        assert_eq!(light.rgb_key, [LIGHT_UNIT, LIGHT_UNIT, LIGHT_UNIT]);
        assert_eq!(light.common_scalar, LIGHT_UNIT);
        assert_eq!(light.top_scalar, LIGHT_UNIT);
        assert_eq!(light.bottom_scalar, LIGHT_UNIT);
        assert_eq!(grid.tint_or_default((0, 0)), DEFAULT_TINT);
    }

    #[test]
    fn default_raised_no_lamps_common_and_bottom_scalars_match_gamemd() {
        let grid = build_cell_light_grid_from_heights([((10, 11), 4)], &LightingConfig::default());
        let light = grid.cell_light_at((10, 11)).expect("light");

        assert_eq!(light.common_scalar, 982);
        assert_eq!(light.top_scalar, 982);
        assert_eq!(light.bottom_scalar, 1014);
    }

    #[test]
    fn test_elevation_boost() {
        let config: LightingConfig = LightingConfig::default();
        let tint_z0: [f32; 3] = cell_tint(&config, 0);
        let tint_z4: [f32; 3] = cell_tint(&config, 4);
        // Default YR units: 1000 + Level(8) * z(4) - Ground(50) = 982.
        assert!(tint_z4[0] > tint_z0[0]);
        assert!((tint_z4[0] - 0.982).abs() < 0.01);
        let grid = build_cell_light_grid_from_heights([((10, 11), 4)], &config);
        let light = grid.cell_light_at((10, 11)).expect("light");
        assert_eq!(light.common_scalar, 982);
        assert_eq!(light.bottom_scalar, 1014);
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
        let grid = build_cell_light_grid_from_heights([((1, 1), 0)], &config);
        let light = grid.cell_light_at((1, 1)).expect("light");
        assert_eq!(light.common_scalar, 500);
        assert_eq!(light.raw_rgb, [1000, 800, 600]);
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
        let grid = build_cell_light_grid_from_heights([((1, 1), 0)], &config);
        let light = grid.cell_light_at((1, 1)).expect("light");
        // Ground/Level use the verified 250-unit scale: 0.5 -> 125.
        assert_eq!(light.common_scalar, 875);
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
    fn test_radiation_point_light_center_and_flags() {
        // spread 10 -> radius 10*256+128 = 2688; center cell (100,100).
        let light = radiation_point_light(100, 100, 2688, 50, [0, 1000, 0]);
        assert_eq!(light.center_x, 100 * 256 + 128);
        assert_eq!(light.center_y, 100 * 256 + 128);
        assert_eq!(light.radius_leptons, 2688);
        assert_eq!(light.intensity, 50);
        assert_eq!(light.tint, [0, 1000, 0]);
        assert!(
            light.active && light.detail,
            "radiation light is active and detail-forced"
        );
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
    fn terrain_object_instances_choose_branch_specific_cell_light() {
        let mut grid = CellLightGrid::new();
        let profile_id = grid
            .profiles
            .profile_id_for_key([LIGHT_UNIT, LIGHT_UNIT, LIGHT_UNIT]);
        grid.insert_light(
            (4, 5),
            CellLight::new(
                profile_id,
                [LIGHT_UNIT, LIGHT_UNIT, LIGHT_UNIT],
                [LIGHT_UNIT, LIGHT_UNIT, LIGHT_UNIT],
                LIGHT_SCALE16_IDENTITY,
                0,
                0,
                1200,
                900,
                1200,
                900,
                900,
            ),
        );

        assert_eq!(
            grid.terrain_object_tint_for_type((4, 5), false),
            [0.9, 0.9, 0.9]
        );
        assert_eq!(
            grid.terrain_object_tint_for_type((4, 5), true),
            [1.2, 1.2, 1.2]
        );
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
        let grid = build_cell_light_grid_from_heights([((1, 1), 0), ((1, 0), 3)], &config);

        assert_eq!(grid.profiles().len(), 1);
        assert_eq!(
            grid.cell_light_at((1, 1)).map(|light| light.profile_id),
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
        let base_scalar = light_float_to_units(base_tint[0]);
        for y in 0..size {
            for x in 0..size {
                let light = build_cell_light_from_raw(
                    &mut grid.profiles,
                    [LIGHT_UNIT, LIGHT_UNIT, LIGHT_UNIT],
                    0,
                    base_scalar,
                    base_scalar,
                );
                grid.insert_light((x, y), light);
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
        assert!((center[0] - 0.893).abs() < 0.01, "r={:.3}", center[0]);
        assert!((center[1] - 0.432).abs() < 0.01, "g={:.3}", center[1]);
        assert!((center[2] - 0.662).abs() < 0.01, "b={:.3}", center[2]);
        let light = grid.cell_light_at((5, 5)).expect("light");
        assert_eq!(light.raw_additive_intensity, 600);
        assert_eq!(light.raw_rgb, [2000, 1000, 1500]);
        assert_eq!(light.common_scalar, 900);
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
    fn lightconvert_normalization_preserves_scale16_and_rgb_key() {
        let mut profiles = LightProfileCache::new();
        let light = build_cell_light_from_raw(&mut profiles, [1050, 1050, 1010], 200, 1150, 1182);

        assert_eq!(light.raw_rgb, [1050, 1050, 1010]);
        assert_eq!(light.raw_additive_intensity, 200);
        assert_eq!(light.scale16, 68812);
        assert_eq!(light.additive_intensity, 209);
        assert_eq!(light.rgb_key, [992, 992, 960]);
        assert_eq!(light.common_scalar, 1150);
        assert_eq!(light.bottom_scalar, 1241);
    }

    #[test]
    fn light_normalize_max_one_resets_to_neutral() {
        let normalized = normalize_light([1, 1, 1], 123);

        assert_eq!(normalized.scale16, LIGHT_SCALE16_IDENTITY);
        assert_eq!(normalized.additive_intensity, 0);
        assert_eq!(normalized.rgb_key, [LIGHT_UNIT, LIGHT_UNIT, LIGHT_UNIT]);
    }

    #[test]
    fn light_normalize_max_two_does_not_reset_to_neutral() {
        let normalized = normalize_light([2, 1, 1], 100);

        assert_eq!(normalized.scale16, 131);
        assert_eq!(normalized.additive_intensity, 0);
        assert_eq!(normalized.rgb_key, [992, 480, 480]);
    }

    #[test]
    fn light_normalize_negative_additive_uses_arithmetic_shift() {
        let normalized = normalize_light([2, 1, 1], -1);

        assert_eq!(normalized.scale16, 131);
        assert_eq!(normalized.additive_intensity, -1);
    }

    #[test]
    fn light_normalize_green_max_uses_scale_denominator_for_quantization() {
        let normalized = normalize_light([19, 22, 0], 0);

        assert_eq!(normalized.scale16, 1441);
        assert_eq!(normalized.rgb_key, [864, 992, 0]);
    }

    #[test]
    fn galite_source_fields_match_rulesmd_units() {
        let light = point_light_from_object(10, 10, 5000, 0.2, [0.05, 0.05, 0.01]).expect("light");

        assert_eq!(light.radius_leptons, 5000);
        assert_eq!(light.intensity, 200);
        assert_eq!(light.tint, [50, 50, 10]);
    }

    #[test]
    fn galite_point_light_contribution_separates_intensity_and_rgb() {
        let mut grid =
            build_cell_light_grid_from_heights([((10, 10), 0)], &LightingConfig::default());
        let light = point_light_from_object(10, 10, 5000, 0.2, [0.05, 0.05, 0.01]).expect("light");
        accumulate_point_lights(&mut grid, &[light]);

        let center = grid.cell_light_at((10, 10)).expect("light");
        assert_eq!(center.raw_additive_intensity, 200);
        assert_eq!(center.raw_rgb, [1050, 1050, 1010]);
        assert_eq!(center.common_scalar, 1150);
        assert_ne!(center.raw_rgb, [10, 10, 2]);
    }

    #[test]
    fn light_intensity_zero_allocates_no_static_source() {
        assert!(point_light_from_object(1, 2, 4096, 0.0, [1.0, 1.0, 1.0]).is_none());
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
