# SDBTNANM frame-10 setter `FUN_00608440` — callers and user-action triggers

Anchor doc: `SDBTNANM_FRAME10_OVERLAY_CONDITION_GHIDRA_REPORT.md`. That report
identified `FUN_00608440 @ 0x00608440` as the sole live writer of
`record[+0xD8] = 1` (the byte that gates the SDBTNANM frame-10 highlight row
drawn by `RightPanel__Draw`). It located 4 direct CALL sites at `0x0078B808`,
`0x0078BF87`, `0x00792DA6`, `0x00793407` but did not identify their enclosing
dialog procs, message values, or triggering user actions. **This pass fills in
those three axes. No code or comments changed; Ghidra MCP used read-only.**

## TL;DR

- All 4 call sites fire inside Westwood Online (`D:\ra2mdpost\wonline.cpp`)
  dialog procs that share the SDBTNANM common chrome. They are NOT triggered
  from the static main-menu shell (`0xE2`) and they are NOT scattered across
  unrelated screens.
- Three of the four (sites 1, 3, 4) fire on **dialog initialization / refresh**
  (Westwood owner-draw custom messages `0x686`, `0x497`, `0x497` respectively).
  One (site 2, wMsg `0x689`) additionally sets a "exit lobby state" pair and
  also signals the WOL wait-loop exit event.
- All 4 sites pass the **parent dialog HWND** to `FUN_00608440`. Two of them
  (sites 1, 2) immediately follow with `SetEvent(EV_EXIT)` on the WOL wait
  handle, indicating that the frame-10 row also marks "this WOL dialog is in
  its exit/teardown phase."
- The frame-10 highlight is therefore not "menu focused" or "post-init"
  generically; it is the **WOL-lobby-family "active state"** marker. Once a
  WOL chat / lobby / verify-connections dialog has dispatched its
  init-refresh path (`0x497`) once, the bit is set and remains set until the
  WindowExtra record is re-allocated. Combined with the absence of any live
  clearer (see anchor doc), the row is **sticky-on** for the lifetime of the
  dialog.
- Standard YR skirmish surface (campaign menu, options, save/load) does NOT
  hit any of the 4 sites. The bit therefore distinguishes WOL screens from
  non-WOL ones in the multi-dialog shell.

## Verified facts — call site map

Every fact below is confirmed by `disassemble_bytes` / `get_assembly_context`
on the live `gamemd.exe` Ghidra image in this session.

### Site 1 — `0x0078B808`

- **Enclosing function entry:** `0x0078AC10` (prologue `SUB ESP, 0x334; PUSH EBX;
  MOV EBX, [ESP+0x348]` — this is `(HWND, uMsg, wParam, lParam)` standard
  DLGPROC signature). Confirmed by byte-pattern search for `81 EC 34 03 00 00`
  and by the matching `ADD ESP, 0x334; RET 0x10` epilogue.
- **Registered as DLGPROC for dialog ID `0x113`** by code at `0x007879FE`
  (`MOV EDX, 0x78ac10; MOV ECX, 0x113; CALL 0x00622650`). `0x00622650` is
  `CreateDialogIndirectParamA` wrapper. Caller `FUN_00787770` is the WOL
  lobby driver (strings `D:\ra2mdpost\wonline.cpp`, `Setting g_JoinLobbyNow`,
  etc.).
- **wMsg triggering this site:** `wMsg = 0x686`. Identified by the jump table
  at `0x0078CEDC` (range `[0x5C2, 0x686]` after `ADD EAX, 0xFFFFFA3E; CMP EAX,
  0xC4`) and the byte-index map at `0x0078CEF8`: only offset `0xC4` (wMsg
  `0x686`) routes to entry index 5, whose target is `0x0078B806` (the block
  containing the FUN_00608440 call).
- **Surrounding code:** `MOV ECX, EBP` (parent HWND from `param_1` saved in
  EBP), `CALL 0x00608440` (set record[+0xD8]=1), `MOV EDX, [0x00B7369C]`
  (event handle), `PUSH EDX`, `CALL [0x007E1234]`. Verified
  `[0x007E1234] = KERNEL32!SetEvent` via `get_external_location` on
  `0x0040FA72`. So the sequence is `FUN_00608440(parent_HWND);
  SetEvent(DAT_00B7369C)`.
