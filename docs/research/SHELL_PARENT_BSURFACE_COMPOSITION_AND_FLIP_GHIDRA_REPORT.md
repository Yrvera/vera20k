# Shell Parent BSurface Composition and Flip — Ghidra Research Report

**Date:** 2026-05-19  
**Primary function:** `WM_PAINT_Handler @ 0x00621E90`  
**Confidence:** High for all six questions — all findings verified by live decompilation.  
**Active in YR:** Yes (standard main-menu shell path, no TS gate).

Prior docs cross-referenced (not re-investigated):
- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md` (line 144 — offscreen cite)
- `RIGHTPANEL_DRAW_AND_LAYOUT_GHIDRA_REPORT.md` (right-panel draw into offscreen)
- `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md` (movie blit path)

---

## 1. Overview

`WM_PAINT_Handler @ 0x00621E90` is the common shell paint handler for all mode-1 dialogs
including `0xE2`. On each `WM_PAINT`, it lazily creates a per-dialog offscreen `BSurface`
(a 16-bpp software surface stored at **dialog record offset `+0x14`**), composes the
right-panel SHP stack and parent background overlay into it, then blits the full surface
to `DAT_00887310` (the global **"AlternateSurface"**). The Bink movie blits separately and
directly — it does NOT go through the per-dialog BSurface.

---

## 2. Answers to the Six Specific Questions

### Q1: Exact field offset of the offscreen BSurface in the dialog record

The dialog record is allocated by `FUN_00624530` as `0x208 bytes` (520 bytes):  
- `record[0x000]` = HWND key (4 bytes)  
- `record[0x004..0x203]` = 512 bytes of window-extra data  
- `record[0x204]` = hash-bucket chain pointer (`record[0x81]` in int-array indexing)

Inside WM_PAINT_Handler, `piVar9 = piVar5 + 1` where `piVar5 = record_base`.  
`piVar9[4]` = `record_base[5]` = byte offset **`0x14`** from the start of the record allocation.

**BSurface pointer is at dialog record byte offset `+0x14`.**

Verified: `decompile_function 0x00621E90` — `piVar9[4]` is both the allocation site target
(`piVar9[4] = (int)piVar5`) and destruction target (`piVar2[5] = 0; vtable[0](1)` in
`FUN_006223C0`). `FUN_006223C0` uses `piVar2[5]` where `piVar2 = record_base` (no `+1` offset),
so `piVar2[5]` = byte offset `5*4 = 0x14`. Both conventions agree.

Additional record offsets established in this investigation:

| Byte offset from record base | Field | Evidence |
|---|---|---|
| `+0x00` | HWND key | hash lookup `*piVar5 == HWND` |
| `+0x14` | offscreen BSurface ptr (this report) | `piVar9[4]` = `piVar2[5]` |
| `+0x20` | init/re-entry guard byte (this report) | `piVar3[8] = 1` in `FUN_0060F4B0` |
| `+0x24` | unused, previously `piVar9[8]` confusion | see §2.Q5 |
| `+0x70` | dialog ID (`record[0x1C]`) | `FUN_0060C540` reads `piVar3[0x1c]` = `0x70 = 28*4` |
| `+0xB4` | paint mode (1 = common shell) | `piVar3[0x2d] = 1` in `FUN_0060C540` |
| `+0xD4` | SDBTNANM overlay flag | `piVar9[0x35]` per RIGHTPANEL doc |

*Note on guard byte:* `piVar9[8]` in `WM_PAINT_Handler` = `(1+8)*4 = 0x24` from record base.
`FUN_0060F4B0` sets this to `1` while enumerating child windows during a dialog reconfigure,
then resets it to `0` at the end. When `piVar9[8] == 1`, `WM_PAINT_Handler` skips all
painting and BSurface allocation to prevent recursive-paint during child subclassing.

### Q2: Allocation timing — INITDIALOG, lazy on first paint, or per-frame?

**Lazy on first paint.** Not at `WM_INITDIALOG`, not per-frame.

```c
// In WM_PAINT_Handler @ 0x00621E90:
piVar5 = (int *)piVar9[4];      // load cached ptr
if (piVar5 == (int *)0x0) {     // NULL → first WM_PAINT → allocate
    piVar5 = operator_new(0x20);
    // ... init fields + PixelBuffer_Init ...
    piVar9[4] = (int)piVar5;    // store: survives all future paints
    DAT_00ac48b4 = DAT_00ac48b4 + 1;  // global allocation counter++
}
// piVar5 is now valid; compose into it
```

`WM_INITDIALOG` (handled in `FUN_00622B50`) does not touch `piVar9[4]`.  
Verified: `decompile_function 0x00621E90`, `decompile_function 0x00622B50`.

### Q3: Surface size

**Screen-sized:** `g_ScreenWidth × g_ScreenHeight`, 16-bpp.

Allocation in `WM_PAINT_Handler`:
```c
GetClientRect(param_1, &local_10);  // dialog 0xE2 was expanded to screen size by FUN_0060C4A0
piVar5[1] = local_10.right;         // width = g_ScreenWidth
piVar5[2] = local_10.bottom;        // height = g_ScreenHeight
PixelBuffer_Init(piVar5 + 5, 0, local_10.right * local_10.bottom * 2);  // 16bpp, 2 bytes/pixel
```

`FUN_0060C4A0` is called from `WM_INITDIALOG` for all mode-1 dialogs including `0xE2`;
it calls `MoveWindow(hwnd, 0, 0, g_ScreenWidth, g_ScreenHeight)`, so `GetClientRect`
returns `{0, 0, g_ScreenWidth, g_ScreenHeight}` on every subsequent paint.

BSurface object layout (from inline construction in `WM_PAINT_Handler`):
- `[0]` = vtable ptr (set to `&vtable__XSurface` then updated to `&vtable__BSurface`)
- `[1]` = width (g_ScreenWidth)
- `[2]` = height (g_ScreenHeight)
- `[3]` = 0
- `[4]` = 2 (pixel format constant)
- `[5+]` = PixelBuffer data region (width × height × 2 bytes)

Object size passed to `operator_new`: `0x20` bytes for the header; the pixel buffer is a
separate allocation inside `PixelBuffer_Init`.

Verified: `decompile_function 0x00621E90` — direct read of `GetClientRect` → field-write
sequence.

### Q4: Per-frame flip sequence — where is the BltFast/Blit-to-main-surface called and with what clip rect?

Full per-frame sequence in `WM_PAINT_Handler` (mode-1 branch, `0xE2` path):

```
1. [lazy] allocate BSurface at record+0x14 if NULL
2. RightPanel__Draw(record_byte_D4 == 0)           — draws SDTP, SDBTNBKGD, SDBTNANM, SDBTM, LWSCRNx into BSurface
3. Background_Overlay(convert, small_shp, large_shp) — draws MNSCRNS/MNSCRNL into BSurface
4. [conditional] Sidebar_TopHighlight() if record byte+0xD5 set
5. [conditional] Minimap_Button() if record byte+0xD6 set  
6. [conditional] RadarBackground() if record byte+0xDB set
7. BLIT: (**(code **)(*DAT_00887310 + 8))(&dest_rect, piVar5, &src_rect, 0, 1)
```

The blit (step 7) is `DAT_00887310->vtable[2]` (vtable byte offset `+8`).

**Dest rect** (`local_40`): window screen coordinates relative to `g_hWnd` client origin,
from `FUN_00775690(param_1, &window_rect)` which calls `GetWindowRect` then subtracts
`ClientToScreen(g_hWnd)`. For dialog `0xE2` at full screen: `{0, 0, g_ScreenWidth, g_ScreenHeight}`.

**Src rect** (`local_20`): `{0, 0, g_ScreenWidth, g_ScreenHeight}` — full BSurface area
(computed as dest_right for width, dest_bottom for height, with left/top forced to zero
because `local_40 = local_30` and `local_3c = local_2c`).

**No sub-rect clipping** is applied. The entire BSurface maps 1:1 to the corresponding
window region of `DAT_00887310`.

`DAT_00887310` is the **"AlternateSurface"** (log string at `0x00827CF8`:
`s_Deleting_AlternateSurface`). It is a `DSurface` (DirectDraw-backed), allocated in
`SidebarSurface_Create @ 0x00533FE2` as `DSurface__Constructor(screen_w, screen_h, 1, 0)`.

Verified: `decompile_function 0x00621E90` (blit site), `decompile_function 0x00533FE2`
(AlternateSurface name + DSurface construction), `decompile_function 0x00560BF0`
(same log string cross-check).

### Q5: Does the offscreen survive across paints (cached), or recomputed every WM_PAINT?

**Cached — survives across paints.** The BSurface is allocated once on first paint and
reused on all subsequent `WM_PAINT` calls until the dialog is destroyed or a resolution
change forces a rebuild.

The `WM_PAINT_Handler` allocation block checks `piVar9[4] == NULL` before calling
`operator_new`. If not NULL, it skips straight to compositing into the existing surface.
The same SHP layers are **re-drawn into the existing surface on every WM_PAINT** —
meaning the pixel content is recomputed each paint, but the buffer allocation itself is
not recycled.

**Lifetime events:**

| Event | Effect on BSurface |
|---|---|
| First `WM_PAINT` | Allocated, stored at record `+0x14`, counter `DAT_00ac48b4++` |
| Subsequent `WM_PAINT` | Existing buffer reused; SHP layers recomposed into it |
| Resolution change | `FUN_006223C0` called per-dialog: `vtable[0](1)` (delete), field cleared to 0, counter `DAT_00ac48b4--` |
| Next `WM_PAINT` after resize | Re-allocated at new screen size |
| Dialog WM_DESTROY | Not explicitly observed in this investigation (deferred — see §7) |

`FUN_006223C0` is the BSurface-only cleanup. It is called in the resolution-change loop
in `FUN_00560BF0` (video mode change), which iterates all registered dialogs:
```c
while (FUN_00775b10() != 0) {
    FUN_006223c0();   // free this dialog's BSurface
    FUN_0060c4a0();   // resize/repaint
    iVar9 = FUN_007759b0();
}
```

Verified: `decompile_function 0x00621E90`, `decompile_function 0x006223C0`,
`decompile_function 0x00560BF0`.

### Q6: Bink movie interaction — does the movie blit go through the offscreen BSurface or directly to DAT_00887310?

**Directly to a different surface — NOT through the per-dialog BSurface and NOT to DAT_00887310.**

The Bink movie path (`0x4F0` → vtable+0x28 → `BinkMovie_ExplicitDraw_005C05F0` →
`BinkMovie_CopyStoredRectToPrimary` → `FUN_00432E40`) blits into the surface stored at
`BinkMovieHandle + 0x0C`, which is set during Bink open (`FUN_00432750`) to either:

- `DAT_00887308` — the primary DirectDraw surface (when `BSurface__Constructor` at
  `BinkMovieHandle+0x20` has a null or zero-vtable BSurface)
- `DAT_0088730C` — the **"HiddenSurface"** (when a valid BSurface exists at `+0x20`)

For the standard main-menu `0x71A` static path, the condition `(piVar6 == 0 || *piVar6 == 0)`
determines which target is chosen. When valid, Bink targets `DAT_0088730C` and the
`FUN_00432C70` fullscreen loop then blits `DAT_0088730C → DAT_00887308` (primary).

**Paint ordering for dialog 0xE2 (per WM_PAINT cycle):**

```
A. FUN_00622B50 handles WM_PAINT (0x0F):
     → calls WM_PAINT_Handler @ 0x00621E90
       → composes RightPanel + Background into per-dialog BSurface (record+0x14)
       → blits BSurface → DAT_00887310 (AlternateSurface)
     → ValidateRect(parent)
     
