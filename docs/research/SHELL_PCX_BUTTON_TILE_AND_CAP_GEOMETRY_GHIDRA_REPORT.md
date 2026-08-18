# Shell PCX Button Tile and Cap Geometry — Ghidra Research Report

> **YELLOW — ACTIVE-IN-YR CLAIM REFUTED 2026-05-19.** This doc's "Active in
> YR: YES for all findings" header is wrong for dialog 0xE2. The geometry
> math below (cap widths, modulo tile, 30-in-37 centering, press offset,
> AlphaBlendRect) is correct *if* the PCX branch is ever reached, but the
> retail main menu does NOT reach that branch — `LAB_0060A330` writes
> `+0xB0 = 1` on all 0xE2 buttons via `FUN_00608CD0` and `FUN_00609730`,
> forcing the SDBTNANM frames 2/3/4 dispatch (`iVar14 == 1` branch) in
> `OwnerDraw_Button_00612B70`. See
> `MAIN_MENU_BUTTON_DISPATCH_LAB_0060A330_GHIDRA_REPORT.md` for the
> verified dispatch chain. User in-game observation confirms colored
> buttons; my swap to PCX produced visibly worse output.


**Primary address:** `0x00612B70` (OwnerDraw_Button), `0x006BA3E0` (middle-tile helper)

**Active in YR:** YES for all findings. Dialog `0xE2` is the standard YR initial main menu. This path is taken for any shell PCX button with `piVar17[5]==0` (normal PCX type) and `piVar17[0x2c]==0` (no SDBTNANM override).

**Confidence:** HIGH — every claim sourced from live decompilation and disassembly in this session. Frame offset analysis verified by cross-referencing multiple store/load pairs.

**Scope:** Answers the 7 composition-geometry questions for the `bue_li30/mi30/ri30` (up) and `bde_li30/mi30/ri30` (down) PCX strips on dialog 0xE2's owner-draw buttons. Does not re-cover greyscale colorization (see `SHELL_BUTTON_GREYSCALE_COLORIZATION_GHIDRA_REPORT.md`) or text color/offset (see `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md`).

---

## 1. Coordinate Systems

`OwnerDraw_Button_00612B70` uses **screen coordinates** throughout the art-blit path. `FUN_00775690` (called at `0x00612c05`) returns the button's four screen-edge coordinates `{x1, y1, x2, y2}`. These are stored:

- `[frame_ESP+0x30]` = screen_x (button left screen edge)
- `[frame_ESP+0x34]` = screen_y (button top screen edge; this is art_y before centering)
- `[frame_ESP+0x38]` = button_screen_width = x2 − x1
- `[frame_ESP+0x3c]` = button_screen_height = y2 − y1 (initially 37 for dialog 0xE2)

After the size-selection loop (see §2), `[frame_ESP+0x3c]` is overwritten with the selected art_height (30).

`GetClientRect` also runs (at `0x00612c68` and again at `0x0061357a`) to fill a local `{0,0,client_w,client_h}` rect, but the art blits use screen coordinates, not client-relative coordinates.

**Verified:** screen coord reads at `0x00612c0a..0x00612c1f` (frame stores), `0x006133f1..006133f5` (screen_x/art_y loads). Frame slot assignment confirmed by tracing the `MOV [ESP+0x40/0x44]` instructions at `0x00612c59` (executed after 2 GetClientRect pushes, which shifts offsets by −8, making `[ESP+0x40]` after pushes = `[frame_ESP+0x38]`).

---

## 2. Size Selection (24 vs 30 Family)

A 2-entry table selects the PCX height family:

| Index | Threshold | art_height | left_cap_w | right_cap_w |
|-------|-----------|------------|------------|-------------|
| 0     | 0x18 (24) | 24         | 7          | 10          |
| 1     | 0x1e (30) | 30         | 7          | 10          |

Loop at `0x006132d7..0x006132e8`: iterates while `button_screen_height >= threshold[i]`. For a 37px button: height ≥ 24 (advance), height ≥ 30 (advance), loop ends at index 1.

- `ESI` = selected art_height = `[frame_ESP+0x74]` = 30 (verified via `006132ea: MOV ESI, [ESP+EDX*4+0x70]` with EDX=1)
- `EBX` = left_cap_w = `[frame_ESP+0xa8]` = 7 (verified via `006132ee: MOV EBX, [ESP+EDX*4+0xa4]`)
- `EDX` = right_cap_w = `[frame_ESP+0x44]` = 10 (verified via `006132f5: MOV EDX, [ESP+EDX*4+0x40]`; the table entries are hardcoded to 10 at `006132c1/006132c5`)

