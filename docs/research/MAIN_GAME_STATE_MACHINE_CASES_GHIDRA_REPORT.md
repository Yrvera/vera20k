# Main_Game State Machine — Full Case Enumeration

Date: 2026-05-19

Scope: enumerate every case in the outer `switch(iVar11)` of `Main_Game @ 0x0052D9A0`,
document which dialog/function each case dispatches to, what state-variable transitions
occur, and the player-visible meaning. Cases 6 and 7 were already partially documented
in `QUIT_CONFIRM_DIALOG_MAIN_MENU_GHIDRA_REPORT.md`. This report extends coverage to
every reachable case.

No Rust code, INI files, or Ghidra annotations were modified.

Source docs used as prior art:
- `QUIT_CONFIRM_DIALOG_MAIN_MENU_GHIDRA_REPORT.md` — cases 6 and 7
- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md` — dialog 0xE2 / FUN_00531CC0

Verified via: `decompile_function 0x0052D9A0` (full function body obtained in one call).

---

## Architecture Overview

`Main_Game @ 0x0052D9A0` is the top-level game-session loop for non-editor YR.
It runs as an infinite `do { … } while (true)` loop. Each iteration:

1. Resets global state (`g_GameActive`, `g_PlayerPtr`, frame counters, etc.)
2. Conditionally enters replay-recording path (if `_DAT_00a8d5f8 & 2` set and recording active)
3. Enters label `LAB_0052dc40` — determines `iVar11` (the state variable)
4. If `g_IsMapEditor == 0` and `DAT_00a8ed5d == 0`:
   - If `g_GameMode == 0`: calls `FUN_00531CC0()` → gets return code 1–6 → `iVar11`
   - If `g_GameMode == 4`: `iVar11 = 0x10`; if `g_GameMode == 7` (never set here): `iVar11 = 0x10`
   - Overrides `iVar11 = 8` if `DAT_00a8ed5d != 0`
5. Calls `Network_ServiceLoop()` unconditionally
6. Executes `switch(iVar11)` — this is the primary state machine
7. After the switch either loops back to step 1 (via `LAB_0052dc40`) or proceeds to
   scenario launch / game execution

**State variable**: local `iVar11` (Ghidra name). Its values are the case keys below.
**Loop-back condition**: if `bVar1` is true, iteration loops back via `LAB_0052dc40`
(true by default; set false only by case 0x10/WOL path with `_DAT_00825c84 = 2`).

Active in YR: Yes (full function is the live main-menu/session orchestrator).

---

## State Machine Entry Routing

Before the switch, `iVar11` is assigned as follows:

| Condition | `iVar11` assigned |
|---|---|
| `g_GameMode == 0` and `DAT_00a8ed5d == 0` | return value of `FUN_00531CC0()` (dialog 0xE2 result, 1–6) |
| `g_GameMode != 0` and `g_GameMode != 4` | `0x12` (but overridden by `if iVar11==7` guard; essentially re-uses pre-set `iVar11` from prior iteration) |
| `g_GameMode == 4` | `0x10` |
| `DAT_00a8ed5d != 0` | `8` (mission briefing / CD-check flow) |
| Replay load active (`_DAT_00a8d5f8 & 2`) | jumps directly to `switchD_0052e356_caseD_1` (scenario launch), bypassing the switch |

`FUN_00531CC0 @ 0x00531CC0` is the main-menu dialog runner. It creates dialog `0xE2`
via `CreateDialogIndirectParamA` with proc `MainMenuDialog0xE2_Proc_00531F60 @ 0x00531F60`,
runs a modal message loop, and returns the button return code. Verified via
`get_function_by_address 0x00531cc0` (confirmed name/body) and the existing anchor doc.

---

## Case Table

### Case 1 — Single Player menu

```
DAT_00AC10C8 = 0;
iVar11 = FUN_0060D380(1);   // 0x0060D380
```

`FUN_0060D380` creates a shell dialog via `FUN_00622650(0)` (confirmed via
`decompile_function 0x0060d380`), runs a message pump (`Process_NetworkMessages`,
`Main_Tick`), and returns the dialog result code. With `param_1 = 1`, it also calls
`FUN_0052B9B0` after showing the dialog (likely triggers EVA welcome-back audio).

The return value becomes the new `iVar11`, which can be any further sub-case
(e.g. campaign → case 0x10, skirmish → case 0x10, back → case 1 again).

- **Player-visible meaning**: Single Player sub-menu opened (campaigns, skirmish).
- **State transitions**: `DAT_00AC10C8 = 0` (resets some SP session flag),
  `iVar11` ← dialog result.
- **Active in YR**: Yes.
- **TS risk**: None; direct dialog dispatch.
- Confidence: Content HIGH / Identity HIGH (confirmed Ghidra label, confirmed body)
  / Binding HIGH (switch branch explicit).

---

### Case 2 — WW Online (Westwood Online / WOL)

```
iVar11 = 0x10;
g_GameMode = 4;
FUN_0053F1F0();             // 0x0053F1F0 — stores param into DAT_00828140
```

`FUN_0053F1F0` is a 6-byte fastcall stub: `DAT_00828140 = param_1; return;`
(confirmed via `decompile_function 0x0053f1f0`). Here it is called with no
explicit argument visible in the decompiler; its role is to set a WOL sub-state
flag. `g_GameMode = 4` marks the session as Internet/WOL mode. `iVar11 = 0x10`
routes the next iteration into case 0x10 (network lobby setup).

- **Player-visible meaning**: WW Online button pressed → WOL lobby connection
  sequence begins. (WOL was discontinued; this path is a dead end in current retail.)
- **State transitions**: `g_GameMode = 4`, `iVar11 = 0x10`.
- **Active in YR**: Conditionally (code is live; WOL servers are offline).
- Confidence: Content HIGH / Identity HIGH / Binding HIGH.

---

### Case 3 — LAN / Network

```
iVar11 = 0x10;
g_GameMode = 3;
FUN_0053F1F0();             // same stub, sets DAT_00828140
```

Same pattern as case 2 but `g_GameMode = 3` (LAN mode). Routes to case 0x10.

- **Player-visible meaning**: Network (LAN) button pressed → LAN lobby launched.
- **State transitions**: `g_GameMode = 3`, `iVar11 = 0x10`.
- **Active in YR**: Yes.
- Confidence: Content HIGH / Identity HIGH / Binding HIGH.

---

### Case 4 — Movies and Credits

```
iVar11 = FUN_0060D380(1);   // same shell-dialog runner as case 1
```

Same `FUN_0060D380` call, same param. The difference from case 1 is that no
`DAT_00AC10C8` reset occurs and the dialog ID dispatched by the shell runner will
be a different one (the Movies & Credits dialog, not the SP menu). The return
value routes further.

- **Player-visible meaning**: Movies & Credits sub-menu opened.
- **State transitions**: `iVar11` ← dialog result.
- **Active in YR**: Yes.
- Confidence: Content MEDIUM (no separate dialog ID confirmed; same runner, different
  caller context) / Identity HIGH / Binding HIGH.

---

### Case 5 — Options

```
iVar11 = 0x12;              // return to main menu next iteration
OptionsClass__ShowLauncherDialog();   // Ghidra decompiler label
```

The label `OptionsClass__ShowLauncherDialog` is inferred by Ghidra from RTTI.
`iVar11 = 0x12` is set first, ensuring that after the options dialog closes the
next iteration calls `FUN_00531CC0()` again (i.e., returns to the main menu).

- **Player-visible meaning**: Options dialog opened; after closing, main menu shown.
- **State transitions**: `iVar11 = 0x12` (main-menu re-enter).
- **Active in YR**: Yes.
- Confidence: Content HIGH / Identity MEDIUM (RTTI label, not independently
  decompiled in this session) / Binding HIGH.

---

### Case 6 — Exit / Quit confirm

Already fully documented in `QUIT_CONFIRM_DIALOG_MAIN_MENU_GHIDRA_REPORT.md`.
Summary:

```
// Load CSF strings: "GUI:ExitAreYouSure", "TXT_OK", "GUI:Cancel"
iVar11 = FUN_005D3490(str1, str2, str3, 0, 0);   // modal CSF message-box
if (iVar11 == 0) {
    iVar11 = 7;
    OptionsClass__WriteToINI();   // 0x005FAD10
}
else {
    iVar11 = 0x12;   // back to main menu
}
```

- **Player-visible meaning**: Quit confirm dialog. "OK" → save options + exit;
  "Cancel" → back to main menu.
- **State transitions**: Affirm: `iVar11 = 7`; Cancel: `iVar11 = 0x12`.
- **Active in YR**: Yes.
- Confidence: All HIGH (verified in anchor doc and this decompilation).

---

### Case 7 — Shutdown / Clean exit

Already partially documented in `QUIT_CONFIRM_DIALOG_MAIN_MENU_GHIDRA_REPORT.md`.
Full decompilation confirms:

```
FUN_00720EA0(1);            // music stop (param=1 means stop)
iVar11 = DAT_00887340;
if (DAT_00887338 != -1) {
    iVar5 = GetRadarTimer();
    iVar11 += (iVar5 - DAT_00887338);   // accumulate elapsed ticks
}
cVar3 = FUN_00720FD0();     // pump/check if music finished
goto joined_r0x0052e7a3;    // fade-out wait loop
```

The `joined_r0x0052e7a3` block waits up to 3000 ms for an active Vox/music
fadeout (`VoxClass__PumpAndCheckActive`, `FUN_00720FD0`), then:

```
FUN_00720EA0(0);            // final music stop
if (DAT_008175b0 == 0) FUN_004A3C30(0);   // fade-out screen
return 0;
```

Returning 0 from `Main_Game` causes the outer `Main_Game @ 0x0048CCC0` loop to exit,
which cascades up through WinMain and terminates the process cleanly.

- **Player-visible meaning**: Screen fades, game process exits.
- **State transitions**: returns 0 from function (terminal).
- **Active in YR**: Yes.
- Confidence: All HIGH.

---

### Case 8 — Mission Briefing / Pre-scenario CD dialog

```
FUN_004C6140();             // struct init (resets 0xD-0xF fields + 0-0xC range)
// local_4 = -1, cStack_108 = 0
// CDFileClass__Constructor() / VocHandle__Init() ...
hWnd = FUN_00622650(0);     // create shell dialog
// SetWindowLongA(hWnd, 8, &uStack_10c) + pump loop
// Wait up to 3000ms for EVA audio
FUN_0072DAA0();
uVar4 = uStack_10c;         // dialog result
*(g_ScenarioClass_Instance + 0x34CC) = uVar4;
switch(DAT_00a8eb64) { ... }   // scenario type dispatch into g_ScenarioClass fields
```

`FUN_004C6140` confirmed as a struct-init helper (via `decompile_function 0x004c6140`).
`FUN_00622650` is the generic shell-dialog creator (confirmed via `decompile_function 0x00622650`).
This case prepares audio (`VocHandle__Init`), creates a pre-scenario dialog (mission briefing
or CD check), waits for the dialog result, then routes into a nested switch on
`DAT_00a8eb64` (sub-case 0–4) that sets `g_ScenarioClass_Instance+0x610` and `+0x60c`
and `g_GameMode = 0`, preparing for `ScenarioClass__Start_Scenario`.

On failure (dialog result == -1), sets `iVar11 = 1` and loops back to main menu.

- **Player-visible meaning**: Mission briefing dialog before a scenario starts.
  If the player cancels, returns to the Single Player menu (iVar11=1 → LAB_0052dc40).
- **State transitions**: Sets scenario params via `g_ScenarioClass_Instance+0x34CC`,
  `+0x610`, `+0x60C`, `g_GameMode = 0`; then breaks to scenario-launch path.
- **Active in YR**: Yes (entered whenever `DAT_00a8ed5d != 0` or directed from case 0x10).
- **TS risk**: None; standard briefing path.
- Confidence: Content HIGH / Identity HIGH / Binding HIGH.

---

### Case 9 — Load Game (Save/Load dialog)

```
g_MapEditorMode++;
CDFileClass__Constructor();
g_MapEditorMode--;
LoadOptionsClass__Constructor();
cVar3 = FUN_005587F0();     // 0x005587F0
LoadOptionsClass__Constructor();
if (cVar3 != 0) {
    bVar2 = true;
    goto switchD_0052e356_caseD_1;   // proceed to scenario launch
}
iVar11 = 1;   // back to SP menu
```

`FUN_005587F0` (confirmed via `decompile_function 0x005587f0`) is a 12-byte stub
that sets `(param_1+4) = 1` and `(param_1+0xC) = 0`, then calls
`CDFileClass__Constructor`. It appears to initialize a load-game state structure.
`g_MapEditorMode` bump is a standard guard used to suppress certain game-mode checks
during CD/file init. `LoadOptionsClass__Constructor` frames the file-picker
dialog that lets the player pick a save.

If the player picks a save (`cVar3 != 0`), `bVar2 = true` and execution jumps
directly to the scenario-launch path. If cancelled, `iVar11 = 1` (back to SP menu).

- **Player-visible meaning**: Load Saved Game file picker.
- **State transitions**: On success: `bVar2 = true` → launch saved game. On cancel:
  `iVar11 = 1` → back to SP menu.
- **Active in YR**: Yes.
- Confidence: Content MEDIUM (LoadOptionsClass not independently verified) /
  Identity HIGH / Binding HIGH.

---

### Case 10 — No-op / back to main menu

```
iVar11 = 1;
```

Sets `iVar11 = 1` (back to SP menu). This appears to be a "cancelled" fall-through
from some other state, or a safety default.

- **Player-visible meaning**: Silent return to Single Player sub-menu.
- **State transitions**: `iVar11 = 1`.
- **Active in YR**: Yes (but narrow trigger; likely a cancel/error path).
- Confidence: Content HIGH / Identity N/A / Binding HIGH.

---

### Case 0xB — Reconnect / post-disconnect campaign re-entry

```
g_GameMode = 5;
// falls through to case 0x10
```

Falls through with no break into the case 0x10/0x11 block.
`g_GameMode = 5` is the "Skirmish"-like solo/LAN-disconnect recovery mode.

- **Player-visible meaning**: Post-disconnect recovery — re-enters lobby/LAN setup
  with `g_GameMode = 5`.
- **State transitions**: `g_GameMode = 5`, then case 0x10 path.
- **Active in YR**: Conditional (reached from WOL case 0x10 sub-case 2 return code 2).
- Confidence: Content HIGH / Identity N/A / Binding HIGH.

---

### Case 0x10 / 0x11 — Network lobby / scenario-type dispatcher

```
CDFileClass__Constructor();
// HouseTypeClass array init
DAT_00a8dab4 = 0;
switch(g_GameMode) {
    case 0: goto scenario-select path
    case 1/2: FUN_005F1950() check + FUN_005B77E0() or FUN_005B49B0()  // Modem host/guest
    case 4:   FUN_0077B2A0()   // WOL lobby (returns 1/2/0/-1/0xFFFFFFFE)
    case 5:   FUN_006AE2C0()   // Skirmish setup dialog
    case 3:   IPXInterfaceClass__Constructor() + FUN_005DB680() + FUN_005DC350()  // LAN IPX
}
```

This is the multiplayer lobby dispatcher. After CD/house init, it branches on
`g_GameMode`:

- **g_GameMode 0** → selects a scenario file (single-player path)
- **g_GameMode 1/2** → `FUN_005B77E0` (ModemHost dialog, confirmed via log string
  `s_ModemHost_Dialog_enter__0082c958` in decompilation) or `FUN_005B49B0`
  (ModemGuest dialog, confirmed via `s_ModemGuest_Dialog_enter__0082c36c`).
  These are Modem (serial/dial-up) dialogs — TS legacy code reachable in YR
  but effectively dead (no modem support in modern OS). **Active in YR: No
  (requires serial/modem hardware and driver; effectively TS legacy).**
- **g_GameMode 3** → LAN/IPX lobby via `FUN_005DB680` (IPX init, confirmed via
  decompile — checks `FUN_00540A80` for network availability) + `FUN_005DC350`
  (full LAN game setup dialog, confirmed via CSF string `s_netdlg2_cpp` log refs).
  **Active in YR: Yes (LAN games still work).**
- **g_GameMode 4** → `FUN_0077B2A0` (WOL main lobby, confirmed via
  `SimpleWonlineDialogControl__Constructor` calls and internal switch on
  `DAT_00a8b244` sub-states 0–6). **Active in YR: No (WOL servers offline).**
- **g_GameMode 5** → `FUN_006AE2C0` (Skirmish setup dialog, confirmed via
  `decompile_function 0x006ae2c0` — creates a shell dialog, pumps, checks for
  return codes `0x617` (start) or `0x5C0` (back)). **Active in YR: Yes.**

After the inner switch completes, a second switch on `g_GameMode` routes
to either scenario launch (`switchD_0052e356_caseD_1`) or back to menu.

- **Player-visible meaning**: Multiplayer lobby (LAN / Skirmish / WOL / Modem).
- **State transitions**: varies by sub-mode; ultimately either reaches scenario
  launch (game starts) or iVar11=0x12 (back to main menu).
- Confidence: Content HIGH (inner functions decompiled) / Identity HIGH / Binding HIGH.

Case 0x11 has the same body as 0x10 in Ghidra (no break before 0x10, they share code).
Confirmed by decompile — `case 0x10: case 0x11:` share the same block.

---

### Case 0xD — Play intro movie then show Movies menu

```
iVar11 = 4;               // Movies & Credits menu next
FUN_005BED40(1,1,1,0);    // play a Bink movie
BSurface__Constructor();  // clear/reset surface
```

`FUN_005BED40` confirmed via `decompile_function 0x005bed40` — it is the movie
playback helper (plays a Bink or AVI file, handles VoxClass pause/resume, surface
clear, and stretching). Here called to play an introductory cinematic before
opening the Movies menu (case 4 next).

- **Player-visible meaning**: Intro/campaign movie plays, then Movies & Credits
  menu opens.
- **State transitions**: `iVar11 = 4`.
- **Active in YR**: Yes.
- Confidence: Content HIGH / Identity HIGH / Binding HIGH.

---

### Case 0xE — Play movie with CD-check then dispatch

```
iVar5 = FUN_0060D380(1);   // shell dialog runner
if (iVar5 == -1 || iVar5 == 0) {
    iVar11 = 4;             // back to Movies menu
} else {
    // vtable call on CD class — check if media available
    cVar3 = (*vtable__CD)(iVar5+8);
    if (cVar3) {
        FUN_005BED40(1,1,1,0);   // play movie
        BSurface__Constructor();
    }
}
```

Opens a dialog (via `FUN_0060D380`) that lets the player pick a movie, then
verifies CD availability via a vtable dispatch on `vtable__CD`, and plays the
movie if the CD is present.

- **Player-visible meaning**: Player picks a movie from a list; game checks disc
  presence and plays the selected movie.
- **State transitions**: `iVar11 = 4` (back to Movies & Credits) if cancelled.
- **Active in YR**: Conditional (CD check; digital installs may skip disc check
  depending on no-CD patches).
- **TS risk**: CD check behavior may be partially dormant on no-CD installs.
- Confidence: Content HIGH / Identity MEDIUM (vtable__CD inferred) / Binding HIGH.

---

### Case 0xF — Play movie via music track then back to Movies

```
iVar11 = 4;
CDFileClass__Constructor();
uVar4 = FUN_00721210(s_INTRO_008263a8);
FUN_00720B20(uVar4);        // start/queue music track
BSurface__Constructor();
```

`FUN_00721210` looks up a music track by name (confirmed by its use across the
function for `s_INTRO` string). `FUN_00720B20` starts playback. This case
appears to play an audio-only or music-backed credits sequence.

- **Player-visible meaning**: Credits or music track plays; then back to Movies
  & Credits menu.
- **State transitions**: `iVar11 = 4`.
- **Active in YR**: Yes.
- Confidence: Content MEDIUM (FUN_00720B20 not independently decompiled) /
  Identity HIGH / Binding HIGH.

---

### Case 0x12 — Main menu re-entry sentinel

`0x12` is the value that causes the state routing at the top of the loop to call
`FUN_00531CC0()` again (the main-menu dialog runner). It is not a switch case
body by itself — Ghidra shows no `case 0x12:` in the switch. Instead, the
pre-switch routing block:

```c
if (iVar11 == 0x12) {
    if (DAT_00a8ed5d == '\\0') {
        iVar11 = FUN_00531CC0();
        goto LAB_0052dca1;
    }
    iVar11 = 8;
}
```

So 0x12 is not a case in the switch — it is the "show main menu" sentinel.
Multiple cases set `iVar11 = 0x12` to return to the main menu.

---

### Case -1 (0xFFFFFFFF) — Replay load / post-game return

```
if ((_DAT_00a8d5f8 & 4) != 0 && FUN_00473C50(0)) {
    _DAT_00a8d5f8 |= 2;
    // load replay packets into ScenarioClass
    FUN_00720EA0(1);   // stop music
    goto switchD_0052e356_caseD_1;  // launch scenario (as replay)
}
goto switchD_0052e161_caseD_0:
    iVar11 = 0x12;   // back to main menu
