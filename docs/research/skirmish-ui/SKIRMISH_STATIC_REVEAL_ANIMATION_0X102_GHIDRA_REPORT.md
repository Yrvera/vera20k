# Skirmish Static Reveal Animation 0x102 - Ghidra Research Report

**Address(es):** `OwnerDraw_Static_006153E0 @ 0x006153E0`, `FUN_0060A5B0 @ 0x0060A5B0`, `FUN_00602490 @ 0x00602490`, `FUN_0060AA60 @ 0x0060AA60`, `FUN_00622B50 @ 0x00622B50`, `FUN_006071E0 @ 0x006071E0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard offline Skirmish setup dialog `0x102` kind-1 Static text reveal state for controls `0x694` title, `0x6EC` game type, and `0x5A8` map label: classification, first-paint state, start messages, timer cadence, reveal count/range/step, paint advancement, stop condition, text-update restart behavior, YR liveness, and Rust handoff.  
**Non-Scope:** Static rect/color/alignment re-audit, non-static labels and edit text, `0x695` status child, Choose Map modal layout, BitFont glyph raster internals, and runtime screenshot capture.  
**Confidence:** High for binary state/timing/count/order and normal YR liveness gates; Medium for exact visual pixels during the transition frame sequence because no runtime capture was taken.  
**Active in YR:** Conditional. The kind-1 reveal code is live in standard YR shell controls and standard offline `0x102` controls qualify, but it starts only when the shell sends `0x4EC`/child `0x4EE`; ordinary first paint and the `FUN_006071E0` `DL=0` common-paint path do not start it.

## Working Notes

- Target question: Does standard offline Skirmish dialog `0x102` run a Static kind-1 text reveal animation for the right-panel title/game/map statics, and what are the count/order/timing/liveness details?
- Non-goals: Do not re-audit settled rects/colors/alignments/chrome, non-static controls, edit control `0x6A0`, status child `0x695`, or Choose Map modal visuals.
- Evidence needed to mark COMPLETE: Prior docs plus Ghidra decompile/xref/disassembly evidence for classification, first-paint state, `0x4EC -> 0x4EE` start, timer cadence, paint advancement, end condition, text-update restart, and Rust surfaces.
- Stop conditions: Every scoped open question resolved or explicitly deferred; no Ghidra mutations; write only this report plus shared claims if needed.

## 1. Overview

The right-panel Skirmish title/game/map Static controls are configured as kind `1` animated text controls. Init gives them reveal state, but does not make them draw immediately: the running byte is clear and `WM_PAINT` skips kind-1 text until a child receives `0x4EE`.

The reveal is a timer-driven character/window count, not a separate right-panel sprite animation. The shell-level `0x4EC` broadcast enumerates qualifying children and sends `0x4EE`; `0x4EE` starts timer `0`, resets reveal count to `1`, invalidates the child, and subsequent paints pass count/range into `FUN_00621040`.

## 2. Class Layout / Key Offsets

Offsets below are per-HWND owner-draw record offsets as seen through the decompiler's `int *piVar11` indexing in `OwnerDraw_Static_006153E0`; byte offset is index times four.

| Offset | Type | Meaning | Active in YR | Evidence |
|---:|---|---|---|---|
| `+0x70` | `i32` | Static kind; `1` means animated text reveal. | Yes | `FUN_0060A5B0` writes `piVar8[0x1C]=1`; `OwnerDraw_Static_006153E0` tests `piVar11[0x1C]`. |
| `+0x80` | `i32` | Current reveal count passed to `FUN_00621040`. | Yes | `FUN_0060A5B0` and `0x4EE` set it to `1`; paint advances it at `0x00615B23..0x00615B37`. |
| `+0x84` | `u32` | Timer interval in ms for timer `0`. | Yes | `FUN_00600CA0` returns `0x1E` for `0x102` controls `0x694/0x6EC/0x5A8`; `0x00616005..0x00616016` passes it to `SetTimer`. |
| `+0x88` | `i32` | Reveal step added after each successful kind-1 paint. | Yes | `FUN_006015E0` returns `1` for these controls; paint adds `[ESI+0x88]` at `0x00615B2D..0x00615B37`. |
| `+0x8C` | `i32` | Reveal range / trailing fade window passed to text wrapper. | Yes | `FUN_00601D20` returns `8`; call-site passes `[ESI+0x8C]` before `FUN_00621040` at `0x00615ACB..0x00615AE8`. |
| `+0x90` | `i32` | Optional sound id; `-1` for scoped Skirmish labels. | Yes as silent branch | `FUN_0060A5B0` writes `-1` unless score-dialog special case; paint skips sound when `[ESI+0x90] == -1` at `0x00615AED..0x00615B09`. |
| `+0xA8` | byte | Animation running byte; kind-1 text draws only when nonzero. | Yes | `FUN_0060A5B0` clears it; `0x4EE` sets it at `0x00615FF9`; paint gate in decompile requires it. |
| `+0x28` | wide text pointer | Owned visible text buffer used for length and draw. | Yes | Static thunk report verifies `0x00610CA0` copies `0x4B2` text here; paint reads `[ESI+0x28]`. |

## 3. Core Logic

### 3.1 Classification And Defaults

Active in YR: Yes. `FUN_00602490` returns true for dialog id `0x102` with child ids `0x694`, `0x6EC`, and `0x5A8`. Evidence: decompile `FUN_00602490 @ 0x00602490`; the first branch covers `0x694` when parent id is `0x102`, and the second branch covers `0x6EC/0x5A8` when parent id is `0x102`.

Active in YR: Yes. During common shell setup, `FUN_00622B50` handles `WM_INITDIALOG`, subclasses recognized shell children, then enumerates children through `FUN_0060A5B0`. `FUN_0060A5B0` calls `FUN_00602490`; if true, it writes kind `1`, clears running byte, sets reveal count `1`, loads interval/step/range, and writes sound id `-1` for standard `0x102`. Evidence: decompile `FUN_00622B50`; decompile `FUN_0060A5B0`; assembly context `0x0060A5E0..0x0060A690`.

For standard offline `0x102`:

| Control | Kind | Initial running byte | Initial count | Interval | Step | Range | Sound |
|---|---:|---:|---:|---:|---:|---:|---:|
| `0x694` title | `1` | `0` | `1` | `0x1E` ms | `1` | `8` | `-1` |
| `0x6EC` game type | `1` | `0` | `1` | `0x1E` ms | `1` | `8` | `-1` |
| `0x5A8` map label | `1` | `0` | `1` | `0x1E` ms | `1` | `8` | `-1` |

### 3.2 First-Paint State

Active in YR: Yes as a negative first-paint rule. `OwnerDraw_Static_006153E0` draws kind `0` text unconditionally when text exists, but kind `1` text additionally requires the running byte at `+0xA8` to be nonzero. Since `FUN_0060A5B0` initializes the scoped controls with running byte `0`, ordinary first paint after setup does not draw their text unless `0x4EE` has already been sent.

Evidence: decompile `OwnerDraw_Static_006153E0`; kind/paint branch reads `piVar11[0x1C]`, requires `piVar11[10] != 0`, and for kind `1` checks `(char)piVar11[0x2A] != 0`. Classification init clears the same byte in `FUN_0060A5B0`.

### 3.3 Reveal Start Message

Active in YR: Conditional. The child start message is `0x4EE`. In `OwnerDraw_Static_006153E0`, `0x4EE` starts only when kind is `1` and running byte is currently zero. It then sets running byte `1`, resets count to `1`, starts timer `0` using interval `+0x84`, and invalidates the child with erase `FALSE`.

Evidence:

- Decompile `OwnerDraw_Static_006153E0`, case `0x4EE`.
- Assembly `0x00615FDB..0x00616026`: compares kind `+0x70` to `1`, tests running byte `+0xA8`, writes `+0xA8 = 1`, writes `+0x80 = 1`, pushes `timer_id=0`, pushes interval from `+0x84`, calls `SetTimer` at `0x00616016`, then calls `InvalidateRect` at `0x00616026`.

The parent message that starts the child reveal is `0x4EC`. `FUN_00622B50` handles `0x4EC` by `EnumChildWindows(parent, FUN_0060AA60, 0)`, and `FUN_0060AA60` calls `FUN_00602490`; only qualifying children receive `0x4EE`. Evidence: decompile `FUN_00622B50`; decompile `FUN_0060AA60`; assembly `0x0060AA70..0x0060AA83` sends `0x4EE` only after `FUN_00602490` returns nonzero.

### 3.4 Timer And Paint Advancement

Active in YR: Yes after `0x4EE`. Timer id `0` does not directly increment the reveal count. `WM_TIMER` for timer id not `0x65` invalidates kind `1` controls; the count advances during the subsequent `WM_PAINT`.

Evidence: decompile `OwnerDraw_Static_006153E0`, `WM_TIMER (0x113)` branch. For timer id not `0x65`, kind `1` immediately calls `InvalidateRect(hwnd, NULL, TRUE)` and returns.

Active in YR: Yes. On each kind-1 paint while running, the proc calls `FUN_00621040` with current count and range, then computes:

```text
target = wcslen(text) + 1 + reveal_range
if current_count < target:
    current_count += step
    if current_count >= target:
        KillTimer(hwnd, 0)
