# Single Player To Skirmish FUN_006071E0 Flags/Assets - Ghidra Research Report

**Date:** 2026-05-27  
**Address(es):** `FUN_006071E0 @ 0x006071E0`, `FUN_00608260 @ 0x00608260`, `FUN_00622B50 @ 0x00622B50`, `FUN_0060C540 @ 0x0060C540`, `FUN_0060CAF0 @ 0x0060CAF0`, `FUN_0060C930 @ 0x0060C930`, `FUN_0060CCC0 @ 0x0060CCC0`, `SinglePlayerDialog0x100_Proc @ 0x0052D640`
**Investigation Mode:** exhaustive-slice for the route-specific flag/assets question  
**Claimed Scope:** standard YR Single Player dialog `0x100` Skirmish control `0x579` route toward offline Skirmish dialog `0x102`; whether that route reaches `FUN_00608260 -> FUN_006071E0`; which `FUN_006071E0` flag bytes and SHP groups are route-active vs generic-helper only.  
**Non-Scope:** full pixel capture of the shell transition, all callers of `FUN_00608260`, campaign/load-game routes, WOL/network dialogs, and complete right-panel first-paint composition outside the transition-helper question.  
**Confidence:** High for route result and flag setter/clearer behavior; High for generic helper gates/assets; Medium for exact final pixels because no runtime framebuffer capture was taken.  
**Active in YR:** Yes for dialog `0x100`, control `0x579`, result `0x0B`, and later `0x102` creation. `FUN_00608260 -> FUN_006071E0` is active in YR shell paths, but **not proven active for the `0x100` Skirmish command route** and is verified not called by the `0x0052D640` command-result proc.

## Working Notes

Target question: For Single Player shell `0x100 -> Skirmish`, determine the `FUN_006071E0/FUN_00608260` record flags and SHP assets/frames that draw, or prove the facts are generic-helper only.

Non-goals: Do not rediscover the whole frame schedule; do not broaden into all shell dialogs; do not modify Rust/INI/assets; do not claim framebuffer-pixel parity.

Evidence needed to mark COMPLETE: decompile plus assembly for the `0x100` command route, `FUN_00608260` gates, `FUN_006071E0` record byte reads, init classifiers for `0x100` and `0x102`, INI sound defaults, current Rust surface scan, and a final route-active/generic-helper split.

Stop conditions: Every seeded route/flag/asset question is resolved or deferred with reason; zero-add pass over `0x0052D640`, `FUN_00608260`, and `FUN_006071E0` adds no new route-active transition question.

## 1. Overview

The native `0x100` Skirmish button does not need the generic shell transition helper to produce the Skirmish route result. The route-active command path is direct: dialog proc `0x0052D640` delegates to common shell proc first, then on `WM_COMMAND` low word `0x579` writes `0x0B` to the dialog result pointer.

`FUN_00608260 -> FUN_006071E0` remains a real YR shell transition helper. However, for this route, the verified helper facts are generic-helper facts unless another caller outside `0x0052D640` is proven to wrap the command. The `0x100` dialog is shell-mode eligible (`+0xB4=1`, `+0xC1=1`), but its optional `FUN_006071E0`/right-panel groups are cleared by init classifiers (`+0xD9=0`, `+0xDA=0`, `+0xDB=0`, `+0xDC=0`), while the later `0x102` Skirmish dialog sets the SDTP/SDMPBTN flags for steady-state first paint.

## 2. Key Offsets And Flags

Offsets below name both the raw shell record byte and, where useful, the decompiler's shifted pointer form. Several helpers do `record + 4` and then read or write `[ptr + N]`.

