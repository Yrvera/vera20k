# Skirmish Native Single Player Route To 0x102 Recheck - Ghidra Research Report

**Date:** 2026-05-27
**Address(es):** `MainMenuDialog0xE2_Proc_00531F60 @ 0x00531F60`, `FUN_00531CC0`, `Main_Game @ 0x0052D9A0`, `FUN_0060D380`, `FUN_006AE2C0`, `FUN_00608260`, `FUN_00608070`, `FUN_00622B50`, `FUN_006071E0`
**Investigation Mode:** coverage-map, scoped to the already-identified route gap and live Rust handoff. The prior caller-chain report is recent and high-confidence, so this pass rechecks the key binary chain and extends the implementation-facing gap map instead of duplicating every prior finding.
**Claimed Scope:** normal main-menu Single Player control `0x683`, native route from return code `1` toward offline Skirmish setup dialog `0x102`, the current Rust shortcut/bridge delta, and what a future strict fix must implement or avoid.
**Non-Scope:** full intermediate Single Player shell resource/control matrix, full pixel capture of transition frames, WOL/network routes, campaign selection internals, and full taxonomy of every caller of `FUN_00608260`.
**Confidence:** High for the main-menu return-code chain and `0x102` launcher. High for current Rust shortcut/bridge surfaces. Medium for the exact user-visible control inside the intermediate Single Player shell that returns `0x0B`, because the containing owner of call site `0x00612690` is still unresolved in the current Ghidra database.
**Active in YR:** Yes. The checked main menu, downstream shell loop, and offline Skirmish launcher are active standard Yuri's Revenge paths, not TS-only legacy.

## 1. Overview

The player-visible mismatch is that Rust reaches the Skirmish setup shell from the main menu through a temporary direct bridge. Native Yuri's Revenge does not treat the main-menu Single Player button as a direct Skirmish launch: it returns code `1`, enters an intermediate shell loop through `FUN_0060D380(1)`, and only a later return code `0x0B` sets `g_GameMode = 5` before `FUN_006AE2C0` creates dialog `0x102`.

This matters because the skipped shell layer owns visible routing, focus, input, sounds, invalidation, and possibly shell transition/reveal timing. Matching only the final `0x102` layout is not enough for route parity.

## 2. Class Layout / Key Offsets

No C++ class layout is fully recovered by this pass. The route uses Win32 dialog result locals, shell-control records, and globals:

| Field / value | Location | Meaning | Evidence |
|---|---:|---|---|
| main-menu result local | local stored at `SetWindowLongA(hwnd, 8, &local_1c)` | Main menu loop result. Initial value `0x12`; button proc writes route codes. | `FUN_00531CC0` |
| Single Player control | `0x683` | Main menu owner-draw button that writes result code `1`. | `MainMenuDialog0xE2_Proc_00531F60` |
| offline Skirmish route code | `0x0B` | Later `Main_Game` route value that sets `g_GameMode = 5`. | `Main_Game` |
| Skirmish game mode | `g_GameMode = 5` | Selects offline Skirmish setup launcher branch. | `Main_Game` |
| Skirmish setup result local | local stored at `SetWindowLongA(hwnd, 8, &local_4)` | `FUN_006AE2C0` pumps until `0x617` Start or `0x5C0` Back. | `FUN_006AE2C0` |
| shell direct-transition eligibility byte | shell record `+0xC1` | Must be nonzero before `FUN_00608260` plays direct transition. | `FUN_00608260`; prior transition report |
| shell paint mode | shell record `+0xB4` (`piVar[0x2D]`) | Must equal `1` for `FUN_00608260`. | `FUN_00608260` assembly/decompile |
| deferred paint byte | shell data `+0xBE` / direct writer equivalent `+0xC2` | Common `WM_PAINT` consumes this and calls `FUN_006071E0` with `DL=0`. | `FUN_00608070`, `FUN_00622B50` |

