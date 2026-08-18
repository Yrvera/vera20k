# FUN_006ACD60 — Team-Control Enable Refresh

## Summary

`FUN_006ACD60` refreshes the enabled/disabled state of all 8 team combo controls in the Skirmish dialog. It reads the `AlliesAllowed` flag from the selected-mode object (`DAT_00a8b23c + 0x3C`), then for each of the 7 AI-type combo slots (slots 0-6) reads the slot's current AI-type selection. If AlliesAllowed is false, all team combos are disabled. If AlliesAllowed is true, team combos for active rows (AI-type item-data != -1) are enabled; team combos for closed rows (item-data == -1) are disabled. Slot 0's team combo (control 0x76D) is handled as a special case outside the loop — it is enabled/disabled based on AlliesAllowed alone. Callers: `FUN_006ACEE0` (on mode-change WM_COMMAND) and `FUN_006AE6E0` (at dialog init).

## Active in YR

**Yes.** Callers confirmed via `get_function_callers 0x006ACD60`: `FUN_006ACEE0` (the WM_COMMAND dispatcher) and `FUN_006AE6E0` (the dialog init handler). Both are in the dialog-0x102 YR offline-Skirmish path. No TS-only gate present.

## Decompilation excerpt (verified via `decompile_function 0x006ACD60`)

```c
void FUN_006acd60(HWND param_1)
{
    // If no dialog HWND passed, get the active window
    if (param_1 == NULL) param_1 = (HWND)FUN_00775940();

    // Read AlliesAllowed flag from selected-mode object at DAT_00a8b23c + 0x3C
    char local_1 = '\0';
    if (DAT_00a8b23c != 0) {
        local_1 = *(char *)(DAT_00a8b23c + 0x3c);
    }

    // Slot 0: enable/disable team combo 0x76D based on AlliesAllowed only
    HWND pHVar1 = GetDlgItem(param_1, 0x76d);
    if (pHVar1 != NULL) {
        EnableWindow(pHVar1, (uint)(local_1 != '\0'));

        // Slots 0-6: enable team combos for active rows if AlliesAllowed
        int iVar4 = 0;
        do {
            // AI-type combo ID map (same as init: 0→0x50B, 1→0x50E, ..., 6→0x51D)
            WPARAM wParam = SendDlgItemMessageA(param_1, iVar3, 0x147, 0, 0); // CB_GETCURSEL
            LRESULT LVar2 = SendDlgItemMessageA(param_1, iVar3, 0x150, wParam, 0); // CB_GETITEMDATA

            // Team combo ID for this slot
            int teamID = FUN_004e5940(iVar4);
            if ((teamID >= 0) && (GetDlgItem(param_1, teamID) != NULL)) {
                BOOL bEnable;
                if (LVar2 == -1)         bEnable = 0;  // row closed
                else if (local_1 == '\0') bEnable = 0;  // AlliesAllowed off
                else                      bEnable = 1;  // active row + AlliesAllowed
                EnableWindow(GetDlgItem(param_1, teamID), bEnable);
            }
            iVar4++;
        } while (iVar4 + 1 < 8);  // iVar3 = iVar4 + 2; loop while iVar3 < 8
    }
}
```

## Behavioral analysis

### AlliesAllowed gating

The function reads `*(char *)(DAT_00a8b23c + 0x3C)` — byte field at `+0x3C` of the selected-mode object. This is the `AlliesAllowed` flag. If the global pointer `DAT_00A8B23C` is null, `local_1` stays `'\0'` (false). This means:
- When no selected mode is set (null pointer): all team combos are disabled
- When selected mode has AlliesAllowed=false: all team combos are disabled
- When selected mode has AlliesAllowed=true: team combos for active rows are enabled

### Per-slot logic

For each slot `iVar4` from 0 to 6:
1. Get the AI-type combo ID for this slot (same ID map as init: 0→0x50B..6→0x51D)
2. Read current selection via `CB_GETCURSEL` (0x147) then `CB_GETITEMDATA` (0x150)
3. Get the team combo ID for this slot via `FUN_004e5940(iVar4)` (returns 0x76D..0x773)
4. Enable/disable based on: active slot (item-data != -1) AND AlliesAllowed

