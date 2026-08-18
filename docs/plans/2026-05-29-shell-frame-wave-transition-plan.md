# Shell Frame-Index Wave Transition — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Replace the dormant whole-screen pixel-slide+crossfade shell bridge with a faithful
gamemd-native SHP **frame-index wave** transition on the Single Player (0x100) → Skirmish (0x102)
leg, and wire it to the real handoff.

**Architecture:** The transition is a presentation-layer effect only (`src/app_shell_transition.rs`
+ the skirmish shell renderer). It animates each shell control's chrome-SHP **frame index** on a
staggered per-control schedule — no render-target translation, no alpha crossfade. `sim/` is not
touched. The destination dialog paints its own controls through the frame schedule; the source
dialog is already gone (instant teardown), so no two-surface compositing is needed.

**Design Doc:** none — this plan is built directly from this session's decisions (recorded under
Key Technical Decisions) and the two pinned Ghidra reports. We skipped a separate `/brainstorm`
doc by the user's choice; `/review-plan` should treat the low-confidence items below as the
verification gate before code lands.

---

## Grounding Summary

- **Docs say (HIGH confidence on the mechanism):**
  `docs/research/FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md` pins the per-tick schedule:
  a schedule array of per-control entry ticks starting at 1 (the stagger), total ticks =
  `max(schedule)+6` ≈ `N+8` where `N` = visible child-button count, `Sleep(30ms)` per tick
  advancing exactly one frame, and a 4-case per-element frame formula
  (`delta<0`→held-before, `0≤delta<6`→`frame=delta*dir+base`, `delta≥6`→held-terminal),
  `dir=+1` slide-in / `-1` slide-out, base-frame constants in §5.2.
  `docs/research/SHELL_DIALOG_SLIDE_TRANSITION_TRANSFORM_GHIDRA_REPORT.md` proves it is a
  frame-index wave, **not** a positional slide/crossfade; the only spatial move is a single
  discrete **+0x50 (80px)** shift of the SDTP/radar shape above a screen-width threshold, keyed by
  phase not a ramp.
- **Ghidra/code verification this session (corrects the audit):** the existing
  `ShellBridgeTransition` is **dormant** — `start_main_menu_to_skirmish` (the only setter of
  `state.main_menu_to_skirmish_transition = Some(...)`) has **no caller** anywhere in `src/`. The
  live 0x100→0x102 behavior is the instant snap in `App::enter_native_skirmish_from_single_player`
  (`src/app.rs:556`), which already matches gamemd's proven instant-swap. So the audit finding
  `mm-transition-should-not-exist-on-this-path` overstated "the animation code is live"; it is
  reachable-by-symbol but unwired. Net: we are mostly *deleting dead code* and *adding* the real
  effect the user wants.
- **Repo pattern this mirrors:** chrome-SHP frame selection already exists —
  `src/app_skirmish_shell_render/chrome.rs:384 right_panel_button_sdbtnanm_frame_index(pressed,
  disabled)` returns a frame index consumed by `push_right_panel_button_shp`. The atlas
  (`SkirmishShellChromeAtlas`) currently bakes only specific frames (`..._frame2`, `..._frame4`,
  overlay `..._frame10`). The wave needs the full ramp range, so the atlas must bake more frames.
  The skirmish renderer already has a `ShellRenderMode::TransitionPreview` mode
  (`src/app_skirmish_shell_render.rs`) — the wave hooks into that.
- **INI keys:** none drive this animation (it is binary-resident timing/frame constants, not INI).
  `ShellButtonSlideSound` is empty in stock `ini/rules.ini`/`rulesmd.ini` — confirmed, so there is
  **no audible slide cue**; do not add one.
- **Still unknown after grounding (→ Deferred Open Questions):** (1) the *exact* SDBTNANM frame
  sequence/direction for slide-in (the two docs use different conventions: TRANSFORM says
  "10→5 then settle at 1", FRAME_SCHEDULE says base 5 + ascending `delta`); (2) whether the native
  0x100→0x102 click actually invokes the helper at all (OQ-09, needs a live debugger trace);
  (3) the retail SHP frame counts for SDBTNANM/SDMPBTN/SDWRNTMP.

## Key Technical Decisions

