---
title: Skirmish Shell Layout and Assets (Ghidra / PE Resource Research Report)
date: 2026-05-16
---

# Skirmish Shell Layout and Assets - Ghidra / PE Resource Research Report

## Scope

This report investigates how the original Yuri's Revenge Skirmish client screen
is visually built: dialog template, control rectangles, slot table columns,
control classes/styles, map preview placement, and what is known about art assets.

This is a visual/client follow-up to
`SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`.

Active in YR: Yes. The resource ID `0x102` is the Skirmish dialog used by the
YR shell path; its control IDs are referenced by `FUN_006040B0` for tooltips and
by the session/skirmish setup code path.

Confidence:

- High for dialog size, font, control IDs, control rectangles, labels, and
  Win32 control classes/styles. These come directly from the `RT_DIALOG`
  resource embedded in `gamemd.exe`.
- High that no shell bitmaps are embedded as Windows bitmap resources: the PE
  resource table contains cursor, icon, menu, dialog, group cursor/icon, and
  version resources, but no `RT_BITMAP`.
- Medium for exact owner-draw art asset names. The dialog template proves where
  owner-drawn controls are, but tracing every draw branch to specific MIX assets
  remains open.

## Prior Work Checked

Relevant prior reports:

- `GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md`
- `SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`
- `SPAWN_POINT_ASSIGNMENT_SYSTEM.md`
- `HOUSE_CREATION_COLOR_SYSTEM.md`
- `SESSIONCLASS_GHIDRA_REPORT.md`

This pass resolves the previous open question "Exact Win32 dialog resource pixel
layout was not extracted." The layout is now extracted from the PE resource.

## 1. Dialog Template Identity

Evidence: PE `RT_DIALOG` resource table in `gamemd.exe`.

Skirmish dialog:

| Field | Value |
|---|---|
| Resource ID | `258` / `0x102` |
| Template kind | `DIALOGEX` |
| Language | `0x409` |
| Dialog rect | `x=0, y=0, w=533, h=369` dialog units |
| Item count | `72` |
| Font | `MS Sans Serif`, 8 pt |
| Style | `0x40000040` |
| Extended style | `0x00000000` |
| Title | empty |

Important detail: the shell layout is not described by SHP files. The base layout
is a compiled Win32 dialog resource. SHPs/other assets only skin or fill parts
that the dialog procedure owner-draws.

## 2. Overall Screen Structure

The Skirmish dialog is 533x369 dialog units. It is divided into:

- left/middle slot table: player/AI rows and per-slot selections.
- right-side map panel: title, map preview, scenario/game-type labels, Start,
  Choose Map, Back buttons.
- lower options area: checkboxes and sliders.
- bottom status strip.

Key right-side controls:

| ID | Rect | Class | Style | Title / role |
|---|---:|---|---|---|
| `0x694` | `(425,1,108,10)` | `STATIC` | `0x50020001` | `GUI:SkirmishGame` |
| `0x468` | `(429,23,96,69)` | `STATIC` | `0x50000004`, ex `0x20` | map thumbnail target |
| `0x6EC` | `(432,103,90,10)` | `STATIC` | `0x50000201` | game type text, starts as `GUI:None` |
| `0x5A8` | `(432,116,90,20)` | `STATIC` | `0x50000001` | scenario/map label, starts as `GUI:None` |
| `0x617` | `(425,149,108,23)` | `BUTTON` | `0x5000000B` | `GUI:StartGame` |
| `0x5AA` | `(425,176,108,23)` | `BUTTON` | `0x5000200B` | `GUI:ChooseMap` |
| `0x5C0` | `(425,346,108,23)` | `BUTTON` | `0x5000000B` | `GUI:Back` |

Important style detail:

- Buttons use low style bits `0x0B`, which is `BS_OWNERDRAW`.
- The map thumbnail control is a transparent static placeholder; actual preview
  and start markers are drawn by code, not by a dialog resource bitmap.

## 3. Slot Table Layout

Column headers:

| Header ID | Rect | Text |
|---|---:|---|
| `0x796` | `(39,21,97,10)` | `GUI:Players` |
| `0x791` | `(191,21,73,10)` | `GUI:Side` |
| `0x792` | `(283,21,42,10)` | `GUI:Color` |
| `0x793` | `(325,21,34,10)` | `GUI:StartPosition` |
| `0x794` | `(363,21,34,10)` | `GUI:Team` |

Rows use a 16-dialog-unit vertical stride:

| Row | Y | Player/AI control | Flag static | Side combo | Color combo | Start combo | Team combo |
|---:|---:|---|---|---|---|---|---|
| 0 | `36` | `0x6A0` edit `(38,36,100,14)` | `0x6DA` `(150,36,32,12)` | `0x6A1` `(191,36,78,74)` | `0x6A2` `(282,36,29,73)` | `0x6A3` `(324,36,25,73)` | `0x76D` `(364,36,25,73)` |
| 1 | `52` | `0x50B` combo `(39,52,100,74)` | `0x6DB` `(150,52,32,12)` | `0x510` `(191,52,78,74)` | `0x522` `(282,52,29,73)` | `0x6A4` `(324,52,25,73)` | `0x76E` `(364,52,25,73)` |
| 2 | `68` | `0x50E` combo `(39,68,100,74)` | `0x6DC` `(150,68,32,12)` | `0x513` `(191,68,78,74)` | `0x523` `(282,68,29,73)` | `0x6A5` `(324,68,25,73)` | `0x76F` `(364,68,25,73)` |
| 3 | `84` | `0x516` combo `(39,84,100,74)` | `0x6DD` `(150,84,32,12)` | `0x51E` `(191,84,78,74)` | `0x524` `(282,84,29,73)` | `0x6A6` `(324,84,25,73)` | `0x770` `(364,84,25,73)` |
| 4 | `100` | `0x51A` combo `(39,100,100,74)` | `0x6DE` `(150,100,32,12)` | `0x514` `(191,100,78,74)` | `0x525` `(282,100,29,73)` | `0x6A7` `(324,100,25,73)` | `0x771` `(364,100,25,73)` |
| 5 | `116` | `0x51B` combo `(39,116,100,74)` | `0x6DF` `(150,116,32,12)` | `0x51F` `(191,116,78,74)` | `0x526` `(282,116,29,73)` | `0x6A8` `(324,116,25,73)` | `0x772` `(364,116,25,73)` |
| 6 | `132` | `0x51C` combo `(39,132,100,74)` | `0x6E0` `(150,132,32,12)` | `0x520` `(191,132,78,74)` | `0x527` `(282,132,29,73)` | `0x6AA` `(324,132,25,73)` | `0x773` `(364,132,25,73)` |
| 7 | `148` | `0x51D` combo `(39,148,100,74)` | `0x6E1` `(150,148,32,12)` | `0x521` `(191,148,78,74)` | `0x528` `(282,148,29,73)` | `0x6AB` `(324,148,25,73)` | `0x774` `(364,148,25,73)` |

Tiny details that matter:

- Row 0's player cell is an `EDIT` control, not the AI/player-type combo used
  by rows 1-7. This is the local player's name field.
- Rows 1-7 use combo boxes in the player column; this is where AI/empty slot
  choices are represented.
- The flag column is eight `STATIC` controls, 32x12 dialog units, with
  `WS_EX_TRANSPARENT` (`0x20`). These are placeholders for country flag rendering.
- Start and Team are narrow 25x73 combo boxes; they are not labels or buttons.

## 4. Control Classes and Styles

Important control style patterns:

| Surface | Class | Typical style | Meaning |
|---|---|---|---|
| Start/Back/Choose buttons | `BUTTON` | `0x5000000B` | visible child owner-drawn button |
| Checkboxes | `BUTTON` | `0x50000003` | auto-checkbox style |
| Player name | `EDIT` | `0x50000080` | edit box, auto-horizontal behavior |
| Chat/input in host screens | `EDIT` | `0x50011004` | multiline/bordered shell input variant |
| Slot combo boxes | `COMBOBOX` | `0x50000213` | dropdown-list, owner-draw fixed, has strings |
| Narrow color/start/team combos | `COMBOBOX` | `0x50200213` | same plus vertical-scroll bit |
| Sliders | `msctls_trackbar32` | `0x50000018` | Win32 common-control trackbar |
| Map preview | `STATIC` | `0x50000004`, ex `0x20` | transparent static placeholder |
| Flag cells | `STATIC` | `0x50000005`, ex `0x20` | transparent static placeholders |

Owner-draw conclusion:

- The slot combo boxes are not plain Windows-looking combos. The `0x213` low
  style includes owner-draw fixed and has-strings behavior.
- The main shell buttons are owner-drawn (`BS_OWNERDRAW`).
- Therefore visual parity needs the dialog template plus owner-draw code/art,
  not the resource template alone.

