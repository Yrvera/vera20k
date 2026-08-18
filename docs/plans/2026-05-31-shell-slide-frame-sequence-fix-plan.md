# Shell First-Paint Slide — Frame-Sequence Fix Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.
> This plan doc lives under `docs/` (gitignored/local-only) — do NOT write commit
> steps for the plan itself; the commit tasks commit the tracked `src/` edits.

**Goal:** Make the shell first-paint button slide play the SDBTNANM animation in the
gamemd-correct direction (controls ramp down 10→5 and settle at 1, instead of ramping
up 5→10 and settling at 10).

**Architecture:** Presentation layer only (`src/app_shell_transition.rs`, app/render
tier). No `sim/` involvement, no fixed-point, no state hash, no determinism impact. The
wave emits an SDBTNANM frame index per button per tick; the three shell renderers draw
that frame. Fixing the index schedule fixes the motion with zero renderer changes.

**Design Doc:** none (`/brainstorm` skipped — this is a one-decision fix grounded in a
verified binary-vs-Rust schedule diff produced this session). Grounding below stands in
for the design doc.

---

## Grounding Summary

- **Binary truth (verified this session, read-only):** `FUN_006071E0` is the slide loop.
  Regular owner-draw button cell on SHOW (`DL=1`, ramp direction `-1`): holds pre-entry
  frame **10**, ramps **10,9,8,7,6,5** (`base=10 + delta*-1`, delta 0..5), settles frame
  **1**. CLOSE (`DL=0`, dir `+1`): holds **1**, ramps **5,6,7,8,9,10**, settles **10**.
  Second cell group: SHOW holds 10, ramps **16..11**, settles **0**; CLOSE holds 0, ramps
  11..16, settles 10. Cadence = fixed `Sleep(0x1e)`=30 ms inside the loop; stagger = +1
  cascade (cell *k* enters at step *k+1*). Source: `decompile_function 0x006071E0` +
  `disassemble_function` @ 0x006079e9 (regular cell), @ 0x607b54 (second group), @
  0x00607f0f (Sleep), @ 0x006076a4 (loop bound). Cross-referenced in
  `docs/research/SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md` (C11) and the
  slide-schedule diff workflow `wf_ba65e076-235`.
- **Rust truth (read this session):** `src/app_shell_transition.rs:33-37`
  `GROUP_A_IN = {before:1, base:5, after:10}` with `WaveDirection::SlideIn => dir +1`
  (lines 92-95) → holds 1, ramps 5→10, settles 10. This is the exact mirror of the
  binary. The current test `group_a_slide_in_holds_1_ramps_5_to_10_then_holds_10`
  (lines 354-368) enshrines the wrong sequence. The doc-comment at lines 24-27 falsely
  claims the constants are "verified from the binary frame schedule."
- **The bug is a swap+flip.** `GROUP_A_OUT` (lines 38-42 = `{before:10, base:10,
  after:1}`) already holds the *correct slide-IN* values; `GROUP_A_IN` holds the correct
  slide-OUT values. Same for Group B. And `dir()` is inverted vs gamemd (gamemd: show=-1,
  close=+1; Rust: SlideIn=+1, SlideOut=-1).
- **Repo pattern:** the module already has `#[cfg(test)] mod tests` unit tests asserting
  frame sequences — we mirror that, just with corrected expectations.
- **INI:** `ShellButtonSlideSound=` empty in `ini/rulesmd.ini` (slide is silent — no
  change). `MenuSlideIn` (GUIMoveInSound) is the slide-start cue; not in scope here.
- **Git state:** `git log -- src/app_shell_transition.rs` shows one clean lineage
  (`985e4bf` schedule+formula → `ee92ae7` static reveal); no parallel-session churn.
- **Still unknown (→ deferred):** whether any shell's buttons fall in gamemd's
  *second cell group* (16→11) vs *regular* (10→5); whether `total_ticks` should be N+7
  or N+8; whether skirmish needs the radar/SDTP own-frame sweep. All Phase 2.

## Key Technical Decisions

- **Fix = swap `GROUP_*_IN` ↔ `GROUP_*_OUT` constants and flip `WaveDirection::dir()`**
  (SlideIn→`-1`, SlideOut→`+1`). With the unchanged formula `base + delta*dir`, this
  yields the binary sequences exactly. — **Confidence:** high — **Source:** verified
  `FUN_006071E0` schedule (this session) + current `app_shell_transition.rs` read.
