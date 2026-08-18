# Sidebar "READY" Text Rendering — Ghidra Analysis

Reverse-engineered from `gamemd.exe` via Ghidra MCP decompilation.
Covers the exact rendering pipeline for the "Ready" text overlay on
sidebar cameo icons when production completes.

## Key Functions

| Address      | Name (inferred)              | Role                                        |
|--------------|------------------------------|---------------------------------------------|
| `0x006a9540` | `StripClass::Draw`           | Main cameo rendering loop (`Sidebar.CPP`)   |
| `0x006a6c30` | `SidebarClass::Draw`         | Parent; calls `StripClass::Draw`            |
| `0x004ca130` | `FactoryClass::HasCompleted` | Returns 1 when production is done           |
| `0x004ca120` | `FactoryClass::GetProgress`  | Returns `factory->Status` (step counter)    |
| `0x00734e60` | `StringTable::LoadString`    | CSF string table lookup (localized text)    |
| `0x004a59e0` | `ComputeTextRect`            | Calculates aligned text bounding box        |
| `0x00621b80` | `AlphaBlendRect`             | Pixel-level alpha tint over a rectangle     |
| `0x004a60e0` | `DrawText`                   | Renders text string to surface              |
| `0x004aed70` | `DrawSHP`                    | Blits SHP frames (cameo icon, overlays)     |
| `0x006ac480` | `DrawProgressBar`            | Clock/bar progress overlay                  |

## Production Completion Check (`0x004ca130`)

```c
// FactoryClass::HasCompleted
// factory->Status lives at offset +0x24
// factory->Object at +0x58, factory->QueuedObject at +0x68
bool HasCompleted(Factory* self) {
    if (self->Object != NULL && self->Status == 0x36) return true;
    if (self->QueuedObject != -1 && self->Status == 0x36) return true;
    return false;
}
```

`Status == 0x36` (54 decimal) = production step counter has reached completion.
`FactoryClass::GetProgress` at `0x004ca120` simply returns `*(int*)(this + 0x24)`.

## Font: GAME.FNT

