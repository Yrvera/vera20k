# Slice 4E — Unify the two skirmish scroll models into ONE parameterized primitive

> Focused realization plan for sub-step 4E of `docs/plans/2026-06-01-shell-substrate-slice4-plan.md`
> (§4E + §2 "Differences that MUST stay parameterized"), gated behind 4C (76a7fa56) AND 4D
> (946e7e1a/097fe6e6), both committed at HEAD. Shape mirrors
> `docs/plans/2026-06-12-slice4c-combo-controlchrome-seam-plan.md` and `…-slice4d-…-plan.md`.
> **Single green-gated commit.**

**Goal:** Collapse the combo-dropdown scroll math (Model A, `state/combos.rs:142-257`) and the
choose-map listbox scroll math (Model B, `layout.rs:644-693`) into one pure `ScrollModel` primitive
whose SIX observable divergences stay explicit parameters, proven bit-for-bit equal to BOTH legacy
impls over a boundary domain BEFORE either inline copy is removed.

**Architecture:** A new pure-geometry module `src/ui/skirmish_shell/scroll.rs` (depends only on
`RectPx` + the scrollbar constants from `layout`; no state/render/ui dependency) owns the shared
thumb-height / thumb-position / pointer→top_index math. `combos.rs` and `layout.rs` keep every
existing public signature and delegate their function BODIES to the primitive — no caller, no
descriptor, no paint seam, no `DialogController`, no test assertion changes. This is a
consolidation slice: the burden-of-proof bar (default DRIFT) is met by an executable
equivalence proof + the frozen 87+30 suite staying GREEN.

**Design Doc:** `docs/plans/2026-06-01-shell-substrate-slice4-plan.md` §4E (scope, six axes,
4-step equivalence proof), §2 (the DRIFT-to-preserve list), §3/§6.2 (frozen-suite invariant +
checkpoint), §7 (parallel safety). No standalone `-design.md` — 4E's spec is §4E of the master
plan, exactly as 4C/4D were derived from §4C/§4D.

---

## Grounding Summary

- **Spec = the in-repo committed behavior, pinned by the frozen suites.** Not a gamemd
  re-derivation: 4E touches the math, so the master plan gates it behind an explicit
  equivalence PROOF (§4E steps 1–4). No new Ghidra/INI research (4A–4D settled the architecture;
  4F owns the seed + O5 widget Ghidra pre-req). No INI key drives the scroll math.
- **The two models already share the same constants.** `COMBO_DROPDOWN_SCROLLBAR_BUTTON_H = 22`
  and `COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H = 14` (`layout.rs:26-27`); Model B's
  `choose_map_listbox_scroll_thumb_rect` (`layout.rs:658-660`) already reuses BOTH directly. So the
  constants are NOT a divergence — only the math wrappers are duplicated.
- **The non-empty thumb-height formula is textually identical** in both models
  (`((track_h * visible) / count).max(MIN_THUMB_H).min(track_h)` with
  `track_h = (scrollbar_h − BUTTON_H*2).max(1)`). **The track-click pointer→index formula is
  character-for-character identical** (`combos.rs:250-256` ≡ `layout.rs:686-692`). **Model A's
  thumb-drag core (`combos.rs:228-234`) is the same formula** as track-click with a different
  anchor (`mouse_y − grab_offset_y` vs `mouse_y − thumb.h/2`). So track-click and drag unify into ONE
  `top_index_from_thumb_top(…, thumb_top_candidate)`.
- **Repo pattern to mirror:** the 4D pattern — keep the existing public function signatures, move
  the BODY to a shared helper, delegate. `ChooseMapModalState` did exactly this for input
  (`choose_map.rs`); 4E does it for scroll geometry.
- **Still-unknown after grounding (→ deferred to the proof, not assumed):** whether the two
  models' divergent EMPTY-thumb paths (A `track_h.max(MIN_THUMB_H)` vs B `None`) ever produce
  observably different output. The proof's Step-1 reachability analysis resolves this; my
  inline reading (below) is that A's empty trigger is unreachable under its scrollbar gate and B's
  reachable degenerate case must stay `None` — the proof test makes this executable.

## Key Technical Decisions

- **Primitive lives in a new `src/ui/skirmish_shell/scroll.rs`, not in `layout.rs`** — keeps the
  pure scroll math in one cohesive file and avoids growing `layout.rs` (already ~1300 lines incl.
  tests) further. — **Confidence:** high. **Source:** repo `~600 lines/file` convention (CLAUDE.md);
  module tree in `mod.rs:7-9`.
- **Three of the six axes drive the primitive's computed values** (`row_h` + `visible_row_source`
  via `visible_rows()`, `empty_path` via `thumb_height()`); the other three (`thumb_drag_enabled`,
  `wheel_active`, cursor storage) are *caller* behavior — the combo wires the drag path + treats the
  wheel as inert + stores its cursor fused in `Option<OpenComboDropdown>`; the listbox skips drag +
  handles the wheel + stores two bare `usize`. All six are RECORDED on `ScrollModel` (per the spec's
  struct shape) so one type documents the full divergence set; `thumb_drag_enabled`/`wheel_active`
  change no value the primitive computes (honored by the call sites; read by the derived
  `Debug`/`PartialEq`). — **Confidence:** high (traced both call sites).
  - **For `row_h` + `visible_row_source` to be load-bearing in production (not just in the proof
    test), the existing `*_visible_row_count` and `*_max_top_index` helpers delegate to
    `model.visible_rows()` / `model.max_top_index()`** — see Task 3 Steps 6–7 and Task 4 Steps 4–5.
    Without this, those two methods are production-dead (a `dead_code` warning) and the
    `visible_row_source` axis is decorative. Each delegation is bit-identical (verified during
    /review-plan).
- **`thumb_height` returns `Option<i32>`**, unifying A (always a value) and B (`None` on empty) via
  the `empty_path` param. The empty branch condition is `item_count == 0 || visible_rows == 0`:
  exact for B (NoThumb, incl. the reachable `visible_rows==0` case); a dead-but-harmless superset
  for A (MaxThumb, unreachable under the gate — proven). — **Confidence:** high. **Source:**
  `combos.rs:142-150`, `layout.rs:655-661`.
