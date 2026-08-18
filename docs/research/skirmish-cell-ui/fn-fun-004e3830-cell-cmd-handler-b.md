# FUN_004E3830 — Country-Combo Control-ID to Row-Index Reverse Lookup

## Summary

Pure inverse lookup: maps a country/flag combo control ID back to its 0-based row
index. This is the exact inverse of `FUN_004E37D0` (task #21). Takes the control ID
from WM_COMMAND's wParam (0x6A1, 0x510, 0x513, 0x51E, 0x514, 0x51F, 0x520, 0x521)
and returns the row index 0–7. Called from the WM_COMMAND dispatcher immediately
after routing to the country-change handler cluster, allowing subsequent per-row
operations to use the row index rather than the raw control ID.

## Address

`0x004E3830` (verified via `decompile_function 0x004E3830`)

## Active in YR

**Yes.** Called by `FUN_006ACEE0` (0x006ACEE0, WM_COMMAND dispatcher, YR-active) and
`FUN_006AE3F0` (0x006AE3F0, DlgProc, YR-active).
(Confirmed via `get_function_callers 0x004E3830`)

## Signature / Parameters

```c
int __fastcall FUN_004e3830(int param_1)
// param_1 = country/flag combo control ID (WM_COMMAND wParam low word)
// returns: 0-based row index (0–7), or 7+delta for unknown IDs
```

No callees — pure computation. (Confirmed via `get_function_callees 0x004E3830`)

## Control-ID → Row-Index Mapping

Confirmed via `decompile_function 0x004E3830`:

| Control ID (param_1) | Row index returned | Corresponds to |
|---|---|---|
| 0x6A1 | 0 | Human-player slot (row 0) |
| 0x510 | 1 | AI slot 1 |
| 0x513 | 2 | AI slot 2 |
| 0x51E | 3 | AI slot 3 |
| 0x514 | 4 | AI slot 4 |
| 0x51F | 5 | AI slot 5 |
| 0x520 | 6 | AI slot 6 |
| 0x521 | 7 | AI slot 7 |

This is the exact inverse of `FUN_004E37D0` (which maps row index → control ID).
Together they form a bidirectional mapping between the 8 country-combo control IDs
and 0-based row indices.

## Relationship to WM_COMMAND Dispatch

In `FUN_006ACEE0` (task #2), the switch cases `0x510, 0x513, 0x514, 0x51E, 0x51F,
0x520, 0x521, 0x6A1` route to this function (`FUN_004E3830`) and then to
`FUN_004E3690` (per-cell command handler, task #20). The call sequence is:
```
FUN_004E3830(param_2)   // param_2 = control ID → row index
FUN_004E3690(...)        // operates on the row
```

## Callers

- `FUN_006ACEE0` (0x006ACEE0) — WM_COMMAND dispatcher (task #2)
- `FUN_006AE3F0` (0x006AE3F0) — DlgProc (task #13, decoder-1)
- `FUN_005E9B60` (0x005E9B60) — out-of-scope caller

## Out-of-scope refs

- `FUN_005E9B60` — caller outside cell-UI scope; not decoded here.

## Unverified (YELLOW)

None. All claims backed by inline Ghidra MCP citations.
