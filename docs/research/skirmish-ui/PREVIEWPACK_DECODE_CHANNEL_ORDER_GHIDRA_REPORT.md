# PreviewPack Decode Channel Order - Ghidra Research Report

**Address(es):** `0x00641B00` normal PreviewPack load/decode, `0x006418B0` PreviewPack write/encode, `0x005E74E0` Skirmish preview refresh caller, `0x00641EE0` selected-map file preview loader
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Skirmish map `[Preview]` / `[PreviewPack]` conversion details: INI binary text conversion, compression/decompression direction, row order, RGB/BGR byte order, and final surface pixel layout.
**Non-Scope:** minimap/radar rendering, `STARTBUT.SHP` overlays, network preview transfer, map chooser list behavior, and generated terrain preview pixel selection beyond the storage boundary.
**Confidence:** High for normal selected `.map` preview decode and writer-side storage order; Medium for exact DirectDraw surface memory layout naming because the surface methods remain vtable calls.
**Active in YR:** Yes for normal selected-map Skirmish previews. Evidence: `0x006ACEE0` calls `0x005E74E0`; `0x005E74E0` reaches `0x00641EE0`; `0x00641EE0` parses the selected map buffer and calls `0x00641B00`.

## 1. Overview

`[PreviewPack]` is INI text carrying base64-like binary data, not raw pixels. In the active selected-map preview path, gamemd reads `[Preview] Size`, allocates a preview `DSurface`, reads `[PreviewPack]` into a compressed byte buffer, LZO-decompresses it, then writes one RGB triple per surface pixel.

The serialized channel order is **RGB**, not BGR. On load, byte 0 is shifted through the DirectDraw red loss/shift globals, byte 1 through green, and byte 2 through blue before the destination surface vtable `+0x24` pixel write.

## 2. Key Data And Offsets

| Item | Purpose | Active in YR | Evidence |
|---|---|---:|---|
| `PTR_s_Preview_007f0048` / string `Preview @ 0x00836DDC` | Section used to read size rect before allocating the preview surface. | Yes | `0x00641B00`; string anchor report |
| `PTR_s_PreviewPack_007f004C` / string `PreviewPack @ 0x00836DD0` | Section name passed to INI binary reader/writer. | Yes | `0x00641BCB`; `0x006418B0`; string anchor report |
| `g_DD_RShift/RLoss`, `g_DD_GShift/GLoss`, `g_DD_BShift/BLoss` | Runtime pixel format conversion between 8-bit RGB channels and DirectDraw packed surface pixels. | Yes | decode assembly `0x00641C95..0x00641CEF`; writer assembly `0x006419D4..0x00641A2F` |
| Destination surface vtable `+0x24` | Writes packed DirectDraw pixel for one `(x,y)`. | Yes | `0x00641CEF` |
| Destination surface vtable `+0x7C/+0x80` | Width and height loop limits. | Yes | `0x00641C4D`, `0x00641C61`, `0x00641CF7`, `0x00641D11` |

## 3. Core Logic

### Load/decode path (`0x00641B00`)

Active in YR: Yes. Evidence: normal Skirmish choose-map path `0x006ACEE0` calls `0x005E74E0`; xrefs show `0x005E74E0` calls `0x00641EE0`; `0x00641EE0` calls `0x00641B00`.

1. Clears INI section cache.
2. Destroys any existing preview surface in the wrapper.
3. Reads `[Preview]` through `FUN_00527CC0`, copies the four returned fields, and uses the third/fourth values as `DSurface` dimensions.
4. Locks/opens a source buffer via the global preview/file surface object.
5. Calls `Pipe__Constructor("PreviewPack", data_ptr, width * height * bytes_per_pixel)` at `0x00641BCB`; this reads the INI binary text into a compressed byte buffer.
6. Wraps that buffer with `LZOStraw__Constructor(1, 0x2000)` and pulls decompressed bytes with `FUN_0055C7C0`.
7. Loops height outer, width inner. For each pixel it requires exactly `3` bytes; short read returns `0`.
8. Converts the 3 bytes into a DirectDraw packed color and writes one destination surface pixel via vtable `+0x24`.

### Channel byte order

Active in YR: Yes. Evidence: active loader `0x00641B00`, assembly `0x00641C77..0x00641CEF`.

The loader calls `FUN_0055C7C0(temp, 3)`. Assembly then maps the three temp bytes as:

| Serialized byte | Assembly source | Channel use |
|---:|---|---|
| byte 0 | `[ESP+0x10]` after the read | red: loss then `g_DD_RShift` |
| byte 1 | `[ESP+0x11]` after the read | green: loss then `g_DD_GShift` |
| byte 2 | `[ESP+0x12]` after the read | blue: loss then `g_DD_BShift` |

Therefore the decompressed `[PreviewPack]` pixel stream is row-major **RGBRGB...**. Rust should expand each triple to RGBA as `[r, g, b, 255]`, not swap red and blue.

### Write/encode path (`0x006418B0`)

Active in YR: Yes for map save/generation paths. Evidence: xrefs from `CDFileClass__Constructor` at `0x00687DEC` / `0x00687E0E`; writer calls `GenerateTerrainPreview` if the preview surface pointer is null.

