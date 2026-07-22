//! Modal sprite helpers for the skirmish shell renderer.
//!
//! Covers Choose Map and validation modal instance construction.

use crate::render::batch::SpriteInstance;
use crate::render::shell_paint;
use crate::render::skirmish_shell_chrome::{SkirmishShellChromeAtlas, SkirmishShellChromeEntry};
use crate::skirmish_modes::SkirmishGameMode;
use crate::ui::skirmish_shell::{
    COMBO_DROPDOWN_ROW_H, COMBO_FACE_H, ChooseMapModalButton, ChooseMapModalLayout,
    RandomMapSetupControl, RandomMapSetupLayout, RandomMapSetupModalState, RectPx,
    SETUP_COMBO_ROWS, SavedSeedBrowserState, SavedSeedControl, SavedSeedLayout, SavedSeedMode,
    SkirmishShellState, ValidationModalLayout, choose_map_listbox_content_rect,
    choose_map_listbox_row_rect, choose_map_listbox_scroll_thumb_rect,
    choose_map_listbox_scrollbar_rect, choose_map_listbox_visible_row_count,
    random_map_setup_dropdown_rect, setup_combo_items, trackbar_pixel_offset,
};

use super::chrome::{
    push_button_30, push_entry_native, push_ownerdraw_two_pixel_bevel_frame, push_rect_outline,
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

/// The five combo rows as hit-test controls, for the enabled/disabled lookup.
const SETUP_COMBO_CONTROLS: [RandomMapSetupControl; 5] = [
    RandomMapSetupControl::MapType0x405,
    RandomMapSetupControl::Time0x3ea,
    RandomMapSetupControl::Theater0x407,
    RandomMapSetupControl::Size0x406,
    RandomMapSetupControl::Resources0x408,
];
const PLAYERS_ROW: usize = 5;
/// Player-count trackbar bounds, matching the option normalizer's clamp.
const PLAYERS_MIN: i32 = 2;
const PLAYERS_MAX: i32 = 8;

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

/// Background for the random-map setup modal. Same asset and same 800-wide-only
/// gate as the choose-map modal — the two dialogs share one background surface.
pub(super) fn random_map_setup_background_entry(
    atlas: &SkirmishShellChromeAtlas,
    layout: &RandomMapSetupLayout,
) -> Option<SkirmishShellChromeEntry> {
    match layout.screen.w {
        800 => atlas.choose_map_background_800_customize_battle,
        _ => None,
    }
}

/// Build the sprite instances for the random-map setup modal `0x105`.
///
/// Frame, background and right column are the choose-map chrome; only the left
/// column differs. The preview box is drawn as an empty outline: rendering the
/// generated terrain into it is a deferred follow-up.
pub(super) fn push_random_map_setup_modal_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &RandomMapSetupLayout,
    modal: &RandomMapSetupModalState,
) {
    if let Some(background) = random_map_setup_background_entry(atlas, layout) {
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

    let chrome = atlas.control_chrome();
    // Rows 0..4 are combos; row 5 is the players trackbar. A collapsed combo
    // occupies only its face, not the dropdown extent the resource reserves.
    for (row, control) in SETUP_COMBO_ROWS.iter().enumerate() {
        let rect = layout.control_rects[row];
        let face = RectPx::new(rect.x, rect.y, rect.w, COMBO_FACE_H);
        // ControlPaint::Combo emits only the swatch and the arrow -- the shell's
        // combo faces come from slots baked into the 0x102 background art, which
        // has none at these positions. Lay a plate down first or the control is
        // an arrow floating on bare background.
        push_solid_rect(
            out,
            atlas,
            face,
            SHELL_MODAL_PANEL_RGB,
            SHELL_DROPDOWN_DEPTH - 0.00009,
        );
        push_ownerdraw_two_pixel_bevel_frame(out, atlas, face, SHELL_DROPDOWN_DEPTH - 0.000095);
        paint_control(
            out,
            &chrome,
            ControlPaint::Combo {
                rect: face,
                swatch: None,
                open: modal.open_combo == Some(*control),
                disabled: !modal.is_enabled(SETUP_COMBO_CONTROLS[row]),
            },
        );
    }
    let players = layout.control_rects[PLAYERS_ROW];
    paint_control(
        out,
        &chrome,
        ControlPaint::Trackbar {
            rect: players,
            thumb_px: trackbar_pixel_offset(
                modal.options.num_players,
                PLAYERS_MIN,
                PLAYERS_MAX,
                1,
                players,
            ),
        },
    );

    // Surprise Me and Generate are owner-draw type 3 for this dialog, so they
    // take the MNBTTN modal button art rather than the generic PCX slices --
    // the same art the message-box modals use. Native size, centred on the
    // control rect.
    let modal_button_frames = shell_paint::ModalButtonFrames {
        up: atlas.modal_button_mnbttn_frame0,
        disabled: atlas.modal_button_mnbttn_frame1,
        pressed: atlas.modal_button_mnbttn_frame2,
    };
    let action_buttons: Vec<shell_paint::ModalButton> = [
        (layout.randomize, RandomMapSetupControl::Randomize0x621),
        (layout.generate, RandomMapSetupControl::Generate0x620),
    ]
    .into_iter()
    .map(|(rect, control)| shell_paint::ModalButton {
        rect,
        pressed: modal.pressed_control == Some(control),
        enabled: modal.is_enabled(control),
    })
    .collect();
    if modal_button_frames.up.is_some() {
        out.extend(shell_paint::paint_modal_sprites(
            None,
            modal_button_frames,
            layout.dialog,
            &action_buttons,
            shell_paint::ModalDepths {
                background: SHELL_DROPDOWN_DEPTH - 0.00011,
                button: SHELL_DROPDOWN_DEPTH - 0.00011,
                text: 0.0,
            },
        ));
    } else {
        // MNBTTN.SHP missing from the install: fall back to the generic slices
        // rather than drawing nothing where a button belongs.
        for button in &action_buttons {
            push_button_30(
                out,
                atlas,
                button.rect,
                button.pressed,
                !button.enabled,
                SHELL_DROPDOWN_DEPTH - 0.00011,
            );
        }
    }

    for (rect, control) in [
        (layout.use_map, RandomMapSetupControl::Ok0x6c5),
        (layout.load, RandomMapSetupControl::Load0x6c2),
        (layout.save, RandomMapSetupControl::Save0x6c3),
        (layout.delete, RandomMapSetupControl::Delete0x6c4),
        (layout.cancel, RandomMapSetupControl::Cancel0x5c0),
    ] {
        push_right_panel_button_shp(
            out,
            atlas,
            rect,
            modal.pressed_control == Some(control),
            !modal.is_enabled(control),
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

    // The progress widgets are hidden in the resource and shown only while the
    // synchronous generate block owns the dialog.
    if modal.generating {
        push_solid_rect(
            out,
            atlas,
            layout.progress_bar,
            SHELL_MODAL_PANEL_RGB,
            SHELL_DROPDOWN_DEPTH - 0.00013,
        );
        push_rect_outline(
            out,
            atlas,
            layout.progress_bar,
            OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
            SHELL_DROPDOWN_DEPTH - 0.00014,
        );
    }

    // An open list paints last so it covers the rows beneath it.
    if let Some(combo) = modal.open_combo {
        let row = combo.row();
        let items = setup_combo_items(combo);
        let list = random_map_setup_dropdown_rect(layout, row, items.len());
        push_solid_rect(
            out,
            atlas,
            list,
            SHELL_DROPDOWN_BG_RGB_PENDING_COMBODROPWIN_SOURCE_CAPTURE,
            SHELL_DROPDOWN_DEPTH - 0.00015,
        );
        push_rect_outline(
            out,
            atlas,
            list,
            OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
            SHELL_DROPDOWN_DEPTH - 0.00016,
        );
        if let Some(selected) = modal.selected_item_index(combo) {
            push_solid_rect(
                out,
                atlas,
                RectPx::new(
                    list.x,
                    list.y + COMBO_DROPDOWN_ROW_H * selected as i32,
                    list.w,
                    COMBO_DROPDOWN_ROW_H,
                ),
                OWNERDRAW_SELECTED_RGB_FROM_DAT_00AC4604_PACKED_000000FF,
                SHELL_DROPDOWN_DEPTH - 0.00017,
            );
        }
    }
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

/// Build the sprite instances for the saved-seed browser.
///
/// It replaces the setup dialog on screen, so it redraws the same background
/// and right-column chrome rather than layering over it.
pub(super) fn push_saved_seed_modal_instances(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    layout: &SavedSeedLayout,
    browser: &SavedSeedBrowserState,
) {
    if layout.screen.w == 800 {
        if let Some(background) = atlas.choose_map_background_800_customize_battle {
            push_entry_native(
                out,
                background,
                layout.screen.x,
                layout.screen.y,
                SHELL_PARENT_BACKGROUND_DEPTH,
            );
        }
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
        layout.list,
        browser.entries.len(),
        browser.top_index,
        browser.selected,
        SHELL_DROPDOWN_DEPTH - 0.00010,
    );
    // The name field is a plain sunken plate; Save is the only mode that has one.
    if let Some(edit) = layout.name_edit {
        push_solid_rect(
            out,
            atlas,
            edit,
            SHELL_MODAL_PANEL_RGB,
            SHELL_DROPDOWN_DEPTH - 0.00010,
        );
        push_ownerdraw_two_pixel_bevel_frame(out, atlas, edit, SHELL_DROPDOWN_DEPTH - 0.00011);
    }
    for (rect, control) in [
        (layout.action, SavedSeedControl::Action),
        (layout.back, SavedSeedControl::Back0x686),
    ] {
        let disabled = control == SavedSeedControl::Action && !browser.action_enabled();
        push_right_panel_button_shp(
            out,
            atlas,
            rect,
            browser.pressed_control == Some(control),
            disabled,
            SHELL_DROPDOWN_DEPTH - 0.00012,
        );
    }
}

/// The saved-seed browser's mode drives only its captions, so the renderer
/// takes it separately from the layout it already carries.
pub(super) const fn saved_seed_mode_of(browser: &SavedSeedBrowserState) -> SavedSeedMode {
    browser.mode
}
