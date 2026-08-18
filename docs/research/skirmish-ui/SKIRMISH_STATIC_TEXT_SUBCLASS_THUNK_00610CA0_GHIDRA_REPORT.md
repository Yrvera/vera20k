# Skirmish Static Text Subclass Thunk 0x00610CA0 - Ghidra Research Report

**Address(es):** `0x00610CA0`, `0x00611B3B..0x00611CAF`, `0x00612318..0x0061234B`, `0x006153E0`, `0x0060F9A0`, `0x005E2EF0`, `0x005E2F60`  
**Investigation Mode:** exhaustive-slice  
**Scope:** common subclass thunk path for Skirmish static text controls, with dynamic `0x4B2` text copied into the owner-draw record and then consumed by `OwnerDraw_Static_006153E0`.  
**Non-Scope:** full non-text behavior of `0x00610CA0`, BitFont internals, combo/list custom draw, runtime capture of final CSF strings.  
**Confidence:** High for Skirmish `0x6EC` and `0x5A8` text plumbing; Medium for broader thunk side effects outside this message slice.

## Summary

`0x00610CA0` is the common shell subclass WndProc installed on owner-drawn shell controls by `FUN_0060F9A0`. Ghidra still has no function boundary for the thunk, so this report uses read-only assembly context for the thunk body and decompiled bounded functions for setup, senders, and static paint consumption.

For Skirmish statics `0x6EC` and `0x5A8`, the active path is: Skirmish init or Choose Map refresh sends `0x4B2` to the child static HWND; `0x00610CA0` copies the incoming wide string into heap-owned record text at `+0x28`; if the static is kind `1` and already animating, the thunk kills/restarts the reveal by sending `0x4EE`; then the thunk dispatches the original message to `OwnerDraw_Static_006153E0`, whose `0x4B2/0x4B4` branch refreshes the cached backing surface and invalidates the child. Later `WM_PAINT` draws from the record text.

## Verified Findings

Active in YR: Yes. `FUN_0060F9A0` maps Win32 class `"Static"` to `OwnerDraw_Static_006153E0`, calls `SetWindowLongA(hwnd, GWL_WNDPROC, 0x00610CA0)`, stores the owner proc in `DAT_00AC18C0`, stores the previous WndProc in `DAT_00AC1B48`, creates/updates the per-HWND record in `DAT_00AC1B00`, snapshots initial text with `WM_GETTEXT`, stores translated/copied text through `FUN_00623560`, and sends `0x497`. Evidence: decompile `0x0060F9A0`; assembly context `0x0060FF05` pushes `0x610CA0`, `0x00610333` pushes `0x497`.

Active in YR: Yes. Ghidra has no function at `0x00610CA0`; the readable boundary starts with `SUB ESP,0x36c` at `0x00610CA0`, so the thunk was examined by read-only assembly context rather than by creating a function. Evidence: `decompile_function 0x00610CA0` returns no function; assembly context at `0x00610CA0`.

Active in YR: Yes. `0x4B2` is the wide-string dynamic text update path. The thunk compares message `0x4B2`, reads existing record text from `[EBX+0x28]`, compares against incoming `lParam`, frees old text via `0x007C8B3D` if needed, allocates `wcslen * 2 + 2` bytes via `0x007C8E17`, copies with `0x007CA489`, and resets `[EBX+0x2C]` to `0`. Evidence: assembly context `0x00611BC1..0x00611C67`; helper shape matches `FUN_00623560`.

Active in YR: Conditional. `0x4B4` is the narrow-string sibling update path, not the verified Skirmish `0x6EC/0x5A8` sender path. It frees `[EBX+0x28]`, checks the incoming `char*`, allocates wide storage, converts using the `%hs` formatting path, and sets `[EBX+0x2C] = 1`. This is active for shell controls that send `0x4B4`; the verified Skirmish static text updates in this slice use `0x4B2`. Evidence: assembly context `0x00611B3B..0x00611B8F`; prior shell thunk report's `0x00835874 = L"%hs"` check.

