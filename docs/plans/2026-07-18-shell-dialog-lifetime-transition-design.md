# Shell Dialog Lifetime Transition and Reveal Parity Design

**Date:** 2026-07-18  
**Status:** Approved by the user; ready for evidence-gated planning  
**Scope:** Entry waves, close waves, transition timing and input gating, completion
messages, and qualifying static-text reveals for the currently rendered shell
dialogs `0xE2`, `0x100`, `0x102`, and `0x6B`.  
**Non-scope:** Implementing missing Campaign, Movies, Load Game, or Random Map
Generator destination dialogs; generic Win32 dialog emulation; unrelated shell
layout or gameplay disparities.

## Goal

Replace screen-change inference with explicit dialog-lifetime transitions so the
Rust shell follows active Yuri's Revenge behavior when a dialog is created,
closed, hidden behind a modal, or revealed again.

The result must keep the source dialog visible through its close wave, delay the
destination until close completion, avoid replaying entry when an existing parent
is merely uncovered, preserve exact tick and frame schedules, and start native
static-text reveals only from verified entry-completion paths.

This design does not certify pixel parity by itself. Every unresolved frame,
gradient, sound, or composition detail remains `UNCHECKED` until supported by
active-`gamemd.exe` evidence and a gamemd-derived executable comparison.

## Evidence Baseline

Primary evidence:

- `docs/research/skirmish-ui/SHELL_FIRST_PAINT_SLIDE_GENERIC_TRIGGER_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_STATIC_REVEAL_ANIMATION_0X102_GHIDRA_REPORT.md`
- `docs/research/BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`
- `docs/gap-scans/_shell-ui-scan-2026-07-06-lanes/gh-3.md`
- `docs/gap-scans/_shell-ui-scan-2026-07-06-lanes/gh-19.md`
- `docs/gap-scans/_shell-ui-scan-2026-07-06-lanes/gh-20.md`
- `docs/gap-scans/_shell-ui-scan-2026-07-06-lanes/gh-21.md`
- `docs/gap-scans/_shell-ui-scan-2026-07-06-lanes/gh-22.md`

Load-bearing verified facts:

- A common dialog subclass starts the entry wave on the first `WM_PAINT` of each
  allowlisted dialog lifetime. The active rendered allowlist in scope is `0xE2`,
  `0x100`, `0x102`, and `0x6B`.
- The common teardown path runs the close wave on an allowlisted visible dialog
  that is actually being closed.
- Main Menu has five regular owner-draw buttons, Single Player has three, and
  Skirmish has two. Back and Exit share the last regular-button schedule value;
  they do not add another slot.
- Entry uses SDBTNANM hold frame 10, ramp `10 -> 5`, settle frame 1, then emits
  completion `0x4EC`. Close uses hold frame 1, ramp `5 -> 10`, settle frame 10,
  then emits `0x4ED`.
- Entry start consumes `GUIMoveInSound` (`MenuSlideIn` in stock data). The
  entry-completion `ShellButtonSlideSound` hook is stock-empty but remains a
  configuration-driven mechanism.
- Opening Choose Map hides the existing Skirmish parent rather than destroying
  or closing it. Closing Choose Map uncovers that same parent and must not replay
  the Skirmish entry wave.
- Skirmish title, game-type, and map labels begin blank and start their kind-1
  reveal from entry completion. The timer invalidates; a successful paint draws
  the current count and only then advances it.

The exact Choose Map control schedule, close-start sound identity, Path-A
BITFONT gradient arithmetic, and special Skirmish chrome composition are not yet
fully verified. They are mandatory evidence gates, not opportunities for guessed
constants.

## Chosen Architecture

The application owns one explicit transition controller:

```text
ShellTransitionController
  Idle
  Entering { surface, wave }
  Closing  { surface, wave, pending_action }
```

`ShellSurface` identifies native dialog lifetimes, not merely visible app screens:

```text
MainMenu0xE2
SinglePlayer0x100
Skirmish0x102
ChooseMap0x6B
```