- **Frame-index wave, not positional slide.** Animate each control's chrome SHP frame index on the
  staggered schedule. — **Confidence:** high — **Source:**
  `SHELL_DIALOG_SLIDE_TRANSITION_TRANSFORM_GHIDRA_REPORT.md §2/§3`,
  `FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md §5`.
- **Schedule: entry tick = slot_index+1; total ticks = max(schedule)+6; one frame per 30ms,
  never skip an index.** This replaces the current `advance_to` catch-up while-loop (a confirmed
  drift: `mm-transition-per-frame-30ms-tick-source`). — **Confidence:** high — **Source:**
  `FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md §4`.
- **Place on the 0x100→0x102 leg only; never on the 0xE2 main-menu leg.** 0xE2 is proven
  instant-swap (gated by record byte +0xC1, set only by Load/Save). — **Confidence:** high —
  **Source:** `SHELL_TRANSITION_ON_MAIN_MENU_CLICK_GHIDRA_REPORT.md §1/§8`; code at `src/app.rs:556`.
- **Render the destination skirmish shell directly with wave-driven frame indices; delete the
  two-target compositor + wgsl pixel/crossfade.** — **Confidence:** high — **Source:**
  current `shell_transition.wgsl:45-48`, `app_shell_transition.rs:117-181`.
- **EXACT slide-in frame sequence — RESOLVED from binary** (`decompile_function 0x006071E0`,
  2026-05-29). Two button groups, both settling at frame 10:
  - **Group A** ("active", counted by `FUN_0060a180` via predicate `FUN_00608cd0` into
    `DAT_00ac1cac`; also the bottom special button): slide-in = held **1** → ramp **5,6,7,8,9,10**
    (`5+delta`) → held **10**. Slide-out = held 10 → ramp 10..5 (`10-delta`) → held **1**.
  - **Group B** ("inactive", the remaining style-`0x...0B` buttons): slide-in = held **0** → ramp
    **11..16** (`11+delta`) → held **10**. Slide-out base 16.
  — **Confidence:** high. **Source:** `FUN_006071E0` blocks `LAB_006079cf` (group A) /
  `local_14c<iStack_148` (group B); terminal idioms `(-(cVar14!=0)&9)+1` and
  `(-(cVar14!=0)&0xFFFFFFF7)+10`; base `iStack_13c`=5/10, `iStack_114`=0xB/0x10.
- **Skirmish right-panel buttons (Start/Choose/Back, all enabled) map to Group A** for the first
  cut. — **Confidence:** medium — **Source:** group split is binary-verified; the per-button A/B
  classification predicate `FUN_00608cd0` is not yet decoded, but all three are enabled main
  buttons so Group A is the faithful default; confirm before marking parity (Task 3 note).
- **Wave terminal frame 10 hands off to normal idle paint** (frame 2/4) on completion — this is
  gamemd's own behavior (wave paints 10, then `SendMessageA(0x4ED)` and the dialog WM_PAINT
  owner-draws the idle frame). The 10→idle transition is faithful, NOT a bug. — **Confidence:**
  high — **Source:** `FUN_006071E0` epilogue.
- **SDMPBTN/SDWRNTMP draws stay enable-gated (+0xDA/+0xD9) and default OFF.** Only the SDBTNANM
  button-column wave ships in the first cut. — **Confidence:** medium (binding untraced) —
  **Source:** `FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md §10 Q1`.

## Open Questions

### Resolved During Planning
- *Is the existing bridge live?* No — `start_main_menu_to_skirmish` is uncalled; the bridge is
  dead code. (grep: only caller-less definition in `src/app_shell_transition.rs`.)
- *Do we need to composite two shells?* No — the destination dialog paints its own controls; the
  source is torn down instantly.
- *Is there an audible cue?* No — `ShellButtonSlideSound` is empty in stock INI.

### Resolved Post-Review (2026-05-29)
- **Exact SDBTNANM frame sequence** — resolved from `FUN_006071E0` decompile (see Key Technical
  Decisions). Held-before/ramp/held-after constants are now exact for both groups.
- **Button group split** — `FUN_0060a180`/`FUN_0060a250` decompiled: Group A counted into
  `DAT_00ac1cac`, Group B is the remainder; both settle at frame 10. Skirmish buttons → Group A.

