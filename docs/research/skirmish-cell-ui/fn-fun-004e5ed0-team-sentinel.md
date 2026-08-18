# FUN_004E5ED0 — Team Combo Selection Setter with Observer Hide

## Summary

Team combo analogue of `FUN_004E5480` (start-position sentinel, task #41) and
`FUN_004E49A0` (color sentinel, task #33). Takes a dialog HWND, a team-combo
control ID, and a target item-data `param_3`. Hides the combo, clears the current
slot's team assignment from `DAT_008B3FC8` (team assignment table, stride 0xC),
then scans all items for one whose CB_GETITEMDATA equals `param_3`, selects it via
CB_SETCURSEL, and if item-data != −2, writes the row index into the team assignment
table. After the scan-and-select, has an additional observer check: if `g_GameMode
== 4` AND `FUN_0077D970()` returns non-zero, hides the combo AND control `0x794`
(an additional team-related control) and returns early without re-showing.

## Address

`0x004E5ED0` (verified via `decompile_function 0x004E5ED0`)

## Active in YR

**Yes.** In-scope callers include `FUN_004E5E20` (task #47, direct caller),
`FUN_006ACD60` (task #17, team-enable), `FUN_006ADDF0` (task #14, row-showhide),
`FUN_006ADC20` (task #3, row-enable, anchor), and `FUN_006AE6E0` (task #1, init,
anchor).
(Callers confirmed via `get_function_callers 0x004E5ED0`)

## Signature / Parameters

```c
void __fastcall FUN_004e5ed0(HWND param_1, int param_2, int param_3)
// param_1 = dialog 0x102 HWND
// param_2 = team-combo control ID (0x76D..0x774)
// param_3 = target item-data to select
```

(verified via `decompile_function 0x004E5ED0`)

## Behavioral Analysis

### Step 1 — Hide combo

```c
pHVar2 = GetDlgItem(param_1, param_2);
BVar3 = ShowWindow(pHVar2, 0);   // hide; save visibility
```

(verified via `decompile_function 0x004E5ED0`)

### Step 2 — Clear prior team assignment

No explicit prior-ownership clear loop is visible in the decompile — the function
proceeds directly to the scan loop without first clearing the old entry. This
differs slightly from `FUN_004E49A0` (task #33, which does clear first). The old
slot's assignment is implicitly overwritten in the scan loop when the new item is
found.

### Step 3 — Scan and select, then write team assignment

```c
LVar4 = SendDlgItemMessageA(param_1, param_2, 0x146, 0, 0);  // CB_GETCOUNT
wParam = 0;
if (0 < LVar4) {
    do {
        LVar5 = SendDlgItemMessageA(param_1, param_2, 0x150, wParam, 0); // CB_GETITEMDATA
        if (LVar5 == param_3) {
            SendDlgItemMessageA(param_1, param_2, 0x14e, wParam, 0);     // CB_SETCURSEL
            if (LVar5 != -2) {
                // reverse-map param_2 → row index iVar6 (0x76D→0 .. 0x774→7)
                *(int *)(&DAT_008b3fc8 + LVar5 * 0xc) = iVar6;
            }
            break;
        }
        wParam++;
    } while ((int)wParam < LVar4);
}
```

The team assignment table is at `DAT_008B3FC8` with stride 0xC (12 bytes).
At entry `c = LVar5` (team item-data), offset `c * 0xC` from base `0x8B3FC8`,
stores the row index `iVar6` that owns this team. Item-data −2 skips the write.
(verified via `decompile_function 0x004E5ED0`)

### Step 4 — Observer mode hide (additional gate vs color/start-pos equivalents)

```c
if ((g_GameMode == 4) && (cVar1 = FUN_0077d970(), cVar1 != '\0')) {
    ShowWindow(pHVar2, 0);           // hide team combo again
    pHVar2 = GetDlgItem(param_1, 0x794);
    ShowWindow(pHVar2, 0);           // also hide control 0x794
    return;                           // early return — skip re-show
}
if (BVar3 != 0) {
    ShowWindow(pHVar2, 5);           // re-show if was visible
}
```

When `g_GameMode == 4` (observer) AND `FUN_0077D970()` returns non-zero (some
observer-mode query), both the team combo and control `0x794` (a related team
control, likely a team label or lock indicator) are hidden. This extra gate does
not exist in the color or start-position equivalents.
(verified via `decompile_function 0x004E5ED0`)

### Reverse-map: team combo ID → row index

| Control ID (param_2) | Row index |
|---|---|
| 0x76D | 0 |
| 0x76E | 1 |
| 0x76F | 2 |
| 0x770 | 3 |
| 0x771 | 4 |
| 0x772 | 5 |
| 0x773 | 6 |
| 0x774 | 7 |

Same control IDs as `FUN_004E5940` (task #44).

## Team Assignment Table (`DAT_008B3FC8`)

| Offset | Size | Content |
|---|---|---|
| `DAT_008B3FC8 + c * 0xC + 0` | 4 | Row index that owns team `c` (or unowned sentinel) |
| `DAT_008B3FC8 + c * 0xC + 4` | 4 | Unknown |
| `DAT_008B3FC8 + c * 0xC + 8` | 4 | Unknown (likely label string ptr) |

The stride 0xC and ownership-field-at-offset-0 pattern matches the color table
(`DAT_008B4040`, task #28) and start-position table (`DAT_008B3F38`, tasks #37–#38).

## Globals Referenced

| Global | Address | Access | Role |
|---|---|---|---|
| `DAT_008B3FC8` | `0x8B3FC8` | WRITE | Team assignment table |
| `g_GameMode` | symbolic | READ | Game mode; 4=observer |

(confirmed via `decompile_function 0x004E5ED0`)

## Callers

In-scope YR-active callers:
- `FUN_004E5E20` @ `0x004E5E20` — team sentinel dispatcher (task #47)
- `FUN_006ACD60` @ `0x006ACD60` — team-enable (task #17)
- `FUN_006ACE80` @ `0x006ACE80` — in-scope team helper
- `FUN_006ADC20` @ `0x006ADC20` — row-enable state machine (anchor task #3)
- `FUN_006ADDF0` @ `0x006ADDF0` — row show/hide (task #14)
- `FUN_006AE6E0` @ `0x006AE6E0` — dialog init (anchor task #1)

Out-of-scope callers: `FUN_005E9DB0`, `FUN_005E9E00`, `FUN_005E9E70`, `FUN_005EA3E0`,
`FUN_005EAE40`, `FUN_005ED5A0`, `FUN_005EE510`.

(confirmed via `get_function_callers 0x004E5ED0`)

## Callees

- `FUN_0077D970` — observer-mode query (out of scope)
- `GetDlgItem`, `ShowWindow`, `SendDlgItemMessageA` — Win32

## Out-of-scope refs

- `FUN_0077D970` — observer mode query; not in cell-UI scope
- Control `0x794` — team-related control (label or lock indicator); covered by broader
  dialog layout decode
- `DAT_008B3FC8` team table — layout covered by task #66 (`decode-struct-colortableentry`
  or a separate team table task)

## Unverified (YELLOW)

- `FUN_0077D970()` return semantics: inferred as "observer active / AlliesAllowed-style
  query" from usage alongside `g_GameMode == 4`; not decompiled in this session.
- Control `0x794` identity: inferred as a team-related label or lock indicator from
  context; not confirmed against the dialog resource layout.
- Team assignment table `DAT_008B3FC8` entry at offsets `+4` and `+8`: not written
  by this function; content unknown.