- **`track_span` uses Model A's form** `(scrollbar.h − BUTTON_H*2 − thumb_h).max(1)` everywhere
  (thumb_y + pointer→index). Provably equal to B's `((scrollbar.h − BUTTON_H*2).max(1) − thumb_h).max(1)`
  for all `thumb_h ≥ 1` (a real thumb is always ≥ MIN_THUMB_H = 14); the proof exercises the
  degenerate `scrollbar.h ≤ 44` cases where the clamp engages. — **Confidence:** high (hand-proof
  below + executable boundary test). **Source:** `combos.rs:168/250`, `layout.rs:663/686`.
- **Public signatures of all delegating functions stay byte-identical** so the frozen 87+30 suite
  + the `choose_map.rs` input tests stay GREEN with zero edits. — **Confidence:** high. **Source:**
  frozen callers in `state/tests.rs:1455/1490/1557/211`, `layout.rs:1273/1254`.

## Open Questions

### Resolved During Planning
- *Are A's and B's empty-thumb paths a real observable divergence?* No (pending the proof's Step-1
  executable confirmation): A's `item_count==0` empty trigger is unreachable because the scrollbar
  gate `needs_scrollbar = item_count > visible_rows` is false when `item_count==0` (and for a combo
  `visible_rows==0 ⟺ item_count==0`), so `combo_dropdown_thumb_height` is never reached with
  `item_count==0`. B's `visible_rows==0 && row_count≥1` (a list rect shorter than one row) IS
  reachable and must keep returning `None`. The two paths are preserved as the `empty_path` param and
  the proof asserts the reachable behavior.
- *Where does the new proof test go?* In a new `#[cfg(test)] mod tests` in the new `scroll.rs` — it
  holds verbatim reference copies of both legacy formulas (the executable pre-4E spec) so the proof
  stays meaningful AFTER the production bodies are rewired to delegate. It does NOT touch the frozen
  `state/tests.rs` (87) or `layout.rs` (30) modules.
- *Does 4E edit `layout.rs` production code?* Yes — §4E's own Files list names `layout.rs` and says
  "DELETE the duplicated … copies (… layout.rs:648-693)". See the frozen-diff note below for how the
  §6.2 "layout.rs diff EMPTY" gate is scoped for THIS sub-step.

### Deferred to Implementation
- The exact boundary-domain bounds (`N` for the count loops, the degenerate `scrollbar_h` set) are
  fixed in Task 2's code below; if a real combo dropdown can exceed `item_count = 24` in stock YR
  (e.g. a Color combo on a many-slot map), bump `N` to cover it — keep the loop tied to the model's
  own visible-row source, not magic numbers.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/ui/skirmish_shell/scroll.rs` | `ScrollModel` struct + pure thumb/track/drag/max_top geometry; the equivalence-proof test module |
| Modify | `src/ui/skirmish_shell/mod.rs` | `mod scroll;` declaration |
| Modify | `src/ui/skirmish_shell/state/combos.rs` | Delete inline scroll math; delegate `combo_dropdown_scroll_thumb_rect`, `top_index_from_thumb_y`, `top_index_from_scrollbar_track_click`, `combo_dropdown_visible_row_count`, `combo_dropdown_max_top_index` to `ScrollModel::combo`; remove private `combo_dropdown_thumb_height` |
| Modify | `src/ui/skirmish_shell/layout.rs` | Delete inline scroll math; delegate `choose_map_listbox_scroll_thumb_rect`, `choose_map_listbox_top_index_from_track_click`, `choose_map_listbox_visible_row_count`, `choose_map_listbox_max_top_index` to `ScrollModel::listbox` (the `#[cfg(test)] mod tests` block is UNTOUCHED) |

`state/choose_map.rs` is NOT edited — its scroll input methods already call the layout helpers, which
keep their signatures. (It is listed in §4E's Files only as the cursor-storage owner; no body change
is needed since the cursor accessor stays caller-side.)

## Interface Changes

- **New `pub` items in the private `mod scroll`:** `struct ScrollModel`, `enum VisibleRowSource`,
  `enum EmptyThumbPath`, ctors `ScrollModel::combo(cap: i32)` / `ScrollModel::listbox()`, and methods
  `visible_rows`, `max_top_index`, `thumb_height`, `thumb_y`, `top_index_from_thumb_top`. The module
  is `mod scroll;` (private to `skirmish_shell`), so nothing leaks outside the shell; descendants
  (`state::combos`, `layout`) reach it via `super::super::scroll` / `super::scroll`.
- **No change** to any existing public signature in `combos.rs` / `layout.rs` / `choose_map.rs`. No
  re-export added to `mod.rs`'s `pub use` lists (the primitive is internal).

## Sim Checklist
N/A — 4E touches only `ui/skirmish_shell/`. No `sim/` edit, no fixed-point math (this is integer
UI-pixel geometry, `i32`/`usize` — consistent with the existing scroll code), no state-hash change,
no tick-ordering impact, no `EntityStore` iteration.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| T1/T2 | `thumb_height` non-empty formula + `MIN_THUMB_H`/`track_h` clamps | The scrollbar thumb size is on screen every time a combo opens or Choose-Map shows >visible rows; a 1px thumb-size drift is visible | Proof test asserts unified == both legacy over the boundary domain |
| T1/T2 | `thumb_y` position (`max_top==0` guard, integer `(track_span*top)/max_top`) | Thumb vertical position per scroll offset; off-by-one is a visible jump | Proof test over `top_index ∈ 0..=max_top` for both sources |
| T1/T2 | `top_index_from_thumb_top` rounding (`(local*max_top + track_span/2)/track_span`) + clamp | Track-click AND thumb-drag landing row; a rounding drift lands the wrong top row | Proof test over `mouse_y ∈ [sb.y−5 .. sb.y+h+5]` (incl. out-of-range clamp), track + drag anchors |
| T1/T2 | `track_span` form equality (A-form vs B-form at degenerate `scrollbar_h ≤ 44`) | The clamp boundary is where the two legacy forms could diverge | Proof test includes `scrollbar_h ∈ {44,45,46}` |
| T2 | Empty-path reachability (A `MaxThumb` dead under gate; B `NoThumb` reachable → `None`) | A wrong unification could draw a full-track thumb on a sub-row-height listbox, or no thumb on a combo | `empty_paths_are_reachability_gated` test |
| T3/T4 | Frozen scroll pins stay GREEN-unchanged | The 87+30 suite is the live regression net for observable scroll behavior | `state/tests.rs`=87, `layout.rs`=30 GREEN; tests.rs diff EMPTY |

