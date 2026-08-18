# Skirmish PreviewPack Decode Format - Ghidra Research Report

Date: 2026-05-20

**Address(es):** `0x006418B0` / `0x00641A78`, `0x005270??` (`Pipe__Constructor(char *section, int buffer, uint size)`), `0x0042DDB0`, `0x0042FD30`, `0x0042FE50`, `0x0055C350`, `0x0055BB90`, `0x0055C0E0`, `0x00631CC0`, `0x00631D10`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** `[Preview]` / `[PreviewPack]` map-file storage boundary: INI text encoding, LZO pipe direction, decoded byte count, row order, and channel-order confidence.  
**Non-Scope:** network preview transfer protocol, `DAT_00AC1154` lifecycle, exact Skirmish menu caller that constructs the preview object, and start-marker overlay projection.  
**Confidence:** High for storage/decode container and byte count; Medium for row-major output; Low for RGB/BGR channel order.  
**Active in YR:** Conditional. The `[PreviewPack]` map-file format is present in standard YR multiplayer map files and the writer path is live in the YR map-save/generation path. The exact standard Skirmish menu load caller was not proven in this slot.

## 1. Overview

`[PreviewPack]` is not raw pixels and not a standalone image format. It is an INI-binary text section containing LZO-compressed pixel triples. `[Preview] Size=left,top,width,height` provides the target drawable dimensions; the expected decompressed byte count is `width * height * 3`.

The writer path proves row-major traversal over the preview surface: outer loop over height, inner loop over width, exactly three bytes emitted per pixel. Dustbowl's retail map data matches this model: `Size=0,0,138,75`, base64-decoded `PreviewPack` LZO chunk raw-size headers sum to `31,050` bytes, exactly `138 * 75 * 3`.

## 2. Key Format Fields

| Item | Meaning | Active in YR | Evidence |
| --- | --- | --- | --- |
| `[Preview] Size=` | Four-int rectangle; third and fourth values are preview width/height for stock maps such as `0,0,138,75`. | Yes, standard map file data. | `Dustbowl.map`; parent report; Rust parser lines `src/map/preview.rs:50-60` currently read first two fields. |
| `[PreviewPack]` values | Text-encoded binary payload, one INI key/value line sequence. | Yes, standard map file data. | `Dustbowl.map` has 275 non-empty lines. |
| INI-binary text codec | Standard base64 alphabet `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/` with `=` padding. | Yes for this format path. | `0x0042FD30` encode helper; `0x0042FE50` decode helper. |
| LZO chunk stream | Compressed payload is divided into chunks with per-chunk compressed/raw lengths. | Yes in writer and decode helpers. | `0x0055C350`, `0x0055BB90`, `0x0055C0E0`; Dustbowl chunk raw sizes. |
| Raw pixel payload size | `preview_width * preview_height * 3`. | Yes for maps using `[PreviewPack]`. | Writer emits `3` bytes per pixel at `0x006418B0`; Dustbowl raw chunk sum is `31,050`. |

## 3. Core Logic

### Writer / Storage Path

Active in YR: Yes for map save/generation paths. Evidence: `CDFileClass__Constructor @ 0x00687DEC` calls `GenerateTerrainPreview`, then `Pipe__Constructor(&local_e8)` on the preview INI object when writing map data.

Verified behavior from `0x006418B0` / `0x00641A78`:

- If the preview surface pointer is null, the path calls `GenerateTerrainPreview`.
- It writes `[Preview]` metadata first.
- It initializes a `BufferPipe` backed by a temporary pixel buffer.
- It constructs `LZOPipe(mode=0, block_size=0x2000)` and links it to the buffer pipe through `0x00631CC0`.
- It locks the preview surface.
- It loops `y = 0..height-1`, then `x = 0..width-1`.
- For each pixel, it reads the surface pixel and writes exactly `3` bytes through `0x0055C350`.
- It flushes the LZO pipe, then writes the compressed bytes into `[PreviewPack]` through the INI binary writer.

Compression direction: `LZOPipe(mode=0)` is the compress side. Evidence: `0x0055C350` takes the non-`+0x0C == 1` branch, accumulates raw bytes until the block buffer fills, calls `0x0055BB90` to compress, then emits a 4-byte chunk header plus compressed block to the downstream pipe.

### INI Binary Text Codec

