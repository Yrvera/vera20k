# frontier-blitter — Surface/Blitter raster back-end (core-services-map profile)

**Slug:** `frontier-blitter`
**Status:** promoted from catalog stub (`_frontier.md` §A3) to full profile.
**Layer:** ui-render (raster output back-end; pure output, no sim coupling).
**Active in YR:** Yes — this is the *only* pixel-emitting back-end; every visible
frame goes through it. No TS-legacy gating on the core blit path.

> **Verification caveat (this session):** the Ghidra MCP instance was NOT running this
> session (`list_instances` → 0 instances; `connect_instance gamemd` → TCP refused;
> tool group could not load). I could not re-decompile addresses live. Every address
> below is therefore carried from prior-session `[ghidra/verified]` research docs **with
> the original inline verification call cited** (e.g. `read_memory 0x007E85D4`,
> `decompile_function 0x...`). Where a doc cites the exact `read_memory`/`decompile`/
> `disassemble_bytes` call that proved a byte/slot, that is reproduced. Treat every
> address as **located, not re-verified-this-session**; a live re-verification pass is the
> first follow-up when Ghidra is back up. No address here was invented this session.

---

## 1. Purpose

The low-level raster back-end: the `Surface` family (`DSurface` = DirectDraw-backed
primary/back/preview surfaces; `BSurface` = plain memory pixel buffers / SHP / file
scratch) plus the `Blitter` template family (the ~50 small per-mode blitter objects:
opaque, remap, translucent, shadow, RLE, A-buffer/warp, fading, tint, shimmer) and the
final copy to the DirectDraw primary surface and window. Everything visible — tactical
world, sidebar, radar, UI dialogs, previews, movies — composites through this layer. It
owns *how a byte becomes a pixel* (remap LUT indexing, 8→16bpp packing, clipping, lock/
unlock), not *what* to draw.

It is **pure output**: no sim state read or written here. Sim decides *what* and *where*;
this layer decides *how the pixels land*.

---

## 2. What it owns (globals / structs, with addresses)

| Owned thing | Address | Role | Evidence (prior-session call) |
|---|---|---|---|
| `vtable__DSurface` | `0x007E85D4` | DirectDraw surface vtable; installed into primary, back-buffer, sidebar, and decoded-preview surfaces | `read_memory 0x007E85D4`; install at `DSurface__Constructor 0x004BA5A0` store `0x004BA5D0`, primary ctor `0x004BA770` store `0x004BA740` (SKIRMISH_PREVIEW_SURFACE_VTABLE_AND_CLIPPING) |
| `vtable__BSurface` | `0x007E2070` | memory-buffer surface vtable (SHP/PCX/file scratch surfaces) | `read_memory 0x007E2070` → slots `0x00411650, 0x007BBAF0, 0x007BBB90` (SHELL_BUTTON_GREYSCALE_COLORIZATION §2.5) |
| `g_PrimarySurface` | (BSS ptr) | the active draw target; redirected to `g_SidebarSurface` during sidebar draw, then restored | `SidebarClass::Draw 0x006A6C30` decompile (SIDEBAR_DRAW_COMPOSITION_ORDER) |
| `g_SidebarSurface` | (BSS ptr) | retained off-screen surface the sidebar paints into before its blit-to-screen | same |
| `DAT_00887308` / `DAT_00887368` | `0x00887308` / `0x00887368` | screen/primary display-chain targets used by `BlitToScreen` and `RenderFrame_main` | SIDEBAR_BLIT_TO_SCREEN_DIRTY_RECTS; TOOLTIP_MANAGER_SIDEBAR_OVERLAP_PIXELS |
| `g_Blitter_dword_flags` | `0x0081DC24` / `0x0081DC28` | flag words read by the two blitter selectors to pick a mode slot | WARP_TRANSLUCENCY_BLITTER_PIXEL_MATH (`0x0081DC24=0x3000`) |
| Blitter instance bank | inside the surface-context object (`param_1` to the selector) | ~50 pre-allocated per-mode blitter objects at fixed offsets `+0x10..+0x168` | VXL_RASTERIZER_DISPATCH §9; TEMPORAL_WARP_PIPELINE §10 |
| Blitter vtable layouts | `0x007E57F8, 0x007E5B70/B58/B40/B28/B10/B00` (and the extended `0x007E5xxx` set) | per-mode blitter vtables installed by `Blitter_init` | VXL_RASTERIZER_DISPATCH §14 (read_memory of each) |
| Surface remap LUT pointers | `surface+0x174` (`convert_base`) / `surface+0x178` (`remap_lut`) | the ConvertClass palette table + the per-house remap LUT the opaque blitter indexes | HOUSE_COLOR_REMAP_PIPELINE §5 (`0x00491740` decompile) |

