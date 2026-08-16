//! Winit lifecycle and window-event routing for the app orchestrator.
//!
//! Event priority and consumption order are player-visible contracts; this
//! module keeps the original handler body intact.

use super::input::dispatch;
use super::{
    ActiveEventLoop, App, AppState, ApplicationHandler, ControlFlow, GameScreen, Instant, KeyCode,
    KeyEventExtModifierSupplement, MouseButton, MouseScrollDelta, PhysicalKey, PhysicalSize,
    SHELL_WINDOW_HEIGHT, SHELL_WINDOW_WIDTH, ShellKey, WindowEvent, WindowId,
    auto_detect_ui_scale,
};

impl App {
    fn resize_surface_for_window_size(state: &mut AppState, size: PhysicalSize<u32>) {
        state.renderer.gpu.resize(size.width, size.height);
        state.renderer.depth_view = state.renderer.gpu.create_depth_texture();
        state.renderer.shell_surface_presenter.resize(&state.renderer.gpu);
        // The frame-index wave is driven by wall-clock ticks and repaints every
        // frame, so a mid-flight resize simply lets it finish; no snap/cancel.
        let new_scale = auto_detect_ui_scale(size.width, size.height);
        if (new_scale - state.match_state.match_presentation.ui_scale).abs() > f32::EPSILON {
            log::info!("UI scale changed: {}x -> {}x", state.match_state.match_presentation.ui_scale, new_scale);
            state.match_state.match_presentation.sidebar_layout_spec = state.match_state.match_presentation.sidebar_layout_spec_base.with_scale(new_scale);
            state.match_state.match_presentation.ui_scale = new_scale;
        }
        Self::invalidate_main_menu_movie_if_base_changed(state);
        crate::app::presentation::sidebar_render::refresh_sidebar_projection(state);
    }

    pub(crate) fn enter_shell_window_mode(state: &mut AppState) {
        state.platform.window.set_resizable(false);
        let target = PhysicalSize::new(SHELL_WINDOW_WIDTH, SHELL_WINDOW_HEIGHT);
        if state.platform.window.inner_size() == target {
            return;
        }
        if let Some(applied_size) = state.platform.window.request_inner_size(target) {
            Self::resize_surface_for_window_size(state, applied_size);
        }
        state.platform.window.request_redraw();
    }

