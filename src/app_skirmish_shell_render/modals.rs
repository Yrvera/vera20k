//! Modal sprite helpers for the skirmish shell renderer.
//!
//! Covers Choose Map and validation modal instance construction.

use crate::render::batch::SpriteInstance;
use crate::render::shell_paint;
use crate::render::skirmish_shell_chrome::{SkirmishShellChromeAtlas, SkirmishShellChromeEntry};
use crate::skirmish_modes::SkirmishGameMode;
use crate::ui::skirmish_shell::{
    ChooseMapModalButton, ChooseMapModalLayout, RectPx, SkirmishShellState, ValidationModalLayout,
    choose_map_listbox_content_rect, choose_map_listbox_row_rect,
    choose_map_listbox_scroll_thumb_rect, choose_map_listbox_scrollbar_rect,
    choose_map_listbox_visible_row_count,
};

use super::chrome::{
    push_entry_native, push_ownerdraw_two_pixel_bevel_frame, push_rect_outline,
    push_right_panel_button_shp, push_solid_rect,
};
use super::controls::{ControlPaint, paint_control};
use super::{
    OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
    OWNERDRAW_SELECTED_RGB_FROM_DAT_00AC4604_PACKED_000000FF,
    SHELL_DROPDOWN_BG_RGB_PENDING_COMBODROPWIN_SOURCE_CAPTURE, SHELL_DROPDOWN_DEPTH,
    SHELL_MODAL_BG_RGB, SHELL_MODAL_PANEL_RGB, SHELL_PARENT_BACKGROUND_DEPTH,
    SHELL_SCROLLBAR_TRACK_RGB_PENDING_SCROLLBAR_SOURCE_CAPTURE,
};

const VALIDATION_MODAL_SPRITE_DEPTHS: shell_paint::ModalDepths = shell_paint::ModalDepths {
    background: SHELL_DROPDOWN_DEPTH - 0.00014,
    button: SHELL_DROPDOWN_DEPTH - 0.00016,
    text: 0.0,
};

pub(super) fn push_choose_map_listbox_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    list: RectPx,
    row_count: usize,
    top_index: usize,
    selected_index: Option<usize>,
    depth: f32,
) {
    let content = choose_map_listbox_content_rect(row_count, list);
    push_solid_rect(
        out,
        atlas,
        list,
        SHELL_DROPDOWN_BG_RGB_PENDING_COMBODROPWIN_SOURCE_CAPTURE,
        depth,
    );
    if let Some(idx) = selected_index {
        let visible_rows = choose_map_listbox_visible_row_count(list);
        if idx >= top_index && idx < top_index + visible_rows {
            let row = idx - top_index;
            let rect = choose_map_listbox_row_rect(content, row);
            if rect.h > 0 {
                push_solid_rect(
                    out,
                    atlas,
                    rect,
                    OWNERDRAW_SELECTED_RGB_FROM_DAT_00AC4604_PACKED_000000FF,
                    depth - 0.00001,
                );
            }
        }
    }
    if let Some(scrollbar) = choose_map_listbox_scrollbar_rect(row_count, list) {
        if let Some(thumb) = choose_map_listbox_scroll_thumb_rect(row_count, top_index, list) {
            push_solid_rect(
                out,
                atlas,
                scrollbar,
                SHELL_SCROLLBAR_TRACK_RGB_PENDING_SCROLLBAR_SOURCE_CAPTURE,
                depth - 0.000015,
            );
            let chrome = atlas.control_chrome();
            paint_control(
                out,
                &chrome,
                ControlPaint::ScrollBar {
                    scrollbar,
                    thumb,
                    pressed_part: None,
                },
            );
        }
    }
    push_ownerdraw_two_pixel_bevel_frame(out, atlas, list, depth - 0.00002);
}

