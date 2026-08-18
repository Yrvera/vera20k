# Bink 0x4F0 Paint Cadence — Static 0x71A on Dialog 0xE2

**Target:** Per-frame Bink draw mechanism for static control 0x71A on dialog 0xE2 (main-menu shell).
**Active in YR:** Yes (main-menu Bink intro movie path, `Ra2ts_s` / `Ra2ts_l`).
**Date:** 2026-05-19
**Scope:** Per-frame update trigger only — not the full Bink subsystem.

---

## 1. Result / Hypothesis Verdict

**The 0x4F0 hypothesis is CONFIRMED, but the sender is the dialog proc — not WM_PAINT_Handler.**

`MainMenuDialog0xE2_Proc_00531F60` (0x00531F60) handles `WM_PAINT` (0x0F) and immediately
sends `SendMessageA(GetDlgItem(hwnd, 0x71A), 0x4F0, 0, 0)` to the static.
`WM_PAINT_Handler` (0x00621E90) does NOT send 0x4F0 at all.
`OwnerDraw_Static_006153E0` (0x006153E0) handles 0x4F0 by calling vtable+0x28 on the movie
handle, which resolves to `BinkMovie_ExplicitDraw_005C05F0` (0x005C05F0) →
`BinkMovie_CopyStoredRectToPrimary`.

---

## 2. Full Two-Stage Pipeline

### Stage 1 — Decode tick (WM_TIMER, ID 0x65, period 0x22 ms ≈ 34 ms / ~29 fps)

When `SendMessageA(hwnd_0x71A, 0x4E4, 0, "Ra2ts_s"/"Ra2ts_l")` is processed by
`OwnerDraw_Static_006153E0`, the handler calls:
```
VQMovieHandle__Constructor(...)  → BinkMovieHandle branch
SetTimer(param_1, 0x65, 0x22, NULL)
```
(verified: decompile_function 0x006153E0, case 0x4E4)

The WM_TIMER handler in `OwnerDraw_Static_006153E0` for `param_3 == 0x65` (timer ID):
```
cVar2 = (**(code **)(*(int *)piVar11[0x16] + 4))();  // vtable+4 = BinkMovie_Update_005C0580
if (cVar2 != '\0') {
    InvalidateRect(param_1, NULL, 0);   // new frame decoded → request repaint
}
```
`BinkMovie_Update_005C0580` (0x005C0580) → `FUN_00433040` (0x00433040) → `FUN_00432e40`
(0x00432e40) which runs the full Bink decode loop: `_BinkWait_4`, `_BinkDoFrame_4`,
`_BinkCopyToBuffer_28`, `_BinkNextFrame_4`. Returns nonzero when a frame was decoded.
(verified: decompile_function 0x006153E0, decompile_function 0x005C0580, decompile_function 0x00433040, decompile_function 0x00432e40)

**Result of Stage 1:** A decoded Bink frame is written to an off-screen buffer.
`InvalidateRect` posts WM_PAINT to the dialog's message queue.

### Stage 2 — Blit tick (WM_PAINT → 0x4F0 → explicit draw)

Windows delivers WM_PAINT (0x0F) to dialog 0xE2. The dialog proc
`MainMenuDialog0xE2_Proc_00531F60` (0x00531F60) handles it:
```c
if (param_2 == 0x0F) {
    pHVar3 = GetDlgItem(param_1, 0x71A);
    SendMessageA(pHVar3, 0x4F0, 0, 0);
}
```
(verified: decompile_function 0x00531F60, confirmed at address 0x005320E5 — contains
`push 0x4F0` opcode `68 F0 04 00 00` via search_byte_patterns)

`OwnerDraw_Static_006153E0` receives 0x4F0 and dispatches:
```c
case 0x4F0:
    if ((int *)piVar11[0x16] != (int *)0x0) {
        (**(code **)(*(int *)piVar11[0x16] + 0x28))();  // vtable+0x28
        return 0;
    }
```
vtable+0x28 on `BinkMovieHandle` (vtable base 0x007EE154, slot index 10) =
`BinkMovie_ExplicitDraw_005C05F0` (0x005C05F0) → `BinkMovie_CopyStoredRectToPrimary`.
(verified: read_memory 0x007EE154 length 64 → byte offset 0x28 = 0x005C05F0;
decompile_function 0x005C05F0)

**Result of Stage 2:** The previously-decoded frame buffer is blitted to the primary surface.

---

## 3. BinkMovieHandle Vtable (0x007EE154) — Relevant Slots

| Byte offset | Address    | Name / role                      | Verified via          |
|-------------|------------|----------------------------------|-----------------------|
| +0x00       | 0x005C0A30 | destructor / constructor         | read_memory 0x7EE154  |
| +0x04       | 0x005C0580 | `BinkMovie_Update_005C0580`      | read_memory 0x7EE154  |
| +0x28       | 0x005C05F0 | `BinkMovie_ExplicitDraw_005C05F0`| read_memory 0x7EE154  |
| +0x14       | 0x005C0570 | IsFinished check (vtable+0x14)   | read_memory 0x7EE154  | <!-- corrected 2026-05-28: was 0x005C0550; binary vtable+0x14 = bytes [70 05 5c 00] = 0x005C0570 via read_memory 0x007EE154 — ROOT_CAUSE: OFFSET_RETYPED_WRONG -->
| +0x1C       | 0x005C05D0 | Loop/restart (vtable+0x1C)       | read_memory 0x7EE154  | <!-- corrected 2026-05-28: was 0x005C0570; binary vtable+0x1C = bytes [d0 05 5c 00] = 0x005C05D0 via read_memory 0x007EE154 — ROOT_CAUSE: OFFSET_RETYPED_WRONG -->