```

For the scoped Skirmish labels, `step = 1` and `range = 8`, so the final threshold is `wcslen(text) + 9`. If the text is empty, the target is `9`; because text pointer must be non-null to enter the draw branch, an empty string still drains through count `1..9` if a non-null empty buffer is present.

Evidence: decompile `OwnerDraw_Static_006153E0`; assembly `0x00615AE8` call to `FUN_00621040`; assembly `0x00615B11..0x00615B49` calls `wcslen`, adds range and `1`, compares current count, adds step, stores count, and calls `KillTimer` when the new count reaches/exceeds target.

The running byte is not cleared when the reveal completes. `KillTimer(hwnd, 0)` stops the timer, leaving running byte nonzero and count at or past target. Later paints still draw the full static text because kind `1` remains running. Evidence: no running-byte clear in `0x00615B11..0x00615B49`; destroy and restart paths clear it separately.

### 3.5 Text Update Restart

Active in YR: Yes for `0x6EC` and `0x5A8`; not observed for `0x694` during ordinary setup. `FUN_005E2EF0` sends `0x4B2` to child `0x6EC`; `FUN_005E2F60` sends `0x4B2` to child `0x5A8`. The common subclass thunk at `0x00610CA0` owns the text copy. If the record is kind `1` and the running byte is nonzero, the thunk kills timer `0`, clears running byte, and sends `0x4EE` to the same child, restarting the reveal at count `1`.

Evidence: decompile `FUN_005E2EF0`; decompile `FUN_005E2F60`; prior report `SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md`; assembly `0x00611C72..0x00611CAF` compares kind `+0x70`, tests running byte `+0xA8`, calls `KillTimer`, clears `+0xA8`, then sends `0x4EE`.

### 3.6 Shell Transition Liveness

Active in YR: Conditional and not TS legacy. No INI or TS-only flag gates the scoped `0x102` classification or child reveal code. The gate is shell message flow.

The common deferred paint call to `FUN_006071E0` is not a reveal start. `FUN_00622B50` common `WM_PAINT` calls `FUN_006071E0` with `DL=0`; that path sends `0x4ED`, not `0x4EC`, and `FUN_006AE3F0` has no standard Skirmish `0x4ED` reveal handler. Evidence: assembly `0x00622CA6..0x00622CAA` zeroes `DL` and calls `0x006071E0`; assembly/decompile in `FUN_006071E0` sends `0x4ED` on zero mode; prior report `SKIRMISH_FUN_006071E0_SHELL_TRANSITION_REDRAW_PATH_GHIDRA_REPORT.md`.

The nonzero transition path is a reveal start. `FUN_00608260` calls `FUN_006071E0` with `DL=1`; `FUN_006071E0` then sends `0x4EC`, which broadcasts `0x4EE` to qualifying children. Evidence: assembly `0x0060833F..0x00608343`; `FUN_006071E0` sends `0x4EC` after nonzero mode; `FUN_0060AA60` child broadcast at `0x0060AA79..0x0060AA83`.

Read-only xrefs prove normal shell callers exist: `FUN_006071E0` is called from `FUN_00622B50` at `0x00622CAA` and from `FUN_00608260` at `0x00608343`; `FUN_00608260` has callers at `0x005E6B49` and `0x00612690`. This report did not expand into a full taxonomy of every user action that reaches those callers.

## 4. INI Keys

No INI keys drive this slice. Timing and reveal count/range are hardcoded shell-control classification constants in `FUN_00600CA0`, `FUN_006015E0`, and `FUN_00601D20`, selected by parent dialog id and child id. Active in YR: Yes for standard shell code; not TS legacy.

## 5. Integration Points

| Integration | Behavior | Active in YR | Evidence |
|---|---|---|---|
| `WM_INITDIALOG` common shell setup | Subclasses children and runs `FUN_0060A5B0` classifier. | Yes | `FUN_00622B50`, `FUN_0060F9A0`, `FUN_0060A5B0`. |
| `0x4EC` parent message | Broadcasts reveal start to qualifying children. | Conditional | `FUN_00622B50`; xref to `FUN_0060AA60` at `0x006230C2`. |
| `0x4EE` child message | Starts reveal only when kind `1` and not already running. | Conditional | `OwnerDraw_Static_006153E0`, assembly `0x00615FDB..0x00616026`. |
| `WM_TIMER` id `0` | Invalidates kind-1 child; paint advances count. | Conditional | `OwnerDraw_Static_006153E0` timer branch. |
| `WM_PAINT` child | Draws only if text exists and kind `1` running byte is nonzero; advances count and can kill timer. | Conditional | `0x00615AE8`, `0x00615B11..0x00615B49`. |
| `0x4B2` text update | `0x6EC/0x5A8` text changes can restart reveal if already running. | Yes/Conditional | `FUN_005E2EF0`, `FUN_005E2F60`, thunk assembly `0x00611C72..0x00611CAF`. |

## 6. Current Rust Implementation Status

Rust currently renders these labels in steady state only:

- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs` has `push_static_label_draw`, which sends the full label to `push_text_draw` and then `shell_text::draw_in_rect`.
- `C:/Users/enok/Documents/ra2-rust-game/src/render/shell_text.rs::draw_in_rect` supports rect clipping and alignment, but has no reveal count/range parameter.
- `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs::SkirmishShellState` has no static reveal state, timer state, transition event, or per-label reveal count.
- `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs` already matches the settled right-panel text rects; this report does not ask for geometry changes.

