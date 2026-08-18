# Slice 5a-ii — In-Game Options (0xBBB) Owner-Draw Paint + Assets — Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained. This is the
> PAINT slice — it renders the native in-game Options dialog (`0xBBB`) over the
> frozen battlefield and retires the egui pause card's draw. Control *interaction*
> (slider drag, checkbox toggle, Back→close+persist, live CSF labels, INI write)
> is **5a-iii**, NOT here.

**Goal:** Emit the active in-game Options dialog (`0xBBB`) pixel-faithfully as a
native owner-draw overlay over the frozen battlefield — SIDEBTTN type-2 buttons,
the existing trackbar/checkbox glyph seam, text statics, the verified
child-resize anchoring — and replace the egui `draw_pause_menu` card's draw.

**Architecture:** Builds on the 5a-i descriptor (`build_in_game_options_descriptor`,
`BgKind`/`RepositionPolicy::InGameOptions`). Adds a `ControlPaint::Button` arm to
the established app-layer paint seam (`app_skirmish_shell_render/controls.rs`),
loads `SIDEBTTN.SHP`/`SIDEBAR.PAL` into the shell-chrome atlas, replaces the
`InGameOptions` baseline in `layout_pass` with the native anchoring, and adds a
new `app_skirmish_shell_render/in_game_options.rs` emitter wired into the in-game
frame. Render-only; no `sim/` touch; the paused freeze is unchanged (the
`service_tick`/session-mode swap is 5b).

**Design Doc:** `docs/plans/2026-06-12-slice5a-ingame-options-dialog-design.md` (§3 D3, §4 5a-ii, §9)

---

## Grounding Summary

- **Docs (verified):** `OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS_GHIDRA_REPORT.md`
  (button type routing, anchoring helpers, overlay-not-panel),
  `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md` (controls, populate path).
  Session grounding lanes saved locally: `docs/plans/_5aii-grounding-lane{A,B,C,D}.md`.
- **Ghidra (re-verified this session):** Active `0xBBB` buttons (Back `0x686`,
  Keyboard `0x52C`, Sound `0x52D`) are owner-draw **type 2** painted by
  `OwnerDraw_Button_00612B70` from **SIDEBTTN.SHP** via **SIDEBAR.PAL**; frame
  **0 = released, 1 = pressed, 2 = flash/checked** (no separate hover frame — the
  flash frame is timer-driven, deferred). WM_DRAWITEM is intercepted by the shared
  framework (`FUN_00622b50` → `FUN_006213a0`, records state + invalidates); the
  per-button subclass paints on WM_PAINT. Anchoring: `ResizeShellChildControl_0060C0C0`
  routes `0x52C`/`0x52D`→`FUN_0060b000`, Back→`FUN_0060b350` (both right-edge,
  active branch = SIDEBTTN dims + **147px** inset), title→`B1D0`, ordinary
  controls→`FUN_0060b7a0` (centered offset), finalizer `B950`. Base design size
  **800×600** (`DAT_007f5be4`/`DAT_007f5bf0`). Overlay-only — no full-screen image
  control is created. **VisualDetails `0x52B` + statics `0x716`/`0x673` are
  `WS_DISABLED` and NOT `WS_VISIBLE` and the proc never shows them → hidden in
  `0xBBB`.**
- **Assets (read from retail this session):** `SIDEBTTN.SHP` = **125×25, 3 frames**,
  in `ra2.mix→sidec01.mix`, palette `SIDEBAR.PAL` (768B; idx0 magenta key).
  `SDBTNANM.SHP` (156×42, 17 frames, its own `SDBTNANM.PAL`) is already loaded.
  `SIDEBTTN.SHP`/`SIDEBAR.PAL` appear today only in comments — never loaded.
- **Repo pattern:** the `ControlPaint`/`paint_control` seam in
  `app_skirmish_shell_render/controls.rs:242-383`; the chrome atlas in
  `render/skirmish_shell_chrome.rs` (SDBTNANM load loop `:219-237` is the template
  for the SIDEBTTN load); the draw-list test idiom in `controls.rs` tests `:648-1191`.
- **INI:** none consumed in 5a-ii (CSF caption text comes from the string table;
  INI persistence is 5a-iii).
- **Still unknown after grounding (→ flagged):** the exact runtime value the buttons
  anchor their *Y* to — gamemd reads sidebar-layout globals (`DAT_00b0fc48` top
  stack anchor, `DAT_00b0fc4c` bottom anchor) at 25px row pitch. VERA must bind the
  button Y to its own in-game **sidebar geometry**, not port the raw global. See
  Key Decision KD-4 + Task 4.

## Key Technical Decisions

