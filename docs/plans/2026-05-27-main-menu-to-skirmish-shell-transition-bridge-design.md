# Main Menu To Skirmish Shell Transition Bridge Design

## Goal

Remove the visible hard snap when the current Rust main menu shortcut enters the native Skirmish shell, using a low-risk offscreen shell transition bridge that is explicitly marked as parity drift until the missing intermediate Single Player shell exists.

## Architecture Context

Current Rust routes the main-menu Single Player button directly into the native Skirmish shell:

- `src/ui/main_menu_shell/state.rs` maps `SinglePlayer0x683` to `MainMenuShellAction::SinglePlayer` and preserves native return code `1`.
- `src/app.rs::handle_main_menu_shell_action` immediately sets `main_menu_show_native_skirmish_shell = true`, enters shell window mode, and ensures Skirmish shell chrome.
- `src/app.rs` render routing chooses either `app_main_menu_shell_render::render_main_menu_shell` or `app_skirmish_shell_render::render_skirmish_shell`.

The two steady renderers are final-screen renderers. They draw directly into the supplied `wgpu::TextureView`, clear their target, and use the app depth view. They are not currently structured to return reusable sprite/text draw lists. That makes a native per-piece transition refactor larger than needed for this bridge.

Research says the exact native path is not direct `0xE2 -> 0x102`. The verified route is main menu button `0x683 -> return code 1 -> Main_Game -> FUN_0060D380(1)`, then a later return code `0x0B -> g_GameMode = 5 -> FUN_006AE2C0` opens offline Skirmish `0x102`. The current direct jump is already a Rust shortcut. This design therefore treats the transition as an app/UI bridge, not as exact `gamemd.exe` parity.

Relevant existing patterns:

- `render/upscale_pass.rs` already owns offscreen render-target textures plus a post-process draw into the swapchain.
- `render/radar_anim.rs` is a small render-side frame/timer state machine using fixed millisecond cadence.
- Main menu and Skirmish shell rendering both live above `sim/`; this bridge must stay in app/render and never affect gameplay state.

## Impact Analysis

Touched modules:

- `src/app.rs`
  - Add `AppState` field for active bridge transition.
  - Change `MainMenuShellAction::SinglePlayer` to start the transition instead of immediate shell flip.
  - Add render branch for active transition before `native_skirmish_shell_active`.
  - Short-circuit shell mouse/key/wheel input while the transition is active.
  - Recreate transition render targets on resize.

- New `src/app_shell_transition.rs`
  - App-level transition state, timing, completion, and orchestration.
  - Calls existing steady renderers into offscreen targets.
  - Performs final transition completion flip into normal Skirmish shell.

- New or extended `src/render/shell_transition_pass.rs`
  - Owns offscreen color/depth textures for source and destination shell screens.
  - Owns a small compositing pipeline that samples the two color textures and writes the final swapchain image.

- `src/app_main_menu_shell_render.rs` and `src/app_skirmish_shell_render.rs`
  - Add a small render-target/depth parameter abstraction so the same steady renderers can draw either to the swapchain or to transition offscreen targets.
  - Add an explicit Skirmish preview render mode for offscreen destination rendering so paint-time UI side effects are suppressed.

Risk areas:

- Offscreen renders must have their own depth textures. Reusing the swapchain depth view for offscreen targets would be wrong.
- Existing renderers clear their target. That is acceptable offscreen, but the compositor must be the only pass that writes the swapchain during active transition.
- Main menu movie stepping should continue while rendering the source frame, matching current menu behavior.
- Skirmish shell owner-draw paint sounds must not fire repeatedly while the destination shell is only being rendered as an offscreen preview.
- Input should be effectively blocked during the short bridge so the user cannot interact with a half-transition shell.
- Resize during transition must recreate offscreen resources or cancel/restart cleanly.

No gameplay/sim impact:

- No `sim/` changes.
- No session packing changes.
- No deterministic tick or lockstep state changes.

## Chosen Approach

Use an offscreen full-screen bridge.

On Single Player mouse-up, Rust starts `ShellBridgeTransition` instead of setting `main_menu_show_native_skirmish_shell = true`. During the bridge, the app renders:

1. Current main menu shell into source offscreen texture.
2. Target Skirmish shell into destination offscreen texture.
3. A compositor pass samples both textures and writes the actual swapchain frame with a horizontal slide plus slight fade.

After the final transition frame, the app sets `main_menu_show_native_skirmish_shell = true`, clears the bridge state, ensures Skirmish chrome, and steady rendering continues through `render_skirmish_shell`.

This is chosen because it fixes the player-visible snap without refactoring both shell renderers into draw-list builders. It is easier to remove or replace when the missing intermediate Single Player shell is implemented.

## Tiny-Detail Ledger

