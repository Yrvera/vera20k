//! Chrome and primitive sprite helpers for the skirmish shell renderer.
//!
//! These helpers build shell chrome, owner-draw button art, bevels, and
//! low-level sprite rectangles without changing render behavior.

use crate::render::batch::SpriteInstance;
use crate::render::skirmish_shell_chrome::{SkirmishShellChromeAtlas, SkirmishShellChromeEntry};
use crate::ui::skirmish_shell::{RectPx, SkirmishShellLayout, SkirmishShellState};

use super::draw_order::{
    LowerStripRole, ParentBackgroundRole, lower_strip_role, parent_background_role,
};
use super::{
    BUTTON_DISABLED_ALPHA, OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
    OWNERDRAW_BEVEL_LIGHT_RGB_FROM_PACKED_00C5BEA7, PRESSED_BUTTON_CONTENT_OFFSET_Y,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ButtonPiece {
    Left,
    Middle,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ButtonSegment {
    pub(super) piece: ButtonPiece,
    pub(super) x: f32,
    pub(super) width: f32,
    pub(super) uv_x_ratio: f32,
    pub(super) uv_width_ratio: f32,
}
#[cfg(test)]
pub(super) fn button_piece_asset_names(
    pressed: bool,
) -> (&'static str, &'static str, &'static str) {
    if pressed {
        ("bde_li30.pcx", "bde_mi30.pcx", "bde_ri30.pcx")
    } else {
        ("bue_li30.pcx", "bue_mi30.pcx", "bue_ri30.pcx")
    }
}

pub(super) fn build_button_segments(
    rect: RectPx,
    left_w: f32,
    mid_w: f32,
    right_w: f32,
) -> Vec<ButtonSegment> {
    if rect.w <= 0 {
        return Vec::new();
    }
    let rect_w = rect.w as f32;
    let left_w = left_w.round().max(1.0).min(rect_w);
    let right_w = right_w.round().max(1.0).min(rect_w);
    let mid_w = mid_w.round().max(1.0);
    let mut segments = vec![ButtonSegment {
        piece: ButtonPiece::Left,
        x: rect.x as f32,
        width: left_w,
        uv_x_ratio: 0.0,
        uv_width_ratio: 1.0,
    }];

    let middle_start = rect.x as f32 + left_w;
    let middle_dest_w = (rect_w - right_w).max(0.0);
    let middle_phase = ((mid_w - middle_dest_w) / 2.0).max(0.0);
    let mut covered = 0.0;
    while covered < middle_dest_w - f32::EPSILON {
        let source_x = (middle_phase + covered) % mid_w;
        let width = (middle_dest_w - covered).min(mid_w - source_x);
        segments.push(ButtonSegment {
            piece: ButtonPiece::Middle,
            x: middle_start + covered,
            width,
            uv_x_ratio: source_x / mid_w,
            uv_width_ratio: width / mid_w,
        });
        covered += width;
    }

    segments.push(ButtonSegment {
        piece: ButtonPiece::Right,
        x: rect.x as f32 + rect_w - right_w,
        width: right_w,
        uv_x_ratio: 0.0,
        uv_width_ratio: 1.0,
    });
    segments
}

pub(super) fn button_segment_sprite_size(
    entry: SkirmishShellChromeEntry,
    segment: ButtonSegment,
) -> [f32; 2] {
    [segment.width, entry.pixel_size[1]]
}

pub(super) fn button_art_y(rect: RectPx, art_h: f32, pressed: bool, disabled: bool) -> f32 {
    let art_h = art_h.round() as i32;
    let pressed_offset = if pressed && !disabled {
        PRESSED_BUTTON_CONTENT_OFFSET_Y
    } else {
        0
    };
    (rect.y + (rect.h - art_h) / 2 + pressed_offset) as f32
}

pub(super) fn push_entry(
    out: &mut Vec<SpriteInstance>,
    entry: SkirmishShellChromeEntry,
    rect: RectPx,
    depth: f32,
) {
    out.push(SpriteInstance {
        position: [rect.x as f32, rect.y as f32],
        size: [rect.w as f32, rect.h as f32],
        uv_origin: entry.uv_origin,
        uv_size: entry.uv_size,
        depth,
        tint: [1.0, 1.0, 1.0],
        alpha: 1.0,
        ..Default::default()
    });
}

pub(super) fn push_entry_sized(
    out: &mut Vec<SpriteInstance>,
    entry: SkirmishShellChromeEntry,
    x: f32,
    y: f32,
    size: [f32; 2],
    depth: f32,
) {
    out.push(SpriteInstance {
        position: [x, y],
        size,
        uv_origin: entry.uv_origin,
        uv_size: entry.uv_size,
        depth,
        tint: [1.0, 1.0, 1.0],
        alpha: 1.0,
        ..Default::default()
    });
}

pub(super) fn push_entry_top_clipped_native(
    out: &mut Vec<SpriteInstance>,
    mut entry: SkirmishShellChromeEntry,
    rect: RectPx,
    depth: f32,
) {
    let src_w = entry.pixel_size[0].round();
    let src_h = entry.pixel_size[1].round();
    if src_w <= 0.0 || src_h <= 0.0 || rect.w <= 0 || rect.h <= 0 {
        return;
    }

    let draw_w = (rect.w as f32).min(src_w);
    let draw_h = (rect.h as f32).min(src_h);
    entry.uv_size[0] *= draw_w / src_w;
    entry.uv_size[1] *= draw_h / src_h;
    push_entry_sized(
        out,
        entry,
        rect.x as f32,
        rect.y as f32,
        [draw_w, draw_h],
        depth,
    );
}

pub(super) fn push_entry_native(
    out: &mut Vec<SpriteInstance>,
    entry: SkirmishShellChromeEntry,
    x: i32,
    y: i32,
    depth: f32,
) {
    push_entry_sized(out, entry, x as f32, y as f32, entry.pixel_size, depth);
}

pub(super) fn push_entry_sized_alpha(
    out: &mut Vec<SpriteInstance>,
    entry: SkirmishShellChromeEntry,
    x: f32,
    y: f32,
    size: [f32; 2],
    depth: f32,
    alpha: f32,
) {
    out.push(SpriteInstance {
        position: [x, y],
        size,
        uv_origin: entry.uv_origin,
        uv_size: entry.uv_size,
        depth,
        tint: [1.0, 1.0, 1.0],
        alpha,
        ..Default::default()
    });
}

pub(super) fn push_entry_fit(
    out: &mut Vec<SpriteInstance>,
    entry: SkirmishShellChromeEntry,
    rect: RectPx,
    depth: f32,
) {
    let [src_w, src_h] = entry.pixel_size;
    if src_w <= 0.0 || src_h <= 0.0 || rect.w <= 0 || rect.h <= 0 {
        return;
    }
    let scale = (rect.w as f32 / src_w).min(rect.h as f32 / src_h);
    let w = (src_w * scale).round();
    let h = (src_h * scale).round();
    let x = rect.x as f32 + ((rect.w as f32 - w) * 0.5).round();
    let y = rect.y as f32 + ((rect.h as f32 - h) * 0.5).round();
    push_entry_sized(out, entry, x, y, [w, h], depth);
}

pub(super) fn push_flag_entry_native_clipped_centered(
    out: &mut Vec<SpriteInstance>,
    mut entry: SkirmishShellChromeEntry,
    rect: RectPx,
    depth: f32,
) {
    let src_w_px = entry.pixel_size[0].round() as i32;
    let src_h_px = entry.pixel_size[1].round() as i32;
    if src_w_px <= 0 || src_h_px <= 0 || rect.w <= 0 || rect.h <= 0 {
        return;
    }

    let src_w = src_w_px as f32;
    let src_h = src_h_px as f32;
    let rect_w = rect.w as f32;
    let rect_h = rect.h as f32;
    let draw_w = src_w.min(rect_w);
    let draw_h = src_h.min(rect_h);
    let x = if src_w_px < rect.w {
        (rect.x + (rect.w - src_w_px) / 2) as f32
    } else {
        rect.x as f32
    };
    let y = if src_h_px < rect.h {
        (rect.y + (rect.h - src_h_px) / 2) as f32
    } else {
        rect.y as f32
    };

    entry.uv_size[0] *= draw_w / src_w;
    entry.uv_size[1] *= draw_h / src_h;
    push_entry_sized(out, entry, x, y, [draw_w, draw_h], depth);
}

pub(super) fn button_entries(
    atlas: &SkirmishShellChromeAtlas,
    pressed: bool,
) -> Option<(
    SkirmishShellChromeEntry,
    SkirmishShellChromeEntry,
    SkirmishShellChromeEntry,
)> {
    if pressed {
        Some((
            atlas.button_down_left_30?,
            atlas.button_down_mid_30?,
            atlas.button_down_right_30?,
        ))
    } else {
        Some((
            atlas.button_up_left_30?,
            atlas.button_up_mid_30?,
            atlas.button_up_right_30?,
        ))
    }
}

pub(super) fn push_button_30(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    pressed: bool,
    disabled: bool,
    depth: f32,
) {
    let Some((left, mid, right)) = button_entries(atlas, pressed && !disabled) else {
        return;
    };
    let alpha = if disabled { BUTTON_DISABLED_ALPHA } else { 1.0 };
    for segment in build_button_segments(
        rect,
        left.pixel_size[0],
        mid.pixel_size[0],
        right.pixel_size[0],
    ) {
        let mut entry = match segment.piece {
            ButtonPiece::Left => left,
            ButtonPiece::Middle => mid,
            ButtonPiece::Right => right,
        };
        if segment.piece == ButtonPiece::Middle
            && (segment.uv_x_ratio > 0.0 || segment.uv_width_ratio < 1.0)
        {
            entry.uv_origin[0] += entry.uv_size[0] * segment.uv_x_ratio;
            entry.uv_size[0] *= segment.uv_width_ratio;
            entry.pixel_size[0] = segment.width;
        }
        push_entry_sized_alpha(
            out,
            entry,
            segment.x,
            button_art_y(rect, entry.pixel_size[1], pressed, disabled),
            button_segment_sprite_size(entry, segment),
            depth,
            alpha,
        );
    }
}

pub(super) const fn modal_button_mnbttn_frame_index(pressed: bool) -> usize {
    if pressed { 1 } else { 0 }
}

pub(super) fn modal_button_mnbttn_position(
    rect: RectPx,
    entry: SkirmishShellChromeEntry,
) -> [f32; 2] {
    let art_w = entry.pixel_size[0].round() as i32;
    let art_h = entry.pixel_size[1].round() as i32;
    [
        (rect.x + (rect.w - art_w) / 2) as f32,
        (rect.y + (rect.h - art_h) / 2) as f32,
    ]
}

pub(super) fn modal_button_mnbttn_entry(
    atlas: &SkirmishShellChromeAtlas,
    pressed: bool,
) -> Option<SkirmishShellChromeEntry> {
    match modal_button_mnbttn_frame_index(pressed) {
        1 => atlas.modal_button_mnbttn_frame1,
        2 => atlas.modal_button_mnbttn_frame2,
        _ => atlas.modal_button_mnbttn_frame0,
    }
}

pub(super) fn push_modal_button_mnbttn(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    pressed: bool,
    depth: f32,
) {
    let Some(entry) = modal_button_mnbttn_entry(atlas, pressed) else {
        push_button_30(out, atlas, rect, pressed, false, depth);
        return;
    };
    let [x, y] = modal_button_mnbttn_position(rect, entry);
    push_entry_sized(out, entry, x, y, entry.pixel_size, depth);
}

pub(super) fn push_right_panel_button_shp(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    pressed: bool,
    disabled: bool,
    depth: f32,
) {
    // Skirmish shell setup classifies Start/Choose/Back as owner-draw type 1.
    // Type 0 PCX pieces are only a missing-asset fallback for this path.
    let entry = match right_panel_button_sdbtnanm_frame_index(pressed, disabled) {
        4 => atlas.right_panel_button_sdbtnanm_frame4,
        _ => atlas.right_panel_button_sdbtnanm_frame2,
    };
    if let Some(entry) = entry {
        push_entry(out, entry, rect, depth);
    } else {
        push_button_30(out, atlas, rect, pressed, disabled, depth);
    }
}

pub(super) const fn right_panel_button_sdbtnanm_frame_index(
    pressed: bool,
    disabled: bool,
) -> usize {
    if pressed && !disabled { 4 } else { 2 }
}

pub(super) fn push_tinted_entry(
    out: &mut Vec<SpriteInstance>,
    entry: SkirmishShellChromeEntry,
    rect: RectPx,
    tint: [f32; 3],
    depth: f32,
) {
    out.push(SpriteInstance {
        position: [rect.x as f32, rect.y as f32],
        size: [rect.w as f32, rect.h as f32],
        uv_origin: entry.uv_origin,
        uv_size: entry.uv_size,
        depth,
        tint,
        alpha: 1.0,
        ..Default::default()
    });
}

pub(super) fn push_solid_rect(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    tint: [f32; 3],
    depth: f32,
) {
    let Some(pixel) = atlas.white_pixel else {
        return;
    };
    push_tinted_entry(out, pixel, rect, tint, depth);
}

pub(super) fn push_rect_outline(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    tint: [f32; 3],
    depth: f32,
) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    push_solid_rect(
        out,
        atlas,
        RectPx::new(rect.x, rect.y, rect.w, 1),
        tint,
        depth,
    );
    push_solid_rect(
        out,
        atlas,
        RectPx::new(rect.x, rect.y + rect.h - 1, rect.w, 1),
        tint,
        depth,
    );
    push_solid_rect(
        out,
        atlas,
        RectPx::new(rect.x, rect.y, 1, rect.h),
        tint,
        depth,
    );
    push_solid_rect(
        out,
        atlas,
        RectPx::new(rect.x + rect.w - 1, rect.y, 1, rect.h),
        tint,
        depth,
    );
}

pub(super) fn push_bevel_ring(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    top_left_tint: [f32; 3],
    bottom_right_tint: [f32; 3],
    depth: f32,
) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    push_solid_rect(
        out,
        atlas,
        RectPx::new(rect.x, rect.y, rect.w, 1),
        top_left_tint,
        depth,
    );
    if rect.h > 1 {
        push_solid_rect(
            out,
            atlas,
            RectPx::new(rect.x, rect.y + 1, 1, rect.h - 1),
            top_left_tint,
            depth,
        );
    }
    push_solid_rect(
        out,
        atlas,
        RectPx::new(rect.x, rect.y + rect.h - 1, rect.w, 1),
        bottom_right_tint,
        depth,
    );
    if rect.w > 1 && rect.h > 2 {
        push_solid_rect(
            out,
            atlas,
            RectPx::new(rect.x + rect.w - 1, rect.y + 1, 1, rect.h - 2),
            bottom_right_tint,
            depth,
        );
    }
}

