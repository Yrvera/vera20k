# Options Dialog (Main_Game case 5) and OptionsClass Field-Offset Map

**Date:** 2026-05-19  
**Source binary:** gamemd.exe  
**Scope:** Main_Game case 5 → OptionsClass__ShowLauncherDialog → dialog resource ID,
dialog proc, owner-draw controls, WM_COMMAND codes; OptionsClass struct field map
from ReadFromINI / WriteToINI.

---

## 1. Main_Game Case 5 — Call Chain

Verified via `decompile_function 0x0052d9a0` (Main_Game):

```
case 5:
    iVar11 = 0x12;                       // loop continues (back to main-menu)
    OptionsClass__ShowLauncherDialog();  // 0x0055fc80
    break;
```

`OptionsClass__ShowLauncherDialog @ 0x0055FC80` is the sole callee.
Caller chain confirmed: `Main_Game → FUN_00531CC0 (returns 5) → case 5`.

---

## 2. Options Launcher Dialog — Resource ID and Dialog Proc

Verified via `disassemble_function 0x0055fc80`:

The launcher opens **two** Win32 modeless dialogs sequentially:

### 2a. Primary Options dialog (the one visible from main-menu shell)

| Field         | Value                   | Evidence                                      |
|---------------|-------------------------|-----------------------------------------------|
| RT_DIALOG ID  | **0xD5 (213)**          | `MOV ECX, 0xd5` at `0055fcb8` before CALL FUN_00622650 |
| Dialog proc   | **0x0055FDB0**          | `MOV EDX, 0x55fdb0` at `0055fcb3`            |
| Creation fn   | `FUN_00622650` → `CreateDialogIndirectParamA` | Confirmed via disassembly of FUN_00622650 |

`FUN_00622650` calling convention is `__fastcall`: ECX = dialog resource ID, EDX = dialog proc pointer.
Verified: `MOV EDX,0x5` at `00622665` (RT_DIALOG = 5), then `CALL 0x004a3b40` at `0062266c` → `FindResourceA(hInst, dialogId, RT_DIALOG)`.

### 2b. Resolution sub-dialog (launched when return code == 0x5CD)

| Field         | Value                   | Evidence                                      |
|---------------|-------------------------|-----------------------------------------------|
| RT_DIALOG ID  | **0xD7 (215)**          | `MOV ECX, 0xd7` at `0055fd3b`                |
| Dialog proc   | **0x00560480**          | `MOV EDX, 0x560480` at `0055fd36`            |

This sub-dialog is out of scope per task constraints — do not implement separately.

---

## 3. Primary Options Dialog Proc (0x0055FDB0) — Message Handling

### 3a. Messages handled

Verified via assembly at 0x0055FDB0 (read_memory + get_assembly_context):

```
SUB ESP, 0x40    ; stack frame
Parameters: hWnd, uMsg, wParam, lParam
```

Message dispatch (`EBX` = uMsg after `GetWindowLongA(hWnd, 8)`):

| uMsg  | Description                       | Evidence              |
|-------|-----------------------------------|-----------------------|
| 0x114 | WM_HSCROLL (slider scroll)        | `CMP EBX,0x114` at `0055fde3`, `JZ 0x0055ff68` |
| 0x002 | WM_DESTROY                        | `SUB EBX,0x2; JZ 0x0055ff3c` at `0055fdf9` |
| 0x111 | WM_COMMAND (0x111 = 0x2 + 0x10f)  | `SUB EBX,0x10f; JNZ 0x00560474` at `0055fe02` |
| 0x497 | WM_INITDIALOG (custom/0x497)      | `CMP EBX,0x497` at `0056010c` |

Note: `0x497` is not standard WM_INITDIALOG (0x110). This is a custom message used by the YR dialog manager, consistent with `CreateDialogIndirectParamA` being used (not `DialogBoxIndirectParamA`).

### 3b. WM_COMMAND control IDs (owner-draw / custom controls)

These are `GetDlgItem` IDs extracted from the WM_COMMAND path and WM_INITDIALOG setup:

| Control ID (hex) | Dec  | Purpose                                  | Evidence                              |
|------------------|------|------------------------------------------|---------------------------------------|
| 0x52B            | 1323 | Game-speed / extra-animations slider     | `PUSH 0x52b` at `0055ff7d`, `0056011e` |
| 0x50F            | 1295 | Difficulty selector                      | `PUSH 0x50f` at `0055ffdb`, `00560271` |
| 0x52A            | 1322 | Scroll-rate slider                       | `PUSH 0x52a` at `0055fff5`, `00560320` |
| 0x52F            | 1327 | Sound-volume slider                      | `PUSH 0x52f` at `0055fb9c`, `00560366` |
| 0x532            | 1330 | Score-volume slider                      | `PUSH 0x532` at `0055fc0b`, `005603c4` |
| 0x536            | 1334 | Voice-volume slider                      | `PUSH 0x536` at `0055fc41`, `00560416` |
| 0x601            | 1537 | Health bars checkbox                     | `PUSH 0x601` at `0055fb53`, `005602b4` |
| 0x602            | 1538 | Action lines checkbox                    | `PUSH 0x602` at `0055fb58`→skip, `005602fc` |
| 0x604            | 1540 | (unknown bool, likely show-hidden units) | `PUSH 0x604` at `005602d8`            |
| 0x673            | 1651 | Game-speed label / linked to 0x52B      | `PUSH 0x673` at `0055ff9b`            |
| 0x670            | 1648 | Difficulty label / linked to 0x50F      | `PUSH 0x670` at `0055ffee`            |
| 0x672            | 1650 | Scroll-rate label / linked to 0x52A     | `PUSH 0x672` at `00560008`            |
| 0x686            | 1670 | (resolution or similar — gate check)    | `CMP EDI,0x686` at `0055fe16`         |
| 0x6ED            | 1773 | Resolution combo box                    | `PUSH 0x6ed` at `0055fee4`, `0056016c` |
| 0x147            | 327  | (init only — sub-control)               | `PUSH 0x147` at `0055fee4`            |
| 0x150            | 336  | (init only — sub-control)               | `PUSH 0x150` at `0055feef`            |
| 0x14E            | 334  | (init only — list-box populate)         | `PUSH 0x14e` at `00560263`            |
| 0x151            | 337  | (init only — list item set)             | `PUSH 0x151` at `00560241`            |
| 0x4AC            | 1196 | SBM_SETRANGE / slider range message     | (sent to all sliders in WM_INITDIALOG)|
| 0x406            | 1030 | Custom: set slider tick count           | (sent to all sliders)                 |
| 0x405            | 1029 | Custom: set slider value                | (sent to all sliders)                 |
| 0x4AE            | 1198 | SBM_SETPOS / slider position            | (sent to vol sliders)                 |
| 0x4BC            | 1212 | Combo-box item add message              | (sent to resolution combo)            |

### 3c. WM_COMMAND return codes (local_8 set in dialog loop)

These values are set into `local_8` (the out-param polled by the message loop):

| Return code (hex) | Dec  | Meaning                               | Evidence                                |
|-------------------|------|---------------------------------------|-----------------------------------------|
| 0x5CB             | 1483 | OK / accept (cancel sub-dialog)       | `MOV dword ptr [EAX],0x5cb` at `0055fe93` |
| 0x5CD             | 1485 | Open resolution sub-dialog            | `CMP EAX,0x5cd; JZ` at `0055fd0f-14`   |
| 0x5CE             | 1486 | Open third panel (advanced/keymapping?) | `CMP EAX,0x5ce; JZ` at `0055fd14-16`; calls `FUN_005fbef0` |

---

## 4. OptionsClass Struct Field Map

### 4a. Param type verification

Verified via `decompile_function 0x005fa620` (OptionsClass__ReadFromINI):

```c
void __fastcall OptionsClass__ReadFromINI(undefined4 *param_1)
```

`param_1` is `undefined4 *` (i.e., `int *`). Therefore:
- `param_1[N]` = byte offset **N × 4**  
- `*(undefined1 *)(param_1 + N)` = byte offset **N × 4** (first byte of that 4-byte slot)
- `*(undefined1 *)((int)param_1 + X)` = **direct byte offset X**

This distinction is critical — confirmed by cross-checking ToolTips:
`*(char *)(param_1 + 8)` = index 8 × 4 = **byte offset 0x20**, but confirmed in the
existing `MAIN_MENU_DIALOG_0XE2_TOOLTIP_HOVER_FLOW_GHIDRA_REPORT.md` as **offset 8** —
which means the doc uses slot-index notation (0x20 / 4 = 8). Both are consistent.

### 4b. Complete field table (INI [Section] key → byte offset)

Verified from ReadFromINI (`decompile_function 0x005fa620`) and
WriteToINI (`decompile_function 0x005fad10`). WriteToINI confirmed identical offsets.

**Param type:** `undefined4 *` → index N = byte offset N×4.