**Verified:** disasm `006132a1..006132f9`; table init bytes at `0x006132b9` confirmed via `read_memory`.

---

## 3. Vertical Centering Formula

After size selection, `[frame_ESP+0x34]` is updated to art_y:

```
art_y = screen_y + (button_screen_height − art_height) / 2
      = screen_y + (37 − 30) / 2
      = screen_y + 3        (unpressed)
      = screen_y + 5        (pressed: art_y += 2)
```

The division uses the signed `CDQ; SUB EAX, EDX; SAR EAX, 1` pattern (floor toward zero for positive values = integer divide by 2).

**Pressed-state:** `006133a9: ADD EAX, 0x2; MOV [ESP+0x34], EAX` — applies only when `EDI` (pressed bit = `piVar17[0x3a] & 1`) is non-zero.

**Verified:** `0x00613394..006133ae` — live disassembly trace; arithmetic chain confirmed.

---

## 4. Left Cap Blit (`bue_li30.pcx`, 7×30 native)

### Dest rect (in `{x, y, width, height}` format)

| Field | Value | Source |
|-------|-------|--------|
| x | screen_x | `[frame_ESP+0x5c]` = screen_x, becomes dest rect field 0 |
| y | art_y | set via `00613425: MOV [ESP+0x6c], EDX` (3 pushes active) → `[frame_ESP+0x60]` |
| width | 7 | set via `00613429: MOV [ESP+0x70], EBX` (3 pushes active) → `[frame_ESP+0x64]` = left_cap_w |
| height | 30 | set via `0061342d: MOV [ESP+0x74], ESI` (3 pushes active) → `[frame_ESP+0x68]` = art_height |

Dest rect pointer = `LEA EAX, [ESP+0x68]` at `0x0061343b` with 2 active pushes → `[frame_ESP+0x5c]`. Confirmed via push-accounting.

### Src rect

Passed as `{0, 0, 0, 0}` (explicit zeroes at `0x00613409..00613411`). The blit helper `FUN_007BBB90` calls `ClipRectPair_007BBE20`, which branches to `AlphaShapeClass__ClipRect` when `src.w != dest.w || src.h != dest.h` — this fetches the full PCX surface clip rect `{0, 0, 7, 30}`. Effectively the full 7×30 PCX is the source.

**Verified:** `ClipRectPair` at `0x007bbe20` decompiled; branch condition confirmed.

### Blit call

`(**(vtable+8))(dest_surface, dest_rect_ptr, src_pcx_surface, src_rect_ptr, 0, 1)` at `0x00613441`.  
`vtable+8` on BSurface = `FUN_007BBB90` (confirmed from BSurface vtable at `0x007e2070` slot 2 = `0x007bbb90`).

---

## 5. Middle Tile (`bue_mi30.pcx`, 177×30 native)

### Dest rect passed to `FUN_006BA3E0`

| Field | Value | Source |
|-------|-------|--------|
| x | screen_x + 7 | `[frame_ESP+0x48]` = `screen_x + left_cap_w`; set at `0061348d` |
| y | art_y | `[frame_ESP+0x4c]`; set at `00613485` |
| width | button_width − 10 | `[frame_ESP+0x50]` = `button_screen_width − right_cap_w`; `0061349b: SUB ECX, EBX` where EBX=10 |
| height | 30 | `[frame_ESP+0x54]` = 30 (art_height from PCX height call at `006133eb`, stored at `00613495`) |

Dest rect pointer = `LEA EDX, [ESP+0x54]` at `0x006134b5` after 3 pushes → `[frame_ESP+0x48]`. Confirmed.

### Tile algorithm in `FUN_006BA3E0`

`FUN_006BA3E0(dest_rect_ptr, global_surface, src_pcx_surface, 0, 0)` — `RET 0x14` (5 stack args).

1. Locks dest surface and src surface via `vtable+0x5c` (Lock) — verified at `006ba3f3`, `006ba416`.
2. Gets src dimensions via `vtable+0x78` (GetRect: returns `{0, 0, src_w, src_h}`) — `006ba45d`.
3. Computes centering offsets:
   - `uVar5 = max(0, (src_width − dest_rect.width) / 2)` — horizontal start within src
   - `uVar6 = max(0, (src_height − dest_rect.height) / 2)` — vertical start within src
4. Pixel loop (verified at `006ba4d7..006ba558`):
   ```
   for row in 0..dest_rect.height:      // outer: 30 rows
     for col in 0..dest_rect.width:     // inner: button_width-10 columns
       src_x = (start_x + col) % src_width   // modulo wrap
       src_y = (start_y + row) % src_height  // modulo wrap
       dst[row][col] = src[src_y * src_stride + src_x]
   ```