pub(super) fn push_ownerdraw_two_pixel_bevel_frame(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    depth: f32,
) {
    push_bevel_ring(
        out,
        atlas,
        rect,
        OWNERDRAW_BEVEL_LIGHT_RGB_FROM_PACKED_00C5BEA7,
        OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
        depth,
    );
    if rect.w > 2 && rect.h > 2 {
        push_bevel_ring(
            out,
            atlas,
            RectPx::new(rect.x + 1, rect.y + 1, rect.w - 2, rect.h - 2),
            OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
            OWNERDRAW_BEVEL_LIGHT_RGB_FROM_PACKED_00C5BEA7,
            depth - 0.00001,
        );
    }
}

pub(super) fn parent_background_entry(
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
) -> Option<SkirmishShellChromeEntry> {
    match parent_background_role(layout)? {
        ParentBackgroundRole::Mnscrns640 => atlas.background_640_mnscrns,
        ParentBackgroundRole::CoopGameSetup800 => atlas.background_800_coop_game_setup,
    }
}

pub(super) fn lower_strip_entry(
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
) -> Option<SkirmishShellChromeEntry> {
    match lower_strip_role(layout) {
        LowerStripRole::Lwscrns640 => atlas.lower_side_640_lwscrns,
        LowerStripRole::LwscrnlLarge => atlas.lower_side_large_lwscrnl,
    }
}

