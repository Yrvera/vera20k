# Skirmish Owner-Draw Controls Visual States Trace

Scenario: standard offline Yuri's Revenge Skirmish setup at 800x600. Scoped controls are the main owner-draw buttons, five option checkboxes, and three trackbars across idle/pressed/checked/value states.

Status: COMPLETE. No Ghidra mutation was performed. No Rust, INI, or in-repo docs were modified.

## Verdict Tally

PASS: 14 | FAIL: 4 | UNCHECKED: 3 | NOT-IMPLEMENTED: 1

PASS means this trace computed literal Rust-side numbers from source and matched them to active standard-YR gamemd evidence. Anything without both sides computed is UNCHECKED.

## Pipeline

Input/event -> `SkirmishShellState` option/button state -> `compute_layout(800,600)` rectangles -> `build_skirmish_shell_instances` PCX/SHP/primitive instances -> `build_shell_text_draws` bitfont labels -> GPU batch draw.

gamemd path: standard offline dialog `0x102` -> common shell owner-draw hook `FUN_0060F9A0` -> `OwnerDraw_Button_00612B70`, `OwnerDraw_Checkbox_006163A0`, `OwnerDraw_Trackbar_0061D950`.

## Stage Results

| Stage | Our output | gamemd output | Verdict |
|---|---:|---:|---|
| Start button control rect | `(644,241,156,42)` from `owner_draw_button_snap` | `(635,242,162,37)` | FAIL |
| Choose Map button control rect | `(644,283,156,42)` from `owner_draw_button_snap` | `(635,286,162,37)` | FAIL |
| Back button control rect | `(644,535,156,42)` | `(644,535,156,42)` | PASS |
| Button released/pressed PCX names | `bue_li30/mi30/ri30`, `bde_li30/mi30/ri30` | same 30-family names | PASS |
| Button horizontal middle behavior | repeated middle segments plus clipped final UV strip | middle PCX tiled, not scaled | PASS |
| Button vertical art size | every segment emitted at `rect.h`, so 42 px on all three current buttons | 30-family PCX pieces are native-height blits inside 37/42 px controls | FAIL |
| Button released/pressed text inset formula | released `(x,y+1,w-2,h)`, pressed `(x+2,y+5,w-2,h)` | same caller contract | PASS |
| Start/Choose final text screen rects | Start released `(644,242,154,42)`, pressed `(646,247,154,42)`; Choose released `(644,284,154,42)` | Start should derive from `(635,242,162,37)`, Choose from `(635,286,162,37)` | FAIL |
| Button hover/focus visual | no separate hover/focus PCX state | no hover/focus PCX in default path | PASS |
| Disabled button visual | renderer passes `disabled=false` for Start/Back/Choose | `WS_DISABLED` forces released art then alpha `0x80` overlay | NOT-IMPLEMENTED |
| Checkbox final rects | `(71,286)`, `(71,314)`, `(71,341)`, `(71,371)`, `(302,369)` with verified fixups | same final fixups for standard 0x102 | PASS |
| Checkbox icon/text geometry | icon `18x18`, label x offset `+26` | icon `18x18`, label left `+0x1A` | PASS |
| Checkbox checked/unchecked assets | `cue_i.pcx` unchecked, `cce_i.pcx` checked | same default path; no standard variant messages | PASS |
| Checkbox input state | icon-only toggle, label click does not toggle | icon-only `18x18` gate | PASS |
| Checkbox disabled overlay/color | no disabled-checkbox render branch verified in Rust for a standard disabled state | callback supports disabled icon alpha/text color, but standard init normally enabled | UNCHECKED |
| Trackbar final rects | game speed `(404,286,128,21)`, credits `(404,314,128,21)`, unit count `(404,340,128,21)` | same, including unit-count `y-1` fixup | PASS |
| Trackbar plaque/value rect | plaque `(482,*,50,21)`, value text `(483,*,49,21)` | right-side 50 px plaque, text `[right-0x31, top, right, bottom]` | PASS |
| Trackbar thumb endpoints | `(405,*,12,21)` at offset 0, `(470,*,12,21)` at offset 65 | `x = left + 1 + pixel_offset`, 12 px thumb gate, active width 65 | PASS |
| Trackbar assets | `trakgrip.pcx`, `trofl/trofm/trofr.pcx`; no `BTN-MINS/PLUS.SHP` | same standard 0x102 PCX path; `BTN-*` not used | PASS |
| Trackbar value mapping | y gate excludes top 4 px; x maps through 65 px active width; credits snap 100 | same formulas from owner-draw trackbar callback | PASS |
| Trackbar numeric display | game speed displays visual position, credits/unit count direct value | `0x529` initialized as `6 - stored`; text formats current trackbar value | PASS |
| Trackbar rail final RGB/bevel pixels | synthetic primitive bevel entry using two fixed RGB constants | gamemd uses `FUN_006208F0` and display-format converted shell globals | UNCHECKED |
| Disabled trackbar visual | no standard disabled render state computed | callback supports disabled thumb overlay/rail color | UNCHECKED |

