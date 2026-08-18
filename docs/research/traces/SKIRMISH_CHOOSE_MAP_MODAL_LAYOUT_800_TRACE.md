# Skirmish Choose Map Modal 0x6B Layout 800x600 Trace

Scenario: standard offline Yuri's Revenge Skirmish at 800x600, click `Choose Map`, then compare dialog `0x6B` layout positions only: dialog/screen rect, `0x6EB`, `0x553`, `0x6C5`, `0x583`, `0x5C0`, title/statics/status/preview, row height, and content/scrollbar rects. Row text contents and accept/cancel behavior are out of scope except where needed for layout liveness.

Verdict rule: PASS requires literal numerical equality for this scenario. If both sides were not computed, the stage is UNCHECKED.

## Sources Used

- Gamemd active path and final rect table: `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md`.
- Gamemd resource inventory: `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`.
- Gamemd owner-draw listbox row/scrollbar mechanics: `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`.
- Gamemd preview/modal callback boundary: `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`.
- Current Rust source: `src/ui/skirmish_shell/layout.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/app_skirmish_shell_render/modals.rs`, `src/app_skirmish_shell_render/text.rs`, `src/ui/skirmish_shell/state/hit_test.rs`.

No Ghidra tools were used in this run; existing read-only Ghidra-backed reports were sufficient.

## Pipeline

`0x102 Choose Map 0x5AA click` -> `gamemd hides setup and creates fullscreen shell dialog 0x6B` -> `ResizeShellChildControl_0060C0C0 routes child controls` -> `OwnerDraw_ListBox_00618D40 sets list item/scrollbar geometry` -> `0x6B WM_PAINT draws shell background, controls, current preview`.

Rust pipeline: `SkirmishShellAction::ChooseMap` -> `open_choose_map_modal` -> `compute_fixed_800_choose_map_modal_layout` -> modal-only render instances/text -> listbox helpers.

## Stage Results

| Stage | Gamemd 800x600 | Current Rust 800x600 | Verdict |
|---|---:|---:|---|
| Entry opens modal | `0x5AA -> 0x005E68A0 -> resource 0x6B`, active in standard YR | `app.rs` routes `ChooseMap` to `open_choose_map_modal` | PASS |
| Parent/setup ownership | setup hidden; `0x6B` modal owns screen | renderer returns modal-only instances when chooser active | PASS |
| Dialog/screen rect | runtime parent `MoveWindow(0,0,800,600)`, not a centered `533x369` pixel modal | `dialog=(133,115,533,369)` | FAIL |
| Mode list `0x6EB` | `(116,127,195,343)` | `(210,193,130,211)` | FAIL |
| Map list `0x553` | `(338,127,195,343)` | `(358,193,130,211)` | FAIL |
| Use Map `0x6C5` | `(644,199,156,42)` | `(558,237,108,23)` | FAIL |
| Create Random Map `0x583` | `(644,241,156,42)` | `(558,264,108,23)` | FAIL |
| Cancel `0x5C0` | `(644,535,156,42)` | `(558,461,108,23)` | FAIL |
| Preview `0x468` | `(644,37,144,112)` | `(561,138,96,69)` | FAIL |
| Title `0x694` | `(635,3,162,16)` | `(558,116,108,10)` | FAIL |
| Select Engagement static | `(120,33,386,20)` | `(213,135,257,12)` | FAIL |
| Game Type heading | `(116,98,195,16)` | `(210,175,130,10)` | FAIL |
| Game Map heading | `(338,98,195,16)` | `(358,175,130,10)` | FAIL |
| Status/help `0x695` | `(10,579,455,20)` | `(135,470,303,12)` | FAIL |
| List row height | `GAME.FNT 17 + 2 = 19` standard inferred row height | `CHOOSE_MAP_LIST_ROW_H = 19` | PASS |
| Visible full rows | `floor(343 / 19) = 18`, 1 px remainder | `floor(211 / 19) = 11`, 2 px remainder | FAIL |
| Mode-list scrollbar | 9 stock modes; 9 <= 18, no scrollbar | 9 stock modes; 9 <= 11, no scrollbar | PASS |
| Map-list scrollbar/content | YR `MISSIONSMD.PKT` has 161 resolved standard-visible stock maps, so overflow. Content `(338,127,175,343)`, scrollbar `(513,127,20,343)` | local filtered map list is overflow-sized in normal installs; helper gives content `(358,193,110,211)`, scrollbar `(468,193,20,211)` | FAIL |
| Modal status hover key mapping | `0x6B` branch maps `0x6EB`, `0x553`, `0x468`, `0x6C5`, `0x583`, `0x5C0` to `STT:Scenario*` keys | Rust maps the same keys in `status_help_key_for_choose_map_hover` | PASS |
| Exact first-frame pixel screenshot | Not captured in this trace | Not captured in this trace | UNCHECKED |
| Native previous-WndProc mouse selection details | Out of scope for layout; ordinary selection path not rederived here | Rust has direct helpers | UNCHECKED |

