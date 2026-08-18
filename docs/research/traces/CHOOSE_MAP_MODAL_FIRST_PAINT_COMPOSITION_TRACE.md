# Choose Map Modal (0x6B) First-Paint Composition Trace

**Scenario:** Player clicks "Choose Map" (0x5AA) on the Skirmish setup shell at 800×600;
modal 0x6B opens. Trace covers full render composition at first paint only.
**Investigator:** Subagent slot 1, trace-swarm batch
**Date:** 2026-06-01
**Sources:** docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_SHELL_COMPOSITION_GHIDRA_REPORT.md,
SKIRMISH_CHOOSE_MAP_MODAL_0X6B_VISUAL_INTEGRATION_GHIDRA_REPORT.md,
current Rust source scan (src/app_skirmish_shell_render.rs + /modals.rs + /draw_order.rs +
/text.rs, src/ui/skirmish_shell/layout.rs, src/render/skirmish_shell_chrome.rs).
No Ghidra mutation tools were called.

---

## 1. Composition Pipeline — Stage-by-Stage

### Stage 0 — Parent setup shell (0x102) visibility while modal is open

| Dimension | gamemd | Our code | Verdict |
|---|---|---|---|
| Setup hides before modal | `ShowWindow(setup,0)` at `0x006AD93C` before `0x005E68A0`; setup is not composited behind modal | `build_skirmish_shell_instances` early-returns at line ~201-204 when `choose_map_layout.is_some()`, suppressing setup sprite instances. Text is also suppressed at lines 558-568. Preview texture and start-marker passes still run unconditionally in `render_skirmish_shell` | **FAIL** (partial) |

**Detail:** When the modal is active, the sprite instance build path correctly early-returns
(`src/app_skirmish_shell_render.rs:201-204`). However the outer render function
(`render_skirmish_shell`) still: (a) calls `ensure_selected_preview_texture`, (b) builds
preview rect from `choose_map_layout.preview`, (c) adds a black preview backdrop sprite to
the modal's instances (`push_solid_rect` at line ~550), (d) draws the preview texture pass
unconditionally, and (e) draws start-marker overlays unconditionally. This means the
committed setup preview image renders into the modal's right-column `0x468` rect on first
paint. gamemd draws `DAT_00AC1154` via the WM_PAINT of `0x6B` — passive browsing does NOT
refresh the preview — but the setup's already-committed preview is painted into the modal
rect. Our code mirrors this for the texture itself, but the backdrop solid rect is injected
into the modal's instance buffer at `SHELL_PREVIEW_BACKDROP_DEPTH=0.00059`, which is
shallower than the modal backdrop depth (`SHELL_DROPDOWN_DEPTH - 0.00008 ≈ 0.00026`),
meaning the backdrop is behind the modal chrome — acceptable. Start-marker overlays are
drawn if a random-map sentinel is selected; those draw on top of the modal, which is
incorrect (gamemd hides the chooser when random map is clicked, markers only exist in
setup view). This is a narrow edge case.

---

### Stage 1 — Background art: MnScrnLCustomizeBattle.shp at width 800

| Dimension | gamemd | Our code | Verdict |
|---|---|---|---|
| SHP loaded | `0x0072D120`: loads SHP at `g_ScreenWidth == 800` exactly; writes `DAT_00B0FAB8` | `src/render/skirmish_shell_chrome.rs:247-262`: calls `render_shp_entry("MnScrnLCustomizeBattle.shp", ...)` when palette available; atlas field `choose_map_background_800_customize_battle` (line 362-364) | **PASS** (load path present) |
| PAL loaded | `0x0072D120:0x0072D14A-0x0072D15A`: loads `.PAL` unconditionally through `0x0072ADE0` | `src/render/skirmish_shell_chrome.rs:421-434`: `load_choose_map_background_palette` loads `MnScrnLCustomizeBattle.PAL`; logs warning and returns `None` if missing | **PASS** |
| SHP drawn at screen origin | `push_entry_native(..., layout.screen.x, layout.screen.y, SHELL_PARENT_BACKGROUND_DEPTH)` in `modals.rs:101-108` when `atlas.choose_map_background_800_customize_battle` is `Some` | **PASS** |
| Fallback when PAL missing | gamemd: no fallback specified for runtime use (modal is still shown, just unstyled); ours: solid backdrop via `push_solid_rect(SHELL_MODAL_BG_RGB)` at `modals.rs:109-115` | **UNCHECKED** (fallback color accuracy unverified against native) |
| Non-800 width | gamemd: SHP not loaded at >800; `0x6B` still uses fullscreen shell move via `0x00622820`/`0x0060C540`; no substitute SHP documented | Our code: `choose_map_background_entry` returns `None` for non-800 → `push_solid_rect` solid fallback | **UNCHECKED** (exact native fallback appearance at >800 not captured) |