## Player-Visible Findings

1. FAIL - Button placement/size: Start is drawn at `(644,241,156,42)` instead of retail `(635,242,162,37)`, and Choose Map at `(644,283,156,42)` instead of `(635,286,162,37)`. The right-side buttons look snapped into the wrong 156 px tile instead of the 162x37 dialog controls. Rust: `src/ui/skirmish_shell/layout.rs:319`, `src/ui/skirmish_shell/layout.rs:463`. gamemd evidence: `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md` final rect table, active `FUN_0060B1D0`.

2. FAIL - Button art height: Rust stretches all three button PCX pieces to the full control height via `[segment.width, rect.h]`. Retail chooses the 30-family art and direct-blits native-height pieces; the chrome/background remains visible outside the art. This makes buttons look vertically stretched, especially the 42 px Back control. Rust: `src/app_skirmish_shell_render.rs:370`. gamemd evidence: `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`, `OwnerDraw_Button_00612B70`, `FUN_006ba3e0`.

3. FAIL - Start/Choose label screen positions: the text inset formula is correct, but it is applied to the wrong Start/Choose rectangles. Start released becomes `(644,242,154,42)` instead of a rect derived from `(635,242,162,37)`; Choose released becomes `(644,284,154,42)` instead of one derived from `(635,286,162,37)`. Rust: `src/app_skirmish_shell_render.rs:1323`, `src/app_skirmish_shell_render.rs:1335`. gamemd evidence: `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md` text rectangle table plus final rect evidence.

4. NOT-IMPLEMENTED - Disabled owner-draw button visual: the render path always passes `disabled=false` for Start, Back, and Choose Map, so the retail disabled visual state cannot appear. Retail forces released art and applies an alpha `0x80` overlay for `WS_DISABLED`. Rust: `src/app_skirmish_shell_render.rs:1171`, `src/app_skirmish_shell_render.rs:1179`, `src/app_skirmish_shell_render.rs:1187`. gamemd evidence: `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`, `0x00613254..0x00613262`, `0x006135F3..0x0061361B`.

5. UNCHECKED - Trackbar rail pixel parity: Rust has a plausible primitive bevel atlas entry, but this trace did not compute gamemd's final display-format pixels from the DirectDraw conversion globals. The rail may look close while still being off by color or corner pixels. Rust: `src/render/skirmish_shell_chrome.rs:19`, `src/render/skirmish_shell_chrome.rs:214`, `src/render/skirmish_shell_chrome.rs:488`. gamemd evidence: `SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`, `FUN_006208F0`.

## Adjacent Findings

- The standard checkbox/trackbar docs `SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md` are stale in their "Current Rust Implementation Status" table. Current Rust now has checkbox rects/state/rendering and trackbar state/rendering.
- The trackbar ranges are hardcoded in `state.rs` and render code for the standard YR defaults. That is acceptable for this visual trace, but long-term parity should bind them to `[MultiplayerDialogSettings]`/Rules where available.
- The renderer still lacks a screenshot/pixel comparison harness for the shell. That keeps rail bevel color, display-format text color, and alpha blending in UNCHECKED territory even when geometry is correct.

## Sources

- Rust read-only scan: `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`, `src/skirmish_launch.rs`, `src/sim/game_options.rs`.
- Verified research docs: `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_OWNERDRAW_VARIANT_WRITERS_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md`, `SKIRMISH_BTN_MINS_PLUS_USE_SITE_GHIDRA_REPORT.md`, `SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_640X480_FINAL_VISIBLE_LAYOUT_TRACE.md`.
