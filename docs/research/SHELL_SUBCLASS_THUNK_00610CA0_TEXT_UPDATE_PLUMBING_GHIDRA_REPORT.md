# Shell Subclass Thunk 0x00610CA0 Text Update Plumbing - Ghidra Research Report

**Address(es):** `0x00610CA0` common subclass thunk; `0x0060F9A0` setup; `0x006153E0` static owner proc; `0x005E2EF0`, `0x005E2F60`, `0x006AE6E0`, `0x006ACEE0` Skirmish senders  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** read-only resolution of dynamic `0x4B2` / `0x4B4` text update plumbing through the common subclass thunk into per-HWND owner-draw records, with Skirmish statics `0x6EC` and `0x5A8` as the concrete active path.  
**Non-Scope:** BitFont draw internals, static paint pixel details, combo/list/dropdown behavior beyond shared text-message plumbing, full inventory of every `0x4B4` sender.  
**Confidence:** High for thunk dispatch, record text lifetime, previous WndProc fallback, Skirmish `0x6EC`/`0x5A8` activity, and static invalidation. Medium for global shell side effects in the broader thunk outside this text-update slice.  
**Active in YR:** Yes for the common shell subclass path and Skirmish dialog `0x102`; `0x4B4` is conditional on callers that send narrow-string updates.

## 1. Overview

`0x00610CA0` is the universal WndProc installed by `FUN_0060F9A0` on shell controls. For dynamic text messages it is not a passive trampoline: it updates the shared per-control record first, then, when the per-control owner procedure exists, dispatches the same message to that owner procedure through `CallWindowProcA`.

For Skirmish statics `0x6EC` and `0x5A8`, the active update path is `0x4B2` with a wide string pointer. The thunk copies that pointed-to text into owned heap storage at record `+0x28`, so the caller's temporary/string-buffer lifetime does not have to survive the repaint. `OwnerDraw_Static_006153E0` then sees the same `0x4B2`/`0x4B4`, refreshes its backing surface if one exists, invalidates the child, and the next `WM_PAINT` consumes the record text.

## 2. Key Records And Tables

