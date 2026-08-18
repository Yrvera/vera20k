# FUN_004E4170 — Country Combo Item-Data Reader with Range Guard

## Summary

Reads the item-data from a country combo for a given item index. If `param_3`
is −1 (0xFFFFFFFF), first calls CB_GETCURSEL to get the currently selected index,
then uses that as the item index for CB_GETITEMDATA. Returns the item-data directly
if it is in the valid country range −3..9. If out of range and the selected-mode
pointer is set, calls vtable slot `+0x28` on it and returns its result; if the
pointer is null, returns −2 (the "random" sentinel).

This is the read-side counterpart to `FUN_004E3F70` (task #25, the write-side
setter). It is used by the WM_COMMAND dispatcher and DlgProc to query the current
country selection after a change notification, with built-in out-of-range guarding.

## Address

`0x004E4170` (verified via `decompile_function 0x004E4170`)

## Active in YR

**Yes.** In-scope callers include `FUN_006ACEE0` (0x006ACEE0, WM_COMMAND dispatcher,
YR-active anchor) and `FUN_006AE3F0` (0x006AE3F0, DlgProc, YR-active).
(Callers confirmed via `get_function_callers 0x004E4170`)

## Signature / Parameters

```c
LRESULT __fastcall FUN_004e4170(
    HWND   param_1,   // dialog 0x102 HWND
    int    param_2,   // country-combo control ID
    WPARAM param_3    // item index to query, or 0xFFFFFFFF (−1) to use CB_GETCURSEL
)
// returns: item-data value, or result of vtable call if out-of-range
```

(verified via `decompile_function 0x004E4170`)

## Behavioral Analysis

### Full decompile

```c
LRESULT __fastcall FUN_004e4170(HWND param_1, int param_2, WPARAM param_3)
{
    LRESULT LVar1;

    if (param_3 == 0xffffffff) {
        param_3 = SendDlgItemMessageA(param_1, param_2, 0x147, 0, 0); // CB_GETCURSEL
    }
    LVar1 = SendDlgItemMessageA(param_1, param_2, 0x150, param_3, 0); // CB_GETITEMDATA
    if ((LVar1 < -3) || (9 < LVar1)) {
        if (DAT_00a8b23c != (int *)0x0) {
            LVar1 = (**(code **)(*DAT_00a8b23c + 0x28))();
            return LVar1;
        }
        LVar1 = -2;   // fallback: return "random" sentinel
    }
    return LVar1;
}
```

(verified via `decompile_function 0x004E4170`)

### param_3 == −1: auto-select current item

When `param_3 == 0xFFFFFFFF` (−1 as WPARAM), the function calls CB_GETCURSEL
(0x147) on the combo to retrieve the currently selected index, then uses that as
the item index for CB_GETITEMDATA. This allows callers to pass −1 as a "give me
the current selection" shorthand without a separate CB_GETCURSEL call.

### Out-of-range return paths

Valid item-data range is −3..9 (observer, random, 0–9 countries — matches the PCX
mapping in `FUN_004E3560`, task #19). When out of range:

1. If `DAT_00A8B23C` (selected-mode pointer) is non-null: calls vtable `+0x28` and
   returns its return value directly.
2. If `DAT_00A8B23C` is null: returns −2 (the "random" sentinel value, matching
   `rani.pcx` in `FUN_004E3560`).

The two out-of-range paths differ in return value — if the vtable call is made
its return is forwarded; otherwise −2 is hardcoded. The caller is expected to
use the returned value to update UI state.

## Globals Accessed

| Global | Address | Usage |
|---|---|---|
| `DAT_00A8B23C` | `0x00A8B23C` | Selected-mode pointer; vtable `+0x28` called when item-data out of range |

(confirmed via `decompile_function 0x004E4170`)

## Callers

In-scope YR-active callers:
- `FUN_006ACEE0` @ `0x006ACEE0` — WM_COMMAND dispatcher (anchor task #2)
- `FUN_006AE3F0` @ `0x006AE3F0` — DlgProc (task #13)

Out-of-scope callers: `FUN_005DBF50`, `FUN_005E8EA0`, `FUN_005E9B60`,
`FUN_005ED400`, `FUN_0077ED00`.

(confirmed via `get_function_callers 0x004E4170`)

## Callees

- `SendDlgItemMessageA` — Win32 (CB_GETCURSEL 0x147, CB_GETITEMDATA 0x150)
- Vtable call `*DAT_00A8B23C + 0x28` — selected-mode method (out of scope, task #68)

No in-scope helper callees; function is self-contained.

## Out-of-scope refs

- Vtable `+0x28` on `DAT_00A8B23C` — selected-mode vtable; covered by task #68

## Unverified (YELLOW)

- Return value of vtable call `*DAT_00A8B23C + 0x28`: decompile forwards it
  directly as `LVar1`; its type and meaning are unknown without reading the vtable.
- Why the null-pointer fallback returns −2 specifically: inferred as "random"
  sentinel from `FUN_004E3560` mapping; no explicit label in binary.