(vtable slot assignments confirmed by read_memory at 0x007EE154, length 64)

---

## 4. Install Path (recap, for cross-reference)

`FUN_00531cc0` (0x00531CC0) creates dialog 0xE2, registers
`MainMenuDialog0xE2_Proc_00531F60` at 0x00531CD4 (confirmed: xrefs_to 0x00531F60 →
`From 00531cd4 in FUN_00531cc0`), then:
```c
pHVar6 = GetDlgItem(pHVar6, 0x71A);
SendMessageA(pHVar6, 0x4E3, 1, 0);        // set looping flag
SendMessageA(pHVar6, 0x4E4, 0, "Ra2ts_l"); // open Bink + SetTimer(0x65, 0x22)
```
(verified: decompile_function 0x00531CC0)

`OwnerDraw_Static_006153E0` case 0x4E3 stores the loop flag. Case 0x4E4 calls
`VQMovieHandle__Constructor`, detects the `.bik` extension, branches to the Bink path,
allocates a `BinkMovieHandle`, calls `FUN_00432750` (0x00432750) for `_BinkOpen_8`,
then `SetTimer(hwnd, 0x65, 0x22, NULL)`.
(verified: decompile_function 0x006153E0 cases 0x4E3/0x4E4; decompile_function 0x00432750)

---

## 5. WM_PAINT_Handler (0x00621E90) — Does NOT send 0x4F0

`WM_PAINT_Handler` at 0x00621E90 handles in-game rendering (`piVar9[0x2c] == 1` sidebar,
`== 2` scenario background, else background PCX). It does NOT contain any SendMessage
call to 0x71A or any 0x4F0 dispatch. The Bink update is entirely driven through the
dialog proc `MainMenuDialog0xE2_Proc_00531F60`.
(verified: decompile_function 0x00621E90 — no 0x4F0 or 0x71A reference)

---

## 6. Timer Cadence vs. Frame Rate

- Timer ID 0x65, period 0x22 = 34 ms → decode tick runs at ~29.4 fps.
- Timer fires → Bink decode; if new frame ready → `InvalidateRect` → WM_PAINT queued.
- WM_PAINT → dialog proc → `SendMessage(0x71A, 0x4F0)` → `BinkMovie_ExplicitDraw`.
- Net result: Bink display rate is gated by the 34 ms Win32 timer, not by the render loop
  or any DirectDraw flip tick. The primary surface update is synchronous with the OS
  paint cycle, not the main game loop.
- `FUN_00432e40` also calls `_BinkWait_4` and loops `while (_BinkWait_4 == 0)`, so the
  actual frame is only advanced when Bink's internal timing says it is ready — the
  34 ms timer merely polls.

**Active in YR:** Yes — standard main-menu path, no TS gate.

---

## 7. Open Questions

1. **`BinkMovie_CopyStoredRectToPrimary` internals** — not fully decompiled; assumed to
   call `_BinkCopyToBuffer_28` or equivalent into the DirectDraw primary. Not needed for
   cadence understanding but relevant for render-pipeline integration.
2. **Looping / end-of-movie** — WM_TIMER 0x65 also checks `(**(code**)(vtable+0x14))()`
   (IsFinished) and if looping flag set (0x4E3 wParam=1), calls vtable+0x1C to restart.
   Not verified beyond the OwnerDraw decompilation.
3. **Multiple 0x4F0 send sites** — `search_byte_patterns F0 04 00 00` returned 24 hits.
   Only one (`0x005320E5`) is a true `push 0x4F0` in the shell path; the others at
   0x0044A0EB, 0x004C0F32 etc. are in different subsystems not examined in this slot.

---

## 8. Load-Bearing Facts (5 max)

1. `MainMenuDialog0xE2_Proc_00531F60` (0x00531F60) sends `SendMessageA(GetDlgItem(hwnd, 0x71A), 0x4F0, 0, 0)` on every WM_PAINT (0x0F). The push is at 0x005320E5 (`68 F0 04 00 00`). Verified: decompile_function 0x00531F60 + search_byte_patterns `68 F0 04 00 00`.

2. `OwnerDraw_Static_006153E0` case 0x4F0 calls `(**(code**)(*(int*)piVar11[0x16] + 0x28))()` = vtable slot +0x28 = 0x005C05F0 (`BinkMovie_ExplicitDraw_005C05F0`). Verified: decompile_function 0x006153E0 + read_memory 0x007EE154.

3. WM_PAINT is itself triggered by `InvalidateRect` inside `OwnerDraw_Static_006153E0`'s WM_TIMER (ID 0x65, 34 ms) handler, after the Bink decode tick (vtable+0x04 = 0x005C0580) returns nonzero indicating a new frame. Verified: decompile_function 0x006153E0.

4. `SetTimer(hwnd_0x71A, 0x65, 0x22, NULL)` is called by OwnerDraw case 0x4E4 immediately after `VQMovieHandle__Constructor` succeeds — this arms the 34 ms decode poll. Verified: decompile_function 0x006153E0 case 0x4E4.

5. `WM_PAINT_Handler` (0x00621E90) does NOT send 0x4F0 to 0x71A; the sender is exclusively the dialog proc. Verified: decompile_function 0x00621E90 — no 0x4F0 or 0x71A reference present.