## 3. Core Logic

### 3.1 Main-menu button identity

`MainMenuDialog0xE2_Proc_00531F60` delegates first to the common shell proc `FUN_00622B50`. If not consumed and the message is `WM_COMMAND (0x111)`, it masks the low word of the command id.

The exact checked mapping is:

| Control id | Native result |
|---:|---:|
| `0x683` Single Player | `1` |
| `0x684` Westwood Online | `2` |
| `0x578` Network | `3` |
| `0x686` Movies/Credits | `4` |
| `0x55C` Options | `5` |
| `0x3EE` Exit | `6` |

The Single Player branch is a local-result write, not a Skirmish setup call.

### 3.2 Main-menu loop

`FUN_00531CC0` creates a shell dialog, stores the result pointer at window long offset `8`, centers/initializes the dialog, sets up RA2TS child `0x71A`, and pumps until the local result changes from `0x12`. It then destroys the dialog via `FUN_00622720` and returns the local result to `Main_Game`.

Important tiny details:

- If dialog creation fails, the result becomes `7`.
- The RA2TS child is positioned at `(screen_width - 800) / 2` and `(screen_height - 600) / 2`, but clamps each axis to `0` when the screen is below the base size.
- At exactly `g_ScreenWidth == 0x280` (640), it sends `"Ra2ts_s"`; otherwise it sends `"Ra2ts_l"`.
- The main menu result remains `0x12` while the dialog is idle.

### 3.3 Main_Game dispatch

`Main_Game` case `1` does:

1. `DAT_00AC10C8 = 0`.
2. `iVar11 = FUN_0060D380(1)`.
3. It does not call `FUN_006AE2C0` in this case.

Later, when the route value is `0x0B`, `Main_Game` executes:

1. `g_GameMode = 5`.
2. Falls into the setup branch.
3. Calls `FUN_006AE2C0()` under `g_GameMode == 5`.
4. If `FUN_006AE2C0()` returns false, resets `g_GameMode = 0` and route value `1`.

This proves the missing route layer is not optional mechanism noise. It is the live switch boundary between the main-menu Single Player result and the offline Skirmish launcher.

### 3.4 Intermediate shell loop

`FUN_0060D380(1)` creates another shell dialog using `FUN_00622650(0)`, stores its result pointer at window long offset `8`, shows and foregrounds it, calls `FUN_0054F720`, and calls `FUN_0052B9B0()` only when the argument is nonzero.

It pumps until its local result becomes nonzero, with this split:

- Always calls `Process_NetworkMessages()`.
- If `g_GameMode == 0` or `g_GameMode == 5` or `DAT_00A8D60E != 0` or `DAT_00A8DAB4 != 0`, it calls `Network_ServiceLoop()`.
- Otherwise it calls `FUN_0055CBF0()` and, if that is false, `Main_Tick()`; a true `Main_Tick()` exits the loop path.

This loop is the native shell layer Rust currently skips.

### 3.5 Offline Skirmish setup launcher

`FUN_006AE2C0`:

1. Initializes house/country state through `FUN_006722F0`, `FUN_00672440`, and per-house vtable calls.
2. Calls `FUN_0072CF40()`.
3. Creates a shell dialog through `FUN_00622650(0)`.
4. Stores the HWND in `DAT_00B0B59C`.
5. Stores a local result pointer with `SetWindowLongA(hwnd, 8, &local_4)`.
6. Calls `FUN_00622800()`.
7. Pumps until result is `0x617` Start, `0x5C0` Back, or `FUN_00623120()` returns `1`.
8. Destroys the dialog, clears `DAT_00B0B59C`, clears `DAT_00AC1154` if set, calls `FUN_0072CF90()` and `FUN_006990A0()`.
9. Returns true only if the result is `0x617`.

No direct call to `FUN_006071E0`, `FUN_00608260`, or `FUN_00608070` appears inside this launcher.