| Field | Binary access | Meaning in this slice | `0x100` value after common init | `0x102` value after common init | Active in YR |
|---|---|---|---|---|---|
| record `+0xB4` | `piVar1[0x2D] == 1` in `FUN_00608260`; assembly setter `[record+4+0xB0]` | Paint/shell mode required by direct transition wrapper | `1` | `1` | Yes |
| record `+0xC1` | `FUN_00608260` checks byte `+0xC1`; `FUN_0060C540`/`FUN_00608380` set it | Direct transition eligibility byte | `1` | `1` | Yes |
| record `+0xD9` (`[record+4+0xD5]`) | `FUN_006071E0` read at `0x006076DE`; `FUN_0060CAF0` writes `[ECX+0xD5]` | Optional SDWRNTMP / top-highlight family flag, depending consumer | `0` | `1` | Conditional |
| record `+0xDA` (`[record+4+0xD6]`) | `FUN_006071E0` read at `0x00607294`; `FUN_0060C930` writes `[ECX+0xD6]` | Optional SDMPBTN/minimap-button family flag | `0` | `1` | Conditional |
| record `+0xDB` (`[record+4+0xD7]`) | `FUN_006071E0` read at `0x0060727D`; `FUN_0060CCC0` writes `+0xDB` | Radar/open extra group flag | `0` | `0` | Conditional, not this route |
| record `+0xDC` | `FUN_0060CDB0` writes `+0xDC` for `0x108`/`0xBC6` only | Related shell classifier, not consumed by this route proof | `0` | `0` | Conditional, not this route |
| `DL` mode byte | `FUN_006071E0` stores `DL` at `0x006071F5`; `FUN_00608260` sets `DL=1`; common paint sets `DL=0` | Transition direction/completion split | no route-active call proven | no route-active call from launcher | Conditional |

## 3. Core Logic

### 3.1 `0x100` route result is direct

Active in YR: Yes. `Main_Game` case `1` reaches the Single Player shell resource `0x100`; sibling report `SKIRMISH_FUN_0060D380_SINGLE_PLAYER_0X100_TO_0X0B_GHIDRA_REPORT.md` verifies the resource/proc. The command proc itself has no Ghidra function boundary, but raw disassembly is clear:

- `0x0052D656..0x0052D663`: calls common shell proc `FUN_00622B50(hwnd,msg,wParam,lParam)` first and returns early only if it returns nonzero.
- `0x0052D6DF..0x0052D6F7`: masks `wParam & 0xFFFF`, compares against `0x579`.
- `0x0052D712..0x0052D720`: writes `0x0B` to the result pointer and returns.

No call to `0x00608260` appears in the `0x0052D640..0x0052D785` proc range. Evidence: Ghidra assembly context for `0x0052D640`, `0x0052D656`, `0x0052D6DF`, and `0x0052D712`.

### 3.2 Common shell proc does not turn `WM_COMMAND 0x579` into the helper

Active in YR: Yes as negative evidence. `FUN_00622B50` handles many common messages, but its `FUN_006071E0` call site is the `WM_PAINT` deferred dirty path:

- `0x00622CA6`: `XOR DL,DL`
- `0x00622CA8`: `MOV ECX,ESI`
- `0x00622CAA`: `CALL 0x006071E0`
- `0x00622CAF`: clears `+0xBE`

The common proc does not handle `WM_COMMAND 0x111` by calling `FUN_00608260`. For the `0x100` Skirmish command, the common proc returns zero and the dialog proc writes `0x0B`.

### 3.3 Direct transition helper gates are satisfied by `0x100`, but no route caller is proven

Active in YR: Conditional. `FUN_00608260` is the direct helper:

1. Requires non-WOL/normal shell gate `FUN_0069BBE0() == 0`.
2. Looks up the shell dialog record.
3. Requires record `+0xC1 != 0`.
4. Requires record `+0xB4 == 1` (`piVar1[0x2D] == 1`).
5. Requires `IsWindowVisible(hwnd) != 0`.
6. Plays `GUIMoveInSound`, disables the parent, enumerates children, sets `DL=1`, calls `FUN_006071E0`, restores children/enabled state, invalidates, and returns `1`.

Assembly evidence for mode: `0x0060833F MOV DL,0x1`; `0x00608341 MOV ECX,ESI`; `0x00608343 CALL 0x006071E0`.

