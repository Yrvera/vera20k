//! Overlay, world effect, and fog snapshot instance builders.
//!
//! Generates SpriteInstances for map overlays (ore/gems, bridges, terrain objects),
//! world-position effects (warp sparkles), and fog-of-war building snapshots.
//! Split from `presentation::instances` to keep files under the 600-line limit.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use std::collections::HashMap;

use crate::app::AppState;
use crate::app::presentation::fire_effects::ProjectileVisual;
use crate::map::lighting::DEFAULT_TINT;
use crate::map::overlay_types::is_bridge_overlay_index;
use crate::map::terrain::{self, TILE_HEIGHT, TILE_WIDTH};
use crate::render::batch::SpriteInstance;
use crate::render::bridge_atlas::is_high_bridge_body_name;
use crate::render::overlay_atlas::{CRATE_BODY_FRAME, OverlaySpriteKey};
use crate::render::sprite_atlas::ShpSpriteKey;
use crate::render::tactical_draw_plan::{
    BlitPolicy, ObjectDraw, RenderZPolicy, SpriteEncoding, TacticalCoord,
};
use crate::rules::art_data::{AnimLayer, AnimTypeRuntimeConfig, anim_translucency_source_alpha};
use crate::rules::house_colors::HouseColorIndex;
use crate::sim::components::WeaponMuzzleFlash;
use crate::sim::projectile::ProjectileCoord;
use crate::util::fixed_math::SimFixed;

use super::helpers::{
    ANIM_DRAW_DEPTH_BIAS_PX, apply_shape_z_adjust, compute_sprite_depth,
    compute_sprite_depth_params, in_view,
};

/// Map terrain entries remain the render metadata source, but once rules-backed
/// simulation authority exists the live cell index decides whether an instance
/// still exists. This keeps loading/fallback screens static without letting a
/// destroyed runtime object remain visible forever.
#[cfg(test)]
fn terrain_object_is_render_visible(
    object: &crate::map::overlay::TerrainObject,
    rules: Option<&crate::rules::ruleset::RuleSet>,
    authority: Option<&crate::sim::production::ProductionState>,
) -> bool {
    let (Some(rules), Some(production)) = (rules, authority) else {
        return true;
    };
    if rules
        .terrain_object_type_case_insensitive(&object.name)
        .is_none()
    {
        return true;
    }
    let Some(stable_id) = production.terrain_object_cells.get(&(object.rx, object.ry)) else {
        return false;
    };
    production
        .terrain_objects
        .get(stable_id)
        .is_some_and(|terrain| terrain.is_live() && terrain.cell() == (object.rx, object.ry))
}

/// Project an AnimClass-like fixed world effect from its exact cell/subcell anchor.
///
/// Native `ObjectClass::DrawIt` passes the effect's CoordStruct directly through
/// `CoordsToClient2`; it does not convert the effect to a terrain-tile origin first.
pub(crate) fn world_effect_screen_position(
    rx: u16,
    ry: u16,
    sub_x: SimFixed,
    sub_y: SimFixed,
    z: u8,
) -> (f32, f32) {
    crate::util::lepton::lepton_to_screen(rx, ry, sub_x, sub_y, z)
}

/// Body frame the native overlay draw selects for a non-bridge overlay cell.
///
/// `Crate=yes` overlays take a dedicated branch that hardcodes frame 0; every
/// other overlay draws its cell overlay-data byte directly (ore density, wall
/// `damage << 4 | connectivity`).
fn overlay_body_frame(is_crate: bool, overlay_data: u8) -> u8 {
    if is_crate {
        CRATE_BODY_FRAME
    } else {
        overlay_data
    }
}

/// Resolve the CellClass overlay identity/data used by the tactical overlay
/// draw. Low-bridge damage and repair mutate the identity in place, so a map
/// pack entry is only the iteration anchor once live cell authority exists.
fn overlay_render_identity(
    static_overlay_id: u8,
    static_overlay_data: u8,
    live_cell: Option<&crate::sim::overlay_grid::OverlayCell>,
) -> Option<(u8, u8)> {
    let Some(live_cell) = live_cell else {
        return Some((static_overlay_id, static_overlay_data));
    };
    if is_bridge_overlay_index(static_overlay_id) {
        return live_cell
            .overlay_id
            .map(|overlay_id| (overlay_id, live_cell.overlay_data));
    }
    (live_cell.overlay_id == Some(static_overlay_id))
        .then_some((static_overlay_id, live_cell.overlay_data))
}

/// Choose the display-only identity for a flat resource cell.
///
/// Active YR `CellClass__DrawOverlay_Body @ 0x0047F6A0` keeps the Cell's
/// overlay identity/data as resource state but selects a coordinate-derived
/// flat image when the resolved cell is not sloped. Missing render metadata,
/// invalid signed indices, slopes, and non-resource overlays retain the live
/// identity without approximation.
fn overlay_display_identity(
    live_overlay_id: u8,
    overlay_data: u8,
    rx: u16,
    ry: u16,
    slope_type: Option<u8>,
    overlay_registry: Option<&crate::map::overlay_types::OverlayTypeRegistry>,
    tiberium_types: Option<&crate::rules::tiberium_type::TiberiumTypeRegistry>,
) -> (u8, u8) {
    let (Some(overlay_registry), Some(tiberium_types)) = (overlay_registry, tiberium_types) else {
        return (live_overlay_id, overlay_data);
    };
    if slope_type != Some(0)
        || !overlay_registry
            .flags(live_overlay_id)
            .is_some_and(|flags| flags.tiberium)
    {
        return (live_overlay_id, overlay_data);
    }

    let display_overlay_id = overlay_registry
        .flat_tiberium_display_overlay_id(tiberium_types, live_overlay_id, rx, ry)
        .unwrap_or(live_overlay_id);
    (display_overlay_id, overlay_data)
}

