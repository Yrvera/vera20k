# Skirmish Shell Pixel-Parity Implementation Plan

> **For Codex:** Execute this plan task-by-task. Each task is self-contained. Do not commit unless the user explicitly asks for commits in the execution session.

**Goal:** Replace the current visible egui Skirmish setup screen with a dedicated pixel-parity Skirmish shell layer that reproduces the researched gamemd.exe dialog 0x102 viewport, right-panel layout, asset usage, and control coordinate behavior.

**Architecture:** This is UI/render/app work only. `ui/skirmish_shell` owns shell state, integer-pixel layout, and hit testing; `render/skirmish_shell_chrome` owns retail shell asset loading and atlas entries; app-level glue routes input/rendering and converts Start Game into the existing loading transition. No `sim/` code changes are part of this plan.

**Design Doc:** `docs/plans/2026-05-16-skirmish-shell-pixel-parity-design.md`

---

## Grounding Summary

- The visible Skirmish setup screen is currently egui-based in `src/ui/main_menu.rs` and is called from `GameScreen::MainMenu` in `src/app.rs`.
- The approved design replaces that visible path, while keeping the existing loading transition and `SkirmishSettings` launch contract.
- Prior Skirmish shell reports identify dialog resource `0x102` as the active offline Skirmish dialog and provide exact resource/control geometry, owner-draw callback mapping, right-panel anchoring, and asset dimensions.
- Live Ghidra verification confirmed `FUN_006AE2C0` is called by `Main_Game`, creates dialog `0x102` through `FUN_00622650`, and exits on Start `0x617` or Back `0x5C0`.
- Live Ghidra verification confirmed `FUN_0060C4A0` moves the Skirmish dialog to `(0,0,g_ScreenWidth,g_ScreenHeight)`.
- Live Ghidra verification confirmed `FUN_0060B1D0` right-anchors selected controls using the 800x600-centered right-panel formula.
- Live Ghidra verification confirmed `FUN_0060B350` computes Back button placement from `SDBTNANM.SHP` dimensions and right-panel globals.
- Live Ghidra verification confirmed `FUN_00775690` converts child HWND screen rects to main shell client/backbuffer coordinates by subtracting `g_hWnd` client origin.
- Existing repo pattern to mirror: sidebar uses render-agnostic layout/hit testing in `src/sidebar/`, asset atlases in `src/render/sidebar_chrome.rs`, and app-level instance builders in `src/app_sidebar_build.rs`.
- `AssetManager::new` already extracts nested MIX archives and exposes `get_ref`, `get_with_source_ref`, and `archive`; shell assets can use this without adding a new archive system.
- Menu shell chrome must load before any map is selected. Current map-load code creates an `AssetManager` inside `load_map`, so this plan adds startup shell asset loading rather than waiting for `load_map`.
- INI-driven shell-visible data comes from `rulesmd.ini` `[Countries]`, `[Sides]`, `[Colors]`, and `[MultiplayerDialogSettings]`; raw geometry and SHP/PCX dimensions come from the binary/resources/assets, not INI.
- No Skirmish-specific INI layout scale/origin key was found; resolution behavior is driven by screen dimensions and `[Video] ScreenWidth`, `[Video] ScreenHeight`, `AllowHiResModes` in the original.
- Open after planning: exact text rendering and every combo/dropdown behavior can be implemented incrementally, but layout rects and Start/Back/Choose/preview/flags must be present from the first visible shell pass.

## Key Technical Decisions

- Dedicated shell module split: `ui/skirmish_shell` for state/layout/input and `render/skirmish_shell_chrome` for assets. **Confidence:** high
  - **Source:** design doc; repo pattern `src/sidebar/mod.rs`, `src/render/sidebar_chrome.rs`, `src/app_sidebar_build.rs`.
- Integer pixel layout is primary; convert to `f32` only when emitting `SpriteInstance`. **Confidence:** high
  - **Source:** `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`; live Ghidra `FUN_0060B1D0`, `FUN_0060B350`.
- Startup shell assets load through a new `AssetManager::new` call in `App::initialize`, because the main menu renders before `load_map`. **Confidence:** high
  - **Source:** repo code `src/app.rs` initialization and `src/app_init.rs::load_map`.
- Keep `SkirmishSettings` as the launch bridge for the first implementation. **Confidence:** high
  - **Source:** current `src/ui/main_menu.rs` and `src/app_init.rs::load_map` signature.
- Do not route the shell through egui. Loading and other non-shell screens may keep egui. **Confidence:** high
  - **Source:** design doc; parity constraint from owner-draw/background reports.
- Shell setup is standard YR active path, not TS legacy. **Confidence:** high
  - **Source:** Skirmish reports all mark active in YR; live Ghidra caller `Main_Game -> FUN_006AE2C0`.
- Asset atlas should include right-panel SHPs and owner-draw PCX pieces as separate named entries rather than merging them into one hardcoded screen bitmap. **Confidence:** medium-high
  - **Source:** `SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`; exact per-control PCX composition remains broader than the first integration pass.

Low-confidence decisions to verify with `/review-plan` before implementation starts:

- Whether the first implementation should render every owner-draw PCX composition exactly, or land shell geometry with key SHP/PCX surfaces first and then continue owner-draw composition in follow-up tasks. This plan keeps visible controls and rects in scope, and treats full combo/dropdown skin parity as a later task only if the selected control behavior is not yet researched enough.

## Open Questions

### Resolved During Planning

- Is dialog `0x102` active in standard YR Skirmish? Resolved yes. `FUN_006AE2C0` is called by `Main_Game` and creates the dialog through `FUN_00622650`; prior reports mark the path active in YR.
- Does the Skirmish shell use a child/modal/custom host? Resolved: child/modeless under `g_hWnd`, then resized to full shell client by `FUN_0060C4A0`.
- Does 1024x768 scale the dialog? Resolved: no uniform scale; right-panel children are centered/anchored through post-creation helpers.
- Does the current app already have a native sprite UI pattern to mirror? Resolved: sidebar layout/chrome/instance-builder split is the closest fit.
- Are layout coordinates INI-driven? Resolved: no Skirmish-specific layout INI key; use binary/resource-recovered geometry.

### Deferred to Implementation

