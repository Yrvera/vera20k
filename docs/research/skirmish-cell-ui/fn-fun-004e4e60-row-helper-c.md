# FUN_004E4E60 — Per-Row Start-Position Combo Control-ID Lookup

## Summary

Pure lookup function: maps a 0-based row index (0–7) to the Win32 dialog
control ID for the start-position combo in that row of dialog 0x102. Row 0
→ 0x6A3, rows 1–7 → 0x6A4..0x6A8, 0x6AA..0x6AB. Note the gap: 0x6A9 is
skipped between rows 5 and 6. No side effects. Called by all anchor functions
and row show/hide cluster — it is the canonical source of start-position combo
control IDs for the dialog.

## Address

`0x004E4E60` (verified via `decompile_function 0x004E4E60`)

## Active in YR

**Yes.** In-scope callers include all five anchor functions for dialog 0x102:
- `FUN_006ACEE0` (0x006ACEE0) — WM_COMMAND dispatcher (task #2)
- `FUN_006ADC20` (0x006ADC20) — row enable state machine (task #3)
- `FUN_006ADF00` (0x006ADF00) — reveal-AI-rows (task #15)
- `FUN_006AE080` (0x006AE080) — hide-AI-rows (task #16)
- `FUN_006AE6E0` (0x006AE6E0) — dialog init (task #1)

(confirmed via `get_function_callers 0x004E4E60`)

## Signature / Parameters

```c
int __fastcall FUN_004e4e60(int param_1)
// param_1 = 0-based row index (0–7)
// returns: Win32 dialog control ID for the start-position combo in that row
```

No callees — pure computation. (confirmed via `get_function_callees 0x004E4E60`)

## Row-Index → Control-ID Mapping

(verified via `decompile_function 0x004E4E60`)

| Row index (param_1) | Control ID | Notes |
|---|---|---|
| 0 | 0x6A3 | Human-player slot |
| 1 | 0x6A4 | AI slot 1 |
| 2 | 0x6A5 | AI slot 2 |
| 3 | 0x6A6 | AI slot 3 |
| 4 | 0x6A7 | AI slot 4 |
| 5 | 0x6A8 | AI slot 5 |
| 6 | 0x6AA | AI slot 6 — note: 0x6A9 is skipped |
| 7 | 0x6AB | AI slot 7 |
| other | 0x6AB + delta | overflow |

The gap at 0x6A9 is intentional in the original binary; control ID 0x6A9 is
absent from the start-position combo set for the skirmish dialog.

## Cross-References

Control IDs returned by this function appear in the WM_COMMAND dispatcher
`FUN_006ACEE0` (task #2) in the CBN_SELCHANGE handler for start-position combos.
The full set `0x6A3..0x6A8, 0x6AA, 0x6AB` matches this function's output table.

## Callers (in scope)

- `FUN_006ACEE0` (0x006ACEE0) — WM_COMMAND dispatcher (task #2)
- `FUN_006ADC20` (0x006ADC20) — row enable state machine (task #3)
- `FUN_006ADF00` (0x006ADF00) — reveal-AI-rows (task #15)
- `FUN_006AE080` (0x006AE080) — hide-AI-rows (task #16)
- `FUN_006AE6E0` (0x006AE6E0) — dialog init (task #1)

Out-of-scope callers: 14 additional callers in wider game code.

(confirmed via `get_function_callers 0x004E4E60`)

## Out-of-scope refs

None — pure lookup, no callees.

## TS-filter

All in-scope callers are the YR offline Skirmish dialog functions. No TS-only
gate. **TS-legacy score: 0.0.**

## Unverified (YELLOW)

None. All claims backed by `decompile_function` and `get_function_callers` / 
`get_function_callees` calls inline.
