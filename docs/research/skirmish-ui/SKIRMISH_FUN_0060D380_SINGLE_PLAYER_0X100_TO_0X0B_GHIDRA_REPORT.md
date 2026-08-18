# Skirmish FUN_0060D380 Single Player 0x100 To 0x0B - Ghidra Research Report

**Date:** 2026-05-27
**Address(es):** `Main_Game @ 0x0052D9A0`, `FUN_0060D380 @ 0x0060D380`, `FUN_00622650 @ 0x00622650`, `SinglePlayerDialog0x100_Proc @ 0x0052D640` (raw PE disassembly; no Ghidra function boundary), `FUN_00559C20`
**Investigation Mode:** coverage-map. The route result/control slice is resolved; full final pixel composition and every transition frame are not claimed.
**Claimed Scope:** standard YR main-menu Single Player result `1`, the intermediate dialog resource/proc used by `FUN_0060D380(1)`, the resource controls in dialog `0x100`, and the exact control command that writes route result `0x0B`.
**Non-Scope:** campaign shell internals after result `8`, load-game shell internals after result `9`, unresolved result `0x0A` target, final screenshot RGB capture, and full taxonomy of unrelated `FUN_00608260` callers.
**Confidence:** High for dialog id/proc, control ids, result writes, and `0x579 -> 0x0B`. Medium for final resized pixel rects because this pass extracted RT_DIALOGEX resource DLU rects but did not exhaustively re-run the common resize helper for dialog `0x100`.
**Active in YR:** Yes. `Main_Game` case `1` reaches this path from the standard main menu Single Player button in Yuri's Revenge.

## 1. Overview

The missing intermediate shell is dialog resource `0x100`, not an unknown Skirmish shortcut. `Main_Game` handles main-menu result `1` by passing `ECX=0x100`, `EDX=0x0052D640`, and stack argument `1` into `FUN_0060D380`.

Dialog `0x100` is the Single Player menu. Its `WM_COMMAND` handler writes route result `0x0B` when command/control id `0x579` (`GUI:Skirmish`) is activated. That later route value is what `Main_Game` consumes to set `g_GameMode = 5` and enter `FUN_006AE2C0`, the standard offline Skirmish setup launcher.

## 2. Class Layout / Key Offsets

This slice is Win32 dialog/result-pointer logic rather than a recovered C++ class.

| Field / value | Location | Meaning | Evidence |
|---|---:|---|---|
| dialog id | `ECX=0x100` before `CALL 0x0060D380` | Single Player intermediate shell resource. | `Main_Game` assembly `0x0052DD39..0x0052DD4B` |
| dialog proc | `EDX=0x0052D640` before `CALL 0x0060D380` | Dialog proc for resource `0x100`. | `Main_Game` assembly `0x0052DD39..0x0052DD4B` |
| show/setup flag | stack push `1` before `CALL 0x0060D380` | Causes `FUN_0060D380` to call `FUN_0052B9B0(hwnd)` for RA2TS child setup. | `0x0052DD39`; `FUN_0060D380` |
| dialog result pointer | `SetWindowLongA(hwnd, 8, &local_4)` | Proc writes route result through this pointer. | `FUN_0060D380`; `0x0052D66B..0x0052D672` |
| idle result | `local_4 = 0` | `FUN_0060D380` pumps while zero. | `FUN_0060D380` |
| Skirmish command | control id `0x579` | Writes route result `0x0B`. | raw PE disassembly `0x0052D6F1..0x0052D720` |

## 3. Core Logic

### 3.1 Main_Game selects dialog `0x100`

In the `Main_Game` switch branch for main-menu result `1`, assembly is:

| Address | Instruction | Meaning |
|---:|---|---|
| `0x0052DD39` | `PUSH 0x1` | Pass true setup flag to `FUN_0060D380`. |
| `0x0052DD3B` | `MOV EDX,0x52D640` | Dialog proc is `0x0052D640`. |
| `0x0052DD40` | `MOV ECX,0x100` | Dialog resource id is `0x100`. |
| `0x0052DD45` | `MOV [0x00AC10C8], EBX` | Clears `DAT_00AC10C8`. |
| `0x0052DD4B` | `CALL 0x0060D380` | Enters intermediate shell loop. |

