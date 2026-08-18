# MAIN_MENU_RA2TS_BACKGROUND_MOVIE_TRACE

**Slot:** 1 of trace-swarm — main-menu background movie (RA2TS)
**Scenario:** Launch main-menu shell at any resolution. Trace asset selection,
placement, playback, codec, z-order, and loop behavior against dialog 0xE2.
**Date:** 2026-05-19

---

## Baseline — Our Code

| File | Key symbol |
|------|-----------|
| `src/ui/main_menu_shell/layout.rs` | `movie_base_for_screen_width`, `movie_origin`, `RA2TS_L_W/H=632x570`, `RA2TS_S_W/H=472x450` |
| `src/app_main_menu_shell_render.rs` | `ensure_movie_for_current_layout`, `render_main_menu_shell`, `MOVIE_DEPTH=0.00095` |
| `src/render/bink_movie.rs` | `BinkMovieSurface::step` — wall-clock accumulator, `looping=true` restart at frame 0 |

---

## Stage Results

### Stage 1 — Asset Selection Rule

**PASS**

gamemd (`FUN_00531CC0` @ 0x00531CC0, confirmed via `decompile_function 0x00531CC0`):
```c
if (g_ScreenWidth == 0x280) {  // == 640
    pcVar12 = s_Ra2ts_s_00825ce8;
} else {
    pcVar12 = s_Ra2ts_l_00825ce0;
}
SendMessageA(pHVar6, 0x4e4, 0, (LPARAM)pcVar12);
```
Only branches on `screen_w == 640`. Any width other than 640 → `ra2ts_l.bik`.

Our code (`layout.rs:143`):
```rust
pub fn movie_base_for_screen_width(screen_w: u32) -> MainMenuMovieBase {
    if screen_w == 640 { MainMenuMovieBase::Ra2tsS } else { MainMenuMovieBase::Ra2tsL }
}
```
Exact match. The same strings are used at 800, 1024, and 1920. No intermediate
size gets the small variant. Both implementations: 640 → `ra2ts_s.bik`, everything
else → `ra2ts_l.bik`.

---

### Stage 2 — Movie Rect Dimensions

**PASS** (native size confirmed; stretching not observed in shell path)

gamemd passes the filename to the MSBinkAnim custom control (0x71A) via message
0x4E4. The OwnerDraw handler (`OwnerDraw_Static_006153E0` @ 0x006153E0) calls
`FUN_00432750` which reads native width/height directly from the Bink header:
```c
iVar3 = **(int **)(param_1 + 4);          // header.width
iStack_3c = (*(int **)(param_1 + 4))[1];  // header.height
```
No scaling is applied before `BinkCopyToBuffer`. The asset survey (in
`MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`) confirms:
- `ra2ts_l.bik` native: 632x570
- `ra2ts_s.bik` native: 472x450

Our constants `RA2TS_L_W/H = 632x570` and `RA2TS_S_W/H = 472x450` (layout.rs:9-12)
match the header values exactly. The movie is rendered at native BIK resolution in
both gamemd and our code — no stretching in either.

---

### Stage 3 — Movie Origin (Centering)

**PASS**

gamemd (`FUN_00531CC0` and `FUN_0052B9B0`, both confirmed via decompile):
```c
if (g_ScreenWidth < 0x321)  // < 801, i.e., <= 800
    X = 0;
else
    X = (g_ScreenWidth + -800) / 2;
if (g_ScreenHeight < 0x259)  // < 601, i.e., <= 600
    Y = 0;
else
    Y = (g_ScreenHeight + -600) / 2;
SetWindowPos(hWnd, NULL, X, Y, -1, -1, 0xD);  // reposition only, keep size
```

Our code (`layout.rs:129-140` — `movie_origin`):
```rust
let x = if screen_w <= SHELL_BASE_W { 0 } else { (screen_w - SHELL_BASE_W) / 2 };
let y = if screen_h <= SHELL_BASE_H { 0 } else { (screen_h - SHELL_BASE_H) / 2 };
```
`SHELL_BASE_W=800`, `SHELL_BASE_H=600`. Comparison `<= 800` is equivalent to
`< 801`. Integer division `(w - 800) / 2` is identical. Numerically exact for all
tested resolutions (640x480, 800x600, 1024x768, 1920x1080).

