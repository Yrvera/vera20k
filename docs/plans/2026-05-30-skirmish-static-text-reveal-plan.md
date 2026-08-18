# Skirmish 0x102 Right-Panel Static Text-Reveal — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Reproduce gamemd's `0x4EC→0x4EE` character-by-character text reveal for the three Skirmish `0x102` right-panel statics (title `0x694`, game-type `0x6EC`, map-label `0x5A8`), firing at shell first-paint slide completion.

**Architecture:** Presentation-only. A per-static reveal counter lives in `SkirmishShellState` (UI layer), is advanced on a 30 ms cadence by the app render loop, started by the existing slide-completion event in `app_shell_transition.rs`, and consumed by a new reveal parameter on `shell_text::draw_in_rect`. `sim/` is untouched.

**Design Doc:** none (user chose "decode then plan"). Premise grounded in `docs/research/skirmish-ui/SKIRMISH_STATIC_REVEAL_ANIMATION_0X102_GHIDRA_REPORT.md` + this session's binary verification of `FUN_00602490`, `FUN_00621040`, `FUN_00434cd0`.

---

## Grounding Summary

- **Docs:** `SKIRMISH_STATIC_REVEAL_ANIMATION_0X102_GHIDRA_REPORT.md` (HIGH-confidence) gives the full state machine: kind-1 statics, running byte, count/interval(30ms)/step(1)/range(8), `0x4EC→0x4EE` start, count-advances-on-paint, stop at `wcslen+9`, `0x4B2` text-update restart. `SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md` establishes the slide fires the `0x4EC` at completion.
- **Ghidra verified this session:** `FUN_00602490` → the qualifying statics for `0x102` are exactly `0x694`, `0x6EC`, `0x5A8` (not `0x695` — gated/deferred). `FUN_006071E0` DL=1 ends with `SendMessageA(parent,0x4EC)`. `FUN_00621040`→`FUN_00434cd0` is the reveal renderer: **draws only the first `count` characters** (`if (param_9<=local_28) stop`); the **last `range`(8) chars before the cursor are blended toward `g_SelectedUnitHighlightColor`** with per-char intensity `(0xFF/range)*k`; chars before that window draw at full base color.
- **Repo pattern:** mirrors the existing slide wave `ShellFrameWave` in `src/app_shell_transition.rs` (Instant-based, +1 unit per 30 ms, no catch-up) and the steady-state text path `shell_text::draw_in_rect` (`src/render/shell_text.rs:57`).
- **INI:** none. Timing/step/range are hardcoded shell-control classification constants in the binary; do NOT invent INI keys.
- **Unknown after grounding:** exact retail pixels of the fade gradient (no runtime capture); the highlight-blend color math is reproduced from `FUN_00434cd0` but flagged medium-confidence on exact channel rounding.

## Key Technical Decisions

- **Reveal state lives in `SkirmishShellState`, advanced by the app loop on a 30 ms Instant cadence (not per-render-frame).** — Matches native's 30 ms timer cadence deterministically; avoids coupling reveal speed to frame rate. **Confidence:** high. **Source:** report §3.4 + `ShellFrameWave` pattern.
- **Reveal STARTS at slide completion, reusing the existing `is_complete` edge in `render_shell_first_paint_slide`.** — This is the native `0x4EC` point. **Confidence:** high. **Source:** Ghidra `FUN_006071E0` `0x00607F95`.
- **`draw_in_rect` gains an optional `Reveal { count, range }`; `None` = fully revealed (steady state). The cutoff is applied inside `BitFont::build_text`, not by re-walking characters in `draw_in_rect`.** — `build_text` is a proportional, stateful layout (`char_spacing` only when `emitted>0`, per-glyph `pixel_width`, distinct space/tab/`\r\n`/missing-glyph paths). Reproducing that walk outside it would drift; threading the window in keeps `reveal==None` byte-identical and the advance correct. **Confidence:** high. **Source:** `src/render/bit_font.rs:102` (`build_text`).
- **Count advances strictly +1 per 30 ms in Rust; native advances +1 per PAINT (usually but not always 30 ms).** — Deliberate, documented divergence; the dominant observable cadence is 30 ms/char. **Confidence:** medium (divergence accepted). **Source:** report §3.4 + Negative Facts.
- **v1 is character-WIPE only — no highlight gradient.** The trailing-8-char tint in `FUN_00434cd0` is NOT a base→highlight RGB lerp: window chars are drawn in the base color modulated through an intensity table derived from `g_SelectedUnitHighlightColor` (`FUN_006612c0`/`FUN_004355b0`/`FUN_004355d0`), intensity `(0xFF/range)*k`, brightest at the cursor. That color path is undecoded and Rust's BitFont is RGB-tint (no palette), so reproducing it now would be a guess (DRIFT). v1 ships the dominant visible effect (the left-to-right wipe) exactly; the gradient is deferred to a separate verified item (see Deferred Open Questions / Task 7). **Confidence:** high for the wipe; gradient explicitly out of v1 scope. **Source:** `FUN_00434cd0` (read this session).

