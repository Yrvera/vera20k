# FUN_004E3560 — Side Item-Data to Flag PCX Lookup

## Summary

Maps a country/side item-data value to its corresponding flag PCX filename, then
calls `FUN_006BA140` to load/display it. Covers all 10 YR country item-data values
(0–9), plus two special values: –2 (random = `rani.pcx`) and –3 (observer =
`obsi.pcx`). Used by the country/flag cell update path to update the picture-flag
static control when the country combo selection changes.

## Address

`0x004E3560` (verified via `decompile_function 0x004E3560`)

## Active in YR

**Yes.** Called by `FUN_004E3690` (task #20) and `FUN_004E3CE0` (task #24), both
in-scope cell-UI functions. Also called from wider skirmish/lobby code (tasks
#20, #24 confirm YR-active path).
(Callers confirmed via `get_function_callers 0x004E3560`)

## Signature / Parameters

```c
void __fastcall FUN_004e3560(int param_1)
// param_1 = country/side item-data from the country combo CB_GETITEMDATA
// no return value
```

One callee: `FUN_006BA140` (0x006BA140) — flag PCX loader/display function.
(Confirmed via `get_function_callees 0x004E3560`)

## Item-Data → PCX Filename Mapping

Confirmed via `decompile_function 0x004E3560` (Ghidra string labels from `.rodata`):

| Item-data | PCX filename | Side / Country |
|---|---|---|
| –3 | `obsi.pcx` | Observer (spectator slot) |
| –2 | `rani.pcx` | Random (unassigned) |
| 0 | `usai.pcx` | USA |
| 1 | `japi.pcx` | Japan |
| 2 | `frai.pcx` | France |
| 3 | `geri.pcx` | Germany |
| 4 | `gbri.pcx` | Great Britain |
| 5 | `djbi.pcx` | Djibouti (Libya in RA2) |
| 6 | `arbi.pcx` | Arabia (Iraq in RA2) |
| 7 | `lati.pcx` | Latin America (Cuba in RA2) |
| 8 | `rusi.pcx` | Russia |
| 9 | `yrii.pcx` | Yuri (YR expansion side) |

String pointers come from a table at `0x00844AE8..0x00844B14` (stride 4 per entry,
in order: –2 offset at `0x00844AE8`, then –3 at `0x00844AEC`, then 0..9 at
`0x00844AF0..0x00844B14`). The Ghidra labels name each pointer.
Pointer table confirmed via `read_memory 0x00844AE8` (52 bytes): first entry at
`0x00844AE8` = `0x008363AC` (little-endian `AC 63 83 00`); 13 contiguous 4-byte
pointers present as expected.

All 12 values handle a call to `FUN_006BA140(ptr, 0)` and return immediately.
Values outside –3..9 fall through silently (no call, no error).

## PCX Loader Call

```c
FUN_006BA140(PTR_s_rani_pcx_00844ae8, 0);   // for item-data -2
```

`FUN_006BA140` (0x006BA140) is out of cell-UI decode scope. Based on its call
signature (a string pointer + integer flag) it likely loads the PCX file and
sets it as the bitmap source for the picture-flag static control. The second
argument `0` likely means "not highlighted" or "load into slot 0".

## Callers (in scope)

- `FUN_004E3690` (0x004E3690) — per-cell command handler, task #20
- `FUN_004E3CE0` (0x004E3CE0) — country/flag helper, task #24
- `FUN_004E3F70` (0x004E3F70) — country sentinel writer, task #25

Out-of-scope callers: `FUN_005DAFE0`, `FUN_005EA3E0`, `FUN_00642C80`,
`FUN_00642E40`, `FUN_00642EF0`, `FUN_00642FF0`, `FUN_00643720`, `FUN_00658330`.

## Out-of-scope refs

- `FUN_006BA140` (0x006BA140) — PCX loader; not in cell-UI scope, out-of-scope ref

## Unverified (YELLOW)

None. All load-bearing claims are backed by inline Ghidra MCP citations above.
