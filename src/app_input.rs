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
use crate::app_entity_pick::{
    SelectionMutation, compute_box_selection_snapshot, compute_click_selection_snapshot,
    compute_type_select_box_mutation, compute_type_select_click_mutation, compute_type_select_tap,
    map_entity_creation_order, pick_entity_at_point,
};
use crate::app_hotkeys::{HotkeyCommand, HotkeyFallback, HotkeyResolution};
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
/// across the sidebar boundary). A `NotConsumed` click hits no live gadget — only
/// the legacy dev/pause/producer press path runs; empty-sidebar / off-window
/// clicks do nothing (gamemd's sidebar-body gadget swallows them, A6). The middle
/// button has no tactical behavior. Right-press is owned by the tactical catcher
/// (viewport-only), so right-clicking dead sidebar chrome no longer deselects —
/// matching gamemd.
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
    // The stock tactical handler has no middle-button case.
    if button == MouseButton::Middle {
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
    crate::app_sidebar_render::refresh_sidebar_projection(state);
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ClickActionRoute<T> {
    ContextOrder(T),
    Selection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionVoicePolicy {
    FirstAdded,
    EveryAdded,
    Suppressed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionActionLinePolicy {
    Start,
    Preserve,
}

const ORDINARY_SELECTION_VOICE_POLICY: SelectionVoicePolicy = SelectionVoicePolicy::FirstAdded;
const TYPE_SELECT_TAP_VOICE_POLICY: SelectionVoicePolicy = SelectionVoicePolicy::EveryAdded;
const HELD_TYPE_SELECT_VOICE_POLICY: SelectionVoicePolicy = SelectionVoicePolicy::Suppressed;
const ORDINARY_SELECTION_ACTION_LINE_POLICY: SelectionActionLinePolicy =
    SelectionActionLinePolicy::Start;
const TYPE_SELECT_TAP_ACTION_LINE_POLICY: SelectionActionLinePolicy =
    SelectionActionLinePolicy::Preserve;

/// Resolve the ordinary tactical action before TypeSelect is allowed to modify
/// a selection/toggle click. Non-held Shift keeps its established toggle path;
/// while TypeSelect is held, Shift attack/move actions still resolve first.
fn route_click_action_before_type_select<T>(
    action: SelectAction,
    shift: bool,
    type_select_held: bool,
    resolve_context_order: impl FnOnce(f32, f32) -> Option<T>,
) -> ClickActionRoute<T> {
    let SelectAction::Click(sx, sy) = action else {
        return ClickActionRoute::Selection;
    };
    if shift && !type_select_held {
        return ClickActionRoute::Selection;
    }
    resolve_context_order(sx, sy)
        .map(ClickActionRoute::ContextOrder)
        .unwrap_or(ClickActionRoute::Selection)
}

/// Tactical-viewport mouse body (routed here when the full-tactical ClickRegion
/// consumes the edge — i.e. a click in the play area, or a captured drag/release
/// that started there). Logic is the legacy handler's tactical path, unchanged;
/// the minimap-drag-end and minimap-begin checks moved to `minimap_mouse`. The
/// stock tactical path has no middle-button case, so the Rust dispatcher ignores it.
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
                // Only an active band box owns a clamped tactical endpoint.
                // A pending press is still an ordinary click at the actual
                // release point, including after sticky capture routing.
                let release_point = if state.selection_state.is_band_box_active() {
                    let (tactical_width, tactical_height) =
                        crate::app_camera::tactical_viewport_size_px(
                            state.render_width(),
                            state.render_height(),
                        );
                    clamp_tactical_drag_endpoint(
                        state.cursor_x,
                        state.cursor_y,
                        tactical_width,
                        tactical_height,
                    )
                } else {
                    (state.cursor_x, state.cursor_y)
                };
                let mut action: SelectAction = state
                    .selection_state
                    .end_drag(release_point.0, release_point.1);
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
                let mut band_preflight_order = None;
                if let SelectAction::BoxSelect(min_x, min_y, max_x, max_y) = action
                    && !shift
                {
                    let order =
                        crate::app_instances::tactical_band_preflight_entity_encounter_order(state);
                    if band_caught_drawn_object(state, &order, min_x, min_y, max_x, max_y) {
                        band_preflight_order = Some(order);
                    } else {
                        action = SelectAction::Click(release_point.0, release_point.1);
                    }
                }
                // TypeSelect modifies only actions that already resolved as
                // selection/toggle. Ground move, attack, and every other
                // context action keep their ordinary priority while T is held.
                let type_select_held = state.type_select.held();
                if matches!(
                    route_click_action_before_type_select(
                        action,
                        shift,
                        type_select_held,
                        |sx, sy| {
                            try_queue_context_order_at_screen_point(state, sx, sy, true)
                                .then_some(())
                        },
                    ),
                    ClickActionRoute::ContextOrder(())
                ) {
                    return;
                }
                let mut queued_selection: Option<SelectionMutation> = None;
                let mut held_type_select_batch = false;
                if let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) {
                    let screen_order =
                        crate::app_instances::tactical_screen_entity_encounter_order(state);
                    let current_selection = selected_stable_ids_in_order(state);
                    let map_order = map_entity_creation_order(sim.entities());
                    let held_type_select = type_select_held;
                    let scope_order = if state.type_select.across_map {
                        map_order.as_slice()
                    } else {
                        screen_order.as_slice()
                    };
                    match action {
                        SelectAction::Click(sx, sy) => {
                            let world_x: f32 = sx / state.zoom_level + state.camera_x;
                            let world_y: f32 = sy / state.zoom_level + state.camera_y;
                            let fog_ref = if state.sandbox_full_visibility {
                                None
                            } else {
                                Some(&sim.fog)
                            };
                            if held_type_select {
                                let picked = pick_entity_at_point(
                                    sim.entities(),
                                    &screen_order,
                                    fog_ref,
                                    preferred_local_owner_name(state).as_deref(),
                                    world_x,
                                    world_y,
                                    CLICK_SELECT_RADIUS,
                                    state.rules.as_ref(),
                                    Some(&sim.houses),
                                    &state.height_map,
                                    Some(&state.tactical_bridge_inverse_map),
                                    Some(&sim.interner),
                                );
                                queued_selection = if let Some(clicked_id) = picked {
                                    Some(compute_type_select_click_mutation(
                                        sim.entities(),
                                        scope_order,
                                        &current_selection,
                                        clicked_id,
                                        shift,
                                        preferred_local_owner_name(state).as_deref(),
                                        state.rules.as_ref(),
                                        Some(&sim.interner),
                                    ))
                                } else {
                                    compute_click_selection_snapshot(
                                        sim.entities(),
                                        &screen_order,
                                        &current_selection,
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
                                    )
                                };
                                held_type_select_batch = true;
                            } else {
                                queued_selection = compute_click_selection_snapshot(
                                    sim.entities(),
                                    &screen_order,
                                    &current_selection,
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
                        }
                        SelectAction::BoxSelect(min_x, min_y, max_x, max_y) => {
                            let fog_ref = if state.sandbox_full_visibility {
                                None
                            } else {
                                Some(&sim.fog)
                            };
                            let z = state.zoom_level;
                            let (min_x, min_y, max_x, max_y) = (
                                min_x / z + state.camera_x,
                                min_y / z + state.camera_y,
                                max_x / z + state.camera_x,
                                max_y / z + state.camera_y,
                            );
                            if held_type_select {
                                queued_selection = Some(compute_type_select_box_mutation(
                                    sim.entities(),
                                    &screen_order,
                                    scope_order,
                                    &current_selection,
                                    fog_ref,
                                    preferred_local_owner_name(state).as_deref(),
                                    min_x,
                                    min_y,
                                    max_x,
                                    max_y,
                                    shift,
                                    state.rules.as_ref(),
                                    Some(&sim.interner),
                                ));
                                held_type_select_batch = true;
                            } else {
                                let preflight_order = band_preflight_order
                                    .as_deref()
                                    .unwrap_or(screen_order.as_slice());
                                queued_selection = compute_box_selection_snapshot(
                                    sim.entities(),
                                    preflight_order,
                                    &screen_order,
                                    &current_selection,
                                    fog_ref,
                                    preferred_local_owner_name(state).as_deref(),
                                    min_x,
                                    min_y,
                                    max_x,
                                    max_y,
                                    shift,
                                    state.rules.as_ref(),
                                    Some(&sim.houses),
                                    Some(&sim.interner),
                                );
                            }
                        }
                        SelectAction::None => {}
                    }
                }
                if let Some(mutation) = queued_selection {
                    if held_type_select_batch {
                        if mutation.select.is_empty() {
                            apply_selection_mutation(
                                state,
                                mutation,
                                false,
                                ORDINARY_SELECTION_VOICE_POLICY,
                            );
                        } else {
                            let prior = state.selection_voice_enabled;
                            state.selection_voice_enabled = false;
                            apply_selection_mutation(
                                state,
                                mutation,
                                false,
                                HELD_TYPE_SELECT_VOICE_POLICY,
                            );
                            state.selection_voice_enabled = prior;
                        }
                    } else {
                        apply_selection_mutation(
                            state,
                            mutation,
                            true,
                            ORDINARY_SELECTION_VOICE_POLICY,
                        );
                    }
                    // Both selection arms of the band-box release open the
                    // action-line window, so the units just picked up flash
                    // whatever they are already doing.
                    apply_selection_action_line_policy(
                        state,
                        ORDINARY_SELECTION_ACTION_LINE_POLICY,
                    );
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
    encounter_order: &[u64],
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
) -> bool {
    let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
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
        encounter_order,
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
fn clamp_tactical_drag_endpoint(
    cursor_x: f32,
    cursor_y: f32,
    tactical_width: u32,
    tactical_height: u32,
) -> (f32, f32) {
    (
        cursor_x.clamp(0.0, tactical_width.saturating_sub(1) as f32),
        cursor_y.clamp(0.0, tactical_height.saturating_sub(1) as f32),
    )
}

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
    // Clamp drag position to the tactical viewport (exclude sidebar area).
    let (tactical_width, tactical_height) =
        crate::app_camera::tactical_viewport_size_px(state.render_width(), state.render_height());
    let clamped_endpoint = clamp_tactical_drag_endpoint(
        state.cursor_x,
        state.cursor_y,
        tactical_width,
        tactical_height,
    );

    // Activation arms the rectangle and nothing else. The call gamemd makes at
    // that moment is a cursor-shape setter, not an unselect — the selection is
    // only replaced on the release, and only when the box caught something.
    // The threshold is measured from the live mouse point. Once active, the
    // rendered/stored endpoint is restricted to the tactical surface.
    state
        .selection_state
        .update_drag(state.cursor_x, state.cursor_y);
    if state.selection_state.is_band_box_active() {
        state.selection_state.drag_current = Some(clamped_endpoint);
    }
}

#[cfg(test)]
mod drag_tests {
    use super::clamp_tactical_drag_endpoint;
    use crate::sim::selection::{SelectAction, SelectionState};

    #[test]
    fn item82_captured_drag_live_and_release_endpoints_stop_at_tactical_rect() {
        let endpoint = clamp_tactical_drag_endpoint(950.0, 650.0, 632, 568);
        assert_eq!(endpoint, (631.0, 567.0));

        let mut selection = SelectionState::new();
        selection.begin_drag(100.0, 100.0);
        selection.update_drag(950.0, 650.0);
        selection.drag_current = Some(endpoint);
        assert_eq!(selection.drag_rect(), Some((100.0, 100.0, 631.0, 567.0)));
        let SelectAction::BoxSelect(min_x, min_y, max_x, max_y) =
            selection.end_drag(endpoint.0, endpoint.1)
        else {
            panic!("active drag must end as a box selection");
        };
        assert_eq!((min_x, min_y, max_x, max_y), (100.0, 100.0, 631.0, 567.0));
    }
}

#[cfg(test)]
mod item83_click_route_tests {
    use super::{ClickActionRoute, route_click_action_before_type_select};
    use crate::app_context_order::object_click_payload;
    use crate::app_entity_pick::compute_type_select_click_mutation;
    use crate::app_types::OrderMode;
    use crate::map::entities::EntityCategory;
    use crate::sim::command::Command;
    use crate::sim::components::Health;
    use crate::sim::entity_store::EntityStore;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::intern::StringInterner;
    use crate::sim::selection::SelectAction;

    #[test]
    fn item83_held_type_select_keeps_attack_and_ground_move_ahead_of_selection() {
        let attack = route_click_action_before_type_select(
            SelectAction::Click(40.0, 40.0),
            true,
            true,
            |_, _| {
                Some(object_click_payload(
                    OrderMode::Move,
                    false,
                    false,
                    1,
                    9,
                    20,
                    20,
                    true,
                ))
            },
        );
        assert_eq!(
            attack,
            ClickActionRoute::ContextOrder(Command::Attack {
                attacker_id: 1,
                target_id: 9,
            })
        );

        let mut selection = vec![1];
        let ground = route_click_action_before_type_select(
            SelectAction::Click(80.0, 70.0),
            false,
            true,
            |_, _| {
                Some(Command::Move {
                    entity_id: 1,
                    target_rx: 14,
                    target_ry: 15,
                    queue: false,
                    group_id: None,
                })
            },
        );
        match ground {
            ClickActionRoute::ContextOrder(Command::Move { .. }) => {}
            ClickActionRoute::Selection => selection.clear(),
            other => panic!("unexpected held-ground route: {other:?}"),
        }
        assert_eq!(
            selection,
            [1],
            "an ordered ground click never reaches selection clear"
        );
    }

    #[test]
    fn item83_held_friendly_selection_falls_through_to_exact_type_batch() {
        let route = route_click_action_before_type_select(
            SelectAction::Click(40.0, 40.0),
            false,
            true,
            |_, _| None::<Command>,
        );

        let mut interner = StringInterner::new();
        let owner = interner.intern("Americans");
        let type_ref = interner.intern("AMCV");
        let mut entities = EntityStore::new();
        for (id, rx) in [(1, 10), (2, 12)] {
            let mut entity = GameEntity::new_at_frame_zero_for_test(
                id,
                rx,
                10,
                0,
                0,
                owner,
                Health {
                    current: 100,
                    max: 100,
                },
                type_ref,
                EntityCategory::Unit,
                0,
                5,
                true,
            );
            entity.lifecycle.object_alive = true;
            entity.lifecycle.in_limbo = false;
            entities.insert(entity);
        }

        let mutation = match route {
            ClickActionRoute::Selection => compute_type_select_click_mutation(
                &entities,
                &[1, 2],
                &[1],
                1,
                false,
                Some("Americans"),
                None,
                Some(&interner),
            ),
            ClickActionRoute::ContextOrder(command) => {
                panic!("friendly selection unexpectedly dispatched {command:?}")
            }
        };
        assert!(mutation.clear);
        assert_eq!(mutation.select, [1, 2]);
    }
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
    let Some(view) = current_sidebar_view(state).cloned() else {
        return;
    };
    state.sidebar_scroll_rows = wheel_scrolled_row(
        view.scroll_rows,
        view.max_scroll_rows,
        wheel_action(delta_lines),
    );
    crate::app_sidebar_render::refresh_sidebar_projection(state);
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
        .and_then(|name| state.sim_runtime.as_ref().map(|rt| &rt.simulation)?.interner.get(&name));
    let fog = match (state.sim_runtime.as_ref().map(|rt| &rt.simulation), owner) {
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
    if let Some(sim) = state.sim_runtime.as_mut().map(|rt| &mut rt.simulation) {
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
        state.platform.frame_pacer.reset_for_immediate_frame();
    }
    log::info!("Debug pause: {}", if state.paused { "ON" } else { "OFF" });
}

/// Handle one-shot gameplay hotkeys (called on key press, not held).
///
/// **Modifier matching is exact.** Every stock binding names a precise modifier
/// set, and a bare-key command is rejected outright while Shift, Ctrl or Alt is
/// held — so holding Ctrl to force-fire or Alt to force-move and tapping a
/// letter does nothing instead of firing Stop or Deploy.
///
/// Dev/debug functions live behind the Ctrl+Shift chord, which stock binds
/// nothing to, so bare keys stay free for stock game hotkeys.
pub(crate) fn handle_type_select_key_edge(
    state: &mut AppState,
    resolution: HotkeyResolution,
    physical_code: winit::keyboard::KeyCode,
    element_state: ElementState,
    repeat: bool,
) -> bool {
    let is_type_select = resolution == HotkeyResolution::Command(HotkeyCommand::TypeSelect);
    if element_state.is_pressed() {
        if !is_type_select {
            return false;
        }
        state
            .type_select
            .press(physical_code, std::time::Instant::now(), repeat);
        return true;
    }
    if !is_type_select && !state.type_select.owns_key(physical_code) {
        return false;
    }
    let execute_tap = state
        .type_select
        .release(physical_code, std::time::Instant::now());
    if execute_tap {
        execute_type_select_tap(state);
    }
    true
}

fn execute_type_select_tap(state: &mut AppState) {
    state.type_select.prepare_tap_scope();
    let result = {
        let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
            return;
        };
        let screen_order = crate::app_instances::tactical_screen_entity_encounter_order(state);
        let map_order = map_entity_creation_order(sim.entities());
        let current = selected_stable_ids_in_order(state);
        let fog = (!state.sandbox_full_visibility).then_some(&sim.fog);
        compute_type_select_tap(
            sim.entities(),
            &screen_order,
            &map_order,
            &current,
            fog,
            preferred_local_owner_name(state).as_deref(),
            state.rules.as_ref(),
            Some(&sim.interner),
            state.type_select.across_map,
        )
    };
    let outcome = result.outcome;
    let across_map = result.across_map;
    apply_selection_mutation(state, result.mutation, false, TYPE_SELECT_TAP_VOICE_POLICY);
    state.type_select.finish_tap(outcome, across_map);
    crate::app_messages::post_type_select_feedback(state, outcome.csf_key());
    // Native marks the tactical display dirty here but does not start action
    // lines. The visible-window event loop already requests a redraw from
    // `about_to_wait`, so the tap needs no duplicate redraw request.
    apply_selection_action_line_policy(state, TYPE_SELECT_TAP_ACTION_LINE_POLICY);
}

pub(crate) fn handle_hotkey_pressed(
    state: &mut AppState,
    resolution: HotkeyResolution,
    physical_code: winit::keyboard::KeyCode,
) {
    match resolution {
        HotkeyResolution::Command(command) => dispatch_retail_hotkey(state, command),
        HotkeyResolution::Fallback(HotkeyFallback::DiplomacyDialog) => {
            // RT_DIALOG 0x73 is owned by the later diplomacy/communication
            // milestone. Preserve the semantic event without inventing a modal.
        }
        HotkeyResolution::Fallback(
            HotkeyFallback::ArrowLeft
            | HotkeyFallback::ArrowUp
            | HotkeyFallback::ArrowRight
            | HotkeyFallback::ArrowDown,
        ) => {}
        HotkeyResolution::Unhandled => {
            if KeyModifiers::from_modifiers_state(state.hotkey_modifiers).dev_chord() {
                handle_dev_hotkey_pressed(state, physical_code);
            }
        }
    }
    crate::app_sidebar_render::refresh_sidebar_projection(state);
}

fn dispatch_retail_hotkey(state: &mut AppState, command: HotkeyCommand) {
    match command {
        HotkeyCommand::StopObject => queue_stop_for_selected(state),
        HotkeyCommand::DeployObject => queue_deploy_undeploy_for_selected(state),
        HotkeyCommand::GuardObject => {
            state.queued_order_mode = OrderMode::Guard;
            log::info!("Order mode armed: Guard");
        }
        HotkeyCommand::StructureTab => {
            apply_sidebar_action(state, SidebarAction::SelectTab(SidebarTab::Building))
        }
        HotkeyCommand::DefenseTab => {
            apply_sidebar_action(state, SidebarAction::SelectTab(SidebarTab::Defense))
        }
        HotkeyCommand::InfantryTab => {
            apply_sidebar_action(state, SidebarAction::SelectTab(SidebarTab::Infantry))
        }
        HotkeyCommand::UnitTab => {
            apply_sidebar_action(state, SidebarAction::SelectTab(SidebarTab::Vehicle))
        }
        // TypeSelect owns both edges in `handle_type_select_key_edge`.
        HotkeyCommand::TypeSelect => {}
        HotkeyCommand::ToggleRepair => apply_sidebar_action(state, SidebarAction::ToggleRepairMode),
        HotkeyCommand::ToggleSell => apply_sidebar_action(state, SidebarAction::ToggleSellMode),
        HotkeyCommand::CenterBase => jump_camera_to_base(state),
        HotkeyCommand::CenterOnRadarEvent => {
            let event = state
                .sim_runtime
                .as_mut()
                .map(|rt| &mut rt.simulation)
                .and_then(|sim| sim.radar_events.cycle_event());
            if let Some((rx, ry)) = event {
                crate::app_camera::center_camera_on_cell(state, rx, ry);
            }
        }
        HotkeyCommand::ScreenCapture => {
            state.retail_screenshot_requested = true;
            state.platform.window.request_redraw();
        }
        HotkeyCommand::View(slot) => crate::app_camera::recall_view_bookmark(state, slot),
        HotkeyCommand::SetView(slot) => crate::app_camera::set_view_bookmark(state, slot),
        HotkeyCommand::TeamSelect(slot) => handle_control_group_command(state, slot, None),
        HotkeyCommand::TeamAddSelect(slot) => {
            handle_control_group_command(state, slot, Some(GroupPressAction::AddToSelection))
        }
        HotkeyCommand::TeamCreate(slot) => {
            handle_control_group_command(state, slot, Some(GroupPressAction::Assign))
        }
        HotkeyCommand::TeamCenter(slot) => {
            handle_control_group_command(state, slot, Some(GroupPressAction::Center))
        }
        // Native SidebarUp/Down execute the same one-row saturated sidebar
        // owner as wheel input (up=1, down=0).
        HotkeyCommand::SidebarUp => sidebar_wheel_scroll(state, 1.0),
        HotkeyCommand::SidebarDown => sidebar_wheel_scroll(state, -1.0),
        HotkeyCommand::Options => handle_options_hotkey(state),
        HotkeyCommand::CenterView
        | HotkeyCommand::ToggleAlliance
        | HotkeyCommand::PlaceBeacon
        | HotkeyCommand::AllToCheer
        | HotkeyCommand::Follow
        | HotkeyCommand::PreviousObject
        | HotkeyCommand::NextObject
        | HotkeyCommand::CombatantSelect
        | HotkeyCommand::PageUser
        | HotkeyCommand::ScatterObject
        | HotkeyCommand::VeterancyNav
        | HotkeyCommand::PlanningMode
        | HotkeyCommand::Delete
        | HotkeyCommand::Taunt(_) => {}
    }
}

fn handle_options_hotkey(state: &mut AppState) {
    if state.paused {
        state.paused = false;
        state.platform.frame_pacer.reset_for_immediate_frame();
        if state.software_cursor.is_some() {
            state.platform.window.set_cursor_visible(false);
        }
        log::info!("Game resumed");
    } else if state.targeting_mode.is_some() {
        state.targeting_mode = None;
        state.building_placement_preview = None;
    } else if state.sidebar_gadget_state.repair_mode_on || state.sidebar_gadget_state.sell_mode_on {
        state.sidebar_gadget_state.repair_mode_on = false;
        state.sidebar_gadget_state.sell_mode_on = false;
    } else {
        state.paused = true;
        state.in_game_options.on_open();
        if state.software_cursor.is_some() {
            state.platform.window.set_cursor_visible(true);
        }
        log::info!("Game paused");
    }
}

/// Dev/debug hotkeys require Ctrl+Shift, a chord stock binds to no command.
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
                state.persistence.invalidate_save_list();
                // Show OS cursor for egui interaction.
                if state.software_cursor.is_some() {
                    state.platform.window.set_cursor_visible(true);
                }
            } else if state.software_cursor.is_some() && !state.paused {
                // Re-hide OS cursor so the software cursor takes over.
                state.platform.window.set_cursor_visible(false);
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

fn quicksave(state: &mut AppState) {
    let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
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
    let bytes = crate::sim::snapshot::GameSnapshot::save_validated(
        sim,
        map_hash,
        rules_h,
        &sim.session.map_name,
        now,
    );
    let tick = sim.session.tick;
    let filename = format!("save_tick{tick}_{now}.bin");
    match state.persistence.repository.write_named(&filename, &bytes) {
        Ok(path) => {
            log::info!(
                "Quicksave: saved {} bytes to {}",
                bytes.len(),
                path.display()
            );
            state.persistence.last_save_tick = Some(tick);
            state.persistence.last_save_instant = Some(std::time::Instant::now());
            state.persistence.invalidate_save_list();
        }
        Err(error) => match error.stage() {
            crate::app::persistence::SaveWriteStage::CreateDirectory => {
                log::error!("Quicksave: failed to create saves dir: {error}")
            }
            crate::app::persistence::SaveWriteStage::WriteFile => {
                log::error!("Quicksave: write failed: {error}")
            }
        },
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
    let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
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
    let bytes =
        crate::sim::snapshot::GameSnapshot::save_validated(sim, map_hash, rules_h, raw_name, now);
    let tick = sim.session.tick;
    let filename = format!("save_{sanitized}_tick{tick}_{now}.bin");
    match state.persistence.repository.write_named(&filename, &bytes) {
        Ok(path) => {
            log::info!("Save As: saved {} bytes to {}", bytes.len(), path.display());
            state.persistence.last_save_tick = Some(tick);
            state.persistence.last_save_instant = Some(std::time::Instant::now());
            state.persistence.invalidate_save_list();
        }
        Err(error) => match error.stage() {
            crate::app::persistence::SaveWriteStage::CreateDirectory => {
                log::error!("Save As: failed to create saves dir: {error}")
            }
            crate::app::persistence::SaveWriteStage::WriteFile => {
                log::error!("Save As: write failed: {error}")
            }
        },
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
    for ch in trimmed.chars().take(64) {
        match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => out.push('_'),
            c if c.is_control() => out.push('_'),
            c => out.push(c),
        }
    }
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
    use crate::sim::snapshot::GameSnapshot;
    use crate::sim::world::Simulation;

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

    #[test]
    fn gsi_17_02_unicode_filename_cap_preserves_exact_envelope_description() {
        let raw_description = "保存".repeat(40);
        let filename_part = sanitize_save_name(&raw_description);
        assert_eq!(filename_part.chars().count(), 64);

        let mut sim = Simulation::new();
        sim.session.map_name = "OFFICIAL.MAP".to_string();
        let bytes = GameSnapshot::save_validated(&sim, 1, 2, &raw_description, 3);
        let header = GameSnapshot::read_header(&bytes).expect("current VERA header");
        assert_eq!(header.description, raw_description);
    }
}

fn quickload(state: &mut AppState) {
    let path = match state
        .persistence
        .repository
        .quickload_path_by_modified_time()
    {
        Some(p) => p,
        None => {
            log::warn!(
                "Quickload: no save files found in {}/",
                state.persistence.repository.directory().display()
            );
            return;
        }
    };
    load_save_file(state, &path);
}

/// Load a save file by path. Used by both quickload and the save/load panel.
pub(crate) fn load_save_file(state: &mut AppState, path: &std::path::Path) {
    let preparation = crate::app::persistence::PreparedLoad::from_repository(
        crate::app::persistence::LoadPreparationView::new(
            &state.persistence.repository,
            state.sim_runtime.as_ref().map(|rt| &rt.simulation),
            state.loaded_map_hash,
            state.rules.as_ref(),
            state.resolved_terrain.as_ref(),
            state.overlay_registry.as_ref(),
            crate::app::persistence::MatchStartupStateView::new(
                &state.active_loading_correlation,
                &state.loaded_startup,
                &state.rust_l0_receipt,
            ),
        ),
        path,
    );
    match preparation {
        Ok(prepared) => commit_prepared_load(state, path, prepared),
        Err(error) => log_prepared_load_error(path, &error),
    }
}

fn log_prepared_load_error(
    path: &std::path::Path,
    error: &crate::app::persistence::PreparedLoadError,
) {
    use crate::app::persistence::PreparedLoadError;

    match error {
        PreparedLoadError::ReadFile(source) => {
            log::warn!("Load: could not read {}: {source}", path.display())
        }
        PreparedLoadError::MissingCurrentSimulation
        | PreparedLoadError::MissingMapHash
        | PreparedLoadError::MissingRules => log::warn!("Load: {error}"),
        PreparedLoadError::Snapshot(source) => log::error!("Load: {source}"),
        PreparedLoadError::MissingTerrainTemplate => {
            log::error!("Load: {error}")
        }
        PreparedLoadError::MissingOverlayRegistry => {
            log::error!("Load: restoration validation failed: {error}")
        }
        PreparedLoadError::Restore(source) => {
            log::error!("Load: restoration validation failed: {source}")
        }
    }
}

/// Apply the enumerated post-prepare replacement bundle. This function has no
/// recoverable failure path; best-effort presentation rebuilds retain their
/// prior valid resources when replacement is unavailable.
fn commit_prepared_load(
    state: &mut AppState,
    path: &std::path::Path,
    prepared: crate::app::persistence::PreparedLoad,
) {
    let native_tiberium_stats = prepared.native_tiberium_stats();
    let (simulation, occupied_overlays, preserved_startup) = prepared.into_parts();
    log::info!(
        "Load: rebuilt native tiberium queues ({} growth, {} spread)",
        native_tiberium_stats.growth_entries,
        native_tiberium_stats.spread_entries,
    );

    crate::app::reset_scenario_exit_runtime(state);
    state.sim_runtime = Some(crate::sim::runtime::SimRuntime::from_simulation(simulation));
    crate::app_transitions::sync_in_game_options_speed_from_sim(state);
    state.combat_lights.clear();
    crate::app_sim_tick::upsert_occupied_overlay_render_entries(state, occupied_overlays);

    // Rebuild sprite/unit atlases so all entity types in the loaded save have
    // atlas entries before the first render frame.
    crate::app_sim_tick::refresh_entity_atlases(state);

    // Rebuild transient lighting from the loaded live entity set so destroyed
    // light-source buildings do not leave stale point lights behind.
    if let Some(resolved_terrain) = state.resolved_terrain.as_ref() {
        state.lighting_grid = crate::app_init::rebuild_lighting_grid_from_sim(
            resolved_terrain,
            &state.map_lighting_config,
            state.sim_runtime.as_ref().map(|rt| &rt.simulation),
            state.rules.as_ref(),
            state.in_game_options.detail_level,
        );
        state.pending_lighting_refresh = None;
        state.applied_lighting_sources.clear();
        state.applied_lighting_profile = None;
        state.applied_lighting_detail_level = state.in_game_options.detail_level.min(2);
        state.last_lighting_view_fingerprint = None;
    }

    // Reset timing to prevent a burst of ticks after the load.
    state.platform.frame_pacer.reset_for_immediate_frame();

    // Close the save/load panel after loading.
    state.show_save_load_panel = false;

    // Same-content in-scenario load retains the accepted startup authority
    // that admitted the running match. Cross-session loads require a new
    // explicit receipt and do not use this route.
    preserved_startup.restore(
        &mut state.active_loading_correlation,
        &mut state.loaded_startup,
        &mut state.rust_l0_receipt,
    );
    state.persistence.last_loaded_save_path = Some(path.to_path_buf());
    crate::app_sidebar_render::refresh_sidebar_projection(state);
    log::info!("Load: restored simulation from {}", path.display());
}

/// The modifier bits a hotkey press carries.
///
/// The engine packs Shift, Ctrl and Alt into the key value as separate bits and
/// the command dispatcher matches the whole value, so every binding names an
/// exact modifier set. A command bound to a bare key is *rejected* while any
/// modifier is held — the base "does this command accept a modified form?"
/// predicate returns false for every stock command — and the dispatcher then
/// looks for a binding of the full chord instead. That is why holding Ctrl to
/// force-fire and tapping a letter does nothing in retail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct KeyModifiers {
    pub(crate) shift: bool,
    pub(crate) ctrl: bool,
    pub(crate) alt: bool,
}

impl KeyModifiers {
    fn from_modifiers_state(modifiers: winit::keyboard::ModifiersState) -> Self {
        Self {
            shift: modifiers.shift_key(),
            ctrl: modifiers.control_key(),
            alt: modifiers.alt_key(),
        }
    }

    /// No modifier held — the only state in which a bare-key binding fires.
    pub(crate) fn none(self) -> bool {
        !self.shift && !self.ctrl && !self.alt
    }

    pub(crate) fn any(self) -> bool {
        !self.none()
    }

    pub(crate) fn only_shift(self) -> bool {
        self.shift && !self.ctrl && !self.alt
    }

    pub(crate) fn only_ctrl(self) -> bool {
        self.ctrl && !self.shift && !self.alt
    }

    pub(crate) fn only_alt(self) -> bool {
        self.alt && !self.shift && !self.ctrl
    }

    /// The VERA-internal dev chord. Stock binds nothing to Ctrl+Shift, so it
    /// collides with no retail hotkey.
    pub(crate) fn dev_chord(self) -> bool {
        self.ctrl && self.shift && !self.alt
    }
}

pub(crate) fn is_shift_held(state: &AppState) -> bool {
    state.hotkey_modifiers.shift_key()
}

pub(crate) fn is_ctrl_held(state: &AppState) -> bool {
    state.hotkey_modifiers.control_key()
}

/// Return `true` if either Alt key is currently held.
///
/// Used in order resolution to detect Alt+Ctrl = attack-move (NOT force-fire),
/// matching gamemd's `What_Action_OnCell` Alt-overrides-Ctrl rule.
pub(crate) fn is_alt_held(state: &AppState) -> bool {
    state.hotkey_modifiers.alt_key()
}

/// Read the player-side selection vector in native order. While a selection
/// command is queued the ledger is already the newest local state; after the
/// sim tick, reconciliation trusts the committed selected bits and admits any
/// lifecycle-transferred selection that was not issued by input.
pub(crate) fn selected_stable_ids_in_order(state: &AppState) -> Vec<u64> {
    let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
        return Vec::new();
    };
    let mut ordered = Vec::new();
    for &id in &state.selection_order {
        if let Some(entity) = sim.entities().get(id) {
            if state.selection_order_pending || entity.selected {
                ordered.push(id);
            }
        }
    }
    if !state.selection_order_pending {
        for entity in sim.entities().values() {
            if entity.selected && !ordered.contains(&entity.stable_id) {
                insert_selected_id(&mut ordered, entity.stable_id, sim, state.rules.as_ref());
            }
        }
    }
    ordered
}

/// Synchronize the app ledger after the due selection commands and lifecycle
/// removals have committed for this frame.
pub(crate) fn reconcile_selection_order_after_sim(state: &mut AppState) {
    let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
        state.selection_order.clear();
        state.selection_order_pending = false;
        return;
    };
    if state.selection_order_pending {
        let before_retain = state.selection_order.len();
        state.selection_order.retain(|id| {
            sim.entities()
                .get(*id)
                .is_some_and(|entity| entity.lifecycle.object_alive)
        });
        if state.selection_order.len() != before_retain {
            state.type_select.reset_scope();
        }
        let committed: Vec<u64> = sim
            .entities()
            .values()
            .filter(|entity| entity.selected)
            .map(|entity| entity.stable_id)
            .collect();
        let same_membership = committed.len() == state.selection_order.len()
            && committed
                .iter()
                .all(|id| state.selection_order.contains(id));
        if !same_membership {
            return;
        }
        state.selection_order_pending = false;
        return;
    }
    let prior_len = state.selection_order.len();
    let mut reconciled: Vec<u64> = state
        .selection_order
        .iter()
        .copied()
        .filter(|id| {
            sim.entities()
                .get(*id)
                .is_some_and(|entity| entity.selected)
        })
        .collect();
    let lifecycle_removed = reconciled.len() < prior_len;
    for entity in sim.entities().values() {
        if entity.selected && !reconciled.contains(&entity.stable_id) {
            insert_selected_id(&mut reconciled, entity.stable_id, sim, state.rules.as_ref());
        }
    }
    if lifecycle_removed {
        state.type_select.reset_scope();
    }
    state.selection_order = reconciled;
    state.selection_order_pending = false;
}

fn apply_selection_mutation(
    state: &mut AppState,
    mutation: SelectionMutation,
    reset_type_select_scope: bool,
    voice_policy: SelectionVoicePolicy,
) -> bool {
    if !mutation.clear && mutation.deselect.is_empty() && mutation.select.is_empty() {
        return false;
    }
    let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
        return false;
    };
    let mut ordered = selected_stable_ids_in_order(state);
    let mut native_selection_mode_reset = mutation.clear;
    if mutation.clear {
        ordered.clear();
    }
    let before_deselect = ordered.len();
    ordered.retain(|id| !mutation.deselect.contains(id));
    native_selection_mode_reset |= ordered.len() != before_deselect;

    let mut successful_adds = Vec::new();
    for id in mutation.select {
        let Some(entity) = sim.entities().get(id) else {
            continue;
        };
        let type_id = sim.interner.resolve(entity.type_ref);
        let admitted = entity.lifecycle.object_alive
            && !entity.lifecycle.in_limbo
            && !entity
                .teleport_state
                .as_ref()
                .is_some_and(|teleport| teleport.warp_out_active())
            && state
                .rules
                .as_ref()
                .is_none_or(|rules| rules.object(type_id).is_none_or(|object| object.selectable));
        if !admitted || ordered.contains(&id) {
            continue;
        }
        successful_adds.push(id);
        native_selection_mode_reset = true;
        insert_selected_id(&mut ordered, id, sim, state.rules.as_ref());
    }

    if reset_type_select_scope {
        state.type_select.reset_scope();
    } else if native_selection_mode_reset {
        state.type_select.note_successful_selection_mutation(false);
    }
    for id in selection_voice_recipients(
        voice_policy,
        state.selection_voice_enabled,
        &successful_adds,
    ) {
        emit_selection_voice(state, *id);
    }
    state.selection_order = ordered.clone();
    state.selection_order_pending = true;
    let owner: String = preferred_local_owner(state).unwrap_or_else(|| "Americans".to_string());
    schedule_command(
        state,
        &owner,
        Command::Select {
            entity_ids: ordered,
            additive: !mutation.clear,
        },
    );
    true
}

fn insert_selected_id(
    ordered: &mut Vec<u64>,
    id: u64,
    sim: &crate::sim::world::Simulation,
    rules: Option<&crate::rules::ruleset::RuleSet>,
) {
    let positive_damage_primary = sim.entities().get(id).is_some_and(|entity| {
        let type_id = sim.interner.resolve(entity.type_ref);
        rules
            .and_then(|rules| rules.object(type_id))
            .and_then(|object| object.primary.as_deref())
            .and_then(|weapon_id| rules.and_then(|rules| rules.weapon(weapon_id)))
            .is_some_and(|weapon| weapon.damage > 0)
    });
    insert_selected_id_by_role(ordered, id, positive_damage_primary);
}

fn insert_selected_id_by_role(ordered: &mut Vec<u64>, id: u64, positive_damage_primary: bool) {
    if positive_damage_primary {
        ordered.insert(0, id);
    } else {
        ordered.push(id);
    }
}

fn selection_voice_recipients(
    policy: SelectionVoicePolicy,
    voice_enabled: bool,
    successful_adds: &[u64],
) -> &[u64] {
    if !voice_enabled || policy == SelectionVoicePolicy::Suppressed {
        return &[];
    }
    match policy {
        SelectionVoicePolicy::FirstAdded => &successful_adds[..successful_adds.len().min(1)],
        SelectionVoicePolicy::EveryAdded => successful_adds,
        SelectionVoicePolicy::Suppressed => &[],
    }
}

#[cfg(test)]
mod item83_selection_order_tests {
    use super::{
        HELD_TYPE_SELECT_VOICE_POLICY, ORDINARY_SELECTION_ACTION_LINE_POLICY,
        ORDINARY_SELECTION_VOICE_POLICY, TYPE_SELECT_TAP_ACTION_LINE_POLICY,
        TYPE_SELECT_TAP_VOICE_POLICY, apply_selection_action_line_policy_at_tick,
        insert_selected_id_by_role, selection_voice_event, selection_voice_recipients,
    };
    use crate::app_target_lines::TargetLineState;
    use crate::audio::events::GameSoundEvent;
    use crate::map::entities::EntityCategory;
    use crate::rules::ini_parser::IniFile;
    use crate::rules::ruleset::RuleSet;
    use crate::sim::components::Health;
    use crate::sim::game_entity::GameEntity;
    use crate::sim::world::Simulation;

    #[test]
    fn item83_positive_damage_technos_prepend_and_noncombat_technos_append() {
        let mut order = vec![10];
        insert_selected_id_by_role(&mut order, 20, true);
        insert_selected_id_by_role(&mut order, 30, true);
        insert_selected_id_by_role(&mut order, 40, false);
        assert_eq!(order, [30, 20, 10, 40]);
    }

    #[test]
    fn item83_voice_policy_emits_every_tap_voice_in_candidate_order_only() {
        let rules = RuleSet::from_ini(&IniFile::from_str(
            "[InfantryTypes]\n0=E1\n1=E2\n\n\
             [VehicleTypes]\n\n[AircraftTypes]\n\n[BuildingTypes]\n\n\
             [E1]\nStrength=100\nArmor=none\nSpeed=4\nVoiceSelect=VoiceOne\n\n\
             [E2]\nStrength=100\nArmor=none\nSpeed=4\nVoiceSelect=VoiceTwo\n",
        ))
        .expect("item83 voice rules");
        let mut sim = Simulation::new();
        let owner = sim.interner.intern("Americans");
        for (id, type_name) in [(1, "E1"), (2, "E2")] {
            let type_ref = sim.interner.intern(type_name);
            sim.entities_mut()
                .insert(GameEntity::new_at_frame_zero_for_test(
                    id,
                    10 + id as u16,
                    10,
                    0,
                    0,
                    owner,
                    Health {
                        current: 100,
                        max: 100,
                    },
                    type_ref,
                    EntityCategory::Infantry,
                    0,
                    5,
                    false,
                ));
        }
        let candidate_order = [2, 1];
        let emitted = |policy| {
            selection_voice_recipients(policy, true, &candidate_order)
                .iter()
                .filter_map(|id| selection_voice_event(&sim, &rules, *id))
                .map(|event| match event {
                    GameSoundEvent::UnitSelected { sound_id } => sound_id,
                    other => panic!("unexpected selection event: {other:?}"),
                })
                .collect::<Vec<_>>()
        };

        assert_eq!(
            emitted(TYPE_SELECT_TAP_VOICE_POLICY),
            vec!["VoiceTwo".to_string(), "VoiceOne".to_string()],
            "tap voices retain successful candidate/Select call order"
        );
        assert!(
            emitted(HELD_TYPE_SELECT_VOICE_POLICY).is_empty(),
            "held exact-type group-select suppresses the whole batch"
        );
        assert_eq!(
            emitted(ORDINARY_SELECTION_VOICE_POLICY),
            vec!["VoiceTwo".to_string()],
            "ordinary band/click policy retains only the first success"
        );
    }

    #[test]
    fn item83_type_select_tap_preserves_action_line_timer_while_mouse_selection_starts_it() {
        let mut target_lines = TargetLineState::default();

        apply_selection_action_line_policy_at_tick(
            &mut target_lines,
            10,
            TYPE_SELECT_TAP_ACTION_LINE_POLICY,
        );
        assert!(
            !target_lines.is_selected_action_active(10),
            "a short TypeSelect tap leaves a zero timer untouched"
        );

        apply_selection_action_line_policy_at_tick(
            &mut target_lines,
            10,
            ORDINARY_SELECTION_ACTION_LINE_POLICY,
        );
        assert!(
            target_lines.is_selected_action_active(10),
            "ordinary click/bandbox selection still opens the action-line window"
        );

        apply_selection_action_line_policy_at_tick(
            &mut target_lines,
            20,
            TYPE_SELECT_TAP_ACTION_LINE_POLICY,
        );
        assert!(
            !target_lines.is_selected_action_active(35),
            "a later TypeSelect tap preserves rather than restarts the existing timer"
        );
    }
}

fn queue_selection_snapshot_command(state: &mut AppState, selected_ids: Vec<u64>, additive: bool) {
    apply_selection_mutation(
        state,
        SelectionMutation {
            clear: !additive,
            select: selected_ids,
            ..Default::default()
        },
        true,
        ORDINARY_SELECTION_VOICE_POLICY,
    );
}

fn queue_stop_for_selected(state: &mut AppState) {
    let selected_ids = selected_stable_ids_in_order(state);
    if selected_ids.is_empty() {
        return;
    }
    let owner: String = preferred_local_owner(state).unwrap_or_else(|| "Americans".to_string());
    for entity_id in selected_ids {
        schedule_command(state, &owner, Command::Stop { entity_id });
    }
}

/// Deploy or undeploy selected entities. KeyD toggles:
/// - Selected unit with `DeploysInto` → `Command::DeployMcv` (MCV → ConYard)
/// - Selected structure with `UndeploysInto` → `Command::UndeployBuilding` (ConYard → MCV)
fn queue_deploy_undeploy_for_selected(state: &mut AppState) {
    let selected_ids = selected_stable_ids_in_order(state);
    let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else { return };
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

/// Double-tap window for the centre-on-group shortcut, in milliseconds.
/// The engine compares `timeGetTime()` against the last recall stamp, so this
/// is wall clock and never sim state.
const GROUP_DOUBLE_TAP_MS: u128 = 800;

/// What a digit press resolves to before any state is touched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroupPressAction {
    /// Ctrl+digit — replace the slot's membership with the current selection.
    Assign,
    /// Shift+digit — add the slot's members to the current selection.
    AddToSelection,
    /// Alt+digit, or a bare double-tap — put the camera on the group.
    Center,
    /// Bare digit — clear the selection and select the slot's members.
    Recall,
}

/// Resolve a control-group digit press.
///
/// The four team commands are separate bindings — bare digit, Shift+digit,
/// Ctrl+digit, Alt+digit — and the dispatcher matches the modifier bits
/// exactly, so a two-modifier chord matches no binding and does nothing.
///
/// The double-tap arm is the subtle one: the recall routine only centres when
/// the current selection is *exactly* the group. It bails out on the first
/// group member that is unselected and on the first selected object outside the
/// group — two distinct tests, not "at least one member selected". After a
/// plain recall that condition holds, which is why the familiar double-tap
/// works; shift-clicking one extra unit between the taps makes the second tap
/// recall instead of centre. Only a plain recall stamps the timer.
fn control_group_press_action(
    modifiers: KeyModifiers,
    slot: usize,
    group: &[u64],
    selected: &[u64],
    last_press: Option<(usize, std::time::Duration)>,
) -> Option<GroupPressAction> {
    if modifiers.only_ctrl() {
        return Some(GroupPressAction::Assign);
    }
    if modifiers.only_shift() {
        return Some(GroupPressAction::AddToSelection);
    }
    if modifiers.only_alt() {
        return Some(GroupPressAction::Center);
    }
    if modifiers.any() {
        // Two modifiers at once: no binding carries that key value.
        return None;
    }
    let within_window = last_press.is_some_and(|(last_slot, elapsed)| {
        last_slot == slot && elapsed.as_millis() < GROUP_DOUBLE_TAP_MS
    });
    let selection_is_exactly_the_group = !group.is_empty()
        && group.iter().all(|id| selected.contains(id))
        && selected.iter().all(|id| group.contains(id));
    if within_window && selection_is_exactly_the_group {
        return Some(GroupPressAction::Center);
    }
    Some(GroupPressAction::Recall)
}

/// Assign `ids` to `slot`, evicting them from every other slot.
///
/// Membership is a single group index stored on the object, so a unit belongs
/// to at most one group and re-grouping it silently drops it from the old one.
/// VERA keeps ten id lists, so the eviction is explicit here.
fn assign_control_group(groups: &mut [Vec<u64>], slot: usize, ids: Vec<u64>) {
    for (index, group) in groups.iter_mut().enumerate() {
        if index != slot {
            group.retain(|id| !ids.contains(id));
        }
    }
    groups[slot] = ids;
}

/// Centre of a set of world points in leptons, with the single worst outlier
/// dropped once there are more than two of them — the engine's centre-on-
/// selection maths, which is a trimmed mean rather than a plain centroid so one
/// straggler cannot drag the view off the main body.
///
/// Ties on "farthest" keep the first candidate (comparison is strictly greater);
/// the original's tie-break is UNVERIFIED.
fn trimmed_centroid_leptons(points: &[(i64, i64)]) -> Option<(i64, i64)> {
    if points.is_empty() {
        return None;
    }
    let count = points.len() as i64;
    let sum = points
        .iter()
        .fold((0i64, 0i64), |acc, p| (acc.0 + p.0, acc.1 + p.1));
    let mean = (sum.0 / count, sum.1 / count);
    if points.len() <= 2 {
        return Some(mean);
    }
    let mut farthest = points[0];
    let mut farthest_dist = i64::MIN;
    for p in points {
        let (dx, dy) = (p.0 - mean.0, p.1 - mean.1);
        let dist = dx * dx + dy * dy;
        if dist > farthest_dist {
            farthest_dist = dist;
            farthest = *p;
        }
    }
    Some((
        (sum.0 - farthest.0) / (count - 1),
        (sum.1 - farthest.1) / (count - 1),
    ))
}

/// Leptons per cell.
const LEPTONS_PER_CELL: i64 = 256;

/// Live members of a control group, in the sim's iteration order.
fn live_group_members(state: &AppState, group: &[u64]) -> Vec<u64> {
    let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else {
        return Vec::new();
    };
    group
        .iter()
        .copied()
        .filter(|id| sim.entities().get(*id).is_some())
        .collect()
}

/// Put the camera on a group's trimmed centroid. Emits no command — the centre
/// arm must stay out of the lockstep stream.
fn center_camera_on_group(state: &mut AppState, group: &[u64]) {
    let points: Vec<(i64, i64)> = {
        let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else { return };
        group
            .iter()
            .filter_map(|id| sim.entities().get(*id))
            .map(|e| {
                (
                    e.position.rx as i64 * LEPTONS_PER_CELL + e.position.sub_x.to_num::<i64>(),
                    e.position.ry as i64 * LEPTONS_PER_CELL + e.position.sub_y.to_num::<i64>(),
                )
            })
            .collect()
    };
    let Some((cx, cy)) = trimmed_centroid_leptons(&points) else {
        return;
    };
    let rx = (cx / LEPTONS_PER_CELL).clamp(0, u16::MAX as i64) as u16;
    let ry = (cy / LEPTONS_PER_CELL).clamp(0, u16::MAX as i64) as u16;
    crate::app_camera::center_camera_on_cell(state, rx, ry);
}

fn handle_control_group_command(
    state: &mut AppState,
    group_idx: usize,
    action_override: Option<GroupPressAction>,
) {
    if group_idx >= state.control_groups.len() {
        return;
    }
    let group = state.control_groups[group_idx].clone();
    let selected = selected_stable_ids_in_order(state);
    // Only live members count towards "the selection is exactly the group" —
    // membership is derived by scanning live objects, so a dead unit has
    // already left its group.
    let live_group = live_group_members(state, &group);
    let last_press = state
        .last_control_group_press
        .map(|(slot, at)| (slot, at.elapsed()));
    let action = action_override.unwrap_or_else(|| {
        control_group_press_action(
            KeyModifiers::default(),
            group_idx,
            &live_group,
            &selected,
            last_press,
        )
        .expect("a bare team-select binding always resolves")
    });

    match action {
        GroupPressAction::Assign => {
            assign_control_group(&mut state.control_groups, group_idx, selected);
        }
        GroupPressAction::Center => {
            // Alt+digit selects the group as well as centring; a bare
            // double-tap centres on a selection that already is the group, so
            // the same centring call covers both.
            if action_override == Some(GroupPressAction::Center) && !live_group.is_empty() {
                queue_selection_snapshot_command(state, live_group.clone(), false);
            }
            center_camera_on_group(state, &live_group);
        }
        GroupPressAction::AddToSelection => {
            if live_group.is_empty() {
                return;
            }
            let mut final_ids = selected;
            final_ids.extend(live_group);
            queue_selection_snapshot_command(state, final_ids, true);
        }
        GroupPressAction::Recall => {
            // A recall on an empty group still clears the selection: the
            // deselect-all runs before the select loop, unconditionally.
            state.last_control_group_press = Some((group_idx, std::time::Instant::now()));
            queue_selection_snapshot_command(state, live_group, false);
            apply_selection_action_line_policy(state, ORDINARY_SELECTION_ACTION_LINE_POLICY);
        }
    }
}

/// Apply the selection caller's action-line policy.
///
/// Band-box release, click-select and control-group recall all call the
/// engine's start-timer helper unconditionally, so the freshly selected units
/// flash their current orders for the 25-frame window. TypeSelect tap preserves
/// the prior timer instead.
fn apply_selection_action_line_policy(state: &mut AppState, policy: SelectionActionLinePolicy) {
    let Some(tick) = state.sim_runtime.as_ref().map(|rt| &rt.simulation).map(|sim| sim.session.tick) else {
        return;
    };
    apply_selection_action_line_policy_at_tick(&mut state.target_lines, tick, policy);
}

fn apply_selection_action_line_policy_at_tick(
    target_lines: &mut crate::app_target_lines::TargetLineState,
    tick: u64,
    policy: SelectionActionLinePolicy,
) {
    if policy == SelectionActionLinePolicy::Start {
        target_lines.start_timer(tick);
    }
}

/// Emit the modeled VoiceSelect side effect for one successful Select call.
fn emit_selection_voice(state: &mut AppState, entity_id: u64) {
    let Some(sim) = state.sim_runtime.as_ref().map(|rt| &rt.simulation) else { return };
    let Some(rules) = &state.rules else { return };

    if let Some(event) = selection_voice_event(sim, rules, entity_id) {
        state.sound_events.push(event);
    }
}

fn selection_voice_event(
    sim: &crate::sim::world::Simulation,
    rules: &crate::rules::ruleset::RuleSet,
    entity_id: u64,
) -> Option<GameSoundEvent> {
    let entity = sim.entities().get(entity_id)?;
    let object = rules.object(sim.interner.resolve(entity.type_ref))?;
    Some(GameSoundEvent::UnitSelected {
        sound_id: object.voice_select.clone()?,
    })
}

/// Jump camera to the local player's base.
///
/// Priority: ConYard (structure with `UndeploysInto=`) → MCV (unit with `DeploysInto=`)
/// → multiplayer start waypoint 0 as fallback.
fn jump_camera_to_base(state: &mut AppState) {
    let owner = preferred_local_owner_name(state);
    let owner_name = owner.as_deref();

    // Collect the target cell from simulation entities before mutating state.
    let target: Option<(u16, u16)> = state.sim_runtime.as_ref().map(|rt| &rt.simulation).and_then(|sim| {
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

#[cfg(test)]
mod control_group_tests {
    use super::{
        GroupPressAction, KeyModifiers, assign_control_group, control_group_press_action,
        trimmed_centroid_leptons,
    };
    use std::time::Duration;

    fn mods(shift: bool, ctrl: bool, alt: bool) -> KeyModifiers {
        KeyModifiers { shift, ctrl, alt }
    }

    const BARE: KeyModifiers = KeyModifiers {
        shift: false,
        ctrl: false,
        alt: false,
    };

    #[test]
    fn ctrl_assigns_shift_adds_alt_centers_bare_recalls() {
        let group = [1u64, 2];
        let selected = [1u64, 2];
        assert_eq!(
            control_group_press_action(mods(false, true, false), 0, &group, &selected, None),
            Some(GroupPressAction::Assign)
        );
        assert_eq!(
            control_group_press_action(mods(true, false, false), 0, &group, &selected, None),
            Some(GroupPressAction::AddToSelection)
        );
        assert_eq!(
            control_group_press_action(mods(false, false, true), 0, &group, &selected, None),
            Some(GroupPressAction::Center)
        );
        assert_eq!(
            control_group_press_action(BARE, 0, &group, &selected, None),
            Some(GroupPressAction::Recall)
        );
    }

    /// Two modifiers at once produce a key value no binding carries, so the
    /// press does nothing at all — Ctrl+Shift+digit must not assign.
    #[test]
    fn two_modifiers_match_no_binding() {
        let group = [1u64];
        assert_eq!(
            control_group_press_action(mods(true, true, false), 0, &group, &group, None),
            None
        );
        assert_eq!(
            control_group_press_action(mods(false, true, true), 0, &group, &group, None),
            None
        );
    }

    #[test]
    fn double_tap_inside_the_window_centers() {
        let group = [1u64, 2, 3];
        assert_eq!(
            control_group_press_action(
                BARE,
                2,
                &group,
                &group,
                Some((2, Duration::from_millis(799)))
            ),
            Some(GroupPressAction::Center)
        );
    }

    #[test]
    fn double_tap_outside_the_window_recalls() {
        let group = [1u64, 2, 3];
        assert_eq!(
            control_group_press_action(
                BARE,
                2,
                &group,
                &group,
                Some((2, Duration::from_millis(800)))
            ),
            Some(GroupPressAction::Recall)
        );
    }

    #[test]
    fn a_different_slot_inside_the_window_recalls() {
        let group = [1u64];
        assert_eq!(
            control_group_press_action(
                BARE,
                3,
                &group,
                &group,
                Some((2, Duration::from_millis(10)))
            ),
            Some(GroupPressAction::Recall)
        );
    }

    /// The centre arm needs the selection to be *exactly* the group. One extra
    /// selected unit outside the group is one of the two bail-outs.
    #[test]
    fn an_extra_selected_unit_outside_the_group_recalls() {
        let group = [1u64, 2];
        let selected = [1u64, 2, 9];
        assert_eq!(
            control_group_press_action(
                BARE,
                1,
                &group,
                &selected,
                Some((1, Duration::from_millis(100)))
            ),
            Some(GroupPressAction::Recall)
        );
    }

    /// And the other bail-out: a group member that is not selected.
    #[test]
    fn a_group_member_left_unselected_recalls() {
        let group = [1u64, 2, 3];
        let selected = [1u64, 2];
        assert_eq!(
            control_group_press_action(
                BARE,
                1,
                &group,
                &selected,
                Some((1, Duration::from_millis(100)))
            ),
            Some(GroupPressAction::Recall)
        );
    }

    /// An empty group never centres, however fast the taps come.
    #[test]
    fn an_empty_group_recalls_rather_than_centering() {
        assert_eq!(
            control_group_press_action(BARE, 4, &[], &[], Some((4, Duration::from_millis(10)))),
            Some(GroupPressAction::Recall)
        );
    }

    /// Membership is one group index per object, so assigning evicts.
    #[test]
    fn assignment_evicts_the_units_from_their_previous_group() {
        let mut groups = vec![vec![1u64, 2, 3], vec![4u64], Vec::new()];
        assign_control_group(&mut groups, 1, vec![2, 4]);
        assert_eq!(groups[0], vec![1, 3]);
        assert_eq!(groups[1], vec![2, 4]);
    }

    #[test]
    fn assigning_an_empty_selection_empties_the_slot() {
        let mut groups = vec![vec![1u64, 2], Vec::new()];
        assign_control_group(&mut groups, 0, Vec::new());
        assert!(groups[0].is_empty());
    }

    /// One and two points are a plain mean; the outlier trim only kicks in
    /// above two.
    #[test]
    fn centroid_of_one_or_two_points_is_the_plain_mean() {
        assert_eq!(trimmed_centroid_leptons(&[(100, 200)]), Some((100, 200)));
        assert_eq!(
            trimmed_centroid_leptons(&[(0, 0), (100, 200)]),
            Some((50, 100))
        );
    }

    /// With more than two points the single farthest one is dropped, so a lone
    /// straggler cannot drag the view off the main body.
    #[test]
    fn centroid_of_three_or_more_drops_the_farthest_point() {
        let points = [(0i64, 0i64), (10, 0), (20, 0), (3000, 0)];
        // Plain mean would be 757; dropping (3000,0) leaves 30/3 = 10.
        assert_eq!(trimmed_centroid_leptons(&points), Some((10, 0)));
    }

    #[test]
    fn centroid_of_nothing_is_nothing() {
        assert_eq!(trimmed_centroid_leptons(&[]), None);
    }
}

#[cfg(test)]
mod modifier_tests {
    use super::KeyModifiers;

    /// A bare-key command fires only with no modifier held; the dispatcher
    /// rejects it under Ctrl, Alt or Shift and then looks for a chord binding
    /// that stock does not have.
    #[test]
    fn bare_key_bindings_require_no_modifier() {
        let bare = KeyModifiers::default();
        assert!(bare.none());
        assert!(!bare.any());
        for m in [
            KeyModifiers {
                shift: true,
                ..Default::default()
            },
            KeyModifiers {
                ctrl: true,
                ..Default::default()
            },
            KeyModifiers {
                alt: true,
                ..Default::default()
            },
        ] {
            assert!(m.any(), "{m:?} should block a bare-key binding");
            assert!(!m.none());
        }
    }

    #[test]
    fn exact_modifier_sets_do_not_overlap() {
        let ctrl_shift = KeyModifiers {
            shift: true,
            ctrl: true,
            alt: false,
        };
        assert!(!ctrl_shift.only_ctrl());
        assert!(!ctrl_shift.only_shift());
        assert!(!ctrl_shift.only_alt());
        assert!(ctrl_shift.dev_chord());
    }
}