---

### Stage 2 — Dialog frame / backdrop

| Dimension | gamemd | Our code | Verdict |
|---|---|---|---|
| Dialog fills screen | `0x00622820` `MoveWindow(parent,0,0,g_ScreenWidth,g_ScreenHeight,0)` — the dialog is fullscreen | `layout.dialog = RectPx::new(0, 0, screen_w, screen_h)` (layout.rs:563) | **PASS** |
| Backdrop color | Underlying art from MnScrnLCustomizeBattle (verified); primitive color used when art is absent | `SHELL_MODAL_BG_RGB` solid rect when background entry is `None` (`modals.rs:109-115`) | **PASS** (when art loads); **UNCHECKED** (fallback color accuracy) |
| Dialog outline | Native owner-draw uses bevel frames, not a plain rect outline; bevel details from `OwnerDraw_Button_00612B70` path | `push_rect_outline(OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68)` at `modals.rs:116-122` | **FAIL** (native is a two-pixel bevel pair, not a single-color outline; bevel light color `00C5BEA7` is absent from the dialog border) |

---

### Stage 3 — Mode list (0x6EB) and Map list (0x553) panels

| Dimension | gamemd | Our code | Verdict |
|---|---|---|---|
| Mode list rect | Resource `(77,78,130,211)` | `dlu_rect(77, 78, 130, 211)` at layout.rs:564 | **PASS** |
| Map list rect | Resource `(225,78,130,211)` | `dlu_rect(225, 78, 130, 211)` at layout.rs:565 | **PASS** |
| Listbox background fill | Owner-draw fills with dark background color from palette (`OwnerDraw_ListBox_00618D40`; exact color via SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT report) | `SHELL_DROPDOWN_BG_RGB_PENDING_COMBODROPWIN_SOURCE_CAPTURE` — explicitly marked as pending capture in the constant name | **UNCHECKED** (color accuracy unverified; name explicitly marks pending) |
| Selected row fill | Full-row highlight; `OwnerDraw_ListBox_00618D40` report: full-row selected fill | `OWNERDRAW_SELECTED_RGB_FROM_DAT_00AC4604_PACKED_000000FF` = `[1.0, 0.0, 0.0]` (pure red) — `DAT_00AC4604` packed `0xFF` decodes as blue in `00BBGGRR` → `R=0xFF, G=0, B=0` = pure red; however display-format conversion at runtime may produce different actual RGB | **UNCHECKED** (runtime DDraw conversion path not verified against captured screenshot) |
| Scrollbar | Owner-draw scrollbar; 20px wide from sibling report | `CHOOSE_MAP_LISTBOX_SCROLLBAR_W=20` correct (layout.rs:39); track color `SHELL_SCROLLBAR_TRACK_RGB_PENDING_SCROLLBAR_SOURCE_CAPTURE` pending | **PARTIAL** (width correct, color unverified) |
| Bevel frame | Two-pixel outer bevel (sibling row-paint report: `+2 text inset` implies bevel) | `push_ownerdraw_two_pixel_bevel_frame` at `modals.rs:73` | **PASS** |
| Row text inset | Sibling row-paint report: `+2` text inset | `RectPx::new(row_rect.x + 2, ...)` at text.rs:859, 885 | **PASS** |
| Draw order (both lists same depth) | Lists are sibling controls, same z-order level | Both pushed at `SHELL_DROPDOWN_DEPTH - 0.00010`; identical depth means GPU tie-break order may differ from native | **UNCHECKED** (depth tie between two listboxes) |

