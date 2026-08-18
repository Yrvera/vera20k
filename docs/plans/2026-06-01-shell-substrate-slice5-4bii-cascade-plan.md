# Slice 5 sub-step 4b-ii — graceful quit cascade Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained. ONE sub-step per cycle
> (4b-ii-a, then 4b-ii-b); build + full test as a separate foreground pass; STOP for the user's
> in-game OK before committing each sub-step to `dev`.

**Goal:** After the main-menu Exit-confirm OK (and after 4b-i's settings persist), reproduce
gamemd's graceful quit teardown — music fades out while any trailing voice plays, music hard-stops,
the screen fades to black — before the window closes, via a non-blocking per-frame state machine.

**Architecture:** A new app-layer `QuitCascade` state machine (mirrors `app_shell_transition::
ShellFrameWave`) is ticked once per frame in `App::render_frame`. It owns timing only; the app
applies its per-frame directives (lower music volume, hard-stop music, exit) and reads its fade
alpha for a full-screen black overlay. No `sim/` involvement; no blocking of the winit loop.

**Design Doc:** `docs/plans/2026-06-01-shell-substrate-slice5-4bii-cascade-design.md`

---

## Grounding Summary

- **Binary (PROOFED this session, Ghidra MCP):** teardown is `Main_Game @0x0052D9A0` case 7 —
  `Theme::Stop(1)` (non-blocking fade: sets music volume-interpolator target→0, rate full-scale
  over **1000 ms**) → vox-wait loop (exits the instant **music-fade-done OR voices-done OR
  `0xBB8`≈48 s ceiling**; pumps sound/net, no render, no Sleep) → `Theme::Stop(0)` (hard stop) →
  `FUN_004A3C30(blackPalette, 0x1E ticks)` palette **fade-to-black ~480 ms** (30 GetRadarTimer
  ticks × 16 ms) → `RET 0`. Full citations in the design doc §1.
- **Repo pattern mirrored:** `app_shell_transition::ShellFrameWave` (`src/app_shell_transition.rs:109`)
  — an `Instant`-based, per-frame, non-blocking phase animation; its `blocks_shell_input`
  (`:207`) already gates the four shell input sites. The continuous-redraw loop
  (`about_to_wait → request_redraw`, `app.rs:2250`) self-drives the machine.
- **Port facilities (verified):** `MusicPlayer::set_volume`/`stop`/`volume` (`src/audio/music.rs:239/184/247`);
  `SfxPlayer` voice slot (`voice_player`, `queued_voice`, `src/audio/sfx.rs:129/131`) with the
  non-blocking predicate `voice_player…!empty()` already used privately (`sfx.rs:367`); the baked
  1×1 opaque `white_pixel` in the skirmish chrome atlas (`skirmish_shell_chrome.rs:84`) + the
  ALPHA_BLENDING passthrough pipeline (`batch.rs`, `draw_with_buffer_passthrough`) for the overlay.
- **INI keys:** none — the cascade timing is hardcoded in gamemd, not INI-driven.
- **Still unknown after grounding:** the exact music-gate-vs-voice interplay (does a trailing voice
  longer than the ~≤1 s music fade get cut, or wait to the 48 s ceiling?) — see Deferred Questions.

## Key Technical Decisions

- **Non-blocking per-frame state machine, not a busy-spin.** — gamemd blocks the thread; the port
  must keep the winit loop turning. Reproduce the *observable* timing across frames.
  **Confidence:** high — **Source:** design doc §2; repo pattern `app_shell_transition.rs:109`.
- **`QuitCascade` is pure (no audio/render deps); the app applies its directives.** Keeps it
  headless-testable with injected `Instant`s + a `voices_active` bool. **Confidence:** high —
  **Source:** mirrors `ShellFrameWave` separation.
- **Wait ends on music-fade-done OR voices-done OR 48 s ceiling (Model A).** The disassembly shows
  `JZ`-exit on each gate (`Main_Game` @0x0052E79C–E7E4); music fades within ≤1 s so the wait is
  effectively fade-bounded, with voices-done as an early-exit. **Confidence:** medium — the music
  gate `FUN_00720FD0`'s "conditional stop" wasn't fully decoded; flagged for in-game verify.
  **Source:** Ghidra `disassemble_function 0x0052D9A0`, `decompile_function 0x00720FD0`.
- **Music fade rate = 1.0 per 1000 ms (volume-proportional).** **Confidence:** high (PROOFED) —
  **Source:** rate setter `FUN_004071a0` sole caller divisor 1000; ms timestamps via
  `Timer__InitPerformanceCounter 0x00409393`.
- **Screen fade ~480 ms, linear, to black.** **Confidence:** high (PROOFED) — **Source:**
  `disassemble_function 0x004A3C30` (`0x1E` ticks × `GetRadarTimer 0x006C8C40` 16 ms/tick;
  `read_memory 0x00884E80` = all-zero black palette).
- **Screen-fade palettized-mode gate is dropped (port always fades).** `DAT_008175b0`/`vtable+0x70`
  is a DirectDraw 8-bit-mode artifact; standard YR menu is palettized → fade runs; the wgpu port has
  no analog, so reproduce the standard observable. **Confidence:** high — **Source:** design §1 fact 9.
- **Black overlay drawn LAST (after the cursor).** gamemd's palette fade blackens the whole
  framebuffer including the cursor. **Confidence:** high — **Source:** design §1; render map.

## Open Questions

### Resolved During Planning
- *Music fade blocks the cascade?* No — `Theme::Stop(1)` is non-blocking (sets target, returns);
  the fade runs concurrently with the wait. (Ghidra `decompile_function 0x004080c0/0x00407170`.)
- *Is `0x1E` a step count?* No — it's a **duration in GetRadarTimer ticks** ⇒ ~480 ms.
- *Where does the menu-theme restart risk live?* `render_frame` re-asserts `play_menu_theme` every
  frame (`app.rs:2710-2715`); must be gated off during the cascade or it restarts music after the
  hard stop (Task 6).

### Deferred to Implementation / In-Game
- **Trailing-voice cut vs. wait-to-ceiling** (Model A vs B): does a main-menu quit even have a voice
  playing, and if one is longer than the ~≤1 s music fade, does gamemd cut it? Confirm in-game; if it
  waits, switch the phase-end to voices-done-only (the ceiling becomes meaningful). LOW.
- **egui-fallback screen fade**: the `white_pixel` atlas is absent on the degraded fallback path;
  the overlay only draws on the SHP path. The fallback runs the audio phases + the 480 ms timer with
  no visible black (a brief hold), then exits. Accepted for the rare broken-assets path. LOW.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `src/app_quit_cascade.rs` | The `QuitCascade` non-blocking phase machine + directives |
| Modify | `src/lib.rs:51` | Register `pub mod app_quit_cascade;` |
| Modify | `src/audio/sfx.rs:478` | Add `SfxPlayer::voices_active()` |
| Modify | `src/app.rs` (AppState struct ~194; init ~2524; `render_frame` 2641; OK handlers 1679/1934) | Field, helper, tick, interception |
| Modify | `src/app_shell_transition.rs:207` | Extend `blocks_shell_input` to freeze input during the cascade |
| Modify | `src/app_main_menu_shell_render.rs:573` | Draw the full-screen black fade overlay (4b-ii-b) |

## Interface Changes

- **New** `app_quit_cascade::QuitCascade` + `QuitCascadeTick` (pub(crate)). Consumed only by `app.rs`.
- **New** `SfxPlayer::voices_active(&self) -> bool` (pub). Consumed by `app.rs` cascade tick.
- **New** `AppState.quit_cascade: Option<QuitCascade>` field. Read by `render_frame` and
  `blocks_shell_input`; the design's render path reads `overlay_alpha()` (4b-ii-b).
- **Changed behavior** of `App::draw_main_menu_dialogs` egui Confirm arm: returns `false` (starts the
  cascade) instead of `true`; the `confirm_quit` consumers (`app.rs:755`, `2779`) no longer exit on
  the quit-confirm — the cascade owns exit. No signature change.
- **Changed** `app_shell_transition::blocks_shell_input` now also returns true while a cascade runs.

## Risk Areas

- **`render_frame` is the highest-blast-radius file.** The cascade tick is gated on
  `quit_cascade.is_some()`, so off the quit path everything is unchanged. Regression: confirm normal
  quit (no cascade artifacts), normal menu music, and in-game paths are untouched.
- **Menu-theme restart after hard stop** (Task 6 gate) — if missed, music restarts mid-cascade.
  Covered by the gate + the in-game check.
- **Borrow conflicts** in the tick (cascade vs music_player vs sfx_player): compute `voices_active`
  before borrowing the cascade mutably (Task 6 code does this).
- **egui-fallback exit path** must not double-exit: the Confirm arm now returns false; verify the
  fallback caller no longer calls `event_loop.exit()` for the quit-confirm.

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| 2 | Music fade rate (1.0/1000 ms, volume-proportional) | The audible fade-out length must match gamemd (~0.4 s at default vol) | Ghidra rate proof (design §1); in-game listen |
| 2 | Wait-end = fade-done OR voices-done OR 48 s | Determines how long trailing audio breathes before teardown | Ghidra `0x0052D9A0` OR-exit; in-game |
| 6 | Persist runs strictly before the cascade mutates volume | The saved ScoreVolume must be the pre-fade value, not 0 | Order in OK handler (Task 5); 4b-i §7.C |
| 8 | Screen fade ~480 ms, linear, to black | The visible fade-to-black duration/curve before the window closes | Ghidra `0x004A3C30` (design §1); in-game |
| 9 | Black overlay covers everything incl. cursor, drawn last | gamemd's palette fade blackens the whole framebuffer | In-game observation |

---

## Tasks — 4b-ii-a (audio cascade: fade + vox-wait + hard stop + exit)

### Task 1: `SfxPlayer::voices_active()`

**Why:** The cascade needs a non-blocking "any EVA/voice still playing" poll; the predicate exists
privately but isn't exposed.

**Files:** Modify `src/audio/sfx.rs` (after `queued_voice_count`, :478).

**Pattern:** mirrors the private liveness check at `sfx.rs:367`
(`voice_player.as_ref().is_some_and(|p| !p.empty())`).

**Step 1: Add the method**
```rust
// src/audio/sfx.rs — inside `impl SfxPlayer`, right after `queued_voice_count`
    /// Whether any EVA/voice line is still playing or waiting in the voice queue.
    /// Non-blocking (rodio `Player::empty()` is a poll). Used by the quit cascade
    /// to wait for trailing voices before tearing down.
    pub fn voices_active(&self) -> bool {
        self.voice_player.as_ref().is_some_and(|p| !p.empty()) || !self.queued_voice.is_empty()
    }
```

**Step 2: Add tests**
```rust
// src/audio/sfx.rs — inside `#[cfg(test)] mod tests` (add module if absent)
    #[test]
    fn voices_active_false_when_idle() {
        let Some(player) = SfxPlayer::new() else { return }; // no audio device in CI → skip
        assert!(!player.voices_active());
    }
```
> Note: `SfxPlayer::new()` opens an audio device; the test early-returns when none is available so
> CI without audio stays green. The predicate is pure boolean logic over two fields; the idle case
> is the meaningful assertion.

**Step 3: Verify** — `cargo test -p vera20k --lib voices_active` → PASS.

**Step 4: Commit.**

### Task 2: `QuitCascade` state machine (2-phase: fade+wait → exit)

**Why:** The core non-blocking teardown timer. Built first so the app integration has a tested
machine to drive.

**Files:** Create `src/app_quit_cascade.rs`; modify `src/lib.rs:51` (register module).

**Pattern:** mirrors `app_shell_transition::ShellFrameWave` (`Instant`-based, per-frame, pure).

**Step 1: Register the module**
```rust
// src/lib.rs — next to `pub mod app_shell_transition;` (:51)
pub mod app_quit_cascade;
```

**Step 2: Write the machine + directives**
```rust
// src/app_quit_cascade.rs
//! Graceful main-menu quit cascade (presentation/teardown only).
//!
//! A non-blocking per-frame state machine, modelled on
//! [`crate::app_shell_transition::ShellFrameWave`]: it owns timing only and
//! returns the effects the app applies each frame (lower music volume, hard-stop
//! music, exit). The original runs this teardown as a blocking thread spin; the
//! port reproduces the same observable timing across frames so the winit event
//! loop keeps turning. Reproduces: music fade-out (concurrent with a bounded wait
//! for trailing voices) → music hard-stop → exit. The screen fade-to-black phase
//! is added in sub-step 4b-ii-b.
//!
//! ## Dependency rules
//! - App layer; depends only on std. No render/sim/audio type deps (the app maps
//!   directives onto MusicPlayer/SfxPlayer), so it stays headless-testable.

use std::time::Instant;

/// Music volume-fade rate: full scale (1.0) over 1000 ms, matching the original's
/// theme stop-with-fade (a full-scale volume interpolator over a 1000 ms divisor).
/// A fade from volume `v` therefore reaches silence in `v * 1000` ms.
const MUSIC_FADE_PER_MS: f64 = 1.0 / 1000.0;

/// Safety ceiling on the fade + trailing-voice wait. The original bounds its vox
/// pump-wait at `0xBB8` GetRadarTimer ticks (3000 × 16 ms). Effectively dominated
/// by the ≤1 s music fade, so this is reached only if audio never reports done.
const WAIT_CEILING_MS: u64 = 0xBB8 * 16; // 48_000 ms

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuitPhase {
    /// Music fading out while any trailing EVA/menu voice plays. Ends the instant
    /// the music fade completes OR voices finish OR the ceiling is hit.
    FadeMusicAndWaitVoices,
    /// Terminal — the app exits the event loop.
    Done,
}

/// Per-frame effects the app applies after ticking the cascade.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct QuitCascadeTick {
    /// New music volume to apply this frame (None once the fade is over).
    pub music_volume: Option<f64>,
    /// Hard-stop the music this frame (one-shot, on the fade→teardown edge).
    pub stop_music: bool,
    /// The cascade has finished — exit the event loop.
    pub finished: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct QuitCascade {
    phase: QuitPhase,
    phase_started_at: Instant,
    /// Live music volume captured when the cascade began (the fade start point).
    start_music_volume: f64,
}

impl QuitCascade {
    /// Begin the cascade. `start_music_volume` is the live music volume at quit
    /// time (the caller has already persisted it before calling this).
    pub(crate) fn start(now: Instant, start_music_volume: f64) -> Self {
        Self {
            phase: QuitPhase::FadeMusicAndWaitVoices,
            phase_started_at: now,
            start_music_volume: start_music_volume.clamp(0.0, 1.0),
        }
    }

    /// Advance the cascade and return the effects to apply this frame.
    /// `voices_active` is the live "any EVA/voice still playing" poll.
    pub(crate) fn tick(&mut self, now: Instant, voices_active: bool) -> QuitCascadeTick {
        let elapsed_ms = now.duration_since(self.phase_started_at).as_millis() as u64;
        match self.phase {
            QuitPhase::FadeMusicAndWaitVoices => {
                let faded =
                    (self.start_music_volume - elapsed_ms as f64 * MUSIC_FADE_PER_MS).max(0.0);
                let music_done = faded <= 0.0;
                if music_done || !voices_active || elapsed_ms >= WAIT_CEILING_MS {
                    self.enter(QuitPhase::Done, now);
                    // 4b-ii-a: hard-stop music and exit on the same edge (the
                    // screen fade is inserted between these in 4b-ii-b).
                    return QuitCascadeTick { stop_music: true, finished: true, ..Default::default() };
                }
                QuitCascadeTick { music_volume: Some(faded), ..Default::default() }
            }
            QuitPhase::Done => QuitCascadeTick { finished: true, ..Default::default() },
        }
    }

    fn enter(&mut self, phase: QuitPhase, now: Instant) {
        self.phase = phase;
        self.phase_started_at = now;
    }
}
```

**Step 3: Add tests**
```rust
// src/app_quit_cascade.rs — append
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(base: Instant, ms: u64) -> Instant {
        base + Duration::from_millis(ms)
    }

    /// The music volume ramps linearly toward 0 while voices stay active.
    #[test]
    fn music_volume_ramps_down_linearly() {
        let t0 = Instant::now();
        let mut c = QuitCascade::start(t0, 1.0);
        let mid = c.tick(at(t0, 500), true);
        assert_eq!(mid.music_volume, Some(0.5));
        assert!(!mid.finished && !mid.stop_music);
        let near = c.tick(at(t0, 900), true);
        assert!((near.music_volume.unwrap() - 0.1).abs() < 1e-9);
    }

    /// When the fade reaches silence, the cascade hard-stops music and finishes.
    #[test]
    fn fade_completion_stops_music_and_finishes() {
        let t0 = Instant::now();
        let mut c = QuitCascade::start(t0, 1.0);
        let end = c.tick(at(t0, 1000), true);
        assert!(end.stop_music && end.finished);
        // Stays finished thereafter.
        assert!(c.tick(at(t0, 1001), true).finished);
    }

    /// A trailing voice finishing early ends the wait before the fade completes.
    #[test]
    fn voices_done_ends_wait_early() {
        let t0 = Instant::now();
        let mut c = QuitCascade::start(t0, 1.0);
        let end = c.tick(at(t0, 200), false); // voices already done
        assert!(end.stop_music && end.finished);
    }

    /// Default menu volume (0.4) fades to silence in ~400 ms.
    #[test]
    fn default_volume_fades_in_400ms() {
        let t0 = Instant::now();
        let mut c = QuitCascade::start(t0, 0.4);
        assert_eq!(c.tick(at(t0, 200), true).music_volume, Some(0.2));
        assert!(c.tick(at(t0, 400), true).finished);
    }
}
```
> Simplify the `near` assertion when typing it in — the intent is `assert!((near.music_volume
> .unwrap() - 0.1).abs() < 1e-9)`; drop the redundant first line.

**Step 4: Verify** — `cargo test -p vera20k --lib app_quit_cascade` → PASS (4 tests).

**Step 5: Commit.**

### Task 3: AppState field + `start_quit_cascade` helper

**Why:** Holds the live cascade and starts it from the OK handlers.

**Files:** Modify `src/app.rs` — struct field (after `shell_slide_active_shell`, ~:198), init
(in the `AppState { … }` literal near `shell_first_paint_slide: None`, ~:2524), helper near the
exit-confirm handlers (after `persist_settings_on_quit`).

**Step 1: Add the field**
```rust
// src/app.rs — in the AppState struct, after `shell_slide_active_shell` (~:198)
    /// Active graceful quit cascade (music fade → trailing-voice wait → hard stop →
    /// screen fade → exit). Some only between Exit-confirm OK and window close;
    /// freezes shell input while it runs.
    pub(crate) quit_cascade: Option<crate::app_quit_cascade::QuitCascade>,
