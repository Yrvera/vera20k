# Skirmish Owner-Draw PCX Palette/Key Behavior Trace

Scenario: standard offline Yuri's Revenge Skirmish setup dialog `0x102`, owner-draw PCX assets used by Start/Choose/Back buttons and flag statics.

Status: COMPLETE. Ghidra was used read-only. No Rust, INI, or in-repo docs were modified.

## Verdict Tally

PASS: 8 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

PASS means this trace computed a concrete Rust-side output and matched it to active standard-YR gamemd evidence. Anything without both sides computed is UNCHECKED.

## Pipeline

Rust path: `build_skirmish_shell_chrome_atlas` -> `PcxFile::from_bytes` -> `render_pcx_entry` for button/control PCXs or `render_flag_pcx_entry` for flag PCXs -> atlas pack -> button/flag sprite emission.

gamemd path: standard shell owner-draw hook `FUN_0060F9A0` -> one-time PCX preload `FUN_0061F210` -> mode-2 PCX cache loader at `0x006B9D00`/`BSurface__Constructor 0x00630310` -> `OwnerDraw_Button_00612B70` or `OwnerDraw_Static_006153E0`.

## Stage Results

| Stage | Our output | gamemd output | Verdict |
|---|---:|---:|---|
| Button PCX asset family | `bue_li30/bue_mi30/bue_ri30` released and `bde_li30/bde_mi30/bde_ri30` pressed | same strings are preloaded and formatted for standard 30-family owner-draw buttons | PASS |
| Button PCX palette source | embedded PCX palette is parsed by `PcxFile::from_bytes`; external shell/sidebar PAL used = `0` | mode-2 loader reads PCX palette and converts through DirectDraw format globals; scoped external PAL use = `0` | PASS |
| Button index-0 alpha rule | `render_pcx_entry` calls `pcx.to_rgba(None)`, so transparent index is absent and index-0 alpha is `255` | button cap blits and middle tile helper copy converted 16-bit pixels; index-0 skip rule = `0` | PASS |
| Button magenta key rule | button PCXs use no RGB color key in Rust | `OwnerDraw_Button_00612B70` and `FUN_006BA3E0` do not pass/use the static magenta keyed blitter for default PCX buttons | PASS |
| Flag PCX asset mapping | 12 standard flag PCX names are loaded into `atlas.flags` from `SKIRMISH_FLAG_PCX_NAMES` | `FUN_0061F210` preloads standard flag PCXs and side selection maps item data to those PCX names | PASS |
| Flag PCX palette source | embedded PCX palette is parsed by `PcxFile::from_bytes`; external shell/sidebar PAL used = `0` | scoped flag path consumes owner-draw cached PCX surfaces; external shell/sidebar PAL use = `0` | PASS |
| Flag transparency key identity | `OWNER_DRAW_FLAG_TRANSPARENT_RGB == [255,0,255]`; alpha is `0` only when the embedded palette RGB equals that key | `OwnerDraw_Static_006153E0` computes the display-format key from RGB magenta before calling `FUN_006BA580` | PASS |
| Flag non-magenta index-0 behavior | color-key path makes index `0` opaque unless its embedded RGB is `[255,0,255]`; synthetic test output alpha = `255` for black index 0 | keyed blitter compares converted source pixels to the RGB-magenta display key, not to palette index `0` or `255` | PASS |
| Final display-format pixel equality | Rust stores RGBA atlas pixels and GPU blends later; no retail 16-bit framebuffer sample was captured in this trace | gamemd converts to active 16-bit DirectDraw surface pixels before blit | UNCHECKED |

## Player-Visible Findings

No scoped FAIL or NOT-IMPLEMENTED findings were found for owner-draw PCX palette/key behavior in the current Rust tree.

The previous bad-color risk from treating all owner-draw PCXs as index-0 transparent is no longer present in the inspected code: button/control PCXs now use `to_rgba(None)`, while flags use the separate RGB-magenta key path.

## Adjacent Findings

- Adjacent geometry issue, not counted in this palette/key verdict: `push_flag_entry_native_clipped_centered` uses floating `round()` when centering smaller flag PCXs. For the standard 800x600 flag case, retail has a 47px PCX in a 48px rect and integer `(48 - 47) / 2 == 0`, while Rust would compute `round(0.5) == 1`. This can shift flags one pixel right even though palette/key behavior is now aligned.
- Exact final color parity still needs a screenshot or framebuffer-level comparison because gamemd's intermediate surface is 16-bit DirectDraw-format while Rust renders RGBA through the GPU.

## Sources

- Rust read-only scan: `src/assets/pcx_file.rs`, `src/render/skirmish_shell_chrome.rs`, `src/app_skirmish_shell_render.rs`.
- Ghidra read-only spot checks: `OwnerDraw_Button_00612B70 @ 0x00612B70`, `OwnerDraw_Static_006153E0 @ 0x006153E0`, `FUN_006BA580 @ 0x006BA580`, `FUN_006BA3E0 @ 0x006BA3E0`, `0x006B9D00`, `BSurface__Constructor @ 0x00630310`, `FUN_0061F210 @ 0x0061F210`, `FUN_006BA140 @ 0x006BA140`.
- Verified research docs: `SKIRMISH_LOWER_PCX_DECODE_PALETTE_KEY_PATH_FOR_FLAG_STATICS_GHIDRA_REPORT.md`, `SKIRMISH_FLAG_PCX_PALETTE_AND_NATIVE_CLIP_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`.
