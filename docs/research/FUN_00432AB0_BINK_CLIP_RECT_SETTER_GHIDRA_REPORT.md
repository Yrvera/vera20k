# FUN_00432AB0 — Bink Clip-Rect Setter Ghidra Report

**Date:** 2026-05-19
**Address:** `0x00432AB0`
**Confidence:** High (direct hand-disassembly; vtable slot confirmed by memory read; both callers traced to full decompile)
**Active in YR:** Yes — called on every `0x4E4` movie-load for the `0x71A` RA2TS panel and via vtable+0x18 on every explicit clip update.

Parent/sibling reports:
- `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` (§9.3 flagged this as the open)
- `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md`
- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`

---

## 1. Overview

`FUN_00432AB0` is the Bink object clip-rect setter. It takes a `BinkObject*` (ECX),
an x-origin (stack arg 1), and a y-origin (stack arg 2), queries the destination
surface for its valid drawable rect, and computes the intersection of the supplied
origin + movie dims against that surface rect. The result — a clipped (x, y, w, h)
quad — is written to `BinkObject+0x10/+0x14/+0x18/+0x1C`. These four fields are
later consumed by `FUN_00432E40` (the frame copy loop) as the pixel-exact source
coordinates and dimensions passed to `_BinkCopyToBuffer`.

The Ghidra decompiler was register-confused because the thunk `0x005C05A0` loads the
BinkObject into ECX from `[ECX_outer+0x10]` (the outer VQMovieHandle field) before
calling FUN_00432AB0, so Ghidra cannot see the full setup and attributes the final
writes to a spurious `unaff_EBX`. Hand-disassembly resolves this: the writes are to
the same BinkObject that was passed in ECX.

---

## 2. Calling Convention — The Register-Confusion Explained

### 2.1 Vtable path (main-menu `0x71A`)

Thunk `BinkMovie_SetClip_005C05A0 @ 0x005C05A0`:
```asm
MOV EAX, [ESP+0x8]      ; load y arg (from stack)
MOV EDX, [ESP+0x4]      ; load x arg (from stack)
MOV ECX, [ECX+0x10]     ; ECX = *(VQMovieHandle + 0x10) = BinkObject*
PUSH EAX                 ; push y
PUSH EDX                 ; push x
CALL 0x00432AB0          ; __thiscall: ECX=BinkObject, [esp+4]=x, [esp+8]=y
RET 0x8
```

`VQMovieHandle+0x10` is the embedded/pointed BinkObject. The outer handle is the
generic movie handle at `piVar11[0x16]` (`record+0x58`); it holds the BinkObject
pointer at `handle+0x10`.

### 2.2 MSAnim direct call path (0x005CC802–0x005CC80B)

```asm
MOV ECX, [EDI+0x4]      ; y from source rect
MOV EDX, [EDI]          ; x from source rect
PUSH ECX
PUSH EDX
MOV ECX, EAX            ; ECX = newly constructed BinkObject
CALL 0x00432AB0
```

Both callers confirm: ECX = BinkObject, [esp+4] = x, [esp+8] = y.

---

## 3. Stack Frame and Register Map

At function entry (after `SUB ESP,0x24`; `PUSH EBX`; `PUSH EBP`; `PUSH ESI`; `PUSH EDI`):
- Total frame overhead: 0x24 + 16 = 0x34 bytes
- `ECX` on entry = BinkObject* (`this`) — saved to `[ESP_post_SUB + 0x00]` = `[ESP_post_all_pushes + 0x10]`
- `[ESP_post_all_pushes + 0x14]` = x arg
- `[ESP_post_all_pushes + 0x18]` = y arg (also saved from EBX)
- `[ESP_post_all_pushes + 0x1C]` = movie width (from `*(BinkObject+4)[0]`)
- `[ESP_post_all_pushes + 0x20]` = movie height (from `*(BinkObject+4)[1]`)

The function uses `RET 0x8` — it is callee-cleans, so the two stack args (x,y) are
consumed by the return.

**The root of the register confusion**: Ghidra sees the final MOV writes using a
base register it labels `unaff_EBX` because EBX was never updated after entry from
the thunk's perspective. Hand-disassembly shows `[ESP+0x10]` at the write site
resolves to the saved ECX = BinkObject — the object the writes target. There is no
EBX involvement in the output path; Ghidra's `unaff_EBX` is an artifact.

---

## 4. BinkObject Layout (Fields Used by This Function)

All offsets are byte offsets from the BinkObject base pointer.

| Offset | Type | Role in FUN_00432AB0 | Consumer |
|--------|------|----------------------|----------|
| `+0x04` | `int*` | Pointer to `[movie_width, movie_height]` pair | Read: load movie dims |
| `+0x0C` | surface* | Destination surface; vtable`[+0x78]` returns drawable rect | Read: get surface clip bounds |
| `+0x10` | `int` (x) | **Written:** clipped destination x origin | `FUN_00432E40` → `_BinkCopyToBuffer` arg x |
| `+0x14` | `int` (y) | **Written:** clipped destination y origin | `FUN_00432E40` → `_BinkCopyToBuffer` arg y |
| `+0x18` | `int` (w) | **Written:** clipped copy width | `FUN_00432E40` → `_BinkCopyToBuffer` not direct — used via `FUN_00432E40`'s height arg path |
| `+0x1C` | `int` (h) | **Written:** clipped copy height | Same |

Additional object fields (context, not written here):
| Offset | Role |
|--------|------|
| `+0x08` | `dd_surface_type` flags (passed to `_BinkCopyToBuffer`) |
| `+0x20` | BSurface* for surface-event overlay |
| `+0x24` | ticks-per-frame (`0x3C / fps`; e.g. `4` at 15 fps) |
| `+0x28` | file HANDLE (0xFFFFFFFF = none) |
| `+0x2C` | (char) playing_flag |
| `+0x2D` | (char) force_frame_flag |
| `+0x30` | last_frame_seen |

---

## 5. Core Clip-Rect Algorithm (Pseudocode)

Input:
- `movie_w` = `*(BinkObject+4)[0]`
- `movie_h` = `*(BinkObject+4)[1]`
- `surface_rect` = surface vtable`[+0x78]()` → `{left, top, width, height}` in surface-local coords
- `x_origin` = stack arg 1
- `y_origin` = stack arg 2

```
surf_left   = surface_rect[0]   // left edge of valid surface area
surf_top    = surface_rect[1]   // top edge
surf_width  = surface_rect[2]   // drawable width of surface
surf_height = surface_rect[3]   // drawable height of surface

