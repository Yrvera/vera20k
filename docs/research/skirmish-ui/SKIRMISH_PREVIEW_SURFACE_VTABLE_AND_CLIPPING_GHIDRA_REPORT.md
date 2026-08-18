# Skirmish Preview Surface Vtable And Clipping - Ghidra Research Report

**Date:** 2026-05-21  
**Address(es):** `0x00640710`, `0x00641B00`, `0x006418B0`, `0x004BA5A0`, `0x004BB080`, `0x004BB0D0`, `0x007BBE20`, `0x004AED70`, `0x004A61C0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** the concrete preview surface wrapper used by offline Skirmish selected-map preview drawing, the `DSurface` vtable slots consumed by `DrawStartPositions` and PreviewPack load/save (`+0x24`, `+0x28`, `+0x78`, plus blit/lock helpers), and clipping bounds for the decoded preview blit and live `STARTBUT.SHP` overlays in child `0x468`.  
**Non-Scope:** PreviewPack channel-order proof, map-list/Choose Map selection, `mmpb.shp` assigned-player marker flow, and full generic font/SHP blitter internals beyond the bounds passed by this path.  
**Confidence:** High for surface class/vtable slots, preview blit clipping, and overlay clip-rect arguments; Medium for final per-pixel SHP/font internals because the generic blitters are large shared render substrate.  
**Active in YR:** Yes for offline Skirmish preview paint when `DAT_00AC1154` and its inner surface are non-null; live `STARTBUT.SHP` overlays remain conditional on `0 < ScenarioClass+0x113C < 9`.

## 1. Overview

The selected-map preview wrapper owns a concrete `DSurface` object. `[PreviewPack]` decode allocates a `0x24` byte `DSurface` wrapper through `DSurface__Constructor @ 0x004BA5A0`, which installs `vtable__DSurface` at `0x007E85D4`; the 4-byte `DAT_00AC1154` wrapper stores that `DSurface*` at offset `+0`.

`DrawStartPositions @ 0x00640710` first aspect-fits the decoded preview surface into child `0x468` and blits it to `DAT_00887310`. That preview blit is bounded by paired source/destination rectangle clipping. The later live `STARTBUT.SHP` marker and numeric label are not passed the fitted preview rect; they request a fresh full-surface rect from `DAT_00887310 +0x78`, so generic SHP/text clipping is against the destination surface bounds, not the preview image rectangle.

## 2. Surface Class And Key Vtable Slots

| Vtable / slot | Target | Behavior in this slice | Active in YR | Evidence |
|---|---:|---|---|---|
| `vtable__DSurface` | `0x007E85D4` | Installed into decoded preview and primary/backbuffer `DSurface` wrappers. | Yes | `DSurface__Constructor @ 0x004BA5A0`, assembly store `0x004BA5D0`; primary constructor `0x004BA770`, store `0x004BA740` |
| `+0x24` | `0x007BAEB0` | Write one pixel at `(x,y)` after calling `+0x5C`; writes 16-bit if bytes-per-pixel is `2`, else one byte; unlocks via `+0x60`; returns `1` on success. | Yes | PreviewPack load loop calls destination `+0x24` at `0x00641CEF`; method decompile `0x007BAEB0` |
| `+0x28` | `0x007BAE60` | Read one pixel at `(x,y)` after `+0x5C`; returns `0` on null lock pointer; reads 16-bit when bytes-per-pixel is `2`, else byte; unlocks via `+0x60`. | Yes | PreviewPack writer calls source `+0x28` at `0x006418B0`; method decompile `0x007BAE60` |
| `+0x78` | `0x00411510` | Writes full surface bounds `{0,0,width,height}` into caller rect and returns that rect pointer by convention. Extra caller-pushed values are ignored by the callee. | Yes | DSurface vtable bytes at `0x007E85D4+0x78`; method decompile `0x00411510` |
| `+0x7C/+0x80` | `0x00411540` / `0x00411550` | Return wrapper width and height from `+0x04/+0x08`. | Yes | PreviewPack row/column loops at `0x00641C4D`, `0x00641C61`; method decompiles |
| `+0x5C/+0x60` | `0x004BAD80` / `0x004BAF40` | Lock/scanline pointer and unlock. `+0x5C` rejects negative `x/y`, locks DD surface if needed, and returns `base + pitch*y + bytes_per_pixel*x`; `+0x60` decrements lock depth and unlocks when it reaches zero. | Yes | Pixel read/write methods call these; method decompiles `0x004BAD80`, `0x004BAF40` |

## 3. Preview Surface Creation And Pixel IO

Active in YR: Yes. Evidence: offline Skirmish choose/init refresh reaches `0x005E74E0 -> 0x00641EE0 -> 0x00641B00`; prior lifecycle report and fresh decompile of `0x00641B00`.

`0x00641B00` creates the selected-map decoded preview as:

```text
read [Preview] rect
operator_new(0x24)
DSurface__Constructor(width, height, 1, 0)
store DSurface* into wrapper[0]
lock/load PreviewPack/LZO bytes
for y in 0..height:
  for x in 0..width:
    read 3 RGB bytes
    convert to packed DirectDraw pixel
    DSurface vtable +0x24(point, packed_pixel)