- Main menu button `0x683` writes return code `1`, not direct `0x102` entry. Source: `SKIRMISH_MAIN_MENU_TO_SHELL_TRANSITION_CALLER_FRAME_COMPOSITION_GHIDRA_REPORT.md` sections 3.1-3.3.
- Current Rust direct `SinglePlayer -> native Skirmish shell` is a bridge/DRIFT from verified native flow. Source: same report section 7.
- Native transition cadence anchor is `Sleep(0x1E)` = 30 ms per frame. Source: `SKIRMISH_FUN_006071E0_SHELL_TRANSITION_REDRAW_PATH_GHIDRA_REPORT.md` section 3.2.
- Native full helper loop computes `max(schedule) + 6` frames. This bridge may approximate the schedule, but must document the chosen count. Source: `FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md` section 4.
- `DL=1` completion sends `0x4EC`, which starts static reveal through `0x4EE`; `DL=0` sends `0x4ED` and does not. This bridge should not claim exact reveal parity unless a dedicated Skirmish reveal state is modeled. Source: transition caller report sections 4.1-4.2.
- Stock `ShellButtonSlideSound=` is empty in `rules.ini` and `rulesmd.ini`; the bridge must not add a new audible stock slide sound. Source: transition caller report section 6.
- Existing main-menu mouse-down sound must remain the only stock audible entry sound for this click path. Source: `src/app.rs::play_main_menu_button_sound`; sound research maps it to `GUIMainButtonSound`.
- During bridge, user input is blocked until completion, matching the modal/blocking nature of the native helper more closely than interactive mid-transition UI. Source: `FUN_00608260` disables parent and children around `FUN_006071E0`.
- Steady-state render output before and after the bridge must remain owned by the existing renderers; the bridge compositor cannot become a permanent alternate Skirmish renderer.

## Design

### Components

#### `ShellBridgeTransition`

Stored on `AppState`:

```rust
pub(crate) main_menu_to_skirmish_transition: Option<ShellBridgeTransition>
```

Fields:

- `started_at: Instant`
- `last_step_at: Instant`
- `frame_index: u32`
- `frame_count: u32`
- `frame_ms: u32`
- `target: ShellBridgeTarget`

Initial constants:

- `frame_ms = 30`
- `frame_count = 14`

The exact native schedule is child-count dependent. `14` is chosen as a bridge constant from the documented `N + 8` guidance for a six-button main menu, but it remains a bridge approximation, not verified parity for direct `0xE2 -> 0x102`.

#### `ShellTransitionTargets`

Render-side resource owner:

- source color texture/view/bind group
- destination color texture/view/bind group
- source depth texture/view
- destination depth texture/view
- width/height

Resources are recreated when `gpu.config.width` or `gpu.config.height` changes.

#### `ShellRenderTarget`

Small borrowed target passed to shell renderers:

```rust
pub(crate) struct ShellRenderTarget<'a> {
    pub(crate) color: &'a wgpu::TextureView,
    pub(crate) depth: &'a wgpu::TextureView,
}
```

The existing steady renderers currently hardcode `state.depth_view`. The implementation must change them to render through this target object, with the normal swapchain path passing `{ color: &view, depth: &state.depth_view }` and the bridge passing `{ color: offscreen_color, depth: offscreen_depth }`.

#### `ShellRenderMode`

Skirmish shell rendering needs an explicit mode:

```rust
pub(crate) enum ShellRenderMode {
    Visible,
    TransitionPreview,
}
```

`Visible` preserves current behavior. `TransitionPreview` must suppress side effects that are only valid for actual visible paint, including `update_owner_draw_button_paint_sound` / `GenericClick` paint-transition playback and mutation of `skirmish_shell_last_painted_pressed_button`.

#### `ShellTransitionPass`

Small compositor:

- WGSL full-screen triangle/quad shader.
- Samples source and destination offscreen textures.
- Uniform contains progress, slide offset, and fade amount.
- Writes to the swapchain target.

Suggested visual formula for first implementation:

- `progress = frame_index / frame_count`
- source screen x offset: `-round(progress * 32)` pixels
- destination screen x offset: `round((1.0 - progress) * 64)` pixels
- source alpha: `1.0 - 0.30 * progress`
- destination alpha: `progress`

This gives a side-shell feel without requiring per-button motion. If it feels too subtle or too faded in visual testing, tune constants before deeper refactors.

### Interfaces / Contracts

`app_shell_transition.rs` should expose:

- `start_main_menu_to_skirmish(state: &mut AppState)`
- `render_main_menu_to_skirmish_transition(state, encoder, target) -> Result<bool>`
  - returns `true` when it consumed/rendered the frame
  - completes and clears transition when final frame is reached

The render function:

1. Ensures Skirmish chrome exists.
2. Ensures offscreen resources match current surface size.
3. Renders main menu shell into source texture.
4. Temporarily renders Skirmish shell into destination texture with `ShellRenderMode::TransitionPreview`.
5. Composites source/destination to swapchain.
6. Advances the fixed 30 ms frame accumulator.
7. On completion, flips `main_menu_show_native_skirmish_shell = true` and clears transition.

