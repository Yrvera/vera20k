# Skirmish Right-Panel Chrome First Paint 800x600 Trace

Date: 2026-05-22

Scenario: standard offline Yuri's Revenge Skirmish setup dialog `0x102` at `800x600`, first paint, right-panel chrome only: `SDTP`, repeated `SDBTNBKGD`, first-paint `SDBTNANM.SHP` frame-10 overlay gate, `SDBTM`, and `LWSCRNL` lower strip.

Scope note: the Rust comparison targets the current recovered/experimental Skirmish shell renderer output for this chrome slice. Normal app reachability through the dev gate is listed as adjacent, not part of this chrome-only verdict.

## Verdict

PASS: 7 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

For the scoped first-paint chrome structure, current Rust matches the verified gamemd contracts at `800x600`: right-panel rects, asset/palette selection, nine background tiles, skipped `SDBTNANM` frame-10 rows, top-clipped native `SDBTM`, `LWSCRNL` lower strip placement, and scoped draw order. Final framebuffer pixel equality remains unchecked because this slot did not capture retail and Rust framebuffers.

## Active YR Evidence

Existing Ghidra reports confirm this is active in standard YR, not dormant TS legacy:

- Standard offline Skirmish reaches dialog `0x102` through `FUN_006AE2C0 -> FUN_0072CF40 -> CreateDialogIndirectParamA(dialog 0x102, proc 0x006AE3F0)`.
- The common parent `WM_PAINT` path calls `RightPanel__Draw @ 0x0072E450`.
- `RightPanel__Draw` uses `SDTP.SHP`, repeated `SDBTNBKGD.SHP`, conditional `SDBTNANM.SHP` frame `10`, `SDBTM.SHP`, then width-selected `LWSCRNS.SHP`/`LWSCRNL.SHP`.
- Standard offline first paint leaves the frame-10 gate byte zero, so `SDBTNANM.SHP` frame `10` is skipped for dialog `0x102`.

## Pipeline

`offline Skirmish dialog 0x102 first WM_PAINT -> common shell parent paint -> RightPanel__Draw -> scoped chrome stack -> child/control paint outside this slot`

## Stage Table

| Stage | Boundary output | gamemd 800x600 | Rust 800x600 | Verdict |
|---:|---|---|---|---|
| 1 | Right-panel destination rects | `SDTP=(632,0,168,199)`, first tile `(632,199,168,42)`, `tile_count=9`, `SDBTM=(632,577,168,23)` | `compute_layout(800,600)` produces the same rects and count. | PASS |
| 2 | `SDTP` source and palette | `SDTP.SHP` frame `0`, `SHELL.PAL`, drawn at `(632,0,168,199)` | Atlas loads `SDTP.SHP` frame `0` with `SHELL.PAL`; renderer emits it to `layout.right_panel.top`. | PASS |
| 3 | `SDBTNBKGD` source, palette, repetition | `SDBTNBKGD.SHP` frame `0`, `SHELL2.PAL`, nine rows at `y=199,241,283,325,367,409,451,493,535` | Atlas loads `SDBTNBKGD.SHP` frame `0` with `SHELL2.PAL`; renderer loops `tile_count=9` with 42 px stride. | PASS |
| 4 | `SDBTNANM` frame-10 first-paint gate | Gate byte is zero; overlay visible row count is `0`. | `right_panel_frame10_overlay_active` returns `false`; no frame-10 role or sprite is emitted. | PASS |
| 5 | `SDBTM` bottom cap sampling | `SDBTM.SHP` frame `0`, `SHELL.PAL`, native `168x65`, visible top source rows `0..22` at `(632,577,168,23)` | `push_entry_top_clipped_native` draws width `168`, height `23`, UV height `23/65`; no vertical full-source stretch. | PASS |
| 6 | `LWSCRNL` lower strip | Non-640 width selects `LWSCRNL.SHP` frame `0`, `SHELL.PAL`, rect `(0,568,632,32)` | `lower_strip_role(800)` selects large strip; `lower_strip_rect` returns `(0,568,632,32)` for native `632x32`. | PASS |
| 7 | Scoped chrome draw order | `SDTP -> 9*SDBTNBKGD -> 0*SDBTNANM -> SDBTM -> LWSCRNL` | `build_skirmish_shell_instances` emits top, tiles, skipped overlay, bottom, then lower strip. | PASS |
| 8 | Aggregate first-frame pixels | Retail framebuffer pixels not captured in this slot. | Rust framebuffer pixels not captured in this slot. | UNCHECKED |

## Checked Rust Evidence

- `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs:319` computes right-panel rects.
- `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs:690` asserts the 800x600 right-panel rects.
- `C:/Users/enok/Documents/ra2-rust-game/src/render/skirmish_shell_chrome.rs:98` loads the scoped SHP atlas entries.
- `C:/Users/enok/Documents/ra2-rust-game/src/render/skirmish_shell_chrome.rs:109` loads `SDTP.SHP` with `SHELL.PAL`.
- `C:/Users/enok/Documents/ra2-rust-game/src/render/skirmish_shell_chrome.rs:116` loads `SDBTNBKGD.SHP` with `SHELL2.PAL`.
- `C:/Users/enok/Documents/ra2-rust-game/src/render/skirmish_shell_chrome.rs:123` loads `SDBTM.SHP` with `SHELL.PAL`.
- `C:/Users/enok/Documents/ra2-rust-game/src/render/skirmish_shell_chrome.rs:130` loads optional `SDBTNANM.SHP` frame `10`.
- `C:/Users/enok/Documents/ra2-rust-game/src/render/skirmish_shell_chrome.rs:158` loads `LWSCRNL.SHP` with `SHELL.PAL`.
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:141` clips native `SDBTM` source by top rows.
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:1011` selects `LWSCRNL` for width `800`.
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:1042` computes the lower strip rect from native asset size and shell bottom.
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:1065` returns `false` for the standard offline first-paint frame-10 overlay gate.
- `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:1123` emits the scoped chrome stack in order.

## Failures

None in scope.

## Not Implemented

None in scope.

## Unchecked

Aggregate pixel equality is unchecked. This report compares numeric rects, asset/frame/palette choices, source clipping, overlay count, and draw order; it does not include a retail screenshot, Rust screenshot, or pixel diff.

## Adjacent Findings

- The recovered Skirmish shell renderer is still gated by `RA2_DEV_SKIRMISH_SHELL` / `dev_skirmish_shell_enabled` in the app path; normal player reachability is outside this chrome-only slot.
- Parent background, Start/Choose/Back owner-draw buttons, text statics, map preview, flags, combos, checkboxes, and trackbars are separate trace-swarm slots.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_RIGHT_PANEL_SHELL_ASSET_PALETTE_SELECTION_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_SDBTM_BOTTOM_CAP_SOURCE_CLIP_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_SDBTNANM_FRAME10_FIRST_PAINT_FLAG_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/traces/SKIRMISH_SHELL_CHROME_FIRST_PAINT_800X600_VISUAL_TRACE.md`
- Rust files checked: `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`, `src/app.rs`.
