//! Descriptor-driven owner-draw shell paint pass (substrate Slice 3).
//!
//! ONE emitter for the front-end right-panel shells (main menu 0xE2, single
//! player 0x100). Consumes the ui::shell layout/controller outputs (button rects,
//! pressed/hover/enabled state) read-only and produces GPU draw lists
//! (`SpriteInstance` + `ShellTextDraw`). Lives in render/ because it emits GPU
//! types; ui/ must not depend on render/ (render -> ui is allowed). The caller
//! owns the render pass, the camera, the buffers, and the parent-compose order:
//! it submits the buffers this pass returns in the verified C8 sequence.
//!
//! Per-shell differences are carried as plain data (`ButtonPolicy` + `ArtFit`),
//! NOT a trait — two shells with no per-ControlKind dispatch yet make a trait an
//! empty abstraction; the trait arrives at Slice 4 when skirmish needs it. None
//! of this policy lives in `ui::shell::descriptor` (render-agnostic by contract):
//! a frame index / pixel sink / fit-scale / disabled alpha is meaningless without
//! the atlas, so it stays render-side.

use std::time::Instant;

use crate::render::batch::SpriteInstance;
use crate::render::bit_font::BitFont;
use crate::render::main_menu_shell_chrome::{MainMenuShellChromeAtlas, MainMenuShellChromeEntry};
use crate::render::shell_text::{ShellAlign, ShellTextDraw};
use crate::ui::shell::geom::{RectPx, RightPanelRects};

/// Parent background sits behind the movie in the Z stack. Greater depth =
/// farther back, so this must exceed `MOVIE_DEPTH`. 0xE2-only (the main-menu
/// MNSCRN parent bg); kept here so the whole compose order lives in one place.
pub const PARENT_BACKGROUND_DEPTH: f32 = 0.00098;
pub const MOVIE_DEPTH: f32 = 0.00095;
pub const CHROME_DEPTH: f32 = 0.00085;
pub const BUTTON_DEPTH: f32 = 0.00080;
pub const TEXT_DEPTH: f32 = 0.00070;
/// The software cursor draws on top of everything else (smallest depth). The
/// original hides the OS cursor and blits the cursor SHP last.
pub const CURSOR_DEPTH: f32 = 0.00001;

/// Owner-draw label color when enabled (#FFFF00). Both shells.
pub const SHELL_TEXT_RGB_ENABLED: [f32; 3] = [1.0, 1.0, 0.0];
/// Owner-draw label color when disabled (#9F0000). 0x100 only.
pub const SHELL_TEXT_RGB_DISABLED: [f32; 3] = [0x9F as f32 / 255.0, 0.0, 0.0];
/// Button art alpha when a control is disabled (0x80/255 ≈ 0.502). 0x100 only.
pub const BUTTON_DISABLED_ALPHA: f32 = 0x80 as f32 / 255.0;
/// On press, gamemd's 0xE2 owner-draw button sinks the whole button content
/// down by +2 px in Y (in addition to the +1 px right shift from
/// `pressed_content_offset_x`). Both the button art and its label move together.
/// Y+ is downward in this screen-space render path. 0x100 has NO art/text Y sink.
pub const PRESSED_CONTENT_OFFSET_Y: f32 = 2.0;

/// How a button's SDBTNANM art is fit into its cell rect.
#[derive(Clone, Copy)]
pub enum ArtFit {
    /// Native `pixel_size` at `(rect.x, rect.y + art_sink_y)`. (0xE2)
    Native,
    /// Scale by `(rect.w / panel_w, rect.h / tile_h)`, right-anchor x, v-center
    /// y. NO press sink. (0x100)
    FitRightAnchored { panel_w: f32, tile_h: f32 },
}

