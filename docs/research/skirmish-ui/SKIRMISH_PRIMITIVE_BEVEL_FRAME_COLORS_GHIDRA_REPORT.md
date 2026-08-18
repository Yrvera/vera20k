# Skirmish Primitive Bevel Frame Colors - Ghidra Research Report

**Address(es):** `FUN_006208F0 @ 0x006208F0` primary; Skirmish callers `OwnerDraw_ComboBox_00617250 @ 0x00617893`, `OwnerDraw_ListBox_00618D40 @ 0x0061926B`, `OwnerDraw_Trackbar_0061D950 @ 0x0061E204/0x0061E269`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Static drain of the primitive beveled rectangle/frame helper used by offline Skirmish collapsed combos, combo dropdown/list frames, and trackbar rail/value-frame primitives. This covers arguments, line geometry, color conversion, 2-pixel bevel color ordering, disabled/grey caller color selection, and Skirmish caller values.  
**Non-Scope:** Runtime screenshot validation, surface vtable raster internals beyond the `+0x30` line and `+0x24` point calls made by this helper, generic non-Skirmish edit/scrollbar/button-variant frames except where they identify shared behavior.  
**Confidence:** High for static formulas and Skirmish call values; Medium for final monitor pixels only because this slot did not capture a retail screenshot.  
**Active in YR:** Yes. Standard offline dialog `0x102` installs `OwnerDraw_ComboBox_00617250`, `OwnerDraw_ListBox_00618D40`, and `OwnerDraw_Trackbar_0061D950` through `FUN_0060F9A0`; all three call `FUN_006208F0` in their active paint paths.

## 1. Overview

`FUN_006208F0` draws a rectangular primitive frame to a surface, using caller coordinates in `[x, y, width, height]` form, not Windows `[left, top, right, bottom]` form. It expands that box outward by the requested border width, then draws one ring per border pixel using the surface vtable line primitive at `+0x30`.

Active in YR: Yes. Evidence: the helper is reached from Skirmish combo paint at `0x00617893`, dropdown/list paint at `0x0061926B`, and trackbar paint at `0x0061E204/0x0061E269`.

## 2. Arguments And Color Globals

Effective signature from call sites and callee prologue:

| Argument | Source | Meaning | Active in YR |
|---|---|---|---|
| `ECX` | caller loads `DAT_00887310` | destination surface pointer | Yes; all scoped Skirmish paint call sites |
| `EDX` | pointer to 4-int box | `[x, y, width, height]` | Yes; caller-local boxes |
| stack arg 1 | pushed `2` in scoped callers | border width / number of rings | Yes; all scoped Skirmish calls use `2` |
| stack arg 2 | caller color or `-1` | fallback single-color value only when border width is not `2` | Conditional; ignored for Skirmish `border=2` frame rings |

Color conversion formula for a 24-bit color value `0x00BBGGRR`:

```text
dd_color =
  ((B >> g_DD_BLoss) << g_DD_BShift) |
  ((G >> g_DD_GLoss) << g_DD_GShift) |
  ((R >> g_DD_RLoss) << g_DD_RShift)
```

The conversion globals are read at `0x008A0DDC`/`0x008A0DD8` for blue loss/shift, `0x008A0DE4`/`0x008A0DE0` for green loss/shift, and `0x008A0DD4`/`0x008A0DD0` for red loss/shift. Active in YR: Yes; these are the same DirectDraw-format globals used throughout shell drawing.

`FUN_0060F9A0` initializes the relevant owner-draw colors before subclassing controls:

| Global | Init value | Use in this helper | Active in YR |
|---|---:|---|---|
| `DAT_00AC1B98` | `0x00C5BEA7` | bevel color A | Yes; written at `0x0060FA91`, read at `0x0062095E` |
| `DAT_00AC1B94` | `0x00807A68` | bevel color B | Yes; written at `0x0060FA9B`, read at `0x00620963` |
| `DAT_00AC4624` | `0x000000FF` | fallback when caller passes `-1` and border width is not `2` | Conditional; read at `0x0062090A`, but Skirmish `border=2` replaces it before drawing lines |
| `DAT_00AC1CA8` | `0x0000009F` | disabled/grey caller-side candidate color | Conditional; Skirmish callers compute it on disabled paths, but `border=2` frame rings still use bevel globals |
| `DAT_00AC1DD8` | `0x00929292` | scrollbar grey candidate, not used by scoped Skirmish combo/list/trackbar frames except scrollbar side paths | Conditional; not required for offline combo/list/trackbar frames in this slice |

## 3. Core Raster Formula

Given input box `[x, y, w, h]` and border width `n`:

```text
left0   = x - n
top0    = y - n
right0  = x + w + n - 1
bottom0 = y + h + n - 1

for i in 0..n-1:
    left   = left0 + i
    top    = top0 + i
    right  = right0 - i
    bottom = bottom0 - i

    draw top edge:    (left, top)        -> (right - 1, top)
    draw left edge:   (left, top + 1)    -> (left, bottom)
    draw bottom edge: (right, bottom)    -> (left, bottom)
    draw right edge:  (right, bottom - 1)-> (right, top + 1)
```

Active in YR: Yes. Evidence: coordinate setup and four `surface->vtable+0x30` calls at `0x00620A1A..0x00620B7A`; loop update at `0x00620C82..0x00620CB5`.

The asymmetric `right - 1`, `top + 1`, and `bottom - 1` endpoints avoid double-writing most corners through line overlap. Active in YR: Yes. Evidence: `0x00620AA5`, `0x00620ADD`, `0x00620B5B`, and the four line-call blocks.

## 4. Two-Pixel Bevel Ordering

All scoped Skirmish callers pass border width `2`. That activates special color ordering:

| Ring | Top/left color | Bottom/right color | Corner pixels |
|---:|---|---|---|
| outer ring `i=0` | converted `DAT_00AC1B98` (`0xC5BEA7`) | converted `DAT_00AC1B94` (`0x807A68`) | mixed corners overwritten with average |
| inner ring `i=1` | converted `DAT_00AC1B94` (`0x807A68`) | converted `DAT_00AC1B98` (`0xC5BEA7`) | mixed corners overwritten with average |

The average color is computed from the unconverted 24-bit globals channel-by-channel:

```text
avg = ((A & 0xFF0000 + B & 0xFF0000) / 2 & 0xFF0000)
    + ((A & 0x00FF00 + B & 0x00FF00) / 2 & 0x00FF00)
    + ((A & 0x0000FF + B & 0x0000FF) / 2)
```

Then `avg` is converted through the same DirectDraw loss/shift globals and written through surface vtable `+0x24` to the mixed bevel corners, i.e. the top-right and bottom-left corners for each ring. Active in YR: Yes. Evidence: color swap around `0x00620A90..0x00620B16`, average computation `0x00620B8B..0x00620C3E`, point writes `0x00620C40..0x00620C7F`.

Important caller implication: when `n == 2`, the caller-supplied fourth argument does not determine the visible frame line colors. It is converted first, but each ring then substitutes the bevel globals before the line calls. Active in YR: Yes for scoped Skirmish callers, because each passes `2` at `0x0061788D`, `0x00619262`, `0x0061E1F4`, and `0x0061E259`.

## 5. Skirmish Caller Values

