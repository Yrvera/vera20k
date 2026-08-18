# FUN_006ACEE0 — Dialog 0x102 WM_COMMAND Dispatcher

## Summary

Central WM_COMMAND handler for the offline Skirmish dialog (0x102). Routes all
cell-change notifications — AI-type, country/flag, color, start-position, team combos
— to their per-row helpers. Also handles the Start (0x617) and Back (0x5C0) buttons
with full validation (start-count vs. player count, minimum-2-player check, team
conflict check, AlliesAllowed vtable check) and, on successful Start, serializes all
row data into session globals and launches the game.

## Address

`0x006ACEE0` — body extends to `0x006ADBD8` (verified via `decompile_function 0x006ACEE0`)

## Active in YR

**Yes.** Only caller is `FUN_006AE3F0` (dialog 0x102 DlgProc, address confirmed via
`get_function_callers 0x006ACEE0`). Dialog 0x102 is the YR offline Skirmish dialog —
no TS-gating flag, live in every standard YR skirmish session.

## Signature / Parameters

```c
void __fastcall FUN_006acee0(
    HWND param_1,   // dialog HWND
    int  param_2,   // wParam low word = control ID
    undefined4 param_3, // wParam high word (notification code, sometimes)
    int  param_4    // lParam / notification code in some branches
)
```

`GetWindowLongA(param_1, 8)` fetches the dialog's extra-data pointer at entry
(verified via `decompile_function 0x006ACEE0`; result stored in `local_234`).

## Control-ID → Dispatch Table

Confirmed from the switch-ladder in the decompilation
(verified via `decompile_function 0x006ACEE0`):

| Control ID(s) | Meaning | Handler(s) |
|---|---|---|
| 0x50B, 0x50E, 0x516, 0x51A, 0x51B, 0x51C, 0x51D | AI-type combos rows 0–6 | `FUN_006ADC20`, `FUN_006ACD60` |
| 0x510, 0x513, 0x514, 0x51E, 0x51F, 0x520, 0x521, 0x6A1 | Country/flag combos rows 0–6 | `FUN_004E3830`, `FUN_004E3690` |
| 0x522, 0x6A2 | Color combo (param_4==1) | `FUN_004E4C20` |
| 0x5AA | Map-selector combo change | map-reload sequence (see below) |
| 0x5C0 | Back button (BN_CLICKED, param_4==0) | session-abort path |
| 0x617 | Start button (BN_CLICKED, param_4==0) | full validation + launch |
| 0x6A3..0x6A8, 0x6AA, 0x6AB | Start-position combos (param_4==1) | `FUN_004E5700` |

## Map-Selector Change (ID 0x5AA)

When the map-selector combo fires (control 0x5AA), the function:

1. Copies `DAT_00A8B8E0` (current map name string) into a local 512-byte buffer via
   a hand-unrolled `strlen`/`memcpy` sequence.
2. Calls `FUN_007CA489(local_200, &DAT_00A8B322)` — sprintf-style formatter
   (CRT out-of-scope).
3. Calls `FUN_00608070()` — unknown; out-of-scope.
4. `ShowWindow(param_1, 0)` — hides the dialog.
5. Calls `FUN_005E68A0()` — returns a mode integer; if `== 2` (Random Map mode),
   takes the random-map branch (loads `s_RandMap_img_00829abc`, reloads preview,
   re-shows dialog, returns).
