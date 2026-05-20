//! Developer overlay panel — runtime knobs and diagnostic readouts.
//!
//! Toggled with backtick (`). Pure egui rendering: data-in / action-out.
//! Caller (app.rs) snapshots state into DevOverlayInfo, draws, and
//! dispatches the returned DevOverlayAction.
//!
//! ## Dependency rules
//! - Part of the app layer — takes pure data in, returns actions out.
//! - No direct AppState dependency in this module (mirrors ui/pause_menu.rs).

use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::app_debug_panel::debug_panel_frame;

/// Number of frames to average for the FPS / frame-time readout.
const FRAME_TIMER_WINDOW: usize = 60;

/// Speed slider hard bounds. Lower bound prevents throttle-math stalls;
/// upper bound is a sane dev maximum (already faster than gamemd allows).
const SPEED_MIN_TPS: u32 = 1;
const SPEED_MAX_TPS: u32 = 200;

/// One row in the inline recent-saves list. Caller builds these from
/// `save_list_cache.entries`. Owned strings so the panel doesn't borrow
/// the cache across the draw call.
pub(crate) struct RecentSaveRow {
    pub path: PathBuf,
    pub display_name: String,
    pub tick: u64,
    pub age_str: String,
}

/// Snapshot of app state passed into `draw_dev_overlay`.
///
/// The text-field buffer is borrowed mutably so egui can edit it in place.
pub(crate) struct DevOverlayInfo<'a> {
    pub sim_speed_tps: u32,
    pub paused: bool,
    pub music_volume: f64,
    pub sfx_volume: f64,
    pub show_pathgrid: bool,
    pub show_cell_grid: bool,
    pub show_heightmap: bool,
    pub show_unit_inspector: bool,
    pub reveal_map: bool,
    pub fps: f32,
    pub frame_ms: f32,
    pub tick_budget_ms: f32,
    pub entity_count: usize,
    pub save_name_buf: &'a mut String,
    pub last_save_tick: Option<u64>,
    pub last_save_age: Option<String>,
    pub last_load_available: bool,
    pub last_load_display: Option<String>,
    pub recent_saves: Vec<RecentSaveRow>,
}

/// Actions produced by the dev overlay each frame.
#[derive(Debug, Clone)]
pub(crate) enum DevOverlayAction {
    None,
    SetGameSpeed(u32),
    SetMusicVolume(f64),
    SetSfxVolume(f64),
    TogglePause,
    StepOneTick,
    TogglePathGrid,
    ToggleCellGrid,
    ToggleHeightmap,
    ToggleUnitInspector,
    ToggleRevealMap,
    ResetGameSpeed,
    SaveAs,
    ReloadLastLoad,
    LoadSave(PathBuf),
}

/// Rolling FPS / frame-time tracker. Sampled once per `render_frame`.
pub(crate) struct FrameTimer {
    samples: VecDeque<Duration>,
    last_tick: Option<Instant>,
}

impl FrameTimer {
    pub(crate) fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(FRAME_TIMER_WINDOW),
            last_tick: None,
        }
    }

    /// Record one frame boundary. Call from the top of `render_frame`.
    pub(crate) fn sample(&mut self, now: Instant) {
        if let Some(prev) = self.last_tick {
            let dt = now - prev;
            if self.samples.len() == FRAME_TIMER_WINDOW {
                self.samples.pop_front();
            }
            self.samples.push_back(dt);
        }
        self.last_tick = Some(now);
    }

    /// Mean frame time in milliseconds over the current window, or 0
    /// if no samples have been recorded yet.
    pub(crate) fn frame_ms_mean(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let total_ns: u128 = self.samples.iter().map(|d| d.as_nanos()).sum();
        let mean_ns: u128 = total_ns / self.samples.len() as u128;
        (mean_ns as f64 / 1_000_000.0) as f32
    }

    /// FPS derived from the mean frame time, or 0 if no samples.
    pub(crate) fn fps(&self) -> f32 {
        let ms = self.frame_ms_mean();
        if ms <= 0.0 { 0.0 } else { 1000.0 / ms }
    }
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new()
    }
}