Note: `compute_responsive_layout` (used at runtime) scales the base movie rect
rather than using `movie_origin` directly. At 1920x1080 the responsive path
produces `(0, 0, 1264, 855)` rather than the retail `(560, 240, 632, 570)`.
This is an intentional design choice (fills window vs letterbox) but is a
**behavioral divergence** from the base `compute_layout` path at oversized
resolutions. Flagged in Adjacent Findings.

---

### Stage 4 — Movie Source Archive

**PASS**

gamemd archive search order (verified in `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`):
- `WinMain` opens `LANGMD.MIX` first (0x006BD7EF), then `LANGUAGE.MIX` second (0x006BD81F).
- MIX constructor inserts each new archive at the head of the search list.
- Therefore `LANGUAGE.MIX` is searched before `LANGMD.MIX`.
- `ra2ts_l.bik` and `ra2ts_s.bik` are served from `LANGUAGE.MIX` when both archives
  contain the file (confirmed by `bik-survey` largest-frame mismatch).

Our code (`app_main_menu_shell_render.rs:268`):
```rust
if asset_name.eq_ignore_ascii_case("ra2ts_l.bik")
    && !source.eq_ignore_ascii_case("language.mix")
{
    log::warn!("ra2ts_l.bik resolved from {source}; retail duplicate priority ...");
}
```
`asset_manager.rs` `OPTIONAL_TOP_LEVEL` lists `language.mix` before `langmd.mix`.
`or_insert` semantics means first-in wins. This mirrors gamemd's `LANGUAGE.MIX`
priority. Log warning fires if another archive wins, which matches the expected
gamemd priority.

---

### Stage 5 — Playback Rate

**FAIL**

gamemd playback mechanism (verified in `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`
and `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md`):

