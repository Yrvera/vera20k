# Slice 4B — Trackbar onto the paint seam, via the `ControlChrome` Default-able subset

> Focused realization plan for sub-step 4B of `docs/plans/2026-06-01-shell-substrate-slice4-plan.md`.
> Decided seam shape (from yesterday's session): **atlas-taking** — `paint_control` resolves chrome
> internally. Blocker: the 53-field `SkirmishShellChromeAtlas` cannot `derive(Default)` (live
> `texture: BatchTexture` field, `skirmish_shell_chrome.rs:34`), so it isn't unit-testable. This plan
> realizes "atlas-taking" via a small **`ControlChrome`** Default-able subset carrying ONLY the
> control glyph `Option`s. This also lightly re-shapes 4A's checkbox arm to match.

## Scope (paint only)
- Route the 3 trackbars (GameSpeed/Credits/UnitCount) through `paint_control` (rail + 3-piece plaque + thumb).
- Re-shape the 4A `Checkbox` arm to the same atlas-taking shape (`checked: bool`, resolve icon inside).
- **No input change.** The trackbar input branch (`handle_option_mouse_*`, Y-gate, drag, x-clamp,
  `game_speed` stored↔visual inversion, HSCROLL/`GenericClick`) is untouched — 4B is the paint seam.
- Byte-identical emission order/depth (draw-list assertion).

## 1. New type — `ControlChrome` (in `src/render/skirmish_shell_chrome.rs`, next to the atlas)
```rust
/// Default-able subset of the chrome atlas carrying ONLY owner-draw control glyph
/// entries — NO GPU `texture` (that field is why the full atlas can't derive Default).
/// The Slice 4 paint seam (`paint_control`) takes this so it resolves chrome inside the
/// emitter yet stays unit-testable via `ControlChrome { trackbar_rail: Some(e), ..Default::default() }`.
/// Grows one control family per sub-step (4B trackbar; 4C combo; 4D scrollbar).
#[derive(Debug, Clone, Copy, Default)]
pub struct ControlChrome {
    pub checkbox_unchecked_cue_i: Option<SkirmishShellChromeEntry>,
    pub checkbox_checked_cce_i: Option<SkirmishShellChromeEntry>,
    pub trackbar_rail: Option<SkirmishShellChromeEntry>,
    pub trackbar_plaque_left_trofl: Option<SkirmishShellChromeEntry>,
    pub trackbar_plaque_mid_trofm: Option<SkirmishShellChromeEntry>,
    pub trackbar_plaque_right_trofr: Option<SkirmishShellChromeEntry>,
    pub trackbar_thumb_trakgrip: Option<SkirmishShellChromeEntry>,
}

impl SkirmishShellChromeAtlas {
    /// Snapshot the control glyph entries into a Default-able, texture-free subset
    /// the paint seam can take (and tests can build by hand). Entries are `Copy`.
    pub fn control_chrome(&self) -> ControlChrome {
        ControlChrome {
            checkbox_unchecked_cue_i: self.checkbox_unchecked_cue_i,
            checkbox_checked_cce_i: self.checkbox_checked_cce_i,
            trackbar_rail: self.trackbar_rail,
            trackbar_plaque_left_trofl: self.trackbar_plaque_left_trofl,
            trackbar_plaque_mid_trofm: self.trackbar_plaque_mid_trofm,
            trackbar_plaque_right_trofr: self.trackbar_plaque_right_trofr,
            trackbar_thumb_trakgrip: self.trackbar_thumb_trakgrip,
        }
    }
}
```
Field names mirror the atlas verbatim so the snapshot is a 1:1 copy (no renaming surface).

## 2. `ControlPaint` enum (controls.rs) — atlas-taking shape
```rust
pub(super) enum ControlPaint {
    Checkbox { checked: bool, rect: RectPx },
    Trackbar { rect: RectPx, thumb_px: i32 },
}
```
- `Checkbox` now carries `checked` (not a pre-resolved `icon`); the icon lookup moves inside.
- `Trackbar` carries the resolved pixel offset `thumb_px` (value→px quantization stays in the
  skirmish-layer caller, reading bounds; only px→rect positioning is in paint).

## 3. `paint_control` — takes `&ControlChrome`, resolves + emits
```rust
pub(super) fn paint_control(
    out: &mut Vec<SpriteInstance>,
    chrome: &ControlChrome,
    paint: ControlPaint,
) {
    match paint {
        ControlPaint::Checkbox { checked, rect } => {
            let icon = if checked { chrome.checkbox_checked_cce_i } else { chrome.checkbox_unchecked_cue_i };
            if let Some(entry) = icon {
                push_entry(out, entry, checkbox_icon_rect(rect), SHELL_CONTROL_DEPTH);
            }
        }
        ControlPaint::Trackbar { rect, thumb_px } => {
            // Emission ORDER preserved byte-for-byte: rail → plaque(mid, left, right) → thumb.
            if let Some(rail) = chrome.trackbar_rail {
                push_entry_native(out, rail, rect.x, rect.y, SHELL_CONTROL_DEPTH);
            }
            paint_trackbar_plaque(out, chrome, rect, SHELL_CONTROL_DEPTH);
            if let Some(thumb) = chrome.trackbar_thumb_trakgrip {
                push_entry(out, thumb, trackbar_thumb_rect(rect, thumb_px), SHELL_CONTROL_DEPTH - 0.00002);
            }
        }
    }
}
```

## 4. Refactor `push_trackbar_plaque` → `paint_trackbar_plaque(&ControlChrome)`
Identical body, `atlas.trackbar_plaque_*` → `chrome.trackbar_plaque_*`. Sole caller is now
`paint_control` (was `push_trackbar_instances:386`). Depth math unchanged: mid at `depth`,
left at `depth - 0.00001`, right at `depth - 0.00001`.

## 5. Caller rewrites (controls.rs)
- **`push_checkbox_instances`**: build `let chrome = atlas.control_chrome();` once; per checkbox:
  `paint_control(out, &chrome, ControlPaint::Checkbox { checked: checkbox_checked(shell, checkbox.id), rect: checkbox.rect })`.
  `checkbox_entry` helper becomes unused → delete it (its logic now lives inline in `paint_control`).
- **`push_trackbar_instances`**: build `chrome` once; per id keep `trackbar_rect_for_id`,
  `trackbar_visual_value`, `shell.trackbar_bounds.range(id)`, `trackbar_pixel_offset(...)` →
  `let px = ...; paint_control(out, &chrome, ControlPaint::Trackbar { rect, thumb_px: px });`.
  Drops the direct `atlas.trackbar_rail` / `push_trackbar_plaque` / `atlas.trackbar_thumb_trakgrip` block.
- Both keep `atlas: &SkirmishShellChromeAtlas` params (they call `atlas.control_chrome()`); signatures unchanged.

## 6. Tests
- **Edit** the 4A draw-list test `checkbox_paint_seam_emits_icon_at_icon_rect_with_control_depth`
  (controls.rs:662) to the new API: build `ControlChrome { checkbox_checked_cce_i: Some(entry), ..Default::default() }`,
  call `paint_control(&mut out, &chrome, ControlPaint::Checkbox { checked: true, rect })`. **Assertions
  unchanged** (1 instance, icon rect, uv, `SHELL_CONTROL_DEPTH`); the empty case becomes
  `ControlChrome::default()` (no entries) → 0 instances. This is OUR 4A test, not a frozen-suite test.
- **NEW** `trackbar_paint_seam_emits_rail_plaque_thumb_in_native_order` (controls.rs): build a
  `ControlChrome` with rail+plaque×3+thumb `Some(_)`; for a fixed rect at min/mid/max `thumb_px`,
  assert the emitted `SpriteInstance` sequence — count, per-instance uv/position/depth — matches the
  pre-seam `push_trackbar_instances` output (rail@DEPTH, mid@DEPTH, left@DEPTH-1e-5, right@DEPTH-1e-5,
  thumb@DEPTH-2e-5, thumb position = `trackbar_thumb_rect(rect, px)`). Also assert missing entries
  (`ControlChrome::default()`) emit nothing.

## 7. Checkpoint + STOP (per plan §6.2)
`cargo build -p vera20k` && `cargo test -p vera20k` (separate bounded pass). Read the literal
`test result:` line. Confirm `state/tests.rs`=87, `layout.rs`=30, `app_skirmish_shell_render.rs`=53
unchanged AND `git diff HEAD -- src/ui/skirmish_shell/state/tests.rs src/ui/skirmish_shell/layout.rs`
is EMPTY. If any check fails, hard-revert this commit and STOP. Commit to `dev` as one isolated 4B commit.

## Files touched
- `src/render/skirmish_shell_chrome.rs` (new `ControlChrome` + `control_chrome()`)
- `src/app_skirmish_shell_render/controls.rs` (enum, `paint_control`, `paint_trackbar_plaque`, two callers, tests)

## Out of scope (unchanged from plan §5.7)
Trackbar input math, bound seeding from MinMoney/MaxMoney (O4, stays hardcoded), combo/listbox arms
(4C/4D), scroll unification (4E). `ControlChrome` grows its combo/scrollbar fields in 4C/4D.