The controller replaces the current `AppScreen` edge detector and the split
`shell_first_paint_slide` / `shell_slide_active_shell` authority. App lifecycle
code explicitly reports creation and close intent:

- `begin_entry(surface)` is called only for a newly created native-equivalent
  dialog instance, or for an independently verified native re-show helper.
- `request_close(surface, pending_action)` retains the source state and renderer,
  starts the close wave, blocks further input, and defers the action.
- The pending action executes exactly once after the terminal close frame and
  completion `0x4ED`.
- A transition cannot overlap another transition. Additional clicks or actions
  while input is blocked have no effect.
- A screen assignment is never sufficient by itself to start an entry wave.

This is an app/UI mechanism. It introduces no dependency from `sim/` to app,
render, shell, sidebar, or audio code.

## Lifecycle and Navigation Contract

### Ordinary replacement dialogs

For Main Menu to Single Player, Single Player to Main Menu, Single Player to
Skirmish, Back navigation, and Exit:

1. Resolve the clicked command and play its independently verified click sound.
2. Perform any invisible validation or immutable preparation required before
   close.
3. Request close on the source dialog with a semantic pending action.
4. Keep drawing the source through every close frame; do not expose the
   destination.
5. Emit close completion, destroy/retire the source lifetime, and execute the
   pending action exactly once.
6. If the action creates another in-scope dialog, start that new lifetime's entry
   wave on its first-paint-equivalent path.

Pending actions remain semantic app-layer commands rather than closures holding
borrowed UI state. Expected variants include opening Single Player, returning to
Main Menu, entering Skirmish, returning to Single Player, beginning a prepared
loading session, finishing Choose Map accept/cancel, and quitting.

### Start Game

- Validate before requesting close.
- A validation failure displays the appropriate modal and neither starts a close
  wave nor changes destination state.
- Success prepares an owned launch payload without making the destination
  visible, closes `0x102`, and begins loading only after close completion.
- The existing shell persistence/RNG close transaction retains its independently
  verified ordering; the transition controller owns visibility and timing, not
  launch-data semantics.

### Choose Map modal

Opening `0x6B` is not an ordinary replacement transition:

1. Preserve the live `0x102` parent state.
2. Hide the parent without requesting its close wave.
3. Create `0x6B` and begin the new modal lifetime's entry wave.

Accepting or cancelling:

1. Capture the accepted result or cancel intent in an owned pending payload.
2. Run the `0x6B` close wave while the modal remains visible.
3. After completion, remove `0x6B` and commit or discard the payload.
4. Reveal the pre-existing `0x102` parent without calling `begin_entry(0x102)`.

The Random Map Generator's explicit chooser re-show behavior is outside this
slice and must not be generalized from the ordinary Choose Map return path.

## Native Wave Specification

`ShellSlideSpec` is keyed by dialog ID and native control identity. It contains:

- the regular Group-A count;
- explicit control schedule entries with native entry ticks;
- Back/Exit special-control timing;
- optional special-chrome schedules and composition data; and
- verified entry-completion reveal qualifiers.

The render API consumes native entry ticks rather than a misleading zero-based
enumeration slot.

| Surface | Verified schedule | Total ticks |
|---|---|---:|
| Main Menu `0xE2` | five regular buttons at 1..5; Exit at 5 | 14 |
| Single Player `0x100` | three regular buttons at 1..3; Back at 3 | 12 |
| Skirmish `0x102` | Start at 1; Choose Map at 2; Back at 2 | 11 |
| Choose Map `0x6B` | unresolved pending focused binary verification | `UNKNOWN` |

Back and Exit use the verified Group-A frame constants through their special
draw block. They are not modeled as an invented trailing Group-B slot.

## Timing, Input, and Audio

- A wave advances once per actually rendered transition frame.
- The next frame may occur no earlier than 30 ms after the prior frame was
  rendered.
- Elapsed wall time never causes multi-tick catch-up. Native blocking-loop
  cadence stretches under delay rather than skipping visual states.
