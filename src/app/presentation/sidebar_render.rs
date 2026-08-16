//! Sidebar view construction, minimap interaction, chrome helpers, and render pass.
//!
//! Refreshes the retained SidebarView projection at explicit state transitions,
//! handles minimap drag/click, resolves sidebar chrome theme, and creates the
//! main wgpu render pass.
//!
//! Instance builders for sidebar layers live in app_sidebar_build.rs.
//!
//! Extracted from app_render.rs to keep files under 400 lines.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use crate::app::AppState;
use crate::app::input::commands::preferred_local_owner_name;
use crate::render::batch::BatchTexture;
use crate::sidebar::{self, SidebarView};
use crate::sim::production;

// Re-export instance builders so callers don't need to know about the split.
pub(crate) use crate::app::presentation::sidebar_build::{
    build_sidebar_cameo_instances, build_sidebar_chrome_instances, build_sidebar_instances,
    build_sidebar_text_instances,
};

// ---------------------------------------------------------------------------
// Sidebar view construction
// ---------------------------------------------------------------------------

/// Return the one retained sidebar projection. Reading it never advances
/// credits, clears targeting, or clamps scroll state.
pub(crate) fn current_sidebar_view(state: &AppState) -> Option<&SidebarView> {
    state.sidebar_projection.view()
}

/// Advance the displayed balance at the authoritative gameplay-frame seam.
pub(crate) fn advance_sidebar_credits_after_frame(
    state: &mut AppState,
    frame_committed: bool,
    tick_lane: crate::sim::world::TickLane,
) {
    if !crate::app::sidebar_projection::credits_advance_for_frame(frame_committed, tick_lane) {
        return;
    }
    let owner_name = preferred_local_owner_name(state).unwrap_or_else(|| "Americans".to_string());
    let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
        return;
    };
    let credits = production::credits_for_owner(sim, &owner_name);
    state
        .sidebar_projection
        .advance_credits(&owner_name, credits);
}

