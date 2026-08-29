//! Draw pass dispatch — creates the wgpu render pass and issues all draw calls in order.
//!
//! Separated from the instance-building phase in mod.rs so the orchestrator stays focused
//! on *what* to render while this module handles *how* to submit it to the GPU.
//!
//! ## Dependency rules
//! - Internal to `presentation::render` — only called from mod.rs via `dispatch_draw_passes()`.

use crate::app::AppState;
use crate::app::presentation::sidebar_render::{
    begin_main_load_pass, begin_main_pass, current_sidebar_chrome_texture,
    current_sidebar_gclock_texture,
};
use crate::app::presentation::ui_overlays::current_software_cursor_texture;
use crate::render::batch::{BatchRenderer, BatchTexture, InstanceBufferPool, SpriteInstance};
use crate::render::bridge_atlas::BridgeAtlas;
use crate::render::overlay_atlas::OverlayAtlas;
use crate::render::tile_atlas::TileAtlas;

use super::merge_passes;

/// Data from the instance-building phase that the draw pass needs beyond `AppState`.
///
/// These are local variables in `render_game()` that can't be accessed through `state`
/// because they're computed fresh each frame and (for the merge passes) need CPU-side
/// depth values that match the uploaded GPU buffers.
pub(super) struct DrawPassData<'a> {
    pub ground: &'a super::draw_plan_lowering::GroundObjectPass,
    pub bridge_unit_instances: &'a [SpriteInstance],
    pub bridge_unit_pages: &'a [usize],
    pub bridge_unit_transition_paged: &'a [Vec<SpriteInstance>],
    pub bridge_shp_paged: &'a [Vec<SpriteInstance>],
    pub unit_instances: &'a [SpriteInstance],
    pub unit_pages: &'a [usize],
    pub unit_transition_paged: &'a [Vec<SpriteInstance>],
    pub shp_paged: &'a [Vec<SpriteInstance>],
    pub flat_layer_draws: &'a [super::draw_plan_lowering::FlatLayerDraw],
    pub ghost_page: u8,
}