Active in YR: Yes. If a `0x4B2` text change occurs while the record is kind `1` and animation byte `[EBX+0xA8]` is nonzero, the thunk calls `KillTimer(hwnd, 0)`, clears `[EBX+0xA8]`, and sends `0x4EE` to the same HWND to restart the reveal. `SKIRMISH_STATIC_REVEAL_ANIMATION_0X102_GHIDRA_REPORT.md` further verifies that completed reveal kills the timer without clearing the running byte, so a later `0x4B2` text change can restart reveal even after timer completion. Evidence: assembly context `0x00611C72` compares `[EBX+0x70]` to `1`, `0x00611C7C` tests `[EBX+0xA8]`, `0x00611C93` calls through the KillTimer import, `0x00611C99` clears `[EBX+0xA8]`, and `0x00611CA9..0x00611CAF` sends `0x4EE`.

Active in YR: Yes. After the shared text copy path, the thunk calls the stored owner proc with the original `hwnd/msg/wParam/lParam`; this is how `OwnerDraw_Static_006153E0` sees the same `0x4B2`. Evidence: assembly context `0x00612318..0x0061234B`, with the call through import slot `0x007E1488` after pushing the stored owner proc and original message arguments.

Active in YR: Yes. `OwnerDraw_Static_006153E0` handles `0x4B2` and `0x4B4` identically after the thunk copy: if backing surface record `[4]` exists, it refreshes from `DAT_00887310` into the surface and invalidates the child, then returns `1`. It does not perform the actual string copy in this branch. Evidence: decompile `OwnerDraw_Static_006153E0 @ 0x006153E0`, case `0x4B2/0x4B4`.

Active in YR: Yes. The static paint path later consumes record text during `WM_PAINT`: for kind `0` or kind `1`, it requires `piVar11[10]` non-null, and kind `1` additionally requires animation byte `piVar11[0x2A] != 0`; it draws `piVar11[0x19]` through `FUN_00621040`, then advances reveal count and can kill timer `0` when done. Evidence: decompile `OwnerDraw_Static_006153E0 @ 0x006153E0`, `WM_PAINT` text branch.

Active in YR: Yes. Skirmish dialog `0x102` classifies controls `0x6EC` and `0x5A8` as kind-1 animated text statics through `FUN_00602490` and `FUN_0060A5B0`: dialog id `0x102` plus control id `0x6EC` or `0x5A8` returns true, then kind `[+0x70]` is set to `1`, reveal count starts at `1`, animation byte starts clear, and interval/step/range are loaded from the shell helper trio. Evidence: decompile `0x00602490`; decompile `0x0060A5B0`.

Active in YR: Yes. Skirmish init calls both text update helpers near the end of setup, and the successful Choose Map path calls them again after map/session refresh. Evidence: decompile `FUN_006AE6E0 @ 0x006AE6E0` includes `FUN_005E2EF0(); FUN_005E2F60();`; decompile `FUN_006ACEE0 @ 0x006ACEE0` calls the same helpers after accepted `0x5AA`.

Active in YR: Yes. Game-type text `0x6EC` is updated by `FUN_005E2EF0`: when its second argument is nonzero, it obtains `GetDlgItem(parent, 0x6EC)`, gets a wide string from `FUN_007B7140()`, and sends `SendMessageA(child, 0x4B2, 0, lParam)`. Evidence: decompile `0x005E2EF0`.

Active in YR: Yes. Map/scenario text `0x5A8` is updated by `FUN_005E2F60`: it obtains `GetDlgItem(parent, 0x5A8)` and sends `SendMessageA(child, 0x4B2, 0, 0x00A8B322)`. Evidence: decompile `0x005E2F60`.

## Record Fields Used By This Slice

| Field | Meaning in this slice | Active in YR | Evidence |
|---|---|---|---|
| `DAT_00AC18C0` entry `+4` | stored owner proc, `0x006153E0` for Static | Yes | `FUN_0060F9A0`; thunk owner-proc call context `0x00612318..0x0061234B` |
| `DAT_00AC1B48` entry `+4` | previous/original WndProc | Yes | `FUN_0060F9A0`; static default branch decompile |
| `DAT_00AC1B00` record `+0x28` | heap-owned wide text pointer | Yes | `FUN_00623560`; thunk `0x4B2` assembly `0x00611BCD..0x00611C58` |
| record `+0x2C` | text/dirty state, `0` for `0x4B2`, `1` for `0x4B4` | Yes/Conditional | thunk contexts `0x00611B8F`, `0x00611C67` |
| record `+0x70` | static kind; `1` means animated text | Yes | `FUN_0060A5B0`; thunk restart check `0x00611C72` |
| record `+0xA8` | text animation running byte | Conditional | `FUN_0060A5B0`; static `0x4EE`; thunk restart `0x00611C7C..0x00611C99` |