```

**Step 2: Initialise it**
```rust
// src/app.rs — in the AppState { … } construction, next to `shell_first_paint_slide: None,` (~:2524)
            quit_cascade: None,
```

**Step 3: Add the start helper**
```rust
// src/app.rs — right after `fn persist_settings_on_quit(...)`
    /// Begin the graceful quit cascade from the main-menu Exit-confirm OK. The
    /// caller persists settings FIRST (so the captured volume is pre-fade), then
    /// calls this instead of exiting immediately; `render_frame` drives it to
    /// completion and then exits the event loop.
    fn start_quit_cascade(state: &mut AppState) {
        let start_volume = state.music_player.as_ref().map_or(0.0, |p| p.volume());
        state.quit_cascade = Some(crate::app_quit_cascade::QuitCascade::start(
            Instant::now(),
            start_volume,
        ));
    }
```

**Step 4: Verify** — `cargo check -p vera20k` → compiles (helper unused warning until Task 5 is OK).

**Step 5: Commit** (with Task 4/5, or stage and continue — small, no behavior change yet).

### Task 4: Freeze shell input during the cascade

**Why:** gamemd processes no input during the (blocking) teardown; the port must ignore clicks/keys
so a stray input can't re-enter the menu mid-fade.

**Files:** Modify `src/app_shell_transition.rs:207`.

**Pattern:** extends the existing gate that already covers all four input sites
(`app.rs:2062/2144/2182/2245`).

**Step 1: Extend the predicate**
```rust
// src/app_shell_transition.rs — replace the body of blocks_shell_input
pub(crate) fn blocks_shell_input(state: &AppState) -> bool {
    state.quit_cascade.is_some()
        || transition_blocks_shell_input(state.shell_first_paint_slide.as_ref())
}
```

**Step 2: Verify** — `cargo check -p vera20k` → compiles.

**Step 3: Commit** (group with Task 3/5).

### Task 5: Intercept the OK handlers to start the cascade

**Why:** Replace the immediate exit on quit-confirm OK with the cascade, on both the SHP and
egui-fallback paths. Persist stays strictly first.

**Files:** Modify `src/app.rs` — SHP OK arm (1679-1683), egui Confirm arm (1934-1941).

**Step 1: SHP path** — replace the OK arm body:
```rust
// src/app.rs:1679 — handle_exit_confirm_modal_mouse_up, OK arm
            // OK -> quit (result 0). Persist settings to RA2MD.INI BEFORE teardown
            // (4b-i), then run the graceful cascade (music fade → trailing-voice
            // wait → hard stop → screen fade → exit) instead of exiting immediately.
            Some(id) if id == crate::ui::shell::modal::control::OK => {
                Self::persist_settings_on_quit(state);
                state.exit_confirm_modal = None;
                Self::start_quit_cascade(state);
            }
