# FUN_006AE6E0 — Dialog 0x102 Custom Init (Message 0x497)

## Summary

`FUN_006AE6E0` is the custom initialization handler for dialog 0x102 (offline Skirmish setup), called on message 0x497 (a WM_INITDIALOG-class custom message) from the DlgProc `FUN_006AE3F0`. It performs the complete initial population of all eight player-slot "rows," configuring each cell control (AI-type combo, country combo, color combo, start-position combo, team combo) from persisted session data, then finishes by initializing the map-selection spinner, game-option checkboxes, and selected-mode pointer. The function constitutes the "dialog is open, fill it in" observable behavior for the skirmish lobby screen.

Caller confirmed: sole caller is `FUN_006AE3F0` (the DlgProc), verified via `get_function_callers 0x006AE6E0`.

## Active in YR

**Yes.** `FUN_006AE6E0` is called from `FUN_006AE3F0` which is registered as the DlgProc for the offline Skirmish dialog. The dialog is accessible from the normal YR main menu skirmish flow. No TS-only gate guards it (verified: no `SpecialFlags`, `FogOfWar`, or Tiberium-path guard in decompile; dialog 0x102 is YR-introduced). Confirmed via `decompile_function 0x006AE6E0`.

## Decompilation excerpt (verified via `decompile_function 0x006AE6E0`)

```c
// Phase 1: session state init (0069b540)
FUN_0069b540();   // resets param_1+0x24, fills 8-entry slot arrays with -1

// Phase 2: populate 7 AI-type combos (slots 1-7, dialog items 0x50B..0x51D)
iVar14 = 0;
do {
    // AI combo ID map: slot 0→0x50B, 1→0x50E, 2→0x516, 3→0x51A,
    //                  4→0x51B, 5→0x51C, 6→0x51D
    pHVar3 = GetDlgItem(param_1, iVar5);
    SendMessageA(pHVar3, 0x14b, 0, 0);    // CB_RESETCONTENT
    // item-data -1: "Closed"  (string table 0x9E)
    // item-data  2: "Computer" easy  (string table 0xA1)
    // item-data  1: "Computer" medium (string table 0xA4)
    // item-data  0: "Human"   (string table 0xA7)
    SendMessageA(pHVar3, 0x14e, WVar6, -1/2/1/0);  // CB_SETITEMDATA
    SendMessageA(pHVar3, 0x14e, 0, 0);   // CB_SETCURSEL first item
    iVar14++;
} while (iVar14 < 7);

// Phase 3: populate per-cell combos for all 8 rows
FUN_004e3b90();   // country combo loader (slots 0-7)
FUN_004e43c0();   // color label loader (9-row table)
FUN_004e4820();   // color combo helper
FUN_004e4f50();   // color helper
FUN_004e5310();   // start-position helper
FUN_004e5ac0();   // team combo helper
FUN_004e5d60();   // team combo helper
FUN_004e3ce0();   // country flag helper
FUN_004e48e0();   // color combo helper
FUN_004e53d0();   // start-position helper
FUN_004e5e20();   // team combo helper

// Phase 4: restore AI-type selection from session data (DAT_00a8b3f0 array)
local_c = 0;
local_10 = &DAT_00a8b3f0;   // slot type array base (0x00A8B3F0)
do {
    // slot type-code → combo item-data:
    //   type 1 (Closed)    → item-data -1
    //   type 4 (Human)     → item-data  0
    //   type 5 (Easy AI)   → item-data  1
    //   type 6 (Hard AI)   → item-data  2
    //   other              → item-data  2 (Hard AI fallback)
    iVar5 = local_10[-1];   // one dword before current ptr = type field
    if (iVar5 == 1) local_14 = -1;
    else if (iVar5 == 4) local_14 = 0;
    else if (iVar5 == 5) local_14 = 1;
    else local_14 = 2;

    // scan combo for matching item-data; select it via CB_SETCURSEL (0x14E)
    LVar7 = SendDlgItemMessageA(param_1, iVar14, 0x146, 0, 0);  // CB_GETCOUNT
    // ... scan loop CB_GETITEMDATA (0x150) → CB_SETCURSEL (0x14E)

    // row helper dispatch (get country/color/startpos/team control IDs)
    iVar14     = FUN_004e37d0(local_c);  // country control ID for this slot
    iVar5      = FUN_004e41d0(local_c);  // color control ID
    nIDDlgItem = FUN_004e4e60(local_c);  // start-position control ID
    nIDDlgItem_00 = FUN_004e5940(local_c); // team control ID

    if (local_14 == -1) {
        // row closed: sentinel-write -2 to all four cell combos
        FUN_004e3f70(0xFFFFFFFE);   // country sentinel
        FUN_004e49a0(0xFFFFFFFE);   // color sentinel
        FUN_004e5480(0xFFFFFFFE);   // start-pos sentinel
        FUN_004e5ed0(0xFFFFFFFE);   // team sentinel
        // then disable all four windows
        EnableWindow(GetDlgItem(param_1, iVar14), 0);
        EnableWindow(GetDlgItem(param_1, iVar5),  0);
        EnableWindow(GetDlgItem(param_1, nIDDlgItem),   0);
        EnableWindow(GetDlgItem(param_1, nIDDlgItem_00), 0);
    } else {
        // row active: restore country and color from session data
        FUN_004e3f70(*local_10);      // country combo: select local_10[0]
        FUN_004e49a0(local_10[1]);    // color combo:   select local_10[1]
    }
    local_10 += 3;  // advance by 3 dwords per slot
    local_c++;
} while (local_c < 7);

// Phase 5: map selection init
// clamp map index (DAT_00a8b3c4), load NumPlayers from selected map
// via FUN_005d5e10 (returns count), then call FUN_005e2f80 (local_4[10])
DAT_00a8b3c4 = local_4[10];   // map start-count from selected-map object +0x28
DAT_00a8b250 = local_4[10];

// Phase 6: team combo refresh
FUN_004e5ac0(); FUN_004e5d60(); FUN_004e5e20();

// Phase 7: team enable loop (slots 1-7), gated by AlliesAllowed flag
iVar14 = 1;
do {
    FUN_004e5940();  // team control ID for this slot
    // if DAT_00a8b23c->field[0xF] != 0 → team = 3, else team = 0xFFFFFFFE
    FUN_004e5ed0(uVar17);
    iVar14++;
} while (iVar14 < 8);

// Phase 8: row visibility per selected map
FUN_006addf0(DAT_00a8b254);

// Phase 9: final state
DAT_00a8b23c = local_4;       // store selected-mode pointer
(*(*local_4 + 0x20))();       // vtable call on selected-mode object
FUN_006acd60();               // team-control enable refresh
FUN_005e2ef0(); FUN_005e2f60(); FUN_005c6120();
```