/// Create the main render pass and dispatch all draw calls in the correct order.
///
/// The frame has two regions, matching the native composition: the **tactical
/// viewport**, scissored to the window minus the sidebar column, and the
/// **chrome**, which owns the whole window and goes down last. Steps 1–10 below
/// are tactical; the screen-fixed block at the end releases the scissor first.
///
/// Draw order follows the original engine's layered rendering:
/// 1. Terrain (zdepth) → 2. Bridge body (zdepth) → 3. Overlays (passthrough) →
/// 4. Bridge entities (merge) → 5. Ground objects, building turrets included
/// (merge) → 7. Bridge railings → 7.5 Particles (layer 3) →
/// 7.7 Bodies above the Ground band (layers 3–4) → 8. Debug → 9. Shroud/fog →
/// 10. UI/sidebar
pub(super) fn dispatch_draw_passes(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    data: &DrawPassData<'_>,
) {
    let pool: &InstanceBufferPool = &state.renderer.instance_pool;
    let transition_cache = state.renderer.vxl_slope_transition_cache.borrow();
    let mut pass = begin_main_pass(encoder, view, &state.renderer.depth_view);

    // Everything from here to the screen-fixed chrome block is battlefield: it
    // belongs to the tactical viewport and must not be able to paint a pixel
    // into the sidebar column. The native engine gets that for free — the
    // battlefield composes into its own surface and every object draw is handed
    // the intersection of its screen rect with the tactical rect as a clip. VERA
    // composes into one target, so the scissor is what enforces it.
    //
    // Without it the guarantee degrades to "the sidebar art happens to cover
    // it", and coverage is per-theme, not universal. Allied and Soviet do cover:
    // side3 is drawn at its own SHP height, not the RON's `side3_height` (which
    // is 0), and the top-housing panel follows it, so the stack runs past the
    // window bottom and only a few pixels of the top strip are bare. Yuri does
    // not. Its atlas is built from sidec02md.mix, whose seven entries are
    // radary.shp, the three background plates, two palettes and key.ini — and
    // the by-hash-ID lookups (`render_entry_by_id`, used for the top strips and
    // the housing panel) have no asset-manager fallback, unlike the by-name
    // ones. They all resolve to None, leaving the whole top-inset block and a
    // strip below side3 unpainted. Those are the holes live terrain, units and
    // any overhanging bracket or health bar were showing through. Clipped, the
    // region reads black, which is what opaque chrome looks like there.
    let (tac_x, tac_y, tac_w, tac_h) = crate::app::input::camera::tactical_viewport_px(state);
    pass.set_scissor_rect(tac_x, tac_y, tac_w, tac_h);

    // --- Step 1: Terrain (Z-depth pipeline for per-pixel depth from TMP Z-data) ---
    draw_pooled_zdepth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        state.match_state.match_presentation.tile_atlas.as_ref(),
        "terrain",
    );

    // --- Step 1.5: Smudges (static decals: craters + scorches) ---
    // The native terrain-tile pass dispatches each cell's smudge right after
    // blitting that cell's tile, so smudges land in the terrain layer — well
    // before the cell-content layer that draws overlays. Drawing them after
    // overlays instead put every crater and scorch mark on top of the ore and
    // walls it should be lying under.
    draw_pooled_passthrough_overlay(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        state.match_state.match_presentation.overlay_atlas.as_ref(),
        "smudge",
    );

    // --- Step 2: Bridge body (Z-depth pipeline) ---
    draw_pooled_bridge_zdepth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        state.match_state.match_presentation.bridge_atlas.as_ref(),
        "overlay_bridge_body",
    );

    // (Bridge body shadows are NOT drawn here. The native cell-content layer
    // runs two full sweeps — every overlay body, then every overlay shadow —
    // so shadows belong after the overlay bodies at step 3.5, not between the
    // bridge body and the overlays.)

    // --- Step 3: Overlays (no depth test — passthrough) ---
    // Overlays don't read the Z-buffer — the tile blitter skips Z-testing
    // for tiles without Z-data (flag 0x02 clear at cell header byte 36).
    // Overlays paint unconditionally over terrain. Without
    // passthrough, adjacent terrain tiles from closer iso rows would
    // occlude overlays via LessEqual depth test ("sinking into ground").
    // Overlays (including walls) stay in the fixed cell family.
    draw_pooled_passthrough_overlay(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        state.match_state.match_presentation.overlay_atlas.as_ref(),
        "overlay",
    );

    // --- Step 3.5: Overlay shadows — bridge decks ---
    //
    // **This is the only shadow pass in the renderer.** GSI-13.11 covers three
    // — ground, object and voxel — and the other two do not exist: a
    // repo-wide search finds no shadow instance emission for infantry,
    // vehicles, buildings or aircraft, and no voxel shadow in any shader. So
    // every tank, soldier, structure and plane in an ordinary skirmish is
    // missing its ground shadow. Recorded, not closed. Trigger: every frame
    // with any unit or building on screen. Player effect: the scene reads flat
    // against retail, which shadows every object. Frequency: continuous.
    // Downstream risk: the SHP-blitter contract the bridge path already honours
    // (1-bit stencil half, composited darken) is the shape the object pass has
    // to reuse, and `BRIDGE_SHADOW_DARKEN_ALPHA` already carries its own
    // recorded lightness drift against the native halve.
    // Second sweep of the native cell-content layer: after every overlay body
    // is down, each overlay-bearing cell draws its shadow half. The atlas bakes
    // these as black texels whose alpha approximates the blitter's darken, so
    // the ordinary passthrough pipeline gets the shape and the
    // composite-on-overlap behaviour. The darken STRENGTH is a known drift —
    // this pass blends in linear space against an sRGB target while the blitter
    // halves the encoded word, leaving the shadow lighter than retail. See
    // `render::bridge_atlas::SHADOW_DARKEN_ALPHA`.
    //
    // Only bridge decks are covered so far — ore, gem and wall shadows still
    // need their own instance bucket and pooled buffer.
    draw_pooled_bridge_passthrough(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        state.match_state.match_presentation.bridge_atlas.as_ref(),
        "overlay_bridge_body_shadow",
    );

    // (Smudges are drawn back at step 1.5, inside the terrain layer, matching
    // the native per-cell tile-then-smudge dispatch. Instance construction now
    // projects the footprint origin with its resolved cell level, so hilltop
    // composites share the terrain tile's elevation. Their depth value is
    // irrelevant either
    // way — this pass neither reads nor writes the depth buffer.)

    // Building selection bracket back/left edges. Drawn before object bodies so
    // the normal SHP merge naturally occludes the hidden bracket edges.
    let bracket_tex = state.match_state.match_presentation.selection_overlay.as_ref().map(|o| o.white_texture());
    draw_pooled_passthrough_texture(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        bracket_tex,
        "selection_brackets_back",
    );
    // gamemd's first object pass calls DrawExtras before the nonzero +0x104
    // display call. Re-submit the front/right bracket stubs here; object body
    // draws can still occlude this first submission, and the later DrawExtras
    // phase submits them again.
    draw_pooled_passthrough_texture(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        bracket_tex,
        "selection_brackets_front_first",
    );

    // --- Step 4: Bridge entities (multi-way Y-merge) ---
    merge_passes::draw_merged_bridge_occluded_pass(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        data.bridge_unit_instances,
        data.bridge_unit_pages,
        data.bridge_unit_transition_paged,
        data.bridge_shp_paged,
        state.match_state.match_presentation.unit_atlas.as_ref(),
        &transition_cache,
        state.match_state.match_presentation.sprite_atlas.as_ref(),
        state.match_state.match_presentation.palette_set.as_ref(),
    );

    // --- Step 5: Ground objects (native integer LayerClass order) ---
    // Terrain, units, infantry, and building-owned pieces share the exact
    // signed X+Y + stable-registration order. Atlas bindings dispatch only
    // after the parent slot has been selected.
    if let Some((ground_buffer, ground_count)) = pool.get("ground_objects") {
        assert_eq!(
            ground_count as usize,
            data.ground.instances.len(),
            "native Ground upload must preserve every lowered instance",
        );
        for run in &data.ground.runs {
            if let super::draw_plan_lowering::GroundTexture::AnimShadowShpPage(page) = run.target {
                // Native DrawIt submits this immediately after its body. End
                // the sRGB pass, edit the aliased encoded destination once per
                // stencil instance, then resume both colour and depth state.
                drop(pass);
                if let Some(texture) = state
                    .match_state
                    .match_presentation
                    .sprite_atlas
                    .as_ref()
                    .and_then(|atlas| atlas.page(page))
                {
                    state.renderer.combat_light_renderer.draw_anim_shadow_run(
                        encoder,
                        &state.renderer.depth_view,
                        &state.renderer.batch_renderer,
                        &texture.texture,
                        ground_buffer,
                        run.start,
                        run.count,
                        [tac_x, tac_y, tac_w, tac_h],
                    );
                }
                pass = begin_main_load_pass(encoder, view, &state.renderer.depth_view);
                pass.set_scissor_rect(tac_x, tac_y, tac_w, tac_h);
            } else {
                merge_passes::draw_native_ground_object_standard_run(
                    &mut pass,
                    &state.renderer.batch_renderer,
                    run,
                    ground_buffer,
                    state.match_state.match_presentation.overlay_atlas.as_ref(),
                    state.match_state.match_presentation.unit_atlas.as_ref(),
                    &transition_cache,
                    state.match_state.match_presentation.sprite_atlas.as_ref(),
                    state.match_state.match_presentation.palette_set.as_ref(),
                );
            }
        }
    }

    // Scheduler-owned effects not yet carrying verified class-specific
    // YSortAdjust remain in the pre-existing residual SHP stream.
    merge_passes::draw_merged_object_pass(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        data.unit_instances,
        data.unit_pages,
        data.unit_transition_paged,
        data.shp_paged,
        state.match_state.match_presentation.unit_atlas.as_ref(),
        &transition_cache,
        state.match_state.match_presentation.sprite_atlas.as_ref(),
        state.match_state.match_presentation.palette_set.as_ref(),
    );

    if let (Some(overlay), Some((buffer, count))) =
        (state.match_state.match_presentation.selection_overlay.as_ref(), pool.get("weapon_waves"))
    {
        state.renderer.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            overlay.white_texture(),
            buffer,
            count,
        );
    }

    // (There is no separate building-turret pass. gamemd draws a building's
    // voxel turret inside the building's own display call, in the sorted
    // ground layer, right after the body — the pass that does run after
    // layer 2 walks the building array to draw a production/ally overlay and
    // never touches a turret. The turret instances are therefore emitted into
    // the same UnitAtlas stream as the vehicles and interleave with them in
    // step 5; see the note in build_instances.)

    // --- Step 7: Bridge railings (passthrough — Z-test ON, Z-write OFF) ---
    // Drawn after the unit/ground merge and before debug. Units and anims sit
    // above the deck body but below the railings.
    draw_pooled_bridge_railing(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        state.match_state.match_presentation.bridge_railing_atlas.as_ref(),
        "overlay_bridge_railing",
    );

    // --- Step 7.5: Particles (Layer 3, above all ground geometry) ---
    // ParticleClass::GetLayer = 3 in the original engine, drawing particles
    // above Layer 2 (buildings, units, turrets).
    // Passthrough pipeline (no depth interaction) — particles are translucent
    // and Y-sorted on the CPU, so no GPU depth read/write needed.
    const PARTICLE_KEYS: [&str; 4] = ["particle_p0", "particle_p1", "particle_p2", "particle_p3"];
    for (i, key) in PARTICLE_KEYS.iter().enumerate() {
        if let Some(page) = state.match_state.match_presentation.sprite_atlas.as_ref().and_then(|a| a.page(i)) {
            if let Some((buf, count)) = pool.get(key) {
                if count == 0 {
                    continue;
                }
                state.renderer.batch_renderer.draw_passthrough_range(
                    &mut pass,
                    &page.texture,
                    buf,
                    0,
                    count,
                );
            }
        }
    }

    // BuildingLightClass registers in layer 3. This pass is exact once its
    // authoritative child-light coordinates are emitted; it deliberately does
    // not substitute the parent building coordinate.
    if let Some((buffer, count)) = pool.get("spotlight_type16") {
        state
            .renderer.batch_renderer
            .draw_spotlight_type16(&mut pass, buffer, count);
    }

    // --- Step 7.7: The band above Ground (gamemd layers 3 and 4) ---
    // The native object loop walks its display layers in numeric order and only
    // layer 2 is sorted. The tagged schedule consumes layer 3 completely before
    // layer 4 and uses the live Submit_Object registration inside each layer,
    // interrupting atlas families without changing order.
    //
    // The SHP half goes through passthrough, which does no depth test at all —
    // the same thing the native sprite blitters do for these layers. The voxel
    // half is stuck with the voxel pipeline's LessEqual test against the
    // terrain buffer; with the sort key now anchored on the body's own ground
    // row (see helpers::ground_sort_row) that test passes for everything the
    // body flies over, so the residual is a cliff face standing in a *nearer*
    // iso row than the body's own cell, which its lifted sprite does not reach.
    let unit_top = pool.get("unit_top");
    let shp_top = pool.get("shp_top");
    for draw in data.flat_layer_draws {
        match draw.target {
            super::draw_plan_lowering::FlatDrawTarget::Unit { page, index } => {
                let (Some(unit_atlas), Some(palette_set), Some((buffer, count))) = (
                    state.match_state.match_presentation.unit_atlas.as_ref(),
                    state.match_state.match_presentation.palette_set.as_ref(),
                    unit_top,
                ) else {
                    continue;
                };
                assert!(
                    index < count,
                    "flat VXL draw index must fit its GPU buffer"
                );
                let texture = unit_atlas.page_texture(page).unwrap_or_else(|| {
                    panic!(
                        "flat VXL draw references missing UnitAtlas page {} of {}",
                        page,
                        unit_atlas.page_count(),
                    )
                });
                state.renderer.batch_renderer.draw_voxel_sprites_range(
                    &mut pass,
                    texture,
                    &palette_set.bind_group,
                    buffer,
                    index,
                    1,
                );
            }
            super::draw_plan_lowering::FlatDrawTarget::Shp {
                page,
                index,
                mode,
            } => {
                let (Some(atlas), Some((buffer, count))) = (
                    state.match_state.match_presentation.sprite_atlas.as_ref(),
                    shp_top,
                ) else {
                    continue;
                };
                assert!(
                    index < count,
                    "flat SHP draw index must fit its GPU buffer"
                );
                let texture = atlas.page(page).unwrap_or_else(|| {
                    panic!(
                        "flat SHP draw references missing SpriteAtlas page {} of {}",
                        page,
                        atlas.page_count(),
                    )
                });
                match mode {
                    super::draw_plan_lowering::ShpCompositeMode::Standard => {
                        state.renderer.batch_renderer.draw_passthrough_range(
                            &mut pass,
                            &texture.texture,
                            buffer,
                            index,
                            1,
                        );
                    }
                    super::draw_plan_lowering::ShpCompositeMode::AnimShadowDestinationHalve => {
                        drop(pass);
                        state.renderer.combat_light_renderer.draw_anim_shadow_run(
                            encoder,
                            &state.renderer.depth_view,
                            &state.renderer.batch_renderer,
                            &texture.texture,
                            buffer,
                            index,
                            1,
                            [tac_x, tac_y, tac_w, tac_h],
                        );
                        pass = begin_main_load_pass(encoder, view, &state.renderer.depth_view);
                        pass.set_scissor_rect(tac_x, tac_y, tac_w, tac_h);
                    }
                }
            }
        }
    }

    // --- Step 7.8: Persistent combat-light vector ---
    // gamemd edits the completed tactical object surface here, tail-to-head,
    // before the later debug/shroud/UI families. End the sRGB/depth pass while
    // the dedicated renderer performs its encoded RGB565 destination edits,
    // then resume both attachments with Load.
    drop(pass);
    state
        .renderer.combat_light_renderer
        .draw(encoder, [tac_x, tac_y, tac_w, tac_h]);
    let mut pass = begin_main_load_pass(encoder, view, &state.renderer.depth_view);
    pass.set_scissor_rect(tac_x, tac_y, tac_w, tac_h);

    // --- Step 8: Debug overlays ---
    // Drawn above entities, below fog and UI.
    // Use filled-diamond texture so cells appear as isometric diamonds, not rectangles.
    let debug_diamond_tex = state
        .match_state.match_presentation.selection_overlay
        .as_ref()
        .map(|o| o.diamond_filled_texture());
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        debug_diamond_tex,
        "debug_pathgrid",
    );
    let grid_tex = state
        .match_state.match_presentation.selection_overlay
        .as_ref()
        .map(|o| o.diamond_outline_texture());
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        grid_tex,
        "debug_cell_grid",
    );
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        debug_diamond_tex,
        "debug_path",
    );
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        debug_diamond_tex,
        "debug_heightmap",
    );

    // --- Step 9: Shroud (GPU ABuffer multiply pass) ---
    // Darkens every scene pixel by the shroud brightness value via
    // per-pixel ABuffer lookup.
    // Fully shrouded areas → black, edge cells → gradient, explored → no change.
    if let Some(ref buf) = state.match_state.match_presentation.shroud_buffer {
        if !state.match_state.sandbox_full_visibility {
            buf.draw(&mut pass);
        }
    }

    // --- Step 10: UI elements ---
    // Factory rally and selected action lines are separate line families.
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        bracket_tex,
        "factory_rally_first",
    );
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        bracket_tex,
        "target_lines",
    );
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        bracket_tex,
        "factory_rally_second",
    );
    // Isometric selection brackets for buildings: white 1px stub lines at 3 roof corners.
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        bracket_tex,
        "building_radius_rings",
    );
    // Stamp the selected buildings' own art into the depth buffer, colour
    // masked off, so the bracket redraw below can be clipped by it. gamemd's
    // building blit writes Z as it paints and its line rasteriser tests every
    // pixel against that Z, which is why a selected Construction Yard there
    // shows only the marks that clear its own silhouette. This runs here, after
    // every colour pass that reads depth, so the stamp cannot disturb anything
    // but the bracket test that immediately follows.
    const SELECTED_DEPTH_KEYS: [&str; 4] = [
        "shp_selected_depth_p0",
        "shp_selected_depth_p1",
        "shp_selected_depth_p2",
        "shp_selected_depth_p3",
    ];
    for (i, key) in SELECTED_DEPTH_KEYS.iter().enumerate() {
        if let Some(page) = state.match_state.match_presentation.sprite_atlas.as_ref().and_then(|a| a.page(i)) {
            if let Some((buf, count)) = pool.get(key) {
                state.renderer.batch_renderer.draw_with_buffer_depth_stamp(
                    &mut pass,
                    &page.texture,
                    buf,
                    count,
                );
            }
        }
    }
    // Final selected-building front bracket redraw: gamemd line pixels test Z
    // but do not write it — the store back into Z sits behind a caller flag
    // this path leaves clear. Each pixel carries its ground-footprint corner's
    // depth, so the marks that fall behind the building art lose the test. The
    // CPU instance builder already samples the tactical ABuffer for this
    // post-shroud redraw.
    draw_pooled_depth_test_texture(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        bracket_tex,
        "selection_brackets_front",
    );
    // Building health pips: discrete pips from pips.shp atlas.
    let building_status_tex = state
        .match_state.match_presentation.selection_overlay
        .as_ref()
        .map(|o| o.pip_texture().unwrap_or_else(|| o.white_texture()));
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        building_status_tex,
        "status_building",
    );
    // Occupant pips for garrisoned buildings (pips.shp frames 6-12).
    let occupant_pip_tex = state.match_state.match_presentation.selection_overlay.as_ref().map(|o| {
        o.occupant_pip_texture()
            .unwrap_or_else(|| o.white_texture())
    });
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        occupant_pip_tex,
        "occupant_pips",
    );
    // Non-building health bar backgrounds: pipbrd.shp bracket sprites.
    let unit_bg_tex = state
        .match_state.match_presentation.selection_overlay
        .as_ref()
        .and_then(|o| o.pipbrd_texture());
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        unit_bg_tex,
        "status_unit_bg",
    );
    // Non-building health bar fills: individual pip sprites from pips.shp (or white_texture fallback).
    let unit_fill_tex = state
        .match_state.match_presentation.selection_overlay
        .as_ref()
        .map(|o| o.unit_pip_texture().unwrap_or_else(|| o.white_texture()));
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        unit_fill_tex,
        "status_unit_fill",
    );
    // Tiberium cargo pips for harvesters (pips2.shp frames 0, 2, 5).
    let cargo_pip_tex = state.match_state.match_presentation.selection_overlay.as_ref().map(|o| {
        o.tiberium_pip_texture()
            .unwrap_or_else(|| o.white_texture())
    });
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        cargo_pip_tex,
        "cargo_pips",
    );
    // Drag rectangle — screen-fixed, use UI camera (zoom=1.0).
    let drag_tex = state.match_state.match_presentation.selection_overlay.as_ref().map(|o| o.drag_texture());
    draw_pooled_ui(&mut pass, &state.renderer.batch_renderer, pool, drag_tex, "drag");
    // Placement preview — world-space, uses world camera (zoom).
    let ghost_tex = state
        .match_state.match_presentation.sprite_atlas
        .as_ref()
        .and_then(|a| a.page(data.ghost_page as usize))
        .map(|p| &p.texture);
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        ghost_tex,
        "placement_ghost",
    );
    let wall_ghost_tex = state.match_state.match_presentation.overlay_atlas.as_ref().map(|a| &a.texture);
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        wall_ghost_tex,
        "placement_wall_ghost",
    );
    let valid_tex = state
        .match_state.match_presentation.selection_overlay
        .as_ref()
        .map(|o| o.preview_valid_texture());
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        valid_tex,
        "placement_valid",
    );
    let invalid_tex = state
        .match_state.match_presentation.selection_overlay
        .as_ref()
        .map(|o| o.preview_invalid_texture());
    draw_pooled_no_depth(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        invalid_tex,
        "placement_invalid",
    );

    // --- Step 10.5: PixelFX water/ore sparkles ---
    // gamemd writes these opaque one-pixel effects at the tactical tail, after
    // object/effect/status/action/placement drawing and before screen-fixed
    // chrome. In VERA the global shroud translation must therefore run first.
    // The passthrough pipeline bypasses depth; an empty buffer when
    // graphics.extra_animations is off short-circuits at count == 0.
    if let (Some(overlay), Some((buf, count))) =
        (state.match_state.match_presentation.selection_overlay.as_ref(), pool.get("cell_sparkles"))
    {
        state.renderer.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            overlay.white_texture(),
            buf,
            count,
        );
    }

    // --- Screen-fixed UI: sidebar, minimap, cursor — use UI camera (zoom=1.0) ---
    // Chrome owns the whole window: the sidebar column, the message list that
    // starts at the tactical origin, tooltips, and the cursor, which the native
    // engine draws over both regions. Release the tactical scissor before any of
    // it goes down.
    pass.set_scissor_rect(0, 0, state.render_width(), state.render_height());
    draw_pooled_ui(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        state.match_state.match_presentation.minimap.as_ref().map(|m| m.white_texture()),
        "sidebar",
    );
    draw_pooled_ui(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        current_sidebar_chrome_texture(state),
        "sidebar_chrome",
    );
    draw_pooled_ui(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        state
            .match_state.match_presentation.sidebar_cameo_atlas
            .as_ref()
            .map(|atlas| &atlas.texture),
        "sidebar_cameo",
    );
    draw_pooled_ui(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        current_sidebar_gclock_texture(state),
        "sidebar_gclock",
    );
    let cameo_overlay_tex = state
        .renderer.bit_font
        .darken_texture()
        .or_else(|| state.match_state.match_presentation.selection_overlay.as_ref().map(|o| o.white_texture()));
    draw_pooled_ui(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        cameo_overlay_tex,
        "sidebar_cameo_overlay",
    );
    draw_pooled_ui(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        Some(state.renderer.bit_font.atlas()),
        "sidebar_text",
    );

    // SidebarClass::Draw @ 0x006A6C30 paints background, gadgets, strip, and
    // power before PowerClass::Draw reaches RadarClass::Draw @ 0x00653100.
    // Radar state/chrome preparation then precedes Update @ 0x00656EC0, whose
    // content blit, viewport rectangle, and generated-content boundary are the
    // final retained radar writes in that order.
    draw_pooled_ui(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        state.match_state.match_presentation.radar_anim.as_ref().map(|ra| ra.texture()),
        "radar_anim",
    );
    draw_pooled_ui(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        state.match_state.match_presentation.minimap.as_ref().map(|m| m.map_texture()),
        "minimap",
    );
    draw_pooled_ui(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        state.match_state.match_presentation.minimap.as_ref().map(|m| m.white_texture()),
        "viewport_rect",
    );
    draw_pooled_ui(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        state.match_state.match_presentation.minimap.as_ref().map(|m| m.white_texture()),
        "radar_content_boundary",
    );
    draw_pooled_ui(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        Some(state.renderer.bit_font.atlas()),
        "message_text",
    );
    draw_pooled_ui(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        state.renderer.bit_font.darken_texture(),
        "tooltip_fill",
    );
    draw_pooled_ui(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        Some(state.renderer.bit_font.atlas()),
        "tooltip_text",
    );

    // Active YR ScreenCaptureCommandClass::Execute (0x00537BC0) hides
    // WWMouse while copying the already-presented client. Preserve the same
    // composition boundary without changing the displayed frame: retain every
    // completed UI/sidebar surface here, then resume and draw the cursor into
    // the ordinary presentation target.
    drop(pass);
    state
        .renderer.retail_screenshot_frame_cache
        .stage_pre_cursor_composition(
            &state.renderer.gpu.device,
            encoder,
            state.renderer.combat_light_renderer.composition_texture(),
            state.renderer.gpu.config.format,
            state.render_width(),
            state.render_height(),
            state.renderer.gpu.config.width,
            state.renderer.gpu.config.height,
        );
    let mut pass = begin_main_load_pass(encoder, view, &state.renderer.depth_view);
    pass.set_scissor_rect(0, 0, state.render_width(), state.render_height());
    draw_pooled_ui(
        &mut pass,
        &state.renderer.batch_renderer,
        pool,
        current_software_cursor_texture(state),
        "software_cursor",
    );
}