## 5. Lower Options Area

Left-side checkboxes:

| ID | Rect | Text |
|---|---:|---|
| `0x54E` | `(48,176,100,10)` | `GUI:ShortGame` |
| `0x693` | `(48,193,100,10)` | `GUI:MCVRepacks` |
| `0x696` | `(48,210,100,10)` | `GUI:CratesAppear` |
| `0x69A` | `(48,228,103,10)` | `GUI:SuperWeaponsAllowed` |

Right-side labels/sliders/options:

| ID | Rect | Text / role |
|---|---:|---|
| `0x699` | `(201,176,60,10)` | `GUI:GameSpeed` |
| `0x529` | `(269,176,85,13)` | game-speed trackbar |
| `0x69B` | `(201,193,60,10)` | `GUI:Credits` |
| `0x511` | `(269,193,85,13)` | credits trackbar |
| `0x69C` | `(201,210,60,10)` | `GUI:UnitCount` |
| `0x50C` | `(269,210,85,13)` | unit-count trackbar |
| `0x69D` | `(201,227,166,11)` | `GUI:BuildOffAlly` |

Notable difference from host screens:

- The Skirmish resource does not include chat list/input controls.
- The Skirmish resource does not include the host/guest page button.
- The Skirmish resource has 72 controls; Host has more controls because it adds
  chat/list and network-only surfaces.

## 6. Map Preview and Start Marker Drawing

Evidence: `DrawStartPositions` at `0x00640710`.

The Skirmish dialog's map preview control is ID `0x468` at `(429,23,96,69)`.
The same ID is used by Host/Guest/Choose Map dialogs.

`DrawStartPositions` behavior:

- Accepts an `HWND` and validates the window rectangle.
- Calls `GetDlgItem(hwnd, 0x468)` to find the map thumbnail control.
- Computes scaled map-to-thumbnail coordinates.
- Reads scenario visible map bounds from `ScenarioClass+0x112C` through
  `ScenarioClass+0x1138`.
- Reads start marker count from `ScenarioClass+0x113C`.
- Only draws markers when the count is `1..8`.
- Reads marker coordinates from `ScenarioClass+0x1140 + i*8` and
  `ScenarioClass+0x1144 + i*8`.
- Applies draw offsets of `-9` X and `-6` Y before drawing the marker shape.
- Draws the numeric label as `i + 1`.

This confirms that the original client lets the user pick numbered starts from
the row combo boxes while seeing matching numbered markers on the 96x69 map
thumbnail.

## 7. Host / Guest Comparison

Host dialog `0xBC`:

- Same overall rect: `533x369`.
- Same title/map/right button column pattern.
- Slot table starts at header y=7 and first row y=22.
- Player column x=49, flags x=28/160 depending marker type, side x=201,
  color x=294, start x=334, team x=374.
- Includes chat output list `0x53F` at `(7,227,405,98)` and chat input `0x540`
  at `(7,328,405,12)`.

Guest dialog `0xBD`:

- Same slot table geometry as Host.
- Uses Accept instead of Start Game.
- Uses Guest tooltip strings for guest-specific controls.

Skirmish differs:

- Slot table is shifted down: headers at y=21 and first row y=36.
- Player column x=39, flag x=150, side x=191, color x=282, start x=324,
  team x=364.
- Chat controls are absent.
- Options occupy more of the lower left/middle panel.

## 8. Asset Findings

PE resource table finding:

- Resource types present: cursor, icon, menu, dialog, group cursor, group icon,
  version.
- No `RT_BITMAP` resources were present.

Implication:

- The original shell layout is embedded in `gamemd.exe`.
- Shell art is not embedded as Windows bitmap resources.
- Any SHP/PCX/palette art used by owner-draw controls must come from normal
  retail asset loading through MIX archives, or the control is drawn with
  primitives/text by the code.

Binary string findings relevant to shell/loading/UI assets:

- `GLSLMD.SHP`
- `GLSSMD.SHP`
- `GLSMD.PAL`
- `DROPDOWN.SHP`
- `DROPUP.SHP`
- `PROGBAR2.SHP`

Confidence caveat:

- These strings prove those assets are known to the binary, but this pass did
  not prove that each is used by the Skirmish dialog specifically. `DROPDOWN.SHP`
  / `DROPUP.SHP` are from dropship UI code, not automatically the shell combo
  dropdown art. `PROGBAR2.SHP` has a string xref in `FUN_00598960`, but that is
  not yet tied to Skirmish setup.