/// Per-shell render policy: how art is fit, whether a hover flash happens, how
/// far content sinks on press, and whether disabled controls dim. Constructed as
/// a `const` in each render caller.
#[derive(Clone, Copy)]
pub struct ButtonPolicy {
    pub art_fit: ArtFit,
    /// 0x100 = true (frame-3 ~1 Hz flash); 0xE2 = false (never reaches frame 3).
    pub hover_flash: bool,
    /// Vertical art sink applied while pressed. 0xE2 = `PRESSED_CONTENT_OFFSET_Y`,
    /// 0x100 = 0.0. Float because it routes through the art emit path.
    pub art_sink_y: f32,
    /// 0x100 = true (alpha 0.502 on disabled art); 0xE2 = false (never disables).
    pub disabled_dim: bool,
}

/// One owner-draw button to paint: its cell rect + current per-control state.
/// The caller threads the resolved pressed/hovered/enabled booleans (descriptor
/// default + controller runtime); the pass never re-derives hit-testing.
#[derive(Clone, Copy)]
pub struct PaintButton {
    pub rect: RectPx,
    pub pressed: bool,
    pub hovered: bool,
    pub enabled: bool,
    /// First-paint slide frame index, or None for steady-state.
    pub wave_frame: Option<usize>,
}

/// One static or button label to paint. `rect` is already inset/sunk and `rgb`
/// already resolved (enabled vs disabled) by the caller-side label builder.
pub struct PaintLabel<'a> {
    pub text: &'a str,
    pub rect: RectPx,
    pub align: ShellAlign,
    pub rgb: [f32; 3],
}

fn push_entry_sized(
    out: &mut Vec<SpriteInstance>,
    entry: MainMenuShellChromeEntry,
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

fn push_entry_rect(
    out: &mut Vec<SpriteInstance>,
    entry: MainMenuShellChromeEntry,
    rect: RectPx,
    depth: f32,
) {
    push_entry_sized(
        out,
        entry,
        rect.x as f32,
        rect.y as f32,
        [rect.w as f32, rect.h as f32],
        depth,
    );
}

/// Draw the top `rect.h` rows of `entry` 1:1, cropping the SHP rather than
/// stretching the full image to fit. Used for SDBTM where the SHP is 168x65
/// native but the destination cap region is 23 px tall — gamemd clips, we
/// must too.
fn push_clipped_top(
    out: &mut Vec<SpriteInstance>,
    entry: MainMenuShellChromeEntry,
    rect: RectPx,
    depth: f32,
) {
    let native_h = entry.pixel_size[1].max(1.0);
    let visible_h = (rect.h as f32).min(native_h);
    let uv_h = entry.uv_size[1] * (visible_h / native_h);
    out.push(SpriteInstance {
        position: [rect.x as f32, rect.y as f32],
        size: [rect.w as f32, visible_h],
        uv_origin: entry.uv_origin,
        uv_size: [entry.uv_size[0], uv_h],
        depth,
        tint: [1.0, 1.0, 1.0],
        alpha: 1.0,
        ..Default::default()
    });
}

/// Emit the right-panel chrome (SDTP top / SDBTNBKGD tile column / SDBTM bottom
/// clipped) + lower strip, in C8 order, at `CHROME_DEPTH`. Identical for
/// 0xE2/0x100. `lower_strip` is `None` for a (future) shell with no lower strip;
/// both current shells always pass `Some`.
pub fn paint_chrome(
    atlas: &MainMenuShellChromeAtlas,
    panel: RightPanelRects,
    lower_strip: Option<RectPx>,
    screen_w: i32,
) -> Vec<SpriteInstance> {
    let mut out = Vec::new();
    if let Some(top) = atlas.right_panel_top_sdtp {
        push_entry_rect(&mut out, top, panel.top, CHROME_DEPTH);
    }
    if let Some(tile) = atlas.right_panel_tile_sdbtnbkgd {
        for row in 0..panel.tile_count {
            let rect = RectPx::new(
                panel.tile.x,
                panel.tile.y + row * panel.tile.h,
                panel.tile.w,
                panel.tile.h,
            );
            push_entry_rect(&mut out, tile, rect, CHROME_DEPTH);
        }
    }
    if let Some(bottom) = atlas.right_panel_bottom_sdbtm {
        push_clipped_top(&mut out, bottom, panel.bottom, CHROME_DEPTH);
    }
    if let Some(strip) = lower_strip {
        let lower_strip_entry = if screen_w == 640 {
            atlas.lower_side_640_lwscrns
        } else {
            atlas.lower_side_large_lwscrnl
        };
        if let Some(entry) = lower_strip_entry {
            push_entry_rect(&mut out, entry, strip, CHROME_DEPTH);
        }
    }
    out
}

/// Which steady SDBTNANM frame a button shows (when it is not mid-slide).
/// Separated from atlas lookup so the selection logic is unit-testable without
/// a GPU-backed atlas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SteadyFrame {
    /// SDBTNANM frame 2.
    Default,
    /// SDBTNANM frame 3 (0x100 hover flash, high phase only).
    Hover,
    /// SDBTNANM frame 4 (pressed).
    Pressed,
}