Important guard: offscreen destination rendering must suppress side effects that are only valid for actual visible Skirmish paint, especially repeated GenericClick paint-transition sounds. This is not optional; the preview mode is part of the contract.

### Data Flow

Click path:

```text
mouse_down
  -> existing GUIMainButtonSound

mouse_up SinglePlayer
  -> start ShellBridgeTransition
  -> main_menu_show_native_skirmish_shell remains false

shell input while transition active
  -> ignored until bridge completes

render frame while transition active
  -> source offscreen: render_main_menu_shell
  -> destination offscreen: render_skirmish_shell TransitionPreview
  -> compositor: source/destination slide+fade to swapchain

final transition frame
  -> main_menu_show_native_skirmish_shell = true
  -> transition = None

next render frame
  -> normal render_skirmish_shell
```

### Error Handling

- If main menu shell render returns fallback during transition, cancel the bridge and use existing fallback path.
- If Skirmish chrome cannot be created, cancel the bridge and leave the user on main menu or use existing fallback behavior.
- If offscreen resource creation fails, log a warning and fall back to immediate flip. Do not crash the app for a bridge-only visual effect.
- On resize, cancel transition to the nearest stable state:
  - if progress < 0.5, return to main menu
  - if progress >= 0.5, complete to Skirmish

This avoids carrying partially valid offscreen textures across a surface resize. A later implementation can preserve progress through resize if that proves useful.

### Testing Strategy

Unit tests:

- `ShellBridgeTransition` advances one frame per 30 ms and clamps at completion.
- Completion sets target state exactly once.
- Progress calculation is monotonic and reaches 1.0 on final frame.
- Resize policy chooses main menu before halfway and Skirmish after halfway.

Focused integration tests where practical:

- Single Player action starts transition and does not immediately set `main_menu_show_native_skirmish_shell`.
- Completing transition sets `main_menu_show_native_skirmish_shell = true`.
- Escape/back behavior during transition is ignored; no half-active shell state remains.
- Mouse and wheel input during transition do not call main-menu or Skirmish shell handlers.
- Skirmish `TransitionPreview` render does not play `GenericClick` and does not mutate `skirmish_shell_last_painted_pressed_button`.

Manual visual verification:

- Start app, click Single Player, confirm no hard snap.
- Confirm transition lasts about `frame_count * 30 ms`.
- Confirm stock install does not play an extra slide sound.
- Confirm final Skirmish shell is interactive after transition.
- Resize window during transition and confirm no panic or corrupted target.

No cargo-wide behavior change is expected, but after implementation run:

- focused UI/state tests for main-menu/skirmish shell
- `cargo check -q`

## Architectural Decisions

- Keep this bridge above `sim/`. It is visual shell routing only.
- Reuse existing steady renderers rather than extracting draw-list builders now. This limits blast radius.
- Refactor steady shell renderers only enough to accept a borrowed color/depth render target and render mode. Do not extract their draw-list internals in this bridge.
- Use offscreen color/depth render targets following the existing post-process shape in `render/upscale_pass.rs`.
- Mark drift in code comments near transition start and design references. This avoids future confusion where the bridge is treated as verified native `gamemd.exe` behavior.
- Do not add audible slide sound for stock rules.
- Do not attempt exact `FUN_006071E0` per-child schedule yet. That requires the intermediate Single Player shell and a better recovery of `0x00612690`.

Tech debt introduced:

- A whole-screen/shell-screen bridge exists where native likely animates shell controls from an intermediate dialog path.
- The compositor is a temporary bridge that may be deleted or narrowed after exact native shell flow is implemented.

Debt containment:

- Keep module and type names containing `Bridge`.
- Put the research-doc path in comments.
- Do not reuse the bridge as a generic "native transition" API without new verification.

## Alternatives Considered

### B. Simple Timer Overlay

Keep rendering main menu, run a timer, optionally darken/slide a mask, then switch to Skirmish. This is lowest effort but still visually feels like hiding a snap rather than transitioning between shells. Rejected because the user asked for a smooth side change and this would likely feel too thin.

### C. Extract Renderer Draw Lists

Refactor main-menu and Skirmish renderers to emit reusable draw lists, then animate individual shell pieces. This is the better long-term path for native-looking per-piece movement, but it touches both steady renderers and increases regression risk. Rejected for the first pass because the user wants to see how a lower-risk bridge looks before paying the refactor cost.

### Strict Parity First

Implement the missing intermediate Single Player shell and verify whether its Skirmish selection path reaches `0x00612690 -> FUN_00608260`. This is the correct eventual parity path, but it does not solve the current visible snap quickly. Deferred with explicit drift label.
