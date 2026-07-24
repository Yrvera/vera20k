//! App init helper functions — map file loading, atlas building, rules/art loading,
//! skirmish seeding, overlay atlas construction.
//!
//! Extracted from app_init.rs for file-size limits.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use crate::assets::asset_manager::AssetManager;
use crate::assets::pal_file::Palette;
use crate::map::basic::BridgeDestroyabilityMode;
use crate::map::houses::HouseColorMap;
use crate::map::map_file::MapFile;
use crate::map::resolved_terrain::ResolvedTerrainGrid;
use crate::map::terrain::TerrainGrid;
use crate::map::theater::{self, TileImage, TileKey};
use crate::map::trigger_graph;
use crate::render::batch::BatchRenderer;
use crate::render::gpu::GpuContext;
use crate::render::sidebar_cameo_atlas::{self, SidebarCameoAtlas};
use crate::render::sprite_atlas::{self, SpriteAtlas};
use crate::render::tile_atlas::{self, TileAtlas};
use crate::render::unit_atlas::{self, UnitAtlas};
use crate::rules::art_data::ArtRegistry;
use crate::rules::ini_parser::IniFile;
use crate::rules::ruleset::RuleSet;
use crate::sim::world::Simulation;

use crate::app_skirmish::deployable_building_types;

pub(crate) fn build_sidebar_cameo_atlas(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    asset_manager: &AssetManager,
    rules: Option<&RuleSet>,
    art: Option<&ArtRegistry>,
) -> Option<SidebarCameoAtlas> {
    let rules = rules?;
    maybe_export_sidebar_cameo_debug(asset_manager, rules, art);
    let palette = load_sidebar_cameo_palette(asset_manager)?;
    sidebar_cameo_atlas::build_sidebar_cameo_atlas(gpu, batch, asset_manager, rules, art, &palette)
}

pub(crate) fn maybe_export_sidebar_cameo_debug(
    asset_manager: &AssetManager,
    rules: &RuleSet,
    art: Option<&ArtRegistry>,
) {
    let enabled = std::env::var("RA2_DEBUG_CAMEO_PALETTES")
        .ok()
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes" || v == "on"
        })
        .unwrap_or(false);
    if !enabled {
        return;
    }

    let palette_names = [
        "cameo.pal",
        "cameomd.pal",
        "mousepal.pal",
        "anim.pal",
        "unittem.pal",
        "unit.pal",
        "temperat.pal",
        "isotem.pal",
    ];
    sidebar_cameo_atlas::export_debug_palette_sheet(
        asset_manager,
        rules,
        art,
        Path::new("debug_sidebar_cameo_palettes.png"),
        &palette_names,
    );
}

pub(crate) fn load_sidebar_cameo_palette(asset_manager: &AssetManager) -> Option<Palette> {
    let palette_names = [
        "cameo.pal",
        "cameomd.pal",
        "mousepal.pal",
        "anim.pal",
        "unittem.pal",
        "unit.pal",
        "temperat.pal",
    ];
    for name in palette_names {
        if let Some(data) = asset_manager.get_ref(name) {
            if let Ok(palette) = Palette::from_bytes(data) {
                log::info!("Sidebar cameos using palette {}", name);
                return Some(palette);
            }
        }
    }
    log::warn!("Sidebar cameo palette not found");
    None
}

pub(crate) fn log_trigger_graph_diagnostics(map_data: &MapFile) {
    let diag = trigger_graph::analyze_trigger_graph(
        &map_data.cell_tags,
        &map_data.tags,
        &map_data.triggers,
        &map_data.events,
        &map_data.actions,
    );
    if diag.cell_tags_total == 0
        && diag.tags_total == 0
        && diag.triggers_total == 0
        && map_data.events.is_empty()
        && map_data.actions.is_empty()
    {
        return;
    }

    log::info!(
        "Trigger graph: cell_tags={}/{} resolved, tags={}/{} trigger refs resolved, triggers={} events={} actions={}",
        diag.cell_tags_resolved,
        diag.cell_tags_total,
        diag.tags_resolved_to_triggers,
        diag.tags_with_trigger_ref,
        diag.triggers_total,
        diag.triggers_with_event,
        diag.triggers_with_action
    );
    if !diag.dangling_cell_tags.is_empty() {
        log::warn!(
            "Trigger graph dangling cell tags (first 8): {:?}",
            &diag.dangling_cell_tags[..diag.dangling_cell_tags.len().min(8)]
        );
    }
    if !diag.dangling_tag_trigger_refs.is_empty() {
        log::warn!(
            "Trigger graph dangling tag->trigger refs (first 8): {:?}",
            &diag.dangling_tag_trigger_refs[..diag.dangling_tag_trigger_refs.len().min(8)]
        );
    }
    if !diag.triggers_missing_event.is_empty() {
        log::warn!(
            "Trigger graph triggers missing events (first 8): {:?}",
            &diag.triggers_missing_event[..diag.triggers_missing_event.len().min(8)]
        );
    }
    if !diag.triggers_missing_action.is_empty() {
        log::warn!(
            "Trigger graph triggers missing actions (first 8): {:?}",
            &diag.triggers_missing_action[..diag.triggers_missing_action.len().min(8)]
        );
    }
}