```

Important details:

- The surface object is concrete `DSurface`, not `BSurface`; `BSurface` appears as temporary pixel-buffer/SHP/file surfaces elsewhere.
- `+0x24` and `+0x28` both lock one point through `+0x5C` and unlock immediately through `+0x60`, so PreviewPack decode/encode pixel IO is point-wise, not a bulk row pointer write at the caller level.
- `+0x78` on the preview surface gives source bounds `{0,0,preview_width,preview_height}`. `DrawStartPositions` copies that rect and uses its width/height for `*1000` integer aspect-fit math.

## 4. Preview Child `0x468` Blit Bounds

Active in YR: Yes. Evidence: `DrawStartPositions @ 0x00640710` is reached from offline Skirmish `WM_PAINT`; it looks up child `0x468` at `0x00640735..0x00640749`.

Preview image sequence:

1. `GetDlgItem(parent, 0x468)` and `FUN_00775690` convert the child HWND rect into main backbuffer coordinates.
2. The preview surface `+0x78` returns source rect `{0,0,w,h}`.
3. The child rect and source aspect ratio are combined using integer `*1000` scale factors and truncation.
4. `DAT_00887310 +0x14` is called with the fitted destination rect before the blit.
5. `DAT_00887310 +0x8` is called with the fitted destination rect and source full rect.
6. `DAT_00887310 +0x8` dispatches into `DSurface +0x0C` (`0x004BB0D0` body) after asking both surfaces for `+0x78` bounds.

`DSurface +0x0C` performs paired rectangle clipping before the final DirectDraw blit. The helper `ClipRectPair @ 0x007BBE20` adjusts source and destination rectangles together:

- If destination left/top is negative, it advances source left/top by the same amount and shrinks both widths/heights.
- If destination right/bottom exceeds destination surface bounds, it shrinks both widths/heights.
- If source left/top is negative, it advances destination left/top and shrinks both.
- If source right/bottom exceeds source surface bounds, it shrinks both.
- It returns false unless all final widths/heights are positive.

Therefore partially clipped preview pixels are bounded by the intersection of the fitted `0x468` preview rect with the main destination surface and the preview source surface. No pixels are drawn outside that clipped blit rectangle.

## 5. STARTBUT Overlay Bounds

Active in YR: Conditional. Evidence: `DrawStartPositions` only enters the overlay loop when `0 < ScenarioClass+0x113C < 9` at `0x006408D4..0x006408E5`; loose Dustbowl lacks `[Header]` and skips this path.

The marker draw does not reuse the fitted preview rect as its clip rectangle:

- At `0x006409A7..0x006409B6`, `DrawStartPositions` passes a fresh temporary rect and flag `0x400` to `DAT_00887310 +0x78`.
- Because `DAT_00887310` is a `DSurface`, `+0x78 -> 0x00411510`, which fills that temporary rect with the full destination surface bounds `{0,0,screen_w,screen_h}`. The pushed `0x400` is not consumed by this callee.
- `CC_Draw_Shape @ 0x004AED70` receives that returned rect pointer as its clip/bounds argument. It also queries the destination surface `+0x78` internally and clips against the passed rect before selecting the SHP blitter.

Result: live `STARTBUT.SHP` frame `0` is positioned at projected anchor `(-9,-6)` as already documented, but if the frame partially crosses the preview-image edge, this path does not clip it to the preview child rectangle. It is bounded by normal destination-surface/SHP clipping. This corrects older wording that treated `DAT_00887310 +0x78` as a preview-rect draw context.

Numeric labels follow the same broad bound:

- At `0x006409F7..0x00640A04`, the label path again calls `DAT_00887310 +0x78` with a temporary rect.
- `FUN_004A61C0 -> FUN_004A5EB0` sets the text clipping rectangle from the passed rect before drawing the font.
- Thus the label rectangle starts at anchor `(-2,-6)` with pushed extents `8` and `0x19`, but the clipping rect passed from this path is the full destination surface, not the preview child.

## 6. Current Rust Implementation Status

Rust now has `[PreviewPack]` decode helpers (`src/map/preview.rs:154`) and parses four-field `[Preview] Size` as rect dimensions in tests, but `parse_preview_section` still leaves `PreviewSection.decoded = None` at `src/map/preview.rs:93..97`.

The dormant marker helpers in `src/app_skirmish_shell_render.rs` currently gate marker and label emission with `preview_rect.contains(x, y)` before drawing (`:325..331`, `:719..723`). That is not the same as gamemd's generic clipped draw for the whole marker/label candidate. For active live-overlay parity, the safer contract is: submit the marker/label whenever the verified overlay loop says to draw it, then let the renderer clip the resulting pixels. Do not crop `STARTBUT.SHP` to the preview image rectangle.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Concrete selected-map preview surface class | verified | `0x00641B00`, `DSurface__Constructor @ 0x004BA5A0`, vtable store `0x004BA5D0` | none |
| DSurface vtable address and relevant slots | verified | `read_memory 0x007E85D4`, method decompiles `0x007BAEB0`, `0x007BAE60`, `0x00411510` | none for this slice |
| PreviewPack pixel write/read slots | verified | load `0x00641CEF`, writer `0x006418B0`; methods `+0x24/+0x28` | channel order belongs to prior report |
| Child `0x468` preview blit clipping | verified | `0x00640735..0x00640887`, `0x004BB080`, `0x004BB0D0`, `0x007BBE20` | exact DirectDraw driver blit internals below final rect call are deferred |
| STARTBUT overlay clip rect | verified | `0x006409A7..0x006409D2`, DSurface `+0x78 -> 0x00411510`, `CC_Draw_Shape @ 0x004AED70` | generic SHP blitter per-pixel internals deferred |
| Numeric label clip rect | touched-not-exhausted | `0x006409F7..0x00640A15`, `FUN_004A61C0`, `FUN_004A5EB0` | exact glyph pixel edge behavior deferred |
| Older docs claiming marker clip to preview rect | conflict-needs-resolution | `SKIRMISH_MAP_PREVIEW_MARKER_ASSET_LAYOUT_GHIDRA_REPORT.md:109`, this report | patch/synthesis outside this slot |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - What concrete class is `wrapper[0]` for selected-map previews? `DSurface`, with `vtable__DSurface = 0x007E85D4`; allocated as `0x24` bytes by `0x00641B00`. Active in YR: Yes. Evidence: `0x00641B00`, `0x004BA5A0`, `0x004BA5D0`.

[RESOLVED] OQ-2 - What do `+0x24` and `+0x28` do? `+0x24` writes a packed pixel at a point; `+0x28` reads one; both use `+0x5C/+0x60` lock/unlock and branch on bytes-per-pixel. Active in YR: Yes. Evidence: `0x007BAEB0`, `0x007BAE60`, call sites `0x00641CEF`, `0x006418B0`.

[RESOLVED] OQ-3 - What does `+0x78` return for these surfaces? Full surface bounds `{0,0,width,height}`, from fields `+0x04/+0x08`. Active in YR: Yes. Evidence: `0x00411510`; DSurface vtable slot at `0x007E85D4+0x78`.

[RESOLVED] OQ-4 - Are preview pixels clipped to child `0x468`? Yes, through the fitted destination rect plus paired source/destination clipping before the blit. Active in YR: Yes. Evidence: `0x00640852..0x00640887`, `0x004BB0D0`, `0x007BBE20`.

[RESOLVED] OQ-5 - Are live `STARTBUT.SHP` and labels clipped to the preview rect? No. This path asks the destination `DSurface +0x78` for a fresh full-surface rect and passes that to SHP/text draw. Active in YR: Conditional on overlay count. Evidence: `0x006409A7..0x00640A04`, `0x00411510`, `0x004AED70`, `0x004A61C0`.

[DEFERRED] OQ-6 - Exact per-pixel behavior inside the generic SHP and font blitters when candidate pixels hit the destination-surface edge. Category: out-of-scope. Reason: this slot resolves the concrete bounds handed to those blitters; their internal raster edge rules are shared render-substrate work.

## Sources

- Ghidra decompiled/read-only: `0x00640710`, `0x00641B00`, `0x006418B0`, `0x00641DB0`, `0x004BA5A0`, `0x004BA770`, `0x004BAD80`, `0x004BAF40`, `0x007BAEB0`, `0x007BAE60`, `0x00411510`, `0x00411540`, `0x00411550`, `0x004BB080`, `0x004BB0D0` assembly context, `0x007BBE20`, `0x004AED70`, `0x004A61C0`, `0x004A5EB0`.
- Ghidra raw bytes: `read_memory 0x007E85D4` for `vtable__DSurface`; `read_memory 0x007E2198` for base `Surface` comparison.
- Prior docs read: `SKIRMISH_MAP_PREVIEW_MARKER_ASSET_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_MAP_PREVIEW_SOURCE_BOUNDS_AND_PREVIEWPACK_GHIDRA_REPORT.md`, `PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_PREVIEW_OBJECT_LIFECYCLE_DAT_00AC1154_GHIDRA_REPORT.md`, `SKIRMISH_MAP_PREVIEW_START_MARKERS_TRACE.md`.
- Rust status scan only: `src/map/preview.rs`, `src/app_skirmish_shell_render.rs`.