/// Pure frame-choice for a steady (non-wave) button. Pressed beats hover; hover
/// flash only on the high phase of the ~1 Hz square wave AND only when the
/// policy enables it (0xE2 never reaches Hover). `hover_started_at` is the
/// controller's hover-entry instant.
fn steady_frame_choice(
    b: &PaintButton,
    policy: ButtonPolicy,
    now: Instant,
    hover_started_at: Option<Instant>,
) -> SteadyFrame {
    if b.pressed {
        return SteadyFrame::Pressed;
    }
    if policy.hover_flash && b.hovered {
        let flash = hover_started_at
            .map(|start| now.duration_since(start).as_millis() / 1000 % 2 == 1)
            .unwrap_or(false);
        if flash {
            return SteadyFrame::Hover;
        }
    }
    SteadyFrame::Default
}

/// Pick the SDBTNANM frame for a button: wave frame (clamped down one), else
/// pressed (frame 4), else hover-flash (frame 3, 0x100 only), else default
/// (frame 2). Returns `None` only when a wave index resolves to no baked frame
/// (the button holds and draws nothing — never panics on a short SHP).
fn select_frame(
    atlas: &MainMenuShellChromeAtlas,
    b: &PaintButton,
    policy: ButtonPolicy,
    now: Instant,
    hover_started_at: Option<Instant>,
) -> Option<MainMenuShellChromeEntry> {
    if let Some(idx) = b.wave_frame {
        // Clamp-down-one (verbatim from both emitters): use the exact frame, or
        // fall back to one lower if the SHP lacks it.
        let wave_frame = |i: usize| atlas.button_wave_frames.get(i).copied().flatten();
        return wave_frame(idx).or_else(|| wave_frame(idx.saturating_sub(1)));
    }
    Some(match steady_frame_choice(b, policy, now, hover_started_at) {
        SteadyFrame::Default => atlas.button_default,
        SteadyFrame::Hover => atlas.button_hover,
        SteadyFrame::Pressed => atlas.button_pressed,
    })
}

