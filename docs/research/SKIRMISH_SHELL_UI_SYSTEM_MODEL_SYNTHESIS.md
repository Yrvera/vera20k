# Skirmish Shell UI System Model Synthesis

Date: 2026-05-22

Scope: standard offline Yuri's Revenge Skirmish setup shell, dialog `0x102`: shell/chrome composition, owner-draw controls, preview surface, row controls, option packing, and immediate start handoff. Non-scope: online/WOL lobbies, full map chooser internals, post-launch gameplay parity beyond the launch-session bridge, and exact retail screenshot color matching.

Output type: model-synthesis with implementation-safe islands. This is not a Rust patch plan.

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---|---|---|---|
| Offline Skirmish uses dialog `0x102`, `FUN_006AE2C0` launcher, `FUN_006AE3F0` proc, and Start/Back result codes `0x617/0x5C0`. | `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`; `SKIRMISH_PREVIEWPACK_CHANNEL_ORDER_AND_MENU_CALLER_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Common shell paint runs before Skirmish-specific preview paint; `WM_PAINT` then calls `DrawStartPositions` if preview object and child `0x468` are eligible. | `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`; preview caller report | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Final 640x480 formula is parent `(0,0,640,480)`, right panel at x=472, preview `(484,37,144,112)`, Start `(484,241,156,42)`, Choose `(484,283,156,42)`, Back `(484,409,156,42)`. | `SKIRMISH_SHELL_640X480_FINAL_VISIBLE_LAYOUT_TRACE.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| 640 parent background is `MNSCRNS.SHP` plus lower strip `LWSCRNS.SHP`; older `MNSCRNL` wording is stale for final 640 visible layout. | 640 trace vs older active-render report | confirmed/stale correction | high | yes | IMPLEMENTATION_SAFE |
| 800 path uses `MnScrnLCoopGameSetup.shp` plus `MnScrnLCoopGameSetup.PAL`; fresh >800 Skirmish does not draw a parent-background SHP because the alternate pointer is not loaded and null-draws. | active-render, high-res hosting reports, GT800 pointer lifecycle report, targeted reconciliation | confirmed | high for parent-background decision; medium for aggregate screenshot parity | yes | IMPLEMENTATION_SAFE: skip parent-background SHP above 800 |
| Start/Choose/Back owner-draw buttons use `bue_*30.pcx` unpressed and `bde_*30.pcx` pressed; `bud_*` is preload-only for this path. | active-render, ownerdraw reports | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Button text must use the caller pixel contract, not just full button rect plus y offset. | `SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`; current Rust scan | confirmed-current gap | high | yes | IMPLEMENTATION_SAFE gap |
| `[PreviewPack]` selected-map preview bytes are row-major RGB triples; current Rust decode matches this. | `SKIRMISH_PREVIEWPACK_CHANNEL_ORDER_AND_MENU_CALLER_GHIDRA_REPORT.md`; `src/map/preview.rs` scan | confirmed-current | high | yes | IMPLEMENTATION_SAFE |
| Live `STARTBUT.SHP` marker overlays are drawn by `DrawStartPositions`, but should not be synthesized merely from gameplay waypoints or a decoded PreviewPack. | preview caller, header/defaults, retail preview census | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Five checkboxes use `cue_i.pcx`/`cce_i.pcx`, toggle only from the 18x18 icon, and Start rereads `BM_GETCHECK`. | checkbox/trackbar report | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Three trackbars use owner-draw primitive rails, `trakgrip.pcx`, optional `trof*` numeric plaques, active width 65 for 128 px controls, and game speed visual `6 - stored`. | checkbox/trackbar report | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Collapsed combos paint a 24 px face, reserve 20 px for arrow, use primitive frame plus `dnarrow*.pcx`, and color swatches fill `(2,2,20,20)` for 44 px color combos. | combo geometry report | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Offline AI row type combo order is item data `-1,2,1,0` for None/Easy/Normal/Hard; difficulty item-data meaning is `0=Hard,1=Normal,2=Easy`. | AI row state report | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Selected-map start count hides/closes AI rows beyond capacity; Start rescans active rows and validates map capacity, minimum players, and same-team rejection. | row visibility report; start handoff report | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Start handoff packs map mirrors, local node, seven AI row arrays, compact launch table, random assignment, checkboxes, trackbars, and forced flags before modal exit. | start handoff report | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Current Rust has a dev-gated experimental shell and partial native-shaped `SkirmishLaunchSession`; it is not yet a default/full retail shell. | `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish.rs` scan | confirmed-current | high | n/a | IMPLEMENTATION_SAFE gap diagnosis |

## Current Model

The standard offline Skirmish setup screen is a fullscreen-hosted Win32 shell dialog `0x102`. Common shell setup subclasses parent and child controls, assigns owner-draw records, computes shell/right-panel layout, then paints parent chrome before Skirmish code draws the map preview through child `0x468`.