---

## The hand-proof `track_span_A ≡ track_span_B` (load-bearing; the proof test makes it executable)

Let `t = scrollbar.h − BUTTON_H*2` and `h = thumb_h` (a real thumb ⇒ `h ≥ MIN_THUMB_H = 14 > 0`).
- A's form: `track_span_A = (t − h).max(1)`.
- B's form: `track_span_B = (t.max(1) − h).max(1)`.

Case `t ≥ 1`: `t.max(1) = t` ⇒ `track_span_B = (t − h).max(1) = track_span_A`. ✓
Case `t ≤ 0`: `t − h ≤ −14 < 1` ⇒ `track_span_A = 1`; and `t.max(1) = 1` ⇒ `t.max(1) − h = 1 − h ≤ −13 < 1`
⇒ `track_span_B = 1 = track_span_A`. ✓

So the unified primitive may use A's form unconditionally and remain bit-identical to B for every
`thumb_h ≥ 1`. Task 2 exercises `scrollbar_h ∈ {44,45,46,…}` so the `t ≤ 0` / `t = 1` boundaries are
covered by an executable assertion, not algebra alone.

---

## Tasks (single commit: `ui: Slice 4E - unify the two skirmish scroll models into one ScrollModel primitive`)

### Task 1: Create the `ScrollModel` primitive in `src/ui/skirmish_shell/scroll.rs`

**Why:** Define the single source of truth for the shared scroll math BEFORE rewiring either model.
This task adds new code only — no existing behavior changes, nothing delegates yet.

**Files:**
- Create: `src/ui/skirmish_shell/scroll.rs`
- Modify: `src/ui/skirmish_shell/mod.rs` (add `mod scroll;`)

**Pattern:** New pure-geometry module; mirrors `layout.rs`'s "free functions over `RectPx` +
constants" style (no state, no render).

**Step 1: Declare the module.** In `src/ui/skirmish_shell/mod.rs`, add after `mod layout;`
(line 7):
```rust
mod scroll;
```
(Private — internal to the shell tree, reachable by `state::combos` and `layout`. No `pub use`
needed; 4E adds no externally-visible API.)

**Step 2: Write the primitive.** Create `src/ui/skirmish_shell/scroll.rs`:
```rust
//! Unified scrollbar geometry for the skirmish shell's two scrollable lists — the
//! combo dropdown (Model A) and the choose-map listbox (Model B). Both lists share
//! identical thumb-height, thumb-position, and pointer→top_index math; the points
//! where they legitimately differ — row height, how the visible-row count is
//! derived, and what an empty list does to the thumb — are explicit parameters on
//! `ScrollModel`. The other three model differences (drag, wheel, cursor storage)
//! are caller behavior and are recorded here only for documentation; they change no
//! value this module computes.
//!
//! Depends only on `RectPx` and the scrollbar constants from `layout`; holds no
//! state, render, or UI dependency (pure integer pixel geometry).

use super::layout::{
    CHOOSE_MAP_LISTBOX_ROW_H, COMBO_DROPDOWN_ROW_H, COMBO_DROPDOWN_SCROLLBAR_BUTTON_H,
    COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H, RectPx,
};

/// Where a model derives its visible-row count. Combo dropdowns cap visible rows
/// per control (Side = 7, Color/Start = 9, AiType/Team = unbounded); the choose-map
/// listbox derives them geometrically from its rect height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibleRowSource {
    /// `item_count.min(cap)`, or unbounded (`item_count`) when `cap == 0`.
    PerControlCap(i32),
    /// `(rect_h / row_h).max(0)`.
    GeometricFromRect,
}

/// What `thumb_height` returns for an empty/degenerate list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyThumbPath {
    /// `track_h.max(MIN_THUMB_H)` — combo Model A (unreachable under its scrollbar gate).
    MaxThumb,
    /// `None` — listbox Model B (reachable when the rect is shorter than one row).
    NoThumb,
}

/// The six divergence axes between the two scroll models, parameterized. The first
/// three drive the geometry below; `thumb_drag_enabled`, `wheel_active`, and the
/// cursor storage (which lives in the caller, not here) are honored by the call
/// sites — the combo wires drag + treats the wheel as inert + stores a fused
/// `Option<OpenComboDropdown>`; the listbox skips drag + handles the wheel + stores
/// two bare `usize`. They are recorded so one struct documents all six.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollModel {
    pub row_h: i32,
    pub visible_row_source: VisibleRowSource,
    pub thumb_drag_enabled: bool,
    pub wheel_active: bool,
    pub empty_path: EmptyThumbPath,
}

impl ScrollModel {
    /// The combo dropdown (Model A): per-control cap, drag on, wheel inert,
    /// full-track thumb on empty. `cap` is the per-combo max-visible (0 = unbounded).
    pub const fn combo(cap: i32) -> Self {
        Self {
            row_h: COMBO_DROPDOWN_ROW_H,
            visible_row_source: VisibleRowSource::PerControlCap(cap),
            thumb_drag_enabled: true,
            wheel_active: false,
            empty_path: EmptyThumbPath::MaxThumb,
        }
    }

    /// The choose-map listbox (Model B): geometric visible rows, no drag, wheel
    /// active, no thumb on empty.
    pub const fn listbox() -> Self {
        Self {
            row_h: CHOOSE_MAP_LISTBOX_ROW_H,
            visible_row_source: VisibleRowSource::GeometricFromRect,
            thumb_drag_enabled: false,
            wheel_active: true,
            empty_path: EmptyThumbPath::NoThumb,
        }
    }

    /// Visible-row count. `PerControlCap(0)` is unbounded (`item_count`); the
    /// geometric source ignores `item_count`.
    pub fn visible_rows(&self, item_count: usize, rect_h: i32) -> usize {
        match self.visible_row_source {
            VisibleRowSource::PerControlCap(cap) => {
                if cap > 0 {
                    item_count.min(cap as usize)
                } else {
                    item_count
                }
            }
            VisibleRowSource::GeometricFromRect => (rect_h / self.row_h).max(0) as usize,
        }
    }

    /// `item_count − visible_rows`, saturating. Matches both legacy `max_top`.
    pub fn max_top_index(&self, item_count: usize, visible_rows: usize) -> usize {
        item_count.saturating_sub(visible_rows)
    }

    /// Thumb height in pixels, or `None` when the list is empty/degenerate under
    /// `empty_path`. `scrollbar_h` is the full track (scrollbar rect) height.
    pub fn thumb_height(
        &self,
        visible_rows: usize,
        item_count: usize,
        scrollbar_h: i32,
    ) -> Option<i32> {
        let track_h = (scrollbar_h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H * 2).max(1);
        if item_count == 0 || visible_rows == 0 {
            return match self.empty_path {
                EmptyThumbPath::MaxThumb => {
                    Some(track_h.max(COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H))
                }
                EmptyThumbPath::NoThumb => None,
            };
        }
        Some(
            ((track_h * visible_rows as i32) / item_count as i32)
                .max(COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H)
                .min(track_h),
        )
    }

    /// Thumb top-Y inside `scrollbar` for `top_index`. `thumb_h` from `thumb_height`;
    /// `max_top` from `max_top_index`.
    pub fn thumb_y(&self, scrollbar: RectPx, thumb_h: i32, top_index: usize, max_top: usize) -> i32 {
        let track_span = (scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H * 2 - thumb_h).max(1);
        scrollbar.y
            + COMBO_DROPDOWN_SCROLLBAR_BUTTON_H
            + if max_top == 0 {
                0
            } else {
                (track_span * top_index.min(max_top) as i32) / max_top as i32
            }
    }

    /// Pointer→top_index — the shared core for BOTH a track click and a thumb drag.
    /// `thumb_top_candidate` is `mouse_y − thumb_h/2` for a track click, or
    /// `mouse_y − grab_offset_y` for a drag.
    pub fn top_index_from_thumb_top(
        &self,
        scrollbar: RectPx,
        thumb_h: i32,
        max_top: usize,
        thumb_top_candidate: i32,
    ) -> usize {
        if max_top == 0 {
            return 0;
        }
        let track_span = (scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H * 2 - thumb_h).max(1);
        let thumb_top = thumb_top_candidate.clamp(
            scrollbar.y + COMBO_DROPDOWN_SCROLLBAR_BUTTON_H,
            scrollbar.y + scrollbar.h - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H - thumb_h,
        );
        let local = thumb_top - scrollbar.y - COMBO_DROPDOWN_SCROLLBAR_BUTTON_H;
        ((local * max_top as i32 + track_span / 2) / track_span) as usize
    }
}
```