## Cross-Doc Check

The new spot checks agree with `SHELL_SUBCLASS_THUNK_00610CA0_TEXT_UPDATE_PLUMBING_GHIDRA_REPORT.md`: the earlier `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md` deferred the exact thunk copy location, and the shell-thunk report resolved it at `0x00611B3B..0x00611C63`. No contradiction was found for the Skirmish `0x6EC` / `0x5A8` path.

One wording trap remains in older static-paint docs: `OwnerDraw_Static_006153E0`'s `0x4B2/0x4B4` case refreshes the cached surface and invalidates; the actual text ownership/copy happens earlier in the common thunk. Implementations should not put the only text-copy behavior inside the static owner proc.

## Coverage Ledger

| Area | Status | Evidence | Remaining |
|---|---|---|---|
| `0x00610CA0` function boundary | verified absent | decompile error; assembly context start `0x00610CA0` | none, no mutation allowed |
| `0x4B2` wide text copy | verified | assembly `0x00611BC1..0x00611C67` | none |
| animation restart on changed text | verified | assembly `0x00611C72..0x00611CAF` | none |
| owner-proc dispatch after thunk copy | verified | assembly `0x00612318..0x0061234B` | none |
| Static proc consumption/invalidation | verified | decompile `0x006153E0` | none |
| Skirmish `0x6EC` sender | verified | decompile `0x005E2EF0`; callers in `0x006AE6E0`, `0x006ACEE0` | exact final CSF string not runtime-read |
| Skirmish `0x5A8` sender | verified | decompile `0x005E2F60`; callers in `0x006AE6E0`, `0x006ACEE0` | exact map-name buffer lifecycle outside this slice |
| `0x4B4` caller inventory | deferred | out of scope | separate shell-wide pass if needed |

## Open Questions

[RESOLVED] Does the thunk merely forward `0x4B2` to the static proc? No. It copies text into record `+0x28`, updates state, may restart kind-1 animation, and only then calls the stored owner proc. Evidence: assembly `0x00611BC1..0x00611CAF`, `0x00612318..0x0061234B`.

[RESOLVED] Is the `0x4B2` source pointer borrowed until paint? No. The thunk allocates owned storage and copies the wide string immediately. Evidence: assembly `0x00611C47..0x00611C5B`; helper `FUN_00623560`.

[RESOLVED] Are Skirmish `0x6EC` and `0x5A8` active users of this exact path? Yes. Evidence: `FUN_005E2EF0`, `FUN_005E2F60`, `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_00602490`.

[DEFERRED] Exact visible strings after all CSF/map-name updates. Category: runtime/string-table content, outside this thunk slot.

[DEFERRED] Full non-text switch behavior of `0x00610CA0`. Category: out of scope; this slot only covers static text update plumbing.

## Sources

- Ghidra read-only assembly context: `0x00610CA0`, `0x00611B3B`, `0x00611BC1`, `0x00611C63`, `0x00611C72`, `0x00611C8A`, `0x00611C99`, `0x00611CA8`, `0x00611CAF`, `0x00612318`, `0x00612344`, `0x0061234B`, `0x0060FF05`, `0x00610333`.
- Ghidra read-only decompile: `0x006153E0`, `0x0060F9A0`, `0x00623560`, `0x00602490`, `0x0060A5B0`, `0x005E2EF0`, `0x005E2F60`, `0x006AE6E0`, `0x006ACEE0`.
- Prior docs checked: `docs/research/skirmish-ui/SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`, `docs/research/SHELL_SUBCLASS_THUNK_00610CA0_TEXT_UPDATE_PLUMBING_GHIDRA_REPORT.md`, `docs/research/OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`.
- INI files checked: none; this is shell HWND/message/CSF/runtime-buffer behavior, not INI-driven.