- **Do NOT change cadence (`WAVE_TICK_MS=30`), stagger (`entry_tick = slot+1`), or the
  4-zone classifier** — all three match the binary. — **Confidence:** high — **Source:**
  same.
- **Defer `total_ticks` N+7-vs-N+8 and Group-B/radar-sweep** to Phase 2 (needs a Ghidra
  re-read of the schedule-array max and the child-cell group split). The current N+8 is
  within one 30 ms tick of the binary and is *not* the visible bug. — **Confidence:**
  medium — **Source:** internal inconsistency in the slide-schedule report (max stagger
  N+1 vs an explicit N+2 schedule entry); resolve by re-reading 0x00607646-0x006076ad.

## Open Questions

### Resolved During Planning
- *Exact corrected constants?* → `GROUP_A_IN={before:10,base:10,after:1}`,
  `GROUP_A_OUT={before:1,base:5,after:10}`, `GROUP_B_IN={before:10,base:16,after:0}`,
  `GROUP_B_OUT={before:0,base:11,after:10}`, `dir(): SlideIn=>-1, SlideOut=>+1`. Verified
  by hand-expanding the formula against the binary sequences (see Parity-Critical Items).
- *Are frames 1/5/10 present in the atlas?* → Yes; `render/main_menu_shell_chrome.rs`
  builds SDBTNANM frames 0..16. (Confirm in Task 6 anyway.)
- *Does the renderer need changes?* → No. It draws whatever frame index the wave returns;
  same asset, same indices as gamemd ⇒ same pixels.

### Deferred to Implementation / Phase 2
- Whether the main-menu/SP/skirmish button columns map to gamemd's *regular* (10→5) cell
  group only, or whether some buttons use the *second cell group* (16→11) ⇒ whether
  `ButtonGroup::B` must be wired (it is dead code today). Needs Ghidra child-enumeration
  read (`FUN_006071E0` index ranges `local_14c`/`iStack_148`, `FUN_0060a180`).
- `total_ticks` = N+7 (max stagger N+1 + 6) or N+8 (an explicit N+2 schedule entry). One
  30 ms tick of tail-hold; imperceptible; re-derive from 0x00607646-0x006076ad.