- `SetTimer(hwnd_0x71A, 0x65, 0x22, NULL)` → fires every **34 ms** (~29.4 Hz).
- Each timer tick calls `BinkMovie_Update` → `_BinkWait_4` → `_BinkDoFrame_4` → `_BinkCopyToBuffer_28` → `_BinkNextFrame_4`.
- `_BinkWait_4` is the gate: it returns nonzero if the next frame is NOT yet due
  (Bink's internal timing says "too early"). The decode loop only runs when Bink
  says the frame is ready.
- `ra2ts_l.bik` / `ra2ts_s.bik` are 15 fps. A frame is due every ~66.7 ms, so
  ~2 timer ticks pass per frame advance.
- Bink's own `_BinkWait` uses the file's `fps_numerator / fps_denominator` header
  values (BINK header at +0x14 / +0x18) to compute readiness, independent of
  wall-clock `Instant::now()`.

Our code (`bink_movie.rs:82-110`):
```rust
pub fn step(&mut self, gpu: &GpuContext, elapsed_secs: f64) -> Result<...> {
    let fps = self.fps();
    for _ in 0..frames_due(&mut self.accumulator_secs, elapsed_secs, fps, 4) {
        // advance frame
    }
}
```
`frames_due` uses a wall-clock accumulator (`elapsed.as_secs_f64()`). This is a
reasonable approximation but differs from gamemd's mechanism:

| Property | gamemd | Our code |
|---------|--------|---------|
| Frame-ready test | `_BinkWait_4` — Bink SDK internal timer, CPU-cycle based | wall-clock accumulator (`Instant::elapsed`) |
| Catch-up cap | Bink internal (loops while `BinkWait==0`) | max 4 frames per `step` call |
| Frame rate source | Bink header fps_num/fps_den at runtime | `BinkFile::header.fps()` — same header |

Player impact: under normal conditions (stable 60+ Hz render loop) both produce
15 fps output, but under load or sleep the behavior diverges. Our accumulator can
drift more than Bink's internal timer under latency spikes. Code: `bink_movie.rs:171-183`.

---

### Stage 6 — Loop Behavior

**FAIL**

gamemd (verified in `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`, section 4):

```text
WM_TIMER 0x65:
  changed = vtable[+0x04]()  // BinkMovie_Update
  if changed: InvalidateRect
  ended = vtable[+0x14]()    // IsFinished: frame >= total OR frame wrapped
  if ended and loop_flag != 0:
      vtable[+0x1C](1)        // BinkGoto(handle, frame=1, wait=1)
      log("Looping movie")
```

Loop mechanism: `BinkGoto(handle, 1, 1)` — jumps to **frame 1** (not frame 0),
waiting for Bink readiness. The Bink SDK numbers frames from 1.

Our code (`bink_movie.rs:135-143` — `restart_at_original_frame_one`):
```rust
fn restart_at_original_frame_one(&mut self) -> Result<(), ...> {
    self.decoder.flush();
    let pkt = self.file.video_packet(0)?;  // index 0 = BIK frame 0 (first)
    let frame = self.decoder.decode_frame(pkt)?;
    self.rgba = frame_to_rgba(frame);
    self.current_frame = 1;
    self.accumulator_secs = 0.0;
    Ok(())
}
```
We restart at packet index 0 (`video_packet(0)`) = BIK frame 0, while gamemd calls
`BinkGoto(handle, 1, 1)` = frame 1 (the BIK SDK counts from 1). Whether frame 0
and frame 1 differ is file-dependent but the call sites diverge: gamemd always
passes `1`, our code always reads packet `0`.

Additionally, our restart clears `accumulator_secs = 0.0`, which may cause a brief
stutter if the movie was partway through a frame interval at loop time. gamemd's
`BinkGoto` with wait=1 synchronizes to Bink timing internally.

Player impact: visible at loop point — potential 1-frame flicker or wrong first
frame after the loop. Fires once per full movie play (~29 seconds at 15 fps /
431 frames). Code: `bink_movie.rs:135-143`.

---

### Stage 7 — Codec / Pixel Format

**UNCHECKED** (partial — codec confirmed, exact pixel format not verified)

gamemd codec:
- Confirmed Bink via `binkw32.dll` imports: `_BinkOpen_8`, `_BinkDoFrame_4`,
  `_BinkCopyToBuffer_28`, `_BinkNextFrame_4`, `_BinkWait_4`.
- `_BinkDDSurfaceType_4` is called on the DirectDraw primary surface (`DAT_00887308 + 0x1C`)
  to auto-detect the surface pixel format. Result stored at `BinkMovieHandle + 0x08`
  and passed to `BinkCopyToBuffer` as the format argument.
- Actual format (RGB565 vs RGB888 vs BGRA8888) depends on the game's DirectDraw
  surface format, which is set by the display initialization path not traced here.
- With `0x80000000` ORed in (`BinkMovie_ExplicitDraw_005C05F0` path), this instructs
  Bink to upscale any YUV data to match the surface format.

Our code:
- `frame_to_rgba` outputs RGBA8888 unconditionally.
- YUV → RGB conversion uses MPEG range for MPEG-range frames, JPEG range for
  JPEG-range frames (verified at `bink_movie.rs:157`).
- We do not query the surface format; we always produce 32-bit RGBA.

Whether this produces visible color differences depends on whether the retail BIK
files use MPEG or JPEG YUV range. The unit test `mpeg_black_and_white` confirms
MPEG path works (Y=16→black, Y=235→white). JPEG path: Y=0→black, Y=255→white.
Cannot PASS this without knowing the exact YUV range flag in the retail BIK headers
at runtime.

---

### Stage 8 — Z-Order / Depth

**UNCHECKED** (gamemd uses a different compositing model)

gamemd draw order (verified in `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
and `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md`):

1. `WM_PAINT_Handler @ 0x00621E90` draws right-panel SHP stack + background overlay to
   offscreen shell surface.
2. `MainMenuDialog0xE2_Proc_00531F60` receives WM_PAINT → sends `0x4F0` to child `0x71A`.
3. `BinkMovie_ExplicitDraw` blits current Bink frame to the **primary DirectDraw surface**
   directly (not the offscreen shell surface).
4. Win32 then draws owner-draw button controls on top via normal `WM_DRAWITEM` dispatch.

This is a DirectDraw primary-surface blit architecture with child window z-ordering
via Win32. There is no depth buffer involved.

Our code uses a GPU depth buffer with float depth values:
- `MOVIE_DEPTH = 0.00095` (back)
- `CHROME_DEPTH = 0.00085`
- `BUTTON_DEPTH = 0.00080`
- `TEXT_DEPTH = 0.00070` (front)

The relative order (movie behind chrome behind buttons behind text) matches the
intent of gamemd's Win32 z-order. Whether the exact rendering artifacts (edge
pixels, sub-pixel blending at z-boundaries) are visible to the player is not
determinable without side-by-side capture. Architecturally, the layering intent
is preserved. Stage is UNCHECKED because the exact compositing paths are
fundamentally different (DirectDraw primary blit vs GPU depth + passthrough).

---

### Stage 9 — Movie Alpha / Tint

**PASS**

gamemd (`FUN_00432E40`, `FUN_00433060`):
`BinkCopyToBuffer` is called with the DirectDraw surface format type as flags
(optionally OR'd with `0x80000000`). No alpha blending, no tint multiplication —
Bink copies directly to the surface pixel-for-pixel. The `0x80000000` flag is
a Bink SDK flag for "upscale" or "surface type hint", not a transparency value.

Our code (`app_main_menu_shell_render.rs:236-246`):
```rust
SpriteInstance {
    tint: [1.0, 1.0, 1.0],
    alpha: 1.0,
    ..
}
```
Full opacity, neutral tint. No color modulation applied. This matches gamemd's
direct pixel copy behavior (no alpha, no tint).

---

## Adjacent Findings

### A. `compute_responsive_layout` diverges from retail at >800px width

`compute_responsive_layout` (`layout.rs:342-371`) stretches the base 800x600 layout
to fill the full swapchain at any resolution. At 1920x1080, the movie rect becomes
`(0, 0, 1264, 855)` — filling the left portion of the screen. gamemd places the movie
at its native `632x570` centered at `(560, 240)` on a 1920x1080 display.

This is an intentional design decision (fills window vs letterbox), documented in the
responsive layout comment: "intentionally drifts from retail pixel parity." It is a
deliberate non-parity choice for modern resolutions.

### B. `BinkGoto` frame argument: 1-based vs 0-based index

`BinkGoto(handle, 1, 1)` in gamemd passes frame `1` (1-indexed per Bink SDK). Our
`video_packet(0)` reads from index `0` (0-indexed in our BinkFile parser). If the
BIK file stores frame 0 and frame 1 as the same content (common in looped movies
where frame 0 is a keyframe), this produces no visible difference. If they differ
(rare in practice for looping intros), the loop-point first frame will be wrong.
Requires retail BIK inspection to fully resolve.

### C. WM_TIMER `0x22` ms / `BinkWait` vs our wall-clock accumulator

The timer cadence (34 ms) is separate from the Bink frame rate (66.7 ms at 15 fps).
gamemd polls twice per frame and lets `_BinkWait` decide if a frame is ready. Our
`frames_due` accumulator produces equivalent output under ideal conditions but
diverges under load. Under CPU pressure, our accumulator can advance 2–3 frames in
one step call where gamemd would advance 1 per `BinkWait` poll.

---

## Sources

- Ghidra MCP (live, read-only): `0x00531CC0`, `0x00531F60`, `0x0052B9B0`,
  `0x006153E0`, `0x00432750`, `0x00432E40`, `0x00432C70`, `0x00432BF0`,
  `0x00433060`, `0x005C0580`, `0x005C05F0`
- Strings: `0x00825CE0` (`Ra2ts_l`), `0x00825CE8` (`Ra2ts_s`),
  `0x00840D4C` (`LANGUAGE.MIX`), `0x00840D5C` (`LANGMD.MIX`)
- Research docs (all in `ra2-rust-game-docs/`):
  - `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`
  - `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md`
  - `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
- Rust source: `src/ui/main_menu_shell/layout.rs`, `src/app_main_menu_shell_render.rs`,
  `src/render/bink_movie.rs`
