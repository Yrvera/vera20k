# Skirmish Right-Panel Buttons Layout 800x600 Trace

Scenario: standard offline Yuri's Revenge Skirmish dialog `0x102` at
`800x600`, scoped to right-panel chrome composition and Start Game `0x617`,
Choose Map `0x5AA`, and Back `0x5C0` owner-draw button rects, released/pressed
button art/text placement, and draw order.

Status: COMPLETE for numeric layout/source/order comparison. No fresh Ghidra
mutation was performed. Existing verified Ghidra reports were used as the
gamemd source of truth; current Rust was read only. Final framebuffer pixel
equality is UNCHECKED because this trace did not capture and diff retail and
Rust screenshots.

## Pipeline

gamemd: standard offline Skirmish launcher -> dialog `0x102` -> common parent
`WM_PAINT` -> `RightPanel__Draw` -> cached parent blit -> child owner-draw
Button paint through `OwnerDraw_Button_00612B70`.

Rust: `compute_layout(800,600)` -> `build_skirmish_shell_instances` emits
right-panel chrome and owner-draw buttons -> `build_shell_text_draws` emits
button labels -> GPU shell text/sprite batches.

## Stage Results

| Stage | gamemd output | Rust output | Verdict |
|---|---|---|---|
| Active standard-YR route | Offline Skirmish creates dialog `0x102`; `0x617`, `0x5AA`, `0x5C0` are active child Button controls routed to owner draw. Evidence: `SKIRMISH_OWNERDRAW_BUTTON_ASSEMBLY_RIGHT_PANEL_GHIDRA_REPORT.md:53-57`, `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md:27-32`. | `OwnerDrawButton::{StartGame0x617, ChooseMap0x5aa, Back0x5c0}` maps to Start/Choose/Back actions and hit-test identities in `src/ui/skirmish_shell/state/hit_test.rs:267-287`. | PASS |
| Right-panel chrome geometry | `SDTP=(632,0,168,199)`, `SDBTNBKGD` rows at `y=199,241,283,325,367,409,451,493,535`, `tile_count=9`, `SDBTM=(632,577,168,23)`, `LWSCRNL=(0,568,632,32)`. Evidence: `SKIRMISH_RIGHT_PANEL_CHROME_FIRST_PAINT_800X600_TRACE.md:30-38`. | `right_panel_rects(800,600)` computes top `(632,0,168,199)`, tile row `(632,199,168,42)`, count `9`, bottom `(632,577,168,23)` in `src/ui/skirmish_shell/layout.rs:452-480`; existing trace confirms the same lower strip. | PASS |
| First-paint chrome order | Parent common paint draws `SDTP`, repeated `SDBTNBKGD`, no frame-10 overlay on standard first paint, `SDBTM`, then `LWSCRNL`, before owner-draw child buttons. Evidence: `SKIRMISH_0X102_FIRST_PAINT_COMPOSITION_VS_RUST_DRAW_ORDER_GHIDRA_REPORT.md:19-30`, `SKIRMISH_OWNERDRAW_BUTTON_ASSEMBLY_RIGHT_PANEL_GHIDRA_REPORT.md:73-85`. | `build_skirmish_shell_instances` emits top, 9 tiles, optional frame-10 only when active, bottom, lower strip, parent background, then owner-draw buttons in `src/app_skirmish_shell_render.rs:192-286`; semantic test asserts this order in `src/app_skirmish_shell_render.rs:1096-1119`. | PASS |
| Button child rects | Start `(644,241,156,42)`, Choose `(644,283,156,42)`, Back `(644,535,156,42)`. Start/Choose use `FUN_0060B000`; Back uses `FUN_0060B350`. Evidence: `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md:40-42`, `:53-60`; `SKIRMISH_OWNERDRAW_BUTTON_ASSEMBLY_RIGHT_PANEL_GHIDRA_REPORT.md:61-71`. | `compute_layout(800,600)` uses `SDBTNANM_W=156`, `SDBTNANM_H=42`; Start/Choose snap through `owner_draw_button_snap_rect`, Back through `back_rect`, in `src/ui/skirmish_shell/layout.rs:6-7`, `:489-515`, `:598-604`. | PASS |
| Button type and released/pressed art | Active classifier sets these three right-panel buttons to owner-draw type `1`. Released/default uses `SDBTNANM.SHP` frame `2`; pressed uses frame `4`; frame `3` is hover/timer-adjacent. Evidence: `SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md:9-18`, `:20-28`. | Atlas loads `SDBTNANM.SHP` frames `2`, `3`, `4`, `10` using `SDBTNANM.PAL` in `src/render/skirmish_shell_chrome.rs:151-166`; `push_right_panel_button_shp` selects frame `2` or `4` in `src/app_skirmish_shell_render/chrome.rs:363-389`. Focused test `right_panel_buttons_use_sdbtnanm_type1_frames` passed. | PASS |
| Active-path button piece placement | No cap/middle/right PCX pieces are used for these three controls on the active type-1 standard path; the button is the SDBTNANM frame in the 156x42 child rect. Older PCX-piece claims are superseded. Evidence: `SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md:35-49`. | Rust follows the type-1 SDBTNANM path when frames are present; PCX `push_button_30` is only a missing-asset fallback in `src/app_skirmish_shell_render/chrome.rs:371-380`. | PASS |
| Released/pressed text rects and color | For Start rect `(644,241,156,42)`, released text rect is `(644,242,154,41)`; pressed is `(646,246,152,37)`. Text is centered and enabled source color is yellow `0x0000FFFF`. Evidence: `SKIRMISH_OWNERDRAW_BUTTON_ASSEMBLY_RIGHT_PANEL_GHIDRA_REPORT.md:179-194`; type-1 color evidence in `SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md:13-16`. | `button_text_rect` keeps fixed right/bottom edges: released `(644,242,154,41)`, pressed `(646,246,152,37)` in `src/app_skirmish_shell_render/text.rs:172-185`; `push_button_label_draw` uses centered alignment and `button_label_color()` in `src/app_skirmish_shell_render/text.rs:136-157`. Focused test `button_text_rect_follows_owner_draw_caller_contract` passed. | PASS |
| Parent chrome vs child button layering | Parent chrome is complete before child owner-draw buttons paint over it. Evidence: `SKIRMISH_OWNERDRAW_BUTTON_ASSEMBLY_RIGHT_PANEL_GHIDRA_REPORT.md:31-33`, `:176-182`. | Rust emits owner-draw button sprites after right-panel/lower-strip/parent background roles in `src/app_skirmish_shell_render.rs:192-286` and `src/app_skirmish_shell_render/draw_order.rs:82-108`. | PASS |
| Inter-button child paint order | Not rechecked in this slot. The three rects do not overlap in this scenario, so no button-to-button overlap pixel is expected, but exact HWND child paint order was not recomputed from gamemd. | Rust submits Start, Back, Choose in `src/app_skirmish_shell_render.rs:263-286`. | UNCHECKED |
| Aggregate framebuffer pixels | Retail framebuffer was not captured in this slot. | Rust framebuffer was not captured in this slot. | UNCHECKED |