- **`DAT_00B7369C` identity:** event handle, NOT an HWND cache. Confirmed via
  `SetEvent(DAT_00b7369c)` in `FUN_00794ba0` (string literal context
  `s_setting_EV_EXIT_0084c654`). It is event slot index 6 in the
  `DAT_00B73684[18]` event array waited on by `FUN_00787770`'s
  `WaitForMultipleObjects` loop. The corresponding `DVar11 == 6` branch in
  that loop hits `LAB_00787F6E: DAT_00b77ad0 = 0; goto LAB_00787F79` (cleanup
  / leave).
- **User-visible trigger (best inference, MEDIUM confidence):** a click on a
  control with ID `0x686` inside the WOL chat / lobby dialog `0x113`.
  Westwood owner-draw infrastructure (`FUN_0060f9a0` at `0x00610333`) sends
  `SendMessageA(parent, controlID_or_customMsg, ...)`, where the control ID
  becomes the wMsg. Control IDs `0x685`, `0x686`, `0x687`, `0x688`, `0x689`,
  `0x68A` are registered as a sequential layout block in `WinMain` near
  `0x006BBA90..0x006BBAFF`. The 0x686 site does NOT mutate
  `[0x00A8B244/248]` (see site 2), so it is a "soft exit" / "back" action
  that just signals the WOL wait loop to drop out without overwriting the
  lobby-state enum.

### Site 2 — `0x0078BF87`

- **Enclosing function entry:** same as site 1 — `0x0078AC10` (DLGPROC for
  dialog ID `0x113`). The dispatcher at `0x0078BF52` (`ADD EAX, 0xFFFFF977;
  CMP EAX, 0xDE`) handles a second wMsg range `[0x689, 0x767]` inside the
  same proc.
- **wMsg triggering this site:** `wMsg = 0x689`. Identified by jump table
  at `0x0078CFC0` and byte-index map at `0x0078CFDC`: only offset `0` (wMsg
  `0x689`) routes to entry index 0 → target `0x0078BF71` (the block).
- **Surrounding code:** `MOV ECX, EBP` (parent HWND), `MOV
  dword ptr [0x00A8B244], 0x5; MOV dword ptr [0x00A8B248], 0x4`, `CALL
  0x00608440` (set highlight bit), then `MOV EDX, [0x00B7369C]; PUSH EDX;
  CALL [0x007E1234]` (= `SetEvent(EV_EXIT)`).
- **`[0x00A8B244]` identity:** WOL lobby state enum. Values 1, 2, 3 are
  active WOL match modes (referenced throughout `FUN_00787770` as
  `DAT_00a8b244 == 3` / `!= 1, 2, 3` gates). Writing `5` here signals an exit
  / disconnected state to the lobby driver. Confirmed by xref scan — the
  variable is heavily read in `FUN_00598960`, `FUN_0077b2a0` and the WOL
  lobby driver.
- **User-visible trigger (best inference, MEDIUM confidence):** a click on a
  control with ID `0x689` inside the WOL lobby dialog `0x113`. Distinct from
  site 1 in that it commits a **state-changing** disconnect (sets
  `lobby_state = 5, sub_state = 4`) in addition to triggering the wait-loop
  exit. Conventional Westwood layout would name this a "Disconnect" or
  "Quit Lobby" button.

### Site 3 — `0x00792DA6`

- **Enclosing function entry:** `0x00792CF0` (`MOV EAX, [ESP+8]; PUSH ESI; CMP
  EAX, 0x110; JA ...; ...`). Standard DLGPROC.
- **Registered as DLGPROC for dialog ID `0xC4`** by `FUN_00792BE0` at
  `0x00792C66..0x00792C79` (`MOV ECX, [0x00B732F0]` = HINSTANCE; `MOV EDX,
  0xC4` = dialog ID; `PUSH 0` (showFlag); `PUSH 0x792CF0` (DLGPROC); `PUSH
  EAX` (parent HWND); `CALL 0x00775700` = `CreateDialogIndirectParamA`
  wrapper).
- **Dialog `0xC4` is in the WW main-shell sub-dialog whitelist** at
  `FUN_00622820`: the explicit check `iVar1 == 0xc4 || iVar1 == 0x130 || ...`
  sets `record[+0xB0] = 2` for every dialog in that family. This whitelist
  includes the prior report's main-menu shell IDs (`0xE2`, `0xCE`, etc.) and
  identifies `0xC4` as a sibling using the same SDBTNANM chrome.
