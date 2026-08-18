# Slice 4D — Listbox / choose-map family onto the paint seam (+ app.rs input migration)

> Focused realization plan for sub-step 4D of `docs/plans/2026-06-01-shell-substrate-slice4-plan.md` (§4D),
> continuing the atlas-taking `ControlChrome` paint seam shipped in 4B (ea000965) and 4C (76a7fa56).
> Full-§4D scope (user decision 2026-06-12): paint seam continuation **plus** the app.rs listbox
> scroll/wheel input migration. Delivered as **two green-gated commits** — **4D.1 paint** (no contended
> files), then **4D.2 input** (touches the contended `app.rs`, minimal write window).

**Goal:** Route the shared scrollbar (combo dropdown + choose-map listbox) through a new `paint_control`
`ScrollBar` arm resolving glyphs from `ControlChrome`, and relocate the choose-map listbox scroll/wheel
input from `app.rs` into the `ui/skirmish_shell/` layer with `app.rs` delegating — both byte-identical to
HEAD.

**Architecture:** The paint seam stays app-layer (`paint_control` in `app_skirmish_shell_render/controls.rs`
takes the texture-free `ControlChrome` subset; `render/` must not depend on the app layer). The input
migration keeps `ChooseMapModalState` (`ui/skirmish_shell/state/choose_map.rs`) the owner; `app.rs`
delegates and never double-handles. No `DialogController` change, no descriptor `ControlKind`, no
`render/shell_paint.rs` dispatch (the realized 4A–4C program never used those — see Grounding).

**Design doc:** `docs/plans/2026-06-01-shell-substrate-slice4-plan.md` §4D; shape mirrors
`docs/plans/2026-06-12-slice4c-combo-controlchrome-seam-plan.md`.

---

## Grounding Summary

- **Spec = the in-repo committed behavior, pinned by `state/tests.rs` (87) + `layout.rs` (30).** This is a
  consolidation/relocation slice, not a gamemd re-derivation: the burden-of-proof bar (default DRIFT) is met
  by **byte-identical** draw-list emission and byte-identical input behavior vs HEAD, proven by draw-list
  assertions + the frozen suite staying GREEN with EMPTY diffs. No new Ghidra/INI research is required (the
  4-PRE + 4A–4C grounding already settled the architecture; 4F owns the INI seed + O5 widget Ghidra pre-req).
- **Realized program diverged from the parent plan's drafted architecture.** 4A/4B/4C did **not** add
  descriptor `ControlKind`s, edit `render/shell_paint.rs`, or touch input. They built the app-layer
  `ControlChrome` + `paint_control(out, &chrome, ControlPaint::X)` seam in `controls.rs`. 4D follows the
  **realized** shape: extend `ControlChrome` + `paint_control`, not the descriptor table.
- **The scrollbar emitter is shared.** `push_dropdown_scrollbar_instances` (`controls.rs:165`) is called by
  both the combo popup (`push_dropdown_instances`, `controls.rs:539`) and the choose-map listbox
  (`push_choose_map_listbox_instances`, `modals.rs:77`). It already **hardcodes** `SHELL_DROPDOWN_DEPTH`
  offsets (−0.00004 track, −0.00005 arrows, −0.00006 thumb, −0.00007 bevel) **regardless of caller** — so a
  `ScrollBar` arm can hardcode the same depths, matching the prior arms' hardcoded-depth pattern (no depth
  parameter needed).
- **The listbox input is already nearly ui-pure.** `handle_choose_map_listbox_scrollbar_mouse_down`
  (`app.rs:1125`) and the wheel core (`app.rs:1175`) operate only on `&mut ChooseMapModalState` + layout
  rects + `crate::ui::skirmish_shell::*` helpers — they move into `choose_map.rs` as methods almost verbatim.
  The OK/Cancel/Random **buttons** + mouse-up commit (`handle_choose_map_modal_mouse_up`, `app.rs:1086`) stay
  app-side (deferred per §5.7).
- **`white_pixel` already lives in `ControlChrome`** (added in 4C). 4D adds the **7** scrollbar glyph fields
  (4 arrows + 3 thumb) and the `_px` fill cores so the `ScrollBar` arm is chrome-only.

## Key Technical Decisions

- **`ScrollBar` arm hardcodes `SHELL_DROPDOWN_DEPTH` offsets** (not a depth param) — **Confidence:** high.
  **Source:** `controls.rs:165-209` (the existing emitter hardcodes them for both callers).
- **`paint_control` takes `&ControlChrome` only** (the seam contract); the arm's fills use new `_px` cores
  reading `chrome.white_pixel` — **Confidence:** high. **Source:** 4A/4B/4C seam shape (`controls.rs:330`).
- **Popup/listbox emitters keep their `&atlas` signatures**, build `let chrome = atlas.control_chrome()`
  internally, and call the arm — so call sites in `app_skirmish_shell_render.rs` (lines 203/369) and
  `modals.rs` are **unchanged** and `app_skirmish_shell_render.rs` test count (53) stays — **Confidence:**
  high. **Source:** 4C's `push_combo_instances` kept its `atlas` param identically (`controls.rs:409`).
- **`_px` refactor scoped to fills used INSIDE the arm only** (`push_solid_rect`, `push_bevel_ring`,
  `push_ownerdraw_two_pixel_bevel_frame`); existing `&atlas` fns become thin wrappers so every other caller
  (preview backdrop `app_skirmish_shell_render.rs:556`, player-name edit, modal dialog outline) stays
  byte-identical — **Confidence:** high. **Source:** caller scan (`push_rect_outline` is NOT used inside the
  arm, so it needs no `_px`).
- **Input migrates as `ChooseMapModalState` methods; `app.rs` delegates once** — **Confidence:** high.
  **Source:** `app.rs:1024-1207` is already `modal`-+-ui-helper-only.

## Open Questions

### Resolved During Planning
- *Does the `ScrollBar` arm need a depth parameter?* No — the shared emitter hardcodes `SHELL_DROPDOWN_DEPTH`
  offsets for both callers (`controls.rs:172-208`), so the arm hardcodes them too.
- *Do the render call sites change (would `app_skirmish_shell_render.rs`=53 move)?* No — emitters keep `&atlas`
  and build `chrome` internally.
- *Where do new input tests go (state/tests.rs is frozen)?* In a new `#[cfg(test)] mod tests` in
  `choose_map.rs` (a non-frozen file).