## Behavioral analysis

### Phase 1 — Session reset (FUN_0069b540)

`FUN_0069b540` (verified via `decompile_function 0x0069b540`) takes a pointer param and:
- Zeros dword at `param_1 + 0x24`
- Fills 8 pairs of dwords at `param_1 + 0x6C` (and `param_1 + 0x4C`) with `0xFFFFFFFF` (-1)

This zeroes and resets the slot tracking state before repopulation.

### Phase 2 — AI-type combo population (slots 1-7)

The AI-type combo for each of 7 slots (dialog item IDs below) is cleared and filled with 4 items in this fixed order:
- String table 0x9E → item-data **-1** = "Closed"
- String table 0xA1 → item-data  **2** = "Computer" (Easy)
- String table 0xA4 → item-data  **1** = "Computer" (Medium)
- String table 0xA7 → item-data  **0** = "Human"

The first item (Closed) is selected by default. Slot 0 (human player, edit control) is excluded from this loop — only slots 1-7 have AI-type combos.

AI-type combo control IDs per slot (verified via `decompile_function 0x006AE6E0`):

| Slot | AI Combo ID |
|------|-------------|
| 1    | 0x50E       |
| 2    | 0x516       |
| 3    | 0x51A       |
| 4    | 0x51B       |
| 5    | 0x51C       |
| 6    | 0x51D       |

Slot 0's analogous control is at 0x50B but not populated here (it is excluded from the AI combo loop; the first loop index `iVar14==0` maps to 0x50B but the CB_RESETCONTENT still fires).

### Phase 3 — Per-cell combo population (helper call-out)

Eleven callee functions populate the actual content of country, color, start-position, and team combos. These are in scope as separate decode tasks.

### Phase 4 — Session data restore (type-code mapping)

