//! In-game input handling — mouse clicks, hotkeys, sidebar interactions,
//! control groups, and selection commands.
//!
//! Context-sensitive order resolution (click → command) lives in
//! app_context_order.rs. This file handles raw input dispatch and UI state.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on everything.

use winit::event::{ElementState, MouseButton};
use winit::keyboard::KeyCode;

use crate::app::AppState;
use crate::app_commands::{
    cancel_build_by_type, cancel_last_build, cycle_active_producer, cycle_local_owner,
    place_ready_building_at_cursor, place_starter_base_for_local_owner, preferred_local_owner,
    preferred_local_owner_name, queue_build_by_type, schedule_command,
    spawn_test_units_for_local_owner, toggle_pause_build_queue,
};
use crate::app_context_order::try_queue_context_order_at_screen_point;
use crate::app_entity_pick::{compute_box_selection_snapshot, compute_click_selection_snapshot};
use crate::app_sidebar_render::current_sidebar_view;
use crate::app_types::OrderMode;
use crate::audio::events::GameSoundEvent;
use crate::map::entities::EntityCategory;
use crate::sidebar::{SidebarAction, SidebarTab};
use crate::sim::command::Command;
use crate::sim::selection::SelectAction;

/// Click radius for single-click selection (pixels in world space).
pub(crate) const CLICK_SELECT_RADIUS: f32 = 30.0;

/// Handle mouse button press/release for selection and move commands.
///
/// The in-game gadget walk runs FIRST (study G22/A8): chrome buttons + cameos
/// fire/consume there; the full-tactical catcher and the minimap region decide
/// WHICH body runs (the regions are sticky, so a drag stays bound to its region
/// across the sidebar boundary). Middle-mouse pan is never a gadget event and is
/// handled directly. A `NotConsumed` click hits no live gadget — only the legacy
/// dev/pause/producer press path runs; empty-sidebar / off-window clicks do
/// nothing (gamemd's sidebar-body gadget swallows them, A6). Right-press is owned
/// by the tactical catcher (viewport-only), so right-clicking dead sidebar chrome
/// no longer deselects — matching gamemd.
pub(crate) fn handle_mouse_input(
    state: &mut AppState,
    button: MouseButton,
    btn_state: ElementState,
) {
    use crate::app_gadget_input::GadgetConsume;
    let pressed = btn_state.is_pressed();
    // Paused in-game Options overlay owns the mouse: route press/release/checkbox/
    // Back here and CONSUME the click so it never reaches the tactical viewport or
    // a gadget (no unit orders behind the overlay). KD-6.
    if state.paused {
        // VERA-internal (gamemd has no pause overlay; gamemd equivalent
        // UNCHECKED): a release that arrives while the overlay owns the mouse
        // never reaches the tactical body, so the capture is dropped here.
        // Leaving it set would freeze edge auto-scroll for the rest of the match.
        if !pressed {
            state.tactical_mouse.left_held = false;
            state.tactical_mouse.right_held = false;
            state.tactical_mouse.release();
        }
        crate::app_in_game_options_input::in_game_options_mouse(state, button, pressed);
        return;
    }
    // Middle-mouse pan: not a gadget event, handle directly.
    if button == MouseButton::Middle {
        if pressed {
            state.middle_mouse_panning = true;
            state.middle_mouse_anchor_x = state.cursor_x;
            state.middle_mouse_anchor_y = state.cursor_y;
        } else {
            state.middle_mouse_panning = false;
        }
        return;
    }
    match crate::app_gadget_input::handle_mouse_button_event(state, button, pressed) {
        GadgetConsume::Tactical => tactical_mouse(state, button, btn_state),
        GadgetConsume::Minimap => minimap_mouse(state, button, btn_state),
        // Consumed (chrome/cameo/control button) → handled by the gadget.
        // NotConsumed → the click hit no live gadget; nothing to do (every
        // in-game surface is now on the retained list — R7 complete).
        GadgetConsume::Consumed | GadgetConsume::NotConsumed => {}
    }
}