- **wMsg triggering this site:** `wMsg = 0x497`. Reached after three `SUB`s
  (`SUB EAX, 0x111; SUB EAX, 0x2; SUB EAX, 0x384` totals `0x497`; the
  `JNZ 0x792e3f` after the last `SUB` falls through only when wMsg == 0x497).
  `0x497` is the **Westwood owner-draw "init / refresh control" custom
  message**; senders include `FUN_0060f9a0 @ 0x00610333`
  (`PUSH 0x497; PUSH EAX; CALL SendMessageA`).
- **Surrounding code:** `SendMessageA(hWnd, 0x4A9, 0, 1)` (control-state
  update), `SetTimer(hWnd, 0x45, [0x0084A248]==0x1B58 ms = 7s, lpTimerFunc)`,
  `FUN_00608440(hWnd)` (set highlight bit), then `GetDlgItem(hWnd, 0x7A9)`
  and `PostMessageA(hWnd, 0x4D5, 0, 0); PostMessageA(hWnd, 0x4D3, 0, 0)`
  (queue follow-up refreshes). Identified the imports via
  `get_external_location`:
  - `[0x007E1234] = KERNEL32!SetEvent`
  - `[0x007E148C] = USER32!KillTimer`
  - `[0x007E1490] = USER32!SendDlgItemMessageA`
  - `[0x007E1494] = USER32!SetTimer`
  - `[0x007E1498] = USER32!ShowWindow`
  - `[0x007E14A4] = USER32!SendMessageA`
  - `[0x007E14A8] = USER32!GetDlgItem`
  - `[0x007E14AC] = USER32!PostMessageA`
- **User-visible trigger (HIGH confidence):** owner-draw infrastructure
  refreshes the dialog `0xC4` chrome (e.g. on creation, on resize, on a
  global theme/state change). This fires when the dialog `0xC4` is shown —
  not on a specific button click. The frame-10 row is enabled as part of
  the chrome init.

### Site 4 — `0x00793407`

- **Enclosing function entry:** `0x00793280` (`MOV EAX, [ESP+8]; PUSH EBX,
  ESI; CMP EAX, 0x111; JA 0x793350; JZ 0x7932D7`). Standard DLGPROC.
- **Registered as DLGPROC for dialog ID `0x130`** by `FUN_00794BA0` at
  `0x00794C5B..0x00794C66` (`PUSH 0x793280` (DLGPROC); `PUSH EAX` (parent
  HWND); `MOV EDX, 0x130` (dialog ID); `CALL 0x00775700`).
- **Dialog `0x130` is also in the WW main-shell whitelist** at
  `FUN_00622820` (explicit `iVar1 == 0x130` check).
- **Caller identity:** `FUN_00794BA0` is the **"Start Game Now" / scenario
  verify-connections driver** in wonline.cpp (strings
  `s_Start_Game_Now_0084c6b0`, `s_Closing_Verify_Connections_Dialo_0084c5b4`,
  `s_Negotiate_Scenario_Transfer_succ_0084c5d8`). So dialog `0x130` is the
  **Verify Connections progress dialog** shown after the host clicks "Start
  Game" in a WOL lobby.
- **wMsg triggering this site:** `wMsg = 0x497` (same owner-draw init/refresh
  message as site 3). Reached via the second dispatch:
  `CMP EAX, 0x132; JC 0x79332C; CMP EAX, 0x138; JBE 0x79341E; CMP EAX, 0x497;
  JNZ 0x79332C`. Verified at `0x00793362..0x00793369`.
- **Surrounding code:** the `0x497` handler walks the 8-entry control-ID
  table at `0x0084A1F0` `= [0x769, 0x761, 0x764, 0x762, 0x765, 0x768, 0x766,
  0x767]` (player slot labels). For each entry it calls `GetDlgItem(hWnd,
  ctrlID)` and `SendMessageA(child, 0x4B2, 0, ...)` to populate the row. The
  block ending with `CALL FUN_00608440` at `0x00793407` sets the
  highlight bit on the parent and then calls `[0x007E1498] = ShowWindow` with
  flag `5` (= `SW_SHOW`).
- **User-visible trigger (HIGH confidence):** owner-draw infrastructure
  refreshes the **Verify Connections progress dialog `0x130`** after it has
  been populated with player-slot rows. Fires once per show of that dialog
  (which happens when the WOL lobby host clicks "Start Game"). The frame-10
  row is enabled as part of the dialog show.

## Identified imports (this report)

