# FUN_004E37D0 — Per-Row Country/Flag Combo Control-ID Lookup

## Summary

Pure lookup function: maps a 0-based row index (0–7) to the Win32 dialog control ID
for the country/flag combo in that row of dialog 0x102. Row 0 → 0x6A1 (a separate
control ID for the human-player country slot), rows 1–7 → 0x510, 0x513, 0x51E,
0x514, 0x51F, 0x520, 0x521. No side effects. Called by all three anchor functions
plus the row show/hide cluster — it is the canonical source of country-combo control
IDs for the entire dialog.

## Address

`0x004E37D0` (verified via `decompile_function 0x004E37D0`)

## Active in YR

**Yes.** Called by all three anchor functions for dialog 0x102:
- `FUN_006ACEE0` (0x006ACEE0) — WM_COMMAND dispatcher
- `FUN_006ADC20` (0x006ADC20) — row enable state machine
- `FUN_006ADF00` (0x006ADF00) — reveal-AI-rows
- `FUN_006AE080` (0x006AE080) — hide-AI-rows
- `FUN_006AE6E0` (0x006AE6E0) — dialog init

All in-scope YR-active. Multiple out-of-scope YR callers further confirm.
(Confirmed via `get_function_callers 0x004E37D0`)

## Signature / Parameters

```c
int __fastcall FUN_004e37d0(int param_1)
// param_1 = 0-based row index (0–7)
// returns: Win32 dialog control ID for the country/flag combo in that row
```

No callees — pure computation. (Confirmed via `get_function_callees 0x004E37D0`)

## Row-Index → Control-ID Mapping

Confirmed via `decompile_function 0x004E37D0`:

| Row index (param_1) | Control ID | Notes |
|---|---|---|
| 0 | 0x6A1 | Human-player slot — distinct control from AI rows |
| 1 | 0x510 | AI slot 1 |
| 2 | 0x513 | AI slot 2 |
| 3 | 0x51E | AI slot 3 |
| 4 | 0x514 | AI slot 4 |
| 5 | 0x51F | AI slot 5 |
| 6 | 0x520 | AI slot 6 |
| 7 | 0x521 | AI slot 7 |
| other | 0x521 + delta | overflow case |

Note: the IDs for rows 1–7 match the second switch arm in `FUN_006ACEE0`
(`case 0x510, 0x513, 0x514, 0x51E, 0x51F, 0x520, 0x521, 0x6A1`) — confirming
this function is the authoritative source for that control set.

## Cross-Reference with FUN_006ACEE0

The WM_COMMAND dispatcher (task #2, `fn-006acee0-cmd.md`) handles notifications
for control IDs: `0x510, 0x513, 0x514, 0x51E, 0x51F, 0x520, 0x521, 0x6A1` →
calls `FUN_004E3830` + `FUN_004E3690`. The control IDs returned by this function
exactly match that set, confirming `FUN_004E37D0` is the ID source for the
country/flag combo notification routing.

## Callers (in scope)

- `FUN_006ACEE0` (0x006ACEE0) — WM_COMMAND dispatcher (task #2)
- `FUN_006ADC20` (0x006ADC20) — row enable state machine (task #3)
- `FUN_006ADF00` (0x006ADF00) — reveal-AI-rows (task #15)
- `FUN_006AE080` (0x006AE080) — hide-AI-rows (task #16)
- `FUN_006AE6E0` (0x006AE6E0) — dialog init (task #1)

Out-of-scope callers: 18 additional callers from wider game code
(confirmed via `get_function_callers 0x004E37D0`).

## Out-of-scope refs

None — pure lookup, no callees.

## Unverified (YELLOW)

None. All claims backed by inline Ghidra MCP citations.