pub(crate) fn parse_debug_spawn_units_env() -> Option<Vec<String>> {
    let raw = std::env::var("RA2_DEBUG_SPAWN_UNITS").ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let enabled_tokens = ["1", "true", "yes", "on"];
    if enabled_tokens
        .iter()
        .any(|v| trimmed.eq_ignore_ascii_case(v))
    {
        return Some(vec![
            "HTNK".to_string(),
            "MTNK".to_string(),
            "E1".to_string(),
        ]);
    }
    let items: Vec<String> = trimmed
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    if items.is_empty() { None } else { Some(items) }
}

/// Build a texture atlas from pre-loaded theater data and the terrain grid.
pub(crate) fn build_tile_atlas(
    asset_manager: &AssetManager,
    lookup: &theater::TilesetLookup,
    palette: &Palette,
    _ext: &str,
    grid: &TerrainGrid,
    gpu: &GpuContext,
    batch: &BatchRenderer,
) -> Option<TileAtlas> {
    let cell_pairs: Vec<(i32, u8)> = grid
        .cells
        .iter()
        .map(|c| (c.tile_id as i32, c.sub_tile))
        .collect();
    let mut needed: HashSet<TileKey> = theater::collect_used_tiles(&cell_pairs);
    // Always include tile_id 0 (clear ground) — used as fallback for missing tiles.
    needed.insert(TileKey {
        tile_id: 0,
        sub_tile: 0,
        variant: 0,
    });
    // Inject the 8 bridge anchor variant tile_ids × all sub_tiles so the
    // atlas has them loaded before any damage hits at runtime. Without
    // this, the first weapon hit on a bridge ramp would be an atlas miss
    // on the variant cell, producing a blank sprite on the same tick.
    if let Some(table) = grid.anchor_variant_table {
        let before = needed.len();
        theater::inject_bridge_anchor_variant_tiles(&mut needed, &table, lookup, asset_manager);
        log::info!(
            "Atlas pre-load: injected {} bridge anchor variant TileKeys",
            needed.len() - before,
        );
    }
    log::info!("Map uses {} unique tile keys", needed.len());

    let images: HashMap<TileKey, TileImage> =
        theater::load_tile_images(asset_manager, lookup, palette, &needed);
    if images.is_empty() {
        log::warn!("No tile images loaded — falling back to single tile");
        return None;
    }

    let atlas: TileAtlas = tile_atlas::build_atlas(gpu, batch, &images);
    log::info!("Atlas built: {} tiles", atlas.tile_count());
    Some(atlas)
}

/// Fallback theater extension from theater name when load_theater fails.
pub(crate) fn theater_ext_for(theater_name: &str) -> &'static str {
    match theater_name.to_uppercase().as_str() {
        "TEMPERATE" => "tem",
        "SNOW" => "sno",
        "URBAN" => "urb",
        "DESERT" => "des",
        "LUNAR" => "lun",
        "NEWURBAN" => "ubn",
        _ => "tem",
    }
}

/// Successfully parsed rules and the exact merged INI that produced them.
///
/// This transient pair keeps match-load consumers on one rules source without
/// making the INI a second persistent rules authority.
pub(crate) struct LoadedRules {
    rules: RuleSet,
    merged_ini: IniFile,
}

impl LoadedRules {
    fn from_merged_ini(merged_ini: IniFile) -> Result<Self, crate::rules::error::RulesError> {
        let rules = RuleSet::from_ini(&merged_ini)?;
        debug_assert_eq!(rules.source_ini_hash(), merged_ini.content_hash());
        Ok(Self { rules, merged_ini })
    }

    pub(crate) fn into_parts(self) -> (RuleSet, IniFile) {
        (self.rules, self.merged_ini)
    }
}

/// Compose the already-parsed rules layers in the current Rust load order.
///
/// Full native application of selected-mode payloads remains unverified; this
/// helper preserves the established Rust order while centralizing it.
fn compose_rules_layers(
    mut ini: IniFile,
    rulesmd: Option<&IniFile>,
    mode: Option<&IniFile>,
    map: Option<&IniFile>,
) -> (IniFile, usize) {
    if let Some(rulesmd) = rulesmd {
        ini.merge(rulesmd);
    }
    if let Some(mode) = mode {
        ini.merge(mode);
    }
    let applied = map.map(|map| ini.merge_rules_overrides(map)).unwrap_or(0);
    (ini, applied)
}