- Exact 640x480 live visual confirmation: formula is verified, but screenshot verification should happen once the Rust shell can run at 640x480.
- Exact font/text parity for all localized shell labels: current plan can use existing FNT/sidebar text infrastructure as a starting point, then compare against retail screenshots.
- Complete owner-draw dropdown/listbox animation and scrollbar behavior: reports identify callback behavior, but implementing every popup detail can be phased after the main visible shell replaces egui.
- Exact original shell background composition beyond the right-panel pieces: reports identify background asset candidates and active shell surfaces; visual capture comparison should guide final composition tuning.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/ui/skirmish_shell/mod.rs` | Public shell model API and module exports |
| Create | `src/ui/skirmish_shell/layout.rs` | Dialog 0x102 integer-pixel layout, control IDs, rect formulas, tests |
| Create | `src/ui/skirmish_shell/state.rs` | Shell state, actions, settings bridge, hit testing, state updates |
| Modify | `src/ui/mod.rs` | Export `skirmish_shell` and update module comment away from egui-only wording |
| Create | `src/assets/pcx_file.rs` | Minimal 8-bit PCX parser for retail shell owner-draw art |
| Modify | `src/assets/mod.rs` | Export `pcx_file` |
| Create | `src/render/skirmish_shell_chrome.rs` | Shell SHP/PCX asset loading, RGBA conversion, atlas entries |
| Modify | `src/render/mod.rs` | Export `skirmish_shell_chrome` |
| Create | `src/app_skirmish_shell_render.rs` | App-layer shell instance builder and render pass |
| Modify | `src/lib.rs` | Export `app_skirmish_shell_render` |
| Modify | `src/app.rs` | Add shell state/assets to `AppState`, initialize assets, route main menu input/rendering |
| Modify | `src/app_init.rs` | Keep `load_map` launch contract compatible with shell state through `SkirmishSettings`; only change if type path moves |
| Optional modify | `src/ui/main_menu.rs` | Retain launch data types and loading screen; remove only unused egui Skirmish drawing when compile warnings demand it |

## Interface Changes

- New `crate::ui::skirmish_shell::SkirmishShellState`, owned by `AppState`.
- New `crate::ui::skirmish_shell::SkirmishShellLayout`, computed per frame from render dimensions.
- New `crate::ui::skirmish_shell::ShellControlId`, preserving original dialog control ids.
- New `crate::ui::skirmish_shell::SkirmishShellAction`, returned by hit testing and state updates.
- New `crate::render::skirmish_shell_chrome::SkirmishShellChromeAtlas`, owned by `AppState`.
- New app helper `crate::app_skirmish_shell_render::render_skirmish_shell`, called only from `GameScreen::MainMenu`.
- `AppState` gains shell fields. Existing `SkirmishSettings` remains the launch handoff to `load_map`.

## Sim Checklist

This plan does not touch `src/sim/`.

- [x] No new sim math.
- [x] No deterministic state hash changes.
- [x] No new sim dependency on render/ui/sidebar/audio/net.
- [x] No tick ordering impact.
- [x] No `EntityStore` iteration impact.

## Risk Areas

- Startup asset loading may fail if RA2/YR path config is invalid. The app should log the missing shell asset reason and render a minimal error shell rather than panic in normal startup.
- Current dirty working tree has unrelated modified files. Do not revert them. If they affect compile during execution, report them as unrelated unless the user asks to fix them.
- Replacing egui `MainMenu` changes input ownership. Mouse coordinates must match the same render coordinate space used by shell sprites, especially when upscale mode is enabled.
- App render code currently assumes `MainMenu` is egui-only. The new path must still submit a frame, present the surface, and leave loading/mission-result/pause egui behavior intact.
- Asset precedence matters. Prefer YR `*md` archives where present and base RA2 fallback through existing `AssetManager` search order.
- Control geometry is parity-critical. Avoid `ui_scale`, layout flex, DPI scaling, or proportional sizing in the shell layout model.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | Dialog 0x102 shell origin `(0,0)` and full shell size | Every visible control and background alignment depends on the shell origin | Unit tests for layout; Ghidra `FUN_0060C4A0` |
| Task 1 | DLU-derived base rects with Win32 rounding | One-pixel differences are visible in button/slot alignment | Tests for Start/Choose/Back base rects |
| Task 1 | Right-anchor formula for `0x617`, `0x5AA`, `0x468` | Right panel must anchor/center at 800x600, 1024x768, 640x480 | Tests for all three resolutions |
| Task 1 | Back rect from `SDBTNANM.SHP` and right-panel tile globals | Back is not at its resource-template position | Tests for final Back rects |
| Task 1 | Color combos and flags remain resource-positioned | Player rows must not move with the right panel | Tests that row controls retain base positions |
| Task 4 | Shell asset dimensions | Button and right-panel sprites must be sampled at original pixel sizes | Asset dimension test/tool output |
| Task 5 | Preview static `0x468` final rect | Start markers must render in child HWND-derived shell/backbuffer coords | Screenshot and unit rect tests |
| Task 6 | Mouse hit testing in shell pixel coordinates | Clicks must land on the same controls the player sees | Hit-test unit tests and manual click test |
| Task 7 | MainMenu no longer uses egui layout | Pixel parity is impossible if egui determines layout | Visual screenshot inspection |
| Task 9 | 800x600 and 1024x768 screenshots | Validates player-visible anchoring and background composition | Side-by-side with report rect overlays |

---

## Tasks

### Task 1: Create `ui/skirmish_shell` Layout Types And Verified Rect Tests

**Why:** Establish the parity-critical coordinate contract before any rendering or input code consumes it.

**Files:**
- Create: `src/ui/skirmish_shell/mod.rs`
- Create: `src/ui/skirmish_shell/layout.rs`
- Modify: `src/ui/mod.rs`

**Pattern:** Mirrors `src/sidebar/mod.rs` plus `src/sidebar/sidebar_view.rs`: render-agnostic rects, layout computation, and unit tests in the UI/layout layer.

**Step 1: Add module export in `src/ui/mod.rs`**

```rust
pub mod skirmish_shell;
```

Also update the module doc comment so it no longer says all `ui/` screens are egui-based. The invariant to keep is that `ui/` remains render-agnostic.

**Step 2: Create `src/ui/skirmish_shell/mod.rs`**

```rust
//! Pixel-parity Skirmish shell model and layout.
//!
//! This module owns render-agnostic dialog 0x102 geometry, state, and hit
//! testing. Rendering code consumes the computed rects from the app/render
//! layers; this module does not depend on assets or wgpu.

mod layout;
mod state;

pub use layout::{
    ColorComboId, RectPx, RIGHT_PANEL_WIDTH, ShellControlId, SkirmishShellLayout,
    compute_layout,
};
pub use state::{
    SkirmishShellAction, SkirmishShellOpponent, SkirmishShellState, apply_action, hit_test,
    launch_settings,
};
```

**Step 3: Create `src/ui/skirmish_shell/layout.rs` with integer rect types and constants**

Use these exact constants for the first pass:

```rust
//! Dialog 0x102 shell layout recovered from gamemd.exe.

