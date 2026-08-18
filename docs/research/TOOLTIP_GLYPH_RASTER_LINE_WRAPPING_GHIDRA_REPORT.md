# Tooltip Glyph Raster / Line Wrapping - Ghidra Research Report

**Address(es):** `0x00478BA0`, `0x00478E30`, `0x00433CF0`, `0x00434CD0`, `0x00434120`, `0x00433C70`, `0x00433C90`, `0x00433CA0`, `0x006AC210`, `0x006A92E0`, `0x00640450`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** In-game sidebar-overlapping tooltip popup glyph measurement, wrapping, text inset, clipping, and glyph raster behavior on the normal visible `CCToolTip` path.  
**Non-Scope:** Tooltip z-order/fill/border proof except as call-path context; shell status-line tooltip child `0x695`; world/unit hover text generation; runtime screenshot capture; DirectDraw mask live sampling.  
**Confidence:** High for measured/drawn tooltip layout and glyph raster mechanics from binary; Medium for current Rust deltas where only focused `rg`/file scans were performed.  
**Active in YR:** Yes for standard in-game sidebar/power/cameo tooltips registered by `SidebarClass__InitSurface` and drawn from `RenderFrame_main`.

## Summary

The normal in-game tooltip popup uses the global `GAME.FNT` `BitFont` path for both box sizing and text drawing. Box size is derived from wrap-aware `BitFont__MeasureText`, then native popup padding is applied; the visible draw uses the full multiline/wrapping `FUN_00434CD0` renderer with clipping enabled, one direct packed-color glyph pass, and no shadow/outline glyph pass.

Line wrapping is not an egui-style layout pass. It is the same fixed-width-threshold state machine used by `BitFont__MeasureText` / `FUN_00434CD0`: spaces record the latest word-break point, CRLF is one newline, tabs advance to 64 px stops, and over-width text wraps at the last recorded space or before the overflowing character.

## Target and Non-Scope

Target question: how normal visible in-game tooltip text is measured, wrapped, inset, clipped, and rasterized when the popup overlaps the sidebar.

Non-goals:

- Do not redo the already verified z-order result that the normal frame draws tooltips after the sidebar blit.
- Do not redo black fill / 1 px sidebar-color border proof except where it supplies text rect coordinates.
- Do not investigate shell dialog status-line tooltip child `0x695`.
- Do not investigate semantic generation for every world/unit hover string.
- Do not edit Rust, INI, or tracked docs.

Evidence needed to mark COMPLETE:

- Tooltip popup path must be shown to call `BitFont__MeasureText` for sizing and `FUN_00434CD0` for visible text draw.
- Exact text inset, width/height values passed to the draw helper, and padding constants must be recovered from binary.
- Wrap/newline/tab behavior must be verified from `BitFont__MeasureText` and the draw helper.
- Glyph raster must be verified for direct pixels vs shadow/outline and for clipping behavior.

Stop conditions:

- If a shell tooltip or alternate retained-sidebar branch is encountered, record it as non-scope unless it contradicts the normal visible path.
- If runtime DD mask identity is required for final colors, defer it to the DirectDraw pixel-format slot.

## Verified Binary Findings

1. **Tooltip sizing calls `BitFont__MeasureText` before placement.**  
   Evidence: `0x00478BA0` assembly `0x00478C24..0x00478C36` pushes the active wide text at descriptor `+0x10`, width/height output locals, and the selected region width, then calls `BitFont__MeasureText @ 0x00433CF0`. Active in YR: Yes, this is `CCToolTip` vtable slot `+0x04`, reached from `FUN_00724AD0 -> active tooltip show/placement`.

2. **Initial popup box padding is measured width `+4` and height `+3`.**  
   Evidence: after the first measure call, `0x00478C43..0x00478C66` adds `4` to measured width and `3` to measured height, then takes max against the active popup record's current `+0x08/+0x0C` width/height. Active in YR: Yes.

3. **If measured width plus padding is at least the selected region width, native remeasures with `region_width - 4`.**  
   Evidence: `0x00478C69..0x00478C83` compares popup width against selected region `+0x08`; when not `<`, subtracts `4` from the region width and calls `BitFont__MeasureText` again. `0x00478C90..0x00478CB3` then reapplies `+4/+3` and maxes width/height again. Active in YR: Yes. This means long tooltip text wraps during sizing instead of simply overflowing.

4. **Visible tooltip text origin is outer `x + 2`, `y + 4`.**  
   Evidence: visible draw path `0x00478E30` computes fill/border rect, then before `FUN_00434CD0` performs `ADD EAX,0x2` at `0x00479024` for the x argument and `ADD ECX,0x4` at `0x00479029` for the y argument. Active in YR: Yes through `RenderFrame_main -> CCToolTip+0x0C(0) -> FUN_00724B80(1) -> CCToolTip+0x10`.

