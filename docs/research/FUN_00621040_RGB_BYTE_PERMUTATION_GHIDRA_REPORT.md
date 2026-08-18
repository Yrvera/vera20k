# FUN_00621040 RGB Byte Permutation — Ghidra Research Report

**Address:** `0x00621040`
**Confidence:** HIGH (content, identity, binding — all verified from live decompilation and disassembly in this session)
**Active in YR:** Yes — called on every WM_PAINT of every owner-draw control on every shell dialog (main menu, skirmish setup, WOL). Hot path, not gated.

---

## 1. Overview

`FUN_00621040` (known alias `ShellText__DrawInRect`) is the shell-text color-conversion and
draw-dispatch wrapper. It receives a 24-bit packed RGB value in `param_5` / `arg_3`, extracts
the three byte-channels in a specific order, applies `g_DD_*Loss` / `g_DD_*Shift` globals to
convert each channel to the active DirectDraw 16-bit display format, ORs the three channel
contributions together into a single 16-bit packed color, stores it into `BitFont::outer[+0x24]`
via `FUN_00433c70`, and dispatches into `FUN_00434cd0` (DrawWithWrap) for the actual glyph
rasterization.

**The constant `DAT_00AC18A4 = 0x0000FFFF`** (loaded from the global initialized to `0xFFFF` by
`FUN_0060FA3F` at `0x0060fa3f`) is the default shell text color. The question this report resolves
is whether `0x0000FFFF` represents cyan `#00FFFF` or yellow `#FFFF00` after the byte permutation.
**Answer: yellow `#FFFF00`.**

---

## 2. Exact Byte Extraction — Disassembly Evidence

Primary source: `disassemble_function(0x00621040)`, verified in this session.

The color argument is passed at `[ESP+0x1c]` (stack arg_3 in the `__fastcall` convention where
ECX=surface, EDX=text). It is loaded into EAX and then split into three channels immediately:

```asm
00621043: MOV EAX, dword ptr [ESP+0x1c]   ; EAX = color arg (e.g. 0x0000FFFF)
00621054: MOV BL, AH                       ; BL  = byte 1 of EAX = bits[8..15]
00621056: AND EBX, 0xff                    ; EBX = byte 1 (zero-extended)
00621066: MOV ESI, EAX                     ; ESI = full color (copy before AND)
00621068: AND EAX, 0xff                    ; EAX = byte 0 = bits[0..7]
0062106d: SHR ESI, 0x10                   ; ESI = byte 2 = bits[16..23]
```

**Channel assignments (source byte → display role):**

| Register after extraction | Source | Meaning |
|---|---|---|
| EBX = `(color >> 8) & 0xFF` | byte 1 | **Green channel input** |
| EAX = `color & 0xFF`         | byte 0 | **Red channel input** |
| ESI = `(color >> 16) & 0xFF` | byte 2 | **Blue channel input** |

For `0x0000FFFF`:
- byte 0 = `0xFF` → **R = 255**
- byte 1 = `0xFF` → **G = 255**
- byte 2 = `0x00` → **B = 0**

This is the byte-order convention `0x00BBGGRR` (little-endian packed: low byte = R, middle = G,
high = B). Under this convention `0x0000FFFF` = R=255, G=255, B=0 = **yellow `#FFFF00`**.

---

## 3. g_DD_*Loss / g_DD_*Shift Global Table

Exact addresses confirmed from `FUN_00621040` disassembly (read sites) and
`DSurface__Constructor @ 0x004ba9d0` (write sites), verified in this session via
`get_xrefs_to(0x008a0dd0)` and `get_assembly_context`.

| Global | Address | Role in formula |
|---|---|---|
| `g_DD_RShift` | `0x008a0dd0` | Right-shift for R contribution |
| `g_DD_RLoss`  | `0x008a0dd4` | Precision-loss for R (how many low bits to drop) |
| `g_DD_BShift` | `0x008a0dd8` | Right-shift for B contribution |
| `g_DD_BLoss`  | `0x008a0ddc` | Precision-loss for B |
| `g_DD_GShift` | `0x008a0de0` | Right-shift for G contribution |
| `g_DD_GLoss`  | `0x008a0de4` | Precision-loss for G |

All six addresses are BSS — zero in static memory, populated at runtime by
`DSurface__Constructor` after querying DirectDraw's surface pixel-format masks
(`DAT_008a0958` = R mask, `DAT_008a095c` = G mask, `DAT_008a0960` = B mask).

**Initialization algorithm** (confirmed from decompile + assembly context of `DSurface__Constructor`):
```
// For each channel mask (R/G/B):
shift = 0; loss = 0
while shift < 16 and (mask & 1) == 0:
    mask >>= 1; shift++
while loss < 8 and (mask & 0x80) == 0:
    mask <<= 1; loss++
```