/// Load rules.ini from MIX archives and retain its exact merged source.
///
/// rulesmd.ini patches rules.ini. Current Rust then applies the selected
/// game-mode payload and finally bounded existing-section map value overrides.
/// Full native selected-mode rules application remains unverified; the order
/// here is intentionally preserved rather than certified.
pub(crate) fn load_rules_with_merged_ini(
    asset_manager: &AssetManager,
    mode_rules_override: Option<&IniFile>,
    map_rules_overrides: Option<&IniFile>,
) -> Option<LoadedRules> {
    // Step 1: Load base rules.ini.
    let ini: IniFile = if let Some((data, source)) = asset_manager.get_with_source("rules.ini") {
        log::info!(
            "Loading rules.ini ({} bytes) from {} (base)",
            data.len(),
            source
        );
        IniFile::from_bytes(&data).ok()?
    } else {
        log::warn!("rules.ini not found in MIX archives");
        return None;
    };

    // Step 2: Parse an optional rulesmd.ini YR patch.
    let rulesmd: Option<IniFile> =
        if let Some((patch_data, patch_source)) = asset_manager.get_with_source("rulesmd.ini") {
            log::info!(
                "Loading rulesmd.ini ({} bytes) from {} (YR patch)",
                patch_data.len(),
                patch_source
            );
            IniFile::from_bytes(&patch_data).ok()
        } else {
            None
        };
    let rulesmd_sections = rulesmd.as_ref().map(IniFile::section_count);
    let mode_sections = mode_rules_override.map(IniFile::section_count);

    // Preserve the current Rust base/YR/mode/map order in one composition.
    let (ini, applied_map_keys) = compose_rules_layers(
        ini,
        rulesmd.as_ref(),
        mode_rules_override,
        map_rules_overrides,
    );

    if let Some(patch_sections) = rulesmd_sections {
        log::info!(
            "Merged {} rulesmd.ini sections on top of rules.ini",
            patch_sections
        );
    }
    if let Some(mode_sections) = mode_sections {
        log::info!(
            "Merged {} game-mode override section(s) into rules",
            mode_sections
        );
    }

    // Map registry allocation remains a separate verified residual.
    if applied_map_keys > 0 {
        log::info!("Applied {} map rules-override key(s)", applied_map_keys);
    }

    match LoadedRules::from_merged_ini(ini) {
        Ok(loaded) => {
            log::info!("RuleSet: {} objects loaded", loaded.rules.object_count());
            Some(loaded)
        }
        Err(e) => {
            log::warn!("Failed to parse merged rules: {}", e);
            None
        }
    }
}

/// Load the merged rules for callers that do not need the transient source.
pub(crate) fn load_rules_ini(
    asset_manager: &AssetManager,
    mode_rules_override: Option<&IniFile>,
    map_rules_overrides: Option<&IniFile>,
) -> Option<RuleSet> {
    load_rules_with_merged_ini(asset_manager, mode_rules_override, map_rules_overrides)
        .map(|loaded| loaded.into_parts().0)
}

/// Seed dialog 0x102's Credits/Unit Count trackbar bounds from
/// `[MultiplayerDialogSettings]`, mirroring gamemd reading MinMoney/MaxMoney/
/// MoneyIncrement and MinUnitCount/MaxUnitCount from the live Rules instance when
/// it builds the skirmish dialog. rulesmd.ini is a YR patch over rules.ini, so we
/// merge the same way `load_rules_ini` does, then read the section. Falls back to
/// the stock-default constants when the rules INI (or a key) is unavailable.
pub(crate) fn load_skirmish_trackbar_bounds(
    asset_manager: &AssetManager,
) -> crate::ui::skirmish_shell::SkirmishTrackbarBounds {
    use crate::ui::skirmish_shell::SkirmishTrackbarBounds;

    let Some((data, _)) = asset_manager.get_with_source("rules.ini") else {
        return SkirmishTrackbarBounds::default();
    };
    let Ok(mut ini) = IniFile::from_bytes(&data) else {
        return SkirmishTrackbarBounds::default();
    };
    if let Some((patch_data, _)) = asset_manager.get_with_source("rulesmd.ini") {
        if let Ok(patch_ini) = IniFile::from_bytes(&patch_data) {
            ini.merge(&patch_ini);
        }
    }
    SkirmishTrackbarBounds::from_multiplayer_dialog_settings(&ini)
}

/// Seed the per-match option values from `[MultiplayerDialogSettings]`,
/// mirroring the original reading this section once into the rules data that
/// both the skirmish setup dialog and the launched match read from. rulesmd.ini
/// is a YR patch over rules.ini, so we merge the same way `load_rules_ini` does,
/// then parse the section. Falls back to the stock-default options when the
/// rules INI (or a key) is unavailable, so stock skirmishes are unchanged.
pub(crate) fn load_skirmish_game_options(
    asset_manager: &AssetManager,
) -> crate::sim::game_options::GameOptions {
    use crate::sim::game_options::GameOptions;

    let Some((data, _)) = asset_manager.get_with_source("rules.ini") else {
        return GameOptions::default();
    };
    let Ok(mut ini) = IniFile::from_bytes(&data) else {
        return GameOptions::default();
    };
    if let Some((patch_data, _)) = asset_manager.get_with_source("rulesmd.ini") {
        if let Ok(patch_ini) = IniFile::from_bytes(&patch_data) {
            ini.merge(&patch_ini);
        }
    }
    GameOptions::from_multiplayer_dialog_settings(&ini)
}

/// Load art.ini from MIX archives and parse into ArtRegistry.
///
/// Like rules, artmd.ini is a YR patch on top of art.ini. We load art.ini
/// first, then merge artmd.ini on top so all base entries are preserved.
pub(crate) fn load_art_ini(asset_manager: &AssetManager) -> Option<(ArtRegistry, IniFile)> {
    // Step 1: Load base art.ini.
    let mut ini: IniFile = if let Some((data, source)) = asset_manager.get_with_source("art.ini") {
        log::info!(
            "Loading art.ini ({} bytes) from {} (base)",
            data.len(),
            source
        );
        match IniFile::from_bytes(&data) {
            Ok(i) => i,
            Err(e) => {
                log::warn!("Failed to parse art.ini: {}", e);
                return None;
            }
        }
    } else {
        log::warn!("art.ini not found in MIX archives");
        return None;
    };

    // Step 2: If artmd.ini exists, merge it on top (YR patch).
    if let Some((patch_data, patch_source)) = asset_manager.get_with_source("artmd.ini") {
        log::info!(
            "Loading artmd.ini ({} bytes) from {} (YR patch)",
            patch_data.len(),
            patch_source
        );
        if let Ok(patch_ini) = IniFile::from_bytes(&patch_data) {
            let patch_sections: usize = patch_ini.section_count();
            ini.merge(&patch_ini);
            log::info!(
                "Merged {} artmd.ini sections on top of art.ini",
                patch_sections
            );
        }
    }

    let reg: ArtRegistry = ArtRegistry::from_ini(&ini);
    log::info!("ArtRegistry: {} entries loaded", reg.len());
    Some((reg, ini))
}