- **KD-1 — Active `0xBBB` buttons use SIDEBTTN type 2 (frames 0/1/2), NOT SDBTNANM/
  MNBTTN/PCX.** — **Confidence:** high — **Source:** chrome doc +
  `decompile 0x00612B70` (mode-2 frame table: down=1, checked=2, else=0).
- **KD-2 — Button right-edge X = `parentWidth − 147`, sized to the SIDEBTTN SHP
  header width (125), height 25.** The `147` (`0x93`) is a literal pixel inset, not
  a struct field; the canvas size is read from the SHP header (+2 width / +4
  height). — **Confidence:** high — **Source:** `decompile 0x0060B000`/`0x0060B350`
  (active branch reads `DAT_00b0f9ec`+2/+4 and subtracts `0x93`), Lane D dims.
- **KD-3 — Ordinary controls (trackbars/checkboxes/value-labels) use the centered
  offset `((screenW−800)/2, (screenH−600)/2)` clamped ≥0, active-only.** At 800×600
  this is a no-op (== 5a-i baseline); at 1024×768 it is +112/+84. — **Confidence:**
  high — **Source:** `decompile 0x0060B7A0` + base size `DAT_007f5be4`=800/`DAT_007f5bf0`=600.
- **KD-4 — Button Y is a 25px row stack anchored to the in-game SIDEBAR geometry,
  not the dialog.** gamemd reads sidebar anchor globals (`DAT_00b0fc48`/`DAT_00b0fc4c`);
  the faithful Rust port binds button Y to VERA's existing sidebar layout. —
  **Confidence:** medium (the VERA sidebar-anchor binding is unverified against the
  gamemd globals at runtime) — **flag for /review-plan + manual visual gate (Task 9).**
- **KD-5 — VisualDetails trackbar `0x52B` + caption `0x716` + value-label `0x673`
  are hidden in `0xBBB`; the emitter skips them.** Modeled with a new
  `ControlDescriptor.visible` template flag (faithful to the WS_VISIBLE bit),
  defaulted `true`, set `false` for `0x52B`. — **Confidence:** high — **Source:**
  `0xBBB` template flags (`0x52B` style `0x48…` = WS_DISABLED, no WS_VISIBLE) +
  `decompile 0x004E1FE0` (populate path never `ShowWindow`s `0x52B`; it only
  conditionally hides GameSpeed).
- **KD-6 — 5a-ii is render-only; ESC still toggles `state.paused` to close.** The
  native overlay replaces the egui card's *draw*; control *clicks* (Back→close,
  slider drag, checkbox toggle) are 5a-iii. A temporary non-chrome quit-to-menu
  keybind is preserved (design §8 Q1, user decision 2026-06-12). — **Confidence:**
  high — **Source:** design §4/§8, Lane C (`app.rs:2914-2922`, `app_input.rs:425-447`).

## Open Questions

### Resolved During Planning
- *Which SHP/frames/palette for active buttons?* SIDEBTTN.SHP, frames 0/1/2, SIDEBAR.PAL (KD-1).
- *Is `0x93` a struct offset?* No — literal 147px inset; canvas size from SHP header (KD-2).
- *Center or stretch at 1024?* Centered offset, base 800×600, active-only (KD-3).
- *Is there panel-background art?* No — overlay-only; statics are text (chrome doc, Lane B item 4).
- *Does VisualDetails show in-game?* No — hidden in `0xBBB` (KD-5).

### Deferred to Implementation / Flagged
- **Button Y sidebar-anchor binding (KD-4).** Bind to VERA's sidebar layout; confirm
  the resulting Y matches gamemd at the manual gate. If VERA's sidebar anchor is not
  readily available to the emitter, the fallback (flagged) is the DLU-derived Y with
  a follow-up — do NOT silently ship DLU-Y as "done."