// ---------------------------------------------------------------------------
// Draw helpers — thin wrappers around BatchRenderer methods with atlas lookup
// ---------------------------------------------------------------------------

/// Draw a pooled buffer with the Z-depth pipeline (per-pixel frag_depth).
/// Uses the tile atlas's pre-built zdepth bind group (color + R8 depth textures).
fn draw_pooled_zdepth<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    batch: &'a BatchRenderer,
    pool: &'a InstanceBufferPool,
    atlas: Option<&'a TileAtlas>,
    key: &'static str,
) {
    if let (Some(a), Some((buf, count))) = (atlas, pool.get(key)) {
        batch.draw_with_buffer_zdepth(pass, &a.zdepth_bind_group, buf, count);
    }
}

fn draw_pooled_bridge_zdepth<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    batch: &'a BatchRenderer,
    pool: &'a InstanceBufferPool,
    atlas: Option<&'a BridgeAtlas>,
    key: &'static str,
) {
    if let (Some(a), Some((buf, count))) = (atlas, pool.get(key)) {
        batch.draw_with_buffer_zdepth(pass, &a.zdepth_bind_group, buf, count);
    }
}

/// Draw a pooled buffer with LessEqual depth test, depth write ON.
/// Used for the base terrain pass and UI/debug passes that write depth.
fn draw_pooled_no_depth<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    batch: &'a BatchRenderer,
    pool: &'a InstanceBufferPool,
    tex: Option<&'a BatchTexture>,
    key: &'static str,
) {
    if let (Some(t), Some((buf, count))) = (tex, pool.get(key)) {
        batch.draw_with_buffer_no_depth(pass, t, buf, count);
    }
}