- Skirmish radar/SDTP own-frame sweep (5→0) + SDTP-secondary (1/6), currently only a
  binary +80 px position shift in Rust. Skirmish-only parity item.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/app_shell_transition.rs:33-52` | Correct `GROUP_*_IN/OUT` constants |
| Modify | `src/app_shell_transition.rs:90-98` | Flip `WaveDirection::dir()` |
| Modify | `src/app_shell_transition.rs:24-27, 91` | Fix the now-correct doc comments |
| Modify | `src/app_shell_transition.rs:354-381` | Rewrite the two group-sequence unit tests |

## Interface Changes

None. `WaveFrames`, `sdbtnanm_frame`, `ButtonGroup`, `WaveDirection` keep their
signatures; only constant values and the `dir()` mapping change. The three render
consumers (`app_main_menu_shell_render.rs`, `app_single_player_shell_render.rs`,
`app_skirmish_shell_render.rs`) are untouched — they already call
`sdbtnanm_frame(slot, ButtonGroup::A)` and draw the result.

## Sim Checklist

N/A — presentation layer only. No `sim/` files touched; no fixed-point, no state hash,
no tick-order or determinism impact.

## Risk Areas

- **Blast radius: low.** One module's constants + tests. The render path is unchanged.
- **Regression risk:** the two existing tests assert the *wrong* sequence and WILL fail
  after the constant change — that is expected; Task 3 rewrites them to the correct
  sequence. Confirm no OTHER test references the old `5→10` ordering (grep in Task 4).
- **Visual regression:** the only observable change is the slide direction — verified
  in-game in Task 6. No steady-state (post-slide) frame changes (idle still frame 2).

## Parity-Critical Items

| Task | Item | Why it matters | Verification |
|------|------|----------------|--------------|
| 1-2 | SDBTNANM slide-IN frame schedule (Group A) | The button slide is visible on EVERY main-menu/SP entry; wrong direction = buttons retreat-then-pop instead of sliding in | Hand-expanded vs `FUN_006071E0` @0x6079e9; in-game side-by-side (Task 6) |
| 1-2 | Slide direction mapping (`dir`) | show counts down, close counts up — inverting both flips the whole animation | `FUN_006071E0` `iStack_174 = -1 show / +1 close` |
| 2 | Group B schedule (16→11 settle 0) | Used IF the second cell group is reached; reversed today | Phase-2 Ghidra confirm before relying |
| 1 | Pre-entry hold = 10, settle = 1 (distinct terminals) | Buttons wait "out" at 10 before their turn, ease to seated 1 | `FUN_006071E0` before/after constants |

Full corrected slide-IN expansion (Group A, dir=-1): pre-hold **10** →
**10,9,8,7,6,5** (delta 0..5) → settle **1** (delta≥6). Group B slide-IN (dir=-1):
pre-hold **10** → **16,15,14,13,12,11** → settle **0**.

---

## Tasks

### Task 1: Flip `WaveDirection::dir()` to match gamemd (show counts down)

**Why:** gamemd ramps the show animation with direction `-1` (frames decrease) and the
close animation with `+1`. The current code has both inverted. This must change together
with the constants (Task 2) so the formula `base + delta*dir` reproduces the binary.

**Files:**
- Modify: `src/app_shell_transition.rs:90-98`

**Pattern:** existing method; value-only change + comment fix.

**Step 1: Replace the `dir()` mapping and its doc comment**
```rust
impl WaveDirection {
    /// frame multiplier: -1 on slide-in (frames count DOWN, e.g. 10→5), +1 on
    /// slide-out (frames count UP). Matches gamemd `FUN_006071E0` ramp direction
    /// (iStack_174 = -1 on show / +1 on close).
    fn dir(self) -> i32 {
        match self {
            WaveDirection::SlideIn => -1,
            WaveDirection::SlideOut => 1,
        }
    }
}
```

**Step 2: Verify (compile only — full test in Task 4)**
Run: `cargo check -p vera20k`
Expected: compiles (tests will fail until Task 3 — that's fine).

### Task 2: Correct the `GROUP_*_IN` / `GROUP_*_OUT` SDBTNANM constants

**Why:** The IN/OUT constant blocks are swapped vs the binary. `GROUP_A_OUT` already holds
the correct slide-IN values; `GROUP_A_IN` holds the correct OUT values. Fix all four so
that, with the corrected `dir()` from Task 1, the formula yields the binary sequences.

**Files:**
- Modify: `src/app_shell_transition.rs:24-52`

**Pattern:** existing constants; value-only change + corrected doc comment.

**Step 1: Replace the doc comment and the four constant blocks**
```rust
/// SDBTNANM frame constants per button group, verified from `FUN_006071E0`.
/// Each tuple is (held_before, ramp_base, held_after). With `dir() = -1` on
/// slide-IN, the IN ramp counts DOWN from `base`; the held terminals are distinct
/// constants, not `base`. Slide-OUT uses `dir() = +1` (ramp counts UP).
/// Group A = regular owner-draw button cell (SDBTNANM 10→5, settle 1).
/// Group B = gamemd's "second cell group" (SDBTNANM 16→11, settle 0) — not yet
/// wired by any consumer (see Phase 2).
pub(crate) struct WaveFrames {
    pub before: i32,
    pub base: i32,
    pub after: i32,
}
/// SHOW: hold 10 → ramp 10,9,8,7,6,5 → settle 1.
pub(crate) const GROUP_A_IN: WaveFrames = WaveFrames {
    before: 10,
    base: 10,
    after: 1,
};
/// CLOSE: hold 1 → ramp 5,6,7,8,9,10 → settle 10.
pub(crate) const GROUP_A_OUT: WaveFrames = WaveFrames {
    before: 1,
    base: 5,
    after: 10,
};
/// SHOW: hold 10 → ramp 16,15,14,13,12,11 → settle 0.
pub(crate) const GROUP_B_IN: WaveFrames = WaveFrames {
    before: 10,
    base: 16,
    after: 0,
};
/// CLOSE: hold 0 → ramp 11,12,13,14,15,16 → settle 10.
pub(crate) const GROUP_B_OUT: WaveFrames = WaveFrames {
    before: 0,
    base: 11,
    after: 10,
};
```

**Step 2: Verify (compile only)**
Run: `cargo check -p vera20k`
Expected: compiles.

### Task 3: Rewrite the two group-sequence unit tests to the binary sequences

**Why:** The existing tests assert the reversed sequences and now lock in the bug. Replace
them with the gamemd-correct expectations so they guard the fix.

**Files:**
- Modify: `src/app_shell_transition.rs:354-381`

**Pattern:** existing `#[cfg(test)] mod tests` unit tests.