/// Tactical-viewport mouse body (routed here when the full-tactical ClickRegion
/// consumes the edge — i.e. a click in the play area, or a captured drag/release
/// that started there). Logic is the legacy handler's tactical path, unchanged;
/// the minimap-drag-end and minimap-begin checks moved to `minimap_mouse`, and
/// middle-pan moved to the dispatcher.
pub(crate) fn tactical_mouse(state: &mut AppState, button: MouseButton, btn_state: ElementState) {
    match button {
        MouseButton::Left => {
            if btn_state.is_pressed() {
                // gamemd takes the mouse capture on the press edge whether or
                // not the band drag arms: the modal gates live inside the
                // drag-arm helper, not around the capture. The capture is what
                // freezes edge auto-scroll for the length of the gesture.
                state.tactical_mouse.left_held = true;
                state.tactical_mouse.captured = true;
                if state.targeting_mode.is_some()
                    || state.sidebar_gadget_state.repair_mode_on
                    || state.sidebar_gadget_state.sell_mode_on
                {
                    return; // suppress selection drag while a targeting / repair / sell mode is active
                }
                state
                    .selection_state
                    .begin_drag(state.cursor_x, state.cursor_y);
            } else {
                state.tactical_mouse.left_held = false;
                state.tactical_mouse.captured = false;
                // Repair / Sell cursor modes consume the click — toggle repair or
                // sell the own building under the cursor. The mode stays active
                // (sticky) so the player can act on several buildings in a row.
                if crate::app_commands::try_repair_sell_mode_click(state) {
                    return;
                }
                if let Some(section) = state.armed_super_weapon_type().map(str::to_owned) {
                    crate::app_commands::launch_super_weapon_at_cursor(state, &section);
                    return;
                }
                if let Some(type_id) = state.armed_building_type().map(str::to_owned) {
                    place_ready_building_at_cursor(state, &type_id);
                    return;
                }
                let mut action: SelectAction = state
                    .selection_state
                    .end_drag(state.cursor_x, state.cursor_y);
                let shift = is_shift_held(state);
                // A band box that caught no drawn object leaves the selection
                // exactly as it was, and the release is handled as an ordinary
                // click at the release point — the native release only clears
                // when something was inside the rectangle.
                //
                // The whole empty/clear/fall-through block sits inside the
                // native "shift is not held" arm, and the fall-through flag is
                // the only thing that lets control reach the click/action path.
                // So a shift drag that catches nothing does nothing at all: it
                // must not walk the army to the release point.
                if let SelectAction::BoxSelect(min_x, min_y, max_x, max_y) = action {
                    if !shift && !band_caught_drawn_object(state, min_x, min_y, max_x, max_y) {
                        action = SelectAction::Click(state.cursor_x, state.cursor_y);
                    }
                }
                // On a single click (not drag-box), try issuing a command first.
                // If the click lands on a friendly unit/building, fall through to
                // selection instead (select_friendly_clicks=true).
                if let SelectAction::Click(_, _) = action {
                    let commanded: bool = try_queue_context_order_at_screen_point(
                        state,
                        state.cursor_x,
                        state.cursor_y,
                        true, // select_friendly_clicks: let friendly clicks fall through to selection
                    );
                    if commanded {
                        return;
                    }
                }
                let mut queued_selection: Option<Vec<u64>> = None;
                if let Some(sim) = &state.simulation {
                    match action {
                        SelectAction::Click(sx, sy) => {
                            let world_x: f32 = sx / state.zoom_level + state.camera_x;
                            let world_y: f32 = sy / state.zoom_level + state.camera_y;
                            let fog_ref = if state.sandbox_full_visibility {
                                None
                            } else {
                                Some(&sim.fog)
                            };
                            queued_selection = compute_click_selection_snapshot(
                                sim.entities(),
                                fog_ref,
                                preferred_local_owner_name(state).as_deref(),
                                world_x,
                                world_y,
                                CLICK_SELECT_RADIUS,
                                shift,
                                state.rules.as_ref(),
                                Some(&sim.houses),
                                &state.height_map,
                                Some(&state.tactical_bridge_inverse_map),
                                Some(&sim.interner),
                            );
                        }
                        SelectAction::BoxSelect(min_x, min_y, max_x, max_y) => {
                            let fog_ref = if state.sandbox_full_visibility {
                                None
                            } else {
                                Some(&sim.fog)
                            };
                            let z = state.zoom_level;
                            queued_selection = compute_box_selection_snapshot(
                                sim.entities(),
                                fog_ref,
                                preferred_local_owner_name(state).as_deref(),
                                min_x / z + state.camera_x,
                                min_y / z + state.camera_y,
                                max_x / z + state.camera_x,
                                max_y / z + state.camera_y,
                                shift,
                                state.rules.as_ref(),
                                Some(&sim.houses),
                                Some(&sim.interner),
                            );
                        }
                        SelectAction::None => {}
                    }
                }
                if let Some(snapshot) = queued_selection {
                    // Emit VoiceSelect for the first selected unit type.
                    emit_selection_voice(state, &snapshot);
                    queue_selection_snapshot_command(state, snapshot, shift);
                }
            }
        }
        MouseButton::Right => {
            if btn_state.is_pressed() {
                state.tactical_mouse.right_held = true;
                // The native right press has no game effect at all: it records
                // the pan anchor and takes the capture, and only does that when
                // no other button already holds it. Everything the player sees
                // happens on the release edge.
                if !state.tactical_mouse.captured {
                    state
                        .tactical_mouse
                        .begin_right_drag((state.cursor_x, state.cursor_y));
                }
            } else {
                state.tactical_mouse.right_held = false;
                if state.tactical_mouse.captured {
                    // The cancel ladder runs only when the drag threshold was
                    // never crossed. A right drag that panned the map ends
                    // silently — the selection survives it.
                    let run_cancel_ladder = !state.tactical_mouse.right_threshold_crossed;
                    state.tactical_mouse.release();
                    if run_cancel_ladder {
                        right_click_cancel_ladder(state);
                    }
                    // The native release tears the band rectangle down too.
                    state.selection_state.cancel_drag();
                }
            }
        }
        _ => {}
    }
}

/// The right-release cancel ladder: cancel exactly one armed cursor mode, and
/// clear the selection only when nothing was armed.
///
/// gamemd walks seven mode flags in a fixed order and returns after the first
/// one it cancels; VERA models the two it has (a targeting/placement cursor and
/// the repair/sell cursor). The final rung — clear the selection — is retail
/// behaviour, not a VERA deviation.
fn right_click_cancel_ladder(state: &mut AppState) {
    if state.targeting_mode.is_some() {
        state.targeting_mode = None;
        state.building_placement_preview = None;
        return;
    }
    if state.sidebar_gadget_state.repair_mode_on || state.sidebar_gadget_state.sell_mode_on {
        state.sidebar_gadget_state.repair_mode_on = false;
        state.sidebar_gadget_state.sell_mode_on = false;
        return;
    }
    queue_selection_snapshot_command(state, Vec::new(), false);
}

/// Did the band rectangle cover any drawn object? Screen-space rectangle in, the
/// native "is the box empty" answer out.
fn band_caught_drawn_object(
    state: &AppState,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
) -> bool {
    let Some(sim) = &state.simulation else {
        return false;
    };
    let z = state.zoom_level;
    let fog_ref = if state.sandbox_full_visibility {
        None
    } else {
        Some(&sim.fog)
    };
    crate::app_entity_pick::band_rect_contains_drawn_object(
        sim.entities(),
        fog_ref,
        preferred_local_owner_name(state).as_deref(),
        min_x / z + state.camera_x,
        min_y / z + state.camera_y,
        max_x / z + state.camera_x,
        max_y / z + state.camera_y,
        Some(&sim.interner),
    )
}

/// Minimap mouse body (routed here when the minimap ClickRegion consumes the
/// edge). gamemd (decompile 0x006539D0) centers the tactical view on press
/// edges (left OR right) and IGNORES held motion — there is no continuous
/// camera-follow (the held branch is dropped in `handle_cursor_moved_in_game`).
pub(crate) fn minimap_mouse(state: &mut AppState, button: MouseButton, btn_state: ElementState) {
    match button {
        MouseButton::Left => {
            if btn_state.is_pressed() {
                crate::app_sidebar_render::try_begin_minimap_drag(state);
            } else if state.minimap_dragging {
                state.minimap_dragging = false;
            }
        }
        MouseButton::Right => {
            // A right-press centers the view on the clicked cell (no command);
            // right-release just releases the gadget's sticky capture.
            if btn_state.is_pressed() && crate::app_sidebar_render::is_cursor_over_minimap(state) {
                crate::app_sidebar_render::update_camera_from_minimap_cursor(state);
            }
        }
        _ => {}
    }
}

/// Handle cursor-move behavior while in-game.
///
/// While a minimap press is held the camera does NOT follow (gamemd ignores
/// held minimap motion); the move is swallowed so it can't start a selection
/// drag. Otherwise this updates the unit-selection drag rectangle.
/// Speed multiplier for middle-mouse camera panning. Each pixel of mouse movement
/// translates to this many pixels of camera scroll, making it feel fast and responsive.
const MIDDLE_MOUSE_PAN_SPEED: f32 = 3.0;

