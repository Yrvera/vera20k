//! Save/load panel — egui overlay for managing save files.
//!
//! Scans the `saves/` directory, reads snapshot headers (without deserializing
//! the full Simulation), and displays a scrollable list. The player can load
//! or delete saves from here.
//!
//! The directory scan is cached — it only runs when the panel first opens or
//! after a save/delete invalidates the cache, not every frame.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on sim/snapshot for header parsing.

use crate::sim::snapshot::{GameSnapshot, GameSnapshotHeader};
use crate::ui::client_theme;

const SAVES_DIR: &str = "saves";
const SAVE_ROW_MAIN_X: f32 = 2.0;
const SAVE_ROW_MAIN_WIDTH: f32 = 249.0;
const SAVE_ROW_DATE_X: f32 = 255.0;
const SAVE_ROW_DATE_WIDTH: f32 = 56.0;
const SAVE_ROW_TIME_X: f32 = 315.0;
const SAVE_ROW_HEIGHT: f32 = 20.0;
const SAVE_ROW_LOAD_WIDTH: f32 = 50.0;
const SAVE_ROW_DELETE_WIDTH: f32 = 20.0;

/// One row in the save file list.
pub(crate) struct SaveEntry {
    /// Absolute path to the .bin file.
    pub path: std::path::PathBuf,
    /// Parsed header metadata.
    pub header: GameSnapshotHeader,
}

/// Cached save-file listing. Stored in `AppState` so the directory is only
/// scanned when explicitly invalidated (panel open, save, delete).
pub(crate) struct SaveListCache {
    pub entries: Vec<SaveEntry>,
    /// When true, the next `draw_save_load_panel` call will rescan before rendering.
    pub dirty: bool,
}

impl SaveListCache {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            dirty: true,
        }
    }

    /// Mark the cache as needing a rescan on next render.
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    /// Rescan if dirty, then clear the flag.
    pub fn refresh_if_dirty(&mut self) {
        if self.dirty {
            self.entries = scan_saves();
            self.dirty = false;
        }
    }
}

/// Action produced by the save/load panel each frame.
pub(crate) enum SaveLoadAction {
    /// Load the save at this path.
    Load(std::path::PathBuf),
    /// Delete the save at this path.
    Delete(std::path::PathBuf),
    /// Close the panel.
    Close,
    /// No action.
    None,
}