```

**Step 2: egui-fallback path** — replace the Confirm arm body (and note it now returns false):
```rust
// src/app.rs:1934 — draw_main_menu_dialogs, ExitConfirmAction::Confirm arm
                    dialogs::ExitConfirmAction::Confirm => {
                        // Persist BEFORE teardown (4b-i), then start the graceful
                        // cascade. Returning false (not true) hands exit to the
                        // cascade so this degraded path matches the SHP path; the
                        // screen-fade overlay is unavailable here (no SHP atlas), so
                        // the fallback runs the audio phases only (4b-ii open item).
                        Self::persist_settings_on_quit(state);
                        state.exit_confirm_modal = None;
                        Self::start_quit_cascade(state);
                        return false;
                    }
```

**Step 3: Verify** — `cargo check -p vera20k` → compiles. (No exit yet — Task 6 wires the tick.)

**Step 4: Commit** (group Tasks 3–5: "wire quit-cascade start, no tick yet" — safe, OK now starts a
cascade that nothing advances until Task 6; do Task 6 in the same commit to avoid a non-exiting OK).

### Task 6: Tick the cascade in `render_frame` and apply effects

**Why:** Advance the machine each frame, apply music volume/stop, prevent the per-frame menu-theme
re-assert from restarting music after the hard stop, and exit on completion.

**Files:** Modify `src/app.rs` — `render_frame` (cascade tick after the startup-splash block, ~:2674;
gate the menu-theme block at 2710-2715).

**Step 1: Tick + apply, right after the `startup_splash_until` block (before the InGame sim block, ~:2674)**
```rust
// src/app.rs — render_frame, immediately after `state.startup_splash_until = None; }` (~:2673)
        // Drive the graceful quit cascade (started on Exit-confirm OK). Compute
        // the voice poll before borrowing the cascade mutably to avoid aliasing.
        if state.quit_cascade.is_some() {
            let now = Instant::now();
            let voices_active = state
                .sfx_player
                .as_ref()
                .is_some_and(|sfx| sfx.voices_active());
            let tick = state
                .quit_cascade
                .as_mut()
                .expect("cascade present")
                .tick(now, voices_active);
            if let (Some(vol), Some(player)) = (tick.music_volume, state.music_player.as_mut()) {
                player.set_volume(vol);
            }
            if tick.stop_music {
                if let Some(player) = state.music_player.as_mut() {
                    player.stop();
                }
            }
            if tick.finished {
                state.quit_cascade = None;
                event_loop.exit();
                return Ok(());
            }
        }