pub(crate) fn handle_cursor_moved_in_game(state: &mut AppState) {
    // Paused in-game Options overlay: drive a live slider drag (visual/stored only —
    // cadence applies on close, KD-8) and swallow the move so it can't begin a
    // selection drag or camera pan behind the overlay.
    if state.paused {
        crate::app_in_game_options_input::in_game_options_drag(state);
        return;
    }
    // Minimap: gamemd re-centers only on press edges and ignores held motion
    // (decompile 0x006539D0: `param_1 & 0x22` early-out). While a minimap press
    // is held we do NOT follow the cursor — but still swallow the move so it
    // doesn't begin a unit selection-drag.
    if state.minimap_dragging {
        return;
    }
    if state.middle_mouse_panning {
        let dx: f32 = state.cursor_x - state.middle_mouse_anchor_x;
        let dy: f32 = state.cursor_y - state.middle_mouse_anchor_y;
        // Divide by zoom so screen-space mouse delta maps to correct world distance.
        state.camera_x -= dx * MIDDLE_MOUSE_PAN_SPEED / state.zoom_level;
        state.camera_y -= dy * MIDDLE_MOUSE_PAN_SPEED / state.zoom_level;
        state.middle_mouse_anchor_x = state.cursor_x;
        state.middle_mouse_anchor_y = state.cursor_y;
        // Clamp after panning so camera stays within map bounds.
        let sw: f32 = state.render_width() as f32;
        let sh: f32 = state.render_height() as f32;
        crate::app_camera::clamp_camera_to_playable_area(state, sw, sh);
        return;
    }
    // Clamp drag position to the tactical viewport (exclude sidebar area).
    let viewport_w = state.render_width() as f32;
    let viewport_h = state.render_height() as f32;
    let drag_x = state.cursor_x.clamp(0.0, viewport_w - 1.0);
    let drag_y = state.cursor_y.clamp(0.0, viewport_h - 1.0);

    // Activation arms the rectangle and nothing else. The call gamemd makes at
    // that moment is a cursor-shape setter, not an unselect — the selection is
    // only replaced on the release, and only when the box caught something.
    state.selection_state.update_drag(drag_x, drag_y);
}

/// The sidebar command one wheel notch resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WheelAction {
    /// Scroll the active build strip up one row.
    SidebarUp,
    /// Scroll the active build strip down one row.
    SidebarDown,
}

/// Resolve a wheel notch to its sidebar command.
///
/// gamemd's window procedure intercepts every wheel message and executes the
/// command named `SidebarDown` when the delta is negative and `SidebarUp`
/// otherwise — a zero delta goes up, because the test is a signed less-than.
/// Magnitude is not a multiplier: one message is one command is one row.
pub(crate) fn wheel_action(delta_lines: f32) -> WheelAction {
    if delta_lines < 0.0 {
        WheelAction::SidebarDown
    } else {
        WheelAction::SidebarUp
    }
}

/// Apply one wheel notch to a strip's scroll row.
///
/// The native scroll refuses to move above row 0 or past the strip's computed
/// visible capacity, so both ends saturate rather than wrap.
pub(crate) fn wheel_scrolled_row(current: usize, max_rows: usize, action: WheelAction) -> usize {
    match action {
        WheelAction::SidebarUp => current.saturating_sub(1),
        WheelAction::SidebarDown => (current + 1).min(max_rows),
    }
}

/// Scroll the active build strip by one row.
///
/// The cursor position is deliberately not consulted. The retail binding is a
/// window message routed straight to a command, not a hit-tested gadget, so the
/// wheel scrolls the sidebar from anywhere on the screen — and there is no world
/// zoom in gamemd for the wheel to reach instead.
pub(crate) fn sidebar_wheel_scroll(state: &mut AppState, delta_lines: f32) {
    let Some(view) = current_sidebar_view(state) else {
        return;
    };
    state.sidebar_scroll_rows = wheel_scrolled_row(
        state.sidebar_scroll_rows,
        view.max_scroll_rows,
        wheel_action(delta_lines),
    );
}

/// Index of a tab's parked scroll row. Exhaustive on purpose: a new tab must
/// claim a slot rather than silently share one.
pub(crate) fn tab_scroll_slot(tab: SidebarTab) -> usize {
    match tab {
        SidebarTab::Building => 0,
        SidebarTab::Defense => 1,
        SidebarTab::Infantry => 2,
        SidebarTab::Vehicle => 3,
    }
}

pub(crate) fn apply_sidebar_action(state: &mut AppState, action: SidebarAction) {
    match action {
        SidebarAction::None => {}
        SidebarAction::SelectTab(tab) => {
            // gamemd's scroll row is per build strip, so switching tabs must not
            // carry the outgoing strip's position over — nor throw it away. Park
            // the row we are leaving and restore the one we are entering.
            if tab != state.active_sidebar_tab {
                state.sidebar_scroll_rows_parked[tab_scroll_slot(state.active_sidebar_tab)] =
                    state.sidebar_scroll_rows;
                state.active_sidebar_tab = tab;
                state.sidebar_scroll_rows = state.sidebar_scroll_rows_parked[tab_scroll_slot(tab)];
            }
        }
        SidebarAction::BuildType(type_id) => {
            queue_build_by_type(state, &type_id);
        }
        SidebarAction::ArmPlacement(type_id) => {
            state.targeting_mode =
                Some(crate::app_types::TargetingMode::BuildingPlacement(type_id));
            state.sidebar_gadget_state.repair_mode_on = false;
            state.sidebar_gadget_state.sell_mode_on = false;
        }
        SidebarAction::ClearPlacementMode => {
            state.targeting_mode = None;
            state.building_placement_preview = None;
        }
        SidebarAction::ArmSuperWeapon(section) => {
            state.targeting_mode = Some(crate::app_types::TargetingMode::SuperWeapon(section));
            // Mutual exclusion: clear building-placement preview AND repair/sell modes.
            state.building_placement_preview = None;
            state.sidebar_gadget_state.repair_mode_on = false;
            state.sidebar_gadget_state.sell_mode_on = false;
            log::info!(
                "SuperWeapon armed: type={}",
                state.armed_super_weapon_type().unwrap_or("")
            );
        }
        SidebarAction::ClearSuperWeaponMode => {
            state.targeting_mode = None;
            log::info!("SuperWeapon targeting cleared");
        }
        SidebarAction::TogglePauseQueue(category) => {
            toggle_pause_build_queue(state, category);
        }
        SidebarAction::CycleProducer(category) => {
            cycle_active_producer(state, category);
        }
        SidebarAction::CancelBuild(type_id) => {
            cancel_build_by_type(state, &type_id);
        }
        SidebarAction::CancelLastBuild => {
            cancel_last_build(state);
        }
        SidebarAction::CycleOwner => {
            cycle_local_owner(state);
        }
        SidebarAction::PlaceStarterBase => {
            place_starter_base_for_local_owner(state);
        }
        SidebarAction::SpawnTestUnits => {
            spawn_test_units_for_local_owner(state);
        }
        SidebarAction::ToggleRepairMode => {
            let g = &mut state.sidebar_gadget_state;
            g.repair_mode_on = !g.repair_mode_on;
            if g.repair_mode_on {
                g.sell_mode_on = false;
                state.targeting_mode = None;
                state.building_placement_preview = None;
            }
        }
        SidebarAction::ToggleSellMode => {
            let g = &mut state.sidebar_gadget_state;
            g.sell_mode_on = !g.sell_mode_on;
            if g.sell_mode_on {
                g.repair_mode_on = false;
                state.targeting_mode = None;
                state.building_placement_preview = None;
            }
        }
        SidebarAction::Deploy => {
            queue_deploy_undeploy_for_selected(state);
        }
    }
}

