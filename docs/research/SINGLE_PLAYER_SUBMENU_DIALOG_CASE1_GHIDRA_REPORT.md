# Single Player Sub-Menu Dialog (Main_Game case 1) — Ghidra Research Report

Date: 2026-05-19
Scope: Dialog 0x100, the sub-menu reached when `Main_Game` case 1 fires
(user clicks Single Player on the main menu, dialog 0xE2 returns WM_COMMAND code 1,
`FUN_00531CC0` returns 1, `Main_Game` switch case 1 dispatches).

No Rust code, INI files, or Ghidra annotations were modified.
All facts verified via direct Ghidra MCP read-only calls as cited.

---

## Entry Point

`Main_Game @ 0x0052D9A0`, case 1 in the switch at `0x0052DCEA`:

```asm
; 0x0052DD39
PUSH  0x1              ; param_1 = 1 (triggers EVA welcome-back audio)
MOV   EDX, 0x0052D640 ; dialog proc address
MOV   ECX, 0x100      ; RT_DIALOG resource ID
MOV   [DAT_00AC10C8], EBX   ; clear SP session flag
CALL  0x0060D380       ; shell dialog runner
MOV   ESI, EAX         ; ESI = return code -> new iVar11
```

Verified via: `disassemble_function 0x0052D9A0` (confirmed bytes at 0x52DD39–0x52DD50).

`FUN_0060D380 @ 0x0060D380` creates the dialog, runs the message/paint pump
(`Process_NetworkMessages`, `FUN_0055CBSF0`, `Main_Tick`), and returns the code
placed in `local_4` by the dialog proc (via `SetWindowLongA(hWnd, 8, &local_4)`).
Verified via: `decompile_function 0x0060D380`.

---

## (a) Sub-Menu Dialog — ID, Proc Address, WM_INITDIALOG

| Item | Value | Verification |
|---|---|---|
| RT_DIALOG resource ID | **0x100** | Assembly at 0x52DD40: `MOV ECX, 0x100` |
| Dialog proc address | **0x0052D640** | Assembly at 0x52DD3B: `MOV EDX, 0x52D640` |
| WM_INITDIALOG handler | **Not present** (see notes) | `read_memory 0x0052D640 512`: no `CMP EDI/msg, 0x110` in proc |

**WM_INITDIALOG notes:** The dialog proc at 0x0052D640 handles three message classes:

| Message | Value | Handling |
|---|---|---|
| WM_PAINT | 0x0F | Custom rendering (checked first) |
| WM_COMMAND | 0x111 | Button dispatch (primary handler — see section c) |
| Custom init | 0x497 | Button label initialization (see below) |
| All others | — | Fall to `EndDialog(hWnd, 0)` / default |

WM_INITDIALOG (0x110) is **not explicitly handled** by this proc. The dialog is
created modeless via `CreateDialogIndirectParamA`; WM_INITDIALOG is delivered and
DefWindowProc returns FALSE (standard for modeless dialogs).

The custom **0x497 message** is sent by `FUN_005D4E70` (called from `FUN_00622650`
immediately after CreateDialogIndirectParamA). This is the game's own
button-initialization protocol. Its handler in this proc does:
1. `GetDlgItem(hDlg, 0x689)` → get LoadSavedGame button HWND
2. Calls two label-setup helpers (`FUN_00558736`, `FUN_00559C15`) to assign
   text strings to the shell button controls

Verified via: `read_memory 0x0052D640 512` + byte-by-byte decode;
`disassemble_function 0x0052D640` (triggered disassembly of 512 bytes).

---

## (b) Visible Child Controls of Dialog 0x100

Source: `FUN_006040B0 @ 0x006040B0` — the tooltip/hover-string lookup function.
For dialog ID 0x100 it maps each control ID to an `STT:` (string-table) key.
Verified via: `decompile_function 0x006040B0` (output confirmed in decompile result).

| Control ID | STT String | String Address | Meaning |
|---|---|---|---|
| **0x688** | `STT:SingleButtonNewCampaign` | 0x008356EC | Campaign button |
| **0x689** | `STT:SingleButtonLoadSavedGame` | 0x008356CC | Load Saved Game button |
| **0x579** | `STT:SingleButtonSkirmish` | 0x008356B0 | Skirmish button |
| **0x686** | `STT:SingleButtonBack` | 0x00835698 | Back / Cancel button |