### Deferred to Implementation
- Exact `compute_choose_map_modal_layout(800, 600)` `map_list`/`mode_list` rects (`map_list.h/19` →
  visible-row count) — read off the helper when writing the wheel-branch fixtures; assertions key off the
  observable `map_top_index`/`mode_top_index` deltas, not hardcoded geometry.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/render/skirmish_shell_chrome.rs` | +7 scrollbar glyph fields on `ControlChrome` + `control_chrome()` copies |
| Modify | `src/app_skirmish_shell_render/chrome.rs` | `_px` fill cores (`push_solid_rect_px`, `push_bevel_ring_px`, `push_ownerdraw_two_pixel_bevel_frame_px`); existing fns → wrappers |
| Modify | `src/app_skirmish_shell_render/controls.rs` | `ScrollBar` arm; `push_scrollbar_thumb`→chrome; delete `push_dropdown_scrollbar_instances` (body→arm); `push_dropdown_instances` builds chrome + calls arm; new draw-list test |
| Modify | `src/app_skirmish_shell_render/modals.rs` | `push_choose_map_listbox_instances` builds chrome + calls arm |
| Modify | `src/ui/skirmish_shell/state/choose_map.rs` | `handle_listbox_mouse_down` + `handle_listbox_wheel` + private `listbox_scrollbar_mouse_down`; new `mod tests` |
| Modify | `src/app.rs` | `handle_choose_map_modal_mouse_down`/`_wheel` delegate to the ui methods; delete migrated `handle_choose_map_listbox_scrollbar_mouse_down` |

## Interface Changes

- **`ControlChrome`** gains 7 `pub` fields (additive; `Default` derive unaffected; 4A/4B/4C construction
  `ControlChrome { .., ..Default::default() }` still compiles).
- **`ControlPaint`** gains a `ScrollBar { scrollbar: RectPx, thumb: RectPx, pressed_part: Option<DropdownScrollbarPart> }`
  variant (additive; `pub(super)` enum, all match sites are in `controls.rs`).
- **`ChooseMapModalState`** gains 2 `pub` methods + 1 private; **no field changes**. The existing methods
  (`scroll_listbox_by_rows`, `set_top_index_clamped`, `select_mode`, `select_map_filtered_row`,
  `mode_row_count`, `map_row_count`, `top_index`) keep their signatures — so the frozen `state/tests.rs`
  callers stay GREEN.
- **`app.rs`** removes the private static `handle_choose_map_listbox_scrollbar_mouse_down` (no external
  callers — confirm before delete).
- **`push_dropdown_scrollbar_instances`** is deleted; both callers route through the arm. **`modals.rs`**
  drops its `use super::controls::push_dropdown_scrollbar_instances;` import.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| 4D.1 T3 | `ScrollBar` arm emission (track→up→down→thumb top/mid/bottom→bevel), exact order/depth/position | The dropdown + listbox scrollbar is on screen every time a combo opens / Choose-Map is open; a 1-instance or 1-depth drift is a visible z-fight or missing arrow | NEW draw-list assertion in `controls.rs` vs the pre-seam `push_dropdown_scrollbar_instances` sequence |
| 4D.1 T2/T5 | `_px` wrappers stay byte-identical for non-arm callers (preview backdrop, player-name caret/selection, modal outline) | Those regions paint every frame; an accidental tint/depth change is visible | Frozen suites GREEN + draw-list parity; wrappers call `_px` with `atlas.white_pixel` (no value change) |
| 4D.1 T6 | Listbox scrollbar double-track-fill preserved (listbox draws a track at `depth−0.000015`, then the arm redraws at `SHELL_DROPDOWN_DEPTH−0.00004`) | Existing layered behavior; dropping either changes the listbox scrollbar pixels | Keep both emissions in `push_choose_map_listbox_instances`; draw-list parity |
| 4D.2 T8 | Four wheel branches: `lines==0`→consume/no-scroll; `lines>0`→`−ceil`; `lines<0`→`+ceil`; map-first-then-mode-else-consume | Mouse wheel over either listbox is a core Choose-Map interaction | NEW per-branch tests in `choose_map.rs` |
| 4D.2 T8 | Scrollbar mouse-down: up-arrow/down-arrow/thumb-hit(no-drag)/track-click rounding | Clicking the listbox scrollbar must land the same `top_index` as HEAD | NEW test in `choose_map.rs`; logic moved verbatim |
| 4D.2 T9 | `app.rs` delegates exactly once (no double-handle) | Double-handling would scroll twice per click | Code inspection: one `modal.handle_listbox_*` call per event; arm returns a single consumed bool |

---

## Deferred to a later slice (out of 4D scope)
- **4E** scroll-model unification (the SIX divergence axes parameterized + equivalence proof) — gated behind
  4C AND 4D both green.
- **4F** C14 defaults seed + the O5 Ghidra widget-surfacing pre-req.
- Choose-Map modal **OK/Cancel/Random buttons + mouse-up commit + modal stacking** (C1/C5) — stay app-side
  (`app.rs:1086-1123` untouched).
- `state/combos.rs` combo input machine; descriptor `ControlKind`; `render/shell_paint.rs`.

---

# COMMIT 4D.1 — Paint: route the shared scrollbar through a `ControlChrome` `ScrollBar` arm

### Task 1: Grow `ControlChrome` with the 7 scrollbar glyph fields

**Why:** The `ScrollBar` arm resolves its glyphs from the texture-free subset; fields must exist first.

**Files:** Modify `src/render/skirmish_shell_chrome.rs`.

**Pattern:** Identical to 4C's combo-glyph growth (struct fields + `control_chrome()` copies).

**Step 1:** In `struct ControlChrome` (after the 4C combo fields, before the closing `}`), add:
```rust
    pub scrollbar_arrow_up_released: Option<SkirmishShellChromeEntry>,
    pub scrollbar_arrow_up_pressed: Option<SkirmishShellChromeEntry>,
    pub scrollbar_arrow_down_released: Option<SkirmishShellChromeEntry>,
    pub scrollbar_arrow_down_pressed: Option<SkirmishShellChromeEntry>,
    pub scrollbar_thumb_top: Option<SkirmishShellChromeEntry>,
    pub scrollbar_thumb_mid: Option<SkirmishShellChromeEntry>,
    pub scrollbar_thumb_bottom: Option<SkirmishShellChromeEntry>,
```

**Step 2:** In `fn control_chrome(&self) -> ControlChrome`, before the closing `}` of the struct literal, add:
```rust
            scrollbar_arrow_up_released: self.scrollbar_arrow_up_released,
            scrollbar_arrow_up_pressed: self.scrollbar_arrow_up_pressed,
            scrollbar_arrow_down_released: self.scrollbar_arrow_down_released,
            scrollbar_arrow_down_pressed: self.scrollbar_arrow_down_pressed,
            scrollbar_thumb_top: self.scrollbar_thumb_top,
            scrollbar_thumb_mid: self.scrollbar_thumb_mid,
            scrollbar_thumb_bottom: self.scrollbar_thumb_bottom,