/// Toggle the unit-inspector debug overlay.
///
/// Beyond flipping `state.debug_unit_inspector`, this allocates per-entity
/// debug logs on enable and frees them on disable, and sets the sim flag
/// `debug_event_logging`. Called by both the X hotkey and the dev overlay
/// checkbox so the two paths cannot drift.
/// Explain every terrain cell that draws as black, and say which cause is responsible.
///
/// A cell renders black for exactly two reasons, and they need completely different
/// fixes: the vision system never revealed it, or it was revealed but its tile key is
/// missing from the atlas. Both look identical on screen, so this counts them separately
/// instead of leaving the diagnosis to guesswork.
pub(crate) fn report_black_cell_causes(state: &mut AppState) {
    let Some(grid) = state.terrain_grid.as_ref() else {
        log::info!("Black-cell report: no terrain grid loaded");
        return;
    };

    let owner = crate::app_commands::preferred_local_owner_name(state)
        .and_then(|name| state.simulation.as_ref()?.interner.get(&name));
    let fog = match (&state.simulation, owner) {
        _ if state.sandbox_full_visibility => None,
        (Some(sim), Some(id)) => Some((id, &sim.fog)),
        _ => None,
    };

    let mut unrevealed: u32 = 0;
    let mut missing_tile: u32 = 0;
    // Checked for every cell regardless of shroud. The fog-gated count above only sees
    // cells the player has explored, so on an unexplored map it can report zero while the
    // rest of the map is full of unresolvable tiles. This one cannot be fooled that way.
    let mut missing_tile_anywhere: u32 = 0;
    let mut missing_samples: Vec<(u16, u16, u16, u8)> = Vec::new();
    let mut unrevealed_samples: Vec<(u16, u16)> = Vec::new();

    for cell in &grid.cells {
        if let Some(atlas) = state.tile_atlas.as_ref() {
            let key = crate::map::theater::TileKey {
                tile_id: cell.tile_id,
                sub_tile: cell.sub_tile,
                variant: 0,
            };
            if atlas.get_uv(key).is_none() {
                missing_tile_anywhere += 1;
            }
        }
        if let Some((id, fog_state)) = fog {
            if !fog_state.is_cell_revealed(id, cell.rx, cell.ry) {
                unrevealed += 1;
                if unrevealed_samples.len() < 12 {
                    unrevealed_samples.push((cell.rx, cell.ry));
                }
                // An unrevealed cell is never drawn, so its tile is irrelevant.
                continue;
            }
        }
        if let Some(atlas) = state.tile_atlas.as_ref() {
            let key = crate::map::theater::TileKey {
                tile_id: cell.tile_id,
                sub_tile: cell.sub_tile,
                variant: 0,
            };
            if atlas.get_uv(key).is_none() {
                missing_tile += 1;
                if missing_samples.len() < 12 {
                    missing_samples.push((cell.rx, cell.ry, cell.tile_id, cell.sub_tile));
                }
            }
        }
    }

    log::info!(
        "Black-cell report: {} cells total | fog {} | unrevealed={} | revealed-but-no-tile={}          | no-tile-anywhere={}",
        grid.cells.len(),
        if fog.is_some() { "ON" } else { "OFF" },
        unrevealed,
        missing_tile,
        missing_tile_anywhere,
    );
    if !unrevealed_samples.is_empty() {
        log::info!("  unrevealed sample (rx,ry): {unrevealed_samples:?}");
    }
    if !missing_samples.is_empty() {
        log::info!("  no-tile sample (rx,ry,tile_id,sub_tile): {missing_samples:?}");
    }
    if unrevealed == 0 && missing_tile == 0 {
        log::info!("  no cell is black for either reason — the black must come from elsewhere");
    }
}

pub(crate) fn toggle_unit_inspector(state: &mut AppState) {
    state.debug_unit_inspector = !state.debug_unit_inspector;
    if let Some(sim) = &mut state.simulation {
        sim.debug_event_logging = state.debug_unit_inspector;
        if state.debug_unit_inspector {
            for entity in sim.entities_mut().values_mut() {
                if entity.debug_log.is_none() {
                    entity.debug_log = Some(crate::sim::debug_event_log::DebugEventLog::new());
                }
            }
            log::info!("Debug unit inspector: ON");
        } else {
            for entity in sim.entities_mut().values_mut() {
                entity.debug_log = None;
            }
            log::info!("Debug unit inspector: OFF");
        }
    }
}

/// Toggle the PathGrid / terrain-cost debug overlay.
///
/// Beyond flipping `state.debug_show_pathgrid`, this resets the per-overlay
/// SpeedType override to None when the overlay turns off, so reopening
/// the overlay defaults back to "auto from selected unit". Called by both
/// the F9/P hotkey and the dev overlay checkbox.
pub(crate) fn toggle_pathgrid_overlay(state: &mut AppState) {
    state.debug_show_pathgrid = !state.debug_show_pathgrid;
    if !state.debug_show_pathgrid {
        state.debug_terrain_cost_speed_type = None;
    }
    log::info!(
        "Debug terrain cost overlay: {}",
        if state.debug_show_pathgrid {
            "ON"
        } else {
            "OFF"
        }
    );
}

/// Toggle debug pause (J hotkey / dev overlay).
///
/// On resume, local frame admission is re-anchored so elapsed modal time
/// cannot cause a catch-up frame.
pub(crate) fn toggle_debug_pause(state: &mut AppState) {
    state.paused = !state.paused;
    if !state.paused {
        state.frame_pacer.reset_for_immediate_frame();
    }
    log::info!("Debug pause: {}", if state.paused { "ON" } else { "OFF" });
}