## Open Questions

### Resolved During Planning
- Which statics reveal for `0x102`? → `0x694`, `0x6EC`, `0x5A8` only (`FUN_00602490`, verified). `0x695` is gated and deferred.
- Does reveal start before or after the slide? → After (slide completion sends `0x4EC`).
- Is there sound? → No; sound id is `-1` (silent). No audio work.
- Is `count` a char index or pixel? → Character index across the whole text; target `wcslen(text)+1+range`.

### Deferred to Implementation
- **Highlight gradient (deferred to Task 7, not v1).** Requires decoding `FUN_006612c0`/`FUN_004355b0`/`FUN_004355d0` and reading `g_SelectedUnitHighlightColor` to reproduce the trailing-8-char intensity-table tint exactly, then mapping it onto Rust's RGB-tint BitFont. Out of v1 scope to avoid shipping a guessed lerp.
- Whether the wrapped-line char counting must include the implicit line breaks the way native `wcslen` does (native counts the raw wide string incl. control chars). Resolve when wiring the multi-line case; the three `0x102` statics are single-line in practice, so the v1 test uses single-line text.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/ui/skirmish_shell/static_reveal.rs` | `StaticReveal` per-label reveal state + advance/start/restart logic (pure, testable) |
| Modify | `src/ui/skirmish_shell/state/player_name.rs` | Add 3 `StaticReveal` fields to `SkirmishShellState` (struct at :218, `impl Default` at :252); restart hook on map/mode text change |
| Modify | `src/ui/skirmish_shell/mod.rs` | `pub mod static_reveal;` (confirm declaration site — `state` is itself a submodule dir alongside `state.rs`) |
| Modify | `src/render/shell_text.rs` | Add `Reveal { count, range }` param to `draw_in_rect`, threaded into `BitFont::build_text` |
| Modify | `src/render/bit_font.rs` | `build_text` gains an optional reveal window; applies the first-`count`-chars cutoff inside its existing cursor/spacing/`emitted` walk |
| Modify | `src/app_skirmish_shell_render/text.rs` | Thread each static's reveal window through `push_static_label_draw`→`push_text_draw`→`draw_in_rect` (statics at :520/:534/:546; helper at :397) |
| Modify | `src/app_shell_transition.rs` | On Skirmish slide `is_complete`, start the 3 statics' reveal; advance reveals on the 30 ms cadence |

## Interface Changes

- `shell_text::draw_in_rect` gains a trailing `reveal: Option<shell_text::Reveal>` parameter (no highlight param in v1 — wipe only). **Every existing caller passes `None`.** Confirmed call sites (verified): `src/app_main_menu_shell_render.rs:153`, `src/app_single_player_shell_render.rs:118`, `src/app_skirmish_shell_render/text.rs:201` (inside `push_text_draw`), plus 5 in-file test calls. Small blast radius.
- `BitFont::build_text` (`src/render/bit_font.rs:102`) gains an optional reveal window arg; `None` keeps output byte-identical.
- `push_text_draw` and `push_static_label_draw` (`src/app_skirmish_shell_render/text.rs:192`, `:397`) gain an `Option<Reveal>` arg to thread the window from the 3 static call sites.
- New public type `shell_text::Reveal { count: u32, range: u32 }`.
- New public type `skirmish_shell::static_reveal::StaticReveal`.
- `SkirmishShellState` (`src/ui/skirmish_shell/state/player_name.rs:218`, manual `impl Default` at `:252`) gains 3 fields, each initialized to `StaticReveal::default()` in that `impl Default`.

## Sim Checklist

Not applicable — no `sim/` files are touched. Confirm during execution that no task imports anything under `src/sim/`.

## Risk Areas

- **`draw_in_rect` signature change has the widest blast radius.** Every shell text caller must compile. Mitigation: Task 2 adds the param with all callers passing `None` in the same task; `cargo check -p vera20k` must pass before proceeding.
- **Regression: blank right-panel text.** If reveal state defaults to `count=0`/running with no start event, labels could render empty. Mitigation: default `StaticReveal` = **fully revealed / inactive** (renders full text); reveal only becomes partial after an explicit start. Task 1 test `default_renders_full_text` guards this.
- **Reveal never starting** (slide-completion edge missed) would just show full text instantly — same as today, no regression. Acceptable failure mode.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| 1 | Count target = `wcslen(text)+1+range` (=len+9), step 1, stop kills advance but leaves full text drawn | Reveal duration ≈ (len+8)*30 ms, visible every Skirmish entry | Ghidra `OwnerDraw_Static_006153E0` `0x00615B11..0x00615B49`; unit test |
| 1 | Cadence 30 ms/char, no catch-up on long frame gaps | Reveal speed must match retail, not frame rate | report §3.4; mirror `ShellFrameWave::advance` |
| 2 | Only first `count` chars drawn; chars ≥ count invisible | The core left-to-right wipe (v1 ships this exactly) | Ghidra `FUN_00434cd0` `if (param_9<=local_28) stop`; render test |
| 7 (deferred) | Trailing `range`=8 chars tinted via `g_SelectedUnitHighlightColor` intensity table `(0xFF/8)*k`, brightest at cursor | The soft highlighted leading edge — NOT a base→highlight lerp; undecoded color path | Decode `FUN_006612c0`/`FUN_004355b0`/`FUN_004355d0` + read `g_SelectedUnitHighlightColor`; in-game side-by-side before locking pixels |
| 4 | Reveal starts at slide completion, NOT first paint | Starting early would desync from the slide | Ghidra `FUN_006071E0` `0x00607F95` (0x4EC at end of DL=1) |
| 4 | Exactly the 3 statics `0x694/0x6EC/0x5A8` reveal (not `0x695`) | Wrong control set = wrong elements animating | Ghidra `FUN_00602490` (verified) |
| 5 | `0x4B2`-equivalent: changing map/mode text while running restarts that label's reveal from count 1 | Visible when the player changes map/mode | Ghidra thunk `0x00611C72..0x00611CAF`; unit test |

---

## Tasks

### Task 1: `StaticReveal` state type (pure logic)

**Why:** The reveal state machine is pure and testable in isolation; define it before wiring render or triggers.

**Files:**
- Create: `src/ui/skirmish_shell/static_reveal.rs`
- Modify: `src/ui/skirmish_shell/mod.rs` (add `pub mod static_reveal;`)

**Pattern:** Mirrors `ShellFrameWave` in `src/app_shell_transition.rs` (Instant-based, +1 per 30 ms, no catch-up).

**Step 1: Define types and constants**
```rust
// src/ui/skirmish_shell/static_reveal.rs
//! gamemd kind-1 static text reveal for the Skirmish 0x102 right-panel labels.
//! Pure presentation state: a per-label character cursor advancing one step per
//! 30 ms, started by the shell first-paint slide completion (the 0x4EC->0x4EE
//! event). Renders the first `count` characters with an 8-character highlight
//! gradient at the leading edge. No sim coupling.