5. **Visible tooltip text draw width is inner width minus `4`; draw height is inner height minus `8`.**  
   Evidence: `0x00479020..0x00479032` computes draw arguments immediately before `CALL 0x00434CD0`: x = left + 2, y = top + 4, max_width = right-ish value after subtracting the x inset, and max_height = rect bottom/height value minus the y inset and top-side locals. The paired border/fill setup earlier uses `ADD ECX,0x4` and `ADD EAX,0x8` at `0x00478F2D..0x00478F37`, matching horizontal 2+2 and vertical 4+4 text padding. Active in YR: Yes.

6. **Tooltip text uses alignment flags `0`, not center/right alignment.**  
   Evidence: the `FUN_00434CD0` call site pushes two fade zeros, then the computed height/width/y/x/surface/font arguments; no nonzero align byte is pushed. The decompiler for `FUN_00434CD0` shows `param_8` controls horizontal center (`&1`) and right (`&2`), but this call path passes `0`. Active in YR: Yes.

7. **The measured/drawn wrapping state machine treats tabs as 64 px stops.**  
   Evidence: `BitFont__MeasureText @ 0x00433CF0` case `9` adds `this+0x28` and subtracts modulo `this+0x28`; `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` verifies `this+0x28 = 0x40`. `FUN_00434CD0` mirrors this case. Active in YR: Yes for any tooltip string containing tab.

8. **CRLF counts as one newline; bare CR or LF each advance one line.**  
   Evidence: `BitFont__MeasureText @ 0x00433CF0` cases `0x0A` and `0x0D` advance line height only when previous character was not `0x0D`; `FUN_00434CD0` has the same previous-character guard. Active in YR: Yes.

9. **Spaces are both drawn/measured and remembered as word-wrap candidates.**  
   Evidence: `0x00433CF0` case `0x20` stores `last_space_pos = next_char_ptr` and `last_space_x`, then falls through to glyph handling; `FUN_00434CD0` stores the last-space pointer and current line width before continuing through glyph handling. Active in YR: Yes. This matters for cameo tooltip strings where spaces are rewritten to newlines before rendering.

10. **Overflow wrap backs up to the last space when available; otherwise it hard-breaks before the overflowing character if the line already has more than one character.**  
    Evidence: `BitFont__MeasureText @ 0x00433CF0` overflow branch tests `last_space_pos`; with no space and `chars_on_line > 1`, it subtracts the current glyph advance and retries the current character on the next line; with a space and `chars_on_line > 1`, it uses `last_space_x` and restarts at `last_space_pos`. `FUN_00434CD0` mirrors this in the visible draw branch. Active in YR: Yes.

11. **Line height is the `GAME.FNT` cell height, 17 px for stock `GAME.FNT`.**  
    Evidence: `BitFont__MeasureText` initializes and increments height from `this+0x1C`; `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` verified stock `GAME.FNT` inner field `+0x0C` / outer `+0x1C` is 17, bitmap rows are 16, giving a 1 px line gap. Active in YR: Yes for `GAME.FNT`.

12. **Glyph raster is direct 1bpp-to-16bpp overwrite, not an alpha blend and not a shadowed draw.**  
    Evidence: `FUN_00434120 @ 0x00434120` resolves glyph bits, then for each set bit writes `*dst = packed_color`; zero bits leave destination unchanged. The visible tooltip function has exactly one call to `FUN_00434CD0` at `0x0047903C` after setting the packed sidebar text color at `0x00479004..0x00479006`; no second offset draw exists in the normal tooltip path. Active in YR: Yes.

13. **Glyph clipping is per-pixel against a BitFont clip rectangle set from the tooltip text rect.**  
    Evidence: visible path calls `FUN_00433C90(font, 1)` at `0x00478FF0..0x00478FF2`, `FUN_00433CA0(font, &rect)` at `0x00478FFB..0x00478FFE`, and `FUN_00433C70(font, packed_color)` at `0x00479003..0x00479006` before `FUN_00434CD0`. `FUN_00434120` checks `this+0x41`, intersects glyph rect with `this+0x30..0x3C`, and writes only in-bounds pixels. Active in YR: Yes.

14. **Missing glyphs use the constructed fallback glyph and XOR the packed color by `0x5555`.**  
    Evidence: `FUN_00434120` falls back to `this+0x08`; when fallback is used, it XORs `param_2` with `0x5555` before raster. `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` verifies the fallback is built from codepoint `0xB0`. Active in YR: Yes for missing codepoints.