No Tutorial button appears in dialog 0x100. The `STT:CampaignAnimTutorial` string
(0x0083567C) is referenced from dialog **0x94** (Campaign selector), not 0x100.
This confirms Tutorial is NOT a direct button on the Single Player sub-menu.

Active in YR: Yes — all four controls are live buttons.

**TS risk:** None observed. No flag-gated or conditional-presence logic in the
tooltip function for these controls.

---

## (c) WM_COMMAND Dispatch — Button Return Codes

The WM_COMMAND handler at **0x0052D6DF** (jumped to from 0x52D67B when uMsg == 0x111)
decodes `wParam & 0xFFFF` (control ID) and stores a return code into `local_4` via
`MOV [EAX], imm32`. The return code becomes `FUN_0060D380`'s return value → new
`iVar11` in `Main_Game`.

Full decode verified via: `read_memory 0x0052D640 512` + Python byte-trace
(addresses and jump targets confirmed by computation).

| Button | Control ID | Return Code | Main_Game Case | Player-Visible Effect |
|---|---|---|---|---|
| Campaign | 0x688 | **0x08** (8) | case 8 | Opens Campaign selector dialog 0x94 (Allied/Soviet picker) |
| Load Saved Game | 0x689 | **0x09** (9) | case 9 | Opens `LoadOptionsClass` saved-game picker |
| Skirmish | 0x579 | **0x0B** (11) | case 0xB | Sets `g_GameMode = 5`; falls through to case 0x10 → skirmish setup |
| Back | 0x686 | **0x12** (18) | case 0x12 | Returns to main menu (`FUN_00531CC0()` called again) |

Control 0x68A (no tooltip entry) → return **0x0A** (10) → case 10 → `iVar11 = 1`
(loops back to Single Player sub-menu). This appears to be an internal
cancel/close-without-action path, not a visible button.

### Assembly Evidence (selected key bytes)

```
; WM_COMMAND handler entry 0x0052D6DF
MOV  ECX, EBX           ; EBX = wParam
AND  ECX, 0x0000FFFF    ; isolate control ID
CMP  ECX, 0x688         ; NewCampaign
JG   0x0052D734         ; if > 0x688 (LoadSavedGame / higher)
JZ   0x0052D723         ; if == 0x688 -> campaign handler
CMP  ECX, 0x579         ; Skirmish
JZ   0x0052D712         ; if == 0x579 -> skirmish handler
CMP  ECX, 0x686         ; Back
JNZ  0x0052D77D         ; if != 0x686 -> unhandled (EndDialog 0)
; fall-through: ECX == 0x686 (Back)
; 0x0052D701:
POP  EDI
MOV  [EAX], 0x12        ; return code = 0x12 (Back)
...
; 0x0052D712: (Skirmish target)
POP  EDI
MOV  [EAX], 0x0B        ; return code = 0x0B
...
; 0x0052D723: (NewCampaign target)
POP  EDI
MOV  [EAX], 0x08        ; return code = 0x08
...
; 0x0052D734: (JG path — ECX > 0x688)
SUB  ECX, 0x689         ; check for 0x689 (LoadSavedGame)
JZ   0x0052D750         ; if == 0x689
DEC  ECX                ; check for 0x68A
JNZ  0x0052D77D         ; if not 0x68A -> unhandled
MOV  [EAX], 0x0A        ; return code = 0x0A (0x68A internal)
...
; 0x0052D750: (LoadSavedGame target)
POP  EDI
MOV  [EAX], 0x09        ; return code = 0x09 (LoadSavedGame)
```

---

## Main_Game Case Meanings (cross-reference)

From `decompile_function 0x0052D9A0`:

| Return Code | Case | Action |
|---|---|---|
| 0x08 (8) | case 8 | Creates dialog **0x94** (Campaign selector, proc 0x0052EC00) via `FUN_00622650(0x94)`. The Campaign selector shows Allied/Soviet icons (controls 0x6EA, 0x6EC) and difficulty slider (0x50F). |
| 0x09 (9) | case 9 | `g_MapEditorMode++; CDFileClass__Constructor(); LoadOptionsClass::Constructor(); FUN_005587F0()` — Load saved game picker |
| 0x0A (10) | case 10 | `iVar11 = 1; break` — loops back to Single Player sub-menu (re-fires case 1) |
| 0x0B (11) | case 0xB | `g_GameMode = 5;` then falls through to case 0x10 → `FUN_006AE2C0()` (Skirmish lobby/setup) |
| 0x12 (18) | case 0x12 | `break` → loops back to `LAB_0052DC40` → calls `FUN_00531CC0()` again (returns to main menu) |

Verified via: `decompile_function 0x0052D9A0` (full function body).

---

## Active in YR

All four visible buttons (Campaign, Load Saved Game, Skirmish, Back) are **Active in YR: Yes**.
No TS-gating flags found in this dialog proc.

- Campaign path (case 8 → dialog 0x94) is live.
- Load Saved Game (case 9) is live.
- Skirmish (case 0xB → `g_GameMode=5`) is live.
- Back (case 0x12 → main menu) is live.

Tutorial: **Not present** in dialog 0x100. The STT string `STT:CampaignAnimTutorial`
exists in the binary (0x0083567C) and is referenced from dialog 0x94 (Campaign selector),
suggesting Tutorial may have been a Campaign sub-option or is a TS-era control not
exposed in YR's single-player menu. Flagged as Open Question.

---

## Open Questions (for follow-up investigations)

1. **Tutorial**: Where is Tutorial accessible in YR? `STT:CampaignAnimTutorial` is in
   dialog 0x94 (control 0x6EB). Is it a clickable icon in the Campaign selector, or
   a TS-legacy dormant control? Requires investigation of dialog 0x94 proc (0x0052EC00).

2. **Dialog 0x94 (Campaign selector)**: Full button layout not investigated here
   (out of scope). Controls seen: 0x6EA (Allied), 0x6EB (Tutorial anim?), 0x6EC (Soviet),
   0x50F (?), 0x40E (Load button), 0x455 (?), 0x686 (Back). Proc at 0x0052EC00.

3. **Dialog proc 0x0052D640 WM_PAINT handler**: What does the custom WM_PAINT
   (0x0F) branch render? Not decoded — out of scope.

4. **Case 0xE (CD check)** at 0x0052DE72: Uses dialog **0x129**, proc 0x0052D870.
   Related to physical CD detection before campaign launch. Not investigated.

5. **Control 0x68A**: Why does the dialog proc handle this unmapped ID? Likely the
   dialog-frame close button (WM_SYSCOMMAND / ESC mapped to this ID internally). Not
   confirmed.

---

## Confidence Summary

| Claim | Level | Evidence |
|---|---|---|
| Dialog ID = 0x100 | HIGH | Assembly `MOV ECX, 0x100` at 0x52DD40, confirmed via `disassemble_function 0x0052D9A0` |
| Dialog proc = 0x0052D640 | HIGH | Assembly `MOV EDX, 0x52D640` at 0x52DD3B |
| Button 0x688 = Campaign → return 0x08 | HIGH | Byte trace in dialog proc; `JZ 0x52D723` + `MOV [EAX], 0x08` at 0x52D724 |
| Button 0x689 = LoadSaved → return 0x09 | HIGH | Byte trace; `SUB ECX, 0x689; JZ 0x52D750` + `MOV [EAX], 0x09` at 0x52D751 |
| Button 0x579 = Skirmish → return 0x0B | HIGH | Byte trace; `CMP ECX, 0x579; JZ 0x52D712` + `MOV [EAX], 0x0B` at 0x52D713 |
| Button 0x686 = Back → return 0x12 | HIGH | Byte trace; fall-through after `CMP ECX, 0x686` + `MOV [EAX], 0x12` at 0x52D702 |
| No Tutorial button in dialog 0x100 | HIGH | FUN_006040B0 lists exactly 4 controls for iVar4==0x100; decompile confirmed |
| Tutorial in dialog 0x94 (not 0x100) | MEDIUM | STT string `CampaignAnimTutorial` found at 0x83567C, xref from FUN_006040B0 in dialog 0x94 section |