/// Reconcile state-derived sidebar inputs and replace the retained immutable
/// projection. This is called only from explicit simulation/input/lifecycle
/// transitions, never from a view consumer.
pub(crate) fn refresh_sidebar_projection(state: &mut AppState) {
    let owner_name: String =
        preferred_local_owner_name(state).unwrap_or_else(|| "Americans".to_string());
    let Some((
        mut build_options,
        mut queue_items,
        mut ready_buildings,
        producer_focus,
        credits,
        power_produced,
        power_drained,
        sw_views,
    )) = (|| {
        let (sim, rules) = (state.sim_runtime.as_ref().map(|rt| &rt.simulation)?, state.rules()?);
        let producer_focus = [
            production::ProductionCategory::Building,
            production::ProductionCategory::Defense,
            production::ProductionCategory::Infantry,
            production::ProductionCategory::Vehicle,
            production::ProductionCategory::Aircraft,
        ]
        .into_iter()
        .filter_map(|category| {
            production::active_producer_for_owner_category(sim, rules, &owner_name, category)
        })
        .collect::<Vec<_>>();
        let owner_iid = sim.interner.get(&owner_name).unwrap_or_default();
        let sw_views = if sim.session.game_options.super_weapons {
            crate::sim::superweapon::superweapon_views_for_owner(sim, rules, &owner_iid)
        } else {
            Vec::new()
        };
        let (power_produced, power_drained) =
            production::power_balance_for_owner(sim, rules, &owner_name);
        Some((
            production::build_options_for_owner(sim, rules, &owner_name),
            production::queue_view_for_owner(sim, rules, &owner_name),
            production::ready_buildings_for_owner(sim, rules, &owner_name),
            producer_focus,
            production::credits_for_owner(sim, &owner_name),
            power_produced,
            power_drained,
            sw_views,
        ))
    })()
    else {
        state.sidebar_projection.replace_view(None);
        return;
    };

    // Resolve CSF display names (e.g., "Name:MTNK" → "Grizzly Battle Tank").
    if let Some(csf) = &state.csf {
        for opt in &mut build_options {
            opt.display_name = resolve_csf_name(csf, &opt.display_name);
        }
        for item in &mut queue_items {
            item.display_name = resolve_csf_name(csf, &item.display_name);
        }
        for ready in &mut ready_buildings {
            ready.display_name = resolve_csf_name(csf, &ready.display_name);
        }
    }
    let display_credits = state
        .sidebar_projection
        .displayed_credits_or_seed(&owner_name, credits);
    let (tab_btn_size, repair_btn_size, sell_btn_size, scroll_down_btn_size, scroll_up_btn_size) = {
        let scale = state.ui_scale;
        let size = |entry: Option<&crate::render::sidebar_chrome::SidebarChromeEntry>| {
            entry.map(|entry| [entry.pixel_size[0] * scale, entry.pixel_size[1] * scale])
        };
        let atlas = current_sidebar_chrome(state);
        (
            size(atlas.and_then(|atlas| atlas.tab_frames[0][0].as_ref())),
            size(atlas.and_then(|atlas| atlas.repair_frames[0].as_ref())),
            size(atlas.and_then(|atlas| atlas.sell_frames[0].as_ref())),
            size(atlas.and_then(|atlas| atlas.scroll_down_frames[0].as_ref())),
            size(atlas.and_then(|atlas| atlas.scroll_up_frames[0].as_ref())),
        )
    };
    sync_targeting_mode(
        &mut state.targeting_mode,
        &mut state.building_placement_preview,
        &ready_buildings,
        &sw_views,
        state.sim_runtime.as_ref().map(|rt| &rt.simulation).map(|s| &s.interner),
    );
    // App targeting state -> the sidebar-owned armed projection (F06 seam).
    let armed_entry = state.targeting_mode.as_ref().map(|mode| match mode {
        crate::app::types::TargetingMode::BuildingPlacement(section) => {
            sidebar::ArmedSidebarEntry::BuildingPlacement(section.clone())
        }
        crate::app::types::TargetingMode::SuperWeapon(section) => {
            sidebar::ArmedSidebarEntry::SuperWeapon(section.clone())
        }
    });
    let mut view = sidebar::build_sidebar_view_with_spec(
        state.sidebar_layout_spec,
        state.render_width() as f32,
        state.render_height() as f32,
        state.active_sidebar_tab,
        display_credits,
        power_produced,
        power_drained,
        tab_btn_size,
        &queue_items,
        &build_options,
        &ready_buildings,
        armed_entry.as_ref(),
        &producer_focus,
        state.sidebar_scroll_rows,
        state.sim_runtime.as_ref().map(|rt| &rt.simulation).map(|sim| &sim.interner),
        &sw_views,
        &state.sidebar_gadget_state,
        repair_btn_size,
        sell_btn_size,
        scroll_down_btn_size,
        scroll_up_btn_size,
    );
    state.sidebar_scroll_rows = view.scroll_rows;
    if let Some(atlas) = state.sidebar_cameo_atlas.as_ref() {
        for item in &mut view.items {
            item.has_cameo_art = atlas.get(&item.type_id).is_some();
        }
    }
    state.sidebar_projection.replace_view(Some(view));
}

pub(crate) fn sync_targeting_mode(
    targeting_mode: &mut Option<crate::app::types::TargetingMode>,
    building_placement_preview: &mut Option<crate::sim::production::BuildingPlacementPreview>,
    ready_buildings: &[production::ReadyBuildingView],
    super_weapons: &[crate::sim::superweapon::SuperWeaponView],
    interner: Option<&crate::sim::intern::StringInterner>,
) {
    let still_valid = match targeting_mode.as_ref() {
        None => true,
        Some(crate::app::types::TargetingMode::BuildingPlacement(armed)) => {
            ready_buildings.iter().any(|ready| {
                interner.map_or(false, |i| {
                    i.resolve(ready.type_id).eq_ignore_ascii_case(armed)
                })
            })
        }
        Some(crate::app::types::TargetingMode::SuperWeapon(section)) => super_weapons
            .iter()
            .any(|sw| sw.is_ready && sw.display_name.eq_ignore_ascii_case(section)),
    };
    if !still_valid {
        *targeting_mode = None;
        *building_placement_preview = None;
    }
}