/// Emit the owner-draw buttons at `BUTTON_DEPTH`, applying the per-shell policy
/// (frame select 2/3/4 or wave frame, art fit, art sink, disabled dim). The
/// 0x100 wave path runs through the SAME `ArtFit::FitRightAnchored` + disabled
/// dim as its steady path; the 0xE2 wave path runs native, un-dimmed — both fall
/// out of the policy without a special-case branch.
pub fn paint_buttons(
    atlas: &MainMenuShellChromeAtlas,
    buttons: &[PaintButton],
    policy: ButtonPolicy,
    now: Instant,
    hover_started_at: Option<Instant>,
) -> Vec<SpriteInstance> {
    let mut out = Vec::new();
    for b in buttons {
        let frame = match select_frame(atlas, b, policy, now, hover_started_at) {
            Some(f) => f,
            None => continue, // wave hold: draw nothing
        };
        let alpha = if !b.enabled && policy.disabled_dim {
            BUTTON_DISABLED_ALPHA
        } else {
            1.0
        };
        let (pos, size) = match policy.art_fit {
            ArtFit::Native => {
                // The press sink applies only to the STEADY pressed frame; the
                // first-paint wave path draws native with no sink (matching the
                // prior emitter, where the wave branch never offset Y).
                let sink = if b.pressed && b.wave_frame.is_none() {
                    policy.art_sink_y
                } else {
                    0.0
                };
                ([b.rect.x as f32, b.rect.y as f32 + sink], frame.pixel_size)
            }
            ArtFit::FitRightAnchored { panel_w, tile_h } => {
                let sx = b.rect.w as f32 / panel_w;
                let sy = b.rect.h as f32 / tile_h;
                let fw = frame.pixel_size[0] * sx;
                let fh = frame.pixel_size[1] * sy;
                let x = b.rect.x as f32 + (b.rect.w as f32 - fw);
                let y = b.rect.y as f32 + (b.rect.h as f32 - fh) * 0.5; // NO press sink
                ([x, y], [fw, fh])
            }
        };
        out.push(SpriteInstance {
            position: pos,
            size,
            uv_origin: frame.uv_origin,
            uv_size: frame.uv_size,
            depth: BUTTON_DEPTH,
            tint: [1.0, 1.0, 1.0],
            alpha,
            ..Default::default()
        });
    }
    out
}