- **CSF caption strings** for the statics/buttons (`GUI:GameOptions`, `GUI:GameSpeed`,
  `GUI:ScrollRate`, `GUI:Back`, `GUI:Keyboard`, `GUI:Sound`, value labels) — resolve
  via the existing CSF/string-table path at emit time (5a-iii owns the *dynamic*
  label swap on drag; 5a-ii renders the initial caption).

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/render/skirmish_shell_chrome.rs` | Load SIDEBTTN.SHP frames 0/1/2 (SIDEBAR.PAL) into the atlas + `ControlChrome` (3 fields + `control_chrome()` copy) |
| Modify | `src/app_skirmish_shell_render/controls.rs` | Add `ControlPaint::Button` variant + arm + pure frame-select helper + draw-list tests |
| Modify | `src/ui/shell/descriptor.rs` | Add `ControlDescriptor.visible` (template WS_VISIBLE, default true) |
| Modify | `src/ui/shell/in_game_options.rs` | Set `visible:false` for VisualDetails; thread `visible` through `options_control` |
| Modify | `src/ui/shell/layout.rs` | Replace the `RepositionPolicy::InGameOptions` baseline arm with the native anchoring (centered ordinary + right-edge buttons); tests at 800×600 + 1024×768 |
| Modify | `src/ui/shell/modal.rs` | Add `visible:true` to its `modal_control` constructor (field add fallout) |
| Modify | `src/ui/main_menu_shell/layout.rs` | Add `visible:true` to its 3 `ControlDescriptor` constructions (`:147`, `:188`, `:199`) (field add fallout) |
| Create | `src/app_skirmish_shell_render/in_game_options.rs` | The Options emitter: descriptor→layout→`ControlPaint` instances + text statics over the frozen frame |
| Modify | `src/app_skirmish_shell_render.rs` | `mod in_game_options;` + a `render_in_game_options_overlay` entry mirroring `render_skirmish_shell` |
| Modify | `src/app.rs` | In the `GameScreen::InGame` paused branch, draw the native overlay instead of the egui card; preserve temp quit-to-menu |

## Interface Changes

- **`ControlPaint` gains `Button { rect, frame, .. }`** (`pub(super)`) — consumed only
  by `paint_control` + the new emitter.
- **`ControlChrome` gains `options_button_sidebttn_frame0/1/2: Option<SkirmishShellChromeEntry>`**
  — populated in `control_chrome()`; read by the `Button` arm.
- **`ControlDescriptor` gains `visible: bool`** — every constructor must set it
  (default `true`). The full set is **6 construction sites in 4 files** (verified by
  grep `ControlDescriptor {`): `in_game_options.rs:145` (per-control), `modal.rs:230`
  (`modal_control`), `shell/layout.rs:97` (test `ctrl` helper), and
  `main_menu_shell/layout.rs:147,188,199` (3 sites). All structs are constructed
  without `..`, so each needs the new field. No exhaustive *destructuring* of
  `ControlDescriptor` exists, so the field add is build-safe once all 6 are updated.
- **`layout_pass`'s `RepositionPolicy::InGameOptions` arm changes output** — was raw
  DLU→px (5a-i baseline), becomes the native anchoring. The 5a-i baseline test
  (`in_game_options_baseline_is_raw_dlu_to_pixel`) is REPLACED by 800×600 +
  1024×768 anchoring tests (it asserted the now-superseded baseline).

## Sim Checklist
N/A — no `sim/` files touched. Render/UI only; the paused world-freeze is unchanged.

## Risk Areas

- **`ControlDescriptor.visible` field add** — blast radius = every `ControlDescriptor`
  construction site. Mitigation: grep + Task 3 build catches misses; default `true`
  preserves all existing behavior.
- **`layout_pass` InGameOptions arm change** — the only consumer is the new 0xBBB
  emitter (the skirmish renderer uses its own `SkirmishShellLayout`, not `layout_pass`),
  so no other shell regresses; the 5a-i baseline test is intentionally replaced.
- **Button Y sidebar binding (KD-4)** — highest parity risk; manual visual gate
  (Task 9) is mandatory before commit.
- **egui card retirement** — must not break the existing ESC pause/resume or the
  temp quit-to-menu; Task 8 keeps ESC-toggle + the keybind.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| 1 | SIDEBTTN.SHP frames 0/1/2 via SIDEBAR.PAL, idx0→alpha | Wrong art/palette = visibly wrong buttons every time Options opens | Lane D dims (125×25) + `frame_to_rgba_ui` magenta key; draw-list test |
| 2 | Frame select 0=released / 1=pressed (2=flash deferred) | Pressed feedback parity | `decompile 0x00612B70` mode-2 table; pure helper test |
| 4 | Button X = `parentW−147`, size 125×25; ordinary centered `((W−800)/2,(H−600)/2)` clamped | 1-px = drift; buttons are NOT at their DLU rect | `decompile 0x0060B000/B7A0`; tests at 800×600 + 1024×768 |
| 4 | Button Y = 25px row off the sidebar anchor (KD-4) | Vertical button placement parity | `decompile 0x0060B000/B350`; **flagged** — manual gate |
| 5 | VisualDetails triplet (`0x52B`/`0x716`/`0x673`) NOT emitted | gamemd hides them in `0xBBB`; emitting = extra controls the player never sees in-game | template WS_VISIBLE + `decompile 0x004E1FE0`; emitter test |
| 5 | Overlay draws over the frozen battlefield (LOAD, not clear) | No opaque panel; chrome composites over the game | chrome doc / Lane B item 4; render-pass LoadOp::Load |

---

## Tasks

### Task 1: Load SIDEBTTN.SHP + SIDEBAR.PAL into the shell-chrome atlas

**Why:** The active `0xBBB` buttons need their art; SIDEBTTN is not loaded today.

**Files:** Modify `src/render/skirmish_shell_chrome.rs`

**Pattern:** The SDBTNANM load loop (`skirmish_shell_chrome.rs:219-237`) + the
`load_named_palette` / `render_shp_entry_labeled` helpers (`:184`, `:585-625`).

**Step 1:** In `build_skirmish_shell_chrome_atlas`, after the SDBTNANM block, load the
palette and bake frames 0..=2:
```rust
// SIDEBTTN.SHP — active in-game Options (0xBBB) owner-draw buttons (type 2),
// 125x25 canvas, 3 frames, drawn through SIDEBAR.PAL (idx0 = magenta key).
let sidebar_palette = load_named_palette(assets, "SIDEBAR.PAL");
if sidebar_palette.is_none() {
    log::warn!("SIDEBAR.PAL missing; in-game Options buttons will not render");
}
for frame in [0usize, 1, 2] {
    push_optional(
        &mut rendered,
        render_shp_entry_labeled(
            assets, "SIDEBTTN.SHP", &format!("sidebttn.shp#{frame}"),
            &sidebar_palette, frame,
        ),
    );
}
```
(Use the exact `push`/`push_optional` + `&Option<Palette>` signature the SDBTNANM/
MNBTTN blocks use; match `render_shp_entry_labeled`'s real parameter order before
writing — confirm at `:585-625`.)

**Step 2:** Add three fields to `struct SkirmishShellChromeAtlas` (`:33-88`):
```rust
    options_button_sidebttn_frame0: Option<SkirmishShellChromeEntry>,
    options_button_sidebttn_frame1: Option<SkirmishShellChromeEntry>,
    options_button_sidebttn_frame2: Option<SkirmishShellChromeEntry>,
```
and populate from `by_label` in the struct build (`:411-468`):
```rust
        options_button_sidebttn_frame0: by_label.get("sidebttn.shp#0").copied(),
        options_button_sidebttn_frame1: by_label.get("sidebttn.shp#1").copied(),
        options_button_sidebttn_frame2: by_label.get("sidebttn.shp#2").copied(),
```

**Step 3:** Add the same three fields to `struct ControlChrome` (`:96-121`) and copy
them in `control_chrome()` (`:123-153`):
```rust
            options_button_sidebttn_frame0: self.options_button_sidebttn_frame0,
            options_button_sidebttn_frame1: self.options_button_sidebttn_frame1,
            options_button_sidebttn_frame2: self.options_button_sidebttn_frame2,
```

**Step 4: Test** (in the `skirmish_shell_chrome.rs` tests, mirroring the existing
asset-presence checks): assert the atlas exposes a non-`None` frame0 with
`pixel_size == [125.0, 25.0]` when assets are present (guard behind the same
asset-availability gate the existing SDBTNANM/PCX tests use; if those tests skip
without retail assets, follow that idiom — do NOT hard-fail in CI-without-assets).

**Step 5: Verify:** `cargo test -p vera20k skirmish_shell_chrome` → PASS (or the
asset-gated skip path). **Step 6: Commit.**

### Task 2: Add the `ControlPaint::Button` variant + pure frame-select helper

**Why:** The seam needs a button arm and a unit-testable state→frame mapping.

**Files:** Modify `src/app_skirmish_shell_render/controls.rs`

**Pattern:** `right_panel_button_sdbtnanm_frame_index` (`chrome.rs:342-347`) /
`shell_paint.rs::modal_button_frame_index` (`shell_paint.rs:373-381`).

**Step 1:** Extend `enum ControlPaint` (`controls.rs:242-256`):
```rust
    /// Active in-game Options (0xBBB) owner-draw button — SIDEBTTN type 2, drawn
    /// at the SHP native 125x25 size, right-edge anchored by the layout pass.
    Button { rect: RectPx, frame: u8 },
```
(`rect` is the already-anchored pixel rect from `layout_pass`; `frame` is the
resolved SIDEBTTN frame index.)

**Step 2:** Add the pure selector (next to the existing frame helpers in this file):
```rust
/// SIDEBTTN type-2 button frame: released 0, pressed 1. Frame 2 is the
/// timer-driven flash/checked state, deferred until the button-flash timer is
/// modeled (5a-ii renders released/pressed only).
pub(super) fn options_button_sidebttn_frame_index(pressed: bool) -> u8 {
    if pressed { 1 } else { 0 }
}
```

**Step 3: Test:**
```rust
    #[test]
    fn options_button_frame_index_released_pressed() {
        assert_eq!(options_button_sidebttn_frame_index(false), 0);
        assert_eq!(options_button_sidebttn_frame_index(true), 1);
    }
```

**Step 4: Verify:** `cargo test -p vera20k options_button_frame_index` → PASS.
**Step 5: Commit.**

### Task 3: Add the `Button` arm to `paint_control`

**Why:** Emit the SIDEBTTN glyph at the button's resolved rect, native size.

**Files:** Modify `src/app_skirmish_shell_render/controls.rs`

**Pattern:** The Checkbox arm (`controls.rs:269-278`) + `push_entry_native`
(`chrome.rs:173-181`) for native-size placement; the modal centered-button shape
(`shell_paint.rs:480-487`) for centering within the rect if needed.

**Step 1:** Add the match arm in `paint_control` (`controls.rs:268+`):
```rust
        ControlPaint::Button { rect, frame } => {
            let entry = match frame {
                1 => chrome.options_button_sidebttn_frame1,
                2 => chrome.options_button_sidebttn_frame2,
                _ => chrome.options_button_sidebttn_frame0,
            };
            if let Some(entry) = entry {
                // SIDEBTTN draws at its native 125x25 canvas; `rect` is the
                // anchored top-left from layout_pass. Native-size push (no scale).
                // `entry` is `Copy` and `push_entry_native` takes it BY VALUE
                // (match the existing arms, e.g. the Trackbar rail push) — no `&`.
                push_entry_native(out, entry, rect.x, rect.y, SHELL_CONTROL_DEPTH);
            }
        }
```
(Confirm `push_entry_native`'s exact signature/coords at `chrome.rs:173-181`; if it
takes the entry's `pixel_size` implicitly, the 125×25 comes from the atlas entry.)

**Step 2: Draw-list test** (mirror `checkbox_paint_seam_emits_icon_at_icon_rect_with_control_depth`
at `controls.rs:717-762`):
```rust
    #[test]
    fn options_button_paint_seam_emits_sidebttn_frame_at_rect() {
        let entry = test_entry([0.10, 0.20], [0.30, 0.40], [125.0, 25.0]);
        let chrome = ControlChrome { options_button_sidebttn_frame0: Some(entry), ..Default::default() };
        let mut out = Vec::new();
        paint_control(&mut out, &chrome, ControlPaint::Button { rect: RectPx::new(653, 198, 125, 25), frame: 0 });
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].position, [653.0, 198.0]);
        assert_eq!(out[0].size, [125.0, 25.0]);
        assert_eq!(out[0].uv_origin, [0.10, 0.20]);
        // empty chrome -> empty output
        let mut empty = Vec::new();
        paint_control(&mut empty, &ControlChrome::default(), ControlPaint::Button { rect: RectPx::new(0,0,1,1), frame: 0 });
        assert!(empty.is_empty());
    }
