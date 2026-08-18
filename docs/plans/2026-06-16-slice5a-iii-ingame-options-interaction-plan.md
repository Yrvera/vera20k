# Slice 5a-iii — In-Game Options (0xBBB) Interaction — Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained. This is the
> INTERACTION slice — it makes the native `0xBBB` overlay (rendered in 5a-ii)
> respond to the mouse: slider drag with the live CSF value-label swap, checkbox
> toggle, Back → close + apply + persist, and the GameSpeed slider driving the
> sim cadence. Asset loading, layout, chrome paint, and static text are DONE
> (5a-i / 5a-ii, committed).

**Goal:** Wire the active in-game Options dialog (`0xBBB`) controls to behavior —
slider drag (GameSpeed/ScrollRate, inverted `6-pos`) with the position-indexed
CSF label swap, checkbox toggles, and Back → apply-then-write `[Options]` to
`RA2MD.INI` — matching gamemd's observable behavior including its quirks.

**Architecture:** Adds an app/ui-level client-options state (NEVER `sim/`) plus a
paused-overlay mouse router. Reuses the 5a-ii emitter (`build_in_game_options_*`),
the `layout_pass_in_game_options` rects, the existing `trackbar_pixel_offset`
geometry, the `tps_for_game_speed` cadence mapping, the `ModalResult::InGameOptions`
persist convention, and the `set_ini_value` single-key INI writer. Render path
(5a-ii) is unchanged except the emitters now read live state instead of populate
defaults.

**Design Doc:** `docs/plans/2026-06-12-slice5a-ingame-options-dialog-design.md` (§4 D4/D5, §5)

---

## Grounding Summary

- **Docs (verified-from-binary this session):**
  - `docs/plans/_5aiii-grounding-laneA-valuelabel.md` — the value-label swap fires
    on slider **drag only** (WM_HSCROLL `SB_THUMBTRACK`); the proc indexes
    `table_base[slider_pos]` → CSF key → `SendMessage(label, 0x4b2, resolved_wstr)`.
    Recovered index→key tables: GameSpeed `0x671` & ScrollRate `0x672` (7 entries,
    indexed by slider position 0..6) = `TXT_SLOWEST, TXT_SLOWER, TXT_SLOW,
    TXT_MEDIUM, TXT_FAST, TXT_FASTER, TXT_FASTEST`; VisualDetails `0x673` (hidden in
    `0xBBB`) = `TXT_LOW, TXT_MEDIUM, TXT_HIGH`. **Quirk to reproduce:** labels are
    NOT set at dialog open — gamemd shows the template default (`GUI:Faster`) next
    to both sliders until the user first drags *that* slider; only then does it
    swap to the position CSF text. (5a-ii already paints `GUI:Faster` at open.)
  - `docs/plans/_5aiii-grounding-laneB-gamespeed-cadence.md` — offline frame target
    `= Options.GameSpeed × 16 ms`, and `Options.GameSpeed = 6 − slider_pos`, so
    `frame_ms = (6 − slider_pos) × 16`. Default internal GameSpeed 3 → 48 ms
    (~20.8 fps); internal 0 → uncapped. Net play queues `EventClass 0x0D` (out of
    scope; offline stores directly).
  - `docs/research/OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md` §4/§5/§6
    — apply path reads `TBM_GETPOS`/`BM_GETCHECK`; GameSpeed/ScrollRate inverted
    `6-pos`, DetailLevel direct, checkboxes `==1`; result `1` → `WriteToINI`
    `RA2MD.INI [Options]` keys (GameSpeed, ScrollRate, DetailLevel, UnitActionLines,
    ShowHidden, ToolTips); result `2` (game-ended) does not persist; no cancel path.
- **Repo state confirmed this session (current, on `dev`):**
  - `tps_for_game_speed(stored)` (`src/app_types.rs:36`) **already** computes
    `round(1000 / (stored × 16))` (= GS3→21, GS1→63), i.e. gamemd's 16-ms-bucket
    model. So GameSpeed cadence is *wiring the slider to this existing fn*, not a
    new pacer. `sim_speed_tps` (`src/app.rs:368`) drives `advance_in_game_runtime`
    (`src/app_sim_tick.rs:287`).
  - Mouse dispatch: `handle_mouse_input` (`src/app_input.rs:44`, called from
    `src/app.rs:2221`) → `app_gadget_input::handle_mouse_button_event` →
    `GadgetConsume::{Tactical,Minimap,Consumed,NotConsumed}`. **No `state.paused`
    gate today** — a click while paused still routes to the tactical/gadget path.
  - The paused render branch (`src/app.rs:2883`) already draws the native overlay;
    `handle_pause_menu` (`src/app.rs:3035`) is compiled but only reachable via the
    dev overlay (`ReturnToMenu` is the temp quit-to-menu).
  - `build_in_game_options_instances` / `build_in_game_options_text_instances`
    (`src/app_skirmish_shell_render/in_game_options.rs`) currently paint *populate
    defaults* (`OPTIONS_TRACKBAR_DEFAULT_POSITION = 3`,
    `options_checkbox_default_checked`, value labels = `GUI:Faster`). They must
    read live client-options + interaction state.
  - `trackbar_pixel_offset(value,min,max,step,rect)` / `trackbar_active_width(rect)`
    / `trackbar_thumb_rect(rect,off)` (`src/ui/skirmish_shell/layout.rs:322-341`) —
    value→pixel; we add the inverse (pixel→value) for drag.
  - `ModalResult::InGameOptions(1).options_persists() == true`, `(2) == false`
    (`src/ui/shell/modal.rs:162`).
  - `set_ini_value(content,&section,&key,&value) -> Vec<u8>` single-key in-place
    INI writer (`src/util/ini_writer.rs:28`). The existing `[Audio] ScoreVolume`
    write (`src/audio/music.rs` + ini_writer) is the pattern to mirror for
    `[Options]`.
- **INI keys driving behavior:** `RA2MD.INI [Options]` GameSpeed, ScrollRate,
  DetailLevel, UnitActionLines, ShowHidden, ToolTips. Defaults (gamemd `SetDefaults`):
  GameSpeed 3, ScrollRate 3, DetailLevel 2, UnitActionLines 1, ShowHidden 0,
  ToolTips 1. (These are *client* options in `RA2MD.INI`, distinct from
  `rulesmd.ini [MultiplayerDialogSettings] GameSpeed` which seeds the skirmish-setup
  speed.)