/// Draw with the UI camera (zoom=1.0) for screen-fixed elements.
/// Uses the overlay pipeline (no depth) but sets bind group 0 to the UI camera
/// so sidebar, minimap, and cursor stay at fixed screen positions regardless of zoom.
fn draw_pooled_ui<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    batch: &'a BatchRenderer,
    pool: &'a InstanceBufferPool,
    tex: Option<&'a BatchTexture>,
    key: &'static str,
) {
    if let (Some(t), Some((buf, count))) = (tex, pool.get(key)) {
        if count == 0 {
            return;
        }
        pass.set_pipeline(batch.overlay_pipeline());
        pass.set_bind_group(0, batch.ui_camera_bind_group(), &[]);
        pass.set_bind_group(1, &t.bind_group, &[]);
        pass.set_vertex_buffer(0, buf.slice(..));
        pass.draw(0..6, 0..count);
    }
}

/// Draw non-wall overlays with depth test bypassed (Always compare).
/// Tiles without embedded Z-data skip Z-testing.
/// Uses the overlay atlas's regular texture bind group (not zdepth_bind_group).
fn draw_pooled_passthrough_overlay<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    batch: &'a BatchRenderer,
    pool: &'a InstanceBufferPool,
    atlas: Option<&'a OverlayAtlas>,
    key: &'static str,
) {
    if let (Some(a), Some((buf, count))) = (atlas, pool.get(key)) {
        batch.draw_with_buffer_passthrough(pass, &a.texture, buf, count);
    }
}