`FUN_0060C540` includes both dialog ids `0x100` and `0x102`; for allowed ids it writes `record+0xB4=1` and `record+0xC1=1` (assembly with shifted pointer: `0x0060C7B9 MOV [ECX+0xB0],EDX`; `0x0060C7BF MOV [ECX+0xBD],DL`). This means `0x100` is eligible if some caller invokes `FUN_00608260(hwnd)`, but this slot did not find that caller on the `0x579 -> 0x0B` path.

### 3.4 Optional SHP group flags differ between `0x100` and `0x102`

Active in YR: Yes for the init classifiers; conditional for drawing.

Common shell init calls `FUN_0060CAF0`, `FUN_0060C930`, `FUN_0060CCC0`, and `FUN_0060CDB0` during `WM_INITDIALOG` in `FUN_00622B50`.

For dialog `0x100`:

- `FUN_0060CAF0` excludes `0x100`; it writes `record+0xD9=0` (`0x0060CB89..0x0060CB8B`).
- `FUN_0060C930` excludes `0x100`; it writes `record+0xDA=0` (`0x0060C9B6..0x0060C9B8`).
- `FUN_0060CCC0` allows only `0x103`/`0xBC7`; it writes `record+0xDB=0` for `0x100`.
- `FUN_0060CDB0` allows only `0x108`/`0xBC6`; it writes `record+0xDC=0` for `0x100`.

For dialog `0x102`:

- `FUN_0060CAF0` includes `0x102`; it writes `record+0xD9=1` (`0x0060CB5A..0x0060CB93`).
- `FUN_0060C930` includes `0x102`; it writes `record+0xDA=1` (`0x0060C99A..0x0060C9C5`).
- `FUN_0060CCC0` excludes `0x102`; `record+0xDB=0`.
- `FUN_0060CDB0` excludes `0x102`; `record+0xDC=0`.

This is the key asset implication: the source dialog `0x100` has no optional SDMPBTN/SDWRNTMP/radar-open helper groups enabled. The destination dialog `0x102` has the two steady-state right-panel chrome flags enabled, but that does not prove the `0x100` click runs `FUN_006071E0`.

### 3.5 Generic `FUN_006071E0` assets and frames

Active in YR: Conditional; these draw only when the helper is actually called.

Generic helper facts verified or inherited from current reports:

- Cadence: one loop sleep is `Sleep(0x1E)` = 30 ms; loop count is max schedule entry plus `6`.
- Direction: `DL=1` path from `FUN_00608260` uses step `-1`, sends `0x4EC`, and plays `ShellButtonSlideSound` after the loop; common paint `DL=0` sends `0x4ED`.
- Stock `ShellButtonSlideSound=` is empty in both base and YR rules, so stock YR has no audible slide-end cue from that key unless rules are modified.
- SDBTNANM regular child/button cells use six-step frame sequences. Prior verified wording: show direction cycles `10,9,8,7,6,5` then settles to `1`; close direction cycles `5,6,7,8,9,10` then settles to `10`.
- `record+0xDA`/shifted `[+0xD6]` gates the SDMPBTN/minimap-button family in transition and steady-state `0x102` chrome.
- `record+0xD9`/shifted `[+0xD5]` gates the SDWRNTMP/top-highlight family depending on consumer; in steady-state `0x102`, it draws `SDTP.SHP` frame `1` through `Sidebar_TopHighlight`.
- `record+0xDB`/shifted `[+0xD7]` gates a radar/open extra group; `0x100` and `0x102` both clear it in common init.

Asset dimensions from local dump:

| Asset | Size | Frames | Route-active for `0x100` click? | Generic-helper role |
|---|---:|---:|---|---|
| `SDBTNANM.SHP` | 156x42 | 17 | Not proven; only if a helper caller wraps the route | Button/cell transition frames and owner-draw button art |
| `SDMPBTN.SHP` | 156x84 | 7 | No for source `0x100` optional flag; yes later as `0x102` steady-state chrome | Minimap/button panel group; frame `0` visible in `0x102` steady-state |
| `SDWRNTMP.SHP` | 168x177 | 6 | No for source `0x100` optional flag | Optional warning/template transition group |
| `SDTP.SHP` | 168x199 | 2 | No route-active transition proof; `0x102` steady-state draws frame `1` | Right-panel top cap/top-highlight |
| `SDBTNBKGD.SHP` | 168x42 | 1 | Not transition-route proven | Repeated right-panel chrome tile |
| `SDBTM.SHP` | 168x65 | 1 | Not transition-route proven | Right-panel bottom cap |

## 4. INI Keys

| INI key | File / default | Effect | Active in YR |
|---|---|---|---|
| `[AudioVisual] ShellButtonSlideSound` | `ini/rules.ini:586`, `ini/rulesmd.ini:712`, empty | Read at `0x00607F59` after `DL!=0` helper completion | Conditional; stock silent |
| `[AudioVisual] GUIMainButtonSound` | `ini/rules.ini:489`, `ini/rulesmd.ini:643`, `MenuClick` | Ordinary main/shell button click family, not this slide-end key | Yes |
| `[AudioVisual] GenericClick` | `ini/rules.ini:577`, `ini/rulesmd.ini:703`, `MenuClick` | Generic owner-draw click family | Yes |

No INI key makes `0x0052D640` call `FUN_00608260`.

## 5. Integration Points

| Boundary | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| `Main_Game` case `1` | Enters `FUN_0060D380(1)` for Single Player shell `0x100` | `Main_Game` decompile; sibling route report | Yes |
| dialog `0x100` command `0x579` | Writes result `0x0B` directly | `0x0052D6F1..0x0052D720` | Yes |
| common shell `WM_PAINT` dirty path | Calls `FUN_006071E0` with `DL=0` only when `+0xBE` is set | `0x00622CA6..0x00622CAF` | Conditional |
| direct helper `FUN_00608260` | Calls `FUN_006071E0` with `DL=1` after gates | `0x00608260`; `0x0060833F..0x00608343` | Conditional, not route-proven |
| `0x102` startup | Later `0x0B -> g_GameMode=5 -> FUN_006AE2C0` creates Skirmish setup | `Main_Game`; `FUN_006AE2C0`; sibling route reports | Yes |

## 6. Current Rust Implementation Status

Current Rust now has a `0x100` Single Player shell surface and action identity:

- `src/ui/single_player_shell/state.rs:8` defines controls `0x688`, `0x689`, `0x579`, `0x686`.
- `src/ui/single_player_shell/state.rs:41` returns native route codes, including `Skirmish -> 0x0B`.
- `src/app.rs:536` opens the Single Player shell after main-menu Single Player.
- `src/app.rs:556` currently enters native Skirmish shell immediately from the Single Player Skirmish action.