**Step 1: Replace both tests (rename to reflect the corrected sequence)**
```rust
    #[test]
    fn group_a_slide_in_holds_10_ramps_10_to_5_then_holds_1() {
        let t0 = Instant::now();
        let mut w = ShellFrameWave::new_first_paint_slide(3, t0);
        // slot 1 enters at tick 2; before that it holds at the "before" terminal = 10.
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 10);
        for _ in 0..2 {
            w.tick += 1;
        } // tick = 2 => delta 0 => base 10
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 10);
        w.tick += 5; // delta 5 => 10 + 5*-1 = 5 (last ramp step)
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 5);
        w.tick += 3; // delta >= 6 => held "after" terminal = 1
        assert_eq!(w.sdbtnanm_frame(1, ButtonGroup::A), 1);
    }

    #[test]
    fn group_b_slide_in_holds_10_ramps_16_to_11_then_holds_0() {
        let t0 = Instant::now();
        let mut w = ShellFrameWave::new_first_paint_slide(3, t0);
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 10); // before-entry (slot 0 enters tick 1)
        w.tick += 1; // delta 0 => base 16
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 16);
        w.tick += 5; // delta 5 => 16 + 5*-1 = 11
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 11);
        w.tick += 3; // delta >= 6 => held "after" = 0
        assert_eq!(w.sdbtnanm_frame(0, ButtonGroup::B), 0);
    }
```

**Step 2: Verify**
Run: `cargo test -p vera20k app_shell_transition`
Expected: PASS — including the unchanged `total_ticks_is_max_schedule_plus_tail` and
`advance_steps_one_frame_per_30ms_and_never_collapses`.

### Task 4: Confirm no other code/test depends on the old (reversed) sequence

**Why:** A second test or a renderer special-case could assume `5→10`. Catch it before
committing.

**Files:** none (search only).

**Step 1: Grep for stale assumptions**
Search the repo for references that hardcode the old ordering or `GROUP_` usage:
- `GROUP_A`, `GROUP_B`, `sdbtnanm_frame`, `ButtonGroup::B`, and any literal frame
  sequence `5, 6, 7, 8, 9, 10` in `src/app_main_menu_shell_render.rs`,
  `src/app_single_player_shell_render.rs`, `src/app_skirmish_shell_render.rs` (+ submods).
Expected: the only `WaveFrames`/`sdbtnanm_frame` consumers pass `ButtonGroup::A` and draw
the returned index with no direction assumption. Note any finding; if a renderer hardcodes
a sequence, it is a separate bug — record it, do not silently fix.

**Step 2: Verify**
Run: `cargo test -p vera20k`
Expected: full suite PASS.

### Task 5: Build, lint, commit

**Why:** Land the certain fix atomically.

**Files:** the modified `src/app_shell_transition.rs`.

**Step 1:** Run `cargo build -p vera20k` — expected: success.
**Step 2:** Run `cargo clippy -p vera20k` — expected: no new warnings in the file.
**Step 3:** Commit on `dev`:
```
fix(ui): shell first-paint slide ramps SDBTNANM 10->5 (was reversed 5->10)

The first-paint button slide held frame 1, ramped 5..10, and settled 10 —
the mirror of gamemd FUN_006071E0, which holds 10, ramps 10..5, and settles 1.
GROUP_*_IN/OUT constants were swapped and WaveDirection::dir() inverted. Buttons
now slide in and settle instead of retreating then popping. Cadence/stagger
unchanged.
```

### Task 6: In-game visual verification (acceptance)

**Why:** The bar is "looks like gamemd." Confirm the rendered motion, not just the unit
test.

**Files:** none.

