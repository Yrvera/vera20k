# FUN_004E41D0 — Per-Row Color Combo Control-ID Lookup

## Summary

Pure lookup function: maps a 0-based row index (0–7) to the Win32 dialog control ID
for the color combo in that row of dialog 0x102. Row 0 → 0x6A2, rows 1–7 →
0x522..0x528. No side effects. This is the color-combo analogue of `FUN_004E37D0`
(task #21, which serves the country/flag combos with IDs 0x6A1 / 0x510..0x521).

## Address

`0x004E41D0` (verified via `decompile_function 0x004E41D0`)

## Active in YR

**Yes.** In-scope callers include all five YR-active anchor functions:
`FUN_006ACEE0` (WM_COMMAND dispatcher), `FUN_006ADC20` (row-enable state machine),
`FUN_006ADF00` (reveal-AI-rows), `FUN_006AE080` (hide-AI-rows), `FUN_006AE6E0`
(dialog init), plus 16 additional out-of-scope callers.
(Callers confirmed via `get_function_callers 0x004E41D0`)

## Signature / Parameters

```c
int __fastcall FUN_004e41d0(int param_1)
// param_1 = 0-based row index (0–7)
// returns: Win32 dialog control ID for the color combo in that row
```

No callees — pure computation. (verified via `decompile_function 0x004E41D0`)

## Row-Index → Control-ID Mapping

Confirmed via `decompile_function 0x004E41D0`:

| Row index (param_1) | Control ID | Notes |
|---|---|---|
| 0 | 0x6A2 | Human-player slot — distinct from AI rows |
| 1 | 0x522 | AI slot 1 |
| 2 | 0x523 | AI slot 2 |
| 3 | 0x524 | AI slot 3 |
| 4 | 0x525 | AI slot 4 |
| 5 | 0x526 | AI slot 5 |
| 6 | 0x527 | AI slot 6 |
| 7 | 0x528 | AI slot 7 |
| other | 0x528 + delta | overflow; `(-(uint)(param_1 != 7) & 0xfffffad7) + 0x528` |

The overflow formula `(-(uint)(param_1 != 7) & 0xfffffad7) + 0x528` evaluates to:
- `param_1 == 7`: mask = 0, result = 0x528 (correct row 7 value)
- `param_1 != 7`: mask = 0xFFFFFFFF & 0xFFFFFAD7 = 0xFFFFFAD7, result =
  `0xFFFFFAD7 + 0x528 = 0xFFFFFFFF` (−1) for unknown row index

Row 0 uses control ID 0x6A2 (human-player slot); rows 1–7 use 0x522..0x528 (AI slots).
This parallels `FUN_004E37D0` which maps row 0 → 0x6A1 (human country) and rows
1–7 → 0x510..0x521 (AI country).

## Relationship to Other Row-Lookup Functions

| Function | Task | Combo type | Row 0 ID | Row 1–7 IDs |
|---|---|---|---|---|
| `FUN_004E37D0` | #21 | Country/flag | 0x6A1 | 0x510, 0x513, 0x51E, 0x514, 0x51F, 0x520, 0x521 |
| `FUN_004E41D0` | #27 | Color | 0x6A2 | 0x522..0x528 |

The human-player color combo (0x6A2) is one above the country combo (0x6A1),
and the AI-row color IDs (0x522..0x528) immediately follow the AI-row country
IDs (0x510..0x521) in dialog resource order.

## Callers

In-scope YR-active callers:
- `FUN_006ACEE0` @ `0x006ACEE0` — WM_COMMAND dispatcher (anchor task #2)
- `FUN_006ADC20` @ `0x006ADC20` — row-enable state machine (anchor task #3)
- `FUN_006ADF00` @ `0x006ADF00` — reveal-AI-rows (task #15)
- `FUN_006AE080` @ `0x006AE080` — hide-AI-rows (task #16)
- `FUN_006AE6E0` @ `0x006AE6E0` — dialog init (anchor task #1)

Out-of-scope callers (16): `CDFileClass__Constructor @ 005E3D10`,
`FUN_0046DD70`, `FUN_005DBF50`, `FUN_005E8EA0`, `FUN_005E9E70`, `FUN_005EA3E0`,
`FUN_005EAEE0`, `FUN_005EB7B0`, `FUN_005EBA00`, `FUN_005EBC00`, `FUN_005EC3A0`,
`FUN_005EC9F0`, `FUN_005ED400`, `FUN_005ED5A0`, `FUN_005EDDE0`, `FUN_0077ED00`.

(confirmed via `get_function_callers 0x004E41D0`)

## Out-of-scope refs

None — pure lookup, no callees.

## Unverified (YELLOW)

None. All load-bearing claims backed by inline Ghidra MCP citations above.