Rust still does not model the native shell transition helper for this route. The existing `src/app_shell_transition.rs` bridge is explicitly DRIFT and still named main-menu-to-Skirmish; it is not a native `FUN_006071E0` implementation and should not be used as parity evidence.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x100` Skirmish command result | verified | `0x0052D6F1..0x0052D720` | none |
| absence of `FUN_00608260` inside `0x0052D640` proc | verified | raw disassembly `0x0052D640..0x0052D785` | external wrapper caller taxonomy remains out of scope |
| common shell `WM_COMMAND` behavior | verified for no helper call | `FUN_00622B50` decompile; helper call only in `WM_PAINT` dirty branch | none for this slice |
| `FUN_00608260` gates and `DL=1` call | verified | decompile and assembly `0x0060833F..0x00608343` | exact owner of unrelated xrefs not covered |
| `FUN_0060C540` `0x100`/`0x102` shell eligibility | verified | decompile; assembly `0x0060C6DC..0x0060C7BF` | none |
| `FUN_0060CAF0`/`FUN_0060C930` optional flag split | verified | decompile; assembly `0x0060C99A..0x0060C9C5`, `0x0060CB5A..0x0060CB98` | none |
| `FUN_0060CCC0`/`FUN_0060CDB0` radar/extra flags | verified | decompile | none |
| `FUN_006071E0` frame schedule | verified by prior docs, spot-checked | `0x006071E0`, `FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE...` | framebuffer capture deferred |
| stock slide sound | verified | `ini/rules.ini:586`, `ini/rulesmd.ini:712`, `0x00607F59` | none |
| current Rust surfaces | verified | `src/ui/single_player_shell/state.rs`, `src/app.rs`, `src/app_shell_transition.rs` | implementation out of scope |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does `0x100` Skirmish button write `0x0B`? -> Yes, `0x579` writes `0x0B` directly.` (evidence: `0x0052D6F1..0x0052D720`)
- `[RESOLVED] OQ-02 - Does `0x0052D640` call `FUN_00608260`? -> No call appears in the proc range; it delegates to common shell first, then writes results itself.` (evidence: `0x0052D640..0x0052D785`)
- `[RESOLVED] OQ-03 - Does common shell proc call `FUN_006071E0` for `WM_COMMAND 0x579`? -> No; its helper call is the `WM_PAINT` dirty-byte branch with `DL=0`.` (evidence: `FUN_00622B50`, `0x00622CA6..0x00622CAF`)
- `[RESOLVED] OQ-04 - Is `0x100` eligible for `FUN_00608260` if some caller invokes it? -> Yes, `FUN_0060C540` allows `0x100` and writes `+0xB4=1`, `+0xC1=1`.` (evidence: `0x0060C6DC..0x0060C7BF`)
- `[RESOLVED] OQ-05 - Does `0x100` enable SDMPBTN/SDWRNTMP optional groups? -> No, `FUN_0060CAF0` and `FUN_0060C930` exclude `0x100` and write zero.` (evidence: `0x0060C9B6..0x0060C9B8`, `0x0060CB89..0x0060CB8B`)
- `[RESOLVED] OQ-06 - Does destination `0x102` enable those two flags? -> Yes, `FUN_0060CAF0` and `FUN_0060C930` include `0x102`.` (evidence: `0x0060C99A..0x0060C9C5`, `0x0060CB5A..0x0060CB98`)
- `[RESOLVED] OQ-07 - Does `0x100` or `0x102` enable the radar/open extra byte `+0xDB`? -> No; `FUN_0060CCC0` allows only `0x103` and `0xBC7`.` (evidence: `FUN_0060CCC0`)
- `[RESOLVED] OQ-08 - Is stock `ShellButtonSlideSound` audible? -> Stock key is empty; helper call is real but stock rules provide no sound index.` (evidence: `ini/rules.ini:586`, `ini/rulesmd.ini:712`, `0x00607F59`)
- `[DEFERRED] OQ-09 - Could an external owner-draw/state-machine caller wrap the `0x100` command with `FUN_00608260` before the result write?` (category: `needs-runtime-debugger`; reason: static proc proof excludes direct call, but exhaustive owner-draw subclass timing for every click message is outside this slot; next-step-if-pursued: live trace `FUN_00608260` while clicking `0x579` in retail)
- `[DEFERRED] OQ-10 - Exact framebuffer pixels for a native `0x100 -> 0x102` transition if a wrapper is later proven?` (category: `needs-runtime-debugger`; reason: requires retail capture or forced helper-state instrumentation; next-step-if-pursued: capture frame-by-frame after route-active caller proof)

Deferred items do not change the main finding: the route result is active and direct; the SHP transition helper is generic-helper only for this slot unless a separate click-wrapper trace proves otherwise.

