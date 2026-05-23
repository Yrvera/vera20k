//! Pixel-parity Skirmish shell model and layout.
//!
//! This module owns render-agnostic dialog 0x102 geometry, state, and hit
//! testing. Rendering code consumes the computed rects from the app/render
//! layers; this module does not depend on assets or wgpu.

mod layout;
mod state;

pub use layout::{
    CHOOSE_MAP_LIST_ROW_H, CHOOSE_MAP_MODAL_H, CHOOSE_MAP_MODAL_W, COMBO_ARROW_RESERVE_W,
    COMBO_DROPDOWN_ROW_H, COMBO_DROPDOWN_SCROLLBAR_BUTTON_H, COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H,
    COMBO_DROPDOWN_SCROLLBAR_W, COMBO_FACE_H, ChooseMapModalButton, ChooseMapModalLayout,
    ColorComboId, RIGHT_PANEL_WIDTH, RectPx, ShellControlId, SkirmishCheckboxId,
    SkirmishCheckboxRect, SkirmishColumnLabelRects, SkirmishRightPanelTextRects, SkirmishRowRects,
    SkirmishShellLayout, SkirmishTrackbarId, SkirmishTrackbarRects, checkbox_icon_rect,
    checkbox_text_rect, choose_map_modal_button_at, choose_map_modal_list_row_at, combo_arrow_rect,
    combo_face_rect, combo_swatch_rect, combo_text_rect, compute_choose_map_modal_layout,
    compute_layout, trackbar_active_width, trackbar_pixel_offset, trackbar_plaque_rect,
    trackbar_thumb_rect, trackbar_value_text_rect,
};
pub use state::{
    ChooseMapModalState, ChooseMapSelection, DropdownScrollDragState, DropdownScrollbarPart,
    DropdownScrollbarPressState, OpenComboDropdown, OwnerDrawButton, SkirmishAiRowType,
    SkirmishComboId, SkirmishComboItem, SkirmishCountryChoice, SkirmishShellAction,
    SkirmishShellOpponent, SkirmishShellState, SkirmishShellUiSound, TrackbarDragState,
    action_for_owner_draw_button, apply_action, combo_dropdown_content_rect,
    combo_dropdown_needs_scrollbar, combo_dropdown_open, combo_dropdown_rect,
    combo_dropdown_scroll_thumb_rect, combo_dropdown_scrollbar_rect,
    combo_dropdown_visible_row_count, combo_enabled, combo_items, combo_rect,
    drain_pending_ui_sounds, game_speed_from_visual_position, game_speed_visual_position,
    handle_option_mouse_down, handle_option_mouse_move, handle_option_mouse_up,
    handle_option_mouse_wheel, hit_test, hit_test_owner_draw_button, launch_session,
    launch_settings, selected_combo_item, selected_combo_item_index, trackbar_mouse_allowed_y,
    trackbar_mouse_value, trackbar_thumb_hit, trackbar_visual_value,
};