Current Rust delta: missing transition-triggered reveal state. However, Rust should not hide these labels on first paint unless it also models the shell transition/start event that sends `0x4EC -> 0x4EE`; otherwise the UI would regress to blank right-panel text.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x694/0x6EC/0x5A8` classification | verified | `FUN_00602490`, `FUN_0060A5B0` | none |
| Init defaults for kind/count/interval/step/range/sound | verified | `FUN_0060A5B0`, `FUN_00600CA0`, `FUN_006015E0`, `FUN_00601D20` | none |
| First-paint skip before `0x4EE` | verified | `OwnerDraw_Static_006153E0` paint branch | runtime screenshot not captured |
| `0x4EC -> 0x4EE` broadcast | verified | `FUN_00622B50`, `FUN_0060AA60`, `0x0060AA79..0x0060AA83` | none for message identity |
| Child `0x4EE` start | verified | `OwnerDraw_Static_006153E0`, `0x00615FDB..0x00616026` | none |
| Timer id `0` invalidation | verified | `OwnerDraw_Static_006153E0` timer branch | exact OS timer coalescing/runtime jitter not captured |
| Paint count advancement and stop | verified | `0x00615AE8`, `0x00615B11..0x00615B49` | none |
| `0x4B2` update restart | verified | `0x00611C72..0x00611CAF`, `FUN_005E2EF0`, `FUN_005E2F60` | exact visible strings not runtime-read |
| `FUN_006071E0` common-paint vs nonzero transition split | verified | `0x00622CA6..0x00622CAA`, `0x0060833F..0x00608343`, prior transition report | full caller taxonomy deferred |
| Rust reveal implementation | verified missing | source scan of `app_skirmish_shell_render.rs`, `shell_text.rs`, `state.rs` | future implementation |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which `0x102` Static controls are in scope? -> `0x694`, `0x6EC`, and `0x5A8` qualify as kind-1 static text controls.` (evidence: `FUN_00602490`, `FUN_0060A5B0`)
- `[RESOLVED] OQ-02 - Is the reveal active in standard YR or TS-only? -> Active in standard YR shell code; no INI/TS-only flag gates it, but message flow conditionally starts it.` (evidence: `FUN_00602490`, `FUN_00622B50`, `FUN_0060AA60`)
- `[RESOLVED] OQ-03 - What is first-paint state? -> kind `1`, running byte `0`, count `1`; ordinary paint does not draw kind-1 text until `0x4EE`.` (evidence: `FUN_0060A5B0`, `OwnerDraw_Static_006153E0`)
- `[RESOLVED] OQ-04 - What starts reveal? -> parent `0x4EC` enumerates children; `FUN_0060AA60` sends child `0x4EE` only to `FUN_00602490` qualifiers.` (evidence: `FUN_00622B50`, `0x0060AA79..0x0060AA83`)
- `[RESOLVED] OQ-05 - What does `0x4EE` write? -> running byte `1`, count `1`, timer `0` with interval `+0x84`, child invalidation erase `FALSE`.` (evidence: `0x00615FDB..0x00616026`)
- `[RESOLVED] OQ-06 - What are scoped interval/step/range values? -> `0x1E` ms, step `1`, range `8` for all three scoped controls.` (evidence: `FUN_00600CA0`, `FUN_006015E0`, `FUN_00601D20`)
- `[RESOLVED] OQ-07 - Does timer directly increment count? -> No; timer invalidates, paint increments count.` (evidence: `OwnerDraw_Static_006153E0` timer branch; `0x00615B11..0x00615B49`)
- `[RESOLVED] OQ-08 - What is the stop condition? -> after draw, if old count is below `wcslen(text)+1+range`, add step; kill timer when new count reaches/exceeds target.` (evidence: `0x00615B11..0x00615B49`)
- `[RESOLVED] OQ-09 - Is running byte cleared on completion? -> No, completion kills timer only; full text remains drawn on later paints.` (evidence: `0x00615B3F..0x00615B49`; no clear in completion branch)
- `[RESOLVED] OQ-10 - Do text updates restart reveal? -> Yes, if kind `1` and already running, `0x00610CA0` kills timer, clears running byte, sends `0x4EE`.` (evidence: `0x00611C72..0x00611CAF`)
- `[RESOLVED] OQ-11 - Does common `WM_PAINT` transition start reveal? -> No; its `DL=0` call sends `0x4ED`, not `0x4EC`.` (evidence: `0x00622CA6..0x00622CAA`, transition report)
- `[RESOLVED] OQ-12 - Does nonzero shell transition start reveal? -> Yes; `FUN_00608260` calls `FUN_006071E0` with `DL=1`, which sends `0x4EC`.` (evidence: `0x0060833F..0x00608343`, transition report)
- `[RESOLVED] OQ-13 - Are INI keys involved? -> No scoped INI keys; constants are selected by shell code based on dialog/control ids.` (evidence: decompile `FUN_00600CA0`, `FUN_006015E0`, `FUN_00601D20`)
- `[RESOLVED] OQ-14 - Does Rust currently model reveal state? -> No per-label reveal/timer/transition state found.` (evidence: source scan of `SkirmishShellState`, `push_static_label_draw`, `shell_text::draw_in_rect`)
- `[DEFERRED] OQ-15 - Exact retail screenshot pixels during transition frames.` (category: `needs-runtime-debugger`; reason: static Ghidra proves state/timing and message order, not final captured pixels; next-step-if-pursued: capture standard retail `0x102` opening and nonzero transition frames)
- `[DEFERRED] OQ-16 - Full user-action taxonomy for every `FUN_00608260` caller.` (category: `out-of-scope`; reason: this slice only needed liveness and reveal contract; next-step-if-pursued: trace callers `0x005E6B49` and `0x00612690` as separate shell-transition investigation)
- `[DEFERRED] OQ-17 - Status child `0x695` text source/reveal behavior.` (category: `out-of-scope`; reason: assigned to separate swarm slot; next-step-if-pursued: investigate `SKIRMISH_STATUS_CHILD_0X695_TEXT_SOURCE`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x694/0x6EC/0x5A8` are kind-1 statics initialized with running `0`, count `1`, interval `30ms`, step `1`, range `8`. | `FUN_00602490`, `FUN_0060A5B0`, `FUN_00600CA0`, `FUN_006015E0`, `FUN_00601D20` | Missing state | `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs` | Add per-label reveal state only when modeling shell transition/reveal events. | Deterministic harness starts Skirmish shell before reveal event: labels are configured but no reveal timer is active until event. Proposed test: `skirmish_static_reveal_initial_state_waits_for_4ee` | Do not unconditionally hide steady-state labels in Rust without also modeling the event that starts them. |
| Parent `0x4EC` broadcasts `0x4EE`; child `0x4EE` sets running, resets count to `1`, starts timer `0` at `0x1E`, and invalidates erase `FALSE`. | `FUN_00622B50`, `0x0060AA79..0x0060AA83`, `0x00615FDB..0x00616026` | Missing transition event model | Future shell transition controller plus `src/ui/skirmish_shell/state.rs` | Represent `0x4EC -> 0x4EE` as an explicit reveal-start event for the three right-panel statics. | Trigger reveal event: all three scoped labels enter running state with count `1` and 30ms cadence. Proposed test: `skirmish_static_reveal_4ec_starts_three_right_panel_statics` | Do not treat `0x4ED` as an alias for `0x4EC`. |
| Each timer tick invalidates; each paint draws with current count/range, then increments count by `1` until `wcslen(text)+1+8`, killing timer but leaving running true. | `OwnerDraw_Static_006153E0`; `0x00615AE8`; `0x00615B11..0x00615B49` | Missing reveal count/range in renderer | `src/render/shell_text.rs`, `src/app_skirmish_shell_render.rs` | Add a text reveal/count parameter or wrapper that reproduces count/range clipping without changing settled rect/alignment. | For a 10-character map label, reveal advances one count per paint after timer invalidation and stops timer at threshold `19`, with full text still drawn after completion. Proposed test: `skirmish_static_reveal_count_advances_on_paint_and_stops_timer` | Do not increment count in the timer callback before paint; binary increments after drawing. |
| `0x4B2` updates to `0x6EC/0x5A8` restart an already-running reveal by killing timer, clearing running, and sending `0x4EE`. | `FUN_005E2EF0`, `FUN_005E2F60`, `0x00611C72..0x00611CAF` | Missing dynamic text restart | `src/ui/skirmish_shell/state.rs`, map/mode selection update path | If changing game type/map label while reveal is running, restart that label from count `1`; after completed reveal, binary still has running true so update also restarts. | During an active reveal, selecting a different map restarts the map label reveal from count `1`. Proposed test: `skirmish_map_label_update_restarts_running_static_reveal` | Do not only update the string buffer while keeping old reveal count. |

## Negative Facts / Do Not Do

- Do not start static reveal from standard common `WM_PAINT`/`FUN_006071E0` `DL=0`. Active in YR: Yes as a negative rule. Evidence: `0x00622CA6..0x00622CAA` passes `DL=0`; zero mode sends `0x4ED`, not `0x4EC`.
- Do not collapse `0x4ED` and `0x4EC`. Active in YR: Yes. Evidence: `FUN_006071E0` has distinct zero/nonzero mode sends; `FUN_00622B50` handles `0x4EC` for reveal, while Skirmish proc `0x006AE3F0` has no `0x4ED` reveal handler.
- Do not increment reveal count from the timer callback. Active in YR: Yes. Evidence: timer branch invalidates for kind `1`; count arithmetic occurs after `FUN_00621040` in paint at `0x00615B11..0x00615B49`.
- Do not clear running byte when reveal completes. Active in YR: Yes. Evidence: completion branch calls `KillTimer` at `0x00615B49` but does not write `+0xA8`; destroy/restart paths clear it elsewhere.
- Do not include edit control `0x6A0` or status child `0x695` in this right-panel static reveal implementation from this report alone. Active in YR: out-of-scope. Evidence: `0x6A0` has separate edit report; `0x695` is gated by helper conditions in `FUN_00602490` and assigned to a separate swarm slot.

## Remaining Uncertainty

- Exact captured pixels during retail transition/reveal were not sampled; static evidence proves state, timing constants, message order, and count math.
- Full caller taxonomy for every path into `FUN_00608260` remains outside this slice; xrefs prove liveness, not every user-visible trigger.
- `0x695` status-child text/reveal behavior is intentionally deferred to its own swarm slot.

## Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`: replace the handoff phrase "add reveal timing only if implementing first-paint animation" with "add reveal timing only as a `0x4EC -> 0x4EE` transition/text-update animation; standard common first paint and `0x4ED` do not start reveal."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`: replace "This is active when the shell posts/sends `0x4EC`; `FUN_006071E0` contains a confirmed `SendMessageA(parent, 0x4EC, 0, 0)` after shell transition playback." with "`FUN_006071E0` sends `0x4EC` only in the nonzero-`DL` transition path; the common `WM_PAINT` deferred caller passes `DL=0` and sends `0x4ED`, which does not start reveal for standard `0x102`."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md`: append to the restart wording: "Because the completed reveal leaves running byte nonzero, a later `0x4B2` text change can restart the reveal even after the timer was killed at completion."

## Sources

- Ghidra read-only decompile/assembly: `OwnerDraw_Static_006153E0 @ 0x006153E0`, `FUN_0060A5B0 @ 0x0060A5B0`, `FUN_00602490 @ 0x00602490`, `FUN_00600CA0 @ 0x00600CA0`, `FUN_006015E0 @ 0x006015E0`, `FUN_00601D20 @ 0x00601D20`, `FUN_00622B50 @ 0x00622B50`, `FUN_0060AA60 @ 0x0060AA60`, `FUN_006071E0 @ 0x006071E0`, `FUN_00608260 @ 0x00608260`, `FUN_006AE3F0 @ 0x006AE3F0`, `FUN_005E2EF0 @ 0x005E2EF0`, `FUN_005E2F60 @ 0x005E2F60`.
- Assembly contexts: `0x00615FDB..0x00616026`, `0x00615AE8`, `0x00615B11..0x00615B49`, `0x0060AA79..0x0060AA83`, `0x00622CA6..0x00622CAA`, `0x0060833F..0x00608343`, `0x00611C72..0x00611CAF`.
- Prior docs: `SKIRMISH_0X102_STATIC_TEXT_RECTS_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_FUN_006071E0_SHELL_TRANSITION_REDRAW_PATH_GHIDRA_REPORT.md`, `SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md`, `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`, `SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`.
- Rust read-only scan: `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/render/shell_text.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`.