| IAT slot     | API                          | Method of identification |
|--------------|------------------------------|--------------------------|
| `[0x007E1234]` | `KERNEL32!SetEvent`        | `get_external_location` → `0x0040FA72` |
| `[0x007E148C]` | `USER32!KillTimer`         | `get_external_location` → `0x0041007C` |
| `[0x007E1490]` | `USER32!SendDlgItemMessageA` | `get_external_location` → `0x00410066` |
| `[0x007E1494]` | `USER32!SetTimer`          | `get_external_location` → `0x0041005A` |
| `[0x007E1498]` | `USER32!ShowWindow`        | `get_external_location` → `0x0041004C` |
| `[0x007E149C]` | `USER32!InvalidateRect`    | `get_external_location` → `0x0041003A` |
| `[0x007E14A0]` | `USER32!EnableWindow`      | `get_external_location` → `0x0041002A` |
| `[0x007E14A4]` | `USER32!SendMessageA`      | `get_external_location` → `0x0041001A` |
| `[0x007E14A8]` | `USER32!GetDlgItem`        | `get_external_location` → `0x0041000C` |
| `[0x007E14AC]` | `USER32!PostMessageA`      | `get_external_location` → `0x0040FFFC` |

## YR-active status — all 4 sites

All four call sites live in `wonline.cpp` dialog procs that are exercised on
the live YR multiplayer path:

- Dialog `0x113` (sites 1, 2) — WOL chat / lobby dialog. Driven by
  `FUN_00787770` which loops `WaitForMultipleObjects` on the EV_EXIT event
  signaled by both sites. This is the standard "Online" → channel-list →
  game-room flow.
- Dialog `0xC4` (site 3) — WOL custom-match sub-dialog. Created by
  `FUN_00792BE0`, which is invoked from `FUN_00787770`'s message path when
  WOL transitions to the custom-match phase.
- Dialog `0x130` (site 4) — Verify Connections progress dialog. Created by
  `FUN_00794BA0`'s "Start Game Now" flow when the host hits Start.

None of the four sites are TS-only or gated behind `SpecialFlags`. All four
are reachable in a vanilla YR retail multiplayer game (the only requirement
is choosing Online → joining a channel → entering a game room → starting a
game).

Standard YR skirmish (`g_GameMode != 4`) does not exercise WOL; an offline
single-player skirmish will never set the frame-10 bit. This matches the
observable symmetry the anchor doc noted: the row toggles only in certain
shell screens, not all of them.

## Semantic conclusion — what the bit means

Combining this pass with the anchor doc:

- `record[+0xD8] = 0` (default): dialog is **not** a WOL-family dialog, or is
  one but has not yet hit a `0x497` init-refresh or a button-driven exit.
  Frame-10 SDBTNANM row is drawn (`param_3 == 1`).
- `record[+0xD8] = 1`: dialog is one of the **WOL-family screens (0x113,
  0xC4, 0x130, ...)** and has either (a) been initialized via owner-draw
  refresh `0x497`, or (b) had a "back" / "disconnect" button (control IDs
  `0x686`, `0x689`) clicked. Frame-10 SDBTNANM row is **not** drawn
  (`param_3 == 0`).

The frame-10 highlight is thus the **"this dialog uses WOL chrome"** marker.
It is set during WOL dialog setup (sites 3, 4) and during WOL dialog
button-exit (sites 1, 2), both of which transition the dialog through its
WOL-specific lifecycle. The marker remains set (sticky-on per the anchor
doc) because there is no live clearer — once a dialog is recognized as
WOL-family, its chrome stays in WOL mode for its lifetime.

This is consistent with what's player-visible: WOL screens have a distinct
chrome treatment vs the offline shell, and that treatment is locked in once
the dialog enters the WOL code path.

## Confidence

- **Function entries (0x0078AC10, 0x00792CF0, 0x00793280):** HIGH. Verified
  by prologue (`SUB ESP, 0x334; PUSH EBX; ...`) and matching epilogue
  (`POP EBX; ADD ESP, 0x334; RET 0x10`). The dispatchers' `MOV EAX, [ESP+8]`
  + comparisons match standard `(HWND, uMsg, wParam, lParam)` DLGPROC
  signature.
- **Dialog IDs (0x113, 0xC4, 0x130):** HIGH. Each verified at the
  `CreateDialogIndirectParamA` wrapper call site (literal `MOV ECX/EDX,
  imm32`).