---

### Stage 4 — Three buttons (Use Map 0x6C5, Create Random Map 0x583, Cancel 0x5C0)

| Dimension | gamemd | Our code | Verdict |
|---|---|---|---|
| Use Map rect | Resource `(425,122,108,23)` | `snap_button_biased_truncate(screen_w, screen_h, dlu_rect(425,122,108,23), panel, SDBTNANM_W)` at layout.rs:556-568 | **PASS** (rect matches; snap function aligns to panel) |
| Create Random Map rect | Resource `(425,149,108,23)` | `snap_button_biased_truncate(..., dlu_rect(425,149,108,23), ...)` at layout.rs:570-577 | **PASS** |
| Cancel rect | Resource `(425,346,108,23)` — near bottom-right | `back_rect(screen_w, panel)` at layout.rs:569; `back_rect` produces the same position as the setup Back button via shared geometry — this is the same right-panel anchor | **PASS** (both compute to the bottom of the right panel, matching the resource) |
| Button art | `OwnerDraw_Button_00612B70`; SDBTNANM SHP frames: frame 2 idle, frame 4 pressed (verified same as setup shell buttons per research doc) | `push_right_panel_button_shp(...)` via SDBTNANM path at `modals.rs:145-161` | **PASS** |
| Button pressed state | Depresses on mouse-down, releases on up | `modal.pressed_button == Some(id)` wired at `modals.rs:157` | **PASS** |
| Draw order relative to lists | Buttons drawn after lists in `push_choose_map_modal_instances`; depth `SHELL_DROPDOWN_DEPTH - 0.00011` (deeper = drawn first in depth test, so buttons are behind lists at same overlap) | `modals.rs:145-161` depth `SHELL_DROPDOWN_DEPTH - 0.00011` vs lists at `- 0.00010`; buttons are numerically deeper → drawn under lists if they overlap; native: buttons are separate controls at right side, no overlap expected | **PASS** (no overlap area; depth ordering inconsequential here) |

---

### Stage 5 — Preview pane (0x468) + outline + image

| Dimension | gamemd | Our code | Verdict |
|---|---|---|---|
| Preview rect | Resource `(428,23,96,69)` — right column, small | `right_anchor(screen_w, screen_h, dlu_rect(428,23,96,69))` at layout.rs:582 | **PASS** |
| Preview outline | Native static `0x468` style `0x50000004/0x20` (SS_BLACKFRAME) — owner-draw draws a dark bevel frame | `push_rect_outline(OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68)` at `modals.rs:162-168` | **FAIL** (single-color outline, not bevel; light bevel component `00C5BEA7` absent) |
| Preview image source | gamemd paints `DAT_00AC1154` (current committed preview) when chooser opens; passive browse does not refresh it per `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md` | Texture drawn from `state.skirmish_preview_texture` — which is the last committed preview; this matches the intended semantics | **PASS** |
| Preview backdrop (black fill behind image) | Native: preview rect background is black (SS_BLACKFRAME / owner-draw background) | `push_solid_rect(..., [0.0,0.0,0.0], SHELL_PREVIEW_BACKDROP_DEPTH)` at render function ~550; depth `0.00059` is behind sprite instances (smallest depth values draw on top) — backdrop is behind everything modal-related | **FAIL** (backdrop depth `0.00059` is shallower than modal chrome at `SHELL_DROPDOWN_DEPTH - 0.00008 ≈ 0.00026`; in a depth-less-wins convention the backdrop renders on top of the modal chrome, not behind it — the backdrop number is larger = farther back; need to confirm wgpu depth convention) |