**Surface object layout (verified fields):** `+0x04` width, `+0x08` height (`DSurface
+0x7C/+0x80` return these); `+0x174` convert base, `+0x178` remap LUT (consumed by the
opaque blitter); lock state behind `+0x5C/+0x60`. Evidence: SKIRMISH_PREVIEW_SURFACE
§2, HOUSE_COLOR_REMAP_PIPELINE §5.

---

## 3. Key functions + globals (located; not re-verified this session)

### 3a. DSurface vtable slots (the surface "API")
All from `SKIRMISH_PREVIEW_SURFACE_VTABLE_AND_CLIPPING` §2, each with its method
decompile cited there:

| Slot | Target | Role |
|---|---|---|
| `+0x24` | `0x007BAEB0` | PutPixel(x,y,packed): locks via `+0x5C`, writes 16-bit if bpp==2 else 1 byte, unlocks via `+0x60` |
| `+0x28` | `0x007BAE60` | GetPixel(x,y): locks, reads 16-bit/byte, unlocks; returns 0 on null lock |
| `+0x30` | `0x007BA5E0` | DrawLine wrapper: clips to surface bounds via `+0x78`, calls line worker `0x007BA610` (SKIRMISH_PRIMITIVE_BEVEL_SURFACE_VTABLE_0X30) |
| `+0x5C` | `0x004BAD80` | Lock / scanline pointer: rejects negative x/y, locks DD surface, returns `base + pitch*y + bpp*x` |
| `+0x60` | `0x004BAF40` | Unlock: decrements lock depth, unlocks at zero |
| `+0x78` | `0x00411510` | GetRect: fills caller rect with `{0,0,width,height}`, returns it |
| `+0x7C/+0x80` | `0x00411540` / `0x00411550` | width / height accessors (read `+0x04/+0x08`) |

`BSurface` parallel slots (SHELL_BUTTON_GREYSCALE §2.5, SHELL_PCX_BUTTON_TILE §11):
`+0x08` → `0x007BBB90` (memcpy blitter), `+0x5C` → `0x007BBAF0` (Lock→buffer base),
`+0x60` → `0x007BBB90`-area unlock, `+0x74` → `0x00411640` (GetPitch = width×bpp),
`+0x78` → `0x00411510` (GetRect).

### 3b. Surface constructors
- `DSurface__Constructor @ 0x004BA5A0` — preview/offscreen DSurface (installs vtable at
  store `0x004BA5D0`). Verified via SKIRMISH_PREVIEW_SURFACE §2.
- `DSurface__Constructor @ 0x004BA770` / variant `0x004BA900` — **primary DirectDraw
  surface**; calls surface vtable `+0x58` to fill the DD surface descriptor and copies
  the runtime R/G/B shift/loss masks. The 16-bpp pack uses *runtime* masks, not a fixed
  RGBA8 format. Verified via DIRECTDRAW_RUNTIME_PIXEL_FORMAT_MASKS (`0x004BA900`,
  GetBitDepth branch `0x004BA9C4 CMP EAX,0x2`).

### 3c. Blitter selector + bank (the ~50 per-mode objects)
- `Blitter_init @ 0x0048EBF0` — `operator_new`s the 50+ small blitter vtable instances
  into the surface-context object. (VXL_RASTERIZER_DISPATCH §9)