### Deferred to Implementation / Review
- **Per-button A/B classification predicate `FUN_00608cd0`.** Not decoded; skirmish's three
  enabled buttons default to Group A (faithful for enabled main buttons). Decode before marking
  parity if any disabled right-panel button can appear during the slide.
- **Trigger reachability (OQ-09).** Whether the native 0x100→0x102 click invokes the helper is
  unproven; this plan ships the mechanism on that leg as the best-faithful placement, labeled
  pending a debugger trace. If the trace later shows no slide, the wave is removed from this leg
  (revert Task 6's trigger), keeping the animator for Load/Save.
- **Retail SHP frame counts.** Handled defensively by clamping the computed frame index to the
  loaded SHP's frame count at draw time (Task 2), but confirm the ramp range exists.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Rewrite | `src/app_shell_transition.rs` | Replace `ShellBridgeTransition` (pixel-slide) with `ShellFrameWave` (frame-index schedule + 4-case formula); keep trigger/consume entry points |
| Delete | `src/render/shell_transition.wgsl` | Pixel-slide+crossfade shader — no longer used |
| Delete/trim | `src/render/shell_transition_pass.rs` | Two-target compositor pass — remove or reduce to nothing referenced |
| Modify | `src/render/mod.rs`, `src/lib.rs` | Drop the deleted shader/pass module wiring |
| Modify | `src/app_skirmish_shell_render/chrome.rs` | Bake the wave's full SDBTNANM frame range into the atlas; add a wave-frame-override entry path |
| Modify | `src/app_skirmish_shell_render.rs` | In `TransitionPreview` mode, drive each right-panel button's SDBTNANM frame from the wave; apply the +80px radar/SDTP shift |
| Modify | `src/app.rs` | `enter_native_skirmish_from_single_player` starts the wave; render call site at ~2287 calls the new render fn; remove dead `start_main_menu_to_skirmish` path |

## Interface Changes

- `app_shell_transition`: replace `ShellBridgeTransition` with `ShellFrameWave` (new pub(crate)
  struct + methods `new_skirmish_slide_in`, `advance`, `frame_for_slot`, `is_complete`). Update the
  single render call site and `AppState.main_menu_to_skirmish_transition` field type. The
  `shell_transition_pass: Option<ShellTransitionPass>` field on `AppState` is removed.
- `app_skirmish_shell_render`: `render_skirmish_shell_to_target(..., ShellRenderMode)` gains access
  to an optional `&ShellFrameWave` (via `AppState`) when mode is `TransitionPreview`.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | Schedule math: entry tick = slot+1, total = max+6, 1 frame/30ms no-skip | Wave shape + duration the player sees on every skirmish entry | Unit tests vs `FUN_006071E0…§4`; in-game cadence |
| Task 1 | 4-case frame formula + `dir` multiplier | Each control's exact frame at each tick | Unit tests vs `…§5` |
| Task 1 | SDBTNANM frame sequence per group (RESOLVED from binary) | Wrong sequence = visibly wrong wave | Unit tests assert exact frames vs `FUN_006071E0` decompile (Group A 1→5..10→10; Group B 0→11..16→10) |
| Task 3 | Skirmish buttons → Group A classification | Wrong group = wrong base frame (5 vs 11) | Default Group A (enabled); decode `FUN_00608cd0` if disabled buttons appear |
| Task 5 | +0x50 (80px) discrete radar/SDTP shift above width threshold | The one real positional move; visible at high res | `…TRANSFORM` doc; in-game at >640 width |
| Task 6 | Trigger placed on 0x100→0x102, never 0xE2 | 0xE2 is proven instant; a transition there is new drift | Confirm no wave on initial main-menu→submenu |
| Task 6 | One-frame-per-30ms, never collapse indices | Current catch-up loop skips frames under load (confirmed drift) | Unit test: slow-tick advances ≤1 index/call |

## Risk Areas

- **Atlas frame baking (Task 2):** if the SDBTNANM SHP lacks a frame the ramp references, the draw
  must clamp, not panic. Regression: render skirmish shell normally (non-transition) still uses
  frame2/frame4 exactly as before.
- **Render call site (`app.rs:2287`):** the new render fn must early-return `false` when no wave is
  active so the normal skirmish shell render path is unaffected.
- **Dead-code removal:** deleting `shell_transition_pass`/`wgsl` must drop all references
  (`render/mod.rs`, `lib.rs`, `app_*`) — `cargo check` catches stragglers.
- **Determinism:** none — presentation only; `sim/` untouched; no state hash impact.

---

## Tasks

### Task 1: `ShellFrameWave` schedule + frame formula (pure logic, tested)

**Why:** The animation math is the parity core; build and test it in isolation before wiring.

**Files:**
- Rewrite: `src/app_shell_transition.rs` (replace `ShellBridgeTransition` block, keep module header
  rewritten to describe the frame-wave; no engine addresses in comments)

**Pattern:** new pattern (frame-schedule animator); mirrors the existing frame-index selection idea
in `app_skirmish_shell_render/chrome.rs`.

**Step 1: Define types + constants**
```rust
//! Single Player -> Skirmish shell frame-index wave transition.
//!
//! Animates each shell control's chrome-SHP frame index on a staggered schedule.
//! No positional slide, no crossfade. Presentation layer only.

use std::time::{Duration, Instant};

/// One animation tick per 30 ms, advancing exactly one frame (never skipped).
pub(crate) const WAVE_TICK_MS: u32 = 30;
/// Extra ticks after the last control's entry so the ramp completes.
pub(crate) const WAVE_TAIL_TICKS: u32 = 6;
/// Linear ramp length (delta 0..=5 inclusive => 6 steps).
pub(crate) const WAVE_RAMP_STEPS: i32 = 6;

/// SDBTNANM frame constants per button group, verified from the binary frame schedule.
/// Each tuple is (held_before, ramp_base, held_after) for slide-IN; slide-OUT swaps
/// held_before<->held_after and negates the ramp direction.
/// Group A = enabled "active" buttons; Group B = the remaining buttons.
pub(crate) struct WaveFrames { pub before: i32, pub base: i32, pub after: i32 }
pub(crate) const GROUP_A_IN: WaveFrames = WaveFrames { before: 1, base: 5, after: 10 };
pub(crate) const GROUP_A_OUT: WaveFrames = WaveFrames { before: 10, base: 10, after: 1 };
pub(crate) const GROUP_B_IN: WaveFrames = WaveFrames { before: 0, base: 11, after: 10 };
pub(crate) const GROUP_B_OUT: WaveFrames = WaveFrames { before: 10, base: 16, after: 0 };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaveDirection { SlideIn, SlideOut }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ButtonGroup { A, B }

impl WaveDirection {
    /// frame multiplier: +1 on slide-in, -1 on slide-out.
    fn dir(self) -> i32 { match self { WaveDirection::SlideIn => 1, WaveDirection::SlideOut => -1 } }
}

#[derive(Debug, Clone)]
pub(crate) struct ShellFrameWave {
    last_step_at: Instant,
    /// 0-based current tick.
    tick: u32,
    /// number of animated control slots (N).
    slot_count: u32,
    /// inclusive max tick = max(entry ticks) + tail.
    total_ticks: u32,
    direction: WaveDirection,
    completion_applied: bool,
}
```

**Step 2: Constructor + schedule derivation**
```rust
impl ShellFrameWave {
    pub(crate) fn new_skirmish_slide_in(slot_count: u32, now: Instant) -> Self {
        Self {
            last_step_at: now,
            tick: 0,
            slot_count,
            total_ticks: Self::total_ticks_for(slot_count),
            direction: WaveDirection::SlideIn,
            completion_applied: false,
        }
    }

    /// Replicates the binary schedule-array build: button slots get entry ticks
    /// 1..=N+1, plus anchor slots; total animation = max(schedule) + WAVE_TAIL_TICKS.
    /// For N animated buttons the max entry is N+2 (the SDMPBTN/radar anchor successor),
    /// so total = N + 2 + 6 = N + 8. Computed explicitly rather than approximated.
    fn total_ticks_for(slot_count: u32) -> u32 {
        // schedule entries: 1..=(slot_count+1) for the button column,
        // plus the anchor successor at (slot_count+1)+1; anchors at 0 do not raise the max.
        let max_entry = slot_count + 2;
        max_entry + WAVE_TAIL_TICKS
    }

    /// Entry tick for a control slot (the stagger): slot 0 enters at tick 1.
    fn entry_tick(slot: u32) -> i32 { slot as i32 + 1 }

    pub(crate) fn is_complete(&self) -> bool { self.tick >= self.total_ticks }

    /// Advance at most ONE tick per call, only once >= 30 ms has elapsed.
    /// Never collapses multiple indices (faithful to one-frame-per-Sleep).
    pub(crate) fn advance(&mut self, now: Instant) {
        let step = Duration::from_millis(u64::from(WAVE_TICK_MS));
        if self.tick < self.total_ticks && now.duration_since(self.last_step_at) >= step {
            self.tick += 1;
            self.last_step_at += step;
        }
    }

    pub(crate) fn mark_completion_applied(&mut self) -> bool {
        if self.completion_applied { return false; }
        self.completion_applied = true;
        true
    }
}
```

**Step 3: 4-case frame formula**
```rust
impl ShellFrameWave {
    /// Frame index for an SDBTNANM button at the current tick.
    /// 4-case: held-before / linear ramp (base + delta*dir) / held-after.
    /// Terminal frames are DISTINCT constants, not `base` (verified from binary).
    pub(crate) fn sdbtnanm_frame(&self, slot: u32, group: ButtonGroup) -> usize {
        let f = match (group, self.direction) {
            (ButtonGroup::A, WaveDirection::SlideIn) => GROUP_A_IN,
            (ButtonGroup::A, WaveDirection::SlideOut) => GROUP_A_OUT,
            (ButtonGroup::B, WaveDirection::SlideIn) => GROUP_B_IN,
            (ButtonGroup::B, WaveDirection::SlideOut) => GROUP_B_OUT,
        };
        let dir = self.direction.dir();
        let delta = self.tick as i32 - Self::entry_tick(slot);
        let frame = if delta < 0 {
            f.before // held at the group's "before" terminal
        } else if delta < WAVE_RAMP_STEPS {
            f.base + delta * dir // 6-step ramp
        } else {
            f.after // held at the group's "after" terminal
        };
        frame.max(0) as usize
    }
}
```

**Step 4: Unit tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_ticks_is_max_schedule_plus_tail() {
        // N=5 buttons => max entry N+2=7, total = 7 + 6 = 13 (≈ N+8).
        let w = ShellFrameWave::new_skirmish_slide_in(5, Instant::now());
        assert_eq!(w.total_ticks, 5 + 2 + WAVE_TAIL_TICKS);
    }

    #[test]
    fn advance_steps_one_frame_per_30ms_and_never_collapses() {
        let t0 = Instant::now();
        let mut w = ShellFrameWave::new_skirmish_slide_in(4, t0);
        w.advance(t0 + Duration::from_millis(29));
        assert_eq!(w.tick, 0);
        w.advance(t0 + Duration::from_millis(30));
        assert_eq!(w.tick, 1);
        // A 1-second gap must still advance only ONE index (no catch-up).
        w.advance(t0 + Duration::from_millis(1030));
        assert_eq!(w.tick, 2);
    }

    #[test]
    fn group_a_slide_in_holds_1_ramps_5_to_10_then_holds_10() {
        let t0 = Instant::now();
        let mut w = ShellFrameWave::new_skirmish_slide_in(3, t0);
        // slot 1 enters at tick 2; before that it holds at the "before" terminal = 1.
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 1);
        for _ in 0..2 { w.tick += 1; } // tick = 2 => delta 0 => base 5
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 5);
        w.tick += 5; // delta 5 => 5 + 5 = 10 (last ramp step)
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 10);
        w.tick += 3; // delta >= 6 => held "after" terminal = 10
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 10);
    }

    #[test]
    fn group_b_slide_in_holds_0_ramps_11_to_16_then_holds_10() {
        let t0 = Instant::now();
        let mut w = ShellFrameWave::new_skirmish_slide_in(3, t0);
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 0); // before-entry (slot 0 enters tick 1)
        w.tick += 1; // delta 0 => base 11
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 11);
        w.tick += 5; // delta 5 => 16
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 16);
        w.tick += 3; // delta >= 6 => held "after" = 10
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 10);
    }
}
```

**Step 5: Verify** — `cargo test -p <crate> shell_transition -- --nocapture` (deferred to the
separate verify pass; do not block here). Expected: PASS.

**Step 6: Commit** — `wip: shell frame-wave schedule + formula`.

---

### Task 2: Bake the wave's SDBTNANM frame range into the chrome atlas

**Why:** The wave references frames 5..=10; the atlas currently bakes only frame2/frame4/overlay10.
Draws must clamp to the SHP's real frame count, never panic.

**Files:**
- Modify: `src/app_skirmish_shell_render/chrome.rs` (atlas struct + bake site)

**Step 1:** Locate `SkirmishShellChromeAtlas` and its SDBTNANM bake. Add a contiguous array
`right_panel_button_sdbtnanm_frames: [Option<SkirmishShellChromeEntry>; 17]` (indices 0..=16 — Group
A uses 1/5..10, Group B uses 0/11..16, both verified from the frame schedule) baked from the loaded
SHP, each `None` if the SHP lacks that frame.

**Step 2:** Add a lookup:
```rust
pub(super) fn right_panel_button_sdbtnanm_frame(
    atlas: &SkirmishShellChromeAtlas,
    frame: usize,
) -> Option<SkirmishShellChromeEntry> {
    atlas.right_panel_button_sdbtnanm_frames.get(frame).copied().flatten()
}
```

**Step 3:** Keep `right_panel_button_sdbtnanm_frame_index(pressed, disabled)` and the existing
`push_right_panel_button_shp` exactly as-is for the **non-transition** path (frame2/frame4). The
wave path uses the new array lookup. No behavior change off-transition.

**Step 4: Verify** — `cargo check`. Expected: compiles; normal skirmish render unchanged.

**Step 5: Commit** — `wip: bake full sdbtnanm frame range for shell wave`.

---

### Task 3: Drive right-panel button frames from the wave in `TransitionPreview`

**Why:** This is where the wave becomes visible — each button shows its scheduled frame.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs` (the `TransitionPreview` branch + button emit loop)
- Modify: `src/app_skirmish_shell_render/chrome.rs` (a `push_right_panel_button_wave` helper)