**Depth convention note:** `SHELL_DROPDOWN_DEPTH = 0.00034`. Subtracting makes values
smaller. If the convention is "smaller depth = front" (standard depth-less), then
`SHELL_PREVIEW_BACKDROP_DEPTH = 0.00059` is *behind* everything modal at ~0.00026-0.00034
range — correct. If "larger = front", order is inverted. The constant name
`SHELL_PARENT_BACKGROUND_DEPTH = 0.00090` is largest for the farthest-back element,
consistent with "smaller = front" convention. Under this reading, the backdrop at 0.00059
is behind modal sprite instances at ~0.00026. **Verdict revised to PASS** for preview
backdrop depth given the convention check.

---

### Stage 6 — Title (0x694), Select Engagement, Game Type heading, Game Map heading

| Dimension | gamemd | Our code | Verdict |
|---|---|---|---|
| Title static `0x694` | Resource `(425,1,108,10)` — small right-column title | `right_anchor(screen_w, screen_h, dlu_rect(425,1,108,10)).translate(0,1)` at layout.rs:577 | **PASS** |
| Title text key | `GUI:ChooseMap` | `"GUI:ChooseMap"` at text.rs:776 | **PASS** |
| Select Engagement static (-1) | Resource `(80,20,257,12)` | `dlu_rect(80, 20, 257, 12)` at layout.rs:578 (struct field `select_engagement`) — note: resource is `(80,20,257,12)` but `dlu_rect` uses first arg as x=80; resource table shows `(-1)` control at `(80,20,257,12)` | **PASS** |
| Select Engagement text key | `GUI:SelectEngagement` | `"GUI:SelectEngagement"` at text.rs:782 | **PASS** |
| Game Type heading (-1) | Resource `(77,60,130,10)` | `dlu_rect(77, 60, 130, 10)` at layout.rs:579 | **PASS** |
| Game Type text key | `GUI:GameType` | `"GUI:GameType"` at text.rs:787 | **PASS** |
| Game Map heading (-1) | Resource `(225,60,130,10)` | `dlu_rect(225, 60, 130, 10)` at layout.rs:580 | **PASS** |
| Game Map text key | `GUI:GameMap` | `"GUI:GameMap"` at text.rs:793 | **PASS** |
| Text depth for statics | Native: drawn by `OwnerDraw_Static_006153E0` at normal shell control depth | All static text at `SHELL_DROPDOWN_TEXT_DEPTH - 0.00008` at text.rs:808 | **UNCHECKED** (exact depth relative to background/buttons not compared) |

---

### Stage 7 — Status strip (0x695)

| Dimension | gamemd | Our code | Verdict |
|---|---|---|---|
| Status rect | Resource `(2,355,303,12)` | `choose_map_status_help_rect(screen_w, screen_h)` at layout.rs:581; function not shown but `status_help` field present in struct | **UNCHECKED** (exact pixel value of `choose_map_status_help_rect` not verified against `(2,355,303,12)`) |
| Status text | `GUI:Blank` by default; dynamic help text when controls are hovered | `choose_map_modal_status_help_text` at text.rs:761-763 returns non-empty `status_help_text`; drawn at text.rs:837-845 | **PASS** (conditional on populated status text) |

---

### Stage 8 — Draw order / depth layering