**Step 3: Verify.** `cargo check -p vera20k` — clean (new module compiles; nothing uses it yet, so
expect dead-code warnings on the new items, which Task 3/4 resolve by wiring them in).

### Task 2: Write the equivalence PROOF test (the GATE — must be GREEN before any rewire)

**Why:** §4E's mandatory equivalence proof. Holds VERBATIM reference copies of both pre-4E formulas
(the executable spec) and asserts the primitive equals them bit-for-bit over the boundary domain,
INCLUDING the empty-path reachability. Because the reference copies live in the test, the proof stays
meaningful after Tasks 3/4 rewire the production bodies to delegate (delegation can't make the proof
tautological).

**Files:** Modify `src/ui/skirmish_shell/scroll.rs` (append a `#[cfg(test)] mod tests`).

**Pattern:** Boundary-enumeration proof; the reference copies are the legacy math transcribed
verbatim from `combos.rs:142-256` and `layout.rs:655-692`.

**Step 1:** Append to `scroll.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use super::super::layout::{
        COMBO_DROPDOWN_SCROLLBAR_BUTTON_H as BUTTON_H,
        COMBO_DROPDOWN_SCROLLBAR_MIN_THUMB_H as MIN_THUMB_H,
    };

    // ---- Verbatim reference copies of the pre-4E legacy math (the executable spec).
    //      These NEVER change; the unified primitive is proven equal to them. ----

    /// Model A: `combo_dropdown_thumb_height` (combos.rs pre-4E).
    fn legacy_combo_thumb_height(visible_rows: usize, item_count: usize, scrollbar_h: i32) -> i32 {
        let track_h = (scrollbar_h - BUTTON_H * 2).max(1);
        if item_count == 0 {
            return track_h.max(MIN_THUMB_H);
        }
        ((track_h * visible_rows as i32) / item_count as i32)
            .max(MIN_THUMB_H)
            .min(track_h)
    }

    /// Model B: thumb-height portion of `choose_map_listbox_scroll_thumb_rect` (layout.rs pre-4E).
    fn legacy_listbox_thumb_height(
        visible_rows: usize,
        row_count: usize,
        scrollbar_h: i32,
    ) -> Option<i32> {
        if row_count == 0 || visible_rows == 0 {
            return None;
        }
        let track_h = (scrollbar_h - BUTTON_H * 2).max(1);
        Some(
            ((track_h * visible_rows as i32) / row_count as i32)
                .max(MIN_THUMB_H)
                .min(track_h),
        )
    }

    /// Model A `thumb_y` (combo_dropdown_scroll_thumb_rect pre-4E).
    fn legacy_combo_thumb_y(scrollbar: RectPx, thumb_h: i32, top_index: usize, max_top: usize) -> i32 {
        let track_span = (scrollbar.h - BUTTON_H * 2 - thumb_h).max(1);
        scrollbar.y
            + BUTTON_H
            + if max_top == 0 {
                0
            } else {
                (track_span * top_index.min(max_top) as i32) / max_top as i32
            }
    }

    /// Model B `thumb_y` (choose_map_listbox_scroll_thumb_rect pre-4E) — track_h-based form.
    fn legacy_listbox_thumb_y(
        scrollbar: RectPx,
        thumb_h: i32,
        top_index: usize,
        max_top: usize,
    ) -> i32 {
        let track_h = (scrollbar.h - BUTTON_H * 2).max(1);
        let track_span = (track_h - thumb_h).max(1);
        scrollbar.y
            + BUTTON_H
            + if max_top == 0 {
                0
            } else {
                (track_span * top_index.min(max_top) as i32) / max_top as i32
            }
    }

    /// Shared pointer→top_index core (identical in A track-click, A drag, B track-click pre-4E).
    fn legacy_pointer_to_top(scrollbar: RectPx, thumb_h: i32, max_top: usize, candidate: i32) -> usize {
        if max_top == 0 {
            return 0;
        }
        let track_span = (scrollbar.h - BUTTON_H * 2 - thumb_h).max(1);
        let thumb_top = candidate.clamp(
            scrollbar.y + BUTTON_H,
            scrollbar.y + scrollbar.h - BUTTON_H - thumb_h,
        );
        let local = thumb_top - scrollbar.y - BUTTON_H;
        ((local * max_top as i32 + track_span / 2) / track_span) as usize
    }

    /// Representative scrollbars incl. degenerate `track_h`-clamp geometries.
    fn scrollbars() -> Vec<RectPx> {
        vec![
            RectPx::new(513, 127, 20, 343),    // the frozen choose-map listbox geometry
            RectPx::new(100, 50, 20, 23 * 7),  // a Side combo dropdown (cap 7, row 23)
            RectPx::new(100, 50, 20, 23 * 9),  // Color/Start dropdown (cap 9)
            RectPx::new(0, 0, 20, 44),         // degenerate: scrollbar.h − 44 == 0  → track_h clamp
            RectPx::new(0, 0, 20, 45),         // degenerate: track_h == 1
            RectPx::new(0, 0, 20, 46),
        ]
    }

    const N: usize = 24; // boundary count ceiling; bump if a stock combo can exceed it

    #[test]
    fn unbounded_combo_never_needs_a_scrollbar() {
        // PerControlCap(0): visible_rows == item_count ⇒ item > visible is always false.
        let m = ScrollModel::combo(0);
        for n in 0..=N {
            assert_eq!(m.visible_rows(n, 9_999), n);
        }
    }

    #[test]
    fn unified_matches_combo_model_over_boundaries() {
        for &cap in &[7i32, 9] {
            let model = ScrollModel::combo(cap);
            for sb in scrollbars() {
                // item_count==0 is unreachable under the gate (see reachability test); start at 1.
                for item_count in 1..=N {
                    let visible_rows = model.visible_rows(item_count, sb.h);
                    if visible_rows == 0 {
                        continue; // unreachable for a combo (visible==0 ⟺ item==0)
                    }
                    if item_count <= visible_rows {
                        continue; // no scrollbar ⇒ thumb never built
                    }
                    let thumb_h = model.thumb_height(visible_rows, item_count, sb.h).unwrap();
                    assert_eq!(
                        thumb_h,
                        legacy_combo_thumb_height(visible_rows, item_count, sb.h),
                        "thumb_h cap={cap} sb={sb:?} n={item_count}"
                    );
                    let max_top = model.max_top_index(item_count, visible_rows);
                    assert_eq!(max_top, item_count.saturating_sub(visible_rows));
                    for top_index in 0..=max_top {
                        assert_eq!(
                            model.thumb_y(sb, thumb_h, top_index, max_top),
                            legacy_combo_thumb_y(sb, thumb_h, top_index, max_top),
                            "thumb_y cap={cap} sb={sb:?} n={item_count} top={top_index}"
                        );
                    }
                    for my in (sb.y - 5)..=(sb.y + sb.h + 5) {
                        // track click anchor:
                        let track_anchor = my - thumb_h / 2;
                        assert_eq!(
                            model.top_index_from_thumb_top(sb, thumb_h, max_top, track_anchor),
                            legacy_pointer_to_top(sb, thumb_h, max_top, track_anchor),
                            "track cap={cap} sb={sb:?} n={item_count} my={my}"
                        );
                        // drag anchor (combo-only) — grab offset of 3px from thumb top:
                        let drag_anchor = my - 3;
                        assert_eq!(
                            model.top_index_from_thumb_top(sb, thumb_h, max_top, drag_anchor),
                            legacy_pointer_to_top(sb, thumb_h, max_top, drag_anchor),
                            "drag cap={cap} sb={sb:?} n={item_count} my={my}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn unified_matches_listbox_model_over_boundaries() {
        let model = ScrollModel::listbox();
        for sb in scrollbars() {
            let visible_rows = model.visible_rows(0, sb.h); // geometric: item_count ignored
            for row_count in 0..=N {
                // thumb_height Option must match B EXACTLY, incl. the reachable
                // visible_rows==0 (rect shorter than one row) → None case.
                assert_eq!(
                    model.thumb_height(visible_rows, row_count, sb.h),
                    legacy_listbox_thumb_height(visible_rows, row_count, sb.h),
                    "listbox thumb_h sb={sb:?} rows={row_count} vis={visible_rows}"
                );
                if row_count == 0 || visible_rows == 0 || row_count <= visible_rows {
                    continue; // no thumb / no scrollbar
                }
                let thumb_h = model.thumb_height(visible_rows, row_count, sb.h).unwrap();
                let max_top = model.max_top_index(row_count, visible_rows);
                assert_eq!(max_top, row_count.saturating_sub(visible_rows));
                for top_index in 0..=max_top {
                    assert_eq!(
                        model.thumb_y(sb, thumb_h, top_index, max_top),
                        legacy_listbox_thumb_y(sb, thumb_h, top_index, max_top),
                        "listbox thumb_y sb={sb:?} rows={row_count} top={top_index}"
                    );
                }
                for my in (sb.y - 5)..=(sb.y + sb.h + 5) {
                    let anchor = my - thumb_h / 2;
                    assert_eq!(
                        model.top_index_from_thumb_top(sb, thumb_h, max_top, anchor),
                        legacy_pointer_to_top(sb, thumb_h, max_top, anchor),
                        "listbox track sb={sb:?} rows={row_count} my={my}"
                    );
                }
            }
        }
    }

    #[test]
    fn empty_paths_are_reachability_gated() {
        // §4E Step-1 proof, executable: A's empty trigger (item_count==0) is
        // UNREACHABLE under the combo scrollbar gate because visible_rows==0 ⟺
        // item_count==0 ⟹ `item > visible` (0 > 0) is false ⟹ no scrollbar. B's
        // visible_rows==0 (rect shorter than one row) IS reachable and yields no thumb.
        let combo = ScrollModel::combo(7);
        assert_eq!(combo.visible_rows(0, 9_999), 0, "combo empty ⟹ 0 visible ⟹ gate closed");

        let listbox = ScrollModel::listbox();
        let short_h = CHOOSE_MAP_LISTBOX_ROW_H - 1; // rect shorter than one row
        assert_eq!(listbox.visible_rows(0, short_h), 0);
        // With rows present and a tall-enough scrollbar the gate is open, but the
        // thumb is None under NoThumb:
        assert_eq!(listbox.thumb_height(0, 5, short_h + 200), None);

        // And the combo's MaxThumb path, if it WERE reached, returns a full-track
        // thumb — pinned so a future change to the (dead) branch is visible:
        let track_h = (100 - BUTTON_H * 2).max(1);
        assert_eq!(combo.thumb_height(0, 0, 100), Some(track_h.max(MIN_THUMB_H)));
    }
}
```