| Storage | Purpose | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00AC18C0` hash table keyed by HWND | owner-draw proc pointer; for Static controls this is `0x006153E0` | `FUN_0060F9A0` stores `pcVar13`; thunk reads it at `0x00610D0A..0x00610D4A` | Yes - installed during shell `WM_INITDIALOG` |
| `DAT_00AC1B48` hash table keyed by HWND | previous/original WndProc returned by `SetWindowLongA(hwnd, GWL_WNDPROC, 0x00610CA0)` | `FUN_0060F9A0`; thunk lookup at `0x00610D56..0x00610D87`; static proc fallback also uses this table | Yes |
| `DAT_00AC1B00` hash table keyed by HWND | full per-control record, keyed by HWND; record data begins at entry `+4` | setup at `0x0060F9A0`; lookup in thunk `0x0061121F..0x00611289` | Yes |
| record `+0x28` | owned wide text buffer pointer used by shared `0x4B2`/`0x4B4` plumbing | helper `FUN_00623560`; thunk text cases `0x00611B3B..0x00611C63` | Yes |
| record `+0x2C` | dirty/text-change state touched by `0x4B4` and reset by `0x4B2` | thunk writes `1` at `0x00611B8F`, writes `0` at `0x00611C67` | Yes |
| record `+0x70` | owner-draw kind; `1` means animated text static | `FUN_0060A5B0` sets `+0x70 = 1`; thunk tests it at `0x00611C72` | Yes for Skirmish `0x6EC`/`0x5A8` |
| record `+0xA8` | text animation running byte | `FUN_0060A5B0`; thunk clears/restarts around `0x00611C7C..0x00611CAF`; static `0x4EE` starts timer | Conditional - only when text animation is already running |

## 3. Text Update Dispatch

Active in YR: Yes. `FUN_0060F9A0` installs `0x00610CA0` by `SetWindowLongA(hwnd, -4, 0x610CA0)` and saves the returned previous WndProc in `DAT_00AC1B48`. The only direct byte-pattern xref to `0x610CA0` is the setup push at `0x0060FF05`, so this is the setup route for the thunk.

Active in YR: Yes. On entry, the thunk reads the owner proc from `DAT_00AC18C0` into a local slot. Later, if the local "call owner proc" flag is still nonzero and the owner proc pointer is nonzero, it calls `CallWindowProcA(ownerProc, hwnd, msg, wParam, lParam)` at `0x00612318..0x0061234B`. This is how `OwnerDraw_Static_006153E0` receives the same `0x4B2`/`0x4B4` after shared text copying.

Active in YR: Yes. `0x4B2` is the wide-string text-set message. At `0x00611BC1..0x00611C63`, the thunk compares the incoming `wchar_t*` against the existing record `+0x28` text pointer: null-to-null and equal-string updates are marked unchanged; otherwise the old heap buffer is freed via `0x007C8B3D`, a new `(wcslen * 2 + 2)` byte buffer is allocated via `0x007C8E17`, and the wide string is copied via `0x007CA489`.

Active in YR: Yes. `0x4B2` resets record `+0x2C` to `0` at `0x00611C67`. If text changed, the control is kind `1`, and animation byte `+0xA8` is set, the thunk kills timer `0`, clears `+0xA8`, and sends `0x4EE` back to the HWND (`0x00611C72..0x00611CAF`). This restarts the typewriter animation for text-animated statics instead of letting an in-progress reveal continue with mismatched text.

Active in YR: Conditional. `0x4B4` is the narrow-string text-set message. At `0x00611B3B..0x00611B8F`, the thunk frees record `+0x28`, tests `char* lParam` for null/empty, allocates `(strlen * 2 + 2)` bytes, and calls a formatting/conversion helper with the UTF-16 format string at `0x00835874` (`L"%hs"`). It then sets record `+0x2C = 1`. This path is active for shell controls that send `0x4B4`; Skirmish `0x6EC`/`0x5A8` use `0x4B2` in the verified paths here.

Active in YR: Yes. After the shared copy path, `OwnerDraw_Static_006153E0` handles `0x4B2` and `0x4B4` identically for statics: if record backing surface `+0x10` exists, it refreshes from `DAT_00887310` into that surface and calls `InvalidateRect(child, NULL, FALSE)`, then returns `1`. Evidence: `0x006153E0`, case `0x4B2/0x4B4`.

Active in YR: Yes. If a message is not handled by the shared thunk/owner-proc route, static controls can still fall back to their previous WndProc. `OwnerDraw_Static_006153E0` looks up `DAT_00AC1B48` and calls `CallWindowProcA(prevProc, hwnd, msg, wParam, lParam)` in its default branch. The thunk itself also has previous-WndProc lookup state, but the material text-update path uses owner-proc dispatch, not previous-proc dispatch.

## 4. Skirmish 0x6EC / 0x5A8 Active Path

Active in YR: Yes. Skirmish dialog `0x102` is classified by `FUN_00602490`: when the dialog record id is `0x102`, control ids `0x6EC` and `0x5A8` return true. `FUN_0060A5B0` then marks those child statics as kind `1` text-animation records (`+0x70 = 1`) with reveal count `1`, timer interval from `FUN_00600CA0`, step from `FUN_006015E0`, and reveal range from `FUN_00601D20`.

Active in YR: Yes. Dialog initialization `FUN_006AE6E0` calls `FUN_005E2EF0()` and `FUN_005E2F60()` near the end of Skirmish setup, after map/session state has been selected. The same two helpers are called again from `FUN_006ACEE0` after successful Choose Map (`0x5AA`) map changes.

Active in YR: Yes. `FUN_005E2EF0` updates game-type/static text `0x6EC`: when its second argument is nonzero, it calls `GetDlgItem(parent, 0x6EC)`, gets a wide string from `FUN_007B7140()`, and sends `SendMessageA(child, 0x4B2, 0, lParam)`. Evidence: `0x005E2EF0` decompile and assembly call site `0x005E2F11`.

Active in YR: Yes. `FUN_005E2F60` updates map/scenario static `0x5A8`: it calls `GetDlgItem(parent, 0x5A8)` and sends `SendMessageA(child, 0x4B2, 0, 0x00A8B322)`. Evidence: `0x005E2F60` decompile and assembly call site `0x005E2F73`.

Active in YR: Yes. `0x00A8B322` is used as the source pointer for the `0x5A8` update; `FUN_006ACEE0` copies the pre-Choose-Map name through local storage and writes it back through `FUN_007CA489(local_200, &DAT_00A8B322)` before running the map choose flow. This report does not claim the semantic contents of every map-name mutation, only that the static update message uses this buffer pointer and the thunk immediately copies from it.

## 5. Open Questions - Final State

[RESOLVED] OQ1 - Does `0x00610CA0` only forward messages? No. It owns shared text-copy behavior for `0x4B2` and `0x4B4`, then conditionally calls the owner proc. Evidence: disassembly `0x00611B3B..0x00611C63`, owner-proc call at `0x00612318..0x0061234B`.

[RESOLVED] OQ2 - Is `0x4B2` pointer lifetime borrowed or copied? Copied. Old record `+0x28` is freed, new heap storage is allocated, and the incoming wide string is copied before repaint. Evidence: `0x00611BCD..0x00611C63`; helper `FUN_00623560` has the same free/allocate/copy shape.

[RESOLVED] OQ3 - Is `0x4B4` the same input type as `0x4B2`? No. `0x4B4` treats `lParam` as `char*`, uses byte `strlen`, allocates a wide buffer, and formats through `L"%hs"` at `0x00835874`. Evidence: `0x00611B3B..0x00611B8F`, memory at `0x00835874`.

[RESOLVED] OQ4 - How do `0x6EC` and `0x5A8` updates reach the static paint path? Sender sends `0x4B2` to child HWND; thunk copies text into record `+0x28`; thunk calls `OwnerDraw_Static_006153E0`; static proc refreshes existing backing surface and invalidates; later `WM_PAINT` draws the record text. Evidence: `0x005E2EF0`, `0x005E2F60`, `0x00610CA0` disassembly, `0x006153E0`.

[RESOLVED] OQ5 - Is this active in standard YR Skirmish? Yes. `FUN_006AE6E0` and `FUN_006ACEE0` call the two Skirmish update helpers, and `FUN_00602490` includes dialog `0x102` plus control ids `0x6EC`/`0x5A8`. Evidence: `0x006AE6E0`, `0x006ACEE0`, `0x00602490`.

[DEFERRED] OQ6 - Full global side effects of `0x00610CA0` for non-text shell messages. Category: out-of-scope. This report only covers text update plumbing, previous WndProc fallback, invalidation, and Skirmish static activity.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x00610CA0` common thunk `0x4B2` branch | verified | disassembly `0x00611BC1..0x00611C63` | none |
| `0x00610CA0` common thunk `0x4B4` branch | verified | disassembly `0x00611B3B..0x00611B8F`; `0x00835874 = L"%hs"` | full caller inventory out of scope |
| owner proc dispatch after shared text copy | verified | disassembly `0x00612318..0x0061234B` | none |
| setup of thunk and hash tables | verified | `FUN_0060F9A0`, xref/pattern hit at `0x0060FF05` | none |
| previous WndProc fallback | verified | `FUN_0060F9A0`; `OwnerDraw_Static_006153E0` default branch | broader non-static fallback not exhausted |
| Static `0x4B2/0x4B4` invalidation | verified | `OwnerDraw_Static_006153E0` case `0x4B2/0x4B4` | none |
| Skirmish `0x6EC` sender | verified | `FUN_005E2EF0`, caller xrefs from `0x006AE6E0` and `0x006ACEE0` | exact CSF final text not decoded |
| Skirmish `0x5A8` sender | verified | `FUN_005E2F60`, `0x00A8B322`, caller xrefs from `0x006AE6E0` and `0x006ACEE0` | exact runtime map name contents not decoded |
| BitFont/static paint internals | deferred | user non-scope; prior docs cover | use prior reports if needed |

## Sources

- Ghidra read-only disassembly / memory: `0x00610CA0..0x00612900`, `0x00835874`, pattern refs to `0x00610CA0`.
- Ghidra decompiled read-only: `FUN_0060F9A0`, `OwnerDraw_Static_006153E0`, `FUN_00623560`, `FUN_00622B50`, `FUN_00602490`, `FUN_0060A5B0`, `FUN_0060AA60`, `FUN_005E2EF0`, `FUN_005E2F60`, `FUN_006AE3F0`, `FUN_006AE6E0`, `FUN_006ACEE0`.
- Prior docs cross-checked: `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`, `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md`, `FUN_0060F9A0_OWNERDRAW_SUBCLASS_SETUP_GHIDRA_REPORT.md`, `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`.
- INI files checked: none; this is shell HWND/message/CSF/runtime-buffer behavior, not INI-driven.