fn draw_pooled_passthrough_texture<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    batch: &'a BatchRenderer,
    pool: &'a InstanceBufferPool,
    tex: Option<&'a BatchTexture>,
    key: &'static str,
) {
    if let (Some(t), Some((buf, count))) = (tex, pool.get(key)) {
        batch.draw_with_buffer_passthrough(pass, t, buf, count);
    }
}

/// Draw a pooled buffer that tests the depth buffer and does not write it.
fn draw_pooled_depth_test_texture<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    batch: &'a BatchRenderer,
    pool: &'a InstanceBufferPool,
    tex: Option<&'a BatchTexture>,
    key: &'static str,
) {
    if let (Some(t), Some((buf, count))) = (tex, pool.get(key)) {
        batch.draw_with_buffer_depth_test(pass, t, buf, count);
    }
}

/// Draw a pooled bridge buffer with passthrough (no depth test, no depth
/// write). Used for the body shadow pass — same texture as the bridge body,
/// just a different draw pipeline.
fn draw_pooled_bridge_passthrough<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    batch: &'a BatchRenderer,
    pool: &'a InstanceBufferPool,
    atlas: Option<&'a BridgeAtlas>,
    key: &'static str,
) {
    if let (Some(a), Some((buf, count))) = (atlas, pool.get(key)) {
        batch.draw_with_buffer_passthrough(pass, &a.texture, buf, count);
    }
}