use std::time::{Duration, Instant};

/// Native timer interval for kind-1 reveal (0x1E ms).
pub const REVEAL_TICK_MS: u32 = 30;
/// Native reveal step added per advance.
pub const REVEAL_STEP: u32 = 1;
/// Native trailing highlight window (range) for these statics.
pub const REVEAL_RANGE: u32 = 8;

/// Per-label reveal cursor. Default = inactive (renders full text).
#[derive(Debug, Clone)]
pub struct StaticReveal {
    /// `None` => inactive: render the whole string (steady state).
    /// `Some(_)` => running or completed reveal with a character cursor.
    running: Option<RunState>,
}

#[derive(Debug, Clone)]
struct RunState {
    /// Native +0x80 reveal count; starts at 1.
    count: u32,
    /// Inclusive target = char_len + 1 + REVEAL_RANGE; advance stops at/after this.
    target: u32,
    last_step_at: Instant,
}

impl Default for StaticReveal {
    fn default() -> Self {
        // Inactive by default so labels render full text until a reveal starts.
        Self { running: None }
    }
}
```

**Step 2: Implement start / advance / window**
```rust
impl StaticReveal {
    /// Begin (or restart) the reveal for `text` at the given instant.
    /// target = char count + 1 + range (native wcslen(text)+1+range).
    pub fn start(&mut self, text: &str, now: Instant) {
        let target = text.chars().count() as u32 + 1 + REVEAL_RANGE;
        self.running = Some(RunState {
            count: 1,
            target,
            last_step_at: now,
        });
    }