## 4. INI Keys

| INI key | File / default | Effect in this route | Status |
|---|---|---|---|
| `[AudioVisual] ShellButtonSlideSound` | `ini/rules.ini:586`, `ini/rulesmd.ini:712`, empty | Used by the nonzero shell transition path in prior docs, but stock YR resolves to no audible slide sound unless overridden. | verified by INI and prior Ghidra docs |
| `[AudioVisual] GUIMainButtonSound` | `ini/rules.ini:489`, `ini/rulesmd.ini:643`, `MenuClick` | Main-menu and owner-draw button click sound family; exact main-menu `0x683` click path is outside this recheck. | touched-not-exhausted |
| `[AudioVisual] GenericClick` | not re-extracted in this pass | Used by other owner-draw button paint/click paths; not the owner of the native route branch. | deferred |

No INI key was found that replaces the `0x683 -> 1 -> FUN_0060D380(1) -> 0x0B -> g_GameMode=5 -> FUN_006AE2C0` dispatch chain.

## 5. Integration Points

| Boundary | Verified behavior | Evidence |
|---|---|---|
| main-menu proc to result local | `0x683` writes `1`; no Skirmish launcher call occurs there. | `MainMenuDialog0xE2_Proc_00531F60` |
| main-menu loop to `Main_Game` | `FUN_00531CC0` returns the result local after dialog destruction. | `FUN_00531CC0` |
| `Main_Game` Single Player case | case `1` calls `FUN_0060D380(1)`. | `Main_Game` |
| intermediate shell to Skirmish launcher | later route `0x0B` sets `g_GameMode=5`; `g_GameMode==5` calls `FUN_006AE2C0`. | `Main_Game` |
| Skirmish setup dialog | `FUN_006AE2C0` owns `0x102` pump boundary and returns only Start/Back result. | `FUN_006AE2C0` |
| direct transition helper | `FUN_00608260` gates on shell record flags and calls `FUN_006071E0` with `DL=1`. | `0x0060833F..0x00608343` |
| deferred common paint transition | `FUN_00622B50` calls `FUN_006071E0` with `DL=0` when dirty byte is set. | `0x00622CA6..0x00622CAA` |

## 6. Current Rust Implementation Status

Current Rust is explicit bridge/DRIFT code for this path:

| Rust surface | Current behavior | Status |
|---|---|---|
| `src/ui/main_menu_shell/state.rs:44` | `SinglePlayer0x683` maps to `MainMenuShellAction::SinglePlayer`. | matches control identity |
| `src/ui/main_menu_shell/state.rs:56` | `return_code_for_action(SinglePlayer)` returns `Some(1)`. | matches native result code |
| `src/app.rs:1491` | `MainMenuShellAction::SinglePlayer` enters shell window mode and starts `start_main_menu_to_skirmish`. | DRIFT: skips native intermediate shell result loop |
| `src/app_shell_transition.rs:1` | Module says it is a temporary bridge for `main-menu -> Skirmish shell shortcut`. | explicitly non-parity |
| `src/app_shell_transition.rs:14` | Bridge frame cadence uses `30 ms`. | matches one native cadence constant, but mechanism is different |
| `src/app_shell_transition.rs:15` | Bridge uses fixed `14` frames. | DRIFT/unchecked vs native schedule `(child_count + 3)` timing and `max + 6` ticks |
| `src/app_shell_transition.rs:88` | Starting the bridge clears both legacy setup flags and installs `ShellBridgeTransition`. | DRIFT vs native `FUN_0060D380(1)` |
| `src/app_shell_transition.rs:154` | Bridge renders main-menu source and Skirmish destination surfaces simultaneously. | DRIFT: native route is dialog/result-loop based |
| `src/app_shell_transition.rs:190` | Completion sets `main_menu_show_native_skirmish_shell = true`. | DRIFT vs later `0x0B -> g_GameMode=5 -> FUN_006AE2C0` |
| `src/render/shell_transition_pass.rs:1` | Offscreen compositor notes it is not a verified native shell transition. | explicit non-parity |
| `src/app.rs:2117` | Main menu render path gives the bridge first chance, then renders Skirmish shell if active. | endpoint reachable, route mismatches |