Active in YR: Yes for INI binary sections, including `[PreviewPack]` when that section is read or written. Evidence: `batch_string_anchor_report("PreviewPack")` reports exactly two documented references, the storage writer and the INI binary reader/writer path.

`0x0042FD30` is the binary-to-text encoder:

- It consumes 3 binary bytes per group.
- It emits 4 text bytes per group.
- It uses the standard base64 alphabet at `0x007E3910`.
- It emits the padding character from `PTR_DAT_007E390C`, consistent with `=`.

`0x0042FE50` is the text-to-binary decoder:

- It maps input bytes through the decode table at `0x007E3914`.
- It ignores entries marked `0xFE`.
- It treats `0xFF` as padding/end.
- It reconstructs up to 3 binary bytes from each 4-character quantum.

### Generic INI Binary Section Reader

Active in YR: Conditional. The helper is active when a caller requests a binary INI section such as `PreviewPack`; the exact Skirmish menu caller is out of scope for this slot and should be covered by the preview lifecycle slot.

The decompiled `Pipe__Constructor(char *section, int buffer, uint size)` path:

- Returns `0` if the section name pointer is null.
- Wraps the destination buffer in a `PixelBuffer`.
- Links a downstream pipe using `0x00631CC0`.
- Looks up the section by CRC/cached binary search.
- Iterates each key/value line in the section.
- Copies each value to a local 128-byte buffer, trims it, and passes it through `0x0042DDB0`.
- Flushes the pipe and returns decoded/forwarded byte count.

`0x0042DDB0` is a base64 pipe adapter. It switches between encode and decode behavior based on the pipe mode field at `+0x0C`, grouping 3-to-4 or 4-to-3 bytes and forwarding the result through `0x00631D10`.

### LZO Decode Direction

Active in YR: Yes in the generic LZO pipe implementation. Evidence: `LZOPipe__Constructor`, `0x0055C350`, `0x0055C0E0`.

`LZOPipe__Constructor` stores the mode argument at pipe offset `+0x0C`. In `0x0055C350`:

- `mode == 0` compresses raw input: buffer raw bytes, call `0x0055BB90`, emit a 4-byte block header plus compressed bytes.
- `mode == 1` decodes chunked input: read a 4-byte block header, use the stored compressed/raw lengths, call `0x0055C0E0`, then forward decompressed bytes.

This establishes the expected map-file decode order:

```text
[PreviewPack] INI values
  -> base64 text decode
  -> LZO chunk decode
  -> raw preview byte stream of width * height * 3
```

## 4. Retail Map Data Check

Active in YR: Yes. `Dustbowl.map` is a retail multiplayer map in the configured RA2/YR install.

Observed data:

- `[Preview] Size=0,0,138,75`
- `[PreviewPack]` has 275 non-empty key/value lines.
- Concatenated encoded text length is 19,216 characters.
- Standard base64 decoding gives 14,410 compressed/chunk bytes.
- LZO chunk headers parse as:
  - compressed `2951`, raw `8192`
  - compressed `4246`, raw `8192`
  - compressed `4187`, raw `8192`
  - compressed `3010`, raw `6474`
- Raw-size sum is `8192 + 8192 + 8192 + 6474 = 31,050`.
- `138 * 75 * 3 = 31,050`.

This independently confirms that `[Preview]` width/height and `[PreviewPack]` decoded byte count match the binary writer's 3-bytes-per-pixel model.

## 5. Row Order And Channel Order

Row order: High confidence row-major, top-to-bottom. Active in YR: Yes for stored previews. Evidence: `0x006418B0` outer height loop, inner width loop, one three-byte write per pixel.

Channel order: Not fully proven. Active in YR: Conditional/unknown at the serialized boundary. Evidence limitation: the writer reads a DirectDraw/surface pixel through vtable `+0x28` and writes three stack bytes, but the decompile does not unambiguously name those stack bytes as RGB or BGR. `GenerateTerrainPreview @ 0x00641140` builds DirectDraw-format colors from RGB source bytes using `g_DD_RShift/GShift/BShift`, which proves the surface color construction path, not the serialized byte order after surface readback.

Practical implication: implementers should treat the byte stream as row-major 3-byte pixels, but verify channel order by decoding one known map and comparing against a retail screenshot or a known pixel sample before locking RGB vs BGR.