```
> Verify `state.sfx_player` is the SFX player field name on `AppState` (the audio map cited
> `state.sfx_player`); if it differs, use the actual field. The music field is `state.music_player`.

**Step 2: Gate the per-frame menu-theme re-assert (so the hard stop sticks)**
```rust
// src/app.rs:2710 — replace the existing music maintenance block under GameScreen::MainMenu
                // The shell loops the menu [INTRO] theme while on the main menu.
                // Suppressed during the quit cascade so the hard music stop is not
                // immediately undone by a re-assert.
                if state.quit_cascade.is_none() {
                    if let (Some(player), Some(assets)) =
                        (&mut state.music_player, &state.asset_manager)
                    {
                        player.play_menu_theme(assets);
                        player.update(assets);
                    }
                }
```

**Step 3: Verify**
- `cargo check -p vera20k` → compiles.
- `cargo test -p vera20k --lib` → read the literal `test result:` line; all pass; skirmish safety
  net (`src/ui/skirmish_shell/state/tests.rs`, 87 tests) GREEN + UNCHANGED.

**Step 4: STOP for in-game OK** — launch, main menu → Exit → OK. Expect: the menu music **fades out**
(~0.4 s at default volume), then the window closes (no abrupt audio cut). Cancel/ESC still behave.
Confirm `RA2MD.INI` `[Audio] ScoreVolume` still persists (4b-i) — the pre-fade value, not 0.

**Step 5: Commit 4b-ii-a** to `dev` (group Tasks 3-6).

### Task 7: 4b-ii-a verification pass

**Why:** Confirm against gamemd before moving on.

**Verify:**
- Music fade audibly ramps (not a hard cut); length ≈ `volume × 1 s` (design §1, rate PROOFED).
- A quit with no trailing voice exits right after the fade; with a short voice, it plays out
  (in-game). Note the Deferred Question (long-voice cut) for the user to eyeball.
- No input reaches the menu during the fade (Task 4).
- `RA2MD.INI` persists the pre-fade `ScoreVolume`.

---

## Tasks — 4b-ii-b (screen fade-to-black)

### Task 8: Add the `FadeToBlack` phase + `overlay_alpha`

**Why:** Insert the ~480 ms fade-to-black between the music hard-stop and exit, and expose the alpha
for the overlay.

**Files:** Modify `src/app_quit_cascade.rs`.

**Step 1: Add the constant + phase + alpha state**
```rust
// src/app_quit_cascade.rs — add near MUSIC_FADE_PER_MS
/// Screen fade-to-black duration. The original fades the palette over `0x1E`=30
/// GetRadarTimer ticks × ~16 ms/tick ≈ 480 ms, linear to a black palette.
const SCREEN_FADE_MS: u64 = 480;
```
```rust
// src/app_quit_cascade.rs — extend the enum
enum QuitPhase {
    FadeMusicAndWaitVoices,
    /// Full-screen fade-to-black over SCREEN_FADE_MS, after music is hard-stopped.
    FadeToBlack,
    Done,
}
```
```rust
// src/app_quit_cascade.rs — add a field to QuitCascade and init it in `start`
    /// Latest fade-to-black alpha (0.0..=1.0), updated each tick during FadeToBlack;
    /// read by the renderer for the black overlay.
    overlay_alpha: f32,