**Step 1:** Add:
```rust
pub(super) fn push_right_panel_button_wave(
    out: &mut Vec<SpriteInstance>,
    atlas: &SkirmishShellChromeAtlas,
    rect: RectPx,
    frame: usize,
    depth: f32,
) {
    match right_panel_button_sdbtnanm_frame(atlas, frame)
        .or_else(|| right_panel_button_sdbtnanm_frame(atlas, frame.saturating_sub(1)))
    {
        Some(entry) => push_entry(out, entry, rect, depth),
        None => { /* SHP lacks the frame: hold last available; clamp handled above */ }
    }
}
```

**Step 2:** In the right-panel button emit loop, when `mode == TransitionPreview` and a
`&ShellFrameWave` is available, assign each button a stable slot index (its dialog/child order)
and call `wave.sdbtnanm_frame(slot, ButtonGroup::A)` -> `push_right_panel_button_wave`. Otherwise
use the existing `push_right_panel_button_shp`. The three skirmish right-panel buttons
(Start/Choose/Back) are all enabled main buttons → Group A (held 1 → ramp 5..10 → held 10). If a
disabled button can appear during the slide, decode `FUN_00608cd0` to classify it as Group B
(held 0 → ramp 11..16 → held 10) — deferred, see Open Questions.

**Step 3:** Slot ordering must be the native child order (top-to-bottom of the right column). Use
the existing button layout order; document that the slot index = position in that ordered list.
On completion the buttons hold at frame 10 for one frame, then Task 4's completion hands back to
the normal idle render (frame 2/4) — this 10→idle handoff is faithful (gamemd repaints via
WM_PAINT after `0x4ED`), not a glitch.

