# MAP PreviewPack Normal Decoder Channel Order - Ghidra Research Report

**Date:** 2026-06-01  
**Address(es):** `0x00641B00` (normal PreviewPack load/decode), `0x004BA770` (DSurface__Constructor, global initializer), `0x006418B0` (PreviewPack write/encode), `0x005E74E0` (selected-map preview wrapper loader)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Byte→channel mapping in the normal (non-random) `[PreviewPack]` decode path — which decompressed byte is red, which is green, which is blue; whether the source is 3 bytes/pixel and the surface format; and how Rust currently maps these bytes.  
**Non-Scope:** Random-map preview (`RandMap.img` PCX path), network preview transfer, `STARTBUT.SHP` overlay projection, DirectDraw surface subclass internals beyond the shift/loss globals.  
**Confidence:** High for channel order (verified from both decode assembly and encode assembly, plus global initializer cross-check).  
**Active in YR:** Yes. The path is reached unconditionally on normal (non-random) stock-map selection in the standard offline Skirmish dialog.

## Summary

OQ-6 resolution: The decompressed `[PreviewPack]` pixel byte stream is row-major **RGB**, not BGR. After LZO decompression, byte 0 = red, byte 1 = green, byte 2 = blue. Rust `src/map/preview.rs` already uses `PREVIEW_CHANNEL_ORDER = PreviewChannelOrder::Rgb` and expands triples as `[r,g,b,255]`. **Rust matches gamemd; no fix needed.**

## 1. Key Offsets / Globals

| Item | Purpose | Active in YR | Evidence |
|---|---:|---|---|
| `0x00641B00` | Normal PreviewPack load/decode; reads 3-byte triples from LZO straw, converts to packed DirectDraw pixel, writes surface via vtable +0x24. | Yes | decompile + disassemble `0x00641B00` |
| `0x008A0DD0` / `0x008A0DD4` | DirectDraw **Red** shift / loss globals | Yes | `DSurface__Constructor` @ `0x004BA9D6` writes these from R bitmask `DAT_008A0958`; decode loop `0x00641CD1`/`0x00641CDD` reads them for byte[0] |
| `0x008A0DE0` / `0x008A0DE4` | DirectDraw **Green** shift / loss globals | Yes | `DSurface__Constructor` @ `0x004BAA0F`/`0x004BAA15` writes from G bitmask `DAT_008A095C`; decode loop `0x00641CC1`/`0x00641CB7` reads for byte[1] |
| `0x008A0DD8` / `0x008A0DDC` | DirectDraw **Blue** shift / loss globals | Yes | `DSurface__Constructor` @ `0x004BAA49`/`0x004BAA4E` writes from B bitmask `DAT_008A0960`; decode loop `0x00641CA3`/`0x00641C95` reads for byte[2] |
| `FUN_0055C7C0` | LZOStraw read of exactly 3 bytes per pixel into `[ESP+0x10..0x12]`. | Yes | `0x00641C6C..0x00641C7C` |

## 2. Core Logic

### 2.1 Decode path inner loop (`0x00641B00`)

Active in YR: Yes. Evidence: disassembly `0x00641B00`.

The decode loop calls `FUN_0055C7C0([ESP+0x10], 3)` at `0x00641C77`. If fewer than 3 bytes are returned the function aborts (branch at `0x00641C7F`). Otherwise the 3-byte buffer at `[ESP+0x10]`..`[ESP+0x12]` is consumed as follows (verified from disassembly `0x00641C89..0x00641CEF`):

```
00641C89: MOV EBX,dword ptr [ESP + 0x12]   ; byte[2] → EBX
00641C8D: MOV EBP,dword ptr [ESP + 0x11]   ; byte[1] → EBP
    -- EBX processed with BLoss/BShift --
00641C95: MOV CL,byte ptr [0x008A0DDC]     ; BLoss → CL
00641CA3: MOV ECX,dword ptr [0x008A0DD8]   ; BShift → ECX
00641CB7: MOV CL,byte ptr [0x008A0DE4]     ; GLoss → CL
00641CC1: MOV ECX,dword ptr [0x008A0DE0]   ; GShift → ECX
    -- EBP processed with GLoss/GShift --
00641CD1: MOV EBP,dword ptr [ESP + 0x10]   ; byte[0] → EBP (reused register)
00641CC9: MOV CL,byte ptr [0x008A0DD4]     ; RLoss → CL
00641CDD: MOV ECX,dword ptr [0x008A0DD0]   ; RShift → ECX
    -- EBP processed with RLoss/RShift --
```