- Shell pointer and keyboard input remain blocked throughout entry and close.
- `MenuSlideIn` plays once before entry tick zero.
- The configured `ShellButtonSlideSound` hook is evaluated once at entry
  completion before the `0x4EC` reveal path. Stock emptiness is not hardcoded.
- Close completion emits `0x4ED` and starts no reveal.
- The close-start cue remains unimplemented until its live identity is proven;
  `MenuSlideOut` must not be wired from name similarity alone.
- The native timeout/error path is preserved as an evidence gate. A guessed
  timeout is not introduced.

## Rendering Contract

Renderers receive an immutable `ShellTransitionFrame` containing the surface,
direction, native tick, resolved control-frame overrides, and special-chrome
phase. They do not read or mutate application transition state directly.

- Main Menu and Single Player use control-ID-based frame overrides so button
  enumeration cannot alter native scheduling.
- Skirmish has a dedicated transition composition for regular buttons,
  `SDMPBTN`, `SDWRNTMP`, and its phase-specific wide-screen offset.
- Transition-only displacement and special frames do not leak into steady-state
  chrome rendering.
- Choose Map receives its own surface transition and reveal set once its schedule
  is verified.
- Outside an active transition, existing steady rendering remains unchanged.
- Background movie, cursor, hover, text, and nonanimated control composition
  during each wave remain `UNCHECKED` until runtime evidence establishes their
  exact inclusion and order.

## Static Reveal Contract

The reveal state models the native distinction between blank waiting state,
active reveal, and completed-but-still-running state:

```text
Waiting   { count: 1, range: 8, running: false }
Running   { count, range: 8, running: true, paint_dirty }
Completed { count: target, range: 8, running: true }
```

- Qualifying labels are blank while waiting; inactive does not mean fully drawn.
- Entry completion `0x4EC` starts the relevant child reveal with count 1 and an
  immediate invalidation request.
- The timer only marks the control dirty without erasing the background.
- A successful paint draws the current count/range and increments afterward.
- The completion target is native wide-text length plus one plus range, currently
  expressed as `UTF-16 code units + 9` pending final code-unit verification.
- Reaching the target stops the timer but retains the native running flag.
- Game-type and selected-map text changes restart their reveals, including after
  completion. A text change while still waiting remains waiting.
- Close completion never starts or restarts a reveal.
- Path-A leading-edge tinting must use the verified native integer interpolation,
  packed pixel format, selected-unit highlight source, clipping, and glyph order.
  The sidebar's unrelated Path-B gradient is not reusable evidence.

## Failure and Fallback Behavior

- A transition render failure retains its pending action and source lifetime; it
  cannot silently navigate early or lose the route.
- Missing assets enter an explicit `UnverifiedFallback`, report the missing
  dependency, and avoid hanging the controller. Such output is never labeled
  parity.
- Unknown dialog specs cannot silently borrow a neighboring dialog's count or
  schedule.
- A partially verified special-chrome path remains disabled or visibly
  unverified instead of substituting guessed frames or coordinates.

## Required Evidence Gates

Before implementation may claim the affected mechanism is closed:

1. Verify `0x6B` Group-A count, control order, special-control schedule, total
   ticks, and kind-1 reveal-qualified children.
2. Verify exact BITFONT Path-A interpolation, integer widths, rounding, RGB
   packing, highlight initialization, clipping, and UTF-16/code-unit behavior.
3. Reconcile `SDMPBTN` and `SDWRNTMP` entry/close frames, anchors, rectangles,
   palettes, draw order, and the phase-specific `+0x50` wide-screen shift.
4. Identify the live close-start sound consumer and exact route/timeout gates.
5. Capture or otherwise prove nonanimated controls, movie/background, hover, and
   cursor composition during every wave phase.
6. Check whether `0xE2` or `0x100` contain any active kind-1 reveal qualifiers.

Implementation may be split so the three fully scheduled dialogs are developed
behind explicit evidence-backed specs while `0x6B` stays blocked. It must not
invent the missing `0x6B` values merely to make the type exhaustive.

## Expected Code Impact

Likely touchpoints:

- `src/ui/shell/slide.rs`
- `src/app_shell_transition.rs`
- `src/app.rs` and shell navigation helpers
- `src/app_main_menu_shell_render.rs`
- `src/app_single_player_shell_render.rs`
- `src/app_skirmish_shell_render.rs` and its submodules
- the Choose Map renderer/state modules
- `src/ui/skirmish_shell/static_reveal.rs`
- `src/render/shell_text.rs`
- `src/render/bit_font.rs`
- `src/render/skirmish_shell_chrome.rs`
- shell audio/rules parsing only where verified configuration hooks are missing
- focused controller, schedule, renderer, reveal, and integration tests

Changes should be additive and narrow. No broad shell rewrite, graphics
dependency upgrade, ECS introduction, or simulation ownership change is
authorized.

## Tiny-Detail Ledger

| Detail | Required outcome |
|---|---|
| Entry trigger | First paint of a new allowlisted dialog lifetime |
| Close trigger | Actual visible allowlisted-dialog teardown |
| Parent hidden by modal | No parent close wave |
| Parent uncovered | No parent entry replay |
| Input | Blocked for every entry and close tick |
| Timing | One rendered state per tick, at least 30 ms apart, no catch-up |
| Entry frames | hold 10, ramp `10 -> 5`, settle 1 |
| Close frames | hold 1, ramp `5 -> 10`, settle 10 |
| Main Menu count | 5 regular controls; Exit shares tick 5; 14 ticks |
| Single Player count | 3 regular controls; Back shares tick 3; 12 ticks |
| Skirmish count | 2 regular controls; Back shares tick 2; 11 ticks |
| Choose Map count | `UNKNOWN` until verified |
| Entry completion | configured completion sound hook, then `0x4EC` reveal path |
| Close completion | `0x4ED`; no reveal |
| Initial qualifying text | Blank, count 1, range 8, not running |
| Reveal cadence | Timer invalidates; paint draws then increments |
| Reveal completion | target `length + 9`; timer stops; running retained |
| Text changes | Restart verified dynamic labels after entry/completion |
| Gradient | Native Path A only; sidebar Path B not substituted |
| Special Skirmish chrome | Evidence-backed phase frames and composition only |
| Destination visibility | Begins only after ordinary source close completion |
| Pending action | Owned and executed exactly once |
| Simulation/RNG | No new dependency or ordering change |

## Verification and Acceptance

Focused automated tests must cover:

1. Exact known control ticks, frame sequences, terminal frames, and total counts.
2. Back/Exit sharing the final Group-A tick without extending the wave.
3. Entry and close directionality and completion-message separation.
4. Exact-once pending-action execution and rejection of overlapping input.
5. Source visibility until close completion and destination invisibility before
   completion.
6. Choose Map hiding and uncovering the same Skirmish parent without parent
   transition replay.
7. Successful and failed Start paths, including no close on failed validation.
8. Blank reveal initialization, count-1 first paint, paint-after-draw increment,
   target behavior, retained running state, and dynamic-text restart.
9. UTF-16/code-unit fixtures after the native counting mechanism is verified.
10. Entry sound and completion-hook ordering, plus absence of an invented close
    cue.
11. Retail asset frame ranges and dimensions for every referenced transition
    animation.

Parity acceptance additionally requires gamemd-derived frame comparisons for
every entry and close tick of `0xE2`, `0x100`, `0x102`, and `0x6B`, including
640-wide, 800-wide, and representative wider layouts, plus reveal progression
captures. Rust-vs-Rust screenshots, hand-authored goldens, and passing unit tests
are regression evidence only; they do not certify parity.

Until those executable comparisons or an exhaustive equivalent proof pass, the
honest status remains `UNVERIFIED` even when implementation tests are green.

## Handoff

The implementation plan should begin with the six evidence gates, then introduce
the lifetime controller and verified dialog specs, migrate navigation one route at
a time, add reveal paint semantics, integrate special chrome only after its
evidence is reconciled, and finish with gamemd-derived visual comparisons.

The plan must account for the dirty shared worktree and inspect current versions
of all touchpoints before assigning edits. It must not overwrite or normalize
unrelated concurrent shell work.