| Order | Layer | gamemd | Our code | Verdict |
|---|---|---|---|---|
| 1 (back) | Background art `MnScrnLCustomizeBattle.shp` | Fullscreen shell move + WM_PAINT | `SHELL_PARENT_BACKGROUND_DEPTH = 0.00090` | **PASS** |
| 2 | Dialog solid backdrop | Below controls | `SHELL_DROPDOWN_DEPTH - 0.00008 ≈ 0.00026` | **PASS** |
| 3 | Dialog outline | Above backdrop | `SHELL_DROPDOWN_DEPTH - 0.00009 ≈ 0.00025` | **PASS** |
| 4 | Listboxes | Above backdrop, below buttons | `SHELL_DROPDOWN_DEPTH - 0.00010 ≈ 0.00024` | **PASS** |
| 5 | Buttons | Above backdrop | `SHELL_DROPDOWN_DEPTH - 0.00011 ≈ 0.00023` | **PASS** |
| 6 | Preview outline | Above listboxes | `SHELL_DROPDOWN_DEPTH - 0.00012 ≈ 0.00022` | **PASS** |
| 7 (front) | Text | Front of modal | `SHELL_DROPDOWN_TEXT_DEPTH - 0.00008/9/10` | **PASS** |
| Special | Preview texture | Drawn in separate render pass, always after sprite instances | Always after main sprite pass (`render_skirmish_shell` lines 648-658) | **PASS** |
| Special | Start marker overlays | Should NOT appear when modal is open (setup view only) | Drawn unconditionally in `render_skirmish_shell` if `draw_start_marker_overlays = true` — modal active does not gate this | **FAIL** (start markers can bleed over modal if a random-map sentinel entry was previously selected) |
| Special | Setup shell chrome | Must be fully hidden | Early return at `build_skirmish_shell_instances:201-204` blocks setup sprites; text suppression at lines 558-568 | **PASS** |

---

### Stage 9 — `choose_map_modal_semantic_draw_order` completeness

`draw_order.rs:120-133` emits:
1. `ChooseMapBackgroundCustomizeBattle800` or `ChooseMapModalBackdrop`
2. `ChooseMapListbox` × 2
3. `ChooseMapOwnerDrawButton` × 3
4. `ChooseMapPreviewStatic`

Missing from the semantic draw order vs. the full composition:
- No role for the preview texture or preview backdrop (those are in a separate pass — acceptable, the semantic order covers atlas-texture sprites only).
- No role for start markers — these should be explicitly excluded when modal is active but are not gated.
- No role for `ValidationModal` when modal is open (irrelevant, mutually exclusive state).

The semantic draw order is used by tests only, not by the actual render path, so missing roles here are a test coverage gap rather than a runtime failure.

---

## 2. Adjacent Findings (not traced)

- Button text inset for pressed state: `button_text_rect` at text.rs:172-186 applies `x+=2, y+=5` for pressed — native presses apply `y+1` per owner-draw report. May be a button-text alignment disparity. Not traced here.
- `choose_map_status_help_rect` pixel value not read in this session.
- Listbox draw-order tie at identical depth values (both at `SHELL_DROPDOWN_DEPTH - 0.00010`).

---

## 3. Stage Summary

| Stage | Element | Verdict |
|---|---|---|
| 0 | Parent setup shell hidden | PASS (sprites) / FAIL (start markers can bleed) |
| 1 | MnScrnLCustomizeBattle.shp background at 800 | PASS |
| 1b | Non-800 background fallback | UNCHECKED |
| 2 | Dialog frame / backdrop | FAIL (single-color outline, not bevel) |
| 3a | Mode list (0x6EB) rect + bevel | PASS |
| 3b | Map list (0x553) rect + bevel | PASS |
| 3c | Listbox background color | UNCHECKED (marked pending in code) |
| 3d | Selected row fill color | UNCHECKED |
| 3e | Scrollbar width | PASS |
| 3f | Scrollbar track color | UNCHECKED (marked pending in code) |
| 4a | Use Map button rect | PASS |
| 4b | Create Random Map button rect | PASS |
| 4c | Cancel button rect | PASS |
| 4d | Button art (SDBTNANM) | PASS |
| 5a | Preview pane rect (0x468) | PASS |
| 5b | Preview pane outline | FAIL (single-color, not bevel) |
| 5c | Preview image source | PASS |
| 5d | Preview backdrop depth | PASS (after depth convention verification) |
| 6a | Title (0x694) rect + text | PASS |
| 6b | Select Engagement rect + text | PASS |
| 6c | Game Type heading rect + text | PASS |
| 6d | Game Map heading rect + text | PASS |
| 7 | Status strip (0x695) text | PASS (conditional) |
| 7b | Status strip rect | UNCHECKED |
| 8 | Draw order / depth layering | PASS (overall); FAIL (start markers bleed when modal active) |
