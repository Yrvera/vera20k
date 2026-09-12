//! Common618D40 list backing, selection and61C690 scrollbar paint.
use super::chrome::*;
use super::*;
use crate::ui::shell::list::{ListScrollPart, ShellListGeometry};

pub(super) fn paint_list(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    geometry: ShellListGeometry,
    top: usize,
    selected: Option<usize>,
    pressed_part: Option<ListScrollPart>,
    solid_fill: bool,
) {
    let depth = SHELL_DROPDOWN_DEPTH - 0.00010;
    if solid_fill {
        push_solid_rect(out, atlas, geometry.content, SHELL_MODAL_PANEL_RGB, depth);
    }
    if let Some(selected) = selected.filter(|i| *i >= top && *i < top + geometry.visible_rows) {
        push_solid_rect(
            out,
            atlas,
            geometry.row(selected - top),
            OWNERDRAW_SELECTED_RGB_FROM_DAT_00AC4604_PACKED_000000FF,
            depth - 0.00001,
        );
    }
    push_rect_outline(
        out,
        atlas,
        geometry.outer,
        OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
        depth - 0.00002,
    );
    if let (Some(bar), Some(thumb)) = (geometry.scrollbar, geometry.thumb) {
        let chrome = atlas.control_chrome();
        let inner = RectPx::new(bar.x + 1, bar.y + 1, 18, bar.h - 2);
        push_solid_rect(
            out,
            atlas,
            inner,
            SHELL_SCROLLBAR_TRACK_RGB_PENDING_SCROLLBAR_SOURCE_CAPTURE,
            depth - 0.00002,
        );
        for (up, y, released, pressed) in [
            (
                true,
                inner.y,
                chrome.scrollbar_arrow_up_released,
                chrome.scrollbar_arrow_up_pressed,
            ),
            (
                false,
                inner.y + inner.h - 22,
                chrome.scrollbar_arrow_down_released,
                chrome.scrollbar_arrow_down_pressed,
            ),
        ] {
            let control = if up {
                ListScrollPart::Up
            } else {
                ListScrollPart::Down
            };
            if let Some(entry) = super::controls::scrollbar_arrow_entry(
                released,
                pressed,
                pressed_part == Some(control),
            ) {
                push_entry_native(out, entry, inner.x, y, depth - 0.00003);
            }
        }
        super::controls::push_scrollbar_thumb(out, &chrome, thumb, depth - 0.00004);
        push_rect_outline(
            out,
            atlas,
            bar,
            OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
            depth - 0.00005,
        );
    }
}