```
(Use the same `test_entry`/`SpriteInstance` field idiom the existing tests use;
confirm helper names at `controls.rs:648-1191`.)

**Step 3: Verify:** `cargo test -p vera20k options_button_paint_seam` → PASS.
**Step 4: Commit.** *(End of Part A — assets + paint primitives are independently
shippable here.)*

### Task 4: Native `InGameOptions` anchoring in `layout_pass`

**Why:** The buttons render at SIDEBTTN's 125×25 right-anchored (NOT their DLU rect);
ordinary controls take the centered offset. This replaces the 5a-i raw-DLU baseline.

**Files:** Modify `src/ui/shell/layout.rs`

**Pattern:** The existing `apply_anchor` + `geom` helpers; the verified gamemd
formulas (KD-2/KD-3/KD-4).

**Step 1:** Replace the `RepositionPolicy::InGameOptions` arm (currently raw DLU→px)
with a call to a new `in_game_options_anchor(c, screen_w, screen_h, button_w, button_h, sidebar)`:
```rust
                RepositionPolicy::InGameOptions => in_game_options_anchor(
                    c, screen_w, screen_h, button_canvas, sidebar_anchor,
                ),
```
where:
- ordinary controls (Trackbar/Checkbox/Static value-labels): start from
  `geom::dlu_rect(...)`, then add the centered offset
  `dx = clamp0((screen_w - 800) / 2)`, `dy = clamp0((screen_h - 600) / 2)`
  (sign-corrected toward 0 exactly as `FUN_0060b7a0`; at 800×600 → 0/0).
- buttons (Back/Keyboard/Sound, `ControlKind::Button`): rect =
  `RectPx::new(screen_w - 147, button_y, button_canvas.w, button_canvas.h)` where
  `button_canvas = (125, 25)` from the SIDEBTTN SHP header, and `button_y` is the
  25px-row stack off the sidebar anchor (KD-4 — see Step 2).

**Step 2 (KD-4 binding — FLAGGED):** Thread the in-game **sidebar anchor Y** into
the layout call. The faithful source is VERA's sidebar layout geometry (the same
anchor gamemd reads from `DAT_00b0fc48`/`DAT_00b0fc4c`). Concretely:
- Back `0x686`: `y = sidebar_bottom_anchor - 25`.
- Sound `0x52D`: `y = sidebar_button_stack_anchor + 0*25`.
- Keyboard `0x52C`: `y = sidebar_button_stack_anchor + 1*25`.
Implement the binding to VERA's sidebar module; if that geometry is not reachable
from `ui/shell/layout.rs` without a layering violation, pass the two anchor Y
values in as parameters (the app layer supplies them from the sidebar renderer) —
do NOT hardcode pixel constants. **Flag for /review-plan:** confirm VERA's sidebar
anchor Y reproduces gamemd's button column Y at 800×600 and 1024×768.

**Step 3:** Make `button_canvas` come from the SHP header at load (passed in), not a
literal — but assert it equals `(125, 25)` in a test for regression.

**Step 4: Replace the 5a-i baseline test** `in_game_options_baseline_is_raw_dlu_to_pixel`
with two anchoring tests:
```rust
    #[test]
    fn in_game_options_ordinary_controls_centered_offset() {
        // Trackbar 0x529 (144,100,128,13): at 800x600 == raw DLU (no shift);
        // at 1024x768 shifted by +112/+84.
        let desc = options_test_descriptor(); // helper building the 0xBBB controls
        let at800 = layout_pass(&desc, 800, 600);
        assert_eq!(rect_for(&at800, 0x0529), geom::dlu_rect(144, 100, 128, 13)); // (216,163,192,21)
        let at1024 = layout_pass(&desc, 1024, 768);
        let base = geom::dlu_rect(144, 100, 128, 13);
        assert_eq!(rect_for(&at1024, 0x0529), RectPx::new(base.x + 112, base.y + 84, base.w, base.h));
    }

    #[test]
    fn in_game_options_buttons_right_edge_sidebttn_size() {
        // Buttons render at SIDEBTTN 125x25, x = screen_w - 147 (NOT the DLU rect).
        let desc = options_test_descriptor();
        let at800 = layout_pass(&desc, 800, 600);
        let back = rect_for(&at800, 0x0686);
        assert_eq!((back.x, back.w, back.h), (800 - 147, 125, 25));
        let at1024 = layout_pass(&desc, 1024, 768);
        assert_eq!(rect_for(&at1024, 0x0686).x, 1024 - 147);
    }