```
```rust
// in `start`, add to the struct literal:
            overlay_alpha: 0.0,
```

**Step 2: Reroute the fade→teardown edge through FadeToBlack and add its branch**
```rust
// src/app_quit_cascade.rs — tick(), FadeMusicAndWaitVoices end edge: replace the early return
                if music_done || !voices_active || elapsed_ms >= WAIT_CEILING_MS {
                    self.enter(QuitPhase::FadeToBlack, now);
                    // Hard-stop music as we enter the visual fade.
                    return QuitCascadeTick { stop_music: true, ..Default::default() };
                }
```
```rust
// src/app_quit_cascade.rs — tick(), add the FadeToBlack arm before the Done arm
            QuitPhase::FadeToBlack => {
                let alpha = (elapsed_ms as f32 / SCREEN_FADE_MS as f32).min(1.0);
                self.overlay_alpha = alpha;
                if elapsed_ms >= SCREEN_FADE_MS {
                    self.enter(QuitPhase::Done, now);
                }
                // Keep rendering the (now fully black on the last frame) overlay;
                // finish on the next tick so the all-black frame is presented.
                QuitCascadeTick { ..Default::default() }
            }
```

**Step 3: Expose the alpha**
```rust
// src/app_quit_cascade.rs — inside impl QuitCascade
    /// Current fade-to-black overlay alpha (0.0 = none, 1.0 = full black).
    pub(crate) fn overlay_alpha(&self) -> f32 {
        self.overlay_alpha
    }
