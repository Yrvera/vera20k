# Skirmish Chrome Control Art Substitutions - Ghidra Research Report

## Superseded Button-Art Note - 2026-05-24

Rows in this report that treat `bue_*30.pcx` / `bde_*30.pcx` as verified
normal art for standard Skirmish sidebar Start/Choose/Back are superseded. The
corrected classifier recheck proves those three right-panel controls are
owner-draw type `1` and use `SDBTNANM.SHP` frames `2`/`4`. Use
`SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md`.

**Address(es):** `OwnerDraw_Button_00612B70`, `OwnerDraw_Checkbox_006163A0`, `OwnerDraw_ComboBox_00617250`, `OwnerDraw_Trackbar_0061D950`, `FUN_006208F0`, `FUN_0061F210`, `FUN_006BA140`, `FUN_006BA580`, `RightPanel__Draw @ 0x0072E450`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** standard offline YR Skirmish dialog `0x102` chrome/control art substitutions: which current Rust PCX/SHP/primitive pieces are verified retail behavior versus placeholders or research candidates.
**Non-Scope:** parent first-paint composition order, text caller rects except where art state depends on text/disabled state, Choose Map modal visuals, and new Rust implementation.
**Confidence:** High for active asset/primitive classification and current Rust deltas; Medium for final primitive RGB pixels because Rust uses RGBA approximations instead of a live 16-bit DirectDraw capture.
**Active in YR:** Yes for the standard offline Skirmish `0x102` paths named below; Conditional or No where explicitly marked.

## 0. Working Notes

Target question: Which current Rust Skirmish shell chrome/control art pieces are verified retail art versus placeholders/substitutions, and which substitutions most visibly block gamemd-alike quality?
Non-goals: Do not investigate parent composition order, text caller rects, or Choose Map modal, except for negative asset facts.
Evidence needed to mark COMPLETE: reconcile current Rust atlas/render surfaces against Ghidra-verified owner-draw button, checkbox, combo, trackbar, scrollbar/arrow, flag, right-panel SHP, palette/key, alpha, and clipping behavior.
Stop conditions: stop after every scoped current Rust substitution is classified as verified, mismatch, placeholder, research-candidate, or out-of-scope; do not modify Rust or other docs.

## 1. Overview

The current Rust Skirmish shell is not blocked by unknown magic asset names. Most visible standard `0x102` control-art families are already known: PCX buttons, checkbox icons, combo arrows, trackbar thumb/plaque, flag PCXs, and right-panel SHPs are retail-backed. The largest current substitutions are where Rust turns binary surface behavior into simplified RGBA sprites: primitive combo/trackbar bevels, button segment height, non-flag PCX transparency, and dropdown/scrollbar composition.

Active in YR: Yes. Evidence: standard offline path reaches dialog `0x102`, `FUN_0060F9A0` subclasses child controls, and the decompiled owner-draw callbacks above are active on those child controls.

## 2. Asset / Substitution Matrix