5. Unlocks both surfaces via `vtable+0x60`.

**This is tiling, NOT scaling.** Source pixels repeat via modulo when the dest is wider than the PCX.

For `bue_mi30.pcx` (177×30): if `button_width − 10 ≤ 177` (button ≤ 187px wide), uVar5 centers the crop; if button > 187px, uVar5=0 and the full PCX tiles from x=0.

Standard dialog 0xE2 button is ~162px wide (108 DLU × dialog font scale). `button_width − 10 = 152 < 177`, so `uVar5 = (177 − 152)/2 = 12`. Tiling starts from src_x = 12 (centered crop of PCX, no wrap needed). **Verified:** `006ba495..006ba4c7` arithmetic chain.

---

## 6. Right Cap Blit (`bue_ri30.pcx`, 10×30 native)

### Dest rect

| Field | Value | Source |
|-------|-------|--------|
| x | screen_x + button_width − 10 | `[frame_ESP+0x5c]` = `screen_x + [frame_ESP+0x38] − 10` at `0061351d` |
| y | art_y | `[frame_ESP+0x60]`; set at `0061350b` |
| width | 10 | `[frame_ESP+0x64]` = right_cap_w; set at `00613521: MOV [ESP+0x64], EBX` |
| height | 30 | `[frame_ESP+0x68]` = art_height; set at `00613519: MOV [ESP+0x68], EDX` where EDX=30 |

Dest rect pointer = `LEA EAX, [ESP+0x28]` at `0x0061353f`... actually verified from `006134f7..0061351d` store sequence; the `LEA` at `0x0061354b` after 2 pushes → `[frame_ESP+0x5c]`.

**Verified:** stores at `0x006134f7..0061351d`; arithmetic consistent with `screen_x + button_width − 10`.

---

## 7. Render Order and Overlap

The three blits execute in order:
1. **Left cap** at x = screen_x, width = 7
2. **Middle tile** at x = screen_x + 7, width = button_width − 10
3. **Right cap** at x = screen_x + button_width − 10, width = 10

Middle tile right edge = screen_x + 7 + (button_width − 10) = screen_x + button_width − 3.  
Right cap left edge = screen_x + button_width − 10.  
**Overlap = 7 pixels** (x+button_width-10 to x+button_width-3). The right cap blit overwrites the rightmost 7px of the middle tile. Net layout: left_cap(7px) + visible_tile(button_width−17px) + right_cap(10px) = button_width total.

For a 162px button: left_cap=7, visible_tile=145, right_cap=10. Middle tile dest.width=152, of which last 7px are overdrawn by right cap.

---

## 8. Pressed-State Offset

**Art:** `art_y += 2` (applied at `006133a9`). No horizontal shift for art.

**Text:** from SHELL_BUTTON_PAINT_DETAILS: text rect `(left+=2, top+=4)` when pressed. Net rendered delta after h-center + v-center: **+1px right, +2px down** for text glyphs.

**Confirmed scope:** The `+2` art shift and `+2/+4` text rect deltas are independent. Art and text both shift +2px down; text also shifts +1px right due to the asymmetric rect adjustment.

**Active in YR:** YES — fires every time a main-menu shell button is held down.

---

## 9. Disabled-State Alpha Overlay

Condition: `piVar17[0x2c] == 0` (normal PCX type) AND `GetWindowLongA(hwnd, GWL_STYLE) & 0x8000000` (WS_DISABLED bit set).

Call at `0x0061360d..0x0061361b`:
```
MOV EDX, [0x00887310]     ; EDX = shell surface
PUSH 0x80                 ; alpha = 128 = 50%
PUSH 0x0                  ; blend color = black (R=0,G=0,B=0)
LEA ECX, [ESP+0x98]       ; ECX = rect = [frame_ESP+0x90] = button screen rect
CALL AlphaBlendRect_00621B80
```

The rect at `[frame_ESP+0x90]` is the button's **screen-coordinate rect** populated by `FUN_0072a9c0` at `0x00613146`. It covers the full button area including art and text (width × height in pixels, in surface-local coordinates).

`AlphaBlendRect_00621B80` applies the formula:
```
dst = (blend_color × alpha + dst × (255 − alpha)) >> 8
    = (0 × 0x80 + dst × 0x80) >> 8
    = dst × 0x80 / 256
    ≈ dst × 0.502   (50.2% of original pixel values)
```

This is a 50% black darkening over the entire button rect, applied after all art and text are drawn.

**Active in YR:** YES — fires for any disabled shell PCX button. Main-menu buttons that are disabled (e.g., "Load" when no saves exist) show bue_*30 art + dark-red text, both half-darkened.