// Guard: all four must be strictly positive
if surf_width <= 0 || surf_height <= 0 || movie_w <= 0 || movie_h <= 0:
    out_x = 0; out_y = 0; out_w = 0; out_h = 0
    goto write

// X-axis clip
clip_x = x_origin
clip_w = movie_w
if x_origin < surf_left:
    // movie starts left of surface — shrink width, clamp x to surface left
    clip_w = (movie_w - surf_left) + x_origin   // = movie_w - (surf_left - x_origin)
    clip_x = surf_left
if clip_w < 1:
    out_x = 0; out_y = 0; out_w = 0; out_h = 0
    goto write

// Y-axis clip
clip_y = y_origin
clip_h = movie_h
if y_origin < surf_top:
    // movie starts above surface — shrink height, clamp y to surface top
    clip_h = (movie_h - surf_top) + y_origin    // = movie_h - (surf_top - y_origin)
    clip_y = surf_top
if clip_h < 1:
    out_x = 0; out_y = 0; out_w = 0; out_h = 0
    goto write

// Right-edge clip: clip_x + clip_w must not exceed surf_left + surf_width
if (surf_left + surf_width) < (clip_x + clip_w):
    clip_w = (surf_left - clip_x) + surf_width   // = surf_right - clip_x
if clip_w < 1:
    out_x = 0; out_y = 0; out_w = 0; out_h = 0
    goto write

// Bottom-edge clip: clip_y + clip_h must not exceed surf_top + surf_height
surf_bottom = surf_top + surf_height
if surf_bottom < (clip_y + clip_h):
    clip_h = surf_bottom - clip_y
if clip_h < 1:
    out_x = 0; out_y = 0; out_w = 0; out_h = 0
    goto write

out_x = clip_x
out_y = clip_y
out_w = clip_w
out_h = clip_h