## Findings

No FAIL or NOT-IMPLEMENTED findings were found inside this slot. The important
current correction is that older traces describing Start/Choose as
`(635,242,162,37)` / `(635,286,162,37)` and using generic gray PCX button art
are stale for this worktree and for current verified gamemd evidence.

## Adjacent Findings

- Standard app reachability remains a broader route/layout slot: the recovered
  pixel shell has historically been dev-gated, but this trace did not evaluate
  menu routing or whether this is now the default player path.
- Disabled Start validation visuals are out of scope for fresh enabled
  Start/Choose/Back layout. Existing research says disabled Start should use
  released frame/art plus alpha behavior, not a separate `bud_*` path.
- Hover/timer frame `3` is adjacent. This trace only compared released/default
  frame `2` and pressed frame `4`.

## Sources

- Existing verified reports: `SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md`,
  `SKIRMISH_OWNERDRAW_BUTTON_ASSEMBLY_RIGHT_PANEL_GHIDRA_REPORT.md`,
  `SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`,
  `SKIRMISH_0X102_FIRST_PAINT_COMPOSITION_VS_RUST_DRAW_ORDER_GHIDRA_REPORT.md`,
  `SKIRMISH_RIGHT_PANEL_CHROME_FIRST_PAINT_800X600_TRACE.md`.
- Rust read-only surfaces: `src/ui/skirmish_shell/layout.rs`,
  `src/ui/skirmish_shell/state/hit_test.rs`, `src/app_skirmish_shell_render.rs`,
  `src/app_skirmish_shell_render/chrome.rs`,
  `src/app_skirmish_shell_render/text.rs`,
  `src/app_skirmish_shell_render/draw_order.rs`,
  `src/render/skirmish_shell_chrome.rs`.
- Verification run: `cargo test -q right_panel_buttons_use_sdbtnanm_type1_frames`;
  `cargo test -q button_text_rect_follows_owner_draw_caller_contract`;
  `cargo test -q semantic_draw_order_records_verified_right_panel_sequence`.

## Verdict Tally

PASS: 8 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0