```
(The full `SkirmishShellChromeAtlas` already owns these fields — `chrome.rs` atlas lines 72-78 — so the copy
compiles unchanged.)

**Step 3 (verify):** `cargo check -p vera20k` — expect clean (only pre-existing warnings).

### Task 2: Add the `_px` fill cores in `chrome.rs`; make existing fills thin wrappers

**Why:** `paint_control` is chrome-only; the arm's track-fill + bevel must resolve `white_pixel` from a
passed-in `Option`, not from `&atlas`. Existing `&atlas` callers must stay byte-identical.

**Files:** Modify `src/app_skirmish_shell_render/chrome.rs`.

**Pattern:** Extract-core + wrapper (the `&atlas` fn forwards `atlas.white_pixel` to the `_px` core).

**Step 1:** Replace `push_solid_rect` (currently `controls.rs`… in `chrome.rs:399-410`) with a core + wrapper:
```rust
pub(super) fn push_solid_rect_px(
    out: &mut Vec<SpriteInstance>,
    white_pixel: Option<SkirmishShellChromeEntry>,
    rect: RectPx,
    tint: [f32; 3],
    depth: f32,
) {
    let Some(pixel) = white_pixel else {
        return;
    };
    push_tinted_entry(out, pixel, rect, tint, depth);
}

pub(super) fn push_solid_rect(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    tint: [f32; 3],
    depth: f32,
) {
    push_solid_rect_px(out, atlas.white_pixel, rect, tint, depth);
}
```

**Step 2:** Convert `push_bevel_ring` to a `_px` core + `&atlas` wrapper. The body is unchanged except every
internal `push_solid_rect(out, atlas, ...)` becomes `push_solid_rect_px(out, white_pixel, ...)`:
```rust
pub(super) fn push_bevel_ring_px(
    out: &mut Vec<SpriteInstance>,
    white_pixel: Option<SkirmishShellChromeEntry>,
    rect: RectPx,
    top_left_tint: [f32; 3],
    bottom_right_tint: [f32; 3],
    depth: f32,
) {
    if rect.w <= 0 || rect.h <= 0 {
        return;
    }
    push_solid_rect_px(out, white_pixel, RectPx::new(rect.x, rect.y, rect.w, 1), top_left_tint, depth);
    if rect.h > 1 {
        push_solid_rect_px(out, white_pixel, RectPx::new(rect.x, rect.y + 1, 1, rect.h - 1), top_left_tint, depth);
    }
    push_solid_rect_px(out, white_pixel, RectPx::new(rect.x, rect.y + rect.h - 1, rect.w, 1), bottom_right_tint, depth);
    if rect.w > 1 && rect.h > 2 {
        push_solid_rect_px(out, white_pixel, RectPx::new(rect.x + rect.w - 1, rect.y + 1, 1, rect.h - 2), bottom_right_tint, depth);
    }
}

pub(super) fn push_bevel_ring(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    top_left_tint: [f32; 3],
    bottom_right_tint: [f32; 3],
    depth: f32,
) {
    push_bevel_ring_px(out, atlas.white_pixel, rect, top_left_tint, bottom_right_tint, depth);
}
```

**Step 3:** Convert `push_ownerdraw_two_pixel_bevel_frame` to a `_px` core + wrapper (body identical, inner
`push_bevel_ring(out, atlas, ...)` → `push_bevel_ring_px(out, white_pixel, ...)`):
```rust
pub(super) fn push_ownerdraw_two_pixel_bevel_frame_px(
    out: &mut Vec<SpriteInstance>,
    white_pixel: Option<SkirmishShellChromeEntry>,
    rect: RectPx,
    depth: f32,
) {
    push_bevel_ring_px(
        out,
        white_pixel,
        rect,
        OWNERDRAW_BEVEL_LIGHT_RGB_FROM_PACKED_00C5BEA7,
        OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
        depth,
    );
    if rect.w > 2 && rect.h > 2 {
        push_bevel_ring_px(
            out,
            white_pixel,
            RectPx::new(rect.x + 1, rect.y + 1, rect.w - 2, rect.h - 2),
            OWNERDRAW_BEVEL_DARK_RGB_FROM_PACKED_00807A68,
            OWNERDRAW_BEVEL_LIGHT_RGB_FROM_PACKED_00C5BEA7,
            depth - 0.00001,
        );
    }
}

pub(super) fn push_ownerdraw_two_pixel_bevel_frame(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    depth: f32,
) {
    push_ownerdraw_two_pixel_bevel_frame_px(out, atlas.white_pixel, rect, depth);
}
```

**Step 4 (verify):** `cargo check -p vera20k` — clean. `push_rect_outline` is left as-is (NOT used inside the
arm).

### Task 3: Add the `ScrollBar` arm to `ControlPaint` + `paint_control`; convert `push_scrollbar_thumb` to chrome

**Why:** This is the seam — one chrome-only emitter reproducing `push_dropdown_scrollbar_instances`
byte-for-byte.

**Files:** Modify `src/app_skirmish_shell_render/controls.rs`.

**Pattern:** 4C's `Combo` arm (resolve glyphs from `&ControlChrome`, hardcoded depths).

**Step 1:** Update the `chrome.rs` import line in `controls.rs` (`use super::chrome::{...}`) to add the new
`_px` cores:
```rust
use super::chrome::{
    push_entry, push_entry_native, push_ownerdraw_two_pixel_bevel_frame,
    push_ownerdraw_two_pixel_bevel_frame_px, push_solid_rect, push_solid_rect_px, push_tinted_entry,
};
```

**Step 2:** Add the variant to `ControlPaint` (after `Combo { .. }`):
```rust
    ScrollBar {
        scrollbar: RectPx,
        thumb: RectPx,
        pressed_part: Option<DropdownScrollbarPart>,
    },