### Slot 0 special case

Slot 0's team combo (0x76D) is handled before the loop: `EnableWindow(GetDlgItem(param_1, 0x76D), (bool)local_1)`. This enables it regardless of AI-type selection (slot 0 is always the human player and never has item-data -1 in normal operation). Inside the loop, slot 0 (`iVar4==0`) also goes through the normal check — but the loop condition `while (iVar3 < 8)` where `iVar3 = iVar4 + 2` means the loop runs while `iVar4 + 2 < 8`, i.e., `iVar4 < 6`. Combined with starting at `iVar4 = 0`, slots 0-5 are checked inside the loop.

Verification: `iVar3 = iVar4 + 2; do {...; iVar3 = iVar4 + 2; iVar4 = iVar4 + 1; } while (iVar3 < 8)` → last iteration when `iVar4 = 5`, `iVar3 = 7 < 8`. At `iVar4 = 6`, `iVar3 = 8`, loop exits. So 6 iterations covering slots 0-5 inside the loop.

### Dialog handle fallback

If `param_1 == NULL`, `FUN_00775940()` is called to retrieve the active/top dialog HWND. This allows callers to pass `NULL` and let the function find the correct dialog.

### AI-type combo IDs in the loop (verified from decompile)

| iVar4 (slot) | AI combo ID |
|---|---|
| 0 | 0x50B |
| 1 | 0x50E |
| 2 | 0x516 |
| 3 | 0x51A |
| 4 | 0x51B |
| 5 | 0x51C |
| 6 | 0x51D (computed: `(-(uint)(iVar4 != 6) & 0xfffffae2) + 0x51d`) |

### Team combo IDs (via FUN_004e5940, verified via `decompile_function 0x004E5940`)

| Slot | Team combo ID |
|---|---|
| 0 | 0x76D |
| 1 | 0x76E |
| 2 | 0x76F |
| 3 | 0x770 |
| 4 | 0x771 |
| 5 | 0x772 |
| 6 | 0x773 |
| 7 | 0x774 |

## Struct field accesses

| Pointer | Offset | Unit | Usage | Frame |
|---------|--------|------|-------|-------|
| `DAT_00A8B23C` (selected-mode ptr) | `+0x3C` | byte | AlliesAllowed flag | internal selected-mode struct field |

## Globals referenced

| Global | Address | Role |
|--------|---------|------|
| `DAT_00A8B23C` | 0x00A8B23C | Selected-mode object pointer — read for AlliesAllowed at `+0x3C` |

## Callers

- `FUN_006ACEE0` @ 0x006ACEE0 — WM_COMMAND dispatcher (on mode change)
- `FUN_006AE6E0` @ 0x006AE6E0 — Dialog init handler

Confirmed via `get_function_callers 0x006ACD60`.

## Callees

| Address | Name | Role |
|---------|------|------|
| 0x00775940 | FUN_00775940 | Active dialog HWND accessor (fallback when param_1=NULL) |
| 0x004E5940 | FUN_004e5940 | Team combo ID mapper (slot → control ID) |
| Win32 | EnableWindow | Enable/disable team combo window |
| Win32 | GetDlgItem | Get control HWND by ID |
| Win32 | SendDlgItemMessageA | CB_GETCURSEL, CB_GETITEMDATA on AI-type combos |

## Out-of-scope refs

- `FUN_00775940` — active dialog accessor, shared utility out of cell scope

## TS-filter

Function is called only from the YR offline Skirmish dialog flow. No TS-only gating found. **TS-legacy score: 0.0.**

## Unverified claims (YELLOW)

- `DAT_00a8b23c + 0x3C` = AlliesAllowed byte — inferred from field name in the manifest and the enable-disable logic; the field name "AlliesAllowed" is not from a struct layout decode of the selected-mode class. The bit is confirmed to gate team-enable behavior.
- `FUN_00775940` semantic as "active dialog HWND accessor" — inferred from usage pattern (returns a HWND used as dialog parent); not independently decoded.