Channel assignment summary (verified via `disassemble_function 0x00641B00`):

| Decompressed byte position | Stack slot | Loss global | Shift global | Channel |
|---:|---|---|---|---|
| byte 0 | `[ESP+0x10]` | `0x008A0DD4` (RLoss) | `0x008A0DD0` (RShift) | **Red** |
| byte 1 | `[ESP+0x11]` | `0x008A0DE4` (GLoss) | `0x008A0DE0` (GShift) | **Green** |
| byte 2 | `[ESP+0x12]` | `0x008A0DDC` (BLoss) | `0x008A0DD8` (BShift) | **Blue** |

The serialized stream is therefore row-major **RGB** (not BGR).

Active in YR: Yes. Evidence: verified via `disassemble_function 0x00641B00`, inner loop `0x00641C89..0x00641CEF`.

### 2.2 Global address → channel mapping (`DSurface__Constructor`)

Active in YR: Yes. Evidence: `disassemble_function 0x004BA770` (`DSurface__Constructor`).

The constructor initializes the DD globals from the DirectDraw surface's reported bit-masks in this order:

```
004BA9CD: MOV EAX,[0x008A0958]   ; R bitmask
004BA9D6: MOV dword ptr [0x008A0DD0],ECX  ; → g_DD_RShift
004BA9DC: MOV dword ptr [0x008A0DD4],EDX  ; → g_DD_RLoss

004BAA00: MOV EAX,[0x008A095C]   ; G bitmask
004BAA0F: MOV dword ptr [0x008A0DE0],ECX  ; → g_DD_GShift
004BAA15: MOV dword ptr [0x008A0DE4],ESI  ; → g_DD_GLoss

004BAA39: MOV ECX,[0x008A0960]   ; B bitmask
004BAA49: MOV [0x008A0DD8],EAX   ; → g_DD_BShift
004BAA4E: MOV dword ptr [0x008A0DDC],EDX  ; → g_DD_BLoss
```

This confirms: `0x008A0DD0/DD4` = R, `0x008A0DE0/DE4` = G, `0x008A0DD8/DDC` = B. The decode loop applies these to byte[0], byte[1], byte[2] respectively → RGB.

Active in YR: Yes. Evidence: verified via `disassemble_function 0x004BA770`, addresses `0x004BA9CD..0x004BAA4E`.

### 2.3 Encode path corroboration (`0x006418B0`)

Active in YR: Yes for map save/generated-preview serialization. Evidence: xrefs from `CDFileClass__Constructor` / map-save path.

The writer at `0x006418B0` reads surface pixels via vtable +0x28 and extracts channels using the same DD globals. Assembly `0x006419D4..0x00641A2F` writes the 3-byte output buffer in red, green, blue order before calling `FUN_0055C350(buffer, 3)`. This corroborates that the stored stream is RGB.

Active in YR: Yes. Evidence: decompile `0x006418B0`.

### 2.4 Pixel format: source bpp and surface format