Loaded by `FUN_00433880` → `FUN_00433990` from the MIX archives.
Stored in global `DAT_0089c4d0` (the game's primary bitmap font object).

### Extracted header (from retail `GAME.FNT`, 1,584,195 bytes)

| Offset | Value  | Meaning                           |
|--------|--------|-----------------------------------|
| 0x00   | `fonT` | Magic (0x546E6F66)                |
| 0x04   | 20     | Characters per row in sheet       |
| 0x08   | 3      | Inter-character spacing           |
| 0x0C   | 16     | (internal field)                  |
| 0x10   | 17     | **Character cell height (pixels)**|
| 0x14   | 29655  | Pixel data row stride             |
| 0x18   | 49     | Pixel data total rows             |

Font object layout (offset from object base):
- `+0x18` (`[6]`): char spacing, set from FNT header offset 0x08 = **3**
- `+0x1C` (`[7]`): char height, set from FNT header offset 0x10 = **17**

Variable-width proportional glyphs — text width is measured per-string
via `FUN_00433ed0` / `FUN_00433cf0`.

## TXT_READY String Lookup

CSF string key `TXT_READY` at address `0x0081b454` (unicode).

Assembly at `0x006a983b` in `StripClass::Draw`:

```asm
006a9822: CALL 0x004ca130          ; HasCompleted(factory)?
006a9827: TEST AL, AL
006a982d: JZ   0x006a984b          ; skip if NOT complete
006a982f: PUSH 0xd53               ; source line (debug)
006a9834: PUSH 0x83fac4            ; "D:\ra2mdpost\Sidebar.CPP"
006a9839: XOR  EDX, EDX            ; no extra string output
006a983b: MOV  ECX, 0x81b454       ; "TXT_READY" CSF key
006a9840: CALL 0x00734e60          ; StringTable::LoadString
006a9845: MOV  [ESP+0x40], EAX     ; store localized wchar_t*
```

Returns the localized "Ready" string (e.g., "Ready" in English,
language-appropriate in other locales).

## Text Rect Computation (`0x004a59e0`)

```c
// __fastcall: ECX = output_rect, EDX = text_string
// stack: x, y, flags, x_pad, y_pad
int* ComputeTextRect(int* out, wchar_t* text, int x, int y,
                     uint flags, int x_pad, int y_pad) {
    int textWidth = MeasureTextWidth(text);
    int fontHeight = font->char_height;  // 17

    if (flags & 0x100)          // horizontal center
        x -= textWidth / 2;
    else if (flags & 0x200)     // right-align
        x -= textWidth;

    out[0] = x - x_pad;                  // rect.x
    out[1] = y - y_pad;                  // rect.y
    out[2] = textWidth + x_pad * 2;      // rect.w
    out[3] = fontHeight + y_pad * 2;     // rect.h
    return out;
}
```

### Call parameters for the Ready text

```c
ComputeTextRect(
    output,
    ready_text,           // "Ready" (localized)
    cameo_x + 30,         // anchor X = horizontal center of 60px cameo
    cameo_y + 1,          // anchor Y = 1px below top edge
    0x142,                // flags: 0x100 (h-center) | 0x40 | 0x02
    2,                    // x_pad = 2px
    1                     // y_pad = 1px
);
```

### Resulting dark rect

```
rect.x = cameo_x + 30 - textWidth/2 - 2
rect.y = cameo_y + 1 - 1 = cameo_y          (top edge of cameo)
rect.w = textWidth + 4
rect.h = 17 + 2 = 19 pixels
```

## Alpha Darkening (`0x00621b80`)

Called as `AlphaBlendRect(0, 0xAF)` — blends black (color=0) at
alpha 175/255 over the computed text rect.

Per-pixel math (16-bit surface):
```
new_pixel = (0 * 175 + old_pixel * 80) >> 8
          ≈ old_pixel * 0.3125
```

Darkens the cameo pixels underneath to ~31% brightness.
Only the text bounding rect is darkened, not the full cameo.

## Text Drawing

After darkening, `FUN_004a60e0` (`DrawText`) renders the localized
string centered in the rect, using the current text color.

## Text Color (Side-Dependent)

Set by `FUN_0072f440` based on side index, called from
`FUN_00534fa0` ("Preparing Mixfiles for Side").

Hardcoded values decoded from instructions at `0x0072a940`:

| Side    | Mode | Bytes (R, G, B)  | RGB Color          |
|---------|------|------------------|--------------------|
| Allied  | 0    | 0xA4, 0xD2, 0xFF | (164, 210, 255) — light sky blue |
| Soviet  | 1    | 0xFF, 0xFF, 0x00 | (255, 255, 0) — yellow           |
| Yuri    | 2    | 0xFF, 0xFF, 0x00 | (255, 255, 0) — yellow           |

Color is converted to display surface pixel format at draw time
using bit-shift descriptors at `DAT_008a0dd0`–`DAT_008a0de4`.

## Visual Layout on 60×48 Cameo

```
┌──────────────────────────────────────────────────────────────┐ ← cameo_y
│                    ▓▓▓▓▓ Ready ▓▓▓▓▓                        │
│              19px dark strip (font 17 + pad 2)               │ ← 40% of cameo
├──────────────────────────────────────────────────────────────┤ ← cameo_y + 19
│                                                              │
│                     (cameo art)                              │ ← remaining 29px
│                   full brightness                            │
│                                                              │
└──────────────────────────────────────────────────────────────┘ ← cameo_y + 48
```

- Dark strip width = text width + 4px (narrower than full cameo)
- Dark strip is horizontally centered on cameo
- Cameo art outside the strip remains at full brightness

## Super Weapon Cameos (alternate path)

When cameo slot type == `0x1F`, the sidebar branches to super weapon logic:

- `FUN_006cc2b0` (`Super.CPP`) returns status text (ready/hold/charge stage)
- `FUN_006cbee0` returns progress bar frame index (0–54, where 54 = fully charged)
- Same text rendering pipeline applies (darken rect + draw text)

## Full Cameo Rendering Order (per slot)

1. **Cameo SHP image** — `DrawSHP` with flag `0x400`
2. **Selection highlight** — colored border tint (if selected)
3. **"Can't build" overlay** — `DrawSHP` with flag `0x401` (darken if unavailable)
4. **Flash effect** — blink overlay using `frame_counter % 16 > 8`
5. **Queue count text** — "×N" at top-right (if multiple queued)
6. **Status/Ready text** — darkened strip + centered text (this document)
7. **Progress bar** — `DrawProgressBar` clock overlay

## Key Constants Summary

| Constant                | Value | Notes                            |
|-------------------------|-------|----------------------------------|
| Cameo size              | 60×48 | pixels                           |
| Font height (GAME.FNT)  | 17    | pixels (cell height)             |
| Production complete     | 0x36  | 54 steps                         |
| Dark strip height       | 19    | font 17 + 2×1 padding            |
| Dark strip padding X    | 2     | pixels each side                 |
| Dark strip padding Y    | 1     | pixels top and bottom            |
| Alpha darken level      | 0xAF  | 175/255 ≈ 69% black blend        |
| Brightness after darken | ~31%  | of original pixel values         |
| Text alignment flags    | 0x142 | 0x100 = h-center                 |
| CSF key                 | `TXT_READY` | Localized "Ready" string   |