## 6. Current Rust Implementation Status

Active in YR comparison: Not applicable to gamemd; this is Rust parity status.

Current Rust only records metadata:

- `src/map/preview.rs:11-15` stores `size: Option<(u32,u32)>` and `has_packed_preview`.
- `src/map/preview.rs:34-42` checks whether `[PreviewPack]` has non-empty values.
- `src/map/preview.rs:50-60` parses the first two comma fields of `Size`, which is wrong for four-field retail map values such as `0,0,138,75`.
- No Rust path decodes `[PreviewPack]` base64, no LZO chunk decode is present for preview data, and no preview surface upload exists in this module.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
| --- | --- | --- | --- |
| `[Preview] Size` width/height meaning | verified | `Dustbowl.map`, `0x006418B0` writer metadata path | none |
| `[PreviewPack]` is INI-binary text | verified | `0x0042FD30`, `0x0042FE50`, string anchor report | none |
| Base64 alphabet and padding | verified | `0x0042FD30`, `0x0042FE50` | none |
| LZO compress side | verified | `0x0055C350`, `0x0055BB90`, `LZOPipe__Constructor(mode=0)` | none |
| LZO decode side | verified as generic pipe behavior | `0x0055C350`, `0x0055C0E0`, mode field `+0x0C` | exact menu caller not covered here |
| Writer row order | verified | `0x006418B0` height outer loop, width inner loop | none |
| Expected raw byte count | verified | `0x006418B0`; Dustbowl raw chunk sum `31,050` | none |
| RGB vs BGR serialized byte order | deferred | surface readback decompile ambiguous | decode-vs-retail pixel sample or tighter assembly/source-byte proof |
| Standard Skirmish preview object caller | deferred | out of this slot | slot 1 / slot 4 lifecycle investigations |

## 8. Open Questions - Final State

[RESOLVED] OQ-1 - Is `[PreviewPack]` plain base64 pixels? No. It is base64 text over LZO chunk data, then raw 3-byte pixels after decompression. Evidence: `0x0042FD30`, `0x0042FE50`, `0x0055C350`, Dustbowl chunk headers.

[RESOLVED] OQ-2 - What is the expected decompressed byte count? `width * height * 3`, with width/height from `[Preview] Size` fields 3 and 4 in four-field retail maps. Evidence: `0x006418B0`, Dustbowl `138 * 75 * 3 = 31,050`.

[RESOLVED] OQ-3 - Which LZO pipe mode compresses? Mode `0` compresses; mode `1` decodes. Evidence: `LZOPipe__Constructor` stores mode at `+0x0C`; `0x0055C350` mode branches call `0x0055BB90` compression or `0x0055C0E0` decompression.

[RESOLVED] OQ-4 - Is pixel order row-major? Yes for writer output: height loop outside width loop. Evidence: `0x006418B0`.

[DEFERRED] OQ-5 - Is serialized channel order RGB or BGR? Category: needs-runtime-debugger. The writer proves three bytes per pixel but the decompile does not conclusively name the byte order after surface readback. Next step: decode Dustbowl and compare a known terrain/building pixel against retail output or inspect the concrete surface `GetPixel` byte write at assembly level with register/stack tracking.

[DEFERRED] OQ-6 - Which exact standard Skirmish menu function calls the `[PreviewPack]` reader for `DAT_00AC1154`? Category: out-of-scope. This belongs to the preview object lifecycle and choose-map refresh swarm slots.

## Sources

- Ghidra: `0x006418B0` / `0x00641A78` preview INI storage writer.
- Ghidra: `0x00687DEC` map-save/generation caller.
- Ghidra: `0x0042DDB0`, `0x0042FD30`, `0x0042FE50` INI binary text codec.
- Ghidra: `LZOPipe__Constructor`, `0x0055C350`, `0x0055BB90`, `0x0055C0E0`, `0x00631CC0`, `0x00631D10` pipe and LZO chunk behavior.
- Ghidra string reports: `PreviewPack` at `0x00836DD0`, `Preview` at `0x00836DDC`.
- Retail data: `<ra2-install>/Dustbowl.map`.
- Parent report: `docs/research/SKIRMISH_MAP_PREVIEW_SOURCE_BOUNDS_AND_PREVIEWPACK_GHIDRA_REPORT.md`.
- Rust status: `src/map/preview.rs`.
