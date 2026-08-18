# FUN_006040B0 — Global Tooltip Text Dispatcher

## Summary

Central tooltip string dispatcher for all YR dialogs. Takes an HWND (dialog or
control) and a control HWND. Looks up the dialog's registration entry in the
owner-draw combo hash table at `DAT_00AC1B00` to read the dialog ID (`iVar4 =
piVar3[0x1C]`), then calls `GetDlgCtrlID(param_2)` to get the control ID. Dispatches
on `(dialog_id, control_id)` pairs and returns a pointer to the corresponding
tooltip `STT_*` string. Returns `NULL` for unknown control/dialog combinations.
Called from `FUN_00604060` and `FUN_00622B50` (the shared WM_NOTIFY/tooltip helper).

## Address

`0x006040B0` (verified via `decompile_function 0x006040B0`)

## Active in YR

**Yes.** Called from `FUN_00622B50` (0x00622B50), which is the shared WM_NOTIFY
handler confirmed called from the DlgProc chain for all YR dialogs.

(confirmed via `get_function_callers 0x006040B0`)

## Signature / Parameters

```c
char * __fastcall FUN_006040b0(int param_1, HWND param_2)
// param_1 = dialog HWND (used to look up dialog ID via owner-draw hash table)
// param_2 = control HWND (control being hovered for tooltip)
// returns: pointer to STT_* tooltip string, or NULL if unknown
```

(verified via `decompile_function 0x006040B0`)

## Behavioral Analysis

### Dialog ID lookup

```c
if (DAT_00AC1B04 != 0) {
    // hash table lookup on DAT_00AC1B00 keyed by param_1
    iVar4 = piVar3[0x1C];  // dialog ID field at +0x70 (index 0x1C * 4)
    goto LAB_006040FC;
}
iVar4 = 0;
LAB_006040FC:
iVar2 = GetDlgCtrlID(param_2);
if (iVar2 == 0 || iVar2 == -1) return NULL;
```

`piVar3[0x1C]` reads the dialog template ID from the owner-draw registration
struct at offset `0x70`. For dialog 0x102 (offline Skirmish), `iVar4 == 0x102`.

(verified via `decompile_function 0x006040B0`)

### Dialog 0x102 (Skirmish) control ID → tooltip string mapping

```
control IDs → STT string name
```

| Control ID(s) | STT string |
|---|---|
| 0x6A0 | `STT_SkirmishEditPlayer` |
| 0x50B, 0x50E, 0x516, 0x51A, 0x51B, 0x51C, 0x51D | `STT_SkirmishComboAIPlayer` |
| 0x6DA..0x6E1 | `STT_SkirmishPictureFlag` |
| 0x6A1, 0x510, 0x513, 0x51E, 0x514, 0x51F, 0x520, 0x521 | `STT_SkirmishComboCountry` |
| 0x6A2, 0x522..0x528 | `STT_SkirmishComboColor` |
| 0x6A3..0x6A8, 0x6AA, 0x6AB | `STT_HostComboStart` (label: `STT_HostComboStart`) |
| 0x76D..0x774 | `STT_HostComboTeam` |
| 0x529 | `STT_SkirmishSliderSpeed` |
| 0x511 | `STT_SkirmishSliderCredits` |
| 0x50C | `STT_SkirmishSliderUnit` |
| 0x5AA | `STT_SkirmishButtonChooseMap` |
| 0x693 | `STT_SkirmishCBoxRedeploys` |
| 0x54E | `STT_SkirmishCBoxShortGame` |
| 0x69A | `STT_SkirmishCBoxSWAllowed` |
| 0x69D | `STT_SkirmishCBoxBuildOffAlly` |
| 0x696 | `STT_SkirmishCBoxCrates` |
| 0x468 | `STT_SkirmishMapThumbnail` |
| 0x6EC | `STT_SkirmishLabelGameType` |
| 0x5A8 | `STT_SkirmishLabelScenario` |
| 0x617 | `STT_SkirmishButtonStartGame` |
| 0x5C0 | `STT_SkirmishButtonBack` |
| Other | NULL |

(verified via `decompile_function 0x006040B0`)

### Other dialog ID cases (out of scope for this task)

The function handles many other dialog IDs in a large switch:
- `0xE2` — Main menu
- `0x100` — Single-player menu
- `0x94` — Campaign screen
- `0xB7` — Load game
- `0xBB` — Network lobby
- `0xBC`, `0xC2` — Host/join screens
- (and many more)

These are outside the cell-UI scope for this decode.

## Cross-Reference with Control ID Tables

The control ID sets in the dialog 0x102 case match exactly the canonical ID
tables from the other decoded functions:
- AI-type combos: `0x50B..0x51D` = `FUN_006ADC20` set ✓
- Flag statics: `0x6DA..0x6E1` = `FUN_004E3320` output ✓
- Country combos: `0x6A1, 0x510..0x521` = `FUN_004E37D0` output ✓
- Color combos: `0x6A2, 0x522..0x528` = `FUN_004E41D0` output ✓
- Start-pos combos: `0x6A3..0x6AB` = `FUN_004E4E60` output ✓
- Team combos: `0x76D..0x774` = `FUN_004E5940` output ✓

## Globals referenced

| Global | Address | Role |
|--------|---------|------|
| `DAT_00AC1B00` | 0x00AC1B00 | Owner-draw combo hash table (dialog registration lookup) |
| `DAT_00AC1B04` | 0x00AC1B04 | Hash table count (guard for lookup) |
| `DAT_00AC1B0C` | 0x00AC1B0C | Hash bucket shift |
| `DAT_00AC1B18` | 0x00AC1B18 | Hash function pointer |

## Callers

| Address | Name | Role |
|---------|------|------|
| 0x00604060 | FUN_00604060 | Tooltip outer handler |
| 0x00622B50 | FUN_00622B50 | Shared WM_NOTIFY handler (called from all DlgProcs) |

(confirmed via `get_function_callers 0x006040B0`)

## Callees

| Address | Name | Role |
|---------|------|------|
| Win32 | GetDlgCtrlID | Get control ID from HWND |

(confirmed via `get_function_callees 0x006040B0`)

## Out-of-scope refs

- `FUN_00604060` — tooltip outer handler; not decoded here
- `FUN_00622B50` — shared WM_NOTIFY handler; confirmed as caller
- Non-Skirmish dialog cases (`0xE2`, `0x100`, etc.) — out of cell-UI scope

## TS-filter

Active in YR for dialog 0x102. The function handles many other dialogs too —
none are TS-only for the Skirmish path. **TS-legacy score: 0.0.**

## Unverified (YELLOW)

- `piVar3[0x1C]` as dialog ID: inferred from the `0x102` case matching the
  Skirmish dialog and cross-referencing with control IDs; the field name
  "dialog ID" at `+0x70` of the registration struct is inferred, not verified
  from the struct definition.
- `STT_HostComboStart` and `STT_HostComboTeam` string names for start-position
  and team combos: these string variable names are from Ghidra's data labels;
  the names suggest they were originally in the network Host dialog but are
  reused in the Skirmish dialog (same control IDs). The string text content was
  not read from `GDlgSupp.csf` in this session.
