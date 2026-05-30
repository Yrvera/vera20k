//! In-game rendering and draw-pass orchestration.
//!
//! `render_game()` is the per-frame entry point. It runs a 7-phase pipeline:
//!
//! 1. **World instances** — terrain tiles, map overlays, bridges, VXL units,
//!    SHP buildings/infantry, world effects, damage fires, fog snapshots
//! 2. **Debug instances** — pathgrid, cell grid, heightmap overlays (toggled by hotkey)
//! 3. **Shroud ABuffer** - CPU shroud buffer rebuilt before UI overlays sample it
//! 4. **UI instances** - minimap dots, selection brackets, health bars, placement preview
//! 5. **Sidebar instances** - chrome, cameos, text, minimap rect, radar animation
//! 6. **Upload** - all instance vectors uploaded to GPU buffer pool
//! 7. **Draw** - render pass created, draw calls dispatched in layer order
//!
//! ## Sub-modules
//! - `build_instances` — phase 1-4 builders: named functions + structs per phase
//! - `draw_passes` — phase 6: render pass creation and GPU draw call dispatch
//! - `merge_passes` — Y-sorted multi-way merge algorithm for interleaving atlas textures
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

mod build_instances;
mod draw_passes;
mod merge_passes;

use anyhow::Result;

// Re-export shared types so any remaining `use crate::app_render::Foo` imports still compile.
// New code should import from `crate::app_types` directly.
pub(crate) use crate::app_types::*;

use crate::app::AppState;
use crate::app_commands::preferred_local_owner_name;
use crate::render::batch::InstanceBufferPool;
use crate::sidebar::SidebarView;

use build_instances::{DebugInstances, SidebarInstances, UiInstances, WorldInstances};

/// Render one in-game frame: terrain, units, overlays, UI, sidebar.
///
/// Orchestrates the 7-phase pipeline described in the module doc.
/// Each phase is a named function call — see `build_instances` for details.
pub(crate) fn render_game(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
) -> Result<Option<SidebarView>> {
    let (sw, sh) = (state.render_width() as f32, state.render_height() as f32);

    let local_owner = preferred_local_owner_name(state);
    // Effective viewport in world pixels — zoom shrinks what's visible.
    let z = state.zoom_level;
    let vsw = sw / z;
    let vsh = sh / z;

    // Phase 1: Build game-world instances (terrain, overlays, entities).
    let world = build_instances::build_world_instances(state, vsw, vsh);

    // Phase 2: Build debug overlay instances (pathgrid, cell grid, heightmap).
    let debug = build_instances::build_debug_instances(state, vsw, vsh);

    // Phase 3: Rebuild shroud ABuffer (CPU blit + GPU upload). The final
    // building-bracket front redraw samples this CPU buffer during UI build.
    let rw = state.render_width();
    let rh = state.render_height();
    let shroud_height_grid = state
        .path_grid
        .as_ref()
        .map(crate::sim::pathfinding::PathGrid::ground_height_grid);
    if let Some(ref mut shroud_buf) = state.shroud_buffer {
        if !state.sandbox_full_visibility {
            if let (Some(sim), Some(owner)) = (&state.simulation, &local_owner) {
                let owner_id = sim.interner.get(owner).unwrap_or_default();
                shroud_buf.rebuild_if_needed(
                    &state.gpu,
                    &sim.fog,
                    owner_id,
                    state.camera_x,
                    state.camera_y,
                    rw,
                    rh,
                    state.zoom_level,
                    shroud_height_grid.as_deref(),
                );
            }
        }
    }

    // Phase 4: Update minimap + build UI instances (selection, health, placement).
    build_instances::update_minimap(state, &local_owner);
    let ui = build_instances::build_ui_instances(state, vsw, vsh);

    // Phase 5: Build sidebar instances.
    let sidebar = build_instances::build_sidebar_instances(state);

    // Phase 6: Upload all instances to GPU buffer pool.
    upload_to_gpu(state, &world, &debug, &ui, &sidebar);
    state.cached_overlay_instances = world.overlay;

    // Phase 7: Dispatch draw calls in render order.
    draw_passes::dispatch_draw_passes(
        state,
        encoder,
        view,
        &draw_passes::DrawPassData {
            bridge_unit_instances: &world.bridge_unit,
            bridge_unit_transition_paged: &world.bridge_unit_transition_paged,
            bridge_shp_paged: &world.bridge_shp_paged,
            unit_instances: &world.unit,
            unit_transition_paged: &world.unit_transition_paged,
            shp_paged: &world.shp_paged,
            wall_instances: &world.wall,
            particle_paged: &world.particle_paged,
            ghost_page: ui.ghost_page,
        },
    );

    // Return unit instances vec to AppState (deferred until after the draw pass
    // because the multi-way merge needs the CPU-side Y values).
    state.cached_unit_instances = world.unit;
    Ok(sidebar.view)
}

