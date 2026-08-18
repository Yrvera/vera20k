# FUN_004E5AC0 — Team Combo String Table Initializer

## Summary

Initializes 4 team name string pointers and a fallback pointer in a static
data table at `DAT_008B3FC0`. Loads string IDs `0x437`, `0x438`, `0x439`,
`0x43A` from the `GDlgSupp` string table into the first four entries, then
writes a pointer to `DAT_00822BA4` (the start-position single-char label
array, from task #37) as the fifth entry. Called once from dialog init. No
parameters, no return value.

## Address

`0x004E5AC0` (verified via `decompile_function 0x004E5AC0`)

## Active in YR

**Yes.** Sole caller is `FUN_006AE6E0` (0x006AE6E0), the dialog init handler.

(confirmed via `get_function_callers 0x004E5AC0`)

## Signature / Parameters

```c
void FUN_004e5ac0(void)
```

No parameters, no return value.

(verified via `decompile_function 0x004E5AC0`)

## Behavioral Analysis

```c
DAT_008B3FC0 = StringTable__LoadString(s_GDlgSupp, 0x437);  // team string 0
DAT_008B3FCC = StringTable__LoadString(s_GDlgSupp, 0x438);  // team string 1
_DAT_008B3FD8 = StringTable__LoadString(s_GDlgSupp, 0x439); // team string 2
_DAT_008B3FE4 = StringTable__LoadString(s_GDlgSupp, 0x43A); // team string 3
_DAT_008B3FF0 = &DAT_00822BA4;                                // fallback / end marker
```

(verified via `decompile_function 0x004E5AC0`)

### Table layout at `DAT_008B3FC0`

Each string pointer is at stride 12 bytes (3 dwords), matching the same
stride used by the start-position table at `DAT_008B3F30`. The entries:

| Offset from 0x8B3FC0 | Field | Value |
|---|---|---|
| 0x00 (0x8B3FC0) | string ptr [0] | `LoadString(0x437)` |
| 0x0C (0x8B3FCC) | string ptr [1] | `LoadString(0x438)` |
| 0x18 (0x8B3FD8) | string ptr [2] | `LoadString(0x439)` |
| 0x24 (0x8B3FE4) | string ptr [3] | `LoadString(0x43A)` |
| 0x30 (0x8B3FF0) | fallback ptr   | `&DAT_00822BA4` |

### String IDs 0x437–0x43A

These are team combo display strings in `GDlgSupp.csf`. The exact localized
text is not read in this session, but from context: the Skirmish dialog has
4 team options (No Team, Team A, Team B, etc. or numeric), matching the 4
string loads.

### Fallback pointer `&DAT_00822BA4`

`DAT_00822BA4` is the start-position single-char ASCII label array (from
`FUN_004E4F50`, task #37 — labels "0".."8"). Its pointer is placed as a
5th entry, likely as a sentinel/end marker or a fallback string for an
overflow entry.

## Globals referenced

| Global | Address | Access | Role |
|--------|---------|--------|------|
| `DAT_008B3FC0` | 0x008B3FC0 | WRITE | Team string table base |
| `DAT_00822BA4` | 0x00822BA4 | READ (addr-of) | Start-pos label array (fallback) |

## Callers

| Address | Name | Role |
|---------|------|------|
| 0x006AE6E0 | FUN_006ae6e0 | Dialog init (task #1) |

(confirmed via `get_function_callers 0x004E5AC0`)

## Callees

| Address | Name | Role |
|---------|------|------|
| 0x00734E60 | StringTable__LoadString | Load string from GDlgSupp table |

(confirmed via `get_function_callees 0x004E5AC0`)

## Out-of-scope refs

- `DAT_00822BA4` and its content: decoded in task #37 as ASCII digit labels for
  start-position entries.

## TS-filter

Sole caller is the YR dialog init. No TS-only gate. **TS-legacy score: 0.0.**

## Unverified (YELLOW)

- String IDs `0x437`–`0x43A` text content: not loaded from `GDlgSupp.csf` in
  this session; inferred as team name labels from their use in the team combo table.
- `&DAT_00822BA4` as "fallback/end marker": position at entry [4] of a 4-entry
  logical team table is consistent with a sentinel; actual role (end marker vs.
  overflow vs. something else) not independently verified by reading callers of
  this table.
- Table stride 12 bytes (3 dwords): inferred from the address pattern
  `0x3FC0, 0x3FCC, 0x3FD8, 0x3FE4, 0x3FF0` — consistent stride of 12;
  confirmed by matching the `FUN_004E4F50` (task #37) table structure.
