# BitFont / Shell Text Rendering — Ghidra Research Report

**Address(es):** `0x00433880`, `0x00433990`, `0x00433CF0`, `0x00433ED0`, `0x00434120`, `0x00434500`, `0x004346C0`, `0x00434700`, `0x004348F0`, `0x00434B90`, `0x00434CD0`, `0x00434AD0`, `0x00621040`, `0x006211D0`, `0x00621B80`, `0x00623880`, `0x004A59E0`, `0x004A60E0`
**Confidence:** HIGH for all algorithm shapes, struct layouts, and address bindings (decompiled). MEDIUM for one byte's purpose (FNT field 0 / inner +0x00) and one color (outer +0x26 second color), which the constructor sets but the measure/draw paths examined here do not consume.
**Active in YR:** Yes for all paths examined. The shell text pipeline is hot — every owner-draw label on every Skirmish/Host/Guest/WOL dialog and every sidebar cameo's text passes through this code on every WM_PAINT. **`FonT` (capital F) format is dormant TS-legacy** — the loader still handles it but no YR asset uses it.

## 1. Overview

The bitmap font system has two layers:

- **Lower layer — `BitFont` class** (`0x00433880` ctor, `0x00434CD0` draw): owns the loaded glyph data, the lookup table, draw state (color/clip/tabs/spacing), and the actual per-glyph rasterization to a 16-bit surface. One global instance lives at `g_GAME_FNT @ 0x0089C4D0`, holding `GAME.FNT`.
- **Upper layer — `BitText` class** (`0x00434AD0` ctor): a wrapper class that exists primarily to *construct* the global `BitFont` at startup. Each owner-draw dialog has a `BitText` lifecycle that triggers the one-time `g_GAME_FNT` initialization.

Above `BitFont`, two shell text-drawing entry points wrap it for owner-draw use:

- **`FUN_00621040`** — takes an RGB color and a RECT, converts color to 16-bit, sets clip, and dispatches into the full draw routine `FUN_00434CD0` (which handles wrap, multiline, fade, alignment). Used by `OwnerDraw_Button`, `OwnerDraw_Static`, `OwnerDraw_Checkbox`, `OwnerDraw_RadioVariant`, `OwnerDraw_ComboBox`, `OwnerDraw_Trackbar`.
- **`FUN_006211D0`** — defaults font to `g_GAME_FNT`, takes integer H/V align modes (not bit flags), and dispatches into `FUN_00434B90` → `FUN_00434500` (single-line draw with a per-character "selected unit highlight" fade). Used by `OwnerDraw_ListBox`, `OwnerDraw_ButtonVariant`, and (via `FUN_00623880`) the Edit-control cursor/selection/password text path.

Glyph data is in `GAME.FNT` (a single binary file from `local.mix`/`localmd.mix`, ~1.58 MB). Format: 28-byte header (magic `fonT`), 131072-byte u16 codepoint→1-based-glyph-index lookup table, then a flat array of fixed-size glyph slots (49 bytes each for GAME.FNT: 1 width byte + 16 bitmap rows × 3 bytes per row). 1-bit-per-pixel glyphs, MSB-first.

## 2. Class Layout / Key Offsets

### Outer BitFont struct (~108 bytes, allocated by caller, populated by `BitFont__Constructor`)

