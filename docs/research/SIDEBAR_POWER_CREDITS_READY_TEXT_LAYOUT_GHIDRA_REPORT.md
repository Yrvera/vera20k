# SIDEBAR_POWER_CREDITS_READY_TEXT_LAYOUT - Ghidra Report

Date: 2026-05-27

Target question: What exact layout anchors and composition rules does standard YR use for the sidebar power bar, credits text, Ready/status text, queue-count text, and their dark rectangles?

Non-goals:
- Do not re-prove the side text color table except where coordinates consume the packed color.
- Do not re-prove CAMEO.PAL vs SIDEBAR.PAL for base cameo and overlay art.
- Do not decode all credit tick economy logic or all power-segment target math.
- Do not edit Rust or older research docs.

Evidence needed to mark COMPLETE:
- Read-only Ghidra decompile for the live draw paths.
- Handoff-critical coordinate and anchoring claims backed by decompile plus executable disassembly-range availability.
- Active-in-YR call-path evidence from `MainGame_SidebarDraw` / `SidebarClass__Draw`.
- Concrete Rust-facing acceptance scenarios.

Stop conditions:
- Ghidra MCP unavailable.
- Any required claim cannot be narrowed to this slot's layout scope.
- Any mutating Ghidra operation would be required.

## Verified Findings

### 1. Power bar origin and draw order

Active in YR: Yes. `SidebarClass__Draw @ 0x006A6C30` calls `StripClass__Draw`, then `PowerClass__Draw(this)`, then blits the sidebar to screen. This is the normal in-game sidebar draw path reached from `MainGame_SidebarDraw @ 0x006D0A30`.

`PowerClass__Draw @ 0x0063FB20` draws on `g_SidebarSurface`, not directly in screen coordinates. The power-bar x coordinate is `5` only when `ScenarioClass+0x34B8 == 0`; otherwise it is `0`, so Soviet side index `1` uses x `0`. The y coordinate starts at `g_SidebarWidth + 0x45`. `SidebarClass__InitSidebarRect @ 0x006A5130` sets `g_SidebarWidth = 0x9E`, so normal y start is `0x9E + 0x45 = 0xE3` (227) on the sidebar surface.

Handoff-critical evidence: decompile `0x0063FB20`, decompile `0x006A6C30`, decompile `0x006A5130`; executable disassembly range checked read-only with `disassemble_bytes 0x0063FB20..0x0063FDBF`.

### 2. POWERP.SHP segments are native SHP draws, including unconditional even-flash blink

Active in YR: Yes. `PowerClass__Draw @ 0x0063FB20` computes `total_segments = (DAT_00B0B504 + 3) / 3`, draws frame `0` for empty top segments, optionally draws frame `4` when flash counter `+0x151C > 0` and even, then draws surplus frame `1`, output frame `2`, and drain frame `3`. Every draw uses `g_PowerBarSHP`, flags `0x400`, brightness `1000`, and advances y by exactly `3`.

The frame-4 blink is not gated by `surplus_segments > 0`; it is drawn whenever the flash counter condition passes. The later surplus loop starts from `1` if the blink occupied the boundary.

Handoff-critical evidence: decompile `0x0063FB20`; executable disassembly range checked read-only with `disassemble_bytes 0x0063FB20..0x0063FDBF`.

### 3. Credits text is anchored on the sidebar surface at x = sidebar-surface width / 2, y = 2

Active in YR: Yes. `MainGame_SidebarDraw @ 0x006D0A30` draws the sidebar/radar background, calls `CreditsClass__Draw((sidebar + 0x551C))`, then continues sidebar composition. `CreditsClass__Draw @ 0x004A2370` reads `g_SidebarSurface` width through vtable `+0x7C`, sets `x = width / 2`, sets `y = 2`, packs the current sidebar text color, and draws with flags `0x4108`. Normal-player mode draws the displayed credits value at `this+0x04`; observer mode uses the same anchor and flags for elapsed-time text.

