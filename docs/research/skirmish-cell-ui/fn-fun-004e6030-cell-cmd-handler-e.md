# FUN_004E6030 - Team Combo Item-Data Reader (CB_GETCURSEL + CB_GETITEMDATA)

## Summary

Minimal helper: reads item-data from a team combo item (or current selection).
Structurally identical to FUN_004E5900 (task 43, start-pos combo reader) but
used for team combo controls. If param_3 == 0xFFFFFFFF (-1), calls CB_GETCURSEL
first to get the current selection index; otherwise uses param_3 directly. Then
calls CB_GETITEMDATA and returns the result in EAX.

## Address

0x004E6030 (verified via decompile_function 0x004E6030)

## Active in YR

Yes. In-scope caller is FUN_006ACEE0 (0x006ACEE0, WM_COMMAND dispatcher, task 2).
(Confirmed via get_function_callers 0x004E6030)

## Signature / Parameters

void __fastcall FUN_004e6030(HWND param_1, int param_2, WPARAM param_3)
  param_1 = dialog 0x102 HWND
  param_2 = team combo control ID (0x76D..0x774)
  param_3 = item index, or 0xFFFFFFFF (-1) to use current selection
  return: item-data value (in EAX via __fastcall; Ghidra shows void)

(verified via decompile_function 0x004E6030)

## Behavioral Analysis

```c
if (param_3 == 0xFFFFFFFF) {
    param_3 = SendDlgItemMessageA(param_1, param_2, 0x147, 0, 0);  // CB_GETCURSEL
}
SendDlgItemMessageA(param_1, param_2, 0x150, param_3, 0);           // CB_GETITEMDATA
```

(verified via decompile_function 0x004E6030)

The CB_GETITEMDATA result is the function's effective return value (in EAX).
Ghidra shows the return type as void because the decompile does not see an
explicit return statement; callers read EAX directly per fastcall convention.

## Structural Parallel

Identical logic to FUN_004E5900 (task 43, start-pos combo reader). Both functions
share the same conditional CB_GETCURSEL -> CB_GETITEMDATA pattern and differ only
in which combo control set they target (team vs. start-position).

## Callees

Confirmed via get_function_callees 0x004E6030:
  SendDlgItemMessageA -- Win32 only

## Callers (in scope)

  FUN_006ACEE0 (0x006ACEE0) - WM_COMMAND dispatcher (task 2)

Out-of-scope: FUN_005E9DB0, FUN_005E9E00, FUN_005ED5A0.
(Confirmed via get_function_callers 0x004E6030)

## Out-of-scope refs

None -- no game function callees.

## Unverified (YELLOW)

  Return value semantics: Ghidra labels return type void but callers use the
  CB_GETITEMDATA result in EAX. Confirmed as a general Ghidra artifact for
  fastcall functions without explicit return statements.
