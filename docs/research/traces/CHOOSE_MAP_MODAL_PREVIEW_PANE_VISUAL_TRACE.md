# Choose Map Modal Preview Pane Visual Trace

**Mechanic:** Choose Map modal (dialog `0x6B`) — preview pane visuals at 800×600.  
**Scenario:** Standard (non-random) map committed, map has PreviewPack image and `[Header]` start waypoints. Modal open at 800×600.  
**Tracer slot:** 4 / trace-swarm batch VERA20k  
**Active in YR:** Yes (verified via `0x005E68A0`, `0x005E6920`, `0x00640710` decompile)  
**Status:** COMPLETE  

---

## Stage Verdicts

| # | Stage | Verdict | Evidence |
|---|---|---|---|
| 1 | Preview anchor rect (control `0x468`) = `(644,37,144,112)` at 800px | PASS | See §1 |
| 2 | Aspect-fit formula (per-mille integer truncation, scale/2000 centering) | PASS | See §2 |
| 3 | Frame/outline + backdrop behind image | PASS | See §3 |
| 4 | Start marker projection (per-mille) | PASS | See §4 |
| 5 | Start marker hotspot offset `(-9,-6)` | PASS | See §5 |
| 6 | 8-marker cap | PASS | See §6 |
| 7 | Markers NOT drawn for random-map sentinel | PASS | See §7 |
| 8 | Modal draws committed preview (not highlighted-row preview) | PASS | See §8 |
| 9 | RGB channel order | PASS | See §9 |
| 10 | Label origin offset `(-2,-6)`, 1-based, Yellow color | FAIL | See §10 |

---

## §1 — Preview Anchor Rect `(644,37,144,112)` at 800px

**Verdict: PASS**

Binary: `DrawStartPositions @ 0x00640710` calls `GetDlgItem(param_2, 0x468)` then `FUN_00775690` to convert to backbuffer coords. `FUN_0060b1d0` places `0x468` via right-anchor formula: `x = screen_w - max((screen_w-800)/2, 0) - 144 - 12`. At 800×600: `x = 800 - 0 - 144 - 12 = 644`. `y = 37`. `w = 144`, `h = 112`. Evidence: `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md §2`.

Rust: `right_anchor(screen_w, screen_h, dlu_rect(428,23,96,69))` with `inset = (156-144)/2 = 6`... wait. `RIGHT_PANEL_WIDTH = 168`; `inset = (168 - 144) / 2 = 12`. `x = 800 - 0 - 144 - 12 = 644`. `y = 37` (from DLU conversion). Unit test at `src/ui/skirmish_shell/layout.rs:1145` asserts `layout.preview = RectPx::new(644, 37, 144, 112)`. Matches binary exactly.

Choose Map modal also uses `right_anchor(screen_w, screen_h, preview_base)` with the same `preview_base = dlu_rect(428, 23, 96, 69)`, yielding identical rect. Confirmed at `src/ui/skirmish_shell/layout.rs:582` and unit test at line 1145.

---

## §2 — Aspect-Fit Formula (Per-Mille Integer Truncation)

**Verdict: PASS**

**Binary (live decompile `0x00640710`):**
```
scale_h = (child_h * 1000) / src_h      // iVar2
scale_w = (child_w * 1000) / src_w      // iStack_38
scale = min(scale_h, scale_w)           // if (iVar2 < iStack_38) iStack_38 = iVar2
iStack_3c = src_w * scale               // src_w * scale
iStack_38 = src_h * scale               // src_h * scale
fit_x = child_x + child_w/2 - iStack_3c/2000
fit_y = child_y + child_h/2 - iStack_38/2000
fitted_w = iStack_3c / 1000
fitted_h = iStack_38 / 1000
```
All divisions are C integer truncation (IDIV). Magic reciprocals `0x10624DD3` (÷1000) and SAR 7 (÷2000) confirmed in prior `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md §3.2`.