**Step 4: Verify** — `cargo check`. Expected: compiles.

**Step 5: Commit** — `wip: wave-driven right-panel button frames in transition preview`.

---

### Task 4: New render entry point (replace the compositor render)

**Why:** Render the destination skirmish shell directly with wave frames; no two-target blend.

**Files:**
- Rewrite: render fn in `src/app_shell_transition.rs` (was
  `render_main_menu_to_skirmish_transition`); keep the same name + signature so `app.rs:2287` is a
  one-line change.

**Step 1:** New body:
```rust
pub(crate) fn render_main_menu_to_skirmish_transition(
    state: &mut AppState,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
) -> Result<bool> {
    if state.main_menu_to_skirmish_transition.is_none() {
        return Ok(false);
    }
    crate::app::App::ensure_skirmish_shell_chrome(state);
    if state.skirmish_shell_chrome.is_none() {
        state.main_menu_to_skirmish_transition = None;
        return Ok(false);
    }
    if let Some(wave) = state.main_menu_to_skirmish_transition.as_mut() {
        wave.advance(std::time::Instant::now());
    }
    // Destination shell paints itself; the wave supplies per-button frames.
    crate::app_skirmish_shell_render::render_skirmish_shell_to_target(
        state, encoder, target,
        crate::app_skirmish_shell_render::ShellRenderMode::TransitionPreview,
    )?;
    if state.main_menu_to_skirmish_transition.as_ref().is_some_and(ShellFrameWave::is_complete) {
        complete_skirmish_slide_in(state);
    }
    Ok(true)
}

fn complete_skirmish_slide_in(state: &mut AppState) {
    let Some(mut wave) = state.main_menu_to_skirmish_transition.take() else { return; };
    if !wave.mark_completion_applied() { return; }
    state.main_menu_show_native_skirmish_shell = true;
    crate::app::App::ensure_skirmish_shell_chrome(state);
}
```