What is verified for Skirmish visuals:

- layout and control geometry.
- owner-draw button and combo style.
- map thumbnail control and numbered start marker draw function.
- transparent static placeholders for flag/map drawing.

What remains unverified:

- exact SHP/PCX filenames for button frames, combo frames, checkbox checks,
  flag icons, and list/dropdown chrome.
- exact palette used for each shell surface.
- whether some control visuals are custom drawn with GDI primitives instead of
  SHPs.

## 9. Rebuild Feasibility

It is absolutely possible to rebuild the client, but there are two levels:

Functional/structural parity:

- High feasibility.
- We now know the exact Skirmish slot layout and IDs.
- We can reproduce the same table using egui or a custom shell renderer.
- We can feed the same data model: player name, AI slot, country, color, start,
  team, options, map preview/start markers.

Visual/pixel parity:

- Medium to high effort.
- We need owner-draw asset tracing for each control type.
- The dialog template gives geometry, but not the painted pixels.
- Because the PE has no bitmap resources, the art lookup must be traced through
  asset loading and draw handlers, or matched by inspecting MIX assets visually.

Recommended implementation target:

- First rebuild a structurally faithful 533x369-equivalent Skirmish screen with
  the exact slot table and map preview marker layout.
- Keep a style layer separate, so later owner-draw asset findings can replace
  egui/default visuals without changing the slot-data logic.

## 10. Current Rust Gap

Current `src/ui/main_menu.rs` is much simpler than the original:

- no 8-row slot table.
- no per-slot start/team controls.
- no map thumbnail marker panel.
- no color selection UI.
- no AI slot count/type controls.
- player/AI country settings exist in data but are not fully surfaced.

Existing useful pieces:

- `src/map/waypoints.rs` parses start waypoints 0-7.
- `src/ui/main_menu.rs` already has `StartPosition`.
- `src/app_skirmish.rs` can spawn MCVs from selected waypoints.
- `src/app_spawn_pick.rs` can draw full-map waypoint markers, though that is
  not original shell UX.

## Open Questions

1. Which exact owner-draw functions paint Skirmish combo rows, button frames,
   checkboxes, and flags?
2. Which exact MIX assets and palettes are used by those owner-draw paths?
3. Does Skirmish dynamically hide/disable rows 2-7 based on AI player count, or
   are all rows present and populated with "None"/closed choices?
4. What is the exact DLU-to-pixel conversion and final placement in 640x480,
   800x600, and higher shell resolutions?
5. Does `DrawStartPositions` use a dedicated marker SHP or a generic shape
   surface loaded elsewhere? The offsets and label behavior are known; the
   concrete marker asset remains unresolved.

## Source Evidence

Local binary/resource evidence:

- `gamemd.exe` PE resource table: `RT_DIALOG` resource `0x102`.
- `gamemd.exe` PE resource table: no `RT_BITMAP` resource type.
- Dialog template parse for resource `0x102`, language `0x409`.
- Dialog template parse for Host `0xBC`, Guest `0xBD`, WOL/new-game variants
  `0xC2` / `0xC9`, and Choose Map `0x6B`.

Ghidra functions rechecked:

- `FUN_006040B0` - dialog control ID to tooltip key dispatcher.
- `DrawStartPositions` at `0x00640710`.
- `SimpleDialogControl::OwnerDraw::Constructor` at `0x00624110`.
- `DialogControl::OwnerDraw::Constructor` at `0x00624130`.
- `OptionsClass::ShowInGameDialog` at `0x004E1D00`.
- `FUN_00623120` - shell/modal loop service path.
- `RulesClass::ReadMultiplayerDialogSettings` at `0x00671EA0`.

Binary strings checked:

- `STT:SkirmishComboAIPlayer`, `STT:SkirmishComboCountry`,
  `STT:SkirmishComboColor`
- `STT:HostComboStart`, `STT:HostComboTeam`
- `GUI:Players`, `GUI:Side`, `GUI:Color`, `GUI:StartPosition`, `GUI:Team`
- `GUI:SkirmishGame`, `GUI:StartGame`, `GUI:ChooseMap`, `GUI:Back`
- UI asset strings including `GLSLMD.SHP`, `GLSSMD.SHP`, `GLSMD.PAL`,
  `DROPDOWN.SHP`, `DROPUP.SHP`, `PROGBAR2.SHP`