/// Preserve the pre-existing SystemTime mix for explicitly unverified legacy
/// and generic loads. Accepted ordinary Windows startup never calls this path;
/// it carries the one stored `GetTickCount` word from `match_bootstrap`.
pub(crate) fn generate_unverified_legacy_match_seed() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    now.subsec_nanos() ^ (now.as_secs() as u32).rotate_left(16)
}

/// Spawn map entities into ECS world and build voxel + SHP sprite atlases.
pub(crate) fn spawn_entities(
    map_data: &MapFile,
    resolved_terrain: &ResolvedTerrainGrid,
    asset_manager: &AssetManager,
    gpu: &GpuContext,
    batch: &BatchRenderer,
    theater_ext: &str,
    theater_name: &str,
    rules: Option<&RuleSet>,
    art: Option<&ArtRegistry>,
    house_colors: &HouseColorMap,
    height_map: &BTreeMap<(u16, u16), u8>,
    theater_unit_palette: Option<&Palette>,
    infantry_sequences: &crate::rules::infantry_sequence::InfantrySequenceRegistry,
    vxl_compute: Option<&mut crate::render::vxl_compute::VxlComputeRenderer>,
    bridge_destroyability_mode: BridgeDestroyabilityMode,
    descriptor: &crate::sim::scenario_session::ScenarioDescriptor,
) -> (
    Option<Simulation>,
    Option<UnitAtlas>,
    Option<SpriteAtlas>,
    Option<crate::render::palette_textures::PaletteSet>,
) {
    let mut sim: Simulation = Simulation::from_descriptor(descriptor);
    // Frame tripwire: every MP start waypoint must sit inside the session
    // bounds (= the fog window, cell-array frame). A start outside means the
    // descriptor was fed wrong-frame bounds (e.g. raw [Map] Size=) and the
    // player's own base would be permanently shrouded.
    for (idx, (rx, ry)) in &descriptor.mp_start_waypoints {
        if *rx >= descriptor.map_width || *ry >= descriptor.map_height {
            log::error!(
                "MP start waypoint {idx} at ({rx},{ry}) lies outside session bounds {}x{} — wrong coordinate frame?",
                descriptor.map_width,
                descriptor.map_height
            );
            debug_assert!(
                false,
                "start waypoint outside session bounds (coordinate-frame mismatch)"
            );
        }
    }
    sim.resolved_terrain = Some(resolved_terrain.clone());
    // Wire the cliff/slope coefficients from [General] into the live World config;
    // it otherwise holds compiled vanilla defaults and never sees a modded INI.
    if let Some(rules) = rules {
        sim.terrain_speed_config =
            crate::sim::pathfinding::terrain_speed::TerrainSpeedConfig::from_general(
                rules.general.tracked_uphill,
                rules.general.tracked_downhill,
                rules.general.wheeled_uphill,
                rules.general.wheeled_downhill,
            );
    }
    // The playfield diamond: [Map] Size width + the raw LocalSize rect, stored
    // verbatim — the isometric transform lives in the validator's diamond test.
    sim.playfield_bounds = Some(crate::sim::cell_rect::PlayfieldBounds {
        base: map_data.header.width as i32,
        off_fc: map_data.header.local_left as i32,
        off_100: map_data.header.local_top as i32,
        off_104: map_data.header.local_width as i32,
        off_108: map_data.header.local_height as i32,
    });
    let bridge_destroyable = map_data
        .special_flags
        .effective_destroyable_bridges(bridge_destroyability_mode);
    let bridge_strength = rules
        .map(|rules| rules.bridge_rules.strength)
        .unwrap_or(1500);
    sim.bridge_state = Some(
        crate::sim::bridge_state::BridgeRuntimeState::from_resolved_terrain(
            resolved_terrain,
            bridge_destroyable,
            bridge_strength,
        ),
    );
    sim.bridge_explosions = rules
        .map(|r| {
            r.bridge_rules
                .explosions
                .iter()
                .map(|s| sim.interner.intern(s))
                .collect()
        })
        .unwrap_or_default();
    sim.metallic_debris = rules
        .map(|r| {
            r.general
                .metallic_debris
                .iter()
                .map(|s| sim.interner.intern(s))
                .collect()
        })
        .unwrap_or_default();
    if let Some(art) = art {
        let mut bridge_anim_sounds = BTreeMap::new();
        for anim_id in sim
            .bridge_explosions
            .iter()
            .chain(sim.metallic_debris.iter())
            .copied()
        {
            let anim_name = sim.interner.resolve(anim_id);
            if let Some(entry) = art.get(anim_name) {
                let sound_name = entry.start_sound.as_ref().or(entry.report.as_ref());
                if let Some(sound_name) = sound_name {
                    let sound_id = sim.interner.intern(sound_name);
                    bridge_anim_sounds.insert(anim_id, sound_id);
                }
            }
        }
        sim.bridge_anim_sounds = bridge_anim_sounds;
    }
    if !map_data.entities.is_empty() {
        let _count: u32 = sim.spawn_from_map_with_resolved(
            &map_data.entities,
            rules,
            height_map,
            Some(resolved_terrain),
        );
        let miner_count: usize = sim
            .entities()
            .values()
            .filter(|e| e.miner.is_some())
            .count();
        log::info!("Miner components attached: {}", miner_count);
    }
    let (unit_atlas, shp_atlas, palette_set) = build_entity_atlases(
        &sim,
        asset_manager,
        gpu,
        batch,
        theater_ext,
        theater_name,
        rules,
        art,
        house_colors,
        theater_unit_palette,
        infantry_sequences,
        vxl_compute,
    );
    // Update VoxelAnimation frame counts from atlas HVA data.
    if let Some(ref atlas) = unit_atlas {
        sim.update_voxel_anim_frame_counts(&atlas.frame_counts);
    }
    (Some(sim), unit_atlas, shp_atlas, palette_set)
}

