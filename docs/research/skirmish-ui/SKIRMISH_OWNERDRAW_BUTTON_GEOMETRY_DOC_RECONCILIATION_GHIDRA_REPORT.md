# Skirmish Owner-Draw Button Geometry Doc Reconciliation - Ghidra Research Report

**Address(es):** `OwnerDraw_Button_00612B70 @ 0x00612B70`, `FUN_006BA3E0 @ 0x006BA3E0`, `FUN_0060B000 @ 0x0060B000`, `FUN_0060B350 @ 0x0060B350`, `FUN_0060F9A0 @ 0x0060F9A0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** reconciliation of current Rust right-panel Start Game `0x617`, Choose Map `0x5AA`, and Back `0x5C0` owner-draw button geometry/art behavior against existing skirmish button docs: final rects, PCX family dimensions, art y placement, pressed art movement, fixed text rect geometry, middle tiling/crop phase, and cap overlap.
**Non-Scope:** text color, disabled text color, click sound, Choose Map modal internals, Start validation modal contents, and non-skirmish buttons. Text color is intentionally left to swarm slot 1.
**Confidence:** High for active path, rects, binary formulas, and current Rust geometry scan; Medium for final visible pixels where `BSurface` clipping of a 10px destination against the 7px `bde_ri30.pcx` source needs runtime screenshot confirmation.
**Active in YR:** Yes. `FUN_006AE2C0` creates and pumps standard offline Skirmish dialog `0x102`; `FUN_0060F9A0` installs `OwnerDraw_Button_00612B70` for Button controls whose low style bits satisfy `(style & 0x0B) == 0x0B`; prior resource reports identify `0x617`, `0x5AA`, and `0x5C0` as those controls.

## 0. Working Notes

**Target question:** Which current Rust owner-draw button geometry/art claims are settled, which older docs are stale, and whether any real current Rust geometry delta remains for Start/Choose/Back.

**Non-goals:** no text color decision; no modal internals; no Rust edits; no broad shell paint-order audit; no new asset replacement.

**Evidence needed to mark COMPLETE:** binary route proof, decompile plus assembly context for rect helpers and owner-draw art/text geometry, retail PCX dimensions for up/down button assets, current Rust scan of layout/render geometry functions, and exact stale-doc replacement wording.

**Stop conditions:** all scoped questions resolved or explicitly deferred; exactly one new report written; no Rust/INI edits; no Ghidra mutation.

## 1. Overview

Most of the older geometry failures are stale. Current Rust already uses the 800x600 snap rects, the `bue/bde_*30.pcx` family, native asset heights, vertical centering, pressed +2 y movement, fixed-right/fixed-bottom text rect geometry, cap overlap, and centered middle source phase.

One real geometry/art mismatch remains in the pressed/down state: retail `bde_li30.pcx`, `bde_mi30.pcx`, and `bde_ri30.pcx` are 27px high, but the binary computes the pressed y baseline from the selected `30` family height before drawing the actual 27px source. Current Rust centers the actual 27px down art, making pressed art 1px too low. `bde_ri30.pcx` is also 7px wide, while the binary right-cap destination stays anchored at `button_right - 10`; current Rust anchors the pressed right cap from the actual 7px width, shifting that seam 3px right.

## 2. Verified Binary Findings

| Finding | Evidence | Active in YR |
|---|---|---|
| Start/Choose/Back route to `OwnerDraw_Button_00612B70` through Button style low bits `0x0B`. | `FUN_0060F9A0` Button branch assigns `OwnerDraw_Button_00612B70` for `(style & 0x0B) == 0x0B`; prior layout reports map `0x617/0x5AA/0x5C0`. | Yes |
| Start and Choose use the `SDBTNANM` snap helper; Back uses the bottom helper. At 800x600 the settled rects are Start `(644,241,156,42)`, Choose `(644,283,156,42)`, Back `(644,535,156,42)`. | `FUN_0060B000` decompile uses `g_SDBTNANM_SHP` width/height and right-panel tile snapping; `FUN_0060B350` computes the last complete tile row; current Rust tests and prior resize-policy report match these rects. | Yes |
| Size-family selection chooses the 30 family for 42px controls, with table cap widths left `7`, right `10`. | Assembly `0x006132B9..0x006132F9`: table seeds `24`, `30`, left cap `7`, right cap `10`, then selects by client height. | Yes |
| Art y is computed from selected family height, not from actual PCX source height: `top + (control_h - 30) / 2`, then `+2` when pressed and not disabled. | Assembly `0x00613394..0x006133AE`: subtract selected `ESI`, signed half divide, add to top, then `ADD EAX,0x2` for pressed. | Yes |
| The actual blit/tile height is then replaced by the loaded source PCX height for each piece. | Middle/right contexts: `0x006134A4` and `0x00613529` call vtable `+0x80`, followed by stores of `EAX` to destination height slots at `0x006134BA` and `0x00613533`; left path uses the same surface height pattern. | Yes |
| Released assets are `bue_li30.pcx 7x30`, `bue_mi30.pcx 177x30`, `bue_ri30.pcx 10x30`; pressed assets are `bde_li30.pcx 7x27`, `bde_mi30.pcx 177x27`, `bde_ri30.pcx 7x27`. | Retail asset dump from `cargo run --bin inspect-pcx-palette` on 2026-05-23. | Yes |
| Middle destination starts at `x + 7`, uses width `button_w - 10`, and is drawn before the right cap. Right cap destination starts at `x + button_w - 10`; this creates the verified 7px overlap in destination coordinates. | Assembly contexts: left blit `0x00613441`, middle rect/call `0x0061348D..0x006134C4`, right rect/blit `0x0061351D..0x0061355D`. | Yes |
| Middle tiling uses centered source crop before modulo copy. | `FUN_006BA3E0` decompile and assembly `0x006BA495..0x006BA4A4` compute `max(0,(src_w - dest_w)/2)`; inner loop around `0x006BA4D7..0x006BA558` modulo-copies source pixels. | Yes |
| Button text rectangle geometry is released `left=x`, `top=y+1`, `right=x+w-2`, `bottom=y+h`; pressed changes only left to `x+2` and top to `y+5`. | `OwnerDraw_Button_00612B70` assembly `0x00613591..0x006135CD`, call to `FUN_00621040` at `0x006135EE`. | Yes |

## 3. Current Rust Implementation Status

| Surface | Current Rust state | Verdict |
|---|---|---|
| `src/ui/skirmish_shell/layout.rs` `owner_draw_button_snap_rect` / `back_rect` | Produces Start `(644,241,156,42)`, Choose `(644,283,156,42)`, Back `(644,535,156,42)` at 800x600. | matches |
| `src/app_skirmish_shell_render/chrome.rs` `button_piece_asset_names` / `button_entries` | Uses `bue_*30` released and `bde_*30` pressed assets. | matches |
| `build_button_segments` for released assets | With `bue` right cap width `10`, middle starts at `x+7`, width `w-10`, right cap starts at `right-10`, and centered source phase is present. | matches |
| `build_button_segments` for pressed assets | Uses actual `bde_ri30.pcx` width `7`; right cap starts at `right-7`, and middle width becomes `w-7`. | mismatch vs binary destination anchor/table width |
| `button_segment_sprite_size` | Uses actual loaded source height. This matches the binary blit height after surface load. | matches for draw height |
| `button_art_y` | Centers using the actual loaded entry height, then applies pressed +2. For 27px pressed art, Rust yields `top + 7 + 2`; binary y baseline is `top + 6 + 2`. | mismatch, pressed art 1px too low |
| `button_text_rect` | Uses fixed right/bottom; pressed left+2/top+5. | matches geometry |
| `push_button_label_draw` color | Out of scope; handled by slot 1. | not assessed here |

## 4. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Standard `0x102` active path | verified | `FUN_006AE2C0`; `FUN_0060F9A0`; prior resource reports | none |
| Start/Choose/Back final rect helpers | verified | `FUN_0060B000`; `FUN_0060B350`; current Rust scan | none |
| PCX family and source dimensions | verified | owner-draw filename block; `inspect-pcx-palette` output | none |
| Art y baseline and pressed +2 | verified | `0x00613394..0x006133AE` | none |
| Piece draw height after source load | verified | `0x006134A4..0x006134BA`, `0x00613529..0x00613533` | left cap exact store path not separately re-disassembled, but same pattern is visible in decompile |
| Cap overlap / right anchor | verified | `0x0061348D..0x0061355D` | exact final pixels for 10px dest vs 7px down-right source need screenshot |
| Middle source phase | verified | `FUN_006BA3E0`; current `build_button_segments` | possible half-pixel vs integer truncation is negligible only if UV sampling is nearest/exact; test should pin integer phase |
| Text rect geometry | verified | `0x00613591..0x006135EE`; current `button_text_rect` | text color excluded |
| Current Rust geometry scan | verified | `chrome.rs`, `text.rs`, `layout.rs`, current tests | no code changed |

## 5. Open Questions - Final State

- `[RESOLVED] OQ-01 - Are scoped buttons active in standard YR Skirmish? -> Yes, through dialog 0x102 and OwnerDraw_Button_00612B70.` (evidence: `FUN_006AE2C0`, `FUN_0060F9A0`)
- `[RESOLVED] OQ-02 - Which rects are settled? -> Start `(644,241,156,42)`, Choose `(644,283,156,42)`, Back `(644,535,156,42)`.` (evidence: `FUN_0060B000`, `FUN_0060B350`, current Rust layout)
- `[RESOLVED] OQ-03 - Is native 30px art height settled? -> Released up-state is 30px; pressed down-state retail PCXs are actually 27px high, while the binary y baseline still uses selected family height 30.` (evidence: `0x006132B9..0x006133AE`; `inspect-pcx-palette`)
- `[RESOLVED] OQ-04 - Does pressed art move? -> Yes, y baseline adds +2 when pressed and not disabled.` (evidence: `0x006133A1..0x006133AE`)
- `[RESOLVED] OQ-05 - Are fixed-right text rect claims settled? -> Yes; current Rust matches geometry.` (evidence: `0x00613591..0x006135EE`; `button_text_rect`)
- `[RESOLVED] OQ-06 - Is middle tiling/cropping phase settled? -> Yes; binary centers source crop before modulo copy, and current Rust has a centered phase path.` (evidence: `FUN_006BA3E0`; `build_button_segments`)
- `[RESOLVED] OQ-07 - Is cap overlap settled? -> Yes for destination coordinates: middle width is `w-10`, right starts at `right-10`, so overlap is 7px.` (evidence: `0x0061348D..0x0061355D`)
- `[RESOLVED] OQ-08 - Which older docs are stale? -> Docs claiming current Rust still stretches art, lacks pressed y movement, or lacks fixed-right text rect are stale.` (evidence: current Rust scan)
- `[DEFERRED] OQ-09 - What are the exact final pixels when destination right cap is 10px but `bde_ri30.pcx` source is 7px?` (category: `needs-runtime-debugger`; reason: static binary proves destination anchor and source dimensions, but final clip fill should be screenshot-confirmed; next-step-if-pursued: capture pressed retail Start button and compare right seam)
- `[DEFERRED] OQ-10 - Does fractional UV phase in Rust ever differ from binary integer truncation by a visible pixel?` (category: `requires-different-system-context`; reason: depends on GPU sampler and atlas texel addressing; next-step-if-pursued: add integer-phase test and screenshot diff)

## 6. Visual/UI Composition Ledger

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `OwnerDraw_Button_00612B70` left blit `0x00613441` | default PCX path `state+0xB0 == 0`, `state+0x14 == 0` | `bue_li30` or `bde_li30` | x, baseline y, table width 7, source height | PCX surface cache | yes | left cap |
| 2 | `FUN_006BA3E0` call `0x006134C4` | same | `bue_mi30` or `bde_mi30` | x+7, baseline y, width `w-10`, source height | modulo tile copy | yes | middle fill |
| 3 | right blit `0x0061355D` | same | `bue_ri30` or `bde_ri30` | x+w-10, baseline y, table width 10, source height | PCX surface cache | yes | right cap / overlap |
| 4 | `FUN_00621040` call `0x006135EE` | `state+0x14 == 0 && state+0x28 != 0` | text | released top+1/right-2; pressed left+2/top+5 | text color out of scope | yes | label geometry |

## 7. Asset Role Matrix

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `bue_li30.pcx` | yes | yes | released | no | yes | no | no | no | filename block; asset dump `7x30` |
| `bue_mi30.pcx` | yes | yes | released | no | yes | no | no | no | filename block; asset dump `177x30` |
| `bue_ri30.pcx` | yes | yes | released | no | yes | no | no | no | filename block; asset dump `10x30` |
| `bde_li30.pcx` | yes | yes | pressed | no | yes | no | no | no | filename block; asset dump `7x27` |
| `bde_mi30.pcx` | yes | yes | pressed | no | yes | no | no | no | filename block; asset dump `177x27` |
| `bde_ri30.pcx` | yes | yes | pressed | no | yes | no | no | no | filename block; asset dump `7x27` |
| `SDBTNANM.SHP` | yes | no for these button faces | no as face art | no | right-panel sizing reference | no | conditional parent overlay | face path inactive | `state+0xB0 == 0` PCX path; rect helpers read SDBTNANM size |

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Pressed y baseline uses selected family height 30, then draws actual 27px down PCX height. | `0x00613394..0x006133AE`; `0x006134A4..0x006134BA`; asset dump | mismatch | `src/app_skirmish_shell_render/chrome.rs` `button_art_y` / `push_button_30` | compute y from selected 30-family baseline for pressed/down assets, while retaining actual source draw height | Press Start at 800x600: `bde_*30` art y is `249`, not current `250`; proposed test `skirmish_pressed_button_art_y_uses_selected_family_height_not_down_pcx_height` | Do not stretch 27px down art to 30px; only fix anchor/baseline |
| Right cap destination anchor uses table right width 10 even when `bde_ri30.pcx` source is 7px wide. | `0x006132B9..0x006132F9`; `0x0061351D..0x0061355D`; asset dump | mismatch in pressed state | `src/app_skirmish_shell_render/chrome.rs` `build_button_segments` | use binary table widths `left=7/right=10` for destination geometry; source UV width may still reflect actual asset | Pressed Back right seam begins at `x+146`, not `x+149`; proposed test `skirmish_pressed_button_right_cap_keeps_binary_table_anchor` | Do not derive cap destination anchors solely from current PCX dimensions |
| Released rects, native up art height, centered middle phase, 7px destination overlap, and fixed text rect geometry are already settled. | `FUN_0060B000`, `FUN_0060B350`, `FUN_006BA3E0`, `0x00613591..0x006135EE`; current Rust scan | none observed | `layout.rs`, `chrome.rs`, `text.rs` | preserve current behavior except pressed baseline/table-width correction | Existing 800x600 button tests keep passing; proposed test `skirmish_ownerdraw_button_geometry_regression_released_state` | Do not resurrect older `(635,242,162,37)` Start/Choose rects or full-rect text centering |

## 9. Negative Facts / Do Not Do

- Do not treat `SDBTNANM.SHP` as the visible face art for these three skirmish buttons. Active in YR: No for this path; it supplies sizing/snap context, while `OwnerDraw_Button_00612B70` uses PCX `bue/bde` assets.
- Do not revive old Start/Choose rects `(635,242,162,37)` and `(635,286,162,37)`. Active in YR: No for current verified `FUN_0060B000` owner-draw snap rects.
- Do not stretch released or pressed PCX art to the 42px HWND height. Active in YR: No; art draw height comes from selected/source surface paths, not the control height.
- Do not assume every `*30` PCX is 30px high or that `bde_ri30.pcx` is 10px wide. Active in YR: No; retail down assets are `7x27`, `177x27`, `7x27`.
- Do not "fix" geometry by changing button text color; color is a separate slot-1 question.

## 10. Stale Docs / Follow-Up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md` replacement wording:
  "Current Rust no longer has the older released-state geometry failures: Start/Choose/Back 800x600 snap rects, released 30px up-art vertical placement, fixed-right/fixed-bottom text rects, destination cap overlap, and centered middle source phase are implemented. Remaining geometry/art delta: pressed/down state must use the binary's selected-30-family y baseline and table right-cap destination width even though the retail `bde_*30` PCXs are 27px high and `bde_ri30.pcx` is 7px wide. Text color is handled by the button color audit."

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_OWNERDRAW_BUTTON_PIXEL_RECHECK_800X600_GHIDRA_REPORT.md` replacement wording:
  "Superseded for current-Rust geometry status by `SKIRMISH_OWNERDRAW_BUTTON_GEOMETRY_DOC_RECONCILIATION_GHIDRA_REPORT.md`: the previous 'only middle source phase remains' claim is stale because current Rust now has centered middle phase. The current geometry deltas are pressed/down-state y baseline (+1px too low in Rust) and pressed right-cap destination anchor/table width."

- `C:/Users/enok/Documents/ra2-rust-game-docs/traces/SKIRMISH_START_CHOOSE_BACK_OWNER_DRAW_BUTTONS_800X600_TRACE.md` replacement wording:
  "Keep as historical trace only. Current Rust has fixed art y for released state, pressed art movement concept, button rects, cap overlap, middle source phase, and text rect geometry. It still needs a pressed/down asset correction because retail down PCXs are 27px high and the binary anchors them from the selected 30-family baseline."

## Sources

- Ghidra read-only decompile: `OwnerDraw_Button_00612B70 @ 0x00612B70`, `FUN_006BA3E0 @ 0x006BA3E0`, `FUN_0060B000 @ 0x0060B000`, `FUN_0060B350 @ 0x0060B350`, `FUN_0060F9A0 @ 0x0060F9A0`.
- Assembly/context evidence: `0x006132B9..0x006132F9`, `0x00613394..0x006133AE`, `0x0061348D..0x006134C4`, `0x0061351D..0x0061355D`, `0x00613591..0x006135EE`, `0x006BA495..0x006BA558`.
- Asset extraction: `cargo run --bin inspect-pcx-palette`, 2026-05-23 output for `bue_*30.pcx` and `bde_*30.pcx`.
- Rust read-only scan: `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render/chrome.rs`, `src/app_skirmish_shell_render/text.rs`, `src/app_skirmish_shell_render.rs` tests.