// ---------------------------------------------------------------------------
// Minimap interaction
// ---------------------------------------------------------------------------

pub(crate) fn is_cursor_over_minimap(state: &AppState) -> bool {
    // Minimap interaction disabled when radar is not online.
    let minimap_visible: bool = state
        .radar_anim
        .as_ref()
        .map_or(true, |ra| ra.is_minimap_visible());
    if !minimap_visible {
        return false;
    }
    let Some(minimap) = &state.minimap else {
        return false;
    };
    let rect = active_minimap_screen_rect(state);
    minimap.contains_screen_point_in_rect(
        state.input.cursor_x,
        state.input.cursor_y,
        rect.x,
        rect.y,
        rect.w,
        rect.h,
    )
}

pub(crate) fn try_begin_minimap_drag(state: &mut AppState) -> bool {
    if !is_cursor_over_minimap(state) {
        return false;
    }
    // If units are selected, left-click on minimap issues a move order
    // to the clicked world position instead of dragging the camera.
    if minimap_move_order_if_selected(state) {
        return true;
    }
    state.minimap_dragging = true;
    state.selection_state.cancel_drag();
    update_camera_from_minimap_cursor(state);
    true
}

/// If there are selected mobile units, issue a move command to the minimap
/// click location and return true. Otherwise return false (caller does camera drag).
fn minimap_move_order_if_selected(state: &mut AppState) -> bool {
    let selected_ids = crate::app::input::dispatch::selected_stable_ids_in_order(state);
    let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
        return false;
    };
    if selected_ids.is_empty() {
        return false;
    }
    // Convert minimap cursor position to world iso coordinates.
    let (target_rx, target_ry) = match minimap_cursor_to_iso(state) {
        Some(coords) => coords,
        None => return false,
    };
    let owner = crate::app::input::commands::preferred_local_owner_name(state)
        .unwrap_or_else(|| "Americans".to_string());
    let owner_id = sim.interner.get(&owner).unwrap_or_default();
    let execute_tick = sim.session.tick;
    let order_mode = state.queued_order_mode;
    let shift_held: bool = crate::app::input::dispatch::is_shift_held(state);
    let mut queued: Vec<crate::sim::command::CommandEnvelope> = Vec::new();
    for &entity_id in &selected_ids {
        let Some(entity) = sim.entities().get(entity_id) else {
            continue;
        };
        // Only issue move to non-structure entities.
        if entity.category == crate::map::entities::EntityCategory::Structure {
            continue;
        }
        let mut goal = (target_rx, target_ry);
        if let Some(grid) = sim.path_grid() {
            if !crate::app::match_runtime::sim_tick::is_any_layer_walkable(grid, goal.0, goal.1) {
                if let Some(nearest) =
                    crate::app::match_runtime::sim_tick::nearest_walkable_cell_layered(grid, goal, 12)
                {
                    goal = nearest;
                }
            }
        }
        let command = match order_mode {
            crate::app::presentation::render::OrderMode::AttackMove => crate::sim::command::Command::AttackMove {
                entity_id,
                target_rx: goal.0,
                target_ry: goal.1,
                queue: shift_held,
            },
            _ => crate::sim::command::Command::Move {
                entity_id,
                target_rx: goal.0,
                target_ry: goal.1,
                queue: shift_held,
                group_id: None,
            },
        };
        queued.push(crate::sim::command::CommandEnvelope::new(
            owner_id,
            execute_tick,
            command,
        ));
    }
    if queued.is_empty() {
        return false;
    }
    // Reset order mode after issuing the command (like the main viewport does).
    if order_mode != crate::app::presentation::render::OrderMode::Move {
        state.queued_order_mode = crate::app::presentation::render::OrderMode::Move;
    }
    if let Some(sim) = state.sim_runtime.as_mut().map(|rt| &mut rt.simulation) {
        let queued = queued
            .into_iter()
            .filter_map(|envelope| {
                crate::app::input::commands::roundtrip_ordinary_local_move(sim, envelope)
            })
            .collect::<Vec<_>>();
        sim.queue_commands(queued);
    }
    true
}