```

**Step 4: Update tests** — the fade→edge no longer finishes immediately; it stops music then fades.
```rust
// src/app_quit_cascade.rs — replace `fade_completion_stops_music_and_finishes` and
// `voices_done_ends_wait_early` assertions to follow the new edge, and add a fade test:
    #[test]
    fn fade_completion_enters_screen_fade_then_finishes() {
        let t0 = Instant::now();
        let mut c = QuitCascade::start(t0, 1.0);
        // Music fade completes at 1000 ms → hard-stop + enter screen fade (not finished).
        // `phase_started_at` resets to this `now` (t0+1000), so the fade clock starts here.
        let edge = c.tick(at(t0, 1000), true);
        assert!(edge.stop_music && !edge.finished);
        // Alpha ramps linearly over the next 480 ms.
        let mid = c.tick(at(t0, 1000 + 240), true);
        assert!((c.overlay_alpha() - 0.5).abs() < 0.01);
        assert!(!mid.finished);
        // At 480 ms the all-black frame is presented (alpha 1.0, not yet finished)…
        let full = c.tick(at(t0, 1000 + 480), true);
        assert!((c.overlay_alpha() - 1.0).abs() < 1e-6);
        assert!(!full.finished);
        // …and the cascade finishes on the next tick.
        assert!(c.tick(at(t0, 1000 + 500), true).finished);
    }