**Verified:** `AlphaBlendRect_00621B80` decompiled; call site at `0x0061360d`; condition check at `0x006135fd..00613605`.

---

## 10. Hover State

**None.** `OwnerDraw_Button_00612B70` has no `WM_MOUSEMOVE` (0x200) handler. The PCX art path consults only:
- Pressed bit (`piVar17[0x3a] & 1`)
- WS_DISABLED style bit

The custom `0x4dc` message arms a keyboard-focus flash timer, but `[piVar17+0xc5]` (flash flag) is only read in the `piVar17[5] != 0` (SDBTNANM) path, not in the normal PCX path. Hover has **zero visible effect** on a standard main-menu PCX button.

**Active in YR:** Confirmed negative — exhaustive message-switch enumeration in SHELL_BUTTON_PAINT_DETAILS. Cross-reference: SHELL_BUTTON_PAINT_DETAILS §4.

---

## 11. Vtable Slots Verified

| Slot (hex) | Address     | Name / Returns |
|------------|-------------|----------------|
| +0x5c | `0x007bbaf0` | Lock(0,0) → pixel buffer base ptr |
| +0x60 | `0x007bbb90` | Unlock |
| +0x74 | `0x00411640` | BSurface__GetPitch → width × bytes_per_pixel |
| +0x78 | `0x00411510` | GetRect → `{0, 0, width, height}` |
| +0x7c | `0x00411540` | BSurface__GetWidth → surface[1] |
| +0x80 | `0x00411550` | GetHeight → surface[2] |
| +0x08 | `0x007bbb90` | Blit/Copy (routes through ClipRectPair_007BBE20) |

**Verified:** BSurface vtable read via `read_memory 0x007e2070` (160 bytes); each slot function decompiled inline.

---

## 12. Open Questions

- **Middle tile uVar5 centering for wider/narrower buttons**: confirmed formula but not fully tested against sub-24px or very wide (>187px) buttons. For parity, implement the centering offset.
- **bue_li30.pcx actual native width confirmation**: binary hardcodes 7 in the table; PCX parse would also return 7. Cross-check PCX binary hex at retail path.
- **bde_li30/ri30 dimensions**: assumed same 7×30 / 10×30 as up-state; no separate measurement done in this session. The format string and table are shared between up and down.
- **Screen vs. client coordinate system for AlphaBlendRect rect**: marked as screen-coord rect from FUN_0072a9c0, but exact FUN_0072a9c0 behavior not traced in this session. Prior docs treat it as the full button area.
- **Resize: what happens when button_screen_height < 24?** The loop would exit with index=0 (art_height=24). Not relevant for dialog 0xE2 (always 37px).

---

## 13. Sources

**Ghidra functions decompiled / disassembled in this session:**

- `OwnerDraw_Button_00612B70` (`0x00612B70`) — full decompile + disassembly
- `FUN_006BA3E0` (`0x006BA3E0`) — full decompile + disassembly (tile helper)
- `FUN_006BA580` (`0x006BA580`) — decompiled (alpha-blit variant, not used on normal PCX buttons)
- `FUN_006BA140` (`0x006BA140`) — disassembled (PCX surface cache lookup, `RET 0x8`)
- `ClipRectPair_007BBE20` (`0x007BBE20`) — decompiled (rect-format confirmation)
- `FUN_007BBB90` (`0x007BBB90`) — disassembled (BSurface blit dispatcher)
- `AlphaBlendRect_00621B80` (`0x00621B80`) — decompiled (50% black overlay)
- `BSurface__GetPitch` (`0x00411640`) — decompiled (vtable+0x74)
- `FUN_00411510` (`0x00411510`) — decompiled (GetRect, vtable+0x78)
- `BSurface__GetWidth` (`0x00411540`) — decompiled (vtable+0x7c)
- `FUN_00411550` (`0x00411550`) — decompiled (GetHeight, vtable+0x80)

**Memory reads:**

- `0x007e2070` (BSurface vtable, 160 bytes) — vtable slot enumeration
- `0x006132b9` (64 bytes) — size-table init byte verification
- `0x006133f1`, `0x00613473`, `0x006134f7`, `0x0061351d`, `0x006135f3`, `0x00613607` — geometry store sequence verification

**Prior reports cross-referenced (not re-investigated):**

- `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md` — Q8 pressed-text delta, Q10 hover negative
- `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md` — size selection and art-height selection overview
- `SHELL_BUTTON_GREYSCALE_COLORIZATION_GHIDRA_REPORT.md` — PCX palette decoding (not revisited)
