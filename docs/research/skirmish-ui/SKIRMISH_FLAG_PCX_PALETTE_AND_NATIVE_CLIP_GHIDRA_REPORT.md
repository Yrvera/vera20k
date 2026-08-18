# Skirmish Flag PCX Palette and Native Clip - Ghidra Follow-up

**Target:** `SKIRMISH_FLAG_PCX_PALETTE_AND_NATIVE_CLIP`  
**Investigation Mode:** exhaustive-slice follow-up  
**Status:** COMPLETE for the offline Skirmish flag static path; no Rust or in-repo files modified.  
**Primary prior report:** `SKIRMISH_FLAG_STATIC_RENDERING_GHIDRA_REPORT.md`  
**Scope:** offline standard YR Skirmish dialog `0x102` flag statics `0x6DA..0x6E1`; PCX selection/cache path; palette/key behavior visible at the owner-draw layer; native-size centering and clipping semantics.  
**Non-scope:** online lobby list rows, non-Skirmish shell statics, and a full generic PCX decoder audit beyond the data needed by this owner-draw path.

## Summary

The prior report's core flag-static behavior is confirmed. Standard offline YR Skirmish uses the side/country combo item data to select a cached PCX surface, stores it into the paired static as owner-draw kind `2`, and paints it as a native-size keyed 16-bit image. The path does not scale the 47x23 flag PCX into the control rectangle. At the 800x600 Skirmish placement previously derived for `0x6DA..0x6E1` (`48x20` pixels), a 47x23 flag is X-centered with integer truncation and vertically clipped rather than squashed.

The palette conclusion is now bounded more tightly: the scoped selector/static/preload path does not read `DIALOG.PAL`, `SHELL.PAL`, `SHELL2.PAL`, `MAINBTTN.PAL`, or an INI palette key. The flag PCXs are loaded by the shared owner-draw PCX cache, and the paint callback receives already-converted surface pixels. Transparency is not palette-index based in the static callback; it is a paint-time comparison against the display-format conversion of RGB magenta (`0xFF00FF`).

## Verified Findings

### 1. Dialog 0x102 flag statics are live standard YR controls

Active in YR: Yes. Evidence: `FUN_006AE6E0` initializes the Skirmish dialog state, `FUN_006ACEE0` routes side-combo `WM_COMMAND` changes, and the row helpers map side combos `0x6A1/0x510/0x513/0x51E/0x514/0x51F/0x520/0x521` to statics `0x6DA..0x6E1` in `FUN_004E3F70` and `FUN_004E3690`. The resource geometry is the standard `RT_DIALOG 0x102` table in `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md:112..119`.

Material detail: there is no separate inactive/disabled flag renderer for these statics. Disabled or inactive rows still use the same kind-2 static image machinery if a PCX pointer is stored.

### 2. Side/country selection uses combo item data, not INI image names

Active in YR: Yes. Evidence: `FUN_004E3A00` clears and repopulates each side combo, inserts Random with item data `-2`, then inserts multiplayer house types where `HouseType+0x1A5` is true, `HouseType+0xB8` is in `-2..9`, and the UI name pointer at `+0x60` is nonzero. The YR `[Countries]` order cross-check is `ini/rulesmd.ini:959..971`.

Material detail: the selected item's `CB_GETITEMDATA` result is the only value fed to `FUN_004E3560`; no country `Image=`, `Prefix=`, `Suffix=`, side name, or PCX filename is read from INI in this path.

### 3. The PCX filename mapping is hardcoded and fuller than side-family grouping

Active in YR: Yes for `-2` and `0..9`; Conditional for `-3`. Evidence: `FUN_004E3560 @ 0x004E3560` switches directly to cached PCX lookup calls for:

| Item data | PCX | Active in YR |
|---:|---|---|
| `-3` | `obsi.pcx` | Conditional: observer/restricted combo branch in `FUN_004E3B90` |
| `-2` | `rani.pcx` | Yes: Random item inserted by `FUN_004E3A00` |
| `0` | `usai.pcx` | Yes |
| `1` | `japi.pcx` | Yes |
| `2` | `frai.pcx` | Yes |
| `3` | `geri.pcx` | Yes |
| `4` | `gbri.pcx` | Yes |
| `5` | `djbi.pcx` | Yes |
| `6` | `arbi.pcx` | Yes |
| `7` | `lati.pcx` | Yes |
| `8` | `rusi.pcx` | Yes |
| `9` | `yrii.pcx` | Yes |

Material detail: `FUN_0061F210 @ 0x0061F210` also preloads `gdii.pcx` and `nodi.pcx`, but `FUN_004E3560` does not select them for standard Skirmish combo item data. They are shared shell assets, not evidence that GDI/Nod flags appear in standard offline Skirmish.

### 4. The load path is owner-draw PCX cache lookup, with no scoped external palette read

Active in YR: Yes. Evidence: `FUN_0060F9A0 @ 0x0060F9A0` runs the shell owner-draw setup and one-time preload when `DAT_00AC48D4 == 0`; that path calls `FUN_0061F210`. `FUN_0061F210` calls `CDFileClass__Constructor(name, 2, 0)` for all scoped flag PCXs. At selection time, `FUN_004E3560` calls `FUN_006BA140(name, 0)`, and `FUN_006BA140 @ 0x006BA140` returns a cached surface pointer or `0`.