- `Blitter_selector @ 0x00490B90` — `(surface_context, flags)` → blitter object ptr;
  branches on the flag word to pick one of the ~50 slots. (TEMPORAL_WARP_PIPELINE §10,
  VXL_RASTERIZER_DISPATCH §9 — full flag→slot table reproduced below)
- `Blitter_selector_extended @ 0x00490E50` — same shape, slot offsets `+0xC8..+0x168`;
  used by the VXL cache path (RLE-encoded cached pixmaps). (WARP_TRANSLUCENCY §Sources)

**Flag→mode matrix** (VXL_RASTERIZER_DISPATCH §9):

| flag bits | mode | typical use |
|---|---|---|
| `flags & 0x10` | shadow | drawn before unit body |
| `flags & 6 == 2` | translucent/blend | special FX |
| `flags & 6 == 4` | **standard opaque + remap** | **vehicle/building bodies (default)** |
| `flags & 6 == 6` | intensity | bright effects |
| `flags & 1` | RLE | SHP (not VXL) |
| `flags & 0x20` | shimmer/heatwave | mirage cloak |
| `flags & 0x4000` | A-buffer alpha | warp-in / temporal |
| `flags & 0x800` | Z-write variant | depth-writing draws |
| `flags & 0x100` | fading | transitions |
| `flags & 0x40` | dynamic-light tint | iron curtain |
| `flags & 0x8000` | special intensity | cloak |

### 3d. The leaf blitter kernels
- **Standard opaque + remap** `0x00491740`: `if (src_b != 0) *dst = convert_base[ remap_lut[src_b] ]`
  where `remap_lut = surface+0x178`, `convert_base = surface+0x174`. Source byte indexes
  the LUT directly (no `byte-16` adjustment); color-0 = transparency tested on the raw
  source byte. **This is the core observable-output kernel** — house-color remap lands
  here. Verified via HOUSE_COLOR_REMAP_PIPELINE §5.
- **Warp/translucency 50% material**: standard slot `+0xA4`, extended `+0x144`, selected
  for `0x2804`. Verified via WARP_TRANSLUCENCY_BLITTER_PIXEL_MATH §10.

### 3e. Blit-to-screen / final copy
- `SidebarClass__BlitToScreen @ 0x006A70E0` — the stub's representative blit consumer
  (see §1 correction). Copies sidebar-surface-local x/y/w/h rects to `DAT_00887308` using
  `ClientToScreen(g_hWnd)` plus the right-sidebar viewport offset (`g_RadarViewportWidth`
  when the right-sidebar option is set). Branch set: no-op, 168×16 top strip, lower body
  from y=158, current dirty rect, or cached partial dirty rect; **no Soviet branch**.
  Verified via SIDEBAR_BLIT_TO_SCREEN_DIRTY_RECTS (`disassemble` + decompile).
- `RenderFrame_main @ 0x004F4580` (alt entry `0x004F44F0`) — the per-frame compositor:
  copies dirty sidebar/display regions to the display chain (`DAT_00887368->vtable+0x0C`),
  then runs the tooltip manager. Verified via SIDEBAR_DRAW_COMPOSITION_ORDER §1,
  TOOLTIP_MANAGER_SIDEBAR_OVERLAP_PIXELS.

### 3f. Z-buffer dirty management (stub's second representative)
- `Tactical_ZBufferDirtyClear @ 0x006D2B60` — stub-named z-buffer dirty management. NOT
  re-verified this session; carried from the stub. The depth surface itself is the
  `Surface::DrawLine_ABuf*_ZClip*` family family used for selection brackets (PRIMARY_SURFACE_ZBUFFER_BRACKET_OWNERSHIP,
  `Tactical::DrawLine3D 0x006DBB60`). **FLAG:** address `0x006D2B60` is unconfirmed this
  session — verify role (z-buffer clear vs tactical dirty-rect clear) on the next Ghidra
  pass.

---

## 4. Plug point (render pass — NOT a PerTickUpdate rung)