**Step 1:** Launch the app to the main menu (use `/run` or the project's run skill).
**Step 2:** Observe the first-paint slide on dialog 0xE2. Expected (matches gamemd):
the six buttons cascade in one-by-one (top first), each starting from SDBTNANM frame 10
and ramping **down** to 5 then settling, easing **into** their seats — NOT appearing then
sliding outward/retreating then snapping. Atlas confirm: frames 1, 5, 10 all render (no
"draw nothing" fallback).
**Step 3:** Repeat into the single-player shell (dialog 0x100) — same inward cascade.
**Step 4:** If a reference is available, side-by-side against gamemd.exe main menu.
Record the result (pass/fail + a screenshot or short note).

---

## Phase 2 — investigate-first (skirmish 0x102 sidebar); do NOT assume

### Task 7: Ghidra — determine button→group mapping (regular vs second cell group)

**Why:** Decide whether `ButtonGroup::B` must be wired. Today it is dead code.

**Step 1:** `decompile_function 0x006071E0` and read the child-cell enumeration that sets
the two index ranges (`0..local_14c` regular, `local_14c..iStack_148` second group) plus
the final-row button; `decompile_function 0x0060a180` (the EnumChildWindows counter).
Determine, per shell (0xE2 6 buttons / 0x100 4 / 0x102 3), which buttons land in which
range.
**Step 2:** If all buttons of all three shells are "regular cells" (10→5), Group B stays
unused and Phase 2b/9 is unnecessary for buttons. If any shell splits into the 16→11
range, record exactly which slots → Group B. Output: a short note appended to
`docs/research/SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md` (local-only doc).

### Task 8: Ghidra — confirm skirmish radar/SDTP frame sweep + shift

**Why:** gamemd sweeps the radar/SDTP panel's own frames (5→0) and SDTP-secondary (1/6)
during the slide; Rust only shifts the static SDTP +80 px and snaps it back
(`app_skirmish_shell_render.rs:101-105, 206-215`).

**Step 1:** `disassemble_function 0x006071E0` @ 0x607d3c (radar/SDTP block) and @ 0x60793a
(SDTP-secondary) — confirm the frame sweep values, the `g_SDTP_SHP`/`g_RadarFrameOpen_SHP`
asset switch per zone, and how the +0x50 (80 px) shift integrates per zone (pre on DL=0,
settle on DL=1, mid unconditional, width≥800).
**Step 2:** Decide whether to add the radar/SDTP frame animation for skirmish parity, or
defer. Record the decision; do not implement here.

### Task 9 (conditional): implement Phase-2 findings

**Why:** Only if Task 7/8 prove a gap that's player-visible on the skirmish shell.

**Step 1:** If Task 7 found Group-B buttons: wire `ButtonGroup::B` at the relevant slots
in `app_skirmish_shell_render.rs` (and/or others) and add a unit test asserting the
16→11→0 sequence (already corrected in Task 2). 
**Step 2:** If Task 8 decided to add the radar/SDTP sweep: extend the skirmish wave to
swap the SDTP/radar SHP frame per tick (not just position), mirroring the per-zone schedule
from Task 8. Add the `total_ticks` N+7-vs-N+8 re-derivation here if Task 7 settled it.
**Step 3:** `cargo test -p vera20k`; in-game verify the skirmish sidebar slide; commit.

## Sources & References

- **Slide-schedule diff (this session):** workflow `wf_ba65e076-235` — gamemd vs Rust
  exact frame schedules.
- **Substrate study (this session):** `docs/research/SHELL_DIALOG_FRAMEWORK_SUBSTRATE_SERVICE.md`
  (C11 slide contract).
- **gamemd.exe addresses (kept here, not in code comments):** `FUN_006071E0` (slide loop;
  regular cell @0x6079e9, second group @0x607b54, radar/SDTP @0x607d3c, SDTP-secondary
  @0x60793a, Sleep @0x00607f0f, loop bound @0x006076a4, schedule build @0x00607646),
  `FUN_00608260` (slide-IN trigger), `FUN_0060a180` (child-cell counter).
- **Rust touchpoints:** `src/app_shell_transition.rs:24-52, 90-98, 354-381`; consumers
  `src/app_main_menu_shell_render.rs`, `src/app_single_player_shell_render.rs`,
  `src/app_skirmish_shell_render.rs`.
- **INI:** `ini/rulesmd.ini` `[AudioVisual] ShellButtonSlideSound=` (empty — silent).
- **Prior commits:** `985e4bf` (schedule+formula, introduced the reversed constants),
  `ee92ae7` (static reveal at slide completion).