**Rust (`src/app_skirmish_shell_render/preview.rs:247-265`):**
```rust
let scale_w = dst.w * 1000 / src_w;
let scale_h = dst.h * 1000 / src_h;
let scale = scale_w.min(scale_h);
let fitted_w = src_w * scale / 1000;
let fitted_h = src_h * scale / 1000;
RectPx::new(
    dst.x + dst.w / 2 - (src_w * scale) / 2000,
    dst.y + dst.h / 2 - (src_h * scale) / 2000,
    fitted_w,
    fitted_h,
)
```
Identical formula and truncation semantics. Live decompile directly corroborates the formula constants.

**Concrete fixture (Dustbowl 138×75 in 144×112):**  
scale_w = 144*1000/138 = 1043, scale_h = 112*1000/75 = 1493. scale = 1043.  
fitted_w = 138*1043/1000 = 143, fitted_h = 75*1043/1000 = 78.  
fit_x = 644 + 72 - (138*1043)/2000 = 644 + 72 - 71 = 645.  
fit_y = 37 + 56 - (75*1043)/2000 = 37 + 56 - 39 = 54.  
Result: `(645, 54, 143, 78)`. Matches binary per `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md §9`.

Note: prior docs reported Rust produced `(644, ?, 144, ?)` from float/round. The current code uses integer truncation matching the binary.

---

## §3 — Frame/Outline + Backdrop Behind Image

**Verdict: PASS**

Binary: `DrawStartPositions` blits preview pixels to `DAT_00887310` (destination surface), then draws `STARTBUT.SHP` and labels on top. The outline/bevel around `0x468` is part of the dialog resource template (a static control chrome), not drawn by `DrawStartPositions` itself.

Rust:
- Backdrop: `push_solid_rect(..., preview_rect, [0.0,0.0,0.0], SHELL_PREVIEW_BACKDROP_DEPTH)` at `src/app_skirmish_shell_render.rs:550-556`. Gated on `preview_instance.is_some()` (same as gamemd — no blit if no surface). Black fill behind the fitted image. ✓
- Outline: `push_rect_outline(..., layout.preview, OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68, ...)` in `src/app_skirmish_shell_render/modals.rs:162-168`. Drawn unconditionally for the choose map modal. This matches the dialog chrome behavior (the static border is always rendered regardless of preview existence).
- `SHELL_PREVIEW_BACKDROP_DEPTH (0.00059) > SHELL_PREVIEW_SURFACE_DEPTH (0.00058)`: backdrop sits behind image. ✓ (verified by unit test at `src/app_skirmish_shell_render.rs:834-836`).

---

## §4 — Start Marker Projection (Per-Mille)

**Verdict: PASS**

**Binary (live decompile `0x00640710`):**
```
x_per_mille = ((WaypointX[i] - StartX) * 1000) / Width
y_per_mille = ((WaypointY[i] - StartY) * 1000) / Height
anchor_x = (x_per_mille * fitted_w) / 1000 + fit_x
anchor_y = (y_per_mille * fitted_h) / 1000 + fit_y
```
Fields read from `g_ScenarioClass_Instance + 0x112c/0x1130/0x1134/0x1138/0x1140+i*8/0x1144+i*8`, sourced from `[Header] StartX/StartY/Width/Height/WaypointN`. Evidence: live decompile shows exact offsets.

**Rust (`src/app_skirmish_shell_render/preview.rs:52-78`):**
```rust
x_per_mille = ((point.x - bounds.origin_x) as i64 * 1000) / bounds.width as i64;
y_per_mille = ((point.y - bounds.origin_y) as i64 * 1000) / bounds.height as i64;
x = fitted_preview_rect.x + (x_per_mille * fitted_preview_rect.w as i64 / 1000) as i32;
y = fitted_preview_rect.y + (y_per_mille * fitted_preview_rect.h as i64 / 1000) as i32;
```
Same formula and truncation. Rust widens to `i64` for intermediate products (avoids overflow for large map coords); truncation direction matches signed IDIV. `PreviewSourceBounds` is populated from `[Header]` only (`src/app_list_maps.rs`). Markers not clipped to fitted rect before submission (destination-surface clip applies) — matches binary.

---

## §5 — Start Marker Hotspot Offset `(-9, -6)`

**Verdict: PASS**