/// Handle one-shot gameplay hotkeys (called on key press, not held).
///
/// Dev/debug functions all live behind the Ctrl+Shift chord (same base keys)
/// so bare keys stay free for stock game hotkeys. The chord never collides
/// with stock modifiers: stock uses bare keys, Ctrl+digit (team assign), and
/// Ctrl/Alt/Ctrl+Shift as CLICK modifiers, not key chords.
pub(crate) fn handle_hotkey_pressed(state: &mut AppState, code: winit::keyboard::KeyCode) {
    if let Some(group_idx) = control_group_index(code) {
        handle_control_group_hotkey(state, group_idx);
        return;
    }
    if code == KeyCode::KeyS && is_shift_held(state) && !is_ctrl_held(state) && !is_alt_held(state)
    {
        state.retail_screenshot_requested = true;
        state.window.request_redraw();
        return;
    }
    if is_ctrl_held(state) && is_shift_held(state) {
        handle_dev_hotkey_pressed(state, code);
        return;
    }
    match code {
        KeyCode::Escape => {
            if state.paused {
                // Unpause — reset timing to prevent sim accumulator spike.
                state.paused = false;
                state.frame_pacer.reset_for_immediate_frame();
                // Re-hide OS cursor so the software cursor takes over.
                if state.software_cursor.is_some() {
                    state.window.set_cursor_visible(false);
                }
                log::info!("Game resumed");
            } else if state.targeting_mode.is_some() {
                state.targeting_mode = None;
                state.building_placement_preview = None;
            } else if state.sidebar_gadget_state.repair_mode_on
                || state.sidebar_gadget_state.sell_mode_on
            {
                state.sidebar_gadget_state.repair_mode_on = false;
                state.sidebar_gadget_state.sell_mode_on = false;
            } else {
                state.paused = true;
                // Opening the in-game Options overlay: reset the transient
                // interaction flags so the drag-gated value-label quirk (stale
                // "Faster" until the slider is first dragged) resets each open.
                state.in_game_options.on_open();
                // Show OS cursor for egui interaction.
                if state.software_cursor.is_some() {
                    state.window.set_cursor_visible(true);
                }
                log::info!("Game paused");
            }
        }
        KeyCode::KeyS => queue_stop_for_selected(state),
        KeyCode::KeyD => queue_deploy_undeploy_for_selected(state),
        KeyCode::KeyG => {
            state.queued_order_mode = OrderMode::Guard;
            log::info!("Order mode armed: Guard");
        }
        KeyCode::KeyQ => {
            apply_sidebar_action(state, SidebarAction::SelectTab(SidebarTab::Building))
        }
        KeyCode::KeyW => apply_sidebar_action(state, SidebarAction::SelectTab(SidebarTab::Defense)),
        KeyCode::KeyE => {
            apply_sidebar_action(state, SidebarAction::SelectTab(SidebarTab::Infantry))
        }
        KeyCode::KeyR => apply_sidebar_action(state, SidebarAction::SelectTab(SidebarTab::Vehicle)),
        KeyCode::KeyT => select_same_type(state, is_shift_held(state)),
        // Camera bookmarks. Stock KEYBOARDMD.INI binds View1..View4 to F1..F4
        // and SetView1..SetView4 to Ctrl+F1..Ctrl+F4 (the same 0x200 Ctrl bit
        // the team-create bindings use). Ctrl+Shift never reaches here — the
        // dev chord took it above — so plain Ctrl is unambiguous.
        KeyCode::F1 | KeyCode::F2 | KeyCode::F3 | KeyCode::F4 => {
            let slot = match code {
                KeyCode::F1 => 0,
                KeyCode::F2 => 1,
                KeyCode::F3 => 2,
                _ => 3,
            };
            if is_ctrl_held(state) {
                crate::app_camera::set_view_bookmark(state, slot);
            } else {
                crate::app_camera::recall_view_bookmark(state, slot);
            }
        }
        KeyCode::KeyH => {
            jump_camera_to_base(state);
        }
        KeyCode::Space => {
            // Spacebar cycles through recent radar events and jumps the camera.
            let event = state
                .simulation
                .as_mut()
                .and_then(|sim| sim.radar_events.cycle_event());
            if let Some((rx, ry)) = event {
                // Centres on the tactical viewport, not the window — the sidebar
                // column is not part of the game view.
                crate::app_camera::center_camera_on_cell(state, rx, ry);
            }
        }
        _ => {}
    }
}