pub(super) fn choose_map_background_entry(
    atlas: &SkirmishShellChromeAtlas,
    layout: &ChooseMapModalLayout,
) -> Option<SkirmishShellChromeEntry> {
    match layout.screen.w {
        800 => atlas.choose_map_background_800_customize_battle,
        _ => None,
    }
}

pub(super) fn push_choose_map_modal_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &ChooseMapModalLayout,
    shell: &SkirmishShellState,
    modes: &[SkirmishGameMode],
) {
    let Some(modal) = shell.choose_map_modal.as_ref() else {
        return;
    };
    let mode_row_count = modal.mode_row_count(modes);
    let selected_mode_index = modes
        .iter()
        .position(|mode| mode.id == modal.selected_mode_id);
    if let Some(background) = choose_map_background_entry(atlas, layout) {
        push_entry_native(
            out,
            background,
            layout.screen.x,
            layout.screen.y,
            SHELL_PARENT_BACKGROUND_DEPTH,
        );
    }
    push_solid_rect(
        out,
        atlas,
        layout.dialog,
        SHELL_MODAL_BG_RGB,
        SHELL_DROPDOWN_DEPTH - 0.00008,
    );
    push_rect_outline(
        out,
        atlas,
        layout.dialog,
        OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
        SHELL_DROPDOWN_DEPTH - 0.00009,
    );
    push_choose_map_listbox_instances(
        out,
        atlas,
        layout.mode_list,
        mode_row_count,
        modal.mode_top_index,
        selected_mode_index,
        SHELL_DROPDOWN_DEPTH - 0.00010,
    );
    push_choose_map_listbox_instances(
        out,
        atlas,
        layout.map_list,
        modal.filtered_record_indices.len(),
        modal.map_top_index,
        modal.highlighted_filtered_index,
        SHELL_DROPDOWN_DEPTH - 0.00010,
    );
    // The modal's right-column buttons are the same owner-draw type-1 class as the
    // setup shell's Start/Choose/Back: SDBTNANM frame 2 idle, frame 4 pressed. They
    // share the right-panel SDBTNANM cell geometry, so draw them through the same
    // path rather than the gray 3-slice PCX (push_button_30).
    for (button, id) in [
        (layout.use_map_button, ChooseMapModalButton::UseMap0x6c5),
        (layout.cancel_button, ChooseMapModalButton::Cancel0x5c0),
        (
            layout.create_random_map_button,
            ChooseMapModalButton::CreateRandomMap0x583,
        ),
    ] {
        push_right_panel_button_shp(
            out,
            atlas,
            button,
            modal.pressed_button == Some(id),
            false,
            SHELL_DROPDOWN_DEPTH - 0.00011,
        );
    }
    push_rect_outline(
        out,
        atlas,
        layout.preview,
        OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
        SHELL_DROPDOWN_DEPTH - 0.00012,
    );
}

pub(super) fn push_validation_modal_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &ValidationModalLayout,
    pressed: bool,
) {
    let frames = shell_paint::ModalButtonFrames {
        up: atlas.modal_button_mnbttn_frame0,
        disabled: atlas.modal_button_mnbttn_frame1,
        pressed: atlas.modal_button_mnbttn_frame2,
    };
    let button = shell_paint::ModalButton {
        rect: layout.ok_button,
        pressed,
        enabled: true,
    };
    if atlas.validation_modal_background_pudlgbgn.is_none() {
        push_solid_rect(
            out,
            atlas,
            layout.dialog,
            SHELL_MODAL_PANEL_RGB,
            VALIDATION_MODAL_SPRITE_DEPTHS.background,
        );
        push_rect_outline(
            out,
            atlas,
            layout.dialog,
            OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
            SHELL_DROPDOWN_DEPTH - 0.00015,
        );
    }
    out.extend(shell_paint::paint_modal_sprites(
        atlas.validation_modal_background_pudlgbgn,
        frames,
        layout.dialog,
        &[button],
        VALIDATION_MODAL_SPRITE_DEPTHS,
    ));
}