The slot type-code array begins at `DAT_00A8B3F0` (confirmed via `get_xrefs_to 0x00A8B3F0`; `[DATA]` from offset 0x006AE8C7 in this function). The pointer `local_10` steps by +3 dwords per slot through this array:

```
local_10[-1] = slot type code (read below)
local_10[0]  = country index   (used by FUN_004e3f70)
local_10[1]  = color index     (used by FUN_004e49a0)
```

Type-code to combo item-data mapping (observable behavior):

| Persisted type code | Item-data selected | Meaning     |
|---------------------|--------------------|-------------|
| 1                   | -1                 | Closed      |
| 4                   | 0                  | Human       |
| 5                   | 1                  | Easy AI     |
| 6                   | 2                  | Hard/Medium AI |
| other               | 2                  | Hard AI fallback |

When type-code maps to -1 (Closed), the function calls all four sentinel writers with value `0xFFFFFFFE` (-2), then calls `EnableWindow(..., 0)` to disable the country, color, start-position, and team controls. This is the "row closed" observable state.

### Row-helper control ID maps (verified via decompile calls)

`FUN_004e37d0` — **country combo** IDs per slot (verified via `decompile_function 0x004E37D0`):

| Slot | Control ID |
|------|-----------|
| 0    | 0x6A1     |
| 1    | 0x510     |
| 2    | 0x513     |
| 3    | 0x51E     |
| 4    | 0x514     |
| 5    | 0x51F     |
| 6    | 0x520     |
| 7    | 0x521     |

`FUN_004e41d0` — **color combo** IDs per slot (verified via `decompile_function 0x004E41D0`):

| Slot | Control ID |
|------|-----------|
| 0    | 0x6A2     |
| 1    | 0x522     |
| 2    | 0x523     |
| 3    | 0x524     |
| 4    | 0x525     |
| 5    | 0x526     |
| 6    | 0x527     |
| 7    | 0x528     |

`FUN_004e4e60` — **start-position combo** IDs per slot (verified via `decompile_function 0x004E4E60`):

| Slot | Control ID |
|------|-----------|
| 0    | 0x6A3     |
| 1    | 0x6A4     |
| 2    | 0x6A5     |
| 3    | 0x6A6     |
| 4    | 0x6A7     |
| 5    | 0x6A8     |
| 6    | 0x6AA     |
| 7    | 0x6AB     |

`FUN_004e5940` — **team combo** IDs per slot (verified via `decompile_function 0x004E5940`):

| Slot | Control ID |
|------|-----------|
| 0    | 0x76D     |
| 1    | 0x76E     |
| 2    | 0x76F     |
| 3    | 0x770     |
| 4    | 0x771     |
| 5    | 0x772     |
| 6    | 0x773     |
| 7    | 0x774     |

### Phase 5 — Map selection init

`FUN_005D5E10` (the MapList singleton accessor, verified via `decompile_function 0x005D5E10`) returns a pointer to the current map list object. Field `+0x28` gives the player count (start count). `local_4[10]` corresponds to byte offset `+0x28` (param type is `int *`, so index 10 × 4 = 0x28).

`DAT_00a8b3c4` receives the start count. `DAT_00a8b250` mirrors it (confirmed: both written at 0x006AEAE5 and 0x006AEAFF in `FUN_006AE6E0` via `get_xrefs_to 0x00a8b3c4`).

### Phase 7 — Team control enable loop

For slots 1-7, the team combo is set to item-data 3 (enabled) if `DAT_00a8b23c->field[0xF]` is non-zero (AlliesAllowed), or 0xFFFFFFFE (sentinel/disabled) if not. This correctly reflects the YR-only team grouping feature.

### Phase 8 — Row visibility

`FUN_006ADDF0` is called with `DAT_00a8b254` (selected map index) to show/hide rows beyond the selected map's start count. This is the observable "only N player rows shown for an N-player map" behavior.

### Phase 9 — Selected-mode setup

`DAT_00a8b23c` receives `local_4` (the map/mode object pointer), and a vtable call on it at `vtable+0x20` is executed. `FUN_006ACD60` then refreshes team-control enable state. `FUN_005E2EF0` and `FUN_005E2F60` are also called (covered by separate decode tasks).

### Map name copy branch (in FUN_006AE6E0, gated by FUN_0069adf0 return)