    /// Advance at most ONE step per call, only once >= 30 ms elapsed since the
    /// last step. Never collapses multiple steps (faithful to one-per-Sleep).
    pub fn advance(&mut self, now: Instant) {
        let step = Duration::from_millis(u64::from(REVEAL_TICK_MS));
        if let Some(run) = self.running.as_mut() {
            if run.count < run.target && now.duration_since(run.last_step_at) >= step {
                run.count += REVEAL_STEP;
                run.last_step_at += step;
            }
        }
    }

    /// Reveal window to hand the renderer, or `None` when inactive (full text).
    /// Once `count >= target` the reveal is complete; we return `None` so the
    /// renderer draws the full string (native leaves full text drawn after stop).
    pub fn window(&self) -> Option<RevealWindow> {
        match &self.running {
            Some(run) if run.count < run.target => Some(RevealWindow {
                count: run.count,
                range: REVEAL_RANGE,
            }),
            _ => None,
        }
    }
}

/// Character reveal window passed to the text renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevealWindow {
    /// Number of leading characters drawn (chars >= count are hidden).
    pub count: u32,
    /// Trailing highlight-gradient width.
    pub range: u32,
}
```

**Step 3: Add tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_renders_full_text() {
        // Inactive default => no reveal window => renderer draws full string.
        assert_eq!(StaticReveal::default().window(), None);
    }

    #[test]
    fn start_sets_count_one_and_target_len_plus_9() {
        let t0 = Instant::now();
        let mut r = StaticReveal::default();
        r.start("ABCDE", t0); // 5 chars => target 5+1+8 = 14
        assert_eq!(r.window(), Some(RevealWindow { count: 1, range: 8 }));
    }

    #[test]
    fn advance_one_step_per_30ms_no_catchup_and_stops_at_target() {
        let t0 = Instant::now();
        let mut r = StaticReveal::default();
        r.start("AB", t0); // target = 2+1+8 = 11
        r.advance(t0 + Duration::from_millis(29));
        assert_eq!(r.window().unwrap().count, 1); // not yet
        r.advance(t0 + Duration::from_millis(30));
        assert_eq!(r.window().unwrap().count, 2);
        // A 1-second gap advances only ONE step (no catch-up).
        r.advance(t0 + Duration::from_millis(1030));
        assert_eq!(r.window().unwrap().count, 3);
        // Drive to target: count reaches 11 => window() becomes None (full text).
        let mut t = t0 + Duration::from_millis(1030);
        for _ in 0..20 {
            t += Duration::from_millis(30);
            r.advance(t);
        }
        assert_eq!(r.window(), None);
    }

    #[test]
    fn restart_resets_count_to_one() {
        let t0 = Instant::now();
        let mut r = StaticReveal::default();
        r.start("ABCDEFGHIJ", t0);
        for i in 1..=5 {
            r.advance(t0 + Duration::from_millis(30 * i));
        }
        r.start("NEWMAP", t0 + Duration::from_millis(500)); // text changed mid-reveal
        assert_eq!(r.window().unwrap().count, 1);
    }
}
```

**Step 4: Verify**
Run: `cargo test -p vera20k static_reveal -- --nocapture`
Expected: `test result: ok` with 4 passed.

**Step 5: Commit** — `feat(skirmish-ui): add StaticReveal kind-1 reveal state machine`

---

### Task 2: Thread a reveal window through `build_text` and `draw_in_rect` (wipe only)

**Why:** The character cutoff is parity-critical; implement it where the cursor/spacing/`emitted` state already lives (`build_text`) so the advance stays exact and the `reveal==None` path is byte-identical. Interface change first so callers in later tasks compile against the final signature. v1 = wipe only; no gradient (see Task 7).

**Files:**
- Modify: `src/render/bit_font.rs:102` (`build_text` gains the reveal arg + cutoff)
- Modify: `src/render/shell_text.rs:57` (new `Reveal` type; `draw_in_rect` gains `reveal` arg, threads it to `build_text` with a running cross-line offset)