This resolves the previous blocker: the intermediate Single Player shell resource and proc are known.

### 3.2 `FUN_00622650` creates the selected resource

`FUN_00622650` uses `ECX & 0xFFFF` as the dialog resource id and `EDX` as the dialog proc.

Key details:

- It calls `FindResourceA(..., resource_id, RT_DIALOG=5)` via `FUN_004A3B40`.
- It increments dialog stack count `DAT_00B72F50` before `CreateDialogIndirectParamA`.
- It passes a local init parameter whose first word is the dialog id.
- On create failure, it decrements `DAT_00B72F50` and returns null.
- On success, it stores the HWND and resource id into the shell dialog stack arrays and updates `DAT_00B72F44` / `DAT_00B72F48` to the current HWND/resource.

### 3.3 `FUN_0060D380` owns the modal-ish loop

`FUN_0060D380` behavior for this route:

1. Calls `FUN_00622650` with the caller-provided `ECX=0x100`, `EDX=0x0052D640`, and stack flag `1`.
2. Initializes `local_4 = 0`.
3. If HWND creation succeeds, stores `&local_4` at `SetWindowLongA(hwnd, 8, ...)`.
4. Shows and foregrounds the dialog.
5. Calls `FUN_0054F720`.
6. Because the argument is nonzero, calls `FUN_0052B9B0(hwnd)`.
7. Pumps until `local_4 != 0`, then destroys the dialog via `FUN_00622720` and returns `local_4`.

Loop split:

- Always calls `Process_NetworkMessages`.
- If `g_GameMode == 0`, `g_GameMode == 5`, `DAT_00A8D60E != 0`, or `DAT_00A8DAB4 != 0`, it calls `Network_ServiceLoop`.
- Otherwise it calls `FUN_0055CBF0`, then `Main_Tick`; if `Main_Tick` returns true, the loop exits toward destruction.

### 3.4 Dialog `0x100` resource controls

Retail `gamemd.exe` RT_DIALOGEX resource `0x100`:

| Index | ID | Class | DLU rect | Style | Title / CSF key | Role |
|---:|---:|---|---:|---:|---|---|
| 0 | `0x694` | Static | `(425,1,108,10)` | `0x50020001` | `GUI:SinglePlayerMenu` | right-panel title |
| 1 | `0x688` | Button | `(425,122,108,23)` | `0x5000000B` | `GUI:NewCampaign` | command writes `8` |
| 2 | `0x689` | Button | `(425,149,108,23)` | `0x5000000B` | `GUI:LoadSavedGame` | command writes `9`; enabled by save scan |
| 3 | `0x579` | Button | `(425,176,108,23)` | `0x5000000B` | `GUI:Skirmish` | command writes `0x0B` |
| 4 | `0x686` | Button | `(425,346,108,23)` | `0x5000000B` | `GUI:MainMenu` | command writes `0x12` |
| 5 | `0x695` | Static | `(2,355,303,12)` | `0x50000200` | `GUI:Blank` | bottom-left status/help |
| 6 | `0x71C` | Static | `(446,29,61,33)` | `0x50000007` | none | side/right-panel image static |
| 7 | `0x71A` | Static | `(0,0,304,266)` | `0x50000000` | none | RA2TS child/static area |

Dialog template:

- DIALOGEX
- style `0x40000040`
- rect `(0,0,533,369)` dialog units
- font `MS Sans Serif`, point size `8`, charset `1`

No child control `0x68A` exists in resource `0x100`, even though the proc accepts command id `0x68A` and writes result `0x0A`.

### 3.5 Dialog proc `0x0052D640`