/// Convert the current minimap cursor position to iso (rx, ry) coordinates.
/// Returns None if no minimap is available.
fn minimap_cursor_to_iso(state: &AppState) -> Option<(u16, u16)> {
    let minimap = state.minimap.as_ref()?;
    let (tactical_w, tactical_h) =
        crate::app::input::camera::tactical_viewport_size_px(state.render_width(), state.render_height());
    let tactical_w = tactical_w as f32;
    let tactical_h = tactical_h as f32;
    let z = state.input.zoom_level;
    let rect = active_minimap_screen_rect(state);
    // camera_top_left_for_screen_point_in_rect returns the camera top-left that
    // would center the viewport on the clicked point. We want the world center point.
    // Visible world area = screen / zoom.
    let (cam_x, cam_y) = minimap.camera_top_left_for_screen_point_in_rect(
        state.input.cursor_x,
        state.input.cursor_y,
        tactical_w / z,
        tactical_h / z,
        rect.x,
        rect.y,
        rect.w,
        rect.h,
    );
    // The center of the viewport is what was clicked.
    let world_x = cam_x + tactical_w / (2.0 * z);
    let world_y = cam_y + tactical_h / (2.0 * z);
    Some(crate::app::match_runtime::sim_tick::world_point_to_cell(
        world_x,
        world_y,
        &state.height_map(),
        Some(&state.tactical_bridge_inverse_map),
    ))
}

pub(crate) fn update_camera_from_minimap_cursor(state: &mut AppState) {
    let Some(minimap) = &state.minimap else {
        return;
    };
    let sw = state.render_width() as f32;
    let sh = state.render_height() as f32;
    let (tactical_w, tactical_h) =
        crate::app::input::camera::tactical_viewport_size_px(state.render_width(), state.render_height());
    let z = state.input.zoom_level;
    let rect = active_minimap_screen_rect(state);
    let (cx, cy) = minimap.camera_top_left_for_screen_point_in_rect(
        state.input.cursor_x,
        state.input.cursor_y,
        tactical_w as f32 / z,
        tactical_h as f32 / z,
        rect.x,
        rect.y,
        rect.w,
        rect.h,
    );
    state.input.camera_x = cx;
    state.input.camera_y = cy;
    crate::app::input::camera::clamp_camera_to_playable_area(state, sw, sh);
}

pub(crate) fn active_minimap_screen_rect(state: &AppState) -> crate::sidebar::Rect {
    let sw = state.render_width() as f32;
    let sh = state.render_height() as f32;
    if current_sidebar_chrome(state).is_some() {
        // These position the minimap content exactly inside the BKGDLG.SHP chrome border.
        const MINIMAP_LEFT: f32 = 13.0;
        const MINIMAP_TOP: f32 = 0.0;
        const MINIMAP_WIDTH: f32 = 140.0;
        const MINIMAP_HEIGHT: f32 = 120.0;

        let spec = state.sidebar_layout_spec;
        let s = state.ui_scale;
        let sidebar_x = sw - spec.sidebar_width + spec.x_offset;
        crate::sidebar::Rect {
            x: sidebar_x + MINIMAP_LEFT * s,
            y: spec.top_inset + MINIMAP_TOP * s,
            w: MINIMAP_WIDTH * s,
            h: MINIMAP_HEIGHT * s,
        }
    } else {
        let (x, y, w, h) = crate::render::minimap::default_minimap_rect(sh);
        crate::sidebar::Rect { x, y, w, h }
    }
}

// ---------------------------------------------------------------------------
// Chrome / theme helpers
// ---------------------------------------------------------------------------