/// Dev/debug hotkeys — all require Ctrl+Shift so the bare keys stay free for
/// stock game hotkeys (bare X/P/L/K/M/N/B/A/Delete/F5-F12 previously
/// shadowed stock functions like Scatter and Beacon).
fn handle_dev_hotkey_pressed(state: &mut AppState, code: winit::keyboard::KeyCode) {
    match code {
        // The hotkey-help overlay is VERA-only and used to sit on bare F1, which
        // stock YR binds to the first camera bookmark. Moved onto the dev chord,
        // which stock binds nothing to.
        KeyCode::F1 => {
            state.show_hotkey_help = !state.show_hotkey_help;
        }
        KeyCode::KeyM => {
            quicksave(state);
        }
        KeyCode::KeyN => {
            quickload(state);
        }
        KeyCode::F5 => {
            state.show_save_load_panel = !state.show_save_load_panel;
            if state.show_save_load_panel {
                state.save_list_cache.invalidate();
                // Show OS cursor for egui interaction.
                if state.software_cursor.is_some() {
                    state.window.set_cursor_visible(true);
                }
            } else if state.software_cursor.is_some() && !state.paused {
                // Re-hide OS cursor so the software cursor takes over.
                state.window.set_cursor_visible(false);
            }
        }
        // Interim order-mode arms until the stock click modifiers
        // (Ctrl+Shift+click attack move, beacon key) are implemented.
        KeyCode::KeyA => {
            state.queued_order_mode = OrderMode::AttackMove;
            log::info!("Order mode armed: AttackMove");
        }
        KeyCode::KeyB => {
            state.queued_order_mode = OrderMode::Move;
        }
        KeyCode::KeyL => {
            state.debug_show_cell_grid = !state.debug_show_cell_grid;
            log::info!(
                "Debug cell grid overlay: {}",
                if state.debug_show_cell_grid {
                    "ON (blue=terrain, yellow=overlay)"
                } else {
                    "OFF"
                }
            );
        }
        KeyCode::KeyK => {
            state.debug_show_heightmap = !state.debug_show_heightmap;
            log::info!(
                "Debug height map overlay: {}",
                if state.debug_show_heightmap {
                    "ON (brighter = higher elevation, blue = bridge deck)"
                } else {
                    "OFF"
                }
            );
        }
        KeyCode::F9 | KeyCode::KeyP => {
            toggle_pathgrid_overlay(state);
        }
        KeyCode::BracketRight => {
            if state.debug_show_pathgrid {
                let current = crate::app_debug_overlays::resolve_debug_speed_type(state);
                let next = current.cycle_next();
                state.debug_terrain_cost_speed_type = Some(next);
                log::info!("Terrain cost overlay: {}", next.name());
            }
        }
        KeyCode::BracketLeft => {
            if state.debug_show_pathgrid {
                let current = crate::app_debug_overlays::resolve_debug_speed_type(state);
                let prev = current.cycle_prev();
                state.debug_terrain_cost_speed_type = Some(prev);
                log::info!("Terrain cost overlay: {}", prev.name());
            }
        }
        KeyCode::F10 | KeyCode::KeyV => {
            state.sandbox_full_visibility = !state.sandbox_full_visibility;
            log::info!(
                "Fog of war: {}",
                if state.sandbox_full_visibility {
                    "OFF (full visibility)"
                } else {
                    "ON"
                }
            );
        }
        KeyCode::F8 => {
            report_black_cell_causes(state);
        }
        KeyCode::KeyX => {
            toggle_unit_inspector(state);
        }
        KeyCode::KeyJ => {
            toggle_debug_pause(state);
        }
        KeyCode::Period => {
            if state.paused {
                state.debug_frame_step_requested = true;
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Quick-save / quick-load
// ---------------------------------------------------------------------------

const SAVES_DIR: &str = "saves";

fn quicksave(state: &mut AppState) {
    let Some(sim) = &state.simulation else {
        log::warn!("Quicksave: no active simulation");
        return;
    };
    let Some(map_hash) = state.loaded_map_hash else {
        log::warn!("Quicksave: active world has no authoritative source-map digest");
        return;
    };
    let Some(rules) = state.rules.as_ref() else {
        log::warn!("Quicksave: active rules are unavailable");
        return;
    };
    let rules_h = crate::app_sim_tick::rules_hash(rules);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let bytes = crate::sim::snapshot::GameSnapshot::save_validated(sim, map_hash, rules_h, now);
    if let Err(e) = std::fs::create_dir_all(SAVES_DIR) {
        log::error!("Quicksave: failed to create saves dir: {e}");
        return;
    }
    let filename = format!("save_tick{}_{}.bin", sim.session.tick, now);
    let path = format!("{SAVES_DIR}/{filename}");
    match std::fs::write(&path, &bytes) {
        Ok(()) => {
            log::info!("Quicksave: saved {} bytes to {}", bytes.len(), path);
            state.last_save_tick = Some(sim.session.tick);
            state.last_save_instant = Some(std::time::Instant::now());
            state.save_list_cache.invalidate();
        }
        Err(e) => log::error!("Quicksave: write failed: {e}"),
    }
}

/// Save the current sim with a user-supplied name (dev overlay "Save As").
///
/// Sanitizes the name (strips path-unsafe chars, trims, length-caps),
/// then writes to `saves/save_{sanitized}_tick{tick}_{unix_secs}.bin` so
/// the existing list-panel parser still works and collisions are
/// impossible. Updates the last-save readout fields. No-ops with a log
/// warning on empty input.
pub(crate) fn save_with_name(state: &mut AppState, raw_name: &str) {
    let sanitized: String = sanitize_save_name(raw_name);
    if sanitized.is_empty() {
        log::warn!("Save As: empty or whitespace-only name, ignored");
        return;
    }
    let Some(sim) = &state.simulation else {
        log::warn!("Save As: no active simulation");
        return;
    };
    let Some(map_hash) = state.loaded_map_hash else {
        log::warn!("Save As: active world has no authoritative source-map digest");
        return;
    };
    let Some(rules) = state.rules.as_ref() else {
        log::warn!("Save As: active rules are unavailable");
        return;
    };
    let rules_h = crate::app_sim_tick::rules_hash(rules);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let bytes = crate::sim::snapshot::GameSnapshot::save_validated(sim, map_hash, rules_h, now);
    if let Err(e) = std::fs::create_dir_all(SAVES_DIR) {
        log::error!("Save As: failed to create saves dir: {e}");
        return;
    }
    let filename = format!("save_{sanitized}_tick{}_{}.bin", sim.session.tick, now);
    let path = format!("{SAVES_DIR}/{filename}");
    match std::fs::write(&path, &bytes) {
        Ok(()) => {
            log::info!("Save As: saved {} bytes to {}", bytes.len(), path);
            state.last_save_tick = Some(sim.session.tick);
            state.last_save_instant = Some(std::time::Instant::now());
            state.save_list_cache.invalidate();
        }
        Err(e) => log::error!("Save As: write failed: {e}"),
    }
}

/// Sanitize a user-typed save name for use in a filename.
///
/// Replaces Windows-reserved characters (`/ \ : * ? " < > |`) with `_`,
/// trims surrounding whitespace, then caps at 64 chars. Returns an empty
/// string for empty/whitespace-only input.
fn sanitize_save_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => out.push('_'),
            c if c.is_control() => out.push('_'),
            c => out.push(c),
        }
    }
    out.truncate(64);
    out
}

#[cfg(test)]
mod wheel_tests {
    use super::{WheelAction, wheel_action, wheel_scrolled_row};

    /// One notch is one row, whichever way it turns and however large the OS
    /// reports the delta. gamemd tests only the sign of the wheel delta and then
    /// executes a command that moves the strip by exactly one.
    #[test]
    fn magnitude_never_scales_the_step() {
        for up in [0.5_f32, 1.0, 3.0, 120.0] {
            assert_eq!(wheel_action(up), WheelAction::SidebarUp, "delta {up}");
        }
        for down in [-0.5_f32, -1.0, -3.0, -120.0] {
            assert_eq!(wheel_action(down), WheelAction::SidebarDown, "delta {down}");
        }
    }

    /// The native test is a signed less-than against zero, so a zero delta goes
    /// up rather than doing nothing.
    #[test]
    fn zero_delta_scrolls_up() {
        assert_eq!(wheel_action(0.0), WheelAction::SidebarUp);
    }

    #[test]
    fn rows_move_one_at_a_time_and_saturate_at_both_ends() {
        assert_eq!(wheel_scrolled_row(0, 4, WheelAction::SidebarDown), 1);
        assert_eq!(wheel_scrolled_row(3, 4, WheelAction::SidebarDown), 4);
        // Refuses to move past the strip's computed capacity.
        assert_eq!(wheel_scrolled_row(4, 4, WheelAction::SidebarDown), 4);
        assert_eq!(wheel_scrolled_row(2, 4, WheelAction::SidebarUp), 1);
        // Refuses to move above row 0.
        assert_eq!(wheel_scrolled_row(0, 4, WheelAction::SidebarUp), 0);
        // A strip that fits entirely on screen cannot scroll at all.
        assert_eq!(wheel_scrolled_row(0, 0, WheelAction::SidebarDown), 0);
    }
}

#[cfg(test)]
mod save_name_tests {
    use super::sanitize_save_name;

    #[test]
    fn empty_returns_empty() {
        assert_eq!(sanitize_save_name(""), "");
        assert_eq!(sanitize_save_name("   "), "");
        assert_eq!(sanitize_save_name("\t\n"), "");
    }

    #[test]
    fn strips_path_separators() {
        assert_eq!(sanitize_save_name("../foo"), ".._foo");
        assert_eq!(sanitize_save_name("a/b\\c"), "a_b_c");
    }

    #[test]
    fn strips_windows_reserved_chars() {
        assert_eq!(sanitize_save_name("a:b*c?d\"e<f>g|h"), "a_b_c_d_e_f_g_h");
    }

    #[test]
    fn keeps_normal_chars() {
        assert_eq!(sanitize_save_name("miner stuck repro"), "miner stuck repro");
        assert_eq!(sanitize_save_name("dock_fix_a"), "dock_fix_a");
    }

    #[test]
    fn caps_at_64_chars() {
        let long: String = "x".repeat(100);
        let out = sanitize_save_name(&long);
        assert_eq!(out.len(), 64);
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(sanitize_save_name("  hello  "), "hello");
    }
}

/// Find the most recent `.bin` save file in the saves directory.
fn most_recent_save_path() -> Option<std::path::PathBuf> {
    let dir = std::fs::read_dir(SAVES_DIR).ok()?;
    dir.filter_map(|entry| {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("bin") {
            let meta = entry.metadata().ok()?;
            Some((path, meta.modified().ok()?))
        } else {
            None
        }
    })
    .max_by_key(|(_, modified)| *modified)
    .map(|(path, _)| path)
}

fn quickload(state: &mut AppState) {
    let path = match most_recent_save_path() {
        Some(p) => p,
        None => {
            log::warn!("Quickload: no save files found in {SAVES_DIR}/");
            return;
        }
    };
    load_save_file(state, &path);
}

/// Load a save file by path. Used by both quickload and the save/load panel.
pub(crate) fn load_save_file(state: &mut AppState, path: &std::path::Path) {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            log::warn!("Load: could not read {}: {e}", path.display());
            return;
        }
    };
    let Some(current_sim) = &state.simulation else {
        log::warn!("Load: no active simulation to restore");
        return;
    };
    let Some(map_hash) = state.loaded_map_hash else {
        log::warn!("Load: active world has no authoritative source-map digest");
        return;
    };
    let Some(rules) = state.rules.as_ref() else {
        log::warn!("Load: active rules are unavailable");
        return;
    };
    let rules_hash = crate::app_sim_tick::rules_hash(rules);
    let expected_map_name = current_sim.session.map_name.clone();
    let snapshot = match crate::sim::snapshot::GameSnapshot::load_validated(
        &bytes,
        map_hash,
        rules_hash,
        &expected_map_name,
    ) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Load: {e}");
            return;
        }
    };

    // Grab cache data from the current sim (these fields are #[serde(skip)]
    // and must be restored after deserialization).
    let terrain_speed_config = current_sim.terrain_speed_config.clone();
    let bridge_explosions = current_sim.bridge_explosions.clone();
    let metallic_debris = current_sim.metallic_debris.clone();
    let bridge_anim_sounds = current_sim.bridge_anim_sounds.clone();
    let effect_frame_counts = current_sim.effect_frame_counts.clone();
    let terrain_costs = current_sim.terrain_costs.clone();

    let resolved_terrain = match state.resolved_terrain.clone() {
        Some(rt) => rt,
        None => {
            log::error!("Load: no resolved_terrain available");
            return;
        }
    };

    // Resolve every saved stable-ID slot before rebuilding derived runtime
    // indexes. A malformed object graph never replaces the active simulation.
    let mut sim = snapshot.sim;
    // Native Main/MapGen RNG objects are process globals, not ScenarioClass
    // save fields. Loading replaces Scenario state but retains these cursors.
    sim.retain_process_rngs_from(current_sim);
    if let Err(error) = sim.restore_after_snapshot_load() {
        log::error!("Load: restoration validation failed: {error}");
        return;
    }
    sim.rebuild_caches_after_load(
        resolved_terrain,
        terrain_speed_config,
        bridge_explosions,
        metallic_debris,
        bridge_anim_sounds,
        effect_frame_counts,
        terrain_costs,
    );
    sim.resolve_type_handles(rules);
    if let Err(error) = sim.restore_move_sound_handles_after_load(rules) {
        log::error!("Load: restoration validation failed: {error}");
        return;
    }
    state.simulation = Some(sim);

    // Rebuild the app-layer dynamic path grid (building footprints + walls).
    crate::app_sim_tick::rebuild_dynamic_path_grid(state);

    // Rebuild sprite/unit atlases so all entity types in the loaded save have
    // atlas entries before the first render frame.
    crate::app_sim_tick::refresh_entity_atlases(state);

    // Rebuild transient lighting from the loaded live entity set so destroyed
    // light-source buildings do not leave stale point lights behind.
    if let Some(resolved_terrain) = state.resolved_terrain.as_ref() {
        state.lighting_grid = crate::app_init::rebuild_lighting_grid_from_sim(
            resolved_terrain,
            &state.map_lighting_config,
            state.simulation.as_ref(),
            state.rules.as_ref(),
        );
    }

    // Reset timing to prevent a burst of ticks after the load.
    state.frame_pacer.reset_for_immediate_frame();

    // Close the save/load panel after loading.
    state.show_save_load_panel = false;

    state.last_loaded_save_path = Some(path.to_path_buf());
    log::info!("Load: restored simulation from {}", path.display());
}