### RGB565 (standard 16bpp, masks R=0xF800 G=0x07E0 B=0x001F)

Confirmed by `DSurface__Constructor`'s explicit branch guard at ~`0x004bab5f`:
`g_DD_BShift==0, g_DD_BLoss==3, g_DD_GShift==5, g_DD_GLoss==2, g_DD_RShift==11 (0xb), g_DD_RLoss==3`
→ sets `DAT_008205d0 = g_DD_GLoss (= 2)` (RGB565 indicator).

| Channel | Mask   | Shift | Loss |
|---------|--------|-------|------|
| R       | 0xF800 | 11    | 3    |
| G       | 0x07E0 | 5     | 2    |
| B       | 0x001F | 0     | 3    |

### RGB555 (15bpp, masks R=0x7C00 G=0x03E0 B=0x001F)

Confirmed by `DSurface__Constructor` branch at `g_DD_BShift==0, g_DD_BLoss==3, g_DD_GShift==5,
g_DD_GLoss==3, g_DD_RShift==10 (0xa), g_DD_RLoss==3` → sets `DAT_008205d0 = 0` (RGB555 indicator).

| Channel | Mask   | Shift | Loss |
|---------|--------|-------|------|
| R       | 0x7C00 | 10    | 3    |
| G       | 0x03E0 | 5     | 3    |
| B       | 0x001F | 0     | 3    |

---

## 4. Full Color Formula and Output for `0x0000FFFF`

The formula (from `FUN_00621040` decompilation, confirmed against disassembly):
```
packed16 = ((G_byte >> GLoss) << GShift)
         | ((B_byte >> BLoss) << BShift)
         | ((R_byte >> RLoss) << RShift)
```

### RGB565

```
R_byte=0xFF, G_byte=0xFF, B_byte=0x00

R contribution: (0xFF >> 3) << 11 = 0x1F << 11 = 0xF800
G contribution: (0xFF >> 2) <<  5 = 0x3F <<  5 = 0x07E0
B contribution: (0x00 >> 3) <<  0 = 0x00 <<  0 = 0x0000

packed16 = 0xF800 | 0x07E0 | 0x0000 = 0xFFE0
```

RGB565 decode of `0xFFE0`: R=31/31, G=63/63, B=0/31 → **#FFFF00 yellow**

### RGB555

```
R contribution: (0xFF >> 3) << 10 = 0x1F << 10 = 0x7C00
G contribution: (0xFF >> 3) <<  5 = 0x1F <<  5 = 0x03E0
B contribution: (0x00 >> 3) <<  0 = 0x00 <<  0 = 0x0000

packed16 = 0x7C00 | 0x03E0 | 0x0000 = 0x7FE0
```

RGB555 decode of `0x7FE0`: R=31/31, G=31/31, B=0/31 → **#FFFF00 yellow**

**Both standard 16bpp modes produce yellow `#FFFF00`, not cyan `#00FFFF`.**

---

## 5. Why Not Cyan?

A strict `0x00RRGGBB` interpretation of `0x0000FFFF` would give R=0x00, G=0xFF, B=0xFF = cyan.
The binary does NOT use `0x00RRGGBB`. It uses `0x00BBGGRR` (low byte = R). This is confirmed
directly from the byte-extraction sequence in the disassembly (§2): byte 0 drives R, byte 1
drives G, byte 2 drives B. So `0x0000FFFF`:
- byte 0 = `0xFF` → R = 255 (full red)
- byte 1 = `0xFF` → G = 255 (full green)
- byte 2 = `0x00` → B = 0 (no blue)
= **yellow**.

The confusion arises because `0x0000FFFF` can be read as ABGR (common OpenGL/DirectX convention
where FFFF = G+B) or as BGR-packed (common Win32 COLORREF where FFFF = R+G). gamemd.exe uses
the BGR-packed / COLORREF convention: low byte is R. The Rust port's constant
`SHELL_BUTTON_TEXT_RGB_FFFF00 = [1.0, 1.0, 0.0]` (R=1.0, G=1.0, B=0.0) is **correct**.

---

## 6. Open Questions — Final State

- `[RESOLVED] OQ1` — Which byte of the 24-bit color arg is R/G/B in FUN_00621040? →
  byte 0=R, byte 1=G, byte 2=B (0x00BBGGRR layout). (evidence: `disassemble_function(0x00621040)`
  instructions at `0x00621054`, `0x00621068`, `0x0062106d`)

