# FUN_006AE080 — Hide AI Rows Beyond Map Start Count

## Summary

`FUN_006AE080` hides all player-slot rows (rows `param_2` through 7) when the
selected map's start-position count drops below the current row count. Called
when the map selector changes to a map with fewer start positions, this function
selects the "closed/sentinel" item-data in each excess AI-type combo, then calls
the per-row enable state machine, and finally calls `ShowWindow(..., 0)` on all
six cell controls in each excess row to make them invisible.

Active in YR: **Yes** — sole caller is `FUN_006ADDF0` (row show/hide adjuster
on map change), which is itself called from the WM_COMMAND handler
`FUN_006ACEE0` during map-selector change (ID 0x5AA). Active in every standard
YR skirmish session when a map with fewer start positions is selected.
(verified via `get_function_callers 0x006AE080`)

## Address

`0x006AE080`
(verified via `decompile_function 0x006AE080`)

## Signature / Parameters

```c
void __fastcall FUN_006ae080(
    HWND param_1,  // dialog HWND
    int  param_2   // first row index to hide (0-based); hides rows param_2..6 (7 slots total, 0-7 range)
)
```

Guard: `if (param_2 < 8)` — no-op if param_2 ≥ 8.
(verified via `decompile_function 0x006AE080`)

## Callers

- `FUN_006ADDF0` @ `0x006ADDF0` — row show/hide adjuster on map change.
  (verified via `get_function_callers 0x006AE080`)

## Callees

(verified via `get_function_callees 0x006AE080`)

| Function | Address | Role |
|---|---|---|
| `FUN_004E3320` | 0x004E3320 | Slot-0 flag/static control ID helper |
| `FUN_004E37D0` | 0x004E37D0 | Per-row helper — returns control ID |
| `FUN_004E41D0` | 0x004E41D0 | Per-row helper — returns control ID |
| `FUN_004E4E60` | 0x004E4E60 | Per-row helper — returns control ID |
| `FUN_004E5940` | 0x004E5940 | Per-row helper — returns control ID |
| `FUN_006ADC20` | 0x006ADC20 | Per-row enable state machine |
| `GetDlgItem` | EXTERNAL | Gets child HWND by control ID |
| `SendDlgItemMessageA` | EXTERNAL | Sends message to dialog item |
| `ShowWindow` | EXTERNAL | Shows/hides window |

## Behavioral Analysis

The function runs two sequential passes over the rows to hide (indices
`param_2 - 1` through 6, inclusive, i.e. `param_2` rows starting from the
excess boundary):

### Pass 1 — Sentinel selection in AI-type combo

For each row index `iVar4` from `param_2 - 1` to 6:

1. Map row index to AI-type combo control ID (same table as used in
   `FUN_006ACEE0`):

   | Row index | Control ID |
   |---|---|
   | 0 | 0x50B |
   | 1 | 0x50E |
   | 2 | 0x516 |
   | 3 | 0x51A |
   | 4 | 0x51B |
   | 5 | 0x51C |
   | 6 | 0x51D |

2. `SendDlgItemMessageA(param_1, ctrlId, CB_GETCOUNT=0x146, 0, 0)` — get item count.

3. If count > 0: iterate items 0..(count-1) via `CB_GETITEMDATA=0x150`; find
   the item whose item-data is `-1` (sentinel / "closed" slot); select it via
   `CB_SETCURSEL=0x14E`.

4. Call `FUN_006ADC20()` — triggers the per-row enable state machine for this
   row (which will propagate the "closed/disabled" state to sibling cells).

(verified via `decompile_function 0x006AE080`)

### Pass 2 — Hide row controls

For each row index `iVar4` from `param_2 - 1` to 6 again:

Gets five additional control IDs from per-row helpers:
- `FUN_004E3320()` — flag/picture-static control ID
- `FUN_004E37D0()` — country combo control ID
- `FUN_004E41D0()` — color combo control ID
- `FUN_004E4E60()` — start-position combo control ID
- `FUN_004E5940()` — team combo control ID

Then calls `ShowWindow(hWnd, SW_HIDE=0)` on all six controls:
1. AI-type combo (`local_18`)
2. Flag/picture static (`iVar3`)
3. Country combo (`nIDDlgItem`)
4. Color combo (`nIDDlgItem_00`)
5. Start-pos combo (`nIDDlgItem_01`)
6. Team combo (`nIDDlgItem_02`)

(verified via `decompile_function 0x006AE080`)

### Row iteration formula

The loop iterates `iVar4 = param_2 - 1` to 6, condition `(iVar4 + 2) < 8`.
So it covers rows `param_2 - 1` through `6` (7 total AI slots; slot 0 is the
human player and is never hidden by this function — it starts at `param_2 - 1`
which is ≥ 0 only when `param_2 ≥ 1`).

When `param_2 = 2` (map supports 2 players: slot 0 = human, slot 1 = AI),
the function hides rows 1 through 6 (5 AI rows hidden, 1 AI row visible).

(verified via `decompile_function 0x006AE080`)

## Interaction with FUN_006ADC20

`FUN_006ADC20` (the per-row enable state machine, task #3) is called within
Pass 1 for each row being hidden. This ensures the row's sibling cell enable
states are correctly cascaded before the controls are hidden in Pass 2.

## Out-of-scope refs

- `FUN_004E3320` — slot-0 flag/static control ID; in scope (task #18)
- `FUN_004E37D0` — row helper; in scope (task #21)
- `FUN_004E41D0` — row helper B; in scope (task #27)
- `FUN_004E4E60` — row helper C; in scope (task #36)
- `FUN_004E5940` — row helper D; in scope (task #44)
- `FUN_006ADC20` — per-row enable state machine; in scope (task #3)

## Unverified (YELLOW)

- The exact semantics of the five per-row control-ID helpers
  (`FUN_004E3320`, `FUN_004E37D0`, `FUN_004E41D0`, `FUN_004E4E60`,
  `FUN_004E5940`) are inferred from their use pattern (called with no
  arguments in a loop then passed to `GetDlgItem`); confirmed as control-ID
  getters but not decompiled in this task.
- The item-data value `-1` as the "closed/sentinel" item is inferred from
  `LVar2 == -1` being the break condition in the sentinel-find loop; consistent
  with other sentinel-writer functions in this system (e.g., `FUN_004E3F70`
  writes -2, but item-data -1 is used here as the sentinel search target).
  YELLOW: the distinction between -1 and -2 sentinel values is not fully
  resolved in this task.