/// Upload all per-frame instance vectors to the GPU buffer pool.
///
/// The pool reuses GPU buffers across frames to avoid per-frame allocation.
/// Buffer names here must match the keys used in `draw_passes::dispatch_draw_passes`.
fn upload_to_gpu(
    state: &mut AppState,
    world: &WorldInstances,
    debug: &DebugInstances,
    ui: &UiInstances,
    sidebar: &SidebarInstances,
) {
    let pool: &mut InstanceBufferPool = &mut state.instance_pool;

    // Debug overlays
    pool.upload(&state.gpu, "debug_pathgrid", &debug.pathgrid);
    pool.upload(&state.gpu, "debug_cell_grid", &debug.cell_grid);
    pool.upload(&state.gpu, "debug_path", &debug.path);
    pool.upload(&state.gpu, "debug_heightmap", &debug.heightmap);

    // Terrain + overlays
    pool.upload(&state.gpu, "terrain", &world.terrain.normal);
    pool.upload(&state.gpu, "terrain_cliff", &world.terrain.cliff_redraw);
    pool.upload(&state.gpu, "overlay", &world.overlay);
    pool.upload(&state.gpu, "overlay_bridge_body", &world.bridge_body);
    pool.upload(
        &state.gpu,
        "overlay_bridge_body_shadow",
        &world.bridge_body_shadow,
    );
    pool.upload(&state.gpu, "overlay_bridge_railing", &world.bridge_railing);
    pool.upload(&state.gpu, "overlay_wall", &world.wall);
    // Smudges: drawn after overlays, before bridge entities. Empty until the
    // SmudgeType SHP atlas registration follow-up lands.
    pool.upload(&state.gpu, "smudge", &world.smudge);

    // Entities (VXL + SHP)
    pool.upload(&state.gpu, "unit", &world.unit);
    pool.upload(&state.gpu, "unit_bridge", &world.bridge_unit);
    const UNIT_TRANSITION_KEYS: [&str; 4] = [
        "unit_transition_p0",
        "unit_transition_p1",
        "unit_transition_p2",
        "unit_transition_p3",
    ];
    const BRIDGE_UNIT_TRANSITION_KEYS: [&str; 4] = [
        "unit_bridge_transition_p0",
        "unit_bridge_transition_p1",
        "unit_bridge_transition_p2",
        "unit_bridge_transition_p3",
    ];
    for (i, page_inst) in world.unit_transition_paged.iter().enumerate() {
        if let Some(key) = UNIT_TRANSITION_KEYS.get(i) {
            pool.upload(&state.gpu, key, page_inst);
        }
    }
    for (i, page_inst) in world.bridge_unit_transition_paged.iter().enumerate() {
        if let Some(key) = BRIDGE_UNIT_TRANSITION_KEYS.get(i) {
            pool.upload(&state.gpu, key, page_inst);
        }
    }
    const SHP_PAGE_KEYS: [&str; 4] = ["shp_p0", "shp_p1", "shp_p2", "shp_p3"];
    const SHP_BRIDGE_KEYS: [&str; 4] = [
        "shp_bridge_p0",
        "shp_bridge_p1",
        "shp_bridge_p2",
        "shp_bridge_p3",
    ];
    for (i, page_inst) in world.shp_paged.iter().enumerate() {
        if i < SHP_PAGE_KEYS.len() {
            pool.upload(&state.gpu, SHP_PAGE_KEYS[i], page_inst);
        }
    }
    for (i, page_inst) in world.bridge_shp_paged.iter().enumerate() {
        if i < SHP_BRIDGE_KEYS.len() {
            pool.upload(&state.gpu, SHP_BRIDGE_KEYS[i], page_inst);
        }
    }
    pool.upload(&state.gpu, "building_turret", &world.building_turret);
    // PixelFX water/ore sparkles — drawn between ground objects (Step 5) and
    // turrets (Step 6). Empty when graphics.extra_animations is off.
    pool.upload(&state.gpu, "cell_sparkles", &world.cell_sparkles);
    const PARTICLE_KEYS: [&str; 4] = ["particle_p0", "particle_p1", "particle_p2", "particle_p3"];
    for (i, page_inst) in world.particle_paged.iter().enumerate() {
        if i < PARTICLE_KEYS.len() {
            pool.upload(&state.gpu, PARTICLE_KEYS[i], page_inst);
        }
    }

    // UI overlays
    pool.upload(&state.gpu, "drag", &ui.drag);
    pool.upload(&state.gpu, "selection_brackets_back", &ui.bracket_back);
    pool.upload(
        &state.gpu,
        "selection_brackets_front_first",
        &ui.bracket_front_first,
    );
    pool.upload(&state.gpu, "selection_brackets_front", &ui.bracket_front);
    pool.upload(&state.gpu, "building_radius_rings", &ui.radius_ring);
    pool.upload(&state.gpu, "status_building", &ui.building_status);
    pool.upload(&state.gpu, "occupant_pips", &ui.occupant_pip);
    pool.upload(&state.gpu, "status_unit_bg", &ui.unit_status_bg);
    pool.upload(&state.gpu, "status_unit_fill", &ui.unit_status_fill);
    pool.upload(&state.gpu, "cargo_pips", &ui.cargo_pip);
    pool.upload(&state.gpu, "software_cursor", &ui.software_cursor);
    pool.upload(&state.gpu, "placement_valid", &ui.placement_valid);
    pool.upload(&state.gpu, "placement_invalid", &ui.placement_invalid);
    pool.upload(&state.gpu, "placement_ghost", &ui.placement_ghost);
    pool.upload(&state.gpu, "placement_wall_ghost", &ui.wall_ghost);
    pool.upload(&state.gpu, "factory_rally_first", &ui.factory_rally_first);
    pool.upload(&state.gpu, "target_lines", &ui.target_line);
    pool.upload(&state.gpu, "factory_rally_second", &ui.factory_rally_second);

    // Sidebar + minimap
    pool.upload(&state.gpu, "minimap", &sidebar.minimap);
    pool.upload(&state.gpu, "viewport_rect", &sidebar.viewport_rect);
    pool.upload(&state.gpu, "sidebar", &sidebar.sidebar);
    pool.upload(&state.gpu, "sidebar_chrome", &sidebar.chrome);
    pool.upload(&state.gpu, "radar_anim", &sidebar.radar_anim);
    pool.upload(&state.gpu, "sidebar_cameo", &sidebar.cameo);
    pool.upload(&state.gpu, "sidebar_gclock", &sidebar.gclock);
    pool.upload(&state.gpu, "sidebar_cameo_overlay", &sidebar.cameo_overlay);
    pool.upload(&state.gpu, "sidebar_text", &sidebar.text);
}

#[cfg(test)]
#[path = "../app_render_tests.rs"]
mod tests;