Fresh decompile does not support the older phrasing "x = screen_width / 2"; it is centered on the current sidebar surface. It also does not corroborate `DrawCreditsSHPBackground @ 0x006D0E60` as a simple `CREDITS.SHP` draw: the fresh decompile shows a frame-0 draw through the global currently named `g_RADAR_SHP`, then sets `DAT_00B0B518 = 1`. Global naming or prior doc mapping needs a separate asset-global audit before calling the background `CREDITS.SHP`.

Handoff-critical evidence: decompile `0x004A2370`, decompile `0x006D0A30`, decompile `0x006D0E60`; executable disassembly range checked read-only with `disassemble_bytes 0x006A97F0..0x006AA087` for sibling text-layout draw code and `0x0063FB20..0x0063FDBF` for power draw.

### 4. Ready/status text and dark rect use center anchor x = cameo_x + 0x1E, y = cameo_y + 1

Active in YR: Yes. `StripClass__Draw @ 0x006A9540` computes cameo-local `cameo_x = DAT_00B0B4FC * column - g_SidebarX + StripClass+0x20` and `cameo_y = DAT_00B0B500 * row + 1 + StripClass+0x24` (plus scroll adjustment when `StripClass+0x3F` is set). For Ready/status text, it calls `ComputeTextRect` at anchor `(cameo_x + 0x1E, cameo_y + 1)` with flags `0x142`, x pad `2`, y pad `1`, darkens that computed rect with `AlphaBlendRect(0, 0xAF)`, then calls `DrawText` with the same anchor, packed sidebar text color, and flags `0x142`.

`ComputeTextRect @ 0x004A59E0` proves the resulting center-aligned dark rect is:
- x = anchor_x - text_width / 2 - 2
- y = anchor_y - 1
- w = text_width + 4
- h = GAME.FNT cell_height + 2

Handoff-critical evidence: decompile `0x006A9540`, decompile `0x004A59E0`; executable disassembly range checked read-only with `disassemble_bytes 0x006A97F0..0x006AA087`.

### 5. Queue-count text is top-right anchored; Hold/status shifts left only when queue text is already present

Active in YR: Yes. In `StripClass__Draw @ 0x006A9540`, queue count text uses anchor `(cameo_x + 0x3C, cameo_y + 1)`, flags `0x242`, x pad `2`, y pad `1`, and the same `AlphaBlendRect(0, 0xAF)` plus packed sidebar text color path. `ComputeTextRect @ 0x004A59E0` shows this right-aligned dark rect is:
- x = anchor_x - text_width - 2
- y = anchor_y - 1
- w = text_width + 4
- h = GAME.FNT cell_height + 2

For Hold/status text after the progress overlay, if queue text was drawn (`bVar2` true), the text anchor changes to `(cameo_x + 2, cameo_y + 1)` and flags `0x42`; otherwise it uses the centered Ready/status anchor `(cameo_x + 0x1E, cameo_y + 1)` and flags `0x142`.

Handoff-critical evidence: decompile `0x006A9540`, decompile `0x004A59E0`; executable disassembly range checked read-only with `disassemble_bytes 0x006A97F0..0x006AA087`.

## Implementation Handoff