| Caller | Call site | Box / width | Caller color argument | Active in YR |
|---|---|---|---|---|
| Collapsed combo face | `OwnerDraw_ComboBox_00617250 @ 0x00617893` | combo-local face box, fixed 24 px height from prior geometry report, border `2` | normal path computes `DAT_00AC4624`; disabled path computes `DAT_00AC1CA8` | Yes; offline `0x102` combos |
| Real listbox frame | `OwnerDraw_ListBox_00618D40 @ 0x0061926B` | list client/backing box prepared before row paint, border `2` | passes `-1`; helper would fallback to `DAT_00AC4624`, but `border=2` substitutes bevel globals | Conditional; active for real owner-drawn `LISTBOX` controls, not standard `ComboDropWin` popup rows |
| Trackbar left rail/base frame | `OwnerDraw_Trackbar_0061D950 @ 0x0061E204` | first trackbar primitive box before the value plaque, border `2` | normal computes `DAT_00AC4624`; disabled computes `DAT_00AC1CA8` | Yes; speed/credits/unit trackbars |
| Trackbar second primitive frame | `OwnerDraw_Trackbar_0061D950 @ 0x0061E269` | second trackbar primitive box at the value/plaque side, border `2` | same computed color as first call | Yes; speed/credits/unit trackbars |

No INI key participates in this primitive frame color path. Active in YR: Yes as binary-only UI drawing state; evidence is the absence of INI reads in `FUN_006208F0` and owner-draw color initialization in `FUN_0060F9A0`.

## 6. Current Rust Implementation Status

Current Rust Skirmish shell code renders asset-backed shell chrome, owner-draw buttons, flags, preview elements, and hit-test state. It does not implement primitive combo/list/dropdown frames or trackbar rails. Evidence: focused source scan found no `FUN_006208F0` analogue, no `trakgrip`/`trofm` rail assets in active Skirmish draw code, and no primitive bevel draw surface in `src/app_skirmish_shell_render.rs` or `src/ui/skirmish_shell`.

Relevant surfaces found:

| Rust surface | Current state |
|---|---|
| `src/app_skirmish_shell_render.rs` | draws shell assets and flags; no combo/list/trackbar primitive bevel renderer |
| `src/ui/skirmish_shell/layout.rs` | has color-combo and flag rects, but no full combo/dropdown/trackbar child model |
| `src/ui/skirmish_shell/state.rs` | hit-tests color combo rectangles only; no open dropdown or slider state |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_006208F0` argument model | verified | prologue/call sites `0x006208F0`, `0x00617893`, `0x0061926B`, `0x0061E204` | none |
| DirectDraw color conversion globals | verified | `0x00620917..0x00620A16`, global writes in `FUN_0060F9A0` | none |
| `border == 2` special bevel color swap | verified | `0x00620A90..0x00620B16`, loop update `0x00620C82..0x00620CB5` | none |
| Average corner color | verified | `0x00620B8B..0x00620C7F` | exact point target names inferred from ring geometry; surface pixel primitive internals out-of-scope |
| Combo collapsed frame caller | verified | `0x00617893`, prior combo geometry report | none |
| Dropdown/list frame caller | verified | `0x0061926B`, prior dropdown geometry report | row-internal paint belongs to slot 2 |
| Trackbar primitive frame callers | verified | `0x0061E204`, `0x0061E269`, prior checkbox/trackbar report | runtime screenshot validation optional |
| Surface vtable line/point internals | deferred | calls to `+0x30` and `+0x24` identified | separate surface primitive investigation if exact Bresenham internals are needed |

## 8. Open Questions - Final State

[RESOLVED] OQ1 - Does `FUN_006208F0` consume `[x,y,w,h]` or `[l,t,r,b]`? It consumes `[x,y,width,height]`, expands by border width, and derives right/bottom. Evidence: `0x00620A1A..0x00620A43`.  
[RESOLVED] OQ2 - Which color globals produce the Skirmish combo/list/trackbar frame? For `border=2`, `DAT_00AC1B98` and `DAT_00AC1B94` drive all frame lines; caller color is ignored for those rings. Evidence: `0x00620A90..0x00620B16`.  
[RESOLVED] OQ3 - Is there a raised/sunken variant flag? No flag in the helper; the 2-pixel border hardcodes an outer raised-like ring and an inner reversed ring by swapping the two globals on the second iteration. Evidence: loop pointer update `0x00620C82..0x00620CB5`.  
[RESOLVED] OQ4 - Does disabled state alter the primitive bevel colors? Not directly for Skirmish `border=2` calls. Callers compute disabled candidate colors, but the helper substitutes bevel globals. Evidence: caller disabled conversions plus `border=2` substitution.  
[DEFERRED] OQ5 - Exact implementation of surface vtable `+0x30` line clipping/raster endpoints below the helper. Category: out-of-scope. Reason: this slice verifies helper-issued endpoints and colors; surface primitive internals are a lower-level renderer substrate.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Skirmish primitive frames use `FUN_006208F0` 2-pixel bevel rings, not PCX art | `0x00617893`, `0x0061926B`, `0x0061E204/269` | missing | `src/app_skirmish_shell_render.rs` plus a small UI primitive renderer | draw combo collapsed frames, dropdown/list frames, and trackbar rail frames from primitive lines | 800x600 Skirmish screen shows combo/trackbar bevels without asset substitution | Do not fake these with scaled textures or egui widgets |
| For border `2`, visible colors are fixed globals `0xC5BEA7` and `0x807A68`, swapped between outer/inner rings | `0x00620A90..0x00620C7F`; init `0x0060FA91/9B` | missing | renderer color conversion / palette surface bridge | reproduce two-ring color ordering and averaged top-right/bottom-left mixed corners | screenshot crop of color combo face and trackbar rail matches 2px beveled frame colors | Do not use caller disabled/base color for the ring pixels; gamemd ignores it for width 2 |
| Box input is `[x,y,w,h]` and helper expands outward by border width | `0x00620A1A..0x00620A43` | unchecked/missing | future combo/dropdown/trackbar layout paint | pass content face boxes, then expand by 2 px in primitive renderer | frame surrounds the 24 px combo face and trackbar primitive boxes with no 1 px drift | Do not treat the last two fields as right/bottom |
| Disabled callers may apply other disabled effects, but this primitive helper still uses fixed bevel globals for `border=2` | `0x0061783D..0x00617886`, `0x0061E145..0x0061E1B9`, `0x00620A90..0x00620B16` | missing | disabled-state paint path | keep bevel frame color stable; apply disabled alpha/text/thumb behavior separately where caller does | disabled combo/trackbar frame retains bevel order while content/text/thumb disabled treatment differs | Do not globally grey the primitive bevel unless a higher caller alpha-blends the region |

## Stale Docs / Follow-up Docs

- `SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`: replace the deferred rail-color sentence with: "Exact static `FUN_006208F0` rail pixels are now resolved by `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md`: Skirmish `border=2` calls use fixed bevel globals `DAT_00AC1B98=0xC5BEA7` and `DAT_00AC1B94=0x807A68`, swapped between outer and inner rings, with averaged mixed corners."
- `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`: the statement "line colors are broader owner-draw chrome context" can now cite this report for the exact `border=2` color ordering.

## Sources

- Ghidra read-only decompile/disassembly: `FUN_006208F0 @ 0x006208F0`
- Ghidra read-only decompile/disassembly: `OwnerDraw_ComboBox_00617250 @ 0x00617250`
- Ghidra read-only decompile/disassembly: `OwnerDraw_ListBox_00618D40 @ 0x00618D40`
- Ghidra read-only decompile/disassembly: `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`
- Ghidra read-only decompile: `FUN_0060F9A0 @ 0x0060F9A0`
- Ghidra xrefs to `FUN_006208F0`: `0x00617893`, `0x0061926B`, `0x0061E204`, `0x0061E269`, plus non-scoped owner-draw callers
- Prior docs: `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_DROPDOWN_WINDOW_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`, `FUN_0060F9A0_OWNERDRAW_SUBCLASS_SETUP_GHIDRA_REPORT.md`
- Rust scan: `rg` over `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`, and `src/ui/skirmish_shell`