**Step 2: Verify (the GATE).** `cargo test -p vera20k --lib ui::skirmish_shell::scroll` — ALL
GREEN. If any assertion fails, the unified math is NOT bit-identical: STOP, do not rewire, fix the
primitive (Task 1) until green. **Do not proceed to Task 3 until this is green.**

### Task 3: Rewire Model A (`combos.rs`) to delegate; remove the inline scroll math

**Why:** Replace the duplicated combo thumb/track/drag math with delegation to the proven primitive.
Public signatures are unchanged, so the frozen combo scroll tests stay GREEN.

**Files:** Modify `src/ui/skirmish_shell/state/combos.rs`.

**Pattern:** 4D body-delegation — keep the signature, move the body to the shared helper.

**Step 1:** Add the import (with the existing `use super::super::layout::{…};` block at the top,
or as a new `use`):
```rust
use super::super::scroll::ScrollModel;
```

**Step 2:** Delete the private `combo_dropdown_thumb_height` (current `combos.rs:142-150`) entirely —
its body now lives in `ScrollModel::thumb_height`.

**Step 3:** Replace the body of `combo_dropdown_scroll_thumb_rect` (current `:152-177`) with the
delegating version (signature byte-identical):
```rust
pub fn combo_dropdown_scroll_thumb_rect(
    state: &SkirmishShellState,
    layout: &SkirmishShellLayout,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
) -> Option<RectPx> {
    let scrollbar = combo_dropdown_scrollbar_rect(state, layout, maps, id)?;
    let model = ScrollModel::combo(combo_dropdown_max_visible_rows(id));
    let visible_rows = combo_dropdown_visible_row_count(state, maps, id);
    let item_count = combo_dropdown_item_count(state, maps, id);
    let thumb_h = model.thumb_height(visible_rows, item_count, scrollbar.h)?;
    let max_top = combo_dropdown_max_top_index(state, maps, id);
    let open_top = state
        .open_combo_dropdown
        .filter(|open| open.id == id)
        .map(|open| open.top_index.min(max_top))
        .unwrap_or(0);
    let thumb_y = model.thumb_y(scrollbar, thumb_h, open_top, max_top);
    Some(RectPx::new(scrollbar.x, thumb_y, scrollbar.w, thumb_h))
}
```
(`thumb_height` returns `Some` for a combo — `MaxThumb` never yields `None` — so `?` never short-
circuits in practice; it is the correct, allocation-free way to thread the `Option`. `thumb_y`
re-applies `top_index.min(max_top)` internally, a no-op on the already-clamped `open_top` — identical
output to the pre-4E inline `(track_span * open_top) / max_top`.)