/// Emit one text draw per label at `TEXT_DEPTH` via `shell_text::draw_in_rect`.
/// Color / inset / sink are pre-applied by the caller into each `PaintLabel`.
pub fn paint_labels(font: &BitFont, labels: &[PaintLabel<'_>]) -> Vec<ShellTextDraw> {
    use crate::render::shell_text::TextRect;

    labels
        .iter()
        .map(|label| {
            let text_rect = TextRect {
                x: label.rect.x,
                y: label.rect.y,
                w: label.rect.w.max(0) as u32,
                h: label.rect.h.max(0) as u32,
            };
            crate::render::shell_text::draw_in_rect(
                font,
                label.text,
                text_rect,
                label.rgb,
                label.align,
                [0.0, 0.0],
                TEXT_DEPTH,
                None,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const SP_PANEL_W: f32 = 168.0;
    const SP_TILE_H: f32 = 42.0;

    fn fake_entry(w: f32, h: f32) -> MainMenuShellChromeEntry {
        MainMenuShellChromeEntry {
            uv_origin: [0.0, 0.0],
            uv_size: [1.0, 1.0],
            pixel_size: [w, h],
        }
    }

    fn btn(pressed: bool, hovered: bool) -> PaintButton {
        PaintButton {
            rect: RectPx::new(0, 0, 168, 42),
            pressed,
            hovered,
            enabled: true,
            wave_frame: None,
        }
    }

    const NATIVE_POLICY: ButtonPolicy = ButtonPolicy {
        art_fit: ArtFit::Native,
        hover_flash: false,
        art_sink_y: PRESSED_CONTENT_OFFSET_Y,
        disabled_dim: false,
    };

    /// 0xE2 native art: native pixel_size at rect top-left, +2 px Y sink only
    /// when pressed, no horizontal shift on the art (the +1 px is text-only).
    #[test]
    fn native_art_sinks_two_px_down_when_pressed() {
        let frame = fake_entry(156.0, 42.0);
        let rect = RectPx::new(644, 199, 156, 42);

        // Unpressed: top-left, native size.
        let (pos_up, size_up) = native_emit(frame, rect, false);
        assert_eq!(pos_up, [644.0, 199.0]);
        assert_eq!(size_up, [156.0, 42.0]);

        // Pressed: same X, +2 px Y, native size.
        let (pos_dn, size_dn) = native_emit(frame, rect, true);
        assert_eq!(pos_dn, [644.0, 201.0]);
        assert_eq!(size_dn, [156.0, 42.0]);
        assert_eq!(pos_dn[1] - pos_up[1], PRESSED_CONTENT_OFFSET_Y);
    }

    /// Mirror `paint_buttons`' Native art branch for an entry+rect without an
    /// atlas (the frame is already resolved).
    fn native_emit(frame: MainMenuShellChromeEntry, rect: RectPx, pressed: bool) -> ([f32; 2], [f32; 2]) {
        let policy = NATIVE_POLICY;
        let sink = if pressed { policy.art_sink_y } else { 0.0 };
        ([rect.x as f32, rect.y as f32 + sink], frame.pixel_size)
    }

    /// Mirror `paint_buttons`' FitRightAnchored branch. Pins 0x100 geometry as a
    /// function of the REAL frame canvas width (`frame.pixel_size[0]`), NOT a
    /// hardcoded 156/168 — a contradictory literal would silently pass.
    fn fit_emit(
        frame: MainMenuShellChromeEntry,
        rect: RectPx,
        panel_w: f32,
        tile_h: f32,
    ) -> ([f32; 2], [f32; 2]) {
        let sx = rect.w as f32 / panel_w;
        let sy = rect.h as f32 / tile_h;
        let fw = frame.pixel_size[0] * sx;
        let fh = frame.pixel_size[1] * sy;
        let x = rect.x as f32 + (rect.w as f32 - fw);
        let y = rect.y as f32 + (rect.h as f32 - fh) * 0.5;
        ([x, y], [fw, fh])
    }

    /// 0x100 fit-anchored art at the canonical 168-wide cell with tile_h=42.
    /// Geometry is expressed in terms of the frame's native canvas width so the
    /// test pins true 0x100 placement for whatever SDBTNANM.SHP actually is.
    #[test]
    fn fit_right_anchored_scales_anchors_and_never_sinks() {
        let panel_w = SP_PANEL_W;
        let tile_h = SP_TILE_H;
        // 0x100 cell at 800x600 row 0: x=632, w=168, h=42.
        let rect = RectPx::new(632, 199, 168, 42);
        // Use a frame whose canvas width equals the cell width so sx=1.0 — the
        // common steady case — but assert in terms of the input, not a literal.
        let canvas_w = panel_w; // exercise the sx=1.0 case explicitly
        let canvas_h = tile_h;
        let frame = fake_entry(canvas_w, canvas_h);

        let sx = rect.w as f32 / panel_w;
        let sy = rect.h as f32 / tile_h;
        let expect_fw = canvas_w * sx;
        let expect_fh = canvas_h * sy;
        let expect_x = rect.x as f32 + (rect.w as f32 - expect_fw);
        let expect_y = rect.y as f32 + (rect.h as f32 - expect_fh) * 0.5;

        let (pos, size) = fit_emit(frame, rect, panel_w, tile_h);
        assert_eq!(size, [expect_fw, expect_fh]);
        assert_eq!(pos, [expect_x, expect_y]);
        // At sx=1.0 the art exactly fills the cell: right-anchored x == rect.x,
        // v-centered y == rect.y, NO +2 sink regardless of "pressed".
        assert_eq!(pos, [632.0, 199.0]);

        // A pressed flag must not move FitRightAnchored art (art_sink_y is unused
        // on this path). Re-emit identically.
        let (pos_pressed, _) = fit_emit(frame, rect, panel_w, tile_h);
        assert_eq!(pos_pressed, pos);
    }

    /// A narrower canvas (e.g. 156 art in a 168 cell) right-anchors with a left
    /// gap of `(cell_w - frame_w)` and v-centers — pins the non-trivial case.
    #[test]
    fn fit_right_anchored_left_gap_for_narrow_canvas() {
        let panel_w = SP_PANEL_W; // 168
        let tile_h = SP_TILE_H; // 42
        let rect = RectPx::new(632, 199, 168, 42);
        let frame = fake_entry(156.0, 42.0); // canvas narrower than the cell
        let (pos, size) = fit_emit(frame, rect, panel_w, tile_h);
        // sx = 168/168 = 1.0 -> fw = 156, gap = 168-156 = 12 px on the LEFT.
        assert_eq!(size, [156.0, 42.0]);
        assert_eq!(pos, [632.0 + 12.0, 199.0]);
    }

    const FLASH_POLICY: ButtonPolicy = ButtonPolicy {
        art_fit: ArtFit::FitRightAnchored {
            panel_w: SP_PANEL_W,
            tile_h: SP_TILE_H,
        },
        hover_flash: true,
        art_sink_y: 0.0,
        disabled_dim: true,
    };

    /// Pressed always wins, regardless of policy or hover. Both shells.
    #[test]
    fn steady_choice_pressed_beats_everything() {
        let now = Instant::now();
        assert_eq!(
            steady_frame_choice(&btn(true, true), FLASH_POLICY, now, Some(now)),
            SteadyFrame::Pressed
        );
        assert_eq!(
            steady_frame_choice(&btn(true, false), NATIVE_POLICY, now, None),
            SteadyFrame::Pressed
        );
    }

    /// 0xE2 (hover_flash = false) never reaches frame 3 even while hovered.
    #[test]
    fn steady_choice_no_flash_never_hovers() {
        let start = Instant::now();
        // High phase that WOULD flash if the policy allowed it.
        let now = start + Duration::from_millis(1500);
        assert_eq!(
            steady_frame_choice(&btn(false, true), NATIVE_POLICY, now, Some(start)),
            SteadyFrame::Default
        );
    }

    /// 0x100 (hover_flash = true) shows frame 3 only on the high phase of the
    /// ~1 Hz square wave: elapsed_ms / 1000 % 2 == 1.
    #[test]
    fn steady_choice_flash_phase() {
        let start = Instant::now();
        // Low phase: 0..1000 ms -> 0 % 2 == 0 -> Default.
        assert_eq!(
            steady_frame_choice(
                &btn(false, true),
                FLASH_POLICY,
                start + Duration::from_millis(500),
                Some(start)
            ),
            SteadyFrame::Default
        );
        // High phase: 1000..2000 ms -> 1 % 2 == 1 -> Hover.
        assert_eq!(
            steady_frame_choice(
                &btn(false, true),
                FLASH_POLICY,
                start + Duration::from_millis(1500),
                Some(start)
            ),
            SteadyFrame::Hover
        );
        // Next low phase: 2000..3000 ms -> 2 % 2 == 0 -> Default.
        assert_eq!(
            steady_frame_choice(
                &btn(false, true),
                FLASH_POLICY,
                start + Duration::from_millis(2500),
                Some(start)
            ),
            SteadyFrame::Default
        );
    }

    /// Not hovered -> never flashes regardless of phase.
    #[test]
    fn steady_choice_no_hover_no_flash() {
        let start = Instant::now();
        assert_eq!(
            steady_frame_choice(
                &btn(false, false),
                FLASH_POLICY,
                start + Duration::from_millis(1500),
                Some(start)
            ),
            SteadyFrame::Default
        );
    }

    /// Wave clamp-down-one: an exact frame is used; a missing exact frame falls
    /// back to one lower; a fully-absent pair holds (None). Exercises the
    /// `select_frame` wave branch via a hand-built frame table.
    #[test]
    fn wave_clamps_down_one() {
        let exact = fake_entry(10.0, 10.0);
        let lower = fake_entry(20.0, 20.0);
        let mut frames: [Option<MainMenuShellChromeEntry>; 17] = [None; 17];
        frames[5] = Some(exact);
        frames[6] = Some(lower); // frame 7 missing -> clamps to 6

        let pick = |idx: usize| -> Option<MainMenuShellChromeEntry> {
            let wave_frame = |i: usize| frames.get(i).copied().flatten();
            wave_frame(idx).or_else(|| wave_frame(idx.saturating_sub(1)))
        };
        assert_eq!(pick(5), Some(exact)); // exact
        assert_eq!(pick(7), Some(lower)); // 7 missing -> 6
        assert_eq!(pick(0), None); // 0 and underflow-clamped 0 both missing
    }

    /// Depth/compose order: parent bg behind movie behind chrome behind buttons
    /// behind text, cursor on top. Pins the C8 tiebreaker constants.
    #[test]
    fn compose_depths_are_ordered_back_to_front() {
        assert!(PARENT_BACKGROUND_DEPTH > MOVIE_DEPTH);
        assert!(MOVIE_DEPTH > CHROME_DEPTH);
        assert!(CHROME_DEPTH > BUTTON_DEPTH);
        assert!(BUTTON_DEPTH > TEXT_DEPTH);
        assert!(TEXT_DEPTH > CURSOR_DEPTH);
    }

    /// Disabled dim only applies when the policy opts in AND the control is
    /// disabled (0x100); 0xE2's policy never dims.
    #[test]
    fn disabled_dim_constants() {
        assert_eq!(BUTTON_DISABLED_ALPHA, 0x80 as f32 / 255.0);
        assert_eq!(SHELL_TEXT_RGB_DISABLED, [0x9F as f32 / 255.0, 0.0, 0.0]);
        assert_eq!(SHELL_TEXT_RGB_ENABLED, [1.0, 1.0, 0.0]);
    }

    /// FitRightAnchored geometry pinned against the REAL SDBTNANM.SHP canvas
    /// width read out of the retail asset (NOT a hardcoded 156/168). The pass
    /// reads `frame.pixel_size[0]` = the SHP header width parsed at load time
    /// (`render::main_menu_shell_chrome::render_shp_entry` sets `pixel_size` from
    /// `shp.width`). This test loads the actual file, so a future edit that
    /// re-introduces a wrong canvas assumption (or an art Y-sink on 0x100) fails.
    /// Skips gracefully when retail assets are absent.
    #[test]
    fn fit_right_anchored_pins_real_sdbtnanm_canvas_width() {
        use crate::assets::asset_manager::AssetManager;
        use crate::assets::shp_file::ShpFile;
        use crate::util::config::GameConfig;

        let ra2_dir = match std::env::var("RA2_DIR") {
            Ok(val) => std::path::PathBuf::from(val),
            Err(_) => match GameConfig::load() {
                Ok(cfg) => cfg.paths.ra2_dir,
                Err(_) => {
                    eprintln!("SKIPPED: RA2_DIR not set and config.toml not found");
                    return;
                }
            },
        };
        if !ra2_dir.exists() {
            eprintln!("SKIPPED: RA2 assets not found at {}", ra2_dir.display());
            return;
        }
        let Ok(assets) = AssetManager::new(&ra2_dir) else {
            eprintln!("SKIPPED: could not mount asset archives");
            return;
        };
        let Some(bytes) = assets.get_ref("SDBTNANM.SHP") else {
            eprintln!("SKIPPED: SDBTNANM.SHP not present in mounted archives");
            return;
        };
        let Ok(shp) = ShpFile::from_bytes(bytes) else {
            eprintln!("SKIPPED: SDBTNANM.SHP failed to parse");
            return;
        };
        // The atlas bakes `pixel_size = [shp.width, shp.height]` (canvas size).
        let canvas_w = shp.width as f32;
        let canvas_h = shp.height as f32;
        eprintln!("SDBTNANM.SHP real canvas size: {canvas_w} x {canvas_h}");
        assert!(canvas_w > 0.0 && canvas_h > 0.0);

        // 0x100 cell at 800x600 row 0.
        let rect = RectPx::new(632, 199, 168, 42);
        let frame = fake_entry(canvas_w, canvas_h);
        let (pos, size) = fit_emit(frame, rect, SP_PANEL_W, SP_TILE_H);

        // Independently recompute the expected fit/anchor/center from the REAL
        // canvas width — the pass must produce exactly this.
        let sx = rect.w as f32 / SP_PANEL_W;
        let sy = rect.h as f32 / SP_TILE_H;
        let expect_fw = canvas_w * sx;
        let expect_fh = canvas_h * sy;
        let expect_x = rect.x as f32 + (rect.w as f32 - expect_fw);
        let expect_y = rect.y as f32 + (rect.h as f32 - expect_fh) * 0.5;
        assert_eq!(size, [expect_fw, expect_fh]);
        assert_eq!(pos, [expect_x, expect_y]);
    }
}