**Binary (live decompile):** STARTBUT.SHP top-left submitted as `iStack_a0 = ... + -9` (X) and `piStack_9c = ... + -6` (Y) before `CC_Draw_Shape`. Constants read directly from decompile output.

**Rust:** `START_MARKER_OFFSET_X = -9`, `START_MARKER_OFFSET_Y = -6` at `src/app_skirmish_shell_render.rs:55-56`. Applied in `start_marker_top_left` at `src/app_skirmish_shell_render/preview.rs:45-50`. Exact match.

---

## §6 — 8-Marker Cap

**Verdict: PASS**

**Binary (live decompile):** Guard: `if (0 < iVar2 && iVar2 < 9)` where `iVar2 = ScenarioClass+0x113C` (NumberStartingPoints). Loop: `do { ... iVar5++ } while (iVar5 < ScenarioClass+0x113C)`. Maximum iterations = 8 (when count = 8 and loop runs 0..7).

**Rust:** `bounds.start_points.iter().take(8)` at `src/app_skirmish_shell_render/preview.rs:67`. Count gate: `app_list_maps.rs` rejects `count <= 0 || count >= 9` when building `PreviewSourceBounds`. Combined effect: at most 8 markers, none drawn for count 0 or ≥9. Matches binary.

---

## §7 — Markers NOT Drawn for Random-Map Sentinel

**Verdict: PASS**

**Binary:** `RandMap.img` preview contains baked `4x4` red marker pixels from `GenerateTerrainPreview @ 0x00641140`. `DrawStartPositions` live `STARTBUT.SHP` overlays draw only when `0 < ScenarioClass+0x113C < 9`. For `RandMap.Sed` the `[Header]` fields default to `-1`, so `NumberStartingPoints = -1`, gate fails, no live overlays. Evidence: `SKIRMISH_MAP_PREVIEW_START_MARKER_LIFECYCLE_BROAD_RECHECK_GHIDRA_REPORT.md §3.8`.

**Rust:** `is_random_map_sentinel_entry` identifies `RandMap.Sed` entries; `preview_has_baked_start_markers = true` for those entries; `should_draw_start_marker_overlays` returns `false` when `preview_has_baked_start_markers` is true (`src/app_skirmish_shell_render/preview.rs:237-245`). No live STARTBUT overlays emitted. Correct behavior.

---

## §8 — Modal Draws Committed Preview, Not Highlighted Row

**Verdict: PASS**

**Binary:** `DrawStartPositions` (called from `0x5E6920` WM_PAINT) consumes `DAT_00AC1154` — the globally committed preview wrapper. No `LB_GETCURSEL` read from listbox `0x553` in the paint path. Passive row highlight has no preview-refresh branch. Evidence: `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md §3.3, §3.4`.

**Rust:** `ensure_selected_preview_texture` uses `state.skirmish_shell_state.selected_map_idx` (committed index, line 199). `preview_rect` comes from `choose_map_layout.preview` when modal is open (line 492-495), but the texture itself remains keyed to committed `selected_map_idx`. `modal.highlighted_filtered_index` is NOT used for the preview texture selection. No texture refresh on highlight-only state changes. Matches binary.

---

## §9 — RGB Channel Order

**Verdict: PASS**

**Binary:** `0x00641B00` writes 3-byte triples as R,G,B per pixel into the preview surface. Evidence: `PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md` (verified binary read of the loader byte-write loop, confirmed RGB order). Corroborated by `SKIRMISH_MAP_PREVIEW_START_MARKER_LIFECYCLE_BROAD_RECHECK_GHIDRA_REPORT.md §3.1` ("RGB order is the settled PreviewPack order from prior channel-order reports; this recheck confirmed the same function").

**Rust:** `PREVIEW_CHANNEL_ORDER = PreviewChannelOrder::Rgb` at `src/map/preview.rs:90`. `push_rgba_from_preview_pixel` expands `[r,g,b]→[r,g,b,255]` (lines 153-159). Correct. Note: `PreviewChannelOrder::Bgr` variant exists but is dead code.

---

## §10 — Label Origin Offset, 1-Based, Yellow Color

**Verdict: FAIL (label color only; origin and 1-based are PASS)**