Ghidra lacks a function boundary for `0x0052D640`, so this pass used raw PE disassembly from `gamemd.exe`. The proc is a standard 4-argument dialog proc:

1. Reads `hwnd`, `msg`, `wParam`, and `lParam` from the stack.
2. Calls common shell proc `FUN_00622B50(hwnd, msg, wParam, lParam)` first.
3. If the common proc returns nonzero, returns that value.
4. Calls `GetWindowLongA(hwnd, 8)` to recover the result pointer.

Handled messages:

| Message | Behavior | Evidence |
|---:|---|---|
| `0x0F` (`WM_PAINT`) | Sends `0x4F0` to child `0x71A`. | `0x0052D761..0x0052D777` |
| `0x111` (`WM_COMMAND`) | Masks `LOWORD(wParam)` and writes route result for known controls. | `0x0052D6DF..0x0052D75E` |
| `0x497` | Enables/disables Load Saved Game button `0x689` based on a load-save scan. | `0x0052D683..0x0052D6D4` |

Command result table:

| Command id | Resource title | Result written | Downstream route |
|---:|---|---:|---|
| `0x688` | `GUI:NewCampaign` | `8` | campaign/new game route, outside this report |
| `0x689` | `GUI:LoadSavedGame` | `9` | load game route, outside this report |
| `0x579` | `GUI:Skirmish` | `0x0B` | offline Skirmish setup route |
| `0x686` | `GUI:MainMenu` | `0x12` | back to main menu loop |
| `0x68A` | no resource child in `0x100` | `0x0A` | accepted by proc, visible trigger unresolved/absent in template |

### 3.6 Load Saved Game enable gate

On message `0x497`, the proc:

1. Gets child `0x689`.
2. Constructs a `LoadOptionsClass` stack object.
3. Calls `FUN_00559C20`.
4. Destroys the `LoadOptionsClass` stack object.
5. Calls `EnableWindow(load_button, result != 0)`.

`FUN_00559C20` scans the configured save path using `FindFirstFileA`, skips entries with attributes masked by `0x116`, excludes `SAVEGAME.NET`, calls the load-options validation vfunc, and returns `1` on the first valid non-network save. Result: Load Saved Game is disabled when no valid save exists.

### 3.7 `0x00612690 -> FUN_00608260` is not the `0x579 -> 0x0B` result write

The previous route report deferred whether the intermediate Skirmish-selection control reaches `0x00612690 -> FUN_00608260`. This pass resolves the core result-write question:

- `0x579 -> 0x0B` happens directly inside dialog proc `0x0052D640`.
- The disassembled `0x0052D640` proc does not call `FUN_00608260`.
- The known `FUN_00608260` xrefs remain `0x005E6B49` and `0x00612690`, neither of which is inside the `0x0052D640` proc range.

This does not prove no visual transition can occur elsewhere before/around the command. It proves `FUN_00608260` is not needed to produce the route result `0x0B`.

## 4. INI Keys

| INI key | File / default | Effect in this slice | Status |
|---|---|---|---|
| `[AudioVisual] ShellButtonSlideSound` | `ini/rules.ini:586`, `ini/rulesmd.ini:712`, empty | Not used by dialog proc `0x0052D640` for `0x579 -> 0x0B`; only relevant to separate nonzero shell transition helper. | verified negative for this route result |
| `[AudioVisual] GUIMainButtonSound` | `ini/rules.ini:489`, `ini/rulesmd.ini:643`, `MenuClick` | Likely participates in owner-draw button click sound through shared button callback, but this report did not re-open the owner-draw click sound path. | touched-not-exhausted |

No INI key changes which resource/control writes result `0x0B`.

## 5. Integration Points