    pub(super) fn enter_game_window_mode(state: &AppState) {
        state.platform.window.set_resizable(true);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        log::info!("Application resumed — creating window and GPU context");
        let capture_dimensions = match (self.shell_capture.as_ref(), self.tactical_capture.as_ref())
        {
            (Some(session), None) => Some((session.request().width(), session.request().height())),
            (None, Some(session)) => Some((session.request().width(), session.request().height())),
            (None, None) => None,
            (Some(_), Some(_)) => {
                log::error!("Shell and tactical capture modes are mutually exclusive");
                event_loop.exit();
                return;
            }
        };
        match Self::initialize(event_loop, capture_dimensions, self.startup_audio) {
            Ok(mut state) => {
                if let Some(session) = self.shell_capture.as_mut() {
                    session.prepare_state(&mut state);
                }
                if let Some(session) = self.tactical_capture.as_mut()
                    && let Err(err) = session.prepare_state(&mut state)
                {
                    log::error!("Tactical capture preparation failed: {err:#}");
                    session.fail(format!("tactical capture preparation failed: {err:#}"));
                    event_loop.exit();
                    return;
                }
                self.state = Some(state);
                log::info!("Initialization complete — showing main menu");
            }
            Err(err) => {
                log::error!("Failed to initialize: {:#}", err);
                if let Some(session) = self.shell_capture.as_mut() {
                    session.fail(format!("shell capture initialization failed: {err:#}"));
                }
                if let Some(session) = self.tactical_capture.as_mut() {
                    session.fail(format!("tactical capture initialization failed: {err:#}"));
                }
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let shell_capture_active = self.shell_capture.is_some();
        let tactical_capture_active = self.tactical_capture.is_some();
        let Some(state) = &mut self.state else { return };

        // This checkpoint is driven entirely from `about_to_wait`. Focused
        // input would contaminate its hidden, no-input production oracle.
        if tactical_capture_active {
            let session = self
                .tactical_capture
                .as_mut()
                .expect("tactical capture session exists");
            match event {
                WindowEvent::CloseRequested => {
                    session.fail("tactical-capture window closed before bundle completion");
                    event_loop.exit();
                }
                WindowEvent::Resized(size) => {
                    let expected =
                        PhysicalSize::new(session.request().width(), session.request().height());
                    if size != expected {
                        session.fail(format!(
                            "tactical-capture surface resized to {}x{}, expected {}x{}",
                            size.width, size.height, expected.width, expected.height
                        ));
                        event_loop.exit();
                    } else {
                        Self::resize_surface_for_window_size(state, size);
                    }
                }
                WindowEvent::Focused(true) => {
                    session.record_focus_violation();
                    event_loop.exit();
                }
                WindowEvent::KeyboardInput { .. }
                | WindowEvent::ModifiersChanged(_)
                | WindowEvent::Ime(_)
                | WindowEvent::CursorMoved { .. }
                | WindowEvent::CursorEntered { .. }
                | WindowEvent::CursorLeft { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::MouseInput { .. }
                | WindowEvent::PinchGesture { .. }
                | WindowEvent::PanGesture { .. }
                | WindowEvent::DoubleTapGesture { .. }
                | WindowEvent::RotationGesture { .. }
                | WindowEvent::TouchpadPressure { .. }
                | WindowEvent::AxisMotion { .. }
                | WindowEvent::Touch(_)
                | WindowEvent::DroppedFile(_)
                | WindowEvent::HoveredFile(_)
                | WindowEvent::HoveredFileCancelled => {
                    session.record_input_violation("window");
                    event_loop.exit();
                }
                _ => {}
            }
            return;
        }

        // A hidden Windows HWND is not guaranteed to receive WM_PAINT, so
        // capture frames are pumped from `about_to_wait`, never window input.
        if shell_capture_active {
            let session = self.shell_capture.as_mut().expect("capture session exists");
            match event {
                WindowEvent::CloseRequested => {
                    log::info!("Shell-capture window close requested");
                    session.fail("shell-capture window closed before bundle completion");
                    event_loop.exit();
                }
                WindowEvent::Resized(size) => {
                    let expected =
                        PhysicalSize::new(session.request().width(), session.request().height());
                    if size != expected {
                        session.fail(format!(
                            "shell-capture surface resized to {}x{}, expected {}x{}",
                            size.width, size.height, expected.width, expected.height
                        ));
                        event_loop.exit();
                    } else {
                        Self::resize_surface_for_window_size(state, size);
                    }
                }
                WindowEvent::Focused(true) => {
                    session.fail("shell-capture window unexpectedly received focus");
                    event_loop.exit();
                }
                WindowEvent::KeyboardInput { .. }
                | WindowEvent::ModifiersChanged(_)
                | WindowEvent::Ime(_)
                | WindowEvent::CursorMoved { .. }
                | WindowEvent::CursorEntered { .. }
                | WindowEvent::CursorLeft { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::MouseInput { .. }
                | WindowEvent::PinchGesture { .. }
                | WindowEvent::PanGesture { .. }
                | WindowEvent::DoubleTapGesture { .. }
                | WindowEvent::RotationGesture { .. }
                | WindowEvent::TouchpadPressure { .. }
                | WindowEvent::AxisMotion { .. }
                | WindowEvent::Touch(_)
                | WindowEvent::DroppedFile(_)
                | WindowEvent::HoveredFile(_)
                | WindowEvent::HoveredFileCancelled => {
                    session.fail("shell-capture received unexpected window input");
                    event_loop.exit();
                }
                _ => {}
            }
            return;
        }

        // Native does not expose menu/game input while its process-start splash
        // owns the display. Keep close/resize/redraw operational, but discard
        // player input until the post-present deadline has elapsed.
        if state
            .frontend.startup_splash
            .as_ref()
            .is_some_and(|splash| splash.is_active(Instant::now()))
            && matches!(
                &event,
                WindowEvent::KeyboardInput { .. }
                    | WindowEvent::ModifiersChanged(_)
                    | WindowEvent::Ime(_)
                    | WindowEvent::CursorMoved { .. }
                    | WindowEvent::CursorEntered { .. }
                    | WindowEvent::CursorLeft { .. }
                    | WindowEvent::MouseWheel { .. }
                    | WindowEvent::MouseInput { .. }
                    | WindowEvent::PinchGesture { .. }
                    | WindowEvent::PanGesture { .. }
                    | WindowEvent::DoubleTapGesture { .. }
                    | WindowEvent::RotationGesture { .. }
                    | WindowEvent::TouchpadPressure { .. }
                    | WindowEvent::AxisMotion { .. }
                    | WindowEvent::Touch(_)
            )
        {
            return;
        }

        // Always let egui see the event first for input handling.
        let egui_response: egui_winit::EventResponse =
            state.renderer.egui.on_window_event(&state.platform.window, &event);

        // In InGame mode, egui only renders non-interactive overlays
        // (mission banner). The custom sidebar handles its own hit-testing.
        // Ignore egui's `consumed` flag in-game to avoid stale UI state
        // from the Loading screen blocking mouse/keyboard input.
        // Exception: when paused or save/load panel is open, egui renders
        // interactive content.
        let egui_consumed: bool = egui_response.consumed
            && (state.frontend.screen != GameScreen::InGame || state.match_state.paused || state.match_state.match_presentation.show_save_load_panel);

        match event {
            WindowEvent::CloseRequested => {
                log::info!("Close requested");
                if Self::native_skirmish_shell_active(state) {
                    // Pump/quit exits write the last durable snapshot after
                    // teardown, without a fresh control pack or RNG draw.
                    Self::teardown_skirmish_shell_for_start(state);
                    state.frontend.offline_skirmish_runtime.persist_snapshot();
                }
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                // A zero dimension is how Windows reports a minimise — that
                // backend never sends `Occluded` — so the hidden flag is
                // derived here as well as from the occlusion event below.
                let hidden = size.width == 0
                    || size.height == 0
                    || state.platform.window.is_minimized().unwrap_or(false);
                Self::set_window_hidden(state, hidden);
                // A minimise carries no usable client size: the surface cannot
                // be configured to 0x0 and the UI-scale heuristic would read a
                // zero viewport. The restore edge delivers the real size.
                if !hidden {
                    Self::resize_surface_for_window_size(state, size);
                }
            }
            WindowEvent::Focused(active) => {
                if !active {
                    // Losing focus takes the mouse capture away, and gamemd
                    // handles that explicitly: its capture-changed case drops
                    // the button-held byte and tears the band rectangle down.
                    // Without this the right-drag pan keeps applying its
                    // anchor-relative step every frame and edge scroll stays
                    // inhibited until the player right-clicks again.
                    state.match_state.input.tactical_mouse = Default::default();
                    state.match_state.input.selection_state.cancel_drag();
                    state.match_state.input.minimap_dragging = false;
                }
                Self::set_window_active(state, active);
            }
            WindowEvent::Occluded(occluded) => {
                // Fully covered on the backends that report it. Windows does
                // not; see the `Resized` arm above.
                Self::set_window_hidden(state, occluded);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                // Native's paused input capture admits Escape only and does not
                // mutate the recorded keyboard state for other input.
                if !state.match_state.paused {
                    state.match_state.input.hotkey_modifiers = modifiers.state();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    // ESC always reaches the handler when in-game (even when paused)
                    // so the player can toggle pause regardless of egui focus.
                    let is_escape: bool =
                        code == KeyCode::Escape && event.state.is_pressed() && !event.repeat;
                    let in_game: bool = state.frontend.screen == GameScreen::InGame;
                    let paused_at_event = in_game && state.match_state.paused;

                    if crate::app::frontend::shell_transition::blocks_shell_input(state) {
                        return;
                    }

                    // A main-menu modal dialog (exit confirm, options, movies,
                    // campaign select) takes ESC first: close it and stay,
                    // never propagating to the shell-close handlers below.
                    // The exit-confirm modal routes through the controller
                    // (on_key → LIFO pop, D-B3); the egui-only dialogs are
                    // not on the stack and keep the direct close.
                    if Self::main_menu_dialog_open(state) {
                        if is_escape {
                            if state.frontend.exit_confirm_modal.is_some() {
                                if !Self::route_exit_confirm_modal_key(state, ShellKey::Escape) {
                                    // Defensive: on_key only fails with an
                                    // empty route — still close consistently.
                                    Self::close_exit_confirm_modal_from_controller(state);
                                    state.platform.window.request_redraw();
                                }
                            } else {
                                Self::close_main_menu_dialogs(state);
                                state.platform.window.request_redraw();
                            }
                        }
                        return;
                    }

                    if Self::native_skirmish_shell_active(state)
                        && event.state.is_pressed()
                        && !event.repeat
                        && Self::shell_key_for_code(code)
                            .is_some_and(|key| Self::route_validation_modal_key(state, key))
                    {
                        return;
                    }

                    if Self::native_skirmish_shell_active(state) && is_escape {
                        if state.frontend.skirmish_shell_state.choose_map_modal.is_some() {
                            // Native chooser `0x6B` has no verified Escape
                            // dismissal. Consume the key without applying the
                            // Cancel transaction or changing its selection.
                            state.platform.window.request_redraw();
                            return;
                        }
                        Self::handle_skirmish_shell_action(
                            state,
                            crate::ui::skirmish_shell::SkirmishShellAction::BackOrExit,
                            event_loop,
                        );
                        state.platform.window.request_redraw();
                        return;
                    }

                    if Self::single_player_shell_active(state) && is_escape {
                        Self::close_single_player_shell(state);
                        state.platform.window.request_redraw();
                        return;
                    }

                    if !crate::app::input::hotkeys::input_admitted_while_paused(
                        paused_at_event,
                        &event.logical_key,
                    ) {
                        return;
                    }

                    if Self::native_skirmish_shell_active(state)
                        && event.state.is_pressed()
                        && !is_escape
                        && Self::handle_skirmish_shell_key_input(state, code, event.text.as_deref())
                    {
                        return;
                    }

                    // The in-scenario modal machine owns Escape: it opens the
                    // in-game menu, backs Options out to its parent menu, and
                    // dismisses the abort confirmation. Escape's in-world
                    // cancel duties (placement/targeting, repair/sell) still
                    // run first — see `in_game_menu_owns_escape`.
                    if in_game && is_escape && Self::in_game_menu_owns_escape(state) {
                        Self::route_in_game_menu_escape(state);
                        state.platform.window.request_redraw();
                        return;
                    }

                    let key_without_modifiers = event.key_without_modifiers();
                    let binding_key = crate::app::input::hotkeys::binding_logical_key(
                        &event.logical_key,
                        &key_without_modifiers,
                        event.location,
                    );
                    let hotkey_resolution = state.match_state.input.hotkey_bindings.resolve_event(
                        binding_key,
                        event.location,
                        state.match_state.input.hotkey_modifiers,
                    );
                    if in_game && (is_escape || !egui_consumed) {
                        let type_select_consumed = dispatch::handle_type_select_key_edge(
                            state,
                            hotkey_resolution,
                            code,
                            event.state,
                            event.repeat,
                        );
                        if event.state.is_pressed() && !event.repeat && !type_select_consumed {
                            dispatch::handle_hotkey_pressed(state, hotkey_resolution, code);
                        }
                    }
                    // A key received by the paused capture changes no held-key
                    // state, including the Escape press that closes it.
                    if in_game && !paused_at_event && !egui_consumed {
                        if event.state.is_pressed() {
                            if let Some(scroll_key) =
                                crate::app::input::hotkeys::fallback_scroll_key(hotkey_resolution)
                            {
                                state.match_state.input.keys_held.insert(scroll_key);
                            } else if crate::app::input::hotkeys::physical_scroll_key(code).is_none() {
                                state.match_state.input.keys_held.insert(code);
                            }
                        } else {
                            // A release always clears a previously admitted
                            // scroll flag, even if NumLock or bindings changed
                            // while the key was held.
                            state.match_state.input.keys_held.remove(&code);
                            if let Some(scroll_key) =
                                crate::app::input::hotkeys::fallback_scroll_key(hotkey_resolution)
                                    .or_else(|| crate::app::input::hotkeys::physical_scroll_key(code))
                            {
                                state.match_state.input.keys_held.remove(&scroll_key);
                            }
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                // When upscaling, remap window coordinates to render-target coordinates.
                let use_render_source_coords = state.renderer.upscale_pass.is_some()
                    && (state.frontend.screen == GameScreen::InGame
                        || state.frontend.screen == GameScreen::SpawnPick);
                let (sx, sy) = if use_render_source_coords {
                    (
                        state.render_width() as f32 / state.renderer.gpu.config.width as f32,
                        state.render_height() as f32 / state.renderer.gpu.config.height as f32,
                    )
                } else {
                    (1.0, 1.0)
                };
                state.match_state.input.cursor_x = position.x as f32 * sx;
                state.match_state.input.cursor_y = position.y as f32 * sy;
                // Keep OS cursor hidden whenever the software cursor is active.
                if state.use_software_cursor() {
                    state.platform.window.set_cursor_visible(false);
                }
                // Shared tooltip service: every move restarts the show delay
                // and hides a visible tip (study S1).
                crate::app::input::tooltips::on_mouse_move(state);
                if crate::app::frontend::shell_transition::blocks_shell_input(state) {
                    return;
                }
                if !egui_consumed
                    && (state.frontend.screen == GameScreen::InGame || state.frontend.screen == GameScreen::SpawnPick)
                {
                    dispatch::handle_cursor_moved_in_game(state);
                }
                if !egui_consumed && Self::native_skirmish_shell_active(state) {
                    Self::handle_skirmish_shell_mouse_move(state);
                }
                if !egui_consumed && Self::single_player_shell_active(state) {
                    Self::handle_single_player_shell_mouse_move(state);
                }
                if Self::score_shell_active(state) {
                    Self::handle_score_shell_mouse_move(state);
                }
                if !egui_consumed
                    && state.frontend.screen == GameScreen::MainMenu
                    && !state.frontend.main_menu_shell_failed
                    && !Self::single_player_shell_active(state)
                    && !Self::native_skirmish_shell_active(state)
                    // While the SHP quit-confirm modal owns the controller, the menu
                    // move handler must not re-activate 0xE2 and reset the gesture.
                    && state.frontend.exit_confirm_modal.is_none()
                {
                    Self::handle_main_menu_shell_mouse_move(state);
                }
            }
            WindowEvent::MouseInput {
                state: btn_state,
                button,
                ..
            } => {
                // Keep OS cursor hidden on click events (not just CursorMoved).
                // Without this, rapid clicks without mouse movement let the OS
                // cursor flash visible between WM_SETCURSOR and the next render.
                if state.use_software_cursor() {
                    state.platform.window.set_cursor_visible(false);
                }
                // Any button press/release kills a visible tooltip + pending
                // timer (all buttons incl. middle — study S1).
                crate::app::input::tooltips::on_button_event(state);
                if crate::app::frontend::shell_transition::blocks_shell_input(state) {
                    return;
                }
                // While a main-menu modal dialog is open, route the click to the
                // SHP quit-confirm modal's OK/Cancel hit-test on the normal shell
                // path; the egui fallback and the other egui dialogs (options/movies/
                // campaign) were already handled by egui above.
                if Self::main_menu_dialog_open(state) {
                    if state.frontend.exit_confirm_modal.is_some()
                        && state.frontend.screen == GameScreen::MainMenu
                        && !state.frontend.main_menu_shell_failed
                        && button == MouseButton::Left
                    {
                        if btn_state.is_pressed() {
                            Self::handle_exit_confirm_modal_mouse_down(state);
                        } else {
                            Self::handle_exit_confirm_modal_mouse_up(state);
                        }
                    }
                    return;
                }
                if Self::score_shell_active(state) {
                    if button == MouseButton::Left {
                        if btn_state.is_pressed() {
                            Self::handle_score_shell_mouse_down(state);
                        } else {
                            Self::handle_score_shell_mouse_up(state);
                        }
                    }
                } else if Self::native_skirmish_shell_active(state) {
                    if button == MouseButton::Left {
                        if btn_state.is_pressed() {
                            Self::handle_skirmish_shell_mouse_down(state);
                        } else {
                            Self::handle_skirmish_shell_mouse_up(state, event_loop);
                        }
                    }
                } else if Self::single_player_shell_active(state) {
                    if button == MouseButton::Left {
                        if btn_state.is_pressed() {
                            Self::handle_single_player_shell_mouse_down(state);
                        } else {
                            Self::handle_single_player_shell_mouse_up(state);
                        }
                    }
                } else if state.frontend.screen == GameScreen::MainMenu
                    && !state.frontend.main_menu_shell_failed
                    && !egui_consumed
                {
                    if button == MouseButton::Left {
                        if btn_state.is_pressed() {
                            Self::handle_main_menu_shell_mouse_down(state);
                        } else {
                            Self::handle_main_menu_shell_mouse_up(state, event_loop);
                        }
                    }
                } else if !egui_consumed && state.frontend.screen == GameScreen::SpawnPick {
                    if button == MouseButton::Left && btn_state.is_pressed() {
                        crate::app::presentation::spawn_pick::handle_spawn_pick_click(state);
                    }
                } else if !egui_consumed && state.frontend.screen == GameScreen::InGame {
                    dispatch::handle_mouse_input(state, button, btn_state);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => (pos.y as f32 / 30.0).clamp(-3.0, 3.0),
                };
                if crate::app::frontend::shell_transition::blocks_shell_input(state) {
                    return;
                }
                if !egui_consumed
                    && state.frontend.screen == GameScreen::MainMenu
                    && Self::native_skirmish_shell_active(state)
                    && Self::handle_skirmish_shell_mouse_wheel(state, lines)
                {
                    state.platform.window.request_redraw();
                    return;
                }
                if !egui_consumed
                    && (state.frontend.screen == GameScreen::SpawnPick
                        || (state.frontend.screen == GameScreen::InGame && !state.match_state.paused))
                {
                    // Every wheel notch scrolls the active build strip by one
                    // row, wherever the cursor is. gamemd routes the wheel
                    // message straight to the SidebarUp / SidebarDown commands
                    // and has no world zoom for it to reach.
                    dispatch::sidebar_wheel_scroll(state, lines);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(err) = Self::render_frame(state, event_loop, None, None) {
                    log::error!("Render: {:#}", err);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let (Some(state), Some(session)) = (self.state.as_mut(), self.tactical_capture.as_mut())
        {
            if let Err(err) = Self::render_frame(state, event_loop, None, Some(&mut *session)) {
                log::error!("Tactical capture render: {err:#}");
                session.fail(format!("tactical capture render failed: {err:#}"));
                event_loop.exit();
                return;
            }
            if !session.is_finished() {
                event_loop.set_control_flow(ControlFlow::WaitUntil(session.next_wake_deadline()));
            }
        } else if let (Some(state), Some(session)) =
            (self.state.as_mut(), self.shell_capture.as_mut())
        {
            if let Err(err) = Self::render_frame(state, event_loop, Some(&mut *session), None) {
                log::error!("Shell capture render: {err:#}");
                session.fail(format!("shell capture render failed: {err:#}"));
                event_loop.exit();
                return;
            }
            if !session.is_finished() {
                let mut deadline = session.next_wake_deadline();
                if let Some(wave_deadline) =
                    crate::app::frontend::shell_transition::main_menu_presented_wake_deadline(state)
                {
                    deadline = deadline.max(wave_deadline);
                }
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            }
        } else if let Some(state) = &self.state {
            if crate::app::frontend::shell_transition::main_menu_presented_is_poisoned(state) {
                event_loop.set_control_flow(ControlFlow::Wait);
            } else if let Some(deadline) =
                crate::app::frontend::shell_transition::main_menu_presented_wake_deadline(state)
            {
                if Instant::now() >= deadline {
                    state.platform.window.request_redraw();
                } else {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
                }
            } else if state.platform.window_hidden {
                // Nothing on screen to keep fresh. Park the redraw loop until a
                // window event (including the un-occlude edge) wakes it, rather
                // than rendering frames no one can see.
                event_loop.set_control_flow(ControlFlow::Wait);
            } else {
                state.platform.window.request_redraw();
            }
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_mut() {
            crate::app::match_runtime::sim_tick::flush_replay_log(state);
        }
        log::logger().flush();
    }
}