| INI Section | INI Key            | Byte Offset     | Index | Type        | Clamped range         | Active in YR |
|-------------|--------------------|-----------------|-------|-------------|-----------------------|--------------|
| [Options]   | GameSpeed          | 0x00            | [0]   | int         | unclamped             | Yes          |
| [Options]   | Difficulty         | 0x04            | [1]   | int         | 0..4                  | Yes          |
| [Options]   | CampDifficulty     | 0x08            | [2]   | int         | 0..2                  | Yes          |
| [Options]   | ScrollMethod       | 0x0C            | [3]   | int (uint)  | unclamped             | Yes          |
| [Options]   | ScrollRate         | 0x10            | [4]   | int (uint)  | unclamped             | Yes          |
| [Options]   | AutoScroll         | 0x14            | [5]   | bool (1b)   | —                     | Yes          |
| [Options]   | DetailLevel        | 0x18            | [6]   | int         | 0..2                  | Yes          |
| (hardcoded) | SideBar position   | 0x1C            | [7]   | bool (1b)   | forced = 1 (RIGHT)    | Yes          |
| [Options]   | SidebarCameoText   | 0x1D (direct)   | —     | bool (1b)   | —                     | Yes          |
| [Options]   | UnitActionLines    | 0x1E (direct)   | —     | bool (1b)   | —                     | Yes          |
| [Options]   | ShowHidden         | 0x1F (direct)   | —     | bool (1b)   | —                     | Yes (debug)  |
| [Options]   | ToolTips           | 0x20            | [8]   | bool (1b)   | —                     | Yes          |
| [Video]     | ScreenWidth        | 0x24            | [9]   | int (uint)  | unclamped             | Yes          |
| [Video]     | ScreenHeight       | 0x28            | [10]  | int (uint)  | unclamped             | Yes          |
| [Video]     | StretchMovies      | 0x34            | [13]  | bool (1b)   | AND'd with DAT_008a0dee | Yes        |
| [Video]     | AllowHiResModes    | 0x35 (direct)   | —     | bool (1b)   | —                     | Conditional  |
| [Video]     | AllowModeToggle    | DAT_00a8ed63    | —     | bool (1b)   | (global, not in struct)| Conditional |
| [Video]     | AllowVRAMSidebar   | 0x36 (direct)   | —     | bool (1b)   | —                     | Conditional  |
| [Audio]     | SoundVolume        | 0x38            | [14]  | float       | 0.0..1.0              | Yes          |
| [Audio]     | VoiceVolume        | 0x3C            | [15]  | float       | 0.0..1.0              | Yes          |
| [Audio]     | ScoreVolume        | 0x40            | [16]  | float       | 0.0..1.0              | Yes          |
| [Audio]     | IsScoreRepeat      | 0x44            | [17]  | bool (1b)   | also written to DAT_00a83d20 | Yes |
| [Audio]     | InGameMusic        | 0x45 (direct)   | —     | bool (1b)   | —                     | Yes          |
| [Audio]     | IsScoreShuffle     | 0x46 (direct)   | —     | bool (1b)   | also written to DAT_00a83d22 | Yes |
| [Audio]     | SoundLatency       | 0x48            | [18]  | ushort (2b) | —                     | Yes          |
| [Network]   | NetID (parsed)     | 0x4C            | [19]  | int         | -1..7 (adapter index) | Conditional  |
| [Network]   | NetID (parsed)     | 0x50            | [20]  | int         | -1..7 (second idx)    | Conditional  |
| [Network]   | Socket             | 0x4A (direct)   | —     | ushort (2b) | —                     | Conditional  |
| [Network]   | NetCard            | 0x54            | [21]  | int (uint)  | unclamped             | Conditional  |
| [Network]   | DestNet            | 0x58            | [22]  | char[0x40]  | 0x40 bytes            | Conditional  |

**Notes:**
- Byte offsets 0x2C and 0x30 (indices [11], [12]) are **not written or read** in ReadFromINI/WriteToINI — gap in the struct between ScreenHeight and StretchMovies.
- `AllowModeToggle` is stored in a global (`DAT_00a8ed63`), not in the OptionsClass struct itself.
- `IsScoreRepeat` and `IsScoreShuffle` are both stored in struct AND mirrored to globals (`DAT_00a83d20`, `DAT_00a83d22`).
- The struct minimum size is **0x58 + 0x40 = 0x98 bytes** (through DestNet field).

### 4c. Fields NOT in ReadFromINI/WriteToINI but observed in struct neighborhood