| Boundary | Verified behavior | Evidence |
|---|---|---|
| main menu result `1` to intermediate shell | `Main_Game` passes `ECX=0x100`, `EDX=0x0052D640`, arg `1` to `FUN_0060D380`. | `0x0052DD39..0x0052DD4B` |
| resource loading | `FUN_00622650` loads RT_DIALOG resource `0x100` and creates with proc `0x0052D640`. | `FUN_00622650` |
| result pointer | `FUN_0060D380` stores `&local_4` at window long offset `8`; proc writes through it. | `FUN_0060D380`; `0x0052D66B..0x0052D672` |
| Skirmish command | `WM_COMMAND` low word `0x579` writes `0x0B`. | `0x0052D6F1..0x0052D720` |
| downstream Skirmish launcher | `Main_Game` route `0x0B` sets `g_GameMode=5`, then `FUN_006AE2C0` creates `0x102`. | previous route recheck |
| common shell message handling | dialog proc delegates to `FUN_00622B50` first. | `0x0052D656..0x0052D663` |
| Load Saved Game enable | message `0x497` enables child `0x689` only if `FUN_00559C20` finds a valid save. | `0x0052D683..0x0052D6D4`; `FUN_00559C20` |

## 6. Current Rust Implementation Status

Current Rust has only the initial main-menu shell and Skirmish setup shell surfaces; it has no implemented dialog `0x100` Single Player shell:

