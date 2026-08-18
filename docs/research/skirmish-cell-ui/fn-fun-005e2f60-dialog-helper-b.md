# FUN_005E2F60 — Dialog Control 0x5A8 Custom-Message Sender (Unconditional)

## Summary

Minimal helper: unconditionally gets control `0x5A8` from dialog `param_1` and
sends custom message `0x4B2` with `wParam = 0` and `lParam = 0xA8B322`
(a hardcoded global address). No guard flag, no side effects beyond the
`SendMessageA` call. Called exclusively from `FUN_006AE6E0` (dialog init,
task #1) and `FUN_006ACEE0` (WM_COMMAND dispatcher, task #2). Structurally
identical to `FUN_005E2EF0` (task #186) except: no `param_2` guard, different
control ID (`0x5A8` vs `0x6EC`), and `lParam` is a hardcoded address rather
than a computed pointer.

## Address

`0x005E2F60` (verified via `decompile_function 0x005E2F60`)

## Active in YR

**Yes.** Sole callers are `FUN_006AE6E0` (init) and `FUN_006ACEE0` (WM_COMMAND
dispatcher) — both core YR offline Skirmish dialog functions.

(confirmed via `get_function_callers 0x005E2F60`)

## Signature / Parameters

```c
void __fastcall FUN_005e2f60(HWND param_1)
// param_1 = dialog 0x102 HWND
```

No guard parameter. Always sends the message when called.

(verified via `decompile_function 0x005E2F60`)

## Behavioral Analysis

```c
hWnd = GetDlgItem(param_1, 0x5a8);
SendMessageA(hWnd, 0x4b2, 0, 0xa8b322);
```

Unconditionally sends message `0x4B2` to control `0x5A8` with a hardcoded
`lParam` of `0x00A8B322`. No conditional branches, no globals read or written,
no return value.

(verified via `decompile_function 0x005E2F60`)

### Control 0x5A8

An unknown dialog control in dialog 0x102. The ID `0x5A8 = 1448` does not
appear in the flag (0x6DA–0x6E1), color-combo (0x6A2, 0x522–0x528),
start-pos-combo (0x6A3–0x6AB), country-combo (0x6A1, 0x510–0x521), or
team-combo (0x76D–0x774) sets documented in the skirmish-cell-ui decode.

### Message 0x4B2

Same custom `WM_USER + 0xB2` message used in `FUN_005E2EF0` (task #186).
Likely a "set data" or "initialize" message for the receiving control's
owner-draw or combo handler.

### lParam 0xA8B322

A hardcoded address `0x00A8B322` passed directly as `lParam`. This falls
within the global data block used by the Skirmish dialog state
(e.g., `DAT_00A8B274` for AI count, `DAT_00A8B29C` for AI country/color
array, `DAT_00A8B394` for slot-0 color). The precise structure at
`0xA8B322` is not decoded in this task.

## Globals Referenced

None accessed directly by this function. `lParam = 0xA8B322` is a literal
address passed to the control; the content at that address is consumed by the
message handler, not by this function.

## Structural Contrast with FUN_005E2EF0

| Aspect | FUN_005E2EF0 (task #186) | FUN_005E2F60 (this task) |
|---|---|---|
| Guard flag | `param_2 != 0` required | None — always sends |
| Control ID | `0x6EC` | `0x5A8` |
| lParam source | `FUN_007B7140()` (computed) | `0xA8B322` (hardcoded) |
| Parameters | `(HWND, int)` | `(HWND)` |

## Callers

| Address | Name | Role |
|---|---|---|
| `0x006AE6E0` | FUN_006ae6e0 | Dialog init (task #1) |
| `0x006ACEE0` | FUN_006acee0 | WM_COMMAND dispatcher (task #2) |

(confirmed via `get_function_callers 0x005E2F60`)

## Callees

| Address | Name | Role |
|---|---|---|
| Win32 | GetDlgItem | Get control 0x5A8 HWND |
| Win32 | SendMessageA | Send message 0x4B2 |

(confirmed via `get_function_callees 0x005E2F60`)

## Out-of-scope refs

- Control `0x5A8` identity — not matched to any known control set in the
  skirmish-cell-ui decode; requires dialog resource layout analysis.
- Message `0x4B2` handler — depends on the window class of control `0x5A8`.
- `0xA8B322` data structure — address is within the Skirmish dialog global
  block but the struct layout at that address is not decoded here.

## TS-filter

Both callers are YR Skirmish dialog core functions. No TS-only gate.
**TS-legacy score: 0.0.**

## Unverified (YELLOW)

- Control `0x5A8` role: not matched to a known control in the dialog layout;
  referred to as "unknown control" pending a dialog resource decode.
- `0xA8B322` struct content: address falls in the Skirmish dialog global block;
  exact layout not read in this session.
- Message `0x4B2` semantics: inferred as a custom WM_USER+N initialization
  message from comparison with other 0x4xx messages in the dialog; handler
  not decoded in this session.