/// Build SpriteInstances for active world-position effects (warp sparkles, etc.).
///
/// Appends to the SHP instance list so they draw in the same depth-sorted pass.
/// Each effect's current frame is looked up in the SHP atlas.
pub(crate) fn build_world_effect_instances(state: &AppState, paged: &mut [Vec<SpriteInstance>]) {
    let (sim, atlas) = match (state.match_state.sim_runtime.as_ref().map(|rt| &rt.simulation), &state.match_state.match_presentation.sprite_atlas) {
        (Some(s), Some(a)) => (s, a),
        _ => return,
    };
    let z = state.match_state.input.zoom_level;
    let (cam_x, cam_y, sw, sh) = (
        state.match_state.input.camera_x,
        state.match_state.input.camera_y,
        state.render_width() as f32 / z,
        state.render_height() as f32 / z,
    );
    for fx in &sim.world_effects {
        if fx.delay_frames > 0 {
            continue;
        }
        let (center_x, center_y) =
            world_effect_screen_position(fx.rx, fx.ry, fx.sub_x, fx.sub_y, fx.z);
        if !in_view(
            center_x - TILE_WIDTH / 2.0,
            center_y - TILE_HEIGHT / 2.0,
            TILE_WIDTH,
            TILE_HEIGHT,
            cam_x,
            cam_y,
            sw,
            sh,
            120.0,
        ) {
            continue;
        }
        let shp_name: &str = sim.interner.resolve(fx.shp_name);
        let key = ShpSpriteKey {
            type_id: shp_name.to_string(),
            facing: 0,
            frame: fx.frame,
            house_color: HouseColorIndex(0),
        };
        let Some(entry) = atlas.get(&key) else {
            continue;
        };
        let depth_y: f32 = center_y + entry.offset_y + entry.pixel_size[1];
        let base_depth: f32 = compute_sprite_depth(state, depth_y, fx.z);
        let cfg: Option<&AnimTypeRuntimeConfig> = state.rules()
            .and_then(|rules| rules.art_registry.anim_runtime_config(shp_name));
        // Anim SHP draws carry the type's ZAdjust= sort bias plus the
        // constant -2px anim bias (negative = toward camera).
        let type_z_adjust: i32 = cfg.map(|c| c.z_adjust).unwrap_or(0);
        let world_height: f32 = state
            .match_state.match_presentation.terrain_grid
            .as_ref()
            .map(|g| g.world_height)
            .unwrap_or(1.0);
        let depth: f32 = apply_shape_z_adjust(
            base_depth,
            type_z_adjust + ANIM_DRAW_DEPTH_BIAS_PX,
            world_height,
        );
        let tint: [f32; 3] = state.match_state.match_presentation.lighting_grid.anim_tint_at((fx.rx, fx.ry), cfg);
        // Source-pixel weight the native blitter family gives this frame:
        // a fixed 25/50/75 stage from `Translucency=`, or the progressive
        // `Translucent=yes` fade keyed on the frame against the type's End.
        let alpha: f32 = anim_instance_alpha(cfg, i32::from(fx.frame), i32::from(fx.total_frames));
        paged[entry.page as usize].push(SpriteInstance {
            position: [center_x + entry.offset_x, center_y + entry.offset_y],
            size: entry.pixel_size,
            uv_origin: entry.uv_origin,
            uv_size: entry.uv_size,
            depth,
            tint,
            alpha,
            ..Default::default()
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnimRenderDestination {
    Ground(ObjectDraw),
    Top,
    Existing,
}

fn anim_render_destination(
    stable_id: u64,
    owner_entity: Option<u64>,
    world_coord: crate::sim::anim_class::AnimWorldCoord,
    config: Option<&AnimTypeRuntimeConfig>,
    ground_order: &crate::app::presentation::render::draw_plan_lowering::NativeGroundOrder,
) -> Option<AnimRenderDestination> {
    // Owner-attached damage fire retains its established parent-adjacent path.
    // gamemd-derived: no-owner `AnimClass::GetLayer @ 0x00424CB0` returns the
    // AnimType layer; `GetYSort @ 0x00422BC0` adds AnimType YSortAdjust.
    if owner_entity.is_some() {
        return Some(AnimRenderDestination::Existing);
    }
    let config = config?;
    match config.layer {
        AnimLayer::Ground => ground_order
            .anim_object_draw(
                stable_id,
                TacticalCoord {
                    x: world_coord.x,
                    y: world_coord.y,
                    z: world_coord.z,
                },
                config.y_sort_adjust,
            )
            .map(AnimRenderDestination::Ground),
        AnimLayer::Top => Some(AnimRenderDestination::Top),
        AnimLayer::Other(_) => Some(AnimRenderDestination::Existing),
    }
}

/// Build ordinary scheduler-owned `AnimClass` sprites.
pub(crate) fn build_anim_class_instances(
    state: &AppState,
    paged: &mut [Vec<SpriteInstance>],
    top_instances: &mut Vec<SpriteInstance>,
    top_pages: &mut Vec<usize>,
    top_ids: &mut Vec<u64>,
    ground_objects: &mut Vec<crate::app::presentation::render::draw_plan_lowering::PlannedGroundObjectInstance>,
    ground_order: &crate::app::presentation::render::draw_plan_lowering::NativeGroundOrder,
) {
    let (sim, atlas) = match (state.match_state.sim_runtime.as_ref().map(|rt| &rt.simulation), &state.match_state.match_presentation.sprite_atlas) {
        (Some(s), Some(a)) => (s, a),
        _ => return,
    };
    let z2 = state.match_state.input.zoom_level;
    let (cam_x, cam_y, sw, sh) = (
        state.match_state.input.camera_x,
        state.match_state.input.camera_y,
        state.render_width() as f32 / z2,
        state.render_height() as f32 / z2,
    );
    for &stable_id in sim.tactical_registration_order() {
        let Some(anim) = sim.anim(stable_id) else {
            continue;
        };
        if anim.runtime.inactive {
            continue;
        }
        let type_name: &str = sim.interner.resolve(anim.type_id);
        let config = state.rules()
            .and_then(|rules| rules.art_registry.anim_runtime_config(type_name));
        if !crate::sim::anim_class::anim_draw_detail_visible(
            crate::sim::anim_class::AnimDrawDetailInput {
                // No authoritative draw-rate degradation producer exists yet.
                frame_rate_below_minimum: false,
                type_detail_level: config.map_or(0, |value| value.detail_level),
                game_detail_level: state.match_state.match_presentation.in_game_options.detail_level as i32,
                hidden: anim.draw_runtime.hidden,
                special_hidden: anim.draw_runtime.special_hidden,
                // The native special-hide type bit remains an explicit residual.
                type_special_hide: false,
            },
        ) {
            continue;
        }
        let Ok(frame) = u16::try_from(anim.runtime.current_frame) else {
            continue;
        };
        let (center_x, center_y, rx, ry, z) = anim_world_render_coords(anim.world_coord);
        if !in_view(
            center_x, center_y, 200.0, 200.0, cam_x, cam_y, sw, sh, 200.0,
        ) {
            continue;
        }
        let tint = state.match_state.match_presentation.lighting_grid.anim_tint_at((rx, ry), config);
        let key = ShpSpriteKey {
            type_id: type_name.to_string(),
            facing: 0,
            frame,
            house_color: HouseColorIndex(0),
        };
        let Some(entry) = atlas.get(&key) else {
            continue;
        };
        let Some(alpha) = anim_instance_alpha_with_flags(
            config,
            anim.draw_flags,
            anim.runtime.current_frame,
            i32::from(
                presentation_anim_frame_count(&atlas.active_anim_frame_counts, type_name)
                    .unwrap_or(0),
            ),
            state.match_state.match_presentation.in_game_options.detail_level as i32,
            anim.draw_runtime,
        ) else {
            continue;
        };
        let (origin_y, world_height) = state
            .match_state.match_presentation.terrain_grid
            .as_ref()
            .map(|grid| (grid.origin_y, grid.world_height))
            .unwrap_or((0.0, 1.0));
        let fire_depth = compute_sprite_depth_params(origin_y, world_height, center_y, z);
        debug_assert!(!anim.terrain_attached || anim.use_cell_drawer);
        let instance = SpriteInstance {
            position: [center_x + entry.offset_x, center_y + entry.offset_y],
            size: entry.pixel_size,
            uv_origin: entry.uv_origin,
            uv_size: entry.uv_size,
            depth: apply_shape_z_adjust(
                fire_depth,
                anim.z_adjust + ANIM_DRAW_DEPTH_BIAS_PX,
                world_height,
            ),
            tint,
            alpha,
            ..Default::default()
        };
        match anim_render_destination(
            anim.stable_id,
            anim.owner_entity,
            anim.world_coord,
            config,
            ground_order,
        ) {
            Some(AnimRenderDestination::Ground(parent)) => ground_objects.push(
                crate::app::presentation::render::draw_plan_lowering::PlannedGroundObjectInstance::object(
                    parent,
                    vec![crate::app::presentation::render::draw_plan_lowering::GroundPieceInstance {
                        target: crate::app::presentation::render::draw_plan_lowering::GroundTexture::ShpPage(
                            entry.page as usize,
                        ),
                        instance,
                    }],
                ),
            ),
            Some(AnimRenderDestination::Top) => {
                top_instances.push(instance);
                top_pages.push(entry.page as usize);
                top_ids.push(anim.stable_id);
            }
            Some(AnimRenderDestination::Existing) => {
                paged[entry.page as usize].push(instance);
            }
            None => {}
        }
    }
}

/// Source-pixel weight one animation frame draws with.
///
/// gamemd picks a blitter family per draw from the art type's `Translucency=` /
/// `Translucent=` keys; `anim_frame_source_alpha` reproduces that selection and
/// returns the weight the family gives the incoming sprite pixel. An art type
/// the registry does not know draws opaque, matching the resolver's own
/// no-keys result.
fn anim_instance_alpha(
    config: Option<&AnimTypeRuntimeConfig>,
    current_frame: i32,
    shp_frame_count: i32,
) -> f32 {
    anim_instance_alpha_with_flags(
        config,
        0,
        current_frame,
        shp_frame_count,
        2,
        crate::sim::anim_class::AnimDrawRuntime::default(),
    )
    .unwrap_or(0.0)
}

fn anim_instance_alpha_with_flags(
    config: Option<&AnimTypeRuntimeConfig>,
    base_flags: u32,
    current_frame: i32,
    shp_frame_count: i32,
    game_detail_level: i32,
    draw_runtime: crate::sim::anim_class::AnimDrawRuntime,
) -> Option<f32> {
    let result = crate::sim::anim_class::anim_translucency_selection(
        crate::sim::anim_class::AnimTranslucencyInput {
            base_flags,
            forced_translucent: draw_runtime.forced_translucent,
            forced_uses_75: draw_runtime.forced_uses_75,
            translucency_detail_level: config.map_or(0, |value| value.translucency_detail_level),
            // Draw-time detail is presentation state and does not enter simulation.
            game_detail_level,
            translucent_ramp: config.is_some_and(|value| value.translucent),
            current_frame,
            frame_count: config
                .and_then(|value| value.raw_shp_frame_count)
                .unwrap_or(shp_frame_count),
            explicit_translucency: config.map_or(0, |value| value.translucency),
            instance_ramp: i32::from(draw_runtime.translucency_ramp),
        },
    );
    result
        .draw
        .then(|| anim_translucency_source_alpha(result.flags))
}

/// SHP header frame count for an animation type, as the translucency resolver
/// wants it.
///
/// `anim_frame_source_alpha` only consults this on the `Translucent=yes`
/// progressive path, and only for a type that never went through asset binding
/// (no `raw_shp_frame_count`, no explicit `End=`). This is presentation-only:
/// simulation timing reads the immutable rules-side asset catalog. Zero when
/// the atlas has no matching shape.
fn anim_shp_frame_count(state: &AppState, type_name: &str) -> i32 {
    state
        .match_state.match_presentation.sprite_atlas
        .as_ref()
        .and_then(|atlas| presentation_anim_frame_count(&atlas.active_anim_frame_counts, type_name))
        .map(i32::from)
        .unwrap_or(0)
}

fn presentation_anim_frame_count(
    frame_counts: &HashMap<String, u16>,
    type_name: &str,
) -> Option<u16> {
    frame_counts.get(type_name).copied().or_else(|| {
        let canonical = type_name.to_ascii_uppercase();
        frame_counts.get(&canonical).copied()
    })
}

fn anim_world_render_coords(
    world: crate::sim::anim_class::AnimWorldCoord,
) -> (f32, f32, u16, u16, u8) {
    const CELL_LEPTONS: i32 = 256;
    const HEIGHT_LEVEL_LEPTONS: i32 = 128;
    let rx = world
        .x
        .div_euclid(CELL_LEPTONS)
        .clamp(0, i32::from(u16::MAX)) as u16;
    let ry = world
        .y
        .div_euclid(CELL_LEPTONS)
        .clamp(0, i32::from(u16::MAX)) as u16;
    let sub_x = crate::util::fixed_math::SimFixed::from_num(world.x.rem_euclid(CELL_LEPTONS));
    let sub_y = crate::util::fixed_math::SimFixed::from_num(world.y.rem_euclid(CELL_LEPTONS));
    let z = world
        .z
        .div_euclid(HEIGHT_LEVEL_LEPTONS)
        .clamp(0, i32::from(u8::MAX)) as u8;
    let (screen_x, screen_y) = crate::util::lepton::lepton_to_screen(rx, ry, sub_x, sub_y, z);
    (screen_x, screen_y, rx, ry, z)
}

/// Build SpriteInstances for visible overlay objects and terrain objects.
///
/// Bridge body, body shadow, and railing instances are emitted separately by
/// `instances::bridges` (Phase D). Low bridges (LOBRDG*) ride in the
/// generic `instances` bucket and use the regular overlay atlas.
pub(crate) fn build_overlay_instances(
    state: &AppState,
    sw: f32,
    sh: f32,
    instances: &mut Vec<SpriteInstance>,
    ground_objects: &mut Vec<crate::app::presentation::render::draw_plan_lowering::PlannedGroundObjectInstance>,
    ground_order: &crate::app::presentation::render::draw_plan_lowering::NativeGroundOrder,
) {
    let atlas = match &state.match_state.match_presentation.overlay_atlas {
        Some(a) => a,
        None => return,
    };
    let (cam_x, cam_y) = (state.match_state.input.camera_x, state.match_state.input.camera_y);
    let (origin_y, world_height) = state
        .match_state.match_presentation.terrain_grid
        .as_ref()
        .map(|g| (g.origin_y, g.world_height))
        .unwrap_or((0.0, 1.0));

    // Playable area bounds — skip overlays outside LocalSize (border filler).
    let local_bounds = state.match_state.match_presentation.terrain_grid.as_ref().and_then(|g| g.local_bounds);

    // Cell visibility for the local owner — used to cull overlays and terrain
    // objects in unrevealed cells. The shroud multiply pass darkens per-pixel,
    // but tall sprites (bridges, trees) extend their canopy into screen-space
    // owned by neighboring cells; if those neighbors are revealed, the canopy
    // shows above the shroud edge. gamemd gates these renders on the cell's
    // explored bit. Computed once and shared by both loops below.
    let cell_visibility_fog: Option<(
        crate::sim::intern::InternedId,
        &crate::sim::vision::FogState,
    )> = if state.match_state.sandbox_full_visibility {
        None
    } else {
        let local_owner_name = crate::app::input::commands::preferred_local_owner_name(state);
        match (state.match_state.sim_runtime.as_ref().map(|rt| &rt.simulation), &local_owner_name) {
            (Some(sim), Some(owner)) => sim.interner.get(owner).map(|id| (id, &sim.fog)),
            _ => None,
        }
    };

    // Overlay entries from [OverlayPack]. `YR TacticalClass::Draw` keeps walls
    // in the fixed cell overlay family, not the `LayerClass` object sort.
    let mut planned_cells = Vec::new();
    let mut next_draw_id = 0u64;
    for entry in state.match_state.match_presentation.overlays.iter() {
        if let Some((owner_id, fog)) = cell_visibility_fog {
            if !fog.is_cell_revealed(owner_id, entry.rx, entry.ry) {
                continue;
            }
        }

        let Some(static_name) = state.match_state.match_presentation.overlay_names.get(&entry.overlay_id) else {
            continue;
        };

        // High-bridge bodies are emitted by `instances::bridges` reading
        // `BridgeRuntimeCell` post-tick. Skip them here so they don't double-
        // render via the static map overlay list.
        if is_high_bridge_body_name(static_name) {
            continue;
        }

        // Low bridges remain in the ordinary overlay pass, but CellClass's
        // live identity—not the map-pack seed—selects damaged/collapsed art.
        let live_overlay_cell = state
            .match_state.sim_runtime
            .as_ref()
            .map(|rt| &rt.simulation)
            .and_then(|sim| sim.overlay_grid.as_ref())
            .map(|grid| grid.cell(entry.rx, entry.ry));
        let Some((live_overlay_id, live_overlay_data)) =
            overlay_render_identity(entry.overlay_id, entry.frame, live_overlay_cell)
        else {
            continue;
        };
        let overlay_registry = state.overlay_registry();
        let overlay_flags = overlay_registry.and_then(|reg| reg.flags(live_overlay_id));
        let slope_type = state
            .terrain_template()
            .and_then(|terrain| terrain.cell(entry.rx, entry.ry))
            .map(|cell| cell.slope_type);
        let (display_overlay_id, live_overlay_data) = overlay_display_identity(
            live_overlay_id,
            live_overlay_data,
            entry.rx,
            entry.ry,
            slope_type,
            overlay_registry,
            state.rules().map(|rules| &rules.tiberium_types),
        );
        let name = if display_overlay_id == live_overlay_id {
            state.match_state.match_presentation.overlay_names.get(&live_overlay_id).cloned()
        } else {
            overlay_registry
                .and_then(|registry| registry.name(display_overlay_id).map(str::to_owned))
        };
        let Some(name) = name else {
            continue;
        };
        if is_high_bridge_body_name(&name) {
            continue;
        }

        // The live identity owns resource type, wall/crate flags, and bridge
        // state even when a flat resource cell selects another display image.
        let is_wall: bool = overlay_flags.map(|f| f.wall).unwrap_or(false);
        let is_crate: bool = overlay_flags.map(|f| f.crate_type).unwrap_or(false);
        let render_frame: u8 = overlay_body_frame(is_crate, live_overlay_data);

        // FA2 IsoView.cpp:5955-5956: track overlays render +CellHeight (15px) lower.
        let track_y_offset: f32 = if overlay_flags.map(|f| f.track).unwrap_or(false) {
            15.0
        } else {
            0.0
        };

        let z: u8 = state
            .height_map()
            .get(&(entry.rx, entry.ry))
            .copied()
            .unwrap_or(0);
        let (screen_x, screen_y) = terrain::iso_to_screen(entry.rx, entry.ry, z);
        let screen_y: f32 = screen_y + track_y_offset;

        // Playable area bounds — skip overlays outside LocalSize (border filler).
        if let Some(ref bounds) = local_bounds {
            if !bounds.contains(screen_x, screen_y) {
                continue;
            }
        }
        if !in_view(
            screen_x, screen_y, 120.0, 120.0, cam_x, cam_y, sw, sh, 120.0,
        ) {
            continue;
        }

        let key = OverlaySpriteKey {
            name: name.clone(),
            frame: render_frame,
        };
        let key_fallback = OverlaySpriteKey {
            name: name.clone(),
            frame: 0,
        };
        let spr = atlas.get(&key).or_else(|| atlas.get(&key_fallback));
        let Some(spr) = spr else { continue };
        let depth_z: u8 = z;
        let depth: f32 = compute_sprite_depth_params(origin_y, world_height, screen_y, depth_z);
        let tint: [f32; 3] = state.match_state.match_presentation.lighting_grid.overlay_tint_at((entry.rx, entry.ry));
        planned_cells.push(crate::app::presentation::render::draw_plan_lowering::PlannedCellInstance {
            draw: crate::render::tactical_draw_plan::CellDraw {
                id: next_draw_id,
                kind: crate::app::presentation::render::draw_plan_lowering::cell_draw_kind(is_wall),
                policy: BlitPolicy::translucent(SpriteEncoding::Terrain, RenderZPolicy::None),
            },
            instance: SpriteInstance {
                position: [
                    screen_x + TILE_WIDTH / 2.0 + spr.offset_x,
                    screen_y + TILE_HEIGHT / 2.0 + spr.offset_y,
                ],
                size: spr.pixel_size,
                uv_origin: spr.uv_origin,
                uv_size: spr.uv_size,
                depth,
                tint,
                alpha: 1.0,
                ..Default::default()
            },
        });
        next_draw_id += 1;
    }

    instances.extend(crate::app::presentation::render::draw_plan_lowering::lower_cell_instances(
        planned_cells,
    ));

    if std::env::var("RA2_DEBUG_BRIDGE_RENDER_BUCKETS").is_ok() {
        log::debug!("Cell overlay instances: {}", instances.len());
    }

    // Terrain objects from [Terrain] section.
    // FA2 IsoView.cpp:6389 applies a -3px Y fudge to terrain objects (trees, rocks):
    //   drawy = ... + f_y/2 - 3 - pic.wMaxHeight/2
    const TERRAIN_OBJECT_Y_FUDGE: f32 = -3.0;

    let Some(sim) = state.match_state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
        return;
    };
    for obj in sim.production.terrain_objects.values() {
        if !obj.is_live() || !obj.in_logic_vector {
            continue;
        }
        let name = sim.interner.resolve(obj.type_ref);
        if let Some((owner_id, fog)) = cell_visibility_fog {
            if !fog.is_cell_revealed(owner_id, obj.rx, obj.ry) {
                continue;
            }
        }

        let z: u8 = state
            .height_map()
            .get(&(obj.rx, obj.ry))
            .copied()
            .unwrap_or(0);
        let (screen_x, screen_y) = terrain::iso_to_screen(obj.rx, obj.ry, z);
        if let Some(ref bounds) = local_bounds {
            if !bounds.contains(screen_x, screen_y) {
                continue;
            }
        }
        if !in_view(
            screen_x, screen_y, 120.0, 120.0, cam_x, cam_y, sw, sh, 120.0,
        ) {
            continue;
        }

        // Animated terrain objects (flags) cycle through all frames using the
        // global idle animation timer. Static terrain uses frame 0.
        let frame: u8 = if let Some(count) = atlas.terrain_anim_frame_count(name) {
            // RA2 terrain animation rate: ~83ms per frame (12 fps).
            const TERRAIN_ANIM_RATE_MS: u32 = 83;
            let tick = state.match_state.match_presentation.idle_anim_elapsed_ms / TERRAIN_ANIM_RATE_MS;
            (tick % count as u32) as u8
        } else {
            0
        };
        let key = OverlaySpriteKey {
            name: name.to_string(),
            frame,
        };
        let Some(spr) = atlas.get(&key) else { continue };

        let depth: f32 = compute_sprite_depth_params(origin_y, world_height, screen_y, z);
        let spawns_tiberium = state.rules()
            .and_then(|rules| rules.terrain_object_type_case_insensitive(name))
            .map(|terrain_type| terrain_type.spawns_tiberium)
            .unwrap_or(false);
        let tint: [f32; 3] = state
            .match_state.match_presentation.lighting_grid
            .terrain_object_tint_for_type((obj.rx, obj.ry), spawns_tiberium);

        let Some(parent) = ground_order.terrain_object_draw(obj.stable_id, obj.rx, obj.ry) else {
            continue;
        };
        ground_objects.push(
            crate::app::presentation::render::draw_plan_lowering::PlannedGroundObjectInstance::object(
                parent,
                vec![crate::app::presentation::render::draw_plan_lowering::GroundPieceInstance {
                    target: crate::app::presentation::render::draw_plan_lowering::GroundTexture::OverlayAtlas,
                    instance: SpriteInstance {
                        position: [
                            screen_x + TILE_WIDTH / 2.0 + spr.offset_x,
                            screen_y + TILE_HEIGHT / 2.0 + spr.offset_y + TERRAIN_OBJECT_Y_FUDGE,
                        ],
                        size: spr.pixel_size,
                        uv_origin: spr.uv_origin,
                        uv_size: spr.uv_size,
                        depth,
                        tint,
                        alpha: 1.0,
                        ..Default::default()
                    },
                }],
            ),
        );
    }
}

/// Build SpriteInstances for garrison muzzle flash animations (OccupantAnim).
///
/// Each flash is positioned at the building's screen origin + pixel offset
/// from art.ini MuzzleFlashN. Mirrors `build_damage_fire_instances` but reads
/// from the `AppState.garrison_muzzle_flashes` queue instead of per-entity overlays.
pub(crate) fn build_garrison_muzzle_flash_instances(
    state: &AppState,
    paged: &mut [Vec<SpriteInstance>],
) {
    let (atlas, art_reg) = match (&state.match_state.match_presentation.sprite_atlas, state.rules().map(|rules| &rules.art_registry)) {
        (Some(a), Some(r)) => (a, r),
        _ => return,
    };
    let z = state.match_state.input.zoom_level;
    let (cam_x, cam_y, sw, sh) = (
        state.match_state.input.camera_x,
        state.match_state.input.camera_y,
        state.render_width() as f32 / z,
        state.render_height() as f32 / z,
    );
    let (origin_y, world_height) = state
        .match_state.match_presentation.terrain_grid
        .as_ref()
        .map(|g| (g.origin_y, g.world_height))
        .unwrap_or((0.0, 1.0));

    for flash in &state.match_state.match_presentation.garrison_muzzle_flashes {
        if !in_view(
            flash.screen_x,
            flash.screen_y,
            200.0,
            200.0,
            cam_x,
            cam_y,
            sw,
            sh,
            200.0,
        ) {
            continue;
        }
        let cfg: Option<&AnimTypeRuntimeConfig> =
            art_reg.anim_runtime_config(&flash.runtime.type_name);
        let start = cfg.map(|config| config.start).unwrap_or(0);
        let frame = (start + flash.runtime.current_frame).max(0) as u16;
        let key = ShpSpriteKey {
            type_id: flash.runtime.type_name.clone(),
            facing: 0,
            frame,
            house_color: HouseColorIndex(0),
        };
        let Some(entry) = atlas.get(&key) else {
            continue;
        };
        let fx: f32 = flash.screen_x + entry.offset_x;
        let fy: f32 = flash.screen_y + entry.offset_y;
        let tint: [f32; 3] = state.match_state.match_presentation.lighting_grid.anim_tint_at((flash.rx, flash.ry), cfg);
        let depth: f32 = garrison_flash_depth(
            origin_y,
            world_height,
            flash.screen_y,
            flash.z,
            flash.z_adjust,
        );
        // (flash.z_adjust carries the native occupied-building value, e.g. -200,
        // applied as a toward-camera sort bias inside garrison_flash_depth.)
        let alpha: f32 = anim_instance_alpha(
            cfg,
            flash.runtime.current_frame,
            anim_shp_frame_count(state, &flash.runtime.type_name),
        );
        paged[entry.page as usize].push(SpriteInstance {
            position: [fx, fy],
            size: entry.pixel_size,
            uv_origin: entry.uv_origin,
            uv_size: entry.uv_size,
            depth,
            tint,
            alpha,
            ..Default::default()
        });
    }
}

fn garrison_flash_depth(
    origin_y: f32,
    world_height: f32,
    screen_y: f32,
    z: u8,
    z_adjust: i32,
) -> f32 {
    // ZAdjust is a signed pixel sort bias with neutral 0; negative pulls the
    // flash toward the camera (the occupied-building flash uses -200 so it
    // draws in front of the wall). The 1000-neutral convention belongs to
    // the per-cell terrain z path, not anim draws. Anim SHP draws also carry
    // the constant -2px bias.
    let base_depth = compute_sprite_depth_params(origin_y, world_height, screen_y, z);
    apply_shape_z_adjust(base_depth, z_adjust + ANIM_DRAW_DEPTH_BIAS_PX, world_height)
}

fn weapon_muzzle_flash_key(flash: &WeaponMuzzleFlash) -> ShpSpriteKey {
    ShpSpriteKey {
        type_id: flash.shp_name.clone(),
        facing: 0,
        frame: flash.frame,
        house_color: HouseColorIndex(0),
    }
}

/// Build SpriteInstances for non-garrison weapon muzzle flash animations.
///
/// These flashes are spawned at a fixed FLH fire origin when combat emits a
/// non-garrison fire event with a weapon `Anim=` entry.
pub(crate) fn build_weapon_muzzle_flash_instances(
    state: &AppState,
    paged: &mut [Vec<SpriteInstance>],
) {
    let atlas = match &state.match_state.match_presentation.sprite_atlas {
        Some(a) => a,
        None => return,
    };
    let z = state.match_state.input.zoom_level;
    let (cam_x, cam_y, sw, sh) = (
        state.match_state.input.camera_x,
        state.match_state.input.camera_y,
        state.render_width() as f32 / z,
        state.render_height() as f32 / z,
    );
    let (origin_y, world_height) = state
        .match_state.match_presentation.terrain_grid
        .as_ref()
        .map(|g| (g.origin_y, g.world_height))
        .unwrap_or((0.0, 1.0));

    for flash in &state.match_state.match_presentation.weapon_muzzle_flashes {
        if !in_view(
            flash.screen_x,
            flash.screen_y,
            96.0,
            96.0,
            cam_x,
            cam_y,
            sw,
            sh,
            96.0,
        ) {
            continue;
        }
        let key = weapon_muzzle_flash_key(flash);
        let Some(entry) = atlas.get(&key) else {
            continue;
        };
        let cfg: Option<&AnimTypeRuntimeConfig> = state.rules()
            .and_then(|rules| rules.art_registry.anim_runtime_config(&flash.shp_name));
        let tint = state.match_state.match_presentation.lighting_grid.anim_tint_at((flash.rx, flash.ry), cfg);
        // Muzzle anims (e.g. GCMUZZLE, VTMUZZLE) carry their art section's
        // ZAdjust= as a sort bias plus the constant -2px anim bias.
        let type_z_adjust: i32 = cfg.map(|c| c.z_adjust).unwrap_or(0);
        let base_depth =
            compute_sprite_depth_params(origin_y, world_height, flash.screen_y, flash.z);
        let depth = apply_shape_z_adjust(
            base_depth,
            type_z_adjust + ANIM_DRAW_DEPTH_BIAS_PX,
            world_height,
        );
        let alpha: f32 =
            anim_instance_alpha(cfg, i32::from(flash.frame), i32::from(flash.total_frames));
        paged[entry.page as usize].push(SpriteInstance {
            position: [
                flash.screen_x + entry.offset_x,
                flash.screen_y + entry.offset_y,
            ],
            size: entry.pixel_size,
            uv_origin: entry.uv_origin,
            uv_size: entry.uv_size,
            depth,
            tint,
            alpha,
            ..Default::default()
        });
    }
}

fn projectile_visual_key(projectile: &ProjectileVisual) -> ShpSpriteKey {
    ShpSpriteKey {
        type_id: projectile.shp_name.clone(),
        facing: 0,
        frame: projectile.frame,
        house_color: HouseColorIndex(0),
    }
}

fn projectile_authoritative_screen_position(
    coordinate: ProjectileCoord,
) -> Option<(f32, f32, u16, u16, u8)> {
    let rx = u16::try_from(coordinate.x.div_euclid(256)).ok()?;
    let ry = u16::try_from(coordinate.y.div_euclid(256)).ok()?;
    let sub_x = SimFixed::from_num(coordinate.x.rem_euclid(256));
    let sub_y = SimFixed::from_num(coordinate.y.rem_euclid(256));
    let z = coordinate.z.clamp(0, i32::from(u8::MAX)) as u8;
    let (screen_x, screen_y) = crate::util::lepton::lepton_to_screen(rx, ry, sub_x, sub_y, z);
    Some((screen_x, screen_y, rx, ry, z))
}

/// Build visible persistent shots from `Simulation::projectiles`.
///
/// YR `BulletClass::AI` linkage: rendering reads the same committed CoordStruct
/// that the next authoritative flight pass will advance.
fn build_authoritative_projectile_instances(state: &AppState, paged: &mut [Vec<SpriteInstance>]) {
    let (sim, rules, atlas) = match (state.match_state.sim_runtime.as_ref().map(|rt| &rt.simulation), state.rules().map(|r| r), &state.match_state.match_presentation.sprite_atlas) {
        (Some(sim), Some(rules), Some(atlas)) => (sim, rules, atlas),
        _ => return,
    };
    let z = state.match_state.input.zoom_level;
    let (cam_x, cam_y, sw, sh) = (
        state.match_state.input.camera_x,
        state.match_state.input.camera_y,
        state.render_width() as f32 / z,
        state.render_height() as f32 / z,
    );
    let (origin_y, world_height) = state
        .match_state.match_presentation.terrain_grid
        .as_ref()
        .map(|grid| (grid.origin_y, grid.world_height))
        .unwrap_or((0.0, 1.0));

    for (_, projectile) in sim.projectiles.iter() {
        let Some(weapon) = rules.weapon(sim.interner.resolve(projectile.payload.weapon)) else {
            continue;
        };
        let Some(projectile_type_id) = weapon.projectile.as_deref() else {
            continue;
        };
        let Some(projectile_type) = rules.projectile(projectile_type_id) else {
            continue;
        };
        let Some(image) = projectile_type.image.as_deref() else {
            continue;
        };
        let Some((screen_x, screen_y, _rx, _ry, projectile_z)) =
            projectile_authoritative_screen_position(projectile.position)
        else {
            continue;
        };
        if !in_view(screen_x, screen_y, 96.0, 96.0, cam_x, cam_y, sw, sh, 96.0) {
            continue;
        }
        let frame_count =
            presentation_anim_frame_count(&atlas.active_anim_frame_counts, image).unwrap_or(32);
        let key = ShpSpriteKey {
            type_id: image.to_string(),
            facing: 0,
            frame: if frame_count == 0 {
                0
            } else {
                u16::from(crate::sim::projectile::projectile_shp_frame(projectile)) % frame_count
            },
            house_color: HouseColorIndex(0),
        };
        let Some(entry) = atlas.get(&key) else {
            continue;
        };
        // In-flight projectile shapes draw at FIXED full brightness: the native
        // bullet draw passes a literal neutral value for both its shadow and
        // body passes and never reads a cell lighting field at all. (An earlier
        // revision sampled the grid here; a merge briefly reintroduced that —
        // do not re-lit projectiles.)
        let tint = DEFAULT_TINT;
        let depth = compute_sprite_depth_params(origin_y, world_height, screen_y, projectile_z);
        paged[entry.page as usize].push(SpriteInstance {
            position: [screen_x + entry.offset_x, screen_y + entry.offset_y],
            size: entry.pixel_size,
            uv_origin: entry.uv_origin,
            uv_size: entry.uv_size,
            depth,
            tint,
            alpha: 1.0,
            ..Default::default()
        });
    }
}

/// Build SpriteInstances for render-only in-flight projectile visuals.
pub(crate) fn build_projectile_visual_instances(
    state: &AppState,
    paged: &mut [Vec<SpriteInstance>],
) {
    build_authoritative_projectile_instances(state, paged);
    let atlas = match &state.match_state.match_presentation.sprite_atlas {
        Some(a) => a,
        None => return,
    };
    let z = state.match_state.input.zoom_level;
    let (cam_x, cam_y, sw, sh) = (
        state.match_state.input.camera_x,
        state.match_state.input.camera_y,
        state.render_width() as f32 / z,
        state.render_height() as f32 / z,
    );
    let (origin_y, world_height) = state
        .match_state.match_presentation.terrain_grid
        .as_ref()
        .map(|g| (g.origin_y, g.world_height))
        .unwrap_or((0.0, 1.0));

    for projectile in &state.match_state.match_presentation.projectile_visuals {
        let t = projectile.progress();
        let screen_x =
            projectile.start_screen_x + (projectile.end_screen_x - projectile.start_screen_x) * t;
        let screen_y =
            projectile.start_screen_y + (projectile.end_screen_y - projectile.start_screen_y) * t;
        if !in_view(screen_x, screen_y, 96.0, 96.0, cam_x, cam_y, sw, sh, 96.0) {
            continue;
        }
        let key = projectile_visual_key(projectile);
        let Some(entry) = atlas.get(&key) else {
            continue;
        };
        // In-flight projectile shapes are not cell-lit at all. gamemd's bullet
        // draw passes the literal full-brightness value in the same argument
        // slot that the animation and techno draws fill from the cell, for both
        // the shadow and the body pass, and the only cell it looks up is for a
        // bridge-height flag bit. No interpolated cell is needed here.
        let tint = DEFAULT_TINT;
        let depth = compute_sprite_depth_params(origin_y, world_height, screen_y, projectile.z);
        paged[entry.page as usize].push(SpriteInstance {
            position: [screen_x + entry.offset_x, screen_y + entry.offset_y],
            size: entry.pixel_size,
            uv_origin: entry.uv_origin,
            uv_size: entry.uv_size,
            depth,
            tint,
            alpha: 1.0,
            ..Default::default()
        });
    }
}

/// Build persistent WaveClass polygon edges from simulation registration state.
pub(crate) fn build_weapon_wave_instances(state: &AppState) -> Vec<SpriteInstance> {
    let mut instances = Vec::new();
    let Some(sim) = state.match_state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
        return instances;
    };
    let observer = crate::app::input::commands::preferred_local_owner(state)
        .as_deref()
        .and_then(|owner| sim.interner.get(owner));
    for wave in crate::app::presentation::fire_effects::build_weapon_wave_visuals(sim, observer) {
        let projected: Vec<[f32; 2]> = crate::render::wave_geometry::draw_order(wave.geometry)
            .into_iter()
            .map(|point| {
                let (x, y) = crate::map::terrain::lepton_to_screen(glam::IVec3::new(
                    point.x, point.y, point.z,
                ));
                [x, y]
            })
            .collect();
        instances.extend(crate::render::wave_geometry::build_wave_instances(
            &projected, wave.tint, 1.0, 0.00045,
        ));
    }
    // Live BuildingLightRuntime producers are intentionally not lowered here:
    // native DrawExtras first uses SpotlightClass type 16's zero-blend
    // shape-blitter/light-mask path, which this renderer does not yet expose.
    // A white quad or alpha approximation would invent visible behavior.
    instances
}

/// Emit one sprite instance per active parachute anim, anchored at the
/// descending GI's screen position with the SHP atlas's pre-baked
/// offset_x/offset_y handling sprite-center anchoring.
///
/// Depth: chute depth = GI body depth − epsilon, so it sorts above the body
/// in the same Layer 2 (Ground) band — matching gamemd's
/// AnimClass::GetLayer override that forces owner-attached anims to Layer 2
/// regardless of art.ini Layer=.
///
/// The body's key arrives in `body_depths` from `build_shp_instances`, which
/// must therefore have run first this frame. It is not re-derived here: the
/// body's key is anchored on the ground row the GI is descending onto, not on
/// the row it is drawn at, and a paradrop starts 216 px above that row. A
/// canopy keyed off the drawn row would sort ~14 iso rows behind the man
/// hanging on it and disappear behind any building in between.
///
/// Palette: AltPalette=yes selects the unit/Convert palette in gamemd. This
/// matches the default palette branch in `sprite_atlas` so long as the
/// PARACH frames are NOT registered in `effect_type_ids` (see Task 8).
pub(crate) fn build_parachute_instances(
    state: &AppState,
    ground_objects: &mut [crate::app::presentation::render::draw_plan_lowering::PlannedGroundObjectInstance],
    body_depths: &super::shp::ParachuteBodyDepths,
) {
    /// Depth epsilon — chute sorts slightly above the GI body. Half of the
    /// per-Z bias used in `compute_sprite_depth_params`. Increase if
    /// z-fighting is observed in-game.
    const CHUTE_DEPTH_EPSILON: f32 = 0.0005;

    /// Vertical lift, in pixels, applied to the chute sprite so the canopy
    /// sits above the GI's head rather than centered on the body. Tunable;
    /// gamemd's PARACH SHP layout produces this offset implicitly through
    /// frame-internal positioning, which our atlas doesn't replicate exactly.
    const CHUTE_Y_LIFT: f32 = 8.0;

    let (sim, atlas) = match (state.match_state.sim_runtime.as_ref().map(|rt| &rt.simulation), &state.match_state.match_presentation.sprite_atlas) {
        (Some(s), Some(a)) => (s, a),
        _ => return,
    };
    let z = state.match_state.input.zoom_level;
    let (cam_x, cam_y, sw, sh) = (
        state.match_state.input.camera_x,
        state.match_state.input.camera_y,
        state.render_width() as f32 / z,
        state.render_height() as f32 / z,
    );
    let config = match state.rules()
        .and_then(|r| r.general.parachute_render.as_ref())
    {
        Some(c) => c,
        None => return,
    };

    for anim in &state.match_state.match_presentation.parachute_anims {
        let entity = match sim.entities().get(anim.target_id) {
            Some(e) => e,
            None => continue,
        };
        // The body's own key for this frame. Absent means the body was culled
        // or never emitted, in which case there is nothing for a canopy to
        // hang on and nothing to sort it against.
        let Some(&body_depth) = body_depths.get(&anim.target_id) else {
            continue;
        };
        // Draw the chute exactly where the body is drawn, so it follows the
        // airborne GI rather than the ground beneath it.
        let (gx, gy) = crate::render::locomotor_visual::screen_position(entity);
        if !in_view(gx, gy, 200.0, 200.0, cam_x, cam_y, sw, sh, 200.0) {
            continue;
        }

        // Single-facing anim (no Facings= in art.ini for PARACH).
        let key = ShpSpriteKey {
            type_id: config.shp_name.clone(),
            facing: 0,
            frame: anim.frame,
            house_color: HouseColorIndex(0),
        };
        let Some(entry) = atlas.get(&key) else {
            // PARACH not yet loaded into the atlas. Logged once at startup
            // by the atlas-load path; per-frame silence here is intentional.
            continue;
        };
        let cx: f32 = gx + entry.offset_x;
        let cy: f32 = gy + entry.offset_y - CHUTE_Y_LIFT;

        // No owner tint and no lighting tint: chutes look identical
        // regardless of dropping house, matching gamemd's AltPalette=yes
        // path (ColorScheme[0]'s ConvertPalette).
        let tint: [f32; 3] = [1.0, 1.0, 1.0];

        // Depth: GI body depth minus a small epsilon so the chute draws on
        // top. ZAdjust=-10 in gamemd is a depth-sort fudge with no precise
        // pixel mapping; in our depth-buffer rendering, lower depth = closer
        // to camera = on top.
        let depth = (body_depth - CHUTE_DEPTH_EPSILON).clamp(0.001, 0.999);

        let Some(parent) = ground_objects
            .iter_mut()
            .find(|object| object.parent.id == anim.target_id)
        else {
            continue;
        };
        parent
            .pieces
            .push(crate::app::presentation::render::draw_plan_lowering::GroundPieceInstance {
                target: crate::app::presentation::render::draw_plan_lowering::GroundTexture::ShpPage(
                    entry.page as usize,
                ),
                instance: SpriteInstance {
                    position: [cx, cy],
                    size: entry.pixel_size,
                    uv_origin: entry.uv_origin,
                    uv_size: entry.uv_size,
                    depth,
                    tint,
                    alpha: 1.0,
                    ..Default::default()
                },
            });
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ANIM_DRAW_DEPTH_BIAS_PX, AnimRenderDestination, CRATE_BODY_FRAME, anim_instance_alpha,
        anim_render_destination, apply_shape_z_adjust, garrison_flash_depth, overlay_body_frame,
        overlay_display_identity, overlay_render_identity, terrain_object_is_render_visible,
        weapon_muzzle_flash_key, world_effect_screen_position,
    };
    use crate::map::overlay::TerrainObject;
    use crate::map::overlay_types::OverlayTypeRegistry;
    use crate::rules::art_data::ArtRegistry;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;
    use crate::rules::tiberium_type::TiberiumTypeRegistry;
    use crate::sim::components::{AnimRuntime, GarrisonMuzzleFlash, WeaponMuzzleFlash};
    use crate::sim::intern::StringInterner;
    use crate::sim::overlay_grid::OverlayCell;
    use crate::sim::production::ProductionState;
    use crate::sim::terrain_object::{TerrainObjectLifecycle, TerrainObjectState};
    use crate::util::fixed_math::SimFixed;

    #[test]
    fn gsi_13_04_wa_top_and_tuntop_ground_use_native_layer_and_ysort_lowering() {
        let art = ArtRegistry::from_ini(&IniFile::from_str(
            "[WA_CUSTOM]\nYSortAdjust=7\n\
             [TUNTOP_CUSTOM]\nLayer=ground\nYSortAdjust=1000\n",
        ));
        let order = crate::app::presentation::render::draw_plan_lowering::NativeGroundOrder::new(&[5, 10, 20]);
        let wa = art.anim_runtime_config("WA_CUSTOM");
        let tuntop = art.anim_runtime_config("TUNTOP_CUSTOM");
        let world = crate::sim::anim_class::AnimWorldCoord {
            x: 400,
            y: 600,
            z: 208,
        };

        assert_eq!(
            anim_render_destination(10, None, world, wa, &order),
            Some(AnimRenderDestination::Top)
        );
        let Some(AnimRenderDestination::Ground(tunnel_draw)) =
            anim_render_destination(20, None, world, tuntop, &order)
        else {
            panic!("ground tile animation must enter TacticalDrawPlan");
        };
        assert_eq!(tunnel_draw.coord.x, 400);
        assert_eq!(tunnel_draw.coord.y, 600);
        assert_eq!(tunnel_draw.coord.z, 208);
        assert_eq!(tunnel_draw.y_sort_adjust, 1000);
        assert_eq!(tunnel_draw.y_sort_key(), 2000);

        let ordinary = order
            .object_draw(
                5,
                crate::render::tactical_draw_plan::TacticalCoord {
                    x: 900,
                    y: 900,
                    z: 0,
                },
                crate::render::tactical_draw_plan::SpriteEncoding::Plain,
            )
            .unwrap();
        let pieces = |parent| {
            crate::app::presentation::render::draw_plan_lowering::PlannedGroundObjectInstance::object(
                parent,
                vec![crate::app::presentation::render::draw_plan_lowering::GroundPieceInstance {
                    target: crate::app::presentation::render::draw_plan_lowering::GroundTexture::ShpPage(0),
                    instance: crate::render::batch::SpriteInstance::default(),
                }],
            )
        };
        let lowered = crate::app::presentation::render::draw_plan_lowering::lower_ground_object_instances(vec![
            pieces(tunnel_draw),
            pieces(ordinary),
        ]);
        assert_eq!(lowered.owners, [5, 20]);
    }

    #[test]
    fn gsi_04_10_render_visibility_distinguishes_unregistered_live_and_destroyed() {
        let ini = IniFile::from_str(
            "[General]\n\
             FixtureOnly=1\n\
             [InfantryTypes]\n\
             [VehicleTypes]\n0=DUMMY\n\
             [AircraftTypes]\n\
             [BuildingTypes]\n\
             [TerrainTypes]\n0=TREE01\n\
             [DUMMY]\nStrength=100\n\
             [TREE01]\nFixtureOnly=1\n",
        );
        let rules = RuleSet::from_ini(&ini).expect("rules");
        let registered = TerrainObject {
            rx: 4,
            ry: 7,
            name: "TREE01".to_string(),
        };
        let unregistered = TerrainObject {
            rx: 8,
            ry: 9,
            name: "MAPONLY".to_string(),
        };
        let mut production = ProductionState::default();

        assert!(terrain_object_is_render_visible(&registered, None, None));
        assert!(terrain_object_is_render_visible(
            &unregistered,
            Some(&rules),
            Some(&production),
        ));
        assert!(!terrain_object_is_render_visible(
            &registered,
            Some(&rules),
            Some(&production),
        ));

        let mut interner = StringInterner::default();
        let stable_id = 1;
        production.terrain_object_cells.insert((4, 7), stable_id);
        production.terrain_objects.insert(
            stable_id,
            TerrainObjectState {
                stable_id,
                in_logic_vector: false,
                type_ref: interner.intern("TREE01"),
                rx: 4,
                ry: 7,
                health: 10,
                max_health: 10,
                occupation_bits: 7,
                lifecycle: TerrainObjectLifecycle::Live,
            },
        );
        assert!(terrain_object_is_render_visible(
            &registered,
            Some(&rules),
            Some(&production),
        ));

        production
            .terrain_objects
            .get_mut(&stable_id)
            .unwrap()
            .lifecycle = TerrainObjectLifecycle::Destroyed;
        assert!(!terrain_object_is_render_visible(
            &registered,
            Some(&rules),
            Some(&production),
        ));
    }

    #[test]
    fn damaged_wall_keeps_its_overlay_data_byte_as_the_render_frame() {
        // A GAWALL segment at damage stage 2 with all four neighbours joined
        // stores 0x2F. The renderer must ask the atlas for exactly that frame;
        // any collapse to 0 draws a pristine, isolated post.
        assert_eq!(overlay_body_frame(false, 0x2F), 0x2F);
        // Damage stage 1, N+E connected.
        assert_eq!(overlay_body_frame(false, 0x13), 0x13);
        // Undamaged, isolated.
        assert_eq!(overlay_body_frame(false, 0x00), 0x00);
    }

    #[test]
    fn crate_overlays_ignore_the_cell_byte_and_draw_frame_zero() {
        // gamemd's overlay-body draw takes a Crate=yes branch that hardcodes
        // the frame; the cell's overlay data never reaches the shape call.
        assert_eq!(overlay_body_frame(true, 0), CRATE_BODY_FRAME);
        assert_eq!(overlay_body_frame(true, 7), CRATE_BODY_FRAME);
        assert_eq!(overlay_body_frame(true, 0x2F), CRATE_BODY_FRAME);
    }

    #[test]
    fn gsi_04_13_low_overlay_renderer_follows_live_identity_through_terminal_collapse() {
        let mut live = OverlayCell {
            overlay_id: Some(0x50),
            overlay_data: 0xA5,
            wall_owner: None,
        };

        assert_eq!(overlay_render_identity(0x4A, 7, None), Some((0x4A, 7)));
        assert_eq!(
            overlay_render_identity(0x4A, 7, Some(&live)),
            Some((0x50, 0xA5)),
            "first-damaged low bridge must select the live overlay variant"
        );

        live.overlay_id = Some(0x64);
        assert_eq!(
            overlay_render_identity(0x4A, 7, Some(&live)),
            Some((0x64, 0xA5)),
            "terminal collapse remains a drawable overlay identity"
        );

        live.overlay_id = Some(0x4D);
        assert_eq!(
            overlay_render_identity(0x4A, 7, Some(&live)),
            Some((0x4D, 0xA5)),
            "repair art must follow the live healthy variant"
        );

        live.overlay_id = None;
        assert_eq!(overlay_render_identity(0x4A, 7, Some(&live)), None);

        live.overlay_id = Some(6);
        assert_eq!(
            overlay_render_identity(5, 3, Some(&live)),
            None,
            "ordinary overlays retain exact identity matching"
        );
    }

    #[test]
    fn gsi_13_05_flat_resource_changes_only_display_identity_on_known_flat_cells() {
        let mut text = String::from(
            "[Tiberiums]\n0=Riparius\n1=Cruentus\n\
             [Riparius]\nImage=1\n\
             [Cruentus]\nImage=2\n\
             [OverlayTypes]\n",
        );
        let mut resource_names = Vec::new();
        for overlay_id in 0..=113 {
            let name = match overlay_id {
                27..=38 => format!("GEM{:02}", overlay_id - 26),
                102..=113 => format!("TIB{:02}", overlay_id - 101),
                _ => format!("FILL{overlay_id:03}"),
            };
            text.push_str(&format!("{overlay_id}={name}\n"));
            if matches!(overlay_id, 27..=38 | 102..=113) {
                resource_names.push(name);
            }
        }
        for name in resource_names {
            text.push_str(&format!("[{name}]\nTiberium=yes\n"));
        }
        let ini = IniFile::from_str(&text);
        let overlays = OverlayTypeRegistry::from_ini(&ini, None);
        let tiberiums = TiberiumTypeRegistry::from_ini(&ini);
        let tib12 = overlays.id_for_name("TIB12").expect("TIB12");
        let tib01 = overlays.id_for_name("TIB01").expect("TIB01");
        let tib05 = overlays.id_for_name("TIB05").expect("TIB05");
        let gem12 = overlays.id_for_name("GEM12").expect("GEM12");
        let gem01 = overlays.id_for_name("GEM01").expect("GEM01");
        let non_tiberium = overlays.id_for_name("FILL000").expect("FILL000");

        assert_eq!(
            overlay_display_identity(tib12, 8, 4, 7, Some(0), Some(&overlays), Some(&tiberiums),),
            (tib05, 8)
        );
        assert_eq!(
            overlay_display_identity(gem12, 11, 0, 7, Some(0), Some(&overlays), Some(&tiberiums),),
            (gem01, 11)
        );
        assert_eq!(
            overlay_display_identity(tib12, 8, 4, 7, Some(2), Some(&overlays), Some(&tiberiums),),
            (tib12, 8),
            "sloped resource cells retain their stored display identity"
        );
        assert_eq!(
            overlay_display_identity(
                non_tiberium,
                9,
                4,
                7,
                Some(0),
                Some(&overlays),
                Some(&tiberiums),
            ),
            (non_tiberium, 9),
            "non-resource overlays remain unchanged"
        );
        assert_eq!(
            overlay_display_identity(
                tib12,
                8,
                u16::MAX,
                1,
                Some(0),
                Some(&overlays),
                Some(&tiberiums),
            ),
            (tib01.checked_sub(1).expect("registered base - 1"), 8),
            "signed base-relative display selection must not change live density state"
        );
    }

    #[test]
    fn weapon_muzzle_flash_key_uses_shp_name_and_frame() {
        let flash = WeaponMuzzleFlash {
            attacker_id: 1,
            shp_name: "MGUN-N".to_string(),
            screen_x: 100.0,
            screen_y: 200.0,
            rx: 10,
            ry: 11,
            z: 0,
            frame: 3,
            total_frames: 4,
            rate_ms: 67,
            elapsed_ms: 0,
        };
        let key = weapon_muzzle_flash_key(&flash);
        assert_eq!(key.type_id, "MGUN-N");
        assert_eq!(key.frame, 3);
        assert_eq!(key.facing, 0);
    }

    #[test]
    fn garrison_flash_depth_applies_native_z_adjust_as_depth_bias() {
        let flash = GarrisonMuzzleFlash {
            building_id: 42,
            runtime: AnimRuntime {
                type_name: "UCFLASH".to_string(),
                current_frame: 0,
                frame_step: 1,
                delay_logic_frames: 0,
                reload_logic_frames: 1,
                rate_elapsed_logic_frames: 0,
                loop_remaining: 1,
                first_ai_guard: false,
                expired: false,
                constructor_reverse: false,
                elapsed_logic_ms: 0,
            },
            pixel_x: 0,
            pixel_y: 0,
            screen_x: 100.0,
            screen_y: 200.0,
            rx: 10,
            ry: 11,
            z: 0,
            z_adjust: -200,
        };

        let world_height: f32 = 1000.0;
        let neutral = garrison_flash_depth(0.0, world_height, flash.screen_y, flash.z, 0);
        let biased =
            garrison_flash_depth(0.0, world_height, flash.screen_y, flash.z, flash.z_adjust);
        assert!(
            biased < neutral,
            "z_adjust=-200 must pull the flash toward the camera (smaller depth), \
             without shifting its screen row"
        );
        let expected_delta: f32 = -200.0 / world_height;
        assert!(
            (biased - neutral - expected_delta).abs() < 1e-6,
            "bias magnitude must be z_adjust pixels over world_height (got {} vs {})",
            biased - neutral,
            expected_delta
        );
    }

    #[test]
    fn apply_shape_z_adjust_is_pixel_exact_with_zero_neutral() {
        let world_height: f32 = 2000.0;
        let base: f32 = 0.5;
        // Neutral is 0, NOT 1000 (the 1000 convention is the per-cell terrain
        // path, a separate mechanism).
        assert_eq!(apply_shape_z_adjust(base, 0, world_height), base);
        // Negative = toward camera (smaller depth), pixel-exact magnitude.
        let toward = apply_shape_z_adjust(base, -300, world_height);
        assert!((toward - (base - 300.0 / world_height)).abs() < 1e-6);
        // Positive = away from camera.
        let away = apply_shape_z_adjust(base, 40, world_height);
        assert!(away > base);
        // Clamped to the valid depth range.
        assert_eq!(apply_shape_z_adjust(0.002, -100_000, world_height), 0.001);
        assert_eq!(apply_shape_z_adjust(0.998, 100_000, world_height), 0.999);
    }

    #[test]
    fn effective_anim_z_adjust_slot_overrides_type() {
        use crate::app::presentation::instances::helpers::effective_anim_z_adjust;
        // Nonzero slot override (e.g. ActiveAnimZAdjust=-100) wins.
        assert_eq!(effective_anim_z_adjust(-100, -300), -100);
        // Zero slot falls back to the anim type's own ZAdjust=.
        assert_eq!(effective_anim_z_adjust(0, -300), -300);
        assert_eq!(effective_anim_z_adjust(0, 0), 0);
    }

    #[test]
    fn anim_draw_bias_constant_matches_native() {
        assert_eq!(ANIM_DRAW_DEPTH_BIAS_PX, -2);
    }

    #[test]
    fn gsi_13_09_anim_emitters_carry_the_art_types_translucency_into_instance_alpha() {
        // BURN-S/M/L (`Translucency=25`) and the wake/warp family
        // (`Translucent=yes`) are the two stock shapes the emitters must honour.
        let ini = IniFile::from_str(
            "[BURNLIKE]\n\
             Translucency=25\n\
             [FIFTYLIKE]\n\
             Translucency=50\n\
             [WAKELIKE]\n\
             Translucent=yes\n\
             End=10\n\
             [PLAIN]\nFixtureOnly=1\n",
        );
        let reg = ArtRegistry::from_ini(&ini);

        // Translucency=N is N percent TRANSPARENT: 25 leaves three quarters of
        // the source. A fire drawn at 0.25 would be nearly invisible.
        let burn = reg.anim_runtime_config("BURNLIKE");
        assert_eq!(anim_instance_alpha(burn, 0, 8), 0.75);
        assert_eq!(anim_instance_alpha(burn, 7, 8), 0.75);
        assert_eq!(
            anim_instance_alpha(reg.anim_runtime_config("FIFTYLIKE"), 0, 8),
            0.5
        );

        // Translucent=yes is a progressive fade against End: opaque through
        // 0.2*End, then 3/4, 1/2, 1/4.
        let wake = reg.anim_runtime_config("WAKELIKE");
        assert_eq!(anim_instance_alpha(wake, 2, 10), 1.0);
        assert_eq!(anim_instance_alpha(wake, 3, 10), 0.75);
        assert_eq!(anim_instance_alpha(wake, 5, 10), 0.5);
        assert_eq!(anim_instance_alpha(wake, 9, 10), 0.25);

        // A type with neither key, and a type the registry never saw, draw
        // opaque — wiring this in cannot dim an animation that was not marked.
        assert_eq!(
            anim_instance_alpha(reg.anim_runtime_config("PLAIN"), 4, 8),
            1.0
        );
        assert_eq!(anim_instance_alpha(None, 4, 8), 1.0);
    }

    #[test]
    fn world_effect_projection_preserves_exact_subcell_anchor() {
        for (rx, ry, sub_x, sub_y, z) in [
            (10, 10, 128, 128, 0),
            (23, 20, 128, 128, 0),
            (41, 17, 32, 224, 0),
            (7, 13, 240, 48, 2),
        ] {
            let sub_x = SimFixed::from_num(sub_x);
            let sub_y = SimFixed::from_num(sub_y);
            assert_eq!(
                world_effect_screen_position(rx, ry, sub_x, sub_y, z),
                crate::util::lepton::lepton_to_screen(rx, ry, sub_x, sub_y, z),
                "WorldEffect must use the native CoordStruct anchor at \
                 ({rx},{ry},{sub_x:?},{sub_y:?},z={z})"
            );
        }
    }
}
