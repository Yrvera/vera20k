# Skirmish Shell Pixel-Parity Design

## Goal

Replace the current visible egui Skirmish setup screen with a dedicated pixel-parity Skirmish shell layer that reproduces the researched gamemd.exe dialog 0x102 viewport, right-panel layout, asset usage, and control coordinate behavior.

## Architecture Context

The current Skirmish setup screen lives in `src/ui/main_menu.rs` and is drawn through egui from the `GameScreen::MainMenu` branch in `src/app.rs`. That path stores menu state in `SkirmishSettings`, returns `MenuAction`, and transitions to `GameScreen::Loading` when `StartSelected` is returned. It is functional, but it intentionally uses a modern client-theme/card UI rather than RA2/YR shell pixels.

The engine already has a separate native sprite-rendering path for in-game UI chrome. `src/sidebar/` owns render-agnostic sidebar layout and hit-testing, `src/render/sidebar_chrome.rs` loads SHP assets into GPU atlases, and `src/app_sidebar_build.rs` converts layout/view data into `SpriteInstance` quads. That pattern is a good fit for shell parity: a `ui/` module should own shell state, layout, and hit testing; a `render/` module should own shell assets and atlas entries; app-level glue should draw and route inputs.

`src/assets/asset_manager.rs` already loads and indexes top-level and nested MIX archives, including shell-relevant nested archives such as `neutral.mix`, `ntrlmd.mix`, `local.mix`, and `localmd.mix`. Runtime code can use `get_ref`, `get_with_source_ref`, `archive`, or explicit `load_nested` in the same style as sidebar cameo/chrome loading. The retail shell assets needed by the Skirmish setup research are therefore accessible through existing asset plumbing.

The `sim/` layer must not participate in this feature. Skirmish shell layout, input, animation, preview drawing, and menu state are presentation concerns. The only handoff to gameplay remains the existing `SkirmishSettings`-style launch data consumed by map loading and skirmish setup.

## Impact Analysis

Touched areas:

- `src/ui/main_menu.rs`: preserve or retire only the visible Skirmish setup path. Keep shared launch data types if useful, but stop using egui for the Skirmish setup screen.
- New `src/ui/skirmish_shell/` module: render-agnostic state, layout constants, derived rects, hit testing, selection/update actions, and conversion to launch settings.
- New `src/render/skirmish_shell_chrome.rs` or equivalent: load shell SHP assets, palette data, and atlas entries.
- `src/app.rs`: route `GameScreen::MainMenu` rendering/input to the Skirmish shell instead of `draw_main_menu_with_maps`; keep transition semantics to `GameScreen::Loading`.
- `src/app_init.rs` / app state construction: initialize shell chrome atlas and any shell font/palette resources once assets are available.
- Input handling in the app layer: route mouse position/clicks to Skirmish shell hit testing while on `GameScreen::MainMenu`.
- Rendering glue: emit shell background/right-panel/button/combo/preview/flag instances in screen-pixel coordinates.

Dependencies and risks:

- The shell must use render dimensions consistently. If the game is using an upscale pass, menu rendering must decide whether to target swapchain pixels or the render-source viewport. The gamemd.exe shell behavior is defined in shell client pixels; the implementation must avoid accidental `ui_scale` scaling.
- Current egui menu controls include only a small subset of original Skirmish options. Pixel parity requires visible shell coordinates and controls from the researched dialog, but some original control behaviors may still need follow-up investigation before every combo/list option is interactive.
- Asset lookup must respect RA2/YR precedence. YR `*md` assets and archives take priority when present; base RA2 assets are fallback.
- The existing working tree may have unrelated compile failures. This design avoids those files except where app-level integration is necessary.
- There are no `sim/` changes, so deterministic tick ordering and state hashing are unaffected.

## Chosen Approach

Build a dedicated Skirmish Shell Layer:

- `ui/skirmish_shell` owns the model: visible controls, dialog 0x102 layout, resolution-derived rects, hit-test results, and user-editable Skirmish settings.
- `render/skirmish_shell_chrome` owns visual assets: shell background pieces, right-panel pieces, buttons, map preview frame/content surfaces, flag art, and any reusable atlas entries.
- App-level code bridges the two: render shell instances on `GameScreen::MainMenu`, route input to the shell model, and translate `SkirmishShellAction::StartGame` into the existing loading transition.

This replaces the current visible egui Skirmish setup screen instead of hiding the parity shell behind a toggle. The current egui path can remain temporarily as dead-simple fallback during implementation only if asset loading fails catastrophically, but it should not be the normal visible path after the feature lands.