15. **Cameo tooltip text deliberately replaces spaces with line feeds before the font sees it.**  
    Evidence: `SidebarClass__GetCameoTooltip @ 0x006A92E0` formats into `DAT_00B07BC4`, measures string length with `FUN_007CA405`, then loops through each UTF-16 char and changes `0x20` to `0x0A`. Active in YR: Yes for valid cameo tooltip IDs when game is active.

16. **Power tooltip text is formatted into a single buffer and then rendered by the same popup BitFont path.**  
    Evidence: `PowerClass__GetTooltipText @ 0x00640450` handles ID `999` by formatting `StringTable` id `0x29E` into `g_PowerTooltipBuf`; `SidebarClass__GetTooltipText @ 0x006AC210` asks `PowerClass__GetTooltipText` first. Active in YR: Yes for the registered power tooltip.

## Active in Standard YR?

Yes for the normal in-game sidebar/power/cameo tooltip popup path:

- `SidebarClass__InitSurface` registers in-game sidebar/power tooltip descriptors; previous report `TOOLTIP_MANAGER_SIDEBAR_OVERLAP_PIXELS_GHIDRA_REPORT.md` verified this active registration path.
- `RenderFrame_main @ 0x004F44F0` calls the tooltip singleton vtable slot `+0x0C(0)` after sidebar blit.
- The normal slot stores `CCToolTip+0x260 = 0`, calls `FUN_00724B80(1)`, then the active popup draw slot reaches `0x00478E30`.
- This report verifies that `0x00478E30` configures the global font and calls `FUN_00434CD0` for the visible text.

Conditional / not claimed:

- The alternate `CCToolTip+0x260 == 1` retained-sidebar branch is real, but this report does not prove it as the standard visible frame path.
- Runtime RGB565/RGB555 mask identity is delegated to `DIRECTDRAW_RUNTIME_PIXEL_FORMAT_MASKS`.
- Shell dialog tooltip/status-line rendering is not this path.

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Tooltip box sizing uses `BitFont__MeasureText`, padding `+4/+3`, and remeasure with `region_width - 4` when too wide. | `0x00478C24..0x00478CB3`, `0x00433CF0` | Missing: no native in-game tooltip popup renderer found; existing `BitFont::wrap_layout` exists but tooltip module absent. | Future tooltip module; `src/render/bit_font.rs`; `src/app_render/draw_passes.rs` | Size popup from native wrap-aware measurement, not UI toolkit tooltip sizing. | Long sidebar tooltip near the split edge wraps before overflow and produces native box size. | `test_sidebar_tooltip_remeasures_with_region_width_minus_4_before_padding`; do not use egui/default tooltip layout. |
| Tooltip text draw inset is x `+2`, y `+4`; text region is horizontally padded by 4 and vertically by 8. | `0x00479024..0x0047903C`, paired `0x00478F2D..0x00478F37` | Missing. | Future tooltip renderer; `src/app_render/draw_passes.rs` | Draw text from the native inner origin inside the black/bordered popup. | Soviet tooltip over a known pixel background has first glyph at border-left +2 and border-top +4. | `test_sidebar_tooltip_text_inset_2_4_and_inner_clip_rect`; do not center text inside popup. |
| Visible tooltip text is one direct packed-color BitFont pass with clipping enabled and no shadow. | `0x00478FF0..0x0047903C`, `0x00434120` | Missing; current Rust bit-font is sprite-atlas based and lacks a tooltip clipping path. | `src/render/bit_font.rs`; future 16-bit-compatible tooltip text draw or pixel-testable atlas path | Clip glyph pixels to the tooltip text rect; zero glyph bits leave background unchanged; no offset second draw. | A clipped tooltip glyph at the box edge has native partial pixels and no shadow/outline offset. | `test_sidebar_tooltip_glyphs_clip_per_pixel_without_shadow`; do not render a shadow, outline, alpha-blended glyph, or SDF font. |
| Tooltip wrapping uses native space/backtrack/CRLF/tab behavior from `BitFont__MeasureText` / `FUN_00434CD0`. | `0x00433CF0`, `0x00434CD0`, `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` | Partially present in `src/render/bit_font.rs::wrap_layout`; tooltip not wired; existing atlas range only packs `0x20..0x0180`. | `src/render/bit_font.rs`; `src/assets/fnt_file.rs`; future tooltip layout | Reuse/extend native-compatible wrap layout for tooltip text, including CRLF and tab stops; avoid UTF-only shortcuts that lose 16-bit codepoint behavior. | Strings with CRLF, bare LF, tabs, last-space wrap, and long no-space words match native measured width/height. | `test_sidebar_tooltip_wrap_crlf_tab_space_and_no_space_overflow`; do not split lines on whitespace with a new policy. |
| Cameo tooltip formatter replaces spaces with LF before rendering. | `SidebarClass__GetCameoTooltip @ 0x006A92E0` | Tooltip text generation unchecked/missing. | Future sidebar tooltip text resolver; likely `src/sidebar/` plus localization/string-table surface | Preserve cameo tooltip pre-layout newline injection before passing text into BitFont measurement/draw. | Cameo tooltip displays name/cost/power on separate native line advances. | `test_cameo_tooltip_replaces_spaces_with_lf_before_measure`; do not rely on font word-wrap to create those lines. |