pub(crate) fn build_entity_atlases(
    sim: &Simulation,
    asset_manager: &AssetManager,
    gpu: &GpuContext,
    batch: &BatchRenderer,
    theater_ext: &str,
    theater_name: &str,
    rules: Option<&RuleSet>,
    art: Option<&ArtRegistry>,
    house_colors: &HouseColorMap,
    theater_unit_palette: Option<&Palette>,
    infantry_sequences: &crate::rules::infantry_sequence::InfantrySequenceRegistry,
    vxl_compute: Option<&mut crate::render::vxl_compute::VxlComputeRenderer>,
) -> (
    Option<UnitAtlas>,
    Option<SpriteAtlas>,
    Option<crate::render::palette_textures::PaletteSet>,
) {
    // Use the theater-specific unit palette if provided, otherwise fall back to search.
    let palette: Option<Palette> = theater_unit_palette.cloned().or_else(|| {
        let pal_names: &[&str] = &["unittem.pal", "unit.pal", "temperat.pal"];
        pal_names.iter().find_map(|name| {
            let data: Vec<u8> = asset_manager.get(name)?;
            Palette::from_bytes(&data).ok()
        })
    });
    // Atlas build no longer needs the palette (tiles store palette indices,
    // not RGB). The palette load above is kept because downstream PaletteSet
    // construction (Task 1.9) will consume it. Skip the build if no palette
    // is available — it indicates a missing theater asset and rendering
    // wouldn't work anyway.
    let unit_atlas: Option<UnitAtlas> = if palette.is_some() {
        unit_atlas::build_unit_atlas(
            gpu,
            batch,
            sim.entities(),
            asset_manager,
            rules,
            art,
            None, // initial build — no existing cache
            vxl_compute,
            Some(&sim.interner),
        )
    } else {
        None
    };
    // Pre-load building types that can be spawned at runtime (e.g., ConYards from MCV deploy).
    let extra_buildings: Vec<&str> =
        deployable_building_types(sim.entities(), rules, Some(&sim.interner));
    let shp_atlas: Option<SpriteAtlas> = palette.as_ref().and_then(|pal| {
        sprite_atlas::build_sprite_atlas(
            gpu,
            batch,
            sim.entities(),
            asset_manager,
            pal,
            theater_ext,
            theater_name,
            rules,
            art,
            house_colors,
            &extra_buildings,
            infantry_sequences,
            None, // initial build — no existing cache
            Some(&sim.interner),
        )
    });
    // Build PaletteSet: theater palette + per-house RGB ramps for the voxel
    // sprite shader. Active houses are derived from the house_colors map
    // (deduplicated; row 0 of the ramp texture is the no-remap fallback).
    let default_ramps = crate::rules::house_colors::HouseColorRamps::default();
    let house_ramps: &crate::rules::house_colors::HouseColorRamps = rules
        .map(|r| &r.house_color_ramps)
        .unwrap_or(&default_ramps);
    let palette_set: Option<crate::render::palette_textures::PaletteSet> =
        palette.as_ref().map(|pal| {
            let mut active: Vec<crate::rules::house_colors::HouseColorIndex> =
                house_colors.values().copied().collect();
            active.sort_by_key(|h| h.0);
            active.dedup();
            crate::render::palette_textures::PaletteSet::new(gpu, pal, house_ramps, &active)
        });
    (unit_atlas, shp_atlas, palette_set)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{LoadedRules, compose_rules_layers, load_rules_with_merged_ini};
    use crate::assets::asset_manager::AssetManager;
    use crate::map::overlay_types::OverlayTypeRegistry;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;

    #[derive(Debug, PartialEq, Eq)]
    struct OverlayRegistryEntrySnapshot {
        id: u8,
        name: String,
        tiberium: bool,
        wall: bool,
        is_veins: bool,
        is_veinhole_monster: bool,
        is_gate: bool,
        crushable: bool,
        crate_type: bool,
        is_rubble: bool,
        is_a_rock: bool,
        land_wheel_speed_zero: bool,
        bridge_deck: bool,
        radar_color: Option<[u8; 3]>,
        track: bool,
        land: Option<String>,
        strength: u16,
        damage_levels: u16,
    }

    fn overlay_registry_snapshot(
        registry: &OverlayTypeRegistry,
    ) -> Vec<OverlayRegistryEntrySnapshot> {
        (0..registry.len())
            .map(|index| {
                let id = u8::try_from(index).expect("overlay registry fits u8 IDs");
                let name = registry.name(id).expect("registered overlay has a name");
                let flags = registry.flags(id).expect("registered overlay has flags");
                OverlayRegistryEntrySnapshot {
                    id,
                    name: name.to_string(),
                    tiberium: flags.tiberium,
                    wall: flags.wall,
                    is_veins: flags.is_veins,
                    is_veinhole_monster: flags.is_veinhole_monster,
                    is_gate: flags.is_gate,
                    crushable: flags.crushable,
                    crate_type: flags.crate_type,
                    is_rubble: flags.is_rubble,
                    is_a_rock: flags.is_a_rock,
                    land_wheel_speed_zero: flags.land_wheel_speed_zero,
                    bridge_deck: flags.bridge_deck,
                    radar_color: flags.radar_color,
                    track: flags.track,
                    land: flags.land.clone(),
                    strength: flags.strength,
                    damage_levels: flags.damage_levels,
                }
            })
            .collect()
    }

    fn retail_assets() -> AssetManager {
        let path = std::env::var_os("RA2_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from("C:/Users/enok/Documents/Command and Conquer Red Alert II")
            });
        assert!(
            path.is_dir(),
            "retail RA2/YR directory does not exist: {}",
            path.display()
        );
        AssetManager::new(&path).expect("load retail RA2/YR assets")
    }

    const RULES_BASE: &str = "[InfantryTypes]\n0=E1\n[E1]\nStrength=125\n\
        [General]\nBuildSpeed=.7\n[CombatDamage]\nC4Delay=.03\n";

    /// AT-9: a map embedding [General]/[CombatDamage] overrides lands those
    /// values in RuleSet, including a sim-consumed path (C4 delay ticks).
    #[test]
    fn map_ini_overrides_rules_values() {
        let mut ini = IniFile::from_str(RULES_BASE);
        let map = IniFile::from_str(
            "[Basic]\nName=Fixture\n[General]\nBuildSpeed=1\n[CombatDamage]\nC4Delay=.06\n",
        );
        ini.merge_rules_overrides(&map);
        let rules = RuleSet::from_ini(&ini).expect("triple-merged rules parse");
        // C4Delay is minutes: ticks = minutes * 60 * 15 => .06 -> 54.
        assert_eq!(rules.c4_delay_ticks, 54);
        // BuildSpeed consumer — assert the deterministic x1000 field, not the
        // f32 mirror: map override 1 -> 1000 (base .7 would be 700).
        assert_eq!(rules.production.build_speed_x1000, 1000);
    }

    /// AT-9 inverse: a map with no rules-shaped sections changes nothing.
    #[test]
    fn map_without_overrides_leaves_rules_unchanged() {
        let mut with_map = IniFile::from_str(RULES_BASE);
        let map = IniFile::from_str("[Basic]\nName=Clean\n[Waypoints]\n0=45035\n");
        with_map.merge_rules_overrides(&map);
        let a = RuleSet::from_ini(&with_map).expect("parse");
        let b = RuleSet::from_ini(&IniFile::from_str(RULES_BASE)).expect("parse");
        assert_eq!(a.c4_delay_ticks, b.c4_delay_ticks);
        assert_eq!(
            a.production.build_speed_x1000,
            b.production.build_speed_x1000
        );
        assert_eq!(
            a.object("E1").map(|o| o.strength),
            b.object("E1").map(|o| o.strength)
        );
    }

    /// AT-10: a key present in both rules.ini and rulesmd.ini resolves to the
    /// rulesmd value through the same merge `load_rules_ini` performs; a
    /// rules.ini-only key survives.
    #[test]
    fn rulesmd_overrides_rules_base() {
        let mut ini = IniFile::from_str("[General]\nBuildSpeed=.7\nFlightLevel=1500\n");
        let patch = IniFile::from_str("[General]\nBuildSpeed=.58\n");
        ini.merge(&patch);
        assert_eq!(
            ini.section("General").unwrap().get("BuildSpeed"),
            Some(".58")
        );
        assert_eq!(
            ini.section("General").unwrap().get("FlightLevel"),
            Some("1500")
        );
        let rules = RuleSet::from_ini(&ini).expect("merged rules parse");
        assert_eq!(rules.production.build_speed_x1000, 580);
    }

    /// Preserve the current Rust mode ordering: after rulesmd and before map
    /// values. Full native rules application of mode payloads is unverified.
    #[test]
    fn mode_override_merges_after_rulesmd_before_map() {
        let mut ini = IniFile::from_str("[General]\nBuildSpeed=.7\nFlightLevel=1500\n");
        let rulesmd = IniFile::from_str("[General]\nBuildSpeed=.58\n");
        ini.merge(&rulesmd);
        let mode = IniFile::from_str("[General]\nBuildSpeed=1\nFlightLevel=1200\n");
        ini.merge(&mode);
        let map = IniFile::from_str("[General]\nFlightLevel=900\n");
        ini.merge_rules_overrides(&map);

        let general = ini.section("General").unwrap();
        // Mode beats rulesmd.
        assert_eq!(general.get("BuildSpeed"), Some("1"));
        // Map beats mode.
        assert_eq!(general.get("FlightLevel"), Some("900"));
    }

    #[test]
    fn map_overlay_flag_wins_after_rulesmd_and_mode() {
        let base = IniFile::from_str(
            "[OverlayTypes]\n1=TIB01\n\
             [Tiberiums]\n0=Riparius\n\
             [Riparius]\nImage=1\n\
             [TIB01]\nTiberium=no\n",
        );
        let rulesmd = IniFile::from_str("[Riparius]\nImage=2\n");
        let mode = IniFile::from_str("[Riparius]\nImage=3\n[TIB01]\nTiberium=no\n");
        let map = IniFile::from_str(
            "[OverlayTypes]\n1=GASAND\n\
             [Tiberiums]\n0=Cruentus\n\
             [Riparius]\nImage=4\n\
             [TIB01]\nTiberium=yes\n",
        );

        let (merged_ini, applied) =
            compose_rules_layers(base, Some(&rulesmd), Some(&mode), Some(&map));
        assert_eq!(applied, 2, "only existing per-type map keys apply");
        let loaded = LoadedRules::from_merged_ini(merged_ini).expect("paired rules");
        let (rules, merged_ini) = loaded.into_parts();
        let registry = OverlayTypeRegistry::from_ini(&merged_ini, None);

        assert_eq!(rules.tiberium_types.types()[0].section, "Riparius");
        assert_eq!(rules.tiberium_types.types()[0].image, 4);
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.id_for_name("TIB01"), Some(0));
        assert_eq!(registry.name(0), Some("TIB01"));
        assert!(registry.flags(0).is_some_and(|flags| flags.tiberium));
        assert_eq!(rules.source_ini_hash(), merged_ini.content_hash());
    }

    #[test]
    #[ignore = "requires retail RA2/YR assets"]
    fn retail_rules_plus_map_override_reaches_production_overlay_registry() {
        let assets = retail_assets();
        let (raw_bytes, _) = assets
            .get_with_source("rulesmd.ini")
            .or_else(|| assets.get_with_source("rules.ini"))
            .expect("production raw rules selection");
        let raw_ini = IniFile::from_bytes(&raw_bytes).expect("parse raw retail rules");
        let raw_registry = OverlayTypeRegistry::from_ini(&raw_ini, None);
        assert_eq!(raw_registry.id_for_name("GASAND"), Some(0));
        assert_eq!(raw_registry.name(0), Some("GASAND"));
        assert!(!raw_registry.flags(0).expect("GASAND flags").tiberium);

        let map = IniFile::from_str("[GASAND]\nTiberium=yes\n");
        let loaded = load_rules_with_merged_ini(&assets, None, Some(&map))
            .expect("retail merged rules pair");
        let (rules, merged_ini) = loaded.into_parts();
        let merged_registry = OverlayTypeRegistry::from_ini(&merged_ini, None);

        assert_eq!(merged_registry.id_for_name("GASAND"), Some(0));
        assert_eq!(merged_registry.name(0), Some("GASAND"));
        assert!(merged_registry.flags(0).expect("GASAND flags").tiberium);
        assert_eq!(rules.source_ini_hash(), merged_ini.content_hash());
    }

    #[test]
    #[ignore = "requires retail RA2/YR assets"]
    fn retail_mount_moras_applies_rules_and_preserves_overlay_registry() {
        let assets = retail_assets();
        let (map_bytes, source) = assets
            .get_with_source("MountMoras.map")
            .expect("MountMoras.map");
        assert_eq!(source, "expandmd01.mix");
        assert_eq!(map_bytes.len(), 103_241);
        let map = IniFile::from_bytes(&map_bytes).expect("parse MountMoras.map");
        assert!(map.section("General").is_some());
        assert!(map.section("GAYARD").is_some());
        assert!(map.section("OverlayTypes").is_none());
        assert!(map.section("Tiberiums").is_none());

        let no_map =
            load_rules_with_merged_ini(&assets, None, None).expect("retail no-map rules pair");
        let with_map = load_rules_with_merged_ini(&assets, None, Some(&map))
            .expect("retail MountMoras rules pair");
        let (no_map_rules, no_map_ini) = no_map.into_parts();
        let (map_rules, map_ini) = with_map.into_parts();

        assert_eq!(
            no_map_rules
                .object("GAYARD")
                .map(|object| object.tech_level),
            Some(4)
        );
        assert_eq!(
            map_rules.object("GAYARD").map(|object| object.tech_level),
            Some(11)
        );
        assert_eq!(no_map_rules.source_ini_hash(), no_map_ini.content_hash());
        assert_eq!(map_rules.source_ini_hash(), map_ini.content_hash());
        assert_ne!(no_map_rules.source_ini_hash(), map_rules.source_ini_hash());

        let no_map_registry = OverlayTypeRegistry::from_ini(&no_map_ini, None);
        let map_registry = OverlayTypeRegistry::from_ini(&map_ini, None);
        assert_eq!(
            overlay_registry_snapshot(&no_map_registry),
            overlay_registry_snapshot(&map_registry)
        );
    }

    /// AT-12 (RC-4): type/weapon/warhead resolution reproduces the engine's
    /// outcomes for the three awkward cases.
    /// - **Forward reference:** the [HTNK] object names `Primary=120mm`, that
    ///   weapon names `Warhead=AP`, and both sections appear LATER in the file.
    ///   The engine resolves from a fully-parsed section table, so order is
    ///   irrelevant — both must resolve.
    /// - **Case-duplicate name:** [HTNK] is redefined as [htnk] further down.
    ///   One record per case-insensitive name, last definition wins
    ///   (Strength 200, not 100), and lookup is case-insensitive.
    /// - **Sectionless registry entry:** GHOST is listed in [VehicleTypes] but
    ///   has no [GHOST] section — silently skipped, no record.
    #[test]
    fn resolution_order_matches_engine() {
        let ini = IniFile::from_str(
            "[General]\nBuildSpeed=.7\n\
             [VehicleTypes]\n0=HTNK\n1=GHOST\n\
             [HTNK]\nStrength=100\nPrimary=120mm\n\
             [120mm]\nDamage=50\nWarhead=AP\n\
             [AP]\nVerses=100%,100%,100%\n\
             [htnk]\nStrength=200\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("fixture rules parse");

        // Forward-referenced weapon + its warhead both resolved.
        assert!(rules.weapon("120mm").is_some(), "forward-referenced weapon");
        assert!(rules.warhead("AP").is_some(), "forward-referenced warhead");

        // Case-duplicate collapses to one record; last definition wins; the
        // lookup is case-insensitive.
        assert!(rules.object("htnk").is_some());
        assert_eq!(rules.object("HTNK").map(|o| o.strength), Some(200));

        // Registry entry with no section produced no record.
        assert!(rules.object("GHOST").is_none());
    }

    /// AT-11 (RC-3): every ported scalar default that maps to a verified
    /// RulesClass constructor default falls back to THAT value when the key is
    /// absent from the INI. Constructor defaults verified from the binary
    /// (immediate stores inside the RulesClass ctor; doubles cross-checked
    /// against RULESCLASS_CONSTRUCTOR_DEFAULTS.csv): FlightLevel=500,
    /// GrowthRate=2.0 min, RepairStep=5, RepairPercent=25%, BuildSpeed=1.0,
    /// ParachuteMaxFallRate=-3, ParadropRadius=1024, URepairRate=.016 min
    /// (→14 ticks), C4Delay=.03 min (→27 ticks).
    ///
    /// Retail rulesmd.ini always supplies its own value for each, so these
    /// fallbacks fire only for a non-retail INI missing the key — matching the
    /// ctor default is behaviour-neutral in real play and faithful to gamemd's
    /// key-absent path.
    ///
    /// EXCLUDED: `VeteranSight` (an `i32` field reading a `double` INI key —
    /// retail "0.0" fails the i32 parse, so its fallback already fires in
    /// normal play; that is a pre-existing representation bug, not a
    /// fallback-only default) and `GapRadius` (no RulesClass ctor field in the
    /// verified offset map to flip to).
    #[test]
    fn ported_defaults_match_ctor_csv() {
        // Sections present but empty: every key below is absent, so each field
        // takes its fallback default (the realistic "key missing" path).
        let rules = RuleSet::from_ini(&IniFile::from_str("[General]\n[CombatDamage]\n"))
            .expect("empty-section rules parse");

        // [General] scalar fallbacks == ctor defaults.
        assert_eq!(rules.general.flight_level, 500, "FlightLevel");
        assert_eq!(
            rules.general.parachute_max_fall_rate, -3,
            "ParachuteMaxFallRate"
        );
        assert_eq!(rules.general.paradrop_radius, 1024, "ParadropRadius");
        assert_eq!(rules.general.repair_step, 5, "RepairStep");
        assert_eq!(rules.general.repair_percent, 25, "RepairPercent (25%)");
        assert_eq!(
            rules.general.unit_repair_rate_ticks, 14,
            "URepairRate .016 min -> 14 ticks"
        );
        assert_eq!(rules.general.growth_rate_minutes, 2.0, "GrowthRate");

        // [General] BuildSpeed -> deterministic x1000 field (1.0 -> 1000).
        assert_eq!(rules.production.build_speed_x1000, 1000, "BuildSpeed 1.0");

        // [CombatDamage] C4Delay .03 min -> 27 ticks.
        assert_eq!(rules.c4_delay_ticks, 27, "C4Delay .03 min -> 27 ticks");
    }

    /// The rules hash (RuleSet::source_ini_hash, stamped into replay/snapshot
    /// headers) is sensitive to a map's *value* overrides — closing the gap
    /// where a registry-only hash let a map override [General]/[CombatDamage]
    /// values without changing the hash, so a replay/snapshot recorded under
    /// the map could play back against base rules undetected.
    #[test]
    fn rules_hash_reflects_map_value_overrides() {
        let no_override = RuleSet::from_ini(&IniFile::from_str(RULES_BASE)).expect("parse");

        let mut with_override = IniFile::from_str(RULES_BASE);
        with_override.merge_rules_overrides(&IniFile::from_str("[General]\nBuildSpeed=2\n"));
        let overridden = RuleSet::from_ini(&with_override).expect("parse");

        assert_ne!(
            no_override.source_ini_hash(),
            overridden.source_ini_hash(),
            "a map BuildSpeed override must change the rules hash"
        );
    }
}