1. Ensures a preview surface exists, generating one when needed.
2. Writes `[Preview]` metadata before `[PreviewPack]`.
3. Creates a `BufferPipe` plus `LZOPipe__Constructor(0, 0x2000)`, then iterates the source surface height outer and width inner.
4. For each `(x,y)`, vtable `+0x28` reads a packed surface pixel.
5. Assembly extracts red first, green second, blue third from the packed pixel using the same DirectDraw globals.
6. It writes exactly `3` bytes with `FUN_0055C350(temp, 3)`.
7. After flushing the LZO pipe, it writes the compressed data into `[PreviewPack]` through the INI binary writer.

The writer corroborates RGB order: at `0x006419D4..0x00641A2F`, the temp buffer passed to `FUN_0055C350(..., 3)` is populated as red, green, blue.

### INI-binary conversion

Active in YR: Yes when reading or writing binary INI sections such as `[PreviewPack]`. Evidence: `Pipe__Constructor` xref from `0x00641BCB`; base64 helpers `0x0042FD30` and `0x0042FE50`.

The generic INI binary path walks the named section's entries, trims/extracts text values, decodes text through `FUN_0042FE50`, and writes the resulting bytes into the caller-provided buffer. The reverse path groups binary bytes through `FUN_0042FD30` into the printable alphabet string before INI storage.

`Dustbowl.map` is a standard loose retail sample containing `[Preview] Size=0,0,138,75` and `[PreviewPack]`; this data shape is active input for the same selected-map loader.

## 4. Current Rust Implementation Status

Active in YR: Not applicable to binary activity; this is implementation status.

Current Rust is already structured around the now-verified RGB model: `src/map/preview.rs:76` defines `PreviewChannelOrder`, `src/map/preview.rs:82` sets `PREVIEW_CHANNEL_ORDER` to `Rgb`, `src/map/preview.rs:143` expands triples to RGBA, and `src/map/preview.rs:155` decodes row-major 3-byte pixels.

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Normal selected-map PreviewPack loader | verified | `0x005E74E0`, `0x00641EE0`, `0x00641B00`; xrefs from `0x006ACEE0` | none for this slice |
| `[Preview]` size use | verified | `0x00641B00`; `Dustbowl.map:252` | none for channel-order slice |
| INI binary text to bytes | verified | `Pipe__Constructor @ 0x00526FB0`, `FUN_0042FE50 @ 0x0042FE50` | exact line wrapping policy is out of scope |
| LZO direction on load | verified | `LZOStraw__Constructor(1, 0x2000)` before `FUN_0055C7C0` at `0x00641B00` | none |
| Pixel row order on load | verified | height loop via vtable `+0x80`, width loop via `+0x7C`, pixel write `+0x24` | none |
| RGB/BGR byte order on load | verified | assembly `0x00641C77..0x00641CEF` | none |
| Writer-side storage order | verified | `0x006418B0`, assembly `0x006419D4..0x00641A2F` | none |
| Final DirectDraw surface memory layout | touched-not-exhausted | vtable `+0x24/+0x28` plus DD shift/loss globals | concrete surface subclass internals not needed for serialized channel order |
| Network preview transfer | not-touched | string anchors `Preview.bin`, `NET_PREVIEW_MODE` | out of scope |

## 6. Open Questions - Final State

[RESOLVED] OQ-1 - Is `[PreviewPack]` text raw pixels? No. It is INI binary text decoded by `FUN_0042FE50`, then LZO-decompressed before pixel use. Evidence: `0x00526FB0`, `0x0042FE50`, `0x00641B00`.

[RESOLVED] OQ-2 - Is compression or decompression applied on load? Load uses LZO decompression through `LZOStraw__Constructor(1, 0x2000)` and `FUN_0055C7C0`. Evidence: `0x00641B00`.

[RESOLVED] OQ-3 - What is the row order? Height outer, width inner, top-left to bottom-right in surface coordinates. Evidence: `0x00641C4D`, `0x00641C61`, `0x00641CF7`, `0x00641D11`.

[RESOLVED] OQ-4 - Is channel order RGB or BGR? RGB. Evidence: byte 0 -> red, byte 1 -> green, byte 2 -> blue at `0x00641C77..0x00641CEF`; writer extracts red/green/blue in that order at `0x006419D4..0x00641A2F`.

[RESOLVED] OQ-5 - Is this active in standard YR Skirmish selected-map preview? Yes. Evidence: `0x006ACEE0` xrefs to `0x005E74E0`; `0x005E74E0` reaches `0x00641EE0`; `0x00641EE0` calls `0x00641B00`.

[DEFERRED] OQ-6 - What is the exact concrete class behind the surface vtable slots? Category: out-of-scope. Reason: the slice only needs the serialized byte order and final packed-color conversion; vtable `+0x24/+0x28` behavior is already bounded by active caller use and DD shift/loss globals.

## Sources

- Ghidra: `0x00641B00`, `0x006418B0`, `0x005E74E0`, `0x00641EE0`, `0x00526FB0`, `0x0042FD30`, `0x0042FE50`, `0x0055C7C0`, `0x0055C350`.
- Ghidra string anchors: `PreviewPack @ 0x00836DD0`, `Preview @ 0x00836DDC`.
- Retail sample: `C:/Users/enok/Documents/Command and Conquer Red Alert II/Dustbowl.map:251-254`.
- Rust status: `C:/Users/enok/Documents/ra2-rust-game/src/map/preview.rs:76`, `:82`, `:143`, `:155`.
