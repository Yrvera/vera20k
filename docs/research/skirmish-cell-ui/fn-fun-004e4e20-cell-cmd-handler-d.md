# FUN_004E4E20 — Color Combo Item-Data Reader (No Range Guard)

## Summary

Minimal helper: reads the item-data from a color combo for a given item index.
If `param_3` is −1 (0xFFFFFFFF), first calls CB_GETCURSEL to get the currently
selected index. Then calls CB_GETITEMDATA on the resulting index. Unlike
`FUN_004E4170` (task #26, the country-combo equivalent), this function does NOT
return the item-data — it sends the message and discards the result. The function
has no visible side effect on the Ghidra decompile other than these two Win32
calls.

Callers are the WM_COMMAND dispatcher `FUN_006ACEE0` and DlgProc `FUN_006AE3F0`,
suggesting it is called during color combo notification handling.

## Address

`0x004E4E20` (verified via `decompile_function 0x004E4E20`)

## Active in YR

**Yes.** In-scope callers `FUN_006ACEE0` (WM_COMMAND dispatcher, YR-active anchor)
and `FUN_006AE3F0` (DlgProc, YR-active).
(Callers confirmed via `get_function_callers 0x004E4E20`)

## Signature / Parameters

```c
void __fastcall FUN_004e4e20(HWND param_1, int param_2, WPARAM param_3)
// param_1 = dialog 0x102 HWND
// param_2 = color-combo control ID
// param_3 = item index, or 0xFFFFFFFF (−1) to use CB_GETCURSEL
// no return value (void)
```

(verified via `decompile_function 0x004E4E20`)

## Full Decompile

```c
void __fastcall FUN_004e4e20(HWND param_1, int param_2, WPARAM param_3)
{
    if (param_3 == 0xffffffff) {
        param_3 = SendDlgItemMessageA(param_1, param_2, 0x147, 0, 0); // CB_GETCURSEL
    }
    SendDlgItemMessageA(param_1, param_2, 0x150, param_3, 0);         // CB_GETITEMDATA
    return;
}
```

(verified via `decompile_function 0x004E4E20`)

## Behavioral Analysis

The function is structurally parallel to `FUN_004E4170` (task #26), which handles
country combos and returns the item-data to the caller with a range guard. This
function handles color combos but:

1. Has no range guard (no −3..9 check, no vtable call on out-of-range)
2. Does **not** return the item-data — declared `void`, result discarded

The CB_GETITEMDATA call has no observable effect if its result is unused. This is
either a Ghidra decompilation artifact (the LRESULT may be in a register that a
subsequent caller reads via convention), or the function's purpose is to ensure
a CB_GETCURSEL side-effect fires before the item-data query.

The `param_3 == −1` auto-select idiom is identical to `FUN_004E4170` (task #26).

## Callers

In-scope YR-active callers:
- `FUN_006ACEE0` @ `0x006ACEE0` — WM_COMMAND dispatcher (anchor task #2)
- `FUN_006AE3F0` @ `0x006AE3F0` — DlgProc (task #13)

Out-of-scope callers: `FUN_005E9C00`, `FUN_005ED400`, `FUN_005ED5A0`.

(confirmed via `get_function_callers 0x004E4E20`)

## Callees

- `SendDlgItemMessageA` — Win32 (CB_GETCURSEL 0x147, CB_GETITEMDATA 0x150)

## Out-of-scope refs

None.

## Unverified (YELLOW)

- The CB_GETITEMDATA result is discarded in the Ghidra decompile. This may be a
  Ghidra artifact where the LRESULT sits in EAX and is consumed by the caller
  directly (inline assembly convention), or the call is genuinely for side-effects
  only. Not verified against the caller site disassembly.