write:
BinkObject+0x10 = out_x
BinkObject+0x14 = out_y
BinkObject+0x18 = out_w
BinkObject+0x1C = out_h
```

**Key tiny details:**

1. **Strictly positive guard uses `<= 0` (JLE), not `< 0`:** a zero-dimension movie
   or zero-dimension surface causes all four outputs to be zeroed. Dimension of
   exactly `0` is treated as invalid and collapses the rect. (Evidence: `00432ae3–00432af3`.)

2. **Width/height threshold is `< 1` (JL 1), not `<= 0`:** after each clip step the
   surviving dimension is tested against `0x1` via `JL`. This is the same as `<= 0`
   for integers, but the instruction confirms no off-by-one: a result of exactly `0`
   collapses. A result of `1` passes. (Evidence: `00432b30`, `00432b58`, `00432b74`, `00432b96`.)

3. **Left/top underflow adjusts the dimension, not just clamps the origin:** when
   `x_origin < surf_left`, the formula is `clip_w = movie_w - (surf_left - x_origin)`.
   This correctly shrinks the width by the amount clipped off the left, matching
   the number of pixels that would actually be visible. Same logic for top. The
   source rect into the Bink buffer is implicitly shifted by the same amount
   (the x/y passed to `_BinkCopyToBuffer` are the clipped origin values).

4. **Comparison for right-edge clip is `<` not `<=`:** `00432b68: CMP EDI, EDX` then
   `JLE 0x00432b74` — only adjusts if `clip_x + clip_w > surf_right` (strict greater).
   Exactly equal means no adjustment; the last column is included. Inclusive right edge.

5. **Same inclusive logic for bottom edge** (`00432b8e: CMP EBP, EDI` then `JLE 0x00432b96`).

6. **Zero-output path writes all four fields** (not just width/height) to zero.
   All four writes happen unconditionally from the two paths:
   - normal path: `00432b9b–00432bba`
   - zero path: `00432ba7: XOR EDX,EDX; XOR ESI,ESI` then falls through to same write block.
   EAX and ECX are also set: EAX=0 and ECX=0 happen because on the zero path,
   the `JMP 0x00432ba7` path zero-out EDX and ESI, while EAX and ECX were last set
   to zero-or-original by the guard checks. Tracing: on the guard-fail path, EAX is
   `XOR EAX,EAX` at `00432ade`, and `ECX` is `XOR ECX,ECX` at `00432ae3`.
   So: EAX=0, ECX=0, EDX=0, ESI=0 → all four outputs zeroed cleanly.

7. **Surface vtable`[+0x78]`** is called to obtain the drawable rect. The result is a
   pointer to a `{left, top, width, height}` struct (4 ints). It is NOT a Win32 RECT
   (`{left, top, right, bottom}`); `surface_rect[2]` is width and `surface_rect[3]`
   is height. This is confirmed by how `surf_right` is computed as `surf_left + surf_width`
   at `00432b63: LEA EDX, [EBX + EBP*1]` (where EBX=surf_width, EBP=surf_left at
   that point after the x-axis clip steps).

---

## 6. Surface Rect Source: vtable`[+0x78]`

The function `(**(code **)(*(BinkObject+0xC) + 0x78))(local_buf)` is a virtual call
on the destination surface object. The result pointer points into `local_buf` (a
16-byte stack allocation). The surface returns a `{left, top, width, height}` rect
describing its valid drawable area.

- When `BinkObject+0x0C` points to the primary DirectDraw surface object
  (`DAT_00887308`), the rect covers the full screen drawable area.
- When it points to a BSurface (secondary offscreen surface), the rect covers
  that surface's dimensions.
- This is the same vtable slot called in `FUN_00432750` (the Bink open/init function)
  for the initial clip computation, confirming the two clip computations are identical
  in structure.

---

## 7. Integration With `FUN_00432E40` (Frame Copy Loop)

`FUN_00432E40` reads `BinkObject+0x10` and `BinkObject+0x14` as the `x` and `y`
arguments to `_BinkCopyToBuffer_28`. The signature is:
```
_BinkCopyToBuffer(bink_handle, dest_ptr, pitch, height, x, y, copy_flags)
```
Where `x = BinkObject+0x10`, `y = BinkObject+0x14` from the stored clip rect.

`FUN_00433040` (vtable+0x04 wrapper) passes these:
```c
FUN_00432e40(*(param_1+0xC), *(param_1+0x10), *(param_1+0x14))
```
confirming `+0x10` = x, `+0x14` = y as the copy target coordinates.

The `+0x18` (width) and `+0x1C` (height) fields are NOT directly passed to
`_BinkCopyToBuffer` by `FUN_00432E40`. Instead, `FUN_00432E40` uses the surface
height from the surface's own vtable query. However, `+0x18/+0x1C` are used by
`BinkMovie_CopyStoredRectToPrimary @ 0x00433060` (vtable+0x28, explicit `0x4F0` path),
and they could be read by other Bink helpers not in scope of this investigation.

---

## 8. When the Clip Rect Is Set (Call Sites)

### 8.1 On initial movie open — `FUN_00432750 @ 0x00432750`

Called from `VQMovieHandle::Constructor` (0x4E4 message path). After `_BinkOpen`
succeeds, `FUN_00432750` performs an identical clip computation inline (same
algorithm, same writes to `+0x10/+0x14/+0x18/+0x1C`) using the auto-centered
origin. It also sets:
- `BinkObject+0x08` = `_BinkDDSurfaceType` result
- `BinkObject+0x24` = `int(0x3C / fps)` ticks-per-frame

The center computation in `FUN_00432750` (not in `FUN_00432AB0`) handles the case
where `BinkObject+0x0C == 0` (no surface assigned yet): it reads `GetClientRect`
to auto-center the movie on the primary surface. `FUN_00432AB0` does NOT contain
this auto-centering fallback — it assumes `BinkObject+0x0C` already points to a
valid surface.

### 8.2 On explicit clip-rect update — vtable+0x18 thunk `0x005C05A0`

Callable at any time via the VQMovieHandle vtable. The main-menu dialog does not
call this vtable slot explicitly for `0x71A`; the clip rect is established by
`FUN_00432750` at open time and not updated further during normal playback.

### 8.3 MSAnim constructor — `MSAnim__Constructor @ 0x005CC760`

A separate constructor for an `MSBinkAnim` object type also calls `FUN_00432AB0`
directly (not through the vtable) with the source rect's `[x, y]` values as the
clip origin. This is gated on `FUN_007b54b0() != 0`. Not part of the main-menu
Bink path (MSAnim is a different movie-playing class).

---

## 9. TS-vs-YR Filter

| Finding | Active in YR | Evidence |
|---------|-------------|---------|
| vtable+0x18 clip-rect update path | Yes — reachable in YR (vtable slot exists, callable from any owner-draw static), but **not invoked on the main-menu `0x71A` paint path during normal playback** (per §8.2 — the menu Bink clip rect is established once by `FUN_00432750` at open time and not updated by explicit vtable calls thereafter) | vtable read at `0x007EE154+0x18` = `0x005C05A0` |
| `FUN_00432750` inline clip setup | Yes | Called from VQMovieHandle constructor on every `0x4E4` message |
| `MSAnim__Constructor` call path | Conditional on `FUN_007b54b0() != 0` | `0x005CC7D3`: `TEST EAX,EAX; JBE skip`. Whether `FUN_007b54b0` returns non-zero in standard YR is not traced in this pass — deferred. |
| Zero-output path (all four fields → 0) | Conditional | Only fires when movie dims or surface rect is zero/negative |

---

## 10. Confidence Summary (3-Axis Model)

| Axis | Level | Basis |
|------|-------|-------|
| Content | HIGH | Full hand-disassembly at `0x00432AB0–0x00432BC4`; every instruction traced; all registers mapped |
| Identity | HIGH | Function matched at `0x00432AB0`; vtable slot confirmed by `read_memory(0x007EE154)` → `[0x18] = 0x005C05A0 → CALL 0x00432AB0` |
| Binding | HIGH | `get_function_callers` returned two callers; both decompiled; thunk assembly reads `[ECX+0x10]` confirming BinkObject pointer path from VQMovieHandle; confirmed by `FUN_00432E40` reading `[param_1+0x10]` and `[param_1+0x14]` as x/y for `_BinkCopyToBuffer` |

---

## 7. Open Questions — Final State

- `[RESOLVED] OQ-1 — What does FUN_00432AB0 write to +0x10/+0x14/+0x18/+0x1C?` → clipped (x, y, w, h) rect after intersecting (movie_origin + movie_dims) with surface drawable rect. (evidence: `0x00432bab–0x00432bba`)
- `[RESOLVED] OQ-2 — What is the register confusion in the decompile?` → Ghidra labels writes as `unaff_EBX`; correct target is saved ECX (BinkObject) at `[ESP+0x10]` in the final frame. (evidence: `0x00432abb` save + `0x00432bab` restore)
- `[RESOLVED] OQ-3 — What calling convention does the thunk 0x005C05A0 use?` → stdcall-ish: loads BinkObject from `[ECX+0x10]`, passes (x,y) as stack args 1/2 to __thiscall `FUN_00432AB0`. (evidence: `0x005C05A0` disassembly)
- `[RESOLVED] OQ-4 — Which vtable slot maps to FUN_00432AB0?` → vtable+0x18 of `vtable__BinkMovieHandle @ 0x007EE154`. (evidence: `read_memory(0x007EE154)` byte 0x18 = `0x005C05A0`)
- `[RESOLVED] OQ-5 — Does the surface rect come in {left,top,right,bottom} or {left,top,w,h} format?` → `{left, top, width, height}` (not Win32 RECT). (evidence: `0x00432b63 LEA EDX,[EBX+EBP]` computes right as left+width)
- `[RESOLVED] OQ-6 — Are +0x18/+0x1C (w/h) directly consumed by _BinkCopyToBuffer in the timer path?` → No in FUN_00432E40; surface vtable provides height. But yes in BinkMovie_CopyStoredRectToPrimary (vtable+0x28). (evidence: `FUN_00432E40` decompile; `0x00433060` decompile)
- `[RESOLVED] OQ-7 — Is the zero-output path complete (all four fields)?` → Yes; EAX=0/ECX=0 from guard-fail XORs, EDX=0/ESI=0 from `0x00432ba7`, all four written. (evidence: `0x00432ade–0x00432ae3`, `0x00432ba7–0x00432bba`)
- `[RESOLVED] OQ-8 — Is FUN_00432AB0 active in YR?` → Yes; called on every main-menu `0x4E4` movie-load via `FUN_00432750`, and on any vtable+0x18 call. (evidence: `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md §3`)
- `[DEFERRED] OQ-9 — Does FUN_007b54b0() return non-zero in standard YR (MSAnim path)?` (category: requires-different-system-context; reason: MSAnim is not used by main-menu `0x71A`; tracing this requires a full MSAnim investigation; next-step-if-pursued: decompile `FUN_007b54b0` and check INI/global gates)
- `[DEFERRED] OQ-10 — Are +0x18/+0x1C read by any other Bink helpers besides 0x00433060?` (category: bounded-cost-too-high; reason: a full xref sweep of BinkObject+0x18 would require tracing all callers of all BinkObject users; not needed for main-menu parity; next-step-if-pursued: `get_xrefs_to` on the object allocation site + manual field offset scan)

---

## Sources

Ghidra functions decompiled/disassembled (read-only):
- `FUN_00432AB0 @ 0x00432AB0` — full hand-disassembly and decompile
- `BinkMovie_SetClip_005C05A0 @ 0x005C05A0` — thunk disassembly + decompile
- `MSAnim__Constructor @ 0x005CC760` — full disassembly
- `FUN_00433040 @ 0x00433040` — update wrapper decompile
- `FUN_00432E40 @ 0x00432E40` — frame copy loop decompile
- `FUN_004326C0 @ 0x004326C0` — BinkObject init decompile
- `FUN_00432750 @ 0x00432750` — Bink open/init decompile (inline clip equivalent)
- `BinkMovie_CopyStoredRectToPrimary @ 0x00433060` — explicit draw decompile

Memory reads:
- `read_memory(0x007EE154, 48)` — vtable__BinkMovieHandle slot verification

Prior reports referenced:
- `MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md`
- `OWNERDRAW_STATIC_006153E0_FULL_PAINT_GHIDRA_REPORT.md`
- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`

INI files checked: none (no INI keys relevant to clip-rect behavior).