**Step 4:** Replace the body of `top_index_from_thumb_y` (current `:214-235`, the drag path):
```rust
pub(super) fn top_index_from_thumb_y(
    state: &SkirmishShellState,
    layout: &SkirmishShellLayout,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
    mouse_y: i32,
    grab_offset_y: i32,
) -> Option<usize> {
    let scrollbar = combo_dropdown_scrollbar_rect(state, layout, maps, id)?;
    let thumb = combo_dropdown_scroll_thumb_rect(state, layout, maps, id)?;
    let max_top = combo_dropdown_max_top_index(state, maps, id);
    let model = ScrollModel::combo(combo_dropdown_max_visible_rows(id));
    Some(model.top_index_from_thumb_top(scrollbar, thumb.h, max_top, mouse_y - grab_offset_y))
}
```
(The primitive's internal `max_top == 0 ⇒ 0` reproduces the deleted `if max_top == 0 { return Some(0) }`
guard.)

**Step 5:** Replace the body of `top_index_from_scrollbar_track_click` (current `:237-257`):
```rust
pub(super) fn top_index_from_scrollbar_track_click(
    state: &SkirmishShellState,
    layout: &SkirmishShellLayout,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
    mouse_y: i32,
) -> Option<usize> {
    let scrollbar = combo_dropdown_scrollbar_rect(state, layout, maps, id)?;
    let thumb = combo_dropdown_scroll_thumb_rect(state, layout, maps, id)?;
    let max_top = combo_dropdown_max_top_index(state, maps, id);
    let model = ScrollModel::combo(combo_dropdown_max_visible_rows(id));
    Some(model.top_index_from_thumb_top(scrollbar, thumb.h, max_top, mouse_y - thumb.h / 2))
}
```