6. Otherwise (standard map): calls several helpers to reload map data
   (`CDFileClass__Constructor` calls — mislabeled by Ghidra; the distinct addresses
   0x005E7BF0, 0x005E74E0, 0x005E6520, 0x00641DB0 are all named `CDFileClass__Constructor`
   by Ghidra's RTTI labeler; per manifest note, 0x005E6520 is actually the
   selected-map start-count function — these are in-task decode scope for tasks
   #59, etc.), then calls `FUN_004E4FC0`, `FUN_004E5310`, `FUN_004E5D60`,
   `FUN_006ADDF0(DAT_00A8B254)` (row-count adjust), re-shows dialog, validates
   map file, runs `FUN_005D5F30`, `FUN_005E2EF0`, `FUN_005E2F60`, `FUN_006ACD60`.

## AI-Type Combo Scan (Start / Back path)

When Start (0x617) or Back (0x5C0) is pressed with `param_4 == 0`, the function
scans all 7 AI-type combos (rows 0–6) to count active AI players:

```
control IDs for rows 0-6:
  row 0 → 0x50B
  row 1 → 0x50E
  row 2 → 0x516
  row 3 → 0x51A
  row 4 → 0x51B
  row 5 → 0x51C
  row 6 → 0x51D
```

For each row:
- `SendDlgItemMessageA(hwnd, id, CB_GETCURSEL=0x147, 0, 0)` → selected index
- `SendDlgItemMessageA(hwnd, id, CB_GETITEMDATA=0x150, sel, 0)` → item data
- If item data is 0, 1, or 2 → the row is an active AI slot → increment count

Result stored in `DAT_00A8B274` (active_ai_count global).
(verified via `decompile_function 0x006ACEE0` + `get_xrefs_to 0x00A8B274` showing
single WRITE at `0x006AD052 in FUN_006ACEE0`)

## Start Button Validation (0x617)

On Start press, four ordered validation gates run before launch. Any failure
re-enables the Start button and returns without launching:

**Gate 1 — start positions vs. AI count:**
```
iVar4 = CDFileClass__Constructor()  // map start count (FUN_005E6520 per manifest)
if (iVar4 < local_24c + 1)          // start slots < total players
  → StringTable error 0x437 (title) / 0x438 (body) via FUN_005D3490 modal
  → EnableWindow(GetDlgItem(hwnd, 0x617), TRUE)
  → return
```

**Gate 2 — minimum 2 players:**
```
if (local_24c + 1 < 2)              // total players (AI + human) < 2
  → StringTable error 0x43F (title) / 0x440 (body)
  → EnableWindow Start, return
```

**Gate 3 — team conflict check:**
```
iVar14 = FUN_004E6030(0xFFFFFFFF)   // find first team-conflicted row
if (iVar14 >= 0):
  loops rows 0-6 again; for each active AI row calls FUN_004E5940(-1) to fix,
  then re-checks FUN_004E6030 twice; if conflict persists after loop:
    → StringTable error 0x457 (title) / 0x458 (body)
    → EnableWindow Start, return
```

**Gate 4 — AlliesAllowed vtable check:**
```
piVar1 = DAT_00A8B23C               // SelectedMode pointer
cVar2 = (**(code **)(*piVar1 + 0x14))(&local_248)  // vtable slot +0x14
if (cVar2 == '\0'):
  if (local_248 == 0x617):          // Start-button sub-check
    → StringTable error 0x469
    → EnableWindow Start, return
  else:
    → FUN_005D5E10()                // different validation error
```

Gate 4 reads the SelectedMode vtable at slot `+0x14` (byte offset, param typed `int *`
so `*piVar1 + 0x14` is `(*piVar1)[0x14/4]` = vtable slot index 5).
`DAT_00A8B23C` confirmed via `get_xrefs_to 0x00A8B23C` showing reads in `FUN_006ACEE0`.

## Session Data Serialization (successful Start)

After all gates pass, the function serializes row data into session globals.

**Player-type array write:**
```
DAT_00A8B3C4 = DAT_00A8B250   // saves current map ID
DAT_00A8B3C8 = DAT_00A8B254   // saves current start count
if DAT_00A8B8D8 <= DAT_00A8B254: DAT_00A8B3C8 = 0
```

**Loop 1 — active-row data (rows 1–7 relative, stored at base+index):**
For each of rows 0–6 (index 1–7 in arrays), if the row's AI-type item-data is
0/1/2 (active AI):
```
(&DAT_00A8B27C)[index] = type (CB_GETITEMDATA)
(&DAT_00A8B29C)[index] = FUN_004E4170(-1)   // country selection
(&DAT_00A8B2BC)[index] = FUN_004E4E20(-1)   // color selection
(&DAT_00A8B2DC)[index] = FUN_004E5900(-1)   // start-position selection
(&DAT_00A8B2FC)[index] = FUN_004E6030(-1)   // team selection
```

**Loop 2 — full player struct array (DAT_00A8B3F0, stride 3):**
All 7 rows unconditionally; for each row, maps item-data to player-type code:
```
item-data -1 → code 1
item-data  0 → code 4
item-data  1 → code 5
item-data  2 → code 6  (else 0 for unknown)
puVar21[-1] = type_code
puVar21[ 0] = country
puVar21[ 1] = color
puVar21 += 3
```

**Option checkboxes (IDs read with BM_GETCHECK=0xF0):**
```
0x529 → spin control; SendMessageA(hwnd, 0x400, ...) → DAT_00A8B268 = 6 - value
0x511 → DAT_00A8B25C
0x50C → DAT_00A8B270
0x54E → DAT_00A8B262 (bool)
0x69A → DAT_00A8B263 (bool)
0x69D → DAT_00A8B264 (bool)
0x693 → DAT_00A8B320 (bool)
0x696 → DAT_00A8B261 (bool)
```
(All confirmed from decompile body; `decompile_function 0x006ACEE0`)

**Player-data struct allocation:**
Allocates 0x85 bytes, fills name, option flags, map-name, start-pos, team into the
struct, appends to a dynamic array at `DAT_00A8DA78`.

**Finalization:**
```
SessionClass__ProcessRandomAssignments()  // resolve unset country/color
GetDlgItem(0x529) + SendMessageA(0x400)  // read final spin value
...
DAT_00A8B260 = 1   // game-ready flag
```
(verified via `decompile_function 0x006ACEE0`)

## Observed Globals Written by This Function

| Global | Address | Written value | Source |
|---|---|---|---|
| `active_ai_count` | `0x00A8B274` | AI-slot count 0–7 | scan loop |
| `DAT_00A8B3C4` | `0x00A8B3C4` | saved map ID | on Start |
| `DAT_00A8B3C8` | `0x00A8B3C8` | saved start count | on Start |
| `DAT_00A8B260` | `0x00A8B260` | 1 = game-ready | successful Start |
| `DAT_00A8B31F` | `0x00A8B31F` | 0 | on Start |
| `DAT_00A8B31D` | `0x00A8B31D` | 0 | on Start |
| `DAT_00A8B26C` | `0x00A8B26C` | 0 | on Start |

All confirmed from decompile output; `get_xrefs_to 0x00A8B274` shows single WRITE
within this function at offset `0x006AD052`.

## Out-of-scope refs

- `FUN_005E68A0` (0x005E68A0) — map-mode selector; not in current scope
- `FUN_00608070` (0x00608070) — unknown helper in map-change path; not in scope
- `FUN_006406E0`, `FUN_006406F0` — map-image preview loader/free; not in scope
- `FUN_0053ECB0` (0x0053ECB0) — player-name helper; not in scope
- `DAT_00A8DA74/78/7C/80/81/84/88` — dynamic-array for player-data structs; not decoded
- `DAT_00A8B3AC`, `DAT_00A8B394` — option fields copied into player struct; not decoded

## Unverified (YELLOW)

None. All load-bearing claims above are backed by inline Ghidra MCP citations.