- **Resolved (user decision + review 2026-06-16):** GameSpeed source-of-truth →
  unify on `options.game_speed`, derive `sim_speed_tps` (KD-3); `RA2MD.INI` path →
  `{config.paths.ra2_dir}/RA2MD.INI` via the existing `persist_settings_on_quit` /
  `write_score_volume_to_ra2md` pattern (`src/app.rs:1566`, `src/audio/music.rs:391`);
  UnitActionLines has a live consumer (`src/app_target_lines.rs:84`); effects apply
  on close, not live (KD-8); input hit-tests the render-cached anchor (KD-6).
- **Still unknown (→ deferred):** whether ScrollRate / DetailLevel / ShowHidden have
  live Rust consumers (Task 8 greps and wires-or-persist-only; no fabrication).

## Key Technical Decisions

- **KD-1 — Client-options state is a new app/ui-level struct, never `sim/`.** Holds
  the six `[Options]` values as gamemd's *internal* representation (GameSpeed/
  ScrollRate 0..6 where 0=fastest, DetailLevel 0..2, three bools), plus transient
  interaction state. — **Confidence:** high — **Source:** design §6, CLAUDE.md
  layering rule.
- **KD-2 — GameSpeed cadence reuses the existing `tps_for_game_speed`.** The Options
  GameSpeed slider stores internal `game_speed = 6 − slider_pos`; the sim cadence is
  `sim_speed_tps = tps_for_game_speed(game_speed)`. No new pacing model. —
  **Confidence:** high (the fn already implements `1000/(speed×16)`) — **Source:**
  `src/app_types.rs:36` + `_5aiii-grounding-laneB`. **Flagged divergence:**
  `tps_for_game_speed(0) == 60` (capped), whereas gamemd internal 0 = uncapped;
  carry this as a known VERA cap, do not "fix" silently.