`FUN_0069adf0` (verified via `decompile_function 0x0069adf0`) is a 3-line strcmp wrapper — it takes an object pointer, checks if `param_1 + 0x58` equals the string `"RandMap"`, and returns a bool. It performs no allocation or copy itself.

All resource allocation and map name copy logic lives in `FUN_006AE6E0`'s own body, conditioned on this bool return:
- If `FUN_0069adf0()` returns true (random map): `FUN_006AE6E0` frees any prior resource object at `DAT_00ac1154`, allocates a new one via `operator_new`, and loads `RandMap.img`. If `*DAT_00ac1154 != 0` after allocation, returns early.
- If false (named map): `FUN_006AE6E0` copies the map name from `*(DAT_00a8b8cc + uVar9*4) + 0x58` into `DAT_00a8b8e0` (the current map name buffer).

## Struct field accesses

| Source pointer | Offset | Unit | Usage | Frame |
|----------------|--------|------|-------|-------|
| `local_4` (map list ptr) | `+0x28` = `local_4[10]` | — | Player/start count for selected map | internal struct field |
| `local_4` vtable | `+0x20` | vtable slot | Call on selected-mode init | vtable call |
| `DAT_00a8b23c` | `+0x3C` (`[0xF]` as int*) | byte | AlliesAllowed flag | internal struct field |
| `DAT_00a8b8cc` | `+ uVar9 * 4` | dword array | Map list entry pointers | internal struct |
| Map entry ptr | `+0x58` | char* | Map name string | internal struct field |
| `g_RulesClass_Instance` | `+0x1480`, `+0x1488` | — | Spinner range (start-positions / game options) | rules class field |
| `g_RulesClass_Instance` | `+0x1490`, `+0x1498` | — | Spinner range (second spinner) | rules class field |
| `g_RulesClass_Instance` | `+0x148c` | ptr | Spinner list LPARAM | rules class field |

## Globals referenced

| Global | Address | Role in this function |
|--------|---------|----------------------|
| `DAT_00A8B3F0` | 0x00A8B3F0 | Slot type/country/color array base (3 dwords per slot) — verified via `get_xrefs_to 0x00A8B3F0` |
| `DAT_00A8B23C` | 0x00A8B23C | Selected-mode object pointer — verified via `get_xrefs_to 0x00A8B23C` |
| `DAT_00A8B3C4` | 0x00A8B3C4 | Selected-map start count (player count) — verified via `get_xrefs_to 0x00A8B3C4` |
| `DAT_00A8B250` | 0x00A8B250 | Mirror of start count / selected map index — verified via `get_xrefs_to 0x00A8B250` |
| `DAT_00A8B254` | 0x00A8B254 | Committed selected map index — verified via `get_xrefs_to 0x00A8B254` |
| `DAT_00A8B3C8` | 0x00A8B3C8 | Selected map combo index — validated/clamped before use |
| `DAT_00A8B8D8` | 0x00A8B8D8 | Map list count (upper bound for map index) |
| `DAT_00A8B8CC` | 0x00A8B8CC | Map list pointer array base |
| `DAT_00A8B8E0` | 0x00A8B8E0 | Current map name buffer |
| `DAT_00A8B268` | 0x00A8B268 | Start-positions spinner value (restored from `DAT_00A8B3CC`) |
| `DAT_00A8B25C` | 0x00A8B25C | Spinner value (restored from `DAT_00A8B3D0`) |
| `DAT_00A8B270` | 0x00A8B270 | Spinner value (restored from `DAT_00A8B3D4`) |
| `DAT_00A8B262`..`DAT_00A8B264`, `DAT_00A8B320`, `DAT_00A8B261` | various | Game-option checkbox states (restored from persisted `DAT_00A8B3D8..DC`) |
| `g_RulesClass_Instance` | named | RulesClass global (used for spinner ranges) |
| `DAT_00AC1154` | 0x00AC1154 | Random-map resource object pointer |
| `DAT_00ABFD70`.. | static init flags | MapList singleton init guard (inside `FUN_005D5E10`) |

## Callers

Only `FUN_006AE3F0` (the DlgProc) — confirmed via `get_function_callers 0x006AE6E0`.

## Callees (summary)