**Binary (live decompile):**
- Loop index `iVar5` starts at 0, is incremented before the label push (`iVar5 = iVar5 + 1`), so label text = 1-based. ✓
- The label is passed as `iVar5` (already incremented) to `FUN_004a61c0`. Label origin: docs confirm `(anchor_x-2, anchor_y-6)` from assembly `0x006409D7..0x00640A15`.
- **Color:** `FUN_0068ca50("Yellow @ 0x00820524")` resolves the "Yellow" color scheme object before the loop; the color is passed through `FUN_004a61c0` which applies display color loss/shift. The final pixel RGB depends on the runtime DirectDraw surface pixel format (RGB555 or RGB565). It is NOT a guaranteed fixed `[255,255,0]`.

**Rust:**
- Label origin: `start_marker_label_origin(x, y) = (x-2, y-6)` at `src/app_skirmish_shell_render/text.rs:947-949`. ✓
- 1-based text: `(idx + 1).to_string()` at line 933. ✓
- **Label color: `SHELL_LABEL_TEXT_RGB = [1.0, 1.0, 0.0]` = pure yellow `[255,255,0]`.** The binary applies `FUN_0068ca50("Yellow")` → color scheme resolution → `FUN_004a61c0` display conversion. If the runtime pixel format is RGB555, the Yellow color `[255,255,0]` → `[248,248,0]` after 3-bit loss, then expanded back. This is a per-pixel color drift whenever the display is 15-bit. Frequency: every map with valid `[Header]` start markers, every frame the preview is painted.

**Player-visible effect:** Start marker numeric labels appear slightly darker yellow (`#F8F800`) vs. Rust's pure `#FFFF00` when the original runs on 16/15-bit surface. Subtle but measurable in a color-diff.

---

## §11 — Choose Map Modal Preview Uses `choose_map_layout.preview` Not `layout.map_preview`

**Verdict: PASS (same rect formula)**

The orchestrator at `src/app_skirmish_shell_render.rs:492-495` selects `choose_map_layout.preview` when the modal is open, otherwise `layout.map_preview`. Both are computed from the same `right_anchor(screen_w, screen_h, dlu_rect(428, 23, 96, 69))` formula, producing identical rects at any resolution. The choose map modal therefore uses the same `0x468`-equivalent anchor. Correct per `DrawStartPositions` using `GetDlgItem(chooser_hwnd, 0x468)`.

---

## Adjacent Findings (out of primary scope, for follow-up)

1. **`start_marker_label_color` uses hard-coded yellow `[1.0,1.0,0.0]`** — the binary resolves through `FUN_0068ca50("Yellow")` + display color loss. If the color scheme table maps "Yellow" to `[248,248,0]` on a 15-bit surface, this is a one-pixel-per-channel drift. Full analysis requires a `FUN_0068ca50` + `FUN_004a61c0` decode pass (out of this slot's scope).

2. **No `push_start_marker_sprites` per-marker clipping check** — confirmed correct; old mismatch noted in `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md §6` has been resolved in current code.

3. **`aspect_fit_rect` previously used float/round** — old mismatch is resolved; current code uses integer truncation. The old one-pixel drift on Dustbowl is gone.

---

## Sources

- Live Ghidra decompile (read-only): `DrawStartPositions @ 0x00640710` (this session).
- Prior verified docs: `SKIRMISH_PREVIEW_STARTBUT_OVERLAY_RECTS_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`, `SKIRMISH_MAP_PREVIEW_START_MARKER_LIFECYCLE_BROAD_RECHECK_GHIDRA_REPORT.md`, `PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md`.
- Rust scanned: `src/app_skirmish_shell_render/preview.rs`, `src/app_skirmish_shell_render/modals.rs`, `src/app_skirmish_shell_render/text.rs`, `src/app_skirmish_shell_render.rs:480-580`, `src/ui/skirmish_shell/layout.rs`, `src/map/preview.rs`.
- Unit tests referenced: `layout.rs:1145` (`layout.preview == (644,37,144,112)`), `app_skirmish_shell_render.rs:834-836` (backdrop depth), `app_skirmish_shell_render.rs:1416-1419` (marker label color/origin).