The visual model is not an egui form. It is a retail-asset shell: right-panel SHPs, width-specific parent/lower-strip backgrounds, owner-draw PCX buttons, PCX/primitive checkboxes and trackbars, primitive combo faces with arrow PCXs, flag PCXs, bitfont text, decoded map preview surface, and conditional `STARTBUT.SHP` marker overlays.

The launch model is also not a direct spawn command. Start disables the button, validates rows/map/team/mode acceptance, packs session globals and node/AI arrays, tears down preview state, then exits the modal loop. Scenario init and spawn placement happen later from that packed state.

## Implementation-Safe Facts

- Use the 640 and 800 layout formulas from the final trace and active render reports. Current Rust layout tests are broadly aligned for key rects.
- Keep PreviewPack decode as RGB, row-major, and fail on bad byte counts.
- Render stock selected-map previews from decoded map PreviewPack data; do not infer overlay markers from gameplay waypoints.
- Model checkbox and trackbar behavior at the UI-state level; no Win32 subclass emulation is needed.
- Preserve AI row item-data semantics. The Rust `AiDifficulty::as_i32()` meaning currently differs from the stock row combo item data and needs care if used as a native handoff value.
- Preserve Start validation/packing as a UI/session contract before scenario init.

## Doc-Patch-Ready Facts

- Older `MNSCRNL.SHP` final-640 wording should be patched to `MNSCRNS.SHP` where it claims final visible 640 parent background.
- Older "Rust starts at most two MCVs" trace wording is stale for the current `SkirmishLaunchSession` path. Current Rust now creates launch houses and can spawn per active slot, but still lacks native random assignment, full mode callbacks, fallback starts, and start-unit budget behavior.
- `SKIRMISH_PREVIEWPACK_DECODE_FORMAT_GHIDRA_REPORT.md` is superseded by the channel-order follow-up for RGB confidence.

## Cross-Doc Conflicts

- Background asset naming conflict: older active-render text says `MNSCRNL.SHP` for the 640 parent path; the newer 640x480 final visible trace and current Rust both use `MNSCRNS.SHP`. Treat the older wording as stale unless rechecked with a retail capture.
- Several current-Rust status sections predate the new launch-session code. Use source scan plus this synthesis for current status, not old trace deltas alone.
- Ghidra spot-check attempt in this session could not decompile numeric or symbol names for `0x006AE3F0`, `0x006ACEE0`, `0x00622B50`, or `0x00640710`; the model therefore relies on the prior live Ghidra reports for those claims.

## Needs Re-Investigation

- Full >800 retail-pixel screenshot comparison for aggregate composition; parent-background SHP selection itself is resolved.
- Full combo dropdown/listbox row paint, beyond collapsed geometry.
- Exact `STARTBUT.SHP` overlay projection and numeric label clipping if live overlays are enabled.
- Full selected `MPModes` post-launch callbacks and start-unit budget generation.
- Exact mode-specific MCV/start-unit placement fallback, including `MCVDeploy` queue behavior.

## Do-Not-Implement Notes

- Do not make the experimental shell the default until the missing visible controls and launch validations are implemented.
- Do not use egui widgets as the parity path for this shell.
- Do not implement `BTN-MINS.SHP`, `BTN-PLUS.SHP`, or `bst_*` art for standard offline Skirmish checkboxes/trackbars.
- Do not decode PreviewPack as BGR.
- Do not cap Skirmish launch/spawn to two players.
- Do not treat `House+0x1605C` as start position; reports identify it as team/adjunct, while start is the separate start field.

## Source Ledger

- `skirmish-ui/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_SHELL_640X480_FINAL_VISIBLE_LAYOUT_TRACE.md`
- `skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_PREVIEWPACK_CHANNEL_ORDER_AND_MENU_CALLER_GHIDRA_REPORT.md`
- `skirmish-ui/SCENARIO_PREVIEW_HEADER_DEFAULTS_AND_DUSTBOWL_SOURCE_PATH_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_PLAYER_AI_ROW_VISIBILITY_ENABLE_RULES_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_AI_ROW_STATE_LABELS_AND_ITEM_DATA_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_START_TO_FULL_INIT_SPAWN_TRACE.md`
- `skirmish-ui/SKIRMISH_GT800_BACKGROUND_POINTER_LIFECYCLE_GHIDRA_REPORT.md`
- `skirmish-ui/SKIRMISH_GT800_BACKGROUND_TARGETED_TRACE_RECONCILIATION.md`
- Rust scan: `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`, `src/skirmish_launch.rs`, `src/app_skirmish.rs`, `src/app_init.rs`, `src/map/preview.rs`.

## Classification

Implementation-safe for 640/800 shell layout formulas, the fresh >800 no-parent-background-SHP decision, core shell paint order, PreviewPack RGB decode, button/checkbox/trackbar/collapsed-combo geometry, AI-row item data, and Start handoff validation/packing shape. Investigation-blocked for aggregate >800 screenshot parity, live marker overlay projection if enabled, dropdown row paint, and post-launch mode-specific spawn/start-unit parity.