| Current Rust piece | Classification | Active in YR | Evidence | Current Rust status |
|---|---|---:|---|---|
| `bue_li30/mi30/ri30.pcx`, `bde_li30/mi30/ri30.pcx` | verified retail art | Yes | `OwnerDraw_Button_00612B70` formats `b%c%c_*30.pcx`; disabled forces released art | names match; Rust stretches pieces to full control height |
| `bud_li/mi/ri24/30.pcx` | do not use for standard buttons | No for standard `0x102` | `batch_string_anchor_report("bud_")` found preload-only xrefs in `FUN_0061F210`; button callback uses fixed second char `'e'` | correctly not loaded into Skirmish atlas |
| `cue_i.pcx`, `cce_i.pcx` | verified standard checkbox art | Yes | checkbox callback default branch; standard init sends only `0xF1` | loaded/rendered |
| `cce_il.pcx`, `cce_ir.pcx` | conditional variant art | No for standard `0x102`; Conditional elsewhere | variant bytes `+0xD9/+0xDA`, no standard writer | correctly not used by standard shell |
| `bst_*.pcx` | preload-only / not standard checkbox art | No for standard `0x102` | `batch_string_anchor_report("bst_")` found only `FUN_0061F210` | correctly not used |
| `trakgrip.pcx`, `trofl/trofm/trofr.pcx` | verified trackbar art | Yes | `OwnerDraw_Trackbar_0061D950` lookups at `0x0061DE9C..0x0061E0AD` | loaded/rendered |
| `skirmish_trackbar_rail` primitive | verified primitive behavior, not a retail file | Yes | trackbar calls `FUN_006208F0` at `0x0061E204` and `0x0061E269` | implemented as pre-rendered RGBA primitive |
| `skirmish_combo_face_*` primitives | verified primitive behavior, not a retail file | Yes | combo calls `FUN_006208F0` at `0x00617893`; face height 24 | implemented as pre-rendered RGBA primitives |
| `dnarrowr/p.pcx`, `gdnarrowr/p.pcx` | verified combo down-arrow art | Yes/Conditional | `FUN_00620720` selected by combo grey/pressed state | loaded/rendered |
| `uparrowr/p.pcx`, `sbgript/m/b.pcx` | broad owner-draw scrollbar art | Conditional | preload list and combo dropdown scrollbar reports; not the base collapsed combo face | loaded/rendered; dropdown styling remains partially primitive |
| flag PCXs `rani/usai/.../yrii/obsi.pcx` | verified flag statics | Yes for `-2,0..9`; Conditional for observer `-3` | `FUN_004E3560`, static kind-2 path | loaded with magenta color key and native clipping |
| `SDTP.SHP#0`, `SDBTNBKGD.SHP`, `SDBTNANM.SHP#10`, `SDBTM.SHP`, `LWSCRNS/LWSCRNL.SHP` | verified base right-panel/lower SHP chrome | Yes/Conditional for `SDBTNANM#10` | `RightPanel__Draw @ 0x0072E450`; palette split reports | loaded; `SDBTM` crop behavior needs checking in current render path |
| `SDTP.SHP#1` | verified Skirmish top preview-head overlay | Yes for standard `0x102` | `FUN_0060CAF0` sets the flag; `Sidebar_TopHighlight @ 0x0072E8C0` draws frame 1 | draw over the frame-0 top cap at `DAT_00B0FC20` |
| `SDMPBTN.SHP#0` | verified lower Battle/map-name right-panel chrome, not preview bitmap/backing | Yes for standard `0x102` | `FUN_0060C930` sets the flag; `Minimap_Button @ 0x0072E860` draws frame 0 at `DAT_00B0FC14` | loaded as `sd_map_button`; use at the computed chrome rect, not `layout.map_preview` |
| `MNSCRNL.SHP`, `dbak6440.pcx`, `dlgsysa/i.pcx` | research candidates for this exact base screen | No for standard fresh `0x102` base paint as direct substitutes | prior right-panel/background and PCX preload reports | classified as `ResearchCandidate`; do not promote without new evidence |
| `BTN-MINS.SHP`, `BTN-PLUS.SHP` | generic `SliderClass`, not standard Skirmish trackbars | No for standard `0x102`; Conditional elsewhere | generic use-site `0x006B1B90/0x006B1BCA`; standard trackbars use PCX/primitive owner-draw | correctly not used |
| `MnScrnLCustomizeBattle.shp` | Choose Map modal asset, not base `0x102` background | No for base `0x102` | settled parent context | correctly research-candidate for base shell |

## 3. Core Findings

### 3.1 Primitive bevels are not placeholders, but Rust's representation is still an approximation

Active in YR: Yes. `OwnerDraw_ComboBox_00617250` and `OwnerDraw_Trackbar_0061D950` call `FUN_006208F0` for collapsed combo faces and trackbar rails/value-side primitive frames. The helper consumes `[x,y,width,height]`, expands by border width `2`, draws two bevel rings, swaps `DAT_00AC1B98 = 0xC5BEA7` and `DAT_00AC1B94 = 0x807A68` between outer/inner rings, and writes averaged mixed corners.

Rust status: mostly matched structurally. `src/render/skirmish_shell_chrome.rs:214` and `:227` pre-render primitive rail/combo entries, and `render_primitive_bevel_entry` mirrors the two-ring swap and averaged corners at `src/render/skirmish_shell_chrome.rs:489`. The remaining substitution is color/surface model: Rust emits fixed RGBA colors, while gamemd converts the RGB globals through active DirectDraw loss/shift globals. This can visibly shift the bevel tone even when geometry is correct.

### 3.2 Buttons use verified PCX triplets, but current Rust scales the 30 px art to taller control rects