## Negative Facts / Do Not Do

- Do not use egui/default tooltip padding or wrapping for in-game sidebar popups; native uses `BitFont__MeasureText` plus explicit `+4/+3`, remeasure `region_width - 4`, and draw inset `+2/+4` (`0x00478C24..0x0047903C`).
- Do not render tooltip glyph shadows or an outline text pass; the normal visible path calls `FUN_00434CD0` once and `FUN_00434120` writes only set glyph bits in the packed text color (`0x0047903C`, `0x00434120`).
- Do not alpha-blend tooltip glyphs or background text pixels; glyph bits overwrite the 16-bit destination and zero bits preserve the black popup fill (`0x00434120`).
- Do not treat CRLF as two new lines; the font code suppresses `\n` immediately after `\r` (`0x00433CF0`, `0x00434CD0`).
- Do not create a new whitespace wrapping policy; native records only the latest space as a wrap candidate and otherwise hard-breaks before the overflowing character (`0x00433CF0`, `0x00434CD0`).
- Do not skip missing-glyph fallback color behavior for localized builds; missing glyphs use fallback glyph plus color XOR `0x5555` (`0x00434120`, `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`).

## Remaining Uncertainty

- Exact runtime DirectDraw masks (`RGB565` vs `RGB555`) are outside this slot; the mechanism uses the packed color provided by existing sidebar text-color packing.
- `FUN_00434CD0` fade parameters exist, but this tooltip call passes zero fade arguments; non-tooltip users of fade remain outside scope.
- Current Rust tooltip state/text resolver is not deeply scanned because no native in-game tooltip module exists yet; implementation deltas are focused on likely surfaces.
- The alternate `CCToolTip+0x260 == 1` retained-sidebar draw branch was not re-investigated here.
- No live screenshot capture was taken to validate runtime pixels against the binary-derived model.

## Stale-Doc Replacement Wording

No direct stale-doc replacement is required for the latest tooltip overlap report. Suggested extension wording for `C:/Users/enok/Documents/ra2-rust-game/docs/research/TOOLTIP_MANAGER_SIDEBAR_OVERLAP_PIXELS_GHIDRA_REPORT.md` if it is later amended:

> In-game sidebar tooltip text is measured with `BitFont__MeasureText` and drawn with `FUN_00434CD0` using native wrap/clipping. The popup box applies measured width `+4` and height `+3`, remeasures long text with `region_width - 4`, then draws text once at outer `x + 2`, `y + 4` in the current sidebar text color. Glyph pixels are direct 1bpp-to-16bpp writes with per-pixel clipping; there is no tooltip text shadow or alpha glyph pass.

## Status

COMPLETE for the scoped normal in-game tooltip glyph raster, line wrapping, padding, and clipping path. Remaining uncertainty is runtime-pixel capture and adjacent systems, not the verified binary layout/draw mechanism.

## Sources

- `RenderFrame_main @ 0x004F44F0` from prior tooltip report / spot-check context.
- `CCToolTip` placement assembly around `0x00478BA0..0x00478DAB`.
- `CCToolTip` visible draw assembly around `0x00478E30..0x00479048`.
- `BitFont__MeasureText @ 0x00433CF0`.
- `FUN_00434CD0` full BitFont draw/wrap helper.
- `FUN_00434120` glyph raster helper.
- `FUN_00433C70`, `FUN_00433C90`, `FUN_00433CA0` font color/clip setters.
- `SidebarClass__GetTooltipText @ 0x006AC210`.
- `SidebarClass__GetCameoTooltip @ 0x006A92E0`.
- `PowerClass__GetTooltipText @ 0x00640450`.
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/TOOLTIP_MANAGER_SIDEBAR_OVERLAP_PIXELS_GHIDRA_REPORT.md`.
- `C:/Users/enok/Documents/ra2-rust-game/docs/research/BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`.
- Focused Rust scan: `src/render/bit_font.rs`, `src/assets/fnt_file.rs`, `src/app_render/draw_passes.rs`, `src/render/sidebar_text.rs`, `src/app_sidebar_text.rs`.