B. Dialog proc MainMenuDialog0xE2_Proc_00531F60 handles WM_PAINT (0x0F):
     → SendMessage(0x71A, 0x4F0, 0, 0)
       → BinkMovie_ExplicitDraw: copies stored Bink frame → DAT_0088730C (HiddenSurface)
         (or directly to DAT_00887308 if in primary mode)
```

The two paint passes target **completely different surfaces** and do not interfere.
`DAT_00887310` (the SHP shell composition destination) never receives any Bink pixels,
and the per-dialog BSurface is invisible to the Bink path.

Verified: `decompile_function 0x00621E90` (no 0x4F0 or 0x71A reference — confirmed by
BINK_0x4F0 doc §5), `decompile_function 0x00432750` (Bink surface assignment),
`decompile_function 0x00432E40` (vtable-dispatch blit into `param_2` surface),
`decompile_function 0x00533FE2` (AlternateSurface creation log string).

---

## 3. Key Constants and Global Addresses

| Global | Role | Log string / evidence |
|---|---|---|
| `DAT_00887310` | AlternateSurface — receives per-dialog BSurface blit | `s_AlternateSurface___dx_d___s_00827b98` |
| `DAT_0088730C` | HiddenSurface — Bink decode target (non-primary mode) | `s_HiddenSurface___dx_d___s_00827c5c` |
| `DAT_00887308` | Primary DirectDraw surface — Bink direct mode target | `s_Deleting_primary_surface_0082a4b8` |
| `DAT_00ac48b4` | Global BSurface allocation counter | read/write in `WM_PAINT_Handler` and `FUN_006223C0` |
| `0x20` | `operator_new` size for BSurface header object | `WM_PAINT_Handler` allocation block |
| `0x208` | `operator_new` size for dialog record | `FUN_00624530` |
| `0x14` | byte offset of BSurface ptr in dialog record | `piVar9[4]` = `piVar2[5]` convergence |
| `0x20` | byte offset of in-init guard byte in dialog record | `piVar9[8]` = `piVar3[8]` (int-indexed) |

---

## 4. Open Questions — Final State

- `[RESOLVED] Q1` — BSurface field offset = record `+0x14` (evidence: `decompile 0x00621E90` + `decompile 0x006223C0`)
- `[RESOLVED] Q2` — Lazy on first paint (evidence: `decompile 0x00621E90` null-check guard)
- `[RESOLVED] Q3` — Screen-sized, 16bpp (evidence: `GetClientRect` → `PixelBuffer_Init` in `0x00621E90`)
- `[RESOLVED] Q4` — Blit at end of mode-1 branch: `(*DAT_00887310 + 8)`, full BSurface → full window rect (evidence: `decompile 0x00621E90`)
- `[RESOLVED] Q5` — Cached; freed only on resolution change by `FUN_006223C0` (evidence: `decompile 0x006223C0`, `decompile 0x00560BF0`)
- `[RESOLVED] Q6` — Bink blits to `DAT_0088730C`/`DAT_00887308`, not through BSurface and not to `DAT_00887310` (evidence: `decompile 0x00432750`, `decompile 0x00432E40`)
- `[DEFERRED] WM_DESTROY cleanup` — whether the BSurface is freed on dialog WM_DESTROY was not traced in this session (category: `bounded-cost-too-high` for this narrow scope; next-step: trace `WM_DESTROY` handler via xrefs to the hash-table remove function)

---

## 5. Sources

Ghidra functions decompiled in this session:

- `WM_PAINT_Handler @ 0x00621E90`
- `FUN_00622B50` (common shell proc)
- `FUN_0060F4B0` (dialog reconfigure / init-guard set)
- `FUN_00624530` (record insert — allocation size `0x208`)
- `FUN_006223C0` (BSurface-only cleanup on resolution change)
- `FUN_00560BF0` (video mode change — resolution change lifecycle)
- `SidebarSurface_Create @ 0x00533FE2` (AlternateSurface construction)
- `FUN_00432750` (Bink open — target surface assignment)
- `FUN_00432E40` (Bink decode + vtable-dispatch blit)
- `FUN_00433040` (BinkMovie_Update wrapper)
- `BinkMovie_ExplicitDraw_005C05F0 @ 0x005C05F0`
- `FUN_00432C70` (Bink fullscreen playback loop)
- `FUN_00775690` (window rect → client-relative converter)
- `FUN_00624760` (hash table lookup returning `record + 1`)
- `FUN_0060C540` (mode-1 whitelist check + paint mode write)

Memory read:
- `read_memory 0x005C0550` (length 48) — decoded vtable slot +0x14 as MOV+CALL pattern
- `read_memory 0x00887310` (length 4) — confirmed null at static analysis time (runtime pointer)

Prior docs cross-referenced:
- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
- `RIGHTPANEL_DRAW_AND_LAYOUT_GHIDRA_REPORT.md`
- `BINK_0x4F0_PAINT_CADENCE_0x71A_GHIDRA_REPORT.md`