```
> Also keep `voices_done_ends_wait_early` but change its assertion to
> `assert!(end.stop_music && !end.finished)` (it now enters the screen fade, not Done).

**Step 5: Verify** — `cargo test -p vera20k --lib app_quit_cascade` → PASS.

**Step 6: Commit** (with Task 9).

### Task 9: Draw the full-screen black fade overlay

**Why:** Render the fade-to-black the player sees, over everything including the cursor.

**Files:** Modify `src/app_main_menu_shell_render.rs` — `render_main_menu_shell_to_target`: build the
overlay buffer BEFORE `begin_render_pass` (~`:468`, alongside `cursor_buffer`), then draw it inside
the pass before `drop(pass)` (`:574`).

**Pattern:** mirrors the existing buffers — ALL are created before the pass (`:460`) and referenced
inside (`:492-572`), because `draw_with_buffer_passthrough<'a>` ties the buffer to the pass lifetime
(`buffer: &'a wgpu::Buffer`, batch.rs:1364-1370). `create_instance_buffer` returns
**`Option<(wgpu::Buffer, u32)>`** (batch.rs:1029), so the count comes from the tuple. `SpriteInstance`
is built like `chrome.rs::push_entry` (`:115`).

**Step 1: Build the overlay buffer BEFORE `begin_render_pass` (after `cursor_texture`, ~:468)**
```rust
// src/app_main_menu_shell_render.rs — before `let mut pass = encoder.begin_render_pass(...)`
    // Quit-cascade fade-to-black: a full-screen black quad over EVERYTHING (incl.
    // the cursor), alpha ramped 0→1 by the cascade. Reuses the 1×1 opaque
    // white_pixel + the ALPHA_BLENDING passthrough pipeline; no new shader. Built
    // here (like every other layer) so it outlives the render pass. Only present on
    // the SHP path (the atlas is unavailable on the egui fallback).
    let fade_alpha = state
        .quit_cascade
        .as_ref()
        .map_or(0.0, |cascade| cascade.overlay_alpha());
    let fade_buffer = if fade_alpha > 0.0 {
        skirmish_chrome
            .and_then(|sk| sk.white_pixel)
            .and_then(|white| {
                let quad = [crate::render::batch::SpriteInstance {
                    position: [0.0, 0.0],
                    size: [state.gpu.config.width as f32, state.gpu.config.height as f32],
                    uv_origin: white.uv_origin,
                    uv_size: white.uv_size,
                    // Passthrough compares depth Always and this draws last, so any
                    // depth sits on top; the frontmost value is used for clarity.
                    depth: 0.0,
                    tint: [0.0, 0.0, 0.0],
                    alpha: fade_alpha,
                    ..Default::default()
                }];
                state.batch_renderer.create_instance_buffer(&state.gpu, &quad)
            })
    } else {
        None
    };
```

