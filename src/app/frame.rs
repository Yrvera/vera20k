//! Top-level frame orchestration, screen dispatch, presentation, and readback.
//!
//! Simulation admission, draw composition, submit/present, transition commits,
//! captures, and loading-after-present remain in their original source order.

use super::{
    ActiveEventLoop, App, AppState, GameScreen, Instant, Result, app_render, app_sim_tick,
    app_transitions, frontend::startup_splash, main_menu,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellFramePreludeStep {
    MaintainIntro,
    ObserveEntry,
}

const MAIN_MENU_SHELL_PRELUDE: &[ShellFramePreludeStep] = &[
    ShellFramePreludeStep::MaintainIntro,
    ShellFramePreludeStep::ObserveEntry,
];

impl App {
    /// Dispatch rendering based on current GameScreen state.
    pub(super) fn render_frame(
        state: &mut AppState,
        event_loop: &ActiveEventLoop,
        mut shell_capture: Option<&mut crate::app::diagnostics::shell_capture::ShellCaptureSession>,
        mut tactical_capture: Option<
            &mut crate::app::diagnostics::tactical_capture::session::TacticalCaptureSession,
        >,
    ) -> Result<()> {
        anyhow::ensure!(
            shell_capture.is_none() || tactical_capture.is_none(),
            "shell and tactical capture cannot share a render"
        );
        if let Some(session) = tactical_capture.as_deref_mut() {
            session.drive_before_render(state)?;
        }
        state.frame_timer.sample(Instant::now());
        crate::app_tooltips::update(state);
        // The message clock has to observe the focus freeze exactly as it
        // observes a modal pause: a banner on screen when the player Alt+Tabs
        // must survive the absence with its remaining lifetime intact, not
        // expire against wall time while the world is stopped. Park the clock
        // and skip the expiry pass; `app_messages::update` closes the span and
        // resumes ownership on the first foreground frame.
        if state.screen == GameScreen::InGame && !state.platform.window_active {
            let wall = crate::app_tooltips::now_ms(state);
            state.message_clock.set_paused(true, wall);
        } else {
            crate::app_messages::update(state);
        }
        if state
            .startup_splash
            .as_ref()
            .is_some_and(|splash| splash.is_active(Instant::now()))
        {
            let splash = state
                .startup_splash
                .as_ref()
                .expect("active startup splash exists");
            startup_splash::render_and_present(
                &state.gpu,
                &state.batch_renderer,
                &state.shell_surface_presenter,
                &state.depth_view,
                splash,
            )?;
            state
                .startup_splash
                .as_mut()
                .expect("active startup splash exists")
                .mark_presented(Instant::now());
            return Ok(());
        }
        state.startup_splash = None;

        // HouseClass keeps simulating for SavourDelay, then blocks on the
        // current outcome Vox before it raises the victory/defeat exit global.
        // Drive that gate before deciding whether another sim frame is legal.
        let scenario_now_ms = crate::app_sim_tick::monotonic_frame_pacer_ms(state, Instant::now());
        Self::consume_executed_abort_exit(state, scenario_now_ms);
        crate::app_sim_tick::drive_local_player_outcome_voice_wait(state, scenario_now_ms);

        // The native victory/defeat handlers synchronously finish their audio
        // teardown before entering the score dialog. Drive the equivalent
        // sequence before either another sim frame or the destination screen.
        Self::drive_scenario_exit(state, scenario_now_ms);

        // Drive the graceful quit cascade (started on Exit-confirm OK). Compute the
        // voice poll before borrowing the cascade mutably to avoid aliasing.
        if state.quit_cascade.is_some() {
            let now = Instant::now();
            let voices_active = state
                .sfx_player
                .as_ref()
                .is_some_and(|sfx| sfx.voices_active());
            let tick = state
                .quit_cascade
                .as_mut()
                .expect("cascade present")
                .tick(now, voices_active);
            if let (Some(vol), Some(player)) = (tick.music_volume, state.music_player.as_mut()) {
                player.set_volume(vol);
            }
            if tick.stop_music {
                if let Some(player) = state.music_player.as_mut() {
                    player.stop();
                }
            }
            if tick.finished {
                state.quit_cascade = None;
                event_loop.exit();
                return Ok(());
            }
        }

        Self::sync_in_game_menu_with_options_overlay(state);

        // Deactivated windows do not simulate. gamemd parks its main tick in a
        // sleep-and-network-only loop while the app is not the foreground, so
        // the world is exactly where the player left it on Alt+Tab return. The
        // gate sits at the call site, not inside the runtime, so a focus edge
        // never re-anchors the frame pacer on its own.
        if tactical_capture.is_none()
            && matches!(state.screen, GameScreen::InGame)
            && state.platform.window_active
            && state.scenario_exit.is_none()
            && state.scenario_outcome.is_none()
        {
            let now = Instant::now();
            let now_ms = app_sim_tick::monotonic_frame_pacer_ms(state, now);
            app_sim_tick::advance_in_game_runtime(state, now_ms);
            // EventClass EXIT is dispatched at the simulation tail. Consume
            // its terminal edge before any outcome route can claim teardown.
            Self::consume_executed_abort_exit(state, now_ms);
            // The SavourDelay expiry is decided in the late house rung of this
            // exact frame. Anchor its 0x78-bucket wall wait to the same observed
            // wall time instead of delaying it to the next render pass.
            crate::app_sim_tick::drive_local_player_outcome_voice_wait(state, now_ms);
            Self::drive_scenario_exit(state, now_ms);
        }

        // Native queues/maintains [INTRO] before arming the 0xE2 first-paint
        // owner. An arm stays silent until the matching surface is acquired.
        for step in MAIN_MENU_SHELL_PRELUDE {
            match step {
                ShellFramePreludeStep::MaintainIntro => Self::maintain_main_menu_intro(state),
                ShellFramePreludeStep::ObserveEntry => {
                    crate::app_shell_transition::prepare_main_menu_first_paint_before_acquire(state)
                }
            }
        }
        match crate::app_shell_transition::poll_main_menu_first_paint_before_acquire(
            state,
            Instant::now(),
        )? {
            crate::app_shell_transition::MainMenuFirstPaintPoll::WaitUntil(_) => {
                return Ok(());
            }
            crate::app_shell_transition::MainMenuFirstPaintPoll::Completed => {
                if let Some(session) = shell_capture.as_deref_mut()
                    && session.completion_handoff()
                        == crate::app::diagnostics::shell_capture::ShellCompletionHandoff::
                            FinalizeExitReturnBeforeAcquire
                {
                    session.complete_entry_sequence_after_wave(state)?;
                    event_loop.exit();
                    return Ok(());
                }
            }
            crate::app_shell_transition::MainMenuFirstPaintPoll::Acquire => {}
        }

        let output: wgpu::SurfaceTexture = state
            .gpu
            .surface
            .get_current_texture()
            .map_err(|e| anyhow::anyhow!("Surface texture: {}", e))?;
        let view: wgpu::TextureView = output.texture.create_view(&Default::default());
        let mut encoder: wgpu::CommandEncoder =
            state
                .gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Frame"),
                });
        let mut pending_main_menu_entry_token = None;
        let mut pending_main_menu_title_receipt = None;

        crate::app_shell_transition::activate_shell_first_paint_after_acquire(state);
        // Advance the Skirmish right-panel static text reveals (started at the
        // slide's completion edge). 30 ms-gated internally; a no-op when idle.
        crate::app_shell_transition::advance_shell_static_reveals(state);
        crate::app_shell_transition::poll_main_menu_title_reveal(state);
        let shell_capture_current_frame = match shell_capture.as_deref_mut() {
            Some(session) => session.should_capture_current_frame(state)?,
            None => false,
        };
        let mut game_render_output: Option<crate::app_render::GameRenderOutput> = None;

        match &state.screen {
            GameScreen::MainMenu => {
                if let crate::app_shell_transition::ShellFirstPaintRenderResult::Rendered {
                    main_menu_entry_token,
                } = crate::app_shell_transition::render_shell_first_paint_slide(
                    state,
                    &mut encoder,
                    &output.texture,
                )? {
                    pending_main_menu_entry_token = main_menu_entry_token;
                } else if Self::native_skirmish_shell_active(state) {
                    crate::app_skirmish_shell_render::render_skirmish_shell(
                        state,
                        &mut encoder,
                        &output.texture,
                    )?;
                } else if Self::single_player_shell_active(state) {
                    match crate::app_single_player_shell_render::render_single_player_shell(
                        state,
                        &mut encoder,
                        &output.texture,
                    )? {
                        crate::app_single_player_shell_render::SinglePlayerShellRenderResult::Rendered => {
                            state.egui.begin_frame(&state.platform.window);
                            if state.show_save_load_panel {
                                Self::handle_save_load_panel(state);
                            }
                            // Campaign selector (and any other menu modal) draws
                            // over the SP shell; confirm-quit cannot originate
                            // here, so its return value is ignored.
                            let _ = Self::draw_main_menu_dialogs(state, false);
                            state.egui.end_frame_and_render(
                                &state.gpu,
                                &mut encoder,
                                &view,
                                &state.platform.window,
                                state.use_software_cursor(),
                            );
                        }
                        crate::app_single_player_shell_render::SinglePlayerShellRenderResult::Fallback => {
                            Self::render_egui_main_menu_fallback(
                                state,
                                &mut encoder,
                                &view,
                                event_loop,
                            )?;
                        }
                    }
                } else if !state.main_menu_shell_failed {
                    match crate::app_main_menu_shell_render::render_main_menu_shell(
                        state,
                        &mut encoder,
                        &output.texture,
                    )? {
                        crate::app_main_menu_shell_render::MainMenuShellRenderResult::Rendered {
                            title_receipt,
                        } => {
                            pending_main_menu_title_receipt = title_receipt;
                            state.egui.begin_frame(&state.platform.window);
                            // The SHP shell renders the quit-confirm as an SHP
                            // overlay (and OK exits via its hit-test), so the egui
                            // exit-confirm is suppressed here; campaign/options/
                            // movies egui dialogs still draw. confirm_quit stays false.
                            let confirm_quit = Self::draw_main_menu_dialogs(state, false);
                            state.egui.end_frame_and_render(
                                &state.gpu,
                                &mut encoder,
                                &view,
                                &state.platform.window,
                                state.use_software_cursor(),
                            );
                            if confirm_quit {
                                state.gpu.queue.submit(std::iter::once(encoder.finish()));
                                output.present();
                                if let Some(token) =
                                    pending_main_menu_entry_token.take()
                                {
                                    crate::app_shell_transition::record_main_menu_entry_presented(
                                        state, token,
                                    )?;
                                }
                                if let Some(receipt) =
                                    pending_main_menu_title_receipt.take()
                                {
                                    anyhow::ensure!(
                                        state
                                            .main_menu_shell_state
                                            .title_reveal
                                            .record_presented(receipt),
                                        "main-menu title receipt was stale at present commit"
                                    );
                                }
                                event_loop.exit();
                                return Ok(());
                            }
                        }
                        crate::app_main_menu_shell_render::MainMenuShellRenderResult::Fallback => {
                            Self::render_egui_main_menu_fallback(
                                state,
                                &mut encoder,
                                &view,
                                event_loop,
                            )?;
                        }
                    }
                } else {
                    Self::render_egui_main_menu_fallback(state, &mut encoder, &view, event_loop)?;
                }
            }
            GameScreen::Loading => {
                match crate::app_loading::render_loading_screen(
                    state,
                    &mut encoder,
                    &output.texture,
                ) {
                    crate::app_loading::LoadingRenderResult::NativeRendered => {}
                    crate::app_loading::LoadingRenderResult::GenericFallback => {
                        let map_name_display = crate::app_loading::loading_map_name(state)
                            .unwrap_or("auto")
                            .to_string();
                        app_transitions::clear_screen(&mut encoder, &view);
                        state.egui.begin_frame(&state.platform.window);
                        main_menu::draw_loading_screen(&state.egui.ctx, &map_name_display);
                        state.egui.end_frame_and_render(
                            &state.gpu,
                            &mut encoder,
                            &view,
                            &state.platform.window,
                            state.use_software_cursor(),
                        );
                    }
                    crate::app_loading::LoadingRenderResult::NativeFailed(err) => {
                        app_transitions::clear_screen(&mut encoder, &view);
                        log::warn!("Could not render native loading screen: {err:#}");
                        crate::app_loading::clear_loading_state(state);
                        crate::app_loading::clear_match_startup_state(state);
                        state.screen = GameScreen::MissionResult {
                            title: "Loading Failed".to_string(),
                            detail: format!("{err:#}"),
                        };
                    }
                }
            }
            GameScreen::InGame => {
                let game_output = if state.upscale_pass.is_some() {
                    // Render game to intermediate texture, then upscale to swapchain.
                    let up = state.upscale_pass.as_ref().unwrap();
                    let game_depth = up.depth_view().clone();
                    let saved_depth = std::mem::replace(&mut state.depth_view, game_depth);
                    let result = app_render::render_game(state, &mut encoder);
                    state.depth_view = saved_depth;
                    let render_output = result?;
                    state.combat_light_renderer.copy_to(
                        &mut encoder,
                        state.upscale_pass.as_ref().unwrap().color_texture(),
                    );
                    state
                        .upscale_pass
                        .as_ref()
                        .unwrap()
                        .draw(&mut encoder, &view);
                    render_output
                } else {
                    let render_output = app_render::render_game(state, &mut encoder)?;
                    state
                        .combat_light_renderer
                        .copy_to(&mut encoder, &output.texture);
                    render_output
                };
                let sidebar_view = game_output.sidebar_view.as_ref();
                // Options (the in-scenario state the menu's Game Controls button
                // opens) draws the native `0xBBB` overlay over the frozen
                // battlefield, before egui. The in-game menu and the abort
                // confirmation are egui cards drawn in the pass below.
                if state.in_game_menu == crate::ui::pause_menu::InGameMenuState::Options {
                    if Self::ensure_skirmish_shell_chrome(state) {
                        crate::app_skirmish_shell_render::render_in_game_options_overlay(
                            state,
                            &mut encoder,
                            &view,
                            sidebar_view,
                        )?;
                    }
                }
                // All sidebar text (credits, Ready labels, queue counts) is now
                // GAME.FNT sprite geometry built in app_render; egui in-game
                // carries only the dev/debug overlays.
                state.egui.begin_frame(&state.platform.window);
                // Debug panels use a light/.NET theme — push light visuals
                // before rendering, then restore the original after.
                let any_debug_panel = state.debug_show_pathgrid
                    || state.debug_unit_inspector
                    || state.show_hotkey_help;
                let prev_visuals = if any_debug_panel {
                    Some(crate::app::diagnostics::debug_panel::push_debug_light_visuals(
                        &state.egui.ctx,
                    ))
                } else {
                    None
                };
                if state.debug_show_pathgrid {
                    crate::app::diagnostics::debug_panel::draw_debug_panel(&state.egui.ctx, state);
                }
                crate::app::diagnostics::debug_panel::draw_event_history_panel(&state.egui.ctx, state);
                if state.show_hotkey_help {
                    crate::app::diagnostics::debug_panel::draw_hotkey_help(&state.egui.ctx);
                }
                if let Some(prev) = prev_visuals {
                    crate::app::diagnostics::debug_panel::pop_debug_light_visuals(&state.egui.ctx, prev);
                }
                if state.show_save_load_panel {
                    Self::handle_save_load_panel(state);
                }
                // The in-scenario modal cards. Options is the native `0xBBB`
                // overlay drawn above; the menu and the abort confirmation are
                // drawn here and their routes committed immediately.
                Self::handle_in_game_menu(state);
                if state.paused {
                    // The dev overlay rides along with any in-scenario modal —
                    // push its own light visuals so its chrome matches the
                    // debug panels.
                    let prev = crate::app::diagnostics::debug_panel::push_debug_light_visuals(&state.egui.ctx);
                    Self::handle_dev_overlay(state);
                    crate::app::diagnostics::debug_panel::pop_debug_light_visuals(&state.egui.ctx, prev);
                }
                state.egui.end_frame_and_render(
                    &state.gpu,
                    &mut encoder,
                    &view,
                    &state.platform.window,
                    state.use_software_cursor(),
                );
                game_render_output = Some(game_output);
            }
            GameScreen::MissionResult { title, detail } => {
                // The fallback card's strings are copied out before the score
                // render takes `state` mutably.
                let (title, detail) = (title.clone(), detail.clone());
                // A finished match presents the native score screen. Result
                // screens with no native analogue (a load failure, a
                // trigger-driven campaign end) carry no model and keep the
                // non-art card.
                let score_rendered = if Self::score_shell_active(state) {
                    matches!(
                        crate::app_score_shell_render::render_score_shell(
                            state,
                            &mut encoder,
                            &output.texture,
                        )?,
                        crate::app_score_shell_render::ScoreShellRenderResult::Rendered
                    )
                } else {
                    false
                };
                if !score_rendered {
                    app_transitions::clear_screen(&mut encoder, &view);
                    state.egui.begin_frame(&state.platform.window);
                    if crate::ui::mission_status::draw_mission_result_screen(
                        &state.egui.ctx,
                        &title,
                        &detail,
                    ) {
                        // Persist the deterministic diagnostic log before the sim
                        // is torn down, symmetric with return_to_main_menu.
                        Self::leave_mission_result_screen(state);
                    }
                    state.egui.end_frame_and_render(
                        &state.gpu,
                        &mut encoder,
                        &view,
                        &state.platform.window,
                        state.use_software_cursor(),
                    );
                }
            }
            GameScreen::SpawnPick => {
                crate::app_spawn_pick::render_spawn_pick(
                    state,
                    &mut encoder,
                    &output.texture,
                    &view,
                )?;
                state.egui.begin_frame(&state.platform.window);
                crate::app_spawn_pick::draw_spawn_pick_overlay(&state.egui.ctx.clone(), state);
                state.egui.end_frame_and_render(
                    &state.gpu,
                    &mut encoder,
                    &view,
                    &state.platform.window,
                    state.use_software_cursor(),
                );
            }
        }

        let entry_sequence_identity = match shell_capture.as_deref_mut() {
            Some(session) if session.is_entry_sequence() => {
                let token = pending_main_menu_entry_token
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("entry-sequence frame produced no token"))?;
                session.observe_entry_sequence_after_render(state, token)?
            }
            _ => None,
        };
        let pending_entry_sequence = if entry_sequence_identity.is_some() {
            Some(crate::render::frame_readback::PendingBgra8Readback::encode(
                &state.gpu.device,
                &mut encoder,
                &output.texture,
                state.gpu.config.format,
                state.gpu.config.width,
                state.gpu.config.height,
            )?)
        } else {
            None
        };
        let tactical_capture_current_frame =
            match (tactical_capture.as_deref_mut(), game_render_output.as_ref()) {
                (Some(session), Some(render_output)) => {
                    session.observe_after_render(state, render_output)?
                }
                (Some(_), None) | (None, _) => false,
            };
        let capture_current_frame = shell_capture_current_frame || tactical_capture_current_frame;
        let pending_capture = if capture_current_frame {
            Some(crate::render::frame_readback::PendingBgra8Readback::encode(
                &state.gpu.device,
                &mut encoder,
                &output.texture,
                state.gpu.config.format,
                state.gpu.config.width,
                state.gpu.config.height,
            )?)
        } else {
            None
        };
        let retail_screenshot_current_frame =
            std::mem::take(&mut state.retail_screenshot_requested);
        let pending_retail_screenshot = state
            .retail_screenshot_frame_cache
            .capture_previous_if_requested(
                retail_screenshot_current_frame,
                &state.gpu.device,
                &mut encoder,
                state.gpu.config.format,
                state.gpu.config.width,
                state.gpu.config.height,
                state.upscale_pass.as_ref(),
            )?;
        let capture_timeout = if capture_current_frame {
            Some(if shell_capture_current_frame {
                shell_capture
                    .as_deref()
                    .expect("shell capture session exists when readback is requested")
                    .readback_timeout()?
            } else {
                tactical_capture
                    .as_deref()
                    .expect("tactical capture session exists when readback is requested")
                    .readback_timeout()?
            })
        } else {
            None
        };
        let submission = state.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        state.retail_screenshot_frame_cache.commit_presented();
        if let Some(token) = pending_main_menu_entry_token.take() {
            crate::app_shell_transition::record_main_menu_entry_presented(state, token)?;
        }
        if let (Some(identity), Some(readback)) = (entry_sequence_identity, pending_entry_sequence)
        {
            shell_capture
                .as_deref_mut()
                .expect("entry-sequence session exists for retained readback")
                .record_entry_sequence_submission(identity, readback, submission.clone())?;
        }
        if let Some(receipt) = pending_main_menu_title_receipt.take() {
            anyhow::ensure!(
                state
                    .main_menu_shell_state
                    .title_reveal
                    .record_presented(receipt),
                "main-menu title receipt was stale at present commit"
            );
        }
        if let Some(pending_capture) = pending_capture {
            let pixels = pending_capture.finish(
                &state.gpu.device,
                submission.clone(),
                capture_timeout.expect("capture timeout exists with pending readback"),
            )?;
            let surface_format = state.gpu.config.format;
            if shell_capture_current_frame {
                shell_capture
                    .as_deref_mut()
                    .expect("shell capture session exists when readback completes")
                    .complete(state, surface_format, &pixels)?;
            } else {
                tactical_capture
                    .as_deref_mut()
                    .expect("tactical capture session exists when readback completes")
                    .complete_after_readback(state, surface_format, &pixels)?;
            }
            event_loop.exit();
        }
        if let Some(pending_screenshot) = pending_retail_screenshot {
            match pending_screenshot.finish(
                &state.gpu.device,
                submission,
                crate::render::screenshot::READBACK_TIMEOUT,
            ) {
                Ok(pixels) => match crate::render::screenshot::write_retail_screenshot(
                    state.gpu.config.width,
                    state.gpu.config.height,
                    state.gpu.config.format,
                    &pixels,
                ) {
                    Ok(path) => log::info!("Saved screenshot {}", path.display()),
                    Err(error) => log::error!("Screenshot write failed: {error:#}"),
                },
                Err(error) => log::error!("Screenshot readback failed: {error}"),
            }
        }

        // Deferred loading: after presenting the Loading screen frame,
        // pump one loading phase. The next patch will continue splitting the
        // remaining legacy load body into smaller phases.
        if matches!(state.screen, GameScreen::Loading) {
            crate::app_loading::loading_screen_presented(state);
            let native_loading = crate::app_loading::is_native_loading_session(state);
            match crate::app_loading::pump_loading_after_present(state) {
                crate::app_loading::LoadingPump::Pending => {
                    state.platform.window.request_redraw();
                }
                crate::app_loading::LoadingPump::Finished(result) => {
                    app_transitions::apply_map_load_result(state, result);
                }
                crate::app_loading::LoadingPump::Failed(err) => {
                    log::warn!("Could not load map: {err:#}");
                    if native_loading {
                        crate::app_loading::clear_loading_state(state);
                        crate::app_loading::clear_match_startup_state(state);
                        state.screen = GameScreen::MissionResult {
                            title: "Loading Failed".to_string(),
                            detail: format!("{err:#}"),
                        };
                    } else {
                        let result = app_transitions::fallback_map_load_result();
                        app_transitions::apply_map_load_result(state, result);
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_menu_intro_precedes_entry_observation() {
        assert_eq!(
            MAIN_MENU_SHELL_PRELUDE,
            [
                ShellFramePreludeStep::MaintainIntro,
                ShellFramePreludeStep::ObserveEntry,
            ]
        );
    }
}