pub(super) fn common_shell_origin(layout: &SkirmishShellLayout) -> (i32, i32) {
    let x = if layout.screen.w > 1023 {
        (layout.screen.w - 800) / 2
    } else {
        0
    };
    let y = if layout.screen.h > 767 {
        (layout.screen.h - 600) / 2
    } else {
        0
    };
    (layout.screen.x + x, layout.screen.y + y)
}

pub(super) fn lower_strip_rect(
    layout: &SkirmishShellLayout,
    entry: SkirmishShellChromeEntry,
) -> RectPx {
    let w = entry.pixel_size[0].round() as i32;
    let h = entry.pixel_size[1].round() as i32;
    let (origin_x, origin_y) = common_shell_origin(layout);
    let shell_h = if layout.screen.h > 767 {
        600
    } else {
        layout.screen.h
    };
    RectPx::new(origin_x, origin_y + shell_h - h, w, h)
}

pub(super) fn right_panel_overlay_rect(
    layout: &SkirmishShellLayout,
    row: i32,
    entry: SkirmishShellChromeEntry,
) -> RectPx {
    let w = entry.pixel_size[0].round() as i32;
    let h = entry.pixel_size[1].round() as i32;
    let x = layout.right_panel.tile.x + layout.right_panel.tile.w - w;
    RectPx::new(x, layout.right_panel.tile.y + row * h, w, h)
}

pub(super) fn sdmpbtn_rect(
    layout: &SkirmishShellLayout,
    entry: SkirmishShellChromeEntry,
) -> RectPx {
    let w = entry.pixel_size[0].round() as i32;
    let h = entry.pixel_size[1].round() as i32;
    let x = layout.right_panel.top.x + layout.right_panel.top.w - w;
    let y = layout.right_panel.tile.y + layout.right_panel.tile.h - h;
    RectPx::new(x, y, w, h)
}

pub(super) fn right_panel_frame10_overlay_active(_shell: &SkirmishShellState) -> bool {
    // Verified standard offline Skirmish first paint leaves the dialog gate at
    // zero, and that gate makes RightPanel__Draw skip the frame-10 overlay.
    false
}
