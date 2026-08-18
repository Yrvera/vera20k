Both files re-verified against current content (line numbers in the task maps match what I read). Here is the plan body.

---

# Shell Substrate Slice 3 — Descriptor-Driven `OwnerDrawControl` Paint Pass — Implementation Plan

> **Scope this run:** Slice 3 only. Slices 0–2 are committed (`geom`, `descriptor`+`layout`, `controller`). This slice collapses the two duplicated shell paint emitters (`app_main_menu_shell_render.rs`, `app_single_player_shell_render.rs`) into ONE descriptor-driven paint pass living in `render/`, with **zero changed pixels per state (idle/hover/pressed) on both shells**. Acceptance is literally a screenshot diff == 0.
>
> **Layering invariant (load-bearing):** `ui/shell/*` depends on `sim/` only — never `render/`/`assets/`. The paint pass emits GPU draws, so it MUST live in `render/` and CONSUME the `ui::shell` descriptors read-only (`render → ui` is allowed). No GPU/atlas type may appear in any `ui/shell/*` signature.
>
> **Ghidra read-only. cargo NOT run inside this plan's edit window** — verification is a separate bounded pass (per `feedback_cargo_separate_verify_pass`).

---

## 0. Grounding summary (live-src re-verified THIS run; file:line quoted)

Both render emitters and all four `ui/shell` substrate files were re-read this session. **The task's §7.5 line ranges are confirmed current** (`app_main_menu_shell_render.rs` 1–768; `app_single_player_shell_render.rs` 1–532), but two of the task's contract-list claims are corrected here against the actual code, because they would have moved pixels if taken literally:

**CORRECTION 1 — pressed-sink is NOT uniform across shells.** The C9 contract list ("pressed sink +2y/+1x") is **main-menu-only on the button ART**:
- Main menu (`app_main_menu_shell_render.rs:80-104`): `push_button_shp` adds `pressed_content_offset_y(pressed)` (= +2.0px) to the art Y. `build_text_draws:286-292` adds the same +2y to the label rect AND `+x_offset` (= `layout.pressed_content_offset_x`, =1) to the label X. So on press, 0xE2 sinks **art +2y, text +2y +1x**.
- Single player (`app_single_player_shell_render.rs:70-95`): `push_button_shp` has **NO Y sink at all** — `y = rect.y + (rect.h - frame_h)*0.5` (a fit-centering, not a press offset), and there is no `pressed_content_offset_y` symbol in the file. `build_text_draws:287-300` applies **only `x_offset`** (=1) on press; the text rect Y is `button.rect.y + 1` with no press term. So on press, 0x100 sinks **art 0y, text 0y +1x**.

The unified pass therefore CANNOT apply a single sink rule. It must thread a per-shell `art_sink_y` and `text_sink_y` (0xE2 = 2.0/2.0, 0x100 = 0.0/0.0) plus a shared `text_sink_x_on_press` (both = `layout.pressed_content_offset_x` = 1).

**CORRECTION 2 — button art geometry differs (native vs fit-scaled), and this is the same load-bearing 156-vs-168 divergence from Slice 0.**
- Main menu (`:88-104`): art drawn at **native** `frame.pixel_size` at `(rect.x, rect.y + sink)`. No scaling, no right-anchor (the 156-wide art is pre-inset inside its atlas entry; `rect` is the 156-wide snapped cell).
- Single player (`:78-95`): art **fit-scaled** by `scale_x = rect.w/168`, `scale_y = rect.h/42`, then **right-anchored** `x = rect.x + (rect.w - frame_w)`, **v-centered** `y = rect.y + (rect.h - frame_h)*0.5`.

These are two different emit formulas. The pass parameterizes them via an `ArtFit` enum (`Native` vs `FitRightAnchored`), NOT one formula.

