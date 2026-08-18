# Skirmish FUN_006071E0 Shell Transition Redraw Path - Ghidra Research Report

**Address(es):** `0x006071E0` primary; key callers `0x00622CAA`, `0x0060805B`, `0x00608343`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `FUN_006071E0` only as it affects standard offline Skirmish dialog `0x102` shell UI layout/paint: transition playback, broadcasts, text reveal start, right-panel redraw flags, invalidation, and visible layout/draw-order implications.
**Non-Scope:** whole shell framework, WOL dialogs, gameplay launch/session flow, low-level `CC_Draw_Shape` raster contract, and full global writer sweep for every owner-draw flag byte.
**Confidence:** High for the scoped call modes, message broadcasts, conditional activity, and no-persistent-layout conclusion. Medium for exact visual contents of every transition frame because this pass did not capture runtime screenshots.
**Active in YR:** Conditional. The function is live in YR shell UI paths. For standard offline Skirmish `0x102`, the common-paint deferred path is active only when parent record byte data `+0xBE` is set; the direct transition path is active when callers such as `0x00608260` invoke it on a visible non-WOL shell-mode dialog.

## 1. Overview

`FUN_006071E0` is not the normal first-paint layout or static draw-order function. It is a transition/redraw animation helper that paints temporary frames directly through the main display surfaces, sleeps `0x1E` ms between frames, and then broadcasts one of two custom messages depending on its `DL` mode.

For standard offline Skirmish `0x102`, the common `WM_PAINT` caller at `0x00622CAA` passes `DL=0`. That mode does not start text reveal; it sends `0x4ED`, which the standard Skirmish proc and common shell proc do not handle as the `0x4EC -> 0x4EE` text-animation start path. The `0x4EC` broadcast exists, but only after a nonzero-`DL` transition call.

## 2. Class Layout / Key Offsets

Offsets are data-record offsets unless noted. The HWND hash node stores the data record at node `+4`, so assembly of the form `ADD EAX, 0x4; [EAX+N]` means data `+N`.

| Offset | Purpose in this slice | Evidence | Active in YR |
|---:|---|---|---|
| data `+0xBE` | Deferred transition/redraw request consumed by common `WM_PAINT`; caller clears it after `FUN_006071E0`. | `0x00622C92..0x00622CAF`; `0x00608070` writes root `+0xC2` equivalent to data `+0xBE`. | Conditional; yes when `0x00608070` schedules it. |
| data `+0xC1` | Shell-mode eligibility byte used by direct transition helper `0x00608260`. | `0x00608260` checks data `+0xC1`; prior `0x0060C540` writes it for `0x102`. | Yes for standard offline `0x102` after common init. |
| data `+0xB4` | Paint mode; `1` is required by `0x00608260` before direct transition playback. | `0x00608260` checks `piVar1[0x2D] == 1`; prior `0x0060C540` writes mode `1` for `0x102`. | Yes for standard offline `0x102`. |
| data `+0xD5` | Optional transition draw flag read into the per-frame path. | `0x006076DE` reads `[data+0xD5]`. | Conditional; default zero unless another shell helper sets it. |
| data `+0xD6` | Optional transition draw flag enabling one right-panel/child group. | `0x00607294` reads `[data+0xD6]`; branch at `0x006076FC` skips that group when zero. | Conditional; default zero unless another shell helper sets it. |
| data `+0xD7` | Optional transition draw flag enabling a radar/right-panel extra blit group. | `0x0060727D` reads `[data+0xD7]`; later checked at `0x00607D3C`. | Conditional; default zero unless another shell helper sets it. |
| data `+0xD4` | Separate `SDBTNANM` frame-10 gate; not changed by `FUN_006071E0`. | Read/write inventory in `SKIRMISH_SDBTNANM_FRAME10_FIRST_PAINT_FLAG_GHIDRA_REPORT.md`; `0x006071E0` reads `+0xD5..+0xD7`, not `+0xD4`. | Yes as separate paint gate; not this function's state change. |

## 3. Core Logic

### 3.1 Call Signature And Modes

Active in YR: Yes, conditional by caller. Assembly shows the function consumes `ECX` as the parent/dialog HWND and `DL` as the transition mode:

- `0x006071F0`: `MOV ESI, ECX`, preserving the HWND.
- `0x006071F5`: stores `DL` to the stack mode byte.
- `0x00607F39..0x00607F48`: tests that mode byte after playback.

The standard common-paint caller is the `DL=0` mode:

- `0x00622CA8`: `MOV ECX, ESI`
- `0x00622CA6`: `XOR DL, DL`
- `0x00622CAA`: `CALL 0x006071E0`

The direct transition helper uses the `DL=1` mode:

- `0x0060833F`: `MOV DL, 0x1`
- `0x00608341`: `MOV ECX, ESI`
- `0x00608343`: `CALL 0x006071E0`

### 3.2 Transition Frame Playback

Active in YR: Conditional. When called, the helper snapshots surface rectangles and global shell art pointers, enumerates child controls, builds a small per-control timing array, then loops for `max(schedule) + 6` frames.

Load-bearing constants and order:

- The timing array is allocated as `(count + 3) * 4` bytes, filled with ascending values beginning at `1`, then has three sentinel/extra slots adjusted before computing `max + 6` (`0x00607646..0x006076AD`).
- Each frame draws shell shapes with `CC_Draw_Shape` using flags `0x400` and draw argument `1000` (`0x00607749..0x00607E75` call patterns).
- The frame window for animated transitions is six frames: branches compare per-item age with `< 6` at `0x0060773F`, `0x00607B74`, and `0x00607D7D`.
- The helper flushes/unlocks both display surfaces, calls `FUN_00406F70`, then sleeps `0x1E` ms each loop (`0x00607E8A..0x00607F11`).

This is temporary transition playback, not a persistent layout edit. No `SetWindowPos`, `MoveWindow`, or child rect write appears inside `FUN_006071E0`; child positions are only read through enumeration and window/surface coordinate conversion.

### 3.3 Child Counts And Right-Panel Groups

Active in YR: Yes for the helper; conditional for each child group. The helper counts visible, enabled, owner-draw shell controls through two enum callbacks:

- `FUN_0060A180 @ 0x0060A180` increments `DAT_00AC1CAC` for visible controls where `FUN_00608CD0` returns true.
- `FUN_0060A250 @ 0x0060A250` increments `DAT_00AC4894` for visible controls where `FUN_00609730` returns true.

For dialog `0x102`, `FUN_00608CD0` includes `0x694`, `0x468`, `0x6EC`, `0x5AA`, `0x5A8`, and `0x617`; `FUN_00609730` includes the Back button `0x5C0`. Evidence: `FUN_00608CD0` and `FUN_00609730` decompiles. Active in YR: Yes for standard offline `0x102` classification; actual transition membership is conditional on visibility and owner-draw metadata.

The optional right-panel/extra draw groups are gated by data `+0xD5`, `+0xD6`, and `+0xD7`; a fresh standard `0x102` first-paint path does not set these bytes in the checked init functions. Evidence: explicit reads at `0x0060727D`, `0x00607294`, `0x006076DE`; first-paint zero-fill in `FUN_00623340` from the frame-10 report; `0x0060C540` writes `+0xB4/+0xC1`, not `+0xD5..+0xD7`. Active in YR: Conditional.

### 3.4 End Broadcasts

Active in YR: Yes, mode-dependent.

If `DL == 0`, the helper sends `0x4ED` to the parent and returns:

- `0x00607F39..0x00607F48`: mode byte tested, zero jumps to `0x00607FA8`.
- `0x00607FAE`: pushes message `0x4ED`.
- `0x00607FB4`: sends it to the parent HWND.

If `DL != 0`, the helper plays a shell transition sound, drains the display chain, then sends `0x4EC`:

- `0x00607F4A..0x00607F5F`: uses Rules audio field at `+0x750` through `0x00750920`.
- `0x00607F64..0x00607F8B`: loops display-chain vtable `+0x28/+0x10` until empty.
- `0x00607F95`: pushes message `0x4EC`.
- `0x00607F9B`: sends it to the parent HWND.