Active in YR: Yes. `OwnerDraw_Button_00612B70` selects the 30-family for the standard 37/42 px Skirmish button clients. Left/right caps are native blits; the middle is tiled by `FUN_006BA3E0`, not stretched. `WS_DISABLED` forces released `bue_*` art and applies `AlphaBlendRect(..., 0x80)`; `bud_*` is not selected.

Rust status: asset names and disabled alpha are mostly aligned (`src/app_skirmish_shell_render.rs:248`, `:343`, `:354`). The visible mismatch is `push_button_30` sends each cap/middle segment at `rect.h` (`src/app_skirmish_shell_render.rs:370`), so the 30 px retail art is vertically stretched to the full control rect. This is a high-visibility "assembled" look blocker.

### 3.3 Checkbox art is verified and Rust now renders the correct default pair

Active in YR: Yes for the standard five option checkboxes. `OwnerDraw_Checkbox_006163A0` selects `cue_i.pcx`/`cce_i.pcx` when `+0xD9 == 0`, and standard Skirmish init sends only `0xF1`, so variant PCXs stay inactive. Label clicks do not toggle; only the 18x18 icon gate does.

Rust status: current Rust has the right assets in the atlas (`src/render/skirmish_shell_chrome.rs:279`), checkbox layout constants (`src/ui/skirmish_shell/layout.rs:9` and `:210`), icon-only hit test (`src/ui/skirmish_shell/state.rs:361`), and checkbox state bridge (`src/ui/skirmish_shell/state.rs:1233`). This is no longer a major chrome-art substitution.

### 3.4 Trackbar PCX art and geometry are mostly implemented; rail color fidelity remains the blocker

Active in YR: Yes. Standard trackbars use `trakgrip.pcx` plus `trofl/trofm/trofr.pcx`; active width is `128 - 50 - 13 = 65`, thumb width is 12, and mouse mapping uses the verified clamp/bias formula. They do not use `BTN-MINS.SHP` or `BTN-PLUS.SHP`.

Rust status: current Rust loads/renderes these assets (`src/render/skirmish_shell_chrome.rs:281`, `src/app_skirmish_shell_render.rs:724`, `:776`) and implements the active-width/value formulas (`src/ui/skirmish_shell/layout.rs:235`, `src/ui/skirmish_shell/state.rs:252`). The visual risk is the pre-rendered RGBA rail and any plaque tiling/stretch behavior; not a missing asset.

### 3.5 Owner-draw PCX palette and alpha/keying are split correctly for flags but still risky for non-flag controls

Active in YR: Yes. `FUN_0061F210` preloads PCXs through the owner-draw PCX cache; `FUN_006BA140` returns converted cached surfaces. The loader uses embedded PCX palettes converted to active 16-bit display format. Flag static transparency is RGB-magenta keyed by `OwnerDraw_Static_006153E0 -> FUN_006BA580`, not index `0`.

Rust status: flags now use `to_rgba_with_color_key([255,0,255])` (`src/render/skirmish_shell_chrome.rs:464`, `src/assets/pcx_file.rs:102`) and native centered/clipped emission (`src/app_skirmish_shell_render.rs:216`). Non-flag PCXs are still loaded through `render_pcx_entry(..., Some(0))` (`src/render/skirmish_shell_chrome.rs:211`, `:449`). That may punch holes in button/checkbox/arrow art if palette index `0` is not transparent in the retail surface path. This needs per-callback confirmation before being treated as harmless.

### 3.6 Right-panel SHP palettes are verified; current "research candidate" caution is justified

Active in YR: Yes for right-panel/lower chrome. `RightPanel__Draw` uses `SHELL.PAL` for `SDTP/SDBTM/LWSCRN*`, `SHELL2.PAL` for `SDBTNBKGD`, and `SDBTNANM.PAL` for `SDBTNANM` frame 10 when the state branch permits. `MNSCRNL.SHP`, `dbak6440.pcx`, and `dlgsys*.pcx` are not verified replacements for the standard base Skirmish parent/chrome path.

Rust status: the palette split is represented in `src/render/skirmish_shell_chrome.rs:103..160`. The candidate classification at `src/render/skirmish_shell_chrome.rs:341` is correct and should stay conservative.

## 4. Current Rust Implementation Status