pub(crate) fn is_shift_held(state: &AppState) -> bool {
    state.keys_held.contains(&KeyCode::ShiftLeft) || state.keys_held.contains(&KeyCode::ShiftRight)
}

pub(crate) fn is_ctrl_held(state: &AppState) -> bool {
    state.keys_held.contains(&KeyCode::ControlLeft)
        || state.keys_held.contains(&KeyCode::ControlRight)
}

/// Return `true` if either Alt key is currently held.
///
/// Used in order resolution to detect Alt+Ctrl = attack-move (NOT force-fire),
/// matching gamemd's `What_Action_OnCell` Alt-overrides-Ctrl rule.
pub(crate) fn is_alt_held(state: &AppState) -> bool {
    state.keys_held.contains(&KeyCode::AltLeft) || state.keys_held.contains(&KeyCode::AltRight)
}

pub(crate) fn selected_stable_ids_sorted(
    entities: &crate::sim::entity_store::EntityStore,
) -> Vec<u64> {
    let mut ids: Vec<u64> = entities
        .values()
        .filter(|e| e.selected)
        .map(|e| e.stable_id)
        .collect();
    ids.sort_unstable();
    ids
}

fn queue_selection_snapshot_command(state: &mut AppState, selected_ids: Vec<u64>, additive: bool) {
    let owner: String = preferred_local_owner(state).unwrap_or_else(|| "Americans".to_string());
    schedule_command(
        state,
        &owner,
        Command::Select {
            entity_ids: selected_ids,
            additive,
        },
    );
}

fn queue_stop_for_selected(state: &mut AppState) {
    let Some(sim) = &state.simulation else { return };
    let mut selected_ids: Vec<u64> = selected_stable_ids_sorted(sim.entities());
    if selected_ids.is_empty() {
        return;
    }
    selected_ids.sort_unstable();
    let owner: String = preferred_local_owner(state).unwrap_or_else(|| "Americans".to_string());
    for entity_id in selected_ids {
        schedule_command(state, &owner, Command::Stop { entity_id });
    }
}

/// Deploy or undeploy selected entities. KeyD toggles:
/// - Selected unit with `DeploysInto` → `Command::DeployMcv` (MCV → ConYard)
/// - Selected structure with `UndeploysInto` → `Command::UndeployBuilding` (ConYard → MCV)
fn queue_deploy_undeploy_for_selected(state: &mut AppState) {
    let Some(sim) = &state.simulation else { return };
    let selected_ids: Vec<u64> = selected_stable_ids_sorted(sim.entities());
    if selected_ids.is_empty() {
        return;
    }
    let owner: String = preferred_local_owner(state).unwrap_or_else(|| "Americans".to_string());
    // Collect commands first to avoid borrow conflict with schedule_command.
    let mut commands: Vec<Command> = Vec::new();
    {
        let rules = state.rules.as_ref();
        for &entity_id in &selected_ids {
            let Some(entity) = sim.entities().get(entity_id) else {
                continue;
            };
            let obj = rules.and_then(|r| r.object(sim.interner.resolve(entity.type_ref)));
            match entity.category {
                crate::map::entities::EntityCategory::Structure => {
                    // Garrisoned building → evacuate occupants.
                    if obj.map_or(false, |o| o.can_be_occupied)
                        && entity.passenger_role.cargo().is_some_and(|c| !c.is_empty())
                    {
                        commands.push(Command::UnloadPassengers {
                            transport_id: entity_id,
                        });
                    } else if rules.is_some_and(|rules| {
                        sim.should_show_undeploy_building_command(entity_id, rules)
                    }) {
                        commands.push(Command::UndeployBuilding { entity_id });
                    }
                }
                crate::map::entities::EntityCategory::Infantry => {
                    // Deploy-fire infantry (GI, GuardianGI, etc.) → toggle deploy.
                    if obj.map_or(false, |o| o.deploy_fire) {
                        commands.push(Command::ToggleInfantryDeploy { entity_id });
                    }
                }
                _ => {
                    if obj.map_or(false, |o| o.deploys_into.is_some()) {
                        commands.push(Command::DeployMcv { entity_id });
                    }
                }
            }
        }
    }
    for cmd in commands {
        schedule_command(state, &owner, cmd);
    }
}