## 9. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `0x100` common init | `FUN_0060C540` allows `0x100` | right-panel/common shell assets | common shell layout | shell palette path | yes | source shell eligibility |
| 2 | `FUN_0060CAF0`/`FUN_0060C930` for `0x100` | excludes `0x100`, writes `+0xD9=0`, `+0xDA=0` | no SDMPBTN/SDWRNTMP optional group from source flags | n/a | n/a | yes negative | clears optional transition/chrome groups |
| 3 | `0x0052D640 WM_COMMAND` | low word `0x579` | no helper SHP draw | n/a | n/a | yes | route result write `0x0B` |
| 4 | `FUN_00608260 -> FUN_006071E0` | requires external caller plus `+0xB4/+0xC1`, visible window | `SDBTNANM`, `SDMPBTN`, `SDWRNTMP`, `SDTP` groups by flags | right-panel/global rects | shell convert path | not proven for `0x579` route | generic transition helper |
| 5 | later `0x102` init/paint | `0x102` sets `+0xD9/+0xDA` | `SDTP.SHP` frame `1`, `SDMPBTN.SHP` frame `0` in steady-state chrome | `DAT_00B0FC20`, `DAT_00B0FC14` | shell convert path | yes after route reaches `0x102` | destination right-panel chrome |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| `SDBTNANM.SHP` | yes | generic helper/owner-draw conditional | not route-proven for `0x579` transition | no | button/chrome | yes | conditional | no | `0x006071E0`; asset survey |
| `SDMPBTN.SHP` | yes | no from source `0x100` optional flag; yes in later `0x102` steady-state | yes after `0x102` first paint, not as source transition | no | yes | no | conditional | no | `FUN_0060C930`, `SKIRMISH_0X102_TOP_PREVIEW...` |
| `SDWRNTMP.SHP` | yes | no from source `0x100` optional flag | no route-active draw proven | no | no | conditional | conditional | source route inactive | `FUN_0060CAF0`, `0x006071E0` |
| `SDTP.SHP` | yes | `0x102` steady-state frame `1`; generic helper conditional | yes after `0x102` first paint | no | yes | top-highlight | conditional | no | `FUN_0060CAF0`; `Sidebar_TopHighlight` report |
| `SDBTNBKGD.SHP` | yes | common right-panel chrome | yes for shell pages | no | yes | no | no | no | asset survey; shell chrome docs |
| `SDBTM.SHP` | yes | common right-panel chrome | yes for shell pages | no | yes | no | no | no | asset survey; shell chrome docs |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x579` writes route result `0x0B` directly; `FUN_00608260` is not called by `0x0052D640`. | `0x0052D6F1..0x0052D720`; no call in `0x0052D640..0x0052D785` | Rust preserves `0x0B` but snaps to `0x102` immediately. | `src/ui/single_player_shell/state.rs`, `src/app.rs::handle_single_player_shell_action`, future route dispatcher | Keep `0x0B` as the route boundary; do not require helper playback to produce the result. | Click Single Player shell Skirmish: route trace records `0x579 -> 0x0B -> g_GameMode=5 equivalent -> 0x102`. Proposed test: `single_player_skirmish_button_emits_route_0x0b_without_transition_helper`. | Do not block route correctness on `FUN_00608260` parity. |
| `0x100` sets `+0xB4/+0xC1` but clears optional `+0xD9/+0xDA/+0xDB/+0xDC`; destination `0x102` sets `+0xD9/+0xDA`. | `FUN_0060C540`, `FUN_0060CAF0`, `FUN_0060C930`, `FUN_0060CCC0`, `FUN_0060CDB0` | Rust has no shell-record flag model for these bytes. | future shell dialog metadata/state; `src/ui/single_player_shell`, `src/ui/skirmish_shell` | Model optional transition/chrome groups per dialog id, not globally. | Initial `0x100` shell has no SDMPBTN/SDWRNTMP optional transition groups; after entering `0x102`, SDTP frame 1 and SDMPBTN frame 0 chrome are enabled. Proposed test: `single_player_and_skirmish_shell_record_flags_match_native_optional_groups`. | Do not copy `0x102` chrome flags back onto `0x100`. |
| Generic helper uses 30 ms cadence and SDBTNANM frame-wave only when an actual helper caller is active. | `0x006071E0`, `0x0060833F..0x00608343`, prior schedule report | Existing bridge is whole-screen compositor with fixed 14 frames, labeled DRIFT. | `src/app_shell_transition.rs`, `src/render/shell_transition_pass.rs` | Keep bridge quarantined or replace it only after route-active helper proof; if implemented, drive frames from child schedule and record flags. | Forced helper test with a synthetic eligible dialog advances SDBTNANM frames on 30 ms ticks and does not draw disabled optional groups for `0x100`. Proposed test: `forced_shell_transition_respects_dialog_optional_group_flags`. | Do not crossfade source/destination render targets and claim native `FUN_006071E0` parity. |

## Negative Facts / Do Not Do

- Do not claim the standard `0x100` Skirmish click natively plays `FUN_00608260 -> FUN_006071E0`; the proc `0x0052D640` writes `0x0B` directly and contains no helper call. Evidence: `0x0052D640..0x0052D785`.
- Do not enable SDMPBTN/SDWRNTMP transition groups for the source Single Player shell. Evidence: `FUN_0060CAF0` and `FUN_0060C930` explicitly write zero for dialog id `0x100`.
- Do not treat `0x102` steady-state SDTP/SDMPBTN chrome as proof that the `0x100` transition drew those assets. Evidence: `0x102` sets `+0xD9/+0xDA`; `0x100` clears them.
- Do not use stock `ShellButtonSlideSound` as an audible cue for this route. Evidence: stock `ShellButtonSlideSound=` is empty in both shipped rules files.
- Do not collapse `0x4EC` and `0x4ED`. Evidence: `FUN_00608260` passes `DL=1`; common paint passes `DL=0`, and prior reports prove different completion messages.

## Stale Docs / Follow-up Docs

- Replace older wording that says "every main-menu/shell button click triggers `FUN_00608260 -> FUN_006071E0`" with: "`FUN_00608260 -> FUN_006071E0` is a live generic shell transition helper, but dialog `0x100` Skirmish control `0x579` writes result `0x0B` directly in proc `0x0052D640`; no route-active helper call is proven for that command."
- Replace implementation-handoff wording that says Rust lacks any Single Player shell module with: "Current Rust has `src/ui/single_player_shell` with control ids and return codes, but `SinglePlayerShellAction::Skirmish` still enters `0x102` immediately and does not model native shell-record flags or `FUN_006071E0`."

## Sources

- Ghidra read-only decompile/assembly: `FUN_006071E0 @ 0x006071E0`, `FUN_00608260 @ 0x00608260`, `FUN_00622B50 @ 0x00622B50`, `FUN_0060C540 @ 0x0060C540`, `FUN_0060CAF0 @ 0x0060CAF0`, `FUN_0060C930 @ 0x0060C930`, `FUN_0060CCC0 @ 0x0060CCC0`, `FUN_0060CDB0 @ 0x0060CDB0`, raw proc bytes `0x0052D640..0x0052D785`, `Main_Game @ 0x0052D9A0`.
- Prior docs: `FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md`, `SHELL_DIALOG_SLIDE_TRANSITION_TRANSFORM_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_FUN_006071E0_SHELL_TRANSITION_REDRAW_PATH_GHIDRA_REPORT.md`, `MAIN_MENU_SHELL_TRANSITION_ASSET_SURVEY_2026_05_27.md`, `skirmish-ui/SKIRMISH_MAIN_MENU_TO_SHELL_TRANSITION_CALLER_FRAME_COMPOSITION_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_FUN_0060D380_SINGLE_PLAYER_0X100_TO_0X0B_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_0X102_TOP_PREVIEW_CHROME_SDTP_SDMPBTN_GHIDRA_REPORT.md`.
- INI checked: `ini/rules.ini`, `ini/rulesmd.ini`.
- Rust read-only scan: `src/ui/single_player_shell/state.rs`, `src/app.rs`, `src/app_shell_transition.rs`.

## Status

COMPLETE for the scoped route-specific flag/assets question. Remaining uncertainty is limited to a possible external click-wrapper trace around `0x00608260`; static route/result evidence says the helper facts are generic-helper only for this slot, not verified route-active behavior for `0x579 -> 0x0B`.