- **KD-3 — `game_speed` is the single source of truth for sim speed; derive
  `sim_speed_tps` from it. (RESOLVED — user decision 2026-06-16: "reflect current
  speed, unify".)** On game start, seed `options.game_speed` from the same source
  that seeds `sim_speed_tps` today (the skirmish-setup speed, internal 1), then set
  `sim_speed_tps = tps_for_game_speed(options.game_speed)`. The slider both reads and
  writes `options.game_speed`, so it always shows the *current* game speed (NOT a
  forced internal-3). The resulting start tps is unchanged (`tps_for_game_speed(1) =
  63`). — **Confidence:** high (decision locked) — **Source:** user 2026-06-16 +
  `src/app_types.rs:36`.
- **KD-4 — Value-label swap is per-slider, drag-gated, indexed by slider position.**
  Each slider has a "dragged-since-open" flag (reset on overlay open). Label text =
  `GUI:Faster` (template default) until dragged, then the CSF key at
  `LABELS[slider_pos]` (`slider_pos = 6 − game_speed`). Reproduce the gamemd quirk
  exactly (stale "Faster" at open, position-correct after drag). — **Confidence:**
  high — **Source:** `_5aiii-grounding-laneA` (disasm + table reads).
- **KD-5 — Back → result `1` → apply-then-persist; the overlay closes via
  `state.paused = false`.** Persist writes only the touched `[Options]` keys via
  `set_ini_value` (mirrors the `[Audio] ScoreVolume` pattern), not a whole-object
  rewrite. result `2` (game ended while open) skips persist (no path triggers it in
  5a-iii's offline scope; encode the gate so it is correct when 5b lands). —
  **Confidence:** high — **Source:** `modal.rs` convention + OPTIONS_PROC §6 +
  design §8 Q1 (temp quit-to-menu preserved).
- **KD-6 — Mouse routing intercepts when `state.paused`, hit-testing CACHED rects.**
  A new `in_game_options_mouse` runs in `handle_mouse_input` before the
  gadget/tactical dispatch and consumes the click when paused. It hit-tests against
  the `InGameOptionsAnchor` **cached on `AppState` by the overlay render pass** —
  NOT a fresh recompute. The button-column Y (KD-4) depends on `sidebar_view`, which
  is a render-local byproduct of `render_game` (`src/app.rs:2862`) and is not
  available at input time; caching the anchor the render already computed both
  solves that and guarantees the hit rects exactly match what was drawn. —
  **Confidence:** high — **Source:** `src/app_input.rs:44` dispatch + `src/app.rs:2862`
  (sidebar_view is render-local) + review 2026-06-16.
- **KD-7 — Keyboard/Sound buttons remain stubs.** They paint the pressed frame but
  take no action (their sub-dialogs are deferred per design §0/§7). — **Confidence:**
  high — **Source:** design decision 2.
- **KD-8 — Effects apply on CLOSE, not live during interaction.** gamemd's
  `ApplyFromInGameDialog` applies every control's effect only when the dialog closes
  (result `1`); during interaction only the *visual/stored* state changes (slider
  thumb + stored value, checkbox check) plus the drag-gated label swap (which gamemd
  *does* update live). The Rust port must match this: the battlefield re-renders
  behind the non-opaque overlay each paused frame (`render_game` runs while paused,
  `src/app.rs:2878`), so applying e.g. `UnitActionLines` live would visibly toggle
  target lines behind the overlay — gamemd shows that change only after Back. Derive
  `sim_speed_tps` and apply all downstream effects in `apply_in_game_options` on
  close. — **Confidence:** high — **Source:** OPTIONS_PROC §5 (apply path is
  close-only) + `src/app.rs:2878` + review 2026-06-16.

## Open Questions

### Resolved During Planning

- *Does the value label show the slider value at open?* No — gamemd shows the
  template default `GUI:Faster` until first drag (`_5aiii-grounding-laneA` §4). 5a-ii
  already ships this; preserve it (KD-4).
- *Is GameSpeed 0..6 a new pacer?* No — `tps_for_game_speed` already implements the
  16-ms-bucket model (KD-2).
- *Which controls persist on cancel?* There is no cancel; every close persists on
  result `1` (OPTIONS_PROC §12, `modal.rs`).

### Resolved by user decision (2026-06-16)

- **GameSpeed source-of-truth (KD-3): "reflect current speed, unify".** The slider
  shows the *current* game speed (seeded from the skirmish setup, internal 1);
  `options.game_speed` is the single source of truth and `sim_speed_tps` is derived
  from it. No forced internal-3; the game-start tps is unchanged (63). /review-plan
  should still confirm the seed wiring does not regress the existing path.

### Resolved during review (2026-06-16)

- **`RA2MD.INI` path (was flagged).** Path is `{config.paths.ra2_dir}/RA2MD.INI`.
  The existing `App::persist_settings_on_quit` (`src/app.rs:1566`) +
  `audio::music::write_score_volume_to_ra2md(&config.paths.ra2_dir, ..)`
  (`src/audio/music.rs:391-402`) is the exact pattern — Task 7 reuses/extends it for
  `[Options]`. No new path helper needed.
- **UnitActionLines consumer (was flagged).** It EXISTS:
  `state.target_lines.set_unit_action_lines_enabled(bool)`
  (`src/app_target_lines.rs:84`, field `unit_action_lines_enabled`). Task 8 wires it
  concretely — not "persist-only if absent."

### Deferred to Implementation / Flagged

- **Downstream consumers for ScrollRate / DetailLevel / ShowHidden.** UnitActionLines
  and GameSpeed have confirmed live consumers (above + `tps_for_game_speed`). For the
  rest, Task 8 wires any live consumer it finds via grep and records the others as
  persist-only with a one-line note each — no fabricated consumers. (DetailLevel is
  hidden in `0xBBB` anyway; ShowHidden is a debug byte with no standard consumer.)

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/ui/shell/in_game_options_state.rs` | Client-options state (values + interaction) + pure helpers (slider_pos↔internal, label-key tables, hit-test). No render/sim deps. |
| Modify | `src/ui/shell/mod.rs` | `pub mod in_game_options_state;` |
| Modify | `src/ui/shell/in_game_options.rs` | Add the per-slider CSF label-key tables (`GAME_SPEED_LABELS`, `SCROLL_RATE_LABELS`) as `control`-adjacent data + the value-label resolver. |
| Modify | `src/app.rs` | Add `in_game_options: InGameOptionsState` + `in_game_options_anchor: Option<InGameOptionsAnchor>` to `AppState`; seed `game_speed` on game start; call `on_open()` at pause; route Back/apply/persist. |
| Modify | `src/app_input.rs` | Intercept mouse when `state.paused` → `in_game_options_mouse`; press/drag/release routing; paused-drag branch in `handle_cursor_moved_in_game`. |
| Create | `src/app_in_game_options_input.rs` | The paused-overlay mouse handler: hit-test the CACHED anchor's laid rects → set pressed/drag + visual/stored state, toggle checkbox bools (visual only), Back → close. |
| Modify | `src/app_skirmish_shell_render/in_game_options.rs` | Emitters take live `&InGameOptionsState`: slider thumb from live value, checkbox checked from live bool, pressed button frame, value-label swap. |
| Modify | `src/app_skirmish_shell_render.rs` | Pass `&state.in_game_options` into the emitter calls; **cache the computed `InGameOptionsAnchor` on `state.in_game_options_anchor`** for the input hit-test. |
| Create | `src/app_options_persist.rs` | `apply_in_game_options` (derive `sim_speed_tps` + ALL downstream effects, on close) and `persist_in_game_options` (the `[Options]` `RA2MD.INI` write via `set_ini_value`). |

## Interface Changes

- **`AppState` gains `in_game_options: InGameOptionsState`** — read by the emitters
  and the input handler; written by the input handler and game-start seed.
- **`AppState` gains `in_game_options_anchor: Option<InGameOptionsAnchor>`** —
  written by the overlay render pass each frame it draws; read by the input handler
  to hit-test the same rects that were drawn (None before the overlay first renders →
  input no-ops). Avoids needing the render-local `sidebar_view` at input time (KD-6).
- **`build_in_game_options_instances` / `build_in_game_options_text_instances` gain a
  `&InGameOptionsState` parameter** — the only callers are
  `render_in_game_options_overlay_with_atlas` and the unit tests. Populate-default
  helpers (`OPTIONS_TRACKBAR_DEFAULT_POSITION`, `options_checkbox_default_checked`)
  are replaced by reads of the state (keep them only as the seed defaults in the
  state ctor).
- **No `sim/` interface changes.** `sim_speed_tps` is app-level and already exists.

## Sim Checklist

N/A — no `sim/` files are touched. Client-options state is app/ui-level
(render/UI/camera/tick-rate options); the sim loop reads the existing
`sim_speed_tps` only. No new fixed-point math, no tick-order change, no state-hash
change.

## Risk Areas

- **Mouse interception when paused (Task 6).** Must consume the click so it does
  NOT also reach `tactical_mouse` (would issue unit orders behind the overlay).
  Regression: a click on empty overlay area while paused issues no tactical command.
- **Hit-test rect provenance (Task 6, KD-6).** The input handler MUST use the cached
  `InGameOptionsAnchor` the render pass stored — recomputing it at input time can't
  get the sidebar-anchored button Y (`sidebar_view` is render-local). If the cache is
  `None` (overlay not yet rendered), the handler no-ops.
- **Apply-on-close (Task 6/7, KD-8).** Interaction must NOT apply effects live; only
  Back (close) applies. Regression: toggling Target Lines while paused must not change
  the lines behind the overlay until Back.
- **Emitter signature change (Task 6 wiring).** The emitters are called from one
  render site + tests; the `&state` borrow must not conflict with `state.bit_font` /
  `state.batch_renderer` in the overlay pass (same disjoint-field pattern 5a-ii
  uses). Build catches misses.
- **GameSpeed source-of-truth (KD-3).** Highest behavioral risk — driving
  `sim_speed_tps` from `options.game_speed` must preserve the current skirmish-speed
  seed. Manual gate + /review-plan.
- **INI persist (Task 7).** Must write only the six `[Options]` keys, preserve every
  other byte (set_ini_value already guarantees this), and not crash if `RA2MD.INI`
  is absent (set_ini_value on empty input yields a fresh section).

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| 2 | Slider drag inverse: `game_speed = 6 − slider_pos`, quantized to the 0..6 step | Wrong inversion = slider moves opposite / off-by-one every drag | `_5aiii-grounding-laneB`; inverse of `trackbar_pixel_offset`; unit test round-trips value↔pixel |
| 4 | Value-label swap = `LABELS[slider_pos]`, drag-gated per slider; stale `GUI:Faster` at open | The exact gamemd quirk; visible every time Options opens & on every drag | `_5aiii-grounding-laneA` tables; unit test on the resolver |
| 4 | Label key tables: GameSpeed/ScrollRate `TXT_SLOWEST..TXT_FASTEST` (pos 0..6) | Wrong key = wrong word under the slider | `_5aiii-grounding-laneA` §2 verbatim table |
| 6 | GameSpeed → `sim_speed_tps = tps_for_game_speed(6 − slider_pos)` | Slider must change the actual game pace, not a cosmetic value | `_5aiii-grounding-laneB`; in-game observation (fast/slow) |
| 7 | Back persists only on result `1`; `6-pos` stored for GameSpeed/ScrollRate, direct for DetailLevel, `==1` for checkboxes | Persisted values must match gamemd byte-for-byte | OPTIONS_PROC §5/§6; reopen reflects; inspect `RA2MD.INI` |
| 7 | result `2` (game-ended) skips persist; no cancel path | Save-on-close contract | `modal.rs` `options_persists`; unit test |
| 6 | Checkbox click toggles `==1` bool; whole control rect clickable | Toggle parity | OPTIONS_PROC §5; hit-test unit test |
| 6/7 | Effects apply on CLOSE only; interaction changes visual/stored state + label live | gamemd applies in `ApplyFromInGameDialog` (close); battlefield re-renders behind the overlay, so a live `UnitActionLines` toggle is observable vs gamemd's on-close (KD-8) | OPTIONS_PROC §5; `src/app.rs:2878` (render while paused) |
| 6 | Hit-test uses the cached `InGameOptionsAnchor` from render, not a recompute | Back rect can't be reproduced at input time (sidebar_view is render-local); cached rects also exactly match what's drawn | `src/app.rs:2862`; KD-6 |

---

## Tasks

### Task 1: Client-options state struct + pure helpers

**Why:** Everything downstream reads/writes this; define it first (interfaces-first).

**Files:** Create `src/ui/shell/in_game_options_state.rs`; modify `src/ui/shell/mod.rs`.

**Pattern:** A plain data struct in `ui/shell` (no render/sim deps), like the other
`ui/shell` descriptor/geom modules.

**Step 1: Define the state + defaults.**
```rust
//! Client-side in-game Options (0xBBB) state: the six [Options] values plus the
//! transient interaction state the overlay needs. App/ui-level only — never sim/.
//!
//! Values are stored in gamemd's INTERNAL representation: GameSpeed/ScrollRate are
//! 0..6 with 0 = fastest (the dialog slider position is `6 - value`); DetailLevel
//! is 0..2 direct. Defaults match gamemd OptionsClass::SetDefaults.

/// GameSpeed/ScrollRate internal range (0 = fastest .. 6 = slowest).
pub const OPTIONS_SPEED_MIN: u32 = 0;
pub const OPTIONS_SPEED_MAX: u32 = 6;
/// DetailLevel range (0 = low .. 2 = high), direct (not inverted).
pub const OPTIONS_DETAIL_MAX: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InGameOptionsState {
    /// Internal GameSpeed 0..6 (0 = fastest). Slider position is `6 - game_speed`.
    pub game_speed: u32,
    /// Internal ScrollRate 0..6 (0 = fastest). Slider position is `6 - scroll_rate`.
    pub scroll_rate: u32,
    /// DetailLevel 0..2 (direct). Hidden in 0xBBB but carried for persistence.
    pub detail_level: u32,
    pub unit_action_lines: bool,
    pub show_hidden: bool,
    pub tooltips: bool,
    /// Transient: which owner-draw button is held (for the pressed frame).
    pub pressed_button: Option<u16>,
    /// Transient: control id of the slider currently being dragged, if any.
    pub dragging_slider: Option<u16>,
    /// Transient per-slider "dragged since this open" — gates the label swap from
    /// the template default ("Faster") to the position CSF text (gamemd quirk).
    pub game_speed_label_dragged: bool,
    pub scroll_rate_label_dragged: bool,
}

impl Default for InGameOptionsState {
    fn default() -> Self {
        // gamemd OptionsClass::SetDefaults: GameSpeed 3, ScrollRate 3,
        // DetailLevel 2, UnitActionLines 1, ShowHidden 0, ToolTips 1.
        Self {
            game_speed: 3,
            scroll_rate: 3,
            detail_level: 2,
            unit_action_lines: true,
            show_hidden: false,
            tooltips: true,
            pressed_button: None,
            dragging_slider: None,
            game_speed_label_dragged: false,
            scroll_rate_label_dragged: false,
        }
    }
}

impl InGameOptionsState {
    /// Reset the transient interaction flags when the overlay (re)opens — gamemd
    /// recreates the dialog, so the label-dragged quirk resets each open.
    pub fn on_open(&mut self) {
        self.pressed_button = None;
        self.dragging_slider = None;
        self.game_speed_label_dragged = false;
        self.scroll_rate_label_dragged = false;
    }
}

/// Slider position (0..6) shown for an internal speed value: `6 - value`.
/// GameSpeed/ScrollRate only (DetailLevel is direct).
pub fn speed_slider_pos(internal: u32) -> u32 {
    OPTIONS_SPEED_MAX - internal.min(OPTIONS_SPEED_MAX)
}

/// Internal speed value from a slider position (0..6): `6 - pos`.
pub fn speed_from_slider_pos(pos: u32) -> u32 {
    OPTIONS_SPEED_MAX - pos.min(OPTIONS_SPEED_MAX)
}
```

**Step 2: Declare the module.** In `src/ui/shell/mod.rs`, add `pub mod
in_game_options_state;` next to the existing `pub mod in_game_options;`.

**Step 3: Tests.**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_gamemd_setdefaults() {
        let s = InGameOptionsState::default();
        assert_eq!((s.game_speed, s.scroll_rate, s.detail_level), (3, 3, 2));
        assert!(s.unit_action_lines && !s.show_hidden && s.tooltips);
    }

    #[test]
    fn slider_pos_inverts_speed_round_trip() {
        for v in 0..=OPTIONS_SPEED_MAX {
            assert_eq!(speed_from_slider_pos(speed_slider_pos(v)), v);
        }
        // Internal 3 (default) sits at the midpoint slider position 3.
        assert_eq!(speed_slider_pos(3), 3);
        // Internal 0 (fastest) is the far slider position 6.
        assert_eq!(speed_slider_pos(0), 6);
    }

    #[test]
    fn on_open_clears_transient_flags() {
        let mut s = InGameOptionsState { game_speed_label_dragged: true, pressed_button: Some(0x686), ..Default::default() };
        s.on_open();
        assert!(!s.game_speed_label_dragged && s.pressed_button.is_none());
    }
}
```

**Step 4: Verify.** `cargo test -p vera20k in_game_options_state` → PASS.
**Step 5: Commit.**

### Task 2: Hit-test + slider-drag inverse helpers

**Why:** Pure geometry the input handler needs; testable without a window.

**Files:** Modify `src/ui/shell/in_game_options_state.rs`; uses
`crate::ui::skirmish_shell::{trackbar_active_width, RectPx}`.

**Pattern:** Inverse of `trackbar_pixel_offset` (`src/ui/skirmish_shell/layout.rs:326`);
hit-rect containment via `RectPx::contains`.

**Step 1: Slider value from mouse X.**
```rust
use crate::ui::skirmish_shell::{trackbar_active_width, RectPx};

/// Quantized slider POSITION (0..max) for a mouse x over a laid trackbar `rect`.
/// Inverse of `trackbar_pixel_offset` (which maps value -> pixel as
/// `(value-min)*active_width/span`, thumb drawn at `rect.x + 1 + offset`).
pub fn trackbar_pos_from_mouse_x(mouse_x: i32, min: i32, max: i32, rect: RectPx) -> i32 {
    let active_width = trackbar_active_width(rect).max(1);
    let span = (max - min).max(1);
    let rel = (mouse_x - (rect.x + 1)).clamp(0, active_width);
    // round to nearest stop
    min + ((rel * span * 2 + active_width) / (active_width * 2)).clamp(0, span)
}
```

**Step 2: Tests** — round-trip against `trackbar_pixel_offset` at the 0..6 stops.
```rust
#[test]
fn mouse_x_maps_back_to_slider_stop() {
    use crate::ui::skirmish_shell::{trackbar_pixel_offset, RectPx};
    let rect = RectPx::new(216, 163, 192, 21); // GameSpeed laid rect @ 800x600
    for pos in 0..=6 {
        let px = trackbar_pixel_offset(pos, 0, 6, 1, rect);
        let thumb_center_x = rect.x + 1 + px;
        assert_eq!(trackbar_pos_from_mouse_x(thumb_center_x, 0, 6, rect), pos, "pos {pos}");
    }
    // Clamps past the ends.
    assert_eq!(trackbar_pos_from_mouse_x(rect.x - 50, 0, 6, rect), 0);
    assert_eq!(trackbar_pos_from_mouse_x(rect.x + 9999, 0, 6, rect), 6);
}
```

**Step 3: Verify.** `cargo test -p vera20k in_game_options_state` → PASS.
**Step 4: Commit.**

### Task 3: CSF label-key tables + value-label resolver

**Why:** The drag-gated label swap needs the verified position→CSF-key tables and a
resolver the emitter calls (KD-4). Pure data + logic.

**Files:** Modify `src/ui/shell/in_game_options.rs`.

**Pattern:** Static tables next to the `control` module; a small resolver fn.

**Step 1: Tables (verbatim from `_5aiii-grounding-laneA` §2).**
```rust
/// GameSpeed/ScrollRate value-label CSF keys, indexed by SLIDER POSITION (0..6).
/// Verbatim from the gamemd CSF pointer tables (slider pos 0 = "slowest" end).
pub const SPEED_LABEL_KEYS: [&str; 7] = [
    "TXT_SLOWEST", // pos 0
    "TXT_SLOWER",  // pos 1
    "TXT_SLOW",    // pos 2
    "TXT_MEDIUM",  // pos 3
    "TXT_FAST",    // pos 4
    "TXT_FASTER",  // pos 5
    "TXT_FASTEST", // pos 6
];
```

**Step 2: Resolver** — returns the CSF key to show for a value label given the
slider position and whether that slider has been dragged this open. Before any
drag, the label keeps the template default `GUI:Faster` (the gamemd quirk).
```rust
/// CSF key for a speed value-label: template default `GUI:Faster` until the slider
/// has been dragged this open, then the position-indexed `SPEED_LABEL_KEYS` entry.
pub fn speed_value_label_key(slider_pos: u32, dragged: bool) -> &'static str {
    if !dragged {
        "GUI:Faster"
    } else {
        SPEED_LABEL_KEYS[(slider_pos as usize).min(SPEED_LABEL_KEYS.len() - 1)]
    }
}
```

**Step 3: Tests.**
```rust
#[test]
fn value_label_is_template_default_until_dragged_then_position_key() {
    assert_eq!(speed_value_label_key(3, false), "GUI:Faster");
    assert_eq!(speed_value_label_key(3, true), "TXT_MEDIUM");
    assert_eq!(speed_value_label_key(6, true), "TXT_FASTEST");
    assert_eq!(speed_value_label_key(0, true), "TXT_SLOWEST");
}
```

**Step 4: Verify.** `cargo test -p vera20k in_game_options` → PASS. **Step 5: Commit.**

### Task 4: Emitters read live state (thumb, checked, pressed, label swap)

**Why:** 5a-ii painted populate defaults; now the overlay reflects live values and
the drag-gated label.

**Files:** Modify `src/app_skirmish_shell_render/in_game_options.rs`.

**Pattern:** The existing emitter loop; add a `state: &InGameOptionsState` param and
read from it instead of the populate-default constants.

**Step 1:** Change `build_in_game_options_instances(chrome, screen_w, screen_h,
anchor)` → add `state: &InGameOptionsState`. In the match:
- `Button`: `let pressed = state.pressed_button == Some(c.id); let frame =
  options_button_sidebttn_frame_index(pressed);`
- `Trackbar`: slider position from the live value —
  GameSpeed → `speed_slider_pos(state.game_speed)`, ScrollRate →
  `speed_slider_pos(state.scroll_rate)` — then
  `trackbar_pixel_offset(pos as i32, 0, 6, 1, rect)`. (VisualDetails stays hidden.)
- `Checkbox`: `let checked = match c.id { control::TARGET_LINES =>
  state.unit_action_lines, control::SHOW_HIDDEN => state.show_hidden,
  control::TOOLTIPS => state.tooltips, _ => false };`

**Step 2:** Change `build_in_game_options_text_instances(font, csf, screen_w,
screen_h, anchor)` → add `state: &InGameOptionsState`. In `in_game_options_static_draws`,
for the value labels, override the CSF key with the live resolver:
```rust
let key = match c.id {
    control::GAME_SPEED_VALUE =>
        speed_value_label_key(speed_slider_pos(state.game_speed), state.game_speed_label_dragged),
    control::SCROLL_RATE_VALUE =>
        speed_value_label_key(speed_slider_pos(state.scroll_rate), state.scroll_rate_label_dragged),
    _ => c.csf_key.unwrap_or(""), // title/captions/footer keep their template key
};
```
(Keep the existing `resolve_static_text` fallback; `speed_value_label_key` returns
either `GUI:Faster` or a `TXT_*` key, both resolved through CSF the same way.)

**Step 3:** Drop the now-unused populate-default constants
(`OPTIONS_TRACKBAR_DEFAULT_POSITION`, `options_checkbox_default_checked`) or keep
them only inside `InGameOptionsState::default`. Update the existing emitter tests to
pass `&InGameOptionsState::default()` and add a test that a dragged GameSpeed slider
emits the position label (build text with `game_speed_label_dragged = true`,
`game_speed = 3` → expects a non-`GUI:Faster` draw for `0x671`).

**Step 4:** Update `src/app_skirmish_shell_render.rs`
`render_in_game_options_overlay_with_atlas` to pass `&state.in_game_options` into
both emitter calls. (`state.in_game_options` lands in Task 5; until then this won't
compile — order Task 5's `AppState` field add to land with this, or stub the field
first. Recommended: do Task 5 Step 1 (the field) before this step.)

**Step 5: Verify.** `cargo test -p vera20k in_game_options` + `cargo build -p
vera20k` → PASS. **Step 6: Commit.**

### Task 5: Add the state to `AppState` + seed on game start

**Why:** Gives the emitters and input handler a live instance; reconciles the sim
speed source-of-truth (KD-3).

**Files:** Modify `src/app.rs`.

**Step 1:** Add two fields to `AppState`, initialized in the constructor
(`src/app.rs:2585` neighborhood, where `sim_speed_tps` is set):
```rust
pub(crate) in_game_options: crate::ui::shell::in_game_options_state::InGameOptionsState, // = ::default()
/// Laid-out 0xBBB anchor cached by the overlay render pass each frame it draws,
/// so the paused mouse handler hit-tests the exact rects that were rendered
/// (the sidebar-anchored button Y is render-derived; see KD-6). None until the
/// overlay first renders.
pub(crate) in_game_options_anchor: Option<crate::ui::shell::layout::InGameOptionsAnchor>, // = None
```

**Step 1b:** In `render_in_game_options_overlay_with_atlas`
(`src/app_skirmish_shell_render.rs`), right after `let anchor =
in_game_options::in_game_options_anchor(atlas, sidebar_view);`, cache it:
`state.in_game_options_anchor = Some(anchor);`. (This is the only place the
sidebar-anchored Y is known.)

**Step 2 (KD-3 — FLAGGED):** At game start, seed `in_game_options.game_speed` from
the same skirmish-setup speed that currently seeds `sim_speed_tps`, then set
`sim_speed_tps = crate::app_types::tps_for_game_speed(in_game_options.game_speed)`.
Concretely: where `sim_speed_tps = default_yr_skirmish_tps()` is set today, instead
set `in_game_options.game_speed = DEFAULT_YR_SKIRMISH_GAME_SPEED` and derive
`sim_speed_tps` from it. **Do not change the resulting tps value** (it must stay
`tps_for_game_speed(1) = 63` unless /review-plan + the user decide the in-game
default should be internal 3). Add a one-line comment that this unifies the two
speed sources.

**Step 3:** When the overlay opens (ESC → `state.paused = true`), call
`state.in_game_options.on_open()` so the label-dragged quirk resets. Place this in
the ESC handler that sets `paused = true` (`src/app_input.rs` pause toggle) —
guard so it only fires on the false→true transition.

**Step 4: Verify.** `cargo build -p vera20k`; `cargo test -p vera20k --lib`
(no regressions). **Step 5: Commit.** *(Task 4 Step 4 now compiles.)*

### Task 6: Paused-overlay mouse handler (press / drag / release / checkbox / Back)

**Why:** Make the controls respond; intercept clicks so they don't fall through to
the tactical viewport (KD-6).

**Files:** Create `src/app_in_game_options_input.rs`; modify `src/app_input.rs`.

**Pattern:** `handle_mouse_input` dispatch (`src/app_input.rs:44`); reuse
`layout_pass_in_game_options` + `in_game_options_anchor` to get the live rects (same
inputs the emitter uses), and the Task 2 hit-test helpers.

**Step 1:** In `handle_mouse_input`, before the gadget dispatch, add:
```rust
if state.paused {
    crate::app_in_game_options_input::in_game_options_mouse(state, button, pressed);
    return; // consume — do not route to tactical/gadget while the overlay is up
}
```
(Use `state.cursor_x/cursor_y` for the hit position, as the rest of input does.)

**Step 2:** Write `in_game_options_mouse(state, button, pressed)`. **Interaction
changes only visual/stored state — NO effects apply here (KD-8); effects + persist
happen on Back/close in Task 7.**
- Read the cached rects: `let Some(anchor) = state.in_game_options_anchor else {
  return; }` (overlay not rendered yet → no-op). `let desc =
  build_in_game_options_descriptor(); let laid = layout_pass_in_game_options(&desc,
  screen_w, screen_h, anchor);`. (Uses the cached anchor — KD-6 — not a recompute.)
- Left button, on `pressed`, hit-test each VISIBLE control's laid rect
  (`rect.contains(cursor_x, cursor_y)`):
  - Button (`BACK`/`KEYBOARD`/`SOUND`): set `pressed_button = Some(id)`.
  - Trackbar (`GAME_SPEED`/`SCROLL_RATE`): set `dragging_slider = Some(id)`, set the
    per-slider drag flag (`game_speed_label_dragged` / `scroll_rate_label_dragged =
    true`), and store the new value:
    `pos = trackbar_pos_from_mouse_x(cursor_x, 0, 6, rect)`,
    `game_speed = speed_from_slider_pos(pos as u32)` (or `scroll_rate`). This updates
    the rendered thumb + the live label only — it does **not** touch `sim_speed_tps`.
  - Checkbox: flip the matching bool (`unit_action_lines` / `show_hidden` /
    `tooltips`) — the rendered check state only. Do **NOT** apply the downstream
    effect here (KD-8). Toggle on press (matches `BS_AUTOCHECKBOX`).
- Left button, on `release` (`!pressed`): if `pressed_button == Some(BACK)` and the
  cursor is still over the Back rect → `in_game_options_close(state)` (Task 7).
  Then clear `pressed_button` and `dragging_slider`. Keyboard/Sound release over the
  button: stub — clear pressed, no action (KD-7).
- Drag: add a paused branch at the TOP of `handle_cursor_moved_in_game`
  (`src/app_input.rs:217`): if `state.paused && dragging_slider.is_some()`,
  re-quantize the value from the current `cursor_x` against the cached anchor's laid
  rect for that slider (store only — same as the press path), then return (swallow
  the move). No cadence push.

**Step 3: Tests.** Unit-test the hit-test routing decision as a pure helper:
`in_game_options_hit(laid, cursor) -> OptionsHit { Button(id) | Slider(id, pos) |
Checkbox(id) | None }`, asserting a cursor over the Back rect → `Button(BACK)`, over
the GameSpeed rail at the far-right → `Slider(GAME_SPEED, 6)`, over the TargetLines
rect → `Checkbox(TARGET_LINES)`, elsewhere → `None`. (Keep the `state`-mutating
glue thin; test the decision.)

**Step 4: Verify.** `cargo test -p vera20k in_game_options` + `cargo build`.
**Step 5: Commit.**

### Task 7: Back → close + apply + persist `[Options]` to RA2MD.INI

**Why:** The save-on-close contract (KD-5); the slider/checkbox values become the
sim cadence + downstream effects, and persist.

**Files:** Create `src/app_options_persist.rs`; modify `src/app.rs`.

**Pattern:** `ModalResult::InGameOptions(1).options_persists()` gate
(`src/ui/shell/modal.rs:162`); the existing `[Audio] ScoreVolume` write
(`App::persist_settings_on_quit` `src/app.rs:1566` → `audio::music::write_score_volume_to_ra2md(&config.paths.ra2_dir, ..)`
`src/audio/music.rs:391-402`), reused/extended for `[Options]`. Path is
`{config.paths.ra2_dir}/RA2MD.INI` (filename const `RA2MD_INI_FILENAME`
`src/audio/music.rs:35`).

**Step 1: Apply (ALL effects, on close only — KD-8)** — `apply_in_game_options(state)`:
- `state.sim_speed_tps = tps_for_game_speed(state.in_game_options.game_speed)` (KD-2).
- Apply each control's downstream effect (Task 8): UnitActionLines →
  `state.target_lines.set_unit_action_lines_enabled(o.unit_action_lines)`
  (`src/app_target_lines.rs:84`); ToolTips / ScrollRate / DetailLevel via their live
  consumer if one exists, else persist-only (Task 8).
- Reset the sim accumulator like `PauseMenuAction::Resume`
  (`src/app.rs:3049-3051`) so the new pace takes effect cleanly on unpause.

**Step 2: Persist** — `persist_in_game_options(state)`, guarded by
`ModalResult::InGameOptions(1).options_persists()` (result `1` only):
```rust
let Some(config) = state.game_config.as_ref() else { return };
let path = config.paths.ra2_dir.join("RA2MD.INI");
let o = &state.in_game_options;
// Internal values are stored verbatim: GameSpeed/ScrollRate already hold
// `6 - slider_pos`; DetailLevel direct; checkboxes as "1"/"0".
let pairs = [
    ("GameSpeed", o.game_speed.to_string()),
    ("ScrollRate", o.scroll_rate.to_string()),
    ("DetailLevel", o.detail_level.to_string()),
    ("UnitActionLines", (o.unit_action_lines as u8).to_string()),
    ("ShowHidden", (o.show_hidden as u8).to_string()),
    ("ToolTips", (o.tooltips as u8).to_string()),
];
let mut bytes = std::fs::read(&path).unwrap_or_default(); // absent file -> fresh section
for (key, val) in &pairs {
    bytes = crate::util::ini_writer::set_ini_value(&bytes, "Options", key, val);
}
if let Err(err) = std::fs::write(&path, &bytes) {
    log::warn!("Failed to persist [Options] to RA2MD.INI: {err}"); // never fatal
}
```

**Step 3: Close** — `in_game_options_close(state)`: `apply_in_game_options(state)`;
`persist_in_game_options(state)` (result `1`); `state.paused = false`; reset timing
(`last_update_time`/`sim_accumulator_ms`) + re-hide the OS cursor (mirror
`PauseMenuAction::Resume`, `src/app.rs:3047-3056`).

**Step 4: result `2` gate (unit test only in 5a-iii).** Encode the persist behind
the `ModalResult::InGameOptions(result).options_persists()` check so a future
game-ended close (result `2`) skips persist. Unit-test: `persist` is called for
result `1`, skipped for `2` (test the gate predicate, not the file write).

**Step 5: Verify.** `cargo test -p vera20k` lib; manual: toggle a checkbox + move a
slider, click Back, reopen — values persist; inspect `RA2MD.INI [Options]`.
**Step 6: Commit.**

### Task 8: Wire downstream effects (live consumers vs persist-only)

**Why:** gamemd applies each control's effect; wire the ones with a live Rust
consumer, record the rest as persist-only — no fabricated consumers.

**Files:** Modify `src/app_options_persist.rs` (+ the specific consumer modules).

All effects are applied inside `apply_in_game_options` (Task 7), i.e. on close only
(KD-8) — NOT during interaction.

**Step 1:** For each option, wire the confirmed live consumer; for the rest, grep for
one and wire it if found, else leave persist-only with a one-line
`// persist-only: no consumer yet` note:
- **GameSpeed** → `state.sim_speed_tps = tps_for_game_speed(game_speed)` (Task 7, done).
- **UnitActionLines** → **confirmed consumer exists:**
  `state.target_lines.set_unit_action_lines_enabled(o.unit_action_lines)`
  (`src/app_target_lines.rs:84`, field `unit_action_lines_enabled`; gate documented
  in `UNITACTIONLINES_OPTION_RENDERPASS_GATE_GHIDRA_REPORT.md`).
- **ToolTips** → the tooltip manager/flag, if present in the Rust UI; else persist-only.
- **ScrollRate** → the camera scroll-speed input, if present (grep
  `scroll_rate`/`scroll_speed`); else persist-only.
- **DetailLevel** → render detail, if present; else persist-only (hidden in `0xBBB`
  anyway).
- **ShowHidden** → debug/option byte; persist-only (no standard consumer).

**Step 2:** Each wired consumer gets a focused assertion or an in-game check note;
each persist-only gets the one-line note in code. Add a unit test that
`apply_in_game_options` with `unit_action_lines = false` leaves
`state.target_lines.unit_action_lines_enabled() == false` (the one confirmed gate).

**Step 3: Verify.** `cargo build -p vera20k` + `cargo test -p vera20k --lib`.
**Step 4: Commit.**

### Task 9: Retire the dev-overlay/egui SetGameSpeed tps path (optional, scoped)

**Why:** Design §5 item 8 — do not keep the arbitrary tps-preset model behind the
native slider. The egui pause card draw is already retired (5a-ii); this removes the
remaining tps-preset *input* paths that contradict the 0..6 model, OR re-expresses
them through `game_speed`.

**Files:** Modify `src/app.rs` (`handle_pause_menu` `SetGameSpeed`, dev overlay
`SetGameSpeed`/`ResetGameSpeed`), `src/app_dev_overlay.rs`.

**Step 1:** `handle_pause_menu` is no longer the draw path (5a-ii). Keep it compiled
for reference (per session note) but ensure its `SetGameSpeed(tps)` is not reachable
in normal play. The **dev overlay** speed slider is a developer tool — keep it, but
make it set `in_game_options.game_speed` (0..6) and derive `sim_speed_tps`, so dev
and the native slider share one source of truth (KD-3). If converting the dev slider
is out of scope, leave it as a dev-only direct-tps override and note that it bypasses
the Options model. **Decide with /review-plan.**

**Step 2: Verify.** `cargo build` + `cargo test -p vera20k --lib`. **Step 3: Commit.**

### Task 10: Manual in-game verification gate (no code)

**Why:** Drag feel, label swap, cadence change, and persistence need a side-by-side
check the unit tests cannot provide.

**Verify (run the app):**
- ESC opens the overlay. GameSpeed/ScrollRate value labels show **"Faster"** at open
  (the quirk), regardless of the live value.
- Drag the GameSpeed slider: the thumb moves and the value label swaps to the
  position word (`Slowest..Fastest`). The game does NOT change speed yet (sim is
  frozen while paused; effects apply on close — KD-8). Drag ScrollRate: its label
  swaps independently.
- Toggle each checkbox: the rendered check state flips immediately, but the effect
  (e.g. target lines behind the overlay) does NOT change until Back (KD-8) — verify
  the lines behind the overlay stay put while toggling, then change after Back.
- Click Back: the overlay closes and the game resumes at the **chosen** speed (the
  cadence + all effects apply now). Re-open ESC: the sliders/checkboxes reflect the
  values just set (apply held), labels back to "Faster" (re-open quirk). Inspect
  `RA2MD.INI [Options]` — the six keys updated, every other key/section byte-preserved.
- A click on empty overlay area while paused issues **no** tactical command.
- Keyboard/Sound: press paints the pressed frame; release does nothing (stub).
- The temp quit-to-menu (dev shortcut) still works.

**Verify against gamemd (already grounded):** value-label tables + drag-gating
(`_5aiii-grounding-laneA`), `frame_ms = (6-pos)×16` cadence
(`_5aiii-grounding-laneB`), apply/persist keys (OPTIONS_PROC §5/§6).

## Sources & References

- **Design doc:** `docs/plans/2026-06-12-slice5a-ingame-options-dialog-design.md` §4/§5/§8
- **Grounding (this session, local):** `docs/plans/_5aiii-grounding-laneA-valuelabel.md`,
  `docs/plans/_5aiii-grounding-laneB-gamespeed-cadence.md`
- **Ghidra reports:** `OPTIONS_PROC_004E1FE0_INIT_PERSIST_PATH_GHIDRA_REPORT.md`
  (apply/persist/INI), `OPTIONS_0XBBB_0XF5_CHROME_OWNERDRAW_ASSETS_GHIDRA_REPORT.md`,
  `UNITACTIONLINES_OPTION_RENDERPASS_GATE_GHIDRA_REPORT.md`
- **gamemd addresses (not in Rust comments):** proc `0x004E1FE0` (WM_HSCROLL label
  swap, custom init), `0x004E1DE0` ApplyFromInGameDialog (`6-pos`), `0x005FAD10`
  WriteToINI, label tables `0x00822730`/`0x0082274C`/`0x00822768`, pacing
  `Main_Tick 0x0055D360` + `FUN_0055E160` (`GameSpeed × 16 ms`).
- **INI keys:** `RA2MD.INI [Options]` GameSpeed/ScrollRate/DetailLevel/
  UnitActionLines/ShowHidden/ToolTips; defaults 3/3/2/1/0/1.
- **Related code:** `src/ui/shell/in_game_options.rs` (descriptor + 5a-iii tables),
  `src/app_skirmish_shell_render/in_game_options.rs` (emitters),
  `src/ui/skirmish_shell/layout.rs:322-341` (trackbar geometry: `trackbar_pixel_offset`
  / `trackbar_active_width` / `trackbar_thumb_rect`),
  `src/app_types.rs:36` (`tps_for_game_speed`), `src/app_input.rs:44/217`
  (mouse/cursor dispatch), `src/ui/shell/modal.rs:162` (persist convention),
  `src/util/ini_writer.rs:28` (`set_ini_value`), `src/app.rs:2862` (sidebar_view is
  render-local), `src/app.rs:2883/3035` (paused render + reference pause handler),
  `src/app.rs:1566` (`persist_settings_on_quit`) + `src/audio/music.rs:391-402`
  (`write_score_volume_to_ra2md` — the RA2MD.INI write pattern),
  `src/app_target_lines.rs:84` (`set_unit_action_lines_enabled` — UnitActionLines
  consumer), `src/ui/shell/geom.rs:34` (`RectPx::contains`).
- **Prior slices:** 5a-i descriptor plan, 5a-ii paint plan
  (`docs/plans/2026-06-15-slice5a-ii-ingame-options-paint-plan.md`), commit
  `e9e2ab27` (5a-ii static text).