pub const SHELL_BASE_W: i32 = 800;
pub const SHELL_BASE_H: i32 = 600;
pub const RIGHT_PANEL_WIDTH: i32 = 168;
pub const SDBTNANM_W: i32 = 156;
pub const SDBTNANM_H: i32 = 42;
pub const SDBTNBKGD_H: i32 = 42;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectPx {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl RectPx {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellControlId {
    StartGame0x617,
    ChooseMap0x5aa,
    Back0x5c0,
    MapPreview0x468,
    PlayerName0x6a0,
    PlayerColor0x6a2,
    AiColor0x522,
    AiColor0x523,
    AiColor0x524,
    AiColor0x525,
    AiColor0x526,
    AiColor0x527,
    AiColor0x528,
    Flag0x6da,
    Flag0x6db,
    Flag0x6dc,
    Flag0x6dd,
    Flag0x6de,
    Flag0x6df,
    Flag0x6e0,
    Flag0x6e1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorComboId {
    Player,
    Ai(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RightPanelRects {
    pub top: RectPx,
    pub tile: RectPx,
    pub tile_count: i32,
    pub bottom: RectPx,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishShellLayout {
    pub screen: RectPx,
    pub right_panel: RightPanelRects,
    pub start_button: RectPx,
    pub choose_map_button: RectPx,
    pub back_button: RectPx,
    pub map_preview: RectPx,
    pub player_name: RectPx,
    pub color_combos: [RectPx; 8],
    pub flags: [RectPx; 8],
}
```

**Step 4: Implement DLU conversion and layout helpers in `layout.rs`**

```rust
const BASE_X: i32 = 6;
const BASE_Y: i32 = 13;

fn mul_div_round(n: i32, numer: i32, denom: i32) -> i32 {
    let value = n * numer;
    if value >= 0 {
        (value + denom / 2) / denom
    } else {
        (value - denom / 2) / denom
    }
}

fn dlu_rect(x: i32, y: i32, w: i32, h: i32) -> RectPx {
    RectPx::new(
        mul_div_round(x, BASE_X, 4),
        mul_div_round(y, BASE_Y, 8),
        mul_div_round(w, BASE_X, 4),
        mul_div_round(h, BASE_Y, 8),
    )
}

fn center_offset(screen: i32, base: i32) -> i32 {
    ((screen - base) / 2).max(0)
}

fn right_anchor(screen_w: i32, screen_h: i32, original: RectPx) -> RectPx {
    let offset_x = center_offset(screen_w, SHELL_BASE_W);
    let offset_y = center_offset(screen_h, SHELL_BASE_H);
    let inset = (RIGHT_PANEL_WIDTH - original.w) / 2;
    RectPx::new(
        screen_w - offset_x - original.w - inset,
        original.y + offset_y,
        original.w,
        original.h,
    )
}

fn right_panel_rects(screen_w: i32, screen_h: i32) -> RightPanelRects {
    let left_margin = if screen_w > 1023 {
        (screen_w - SHELL_BASE_W) / 2
    } else {
        0
    };
    let top_margin = if screen_h > 767 {
        (screen_h - SHELL_BASE_H) / 2
    } else {
        0
    };
    let effective_right = screen_w - left_margin;
    let top = RectPx::new(effective_right - RIGHT_PANEL_WIDTH, top_margin, 168, 199);
    let tile = RectPx::new(top.x, top.y + top.h, 168, SDBTNBKGD_H);
    let effective_h = if screen_h > 767 {
        screen_h - top_margin * 2
    } else {
        screen_h
    };
    let remaining = effective_h - top.h;
    let tile_count = (remaining / SDBTNBKGD_H).saturating_sub(0).min(9);
    let bottom_y = tile.y + tile_count * SDBTNBKGD_H;
    let bottom_h = screen_h - top_margin - bottom_y;
    RightPanelRects {
        top,
        tile,
        tile_count,
        bottom: RectPx::new(top.x, bottom_y, 168, bottom_h.max(0)),
    }
}

fn back_rect(screen_w: i32, panel: RightPanelRects) -> RectPx {
    let offset_x = center_offset(screen_w, SHELL_BASE_W);
    RectPx::new(
        screen_w - offset_x - SDBTNANM_W,
        panel.tile.y + (panel.tile_count - 1) * SDBTNBKGD_H,
        SDBTNANM_W,
        SDBTNANM_H,
    )
}
```

When implementing `right_panel_rects`, adjust the tile count formula so tests below pass exactly. The source of truth is the expected rect table, not a guessed simplification.

**Step 5: Implement `compute_layout` in `layout.rs`**

```rust
pub fn compute_layout(screen_w: u32, screen_h: u32) -> SkirmishShellLayout {
    let screen_w = screen_w as i32;
    let screen_h = screen_h as i32;

    let start_base = dlu_rect(425, 149, 108, 23);
    let choose_base = dlu_rect(425, 176, 108, 23);
    let preview_base = dlu_rect(429, 23, 96, 69);
    let panel = right_panel_rects(screen_w, screen_h);

    let color_combos = [
        dlu_rect(282, 36, 29, 73),
        dlu_rect(282, 52, 29, 73),
        dlu_rect(282, 68, 29, 73),
        dlu_rect(282, 84, 29, 73),
        dlu_rect(282, 100, 29, 73),
        dlu_rect(282, 116, 29, 73),
        dlu_rect(282, 132, 29, 73),
        dlu_rect(282, 148, 29, 73),
    ];
    let flags = [
        dlu_rect(150, 36, 32, 12),
        dlu_rect(150, 52, 32, 12),
        dlu_rect(150, 68, 32, 12),
        dlu_rect(150, 84, 32, 12),
        dlu_rect(150, 100, 32, 12),
        dlu_rect(150, 116, 32, 12),
        dlu_rect(150, 132, 32, 12),
        dlu_rect(150, 148, 32, 12),
    ];

    SkirmishShellLayout {
        screen: RectPx::new(0, 0, screen_w, screen_h),
        right_panel: panel,
        start_button: right_anchor(screen_w, screen_h, start_base),
        choose_map_button: right_anchor(screen_w, screen_h, choose_base),
        back_button: back_rect(screen_w, panel),
        map_preview: right_anchor(screen_w, screen_h, preview_base),
        player_name: dlu_rect(38, 36, 100, 14),
        color_combos,
        flags,
    }
}
```

**Step 6: Add layout tests in `layout.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::{RectPx, compute_layout};

    #[test]
    fn key_rects_match_800x600() {
        let layout = compute_layout(800, 600);
        assert_eq!(layout.start_button, RectPx::new(635, 242, 162, 37));
        assert_eq!(layout.choose_map_button, RectPx::new(635, 286, 162, 37));
        assert_eq!(layout.map_preview, RectPx::new(644, 37, 144, 112));
        assert_eq!(layout.back_button, RectPx::new(644, 535, 156, 42));
    }

    #[test]
    fn key_rects_match_1024x768() {
        let layout = compute_layout(1024, 768);
        assert_eq!(layout.start_button, RectPx::new(747, 326, 162, 37));
        assert_eq!(layout.choose_map_button, RectPx::new(747, 370, 162, 37));
        assert_eq!(layout.map_preview, RectPx::new(756, 121, 144, 112));
        assert_eq!(layout.back_button, RectPx::new(756, 619, 156, 42));
    }

    #[test]
    fn key_rects_match_640x480_formula() {
        let layout = compute_layout(640, 480);
        assert_eq!(layout.start_button, RectPx::new(475, 242, 162, 37));
        assert_eq!(layout.choose_map_button, RectPx::new(475, 286, 162, 37));
        assert_eq!(layout.map_preview, RectPx::new(484, 37, 144, 112));
        assert_eq!(layout.back_button, RectPx::new(484, 409, 156, 42));
    }

    #[test]
    fn color_combos_and_flags_do_not_right_anchor() {
        let layout_800 = compute_layout(800, 600);
        let layout_1024 = compute_layout(1024, 768);
        assert_eq!(layout_800.color_combos, layout_1024.color_combos);
        assert_eq!(layout_800.flags, layout_1024.flags);
        assert_eq!(layout_800.color_combos[0], RectPx::new(423, 59, 44, 119));
        assert_eq!(layout_800.flags[0], RectPx::new(225, 59, 48, 20));
    }
}
```

**Step 7: Verify**

Run: `cargo test skirmish_shell::layout -- --nocapture`

Expected: the new layout tests pass. If existing unrelated dirty files break compilation, record the exact unrelated errors and do not modify them in this task.

### Task 2: Add Skirmish Shell State, Actions, Settings Bridge, And Hit Testing

**Why:** The app needs a render-independent shell model before the visible egui menu can be replaced.

**Files:**
- Create: `src/ui/skirmish_shell/state.rs`
- Modify: `src/ui/skirmish_shell/mod.rs` only if exports need adjustment

**Pattern:** Mirrors `src/sidebar/sidebar_view.rs` hit-testing style: input point plus layout produces a semantic action; app code applies actions to state.

**Step 1: Define state and actions in `state.rs`**

```rust
//! Skirmish shell state and hit testing.

use crate::app_init::MapMenuEntry;
use crate::ui::main_menu::{SkirmishCountry, SkirmishSettings, StartPosition};

use super::layout::{ColorComboId, RectPx, ShellControlId, SkirmishShellLayout};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkirmishShellAction {
    None,
    StartGame,
    BackOrExit,
    ChooseMap,
    SelectColor(ColorComboId),
    SelectMap(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishShellOpponent {
    pub enabled: bool,
    pub country: SkirmishCountry,
    pub color_index: usize,
    pub start_position: StartPosition,
    pub team: i32,
}

#[derive(Debug, Clone)]
pub struct SkirmishShellState {
    pub selected_map_idx: usize,
    pub player_country: SkirmishCountry,
    pub player_color_index: usize,
    pub player_start_position: StartPosition,
    pub starting_credits: i32,
    pub short_game: bool,
    pub zoom_enabled: bool,
    pub opponents: Vec<SkirmishShellOpponent>,
}
```

**Step 2: Implement default state from current launch defaults**

```rust
impl Default for SkirmishShellState {
    fn default() -> Self {
        let settings = SkirmishSettings::default();
        Self {
            selected_map_idx: settings.selected_map_idx,
            player_country: settings.player_country,
            player_color_index: 0,
            player_start_position: settings.start_position,
            starting_credits: settings.starting_credits,
            short_game: settings.short_game,
            zoom_enabled: settings.zoom_enabled,
            opponents: vec![SkirmishShellOpponent {
                enabled: true,
                country: settings.ai_country,
                color_index: 5,
                start_position: StartPosition::Random,
                team: 0,
            }],
        }
    }
}
```

**Step 3: Implement settings bridge**

```rust
pub fn launch_settings(state: &SkirmishShellState) -> SkirmishSettings {
    let ai_country = state
        .opponents
        .iter()
        .find(|opponent| opponent.enabled)
        .map(|opponent| opponent.country)
        .unwrap_or(SkirmishCountry::Russia);

    SkirmishSettings {
        selected_map_idx: state.selected_map_idx,
        player_country: state.player_country,
        ai_country,
        starting_credits: state.starting_credits,
        start_position: state.player_start_position,
        short_game: state.short_game,
        zoom_enabled: state.zoom_enabled,
    }
}
```

**Step 4: Implement hit testing**

```rust
fn hit_rect(rect: RectPx, x: i32, y: i32, action: SkirmishShellAction) -> SkirmishShellAction {
    if rect.contains(x, y) {
        action
    } else {
        SkirmishShellAction::None
    }
}

pub fn hit_test(layout: &SkirmishShellLayout, x: i32, y: i32) -> SkirmishShellAction {
    let start = hit_rect(layout.start_button, x, y, SkirmishShellAction::StartGame);
    if start != SkirmishShellAction::None {
        return start;
    }

    let choose = hit_rect(layout.choose_map_button, x, y, SkirmishShellAction::ChooseMap);
    if choose != SkirmishShellAction::None {
        return choose;
    }

    let back = hit_rect(layout.back_button, x, y, SkirmishShellAction::BackOrExit);
    if back != SkirmishShellAction::None {
        return back;
    }

    for (idx, rect) in layout.color_combos.iter().copied().enumerate() {
        if rect.contains(x, y) {
            return if idx == 0 {
                SkirmishShellAction::SelectColor(ColorComboId::Player)
            } else {
                SkirmishShellAction::SelectColor(ColorComboId::Ai(idx - 1))
            };
        }
    }

    SkirmishShellAction::None
}
```

**Step 5: Implement state application**

```rust
pub fn apply_action(
    state: &mut SkirmishShellState,
    action: SkirmishShellAction,
    maps: &[MapMenuEntry],
) -> SkirmishShellAction {
    match action {
        SkirmishShellAction::None => SkirmishShellAction::None,
        SkirmishShellAction::StartGame => SkirmishShellAction::StartGame,
        SkirmishShellAction::BackOrExit => SkirmishShellAction::BackOrExit,
        SkirmishShellAction::ChooseMap => {
            if !maps.is_empty() {
                state.selected_map_idx = (state.selected_map_idx + 1) % maps.len();
            }
            SkirmishShellAction::None
        }
        SkirmishShellAction::SelectMap(idx) => {
            if idx < maps.len() {
                state.selected_map_idx = idx;
            }
            SkirmishShellAction::None
        }
        SkirmishShellAction::SelectColor(target) => {
            match target {
                ColorComboId::Player => state.player_color_index = (state.player_color_index + 1) % 8,
                ColorComboId::Ai(idx) => {
                    if let Some(opponent) = state.opponents.get_mut(idx) {
                        opponent.color_index = (opponent.color_index + 1) % 8;
                    }
                }
            }
            SkirmishShellAction::None
        }
    }
}
```

**Step 6: Add hit-test tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::skirmish_shell::compute_layout;

    #[test]
    fn hit_test_start_choose_and_back() {
        let layout = compute_layout(800, 600);
        assert_eq!(hit_test(&layout, 636, 243), SkirmishShellAction::StartGame);
        assert_eq!(hit_test(&layout, 636, 287), SkirmishShellAction::ChooseMap);
        assert_eq!(hit_test(&layout, 645, 536), SkirmishShellAction::BackOrExit);
    }

    #[test]
    fn hit_test_uses_exclusive_bottom_right_edges() {
        let layout = compute_layout(800, 600);
        assert_eq!(hit_test(&layout, 635 + 162, 242), SkirmishShellAction::None);
        assert_eq!(hit_test(&layout, 635, 242 + 37), SkirmishShellAction::None);
    }

    #[test]
    fn launch_settings_preserves_current_load_contract() {
        let shell = SkirmishShellState::default();
        let settings = launch_settings(&shell);
        assert_eq!(settings.selected_map_idx, shell.selected_map_idx);
        assert_eq!(settings.starting_credits, shell.starting_credits);
        assert_eq!(settings.short_game, shell.short_game);
    }
}
```

**Step 7: Verify**

Run: `cargo test skirmish_shell -- --nocapture`

Expected: layout and state tests pass, subject only to unrelated pre-existing build failures.

### Task 3: Add Minimal PCX Parser And Shell Chrome Asset Atlas Loader

**Why:** The renderer needs original shell/right-panel/control art as named atlas entries before it can draw a pixel shell.

**Files:**
- Create: `src/assets/pcx_file.rs`
- Modify: `src/assets/mod.rs`
- Create: `src/render/skirmish_shell_chrome.rs`
- Modify: `src/render/mod.rs`

**Pattern:** Mirrors existing asset parsers in `src/assets/` for a small self-contained parser, then mirrors `src/render/sidebar_chrome.rs` for loading art through `AssetManager`, converting to RGBA, packing into a single `BatchTexture`, and retaining UV/pixel-size entries.

**Step 1: Export the PCX parser**

Add to `src/assets/mod.rs`:

```rust
pub mod pcx_file;
```

**Step 2: Create `src/assets/pcx_file.rs`**

```rust
//! Minimal PCX parser for RA2 shell owner-draw art.
//!
//! Supports the retail 8-bit, one-plane, RLE-compressed PCX files used by
//! shell controls. The parser keeps embedded VGA palettes in 8-bit RGB.

use crate::assets::error::AssetError;

#[derive(Debug, Clone)]
pub struct PcxFile {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
    pub palette: [[u8; 3]; 256],
}

impl PcxFile {
    pub fn from_bytes(data: &[u8]) -> Result<Self, AssetError> {
        if data.len() < 128 + 769 {
            return Err(pcx_error("PCX too short"));
        }
        if data[0] != 0x0A || data[2] != 1 || data[3] != 8 {
            return Err(pcx_error("Unsupported PCX header"));
        }
        let x_min = u16::from_le_bytes([data[4], data[5]]);
        let y_min = u16::from_le_bytes([data[6], data[7]]);
        let x_max = u16::from_le_bytes([data[8], data[9]]);
        let y_max = u16::from_le_bytes([data[10], data[11]]);
        let planes = data[65];
        if planes != 1 {
            return Err(pcx_error("Only 1-plane PCX is supported"));
        }
        let bytes_per_line = u16::from_le_bytes([data[66], data[67]]) as usize;
        let width = x_max
            .checked_sub(x_min)
            .and_then(|v| v.checked_add(1))
            .ok_or_else(|| pcx_error("Invalid PCX width"))?;
        let height = y_max
            .checked_sub(y_min)
            .and_then(|v| v.checked_add(1))
            .ok_or_else(|| pcx_error("Invalid PCX height"))?;
        if data[data.len() - 769] != 0x0C {
            return Err(pcx_error("Missing PCX VGA palette"));
        }

        let expected_scan = bytes_per_line * height as usize;
        let encoded = &data[128..data.len() - 769];
        let mut scan = Vec::with_capacity(expected_scan);
        let mut i = 0usize;
        while i < encoded.len() && scan.len() < expected_scan {
            let byte = encoded[i];
            i += 1;
            if byte & 0xC0 == 0xC0 {
                if i >= encoded.len() {
                    return Err(pcx_error("Truncated PCX RLE run"));
                }
                let count = (byte & 0x3F) as usize;
                let value = encoded[i];
                i += 1;
                scan.extend(std::iter::repeat(value).take(count));
            } else {
                scan.push(byte);
            }
        }
        if scan.len() < expected_scan {
            return Err(pcx_error("PCX RLE stream ended early"));
        }

        let mut pixels = Vec::with_capacity(width as usize * height as usize);
        for row in 0..height as usize {
            let start = row * bytes_per_line;
            pixels.extend_from_slice(&scan[start..start + width as usize]);
        }

        let mut palette = [[0u8; 3]; 256];
        let pal = &data[data.len() - 768..];
        for (idx, rgb) in palette.iter_mut().enumerate() {
            rgb.copy_from_slice(&pal[idx * 3..idx * 3 + 3]);
        }

        Ok(Self {
            width,
            height,
            pixels,
            palette,
        })
    }

    pub fn to_rgba(&self, transparent_index: Option<u8>) -> Vec<u8> {
        let mut rgba = Vec::with_capacity(self.pixels.len() * 4);
        for &idx in &self.pixels {
            let [r, g, b] = self.palette[idx as usize];
            let a = if transparent_index == Some(idx) { 0 } else { 255 };
            rgba.extend_from_slice(&[r, g, b, a]);
        }
        rgba
    }
}

fn pcx_error(detail: &str) -> AssetError {
    AssetError::ParseError {
        format: "PCX".to_string(),
        detail: detail.to_string(),
    }
}
```

**Step 3: Add PCX parser tests**

Add one hand-built tiny PCX test covering RLE and palette extraction:

```rust
#[cfg(test)]
mod tests {
    use super::PcxFile;

    #[test]
    fn parses_8bit_rle_pcx_with_embedded_palette() {
        let mut data = vec![0u8; 128];
        data[0] = 0x0A;
        data[2] = 1;
        data[3] = 8;
        data[8..10].copy_from_slice(&1u16.to_le_bytes());
        data[10..12].copy_from_slice(&1u16.to_le_bytes());
        data[65] = 1;
        data[66..68].copy_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&[0xC4, 1]);
        data.push(0x0C);
        let mut pal = vec![0u8; 768];
        pal[3] = 10;
        pal[4] = 20;
        pal[5] = 30;
        data.extend_from_slice(&pal);

        let pcx = PcxFile::from_bytes(&data).expect("pcx");
        assert_eq!((pcx.width, pcx.height), (2, 2));
        assert_eq!(pcx.pixels, vec![1, 1, 1, 1]);
        assert_eq!(pcx.palette[1], [10, 20, 30]);
        assert_eq!(pcx.to_rgba(None)[0..4], [10, 20, 30, 255]);
    }
}
```

**Step 4: Export the render module**

Add to `src/render/mod.rs`:

```rust
pub mod skirmish_shell_chrome;
```

**Step 5: Define entries and atlas in `src/render/skirmish_shell_chrome.rs`**

```rust
//! Skirmish shell chrome atlas.
//!
//! Loads retail shell/right-panel SHP and PCX art used by the dialog 0x102
//! Skirmish setup screen, then packs it into a GPU texture for batched drawing.

use crate::assets::asset_manager::AssetManager;
use crate::assets::pal_file::Palette;
use crate::assets::pcx_file::PcxFile;
use crate::assets::shp_file::ShpFile;
use crate::render::batch::{BatchRenderer, BatchTexture};
use crate::render::gpu::GpuContext;

const ATLAS_PADDING: u32 = 2;

#[derive(Debug, Clone, Copy)]
pub struct SkirmishShellChromeEntry {
    pub uv_origin: [f32; 2],
    pub uv_size: [f32; 2],
    pub pixel_size: [f32; 2],
}

pub struct SkirmishShellChromeAtlas {
    pub texture: BatchTexture,
    pub sd_top: Option<SkirmishShellChromeEntry>,
    pub sd_tile: Option<SkirmishShellChromeEntry>,
    pub sd_bottom: Option<SkirmishShellChromeEntry>,
    pub sd_button_anim: Option<SkirmishShellChromeEntry>,
    pub sd_map_button: Option<SkirmishShellChromeEntry>,
    pub background_large: Option<SkirmishShellChromeEntry>,
    pub background_small: Option<SkirmishShellChromeEntry>,
    pub button_up_left_30: Option<SkirmishShellChromeEntry>,
    pub button_up_mid_30: Option<SkirmishShellChromeEntry>,
    pub button_up_right_30: Option<SkirmishShellChromeEntry>,
    pub button_down_left_30: Option<SkirmishShellChromeEntry>,
    pub button_down_mid_30: Option<SkirmishShellChromeEntry>,
    pub button_down_right_30: Option<SkirmishShellChromeEntry>,
    pub start_marker: Option<SkirmishShellChromeEntry>,
    pub preview_marker: Option<SkirmishShellChromeEntry>,
    pub flags: Vec<(String, SkirmishShellChromeEntry)>,
}
```

**Step 6: Implement a local rendered-entry staging type**

```rust
struct RenderedShellEntry {
    label: String,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}
```

**Step 7: Implement SHP rendering helper**

```rust
fn render_shp_entry(
    assets: &AssetManager,
    file_name: &str,
    palette: &Palette,
    frame: usize,
) -> Option<RenderedShellEntry> {
    let bytes = assets.get_ref(file_name)?;
    let shp = ShpFile::from_bytes(bytes).ok()?;
    let rgba = shp.frame_to_rgba(frame, palette).ok()?;
    Some(RenderedShellEntry {
        label: file_name.to_ascii_lowercase(),
        width: shp.width as u32,
        height: shp.height as u32,
        rgba,
    })
}
```

Use the actual `ShpFile` field names from `src/assets/shp_file.rs`; if the fields are `frame_width`/`frame_height`, use those names. The expected dimensions must remain checked against the research table.

**Step 8: Implement PCX rendering helper**

```rust
fn render_pcx_entry(
    assets: &AssetManager,
    file_name: &str,
    transparent_index: Option<u8>,
) -> Option<RenderedShellEntry> {
    let bytes = assets.get_ref(file_name)?;
    let pcx = PcxFile::from_bytes(bytes).ok()?;
    Some(RenderedShellEntry {
        label: file_name.to_ascii_lowercase(),
        width: pcx.width as u32,
        height: pcx.height as u32,
        rgba: pcx.to_rgba(transparent_index),
    })
}
```

**Step 9: Implement atlas packing**

Pack entries with a simple shelf layout, matching the sidebar atlas style:

```rust
fn pack_entries(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    entries: &[RenderedShellEntry],
) -> Option<(BatchTexture, Vec<SkirmishShellChromeEntry>)> {
    if entries.is_empty() {
        return None;
    }

    let atlas_width = 1024u32;
    let mut x = 0u32;
    let mut y = 0u32;
    let mut row_h = 0u32;
    let mut placements = Vec::with_capacity(entries.len());

    for entry in entries {
        if x + entry.width + ATLAS_PADDING > atlas_width {
            x = 0;
            y += row_h + ATLAS_PADDING;
            row_h = 0;
        }
        placements.push((x, y));
        x += entry.width + ATLAS_PADDING;
        row_h = row_h.max(entry.height);
    }

    let atlas_height = (y + row_h).next_power_of_two().max(1);
    let mut rgba = vec![0u8; (atlas_width * atlas_height * 4) as usize];

    for (entry, (px, py)) in entries.iter().zip(placements.iter().copied()) {
        for row in 0..entry.height {
            let src = (row * entry.width * 4) as usize;
            let dst = (((py + row) * atlas_width + px) * 4) as usize;
            let len = (entry.width * 4) as usize;
            rgba[dst..dst + len].copy_from_slice(&entry.rgba[src..src + len]);
        }
    }

    let texture = batch.create_texture(gpu, &rgba, atlas_width, atlas_height);
    let atlas_entries = entries
        .iter()
        .zip(placements)
        .map(|(entry, (px, py))| SkirmishShellChromeEntry {
            uv_origin: [px as f32 / atlas_width as f32, py as f32 / atlas_height as f32],
            uv_size: [
                entry.width as f32 / atlas_width as f32,
                entry.height as f32 / atlas_height as f32,
            ],
            pixel_size: [entry.width as f32, entry.height as f32],
        })
        .collect();
    Some((texture, atlas_entries))
}
```

**Step 10: Implement `build_skirmish_shell_chrome_atlas`**

```rust
pub fn build_skirmish_shell_chrome_atlas(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    assets: &AssetManager,
) -> Option<SkirmishShellChromeAtlas> {
    let palette_bytes = assets
        .get_ref("sidebar.pal")
        .or_else(|| assets.get_ref("SHELL.PAL"))
        .or_else(|| assets.get_ref("DIALOG.PAL"))?;
    let palette = Palette::from_bytes(palette_bytes).ok()?;

    let mut rendered = Vec::new();
    let mut push = |entry: Option<RenderedShellEntry>| {
        if let Some(entry) = entry {
            rendered.push(entry);
        }
    };

    push(render_shp_entry(assets, "SDTP.SHP", &palette, 0));
    push(render_shp_entry(assets, "SDBTNBKGD.SHP", &palette, 0));
    push(render_shp_entry(assets, "SDBTM.SHP", &palette, 0));
    push(render_shp_entry(assets, "SDBTNANM.SHP", &palette, 0));
    push(render_shp_entry(assets, "SDMPBTN.SHP", &palette, 0));
    push(render_shp_entry(assets, "MNSCRNL.SHP", &palette, 0));
    push(render_shp_entry(assets, "MNSCRNS.SHP", &palette, 0));
    push(render_shp_entry(assets, "STARTBUT.SHP", &palette, 0));
    push(render_shp_entry(assets, "mmpb.shp", &palette, 0));
    push(render_pcx_entry(assets, "bue_li30.pcx", Some(0)));
    push(render_pcx_entry(assets, "bue_mi30.pcx", Some(0)));
    push(render_pcx_entry(assets, "bue_ri30.pcx", Some(0)));
    push(render_pcx_entry(assets, "bde_li30.pcx", Some(0)));
    push(render_pcx_entry(assets, "bde_mi30.pcx", Some(0)));
    push(render_pcx_entry(assets, "bde_ri30.pcx", Some(0)));
    push(render_pcx_entry(assets, "usai.pcx", Some(0)));
    push(render_pcx_entry(assets, "rusi.pcx", Some(0)));
    push(render_pcx_entry(assets, "yrii.pcx", Some(0)));
    push(render_pcx_entry(assets, "obsi.pcx", Some(0)));

    let (texture, entries) = pack_entries(gpu, batch, &rendered)?;
    let mut iter = entries.into_iter();
    Some(SkirmishShellChromeAtlas {
        texture,
        sd_top: iter.next(),
        sd_tile: iter.next(),
        sd_bottom: iter.next(),
        sd_button_anim: iter.next(),
        sd_map_button: iter.next(),
        background_large: iter.next(),
        background_small: iter.next(),
        button_up_left_30: iter.next(),
        button_up_mid_30: iter.next(),
        button_up_right_30: iter.next(),
        button_down_left_30: iter.next(),
        button_down_mid_30: iter.next(),
        button_down_right_30: iter.next(),
        start_marker: iter.next(),
        preview_marker: iter.next(),
        flags: vec![
            ("usai.pcx".to_string(), iter.next()?),
            ("rusi.pcx".to_string(), iter.next()?),
            ("yrii.pcx".to_string(), iter.next()?),
            ("obsi.pcx".to_string(), iter.next()?),
        ],
    })
}
```

Execution note: keep the `rendered` push order and `iter.next()` assignment order aligned. If any mandatory SHP is missing, return `None` and log the filename. If an optional flag PCX is missing, omit only that flag entry and keep rendering the shell chrome.

**Step 11: Add dimension assertions as a helper test where possible**

Use a unit test for pure name/dimension constants if retail assets are not available in CI. If retail assets are available locally, use an ignored test:

```rust
#[test]
#[ignore]
fn retail_shell_shp_dimensions_match_research() {
    let config = crate::config::GameConfig::load().expect("game config");
    let assets = AssetManager::new(&config.paths.ra2_dir).expect("asset manager");
    let palette = Palette::from_bytes(assets.get_ref("sidebar.pal").expect("sidebar.pal"))
        .expect("palette");
    let sdbtn = render_shp_entry(&assets, "SDBTNANM.SHP", &palette, 0).expect("SDBTNANM");
    assert_eq!((sdbtn.width, sdbtn.height), (156, 42));
}
```

**Step 12: Verify**

Run: `cargo test pcx_file -- --nocapture`

Run: `cargo check`

Run retail asset check manually when asset path is configured: `cargo test retail_shell_shp_dimensions_match_research -- --ignored --nocapture`

Expected: PCX parser tests pass; shell chrome module compiles; ignored asset test passes locally when retail assets are present.

### Task 4: Add App-Layer Shell Sprite Instance Builder

**Why:** Keep render pass glue out of `ui/` and avoid bloating `app.rs` with shell drawing details.

**Files:**
- Create: `src/app_skirmish_shell_render.rs`
- Modify: `src/lib.rs`

**Pattern:** Mirrors `src/app_sidebar_build.rs` and `src/app_sidebar_render.rs`: app layer may depend on UI state, render atlas, and batch renderer.

**Step 1: Export the app helper**

Add to `src/lib.rs` with the other app modules:

```rust
pub mod app_skirmish_shell_render;
```

**Step 2: Define helper functions in `src/app_skirmish_shell_render.rs`**

```rust
//! Skirmish shell sprite construction and render pass.
//!
//! Part of the app layer: may depend on ui and render modules. Keeps the
//! `GameScreen::MainMenu` branch in `app.rs` small.

use crate::app::AppState;
use crate::render::batch::SpriteInstance;
use crate::render::skirmish_shell_chrome::{SkirmishShellChromeAtlas, SkirmishShellChromeEntry};
use crate::ui::skirmish_shell::{RectPx, SkirmishShellLayout, compute_layout};

fn push_entry(
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

fn push_entry_native(
    out: &mut Vec<SpriteInstance>,
    entry: SkirmishShellChromeEntry,
    x: i32,
    y: i32,
    depth: f32,
) {
    out.push(SpriteInstance {
        position: [x as f32, y as f32],
        size: entry.pixel_size,
        uv_origin: entry.uv_origin,
        uv_size: entry.uv_size,
        depth,
        tint: [1.0, 1.0, 1.0],
        alpha: 1.0,
        ..Default::default()
    });
}
```

**Step 3: Build shell instances**

```rust
pub fn build_skirmish_shell_instances(
    atlas: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
) -> Vec<SpriteInstance> {
    let mut instances = Vec::new();

    if let Some(bg) = if layout.screen.w <= 640 {
        atlas.background_small
    } else {
        atlas.background_large
    } {
        push_entry_native(&mut instances, bg, 0, 0, 0.00090);
    }

    if let Some(top) = atlas.sd_top {
        push_entry(&mut instances, top, layout.right_panel.top, 0.00080);
    }

    if let Some(tile) = atlas.sd_tile {
        for row in 0..layout.right_panel.tile_count {
            let rect = RectPx::new(
                layout.right_panel.tile.x,
                layout.right_panel.tile.y + row * layout.right_panel.tile.h,
                layout.right_panel.tile.w,
                layout.right_panel.tile.h,
            );
            push_entry(&mut instances, tile, rect, 0.00079);
        }
    }

    if let Some(bottom) = atlas.sd_bottom {
        push_entry(&mut instances, bottom, layout.right_panel.bottom, 0.00078);
    }

    if let Some(button) = atlas.sd_button_anim {
        push_entry(&mut instances, button, layout.back_button, 0.00060);
    }

    instances
}
```

Execution note: extend this builder as more atlas entries are loaded. The first visible pass must draw right-panel pieces and the key button/preview regions at the researched coordinates.

**Step 4: Add render pass function**

```rust
pub fn render_skirmish_shell(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
) -> anyhow::Result<crate::ui::skirmish_shell::SkirmishShellAction> {
    let layout = compute_layout(state.render_width(), state.render_height());
    let action = crate::ui::skirmish_shell::SkirmishShellAction::None;

    let Some(atlas) = state.skirmish_shell_chrome.as_ref() else {
        return Ok(action);
    };

    let instances = build_skirmish_shell_instances(atlas, &layout);
    state.batch_renderer.update_camera(
        &state.gpu,
        state.render_width() as f32,
        state.render_height() as f32,
        0.0,
        0.0,
        1.0,
    );

    let Some((buffer, count)) = state
        .batch_renderer
        .create_instance_buffer(&state.gpu, &instances)
    else {
        return Ok(action);
    };

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Skirmish Shell"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(crate::app_types::CLEAR_COLOR),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    state
        .batch_renderer
        .draw_with_buffer(&mut pass, &atlas.texture, &buffer, count);
    drop(pass);

    Ok(action)
}
```

If `RenderPassDescriptor` fields differ in the pinned `wgpu` version, mirror the exact shape used by `src/app_sidebar_render.rs` or `src/app_render/*` in this repo.

**Step 5: Verify**

Run: `cargo check`

Expected: app helper compiles. It is not wired into `MainMenu` yet.

### Task 5: Initialize Shell State And Startup Assets In `AppState`

**Why:** The shell must render on the main menu before any map is loaded, so state and chrome must exist during app initialization.

**Files:**
- Modify: `src/app.rs`

**Pattern:** Mirrors existing startup initialization for `BatchRenderer`, `SidebarTextRenderer`, `software_cursor`, and app state fields.

**Step 1: Add fields to `AppState`**

Add near existing `skirmish_settings` / UI fields:

```rust
pub(crate) skirmish_shell_state: crate::ui::skirmish_shell::SkirmishShellState,
pub(crate) skirmish_shell_chrome:
    Option<crate::render::skirmish_shell_chrome::SkirmishShellChromeAtlas>,
```

**Step 2: Build a startup asset manager in `App::initialize`**

After `batch_renderer` is created and after `game_config` is loaded:

```rust
let startup_asset_manager = game_config
    .as_ref()
    .and_then(|cfg| match crate::assets::asset_manager::AssetManager::new(&cfg.paths.ra2_dir) {
        Ok(manager) => Some(manager),
        Err(err) => {
            log::warn!("Could not load startup shell assets: {err:#}");
            None
        }
    });

let skirmish_shell_chrome = startup_asset_manager.as_ref().and_then(|assets| {
    crate::render::skirmish_shell_chrome::build_skirmish_shell_chrome_atlas(
        &gpu,
        &batch_renderer,
        assets,
    )
});
```

Do not store this startup `AssetManager` in `AppState` unless the implementation needs it for later shell-only assets. The existing `load_map` path still creates its own `AssetManager` and returns it with `MapLoadResult`.

**Step 3: Initialize fields in `AppState`**

Inside the `Ok(AppState { ... })` literal:

```rust
skirmish_shell_state: crate::ui::skirmish_shell::SkirmishShellState::default(),
skirmish_shell_chrome,
```

Keep existing `skirmish_settings: SkirmishSettings::default()` for compatibility with `load_map` until Task 7 updates the launch bridge.

**Step 4: Verify**

Run: `cargo check`

Expected: new fields initialize cleanly; startup logs warn rather than panic when shell assets fail.

### Task 6: Route MainMenu Mouse Input To Skirmish Shell Hit Testing

**Why:** Clicks must be handled by the same rect model used for rendering.

**Files:**
- Modify: `src/app.rs`

**Pattern:** Mirrors current `SpawnPick` and `InGame` mouse routing in `window_event`, but uses shell hit testing for `GameScreen::MainMenu`.

**Step 1: Add a helper inside `impl App` or as a private function in `src/app.rs`**

```rust
fn handle_skirmish_shell_click(state: &mut AppState, event_loop: &ActiveEventLoop) {
    let layout = crate::ui::skirmish_shell::compute_layout(
        state.render_width(),
        state.render_height(),
    );
    let action = crate::ui::skirmish_shell::hit_test(
        &layout,
        state.cursor_x.round() as i32,
        state.cursor_y.round() as i32,
    );
    let action = crate::ui::skirmish_shell::apply_action(
        &mut state.skirmish_shell_state,
        action,
        &state.available_maps,
    );

    match action {
        crate::ui::skirmish_shell::SkirmishShellAction::StartGame => {
            let settings = crate::ui::skirmish_shell::launch_settings(&state.skirmish_shell_state);
            state.skirmish_settings = settings;
            let map_name = state
                .available_maps
                .get(state.skirmish_settings.selected_map_idx)
                .map(|m| m.file_name.clone())
                .unwrap_or_else(|| "auto".to_string());
            state.screen = GameScreen::Loading { map_name };
            state.zoom_level = 1.0;
            state.zoom_target = 1.0;
        }
        crate::ui::skirmish_shell::SkirmishShellAction::BackOrExit => {
            event_loop.exit();
        }
        crate::ui::skirmish_shell::SkirmishShellAction::None
        | crate::ui::skirmish_shell::SkirmishShellAction::ChooseMap
        | crate::ui::skirmish_shell::SkirmishShellAction::SelectColor(_)
        | crate::ui::skirmish_shell::SkirmishShellAction::SelectMap(_) => {}
    }
}
```

If this helper cannot borrow `event_loop` cleanly in the current `window_event` structure, return a small local enum from the helper and call `event_loop.exit()` in the match arm.

**Step 2: Update `WindowEvent::MouseInput`**

Add this branch before `SpawnPick` / `InGame` handling:

```rust
if state.screen == GameScreen::MainMenu {
    if button == MouseButton::Left && btn_state.is_pressed() {
        Self::handle_skirmish_shell_click(state, event_loop);
    }
} else if !egui_consumed && state.screen == GameScreen::SpawnPick {
    if button == MouseButton::Left && btn_state.is_pressed() {
        crate::app_spawn_pick::handle_spawn_pick_click(state);
    }
} else if !egui_consumed && state.screen == GameScreen::InGame {
    app_input::handle_mouse_input(state, button, btn_state);
}
```

Keep cursor coordinate scaling exactly as the existing `CursorMoved` code computes it; shell layout and cursor values both use render coordinates.

**Step 3: Verify**

Run: `cargo check`

Expected: click routing compiles. Manual verification happens after Task 7 draws the shell.

### Task 7: Replace MainMenu egui Rendering With Skirmish Shell Render Path

**Why:** This is the actual visible replacement requested by the user.

**Files:**
- Modify: `src/app.rs`
- Optional modify: `src/ui/main_menu.rs`

**Pattern:** MainMenu remains an app-level screen state, but its rendering is now native batch sprites like sidebar chrome rather than egui layout.

**Step 1: Replace the `GameScreen::MainMenu` render branch**

Replace the branch that calls `main_menu::draw_main_menu_with_maps` with:

```rust
GameScreen::MainMenu => {
    crate::app_skirmish_shell_render::render_skirmish_shell(
        state,
        &mut encoder,
        &view,
    )?;
}
```

Do not call `state.egui.begin_frame` or `end_frame_and_render` in this branch after replacement. Loading, InGame overlays, MissionResult, SpawnPick overlay, and Pause screens keep their existing egui flow.

**Step 2: Remove now-unused `MenuAction` import if needed**

If `MenuAction` is unused after the replacement, change:

```rust
use crate::ui::main_menu::{self, MenuAction, SkirmishSettings};
```

to:

```rust
use crate::ui::main_menu::{self, SkirmishSettings};
```

Keep `main_menu` imported if the loading screen still uses `main_menu::draw_loading_screen`.

**Step 3: Keep loading screen intact**

Do not alter the `GameScreen::Loading` branch in this task. It may remain egui until a separate shell-loading-screen parity design exists.

**Step 4: Verify**

Run: `cargo check`

Run the app if the working tree compiles: `cargo run`

Expected: the first visible screen is the custom Skirmish shell render path, not the egui card menu. If shell assets are missing, the frame should still clear and log the missing asset condition.

### Task 8: Add Shell Text, Button, Preview, And Flag Instance Coverage

**Why:** The shell must show the key player-visible controls, not only background/right-panel chrome.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs`
- Modify: `src/render/skirmish_shell_chrome.rs` if additional PCX/flag entries are needed

**Pattern:** Mirrors `src/app_sidebar_build.rs` text/cameo layering and `src/render/sidebar_text.rs` FNT-backed text generation.

**Step 1: Add button PCX entries to the chrome atlas**

Add fields for:

```rust
pub button_up_left_30: Option<SkirmishShellChromeEntry>,
pub button_up_mid_30: Option<SkirmishShellChromeEntry>,
pub button_up_right_30: Option<SkirmishShellChromeEntry>,
pub button_down_left_30: Option<SkirmishShellChromeEntry>,
pub button_down_mid_30: Option<SkirmishShellChromeEntry>,
pub button_down_right_30: Option<SkirmishShellChromeEntry>,
```

Load the retail names:

- `bue_li30.pcx`
- `bue_mi30.pcx`
- `bue_ri30.pcx`
- `bde_li30.pcx`
- `bde_mi30.pcx`
- `bde_ri30.pcx`

The normal unpressed state uses `bue_*30`; pressed can be added once button press state exists.

**Step 2: Add flag entries**

Load at least these flag PCXs:

- `usai.pcx`
- `rusi.pcx`
- `yrii.pcx`
- `obsi.pcx`

Store them in `flags: Vec<(String, SkirmishShellChromeEntry)>`. Use lowercase labels so lookup can be case-insensitive.

**Step 3: Draw composed 30px-family buttons**

Add a helper that draws left cap, tiled middle, and right cap into a destination rect. If tiling requires many quads, use repeated native-width quads and clip only when the final segment is narrower. The first implementation may stretch the middle cap only if visual verification flags it and a follow-up immediately replaces it with tiling; the preferred implementation is tiled because `FUN_006BA3E0` tiles middle pieces.

**Step 4: Draw key button labels with existing FNT text**

Use `state.sidebar_text.build_text` for labels until a dedicated shell text renderer is introduced. Draw:

- `Start Game` centered in `layout.start_button`
- `Choose Map` centered in `layout.choose_map_button`
- `Back` centered in `layout.back_button`

If CSF localization is available in `state.csf`, resolve `GUI:StartGame`, `GUI:ChooseMap`, and `GUI:Back`; otherwise use the English strings above.

**Step 5: Draw map preview backing and start marker surface**

Use `layout.map_preview` as the destination. In this task, draw a dark preview backing rect or the available preview texture if one already exists in `MapMenuEntry.preview`. The start marker asset `STARTBUT.SHP` must be drawn at `layout.map_preview`-relative projected positions once the map preview projection code is connected.

**Step 6: Draw row flags in the eight flag rects**

For each `layout.flags[idx]`, choose a flag entry from player/opponent country where known:

- player row: player country flag
- opponent rows: opponent country flag
- empty rows: observer/blank flag if available

Render at native aspect centered inside the researched flag rect. Do not move the rect itself.

**Step 7: Verify**

Run: `cargo check`

Run the app if possible and inspect:

- Start button at `(635,242,162,37)` for 800x600 render size.
- Choose Map at `(635,286,162,37)`.
- Back at `(644,535,156,42)`.
- Map preview at `(644,37,144,112)`.
- Flags start at the fixed resource-derived left table positions.

### Task 9: Add Layout And Hit-Test Regression Tests To Guard Future Drift

**Why:** Coordinate regressions are easy to introduce when render/window/upscale code changes.

**Files:**
- Modify: `src/ui/skirmish_shell/layout.rs`
- Modify: `src/ui/skirmish_shell/state.rs`

**Pattern:** Same-file `#[cfg(test)]` unit tests, matching existing Rust convention in `src/sidebar/sidebar_view.rs`.

**Step 1: Add right-panel global tests**

Test exact `right_panel` values:

```rust
#[test]
fn right_panel_globals_match_research_modes() {
    let a = compute_layout(800, 600);
    assert_eq!(a.right_panel.top, RectPx::new(632, 0, 168, 199));
    assert_eq!(a.right_panel.tile, RectPx::new(632, 199, 168, 42));
    assert_eq!(a.right_panel.tile_count, 9);
    assert_eq!(a.right_panel.bottom, RectPx::new(632, 577, 168, 23));

    let b = compute_layout(1024, 768);
    assert_eq!(b.right_panel.top, RectPx::new(744, 84, 168, 199));
    assert_eq!(b.right_panel.tile, RectPx::new(744, 283, 168, 42));
    assert_eq!(b.right_panel.tile_count, 9);
    assert_eq!(b.right_panel.bottom, RectPx::new(744, 661, 168, 23));

    let c = compute_layout(640, 480);
    assert_eq!(c.right_panel.top, RectPx::new(472, 0, 168, 199));
    assert_eq!(c.right_panel.tile, RectPx::new(472, 199, 168, 42));
    assert_eq!(c.right_panel.tile_count, 6);
    assert_eq!(c.right_panel.bottom, RectPx::new(472, 451, 168, 29));
}
```

**Step 2: Add large-screen no-scale test**

```rust
#[test]
fn large_screen_offsets_without_scaling() {
    let layout = compute_layout(1280, 960);
    assert_eq!(layout.start_button.w, 162);
    assert_eq!(layout.start_button.h, 37);
    assert_eq!(layout.map_preview.w, 144);
    assert_eq!(layout.map_preview.h, 112);
}
```

**Step 3: Add color hit-test tests**

```rust
#[test]
fn hit_test_color_combos() {
    let layout = compute_layout(800, 600);
    assert_eq!(
        hit_test(&layout, layout.color_combos[0].x, layout.color_combos[0].y),
        SkirmishShellAction::SelectColor(ColorComboId::Player)
    );
    assert_eq!(
        hit_test(&layout, layout.color_combos[1].x, layout.color_combos[1].y),
        SkirmishShellAction::SelectColor(ColorComboId::Ai(0))
    );
}
```

**Step 4: Verify**

Run: `cargo test skirmish_shell -- --nocapture`

Expected: all shell layout and hit-test tests pass.

### Task 10: Wire Shell Launch Data Into Existing Map Load Path

**Why:** The visible shell must start the selected map with the same data currently accepted by `load_map`.

**Files:**
- Modify: `src/app.rs`
- Modify: `src/app_init.rs` only if the `SkirmishSettings` type path moves

**Pattern:** Keep current `GameScreen::Loading { map_name }` transition and `load_map(..., &state.skirmish_settings, ...)` contract.

**Step 1: Confirm `load_map` still accepts `SkirmishSettings`**

Keep this signature unless later design changes it:

```rust
pub fn load_map(
    gpu: &GpuContext,
    batch: &BatchRenderer,
    requested_map: Option<&str>,
    skirmish_settings: &crate::ui::main_menu::SkirmishSettings,
    mut vxl_compute: Option<&mut crate::render::vxl_compute::VxlComputeRenderer>,
) -> Result<MapLoadResult>
```

If `SkirmishSettings` moved out of `main_menu.rs`, update the import path consistently and only move the type; do not alter map loading behavior in this task.

**Step 2: Before Start Game transition, copy shell settings**

Ensure Task 6's click handler includes:

```rust
state.skirmish_settings = crate::ui::skirmish_shell::launch_settings(
    &state.skirmish_shell_state,
);
```

This preserves all current downstream assumptions.

**Step 3: Verify**

Run: `cargo check`

If the app compiles, run `cargo run`, click Start Game, and verify the screen transitions to Loading with the selected map name.

### Task 11: Preserve Existing egui Loading And Non-Menu Screens

**Why:** This feature replaces Skirmish setup only. Loading, mission result, pause, save/load, and debug panels are outside the researched shell target.

**Files:**
- Modify: `src/app.rs` if Task 7 accidentally introduced broad egui removal
- Optional modify: `src/ui/main_menu.rs` to leave only shared types/loading screen

**Pattern:** Narrow replacement. Avoid broad UI refactors.

**Step 1: Inspect `GameScreen` branches**

Confirm these branches still call egui as before:

- `GameScreen::Loading`
- `GameScreen::InGame` overlays
- `GameScreen::MissionResult`
- `GameScreen::SpawnPick` overlay

**Step 2: Keep `draw_loading_screen` available**

If `src/ui/main_menu.rs` has unused Skirmish menu functions after Task 7, remove only unused functions that cause compiler warnings. Keep:

- `SkirmishSettings`
- `SkirmishCountry`
- `StartPosition`
- `CREDITS_OPTIONS`
- `draw_loading_screen`

**Step 3: Verify**

Run: `cargo check`

Expected: no accidental dependency on the old egui Skirmish menu path remains in `MainMenu`; loading screen still renders.

### Task 12: Visual Verification Across Resolutions

**Why:** The player-visible target is pixel layout, not just compilation.

**Files:**
- No source changes unless verification finds a mismatch

**Pattern:** Rendering verification against Ghidra report rects and retail behavior.

**Step 1: Run at default 1024x768**

Run: `cargo run`

Expected key rects:

- Start: `(747,326,162,37)`
- Choose Map: `(747,370,162,37)`
- Preview: `(756,121,144,112)`
- Back: `(756,619,156,42)`

Use a screenshot or debug overlay to confirm positions.

**Step 2: Run at 800x600**

Set the app/window/render config to 800x600 using the existing project config mechanism. Run the app and confirm:

- Start: `(635,242,162,37)`
- Choose Map: `(635,286,162,37)`
- Preview: `(644,37,144,112)`
- Back: `(644,535,156,42)`

**Step 3: Run at 640x480 if the app supports it**

Confirm formula rects:

- Start: `(475,242,162,37)`
- Choose Map: `(475,286,162,37)`
- Preview: `(484,37,144,112)`
- Back: `(484,409,156,42)`

This closes the open formula-only verification gap from the follow-up report.

**Step 4: Verify input alignment**

Click inside each key rect. Expected:

- Start enters Loading.
- Choose Map cycles or opens the current simple map selection behavior.
- Back exits.
- Color combo rect cycles the visible color state once color rendering is connected.

**Step 5: Verify no sim impact**

Run the normal focused tests for launch and shell:

```powershell
cargo test skirmish_shell -- --nocapture
cargo check
```

If unrelated dirty files break these commands, report the unrelated file/errors and stop for user direction.

## Sources & References

- **Design doc:** `docs/plans/2026-05-16-skirmish-shell-pixel-parity-design.md`
- **Ghidra reports:**
  - `docs/research/SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`
  - `docs/research/SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`
  - `docs/research/SPECIAL_FLAGS_SYSTEM.md`
- **Live Ghidra verification:**
  - `FUN_006AE2C0`: active offline Skirmish loop; caller `Main_Game`; exits on `0x617`/`0x5C0`.
  - `FUN_00622650`: `CreateDialogIndirectParamA(DAT_00B732F0, template, g_hWnd, proc, ...)`.
  - `FUN_006AE3F0`: Skirmish dialog proc; handles paint, command, and init delegation.
  - `FUN_0060C4A0`: `MoveWindow(hwnd,0,0,g_ScreenWidth,g_ScreenHeight,0)`.
  - `FUN_0060B1D0`: right-panel anchor helper.
  - `FUN_0060B350`: Back-button anchor helper.
  - `FUN_00775690`: HWND rect to main shell client/backbuffer rect conversion.
- **INI keys:**
  - `ini/rulesmd.ini [Countries] 0=Americans ... 9=YuriCountry`
  - `ini/rulesmd.ini [Sides] GDI=..., Nod=..., ThirdSide=YuriCountry`
  - `ini/rulesmd.ini [MultiplayerDialogSettings] Money=10000, UnitCount=10, GameSpeed=1, ShortGame=yes, MCVRedeploys=yes, Crates=yes, AllyChangeAllowed=yes`
  - `ini/rulesmd.ini [Colors] Gold, DarkRed, Orange, Magenta, Purple, DarkBlue, DarkSky, DarkGreen`
  - Original binary video option strings: `[Video] ScreenWidth`, `[Video] ScreenHeight`, `AllowHiResModes`
- **Related code:**
  - `src/ui/main_menu.rs`: current egui Skirmish settings/types/loading screen.
  - `src/app.rs`: screen state, event routing, initialization, render frame.
  - `src/app_init.rs`: map loading consumes `SkirmishSettings` and creates map-load `AssetManager`.
  - `src/sidebar/mod.rs`, `src/sidebar/sidebar_view.rs`: render-agnostic UI layout and hit testing.
  - `src/render/sidebar_chrome.rs`: SHP/palette atlas loader pattern.
  - `src/app_sidebar_build.rs`: app-layer sprite instance builder pattern.
  - `src/render/batch.rs`: `SpriteInstance`, camera update, batch drawing.
  - `src/assets/asset_manager.rs`: retail archive lookup and nested MIX extraction.

## Post-Plan Self-Review

- Spec coverage: every design-doc section maps to tasks 1 through 12.
- Vague-step scan: no unresolved task uses open-ended "add appropriate" language.
- Architecture check: UI state/layout is separate from render atlas and app glue.
- Interface ordering: layout/state interfaces land before rendering and app routing.
- Risk coverage: startup asset loading, upscale coordinates, egui replacement, and visual verification are explicit tasks.
- Self-containment: each task lists files, snippets, expected behavior, and verification commands.
- Sim compliance: no `sim/` task exists.
- Grounding coverage: plan cites reports, live Ghidra functions, repo patterns, and INI sections.
- Confidence tagging: key decisions are tagged; lower-confidence owner-draw composition scope is flagged for review.
- Deferred questions: 640x480 live capture, exact text parity, dropdown/listbox behavior, and full background composition are listed openly.
- Parity-critical items: populated with rects, anchors, preview, input alignment, and visual checks.