`0x4EC` is the text reveal broadcast, not `0x4ED`. `FUN_00622B50` handles `0x4EC` by `EnumChildWindows(parent, FUN_0060AA60, 0)`, and `FUN_0060AA60` sends `0x4EE` to qualifying children. Evidence: `FUN_00622B50` decompile and `0x0060AA79..0x0060AA83`. Active in YR: Yes when `0x4EC` is sent.

For standard Skirmish dialog proc `0x006AE3F0`, `0x4ED` has no local handler after the common proc returns zero. Evidence: `0x006AE417..0x006AE4AB` branches only for `0x497`, `0x0F`, `0x111`, and `0x4E9` after common dispatch. Active in YR: Yes as a negative fact for standard offline `0x102`.

## 4. INI Keys

No INI keys drive this slice. The behavior is shell resource/owner-draw metadata plus Rules audio field `+0x750` on the nonzero-`DL` transition path. Active in YR: Yes for the binary path; no `rulesmd.ini` or `artmd.ini` key was identified or needed for this scoped function.

## 5. Integration Points

Active in YR: Yes, conditional by entry point.

| Entry point | What it does | Evidence | Standard offline `0x102` implication |
|---|---|---|---|
| `FUN_00622B50` `WM_PAINT` | After normal cached parent paint, if data `+0xBE` is nonzero, sends `0x4E2` to child `0x71A`, calls `FUN_006071E0` with `DL=0`, clears `+0xBE`, validates parent. | `0x00622C86..0x00622CB9` | Deferred redraw/transition path only; no text reveal start. |
| `FUN_00608070` | Non-WOL visible mode-1 helper: disables children, sets data `+0xBE`, invalidates, pumps/waits up to `5000` ms until `+0xBE` clears. | `0x00608070` decompile | Schedules the `DL=0` paint-time path; not normal initial paint. |
| `FUN_00608260` | Non-WOL visible mode-1 helper: plays sound, disables children, calls `FUN_006071E0` with `DL=1`, reenables children, invalidates. | `0x00608260` decompile; `0x0060833F..0x00608343` | This is the active `0x4EC` text-reveal start path when invoked later. |
| `FUN_006AE3F0` | Standard Skirmish proc delegates to common shell first; after that it handles preview/start markers on `WM_PAINT`. | `0x006AE404..0x006AE480` | `FUN_006071E0` is before Skirmish preview paint only if common handler's `+0xBE` branch runs. |

## 6. Current Rust Implementation Status

Rust currently has only the steady-state renderer for Skirmish shell composition:

- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:461` isolates `right_panel_frame10_overlay_active`.
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:467` defines the semantic steady-state draw order.
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:488` emits repeated `SDBTNANM` frame-10 overlay when the helper returns true.

Rust does not appear to model `FUN_006071E0` transition playback, its `DL=0/1` broadcast split, or the `0x4EC -> 0x4EE` text reveal start as a separate later shell transition. This is acceptable for first static paint only if Rust does not use the transition helper as an excuse to draw frame-10 overlay or start text reveal immediately.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_006071E0` primary body | verified | `0x006071E0` decompile and assembly contexts | runtime screenshot of transition frames not captured |
| `DL=0` common-paint caller | verified | `0x00622CA6..0x00622CAF` | none for standard deferred redraw |
| `DL=1` direct transition caller | verified | `0x00608260`; `0x0060833F..0x00608343` | exact UI action caller context deferred |
| `0x4EC` text broadcast chain | verified | `0x00607F95`, `FUN_00622B50`, `0x0060AA60` | none for message identity |
| `0x4ED` standard Skirmish handling | verified negative | `0x006AE3F0` handled-message branches | no broader shell audit outside `0x102` |
| Optional flags `+0xD5..+0xD7` | touched-not-exhausted | reads at `0x0060727D`, `0x00607294`, `0x006076DE` | full global writer inventory deferred |
| Steady-state first-paint layout impact | verified negative for this slice | no move/layout APIs in `0x006071E0`; caller conditional `+0xBE` | runtime capture could refine transition visuals |
| Rust render comparison | verified for relevant files | source lines in section 6 | implementation change out of scope |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ1 - Is FUN_006071E0 on a live YR path? -> Yes, conditionally; direct xrefs include common shell paint `0x00622CAA` and transition helper `0x00608343`.` (evidence: `get_function_xrefs 0x006071E0`)`
- `[RESOLVED] OQ2 - What arguments does it consume? -> `ECX` is parent HWND and `DL` is the mode byte.` (evidence: `0x006071F0`, `0x006071F5`)`
- `[RESOLVED] OQ3 - Does standard common paint call it with text-start mode? -> No; common `WM_PAINT` passes `DL=0`.` (evidence: `0x00622CA6..0x00622CAA`)`
- `[RESOLVED] OQ4 - Which message starts text reveal? -> `DL!=0` sends `0x4EC`; common shell then broadcasts `0x4EE` to qualifying children.` (evidence: `0x00607F95`, `0x00622B50`, `0x0060AA60`)`
- `[RESOLVED] OQ5 - What does `DL=0` send? -> It sends `0x4ED`, not `0x4EC`.` (evidence: `0x00607FA8..0x00607FB4`)`
- `[RESOLVED] OQ6 - Does standard Skirmish `0x102` handle `0x4ED` as text reveal? -> No handler found in `0x006AE3F0` after common proc returns zero.` (evidence: `0x006AE417..0x006AE4AB`)`
- `[RESOLVED] OQ7 - Does the helper reposition children or alter layout? -> No layout/move API appears in the function; it reads windows/rects and paints temporary frames.` (evidence: `0x006071E0` decompile; calls observed are enum, surface, draw, sleep, send-message)`
- `[RESOLVED] OQ8 - Does it change the frame-10 steady-state gate? -> No; it reads `+0xD5..+0xD7`, while frame-10 uses separate `+0xD4`.` (evidence: `0x0060727D`, `0x00607294`, `0x006076DE`; frame-10 report `0x00621FEC`)`
- `[RESOLVED] OQ9 - Which standard `0x102` controls are counted for transition scheduling? -> `FUN_00608CD0` includes `0x694/0x468/0x6EC/0x5AA/0x5A8/0x617`; `FUN_00609730` includes `0x5C0`.` (evidence: `FUN_00608CD0`, `FUN_00609730`)`
- `[RESOLVED] OQ10 - Does the helper invalidate by itself? -> No direct `InvalidateRect` inside primary body; callers schedule/cleanup invalidation around it.` (evidence: `0x00608070`, `0x00608260`, `0x00622B50`)`
- `[DEFERRED] OQ11 - Exact visual contents of every transition frame.` (category: `needs-runtime-debugger`; reason: static Ghidra proves ordering and constants but not final pixels; next-step-if-pursued: capture retail transition frames while forcing `+0xBE` and direct `DL=1` paths)`
- `[DEFERRED] OQ12 - Full writer inventory for data `+0xD5..+0xD7`.` (category: `bounded-cost-too-high`; reason: not needed to answer standard first paint or broadcast split; next-step-if-pursued: perform a separate shell flag writer sweep)`

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard common-paint deferred call to `FUN_006071E0` uses `DL=0` and sends `0x4ED`, not text-start `0x4EC`. | `0x00622CA6..0x00622CAA`; `0x00607FAE` | Missing/unchecked transition-message state | `src/app_skirmish_shell_render.rs`; future shell text animation state | Do not start animated shell text reveal as part of standard first paint or `DL=0` deferred redraw. | Open standard offline Skirmish first view: title/map/mode text animation is not started by pretending `0x4ED` is `0x4EC`. Proposed test: `skirmish_first_paint_does_not_broadcast_text_reveal` | Do not collapse `0x4ED` and `0x4EC`; they are different messages in this path. |
| `0x4EC` text reveal occurs only after a nonzero-`DL` transition call, which plays sound/drains display chain first. | `0x00607F4A..0x00607F9B`; `0x0060833F..0x00608343`; `0x0060AA60` | Missing transition event model | Future shell transition controller plus `src/app_skirmish_shell_render.rs` text reveal state | Model `0x4EC -> 0x4EE` as a later transition completion event, not as steady-state render order. | Trigger a transition path that calls the helper with `DL=1`; text reveal starts after the transition completion event. Proposed test: `skirmish_text_reveal_starts_after_forward_transition_completion` | Do not start reveal before the sound/display-chain flush order. |
| `FUN_006071E0` is temporary redraw playback and does not persistently change child layout, preview order, or the steady-state frame-10 gate. | no move/layout APIs in `0x006071E0`; reads `+0xD5..+0xD7` not `+0xD4`; frame-10 report `0x00621FEC` | Existing Rust steady-state renderer should stay separate; overlay helper currently forced true per older code | `src/app_skirmish_shell_render.rs:461`, `:467`, `:488` | Keep transition playback separate from semantic steady-state draw order; do not use this path to justify frame-10 overlay on standard first paint. | Standard first-paint draw-order test has no transition sprites and no frame-10 overlay for offline `0x102`. Proposed test: `skirmish_first_paint_transition_path_does_not_enable_frame10_overlay` | Do not bake transition frames into the normal Skirmish render stack. |

### Negative Facts / Do Not Do

- Do not treat the prior broad claim "`FUN_006071E0` sends `0x4EC` after playback" as unconditional. Evidence: `DL=0` branch sends `0x4ED` at `0x00607FAE`; `0x4EC` is only pushed at `0x00607F95` after `DL!=0`.
- Do not implement `FUN_006071E0` as a first-visible layout mutator. Evidence: no `MoveWindow`/`SetWindowPos`/child placement call in `0x006071E0`; layout movement remains in common init/layout helpers outside this function.
- Do not use this function to justify standard first-paint `SDBTNANM` frame-10 overlay. Evidence: this function reads data `+0xD5..+0xD7`; the frame-10 gate is separate data `+0xD4` read at `0x00621FEC`.
- Do not treat `0x4ED` as a text-reveal alias in standard `0x102`. Evidence: common shell handles `0x4EC`; `0x006AE3F0` does not handle `0x4ED` in its post-common branch.
- Do not assume all buttons/statics participate in transition scheduling. Evidence: membership is filtered by `FUN_00608CD0` and `FUN_00609730` plus visibility and owner-draw metadata, not by raw child enumeration alone.

### Remaining Uncertainty

- Exact pixels of each transition frame remain uncaptured; static evidence proves draw calls, constants, and message ordering, not screenshot-perfect animation.
- Full writer inventory for optional data `+0xD5..+0xD7` remains deferred; standard first-paint implications do not require it, but a full transition implementation would.
- The exact user action/caller taxonomy for every `DL=1` caller is outside this slot; `0x00608260` proves a live non-WOL mode-1 transition path, but this report does not claim every trigger.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md` replacement wording for Section 5 sentence: "`FUN_006071E0` sends `0x4EC` only when called with nonzero `DL` after transition playback; the standard common-paint deferred caller passes `DL=0` and sends `0x4ED`, which does not start the `0x4EC -> 0x4EE` text reveal path for dialog `0x102`."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md` replacement wording for Open Question 2: "`0x006071E0` is now covered by `SKIRMISH_FUN_006071E0_SHELL_TRANSITION_REDRAW_PATH_GHIDRA_REPORT.md`: it is a conditional transition playback helper, not first-paint steady-state composition; common paint calls it with `DL=0` and sends `0x4ED`, while `DL=1` sends `0x4EC` to start text reveal."

## Sources

- Ghidra decompiled / assembly checked: `0x006071E0`, `0x00622B50`, `0x00608070`, `0x00608260`, `0x0060A180`, `0x0060A250`, `0x00608CD0`, `0x00609730`, `0x0060AA60`, `0x006AE3F0`, `0x00625070`, `0x00624760`, `0x0072A9C0`, `0x0072E280`, `0x0072E2C0`, `0x0072D450`.
- Ghidra xrefs: `get_function_xrefs 0x006071E0`; bulk xrefs for `0x0060AA60`, `0x00607FD0`, `0x00608260`.
- Prior docs referenced: `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`, `SKIRMISH_SDBTNANM_FRAME10_FIRST_PAINT_FLAG_GHIDRA_REPORT.md`.
- Rust source checked: `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/render/skirmish_shell_chrome.rs`.