```
Update the enum doc comment's trailing sentence to: `4C adds the collapsed Combo face; 4D adds the shared ScrollBar; later steps add listbox arms.`

**Step 3:** Convert `push_scrollbar_thumb` to read from `&ControlChrome` (sole caller is the arm). Replace its
`atlas: &SkirmishShellChromeAtlas` param with `chrome: &ControlChrome` and each `atlas.scrollbar_thumb_*`
with `chrome.scrollbar_thumb_*`:
```rust
pub(super) fn push_scrollbar_thumb(
    out: &mut Vec<SpriteInstance>,
    chrome: &ControlChrome,
    rect: RectPx,
    depth: f32,
) {
    let top_h = chrome
        .scrollbar_thumb_top
        .map(|entry| entry.pixel_size[1].round() as i32)
        .unwrap_or(0);
    let bottom_h = chrome
        .scrollbar_thumb_bottom
        .map(|entry| entry.pixel_size[1].round() as i32)
        .unwrap_or(0);
    if let Some(top) = chrome.scrollbar_thumb_top {
        push_entry_native(out, top, rect.x, rect.y, depth);
    }
    if let Some(bottom) = chrome.scrollbar_thumb_bottom {
        push_entry_native(out, bottom, rect.x, rect.y + rect.h - bottom_h, depth);
    }
    if let Some(mid) = chrome.scrollbar_thumb_mid {
        let mid_y = rect.y + top_h;
        let mid_h = rect.h - top_h - bottom_h;
        if mid_h > 0 {
            push_entry(out, mid, RectPx::new(rect.x, mid_y, rect.w, mid_h), depth);
        }
    }
}
```

**Step 4:** Delete `push_dropdown_scrollbar_instances` entirely (its body becomes the arm). Add the arm to
`paint_control` (after the `Combo` arm), reproducing the deleted emitter byte-for-byte with `_px` cores and
`chrome` glyphs:
```rust
        ControlPaint::ScrollBar {
            scrollbar,
            thumb,
            pressed_part,
        } => {
            // Byte-for-byte the pre-seam push_dropdown_scrollbar_instances: track fill
            // → up arrow → down arrow → thumb(top/mid/bottom) → bevel frame, at the
            // hardcoded SHELL_DROPDOWN_DEPTH offsets the shared emitter used for BOTH
            // the combo popup and the choose-map listbox.
            push_solid_rect_px(
                out,
                chrome.white_pixel,
                scrollbar,
                SHELL_SCROLLBAR_TRACK_RGB_PENDING_SCROLLBAR_SOURCE_CAPTURE,
                SHELL_DROPDOWN_DEPTH - 0.00004,
            );
            let up_entry = scrollbar_arrow_entry(
                chrome.scrollbar_arrow_up_released,
                chrome.scrollbar_arrow_up_pressed,
                pressed_part == Some(DropdownScrollbarPart::UpArrow),
            );
            if let Some(up) = up_entry {
                push_entry_native(out, up, scrollbar.x, scrollbar.y, SHELL_DROPDOWN_DEPTH - 0.00005);
            }
            let down_entry = scrollbar_arrow_entry(
                chrome.scrollbar_arrow_down_released,
                chrome.scrollbar_arrow_down_pressed,
                pressed_part == Some(DropdownScrollbarPart::DownArrow),
            );
            if let Some(down) = down_entry {
                push_entry_native(
                    out,
                    down,
                    scrollbar.x,
                    scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H,
                    SHELL_DROPDOWN_DEPTH - 0.00005,
                );
            }
            push_scrollbar_thumb(out, chrome, thumb, SHELL_DROPDOWN_DEPTH - 0.00006);
            push_ownerdraw_two_pixel_bevel_frame_px(
                out,
                chrome.white_pixel,
                scrollbar,
                SHELL_DROPDOWN_DEPTH - 0.00007,
            );
        }