- `[RESOLVED] OQ2` — What are the runtime values of g_DD_*Shift/*Loss for RGB565 and RGB555?
  → RGB565: RShift=11, RLoss=3, GShift=5, GLoss=2, BShift=0, BLoss=3.
    RGB555: RShift=10, RLoss=3, GShift=5, GLoss=3, BShift=0, BLoss=3.
  (evidence: `DSurface__Constructor @ 0x004ba9d0` decompile + assembly context at `0x004ba9d6`,
  `0x004ba9dc`, `0x004baa09`, `0x004baa0f`, `0x004baa15`, `0x004baa49`, `0x004baa4e`)

- `[RESOLVED] OQ3` — Does FUN_00621040 produce yellow or cyan for input 0x0000FFFF? →
  **Yellow #FFFF00** in both RGB565 (packed16=0xFFE0) and RGB555 (packed16=0x7FE0).
  (evidence: formula application in §4; disassembly of `FUN_00621040`; branch guards in
  `DSurface__Constructor`)

- `[RESOLVED] OQ4` — Is FUN_00433c70 (SetColor) simply a store? → Yes: stores `param_2` into
  `BitFont::outer[+0x24]` and returns old value. No transformation.
  (evidence: `decompile_function(0x00433c70)`)

- `[RESOLVED] OQ5` — Is the Rust port's SHELL_BUTTON_TEXT_RGB_FFFF00 = [1.0, 1.0, 0.0] correct?
  → Yes. RGB(255, 255, 0) = yellow #FFFF00 matches the binary output.
  (evidence: `src/app_main_menu_shell_render.rs:21`)

- `[DEFERRED] OQ6` — What is DAT_008205d0 (the display-mode flag) read by, and does it affect
  any other color path not covered here? (category: `out-of-scope`; reason: scope is FUN_00621040
  byte permutation only; the display-mode flag identifies 565/555/other but the permutation
  formula is unconditional regardless of mode — Loss/Shift handle it; next-step-if-pursued:
  search xrefs to `0x008205d0` for any conditional rendering based on this flag.)

- `[DEFERRED] OQ7` — What is the packed16 output for the disabled-text color {R=0x48, G=0x00,
  B=0x00} (dark red #480000) under the same formula? (category: `out-of-scope`; reason: this
  report is scoped to the yellow/cyan ambiguity for `DAT_00AC18A4 = 0x0000FFFF`; the disabled
  path is fully documented in `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md`; next-step-if-pursued:
  apply the same formula: RGB565 → R=(0x48>>3)<<11=0x4800, G=0, B=0 → 0x4800 = dull dark red.)

---

## 7. Current Rust Implementation Status

| File | Constant | Value | Status |
|---|---|---|---|
| `src/app_main_menu_shell_render.rs:21` | `SHELL_BUTTON_TEXT_RGB_FFFF00` | `[1.0, 1.0, 0.0]` | **Correct** — yellow #FFFF00 |
| `src/app_main_menu_shell_render.rs:113` | used for button text | references above | **Correct** |

No parity bug. The Rust port's yellow `#FFFF00` is the exact correct color that
`FUN_00621040` produces for `DAT_00AC18A4 = 0x0000FFFF`.

---

## 8. Sources

**Ghidra functions decompiled / disassembled (read-only, this session):**

- `FUN_00621040 @ 0x00621040` — decompile + full disassembly
- `DSurface__Constructor @ 0x004ba9d0` — decompile (Loss/Shift init algorithm)
- `FUN_00433c70 @ 0x00433c70` — decompile (SetColor = simple store to outer[+0x24])

**Ghidra read-only queries:**

- `get_xrefs_to(0x008a0dd0)` — confirmed write sites in `DSurface__Constructor` at
  `0x004ba9d6` and `0x004ba9ee`
- `get_assembly_context(0x004ba9d6, 0x004ba9ee, 0x004baa00, 0x004baa05, 0x004baa09, 0x004baa0f, 0x004baa15)` —
  confirmed full write sequence mapping addresses to R/G/B channels
- `read_memory(0x008a0dd0, 24)` — confirmed all BSS (zeros at static analysis time, runtime-init)
- `read_memory(0x00AC18A4, 4)` — confirmed BSS (populated at runtime by FUN_0060FA3F)

**Prior documents referenced (read-only):**

- `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` — §3.5 documents FUN_00621040's algorithm shape
  and g_DD_*Loss/*Shift layout; confirmed consistent with live decompilation
- `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md` — §3 Q3 documents arg_3 as color
  `0x00BBGGRR`; confirmed consistent
- `MAIN_MENU_TITLE_TEXT_RENDER_GHIDRA_REPORT.md` — §8 is the open question this report resolves

**INI files:** none — no INI surface for this system.

**Rust files inspected (read-only):**

- `src/app_main_menu_shell_render.rs:21` — `SHELL_BUTTON_TEXT_RGB_FFFF00 = [1.0, 1.0, 0.0]`
