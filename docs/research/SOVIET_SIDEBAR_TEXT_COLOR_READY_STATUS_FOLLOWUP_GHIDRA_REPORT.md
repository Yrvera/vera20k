# Soviet Sidebar Text Color / Ready Status Follow-Up Ghidra Report

Date: 2026-05-27T11:42+02:00

## Target Question

Verify `SetSidebarTextColor` / `FUN_0072F440` and the `StripClass::Draw`
Ready/status consumer path sufficiently to answer:

- What in-game sidebar text color does Soviet use?
- Do Ready/status and queue-count text consume that same color?
- Is Rust `side_highlight_color` the right semantic value for Ready/status text?

## Non-Goals

- Do not re-investigate full bitmap-font glyph rasterization, clipping, drop
  shadow, or fade behavior.
- Do not re-investigate shell/loading-screen DIALOG palettes.
- Do not inspect radar/left-panel SHP selectors.
- Do not modify Rust, INI, or published research docs outside this report.

## Evidence Needed To Mark COMPLETE

- Ghidra MCP decompile plus assembly context for `SetSidebarTextColor`.
- Ghidra MCP evidence for the caller argument used by active side-MIX init.
- Color literal writes for the three source RGB byte triples.
- Ghidra MCP decompile plus assembly context proving `StripClass::Draw` passes the
  selected color to Ready/status text consumers.

## Stop Conditions

- If Ghidra MCP read-only tools are unavailable, stop as FAILED and do not use raw
  binary disassembly or old docs as a substitute.
- If no consumer path from `DAT_00b0fa1c/fa1e` to Ready/status text is found,
  stop at UNCHECKED and do not make a Rust handoff.
- If a mutating Ghidra operation would be needed, stop and record uncertainty.

## Verified Findings

1. **Active in YR: Yes.** `InitSideMixFiles @ 0x00534FA0` preserves the post-Yuri-substitution side in `ESP+0x10`, calls `PaletteLoad @ 0x0072F350`, then moves `ESP+0x10` into `ECX` before `CALL 0x0072F440`. Evidence: decompile `0x00534FA0`; assembly `0x00534FA0..0x00534FC0` (`side 2 -> 1`, store side) and `0x005352E3..0x005352EC` (`PaletteLoad`; `MOV ECX,[ESP+0x10]`; `CALL 0x0072F440`).

2. **Active in YR: Yes.** `SetSidebarTextColor @ 0x0072F440` branches exactly on side `0`, side `1`, and fallback `2+`: side 0 copies `DAT_00b0f9d8/f9da`, side 1 copies `DAT_00b0fb04/fb06`, and fallback copies `DAT_00b0faa0/faa2` into `DAT_00b0fa1c/fa1e`. Evidence: decompile `0x0072F440`; assembly `0x0072F440..0x0072F495`.

3. **Active in YR: Yes.** The source RGB byte triples are initialized as Allied `(0xA4,0xD2,0xFF)`, Soviet `(0xFF,0xFF,0x00)`, and fallback/Yuri `(0xFF,0xFF,0x00)`. Evidence: Ghidra assembly context `0x0072A940..0x0072A955`, `0x0072A960..0x0072A973`, and `0x0072A980..0x0072A993`.

4. **Active in YR: Yes.** `StripClass::Draw @ 0x006A9540` converts the selected RGB bytes into the current DirectDraw surface pixel format once per slot before text drawing: `DAT_00b0fa1c` low byte is red, `DAT_00b0fa1c` high byte is green, and `DAT_00b0fa1e` is blue. Evidence: decompile `0x006A9540`; assembly `0x006A9BF1..0x006A9C46`.

5. **Active in YR: Yes.** The same packed color is passed to queue-count text, Ready/status text, and Hold/status text `DrawText` calls. Queue count path uses `ComputeTextRect/AlphaBlendRect/DrawText` at `0x006A9D0C..0x006A9D7B` with flags `0x242`; Ready/status path uses `0x006A9DCC..0x006A9E36` with flags `0x142`; Hold/status split uses `0x006A9F19..0x006A9F7D` or `0x006A9FAE..0x006AA014` with flags `0x42` or `0x142`. `DrawText @ 0x004A60E0` forwards its color argument into `FUN_004A5EB0`, which calls `FUN_00433C70(color)` before glyph drawing at `0x004A5FCF..0x004A5FED`.

## Implementation Handoff

- Soviet/Yuri Ready/status text should use RGB `(255,255,0)` and Allied should use `(164,210,255)` from the sidebar text-color global, not a DIALOG palette. Rust values in `src/render/sidebar_text.rs::side_highlight_color` match the verified RGB table, and `src/app_render/build_instances.rs` already uses that value for Ready text. Suggested test: `test_sidebar_ready_text_color_matches_set_sidebar_text_color_by_side`.

- Queue-count text and Ready/status text must share the same selected sidebar text color. Rust currently passes one `ready_tint` into `build_sidebar_text_instances` for both paths in `src/app_sidebar_build.rs`, matching this color-sharing fact. Suggested test: `test_sidebar_queue_count_and_ready_text_share_side_text_color`.

- Treat the Rust function name/comment as semantically loose: for this consumer the verified native concept is "current sidebar text color", not a proven selected-unit fade endpoint. Suggested test/doc cleanup anchor: `test_sidebar_text_color_function_returns_native_sidebar_text_rgb_table`.

## Negative Facts / Do Not Do

- Do not use DIALOG palettes for in-game Ready/status or queue-count text.
- Do not make Soviet Ready/status text red or side-chrome-derived; Soviet uses yellow.
- Do not give Yuri a distinct Ready/status color; active side setup maps side 2 to side 1 before `SetSidebarTextColor`, and the fallback source is also yellow.
- Do not color Ready/status separately from queue-count text; the same packed value feeds both.
- Do not describe Ready text as requiring the selected-unit fade path based on this slot; this slot proves static color use for `StripClass::Draw` text calls only.

## Remaining Uncertainty

- This slot did not prove every non-cameo sidebar text consumer that may read
  `DAT_00b0fa1c/fa1e`, such as credits or radar-side text.
- The exact runtime DirectDraw bit-shift descriptor values are not rechecked here;
  this report verifies the byte inputs and conversion path.
- The broader selected-unit fade/highlight path remains outside this target.

## Stale-Doc Wording

- `src/render/sidebar_text.rs` comments saying "only the Ready cameo text needs
  build_text_with_fade" should be replaced with wording that Ready/status text
  uses the current sidebar text color table and the normal single-color text path.
- If updating docs later, prefer "current sidebar text color" over "selected unit
  highlight color" for Ready/status and queue-count consumers.

## Status

COMPLETE.