| Surface | Status |
|---|---|
| `src/render/skirmish_shell_chrome.rs` | Broadly correct asset list; primitive bevel pre-rendering present; flags use magenta color key; non-flag PCXs still use transparent index `0`; candidate classification is useful. |
| `src/app_skirmish_shell_render.rs` | Buttons, combos, checkboxes, trackbars, flags, dropdown pieces render; button segment height and dropdown/scrollbar styling are still the most visible art substitutions. |
| `src/ui/skirmish_shell/layout.rs` | Constants for combo, checkbox, trackbar geometry match the verified owner-draw contracts. |
| `src/ui/skirmish_shell/state.rs` | Checkbox icon hit, trackbar y gate/value mapping, game-speed inversion, option launch bridge are now present. |
| `src/assets/pcx_file.rs` | Embedded palette parser exists; supports both index alpha and RGB color-key alpha. |

## 5. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Button PCX triplet and disabled alpha | verified | `OwnerDraw_Button_00612B70`; `0x00613254`, `0x006135F3`; Rust scan | native-height implementation check |
| `bud_*` standard-button non-use | verified-negative | `batch_string_anchor_report("bud_")`; button callback filename format | none for standard `0x102` |
| Checkbox default/variant art split | verified | `OwnerDraw_Checkbox_006163A0`; variant writer report | none for standard `0x102` |
| `bst_*` standard-checkbox non-use | verified-negative | `batch_string_anchor_report("bst_")`; checkbox callback | none for standard `0x102` |
| Combo primitive face and arrow PCXs | verified | `OwnerDraw_ComboBox_00617250`; `FUN_00620720`; `FUN_006208F0` | dropdown row/internal paint belongs to slot 2 |
| Trackbar thumb/plaque/rail | verified | `OwnerDraw_Trackbar_0061D950`; assembly `0x0061DE9C`, `0x0061E00C`, `0x0061E204`, `0x0061E269` | live 16-bit pixel capture optional |
| `BTN-MINS/PLUS` standard-trackbar non-use | verified-negative | `SKIRMISH_BTN_MINS_PLUS_USE_SITE`; trackbar callback | none for standard `0x102` |
| Flag PCX palette/key/native clipping | verified | `FUN_004E3560`, `OwnerDraw_Static_006153E0`, `FUN_006BA580`; Rust scan | screenshot validation after final render |
| Right-panel SHP palette split | verified | `RightPanel__Draw @ 0x0072E450`; palette reports | check current SDBTM source crop if not already tested |
| Scrollbar/dropdown art | touched-not-exhausted | PCX preload strings and current Rust dropdown renderer | row/scrollbar exact paint owned by slot 2 |
| Choose Map modal art | deferred | user scope excludes modal except negative facts | slot 4 |
| Parent first-paint composition | deferred | user scope excludes parent composition | slot 1 |

## 6. Open Questions - Final State