/// Draw a pooled buffer using the bridge railing atlas with passthrough
/// (Z-test ON, Z-write OFF).
fn draw_pooled_bridge_railing<'a>(
    pass: &mut wgpu::RenderPass<'a>,
    batch: &'a BatchRenderer,
    pool: &'a InstanceBufferPool,
    atlas: Option<&'a crate::render::bridge_railing_atlas::BridgeRailingAtlas>,
    key: &'static str,
) {
    if let (Some(a), Some((buf, count))) = (atlas, pool.get(key)) {
        batch.draw_with_buffer_passthrough(pass, &a.texture, buf, count);
    }
}

#[cfg(test)]
mod tests {
    const SOURCE: &str = include_str!("draw_passes.rs");

    fn source_offset(needle: &str) -> usize {
        SOURCE
            .find(needle)
            .unwrap_or_else(|| panic!("missing production draw anchor {needle:?}"))
    }

    #[test]
    fn gsi_13_01_pixel_fx_is_last_tactical_write_before_screen_chrome() {
        let shroud = source_offset("// --- Step 9: Shroud");
        let target_lines = source_offset("\"target_lines\"");
        let status = source_offset("\"status_unit_fill\"");
        let placement = source_offset("\"placement_invalid\"");
        let sparkle = source_offset("pool.get(\"cell_sparkles\")");
        let screen_fixed = source_offset("// --- Screen-fixed UI:");
        let full_window_scissor = source_offset(
            "pass.set_scissor_rect(0, 0, state.render_width(), state.render_height());",
        );
        let first_screen_submission = source_offset("\"minimap\"");

        assert!(shroud < target_lines);
        assert!(target_lines < status);
        assert!(status < placement);
        assert!(placement < sparkle);
        assert!(sparkle < screen_fixed);
        assert!(screen_fixed < full_window_scissor);
        assert!(full_window_scissor < first_screen_submission);

        let final_tactical_slice = &SOURCE[sparkle..screen_fixed];
        assert_eq!(
            final_tactical_slice.matches(".draw").count(),
            1,
            "PixelFX must remain the final tactical draw submission"
        );
    }