- **wMsg values (0x686, 0x689, 0x497, 0x497):** HIGH. Each verified at the
  arithmetic dispatch (`ADD EAX, ...; CMP EAX, ...; JMP [table]`) and the
  byte-index map.
- **Import identifications:** HIGH. Verified via `get_external_location`
  against the live Ghidra image.
- **User-action mapping for sites 1/2 (button clicks for control IDs
  `0x686`, `0x689`):** MEDIUM. The wMsg-equals-control-ID convention is
  consistent with the owner-draw infrastructure (`FUN_0060f9a0`), but the
  exact button labels are not extracted from the dialog resource here. The
  evidence is the Westwood layout-registration block in `WinMain` at
  `0x006BBA80..0x006BBAFF` that registers IDs `0x685..0x68A` as a contiguous
  control group, and the absence of any other `SendMessage(..., 0x686, ...)`
  / `PostMessage(..., 0x689, ...)` sender in the binary. To raise to HIGH,
  extract the RT_DIALOG resource and read the button text.
- **User-action mapping for sites 3/4 (owner-draw refresh on dialog show):**
  HIGH. The wMsg `0x497` sender at `FUN_0060f9a0+0x99` is unambiguously the
  Westwood "chrome init/refresh" path, and both call sites' surrounding code
  (start timer, populate slots, ShowWindow) are clearly dialog-initialization
  work.

## Unknowns / deferred

- **Button text for control IDs `0x686` and `0x689`.** Would require parsing
  the RT_DIALOG resource for dialog `0x113` (or reading the dialog template
  via `LoadResource` / `LockResource` from the binary directly). Both
  control IDs would resolve to STT/TXT string-table entries.
- **Whether dialog `0x113` is the WOL "Chat Lobby" (channel list) or the
  WOL "Game Room" (player list).** Both share the wonline.cpp DLGPROC family
  and `FUN_00787770` driver; the distinction is gated by
  `DAT_00a8b244 == 1/2/3`. The frame-10 setter does not differentiate; it
  fires for both.
- **The full set of WOL-family dialogs that hit this code path.** The
  `FUN_00622820` whitelist contains 19 dialog IDs (0x10D, 0xD9, 0xF0, 0xCE,
  0x120, 0x121, 0x115, 0xD3, 0xCF, 0x11F, 0xC3, 0x11B, 0xE1, 0x11E, 0xC4,
  0x130, 0xD0, 0xFC, 0x126). Only 0x113 / 0xC4 / 0x130 are confirmed via the
  4 setter call sites. Other dialogs may set the bit through an indirect
  call path that this pass did not enumerate. The anchor doc's
  observation that no clearer is live still applies.

## Symbol table (this report)

| Symbol | Address | Status |
|---|---|---|
| Dialog `0x113` DLGPROC | `0x0078AC10` | live; not in Ghidra function list yet |
| Dialog `0xC4` DLGPROC  | `0x00792CF0` | live; not in Ghidra function list yet |
| Dialog `0x130` DLGPROC | `0x00793280` | live; not in Ghidra function list yet |
| WOL lobby driver | `FUN_00787770 @ 0x00787770` | exists |
| WOL custom-match dialog launcher | `FUN_00792BE0 @ 0x00792BE0` | exists |
| WOL Start-Game / Verify-Connections driver | `FUN_00794BA0 @ 0x00794BA0` | exists |
| WW common DLGPROC pre-dispatcher | `FUN_00622B50 @ 0x00622B50` | exists |
| WW main-shell whitelist + chrome setup | `FUN_00622820 @ 0x00622820` | exists |
| WW CreateDialog wrapper (3-arg) | `FUN_00622650 @ 0x00622650` | exists |
| WW CreateDialog wrapper (5-arg) | `FUN_00775700 @ 0x00775700` | exists |
| WW owner-draw refresh-sender (sends 0x497) | `FUN_0060f9a0 @ 0x0060F9A0` | exists |
| WOL EV_EXIT event handle | `DAT_00B7369C` | live; HANDLE not HWND |
| WOL event array base | `DAT_00B73684 [18 events]` | live |
| Player-slot control IDs (dlg 0x130) | `0x0084A1F0` `[0x769,0x761,0x764,0x762,0x765,0x768,0x766,0x767]` | confirmed |
| Verify-connections timer interval (ms) | `[0x0084A248] = 0x1B58 = 7000` | confirmed |
| Layout-registration block (ctrl IDs 0x685..0x68A) | `0x006BBA80..0x006BBAFF` in WinMain | confirmed |