- `[RESOLVED] OQ-01 - Are primitive combo faces and trackbar rails placeholders? -> No; they are verified gamemd primitive draw calls, but Rust's RGBA pre-render is a color-model approximation.` (evidence: `FUN_006208F0`, `0x00617893`, `0x0061E204/269`, `src/render/skirmish_shell_chrome.rs:489`)
- `[RESOLVED] OQ-02 - Are button PCX assets verified? -> Yes; normal standard Skirmish buttons use `bue_*30` released and `bde_*30` pressed.` (evidence: `OwnerDraw_Button_00612B70`; `src/app_skirmish_shell_render.rs:248`)
- `[RESOLVED] OQ-03 - Does standard disabled Start use `bud_*` art? -> No; disabled forces released art and alpha `0x80`.` (evidence: `0x00613254`, `0x006135F3`, `batch_string_anchor_report("bud_")`)
- `[RESOLVED] OQ-04 - Are standard checkbox assets verified? -> Yes; `cue_i.pcx` unchecked and `cce_i.pcx` checked.` (evidence: `OwnerDraw_Checkbox_006163A0`; `src/render/skirmish_shell_chrome.rs:279`)
- `[RESOLVED] OQ-05 - Are `bst_*` checkbox assets standard Skirmish art? -> No; they are preload-only in this slice.` (evidence: `batch_string_anchor_report("bst_")`)
- `[RESOLVED] OQ-06 - Are trackbar assets verified? -> Yes; `trakgrip.pcx` and `trofl/trofm/trofr.pcx`, plus primitive rail.` (evidence: `OwnerDraw_Trackbar_0061D950`; `src/app_skirmish_shell_render.rs:724`, `:776`)
- `[RESOLVED] OQ-07 - Are `BTN-MINS/PLUS.SHP` part of standard Skirmish sliders? -> No; they belong to a generic slider path, not `0x102` owner-draw trackbars.` (evidence: `SKIRMISH_BTN_MINS_PLUS_USE_SITE_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-08 - Are flag PCXs decoded with shell palettes? -> No; owner-draw PCXs use embedded palettes; flags key RGB magenta.` (evidence: `FUN_0061F210`, `FUN_006BA140`, `FUN_006BA580`, `src/render/skirmish_shell_chrome.rs:464`)
- `[RESOLVED] OQ-09 - Should `MNSCRNL/dbak6440/dlgsys*` be promoted to base Skirmish chrome? -> No for this slice; they remain research candidates or generic preload assets.` (evidence: right-panel/background reports; `src/render/skirmish_shell_chrome.rs:341`)
- `[RESOLVED] OQ-10 - Exact standard combo popup row owner and scrollbar boundary.` Standard combo popup rows are owned by `ComboDropWin`; real `LISTBOX` rows are owned by `OwnerDraw_ListBox_00618D40`, including Choose Map `0x6EB`/`0x553`. (evidence: `SKIRMISH_COMBODROPWIN_0060D540_FUNCTION_BOUNDARY_GHIDRA_REPORT.md`; `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-11 - Live retail 16-bit screenshots for primitive bevel RGB deltas.` (category: needs-runtime-debugger; reason: static behavior is verified but final display-format capture would quantify color mismatch; next-step-if-pursued: capture crops of combo face and trackbar rail)

Zero-add pass result: re-reading the scoped owner-draw callbacks and current Rust atlas/render/state surfaces added no new standard `0x102` asset families beyond the deferred dropdown/scrollbar internals.

## 7. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Button caps are native PCX blits and middle is tiled; the 30 px art is not vertically stretched to the 37/42 px control rect | `OwnerDraw_Button_00612B70`; `FUN_006BA3E0`; current `push_button_30` uses `rect.h` | mismatch | `src/app_skirmish_shell_render.rs::push_button_30` | draw PCX pieces at native height with correct vertical placement, tile middle only | 800x600 Start/Choose/Back buttons keep 30 px art height and no stretched bevel texture | do not swap in `bud_*` or scale caps to the full control height |
| Primitive combo/trackbar bevels are `FUN_006208F0` surfaces using DirectDraw-converted globals | `FUN_006208F0`; callers `0x00617893`, `0x0061E204/269`; current RGBA primitive atlas | partial approximation | `src/render/skirmish_shell_chrome.rs::render_primitive_bevel_entry` | either document/freeze RGBA approximation or derive colors from the same DirectDraw-format path used by the shell renderer | combo face and trackbar rail crop match retail bevel tones, not only geometry | do not replace these primitives with unknown PCX art |
| Non-flag owner-draw PCXs are converted cached surfaces; transparency is callback-specific, not blanket index-0 alpha | `FUN_0061F210`, `FUN_006BA140`; current `render_pcx_entry(..., Some(0))` | unchecked/maybe mismatch | `src/render/skirmish_shell_chrome.rs::render_pcx_entry` and PCX atlas call sites | audit button/checkbox/arrow PCX alpha: keep all pixels opaque unless a callback proves keyed blit | button/checkbox/arrow atlas previews show no unintended holes from index 0 | do not apply flag magenta key globally to buttons or assume index 0 is transparent |
| `SDBTM.SHP` bottom cap is clipped from native source top rows, not vertically resampled | `SKIRMISH_SDBTM_BOTTOM_CAP_SOURCE_CLIP_GHIDRA_REPORT.md`; current chrome atlas/render path should be checked | verify-current | `src/app_skirmish_shell_render.rs` right-panel bottom draw | ensure source UV height is cropped to visible cap height | 800x600 bottom cap shows top 23 source rows, not compressed 65-row art | do not scale full `SDBTM` into the remainder height |
| Scrollbar/dropdown uses owner-draw arrow/thumb PCXs plus callback-specific track/border behavior | PCX preload and combo/dropdown reports; current `push_dropdown_scrollbar_instances` mixes solid rect/outline with PCXs | partial placeholder | `src/app_skirmish_shell_render.rs::push_dropdown_scrollbar_instances` | replace solid/outline approximations only after slot-2 exact paint handoff | opened combo dropdown scrollbar matches retail arrows, thumb, track, and border | do not invent `dlgsys*`/generic dialog chrome as dropdown scrollbar art |

Proposed Rust test names:

- `skirmish_buttons_preserve_native_30px_pcx_height`
- `skirmish_non_flag_pcx_controls_do_not_key_index_zero_without_evidence`
- `skirmish_primitive_bevel_entries_match_fun_006208f0_geometry_and_colors`
- `skirmish_sdbtm_uses_top_source_crop_not_vertical_resample`
- `skirmish_dropdown_scrollbar_uses_verified_ownerdraw_pieces_only`

## 8. Negative Facts / Do Not Do

- Do not treat `bud_*` as standard disabled Start/Choose/Back art. Active in YR: No for standard `0x102`; evidence: button callback forces `bue_*` plus alpha and `bud_*` xrefs are preload-only.
- Do not use `bst_*` for standard Skirmish checkboxes. Active in YR: No for standard `0x102`; evidence: checkbox callback selects `cue_i/cce_i` and `bst_*` xrefs are preload-only.
- Do not replace primitive combo faces or trackbar rails with PCX/SHP art. Active in YR: Yes for primitive path; evidence: `FUN_006208F0` callers.
- Do not use `BTN-MINS.SHP`/`BTN-PLUS.SHP` for standard Skirmish trackbars. Active in YR: No for standard `0x102`; evidence: owner-draw trackbar PCX/primitive path, generic slider report.
- Do not promote `MNSCRNL.SHP`, `dbak6440.pcx`, `dlgsysa.pcx`, or `dlgsysi.pcx` to base Skirmish shell background/chrome without a new direct standard `0x102` draw proof. Active in YR: No or Conditional outside this base path; evidence: right-panel/background and PCX preload reports.

## 9. Remaining Uncertainty

- Exact combo dropdown listbox/scrollbar row paint is deferred to slot 2, so this report only classifies current obvious placeholder surfaces.
- A live retail 16-bit capture would quantify whether Rust's RGBA primitive bevel colors are perceptibly off.
- Non-flag PCX index-0 alpha needs a focused callback-by-callback check; flags are resolved as RGB-magenta keyed, but buttons/checkboxes/arrows are not proven to use the same key rule.

## 10. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`
  - Replace current Rust status wording that says checkbox/trackbar rendering is missing with: "Current Rust now loads and renders standard checkbox `cue_i/cce_i`, trackbar `trakgrip/trof*`, primitive rail, and option state/input. Remaining art fidelity risks are primitive color model, non-flag PCX alpha, and final pixel screenshot validation."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md`
  - Add follow-up wording: "Rust has a pre-rendered RGBA implementation of the verified `FUN_006208F0` geometry; this should be audited against the active DirectDraw-converted color values before treating bevel pixel color as final."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`
  - Add current-Rust update: "Asset names and disabled alpha are present, but `push_button_30` still scales the cap/middle/right pieces to the full control height; retail blits/tile-fills native 30 px PCX art."

## Sources

- Ghidra read-only decompile/refresh: `OwnerDraw_Button_00612B70`, `OwnerDraw_Checkbox_006163A0`, `OwnerDraw_ComboBox_00617250`, `OwnerDraw_Trackbar_0061D950`, `FUN_006208F0`, `FUN_0061F210`, `FUN_006BA140`.
- Ghidra string-anchor reports: `bue_li30`, `bud_`, `bst_`.
- Ghidra assembly context: `0x0061DE9C`, `0x0061E00C`, `0x0061E1F3`, `0x0061E259`, `0x00613254`, `0x006135F3`.
- Prior reports: `SKIRMISH_PRIMITIVE_BEVEL_FRAME_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_OWNERDRAW_VARIANT_WRITERS_GHIDRA_REPORT.md`, `SKIRMISH_BTN_MINS_PLUS_USE_SITE_GHIDRA_REPORT.md`, `SKIRMISH_SDBTM_BOTTOM_CAP_SOURCE_CLIP_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_LOWER_PCX_DECODE_PALETTE_KEY_PATH_FOR_FLAG_STATICS_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`.
- Rust read-only scan: `src/render/skirmish_shell_chrome.rs`, `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/assets/pcx_file.rs`, `src/sim/game_options.rs`.