The recent layout fix means the final Skirmish shell endpoint can use `compute_layout` and match verified `0x102` positions, but endpoint layout parity does not prove route parity.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x683` main-menu button result | verified | `MainMenuDialog0xE2_Proc_00531F60` decompile | none |
| main-menu dialog pump | verified | `FUN_00531CC0` decompile | full visual composition of main menu outside this route pass |
| `Main_Game` case `1` | verified | `Main_Game` decompile | none |
| `FUN_0060D380(1)` intermediate shell loop | verified for entry/pump/result shape | `FUN_0060D380` decompile | exact dialog resource/control matrix that emits `0x0B` |
| `Main_Game` route `0x0B` | verified | `Main_Game` decompile | exact upstream control/action that writes `0x0B` |
| `FUN_006AE2C0` offline Skirmish launcher | verified | `FUN_006AE2C0` decompile | full internals are covered by separate `0x102` reports |
| `FUN_00608260` direct transition helper | verified for gates and `DL=1` call | decompile plus assembly `0x0060833F..0x00608343` | exact user action taxonomy deferred |
| `FUN_00608070` deferred transition helper | verified for gate/write/wait shape | decompile | all caller contexts deferred |
| `FUN_00622B50` common paint transition | verified for `DL=0`, `0x4ED` split | decompile plus assembly `0x00622CA6..0x00622CAA` | full paint-path already covered in sibling docs |
| call site `0x00612690 -> FUN_00608260` | touched-not-exhausted | Ghidra xrefs show caller address but no containing function in current DB | recover boundary or runtime trace |
| call site `0x005E6B49 -> FUN_00608260` | touched-not-exhausted | Ghidra xrefs; prior docs place it in Choose Map return path | not initial route; no further work for this scope |
| current Rust main-menu result identity | verified | `src/ui/main_menu_shell/state.rs` source scan | none |
| current Rust route implementation | verified mismatch | `src/app.rs`, `src/app_shell_transition.rs`, `src/render/shell_transition_pass.rs` source scan | replace bridge with native route model |
| full framebuffer equality | deferred | no capture in this pass | retail/Rust screenshot diff after native route exists |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is this a duplicate of a recent high-confidence report? -> Partly; the prior report covers the route chain, and this pass is a gap-focused recheck/handoff extension.` (evidence: `SKIRMISH_MAIN_MENU_TO_SHELL_TRANSITION_CALLER_FRAME_COMPOSITION_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-02 - Does `0x683` directly launch `0x102`? -> No; it writes result code `1`.` (evidence: `MainMenuDialog0xE2_Proc_00531F60`)
- `[RESOLVED] OQ-03 - What consumes result code `1`? -> `Main_Game` case `1` calls `FUN_0060D380(1)`.` (evidence: `Main_Game @ 0x0052D9A0`)
- `[RESOLVED] OQ-04 - What does `FUN_0060D380(1)` do at a high level? -> Creates/pumps an intermediate shell dialog until its local result becomes nonzero.` (evidence: `FUN_0060D380`)
- `[RESOLVED] OQ-05 - What later route reaches offline Skirmish setup? -> Route `0x0B` sets `g_GameMode=5`, then `g_GameMode==5` calls `FUN_006AE2C0`.` (evidence: `Main_Game`)
- `[RESOLVED] OQ-06 - Does `FUN_006AE2C0` call `FUN_006071E0` directly on entry? -> No direct call appears in the decompile.` (evidence: `FUN_006AE2C0`; xref `FUN_006AE2C0` only from `Main_Game`)
- `[RESOLVED] OQ-07 - Does the nonzero shell transition path send `0x4EC`? -> Yes; `FUN_00608260` calls `FUN_006071E0` with `DL=1`.` (evidence: `0x0060833F..0x00608343`)
- `[RESOLVED] OQ-08 - Does common deferred paint send the same message? -> No; `FUN_00622B50` calls `FUN_006071E0` with `DL=0`, which is the `0x4ED` path in prior docs.` (evidence: `0x00622CA6..0x00622CAA`; `FUN_006071E0`)
- `[RESOLVED] OQ-09 - Is stock `ShellButtonSlideSound` audible? -> Stock value is empty in base and YR rules; the call remains real but resolves to no stock audible slide sound unless overridden.` (evidence: `ini/rules.ini:586`, `ini/rulesmd.ini:712`)
- `[RESOLVED] OQ-10 - Does Rust preserve the numeric main-menu return identity? -> Yes; `return_code_for_action(SinglePlayer)` returns `Some(1)`.` (evidence: `src/ui/main_menu_shell/state.rs:56`)
- `[RESOLVED] OQ-11 - Does Rust implement the native intermediate shell route? -> No; it starts a bridge transition directly from `MainMenuShellAction::SinglePlayer`.` (evidence: `src/app.rs:1491`, `src/app_shell_transition.rs:84`)
- `[RESOLVED] OQ-12 - Is the Rust bridge marked as parity? -> No; local comments explicitly mark it bridge/DRIFT.` (evidence: `src/app_shell_transition.rs:1`, `src/render/shell_transition_pass.rs:1`)
- `[RESOLVED] OQ-13 - Is the bridge frame count verified against native route schedule? -> No; it is fixed at `14` frames and is not the native child-schedule calculation.` (evidence: `src/app_shell_transition.rs:15`; prior `FUN_006071E0` schedule report)
- `[DEFERRED] OQ-14 - Which exact intermediate Single Player shell control writes or causes route `0x0B`?` (category: `requires-different-system-context`; reason: current Ghidra pass verifies `0x0B` consumer but did not recover the intermediate dialog resource/control owner; next-step-if-pursued: investigate `FUN_0060D380` dialog resource and command proc to the `0x0B` write)
- `[DEFERRED] OQ-15 - Does the intermediate Skirmish selection control call `0x00612690 -> FUN_00608260`?` (category: `needs-runtime-debugger`; reason: xref exists but current database has no containing function boundary at `0x00612690`; next-step-if-pursued: recover function boundary or live trace click from intermediate shell to `0x0B`)
- `[DEFERRED] OQ-16 - What are the exact pixels for the full route from main-menu click through intermediate shell to `0x102` first paint?` (category: `needs-runtime-debugger`; reason: requires retail framebuffer capture and Rust side-by-side after route exists; next-step-if-pursued: add capture trace for transition frames)
- `[DEFERRED] OQ-17 - Full caller taxonomy for every `FUN_00608260` xref?` (category: `out-of-scope`; reason: this report only needs initial Single Player route and known Choose Map exclusion; next-step-if-pursued: dedicated shell-transition caller taxonomy)
- `[RESOLVED] OQ-18 - Is this TS legacy only? -> No; checked functions are active YR shell/main-menu/skirmish paths.` (evidence: active `Main_Game`, main menu `0xE2`, and `FUN_006AE2C0` paths)

Deferred entries are material enough that this report remains a coverage-map. It is sufficient to block a "direct bridge is parity" claim, and sufficient to define the next implementation target, but not sufficient to implement every pixel of the missing intermediate shell.

## 9. Visual/UI Composition Ledger

This report does not close full visual composition for the missing intermediate shell. It records the verified route/transition visual obligations that affect a future implementation.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | main-menu dialog `0xE2` | active when main menu is shown; button `0x683` writes route code | main-menu shell assets from sibling reports | parent/dialog owned by shell common init | shell palette path from sibling docs | yes | source route UI |
| 2 | `FUN_0060D380(1)` intermediate shell | active after main-menu result `1` | unresolved intermediate shell resource/assets | unresolved | unresolved | yes | missing native layer |
| 3 | optional `FUN_00608260 -> FUN_006071E0(DL=1)` | requires shell record `+0xC1 != 0`, `+0xB4 == 1`, visible HWND | shell transition SHPs including `g_SDTP_SHP`, `g_SDBTNANM_SHP`, radar globals | child/shell rects read during helper | display-surface draw path | conditional | native transition/reveal trigger |
| 4 | optional common `WM_PAINT -> FUN_006071E0(DL=0)` | requires dirty byte consumed by `FUN_00622B50` | same transition helper family | reads shell/window rects | display-surface draw path | conditional | deferred redraw, not reveal start |
| 5 | `FUN_006AE2C0` setup dialog `0x102` | active after `0x0B -> g_GameMode=5` | `0x102` Skirmish shell assets from sibling reports | fullscreen parent and verified child matrix | shell/chrome palette path from sibling docs | yes | final Skirmish setup UI |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| main-menu RA2TS child assets `Ra2ts_s` / `Ra2ts_l` | yes | yes | yes, main menu | no | yes | no | no | no | `FUN_00531CC0` |
| intermediate Single Player shell assets | unresolved | unresolved | expected, but not enumerated | unresolved | unresolved | unresolved | unresolved | no claim | `FUN_0060D380` creates/pumps shell dialog; resource matrix deferred |
| transition SHPs/globals used by `FUN_006071E0` | yes when helper runs | yes when helper runs | conditional | no | yes | yes | yes | no | `FUN_006071E0`; prior transition report |
| `0x102` Skirmish setup assets | yes | yes | yes after route `0x0B` | yes | yes | conditional | no | no | `FUN_006AE2C0`; sibling `0x102` reports |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Main-menu Single Player writes route code `1`, not direct Skirmish setup. | `MainMenuDialog0xE2_Proc_00531F60` | Rust preserves return-code helper but action handler uses it as direct shortcut trigger. | `src/ui/main_menu_shell/state.rs`, `src/app.rs::handle_main_menu_shell_action` | Keep numeric identity, but route through a shell-result dispatcher rather than direct `main_menu_show_native_skirmish_shell`. | Clicking `0x683` records route `1` and enters an intermediate shell state; `0x102` is not active immediately or after a Rust-only bridge. | Do not remove the `Some(1)` identity; it is correct. |
| `Main_Game` case `1` calls `FUN_0060D380(1)`. | `Main_Game @ 0x0052D9A0` | Missing. | new or existing app shell-flow state above main menu/skirmish render | Represent the intermediate shell loop as a first-class app UI state with a dialog result local equivalent. | A trace from main menu click shows route state `1 -> intermediate shell`, with no direct `g_GameMode=5` equivalent until later action. | Do not make the bridge smoother and call that parity. |
| Offline Skirmish setup starts only after route `0x0B` sets `g_GameMode=5`. | `Main_Game` | Missing; Rust sets `main_menu_show_native_skirmish_shell` when bridge completes. | `src/app.rs`, future shell route module | Add a Skirmish-selection action in the intermediate shell that yields native result `0x0B`, then activates the existing `0x102` shell endpoint. | Selecting Skirmish from the intermediate shell sets route `0x0B`, then first `0x102` frame uses verified layout. | Do not let `MainMenuShellAction::SinglePlayer` set the final Skirmish shell flag. |
| `FUN_006AE2C0` pumps until `0x617` Start or `0x5C0` Back and returns true only on Start. | `FUN_006AE2C0` | Existing Skirmish state has its own app-level actions; route integration is partial. | `src/ui/skirmish_shell/state.rs`, `src/app.rs` | Ensure Back returns to the prior shell route, not process exit, when entered through the native route. | Press Back in `0x102` returns to the native shell flow exactly as `FUN_006AE2C0` false return does. | Do not treat Back as global app exit in the native route. |
| Native direct transition helper, when used, is `FUN_00608260 -> FUN_006071E0(DL=1)`, not a crossfade between two full render targets. | `0x0060833F..0x00608343`; `FUN_006071E0` | Rust bridge composites main-menu and Skirmish offscreen targets with fixed 14 frames. | `src/app_shell_transition.rs`, `src/render/shell_transition_pass.rs`, future shell transition model | Replace or quarantine the bridge once the native intermediate route exists; implement native helper only at the verified user action that reaches it. | Transition trace shows child-schedule ticks, `30 ms` sleep/cadence, and final `0x4EC` reveal only on nonzero mode. | Do not globally fire `0x4EC` or static reveal on ordinary first paint or `DL=0`. |
| Common deferred paint transition is `DL=0` and sends `0x4ED`, not reveal-start `0x4EC`. | `0x00622CA6..0x00622CAA`; prior static reveal report | Rust lacks exact event split. | future shell event/reveal state | Preserve separate event identities if implementing transition/reveal. | A common invalidation redraw does not start right-panel static reveal; only nonzero transition does. | Do not collapse `0x4ED` and `0x4EC`. |
| Stock `ShellButtonSlideSound` is empty but the call path is real. | `ini/rules.ini:586`, `ini/rulesmd.ini:712`; prior transition report | Rust bridge has no native audio contract. | audio/shell route integration | If custom rules set `ShellButtonSlideSound`, nonzero transition should attempt the same audio lookup; stock YR remains silent for that key. | With stock INI, no slide sound is heard; with override, the native transition path plays it at the verified site. | Do not use this key for ordinary owner-draw button clicks. |

### Stale Docs / Follow-up Docs

- No contradiction with `SKIRMISH_MAIN_MENU_TO_SHELL_TRANSITION_CALLER_FRAME_COMPOSITION_GHIDRA_REPORT.md` was found. This report confirms its core route claim.
- The previous implementation handoff's "pragmatic smoothness" option is now implemented in Rust as an explicit bridge. It should remain labeled DRIFT and be removed or quarantined once the native intermediate shell route exists.
- The next research target should be narrow: `FUN_0060D380 intermediate Single Player shell resource/control that emits 0x0B`, with specific attention to whether the Skirmish-selection action reaches `0x00612690 -> FUN_00608260`.

## Sources

- Ghidra read-only decompile: `MainMenuDialog0xE2_Proc_00531F60 @ 0x00531F60`, `FUN_00531CC0`, `Main_Game @ 0x0052D9A0`, `FUN_0060D380`, `FUN_006AE2C0`, `FUN_00608260`, `FUN_00608070`, `FUN_00622B50`, `FUN_006071E0`.
- Ghidra xrefs: `FUN_0060D380`, `FUN_006AE2C0`, `FUN_00608260`, `FUN_006071E0`.
- Ghidra assembly contexts: `0x0060833F..0x00608343`, `0x00622CA6..0x00622CAA`.
- Prior docs: `docs/research/skirmish-ui/SKIRMISH_MAIN_MENU_TO_SHELL_TRANSITION_CALLER_FRAME_COMPOSITION_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_FUN_006071E0_SHELL_TRANSITION_REDRAW_PATH_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_STATIC_REVEAL_ANIMATION_0X102_GHIDRA_REPORT.md`, `docs/research/traces/SKIRMISH_STANDARD_ROUTE_LAYOUT_REACHABILITY_800_TRACE.md`.
- INI checked: `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`, `ini/mpmodesmd.ini`.
- Rust read-only scan: `src/ui/main_menu_shell/state.rs`, `src/app.rs`, `src/app_shell_transition.rs`, `src/render/shell_transition_pass.rs`, `src/app_skirmish_shell_render.rs`.