| Rust surface | Current behavior | Status |
|---|---|---|
| `src/ui/main_menu_shell/state.rs` | Has `SinglePlayer0x683 -> MainMenuShellAction::SinglePlayer` and return code `1`. | matches initial identity |
| `src/app.rs` | `MainMenuShellAction::SinglePlayer` starts the temporary bridge to Skirmish. | DRIFT vs `0x100` intermediate shell |
| `src/app_shell_transition.rs` | Bridge directly previews/completes into Skirmish shell. | DRIFT/non-native |
| `src/ui/` | Contains `main_menu_shell` and `skirmish_shell`; no `single_player_shell` module. | missing |
| `rg` for `SinglePlayerMenu`, `NewCampaign`, `LoadSavedGame`, `0x579`, `0x688`, `0x689` in `src/` | No implemented intermediate shell controls found outside main-menu identity and unrelated numeric constants. | missing |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `Main_Game` callsite for main-menu result `1` | verified | `0x0052DD39..0x0052DD4B` | none |
| `FUN_0060D380` loop mechanics | verified | Ghidra decompile + assembly | none for this slice |
| `FUN_00622650` resource/proc creation | verified | Ghidra decompile + assembly | none |
| RT_DIALOGEX `0x100` resource controls | verified | direct PE resource extraction | final USER32/resized pixel rects not fully exhausted |
| dialog proc `0x0052D640` command result writes | verified | raw PE disassembly | no Ghidra function boundary; report cites addresses |
| `0x579 -> 0x0B` source | verified | `0x0052D6F1..0x0052D720`; resource `0x100` child id `0x579` | none |
| `0x688 -> 8`, `0x689 -> 9`, `0x686 -> 0x12` | verified | `0x0052D6DF..0x0052D75E`; resource `0x100` | downstream routes out of scope |
| `0x68A -> 0x0A` | touched-not-exhausted | proc branch `0x0052D734..0x0052D74D`; no resource child in `0x100` | identify dynamic/legacy trigger if needed |
| Load Saved Game enable gate | verified at high level | `0x0052D683..0x0052D6D4`; `FUN_00559C20` | exact save-path string expansion not needed for Skirmish route |
| `0x00612690 -> FUN_00608260` as source of `0x0B` | verified negative for this proc | `0x0052D640..0x0052D785` contains no call to `0x00608260` | full transition caller taxonomy still out of scope |
| current Rust intermediate shell | verified missing | Codegraph + `rg` over `src/` | implement after contract update/plan |
| full route framebuffer pixel parity | deferred | no retail capture in this pass | trace after implementation |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which resource/proc does `FUN_0060D380(1)` use after main-menu result `1`? -> Resource `0x100`, proc `0x0052D640`, stack argument `1`.` (evidence: `0x0052DD39..0x0052DD4B`)
- `[RESOLVED] OQ-02 - What is dialog `0x100`? -> Single Player menu with title `GUI:SinglePlayerMenu` and buttons New Campaign, Load Saved Game, Skirmish, Main Menu.` (evidence: RT_DIALOGEX `0x100` resource extraction)
- `[RESOLVED] OQ-03 - Which control emits route `0x0B`? -> `WM_COMMAND` low word `0x579`, resource title `GUI:Skirmish`, writes `0x0B` to the result pointer.` (evidence: `0x0052D6F1..0x0052D720`)
- `[RESOLVED] OQ-04 - Does dialog proc `0x0052D640` call `FUN_00608260` to write `0x0B`? -> No; result write is direct and the proc contains no call to `0x00608260`.` (evidence: raw PE disassembly `0x0052D640..0x0052D785`)
- `[RESOLVED] OQ-05 - What does `0x686` do in dialog `0x100`? -> Writes `0x12`, returning to the main menu loop.` (evidence: `0x0052D6F9..0x0052D70F`; RT_DIALOGEX `0x100`)
- `[RESOLVED] OQ-06 - What does `0x688` do? -> Writes `8` for New Campaign route.` (evidence: `0x0052D6E7..0x0052D731`; RT_DIALOGEX `0x100`)
- `[RESOLVED] OQ-07 - What does `0x689` do? -> Writes `9` for Load Saved Game route, and its enabled state is controlled by message `0x497`.` (evidence: `0x0052D734..0x0052D75E`; `0x0052D683..0x0052D6D4`)
- `[RESOLVED] OQ-08 - Is `0x68A` present in the resource? -> No; proc accepts it and writes `0x0A`, but RT_DIALOGEX `0x100` has no child `0x68A`.` (evidence: proc branch `0x0052D73C..0x0052D74D`; resource extraction)
- `[RESOLVED] OQ-09 - How is Load Saved Game disabled when no saves exist? -> Message `0x497` scans valid saves through `FUN_00559C20` and calls `EnableWindow(0x689, scan_result)`.` (evidence: `0x0052D683..0x0052D6D4`; `FUN_00559C20`)
- `[RESOLVED] OQ-10 - Does Rust implement dialog `0x100`? -> No; there is no `single_player_shell` UI module and no `SinglePlayerMenu`/`0x579` implementation in `src/`.` (evidence: Codegraph file scan; `rg` over `src/`)
- `[DEFERRED] OQ-11 - Exact final resized pixel rects for every dialog `0x100` child.` (category: `bounded-cost-too-high`; reason: this pass extracted the dialog resource and route source but did not exhaustively trace `ResizeShellChildControl` for `0x100`; next-step-if-pursued: run a layout-focused trace/contract for dialog `0x100`)
- `[DEFERRED] OQ-12 - Exact final route framebuffer pixels through first `0x102` paint.` (category: `needs-runtime-debugger`; reason: requires retail/Rust capture after implementation; next-step-if-pursued: trace-action capture from main menu click to `0x102`)
- `[DEFERRED] OQ-13 - What is the user-visible/dynamic source of proc-only command `0x68A`?` (category: `out-of-scope`; reason: not needed for Skirmish `0x579 -> 0x0B`; next-step-if-pursued: separate Single Player menu full-route taxonomy)

## 9. Visual/UI Composition Ledger

This is a route/control report, not a full pixel-composition report. The table records only verified active composition owners and known assets/controls for dialog `0x100`.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 1 | `FUN_00622650(0x100, 0x0052D640, 1)` | active after main-menu result `1` | RT_DIALOGEX `0x100` | dialog DLU `(0,0,533,369)` | Win32 dialog resource path | yes | creates intermediate shell |
| 2 | common shell init/owner draw | proc delegates to `FUN_00622B50`; common init subclasses children | shared shell assets from existing shell docs | final child rects through common resize, not exhausted here | shell palette path | yes | common shell chrome/control setup |
| 3 | title static `0x694` | resource child exists | text `GUI:SinglePlayerMenu` | DLU `(425,1,108,10)` | common owner-draw static | yes | right-panel title |
| 4 | buttons `0x688`, `0x689`, `0x579`, `0x686` | resource children exist; proc handles WM_COMMAND | `GUI:NewCampaign`, `GUI:LoadSavedGame`, `GUI:Skirmish`, `GUI:MainMenu` | DLU rows y `122`, `149`, `176`, `346` | common owner-draw button path | yes | route commands |
| 5 | status static `0x695` | resource child exists | `GUI:Blank` | DLU `(2,355,303,12)` | common owner-draw/status path | yes | bottom-left help/status |
| 6 | RA2TS child `0x71A` | resource child exists; setup flag calls `FUN_0052B9B0`; WM_PAINT sends `0x4F0` | `Ra2ts_s`/`Ra2ts_l` path from sibling docs | DLU `(0,0,304,266)`, then positioned by `FUN_0052B9B0` | RA2TS child message path | yes | left/main shell art area |

Asset role matrix:

| Asset / control | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| RT_DIALOGEX `0x100` | yes | yes | yes | no | container | no | no | no | PE resource extraction; `0x0052DD40` |
| `GUI:SinglePlayerMenu` static `0x694` | yes | yes | yes | no | title/chrome | no | no | no | RT_DIALOGEX `0x100` |
| `GUI:Skirmish` button `0x579` | yes | yes | yes | no | command control | no | no | no | RT_DIALOGEX `0x100`; `0x0052D6F1..0x0052D720` |
| `GUI:LoadSavedGame` button `0x689` | yes | conditional enabled | yes when dialog shown | no | command control | no | no | no | RT_DIALOGEX `0x100`; message `0x497` |
| RA2TS child `0x71A` | yes | yes | yes | main shell art | chrome/content area | no | no | no | RT_DIALOGEX `0x100`; `FUN_0052B9B0`; WM_PAINT `0x4F0` |
| `FUN_00608260` transition helper | no claim for this command result | no call in proc | not needed for `0x579 -> 0x0B` result | no | no | possible elsewhere | conditional elsewhere | inactive for direct result write | raw PE disassembly of `0x0052D640` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Main-menu result `1` opens dialog resource `0x100` with proc `0x0052D640` through `FUN_0060D380(1)`. | `0x0052DD39..0x0052DD4B`; `FUN_0060D380`; `FUN_00622650` | Rust starts direct Skirmish bridge. | `src/app.rs`, future `src/ui/single_player_shell/`, future render glue | Add a real intermediate Single Player shell state for standard route. | Clicking main-menu Single Player shows Single Player menu, not Skirmish setup. | Do not activate `0x102` from `MainMenuShellAction::SinglePlayer`. |
| Dialog `0x100` contains Skirmish button `0x579` at DLU `(425,176,108,23)` with title `GUI:Skirmish`. | RT_DIALOGEX `0x100` | Missing. | future `single_player_shell::layout/state` | Add control identity/hit-test/render for `0x579`. | Clicking the Skirmish row in the intermediate shell produces route result `0x0B`. | Do not reuse main-menu button `0x683` as Skirmish. |
| `0x579` writes result `0x0B` directly to the dialog result pointer. | `0x0052D6F1..0x0052D720` | Missing. | app shell route dispatcher | Route result `0x0B` must be the standard activation boundary for offline Skirmish setup. | `0x579 -> 0x0B -> g_GameMode=5 equivalent -> 0x102` in route trace. | Do not require `FUN_00608260` to produce the result. |
| `0x686` writes `0x12`, returning to the main menu loop. | `0x0052D6F9..0x0052D70F`; resource `0x100` | Missing for intermediate shell. | app shell route dispatcher | Main Menu button in Single Player shell returns to main menu, not app exit. | Clicking `GUI:MainMenu` hides dialog `0x100` and returns to main menu. | Do not map this button to process exit. |
| `0x689` Load Saved Game is enabled only when valid saves are found. | `0x0052D683..0x0052D6D4`; `FUN_00559C20` | Missing. | future single-player shell state plus save scanner | Disable Load Saved Game when no valid save exists; write result `9` only if active/clickable. | Empty save directory disables `0x689`; valid save enables it. | Do not leave it always enabled. |
| Dialog proc handles `0x68A -> 0x0A`, but resource `0x100` has no `0x68A` child. | proc branch `0x0052D73C..0x0052D74D`; RT_DIALOGEX `0x100` | Missing, but no visible resource control. | none until further research | No required visible control for `0x68A` in standard resource `0x100`. | None for Skirmish route. | Do not invent a visible button from proc-only branch. |
| `0x0052D640` does not call `FUN_00608260` for `0x579 -> 0x0B`. | raw PE disassembly `0x0052D640..0x0052D785` | Rust bridge uses non-native full-target compositor. | transition code | Keep transition/reveal work separate from the route result implementation. | Route result test passes without invoking transition helper. | Do not block `0x579 -> 0x0B` implementation on `0x00612690`. |

### 2026-07-25 current-Rust correction

The `Current Rust delta` cells above describe the 2026-05-27 checkout.
Production `dev` at `e726da11` now contains:

- `src/ui/single_player_shell/` layout/state with the stock `0x688`, `0x689`,
  `0x579`, and `0x686` controls;
- `src/app_single_player_shell_render.rs` production shell composition;
- main-menu result `1` routing to `0x100`, `0x579` routing to offline
  Skirmish `0x102`, and `0x686` routing back to `0xE2`;
- save-list-driven enablement of `0x689`; and
- no invented visible `0x68A` control.

Focused layout/state/route tests validate those Rust contracts, but live input,
focus, cursor, transition, text, audio, and aggregate pixels remain
`UNVERIFIED`. Production `e726da11` also retains a collapsed
`0xE2 -> 0x100 -> Back` movie-session reuse defect when `0x100` never paints;
reviewed feature commit `3a96251e` corrects it but was not yet integrated when
this amendment was written. Accordingly, these implemented rows are not exact
parity certifications.

### Follow-up Contracts / Plans

- Update `docs/contracts/2026-05-27-native-single-player-route-to-skirmish-0x102-implementation-contract.md`: the previous `BLOCKED` row for the source of `0x0B` is now resolved by this report. Remaining blockers are final pixel layout/capture and unrelated transition caller taxonomy.
- A verified fix can now implement the intermediate Single Player route at the mechanism level, with pixel-perfect final layout requiring an additional focused layout trace if the implementation needs exact final rects beyond the resource DLU and established shell helper behavior.

## Sources

- Ghidra decompiled/read-only: `FUN_0060D380`, `FUN_00622650`, `FUN_00622800`, `FUN_0052B9B0`, `FUN_00622720`, `FUN_00559C20`, `LoadOptionsClass__Constructor/Destructor`.
- Ghidra disassembly: `Main_Game @ 0x0052D9A0`, especially `0x0052DD39..0x0052DD4B`.
- Raw PE disassembly with Capstone over retail `gamemd.exe`: `0x0052D640..0x0052D785`, `0x0052D790..0x0052D845`, `0x0052D870..0x0052D996`, `0x00612690`, `0x005E6B49`.
- PE RT_DIALOGEX extraction from retail `gamemd.exe`: resources `0x100`, `0x101`, `0x102`, `0x129`, `0x0E2`.
- Prior reports: `docs/research/skirmish-ui/SKIRMISH_NATIVE_SINGLE_PLAYER_ROUTE_TO_0X102_RECHECK_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_MAIN_MENU_TO_SHELL_TRANSITION_CALLER_FRAME_COMPOSITION_GHIDRA_REPORT.md`.
- Current Rust scan: `src/app.rs`, `src/app_shell_transition.rs`, `src/ui/main_menu_shell/state.rs`, `src/ui/`.