**Step 2: Draw it inside the pass, immediately before `drop(pass);` (:574)**
```rust
// src/app_main_menu_shell_render.rs — before `drop(pass);`
    if let (Some((buffer, count)), Some(sk_chrome)) = (fade_buffer.as_ref(), skirmish_chrome) {
        state.batch_renderer.draw_with_buffer_passthrough(
            &mut pass,
            &sk_chrome.texture,
            buffer,
            *count,
        );
    }
```
> Verified: `SkirmishShellChromeEntry { uv_origin, uv_size }` (skirmish_shell_chrome.rs:28-29), atlas
> `texture: BatchTexture` (`:34`), `white_pixel: Option<…>` (`:84`, Copy); `skirmish_chrome =
> state.skirmish_shell_chrome.as_ref()` (`:408`, in scope at both points); `SpriteInstance` derives
> `Default`.

**Step 2: Verify**
- `cargo check -p vera20k` → compiles.
- `cargo test -p vera20k --lib` → all pass; safety net GREEN + UNCHANGED.

**Step 3: STOP for in-game OK** — Exit → OK: music fades, then the **screen fades to black over
~0.5 s** (cursor fades into black too), then the window closes. Cancel/ESC unaffected.

**Step 4: Commit 4b-ii-b** to `dev`.

### Task 10: 4b-ii-b verification + adversarial review

**Why:** Confirm the full cascade against gamemd and guard the parity-critical timing.

**Verify:**
- Order holds: persist → music fade + voice wait → hard stop → ~480 ms fade-to-black → exit.
- Fade-to-black duration ≈ 0.5 s, linear, covers the whole screen incl. cursor (design §1, PROOFED).
- Non-blocking: the window stays responsive to the OS (no spin/hang) through the cascade.
- Run a short **adversarial review workflow** (independent lenses): (a) persist strictly precedes any
  volume mutation/exit; (b) phase-end gated on the voice/fade checks, not a fixed sleep; (c) the
  ~480 ms / `volume×1000 ms` durations match the PROOFED constants; (d) nothing blocks the frame loop;
  (e) no regression to normal menu music or in-game paths.

## Sources & References

- **Design doc:** `docs/plans/2026-06-01-shell-substrate-slice5-4bii-cascade-design.md` (PROOFED §1).
- **gamemd.exe (Ghidra, this session):** `Main_Game 0x0052D9A0` case 7; `Theme::Stop FUN_00720EA0`
  (fade non-blocking, `0x004080c0`→`VolumeInterp__SetTarget 0x00407170`; rate `FUN_004071a0`/`0x00401000`
  divisor 1000; ms via `Timer__InitPerformanceCounter 0x00409393`); vox-wait `0x0052E79C-E7E4`
  (`VoxClass__PumpAndCheckActive 0x007529E0`, music gate `0x00720FD0`, `Network_ServiceLoop 0x0048D080`);
  `GetRadarTimer 0x006C8C40` (timeGetTime>>4); screen fade `FUN_004A3C30` (`0x1E` ticks, black palette
  `0x00884E80`). Addresses kept here, not in Rust comments.
- **Repo patterns:** `src/app_shell_transition.rs:109` (ShellFrameWave), `:207` (input gate);
  `src/audio/sfx.rs:367` (voice liveness), `:478` (accessor); `src/audio/music.rs:184/239/247`;
  `src/app_main_menu_shell_render.rs:460/541/569` (passthrough draws); `chrome.rs:115` (push_entry);
  `render/batch.rs:42` (SpriteInstance), `:1025` (create_instance_buffer);
  `render/skirmish_shell_chrome.rs:84` (white_pixel).
- **INI keys:** none (cascade timing is hardcoded in gamemd).
- **Prior sub-step:** 4b-i persist (`src/util/ini_writer.rs`, `src/audio/music.rs` writer,
  `app.rs persist_settings_on_quit`) — committed pending in-game OK.