**Step 6:** Delegate `combo_dropdown_visible_row_count` (current `:61-73`) to the model so the
`PerControlCap` axis is load-bearing in production (signature byte-identical; bit-identical output —
`rect_h` is ignored by `PerControlCap`, pass `0`):
```rust
pub fn combo_dropdown_visible_row_count(
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
) -> usize {
    let item_count = combo_items(state, maps, id).len();
    ScrollModel::combo(combo_dropdown_max_visible_rows(id)).visible_rows(item_count, 0)
}
```
(Equivalent to the deleted `if max_rows > 0 { item_count.min(max_rows as usize) } else { item_count }`
— `ScrollModel::combo`'s `PerControlCap(cap)` is exactly that with `cap = combo_dropdown_max_visible_rows(id)`.)

**Step 7:** Delegate `combo_dropdown_max_top_index` (current `:91-98`) to the model (signature
byte-identical; `saturating_sub` is unchanged):
```rust
pub(super) fn combo_dropdown_max_top_index(
    state: &SkirmishShellState,
    maps: &[MapMenuEntry],
    id: SkirmishComboId,
) -> usize {
    let item_count = combo_dropdown_item_count(state, maps, id);
    let visible_rows = combo_dropdown_visible_row_count(state, maps, id);
    ScrollModel::combo(combo_dropdown_max_visible_rows(id)).max_top_index(item_count, visible_rows)
}
```

**Step 8: Verify.** `cargo check -p vera20k` — clean (the model's `visible_rows()` + `max_top_index()`
are now reachable from production, so NO `dead_code` warning). Then
`cargo test -p vera20k --lib ui::skirmish_shell::state::tests` — the combo scroll pins
(`dropdown_wheel_is_inert_and_content_click_uses_top_index`,
`dropdown_scrollbar_arrows_step_and_drag_clamp_top_index`,
`skirmish_side_dropdown_scrollbar_track_click_jumps_to_native_top_index`,
`side_combo_exposes_random_country_and_verified_dropdown_cap`) GREEN; count still **87**.

### Task 4: Rewire Model B (`layout.rs`) to delegate; remove the inline scroll math

**Why:** Replace the duplicated listbox thumb/track math with delegation. The `#[cfg(test)] mod tests`
block (30 tests) is NOT touched.

**Files:** Modify `src/ui/skirmish_shell/layout.rs`.

**Pattern:** 4D body-delegation; signatures byte-identical.

**Step 1:** Add the import near the top of `layout.rs` (with the other `use` lines):
```rust
use super::scroll::ScrollModel;
```
(`layout` and `scroll` are siblings under `skirmish_shell`; `scroll.rs` imports constants FROM
`layout` — mutual `use` between sibling modules is fine in one crate, no cycle.)

**Step 2:** Replace the body of `choose_map_listbox_scroll_thumb_rect` (current `:648-672`):
```rust
pub fn choose_map_listbox_scroll_thumb_rect(
    row_count: usize,
    top_index: usize,
    rect: RectPx,
) -> Option<RectPx> {
    let scrollbar = choose_map_listbox_scrollbar_rect(row_count, rect)?;
    let visible_rows = choose_map_listbox_visible_row_count(rect);
    let model = ScrollModel::listbox();
    let thumb_h = model.thumb_height(visible_rows, row_count, scrollbar.h)?;
    let max_top = choose_map_listbox_max_top_index(row_count, rect);
    let thumb_y = model.thumb_y(scrollbar, thumb_h, top_index, max_top);
    Some(RectPx::new(scrollbar.x, thumb_y, scrollbar.w, thumb_h))
}
```
(`thumb_height` returns `None` under `NoThumb` exactly when the pre-4E `row_count == 0 || visible_rows
== 0` guard did — including the reachable `visible_rows == 0` case — so `?` reproduces the early
`return None`. `thumb_y`'s A-form `track_span` equals the pre-4E B-form `track_h − thumb_h` for all
`thumb_h ≥ 1`, proven in Task 2.)

**Step 3:** Replace the body of `choose_map_listbox_top_index_from_track_click` (current `:674-693`):
```rust
pub fn choose_map_listbox_top_index_from_track_click(
    row_count: usize,
    top_index: usize,
    rect: RectPx,
    mouse_y: i32,
) -> Option<usize> {
    let scrollbar = choose_map_listbox_scrollbar_rect(row_count, rect)?;
    let thumb = choose_map_listbox_scroll_thumb_rect(row_count, top_index, rect)?;
    let max_top = choose_map_listbox_max_top_index(row_count, rect);
    let model = ScrollModel::listbox();
    Some(model.top_index_from_thumb_top(scrollbar, thumb.h, max_top, mouse_y - thumb.h / 2))
}
```
(The primitive's `max_top == 0 ⇒ 0` reproduces the deleted `if max_top == 0 { return Some(0) }`.)

**Step 4:** Delegate `choose_map_listbox_visible_row_count` (current `:603-605`) to the model so the
`GeometricFromRect` axis is load-bearing in production (signature byte-identical; `item_count` is
ignored by the geometric source, pass `0`):
```rust
pub fn choose_map_listbox_visible_row_count(rect: RectPx) -> usize {
    ScrollModel::listbox().visible_rows(0, rect.h)
}
```
(Equivalent to the deleted `(rect.h / CHOOSE_MAP_LISTBOX_ROW_H).max(0) as usize` — `ScrollModel::listbox`
sets `row_h = CHOOSE_MAP_LISTBOX_ROW_H` and `GeometricFromRect` computes exactly that.)

**Step 5:** Delegate `choose_map_listbox_max_top_index` (current `:644-645`) to the model (signature
byte-identical; `saturating_sub` unchanged) so `ScrollModel::max_top_index` is production-reachable:
```rust
pub fn choose_map_listbox_max_top_index(row_count: usize, rect: RectPx) -> usize {
    ScrollModel::listbox().max_top_index(row_count, choose_map_listbox_visible_row_count(rect))
}
```
(`choose_map_listbox_max_top_index` is referenced by the frozen tests and by Steps 2/3 — the signature
is unchanged so all callers + frozen tests stay GREEN.)

**Step 6: Verify.** `cargo check -p vera20k` — clean (no `dead_code` warning — both model methods are
now reached from production via Steps 2–5). Then
`cargo test -p vera20k --lib ui::skirmish_shell::layout` — the listbox scroll pins
(`choose_map_modal_scrollbar_thumb_and_track_map_to_top_index`,
`choose_map_modal_listbox_hit_testing_reserves_scrollbar_width`) GREEN; count still **30**. Also
`cargo test -p vera20k --lib ui::skirmish_shell::state::choose_map` — the 4D wheel/mousedown tests
GREEN (they exercise the delegated track-click path through `set_top_index_clamped`/scroll helpers).

### Task 5: Checkpoint + STOP, format, commit (per master plan §6.2)

**Step 1 (build):** `cargo build -p vera20k` — read the literal final line.

**Step 2 (test, separate bounded pass — per `feedback_cargo_separate_verify_pass`):** run, reading each
literal `test result:` line:
- `cargo test -p vera20k --lib ui::skirmish_shell::scroll` → the 4 new proof tests GREEN.
- `cargo test -p vera20k --lib ui::skirmish_shell::state` → frozen `tests` module **87** + the 4D
  `choose_map` tests, all GREEN.
- `cargo test -p vera20k --lib ui::skirmish_shell::layout` → **30** GREEN.
- `cargo test -p vera20k --lib app_skirmish_shell_render` → **53** unchanged (paint emitters consume
  the same rects; no paint code changed, so this is a sanity guard).

**Step 3 (frozen-diff gate — scoped for 4E):**
- `git diff HEAD -- src/ui/skirmish_shell/state/tests.rs` must be **EMPTY** (untouched).
- `git diff HEAD -- src/ui/skirmish_shell/layout.rs` is NOT empty (4E legitimately edits the two scroll
  function bodies per §4E's Files list) — but it must touch ONLY
  `choose_map_listbox_scroll_thumb_rect` + `choose_map_listbox_top_index_from_track_click` + the new
  `use super::scroll::ScrollModel;`. **Inspect the diff hunks: NONE may fall inside the
  `#[cfg(test)] mod tests` block (the 30 tests), and the test count must read 30.** This is the §6.2
  "layout.rs frozen" invariant scoped to the test assertions, since §4E is the one sub-step that
  edits `layout.rs` production code. If any hunk touches a test assertion: hard-revert and STOP.

**Step 4 (format):** `rustfmt --edition 2024 --check` each edited file
(`scroll.rs`, `mod.rs`, `state/combos.rs`, `layout.rs`); hand-apply ONLY to your regions. `combos.rs`
and `layout.rs` have pre-existing non-conforming regions (per the 4C/4D discipline) — do NOT churn
untouched lines. Per CLAUDE.md, NEVER run crate-wide `cargo fmt`.

**Step 5 (commit ONLY the 4E files):**
```
git add src/ui/skirmish_shell/scroll.rs src/ui/skirmish_shell/mod.rs \
        src/ui/skirmish_shell/state/combos.rs src/ui/skirmish_shell/layout.rs
git commit -m "ui: Slice 4E - unify the two skirmish scroll models into one ScrollModel primitive"
```
Leave the parallel session's dirty tree (`src/rules/*`, `src/sim/*`) untouched.

**If ANY check in Steps 1–3 fails: hard-revert this commit and STOP — do not layer fixes.**

---

## Risk Areas (from §4E + impact analysis)

- **Math drift (highest):** any non-bit-identical change to thumb size/position/landing row is a
  visible scroll regression every time a combo opens or Choose-Map shows overflow. **Guard:** Task 2's
  boundary proof against verbatim legacy copies (gates Tasks 3/4) + the frozen 87+30 suite.
- **Empty-path divergence (the SIXTH axis, §2):** collapsing A's `MaxThumb` and B's `None` into one
  `thumb_height` risks drawing the wrong thumb on a degenerate list. **Guard:** `empty_path` stays a
  param; `empty_paths_are_reachability_gated` proves A's branch is dead under the gate and B's
  reachable `visible_rows==0` stays `None`.
- **`track_span` form choice:** A-form vs B-form differ only at `scrollbar_h ≤ 44`. **Guard:** the
  hand-proof above + degenerate `scrollbar_h ∈ {44,45,46}` in `scrollbars()`.
- **Signature drift breaking the frozen suite:** if a delegating function's signature shifts, the
  frozen tests won't compile. **Guard:** Tasks 3/4 keep every signature byte-identical (shown in full).
- **Parallel sessions:** the four 4E files are CLEAN at HEAD (git status shows only `src/rules/*` +
  `src/sim/*` dirty, another session's WIP). `app.rs`/`app_skirmish_shell_render*` are NOT touched by
  4E. Re-verify each 4E file is still clean by CONTENT immediately before editing; if another
  session's edits appear, WAIT — do not fix/revert/stash (CLAUDE.md parallel-sessions rule).

## Post-Plan Self-Review

- **Spec coverage:** all six §2 axes accounted for — (a) row_h, (b) visible_row_source, (f) empty_path
  drive `ScrollModel` geometry; (c) drag, (d) wheel, (e) cursor are recorded + honored caller-side.
  §4E's 4-step proof = Task 2 (Step1 reachability, Step2 boundary equivalence both sources, Step3
  max_top match, Step4 delete-after-green = Tasks 3/4).
- **Placeholder scan:** none — every task has complete code.
- **Sim compliance:** N/A (ui only); confirmed no `sim/` touch, integer geometry.
- **Frozen invariant:** `state/tests.rs` untouched; `layout.rs` test module untouched (production-only
  edit, scoped gate documented).
- **Confidence tagging:** `/review-plan` resolved the prior axis-(a)/(b) gap — `row_h` +
  `visible_row_source` are now load-bearing in production (Task 3 Steps 6–7, Task 4 Steps 4–5 delegate
  the `*_visible_row_count` / `*_max_top_index` helpers through the model), so only the two truly
  recorded-only flags (`thumb_drag_enabled`/`wheel_active`) are documentation, which matches the
  spec's struct shape. No `dead_code` warning remains.

## Sources & References

- **Design doc:** `docs/plans/2026-06-01-shell-substrate-slice4-plan.md` §4E (scope + 4-step proof),
  §2 (six DRIFT axes incl. the empty-path), §3/§6.2 (frozen suite + checkpoint), §7 (parallel safety).
- **Sibling realization plans (shape):** `docs/plans/2026-06-12-slice4c-combo-controlchrome-seam-plan.md`,
  `docs/plans/2026-06-12-slice4d-listbox-controlchrome-seam-plan.md`.
- **Prior commits:** 76a7fa56 (4C), 946e7e1a (4D.1), 097fe6e6 (4D.2) — 4C+4D gate satisfied.
- **Current code (re-verify by content before edit):** `src/ui/skirmish_shell/state/combos.rs:142-257`
  (Model A scroll math), `src/ui/skirmish_shell/layout.rs:644-693` (Model B scroll math), `:26-27`
  (shared `BUTTON_H=22`/`MIN_THUMB_H=14`), `:24` (`COMBO_DROPDOWN_ROW_H=23`), `:37-38`
  (`CHOOSE_MAP_LISTBOX_ROW_H=19`); `src/ui/skirmish_shell/mod.rs:7-9` (module decls);
  `src/ui/skirmish_shell/state/choose_map.rs` (cursor-storage owner, unedited).
- **Frozen tests (the net):** `state/tests.rs` combo pins `:1426/1455/1490/1557` + `:211`;
  `layout.rs` listbox pins `:1254/1273`.
- **INI:** none — the scroll math is geometry, not INI-driven.
</content>
</invoke>