Material detail: within `FUN_0060F9A0`, `FUN_0061F210`, `FUN_004E3560`, `FUN_006BA140`, `FUN_00603D30`, `OwnerDraw_Static_006153E0`, and `FUN_006BA580`, this pass found no read of `DIALOG.PAL`, `SHELL.PAL`, `SHELL2.PAL`, `MAINBTTN.PAL`, and no INI palette key. The matching retail asset evidence remains `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md:129..137`: checked flag PCXs are 8-bit, 1-plane RLE PCX files with `bytes_per_line=48` and embedded VGA palette marker `0x0C`.

Inference, not a new binary proof: the generic cache loader is responsible for converting the PCX's embedded palette/indexed pixels into the surface consumed later. This follow-up proves the scoped Skirmish flag path consumes cached converted PCX surfaces and does not itself attach an external palette.

### 5. Kind-2 static storage is the only handoff to paint

Active in YR: Yes. Evidence: `FUN_00603D30 @ 0x00603D30` finds the owner-draw static state by `HWND`, writes kind `2`, writes the image/surface pointer, and invalidates the control with erase enabled. Programmatic initialization (`FUN_004E3F70`) and user side changes (`FUN_006ACEE0 -> FUN_004E3830 -> FUN_004E3690`) both reach this setter.

Material detail: a cache miss or unmapped item data stores/propagates a null image pointer. `OwnerDraw_Static_006153E0` then restores the saved background and skips the kind-2 image branch, so the observable fallback is blank/background, not a default random or side-family flag.

### 6. Magenta transparency is RGB-keyed at paint time

Active in YR: Yes. Evidence: in `OwnerDraw_Static_006153E0 @ 0x006153E0`, the kind-2 branch computes a transparent key from RGB magenta (`R=0xFF, G=0, B=0xFF`) through the DirectDraw channel loss/shift globals before calling `FUN_006BA580`. `FUN_006BA580 @ 0x006BA580` copies 16-bit source pixels to the destination except pixels equal to the passed key.

Material detail: the static callback does not test an 8-bit PCX palette index such as `255`. By the time this branch runs, transparency is a comparison against the converted display-format magenta value.

### 7. Flags are native-size, centered only when smaller, and clipped when larger

Active in YR: Yes. Evidence: `OwnerDraw_Static_006153E0` reads source width/height through image vtable slots `+0x7C` and `+0x80`. In the kind-2 branch, if source width is smaller than the control rect width it sets `dest_x = left + (rect_w - src_w) / 2` and uses source width as the draw width. If source height is smaller, it similarly centers vertically. If either source dimension is greater than or equal to the control rect dimension, the destination dimension remains the control rect dimension; the blit is clipped, not scaled.

Material detail: the retail flag PCXs checked in `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md:129..137` are `47x23`. The derived 800x600 Skirmish placement for flag statics is `48x20` pixels in `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md:271..278`. Therefore the normal visible case is a native 47-pixel-wide image with integer `(48 - 47) / 2 == 0` X offset and vertical clipping to 20 rows, not a 47x23-to-48x20 stretch.

## Cross-doc Notes

- `SKIRMISH_FLAG_STATIC_RENDERING_GHIDRA_REPORT.md` is corroborated on active controls, item-data PCX mapping, kind-2 storage, magenta keying, null blank behavior, and native clipping.
- Its lower-PCX-decoder caveat should now be narrowed: the exact generic decoder internals are still not named here, but the Skirmish flag path itself is no longer open. It consumes owner-draw cached converted PCX surfaces and performs only display-format magenta keying in the static renderer.
- Existing Rust notes that draw these flags with fit/scaling remain implementation mismatches; this report makes no code edits.

## Coverage Ledger

| Area | Status | Evidence | Open |
|---|---|---|---|
| Dialog `0x102` side flag statics active | verified | `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_004E3F70`, `FUN_004E3690`; layout doc lines 112..119 | none |
| Item-data to PCX mapping | verified | `FUN_004E3560 @ 0x004E3560`; string anchors `0x00836328..0x008363AC` | none |
| PCX preload/cache lookup | verified | `FUN_0060F9A0`, `FUN_0061F210`, `FUN_006BA140` | generic cache internals not expanded |
| External palette non-use in scoped path | verified negative for scoped functions | no `DIALOG.PAL`/`SHELL.PAL`/`MAINBTTN.PAL` read in selector/static/preload/cache/blit functions | none for Skirmish flag path |
| Embedded PCX palette asset evidence | verified by prior asset probe | ownerdraw asset mapping doc lines 129..137 | fresh archive extraction optional only |
| Magenta transparent key | verified | `OwnerDraw_Static_006153E0`, `FUN_006BA580` | none |
| Native centering/clipping | verified | `OwnerDraw_Static_006153E0`; high-res placement report | none |

## Sources

- Ghidra read-only decompile: `FUN_004E3560`, `FUN_0060F9A0`, `FUN_0061F210`, `FUN_006BA140`, `FUN_00603D30`, `OwnerDraw_Static_006153E0`, `FUN_006BA580`, `FUN_004E3A00`, `FUN_004E3B90`, `FUN_004E3F70`, `FUN_004E3690`, `FUN_006AE6E0`, `FUN_006ACEE0`.
- Existing docs: `SKIRMISH_FLAG_STATIC_RENDERING_GHIDRA_REPORT.md`, `SKIRMISH_FLAG_STATICS_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`.
- INI cross-check: `ini/rulesmd.ini:959..971`.