/// Scan the saves directory and collect entries with valid headers.
fn scan_saves() -> Vec<SaveEntry> {
    let Ok(dir) = std::fs::read_dir(SAVES_DIR) else {
        return Vec::new();
    };
    let mut entries: Vec<SaveEntry> = Vec::new();
    for item in dir {
        let Ok(item) = item else { continue };
        let path = item.path();
        if path.extension().and_then(|e| e.to_str()) != Some("bin") {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(header) = GameSnapshot::read_header(&bytes) else {
            continue;
        };
        entries.push(SaveEntry { path, header });
    }
    // Most recent first.
    entries.sort_by(|a, b| b.header.save_timestamp.cmp(&a.header.save_timestamp));
    entries
}

/// Format a Unix timestamp with the user's Windows short-date and time formats.
///
/// Retail places these in separate list-view subitems. This compatibility
/// wrapper joins the two localized fields for callers that currently expose a
/// single text slot.
pub(crate) fn format_timestamp(unix_secs: u64) -> String {
    format_timestamp_parts(unix_secs)
        .map(|(date, time)| format!("{date} {time}"))
        .unwrap_or_else(|| format!("timestamp {unix_secs}"))
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct NativeFileTime {
    low: u32,
    high: u32,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct NativeSystemTime {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
}

#[cfg(windows)]
pub(crate) fn format_timestamp_parts(unix_secs: u64) -> Option<(String, String)> {
    const WINDOWS_EPOCH_SECONDS: u64 = 11_644_473_600;
    const TICKS_PER_SECOND: u64 = 10_000_000;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn FileTimeToLocalFileTime(
            source: *const NativeFileTime,
            local: *mut NativeFileTime,
        ) -> i32;
        fn FileTimeToSystemTime(
            file_time: *const NativeFileTime,
            system_time: *mut NativeSystemTime,
        ) -> i32;
    }

    let ticks = unix_secs
        .checked_add(WINDOWS_EPOCH_SECONDS)?
        .checked_mul(TICKS_PER_SECOND)?;
    let source = NativeFileTime {
        low: ticks as u32,
        high: (ticks >> 32) as u32,
    };
    let mut local = NativeFileTime { low: 0, high: 0 };
    let mut system = NativeSystemTime {
        year: 0,
        month: 0,
        day_of_week: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        milliseconds: 0,
    };
    // SAFETY: all pointers refer to live, correctly laid-out Win32 structs.
    unsafe {
        if FileTimeToLocalFileTime(&source, &mut local) == 0
            || FileTimeToSystemTime(&local, &mut system) == 0
        {
            return None;
        }
    }
    format_local_system_time(&system)
}

#[cfg(windows)]
fn format_local_system_time(system: &NativeSystemTime) -> Option<(String, String)> {
    const LOCALE_USER_DEFAULT: u32 = 0x0400;
    const DATE_SHORTDATE: u32 = 1;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetDateFormatA(
            locale: u32,
            flags: u32,
            date: *const NativeSystemTime,
            format: *const u8,
            output: *mut u8,
            output_len: i32,
        ) -> i32;
        fn GetTimeFormatA(
            locale: u32,
            flags: u32,
            time: *const NativeSystemTime,
            format: *const u8,
            output: *mut u8,
            output_len: i32,
        ) -> i32;
    }

    let mut date = [0u8; 128];
    let mut time = [0u8; 128];
    // SAFETY: Win32 receives a valid SYSTEMTIME and fixed 128-byte outputs,
    // exactly matching the native load/save dialog.
    let (date_len, time_len) = unsafe {
        (
            GetDateFormatA(
                LOCALE_USER_DEFAULT,
                DATE_SHORTDATE,
                system,
                std::ptr::null(),
                date.as_mut_ptr(),
                date.len() as i32,
            ),
            GetTimeFormatA(
                LOCALE_USER_DEFAULT,
                0,
                system,
                std::ptr::null(),
                time.as_mut_ptr(),
                time.len() as i32,
            ),
        )
    };
    if date_len <= 0 || time_len <= 0 {
        return None;
    }
    let date_payload = &date[..date_len.saturating_sub(1) as usize];
    let time_payload = &time[..time_len.saturating_sub(1) as usize];
    Some((
        crate::util::native_string::acp_decode(date_payload),
        crate::util::native_string::acp_decode(time_payload),
    ))
}

#[cfg(not(windows))]
pub(crate) fn format_timestamp_parts(_unix_secs: u64) -> Option<(String, String)> {
    None
}

/// Draw the save/load panel. Returns an action for the caller to execute.
///
/// The caller must pass `&mut SaveListCache` so the panel can refresh once
/// on open rather than scanning the filesystem every frame.
pub(crate) fn draw_save_load_panel(
    ctx: &egui::Context,
    cache: &mut SaveListCache,
) -> SaveLoadAction {
    cache.refresh_if_dirty();

    let palette = client_theme::apply_client_theme(ctx);
    let mut action = SaveLoadAction::None;

    // Semi-transparent backdrop.
    egui::Area::new("saveload_backdrop".into())
        .fixed_pos(egui::pos2(0.0, 0.0))
        .interactable(false)
        .show(ctx, |ui| {
            let screen = ctx.content_rect();
            ui.painter().rect_filled(
                screen,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 120),
            );
        });

    egui::Window::new("")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(client_theme::card_frame(palette.panel, palette.line))
        .min_width(500.0)
        .max_height(500.0)
        .show(ctx, |ui| {
            ui.set_max_width(500.0);
            ui.vertical(|ui| {
                client_theme::section_label(ui, "SAVES", palette);
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Save / Load")
                        .size(28.0)
                        .strong()
                        .color(palette.text),
                );
                ui.label(
                    egui::RichText::new("Press M to quicksave, or click a row to load.")
                        .size(13.0)
                        .color(palette.text_muted),
                );

                ui.add_space(12.0);

                if cache.entries.is_empty() {
                    ui.add_space(20.0);
                    ui.label(
                        egui::RichText::new("No saves found. Press M to create one.")
                            .size(14.0)
                            .color(palette.text_muted),
                    );
                    ui.add_space(20.0);
                } else {
                    // Header row.
                    ui.horizontal(|ui| {
                        let spacing = ui.spacing().item_spacing.x;
                        let columns_width = (ui.available_width()
                            - SAVE_ROW_LOAD_WIDTH
                            - SAVE_ROW_DELETE_WIDTH
                            - spacing * 2.0)
                            .max(SAVE_ROW_TIME_X);
                        let (columns, _) = ui.allocate_exact_size(
                            egui::vec2(columns_width, SAVE_ROW_HEIGHT),
                            egui::Sense::hover(),
                        );
                        let origin = columns.left_top();
                        ui.put(
                            egui::Rect::from_min_size(
                                egui::pos2(origin.x + SAVE_ROW_MAIN_X, origin.y),
                                egui::vec2(SAVE_ROW_MAIN_WIDTH, SAVE_ROW_HEIGHT),
                            ),
                            egui::Label::new(
                                egui::RichText::new("Map")
                                    .size(12.0)
                                    .strong()
                                    .color(palette.text_muted),
                            ),
                        );
                        ui.put(
                            egui::Rect::from_min_size(
                                egui::pos2(origin.x + SAVE_ROW_DATE_X, origin.y),
                                egui::vec2(SAVE_ROW_DATE_WIDTH, SAVE_ROW_HEIGHT),
                            ),
                            egui::Label::new(
                                egui::RichText::new("Date")
                                    .size(12.0)
                                    .strong()
                                    .color(palette.text_muted),
                            ),
                        );
                        ui.put(
                            egui::Rect::from_min_max(
                                egui::pos2(origin.x + SAVE_ROW_TIME_X, origin.y),
                                egui::pos2(columns.right(), origin.y + SAVE_ROW_HEIGHT),
                            ),
                            egui::Label::new(
                                egui::RichText::new("Time")
                                    .size(12.0)
                                    .strong()
                                    .color(palette.text_muted),
                            ),
                        );
                    });
                    ui.add_space(4.0);
                    ui.separator();

                    // Scrollable list of saves.
                    egui::ScrollArea::vertical()
                        .max_height(350.0)
                        .show(ui, |ui| {
                            for entry in &cache.entries {
                                let row_id = egui::Id::new(&entry.path);
                                let resp = ui
                                    .push_id(row_id, |ui| {
                                        ui.horizontal(|ui| {
                                            let spacing = ui.spacing().item_spacing.x;
                                            let columns_width = (ui.available_width()
                                                - SAVE_ROW_LOAD_WIDTH
                                                - SAVE_ROW_DELETE_WIDTH
                                                - spacing * 2.0)
                                                .max(SAVE_ROW_TIME_X);
                                            let (columns, _) = ui.allocate_exact_size(
                                                egui::vec2(columns_width, SAVE_ROW_HEIGHT),
                                                egui::Sense::hover(),
                                            );
                                            let origin = columns.left_top();

                                            // Native main-text column.
                                            let map_label = if entry.header.map_name.len() > 18 {
                                                format!("{}...", &entry.header.map_name[..18])
                                            } else {
                                                entry.header.map_name.clone()
                                            };
                                            ui.put(
                                                egui::Rect::from_min_size(
                                                    egui::pos2(
                                                        origin.x + SAVE_ROW_MAIN_X,
                                                        origin.y,
                                                    ),
                                                    egui::vec2(
                                                        SAVE_ROW_MAIN_WIDTH,
                                                        SAVE_ROW_HEIGHT,
                                                    ),
                                                ),
                                                egui::Label::new(
                                                    egui::RichText::new(map_label)
                                                        .size(13.0)
                                                        .color(palette.text),
                                                ),
                                            );

                                            // Native short-date and time occupy
                                            // separate owner-draw list columns.
                                            let (date, time) =
                                                format_timestamp_parts(entry.header.save_timestamp)
                                                    .unwrap_or_else(|| {
                                                        (
                                                            format!(
                                                                "timestamp {}",
                                                                entry.header.save_timestamp
                                                            ),
                                                            String::new(),
                                                        )
                                                    });
                                            ui.put(
                                                egui::Rect::from_min_size(
                                                    egui::pos2(
                                                        origin.x + SAVE_ROW_DATE_X,
                                                        origin.y,
                                                    ),
                                                    egui::vec2(
                                                        SAVE_ROW_DATE_WIDTH,
                                                        SAVE_ROW_HEIGHT,
                                                    ),
                                                ),
                                                egui::Label::new(
                                                    egui::RichText::new(date)
                                                        .size(13.0)
                                                        .color(palette.text_muted),
                                                ),
                                            );
                                            ui.put(
                                                egui::Rect::from_min_max(
                                                    egui::pos2(
                                                        origin.x + SAVE_ROW_TIME_X,
                                                        origin.y,
                                                    ),
                                                    egui::pos2(
                                                        columns.right(),
                                                        origin.y + SAVE_ROW_HEIGHT,
                                                    ),
                                                ),
                                                egui::Label::new(
                                                    egui::RichText::new(time)
                                                        .size(13.0)
                                                        .color(palette.text_muted),
                                                ),
                                            );

                                            // Load button.
                                            if ui
                                                .add_sized(
                                                    egui::vec2(SAVE_ROW_LOAD_WIDTH, 22.0),
                                                    egui::Button::new(
                                                        egui::RichText::new("Load")
                                                            .size(12.0)
                                                            .color(palette.accent),
                                                    ),
                                                )
                                                .clicked()
                                            {
                                                action = SaveLoadAction::Load(entry.path.clone());
                                            }

                                            // Delete button.
                                            if ui
                                                .add_sized(
                                                    egui::vec2(SAVE_ROW_DELETE_WIDTH, 22.0),
                                                    egui::Button::new(
                                                        egui::RichText::new("X")
                                                            .size(12.0)
                                                            .color(palette.danger),
                                                    ),
                                                )
                                                .clicked()
                                            {
                                                action = SaveLoadAction::Delete(entry.path.clone());
                                            }
                                        });
                                    })
                                    .response;
                                // Subtle hover highlight.
                                if resp.hovered() {
                                    ui.painter().rect_filled(
                                        resp.rect,
                                        2.0,
                                        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 10),
                                    );
                                }
                            }
                        });
                }

                ui.add_space(12.0);
                if ui
                    .add_sized(
                        egui::vec2(160.0, 36.0),
                        egui::Button::new(egui::RichText::new("Close").size(16.0).strong()),
                    )
                    .clicked()
                {
                    action = SaveLoadAction::Close;
                }
                ui.add_space(4.0);
            });
        });

    action
}

#[cfg(all(test, windows))]
mod tests {
    use super::{NativeSystemTime, format_local_system_time};

    #[test]
    fn fixed_system_time_uses_platform_short_date_and_time() {
        let fixed = NativeSystemTime {
            year: 2026,
            month: 7,
            day_of_week: 4,
            day: 30,
            hour: 13,
            minute: 45,
            second: 12,
            milliseconds: 0,
        };
        let (date, time) = format_local_system_time(&fixed).expect("Win32 locale formatting");
        assert!(!date.is_empty());
        assert!(!time.is_empty());
        assert!(!date.contains("ago"));
        assert!(!time.contains("ago"));
    }
}