| Verified behavior | Rust delta | Affected surface | Acceptance scenario | Proposed test name | Risk |
|---|---|---|---|---|---|
| Soviet power bar draws at sidebar-surface x `0`, y `227`, with native frame size and 3px y advance | Rust uses `layout.tabs_y + spec.power_bar_top_y` and stretches frames to `power_bar_width` | `src/app_sidebar_build.rs`, `src/sidebar/layout_spec.rs` | Soviet first frame with a stable power state places `POWERP.SHP` frame 0/1/2/3 segments starting at the binary y origin and x 0 relative to the sidebar surface | `test_soviet_power_bar_origin_uses_native_sidebar_surface_offsets` | HIGH screenshot parity |
| Frame-4 blink draws on even flash values even when surplus count is zero | Rust gates blink with `flashing && n_surplus > 0` | `src/app_sidebar_build.rs`, `src/sidebar/power_bar_anim.rs` | Low/no-surplus power transition still draws a single frame-4 boundary segment on flash counter 10/8/6/4/2 | `test_power_bar_even_flash_draws_boundary_without_surplus_segments` | MEDIUM visible transition drift |
| Credits text uses GAME.FNT-like text on `g_SidebarSurface` centered at surface width/2, y=2, color table, flags `0x4108` | Rust draws credits with egui proportional font, hardcoded color `(230,240,255)`, center-y offset from radar | `src/app_sidebar_text.rs`, sidebar bit-font renderer | Soviet credits text appears centered in the sidebar surface top strip at y=2 in yellow, not as an egui overlay offset from radar | `test_soviet_credits_text_anchor_and_color_match_sidebar_surface` | HIGH screenshot/text parity |
| Ready and queue dark rects are exact `ComputeTextRect` outputs | Rust is close, but comments/logic should preserve exact `0x142`, `0x242`, and queue-present Hold/status `0x42` split | `src/app_sidebar_build.rs` | A ready Soviet cameo with a queue-count badge keeps queue text top-right and shifts Hold/status left only on the queue-present path | `test_sidebar_ready_queue_dark_rects_use_native_anchors_and_flags` | MEDIUM overlay parity |

## Negative Facts / Do Not Do

- Do not place the Soviet power bar from tab layout coordinates; `PowerClass__Draw @ 0x0063FB20` uses sidebar-surface x `0`/`5` and y `g_SidebarWidth + 0x45`.
- Do not suppress the frame-4 blink when `surplus_segments == 0`; `0x0063FB20` gates blink only on positive even flash counter.
- Do not render credits with egui/system font for parity; `CreditsClass__Draw @ 0x004A2370` uses the sidebar surface text path and flags `0x4108`.
- Do not describe credits text x as full `screen_width / 2`; fresh decompile uses `g_SidebarSurface` width / 2.
- Do not call `0x006AC480` `DrawProgressBar`; fresh Ghidra decompiles it as `SidebarClass__DrawCameoText`, and `StripClass__Draw @ 0x006A9540` draws the GCLOCK/progress SHP directly.

## Remaining Uncertainty

- Exact `POWERP.SHP` frame native width/offset was not dumped from retail assets in this slot; binary proves native SHP draw and 3px y advance, not the decoded image dimensions.
- The exact asset-global identity behind `DrawCreditsSHPBackground @ 0x006D0E60` needs a separate global/SHP-load audit because fresh decompile shows a `g_RADAR_SHP`-named pointer where older docs claim `CREDITS.SHP`.
- Runtime DirectDraw bit-shift descriptor values for the packed text color were not rechecked; this slot only traced coordinate/color consumers.
- Stock observer sidebar elapsed-time formatting used the same anchor but was not expanded beyond the visible layout facts.

## Stale-Doc Wording

- `docs/research/SIDEBAR_READY_TEXT_RENDERING.md`: replace "`0x006ac480` | `DrawProgressBar` | Clock/bar progress overlay" with "`0x006AC480` | `SidebarClass__DrawCameoText` | Draws cameo text through GAME.FNT; `StripClass::Draw @ 0x006A9540` draws the GCLOCK/progress SHP directly."
- `docs/research/ADDRESS_MAP.md`: replace "`0x006AC480` | DrawProgressBar" with "`0x006AC480` | SidebarClass__DrawCameoText".
- `docs/research/CREDITS_COUNTER_SYSTEM.md`: replace "x = screen_width / 2" with "x = `g_SidebarSurface` width / 2; credits text is centered within the sidebar surface."
- `docs/research/CREDITS_COUNTER_SYSTEM.md`: replace "DrawCreditsSHPBackground draws CREDITS.SHP frame 0" with "Fresh `0x006D0E60` decompile shows a frame-0 draw through the global currently named `g_RADAR_SHP`; the older CREDITS.SHP mapping needs a separate asset-global audit before being used as implementation proof."

Status: COMPLETE.