**Pattern:** Extends `build_text`'s existing per-char walk; extends `draw_in_rect`'s per-line loop.

**Step 1: Add the `Reveal` type (shell_text.rs)**
```rust
// src/render/shell_text.rs (near ShellAlign)
/// Character reveal window for kind-1 static text animation (v1: wipe only).
/// `count` = number of leading characters drawn; characters at index >= count
/// are not emitted. `range` is carried for the deferred highlight gradient
/// (Task 7) and is unused by the v1 wipe.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Reveal {
    pub count: u32,
    pub range: u32,
}
```

**Step 2: Add a character-cutoff to `build_text`**

`build_text` already iterates `for ch in text.chars()` with `cursor_x`, `spacing`, and an `emitted` counter. Add a parameter `reveal_cutoff: Option<(u32, u32)>` = `(already_consumed, count)` where `already_consumed` is how many *revealable characters* preceded this segment (for multi-line threading) and `count` is the global reveal count. Maintain a local `consumed` starting at `already_consumed`. The cutoff must count characters the same way the native reveal counts (`wcslen` counts every wide char including spaces; `\r`/`\n` do not occur in the single-line `0x102` statics). So increment `consumed` for every `ch` that is not `\r`/`\n` (i.e. before the tab/space/glyph branches advance the cursor), and **before emitting**, if `count` is set and `consumed >= count`, `break` (stop drawing). Return the updated `consumed` count (change return type to `(Vec<SpriteInstance>, u32)`, or take `&mut u32`) so `draw_in_rect` can thread it across wrapped lines. Keep behavior identical when `reveal_cutoff` is `None`.

> Parity note: do NOT re-walk characters outside `build_text` — its advance is proportional (`char_spacing` only when `emitted>0`, per-glyph `pixel_width`, distinct `space_width`/tab paths). Duplicating it elsewhere would drift.

**Step 3: Change `draw_in_rect` signature and thread the window**
```rust
pub fn draw_in_rect(
    font: &BitFont,
    text: &str,
    rect: TextRect,
    color: [f32; 3],
    flags: ShellAlign,
    cam_offset: [f32; 2],
    depth: f32,
    reveal: Option<Reveal>,
) -> ShellTextDraw {
```
In the per-line loop, carry a `consumed: u32` (start 0) across lines and pass `reveal.map(|r| (consumed, r.count))` into `build_text`; update `consumed` from its return. Once `consumed >= reveal.count` after a line, stop adding further lines. When `reveal` is `None`, pass `None` (unchanged output).

**Step 4: Update existing `draw_in_rect` callers to pass `None`**

Add `, None` to the 3 non-test call sites — `src/app_main_menu_shell_render.rs:153`, `src/app_single_player_shell_render.rs:118`, `src/app_skirmish_shell_render/text.rs:201` (inside `push_text_draw`) — and the 5 in-file test calls in `shell_text.rs`.