    #[test]
    fn gsi_13_01_pixel_fx_tail_remains_passthrough_and_tactically_scissored() {
        let tactical_scissor = source_offset("pass.set_scissor_rect(tac_x, tac_y, tac_w, tac_h);");
        let sparkle = source_offset("pool.get(\"cell_sparkles\")");
        let full_window_scissor = source_offset(
            "pass.set_scissor_rect(0, 0, state.render_width(), state.render_height());",
        );

        assert!(tactical_scissor < sparkle);
        assert!(sparkle < full_window_scissor);
        assert!(SOURCE[sparkle..full_window_scissor].contains("draw_with_buffer_passthrough"));
    }

    #[test]
    fn gsi_04_01_retained_sidebar_radar_subpass_ends_with_content_boundary() {
        let sidebar = source_offset("\"sidebar\"");
        let chrome = source_offset("\"sidebar_chrome\"");
        let cameo = source_offset("\"sidebar_cameo\"");
        let gclock = source_offset("\"sidebar_gclock\"");
        let cameo_overlay = source_offset("\"sidebar_cameo_overlay\"");
        let sidebar_text = source_offset("\"sidebar_text\"");
        let radar_anim = source_offset("\"radar_anim\"");
        let minimap = source_offset("\"minimap\"");
        let viewport = source_offset("\"viewport_rect\"");
        let boundary = source_offset("\"radar_content_boundary\"");
        let message = source_offset("\"message_text\"");

        assert!(sidebar < chrome);
        assert!(chrome < cameo);
        assert!(cameo < gclock);
        assert!(gclock < cameo_overlay);
        assert!(cameo_overlay < sidebar_text);
        assert!(sidebar_text < radar_anim);
        assert!(radar_anim < minimap);
        assert!(minimap < viewport);
        assert!(viewport < boundary);
        assert!(boundary < message);

        // An oversize native viewport edge may leave the 140x108 aperture but
        // still remain inside g_SidebarSurface. No later retained-sidebar or
        // radar batch may repaint that accepted line before the screen-overlay
        // strata begin.
        let retained_tail = &SOURCE[boundary..message];
        for later_retained_batch in [
            "\"sidebar\"",
            "\"sidebar_chrome\"",
            "\"sidebar_cameo\"",
            "\"sidebar_gclock\"",
            "\"sidebar_cameo_overlay\"",
            "\"sidebar_text\"",
            "\"radar_anim\"",
            "\"minimap\"",
            "\"viewport_rect\"",
        ] {
            assert!(
                !retained_tail.contains(later_retained_batch),
                "{later_retained_batch} must not overwrite the final radar outline"
            );
        }
    }

    #[test]
    fn gsi_04_01_tooltip_and_cursor_remain_after_the_retained_sidebar_surface() {
        let viewport = source_offset("\"viewport_rect\"");
        let boundary = source_offset("\"radar_content_boundary\"");
        let message = source_offset("\"message_text\"");
        let tooltip_fill = source_offset("\"tooltip_fill\"");
        let tooltip_text = source_offset("\"tooltip_text\"");
        let screenshot_boundary = source_offset("stage_pre_cursor_composition");
        let cursor = source_offset("\"software_cursor\"");

        assert!(viewport < boundary);
        assert!(boundary < message);
        assert!(message < tooltip_fill);
        assert!(tooltip_fill < tooltip_text);
        assert!(tooltip_text < screenshot_boundary);
        assert!(screenshot_boundary < cursor);
    }
}
