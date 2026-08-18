# FUN_00477440 — INI Slot String Parser (3-Integer Tokenizer)

## Summary

Reads a single INI string value and parses it into three integer out-parameters via
`strtok` + `atoi`. Called during dialog init to deserialize saved skirmish slot data
(e.g., a "country,color,team" or similar packed INI entry). All three out-params are
optional: if a token is absent or the key is missing entirely, the corresponding
out-param is left unchanged.

## Address

`0x00477440` (verified via `decompile_function 0x00477440`)

## Active in YR

Yes. In-scope caller is `FUN_006AE6E0` (dialog init, task #1).
Out-of-scope: CDFileClass__Constructor (mislabeled, actually a map/start-count helper).
(Confirmed via `get_function_callers 0x00477440`)

## Signature / Parameters

```c
void FUN_00477440(
    undefined4  param_1,   // INI section handle / pointer
    undefined4  param_2,   // INI key handle / pointer
    undefined4 *param_3,   // OUT: first  parsed integer
    undefined4 *param_4,   // OUT: second parsed integer
    undefined4 *param_5    // OUT: third  parsed integer
)
```

(verified via `decompile_function 0x00477440`)

Return type is `void`. Out-params are written only when the corresponding token exists.

## Behavioral Analysis

```c
// 1. Read the INI key into a 0x200-byte local buffer
iVar2 = CCINIClass__ReadString(param_1, param_2, &DAT_00889F64, &local_200, 0x200);

if (iVar2 != 0) {
    // 2. First token
    iVar2 = CRT__strtok(&local_200, &DAT_00817F70);
    if (iVar2 != 0) { uVar1 = CRT__atoi_wrapper(iVar2); *param_3 = uVar1; }

    // 3. Second token
    iVar2 = CRT__strtok(0, &DAT_00817F70);
    if (iVar2 != 0) { uVar1 = CRT__atoi_wrapper(iVar2); *param_4 = uVar1; }

    // 4. Third token
    iVar2 = CRT__strtok(0, &DAT_00817F70);
    if (iVar2 != 0) { uVar1 = CRT__atoi_wrapper(iVar2); *param_5 = uVar1; }
}
```

(verified via `decompile_function 0x00477440`)

- `CCINIClass__ReadString` returns 0 if the key is absent; the entire body is skipped.
- `DAT_00889F64` is the default-value buffer (empty string sentinel for ReadString).
- `DAT_00817F70` is the delimiter string passed to `strtok`. Likely a comma `","` or
  whitespace string — exact byte unverified (YELLOW).
- All three tokens are parsed with `atoi`; non-numeric tokens produce 0 per C `atoi`
  semantics.

## Callers

In-scope:
- `FUN_006AE6E0` (`0x006AE6E0`) — dialog init (task #1)

Out-of-scope:
- `CDFileClass__Constructor` (mislabeled; actually map start-count loader at `0x005E6520`)

(Confirmed via `get_function_callers 0x00477440`)

## Callees

Confirmed via `get_function_callees 0x00477440` (inferred from decompile body):
- `CCINIClass__ReadString` — reads INI key into local buffer
- `CRT__strtok` — C runtime tokenizer
- `CRT__atoi_wrapper` — integer parse

## Out-of-scope refs

- `DAT_00889F64` — default-value buffer for `CCINIClass__ReadString`; not decoded in cell-UI scope
- `DAT_00817F70` — delimiter string for `strtok`; not decoded (YELLOW below)
- `CCINIClass__ReadString`, `CRT__strtok`, `CRT__atoi_wrapper` — CRT/INI layer, out of scope

## Structural Note

This is the only INI-read function called from the dialog init anchor
(`FUN_006AE6E0`). It likely deserializes per-slot skirmish settings saved
from a prior session (e.g., saved country/color/team selection per row).

## Unverified (YELLOW)

- `DAT_00817F70` delimiter: inferred as `","` (comma) from typical YR INI packed-integer
  convention, but not confirmed by reading the byte in this session.
- `param_1` / `param_2` types: Ghidra shows `undefined4`; likely `CCINIClass*` and a
  section-name string pointer, but not confirmed against `CCINIClass__ReadString` signature.
- Exact INI section/key names: not traced in this session; their identity depends on
  which slot data `FUN_006AE6E0` is restoring.