**Step 2:** Remove `ShellTransitionPass` usage and the `shell_transition_pass` field reads here.

**Step 3: Verify** — `cargo check` (will fail until Task 7 removes the field; that's expected — do
Tasks 4→7 before checking).

**Step 4: Commit** — `wip: direct destination-render for shell wave`.

---

### Task 5: Discrete +80px radar/SDTP shift (high-res only)

**Why:** The single real positional move in the native transition; visible above the width
threshold.

**Files:**
- Modify: `src/app_skirmish_shell_render.rs` (top preview/SDTP emit, transition mode)

**Step 1:** In `TransitionPreview`, while the wave is mid-flight (not complete), offset the
SDTP/radar shape draw X by `+80` px **only** when the shell render width exceeds the documented
threshold; snap to 0 on completion. Gate behind a named const `RADAR_TRANSITION_SHIFT_PX: i32 = 80`.

**Step 2:** If the skirmish shell does not draw an SDTP/radar shape at this stage, skip with a
`log::debug!` note and record in the plan's deferred list (do not fabricate a shape). Confirm by
reading the SDTP draw in `app_skirmish_shell_render.rs` first.

**Step 3: Verify** — `cargo check`.

**Step 4: Commit** — `wip: radar/SDTP discrete shift during shell wave`.

