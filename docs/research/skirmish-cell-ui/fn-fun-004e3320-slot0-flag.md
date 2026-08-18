# FUN_004E3320 — Slot-0 Flag/Static Control-ID Lookup

## Summary

Pure lookup function: maps a 0-based row index (0–7) to the Win32 dialog control ID
for the picture/flag static control in that row of dialog 0x102. Row 0 → 0x6DA,
row 1 → 0x6DB, ..., row 7 → 0x6E1. No side effects. Called by the reveal-rows
(`FUN_006ADF00`) and hide-rows (`FUN_006AE080`) functions to find the flag static
HWND for each slot, plus several other callers outside the cell-UI scope.

## Address

`0x004E3320` (verified via `decompile_function 0x004E3320`)

## Active in YR

**Yes.** Called by `FUN_006ADF00` (0x006ADF00, task #15 — reveal-AI-rows, YR-active)
and `FUN_006AE080` (0x006AE080, task #16 — hide-AI-rows, YR-active).
Additional callers `FUN_005EA3E0`, `FUN_005EB7B0`, `FUN_005EBA00`, `FUN_005EBC00`,
`FUN_00658330` are outside cell-UI scope but confirm wider YR usage.
(Callers confirmed via `get_function_callers 0x004E3320`)

## Signature / Parameters

```c
int __fastcall FUN_004e3320(int param_1)  // param_1 = 0-based row index (0–7)
// returns: Win32 dialog control ID for the flag/picture static in that row
```

No callees — pure computation. (Confirmed via `get_function_callees 0x004E3320` which
returned no callees.)

## Row-Index → Control-ID Mapping

Confirmed via `decompile_function 0x004E3320`:

| Row index (param_1) | Control ID | Notes |
|---|---|---|
| 0 | 0x6DA | Slot 0 — human player flag static |
| 1 | 0x6DB | Slot 1 — AI row flag static |
| 2 | 0x6DC | Slot 2 |
| 3 | 0x6DD | Slot 3 |
| 4 | 0x6DE | Slot 4 |
| 5 | 0x6DF | Slot 5 |
| 6 | 0x6E0 | Slot 6 |
| 7 | 0x6E1 | Slot 7 |
| other | 0x6E1 + delta | overflow case; param_1 == 8 would return 0xFFFF91F (bogus) |

The eight IDs form a contiguous range `0x6DA..0x6E1` (8 entries, stride 1).

## Behavioral Analysis

Completely stateless lookup. Returns the resource dialog control ID for the
flag/picture static (likely an SS_BITMAP static showing the player's country flag
or a generic placeholder). This control is one of the six per-row controls revealed
or hidden by `FUN_006ADF00` / `FUN_006AE080` (task #15, #16).

The function is not row-self-aware — the caller must pass the correct row index.
In `FUN_006ADF00`, the call is `iVar2 = FUN_004E3320()` without visible arguments
in the Ghidra decompile; the row index is likely passed via fastcall register ECX
(the function takes one `int` parameter in the `__fastcall` convention — first
integer in ECX).

## Callers (in scope)

- `FUN_006ADF00` (0x006ADF00) — task #15, reveal-AI-rows
- `FUN_006AE080` (0x006AE080) — task #16, hide-AI-rows

Out-of-scope callers: `FUN_005EA3E0`, `FUN_005EB7B0`, `FUN_005EBA00`,
`FUN_005EBC00`, `FUN_00658330` — usage outside dialog 0x102, not decoded here.

## Out-of-scope refs

None — function has no callees and all in-scope callers are tracked above.

## Unverified (YELLOW)

None. All claims backed by inline Ghidra MCP citations above.