This approach follows the existing sidebar split: render-agnostic UI layout in `ui`/`sidebar`, asset loading in `render`, and app orchestration in `app`. It avoids a premature generic RA2 dialog framework while preserving enough structure to generalize later.

## Tiny-Detail Ledger

- Dialog 0x102 is selected by the live offline Skirmish setup path: `Main_Game -> FUN_006AE2C0`, with dialog proc `0x006AE3F0`, dialog resource id `0x102`, and creation through `FUN_00622650`. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, dialog creation call path.
- The dialog is created with `CreateDialogIndirectParamA(..., g_hWnd, proc, ...)` and resource style `0x40000040` includes `WS_CHILD`; it is modeless/child-hosted under the main shell window, not a standalone modal screen. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, dialog creation findings.
- On init, dialog 0x102 is moved to `(0,0,g_ScreenWidth,g_ScreenHeight)` by `FUN_0060C4A0`; shell origin is the main shell client origin. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, init/fullscreen host findings.
- Resource DLU baseline is `DIALOGEX 0,0,533,369`, MS Sans Serif 8, `baseX=6`, `baseY=13`; conversion is `x=MulDiv(dlu_x,6,4)`, `y=MulDiv(dlu_y,13,8)`. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, DLU baseline.
- There is no uniform scale transform for dialog children. Only selected controls are post-created/moved. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, selective transforms.
- Right-anchor controls selected by `FUN_00608CD0` for dialog 0x102 include `0x468`, `0x5AA`, `0x5A8`, `0x617`, `0x6EC`, and `0x694`; these use `FUN_0060B1D0`. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, selective transforms.
- Back button control `0x5C0` is selected by `FUN_00609730` and moved by `FUN_0060B350`. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, selective transforms.
- Right-anchor formula is `offset_x=max(0,(parent_w-800)/2)`, `offset_y=max(0,(parent_h-600)/2)`, `x=parent_w-offset_x-child_w-inset`, `y=original_y+offset_y`. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, right-anchor formula.
- For researched right-anchor children `0x617`, `0x5AA`, and `0x468`, child metadata `+0xE0==0`, so the inset is the default `(168-child_w)/2`. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`, child `+0xE0` finding.
- Color combos `0x6A2` and `0x522..0x528`, plus flag statics `0x6DA..0x6E1`, are not moved/scaled by the post-creation transform path and keep resource pixel coordinates. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, selective transform exclusions.
- One-pixel fixes for dialog 0x102: `0x50C` y-1; `0x54E`, `0x693`, `0x696`, `0x69A` x-1; `0x6A0` x+1 and w+1. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, one-pixel adjustment findings.
- Right-panel asset dimensions are: `SDBTNANM.SHP` 156x42, `SDBTNBKGD.SHP` 168x42, `SDTP.SHP` 168x199, `SDBTM.SHP` 168x65, `SDMPBTN.SHP` 156x84, `SDWRNTMP.SHP` 168x177. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`, retail asset dimensions.
- Back button formula is `x=parent_width-offset_x-156`, `w=156`, `h=42`, `y=((fc28.y-fc24.y)/fc24.h - 1)*fc24.h + fc24.y`. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`, Back y formula.
- Right-panel layout globals: 800x600 uses `fc20=(632,0,168,199)`, `fc24=(632,199,168,42)`, tile count 9, `fc28=(632,577,168,23)`; 1024x768 uses `fc20=(744,84,168,199)`, `fc24=(744,283,168,42)`, tile count 9, `fc28=(744,661,168,23)`; 640x480 uses `fc20=(472,0,168,199)`, `fc24=(472,199,168,42)`, tile count 6, `fc28=(472,451,168,29)`. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`, right-panel globals.
- Final key rects at 800x600: Start `0x617=(635,242,162,37)`, Choose Map `0x5AA=(635,286,162,37)`, preview `0x468=(644,37,144,112)`, Back `0x5C0=(644,535,156,42)`. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`, final rects.
- Final key rects at 1024x768: Start `0x617=(747,326,162,37)`, Choose Map `0x5AA=(747,370,162,37)`, preview `0x468=(756,121,144,112)`, Back `0x5C0=(756,619,156,42)`. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`, final rects.
- Formula-derived key rects at 640x480: Start `0x617=(475,242,162,37)`, Choose Map `0x5AA=(475,286,162,37)`, preview `0x468=(484,37,144,112)`, Back `0x5C0=(484,409,156,42)`; live screenshot verification remains open. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`, 640 formula note.
- Owner-draw child callbacks paint in child-client coordinates, while cached parent backgrounds are aligned through HWND-to-shell conversion. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, background/surface origin findings.
- `FUN_00775690(hwnd, rect)` converts an HWND window rect to main shell client/backbuffer coordinates by subtracting `g_hWnd` client screen origin; `DAT_00887310` is the main display/backbuffer. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, background/surface origin findings.
- `DrawStartPositions` finds preview HWND `0x468`, calls `FUN_00775690`, then draws map preview/start markers to `DAT_00887310`; preview overlays must be placed in final shell/backbuffer coordinates, not local combo/list coordinates. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, map preview drawing.
- Higher-resolution right-panel behavior centers an 800x600 shell content region for `screen_w > 1023` and `screen_h > 767` in the researched right-panel layout code; no Skirmish-specific INI scale/origin option was found. Source: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, resolution behavior.

## Design

### Components

#### `ui/skirmish_shell`

Create a render-agnostic Skirmish shell module with these responsibilities:

- Store the editable Skirmish setup state: selected map, player side/country/color, opponent rows, credits, short-game-style options, and start position data needed by current launch code.
- Compute shell layout from screen width/height using the verified dialog 0x102 formulas.
- Provide named rects for controls by original control id where known.
- Provide hit testing from shell pixel coordinates to semantic actions.
- Update selection state in response to actions.
- Export launch settings compatible with the current `SkirmishSettings`/loading path.

Suggested internal types:

```rust
pub struct SkirmishShellState { ... }
pub struct SkirmishShellLayout { ... }
pub struct SkirmishShellControlRects { ... }
pub enum SkirmishShellAction { None, StartGame, BackOrExit, ChooseMap, SelectMap(usize), SelectColor(...), ... }
pub enum ShellControlId { Id0x617, Id0x5AA, Id0x5C0, Id0x468, ... }
```

Use integer pixel rects for shell layout. Convert to `f32` only at the rendering boundary when building `SpriteInstance`s. This avoids subpixel drift in the researched formulas.

#### `render/skirmish_shell_chrome`

Create a shell chrome asset loader and atlas in the render layer. It should mirror the sidebar chrome structure without sharing sidebar-specific names:

- Load SHP files used by the Skirmish shell/right panel.
- Preserve pixel dimensions from decoded SHP frames.
- Store named atlas entries for background panels, right-panel pieces, normal/pressed/disabled button frames, map preview chrome, flags, and combo surfaces.
- Track source archive/name in logs for asset precedence debugging.

Suggested types:

```rust
pub struct SkirmishShellChromeSet { ... }
pub struct SkirmishShellChromeAtlas { ... }
pub struct SkirmishShellChromeEntry { uv_origin: [f32; 2], uv_size: [f32; 2], pixel_size: [f32; 2] }
```

The module should prefer existing SHP decode, palette, and atlas-builder utilities. If missing assets are non-critical, render a clearly logged fallback rectangle for development, but do not silently substitute approximate dimensions.

#### App Integration

`AppState` should own:

- `skirmish_shell_state`
- `skirmish_shell_chrome`
- any shell text/font resources needed outside egui

The `GameScreen::MainMenu` branch should:

1. Clear or draw the shell background.
2. Build `SkirmishShellLayout` from the active shell render size.
3. Build shell sprite instances from the layout and shell state.
4. Render those instances through the existing batch renderer.
5. Route mouse input to `ui/skirmish_shell::hit_test`.
6. Convert `StartGame` into the existing `GameScreen::Loading { map_name }` transition.

Do not route shell rendering through egui. egui can be removed from this screen's normal path once shell text/input are available.

### Interfaces / Contracts

`ui/skirmish_shell` should expose a small API:

```rust
pub fn compute_layout(screen_w: u32, screen_h: u32) -> SkirmishShellLayout;
pub fn hit_test(layout: &SkirmishShellLayout, x: i32, y: i32) -> SkirmishShellAction;
pub fn apply_action(state: &mut SkirmishShellState, action: SkirmishShellAction, maps: &[MapMenuEntry]);
pub fn launch_settings(state: &SkirmishShellState) -> SkirmishSettings;
```

Rendering should consume layout/state but not mutate game setup except for animation timers:

```rust
pub fn build_skirmish_shell_instances(
    chrome: &SkirmishShellChromeAtlas,
    layout: &SkirmishShellLayout,
    state: &SkirmishShellState,
) -> Vec<SpriteInstance>;
```

If animated button frames need time state, keep that in a shell presentation state owned by app/UI, not in `render/`.

### Data Flow

Startup:

1. `AssetManager` loads retail archives as it does today.
2. App init builds `SkirmishShellChromeSet` from shell assets.
3. `AppState` initializes `SkirmishShellState` from available maps and current default settings.

Frame render on main menu:

1. App computes shell target size.
2. `ui/skirmish_shell::compute_layout` returns dialog/fullscreen rects, right-panel rects, and control rects.
3. Render glue builds shell background/right-panel/control/map-preview/flag instances.
4. Batch renderer draws shell instances in shell pixel coordinates with camera offset zero.

Input:

1. Window mouse coordinates are converted into the same shell pixel coordinate space used for rendering.
2. `hit_test` maps point to original control-id-backed shell actions.
3. `apply_action` updates shell state or returns `StartGame`.
4. `StartGame` reuses current map selection/loading path.

Preview:

The map preview static `0x468` must be treated as a child HWND whose final shell/backbuffer rect is known from layout. Any preview image and start-position markers are clipped/positioned inside that final rect. The draw code should not assume local dialog-relative preview origin once the final shell rect has been computed.

### Error Handling

- Missing mandatory shell assets should be logged with the requested filename and source archive search context.
- The app may show a minimal non-egui fallback only when shell assets fail to load, but that fallback should be treated as an error/development path, not the intended UI.
- If a researched control has no asset yet, keep its layout rect in the model and render a debug placeholder behind a feature flag or development log. Do not drop the control from layout.
- Invalid selected map indices should clamp to zero, matching the current menu's defensive behavior.

### Testing Strategy

Unit tests:

- `compute_layout(800,600)` matches the researched key rects for `0x617`, `0x5AA`, `0x468`, and `0x5C0`.
- `compute_layout(1024,768)` matches the researched key rects for the same controls.
- `compute_layout(640,480)` matches the formula-derived key rects and keeps the 640 branch explicit.
- Color combo and flag static rects remain resource-derived and are not transformed by the right-anchor path.
- Right-anchor formula handles larger widths/heights without applying uniform scale.
- Hit testing maps exact rect interiors and boundaries to the expected semantic actions.

Asset tests/tooling:

- Verify required shell SHPs load from the expected archives and decoded dimensions match the research ledger.
- Log source archive precedence for YR/base fallback.

Visual verification:

- Capture 800x600 and 1024x768 screenshots and compare key control positions against the researched rects.
- Capture 640x480 after implementation if the app supports that window/render size; this closes the current formula-only verification gap.
- Verify the right panel is centered/anchored at higher resolutions, not scaled.
- Verify map preview start markers land inside final `0x468` shell/backbuffer rect.

Regression:

- Starting a selected map still reaches `GameScreen::Loading` and then the existing Skirmish setup path.
- No `sim/` tests should change, because this is UI/render-only.

## Architectural Decisions

- Follow the sidebar split between render-agnostic layout/hit testing and render-layer asset atlases. This keeps UI behavior testable without GPU setup.
- Use integer shell pixel coordinates as the primary layout representation. gamemd.exe's behavior is pixel/DLU-derived, and introducing scaled floats earlier would invite off-by-one drift.
- Replace the visible egui Skirmish setup path. Keeping both as normal paths would create two competing sources of UI truth and make parity testing ambiguous.
- Do not build a generic RA2 dialog host yet. The research target is dialog 0x102, and a generic host would need broader resource parsing, control class behavior, and owner-draw abstraction before it can be trusted.
- Keep map launch settings compatible with existing `SkirmishSettings` until the parity shell requires a richer launch model. This limits the blast radius to menu/presentation code.

Tech debt accepted:

- The first dedicated shell layer will still encode dialog 0x102 knowledge directly rather than interpreting arbitrary Win32 dialog resources. This is acceptable because the current parity target is one visible Skirmish setup screen. The control-id-backed model should make later generalization straightforward if more shell dialogs are researched.

## Alternatives Considered

### Generic RA2 Dialog Host Layer

This would parse dialog resources, model Win32 child controls, and host owner-draw behavior generically. It is attractive long term, but it is too broad for the current implementation target. It risks spending most effort on framework behavior before the player-visible Skirmish screen matches gamemd.exe.

### App-Local Pixel Shell Implementation

This would place hardcoded rects and draw calls directly in app/render glue. It would be fast, but it would blur state, hit testing, asset lookup, and rendering responsibilities. It also makes future parity fixes harder because the binary-derived layout rules would be scattered through app code.

### Keep egui And Skin It

This would not meet the parity goal. The researched behavior depends on exact child HWND-derived rectangles, right-panel anchoring, parent-background capture offsets, and SHP dimensions. egui layout cannot reproduce those details reliably without fighting the toolkit.
