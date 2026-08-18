# Slice 4C — Combo faces onto the paint seam (collapsed faces only)

> Focused realization plan for sub-step 4C of `docs/plans/2026-06-01-shell-substrate-slice4-plan.md`,
> continuing the atlas-taking `ControlChrome` seam shipped in 4B (commit ea000965).
> **Rescoped after `/review-plan` (2026-06-12):** the original 4C tried to route the dropdown scrollbar +
> popup through the seam too, but the scrollbar emitter (`push_dropdown_scrollbar_instances`) is **shared
> with the choose-map listbox modal** (`modals.rs:77`), so converting it would force a `modals.rs` edit and
> blur the 4C/4D boundary. The `ScrollBar` ControlKind + popup chrome conversion therefore **defers to 4D**,
> where `modals.rs` and the listbox already migrate together (and feeds 4E's scroll-model unification).
> 4C is now **Combo-face-only: single-file, modals-untouched, exactly the 4B shape. Paint-only.**

## Scope
- Route the collapsed combo **faces** (face glyph + resolved swatch + arrow) through `paint_control` (new
  `Combo` arm).
- Byte-identical emission: face at `SHELL_CONTROL_DEPTH`, swatch at `SHELL_SWATCH_DEPTH`, arrow at
  `SHELL_CONTROL_DEPTH − 0.00001`.
- Input untouched (`state/combos.rs` machine stays as-is); the open dropdown popup
  (`push_dropdown_instances`) and the shared scrollbar emitter stay `&atlas`, **unchanged**.

## Deferred to 4D (was in the original 4C scope)
- `ScrollBar` ControlKind / `paint_control` ScrollBar arm.
- Converting the **shared** `push_dropdown_scrollbar_instances` + `push_scrollbar_thumb` + the popup fills
  (`push_solid_rect`/bevel via a `white_pixel`-parameterized `_px` core) to chrome.
- Growing `ControlChrome` with the scrollbar fields (4 arrows + 3 thumb).
These land in 4D because `modals.rs:71/77/80` already use the same scrollbar chrome, and 4D migrates the
choose-map listbox + its scrollbar.

## 1. Grow `ControlChrome` (in `src/render/skirmish_shell_chrome.rs`)
Add the **9** combo-face glyph fields (names mirror the atlas verbatim); `control_chrome()` copies them:
```
white_pixel,
combo_face_150, combo_face_117, combo_face_44, combo_face_38,
combo_arrow_down_released, combo_arrow_down_pressed,
combo_arrow_down_gray_released, combo_arrow_down_gray_pressed,
```
(joins the existing checkbox×2 + trackbar×5 → 16 fields total.)

## 2. `ControlPaint` — add the `Combo` arm (controls.rs)
```rust
pub(super) enum ControlPaint {
    Checkbox { checked: bool, rect: RectPx },
    Trackbar { rect: RectPx, thumb_px: i32 },
    Combo { rect: RectPx, swatch: Option<[f32; 3]>, open: bool, disabled: bool },
}
```
The arm reproduces `push_combo_face` byte-for-byte, resolving glyphs from `&ControlChrome` (the caller
resolves the swatch RGB so the arm stays chrome-only):
```rust
ControlPaint::Combo { rect, swatch, open, disabled } => {
    if let Some(face) = combo_face_entry(chrome, rect) {
        push_entry(out, face, combo_face_rect(rect), SHELL_CONTROL_DEPTH);
    }
    if let (Some(tint), Some(white)) = (swatch, chrome.white_pixel) {
        push_tinted_entry(out, white, combo_swatch_rect(rect), tint, SHELL_SWATCH_DEPTH);
    }
    let arrow = match (disabled, open) {
        (true, true) => chrome.combo_arrow_down_gray_pressed,
        (true, false) => chrome.combo_arrow_down_gray_released,
        (false, true) => chrome.combo_arrow_down_pressed,
        (false, false) => chrome.combo_arrow_down_released,
    };
    if let Some(arrow) = arrow {
        let arrow_rect = combo_arrow_rect(rect);
        push_entry_native(out, arrow, arrow_rect.x, arrow_rect.y, SHELL_CONTROL_DEPTH - 0.00001);
    }
}
```
(Depth is hardcoded — every `push_combo_face` caller already passes `SHELL_CONTROL_DEPTH`, like the 4B
trackbar arm.) Convert `combo_face_entry` to take `&ControlChrome` (sole caller is this arm).

## 3. Caller rewrite (controls.rs) — `push_combo_instances`
Build `let chrome = atlas.control_chrome();` once; for each face keep the existing per-face `color_index`
resolution (side/start/team → `None`; color faces → `Some(player_color_index)` /
`(!sibling_disabled).then_some(opponent.color_index)`) and pass
`swatch: color_index.map(|ci| house_color_tint(color_schemes, ci))`, plus `open` and `disabled`, into
`paint_control(out, &chrome, ControlPaint::Combo { .. })`. Iteration order over
side/color/start/team + per-opponent rows is unchanged. `push_combo_face` is deleted (its body now lives in
the `Combo` arm). `push_combo_instances` keeps its `atlas`/`color_schemes` params (call site unchanged).

## 4. Tests (controls.rs; never edit the frozen suites)
- **NEW `combo_face_paint_seam_emits_face_swatch_arrow`** (draw-list assertion, §1.4): with a synthetic
  `ControlChrome` (face + arrows + white_pixel) and a fixed face rect, assert the emitted sequence for the
  four cases — (a) color face open, (b) color face closed, (c) plain face, (d) disabled face — matches the
  pre-seam `push_combo_face` output: face glyph at `combo_face_rect`/`SHELL_CONTROL_DEPTH`; swatch (color
  cases only) at `combo_swatch_rect`/`SHELL_SWATCH_DEPTH`/resolved tint; arrow variant per `(disabled,open)`
  at `combo_arrow_rect`/`SHELL_CONTROL_DEPTH − 0.00001`. Also assert `ControlChrome::default()` (no glyphs)
  → nothing emitted.
- Input pins stay GREEN-unchanged in the frozen `state/tests.rs` (input untouched, so wheel-inert /
  two-clicks-to-switch / reverse-order are already pinned there).

## 5. Checkpoint + STOP (per plan §6.2)
`cargo build -p vera20k` && `cargo test -p vera20k` (separate bounded pass). Read the literal
`test result:` line. Confirm `state/tests.rs`=87, `layout.rs`=30 unchanged AND
`git diff HEAD -- src/ui/skirmish_shell/state/tests.rs src/ui/skirmish_shell/layout.rs` EMPTY. Format only
edited files (`rustfmt --edition 2024`, hand-apply to my regions only — `controls.rs` has pre-existing
non-conforming tests). Commit only my two files (leave the parallel session's dirty tree alone).

## Files touched
- `src/render/skirmish_shell_chrome.rs` (9 new `ControlChrome` fields + `control_chrome()` copies)
- `src/app_skirmish_shell_render/controls.rs` (enum `Combo` arm, `combo_face_entry` → chrome, delete
  `push_combo_face`, rewrite `push_combo_instances`, new test)

## Out of scope (unchanged from plan §5.7, plus the 4D deferral above)
`state/combos.rs` input/state machine; `modals.rs`; `push_dropdown_instances` + the shared scrollbar
emitter; listbox/choose-map (4D); scroll-model unification (4E); defaults seed (4F).