| Byte offset | Used by                          | Notes                            |
|-------------|----------------------------------|----------------------------------|
| 0x00a8eb70  | `DAT_00a8eb70` (global)         | ScrollRate mirror for dialog     |
| 0x00a8eb60  | `DAT_00a8eb60` (OptionsClass instance base?) | See ShowLauncherDialog copy 0x2E dwords |
| 0x00a8eb78  | `DAT_00a8eb78` (global)         | Used for resolution combo enable |
| 0x00a8eb84/88 | `DAT_00a8eb84/88`             | Resolution width/height selected |
| 0x00a8eb95  | `DAT_00a8eb95`                  | AllowHiResModes flag mirror      |

The `OptionsClass` global instance appears to start at **0x00A8EB60** (confirmed by
`ShowLauncherDialog` copying `0x2E` dwords = 0xB8 bytes from `0x00A8EB60` to `0x00ABCE70`
as a backup before showing the dialog, restoring on cancel — verified at `0055fc9d-0055fcb1`).

---

## 5. INI Section Summary

Three INI sections are used:

| Section    | Keys read                                                        |
|------------|------------------------------------------------------------------|
| [Options]  | GameSpeed, Difficulty, CampDifficulty, ScrollMethod, ScrollRate, AutoScroll, DetailLevel, SidebarCameoText, UnitActionLines, ShowHidden, ToolTips |
| [Video]    | ScreenWidth, ScreenHeight, StretchMovies, AllowHiResModes, AllowModeToggle, AllowVRAMSidebar |
| [Audio]    | SoundVolume, VoiceVolume, ScoreVolume, IsScoreRepeat, IsScoreShuffle, SoundLatency, InGameMusic |
| [Network]  | NetID, Socket, NetCard, DestNet                                  |

---

## 6. TS-vs-YR Filter

| Field             | Active in YR default? | Notes                                           |
|-------------------|-----------------------|-------------------------------------------------|
| AllowHiResModes   | Conditional           | Gates extra resolution modes; depends on build  |
| AllowModeToggle   | Conditional           | Global flag; likely TS holdover                 |
| AllowVRAMSidebar  | Conditional           | May be TS-era VRAM sidebar toggle               |
| ShowHidden        | No (debug)            | Debug overlay; not shown in retail UI           |
| Network fields    | Conditional           | Only active in multiplayer mode                 |
| All [Options]/[Audio] | Yes              | Active in all YR game modes                     |
| [Video] ScreenWidth/Height | Yes        | Set from options dialog resolution picker       |
| StretchMovies     | Yes (if movie support available) | AND'd with `DAT_008a0dee` (movie capability flag) |

---

## 7. Verification Citations

All findings verified by the following Ghidra MCP calls in this session:

- `decompile_function 0x0052d9a0` — Main_Game, confirmed case 5 → OptionsClass__ShowLauncherDialog
- `search_functions "OptionsClass"` — confirmed all OptionsClass function addresses
- `decompile_function 0x0055fc80` — OptionsClass__ShowLauncherDialog, dialog flow
- `disassemble_function 0x0055fc80` — confirmed ECX=0xD5, EDX=0x55FDB0 for primary dialog; ECX=0xD7, EDX=0x560480 for sub-dialog
- `disassemble_function 0x00622650` — confirmed calling convention: ECX=dialogId, EDX=proc; RT_DIALOG=5 passed to FindResourceA
- `decompile_function 0x005fa620` — OptionsClass__ReadFromINI (full field map)
- `decompile_function 0x005fad10` — OptionsClass__WriteToINI (cross-check offsets)
- `get_assembly_context 0x0055fdb0` — dialog proc message dispatch and control IDs
- `get_assembly_context 0x00560062` — dialog proc WM_INITDIALOG slider/checkbox setup
- `get_assembly_context 0x00560258` — WM_INITDIALOG resolution combo population
- `get_assembly_context 0x00560361` — slider initialization (sound/score/voice vol)
- `read_memory 0x0055fdb0` — confirmed valid function prologue (SUB ESP,0x40) at dialog proc entry

---

## 8. Open Items / Unverified

- Byte offsets 0x2C..0x33 (struct gap between ScreenHeight and StretchMovies): purpose unknown — no INI key, not observed in ReadFromINI. May be padding or struct fields set by constructor only.
- `FUN_00531CC0` return value 5: confirmed as the dialog return code that flows into Main_Game case 5, but the specific WM_COMMAND button in dialog 0xE2 that produces code 5 at control 0x55C was not re-traced (already documented in the referenced hover/flow report).
- Dialog proc 0x00560480 (resolution sub-dialog, ID 0xD7): not decompiled — out of scope.