```
(`scrollbar_arrow_entry`, `DropdownScrollbarPart`, `COMBO_DROPDOWN_SCROLLBAR_BUTTON_H`, `SHELL_DROPDOWN_DEPTH`,
`SHELL_SCROLLBAR_TRACK_RGB_PENDING_SCROLLBAR_SOURCE_CAPTURE` are already imported in `controls.rs`.)

**Step 5 (verify):** `cargo check -p vera20k` — expect an error at `push_dropdown_instances` and in
`modals.rs` (callers of the deleted fn); fixed in Tasks 4–5.

### Task 4: Route the combo popup (`push_dropdown_instances`) through the arm

**Why:** First caller of the deleted shared emitter.

**Files:** Modify `src/app_skirmish_shell_render/controls.rs`.

**Step 1:** In `push_dropdown_instances`, build the chrome once near the top (after the early-return guards
that establish `dropdown`/`content`), e.g. right before the `if needs_scrollbar {` block:
```rust
    let chrome = atlas.control_chrome();
```

**Step 2:** Replace the `push_dropdown_scrollbar_instances(out, atlas, scrollbar, thumb, pressed_part);` call
with:
```rust
            paint_control(
                out,
                &chrome,
                ControlPaint::ScrollBar {
                    scrollbar,
                    thumb,
                    pressed_part,
                },
            );
```
(The surrounding `push_solid_rect`/`push_ownerdraw_two_pixel_bevel_frame` fills in this fn keep their `&atlas`
wrappers — byte-identical. `push_dropdown_instances` keeps its `&atlas` signature; call site at
`app_skirmish_shell_render.rs:369` is unchanged.)

**Step 3 (verify):** `cargo check -p vera20k` — `controls.rs` now clean; `modals.rs` still errors (Task 5).

### Task 5: Route the choose-map listbox (`push_choose_map_listbox_instances`) through the arm

**Why:** Second caller of the deleted shared emitter.

**Files:** Modify `src/app_skirmish_shell_render/modals.rs`.

**Step 1:** Update imports: drop `use super::controls::push_dropdown_scrollbar_instances;` and add
`use super::controls::{ControlPaint, paint_control};`.

**Step 2:** In `push_choose_map_listbox_instances`, inside the `if let Some(scrollbar) = … { if let Some(thumb)
= … {` block, keep the existing track-prefill `push_solid_rect(out, atlas, scrollbar, …, depth − 0.000015);`
(parity item 4D.1 T6 — the listbox's own track layer), then replace
`push_dropdown_scrollbar_instances(out, atlas, scrollbar, thumb, None);` with:
```rust
            let chrome = atlas.control_chrome();
            paint_control(
                out,
                &chrome,
                ControlPaint::ScrollBar {
                    scrollbar,
                    thumb,
                    pressed_part: None,
                },
            );
```
(`push_choose_map_listbox_instances` keeps `&atlas`; the modal-level fills/outline/background stay on `&atlas`
wrappers. Call sites at `modals.rs:130/139` unchanged.)

**Step 3 (verify):** `cargo check -p vera20k` — fully clean.

### Task 6: NEW draw-list test — `scrollbar_paint_seam_emits_track_arrows_thumb_bevel`

**Why:** Pin the arm's emission byte-for-byte against the pre-seam shared emitter (parity item 4D.1 T3).

**Files:** Modify `src/app_skirmish_shell_render/controls.rs` (add to `mod tests`, before
`checkbox_icon_rect_right_edge_is_half_open`).

**Pattern:** 4C's `combo_face_paint_seam_emits_face_swatch_arrow` (synthetic `ControlChrome` + per-instance
position/uv/depth assertions).

**Step 1:** Add:
```rust
    #[test]
    fn scrollbar_paint_seam_emits_track_arrows_thumb_bevel() {
        // Draw-list assertion (Slice 4 §1.4): the ScrollBar arm reproduces the
        // pre-seam push_dropdown_scrollbar_instances sequence — track fill →
        // up arrow → down arrow → thumb(top/mid/bottom) → 2-ring bevel frame — at
        // the hardcoded SHELL_DROPDOWN_DEPTH offsets, for both pressed states.
        let white = SkirmishShellChromeEntry {
            uv_origin: [0.01, 0.02],
            uv_size: [0.03, 0.04],
            pixel_size: [1.0, 1.0],
        };
        let up_r = SkirmishShellChromeEntry { uv_origin: [0.11, 0.12], uv_size: [0.13, 0.14], pixel_size: [20.0, 22.0] };
        let up_p = SkirmishShellChromeEntry { uv_origin: [0.21, 0.22], uv_size: [0.23, 0.24], pixel_size: [20.0, 22.0] };
        let dn_r = SkirmishShellChromeEntry { uv_origin: [0.31, 0.32], uv_size: [0.33, 0.34], pixel_size: [20.0, 22.0] };
        let dn_p = SkirmishShellChromeEntry { uv_origin: [0.41, 0.42], uv_size: [0.43, 0.44], pixel_size: [20.0, 22.0] };
        let th_t = SkirmishShellChromeEntry { uv_origin: [0.51, 0.52], uv_size: [0.53, 0.54], pixel_size: [20.0, 6.0] };
        let th_m = SkirmishShellChromeEntry { uv_origin: [0.61, 0.62], uv_size: [0.63, 0.64], pixel_size: [20.0, 4.0] };
        let th_b = SkirmishShellChromeEntry { uv_origin: [0.71, 0.72], uv_size: [0.73, 0.74], pixel_size: [20.0, 6.0] };
        let chrome = ControlChrome {
            white_pixel: Some(white),
            scrollbar_arrow_up_released: Some(up_r),
            scrollbar_arrow_up_pressed: Some(up_p),
            scrollbar_arrow_down_released: Some(dn_r),
            scrollbar_arrow_down_pressed: Some(dn_p),
            scrollbar_thumb_top: Some(th_t),
            scrollbar_thumb_mid: Some(th_m),
            scrollbar_thumb_bottom: Some(th_b),
            ..Default::default()
        };
        // scrollbar tall enough for top+bottom thumb caps + a positive mid span.
        let scrollbar = RectPx::new(300, 100, 20, 120);
        let thumb = RectPx::new(300, 144, 20, 30);

        // Default (no pressed part): both arrows show the RELEASED glyph.
        let mut out = Vec::new();
        paint_control(
            &mut out,
            &chrome,
            ControlPaint::ScrollBar { scrollbar, thumb, pressed_part: None },
        );
        // track(1) + up(1) + down(1) + thumb top/bottom/mid(3) + 2 bevel rings × 4
        // edges each (each ring: top/left/bottom/right solid rects) = 8.
        assert_eq!(out.len(), 14, "track + 2 arrows + 3 thumb + 8 bevel edges");

        // 0: track fill — white pixel tinted, scrollbar rect, DEPTH-0.00004.
        assert_eq!(out[0].position, [scrollbar.x as f32, scrollbar.y as f32]);
        assert_eq!(out[0].size, [scrollbar.w as f32, scrollbar.h as f32]);
        assert_eq!(out[0].uv_origin, white.uv_origin);
        assert_eq!(out[0].depth, SHELL_DROPDOWN_DEPTH - 0.00004);

        // 1: up arrow — released, native at scrollbar origin, DEPTH-0.00005.
        assert_eq!(out[1].position, [scrollbar.x as f32, scrollbar.y as f32]);
        assert_eq!(out[1].uv_origin, up_r.uv_origin);
        assert_eq!(out[1].depth, SHELL_DROPDOWN_DEPTH - 0.00005);

        // 2: down arrow — released, native at the bottom button slot, DEPTH-0.00005.
        assert_eq!(
            out[2].position,
            [scrollbar.x as f32, (scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H) as f32]
        );
        assert_eq!(out[2].uv_origin, dn_r.uv_origin);
        assert_eq!(out[2].depth, SHELL_DROPDOWN_DEPTH - 0.00005);

        // 3: thumb top — native at thumb origin, DEPTH-0.00006.
        assert_eq!(out[3].position, [thumb.x as f32, thumb.y as f32]);
        assert_eq!(out[3].uv_origin, th_t.uv_origin);
        assert_eq!(out[3].depth, SHELL_DROPDOWN_DEPTH - 0.00006);

        // 4: thumb bottom — native, bottom-aligned in the thumb rect.
        let bottom_h = th_b.pixel_size[1].round() as i32;
        assert_eq!(out[4].position, [thumb.x as f32, (thumb.y + thumb.h - bottom_h) as f32]);
        assert_eq!(out[4].uv_origin, th_b.uv_origin);

        // 5: thumb mid — stretched between caps.
        let top_h = th_t.pixel_size[1].round() as i32;
        assert_eq!(out[5].position, [thumb.x as f32, (thumb.y + top_h) as f32]);
        assert_eq!(out[5].size, [thumb.w as f32, (thumb.h - top_h - bottom_h) as f32]);
        assert_eq!(out[5].uv_origin, th_m.uv_origin);

        // 6..10: outer bevel ring (4 edges) at DEPTH-0.00007; 10..14: inner ring at -0.00008.
        for inst in &out[6..10] {
            assert_eq!(inst.depth, SHELL_DROPDOWN_DEPTH - 0.00007, "outer bevel ring depth");
        }
        for inst in &out[10..14] {
            assert_eq!(inst.depth, SHELL_DROPDOWN_DEPTH - 0.00007 - 0.00001, "inner bevel ring depth");
        }

        // Pressed up-arrow swaps ONLY the up glyph to the pressed uv.
        let mut pressed = Vec::new();
        paint_control(
            &mut pressed,
            &chrome,
            ControlPaint::ScrollBar { scrollbar, thumb, pressed_part: Some(DropdownScrollbarPart::UpArrow) },
        );
        assert_eq!(pressed[1].uv_origin, up_p.uv_origin);
        assert_eq!(pressed[2].uv_origin, dn_r.uv_origin);

        // Empty chrome → nothing emitted.
        let mut empty = Vec::new();
        paint_control(
            &mut empty,
            &ControlChrome::default(),
            ControlPaint::ScrollBar { scrollbar, thumb, pressed_part: None },
        );
        assert!(empty.is_empty());
    }
```
Note the bevel-inner only emits when `rect.w > 2 && rect.h > 2` (20×120 satisfies it → 2 instances). `mod
tests` already imports `DropdownScrollbarPart`? It uses `use super::*;` — `DropdownScrollbarPart` is imported
at the top of `controls.rs`, so it is in scope. Confirm `SHELL_DROPDOWN_DEPTH` is too (it is — top imports).

**Step 2 (verify):** `cargo test -p vera20k --lib app_skirmish_shell_render::controls` — all GREEN incl. the
new test.

### Task 7: 4D.1 checkpoint + STOP, format, commit

**Step 1 (build):** `cargo build -p vera20k` — read the literal final line.

**Step 2 (test, separate bounded pass):** `cargo test -p vera20k --lib app_skirmish_shell_render::controls`
and `… ui::skirmish_shell::state::tests` and `… ui::skirmish_shell::layout` — read each literal
`test result:` line. Confirm `state/tests.rs`=**87**, `layout.rs`=**30**, `app_skirmish_shell_render.rs`=**53**
unchanged.

**Step 3 (frozen-diff gate):** `git diff HEAD -- src/ui/skirmish_shell/state/tests.rs src/ui/skirmish_shell/layout.rs`
must be **EMPTY**. If ANY check fails: hard-revert this commit and STOP.

**Step 4 (format):** `rustfmt --edition 2024 --check` each edited file; hand-apply ONLY to your regions
(`controls.rs` has pre-existing non-conforming tests/enum variants — do NOT churn untouched lines, per the 4C
discipline).

**Step 5 (commit ONLY the 4 paint files):**
```
git add src/render/skirmish_shell_chrome.rs src/app_skirmish_shell_render/chrome.rs \
        src/app_skirmish_shell_render/controls.rs src/app_skirmish_shell_render/modals.rs
git commit -m "ui: Slice 4D.1 - shared scrollbar onto the paint seam via ControlChrome ScrollBar arm"
```
Leave the parallel session's dirty tree (`src/rules/*`, `src/sim/*`) untouched.

---

# COMMIT 4D.2 — Input: migrate choose-map listbox scroll/wheel into the ui layer

### Task 8: Add the listbox input methods to `ChooseMapModalState`

**Why:** Relocate the behavior to its owner so `app.rs` becomes a thin delegator (parity items 4D.2 T8).

**Files:** Modify `src/ui/skirmish_shell/state/choose_map.rs`.

**Pattern:** Verbatim port of `app.rs:1024-1207` minus the button/dialog parts; methods on the modal state.

**Step 1:** Extend the `use super::super::layout::{…};` import to bring in the layout type + helpers the
methods need:
```rust
use super::super::layout::{
    ChooseMapListboxId, ChooseMapModalButton, ChooseMapModalLayout, RectPx,
    COMBO_DROPDOWN_SCROLLBAR_BUTTON_H, choose_map_listbox_rect, choose_map_listbox_row_at,
    choose_map_listbox_scroll_thumb_rect, choose_map_listbox_scrollbar_rect,
    choose_map_listbox_top_index_from_track_click, choose_map_listbox_visible_row_count,
};
```
(Confirm each symbol's exact module path when editing — some may re-export from `super::super` root rather
than `layout`; adjust the `use` accordingly. `ChooseMapModalButton` is already imported — keep it.)

**Step 2:** Inside `impl ChooseMapModalState`, add the private scrollbar-mousedown method (verbatim port of
`app.rs:1125-1173`, `Self::…(modal, …)` → `self`):
```rust
    fn listbox_scrollbar_mouse_down(
        &mut self,
        id: ChooseMapListboxId,
        list: RectPx,
        row_count: usize,
        x: i32,
        y: i32,
    ) -> bool {
        let Some(scrollbar) = choose_map_listbox_scrollbar_rect(row_count, list) else {
            return false;
        };
        if !scrollbar.contains(x, y) {
            return false;
        }
        let visible_rows = choose_map_listbox_visible_row_count(list);
        if y < scrollbar.y + COMBO_DROPDOWN_SCROLLBAR_BUTTON_H {
            self.scroll_listbox_by_rows(id, row_count, visible_rows, -1);
            return true;
        }
        if y >= scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H {
            self.scroll_listbox_by_rows(id, row_count, visible_rows, 1);
            return true;
        }
        if let Some(thumb) = choose_map_listbox_scroll_thumb_rect(row_count, self.top_index(id), list) {
            if thumb.contains(x, y) {
                return true;
            }
            if let Some(top_index) =
                choose_map_listbox_top_index_from_track_click(row_count, self.top_index(id), list, y)
            {
                self.set_top_index_clamped(id, row_count, visible_rows, top_index);
            }
        }
        true
    }
```

**Step 3:** Add the public mouse-down dispatch (verbatim port of `app.rs:1035-1080`, the non-button
`else`-block; returns `false` when nothing is consumed so the caller does the `dialog.contains` fallthrough):
```rust
    /// Dispatch a modal mouse-down that already missed the OK/Cancel/Random buttons:
    /// listbox scrollbars (mode, then map), then a mode-row click (re-filters), then a
    /// map-row click. Returns true if consumed. The button hit-test + dialog-contains
    /// fallthrough stay app-side (4D defers the modal chrome/buttons).
    pub fn handle_listbox_mouse_down(
        &mut self,
        layout: &ChooseMapModalLayout,
        modes: &[SkirmishGameMode],
        records: &[SkirmishScenarioRecord],
        x: i32,
        y: i32,
    ) -> bool {
        let mode_row_count = self.mode_row_count(modes);
        let map_row_count = self.map_row_count();
        if self.listbox_scrollbar_mouse_down(ChooseMapListboxId::Mode0x6eb, layout.mode_list, mode_row_count, x, y) {
            return true;
        }
        if self.listbox_scrollbar_mouse_down(ChooseMapListboxId::Map0x553, layout.map_list, map_row_count, x, y) {
            return true;
        }
        if let Some(mode_idx) =
            choose_map_listbox_row_at(layout.mode_list, mode_row_count, self.mode_top_index, x, y)
        {
            if let Some(mode) = modes.get(mode_idx) {
                self.select_mode(mode.id, modes, records);
            }
            return true;
        }
        if let Some(filtered_idx) =
            choose_map_listbox_row_at(layout.map_list, map_row_count, self.map_top_index, x, y)
        {
            self.select_map_filtered_row(filtered_idx);
            return true;
        }
        false
    }
```

**Step 4:** Add the wheel dispatch (verbatim port of `app.rs:1175-1207`, with `self` already the unwrapped
modal — the None case stays in the app wrapper):
```rust
    /// Mouse wheel over the modal. Four branches preserved byte-for-byte: cursor over
    /// map_list (checked FIRST) → map, else mode_list → mode, else consume; lines==0 →
    /// consume/no-scroll; lines>0 → up by ceil(|lines|); lines<0 → down by ceil(|lines|).
    pub fn handle_listbox_wheel(
        &mut self,
        layout: &ChooseMapModalLayout,
        modes: &[SkirmishGameMode],
        x: i32,
        y: i32,
        lines: f32,
    ) -> bool {
        let id = if layout.map_list.contains(x, y) {
            ChooseMapListboxId::Map0x553
        } else if layout.mode_list.contains(x, y) {
            ChooseMapListboxId::Mode0x6eb
        } else {
            return true;
        };
        if lines == 0.0 {
            return true;
        }
        let rows = if lines > 0.0 {
            -(lines.abs().ceil().max(1.0) as i32)
        } else {
            lines.abs().ceil().max(1.0) as i32
        };
        let list = choose_map_listbox_rect(layout, id);
        let visible_rows = choose_map_listbox_visible_row_count(list);
        let row_count = match id {
            ChooseMapListboxId::Mode0x6eb => self.mode_row_count(modes),
            ChooseMapListboxId::Map0x553 => self.map_row_count(),
        };
        self.scroll_listbox_by_rows(id, row_count, visible_rows, rows);
        true
    }
```

**Step 5 (verify):** `cargo check -p vera20k` — `choose_map.rs` compiles (app.rs still has the old handlers,
fixed in Task 9).

### Task 9: Make `app.rs` delegate (no double-handle); delete the migrated static

**Why:** `app.rs` becomes a thin wrapper; behavior owner is the modal (parity item 4D.2 T9).

**Files:** Modify `src/app.rs`. **CONTENDED — re-verify each block by CONTENT immediately before editing; if
another session's edits are present, WAIT (do not fix/revert/stash).**

**Step 1:** Replace the body of `handle_choose_map_modal_mouse_down` (currently `app.rs:1024-1084`) — keep the
button branch + final `dialog.contains`; replace the listbox `else`-block with one delegating call:
```rust
    fn handle_choose_map_modal_mouse_down(state: &mut AppState) -> bool {
        let layout = Self::skirmish_choose_map_layout(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        let Some(modal) = state.skirmish_shell_state.choose_map_modal.as_mut() else {
            return false;
        };
        if let Some(button) = crate::ui::skirmish_shell::choose_map_modal_button_at(&layout, x, y) {
            modal.pressed_button = Some(button);
            Self::play_main_menu_button_sound(state);
            return true;
        }
        if modal.handle_listbox_mouse_down(
            &layout,
            &state.skirmish_modes,
            &state.skirmish_scenario_records,
            x,
            y,
        ) {
            return true;
        }
        layout.dialog.contains(x, y)
    }
```
(NLL note: `modal`'s mutable borrow of `state.skirmish_shell_state` and the `&state.skirmish_modes` /
`&state.skirmish_scenario_records` shared borrows are disjoint fields — compiles. The button branch's last use
of `modal` is before `play_main_menu_button_sound(state)`, so the `&mut state` reborrow is fine, exactly as
the pre-migration code.)

**Step 2:** Delete the now-migrated static `handle_choose_map_listbox_scrollbar_mouse_down`
(`app.rs:1125-1173`) entirely. First confirm it has NO callers other than the one just removed:
```
rg -n "handle_choose_map_listbox_scrollbar_mouse_down" src/
```
Expect zero matches after the Step-1 edit. If any remain, STOP and reassess.

**Step 3:** Replace the body of `handle_choose_map_modal_mouse_wheel` (`app.rs:1175-1207`) with the delegating
wrapper (the None case stays here):
```rust
    fn handle_choose_map_modal_mouse_wheel(state: &mut AppState, lines: f32) -> bool {
        let layout = Self::skirmish_choose_map_layout(state);
        let x = state.cursor_x.round() as i32;
        let y = state.cursor_y.round() as i32;
        let Some(modal) = state.skirmish_shell_state.choose_map_modal.as_mut() else {
            return false;
        };
        modal.handle_listbox_wheel(&layout, &state.skirmish_modes, x, y, lines)
    }
```

**Step 4 (verify):** `cargo check -p vera20k` — clean. Confirm the dispatch callers (`app.rs:1410/1462/1525`,
re-verify by content) are unchanged: each still calls the wrapper exactly once.

### Task 10: NEW input tests in `choose_map.rs`

**Why:** Pin the four wheel branches + scrollbar dispatch + last-partial-row boundary (parity items 4D.2 T8).
These live in `choose_map.rs` (non-frozen), NOT `state/tests.rs`.

**Files:** Modify `src/ui/skirmish_shell/state/choose_map.rs` (add `#[cfg(test)] mod tests` at end of file).

**Step 1:** Add the test module. Fixtures use the public `stock_skirmish_modes()` +
`compute_choose_map_modal_layout(800, 600)`; map rows are set by assigning `filtered_record_indices` directly
(avoids needing scenario-record fixtures):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::skirmish_modes::stock_skirmish_modes;
    use super::super::layout::compute_choose_map_modal_layout;

    fn modal_with_map_rows(n: usize, modes: &[SkirmishGameMode]) -> ChooseMapModalState {
        let mut modal = ChooseMapModalState::open(modes[0].id, None, modes, &[]);
        modal.filtered_record_indices = (0..n).collect();
        modal.highlighted_filtered_index = (n > 0).then_some(0);
        modal.map_top_index = 0;
        modal
    }

    #[test]
    fn wheel_over_map_list_scrolls_down_on_negative_lines() {
        // lines<0 → +ceil(|lines|) rows; cursor over map_list routes to the map list.
        let modes = stock_skirmish_modes();
        let layout = compute_choose_map_modal_layout(800, 600);
        let mut modal = modal_with_map_rows(20, &modes);
        let (x, y) = (layout.map_list.x + 1, layout.map_list.y + 1);
        assert!(modal.handle_listbox_wheel(&layout, &modes, x, y, -1.5));
        assert_eq!(modal.map_top_index, 2, "ceil(1.5)=2 rows down");
        assert_eq!(modal.mode_top_index, 0, "map wheel leaves mode list alone");
    }

    #[test]
    fn wheel_positive_lines_scroll_up_and_clamp_at_zero() {
        let modes = stock_skirmish_modes();
        let layout = compute_choose_map_modal_layout(800, 600);
        let mut modal = modal_with_map_rows(20, &modes);
        let (x, y) = (layout.map_list.x + 1, layout.map_list.y + 1);
        modal.map_top_index = 5;
        assert!(modal.handle_listbox_wheel(&layout, &modes, x, y, 2.0));
        assert_eq!(modal.map_top_index, 3, "ceil(2.0)=2 rows up");
    }

    #[test]
    fn wheel_zero_lines_consumes_without_scrolling() {
        let modes = stock_skirmish_modes();
        let layout = compute_choose_map_modal_layout(800, 600);
        let mut modal = modal_with_map_rows(20, &modes);
        modal.map_top_index = 4;
        let (x, y) = (layout.map_list.x + 1, layout.map_list.y + 1);
        assert!(modal.handle_listbox_wheel(&layout, &modes, x, y, 0.0));
        assert_eq!(modal.map_top_index, 4, "zero lines: consumed, no scroll");
    }

    #[test]
    fn wheel_over_mode_list_leaves_map_index_untouched() {
        // Routing branch: cursor over mode_list must NOT scroll the map list.
        let modes = stock_skirmish_modes();
        let layout = compute_choose_map_modal_layout(800, 600);
        let mut modal = modal_with_map_rows(20, &modes);
        modal.map_top_index = 4;
        let (x, y) = (layout.mode_list.x + 1, layout.mode_list.y + 1);
        assert!(modal.handle_listbox_wheel(&layout, &modes, x, y, -2.0));
        assert_eq!(modal.map_top_index, 4, "mode wheel does not move the map list");
    }

    #[test]
    fn wheel_outside_both_lists_consumes_without_scrolling() {
        let modes = stock_skirmish_modes();
        let layout = compute_choose_map_modal_layout(800, 600);
        let mut modal = modal_with_map_rows(20, &modes);
        modal.map_top_index = 4;
        // (0,0) is the screen corner, outside both listbox rects.
        assert!(modal.handle_listbox_wheel(&layout, &modes, 0, 0, -3.0));
        assert_eq!(modal.map_top_index, 4, "outside lists: consumed, no scroll");
        assert_eq!(modal.mode_top_index, 0);
    }

    #[test]
    fn mouse_down_scrollbar_down_arrow_steps_map_top_index() {
        // Click the map listbox's down-arrow button → +1 row.
        let modes = stock_skirmish_modes();
        let layout = compute_choose_map_modal_layout(800, 600);
        let mut modal = modal_with_map_rows(40, &modes);
        let map = layout.map_list;
        let scrollbar = crate::ui::skirmish_shell::choose_map_listbox_scrollbar_rect(40, map)
            .expect("scrollbar present with 40 rows");
        let x = scrollbar.x + 1;
        let y = scrollbar.y + scrollbar.h - 1; // inside the bottom arrow button
        assert!(modal.handle_listbox_mouse_down(&layout, &modes, &[], x, y));
        assert_eq!(modal.map_top_index, 1);
    }

    #[test]
    fn mouse_down_map_row_selects_filtered_row() {
        let modes = stock_skirmish_modes();
        let layout = compute_choose_map_modal_layout(800, 600);
        let mut modal = modal_with_map_rows(20, &modes);
        modal.highlighted_filtered_index = None;
        let content = crate::ui::skirmish_shell::choose_map_listbox_content_rect(20, layout.map_list);
        // Second visible row (row index 1) at top_index 0.
        let (x, y) = (content.x + 1, content.y + 1 * crate::ui::skirmish_shell::CHOOSE_MAP_LISTBOX_ROW_H + 1);
        assert!(modal.handle_listbox_mouse_down(&layout, &modes, &[], x, y));
        assert_eq!(modal.highlighted_filtered_index, Some(1));
    }
}
```
Confirm the exact paths/names of `choose_map_listbox_scrollbar_rect`, `choose_map_listbox_content_rect`,
`CHOOSE_MAP_LISTBOX_ROW_H`, and `compute_choose_map_modal_layout` re-exports when editing (they may live at
the `crate::ui::skirmish_shell` root rather than `layout`); adjust `use`/paths if a name doesn't resolve. If
`compute_choose_map_modal_layout(800, 600)` yields a `map_list` with fewer than ~4 visible rows (so
`map_top_index` can't reach 2/3), bump the screen size or read `choose_map_listbox_visible_row_count(map_list)`
and adjust the expected deltas — keep the assertions tied to computed visible-row counts, not magic numbers.

**Step 2 (verify):** `cargo test -p vera20k --lib ui::skirmish_shell::state::choose_map` — all new tests
GREEN.

### Task 11: 4D.2 checkpoint + STOP, format, commit

**Step 1 (build):** `cargo build -p vera20k`.

**Step 2 (test, separate bounded pass):** run, reading each literal `test result:` line:
- `cargo test -p vera20k --lib ui::skirmish_shell::state` (incl. the frozen `tests` module **87** + new
  `choose_map` tests).
- `cargo test -p vera20k --lib ui::skirmish_shell::layout` → **30**.
- `cargo test -p vera20k --lib app_skirmish_shell_render` → `app_skirmish_shell_render.rs`=**53** unchanged.

**Step 3 (frozen-diff gate):** `git diff HEAD -- src/ui/skirmish_shell/state/tests.rs src/ui/skirmish_shell/layout.rs`
must be **EMPTY**. If ANY check fails: hard-revert this commit and STOP.

**Step 4 (no-double-handle inspection):** confirm `app.rs:1410/1462/1525` each invoke the modal wrapper once,
and the wrappers call `modal.handle_listbox_*` once — a single consumed bool per event.

**Step 5 (format):** `rustfmt --edition 2024 --check` each edited file; hand-apply ONLY to your regions.

**Step 6 (commit ONLY the 2 input files):**
```
git add src/ui/skirmish_shell/state/choose_map.rs src/app.rs
git commit -m "ui: Slice 4D.2 - migrate choose-map listbox scroll/wheel input into the ui layer"
```
Leave the parallel session's dirty tree untouched.

---

## Sim Checklist
N/A — 4D touches only `render/`, `app_skirmish_shell_render/`, `ui/skirmish_shell/`, and `app.rs`. No `sim/`
edit, no fixed-point math, no state-hash change, no tick-ordering impact.

## Risk Areas
- **Shared scrollbar emitter** (parity 4D.1 T3/T6): both the combo popup and the choose-map listbox depend on
  the deleted `push_dropdown_scrollbar_instances`. Guard: the arm reproduces it byte-for-byte (draw-list test)
  and both call sites route through the same arm; the listbox's own pre-fill track layer is preserved.
- **`_px` wrapper byte-identity** (parity 4D.1 T2): a wrong wrapper would silently shift every solid-rect /
  bevel in the shell. Guard: wrappers forward `atlas.white_pixel` with no value change; frozen suites + the
  draw-list test catch drift.
- **`app.rs` contention** (§7): re-verify each block by content immediately before editing; WAIT on another
  session's in-progress edits. The 4D.2 write window is two functions + one deletion — keep it minimal.
- **NLL borrow shape** in `handle_choose_map_modal_mouse_down`: the disjoint-field borrows compile only with
  the field-direct access shown; do not route `modes`/`records` through a `&self`/`&mut self` whole-`state`
  method.

## Sources & References
- **Design doc:** `docs/plans/2026-06-01-shell-substrate-slice4-plan.md` §4D (scope, wheel-branch contract,
  modal boundary), §1.1/§1.4 (input-stays-skirmish + draw-order invariant), §3/§6.2 (frozen-suite invariant +
  per-sub-step checkpoint), §7 (contended `app.rs`).
- **Sibling realization plan:** `docs/plans/2026-06-12-slice4c-combo-controlchrome-seam-plan.md` (shape; the
  "Deferred to 4D" list = this slice's paint scope).
- **Prior commits:** ea000965 (4B trackbar seam), 76a7fa56 (4C combo seam — added `white_pixel` to
  `ControlChrome`).
- **Current code (re-verify by content before edit):** `controls.rs:165-209` (shared scrollbar emitter,
  hardcoded depths), `controls.rs:136-163` (`push_scrollbar_thumb`), `controls.rs:517-583`
  (`push_dropdown_instances`), `modals.rs:35-81` (`push_choose_map_listbox_instances`), `chrome.rs:399-521`
  (fill helpers), `app.rs:1024-1207` (the migrating handlers), `app.rs:1410/1462/1525` (dispatch callers),
  `choose_map.rs` (`ChooseMapModalState` owner), `skirmish_shell_chrome.rs:96-121` (`ControlChrome` +
  `control_chrome()`).
- **INI:** none — 4D seeds no constants (the C14 seed is 4F).