fn control_group_index(code: KeyCode) -> Option<usize> {
    match code {
        KeyCode::Digit0 => Some(0),
        KeyCode::Digit1 => Some(1),
        KeyCode::Digit2 => Some(2),
        KeyCode::Digit3 => Some(3),
        KeyCode::Digit4 => Some(4),
        KeyCode::Digit5 => Some(5),
        KeyCode::Digit6 => Some(6),
        KeyCode::Digit7 => Some(7),
        KeyCode::Digit8 => Some(8),
        KeyCode::Digit9 => Some(9),
        _ => None,
    }
}

fn handle_control_group_hotkey(state: &mut AppState, group_idx: usize) {
    if group_idx >= state.control_groups.len() {
        return;
    }
    if is_ctrl_held(state) {
        let ids = state
            .simulation
            .as_ref()
            .map(|sim| selected_stable_ids_sorted(sim.entities()))
            .unwrap_or_default();
        state.control_groups[group_idx] = ids;
        return;
    }

    let group = state.control_groups[group_idx].clone();
    if group.is_empty() {
        return;
    }
    let additive = is_shift_held(state);
    let mut final_ids = if additive {
        state
            .simulation
            .as_ref()
            .map(|sim| selected_stable_ids_sorted(sim.entities()))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    final_ids.extend(group);
    final_ids.sort_unstable();
    final_ids.dedup();
    let owner: String = preferred_local_owner(state).unwrap_or_else(|| "Americans".to_string());
    schedule_command(
        state,
        &owner,
        Command::Select {
            entity_ids: final_ids,
            additive,
        },
    );
}

fn select_same_type(state: &mut AppState, additive: bool) {
    let Some(sim) = &state.simulation else { return };
    let anchor = sim
        .entities()
        .values()
        .find(|e| e.selected)
        .map(|e| (e.type_ref, e.owner));
    let Some((type_id, owner_id)) = anchor else {
        return;
    };

    let mut matching_ids: Vec<u64> = sim
        .entities()
        .values()
        .filter_map(|e| (e.type_ref == type_id && e.owner == owner_id).then_some(e.stable_id))
        .collect();
    if additive {
        matching_ids.extend(selected_stable_ids_sorted(sim.entities()));
    }
    matching_ids.sort_unstable();
    matching_ids.dedup();
    let owner = sim.interner.resolve(owner_id).to_string();
    let local_owner: String = preferred_local_owner(state).unwrap_or_else(|| owner.clone());
    schedule_command(
        state,
        &local_owner,
        Command::Select {
            entity_ids: matching_ids,
            additive,
        },
    );
}

/// Emit VoiceSelect sound for the first entity in a selection snapshot.
fn emit_selection_voice(state: &mut AppState, snapshot: &[u64]) {
    let Some(first_id) = snapshot.first() else {
        return;
    };
    let Some(sim) = &state.simulation else { return };
    let Some(rules) = &state.rules else { return };

    // Find the entity's type and look up its VoiceSelect sound.
    if let Some(entity) = sim.entities().get(*first_id) {
        if let Some(obj) = rules.object(sim.interner.resolve(entity.type_ref)) {
            if let Some(ref voice_id) = obj.voice_select {
                state.sound_events.push(GameSoundEvent::UnitSelected {
                    sound_id: voice_id.clone(),
                });
            }
        }
    }
}

/// Jump camera to the local player's base.
///
/// Priority: ConYard (structure with `UndeploysInto=`) → MCV (unit with `DeploysInto=`)
/// → multiplayer start waypoint 0 as fallback.
fn jump_camera_to_base(state: &mut AppState) {
    let owner = preferred_local_owner_name(state);
    let owner_name = owner.as_deref();

    // Collect the target cell from simulation entities before mutating state.
    let target: Option<(u16, u16)> = state.simulation.as_ref().and_then(|sim| {
        let rules = state.rules.as_ref();
        // First pass: look for a ConYard (structure with UndeploysInto=).
        let conyard = sim.entities().values().find(|e| {
            e.category == EntityCategory::Structure
                && owner_name.map_or(true, |o| {
                    sim.interner.resolve(e.owner).eq_ignore_ascii_case(o)
                })
                && rules
                    .and_then(|r| r.object(sim.interner.resolve(e.type_ref)))
                    .map_or(false, |o| o.undeploys_into.is_some())
        });
        if let Some(entity) = conyard {
            log::info!(
                "H: jumping to ConYard {} at ({}, {})",
                sim.interner.resolve(entity.type_ref),
                entity.position.rx,
                entity.position.ry
            );
            return Some((entity.position.rx, entity.position.ry));
        }
        // Second pass: look for an MCV (unit with DeploysInto=).
        let mcv = sim.entities().values().find(|e| {
            e.category != EntityCategory::Structure
                && owner_name.map_or(true, |o| {
                    sim.interner.resolve(e.owner).eq_ignore_ascii_case(o)
                })
                && rules
                    .and_then(|r| r.object(sim.interner.resolve(e.type_ref)))
                    .map_or(false, |o| o.deploys_into.is_some())
        });
        if let Some(entity) = mcv {
            log::info!(
                "H: jumping to MCV {} at ({}, {})",
                sim.interner.resolve(entity.type_ref),
                entity.position.rx,
                entity.position.ry
            );
            return Some((entity.position.rx, entity.position.ry));
        }
        log::info!(
            "H: no ConYard/MCV found (owner={:?}, entities={}, rules={})",
            owner_name,
            sim.entities().len(),
            rules.is_some()
        );
        None
    });

    if let Some((rx, ry)) = target {
        crate::app_camera::center_camera_on_cell(state, rx, ry);
        return;
    }

    // Fallback: jump to the first multiplayer start waypoint.
    if let Some(wp) = crate::map::waypoints::first_multiplayer_start(&state.waypoints) {
        log::info!(
            "H: falling back to start waypoint at ({}, {})",
            wp.rx,
            wp.ry
        );
        crate::app_camera::center_camera_on_cell(state, wp.rx, wp.ry);
    } else {
        log::info!("H: no base or start waypoint found");
    }
}