**Out-of-sim render back-end.** Per the LogicClass spine spec, the per-frame order is
`Process_QueuedEvents → RenderFrame → [state-hash record/verify] → PerTickUpdate(ladder)`
(LOGICCLASS_PERTICKUPDATE_SPINE_SPEC, "execution order" line). The blitter therefore runs
inside the **RenderFrame phase**, which executes *before* `LogicClass::PerTickUpdate
@ 0x0055AFB0` in the frame loop — it is **not** any of the 28 ladder rungs (A–AB).

Render entry points that drive it:
- `TacticalClass_Draw @ 0x006D3D10` — world viewport draw → object render loop
  `Tactical_ObjectRenderingLoop @ 0x006D8DB0` → per-object SHP/VXL draw → `Blitter_selector`
  → leaf kernel → DSurface scanline write.
- `MainGame_SidebarDraw @ 0x006D0A30` → `SidebarClass::Draw @ 0x006A6C30` → strip/power
  draws into `g_SidebarSurface` → `SidebarClass__BlitToScreen @ 0x006A70E0`.
- `RadarClass__Draw @ 0x00653100` (radar minimap) and all `GadgetClass`/`shell-dialog`
  dialog draws — same DSurface/BSurface slots.
- `RenderFrame_main @ 0x004F4580` — the final per-frame compositor + display-chain copy.

There is **no per-tick AI head** for this service: it has no `+0x5C` entry on any object
or array; the stub's note "no single AI entry" is correct.

---

## 5. Outgoing edges (this service depends on →)

| → Service | Via symbol | Evidence |
|---|---|---|
| `lookup-tables` | the per-house remap LUT (`surface+0x178`) + ConvertClass palette table (`surface+0x174`) the opaque kernel `0x00491740` indexes; runtime 8→16bpp R/G/B shift/loss masks | HOUSE_COLOR_REMAP_PIPELINE §5; DIRECTDRAW_RUNTIME_PIXEL_FORMAT_MASKS — the xlat/remap/convert tables are exactly the lookup-tables family |
| `drawing-helpers` | the SHP/text/line draw helpers (`CC_Draw_Shape 0x004AED70`, `DrawText 0x004A60E0`, `Tactical::DrawLine3D 0x006DBB60`) call into surface vtable slots `+0x08/+0x24/+0x30` — drawing-helpers is the *caller* layer that sits directly on the blitter primitives | SHELL_BUTTON_GREYSCALE §2.5; PRIMARY_SURFACE_ZBUFFER_BRACKET_OWNERSHIP; SKIRMISH_PRIMITIVE_BEVEL_SURFACE §2 (this is a tight two-way coupling; see §6) |

That is the full outgoing set — the stub's "otherwise self-contained" is accurate. The
blitter reads *tables* (lookup-tables) and is *called by* the draw-helper primitives; it
holds no other service dependency.

---

## 6. Incoming edges (→ this service)