/// Draw the dev overlay. Returns the chosen action, if any.
pub(crate) fn draw_dev_overlay(
    ctx: &egui::Context,
    info: &mut DevOverlayInfo<'_>,
) -> DevOverlayAction {
    let mut action = DevOverlayAction::None;

    egui::Window::new("Developer Overlay (`)")
        .default_pos([ctx.content_rect().max.x - 340.0, 200.0])
        .default_width(320.0)
        .frame(debug_panel_frame())
        .collapsible(true)
        .resizable(true)
        .show(ctx, |ui| {
            ui.visuals_mut().override_text_color = Some(egui::Color32::from_rgb(30, 30, 30));

            // Sim
            ui.label(egui::RichText::new("Sim").strong());
            ui.horizontal(|ui| {
                let mut tps = info.sim_speed_tps;
                let resp = ui.add(
                    egui::Slider::new(&mut tps, SPEED_MIN_TPS..=SPEED_MAX_TPS)
                        .text("tps")
                        .integer(),
                );
                if resp.changed() && tps != info.sim_speed_tps {
                    action = DevOverlayAction::SetGameSpeed(tps);
                }
                if ui.button("Reset").clicked() {
                    action = DevOverlayAction::ResetGameSpeed;
                }
            });
            ui.horizontal(|ui| {
                let pause_label = if info.paused { "Resume" } else { "Pause" };
                if ui.button(pause_label).clicked() {
                    action = DevOverlayAction::TogglePause;
                }
                if ui
                    .add_enabled(info.paused, egui::Button::new("Step 1 tick"))
                    .clicked()
                {
                    action = DevOverlayAction::StepOneTick;
                }
                ui.label(format!("paused={}", if info.paused { "ON" } else { "OFF" }));
            });
            ui.label(format!(
                "Tick budget: {:.2} ms  ({} tps)",
                info.tick_budget_ms, info.sim_speed_tps
            ));
            ui.label(format!("Entities: {}", info.entity_count));

            ui.separator();

            // Render
            ui.label(egui::RichText::new("Render").strong());
            ui.label(format!(
                "FPS: {:.1}   Frame: {:.2} ms",
                info.fps, info.frame_ms
            ));

            ui.separator();

            // Audio
            ui.label(egui::RichText::new("Audio").strong());
            let mut music = info.music_volume as f32;
            if ui
                .add(egui::Slider::new(&mut music, 0.0..=1.0).text("Music"))
                .changed()
            {
                action = DevOverlayAction::SetMusicVolume(music as f64);
            }
            let mut sfx = info.sfx_volume as f32;
            if ui
                .add(egui::Slider::new(&mut sfx, 0.0..=1.0).text("SFX"))
                .changed()
            {
                action = DevOverlayAction::SetSfxVolume(sfx as f64);
            }

            ui.separator();

            // Debug Overlays
            ui.label(egui::RichText::new("Debug Overlays").strong());
            let mut b = info.show_pathgrid;
            if ui.checkbox(&mut b, "PathGrid (F9/P)").changed() {
                action = DevOverlayAction::TogglePathGrid;
            }
            let mut b = info.show_cell_grid;
            if ui.checkbox(&mut b, "Cell grid (L)").changed() {
                action = DevOverlayAction::ToggleCellGrid;
            }
            let mut b = info.show_heightmap;
            if ui.checkbox(&mut b, "Heightmap (K)").changed() {
                action = DevOverlayAction::ToggleHeightmap;
            }
            let mut b = info.show_unit_inspector;
            if ui.checkbox(&mut b, "Unit inspector (X)").changed() {
                action = DevOverlayAction::ToggleUnitInspector;
            }
            let mut b = info.reveal_map;
            if ui.checkbox(&mut b, "Reveal map (F10/V)").changed() {
                action = DevOverlayAction::ToggleRevealMap;
            }

            ui.separator();

            // Save / Load
            ui.label(egui::RichText::new("Save / Load").strong());
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.add(
                    egui::TextEdit::singleline(info.save_name_buf)
                        .desired_width(180.0)
                        .hint_text("save name"),
                );
                let can_save = !info.save_name_buf.trim().is_empty();
                if ui
                    .add_enabled(can_save, egui::Button::new("Save As"))
                    .clicked()
                {
                    action = DevOverlayAction::SaveAs;
                }
            });

            // Recent saves (top 5).
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Recent:").italics());
            if info.recent_saves.is_empty() {
                ui.label(
                    egui::RichText::new("(no saves)")
                        .italics()
                        .color(egui::Color32::from_rgb(140, 140, 140)),
                );
            } else {
                for row in &info.recent_saves {
                    ui.horizontal(|ui| {
                        if ui.button("Load").clicked() {
                            action = DevOverlayAction::LoadSave(row.path.clone());
                        }
                        ui.label(format!(
                            "{}  tick {}  {}",
                            row.display_name, row.tick, row.age_str
                        ));
                    });
                }
            }

            ui.add_space(4.0);
            let reload_label = match &info.last_load_display {
                Some(name) => format!("Reload last load: {name}"),
                None => "Reload last load".to_string(),
            };
            if ui
                .add_enabled(info.last_load_available, egui::Button::new(reload_label))
                .clicked()
            {
                action = DevOverlayAction::ReloadLastLoad;
            }

            // Last-save readout.
            match (info.last_save_tick, &info.last_save_age) {
                (Some(tick), Some(age)) => {
                    ui.label(format!("Last save: tick {tick} ({age})"));
                }
                (Some(tick), None) => {
                    ui.label(format!("Last save: tick {tick}"));
                }
                _ => {
                    ui.label(
                        egui::RichText::new("Last save: (none this session)")
                            .italics()
                            .color(egui::Color32::from_rgb(140, 140, 140)),
                    );
                }
            }
        });

    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_timer_empty_returns_zero() {
        let t = FrameTimer::new();
        assert_eq!(t.frame_ms_mean(), 0.0);
        assert_eq!(t.fps(), 0.0);
    }

    #[test]
    fn frame_timer_single_sample_is_still_zero() {
        // First sample establishes the baseline; no delta yet.
        let mut t = FrameTimer::new();
        t.sample(Instant::now());
        assert_eq!(t.frame_ms_mean(), 0.0);
    }

    #[test]
    fn frame_timer_two_samples_record_one_delta() {
        let mut t = FrameTimer::new();
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_millis(16);
        t.sample(t0);
        t.sample(t1);
        let mean = t.frame_ms_mean();
        assert!((mean - 16.0).abs() < 0.5, "expected ~16ms, got {mean}");
        let fps = t.fps();
        assert!((fps - 62.5).abs() < 5.0, "expected ~62.5 fps, got {fps}");
    }

    #[test]
    fn frame_timer_window_caps_at_60() {
        let mut t = FrameTimer::new();
        let t0 = Instant::now();
        for i in 0..200 {
            t.sample(t0 + Duration::from_millis(16 * i));
        }
        assert_eq!(t.samples.len(), FRAME_TIMER_WINDOW);
    }
}
