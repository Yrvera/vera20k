# FUN_004E5900 — Combo Item-Data Reader (CB_GETCURSEL + CB_GETITEMDATA)

## Summary

Minimal helper: reads the item-data from a specified combo item (or the current
selection). Takes a dialog HWND, a combo control ID, and an item index. If the
item index is `0xFFFFFFFF` (-1), it first calls `CB_GETCURSEL` (0x147) to get
the currently selected item index, then calls `CB_GETITEMDATA` (0x150) on that
index. Otherwise, calls `CB_GETITEMDATA` directly on the provided index. The
result of `CB_GETITEMDATA` is discarded in the decompile — the actual return
value is in EAX (fastcall convention) and callers use it. Called from both the
DlgProc (`FUN_006AE3F0`) for tooltip dispatch and the WM_COMMAND dispatcher.

## Address

`0x004E5900` (verified via `decompile_function 0x004E5900`)

## Active in YR

**Yes.** Called from `FUN_006ACEE0` (0x006ACEE0, WM_COMMAND dispatcher) and
`FUN_006AE3F0` (0x006AE3F0, DlgProc). Both are in the YR offline Skirmish path.

(confirmed via `get_function_callers 0x004E5900`)

## Signature / Parameters

```c
void __fastcall FUN_004e5900(HWND param_1, int param_2, WPARAM param_3)
// param_1 = dialog 0x102 HWND
// param_2 = combo control ID
// param_3 = item index, or 0xFFFFFFFF (-1) to use current selection
// return: item-data value (in EAX via __fastcall)
```

(verified via `decompile_function 0x004E5900`)

## Behavioral Analysis

```c
if (param_3 == 0xFFFFFFFF) {
    param_3 = SendDlgItemMessageA(param_1, param_2, 0x147, 0, 0); // CB_GETCURSEL
}
SendDlgItemMessageA(param_1, param_2, 0x150, param_3, 0);          // CB_GETITEMDATA
```

When called with `param_3 = -1` (auto-select), the function retrieves the
current selection first. The `CB_GETITEMDATA` result is the effective return
value — callers receive it in EAX.

Despite Ghidra marking the return type as `void`, the function is used as a
query returning the item-data LRESULT. This is a common Ghidra decompile
artifact for fastcall functions that return in EAX without an explicit return
statement.

(verified via `decompile_function 0x004E5900`)

## Usage in context

- **WM_COMMAND dispatcher (`FUN_006ACEE0`)**: used to read item-data from
  start-position or other combos on CBN_SELCHANGE.
- **DlgProc (`FUN_006AE3F0`)**: used in tooltip dispatch to determine which
  item the cursor is over and what string ID to show.

## Callers

| Address | Name | Role |
|---------|------|------|
| 0x005E9CE0 | FUN_005e9ce0 | out-of-scope |
| 0x005ED5A0 | FUN_005ed5a0 | out-of-scope |
| 0x005EE6A0 | FUN_005ee6a0 | out-of-scope |
| 0x006ACEE0 | FUN_006acee0 | WM_COMMAND dispatcher (task #2) |
| 0x006AE3F0 | FUN_006ae3f0 | DlgProc (task #13) |

(confirmed via `get_function_callers 0x004E5900`)

## Callees

| Address | Name | Role |
|---------|------|------|
| Win32 | SendDlgItemMessageA | CB_GETCURSEL (0x147), CB_GETITEMDATA (0x150) |

(confirmed via `get_function_callees 0x004E5900`)

## Out-of-scope refs

None beyond out-of-scope callers listed above.

## TS-filter

Called from YR dialog core functions. No TS-only gate. **TS-legacy score: 0.0.**

## Unverified (YELLOW)

- Return value semantics: Ghidra labels return type `void` but the function is
  used as a query — the `CB_GETITEMDATA` result is in EAX after the call.
  Callers reading EAX confirm the function returns a value; exact calling context
  of each caller not individually verified.
- Task name "start-handler-b": despite the task label, this function is a general
  combo item-data reader, not exclusively a start-position handler. The name
  reflects its position in the manifest's sequential labeling.