| Offset | Type | Purpose | Default (constructor) |
|---|---|---|---|
| `+0x00` | `void*` | vtable pointer (`&vtable__BitFont`) | set |
| `+0x04` | `inner*` | pointer to 36-byte inner FNT data struct (or 0 if load failed) | result of `BitFont__LoadFontData` |
| `+0x08` | `byte*` | **fallback glyph** (50 bytes, inverted codepoint 0xB0) | 0 → built by `FUN_00434700` after successful load |
| `+0x0C` | `u16*` | destination surface pixel buffer (locked Lock()) | 0 → set by `SetSurface (FUN_004348F0)` per-paint |
| `+0x10` | `u32` | surface pitch in **16-bit words** (= byte pitch / 2) | set by SetSurface |
| `+0x18` | `u32` | bytes per glyph bitmap row (= 3 for GAME.FNT) | copied from inner +0x04 by ctor |
| `+0x1C` | `u32` | **line height** / cell_height (= 17 for GAME.FNT) | copied from inner +0x0C by ctor |
| `+0x20` | `u32` | tab origin x (subtracted before `% tab_width` for alignment) | 0 |
| `+0x24` | `u16` | **default text color** (16-bit packed in display format) | `0x7FFF` (white RGB555 / pale-grey RGB565) |
| `+0x26` | `u16` | **second color** (purpose not yet identified — see Open Questions) | `0x3555` |
| `+0x28` | `u32` | **tab width** (px) | `0x40` (64) |
| `+0x2C` | `u32` | **inter-character spacing** (px added after each glyph's width) | `1` |
| `+0x30` | `i32` | clip rect left | 0 (defaulted to full surface by SetSurface if zero) |
| `+0x34` | `i32` | clip rect top | 0 |
| `+0x38` | `i32` | clip rect right (inclusive) | 0 |
| `+0x3C` | `i32` | clip rect bottom (inclusive) | 0 |
| `+0x40` | `u8` | draw enable flag (set by `FUN_00433C90`; passed as `param_2` to that setter) | `1` |
| `+0x41` | `u8` | **clip enable flag** (1 = clip per-pixel against rect, 0 = skip clip) | `1` |

### Inner FNT data struct (36 bytes = `0x24`, allocated by `BitFont__LoadFontData`)

| Offset | Type | Source | Value for GAME.FNT |
|---|---|---|---|
| `+0x00` | `u32` | FNT bytes `[4..8]` (header field 0) | unknown — possibly 20 ("chars per row in sheet" per prior doc, unverified — see OQ #1) |
| `+0x04` | `u32` | FNT bytes `[8..12]` (header field 1) | **3** = `bytes_per_row` (bitmap row stride within glyph slot) |
| `+0x08` | `u32` | FNT bytes `[12..16]` (header field 2) | **16** = `bitmap_rows` (rows of pixels per glyph) |
| `+0x0C` | `u32` | FNT bytes `[16..20]` (header field 3) | **17** = `cell_height` (line advance — note: `bitmap_rows + 1` for line gap) |
| `+0x10` | `u32` | FNT bytes `[20..24]` (header field 4) | **29655** = `num_glyph_slots` (total slots in glyph data, regardless of how many are referenced) |
| `+0x14` | `u32` | FNT bytes `[24..28]` (header field 5) | **49** = `glyph_stride` (bytes per slot; = 1 + bytes_per_row × bitmap_rows = 1 + 3×16) |
| `+0x18` | `u16*` | lookup table base (131072 bytes = 65536 u16 entries) | allocated, then read from file directly |
| `+0x1C` | `byte*` | glyph data base (`num_glyph_slots × glyph_stride` = 1,453,095 bytes) | allocated, then read |
| `+0x20` | `u32` | non-zero lookup count (cached glyph count) | counted at load: number of codepoints with `lookup[cp] != 0` |

### FNT file format — `fonT` magic (active)

| Offset | Type | Purpose | Value (GAME.FNT) |
|---|---|---|---|
| `0x00` | `u32` | magic | `0x546E6F66` (LE bytes `66 6F 6E 54` = "fonT") |
| `0x04` | `u32` | field 0 | 20 (purpose unclear; stored at inner +0x00 but not consumed in measure/draw paths examined) |
| `0x08` | `u32` | field 1 = `bytes_per_row` | **3** (bytes per glyph bitmap row) |
| `0x0C` | `u32` | field 2 = `bitmap_rows` | **16** |
| `0x10` | `u32` | field 3 = `cell_height` | **17** (= line height advance, also = bitmap_rows + 1px line gap) |
| `0x14` | `u32` | field 4 = `num_glyph_slots` | **29655** |
| `0x18` | `u32` | field 5 = `glyph_stride` | **49** (= 1 + bytes_per_row × bitmap_rows) |
| `0x1C..0x2001B` | `u16[65536]` | **codepoint lookup table** — `lookup[cp]` = 1-based glyph index, 0 means "no glyph for this codepoint" | direct file read |
| `0x2001C..end` | glyph slot array, each `glyph_stride` (49) bytes | direct file read; each glyph slot: byte 0 = pixel width, bytes 1..48 = 16 rows × 3 bytes, 1-bit-per-pixel, MSB-first |

**Note: the `+1` line gap.** `cell_height` (17) = `bitmap_rows` (16) + 1 inter-line gap. The draw and measure code advances `local_1c += outer[+0x1C]` (= 17) on each newline, which gives one blank pixel row between text lines.

### FNT file format — `FonT` magic (DORMANT TS-LEGACY)

`BitFont__LoadFontData` also handles `0x744E6F46` ("FoNt" — bytes 0x46,0x6F,0x4E,0x74 in LE = 'F','o','N','t'; corrected 2026-05-28: was described as "FonT" but binary `iStack_44 == 0x744e6f46` decodes to "FoNt" — ROOT_CAUSE: INFERENCE_HARDENED, the 'n'/'N' byte order was not read literally from the binary). This path:

- Reads a **36-byte** header (vs 28 bytes for `fonT`) at offset 0
- Stores header fields into a different inner-struct offset pattern (uses inner +0x14 as glyph_stride directly; uses inner +0x10 as a separate "total glyphs" counter)
- After header, reads a packed table of `N × 12-byte` blocks where each block is `{u32 index_base, u32 first_codepoint, u32 last_codepoint}`
- Builds the 65536-entry lookup table by expanding each block: `lookup[first..=last] = index_base+1, index_base+2, ...`
- Then jumps to the standard glyph-data read

**Active in YR: No (dormant).** The only `.FNT` string in `gamemd.exe` is `GAME.FNT @ 0x00818b98`, and `GAME.FNT` uses the lowercase-magic `fonT` format. The `FonT` path exists as dead support code, likely from Tiberian Sun. **A Rust reimplementation does not need to handle `FonT` for any documented YR/RA2 asset, but the loader code path is there if needed.**

## 3. Core Logic

### 3.1 Initialization chain

```
BitText::Constructor(this)                  // 0x00434AD0
├─ this->vtable = &vtable__BitText
├─ operator_new(0x44)                       // allocates something (note: appears unused
│                                              in the visible decomp — possibly a sub-struct
│                                              or dead code; the actual BitText object is `this`)
└─ g_GAME_FNT = BitFont__Constructor("GAME.FNT")    // 0x00433880
   ├─ this[*] = constructor defaults (table above)
   ├─ this[1] = BitFont__LoadFontData("GAME.FNT")    // 0x00433990
   │  ├─ open("GAME.FNT") via CCFileClass
   │  ├─ alloc inner struct (36 bytes), zero it
   │  ├─ alloc lookup table (131072 bytes), zero it
   │  ├─ read 28 bytes; check magic:
   │  │    if fonT: read fields directly + 131072-byte lookup
   │  │    if FonT: read 36-byte header + sparse-block expansion (TS-legacy)
   │  │    else: free and fail
   │  ├─ scan lookup table, count non-zero entries → inner +0x20
   │  ├─ alloc glyph data (num_glyph_slots × glyph_stride bytes), read
   │  └─ return inner ptr or 0 on failure
   ├─ if load succeeded:
   │     this[+0x18] = inner[+0x04] (bytes_per_row)
   │     this[+0x1C] = inner[+0x0C] (cell_height)
   │     call FUN_00434700(this)                     // build fallback glyph
   │     return this
   └─ else: re-zero outer fields and return this with this[+0x04] = 0
              (BitFont with no data — measure returns 0, draw is a no-op)
```

`FUN_00434700` builds the fallback glyph for missing codepoints:

```
alloc 50 bytes (= bytes_per_row × bitmap_rows + 2 = 3×16+2 = 50)
outer[+0x08] = alloc'd buffer
g = lookup_glyph(codepoint 0xB0)          // 0xB0 = '°' degree sign in CP1252
if g exists:
  outer[+0x08][0] = g[0]                  // copy width unchanged
  for i in 1..50:
    outer[+0x08][i] = ~g[i]               // BITWISE INVERT each bitmap byte
```

Result: missing codepoints get rendered as **a bitwise-inverted '°' glyph**, with the **text color XOR'd by 0x5555** to make it visually distinct. (See §3.4.)

### 3.2 Width/height measurement — `BitFont__MeasureText` (0x00433CF0)

Signature: `MeasureText(this, wchar_t* text, int* out_width, int* out_height, int max_width)`

```
if (inner == NULL or text == NULL):
  *out_width = 0; *out_height = 0; return 0

line_height = inner[+0x0C]   // cell_height = 17
y = line_height              // running y (first line's bottom)
x = 0                        // running x on this line
chars_on_line = 0
max_width_seen = 0
last_space_pos = NULL        // pointer to wchar after the last space on this line
last_space_x = 0             // x position at that space
prev_char = 0

for each c in text:
  switch c:
    case '\t' (9):
      x += tab_width (outer[+0x28] = 64)
      x -= x % tab_width
      // (no chars_on_line increment; tab is invisible)
    case '\n' (10):
    case '\r' (13):
      if prev_char != '\r':                       // \r\n pair counts as ONE newline
        x = 0; chars_on_line = 0
        y += line_height
    case ' ' (32):
      last_space_pos = next_char_ptr               // remember this space for word wrap
      last_space_x = max_width_seen
      // FALLTHROUGH to default
    default:
      glyph = inner[+0x18][c]                      // lookup
      if (glyph_idx != 0): use glyph at glyph_data + stride * (idx-1)
      else: use outer[+0x08] fallback if present
      if (no glyph and no fallback): skip this char (no advance, no count)
      else:
        chars_on_line++
        x += outer[+0x2C] (= 1) + glyph_width
        if (max_width == 0 OR x <= max_width):
          max_width_seen = max(max_width_seen, x)
        else:                                       // overflow — wrap
          if (last_space_pos == NULL):              // no space on this line
            if (chars_on_line > 1):
              x -= (1 + glyph_width)                // back up
              next_char_ptr = current_char_ptr     // retry this char on next line
            max_width_seen = max(max_width_seen, x)
          else if (chars_on_line > 1):
            max_width_seen = last_space_x           // line width = up to space
            next_char_ptr = last_space_pos          // resume after space
          x = 0; chars_on_line = 0; y += line_height; last_space_pos = NULL
  prev_char = c

// at null terminator:
*out_width = max_width_seen
*out_height = y                                    // total height including last line
return 1
```

**Tiny details:**

- The `+1` in `outer[+0x2C] + glyph_width` is the **constructor-default inter-character spacing**, NOT a loaded FNT field. Hardcoded to `1`. This resolves the prior conflict: `SIDEBAR_READY_TEXT_RENDERING.md`'s claim that FNT field 0x08 (=3) is "Inter-character spacing" is **WRONG**. Field 0x08 is `bytes_per_row` for the bitmap stride; the Rust impl at `src/assets/fnt_file.rs:171` (adding 1 per pair) is **correct**.
- The `\r\n` CRLF pair is treated as ONE newline (the `\n` after a `\r` is suppressed). A bare `\n` or `\r` advances a line.
- The output height (`*out_height`) is `line_height × number_of_lines`. For a single-line string it equals `line_height` (= 17). Total wrapped lines × 17.
- Word wrap is **lazy**: only the first overflow per line triggers it; the engine doesn't try to redistribute words. Backs up to the last space if present; otherwise hard-breaks before the overflowing character.
- A `space` always records itself as the wrap candidate AND falls through to be drawn (the space itself ends up at the end of the previous line; the next-line render starts at the character after the space).

### 3.3 Per-glyph rasterization — `FUN_00434120`

Signature: `DrawGlyph(this, wchar_t c, int x, int y, u32 color_override)`

```
if (c == '\t'):
  return x + tab_width - ((x - outer[+0x20]) mod tab_width)
                          ^^^ tab origin (= 0 by default)

color = outer[+0x24]                       // packed 16-bit text color
if (color_override != 0xFFFFFFFF):
  color = (u16) color_override

glyph = lookup_glyph(c)                    // via inner[+0x18][c]
if (glyph == NULL):
  color ^= 0x5555                          // MISSING: invert alternating bits
  glyph = outer[+0x08] (fallback)
  if (glyph == NULL): return x             // no advance

if (outer[+0x0C] == NULL): return x        // no surface — no advance

bitmap = glyph + 1                         // skip width byte
width = glyph[0]
rows = inner[+0x08]                        // bitmap_rows = 16
bytes_per_row = outer[+0x18]               // = 3
pitch_words = outer[+0x10]                 // surface pitch in u16

if (outer[+0x41] != 0):                    // clip enabled
  rect = {x, y, x+width-1, y+rows-1}
  clipped = intersect(rect, outer[+0x30..+0x3F])
  if (no intersection): return x + outer[+0x2C] + width

  if (clip < rect): use clipped blit path (skip pixels outside clip)
  else: use unclipped path

// Unclipped blit:
for row in 0..rows:
  src = bitmap + row * bytes_per_row
  dst = surface + (y+row) * pitch_words + x
  for byte_idx in 0..bytes_per_row:
    b = src[byte_idx]
    if (b & 0x80): dst[0] = color
    if (b & 0x40): dst[1] = color
    if (b & 0x20): dst[2] = color
    if (b & 0x10): dst[3] = color
    if (b & 0x08): dst[4] = color
    if (b & 0x04): dst[5] = color
    if (b & 0x02): dst[6] = color
    if (b & 0x01): dst[7] = color
    // partial-byte check at end:
    //   if (width - byte_idx*8) < N: stop after N bits

return x + outer[+0x2C] + width            // advance
```

**Tiny details:**

- Color override: `0xFFFFFFFF` sentinel means "use outer[+0x24]". Any other value is taken as a `u16` (cast). Callers that want the configured color pass `0xFFFFFFFF`; callers like the fade path inject a tinted color directly.
- Missing-glyph XOR `0x5555`: in RGB565, this swaps **bit 0 of red, bits 0 and 2 of green, bits 0 and 2 of blue** (every odd bit position in the 16-bit word). The visual effect is a desaturated/dithered version of the original color, distinguishing missing glyphs from rendered ones at a glance.
- The 8-pixel-per-byte loop unrolls all 8 bit positions but breaks out at width boundaries via `(width - bits_drawn) < 1` checks. So a width=10 glyph reads bytes [0, 1, 2] but only draws bits 0-7 of byte 0, bits 0-1 of byte 1, plus the 2 leftover bits of byte 2 are silently consumed (the 3rd byte for a width-10 glyph contains 2 valid pixels + 6 padding bits).
- The clipped blit path increments `bitmap` and `dst` by `bytes_per_row` and `pitch_words` respectively per row — identical to the unclipped path but with per-pixel bounds checks.
- Transparent zero bits: pixels where the glyph bit is 0 are **left unchanged** in the destination buffer. Background shows through.

### 3.4 Full draw with wrap — `FUN_00434CD0` (the big one, 1602 bytes)

Signature: `DrawWithWrap(this, surface, wchar_t* text, int x, int y, int max_width, int max_height, u8 align_flags, int fade_count, int fade_range)`

The structure mirrors `BitFont__MeasureText`'s character iteration, but each non-control character triggers a "flush" that draws the accumulated chars-since-last-line-or-wrap-boundary in one batch.

**Alignment flag bits (`align_flags` aka `param_8`):**

- bit `0x01` set → horizontal CENTER (`x = base_x + (max_width - line_width) / 2`)
- bit `0x02` set → horizontal RIGHT (`x = base_x + (max_width - line_width)`)
- neither → horizontal LEFT (no adjustment)
- bit `0x04` set → vertical center is computed by `FUN_00621040` BEFORE calling (not in this function)

**Per-line flush:**

```
local_24 = 0
if (line_x < max_width):
  if (align_flags & 1): local_24 = (max_width - line_x) / 2     // center
  else if (align_flags & 2): local_24 = max_width - line_x       // right
draw_x = base_x + local_24

for each char in segment-since-last-newline:
  if (fade_count != 0):
    fade_progress = (fade_count - char_idx) - 1                  // characters past head
    visible_idx = (char_idx - fade_count) + 1 + fade_range
    if (char_idx >= fade_count): break (stop drawing)            // not yet revealed
    if (fade_progress < fade_range):                              // within fade band
      color = tint(text_color, fade_progress / fade_range)         // gradient via g_SelectedUnitHighlightColor
    else:
      color = text_color
  else:
    color = text_color
  
  if (color == 0xFFFFFFFF): break (sentinel)
  draw_x = DrawGlyph(c, draw_x, line_y, color)
```

After each line (or when wrap fires), `line_y += line_height`; if `max_height != 0 && line_y >= max_height`, drawing stops.

**The fade effect:** `param_9` (fade_count) and `param_10` (fade_range) implement a **typewriter-style reveal**:

- If `fade_count == 0`: no fade, all chars drawn in `text_color`.
- If `fade_count > 0`: only the first `fade_count` characters are drawn (the rest are hidden).
- Within that, the **last `fade_range` characters** are tinted progressively toward the highlight color (`g_SelectedUnitHighlightColor`), with the most-recently-revealed character closest to the highlight color.

This is used for animated text reveals (e.g., production-complete callouts, mission briefings). The Skirmish shell does **not** use this effect — calls from `FUN_00621040` pass `param_9 = 0`, `param_10 = 0`.

### 3.5 Shell text wrapper — `FUN_00621040`

Signature (inferred from call site to `FUN_00434CD0`):
```
FUN_00621040(surface, BitFont* font, wchar_t* text, RECT* rect, u32 rgb, u8 flags, ?, ?, fade_count, fade_range)
```

```
// Unpack rect
x = rect->left
y = rect->top
w = rect->right - rect->left
h = rect->bottom - rect->top

// Vertical center (flag 0x04)
if (flags & 4):
  MeasureText(font, text, &measured_w, &measured_h, w)
  y += (h - measured_h) / 2

// Pack RGB → 16-bit display format
packed16 = ((G & 0xFF) >> g_DD_GLoss) << g_DD_GShift
         | ((B & 0xFF) >> g_DD_BLoss) << g_DD_BShift
         | ((R & 0xFF) >> g_DD_RLoss) << g_DD_RShift

// Configure BitFont state
font->SetEnable(1)               // FUN_00433C90: outer[+0x41] = 1
font->SetClipRect(rect)          // FUN_00433CA0: outer[+0x30..+0x3F] = rect
font->SetColor(packed16)         // FUN_00433C70: outer[+0x24] = packed16

// Draw
FUN_00434CD0(font, surface, text, x, y, w, h, flags, fade_count, fade_range)
return 0
```

**Tiny details:**

- The `flags` parameter is **the same byte** passed to `FUN_00434CD0` as `align_flags`. So `0x01` = h-center, `0x02` = h-right, `0x04` = v-center. Bits 3-7 are not consumed in the paths examined.
- The clip rect IS the text rect (`rect`). There is no separate clip-vs-draw distinction. Text outside the rect is clipped pixel-by-pixel by `FUN_00434120`.
- Color conversion is the **8-bit-input-to-16-bit-display** standard: `(channel >> loss) << shift`. For RGB565: loss = 3/2/3, shift = 11/5/0. For RGB555: loss = 3/3/3, shift = 10/5/0.

### 3.6 Lower draw wrapper — `FUN_006211D0`

```
if (!g_GameRunning): return 0
if (surface == 0): surface = DAT_00887310            // default main backbuffer
if (font == 0): font = g_GAME_FNT                     // default to GAME.FNT

MeasureText(font, text, &measured_w, &measured_h, max_width)

// Horizontal align (h_mode is an integer, not a bit flag)
switch (h_mode):
  case 1: x += ((max_width - measured_w) + 1) / 2          // center
  case 2: x -= (measured_w + 1) / 2                         // anchor-from-mid (right shift by half)
  case 3: x += (-1 - measured_w)                            // right-1
  case 4: x += (max_width - measured_w) - 1                 // right-edge-1
  default: no adjust (left)

// Vertical align (v_mode integer)
switch (v_mode):
  case 1: y_off = ((rect_h - measured_h) + 1) / 2           // center
  case 2: y_off = -((measured_h + 1) / 2)                   // anchor-from-mid
  case 3: y_off = -1 - measured_h                           // above
  default: no adjust

// Set clip to bounding rect, set color
SetEnable(1); SetClipRect(rect); SetColor(packed16)

return FUN_00434B90(font, surface, text, x, y, max_width, fade_param) - x
```

**Tiny details:**

- This uses **integer mode codes** (1, 2, 3, 4) for alignment, **not bit flags**. Different convention from `FUN_00621040`. Callers must know which API they're hitting.
- The return value is `final_x - initial_x` — the pixel advance consumed by the drawn text. Useful for chaining draws (Edit-control text segments).
- `FUN_00434B90` is a thin wrapper that calls `FUN_004348F0` (SetSurface), `FUN_00434110` (SetTabOrigin), `FUN_00434500` (the actual draw — see below), and `FUN_00434990` (SetSurface cleanup).

### 3.7 Single-line draw with selected-unit fade — `FUN_00434500`

Used by `FUN_006211D0`. **Different fade implementation** from `FUN_00434CD0`:

```
saved_color = outer[+0x24]
char_idx = 0
line_offset = (9 - fade_param) * 0x1F        // initial fade offset (param_6 = fade_param)
chars_to_fade = fade_param

for each c in text:
  if (chars_to_fade != 0):
    if (chars_to_fade <= char_idx):           // fade exhausted, restore color and bail
      outer[+0x24] = saved_color
      MeasureText to advance text pointer to end
      return x + measured
    if (local_4 < 8):                           // within fade band (LAST 8 chars of fade window)
      // corrected 2026-05-28: was "first 8 chars" but binary uses `local_4` (= fade_param-1-char_idx),
      // tint fires when local_4 < 8, i.e. for chars (fade_param-8)..(fade_param-1) — ROOT_CAUSE: INFERENCE_HARDENED
      tint = FUN_006612C0(line_offset, &g_SelectedUnitHighlightColor)  // compute gradient point
      outer[+0x24] = repack(saved_color tinted with `tint`)
    line_offset += 0x1F
    char_idx++
  if (c != '\r' && c != '\n'):
    x = DrawGlyph(c, x, y, 0xFFFFFFFF)         // -1 = use outer[+0x24] (which may be faded)
  if (limit != 0 && --limit == 0): break

outer[+0x24] = saved_color                     // restore
return x
```

**The "selected unit highlight" effect:** the **last 8 characters** of the fade window (`chars (fade_param-8)..(fade_param-1)`) fade from the highlight color toward the normal text color; characters before that point draw in the normal color. (corrected 2026-05-28: was "first 8 characters" — binary `local_4 = fade_param-1-char_idx; if (local_4 < 8)` means the tint fires for the tail of the fade band, not the head — ROOT_CAUSE: INFERENCE_HARDENED.) This is the ListBox/Edit selected-item highlight effect. Different from the typewriter fade in `FUN_00434CD0`.

`g_SelectedUnitHighlightColor` is the gradient endpoint (the bright "freshly-highlighted" color). Its identity is configured by the side index per `FUN_0072F440` documented in `SIDEBAR_READY_TEXT_RENDERING.md`:

- Allied: RGB(164, 210, 255) — light sky blue
- Soviet: RGB(255, 255, 0) — yellow
- Yuri: RGB(255, 255, 0) — yellow

### 3.8 Alpha blend — `AlphaBlendRect` (0x00621B80)

Signature: `AlphaBlendRect(RECT* rect, surface, packed_16bit_blend_color, alpha_0_to_255)`
(parameters are conveyed through fastcall + stack; Ghidra's decomp is jumbled but the per-pixel math is clear.)

```
pixels = surface->Lock()
pitch_words = surface->Pitch() / 2

for row in rect.top..rect.bottom:
  for col in rect.left..rect.right:
    p = pixels[row * pitch_words + col]
    
    // Separate R/G/B using channel masks DAT_00AC48B8 and DAT_00AC48BC
    // (these masks are runtime-init BSS — values set by DirectDraw based on display format)
    R_old = p & R_mask;  G_old = p & G_mask;  B_old = p & B_mask
    R_blend = blend_color & R_mask;  // etc.
    
    R_new = (R_blend * alpha + R_old * (255 - alpha)) >> 8
    G_new = (G_blend * alpha + G_old * (255 - alpha)) >> 8
    B_new = (B_blend * alpha + B_old * (255 - alpha)) >> 8
    pixels[row * pitch_words + col] = (R_new & R_mask) | (G_new & G_mask) | (B_new & B_mask)

surface->Unlock()
```

**Used for:**

- Disabled button overlay: `AlphaBlendRect(button_rect, 0, 0x80)` — blends black at 128/255 (50%) over the drawn button
- "Ready" cameo text background: `AlphaBlendRect(text_rect, 0, 0xAF)` — blends black at 175/255 (~69%) over cameo
- Other UI fades (general-purpose helper)

**Channel masks** at `DAT_00AC48B8` / `DAT_00AC48BC`: stored as a packed pair where `DAT_00AC48B8` holds `R_mask | (G_mask << 16)` (or similar) and `DAT_00AC48BC` holds `B_mask`. The Ghidra decomp uses `DAT_00AC48B8 & 0xFFFF` and `DAT_00AC48B8._2_2_` (upper half) as the two channel masks of `DAT_00AC48B8`. These are zero in static memory because they're BSS — written by DirectDraw init.

### 3.9 Sidebar helpers — `ComputeTextRect` (0x004A59E0) and `DrawText` (0x004A60E0)

These match what `SIDEBAR_READY_TEXT_RENDERING.md` documented; verified intact:

- `ComputeTextRect(out_rect, text, x, y, flags, x_pad, y_pad)`:
  - Uses `g_GAME_FNT` to measure width and read `cell_height = 17`
  - flag `0x100` = h-center (subtract `width/2` from x)
  - flag `0x200` = h-right (subtract `width` from x)
  - `out_rect` = `(x - x_pad, y - y_pad, width + 2*x_pad, fontHeight + 2*y_pad)`
- `DrawText`: wraps `FUN_004A5EB0` (which calls into `FUN_006211D0`)

### 3.9b CORRECTION (2026-05-17): Sidebar cameo Ready text uses Path A, not Path B — no fade

Verified live in Ghidra. The real sidebar cameo paint path is:

```
StripClass__Draw (0x006A9540)
  └─ SidebarClass__DrawCameoText (0x006AC480)
        └─ FUN_00434CD0(font=g_GAME_FNT, surface=g_SidebarSurface,
                        text, x, y, max_width,
                        max_height=0, align_flags=0,
                        fade_count=0, fade_range=0)
```

This contradicts the prior assumption (and §6 item 5 below) that
`SidebarClass__DrawCameoText` reached `FUN_00434500` (Path B) with a
non-zero fade. It does not. The function calls `FUN_00434CD0` (Path A
DrawWithWrap) directly with **all four trailing args = 0**:

- `max_height = 0` — no vertical cap
- `align_flags = 0` — alignment is pre-applied by the caller (via
  `ComputeTextRect` + manual y offset using `cell_height` at
  `font + 0x1C`)
- `fade_count = 0` and `fade_range = 0` — typewriter-style fade
  disabled

Color comes from `FUN_00517440(local_30)` — the static side-dependent
3-byte RGB lookup documented in `SIDEBAR_READY_TEXT_RENDERING.md` §
"Text Color (Side-Dependent)". No animation; no per-character
gradient.

**Implication for the Rust port:** The sidebar Ready cameo text is a
**flat side-color tint** in gamemd.exe. There is no fade pulse to
reproduce. The `build_text_with_fade` math in
`src/render/sidebar_text.rs` is correct on its own, but no live caller
in the sidebar paint path needs it. The plain `build_text` call
already in `app_sidebar_build.rs` for Ready text matches gamemd's
behavior exactly.

The fade math in `FUN_00434500` exists for **`OwnerDraw_ListBox`
selected-item highlight** (and the Edit control's selection rendering
via `FUN_00623880`) — not for sidebar cameos. Those are out of scope
for current shell-text parity work.

This finding obsoletes §6 item 5's "fades exist for sidebar cameo
highlights" sentence. The actual users of Path B's fade are the
ListBox / Edit code paths.

### 3.10 Edit-control text — `FUN_00623880`

Renders an edit control's visible text with cursor, selection highlight, and password masking:

1. Get the text buffer; clip to 0x800 chars
2. If a selection exists, splice the selection content into a scratch buffer
3. If password flag (`in_stack_0000104c != 0`): replace every codepoint with `*` (0x002A — uppercase ASCII)
4. Compute scroll offset so cursor is visible (5px margin from edges)
5. Draw three text segments via `FUN_006211D0`:
   - Segment 1: text BEFORE the selection
   - Segment 2: the SELECTED text (typically drawn with inverse colors — handled by changing draw color before this call)
   - Segment 3: text AFTER the selection
6. If cursor is visible AND no selection: draw a 2-pixel-wide vertical line via `FUN_00620050` at the cursor x position, color `DAT_00AC184C` (white default)

The cursor is drawn TWICE with x and x+1 — that gives a 2-px-wide caret (1 pixel anti-aliased? or just bold).

## 4. INI Keys

| Key | Section | Default | Effect |
|---|---|---|---|
| — | — | — | The BitFont/BitText system has **no INI surface**. `GAME.FNT` is hardcoded by name in `BitText__Constructor` (string literal at `0x00818b98`). No font path, scale, or substitution is INI-driven. |

## 5. Integration Points

### Who initializes `g_GAME_FNT`?

`BitText__Constructor` (`0x00434AD0`) — and ONLY it. Writes are at offsets `0x00434AF3` and `0x00434AFE` inside the constructor. The xref scan confirms exactly two WRITE sites, both from `BitText__Constructor`. No other code writes `g_GAME_FNT`.

`BitText__Constructor` is in turn called when the shell creates its persistent UI state. The exact caller chain wasn't traced in this pass (out of scope for parity-relevant text rendering), but the global is initialized **once** at process startup, before any shell paint.

### Who reads `g_GAME_FNT`?

Confirmed by xref scan:

- `FUN_006211D0` — defaults the font parameter to `g_GAME_FNT` when caller passes 0
- `ComputeTextRect` (`0x004A59E0`) — reads to measure text and get `cell_height`
- `FUN_004A5EB0` — DrawText core
- `CCFileClass__Constructor` (multiple read sites; possibly unrelated, likely a debug-string fmt context)
- `FUN_006c9d40`, `FUN_005e2820`, `FUN_0060f4b0`, `FUN_00624530`
- `FUN_0060F9A0` (the owner-draw hook setup — reads g_GAME_FNT to install a font reference into per-control metadata)
- `OwnerDraw_ListBox` (`0x0061C010`)
- `OwnerDraw_ComboBox` (two sites: `0x00617FE3`, `0x0061826C`)
- `SidebarClass__DrawCameoText`
- Other minor sites

**Result: the entire shell text path uses `g_GAME_FNT` exclusively.** No code swaps it for a different font. `MSFont` (FULLFNT3.SHP) and `ScoreFontClass` (BIGFONT.SHP) are completely separate classes used by Map Select and the score screen respectively — they do not interact with `BitFont` or `g_GAME_FNT`.

### Shell text call paths

**Path A — owner-draw control labels (Button, Static, Checkbox, RadioVariant, ComboBox, Trackbar):**

```
OwnerDraw_*::OnPaint
  ├─ build local RECT for text area
  ├─ choose RGB color (per-control)
  └─ FUN_00621040(surface, font, text, rect, rgb, flags, …, 0, 0)
       ├─ if flags & 4: MeasureText to find text_h, adjust y for vertical center
       ├─ SetEnable(1), SetClipRect(rect), SetColor(packed_rgb_to_16bit)
       └─ FUN_00434CD0 (full draw with wrap, clip, multi-line, optional fade)
```

**Path B — list/dropdown items, button-variant text, edit-control segments:**

```
OwnerDraw_ListBox::OnPaint (or ButtonVariant, or FUN_00623880)
  └─ FUN_006211D0(rgb, font_or_NULL_for_g_GAME_FNT, …, text, …, h_mode, v_mode, surface_or_NULL, fade)
       ├─ default font = g_GAME_FNT, surface = DAT_00887310 (main backbuffer)
       ├─ MeasureText, apply H/V align offsets
       ├─ SetEnable, SetClipRect, SetColor
       └─ FUN_00434B90 → FUN_00434500 (single-line draw with selected-unit fade)
```

**Path C — Edit-control (cursor/selection/password):**

```
FUN_00623880 (called from OwnerDraw_Edit / OwnerDraw_NewEdit paint)
  ├─ apply password mask if set (every codepoint → '*')
  ├─ adjust scroll offset so cursor is in view
  ├─ FUN_006211D0(...) for text before selection
  ├─ FUN_006211D0(...) for selection (caller pre-sets selection color)
  ├─ FUN_006211D0(...) for text after selection
  └─ if cursor visible and no selection: FUN_00620050 to draw 2px caret
```

### Where in the tick cycle?

All shell text rendering is on the WM_PAINT path of dialog windows (not the simulation tick). The Skirmish dialog `0x102` paints whenever Windows invalidates it (initial show, focus change, control update, etc.). The owner-draw callbacks paint each control through Path A or B. No tick ordering concern — these run synchronously inside the message loop.

## 6. Current Rust Implementation Status

| Rust file | What it does | Status vs binary |
|---|---|---|
| `src/assets/fnt_file.rs` (273 lines) | Parses GAME.FNT (fonT magic only); decodes 1bpp glyph rows to RGBA white-on-transparent; `text_width()` adds 1px per glyph pair | **Faithful for fonT format.** Header field semantics match (Rust `bytes_per_row` = file 0x08 = 3, etc.). Missing: FonT (capital F) format — but no YR asset uses it, so OK to skip. Missing: full struct field 0 (file 0x04 = 20) — unused in measure/draw, can be ignored. |
| `src/render/sidebar_text.rs` | Packs glyphs 0x20–0x180 into a GPU atlas; emits sprite quads; has 1×1 "darken" texture for Ready overlay | **Codepoint range too narrow.** GAME.FNT contains ~29K glyph slots (Korean, Chinese, Russian, Latin extended). For localized YR builds, characters above 0x180 will fall back to missing-glyph behavior. Also: no per-pixel clip, no vertical-center, no fade — these features exist in the binary but aren't yet wired into the sprite renderer. |
| `src/app_skirmish_shell_render.rs` (line 242, 246) | Uses `state.sidebar_text.text_width()` and `build_text()` for shell labels | **Inherits sidebar_text gaps** — same codepoint range limit, no v-center, no fade. For Skirmish setup screen specifically, basic ASCII labels render correctly but localized strings or strings needing v-center within a button won't match binary exactly. |
| `src/app_sidebar_text.rs` | App-layer text builder | Same scope as sidebar_text |

**Recommendations** (the brainstorm follow-up will cover):

1. **Pixel-faithful spacing is already correct** — the Rust `+1` per pair matches the binary's hardcoded inter-character spacing of 1.
2. **Line height is correct** — both use `cell_height = 17`.
3. **Glyph rasterization is correct** — both decode 16 rows × 3 bytes per row, 1-bit-per-pixel, MSB-first.
4. **For owner-draw shell parity**, need to add (extending `sidebar_text` or forking `shell_text`):
   - Vertical centering helper (measure → offset y by `(rect_h - text_h) / 2`)
   - Clip rect (skip pixels outside)
   - Multi-line wrap (at last space, else hard cut)
   - Tab stops (advance to next multiple of 64px)
   - Word-wrap-on-overflow with backtrack
   - CRLF normalization (`\r\n` = 1 newline)
   - Missing-glyph fallback with color XOR 0x5555
   - Optional fade overlays (typewriter and selected-unit-highlight variants)
5. **For localization**, expand codepoint range from `0x20..0x180` to the full lookup (or load on demand based on locale).

## 7. Open Questions

1. **FNT field 0 (file offset 0x04, inner +0x00, GAME.FNT value 20)** — purpose unverified. The constructor copies this dword into the inner struct but no read site in `MeasureText`, `DrawGlyph`, or the wrappers I examined consumes it. The prior `SIDEBAR_READY_TEXT_RENDERING.md` calls it "Characters per row in sheet" — plausible if the FNT file once had a 2D glyph-sheet layout, but the current code uses a 1D array of slots indexed by `glyph_stride`, so this field is **almost certainly unused at runtime**. A targeted xref scan on inner +0x00 reads could confirm; for parity, Rust can keep ignoring it.

2. **Outer +0x26 = 0x3555 second color** — constructor sets a second u16 color immediately after the primary text color, but no path examined reads it. Possibilities: shadow color, fade endpoint color, outline color for a feature not used in standard shells. Not needed for shell-text parity.

3. **Channel masks `DAT_00AC48B8` / `DAT_00AC48BC` and `g_DD_*Loss/Shift` exact values** — these are runtime-init BSS, populated by DirectDraw on start. Static read returns zeros. The arithmetic SHAPE is fully documented (`(channel >> loss) << shift` for pack; reverse for extract). For final pixel parity, the native 16-bit packing is a parity constraint even when Rust ultimately presents through a 32-bit RGBA/BGRA target. The current local `DDrawCompat-gamemd.log` selects `D3DDDIFMT_R5G6B5`, and three sealed executable shell frames from the enrolled AMD/DDrawCompat/DXGI guard independently exercise exactly 32 red/blue and 64 green presentation values. Local RGB565 fixtures must therefore reproduce the same packed result and that guard-observed expansion before claiming exact pixels. This is scoped evidence for the enrolled presentation chain, not a universal RGB565 claim or a universal GPU transfer function.

4. **Vtable for BitFont and BitText** — vtables at `&vtable__BitFont` and `&vtable__BitText` weren't dereferenced. They probably contain destructors and possibly Load/Save methods. Not relevant to shell text parity.

5. **`FUN_00434500` selected-unit fade vs `FUN_00434CD0` param_9/10 fade** — two different fade implementations. The Skirmish shell doesn't use either (Path A passes 0, 0; Path B doesn't expose the fade through any documented caller). The fades exist for sidebar cameo highlights (Path B's, via `SidebarClass__DrawCameoText`) and possibly other animated UI. Not blocking for shell parity.

6. **The `0x00435584` write to `g_GAME_FNT`** — appears in xref scan attributed to `BitText__Constructor` but lies outside the visible body of the 57-byte function at `0x00434AD0..0x00434B09`. Likely Ghidra has merged a second BitText overload or a related function under the same symbol. Worth investigating for completeness but not blocking — the visible write at `0x00434AF3` is the live shell-init path.

## Sources

**Ghidra functions decompiled (all in `gamemd.exe`):**

- `BitFont__Constructor @ 0x00433880`
- `BitFont__LoadFontData @ 0x00433990`
- `BitFont__MeasureText @ 0x00433CF0`
- `BitFont__GetTextWidth @ 0x00433ED0`
- `BitFont__SetColor (FUN_00433C70)` — outer +0x24 setter
- `BitFont__SetEnable (FUN_00433C90)` — outer +0x41 setter
- `BitFont__SetClipRect (FUN_00433CA0)` — outer +0x30..+0x3F setter
- `BitFont__SetTabOrigin (FUN_00434110)` — outer +0x20 setter
- `BitFont__SetSurface (FUN_004348F0)` — surface lock + outer +0x0C/+0x10/+0x30 setup
- `BitFont__DrawGlyph (FUN_00434120)` — per-glyph 1bpp blit
- `BitFont__DrawLineWithFade (FUN_00434500)` — single-line draw + selected-unit fade
- `BitFont__GlyphAt (FUN_004346C0)` — lookup helper
- `BitFont__BuildFallbackGlyph (FUN_00434700)` — inverted '°' fallback
- `BitFont__DrawWithWrap (FUN_00434CD0)` — full draw, 1602 bytes, wrap/clip/fade/alignment
- `BitFont__DrawLine (FUN_00434B90)` — wrapper around `FUN_00434500`
- `BitText__Constructor @ 0x00434AD0` — initializes `g_GAME_FNT`
- `AlphaBlendRect @ 0x00621B80`
- `ShellText__DrawInRect (FUN_00621040)` — RGB+RECT+flags wrapper into `FUN_00434CD0`
- `ShellText__DrawWithAlign (FUN_006211D0)` — integer-mode alignment, defaults to `g_GAME_FNT`
- `ShellText__DrawEditWithCursor (FUN_00623880)` — Edit-control text path
- `ComputeTextRect @ 0x004A59E0` — sidebar text-rect helper
- `DrawText @ 0x004A60E0` — sidebar wrapper

**Memory reads:**

- `g_GAME_FNT @ 0x0089C4D0` — xref scan confirmed 2 writes (BitText__Constructor) and ~30 reads (shell text path)
- `s_GAME_FNT @ 0x00818B98` — the ASCII string "GAME.FNT" passed to `BitFont__Constructor`
- `0x008A0DD0–0x008A0DE4` — g_DD_RLoss/RShift/etc. — confirmed BSS (24 bytes static zero)
- `0x00AC48B8 / 0x00AC48BC` — AlphaBlend channel masks — confirmed BSS (16 bytes static zero)

**Prior docs cross-referenced:**

- `C:/Users/enok/Documents/ra2-rust-game-docs/SIDEBAR_READY_TEXT_RENDERING.md` — partially supplanted by this report. The doc's "Font: GAME.FNT" section had wrong claims about FNT field 0x08 (= bytes_per_row, not "Inter-character spacing") and wrong claims about BitFont struct +0x18/+0x1C (those positions in the **inner** struct hold pointers, not values — though the **outer** struct does have `bytes_per_row` and `cell_height` at those offsets, copied from inner +0x04 and inner +0x0C by the constructor). The doc's "Ready" call-site analysis and side color table are still correct.
- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md` §1.2 — named `FUN_00621040`/`FUN_006211D0`/`FUN_00623880` correctly; this report extends with full algorithm and signature.
- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md` §0x00621040 — documented flag 0x04 (vertical center) and the color-conversion shape correctly. This report adds the full body and the missing flag bits.
- `docs/plans/2026-05-17-bitfont-shell-text-investigation-plan.md` — the scoping plan executed by this report. All 19 inventory items addressed (plus the 4 review refinements).

**INI files checked:** none — no INI surface exists.

**Rust files inspected:** `src/assets/fnt_file.rs`, `src/render/sidebar_text.rs` (first 80 lines), `src/app_skirmish_shell_render.rs:242`.