pub(crate) fn current_sidebar_chrome_texture(state: &AppState) -> Option<&BatchTexture> {
    current_sidebar_chrome(state).map(|atlas| &atlas.texture)
}

pub(crate) fn current_sidebar_gclock_texture(state: &AppState) -> Option<&BatchTexture> {
    current_sidebar_chrome(state).and_then(|atlas| atlas.gclock_texture.as_ref())
}

pub(crate) fn current_sidebar_theme(
    state: &AppState,
) -> crate::render::sidebar_chrome::SidebarTheme {
    preferred_local_owner_name(state)
        .and_then(|owner| {
            sidebar_theme_for_owner_sources(state.sim_runtime.as_ref().map(|rt| &rt.simulation), &state.house_roster, &owner)
        })
        .unwrap_or(crate::render::sidebar_chrome::SidebarTheme::Allied)
}

pub(crate) fn current_sidebar_chrome(
    state: &AppState,
) -> Option<&crate::render::sidebar_chrome::SidebarChromeAtlas> {
    let set = state.sidebar_chrome.as_ref()?;
    let theme = current_sidebar_theme(state);
    set.for_theme(theme)
}

pub(crate) fn sidebar_theme_for_owner_sources(
    simulation: Option<&crate::sim::world::Simulation>,
    house_roster: &crate::map::houses::HouseRoster,
    owner: &str,
) -> Option<crate::render::sidebar_chrome::SidebarTheme> {
    if let Some(house) = house_roster
        .houses
        .iter()
        .find(|house| house.name.eq_ignore_ascii_case(owner))
    {
        let side = house
            .side
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let country = house
            .country
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();

        if side.contains("yuri") || country.contains("yuri") {
            return Some(crate::render::sidebar_chrome::SidebarTheme::Yuri);
        }
        if side.contains("soviet")
            || matches!(
                country.as_str(),
                "russia" | "iraq" | "cuba" | "libya" | "soviet"
            )
        {
            return Some(crate::render::sidebar_chrome::SidebarTheme::Soviet);
        }
        return Some(crate::render::sidebar_chrome::SidebarTheme::Allied);
    }

    // Map-loaded houses can still default a missing Side= to Allied. Preserve
    // the existing roster decision until that producer is exact; ordinary
    // explicit launch names miss the map roster and carry the resolved live
    // side. A deliberate name collision therefore keeps the roster decision.
    let simulation = simulation?;
    let live_house = crate::sim::house_state::house_state_for_owner(
        &simulation.houses,
        owner,
        &simulation.interner,
    )?;
    match live_house.side_index {
        0 => Some(crate::render::sidebar_chrome::SidebarTheme::Allied),
        1 => Some(crate::render::sidebar_chrome::SidebarTheme::Soviet),
        2 => Some(crate::render::sidebar_chrome::SidebarTheme::Yuri),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// CSF display name resolution
// ---------------------------------------------------------------------------

/// Resolve a display name through the CSF string table.
///
/// Rules `Name=` values are CSF keys (e.g., `"Name:MTNK"`). Retail emits its
/// visible `MISSING:'<key>'` marker when the initialized table lacks a key.
fn resolve_csf_name(csf: &crate::assets::csf_file::CsfFile, name: &str) -> String {
    csf.text(name).into_owned()
}

// ---------------------------------------------------------------------------
// Render pass creation
// ---------------------------------------------------------------------------

/// Create the main render pass with depth buffer and clear.
pub(crate) fn begin_main_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
    depth_view: &'a wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Main Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(crate::app::presentation::render::CLEAR_COLOR),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth_view,
            stencil_ops: None,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    })
}

/// Resume the tactical composition after a destination-dependent encoded-byte
/// surface edit. Both attachments retain the work produced by the first pass.
pub(crate) fn begin_main_load_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
    depth_view: &'a wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Main Pass (resume after combat lights)"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth_view,
            stencil_ops: None,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            }),
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    })
}