| ← Service | Via symbol | Evidence |
|---|---|---|
| `frontier-render-tactical` | `TacticalClass_Draw 0x006D3D10` → object render loop → `Blitter_selector 0x00490B90` → DSurface scanline write | VXL_RASTERIZER_DISPATCH §9; WARP_TRANSLUCENCY §6 (`TechnoClass::Render 0x00706ED0` calls the selector) |
| `frontier-render-layer` | the z-sorted draw list is walked and each object submits a blit | _frontier.md §A2; same object render loop |
| `frontier-sidebar` | `SidebarClass::Draw 0x006A6C30` paints into `g_SidebarSurface`, then `SidebarClass__BlitToScreen 0x006A70E0` | SIDEBAR_DRAW_COMPOSITION_ORDER §1; SIDEBAR_BLIT_TO_SCREEN_DIRTY_RECTS |
| `frontier-radar` | `RadarClass__Draw 0x00653100` / `RadarClass__RenderCellPixel 0x00655C50` write per-cell pixels to the radar surface | HOUSE_COLOR_REMAP_PIPELINE §5 (radar cell pixel) |
| `gadget-dialog` / `shell-dialog` | every dialog control owner-draw (`FUN_006208F0`, button tiles) calls DSurface/BSurface vtable slots `+0x08/+0x24/+0x30` | SKIRMISH_PRIMITIVE_BEVEL_SURFACE §2; SHELL_PCX_BUTTON_TILE §11 |
| `drawing-helpers` | `CC_Draw_Shape 0x004AED70`, `DrawText 0x004A60E0` resolve to surface vtable blit slots | SHELL_BUTTON_GREYSCALE §2.5 (two-way: drawing-helpers both depends-on and is the principal caller) |
| `frontier-anim` / `frontier-bullet` / `frontier-particle` / `frontier-voxelanim` | each transient-object Draw routes through the same selector/leaf kernels (SHP RLE path for sprites, VXL path for voxel debris) | VXL_RASTERIZER_DISPATCH (voxel); object render loop (sprites) |
| `frontier-mix-vfs` (load-time) | decoded SHP/PCX/preview assets become `BSurface`/`DSurface` source pixels (e.g. PreviewPack decode writes via `+0x24`) | SKIRMISH_PREVIEW_SURFACE §3; MAP_PREVIEWPACK_NORMAL_DECODER |
| `frontier-saveload` | the primary/sidebar surfaces are reconstructed on load (not serialized as pixels); listed for completeness — minimal coupling | (structural) |

This service is a **leaf sink** of the render graph: nearly every UI/render service points
*into* it; it points *out* only to `lookup-tables` and `drawing-helpers`.

---

## 7. Active-in-YR / TS-legacy

- **Core blit path: fully active in YR.** Every frame composites through DSurface +
  `Blitter_selector` + the leaf kernels. No gating.
- **16-bpp DirectDraw with runtime masks: active.** YR primary surface is 16-bpp; the pack
  uses runtime R/G/B shift/loss read from the DD surface descriptor (DIRECTDRAW_RUNTIME_PIXEL_FORMAT_MASKS).
  An 8-bit (palettized) surface path exists in the `+0x24`/`+0x28` code (the `bpp != 2`
  byte branch) but YR runs 16-bpp — the 8-bit branch is effectively dormant in stock YR.
- **No TS-legacy on the core path.** The blitter family is shared engine code, not TS-gated.
  (The A-buffer/warp and shimmer modes are YR-active FX, not legacy.)

---

## 8. Stub corrections

1. **Representative function:** the stub named `SidebarClass__BlitToScreen @ 0x006A70E0`
   as the representative. That is a *consumer* of the back-end (a blit-to-screen copy), not
   the back-end's core. The truer representatives of the service are
   **`Blitter_selector @ 0x00490B90`** (the mode dispatcher) and the **standard opaque
   remap kernel `0x00491740`** (the observable pixel-output kernel) plus the **DSurface
   vtable `0x007E85D4`** (the surface API). `BlitToScreen` is retained as the *final-copy*
   representative. (All three core addresses are corroborated across VXL_RASTERIZER_DISPATCH,
   HOUSE_COLOR_REMAP_PIPELINE, TEMPORAL_WARP_PIPELINE, SKIRMISH_PREVIEW_SURFACE.)
2. **`Tactical_ZBufferDirtyClear @ 0x006D2B60`:** unconfirmed this session — keep but
   re-verify role on next Ghidra pass.
3. **Most-depends-on:** stub said `lookup-tables` only. Correct, but add `drawing-helpers`
   as the second (tight two-way) edge — the SHP/text/line primitives sit directly on the
   surface vtable.

---

## 9. Open items for the live re-verification pass

- Re-run `read_memory 0x007E85D4` and `decompile_function 0x00490B90 / 0x00491740` to
  re-confirm the slot bindings and the opaque kernel this session could not re-check.
- Confirm `Tactical_ZBufferDirtyClear @ 0x006D2B60` identity and role.
- Enumerate the full ~50-slot blitter bank offset table (only the flag→mode branches are
  documented; the exact `+0x10..+0x168` offset→vtable map is partial in
  VXL_RASTERIZER_DISPATCH §9/§14).