---

### Task 6: Wire the trigger into the real handoff

**Why:** Start the wave on Single Player → Skirmish instead of the instant snap.

**Files:**
- Modify: `src/app.rs:556` (`enter_native_skirmish_from_single_player`)
- Modify: `src/app_shell_transition.rs` (trigger helper)

**Step 1:** Replace the dead `start_main_menu_to_skirmish*` with:
```rust
pub(crate) fn start_skirmish_slide_in(state: &mut AppState, slot_count: u32) {
    state.main_menu_to_skirmish_transition =
        Some(ShellFrameWave::new_skirmish_slide_in(slot_count, std::time::Instant::now()));
}
```

**Step 2:** In `enter_native_skirmish_from_single_player`, after `ensure_skirmish_shell_chrome`,
compute the right-panel button count `N` (the animated slots) and call
`crate::app_shell_transition::start_skirmish_slide_in(state, n)` instead of leaving
`main_menu_to_skirmish_transition = None`. Keep `main_menu_show_native_skirmish_shell = true` so the
shell is the active screen; the wave just controls button frames for its duration.

**Step 3:** Do **not** add any trigger on the 0xE2 main-menu leg. Add a code-level assertion/comment
(no engine address) that this leg stays instant.

**Step 4: Verify** — `cargo check`.

**Step 5: Commit** — `feat: shell frame-wave on single-player -> skirmish`.

---

### Task 7: Delete the dead pixel-slide compositor

**Why:** Remove the now-unused crossfade shader, two-target pass, and `AppState` field.