Source is 3 bytes per pixel (24-bit RGB). The destination surface is the standard YR DirectDraw 16-bpp surface (verified via `DSurface__Constructor`'s `GetBitDepth` vtable return == 2 branch at `0x004BA9C4 CMP EAX, 0x2`). The runtime conversion from 8-bit-per-channel RGB to packed 16-bpp is done by the shift/loss globals (which encode the mask's bit position and precision loss for the target surface's native pixel format, e.g. RGB565 or RGB555 depending on hardware).

Active in YR: Yes. Evidence: `DSurface__Constructor` `0x004BA9C4`; DD global initialization loop extracting shift from mask lsb scan and loss from msb scan.

## 3. Current Rust Implementation Status

| Rust surface | Status |
|---|---|
| `src/map/preview.rs` line 90 | `PREVIEW_CHANNEL_ORDER = PreviewChannelOrder::Rgb` — **matches gamemd** |
| `src/map/preview.rs` line 151-159 | `push_rgba_from_preview_pixel` expands `[pixel[0], pixel[1], pixel[2], 255]` for Rgb — **correct** |
| `src/map/preview.rs` line 184 | `rgb.chunks_exact(3)` per-pixel loop — **correct** |
| Test `decode_preview_pack_literal_chunk_to_rgba` | Asserts `[1,2,3] → [1,2,3,255]`, `[4,5,6] → [4,5,6,255]` — **correct** |

**No change needed.** Rust already correctly treats byte 0 as R, byte 1 as G, byte 2 as B.

## 4. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required effect | Acceptance scenario | Risk |
|---|---|---|---|---|---|---|
| Decompressed PreviewPack stream is row-major RGB: byte 0 = red, byte 1 = green, byte 2 = blue. | Decode assembly `0x00641C89..0x00641CEF`; global init `0x004BA9D6..0x004BAA4E`; encode corroboration `0x006418B0`. | **None — Rust already matches.** `PREVIEW_CHANNEL_ORDER = Rgb`; triples expanded as `[r,g,b,255]`. | `src/map/preview.rs` | Keep as-is. Do not change channel order. | Decode a two-pixel RGB fixture `[1,2,3,4,5,6]` and get RGBA `[1,2,3,255,4,5,6,255]`. Test: `decode_preview_pack_literal_chunk_to_rgba` (already passes). | Do not swap to BGR. Do not treat the DirectDraw packed pixel format as the serialized file format. |

## 5. Negative Facts / Do Not Do

- Do not decode `[PreviewPack]` as BGR. Active in YR: Yes. Evidence: byte 0 is routed through `g_DD_RLoss/RShift` at `0x00641CC9/0x00641CDD`, not B globals. BGR would discolor every map preview red/blue-swapped.
- Do not confuse the packed-surface 16-bpp pixel format with the serialized file format. Active in YR: Yes. Evidence: the conversion from RGB bytes → packed pixel is done at decode time via DD shift/loss globals; the stored bytes are 8-bit-per-channel RGB.
- Do not use the random-map `RandMap.img` PCX path for this analysis. Active in YR: N/A. Evidence: that is a separate loader (`0x00641DB0`) reading a PCX file, not the PreviewPack INI-binary path at `0x00641B00`.
- Do not skip the LZO decompression stage. Active in YR: Yes. Evidence: `0x00641BCB` reads base-64-decoded INI binary into a compressed buffer; `LZOStraw__Constructor(1, 0x2000)` at `0x00641C1B` decompresses before pixel read.
- Do not implement a 32-bpp surface path for PreviewPack. Active in YR: No. Evidence: `DSurface__Constructor` bit-depth branch at `0x004BA9C4` — stock YR surfaces are 16-bpp for the preview path.

## 6. Remaining Uncertainty

None for channel order. The byte→channel mapping is doubly verified (decode loop + encode corroboration + global initializer cross-check). The only out-of-scope item is the exact concrete subclass implementing surface vtable +0x24/+0x28 — not needed for serialized channel order.

## 7. Stale Doc Replacement

`docs/research/skirmish-ui/PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md` section 6 `[DEFERRED] OQ-6` asks "What is the exact concrete class behind the surface vtable slots?" — that OQ is about the surface subclass, not channel order, and can remain deferred as stated. The channel order OQ from the swarm brief is **fully resolved** by `docs/research/skirmish-ui/SKIRMISH_PREVIEWPACK_CHANNEL_ORDER_AND_MENU_CALLER_GHIDRA_REPORT.md` §3.2 and this report independently corroborates it.

If a doc update is needed for the swarm-triggering doc `SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`, OQ-9 in that doc already says: "PreviewPack text is INI-binary decoded, then LZO-decompressed, then consumed in row-major RGB byte order before packed-color conversion." No change needed there either.

## 8. Unverified

(None — all load-bearing claims were verified by live Ghidra disassembly this session.)

## Sources

- `disassemble_function 0x00641B00` — decode inner loop `0x00641C89..0x00641CEF` (channel assignment)
- `disassemble_function 0x004BA770` — `DSurface__Constructor` global init `0x004BA9CD..0x004BAA4E` (channel→global address mapping)
- `decompile_function 0x00641B00` — decode path structure, short-read abort
- `decompile_function 0x006418B0` — encoder corroboration (writes R then G then B)
- `get_assembly_context 0x00641C89` — confirmed inner loop instruction sequence
- Existing docs (verified findings, not ground truth): `docs/research/skirmish-ui/SKIRMISH_PREVIEWPACK_CHANNEL_ORDER_AND_MENU_CALLER_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md`
- Rust read: `src/map/preview.rs`, `src/app_skirmish_shell_render/preview.rs`