**CONFIRMED claims from the task maps (used verbatim):**
- Composition / depth order both shells: parent_bg `0.00098` (0xE2 only) → movie `0.00095` → chrome `0.00085` → buttons `0.00080` → text `0.00070` → cursor `0.00001`. (`app_main_menu_shell_render.rs:21-34`; `app_single_player_shell_render.rs:18-24`.)
- Hover flash: 0x100 only. `button_frame(atlas, pressed, hover_highlight)` selects `button_hover` (SDBTNANM frame 3) when `now.duration_since(hover_started_at).as_millis()/1000 % 2 == 1` (`:56-68, :156-159`). 0xE2's `button_frame(atlas, pressed)` has **no hover param** and never returns frame 3 (`:67-76`, comment "frame-3 focus-flash is never reached on this dialog").
- Text colors: enabled `[1.0,1.0,0.0]` (#FFFF00) both shells; disabled `[0x9F/255,0,0]` (#9F0000) + button alpha `0x80/255` ≈ 0.502 are **0x100 only** (`app_single_player_shell_render.rs:28-29, 92, 203, 307-311`). 0xE2 has no disabled control and always passes #FFFF00 (`:157`).
- Statics differ: 0xE2 draws title (`GUI:MainMenu`), version (`"GUI:Version" + version_txt`), and a hover tooltip line (`:295-320`). 0x100 draws only title (`GUI:SinglePlayerMenu`) (`:315-323`); its `status_help`/`side_image_static` rects exist in the layout but are not drawn by `build_text_draws`.
- One atlas, already shared: `MainMenuShellChromeAtlas` is built once and reused by both shells (`render/main_menu_shell_chrome.rs:33-140`; consumed by both render files via `state.main_menu_shell_chrome`). No packed-pixel duplication exists today; Slice 3 must not introduce any.
- First-paint wave: both shells call `wave.sdbtnanm_frame(slot, ButtonGroup::A)` and clamp-down-one via `button_wave_frames.get(idx).copied().flatten()` (0xE2 `:117-127`, 0x100 `:185-206`). 0x100's wave path additionally fit-scales + right-anchors + applies disabled alpha; 0xE2's draws native.
- `shell_text::draw_in_rect` signature (the shared text seam, unchanged): `(font, text, TextRect, color:[f32;3], flags:ShellAlign, cam_offset, depth, reveal:Option<Reveal>) -> ShellTextDraw` (`render/shell_text.rs:67-77`). Both shells already call it identically.

**Layout struct shapes (consumed read-only):** `MainMenuShellLayout` has `buttons: [MainMenuButtonRect;6]` + `right_panel`, `lower_strip`, `title`, `version_line`, `tooltip_line`, `pressed_content_offset_x`, `screen` (`ui/main_menu_shell/layout.rs:43-57`). `SinglePlayerShellLayout` has `buttons:[SinglePlayerButtonRect;4]` + `right_panel`, `lower_strip`, `title`, `pressed_content_offset_x`, `screen` (`ui/single_player_shell/layout.rs:28-39`). Each `*ButtonRect` is `{ id, rect: RectPx }`.

**Unchecked:** the skirmish emitter (`app_skirmish_shell_render/`) is explicitly OUT of Slice 3 (study/plan put it in Slice 4) — do not touch it.

---

## 1. Architecture — module path, pass signature, thin callers

### 1.1 New module: `src/render/shell_paint.rs`

Lives in `render/` (consumes `ui::shell` + `ui::*_shell` layouts read-only; emits `SpriteInstance` + `ShellTextDraw`). This is the ONE copy of the previously-duplicated geometry. It does NOT own the render pass, the camera, the buffers, the encoder, or the offscreen/flip — those stay in the per-shell `render_*_to_target` callers (matching the existing `app_shell_transition` model: emit draw lists, the caller composes). This keeps the C8 compose order owned by the caller and avoids the pass needing GPU lifecycle knowledge.

Register in `src/render/mod.rs`: add `pub mod shell_paint;` to the module list.

### 1.2 Core data — render-side, NOT in `ui/shell`

The pass takes a small render-side spec struct that captures every per-shell difference as plain data. **None of this goes in `ui::shell::descriptor`** (justified in §4):

```rust
//! src/render/shell_paint.rs
//! Descriptor-driven owner-draw shell paint pass (substrate Slice 3).
//!
//! ONE emitter for the front-end right-panel shells (main menu 0xE2, single
//! player 0x100). Consumes the ui::shell layout/controller outputs (button rects,
//! pressed/hover state) read-only and produces GPU draw lists (SpriteInstance +
//! ShellTextDraw). Lives in render/ because it emits GPU types; ui/ must not
//! depend on render/ (render -> ui is allowed). The caller owns the render pass,
//! the camera, the buffers, and the parent-compose order (C8).

use std::time::Instant;
use crate::render::batch::SpriteInstance;
use crate::render::main_menu_shell_chrome::{MainMenuShellChromeAtlas, MainMenuShellChromeEntry};
use crate::render::shell_text::{ShellAlign, ShellTextDraw};
use crate::ui::shell::geom::RectPx;

pub const PARENT_BACKGROUND_DEPTH: f32 = 0.00098;
pub const MOVIE_DEPTH: f32 = 0.00095;
pub const CHROME_DEPTH: f32 = 0.00085;
pub const BUTTON_DEPTH: f32 = 0.00080;
pub const TEXT_DEPTH: f32 = 0.00070;
pub const CURSOR_DEPTH: f32 = 0.00001;

pub const SHELL_TEXT_RGB_ENABLED: [f32; 3] = [1.0, 1.0, 0.0];          // #FFFF00
pub const SHELL_TEXT_RGB_DISABLED: [f32; 3] = [0x9F as f32 / 255.0, 0.0, 0.0]; // #9F0000
pub const BUTTON_DISABLED_ALPHA: f32 = 0x80 as f32 / 255.0;           // 0.502
pub const PRESSED_CONTENT_OFFSET_Y: f32 = 2.0;

/// How a button's SDBTNANM art is fit into its cell rect.
#[derive(Clone, Copy)]
pub enum ArtFit {
    /// Native pixel_size at (rect.x, rect.y + art_sink_y). (0xE2)
    Native,
    /// Scale by (rect.w/panel_w, rect.h/tile_h), right-anchor x, v-center y. (0x100)
    FitRightAnchored { panel_w: f32, tile_h: f32 },
}

/// Whether this shell shows a hover flash (frame 3) and/or sinks art on press.
#[derive(Clone, Copy)]
pub struct ButtonPolicy {
    pub art_fit: ArtFit,
    pub hover_flash: bool,   // 0x100 true, 0xE2 false
    pub art_sink_y: f32,     // 0xE2 = PRESSED_CONTENT_OFFSET_Y, 0x100 = 0.0
    pub text_sink_y: f32,    // 0xE2 = PRESSED_CONTENT_OFFSET_Y, 0x100 = 0.0
    pub disabled_dim: bool,  // 0x100 true (alpha 0.502), 0xE2 false
}

/// One owner-draw button to paint: its cell rect + current per-control state.
#[derive(Clone, Copy)]
pub struct PaintButton {
    pub rect: RectPx,
    pub pressed: bool,
    pub hovered: bool,
    pub enabled: bool,
    /// First-paint slide frame, or None for steady-state.
    pub wave_frame: Option<usize>,
}

/// One static or button label to paint.
pub struct PaintLabel<'a> {
    pub text: &'a str,
    pub rect: RectPx,            // already inset/sunk by the caller-side builder
    pub align: ShellAlign,
    pub rgb: [f32; 3],
}
```

### 1.3 Pass entry points

Three free functions (no trait needed for two shells — a trait adds indirection with no second implementor yet; a `ButtonPolicy` struct is the cleaner Rust for "two shells, parameterized"):

```rust
/// Emit the right-panel chrome (SDTP top / SDBTNBKGD tile column / SDBTM bottom
/// clipped) + lower strip, in C8 order, at CHROME_DEPTH. Identical for 0xE2/0x100.
pub fn paint_chrome(
    atlas: &MainMenuShellChromeAtlas,
    panel: crate::ui::shell::geom::RightPanelRects,
    lower_strip: Option<RectPx>,   // None for shells with no lower strip (future 0x102)
    screen_w: i32,
) -> Vec<SpriteInstance>;

/// Emit the owner-draw buttons at BUTTON_DEPTH, applying the per-shell policy
/// (frame select 2/3/4 or wave frame, art fit, art sink, disabled dim).
pub fn paint_buttons(
    atlas: &MainMenuShellChromeAtlas,
    buttons: &[PaintButton],
    policy: ButtonPolicy,
    now: Instant,
    hover_started_at: Option<Instant>,
) -> Vec<SpriteInstance>;

/// Emit one text draw per label at TEXT_DEPTH via shell_text::draw_in_rect.
/// Color/inset/sink are pre-applied by the caller into each PaintLabel.
pub fn paint_labels(
    font: &crate::render::bit_font::BitFont,
    labels: &[PaintLabel<'_>],
) -> Vec<ShellTextDraw>;
```

Inside `paint_buttons`, the frame-select logic that today is duplicated is unified once:

```rust
fn select_frame(
    atlas: &MainMenuShellChromeAtlas, b: &PaintButton, policy: ButtonPolicy,
    now: Instant, hover_started_at: Option<Instant>,
) -> Option<MainMenuShellChromeEntry> {
    if let Some(idx) = b.wave_frame {
        // clamp-down-one (verbatim from both emitters)
        return atlas.button_wave_frames.get(idx).copied().flatten()
            .or_else(|| atlas.button_wave_frames.get(idx.saturating_sub(1)).copied().flatten());
    }
    if b.pressed { return Some(atlas.button_pressed); }            // frame 4
    if policy.hover_flash && b.hovered {                            // frame 3, ~1 Hz
        let flash = hover_started_at
            .map(|s| now.duration_since(s).as_millis() / 1000 % 2 == 1)
            .unwrap_or(false);
        if flash { return Some(atlas.button_hover); }
    }
    Some(atlas.button_default)                                      // frame 2
}
```

**Why this reproduces both shells bit-for-bit:**
- 0xE2 passes `hover_flash: false`, so the hover branch is dead → only frames 2/4, exactly as `button_frame(atlas, pressed)` (`:67-76`).
- 0x100 passes `hover_flash: true` with `hover_started_at` from controller/state → frames 2/3/4 via the same `%1000%2` math (`:156-159`).
- Wave-frame clamp is verbatim from both (`:117-118`, `:185-186`).

The emit step inside `paint_buttons`:

```rust
let frame = match select_frame(...) { Some(f) => f, None => continue }; // None => hold, draw nothing
let alpha = if !b.enabled && policy.disabled_dim { BUTTON_DISABLED_ALPHA } else { 1.0 };
let (pos, size) = match policy.art_fit {
    ArtFit::Native => {
        let sink = if b.pressed { policy.art_sink_y } else { 0.0 };
        ([b.rect.x as f32, b.rect.y as f32 + sink], frame.pixel_size)
    }
    ArtFit::FitRightAnchored { panel_w, tile_h } => {
        let sx = b.rect.w as f32 / panel_w;
        let sy = b.rect.h as f32 / tile_h;
        let fw = frame.pixel_size[0] * sx;
        let fh = frame.pixel_size[1] * sy;
        let x = b.rect.x as f32 + (b.rect.w as f32 - fw);
        let y = b.rect.y as f32 + (b.rect.h as f32 - fh) * 0.5; // NO press sink (art_sink_y=0)
        ([x, y], [fw, fh])
    }
};
out.push(SpriteInstance { position: pos, size, uv_origin: frame.uv_origin,
    uv_size: frame.uv_size, depth: BUTTON_DEPTH, tint: [1.0,1.0,1.0], alpha, ..Default::default() });
```

Note: the wave-frame path in 0x100 today ALSO fit-scales + right-anchors + dims (`:190-205`) — that falls out naturally because `ArtFit::FitRightAnchored` + `disabled_dim` apply regardless of whether the frame came from wave or steady. In 0xE2 the wave path draws native un-dimmed (`:118-126`) — also falls out (`ArtFit::Native`, `disabled_dim:false`). **Byte-identical to both.**

### 1.4 Thin callers — what `app_*_shell_render.rs` become

The two `render_*_to_target` functions keep ALL of their existing structure (movie step, camera update, buffer creation, render-pass descriptor with `CLEAR_COLOR` + depth clear `1.0`, the per-text-draw scissor loop, the cursor-last draw, the `Fallback` early-outs). Only the **instance-building helpers** are deleted and replaced by calls into `shell_paint`, plus a small per-shell "builder" that maps the shell's layout + state into `PaintButton`/`PaintLabel`.

Main-menu caller body (replaces `build_parent_background_instances`/`build_movie_instances`/`build_chrome_instances`/`build_button_instances`/`build_text_draws` calls at `:535-549`):

```rust
let background_instances = build_parent_background_instances(chrome, &layout); // KEPT local: 0xE2-only MNSCRN bg
let movie_instances = vec![movie_instance(&layout)];                          // KEPT local (trivial)
let chrome_instances = shell_paint::paint_chrome(
    chrome, layout.right_panel, Some(layout.lower_strip), layout.screen.w);
let buttons = main_menu_paint_buttons(&layout, &state.main_menu_shell_state, wave.as_ref());
let button_instances = shell_paint::paint_buttons(
    chrome, &buttons, MAIN_MENU_BUTTON_POLICY, Instant::now(), None);
let labels = main_menu_paint_labels(state, &layout); // title/version/tooltip + button labels
let text_draws = shell_paint::paint_labels(&state.bit_font, &labels);
```

where `MAIN_MENU_BUTTON_POLICY = ButtonPolicy { art_fit: ArtFit::Native, hover_flash: false, art_sink_y: 2.0, text_sink_y: 2.0, disabled_dim: false }`, and `main_menu_paint_buttons` builds `PaintButton`s with `pressed = state.pressed == Some(id)`, `hovered/enabled` per current code, `wave_frame = wave.map(|w| w.sdbtnanm_frame(slot, ButtonGroup::A) as usize)`.

`main_menu_paint_labels` reproduces `build_text_draws:276-320` exactly: per-button label rect `{ x + x_offset, y + 1 + text_sink_y_i32, (w-2).max(0), (h-1).max(0) }` with `x_offset = pressed? pressed_content_offset_x : 0` and `text_sink_y_i32 = pressed? 2 : 0`, color always #FFFF00, align `H_CENTER|V_CENTER`; then title (`H_CENTER`), version line (`H_CENTER`), and tooltip line if hovered (`H_CENTER`).

Single-player caller mirrors this with `SP_BUTTON_POLICY = ButtonPolicy { art_fit: ArtFit::FitRightAnchored { panel_w: 168.0, tile_h: 42.0 }, hover_flash: true, art_sink_y: 0.0, text_sink_y: 0.0, disabled_dim: true }`, passing `Instant::now()` and `state.single_player_shell_state.hover_started_at`. Its `sp_paint_labels` reproduces `build_text_draws:278-323`: per-button rect `{ x + x_offset, y + 1, (w-2).max(0), (h-1).max(0) }` (NO y sink), color `enabled? #FFFF00 : #9F0000`, plus the single title (`H_CENTER`, #FFFF00). The `enabled` flag uses the existing `LoadSavedGame0x689 || load_saved_game_enabled` guard (`:284-285`).

The shell-specific small builders (`main_menu_paint_buttons`/`main_menu_paint_labels`, `sp_paint_buttons`/`sp_paint_labels`) stay in each `app_*_shell_render.rs` because they read shell-specific layout/state types (`MainMenuShellLayout` vs `SinglePlayerShellLayout`, the two `*ControlId` enums, the two `*ShellState` structs). They are pure mapping layers (layout+state → `Vec<PaintButton>`/`Vec<PaintLabel>`) with no GPU types and no geometry math — the geometry now lives once in `shell_paint`.

---

## 2. C8 / C9 / C10 reproduced EXACTLY — every constant carried over verbatim

### C8 — composition order (per-shell, owned by the caller's draw sequence)

Draw order is enforced by the caller emitting buffers in this sequence (each at its depth; the render pass uses passthrough so submission order is the tiebreaker the current code already relies on):

| Layer | 0xE2 depth | 0x100 depth | Source |
|---|---|---|---|
| parent background (MNSCRN) | 0.00098 | — (none) | `app_main_menu_shell_render.rs:21`; 0x100 has no parent bg |
| movie (RA2TS) | 0.00095 | 0.00095 | `:22` / `:18` |
| chrome (SDTP/SDBTNBKGD×n/SDBTM clipped + LWSCRN strip) | 0.00085 | 0.00085 | `:23` / `:19` |
| buttons (SDBTNANM) | 0.00080 | 0.00080 | `:30` / `:20` |
| text (labels + statics) | 0.00070 | 0.00070 | `:31` / `:21` |
| cursor (Default SHP frame 0) | 0.00001 | 0.00001 | `:34` / `:24` |

The clear is `CLEAR_COLOR` + depth `1.0` (`:598-605` / `:467-476`). **0xE2 keeps its parent-background buffer drawn first** (it has no analog in 0x100); the pass's `paint_chrome` does NOT emit the parent bg — that stays a local `build_parent_background_instances` call in the 0xE2 caller (it is 0xE2-unique: MNSCRNS at w==640 / MNSCRNL otherwise, at `shell_origin` letterbox, native size). Inverting right-panel vs background order would change the visible result (study §5 flag); the caller's submission order preserves it.

### C9 — button art frames + pressed sink + hover flash

| Constant | Value | Applies to | Source |
|---|---|---|---|
| SDBTNANM frame: default | 2 (`atlas.button_default`) | both | `main_menu_shell_chrome.rs:121` |
| SDBTNANM frame: hover | 3 (`atlas.button_hover`) | **0x100 only** | `:122`; selected only when `hover_flash` |
| SDBTNANM frame: pressed | 4 (`atlas.button_pressed`) | both | `:123` |
| Hover flash cadence | `elapsed_ms / 1000 % 2 == 1` | 0x100 only | `app_single_player_shell_render.rs:156-159` |
| `PRESSED_CONTENT_OFFSET_Y` (art sink) | 2.0 | **0xE2 art only** | `app_main_menu_shell_render.rs:40, 80-86, 102` |
| Art Y sink on 0x100 | **0.0** (no sink) | 0x100 | `app_single_player_shell_render.rs:70-95` (absent) |
| Fit scale (0x100) | `rect.w/168`, `rect.h/42` | 0x100 | `:79-80, 190-191` |
| Right-anchor x (0x100) | `rect.x + (rect.w - frame_w)` | 0x100 | `:83, 194` |
| V-center y (0x100) | `rect.y + (rect.h - frame_h)*0.5` | 0x100 | `:84, 195` |
| Native art (0xE2) | `frame.pixel_size` at `(rect.x, rect.y + sink)` | 0xE2 | `:101-103` |
| Wave clamp-down-one | `get(idx).flatten().or(get(idx-1))` | both | `:117-118` / `:185-186` |
| Disabled button alpha | `0x80/255` ≈ 0.502 | **0x100 only** | `:29, 92, 203` |
| SDBTNANM cell width | 156 (0xE2) / 168 (0x100) | per-shell | `paint_chrome`/layout — Slice 0 `cell_w` |

The pressed-sink (+2y on art) is **0xE2-only** and is carried as `ButtonPolicy.art_sink_y`. The +1x text shift is carried as the existing `layout.pressed_content_offset_x` (=1) applied in the label builders, for BOTH shells.

### C10 — text color permutation

| Constant | Value | Applies to | Source |
|---|---|---|---|
| Enabled text | `[1.0, 1.0, 0.0]` (#FFFF00) | both | `app_main_menu_shell_render.rs:35` / `app_single_player_shell_render.rs:27` |
| Disabled text | `[0x9F/255, 0.0, 0.0]` (#9F0000) | **0x100 only** | `:28, 307-311` |
| Glyph blit | single 1-bpp, no shadow | both | `shell_text::draw_in_rect` (unchanged) |
| Text rect inset | top `+1`, right `-2` (`w-2`, `h-1`) | both | `:287-292` / `:295-300` |
| Text X shift on press | `+ pressed_content_offset_x` (=1) | both | `:279-283` / `:287-294` |
| Text Y sink on press | `+2` | **0xE2 only** | `:286, 289` (0x100 has none) |
| Per-text scissor | set before each text draw | both | `:646-651` / `:508-513` |

### Per-shell differences that MUST stay parameterized

1. **Cell width 156 vs 168** → `ButtonPolicy.art_fit` carries `panel_w` for the fit path; the cell rect itself already comes from the layout (Slice 0 `cell_w`). Never collapse to one width.
2. **Disabled dimming (#9F0000 text + alpha 0.502)** → 0x100 only, gated by `disabled_dim` + `enabled` flag from the `LoadSavedGame` guard. 0xE2 never disables.
3. **Hover flash (frame 3)** → 0x100 only (`hover_flash: true`); 0xE2 `false`.
4. **Pressed art sink (+2y)** → 0xE2 only (`art_sink_y: 2.0`); 0x100 `0.0`.
5. **Statics** → 0xE2 has title+version+tooltip; 0x100 has title only. Built per-shell in `*_paint_labels`.
6. **Parent background (MNSCRN)** → 0xE2 only; stays a local helper in the 0xE2 caller, not in `paint_chrome`.

---

## 3. ONE atlas pack, zero changed packed pixels

No change to `render/main_menu_shell_chrome.rs` and no change to `src/app.rs`'s startup pack load. The `MainMenuShellChromeAtlas` is already built once and shared by both shells via `state.main_menu_shell_chrome` (the two render files both read the same `chrome` handle: `app_main_menu_shell_render.rs:525-528`, `app_single_player_shell_render.rs:399-402`). Slice 3 keeps consuming that single atlas read-only through the `&MainMenuShellChromeAtlas` parameter on every `shell_paint` function. **No atlas-pack copy is created or deleted** — the task's "retire one atlas-pack copy" refers to the *render-emitter* duplication (the two `push_entry_*`/`build_chrome_instances` copies), which IS retired here; the actual packed texture was never duplicated. The acceptance check confirms the atlas log line is unchanged (same pieces, same dimensions).

---

## 4. What new data the `ControlDescriptor` needs — verdict: NONE this slice

**Verdict: keep all paint policy render-side (`render/shell_paint.rs`), add nothing to `ui::shell::descriptor`.**

Justification against the `ui → render` rule and against scope:

- **Art frame indices (2/3/4), pixel sink offsets, fit-scale, disabled alpha, depths** are all render concepts (atlas frames, pixel/UV math, alpha blending). Putting them in `ui::shell` would not *break* the layering rule by itself (they are plain numbers), but it would put **render semantics into the render-agnostic layer**, which is exactly the "internals are not the spec, but keep the seam clean" principle — `descriptor.rs`'s own doc-comment says it is "render-agnostic data describing a dialog." A frame index is meaningless without the atlas. So they belong in `shell_paint`.
- **The enable/disable runtime state** the pass needs already exists in the right place: `ControlDescriptor.enabled` (static template default, `descriptor.rs:96-99`) + `DialogController.set_disabled`/`disabled` (runtime, `controller.rs:138-146, 167`). The pass receives the *resolved* `enabled` boolean per `PaintButton` from the caller (which combines descriptor default + controller runtime). No new descriptor field.
- **Hover/press state** already lives in `DialogController` (`pressed()`, `hovered()`, `hover_started_at()` — `controller.rs:119-129`) and, today, in the per-shell `*ShellState`. The caller threads these into `PaintButton`. No new descriptor field.
- **The one borderline candidate — a per-shell `ButtonPolicy`** (art_fit / hover_flash / sink / dim) — is a *render* policy (how art is fit, which frame flashes, how disabled looks). It is constructed as a `const` in each render caller (`MAIN_MENU_BUTTON_POLICY` / `SP_BUTTON_POLICY`). Putting it in `descriptor.rs` would import render meaning into `ui/`. **Keep it render-side.**

**Net: `ui::shell::descriptor` and `ui::shell::layout` are NOT edited in Slice 3.** This is the cleanest layering outcome — the descriptor stays a pure dialog-shape table, and all GPU-flavored policy stays in `render/`.

(Forward note for Slice 4: when skirmish controls fold in, `ControlKind` already distinguishes Button/Static/Checkbox/etc.; the paint pass will branch on `kind` then. That is a render-side branch reading a `ui`-side enum — still `render → ui`. No `ui`-side render data needed.)

---

## 5. Exact retire/replace edits, keyed to VERIFIED current line ranges

> Re-verify by **content/function name** immediately before each edit (these files are render-hot; a parallel session may shift line numbers). Anchors below are the function signatures, not bare line numbers.

### 5.1 NEW file `src/render/shell_paint.rs`
Create per §1.2–§1.3 (constants, `ArtFit`, `ButtonPolicy`, `PaintButton`, `PaintLabel`, `paint_chrome`, `paint_buttons`, `select_frame`, `paint_labels`, plus the verbatim `push_entry_sized`/`push_entry_rect`/`push_clipped_top` helpers lifted from either emitter — they are byte-identical between the two: compare `app_main_menu_shell_render.rs:47-65, 188-227` to `app_single_player_shell_render.rs:36-54, 208-243`). Include a `#[cfg(test)]` module porting the existing emitter unit tests (see §6.3).

### 5.2 `src/render/mod.rs`
Add `pub mod shell_paint;` to the module list (alphabetical near `shell_text`/`shell_transition_pass`).

### 5.3 `src/app_main_menu_shell_render.rs`
- **DELETE** (now in `shell_paint`): `push_entry_sized` (`:47-65`), `button_frame` (`:67-76`), `pressed_content_offset_y` (`:78-86`), `push_button_shp` (`:88-104`), `push_button_wave_frame` (`:106-128`), `build_button_instances` (`:165-186`), `push_entry_rect` (`:188-202`), `push_clipped_top` (`:204-227`), `build_chrome_instances` (`:229-260`). DELETE the depth/color/sink consts that moved to `shell_paint` (`PARENT_BACKGROUND_DEPTH`…`PRESSED_CONTENT_OFFSET_Y`, `:21-40`) — re-import the ones still used locally (`PARENT_BACKGROUND_DEPTH`, `MOVIE_DEPTH`, `CURSOR_DEPTH`, `CLEAR`-adjacent) from `shell_paint`.
- **KEEP local** (0xE2-unique): `shell_origin` (`:330-342`), `select_parent_background`/`parent_background_entry`/`build_parent_background_instances` (`:344-389`), `movie_instance`/`build_movie_instances` (`:391-406`), `ensure_movie_for_current_layout` (`:408-454`), `menu_cursor_instance` (`:456-479`), `resolve_csf` (`:130-136`), `push_label` (`:138-163`) — but rewrite `push_label`/`build_text_draws` (`:262-323`) into the new `main_menu_paint_labels` that returns `Vec<PaintLabel>` consumed by `shell_paint::paint_labels`.
- **REPLACE** the `render_main_menu_shell_to_target` body's build section (`:535-549`) with the §1.4 calls. Keep the render-pass/buffer/scissor/cursor structure (`:551-665`) verbatim.

### 5.4 `src/app_single_player_shell_render.rs`
- **DELETE** (now in `shell_paint`): `push_entry_sized` (`:36-54`), `button_frame` (`:56-68`), `push_button_shp` (`:70-95`), `build_button_instances` (`:133-173`), `push_button_wave_frame` (`:175-206`), `push_entry_rect` (`:208-222`), `push_clipped_top` (`:224-243`), `build_chrome_instances` (`:245-276`). DELETE the consts that moved (`:18-29`); re-import `MOVIE_DEPTH`/`CURSOR_DEPTH` from `shell_paint`.
- **KEEP local**: `resolve_csf` (`:97-103`), `push_label` (`:105-131`) → fold into `sp_paint_labels` returning `Vec<PaintLabel>`; `movie_instance` (`:328-339`), `shell_cursor_instance` (`:341-364`); `render_single_player_shell` outer (`:366-394`).
- **REPLACE** the build section (`:409-421`) with the §1.4 SP calls (`paint_chrome`, `paint_buttons` with `SP_BUTTON_POLICY` + `Instant::now()` + `hover_started_at`, `paint_labels` from `sp_paint_labels`). Keep the render-pass/buffer/scissor/cursor structure (`:423-528`) verbatim.

### 5.5 Do NOT touch
`render/main_menu_shell_chrome.rs`, `src/app.rs`, `app_skirmish_shell_render/*`, `ui/shell/*`, `ui/main_menu_shell/*`, `ui/single_player_shell/*`, `render/shell_text.rs`, `render/shell_transition_pass.rs`, `app_shell_transition.rs`.

---

## 6. Acceptance

### 6.1 Screenshot diff == 0 per state, BOTH shells (the bar)
Capture pre-refactor baselines first (before any edit), then post-refactor, and diff:
- **0xE2** at 800×600 and 1024×768, three states: idle (no hover/press), hover (cursor over a button — confirm NO frame-3 flash, since 0xE2 never flashes), pressed (mouse-down on a button — art+text sink +2y, text +1x). Plus a first-paint slide capture (buttons mid-ramp) and the tooltip line on hover.
- **0x100** at 800×600 and 1024×768, three states: idle, hover (confirm ~1 Hz frame-3 flash present), pressed (art does NOT sink, text shifts +1x only). Plus the LoadSavedGame-disabled state (dimmed art alpha 0.502 + #9F0000 text) when no saves exist, and a first-paint slide capture.
- A side-by-side diff must show **0 changed pixels** for every state on both shells.

### 6.2 `cargo build`/`check`/`test` clean (separate bounded pass)
Run `cargo check -p vera20k` then `cargo test -p vera20k` as a separate foreground pass after the edit window (not inside it). Read the literal `test result:` line. Watch for: a missed import of `shell_paint::*` consts in either caller; a borrow conflict from passing `&state.main_menu_shell_chrome` and `&state.bit_font` (both immutable borrows — fine, matches current code).

### 6.3 Keep the existing render path testable
Port the two emitters' unit tests into `shell_paint`'s `#[cfg(test)]`:
- `pressed_button_sinks_content_two_px_down` (`app_main_menu_shell_render.rs:676-684`) → assert `ArtFit::Native` + `art_sink_y: 2.0` produces +2y when pressed, 0 otherwise.
- `button_shp_draws_native_size_at_rect_top_left` (`:726-760`) → assert `ArtFit::Native` emits native `pixel_size` at `(644,199)` unpressed, `(644,201)` pressed (no X shift).
- A NEW test for `ArtFit::FitRightAnchored`: a 168×42 cell with a 156×42 native frame → scaled to fill (sx=1.0), right-anchored x = `rect.x + (168-168) = rect.x`, v-centered, NO press sink. Pin the 0x100 geometry so a future edit can't silently re-introduce a sink.
- `parent_background_renders_behind_movie` / `select_parent_background` / `shell_origin` / `movie_instance` tests stay in the 0xE2 caller (they test 0xE2-local helpers that did not move).
- A NEW `select_frame` test: `hover_flash: false` never returns `button_hover` even when `hovered`; `hover_flash: true` returns it on the high phase; pressed always returns `button_pressed`; wave_frame clamps down one.

---

## 7. Parallel-session safety

- **Minimize the write window.** Land all of §5 (new file + both caller rewrites) in one tight batch, then run the verify pass. The two `app_*_shell_render.rs` are render-hot and another session may be mid-edit.
- **Re-verify by content before editing.** Before deleting any helper, confirm the function signature still matches the anchors in §5 (e.g. `fn push_button_shp(out, atlas, rect, pressed, depth)` for 0xE2 vs `fn push_button_shp(out, atlas, rect, pressed, hover_highlight, enabled)` for 0x100 — they differ; do not cross-wire). If a signature has changed under you, STOP and re-anchor; do not "fix" another session's in-progress code (per CLAUDE.md parallel-sessions rule).
- **If `cargo check` fails in files you did NOT touch** (`ui/skirmish_shell/*`, `app_skirmish_shell_render/*`, unrelated `app.rs`), assume it's another session's work — continue or wait, don't revert.
- **Rollback is contained:** new file + two callers + one `mod.rs` line. `git restore` the two callers, delete `render/shell_paint.rs`, drop the `pub mod shell_paint;` line. No `ui/`, no atlas, no sim/app state touched.

---

## 8. Open questions for the human

1. **Trait vs functions.** This plan uses three free functions + a `ButtonPolicy` struct rather than the `OwnerDrawControl` *trait* the design doc names (§5 Slice 3 row). With only two shells and no per-control-kind dispatch yet, a trait adds an empty abstraction. A trait becomes worthwhile at Slice 4 (skirmish combo/trackbar/checkbox/listbox = real `ControlKind` variants). **OK to ship Slice 3 as functions and introduce the trait at Slice 4, or do you want the trait shape now for forward-compat?**
2. **0x100 wave-path art on disabled buttons.** Today 0x100's wave frames are drawn fit-scaled, right-anchored, AND dimmed if disabled (`:148-153, 203`), while a disabled button still slides in. The unified `paint_buttons` reproduces this (dim applies regardless of wave vs steady). **Confirm this is the intended observable — a disabled LoadSavedGame button visibly slides in dimmed during first-paint** — or is that a pre-existing quirk you want left exactly as-is (default: leave as-is, zero-pixel-change bar requires it)?
3. **Hover-flash clock source.** Both shells today derive flash from `Instant::now()` per frame against `hover_started_at` (wall clock). This is render-time, non-deterministic, and fine for a menu (not sim). The pass keeps `Instant` (no change). **Confirm you do not want this moved onto a deterministic frame/tick counter** (it would be a behavior change to the flash cadence and thus break the zero-diff bar, so default is: keep `Instant`).
4. **Const ownership.** §1.2 hoists the depth/color/sink consts into `shell_paint` and re-imports them in the callers. The 0xE2 caller still needs `PARENT_BACKGROUND_DEPTH` for its local parent-bg helper. **OK to re-export it from `shell_paint`** (single source of truth), or keep `PARENT_BACKGROUND_DEPTH` local to 0xE2 since it's 0xE2-unique? (Default: hoist all depths to `shell_paint` so the compose order lives in one place; assert `PARENT_BACKGROUND_DEPTH > MOVIE_DEPTH` in a test there.)

---

### Plan correctness note (load-bearing)
The task's contract list framed C9 pressed-sink as uniform "+2y/+1x." Live src proves it is **0xE2-only on the art** (0x100 has zero art sink and zero text Y sink; only the +1x text shift is shared). This plan preserves each shell's *actual* behavior via `ButtonPolicy.art_sink_y`/`text_sink_y` rather than applying a uniform sink — applying a uniform +2y would have moved every 0x100 button and label, failing the zero-diff bar. This is the single most important divergence to get right in Slice 3.

---

# POST-REVIEW REQUIRED CHANGES (apply during implementation; judge + 2 reviewers = GO)

These tighten the zero-pixel bar; none block GO. Open question resolved: ship Slice 3 as
**free functions + a ButtonPolicy struct** (NOT a trait); the trait arrives at Slice 4 when
skirmish needs per-ControlKind dispatch.

1. **SP enabled-gate on press AND hover.** In the single-player button builder set
   PaintButton.pressed = (enabled && pressed_owner_draw_button==Some(id)) and
   PaintButton.hovered = (enabled && hovered_owner_draw_button==Some(id)) -- match
   app_single_player_shell_render.rs:155-156 so a disabled LoadSavedGame can never paint
   frame 4 (pressed) or flash frame 3.
2. **Both callers pass Some(layout.lower_strip) to paint_chrome, never None.** The
   Option<RectPx> is for a future shell; both current shells always emit the LWSCRN strip.
3. **Text Y-sink stays i32 in the label builder, separate from the f32 art_sink_y.** On
   0xE2 press the label rect is {x+x_offset, y+1+(pressed?2:0), (w-2).max(0),(h-1).max(0)}
   with the +2 applied as i32 (matching pressed_content_offset_y(pressed) as i32), NOT
   routed through the float art path. Keep art_sink_y (f32) and text_sink_y (i32) distinct
   even though both = 2 on 0xE2.
4. **Per-shell sink + art-fit must stay parameterized:** 0xE2 = art_sink_y 2.0 / text_sink_y 2
   / ArtFit::Native; 0x100 = art_sink_y 0.0 / text_sink_y 0 / ArtFit::FitRightAnchored
   (scale_x=rect.w/168, scale_y=rect.h/42, x=rect.x+(rect.w-frame_w), y v-centered). text
   sink_x on press = layout.pressed_content_offset_x (=1) both shells. A single uniform sink
   would move every 0x100 control -- the #1 trap.
5. **Fix the FitRightAnchored pinning test against the REAL SDBTNANM.SHP canvas width.** The
   art frame_w = frame.pixel_size[0] (native canvas_w read at parse time in
   main_menu_shell_chrome.rs), NOT a hardcoded 156/168. Read the real canvas width (or assert
   via the actual atlas entry) before writing the test so it pins true 0x100 geometry, not a
   contradictory placeholder. The pass body is already safe (reads frame.pixel_size).
6. **Narrow the const-delete range.** Move only PARENT_BACKGROUND_DEPTH, MOVIE_DEPTH,
   CHROME_DEPTH, BUTTON_DEPTH, TEXT_DEPTH, CURSOR_DEPTH, the #FFFF00 text const, and
   PRESSED_CONTENT_OFFSET_Y to shell_paint. KEEP SHELL_LETTERBOX_W/H_THRESHOLD and
   SHELL_BASE_W/H local to the main-menu file (used by shell_origin).
7. **Re-anchor every delete/edit range to FUNCTION NAME, re-verify the signature by content
   immediately before editing** (render-hot, parallel-session edits). Keep the two
   push_button_shp signatures distinct: 0xE2 = (out,atlas,rect,pressed,depth); 0x100 =
   (out,atlas,rect,pressed,hover_highlight,enabled). Do not cross-wire.
8. **Acceptance must verify caller buffer-submission order byte-exact** (passthrough render;
   submission order is the tiebreaker): 0xE2 = parent-bg -> movie -> chrome -> buttons ->
   text -> cursor; 0x100 = movie -> chrome -> buttons -> text -> cursor. Parent-bg (MNSCRN)
   stays 0xE2-only + FIRST. Assert the chrome-atlas log line ("...atlas: WxH px, N pieces")
   is byte-identical pre/post (no repack). Flash invariants: 0xE2 NO frame-3 flash
   (hover_flash=false); 0x100 ~1 Hz flash keyed off state.single_player_shell_state
   .hover_started_at (NOT a fresh Instant::now()).

Layering/scope guards: shell_paint.rs lives in render/, adds NOTHING to ui::shell::descriptor,
and consumes only plain geom (RectPx/RightPanelRects) + pre-resolved pressed/hovered/enabled
booleans threaded by the caller -- it must NOT call DialogController or re-derive hit-testing.
Do not touch ui/shell/*, the input shells, the DialogController, app_skirmish_shell_render/*,
or main_menu_shell_chrome.rs pack code.