**Step 5: Add render unit tests**
```rust
#[test]
fn reveal_draws_only_first_count_chars() {
    let font = test_font(); // glyphs x,a,b
    let full = draw_in_rect(&font, "xax", rect_100x30(), [1.0,1.0,1.0],
        ShellAlign::NONE, [0.0,0.0], 0.5, None);
    let revealed = draw_in_rect(&font, "xax", rect_100x30(), [1.0,1.0,1.0],
        ShellAlign::NONE, [0.0,0.0], 0.5, Some(Reveal { count: 2, range: 8 }));
    assert!(revealed.instances.len() < full.instances.len());
    assert!(!revealed.instances.is_empty());
}

#[test]
fn reveal_none_matches_full_draw() {
    let font = test_font();
    let a = draw_in_rect(&font, "xax", rect_100x30(), [1.0,1.0,1.0],
        ShellAlign::NONE, [0.0,0.0], 0.5, None);
    let full_glyphs = font.build_text("xax", 0.0, 0.0, 1.0, 0.5, [1.0,1.0,1.0], [0.0,0.0]);
    assert_eq!(a.instances.len(), full_glyphs.len());
}
```
(Add a `rect_100x30()` helper. If `build_text`'s signature changed, update this call accordingly.)

**Step 6: Verify**
Run: `cargo check -p vera20k` then `cargo test -p vera20k shell_text -- --nocapture` and `cargo test -p vera20k bit_font -- --nocapture`
Expected: compiles (all callers updated); reveal tests pass; existing bit_font tests still pass.

**Step 7: Commit** — `feat(render): character reveal cutoff in build_text/draw_in_rect (wipe v1)`

---

### Task 3: Add reveal fields to `SkirmishShellState`

**Why:** Hold the three labels' reveal cursors in UI state so the trigger and renderer can reach them.

**Files:**
- Modify: `src/ui/skirmish_shell/state/player_name.rs` (struct `SkirmishShellState` at `:218`; manual `impl Default` at `:252`)

**Pattern:** Plain fields; `StaticReveal::default()` is inactive (renders full text).

**Step 1: Add fields to `SkirmishShellState` (struct at `state/player_name.rs:218`)**
```rust
use crate::ui::skirmish_shell::static_reveal::StaticReveal;

// inside pub struct SkirmishShellState { ... }
/// Title static 0x694 reveal cursor.
pub title_reveal: StaticReveal,
/// Game-type static 0x6EC reveal cursor.
pub game_type_reveal: StaticReveal,
/// Map-label static 0x5A8 reveal cursor.
pub map_label_reveal: StaticReveal,
```

**Step 1b: Initialize them in the manual `impl Default for SkirmishShellState` (`:252`)**
This struct has a hand-written `Default` (not derived), so add the three fields there:
```rust
title_reveal: StaticReveal::default(),
game_type_reveal: StaticReveal::default(),
map_label_reveal: StaticReveal::default(),
```

**Step 2: Add a start-all helper**
```rust
impl SkirmishShellState {
    /// Start the 0x4EC->0x4EE reveal for all three right-panel statics using
    /// their current text. Called at shell first-paint slide completion.
    pub fn start_right_panel_static_reveals(
        &mut self,
        title: &str,
        game_type: &str,
        map_label: &str,
        now: std::time::Instant,
    ) {
        self.title_reveal.start(title, now);
        self.game_type_reveal.start(game_type, now);
        self.map_label_reveal.start(map_label, now);
    }

    /// Advance all three reveals one cadence step.
    pub fn advance_right_panel_static_reveals(&mut self, now: std::time::Instant) {
        self.title_reveal.advance(now);
        self.game_type_reveal.advance(now);
        self.map_label_reveal.advance(now);
    }
}
```

**Step 3: Verify**
Run: `cargo check -p vera20k`
Expected: compiles.

**Step 4: Commit** — `feat(skirmish-ui): hold per-static reveal cursors in SkirmishShellState`

---

### Task 4: Trigger reveal at slide completion; advance each frame

**Why:** Wire the native `0x4EC` event. The slide already detects completion in `render_shell_first_paint_slide`; that edge is the start point.

**Files:**
- Modify: `src/app_shell_transition.rs` (the `is_complete` branch in `render_shell_first_paint_slide`, ~lines 296-302; and add an advance call where the slide advances)
- Modify: `src/app_skirmish_shell_render/text.rs` (extract the 3 label strings into a shared helper; thread each reveal window into `push_static_label_draw`)

**Pattern:** Reuses the existing completion edge and Instant cadence already in this module.

**Step 1: Single source of truth for the 3 label strings**

The 3 statics are currently built inline at `src/app_skirmish_shell_render/text.rs:519-552`: title = `localized_label(state, "GUI:SkirmishGame", "Skirmish Game")`; game_type = from `state.skirmish_modes` matched on `shell.selected_mode_id` (fallback `GUI:Battle`); map_label = `maps.get(shell.selected_map_idx).map(|m| m.display_name).unwrap_or("None")`. Extract these into a `pub(crate) fn skirmish_right_panel_label_strings(state: &AppState) -> (String, String, String)` so both the renderer and the reveal-start use one definition. (The renderer keeps calling `push_static_label_draw` with these same strings.)

**Step 2: Start reveals on the Skirmish slide's completion edge**

In `render_shell_first_paint_slide`, where the wave is found complete and cleared (currently `state.shell_first_paint_slide = None;` after `is_complete`), add — only for `ShellSlideKind::Skirmish`:
```rust
if /* wave just completed */ {
    if matches!(kind, ShellSlideKind::Skirmish) {
        let now = Instant::now();
        let (title, game_type, map_label) =
            crate::app_skirmish_shell_render::text::skirmish_right_panel_label_strings(state);
        state.skirmish_shell_state.start_right_panel_static_reveals(
            &title, &game_type, &map_label, now,
        );
    }
    state.shell_first_paint_slide = None;
}
```
(Adjust the module path to wherever the helper lands; confirm `skirmish_shell_state` is the `AppState` field name holding `SkirmishShellState`.)

**Step 3: Thread the reveal window into the static renderer**

In `text.rs`, the 3 `push_static_label_draw` calls (`:520/:534/:546`) now pass each static's `reveal.window()`:
- title → `state.skirmish_shell_state.title_reveal.window()`
- game_type → `…game_type_reveal.window()`
- map_label → `…map_label_reveal.window()`
`push_static_label_draw` and `push_text_draw` gain an `Option<Reveal>` param forwarded to `draw_in_rect`. All other `push_text_draw` / `push_static_label_draw` callers pass `None`.

**Step 4: Advance reveals on the 30 ms cadence**

Once-per-frame, while any Skirmish reveal is active, call `state.skirmish_shell_state.advance_right_panel_static_reveals(Instant::now())`. Put it next to the existing slide `wave.advance(...)` call so it runs on the same render tick (the `advance` is internally 30 ms-gated, so calling it every frame is correct and never over-advances).

**Step 5: Verify (behavioral)**
- `cargo check -p vera20k`.
- Manual/in-game: open Skirmish; after the controls slide in, the title/game-type/map labels reveal left-to-right ~one char per 30 ms (v1: hard wipe, no gradient yet), matching retail YR cadence. (Confirm against gamemd side-by-side.)

**Step 6: Commit** — `feat(skirmish-ui): start static reveal at slide completion and advance it`

---

### Task 5: Restart reveal on map/mode text change

**Why:** Native `0x4B2` text update restarts a running reveal (the player changing map or game type re-triggers the animation on that label).

**Files:**
- Modify: wherever `selected_map_idx` and `selected_mode_id` are mutated. Grep first: `grep -rn "selected_map_idx\s*=" src/` and `grep -rn "selected_mode_id\s*=" src/` (likely in `src/ui/skirmish_shell/` click/input handlers, not necessarily `state.rs`).

**Pattern:** Call `StaticReveal::start` on the affected label when its text changes. Resolve the new label string the same way the renderer does (reuse `skirmish_right_panel_label_strings` from Task 4, or recompute the single affected label).

**Step 1: Restart on map selection change**

After `selected_map_idx` changes, call `self.map_label_reveal.start(<new map label>, Instant::now())`. Match native: restart whether or not the prior reveal had completed (completed reveals still restart). If the mutation site lacks access to `maps`/`Instant`, route the restart through a small method on `SkirmishShellState` that takes the new string + now.

**Step 2: Restart on game-mode change**

After `selected_mode_id` changes, call `self.game_type_reveal.start(<new game type label>, Instant::now())`.

> Title `0x694` is not restarted during ordinary setup (report §3.5 — `0x4B2` restart observed only for `0x6EC`/`0x5A8`). Do NOT restart `title_reveal` on these events.

**Step 3: Add unit tests**
```rust
#[test]
fn map_change_restarts_map_label_reveal_from_count_one() {
    let now = std::time::Instant::now();
    let mut s = SkirmishShellState::default();
    s.map_label_reveal.start("OLD MAP", now);
    for i in 1..=4 { s.map_label_reveal.advance(now + std::time::Duration::from_millis(30 * i)); }
    // simulate selecting a new map -> mutator calls start("NEW MAP", later)
    s.map_label_reveal.start("NEW MAP", now + std::time::Duration::from_millis(500));
    assert_eq!(s.map_label_reveal.window().unwrap().count, 1);
}
```
(If the real mutator is a method, call that method instead of `start` directly so the test exercises the wiring.)

**Step 4: Verify**
Run: `cargo test -p vera20k skirmish_shell -- --nocapture`
Expected: pass.

**Step 5: Commit** — `feat(skirmish-ui): restart map/game-type reveal on text change (0x4B2 parity)`

---

### Task 6: Verification against gamemd.exe

**Why:** Confirm observable parity before declaring done.

**Verify:**
- **Start timing:** reveal begins only after the slide finishes (not during/ before). gamemd: `FUN_006071E0` sends `0x4EC` at the end of the DL=1 slide (`0x00607F95`).
- **Control set:** exactly the title/game-type/map labels animate; the player status child does not. gamemd: `FUN_00602490` → `0x694/0x6EC/0x5A8` for `0x102`.
- **Cadence + duration:** ~one character per 30 ms; a label of N chars finishes in ≈ (N+8)*30 ms. gamemd: report §3.4, target `wcslen+9`.
- **Visual:** left-to-right wipe with a soft highlighted leading edge (last ~8 chars), full color behind it. gamemd: `FUN_00434cd0` first-`count`-chars + `g_SelectedUnitHighlightColor` blend. Read `g_SelectedUnitHighlightColor` from the binary and compare the Rust `highlight` constant + the blend `t` direction to retail in-game; adjust if the gradient leans the wrong way.
- **Restart:** change the map and game type mid/post-reveal → that label restarts from the first character. gamemd: thunk `0x00611C72..0x00611CAF`.
- **No regression:** with reveal inactive (e.g., re-entering an already-shown shell without a fresh slide), labels render full text; no blank right panel.

**How:** `/fidelity-check` the reveal against the cited addresses, plus a side-by-side run of gamemd vs this engine opening Skirmish.

**Commit:** none (verification only) unless adjustments are made.

---

### Task 7 (DEFERRED — separate work item): highlight leading-edge gradient

**Why:** v1 ships the character wipe only. The native trailing-8-char tint is a distinct, undecoded color path; do NOT implement it as a guessed lerp.

**Blocked on decode (do first):**
- `FUN_006612c0` — what it computes from `(intensity, &g_SelectedUnitHighlightColor)`.
- `FUN_004355b0` / `FUN_004355d0` — the base-color → palette-index → final-color path the window chars take.
- Read `g_SelectedUnitHighlightColor` (`read_memory` the global).

**Then:** decide how the palette-indexed intensity table maps onto Rust's RGB-tint `BitFont`. Native math (from `FUN_00434cd0`): a char is in the window when `count - charIndex - 1 < range`; intensity `= (0xFF / range) * (range - (count - charIndex - 1))`, i.e. brightest at the cursor, dimming back over `range`(8) chars; chars before the window draw at full base color. Reproduce exactly or, if Rust's non-palette path can't match, capture retail frames and approximate only with the user's sign-off (per CLAUDE.md, an approximation here needs justification).

**Acceptance:** side-by-side with gamemd shows the same leading-edge tint and direction; `range`=8 window.

## Sources & References

- **Canonical research:** `docs/research/skirmish-ui/SKIRMISH_STATIC_REVEAL_ANIMATION_0X102_GHIDRA_REPORT.md`
- **Trigger/slide context:** `docs/research/skirmish-ui/SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md`, `docs/research/traces/SHELL_SLIDE_COMPLETION_TEXTREVEAL_SOUND_TRACE.md`, `docs/research/traces/SHELL_SLIDE_SWARM_RECONCILIATION.md`
- **gamemd addresses (kept here, not in Rust comments):** `FUN_00602490` (qualifying statics), `OwnerDraw_Static_006153E0` (state/timer/paint advance), `FUN_0060A5B0` (classification defaults), `FUN_00600CA0`/`FUN_006015E0`/`FUN_00601D20` (interval/step/range), `FUN_0060AA60` (0x4EE dispatch), `FUN_00622B50` (0x4EC handler), `FUN_006071E0` `0x00607F95` (0x4EC send), `FUN_00621040`→`FUN_00434cd0` (reveal render: first-`count`-chars cutoff). Restart thunk `0x00611C72..0x00611CAF`. **Gradient (Task 7, undecoded):** `FUN_006612c0`/`FUN_004355b0`/`FUN_004355d0` + global `g_SelectedUnitHighlightColor`.
- **INI:** none (hardcoded shell-classification constants).
- **Related code (verified paths):** `src/app_shell_transition.rs` (`ShellFrameWave`, slide completion ~:296-302), `src/render/shell_text.rs:57` (`draw_in_rect`), `src/render/bit_font.rs:102` (`build_text`), `src/app_skirmish_shell_render/text.rs` (`push_static_label_draw:397`, statics `:520/:534/:546`), `src/ui/skirmish_shell/state/player_name.rs:218` (`SkirmishShellState`; `impl Default` `:252`). Existing `draw_in_rect` callers: `app_main_menu_shell_render.rs:153`, `app_single_player_shell_render.rs:118`.