```
(`options_test_descriptor`/`rect_for` are local test helpers; thread fixed sidebar
anchor Y values into the test so button Y is deterministic.)

**Step 5: Verify:** `cargo test -p vera20k ui::shell::layout` → PASS.
**Step 6: Commit.**

### Task 5: Add `ControlDescriptor.visible`; hide the VisualDetails triplet

**Why:** VisualDetails `0x52B` (+ statics `0x716`/`0x673`) is not visible in `0xBBB`;
the descriptor must carry the template WS_VISIBLE bit so the emitter skips it (KD-5).

**Files:** Modify `src/ui/shell/descriptor.rs`, `src/ui/shell/in_game_options.rs`,
`src/ui/shell/modal.rs`, `src/ui/shell/layout.rs` (test helper)

**Step 1:** Add to `struct ControlDescriptor` (`descriptor.rs:81-99`), after `enabled`:
```rust
    /// Resource-template `WS_VISIBLE`. `false` controls are created hidden and the
    /// proc never shows them, so they are not emitted/hit-tested. (Active 0xBBB
    /// hides the VisualDetails trackbar + its caption/label.)
    pub visible: bool,
```

**Step 2:** Set it in every constructor (grep `ControlDescriptor {`):
- `in_game_options.rs::options_control` — add a `visible: bool` param; pass `true`
  for all controls EXCEPT VisualDetails (`false`). Update the 9 call sites:
  VisualDetails gets `visible:false`, the rest `visible:true`.
- `modal.rs::modal_control` (`:230`) — `visible: true`.
- `shell/layout.rs` test `ctrl` helper (`:97`) — `visible: true`.
- `main_menu_shell/layout.rs` — `visible: true` at all 3 constructions (`:147`, `:188`, `:199`).
- (grep `ControlDescriptor {` once more to confirm no site was missed.)

**Step 3: Test** (in `in_game_options.rs` tests):
```rust
    #[test]
    fn visualdetails_is_hidden_rest_visible() {
        let d = build_in_game_options_descriptor();
        let vis = |id: u16| d.controls.iter().find(|c| c.id == id).unwrap().visible;
        assert!(!vis(control::VISUAL_DETAILS), "VisualDetails hidden in 0xBBB");
        for id in [control::BACK, control::KEYBOARD, control::SOUND, control::GAME_SPEED,
                   control::SCROLL_RATE, control::TARGET_LINES, control::SHOW_HIDDEN, control::TOOLTIPS] {
            assert!(vis(id), "control {id:#06x} visible");
        }
    }
```

**Step 4: Verify:** `cargo test -p vera20k ui::shell` → PASS (all shell suites).
**Step 5: Commit.**

### Task 6: Create the Options emitter `app_skirmish_shell_render/in_game_options.rs`

**Why:** Turn the descriptor + anchored layout into the sprite-instance draw list.

**Files:** Create `src/app_skirmish_shell_render/in_game_options.rs`; modify
`src/app_skirmish_shell_render.rs` (`mod in_game_options;`).

**Pattern:** `build_skirmish_shell_instances` (`app_skirmish_shell_render.rs:187-381`)
+ the per-family `push_*` wrappers; reuse `controls::paint_control`.

**Step 1:** Write `build_in_game_options_instances(chrome, font, screen_w, screen_h,
sidebar_anchor, state) -> Vec<SpriteInstance>`:
- `let desc = build_in_game_options_descriptor();`
- `let laid = layout_pass(&desc, screen_w, screen_h);` (now natively anchored).
- For each `(control, laid_rect)`: skip if `!control.visible`. Match `control.kind`:
  - `Button` → `paint_control(.., ControlPaint::Button { rect: laid_rect, frame: options_button_sidebttn_frame_index(state.pressed(control.id)) })` (in 5a-ii `pressed` is always false — no input yet — so frame 0).
  - `Trackbar` → compute `thumb_px` from the control's value (GameSpeed/ScrollRate `6-pos` inverted, range 0..6; VisualDetails is hidden so not reached) using the same quantization as `push_trackbar_instances` (`controls.rs:425-429`), then `ControlPaint::Trackbar`. (Static initial value source for 5a-ii: the populate defaults — GameSpeed 3, ScrollRate 3 — until 5a-iii wires real values.)
  - `Checkbox` → `ControlPaint::Checkbox { checked, rect }` (initial checked from the populate defaults: TargetLines 1, ShowHidden 0, Tooltips 1).
- Emit the **text statics** (title `0x694`, captions `0x714`/`0x715`, value labels
  `0x671`/`0x672`, footer `0x695`) via the existing BitFont static-text path used by
  the skirmish shell (mirror the text emit in `build_skirmish_shell_instances`).
  Skip the hidden statics `0x716`/`0x673`. CSF lookups resolve the caption keys
  (`GUI:GameOptions`, `GUI:GameSpeed`, `GUI:ScrollRate`, value labels, `GUI:Blank`).

**Step 2: Draw-list test** — build a sentinel `ControlChrome` + font stub, call the
emitter at 800×600, assert: (a) the VisualDetails triplet is absent, (b) three
`Button` instances at `x == 800-147`, (c) two visible trackbars + three checkboxes
present, (d) empty-chrome → no button instances. Name it
`ingame_options_emitter_emits_visible_controls_skips_visualdetails`.

**Step 3: Verify:** `cargo test -p vera20k in_game_options` → PASS.
**Step 4: Commit.**

### Task 7: Wire the overlay render into the in-game frame

**Why:** Draw the emitter's instances over the frozen battlefield.

**Files:** Modify `src/app_skirmish_shell_render.rs` (+ a thin entry called from `app.rs`).

**Pattern:** `render_skirmish_shell` / `render_skirmish_shell_with_atlas`
(`app_skirmish_shell_render.rs:407-422`, `:461-721`).

**Step 1:** Add `render_in_game_options_overlay(state, encoder, view)`:
- Build instances via `build_in_game_options_instances(...)` using the live
  `ControlChrome` (`atlas.control_chrome()`), font, screen size, and the sidebar
  anchor.
- Open ONE render pass on `view` with **`LoadOp::Load`** (compose over the
  battlefield — do NOT clear), draw the instance buffer via
  `draw_with_buffer_passthrough` (no Z-test), then the scissored text buffers, then
  the cursor — mirroring `render_skirmish_shell_with_atlas:633-717`.

**Step 2: Verify:** `cargo build -p vera20k` → compiles. **Step 3: Commit.**

### Task 8: Replace the egui pause-card draw with the native overlay

**Why:** Retire the egui Options placeholder for the parity route (design §4).

**Files:** Modify `src/app.rs`

**Pattern:** The `GameScreen::InGame` paused block (`app.rs:2914-2922`) + the ESC
handler (`app_input.rs:425-447`).

**Step 1:** In the `GameScreen::InGame` branch, where `if state.paused` currently
calls `draw_pause_menu` / the egui card (`app.rs:2914-2922`), instead call
`render_in_game_options_overlay(state, encoder, view)` BEFORE
`state.egui.end_frame_and_render` and DO NOT draw the egui card.

**Step 2:** Keep ESC → toggle `state.paused` (`app_input.rs:425-447`) unchanged —
in 5a-ii ESC opens AND closes the native overlay (no Back-click routing yet; that's
5a-iii). Keep `handle_pause_menu` (`app.rs:3001-3057`) compiled but no longer
invoked for the draw; its `ReturnToMenu` path is the **temporary quit-to-menu**
(design §8 Q1) — preserve it as a keybind/dev shortcut so quit-to-menu survives
the egui-card retirement. (If `ReturnToMenu` was only reachable via the egui
button, wire it to an existing keybind here and note it as the temp escape.)

**Step 3: Verify:** `cargo build -p vera20k` → compiles; `cargo test -p vera20k`
lib suite → 0 failed. **Step 4: Commit.**

### Task 9: Manual in-game visual stop gate + verification against gamemd

**Why:** The buttons' sidebar-anchored Y (KD-4) and the overall composition need
a side-by-side check the unit tests cannot provide.

**Verify (run the app, no code):**
- Open active in-game Options (ESC) at 800×600. Keyboard, Sound, Back render the
  SIDEBTTN button art (frame 0), right-edge anchored (~`x=653`), 125×25.
- GameSpeed + ScrollRate trackbars + the three checkboxes render via the existing
  glyph seam; the title/caption/footer text statics render.
- **VisualDetails slider + its caption/label are ABSENT** (hidden in `0xBBB`).
- The dialog composites over the frozen battlefield (no opaque panel; the game shows
  behind the chrome).
- ESC closes the overlay; the temp quit-to-menu still works.
- At 1024×768, ordinary controls shift by +112/+84; buttons stay right-anchored.
- **KD-4 gate:** confirm the button column Y matches gamemd's (open the original
  in-game Options side-by-side). If VERA's sidebar anchor is off, fix the binding
  before commit — do NOT ship DLU-Y as final.

**Verify against gamemd (Ghidra, already grounded):** `0x00612B70` (frames),
`0x0060B000`/`0x0060B350` (button X = parentW−147, SIDEBTTN dims),
`0x0060B7A0` (centered offset, base 800×600), `0x004E1FE0` (VisualDetails never shown).

## Sources & References

- **Design doc:** `docs/plans/2026-06-12-slice5a-ingame-options-dialog-design.md`
- **5a-i plan (shipped):** `docs/plans/2026-06-12-slice5a-i-ingame-options-descriptor-plan.md`
- **Prior (superseded-naming) plan, reusable structure:** `docs/plans/2026-06-02-shell-substrate-slice5b-options-plan.md` §3
- **Ghidra reports:** `OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS_GHIDRA_REPORT.md`,
  `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`,
  `RESIZESHELLCHILDCONTROL_AND_REPOS_HELPERS_GHIDRA_REPORT.md`,
  `DLU_TO_PIXEL_FOR_SHELL_DIALOGS_GHIDRA_REPORT.md`
- **Session grounding (local):** `docs/plans/_5aii-grounding-lane{A,B,C,D}.md`
- **gamemd addresses (not in Rust comments):** `0x00612B70` OwnerDraw_Button (mode-2
  frames 0/1/2), `0x0060B000`/`0x0060B350` button right-edge anchor (147px inset,
  SHP-header dims), `0x0060B7A0` centered offset (base `0x007f5be4`=800/`0x007f5bf0`=600),
  `0x0060C0C0` resize dispatcher, `0x004E1FE0` proc (VisualDetails hidden),
  `0x004E1D00` ShowInGameDialog.
- **Assets:** `SIDEBTTN.SHP` (125×25, 3 frames, `ra2.mix→sidec01.mix`), `SIDEBAR.PAL`,
  `SDBTNANM.SHP` (already loaded).
- **Related code:** `src/app_skirmish_shell_render/controls.rs` (paint seam),
  `src/render/skirmish_shell_chrome.rs` (atlas), `src/ui/shell/{descriptor,layout,in_game_options}.rs`,
  `src/app.rs:2914-2922`/`3001-3057`, `src/app_input.rs:425-447`, `src/app_sim_tick.rs:203-211`.