**Files:**
- Delete: `src/render/shell_transition.wgsl`
- Delete: `src/render/shell_transition_pass.rs`
- Modify: `src/render/mod.rs`, `src/lib.rs` (drop module decls)
- Modify: `src/app.rs` (remove `shell_transition_pass` field + all `= None` resets at lines
  ~515/540/560/569/593/618 that referenced it; keep the `main_menu_to_skirmish_transition` resets)
- Modify: `src/app_main_menu_shell_render.rs` (drop the `source_render_target` transition usage)

**Step 1:** Remove the field `shell_transition_pass` from `AppState` and every reference.

**Step 2:** Remove `ResizeTransitionResolution`/`resolve_resize` if now unused (grep first; the
resize-mid-transition policy for a frame wave is "let it finish" — no half-way snap needed).

**Step 3: Verify** — `cargo check`. Expected: compiles with no dangling references.

**Step 4: Commit** — `chore: remove dead shell pixel-slide compositor`.

---

### Task 8: Verification pass (separate, bounded, foreground)

**Why:** Per project rule, slow cargo runs are a separate pass, not buried in the work.

**Verify:**
- `cargo check` — clean.
- `cargo test shell` and `cargo test skirmish_shell` — the Task 1 schedule/formula tests pass;
  pre-existing skirmish render tests still pass (e.g. `right_panel_buttons_use_sdbtnanm_type1_frames`
  must remain green — proves the non-transition path is unchanged).
- **In-game:** launch, Single Player → Skirmish; confirm the right-column buttons animate as a
  staggered frame wave (top-to-bottom), ~`(N+8)*30ms` total, no horizontal screen slide, no
  crossfade, no slide sound; the initial main-menu → Single Player step stays an instant swap.
- **Parity caveat to record in the commit/notes:** the exact SDBTNANM frame sequence (Task 1
  LOW-confidence constants) and trigger reachability (OQ-09) remain to be confirmed against the
  retail SHP and a debugger trace before this is marked parity-confirmed.

---

## Sources & References

- **Ghidra reports:** `docs/research/FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md`
  (frame schedule + formula, §4/§5/§7), `docs/research/SHELL_DIALOG_SLIDE_TRANSITION_TRANSFORM_GHIDRA_REPORT.md`
  (frame-wave-not-slide proof + +0x50 shift), `docs/research/SHELL_TRANSITION_ON_MAIN_MENU_CLICK_GHIDRA_REPORT.md`
  (0xE2 = instant swap, +0xC1 gate), `docs/research/SHELL_MENU_TRANSITION_SYSTEM_MODEL_SYNTHESIS.md`
  (route 0x683→1→0x100→0x579→0x0B→0x102).
- **Audit findings:** `docs/research/UI_PARITY_AUDIT_2026_05_29.findings.json` — ids
  `mm-transition-mechanism-is-pixel-slide-crossfade-not-shp-frame-wave`,
  `mm-transition-per-frame-30ms-tick-source`, `mm-transition-should-not-exist-on-this-path`
  (note: "code is live" claim corrected here — bridge is unwired).
- **Current code:** `src/app_shell_transition.rs`, `src/render/shell_transition.wgsl`,
  `src/render/shell_transition_pass.rs`, `src/app.rs:556` / `:1603` / `:2287`,
  `src/app_skirmish_shell_render.rs`, `src/app_skirmish_shell_render/chrome.rs:363-389`.
- **INI:** `ShellButtonSlideSound` empty in `ini/rules.ini` / `ini/rulesmd.ini` (no audible cue).
- **Binary addresses** (kept here, not in `.rs`): `FUN_006071E0` (frame schedule + 4-case formula;
  `decompile_function 0x006071E0`, 2026-05-29 — verified: Group A `LAB_006079cf` held=1/ramp 5+delta/
  after=10; Group B held=0/ramp 11+delta/after=10; `+0x50` shift gated by `g_ScreenWidth >=
  DAT_007f5be4`; `Sleep(0x1e)` one frame/iter; epilogue `0x4ED` in / `0x4EC`+`VocClass__PlayAtPos`
  out), caller `FUN_00608260 @0x00608343`, group-count callbacks `FUN_0060a180` (→`DAT_00ac1cac`,
  predicate `FUN_00608cd0`) / `FUN_0060a250` (→`DAT_00ac4894`, predicate `FUN_00609730`), gate byte
  +0xC1 setter in `CDFileClass__Constructor`. Cite in docs only.
