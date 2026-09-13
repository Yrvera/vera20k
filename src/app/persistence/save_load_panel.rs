//! Save/load panel — egui overlay for managing save files.
//!
//! Displays the persistence domain's cached snapshot-header list. The player
//! can load or delete saves from here.
//!
//! The repository listing is cached — it only refreshes when the panel first
//! opens or after a save/delete invalidates the cache, not every frame.
//!
//! ## Dependency rules
//! - Part of the app layer — may depend on sim/snapshot for header parsing.

use crate::app::persistence::SaveEntry;
use crate::sim::snapshot::GameSnapshotHeader;
use crate::ui::client_theme;

const SAVE_ROW_MAIN_X: f32 = 2.0;
const SAVE_ROW_MAIN_WIDTH: f32 = 249.0;
const SAVE_ROW_DATE_X: f32 = 255.0;
const SAVE_ROW_DATE_WIDTH: f32 = 56.0;
const SAVE_ROW_TIME_X: f32 = 315.0;
const SAVE_ROW_HEIGHT: f32 = 20.0;
const SAVE_ROW_LOAD_WIDTH: f32 = 50.0;
const SAVE_ROW_DELETE_WIDTH: f32 = 20.0;

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

#[cfg(test)]
mod gsi_17_02_tests {
    use super::save_row_main_text;
    use crate::app::persistence::SaveRepository;
    use crate::sim::snapshot::GameSnapshot;
    use crate::sim::world::Simulation;

    #[test]
    fn gsi_17_02_description_survives_filename_change_and_drives_list_text() {
        let test_dir = std::env::temp_dir().join(format!(
            "vera20k-gsi-17-02-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time after Unix epoch")
                .as_nanos()
        ));
        let mut sim = Simulation::new();
        sim.session.map_name = "OFFICIAL.MAP".to_string();
        let bytes = GameSnapshot::save_validated(&sim, 1, 2, "Northern ridge", 3);
        let repository = SaveRepository::at(&test_dir);
        let renamed_path = repository
            .write_named("completely_different_name.bin", &bytes)
            .expect("write save fixture");

        let entries = repository.panel_entries_by_embedded_time();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, renamed_path);
        assert_eq!(entries[0].header.description, "Northern ridge");
        assert_eq!(save_row_main_text(&entries[0].header), "Northern ridge");

        std::fs::remove_dir_all(&test_dir).expect("remove isolated save-list fixture");
    }

    #[test]
    fn gsi_17_02_empty_description_falls_back_to_unicode_safe_map_name() {
        let mut sim = Simulation::new();
        sim.session.map_name = "地图地图地图地图地图地图地图地图地图地图".to_string();
        let bytes = GameSnapshot::save_validated(&sim, 1, 2, "", 3);
        let header = GameSnapshot::read_header(&bytes).expect("current header");

        assert_eq!(
            save_row_main_text(&header),
            "地图地图地图地图地图地图地图地图地图..."
        );
    }
}

fn save_row_main_text(header: &GameSnapshotHeader) -> String {
    const MAX_VISIBLE_CHARS: usize = 18;

    let text = if header.description.is_empty() {
        &header.map_name
    } else {
        &header.description
    };
    let mut chars = text.chars();
    let prefix: String = chars.by_ref().take(MAX_VISIBLE_CHARS).collect();
    if chars.next().is_some() {
        format!("{prefix}...")
    } else {
        prefix
    }
}

/// Format a Unix timestamp with the user's Windows short-date and time formats.
///
/// Retail places these in separate list-view subitems. This compatibility
/// wrapper joins the two localized fields for callers that currently expose a
/// single text slot.
pub(crate) fn format_timestamp(unix_secs: u64) -> String {
    crate::util::native_file_time::format_timestamp_parts(unix_secs)
        .map(|(date, time)| format!("{date} {time}"))
        .unwrap_or_else(|| format!("timestamp {unix_secs}"))
}

/// Draw the save/load panel. Returns an action for the caller to execute.
///
/// The caller passes the persistence domain's current save-list view.
pub(crate) fn draw_save_load_panel(
    ctx: &egui::Context,
    entries: &[SaveEntry],
) -> SaveLoadAction {
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

                if entries.is_empty() {
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
                                egui::RichText::new("Description")
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
                            for entry in entries {
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
                                            let description = save_row_main_text(&entry.header);
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
                                                    egui::RichText::new(description)
                                                        .size(13.0)
                                                        .color(palette.text),
                                                ),
                                            );

                                            // Native short-date and time occupy
                                            // separate owner-draw list columns.
                                            let (date, time) =
                                                crate::util::native_file_time::format_timestamp_parts(entry.header.save_timestamp)
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