## Main Findings

### FAIL - Current Rust Centers Raw Resource Coordinates Instead Of Using 0x6B Shell Helpers

Gamemd does not draw a centered `533x369` pixel modal. Resource `0x6B` is DIALOGEX dialog units, the parent window is moved to fullscreen `(0,0,800,600)`, and each child passes through `ResizeShellChildControl_0060C0C0`. The latest verified 800x600 final table puts the listboxes at `(116,127,195,343)` and `(338,127,195,343)`, while Rust puts them at `(210,193,130,211)` and `(358,193,130,211)`.

Player-visible effect: the chooser body is shifted downward/right and compressed; the player sees the listboxes, headings, preview, status strip, and buttons in the wrong places.

Rust evidence: `compute_choose_map_modal_layout` centers `CHOOSE_MAP_MODAL_W/H` and uses raw child offsets in `src/ui/skirmish_shell/layout.rs`.

Gamemd evidence: `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md` lines 25-46.

### FAIL - Right Panel Buttons Are In The Wrong Pixel Rects

Gamemd final button rects are Use Map `(644,199,156,42)`, Create Random Map `(644,241,156,42)`, Cancel `(644,535,156,42)`. Rust renders and hit-tests them at `(558,237,108,23)`, `(558,264,108,23)`, and `(558,461,108,23)`.

Player-visible effect: the obvious shell buttons are not where retail places them; Cancel is especially wrong because retail puts it on the bottom right tile row.

Rust evidence: `src/ui/skirmish_shell/layout.rs` `use_map_button`, `create_random_map_button`, `cancel_button`; `src/app_skirmish_shell_render/modals.rs` draws those rects.

Gamemd evidence: `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md` lines 38-40 and helper evidence lines 50-56.

### FAIL - Preview, Title, Headings, And Status Strip Use Wrong Final Rects

Gamemd final rects: preview `(644,37,144,112)`, title `(635,3,162,16)`, Select Engagement `(120,33,386,20)`, Game Type `(116,98,195,16)`, Game Map `(338,98,195,16)`, status `(10,579,455,20)`. Rust uses centered-modal local positions: preview `(561,138,96,69)`, title `(558,116,108,10)`, Select Engagement `(213,135,257,12)`, Game Type `(210,175,130,10)`, Game Map `(358,175,130,10)`, status `(135,470,303,12)`.

Player-visible effect: the modal reads like a smaller floating panel instead of the retail fullscreen shell composition.

### FAIL - List Capacity And Overflow Geometry Are Wrong

Gamemd listboxes are `195x343`; with standard row height `19`, they expose 18 full rows. Rust listboxes are `130x211`; with the same row height, they expose 11 full rows. For an overflowing standard Battle map list, gamemd content/scrollbar rects are `(338,127,175,343)` and `(513,127,20,343)`, while Rust computes `(358,193,110,211)` and `(468,193,20,211)`.

Player-visible effect: fewer rows are visible, the scrollbar is much shorter and shifted, and lower-row hit testing cannot be pixel-equivalent.

### PASS - Row Height Constant Now Matches The Resolved Standard Owner-Draw Formula

The owner-draw listbox report resolves standard row height as active shell font height `17` plus `2`, yielding `19`. Rust now uses `CHOOSE_MAP_LIST_ROW_H = 19`. This does not save layout parity because the containing list rects are wrong.

## Adjacent Findings

- Create Random Map command behavior is outside this trace. Existing reports mark it as a real command path; this trace only compares button placement.
- Exact RGB/screenshot parity remains UNCHECKED because no native and Rust screenshots were captured in this slot.
- Preview refresh timing is out of scope, but existing evidence says passive map-list highlight does not refresh the preview; Use Map commit is the normal refresh boundary.

## Verdict Tally

PASS: 5 | FAIL: 14 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Status

COMPLETE for the requested layout-position trace. The layout verdict is FAIL: current Rust still uses a centered raw-resource modal model, while active gamemd uses fullscreen `0x6B` shell helper routing.
