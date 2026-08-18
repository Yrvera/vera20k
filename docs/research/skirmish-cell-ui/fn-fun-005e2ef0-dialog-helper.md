# FUN_005E2EF0 — Dialog Control 0x6EC Custom-Message Sender

## Summary

Short helper: if `param_2` is non-zero, gets control `0x6EC` from dialog `param_1`,
calls `FUN_007B7140()` to obtain an `LPARAM` value, then sends custom message `0x4B2`
to that control via `SendMessageA`. If `param_2` is zero, returns immediately with no
effect. Called exclusively from `FUN_006AE6E0` (dialog init, task #1) and
`FUN_006ACEE0` (WM_COMMAND dispatcher, task #2).

## Address

`0x005E2EF0` (verified via `decompile_function 0x005E2EF0`)

## Active in YR

**Yes.** Sole callers are `FUN_006AE6E0` (init) and `FUN_006ACEE0` (WM_COMMAND
dispatcher) — both core YR offline Skirmish dialog functions.

(confirmed via `get_function_callers 0x005E2EF0`)

## Signature / Parameters

```c
void __fastcall FUN_005e2ef0(HWND param_1, int param_2)
// param_1 = dialog 0x102 HWND
// param_2 = enable flag; if 0, function is a no-op
```

(verified via `decompile_function 0x005E2EF0`)

## Behavioral Analysis

```c
if (param_2 != 0) {
    hWnd  = GetDlgItem(param_1, 0x6ec);
    lParam = FUN_007b7140();
    SendMessageA(hWnd, 0x4b2, 0, lParam);
}
```

The function is entirely guarded by `param_2 != 0`. When the flag is false the
function is a pure no-op.

(verified via `decompile_function 0x005E2EF0`)

### Control 0x6EC

An unknown dialog control in dialog 0x102. It receives custom message `0x4B2`
with `wParam = 0` and `lParam` produced by `FUN_007B7140`. The control ID `0x6EC`
does not appear in the flag (0x6DA–0x6E1), color, start-pos, country, or team
combo ID sets documented elsewhere in the skirmish-cell-ui decode.

### Message 0x4B2

A custom `WM_USER`-range message (`WM_USER = 0x400`, so `0x4B2 = WM_USER + 0xB2`).
The exact semantics depend on control 0x6EC's window class and its message handler.
In the RA2/YR skirmish dialog, similar `WM_USER+N` messages are used for owner-draw
setup and combo-box data injection (e.g., `0x4C2`, `0x4DD`, `0x4DE` in the color
combos). `0x4B2` may be a "set data" or "initialize" message for this control.

### FUN_007B7140 — lParam producer

```c
undefined2 * __fastcall FUN_007b7140(undefined4 *param_1)
{
    puVar1 = (undefined2 *)*param_1;
    if (puVar1 == NULL) { puVar1 = &DAT_00887734; }
    return puVar1;
}
```

Dereferences `param_1` (ECX — implicit `this`) and returns either the dereferenced
pointer or a fallback global at `DAT_00887734` when null. In context of
`FUN_005E2EF0`, this function is called with no visible argument — ECX carries
whatever is in the fastcall register at the call site. The returned pointer is
passed as `lParam` to `SendMessageA`.

(verified via `decompile_function 0x007B7140`)

## Globals Referenced

None directly. `FUN_007B7140` uses `DAT_00887734` as a fallback but this is
out of scope for this function.

## Callers

| Address | Name | Role |
|---|---|---|
| `0x006AE6E0` | FUN_006ae6e0 | Dialog init (task #1) |
| `0x006ACEE0` | FUN_006acee0 | WM_COMMAND dispatcher (task #2) |

(confirmed via `get_function_callers 0x005E2EF0`)

## Callees

| Address | Name | Role |
|---|---|---|
| `0x007B7140` | FUN_007b7140 | lParam producer (pointer dereference with fallback) |
| Win32 | GetDlgItem | Get control 0x6EC HWND |
| Win32 | SendMessageA | Send message 0x4B2 |

(confirmed via `get_function_callees 0x005E2EF0`)

## Out-of-scope refs

- Control `0x6EC` identity — not matched against any known control set in the
  skirmish-cell-ui decode; would require dialog resource layout analysis to name.
- Message `0x4B2` handler — depends on the window class of control `0x6EC`;
  not decoded in this task.
- `FUN_007B7140` full semantics — decompiled inline; the object it dereferences
  via ECX is not independently identified.
- `DAT_00887734` — fallback pointer target; not decoded in this task.

## TS-filter

Both callers are YR Skirmish dialog core functions. No TS-only gate.
**TS-legacy score: 0.0.**

## Unverified (YELLOW)

- Control `0x6EC` role: not matched to a known control in the dialog layout;
  referred to as "unknown control" pending a dialog resource decode.
- Message `0x4B2` semantics: inferred as a custom WM_USER+N initialization/data
  message from comparison with other 0x4xx messages in the dialog; the actual
  handler not read in this session.
- `FUN_007B7140` ECX source at the call site: Ghidra shows no explicit argument
  pushed before the call in `FUN_005E2EF0`; the implicit ECX value at the call
  site is not traced in this session.