```

`FUN_00473C50` (confirmed present via `get_function_by_address 0x00473c50`) is
the replay-available check. `FUN_00473D10` (0x00473D10, confirmed) reads replay
packets. This case handles the post-game state when the engine returns from a
completed scenario: if replay data is available and replay flag is set, load it;
otherwise return to main menu.

- **Player-visible meaning**: After a game ends, either starts replay playback or
  returns to main menu.
- **State transitions**: Replay: sets `_DAT_00a8d5f8 |= 2`, loads replay frames,
  goes to scenario launch. No replay: `iVar11 = 0x12`.
- **Active in YR**: Yes (post-game return path; replay is conditional on recording flag).
- Confidence: Content HIGH / Identity MEDIUM (FUN_00473C50 not decompiled in-session) /
  Binding HIGH.

---

## State Variable Summary Table

| Case | Dialog/Function dispatched | Next `iVar11` / exit | Player-visible meaning | Active YR |
|---:|---|---|---|---|
| `1` | `FUN_0060D380(1)` — shell dialog runner | dialog result | Single Player sub-menu | Yes |
| `2` | `FUN_0053F1F0()` stub; sets `g_GameMode=4` | `0x10` | WW Online button | No (WOL offline) |
| `3` | `FUN_0053F1F0()` stub; sets `g_GameMode=3` | `0x10` | LAN button | Yes |
| `4` | `FUN_0060D380(1)` — shell dialog runner | dialog result | Movies & Credits sub-menu | Yes |
| `5` | `OptionsClass__ShowLauncherDialog()` | `0x12` (main menu) | Options dialog | Yes |
| `6` | `FUN_005D3490` — CSF message-box quit confirm | `7` (affirm) or `0x12` | Quit confirm dialog | Yes |
| `7` | `FUN_00720EA0(1)` music stop + fade | `return 0` | Screen fade + process exit | Yes |
| `8` | `FUN_00622650(0)` — pre-scenario/briefing dialog | scenario params or `iVar11=1` | Mission briefing | Yes |
| `9` | `LoadOptionsClass::Constructor` + load picker | `bVar2=true` → launch or `iVar11=1` | Load Saved Game | Yes |
| `10` | (no-op) | `iVar11=1` | Cancel/back sentinel | Yes (narrow) |
| `0xB` | sets `g_GameMode=5`, falls to 0x10 | 0x10 block | Post-disconnect recovery | Conditional |
| `0x10/0x11` | LAN/WOL/Skirmish/Modem lobby dispatcher | varies | Multiplayer lobby | Partial |
| `0xD` | `FUN_005BED40` movie play | `iVar11=4` | Intro cinematic then Movies | Yes |
| `0xE` | `FUN_0060D380` pick-movie + CD check + `FUN_005BED40` | `iVar11=4` | Pick and play movie | Conditional |
| `0xF` | music track via `FUN_00720B20` | `iVar11=4` | Audio credits track | Yes |
| `-1` | replay check; loads replay or back to menu | `0x12` or launch | Post-game / replay | Yes |

---

## Scenario Launch Path (post-switch)

After the switch, if `bVar2 == true` or `DAT_00a8b8b8 != 0`, execution reaches
`switchD_0052e356_caseD_1` which calls:

```
FUN_0054F720()
FUN_0052FC20()
FUN_006370B0()
// ... terrain, display chain, surface setup ...
return 1;   // game scenario ran; outer loop re-enters Main_Game
```

Otherwise the path enters `ScenarioClass__Start_Scenario(scenario_id)` to load
and run the scenario. On success the loop continues; on map-editor return it
returns 0.

---

## Open Questions (for other slots / follow-up)

1. **`FUN_0060D380` sub-dialog IDs**: Which specific dialog resource IDs does it
   dispatch for case 1 (SP) vs case 4 (Movies)? The dialog runner uses
   `FUN_00622650(0)` which loads a dialog by template; the template selection
   is controlled by the caller context not visible in this decompilation.
2. **`OptionsClass__ShowLauncherDialog` body (case 5)**: Not decompiled in this
   session. Needs its own slot.
3. **`FUN_0077B2A0` WOL inner state machine (case 0x10 g_GameMode=4)**: The WOL
   loop has a complex inner `DAT_00a8b244` sub-state switch (0–6). Full
   documentation deferred.
4. **`FUN_006AE2C0` Skirmish dialog (case 0x10 g_GameMode=5)**: Return codes
   `0x617` and `0x5C0` are confirmed; full option layout (map/color/house pickers)
   not documented.
5. **Case 0xB trigger frequency**: Only reached from WOL `FUN_0077B2A0` returning
   code 2. WOL offline means this fires zero times in standard play.
6. **`FUN_00473C50` / `FUN_00473D10` replay system**: Not decompiled; replay
   mechanics deferred.
7. **`DAT_00a8eb64` sub-cases in case 8**: The inner `switch(DAT_00a8eb64)`
   sub-cases 0–4 set `g_ScenarioClass_Instance+0x610` and `+0x60c` for
   scenario type (campaign/skirmish/multiplayer). Not fully mapped.

---

## Confidence Summary

All findings derive from `decompile_function 0x0052D9A0` (the full function body,
obtained in one call; the switch structure is fully visible in the output).
Supporting function identities confirmed via `decompile_function` or
`get_function_by_address` for each cited address. No addresses are invented.

Overall confidence: **HIGH** for the outer switch structure and case enumeration.
**MEDIUM** for inner dialog body details of cases 4, 9, 0xE, 0xF where the
dispatched function bodies were not independently decompiled in this session.
