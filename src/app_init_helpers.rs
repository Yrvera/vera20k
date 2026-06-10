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

/// Load rules.ini from MIX archives and parse into RuleSet.
///
/// In YR, rulesmd.ini is a PATCH on top of rules.ini — it only contains
/// the changes/additions that Yuri's Revenge makes. We must load rules.ini
/// first as the base, then merge rulesmd.ini on top. Without this merge,
/// buildings are missing key properties like Foundation sizes.
///
/// `map_rules_overrides` is the selected map's parsed INI: maps may override
/// rules *values* on top of the merged result (the original re-reads its
/// rules from the map file). Pass `None` on pre-map paths (startup shell).
pub(crate) fn load_rules_ini(
    asset_manager: &AssetManager,
    map_rules_overrides: Option<&IniFile>,
) -> Option<RuleSet> {
    // Step 1: Load base rules.ini.
    let mut ini: IniFile = if let Some((data, source)) = asset_manager.get_with_source("rules.ini")
    {
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

    // Step 2: If rulesmd.ini exists, merge it on top (YR patch).
    if let Some((patch_data, patch_source)) = asset_manager.get_with_source("rulesmd.ini") {
        log::info!(
            "Loading rulesmd.ini ({} bytes) from {} (YR patch)",
            patch_data.len(),
            patch_source
        );
        if let Ok(patch_ini) = IniFile::from_bytes(&patch_data) {
            let patch_sections: usize = patch_ini.section_count();
            ini.merge(&patch_ini);
            log::info!(
                "Merged {} rulesmd.ini sections on top of rules.ini",
                patch_sections
            );
        }
    }

    // Step 3: map rules overrides — the original re-reads its rules from the
    // map file after the main load, so maps may override value sections
    // ([General], [CombatDamage], per-type sections, ...). Registry lists are
    // excluded until allocation-from-map semantics are verified.
    if let Some(map_ini) = map_rules_overrides {
        let applied = ini.merge_rules_overrides(map_ini);
        if applied > 0 {
            log::info!("Applied {} map rules-override key(s)", applied);
        }
    }

    match RuleSet::from_ini(&ini) {
        Ok(rules) => {
            log::info!("RuleSet: {} objects loaded", rules.object_count());
            Some(rules)
        }
        Err(e) => {
            log::warn!("Failed to parse merged rules: {}", e);
            None
        }
    }
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

/// Draw one fresh per-match seed. The SP analog of the original fixing its
/// global RNG seed once per game before any setup-phase draw; we are bound to
/// reaching one shared u32 seed, not to the original's entropy source. MP
/// will hand the host's seed over the wire through the same descriptor.
pub(crate) fn generate_match_seed() -> u32 {
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
        let miner_count: usize = sim.entities().values().filter(|e| e.miner.is_some()).count();
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
    let house_ramps: &crate::rules::house_colors::HouseColorRamps =
        rules.map(|r| &r.house_color_ramps).unwrap_or(&default_ramps);
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
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;

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
        assert_eq!(a.production.build_speed_x1000, b.production.build_speed_x1000);
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
        assert_eq!(ini.section("General").unwrap().get("BuildSpeed"), Some(".58"));
        assert_eq!(
            ini.section("General").unwrap().get("FlightLevel"),
            Some("1500")
        );
        let rules = RuleSet::from_ini(&ini).expect("merged rules parse");
        assert_eq!(rules.production.build_speed_x1000, 580);
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
        assert_eq!(rules.general.parachute_max_fall_rate, -3, "ParachuteMaxFallRate");
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
}