| Address | Name | Role in this function |
|---------|------|-----------------------|
| 0x0069B540 | FUN_0069b540 | Session slot array reset |
| 0x004E3B90 | FUN_004e3b90 | Country combo loader |
| 0x004E43C0 | FUN_004e43c0 | Color label loader |
| 0x004E4820 | FUN_004e4820 | Color combo helper |
| 0x004E4F50 | FUN_004e4f50 | Color helper |
| 0x004E5310 | FUN_004e5310 | Start-position helper |
| 0x004E5AC0 | FUN_004e5ac0 | Team combo helper |
| 0x004E5D60 | FUN_004e5d60 | Team combo helper |
| 0x004E3CE0 | FUN_004e3ce0 | Country flag helper |
| 0x004E48E0 | FUN_004e48e0 | Color combo helper |
| 0x004E53D0 | FUN_004e53d0 | Start-position helper |
| 0x004E5E20 | FUN_004e5e20 | Team combo helper |
| 0x004E37D0 | FUN_004e37d0 | Country combo ID mapper |
| 0x004E41D0 | FUN_004e41d0 | Color combo ID mapper |
| 0x004E4E60 | FUN_004e4e60 | Start-pos combo ID mapper |
| 0x004E5940 | FUN_004e5940 | Team combo ID mapper |
| 0x004E3F70 | FUN_004e3f70 | Country sentinel/selection writer |
| 0x004E49A0 | FUN_004e49a0 | Color sentinel/selection writer |
| 0x004E5480 | FUN_004e5480 | Start-pos sentinel/selection writer |
| 0x004E5ED0 | FUN_004e5ed0 | Team sentinel/selection writer |
| 0x004E4FC0 | FUN_004e4fc0 | Color helper (post-map init) |
| 0x005D5E10 | FUN_005d5e10 | MapList singleton accessor |
| 0x005D5F30 | FUN_005d5f30 | MapList check helper (called from 005e2f80) |
| 0x005E2F80 | FUN_005e2f80 | Map list accessor (calls 005d5e10 or 005d5f30) |
| 0x005D63E0 | FUN_005d63e0 | Map validity check |
| 0x005D5ED0 | FUN_005d5ed0 | Map entry helper |
| 0x006ADDF0 | FUN_006addf0 | Row show/hide based on map start count |
| 0x006ACD60 | FUN_006acd60 | Team-control enable refresh |
| 0x0069ADF0 | FUN_0069adf0 | Random-map name check |
| 0x005E2EF0 | FUN_005e2ef0 | Dialog helper (out of scope) |
| 0x005E2F60 | FUN_005e2f60 | Dialog helper (out of scope) |
| 0x005C6120 | FUN_005c6120 | (out of scope) |

## Out-of-scope refs

- `FUN_005C6120` @ 0x005C6120 — appears to be a UI subsystem call not cell-specific; out of current scope
- `FUN_005E2EF0` @ 0x005E2EF0 — dialog helper, task #186 already created
- `FUN_005E2F60` @ 0x005E2F60 — dialog helper, task #189 already created
- `FUN_006406E0` / `FUN_006406F0` — resource object create/destroy (random map resource), out of scope
- `FUN_0058BB30` — out of scope
- Vtable call `*DAT_00a8b23c + 0x20` and `*DAT_00a8b23c + 4` — selected-mode vtable, covered by selected-mode system decode

## TS-filter

This entire function is dialog 0x102 which is YR-introduced. No TS-gated paths found in the decompilation. **TS-legacy score: 0.0.** All callee helpers verified as YR-active by caller-chain analysis (sole entry via `FUN_006AE3F0` DlgProc, reachable from standard YR skirmish menu).

## Unverified claims (YELLOW)

- String table indices 0x9E, 0xA1, 0xA4, 0xA7 for AI-type combo items — confirmed in decompile as `StringTable__LoadString` calls with those indices, but string content not verified by reading the string table directly. These are load-bearing for observable text labels.
- `g_RulesClass_Instance + 0x1480`, `+0x1488`, `+0x1490`, `+0x1498`, `+0x148c` — spinner range fields. Field names not verified; only the offsets are observed in the decompile. Semantic meaning (min/max/list) inferred from context (`WM_PBM_SETRANGE` / `WM_PBM_SETPOS` / `WM_PBM_SETARRAY` messages 0x406, 0x405, 0x4AB).
- `DAT_00A8B3EC` xref at 0x006AE931 in `FUN_006AE6E0` — this dword is read as a counter bound; exact semantic not independently confirmed beyond what the decompile shows.
