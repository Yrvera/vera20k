---
title: Skirmish and Multiplayer Start-Position UX (Ghidra Research Report)
date: 2026-05-16
---

# Skirmish and Multiplayer Start-Position UX - Ghidra Research Report

## Scope

This report investigates how `gamemd.exe` exposes player start positions in the
pre-game shell UX, especially for Skirmish / custom multiplayer setup. It extends
the existing spawn assignment research by focusing on the visible menu controls,
dialog IDs, labels, persistence, and how those choices flow into scenario start
assignment.

Active in YR: Yes. The investigated functions are the YR shell dialog tooltip
dispatcher, skirmish settings reader/writer, scenario house creation, map preview
start-marker drawing, and scenario start assignment.

Confidence: High for the presence of Skirmish start/team controls, Host/Guest
start controls, persisted slot triples, house start-location fields, and waypoint
assignment behavior. Medium for exact pixel layout of the Win32 dialog resources,
because the dialog templates themselves were not extracted in this pass.

## Prior Work Checked

Existing relevant reports:

- `SPAWN_POINT_ASSIGNMENT_SYSTEM.md`
- `HOUSE_CREATION_COLOR_SYSTEM.md`
- `SESSIONCLASS_GHIDRA_REPORT.md`
- `LOBBY_SESSION_HOUSE_CREATION_GHIDRA_REPORT.md`
- `GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md`
- `GAME_START_INITIALIZATION.md`

Important prior-doc conflict resolved here:

- `SESSIONCLASS_GHIDRA_REPORT.md` says Skirmish `SlotXX` stores side, color,
  start location.
- `LOBBY_SESSION_HOUSE_CREATION_GHIDRA_REPORT.md` described `DAT_00a8b2fc`
  as ally in one table.
- `ScenarioClass::Create_Houses` at `0x00687F10` resolves this for runtime
  usage: `DAT_00a8b2dc` / `NodeNameTag+0x5B` becomes `House+0x16058`
  team, and `DAT_00a8b2fc` / `NodeNameTag+0x63` becomes `House+0x1605C`
  start location. The "ally" wording for `DAT_00a8b2fc` is wrong for this
  path.

## Player-Facing UX Findings

### 1. Skirmish has start-position and team controls, but they reuse Host strings

Evidence: tooltip/control dispatcher `FUN_006040B0`.

For dialog kind `0x102` (Skirmish), the dispatcher maps these control ID ranges:

| Control IDs | Tooltip returned | Meaning |
|---|---|---|
| `0x50B`, `0x50E`, `0x516`, `0x51A`-`0x51D` | `STT:SkirmishComboAIPlayer` | AI-player add/remove/difficulty player slot controls |
| `0x6A1`, `0x510`, `0x513`, `0x51E`, `0x514`, `0x51F`, `0x520`, `0x521` | `STT:SkirmishComboCountry` | Country/faction controls |
| `0x6A2`, `0x522`-`0x528` | `STT:SkirmishComboColor` | Color controls |
| `0x6A3`, `0x6A4`, `0x6A5`, `0x6A6`, `0x6A7`, `0x6A8`, `0x6AA`, `0x6AB` | `STT:HostComboStart` | Start-position controls |
| `0x76D`-`0x774` | `STT:HostComboTeam` | Team controls |

Tiny detail that matters: there is no separate `STT:SkirmishComboStart` string.
A scan limited to `SkirmishCombo*` misses the start-location controls. The
Skirmish dialog reuses Host tooltip strings for start and team.

Why this matters: for our menu parity, a Skirmish 1v1 setup should expose at
least per-slot start selection and team selection if we are matching original
YR custom-game UX. It is not only an in-game clickable map phase.

### 2. Host and Guest multiplayer lobbies also expose start-position controls

Evidence: same dispatcher `FUN_006040B0`.

For Host dialogs (`0xBC`, `0xC2`), control IDs `0x6A3`-`0x6AB` return
`STT:HostComboStart`. For Guest dialogs (`0xBD`, `0xC9`), the same ID range
returns `STT:GuestComboStart`.

The adjacent `0x76D`-`0x774` range returns Host/Guest team tooltips. This places
start and team controls as parallel per-player/per-slot columns in the hosted
game UI.

### 3. The visible label is "Start", and the tooltip says it is the player's start position

Evidence: `langmd.mix` string data and binary string references.

Relevant CSF/string-table keys found:

- `GUI:StartPosition` - visible column/header text for "Start".
- `GUI:StartSelection` - selection prompt text for choosing a start location.
- `STT:HostComboStart` - tooltip text: player's start position.
- `STT:GuestComboStart` - tooltip text: player's start position.
- `GUI:Team` / `GUI:TeamSelection` - parallel team UI text.
- `GUI:NoneAsSymbols` - "---" style none value.
- `GUI:RandomAsSymbols` - random-symbol value.

Confidence note: the exact localized English value was read from CSF payload
strings in `langmd.mix`; the Ghidra evidence is the key references and control
mapping. The values align with the control purpose and prior shell reports.

### 4. Start combo choices are string-driven and include normal 1-8 positions

Evidence: `gamemd.exe` string cluster near `D:\ra2mdpost\GDlgSupp.cpp`.

The cluster contains:

- `GUI:RandomAsSymbols`
- `STT:HostComboStart`
- literal entries `0`, `8`, `7`, `6`, `5`, `4`, `3`, `2`, `1`
- `LETTER_D`, `LETTER_C`, `LETTER_B`, `LETTER_A`
- `GUI:NoneAsSymbols`

Verified conclusion:

- The normal map-start choices are represented as numbered positions, matching
  multiplayer waypoints 0-7 / player-facing positions 1-8.
- Random and none entries are available in the string population code.

Open detail:

- The exact mode-specific meaning of `0`, `A`-`D`, and their ordering was not
  fully traced through the combo-population handler. They likely cover random,
  none, and special team/co-op mode presentation, but this should remain an
  open question until the population handler is traced.

### 5. Start markers are also drawn on the map preview

Evidence: `DrawStartPositions` at `0x00640710`, listed in `ADDRESS_MAP.md` as
"DrawStartPositions (map preview markers)".

Observed behavior from decompilation:

- The function is Win32-dialog-facing: signature includes `HWND`, calls
  `ValidateRect`, and looks up dialog item `0x468`, the map thumbnail control
  used by Skirmish/Host/Guest screens.
- It scales scenario map coordinates into the thumbnail rectangle.
- It reads `ScenarioClass+0x113C` as the number of starting points.
- It loops while the count is greater than zero and less than nine, so it draws
  up to eight starting points.
- It reads start marker coordinates from `ScenarioClass+0x1140 + i*8` and
  `ScenarioClass+0x1144 + i*8`.
- It draws a shape offset around the scaled point, using `-9` on X and `-6` on Y
  before drawing the marker.
- It also draws a text label for each marker with `i + 1`, so the preview shows
  numbered starts.

Why this matters: original UX is not "choose by clicking the tactical renderer."
The shell shows a map thumbnail with numbered start markers, and the player uses
combo boxes to assign slot starts.

## Persistence and Runtime Flow

### 6. Skirmish `SlotXX` stores three integers per slot

Evidence: `SessionClass::ReadSkirmishSettings` at `0x00697F10` and
`SessionClass::WriteSkirmishSettings` at `0x00698F90`.

Read flow:

- Reads `[Skirmish]` keys for `GameMode`, `ScenIndex`, `GameSpeed`, `Credits`,
  `UnitCount`, `ShortGame`, `SuperWeaponsAllowed`, `BuildOffAlly`,
  `MCVRepacks`, and `CratesAppear`.
- Loops `Slot01` through `Slot07` only (`i = 1; i < 8`).
- For slot 1, the first default comes from one parameter; for slots 2-7, the
  first default comes from another parameter.
- The second and third defaults are both `-2`.
- Calls `FUN_00477440(section, "Slot%02d", &a, &b, &c)`.
- Stores the three values into `param_1[i*3+7]`, `param_1[i*3+8]`,
  `param_1[i*3+9]`.

Helper detail:

- `FUN_00477440` reads the INI value as a string, tokenizes by comma, and parses
  up to three integers with `atoi`.
- `FUN_00477510` writes the value back with format `"%d,%d,%d"`.

Tiny detail that matters: the read/write helpers prove the on-disk shape
(`SlotXX=a,b,c`) but not the semantic names. The semantic resolution comes from
the dialog controls and `Create_Houses`, not from the parser.

### 7. Runtime semantic mapping is country, color, team/start depending on array family

Evidence: `ScenarioClass::Create_Houses` at `0x00687F10`.

Human player `NodeNameTag` fields:

| Offset | Runtime use in `Create_Houses` |
|---|---|
| `+0x4B` | Country/faction index |
| `+0x53` | Color priority/index |
| `+0x5B` via `NodeNameTag__GetTeam()` | Team/alliance group |
| `+0x63` | Start location copied to `House+0x1605C` |
| `+0x6B` | Observer flag; `-1` marks observer |

AI parallel arrays:

| Global | Runtime use |
|---|---|
| `DAT_00a8b29c[i]` | AI country/faction |
| `DAT_00a8b2bc[i]` | AI color |
| `DAT_00a8b2dc[i]` | AI team |
| `DAT_00a8b2fc[i]` | AI start location |
| `DAT_00a8b27c[i]` | AI difficulty |

If an AI start location is not `-1`, `Create_Houses` sets
`ScenarioClass+0x11E0 = 1`, indicating explicit spawn-related data exists.

### 8. Scenario assignment respects preassigned starts, then fills the rest

Evidence: `ScenarioClass::AssignStartingPoints` at `0x005EE9D0`,
`ScenarioClass::Gather_Start_Positions` at `0x00688380`, and the existing
`SPAWN_POINT_ASSIGNMENT_SYSTEM.md`.

Verified flow:

- `Gather_Start_Positions` collects multiplayer start cells from waypoints 0-7.
- The scan stops on the first invalid/sentinel waypoint. Map starts must be
  contiguous from 0.
- If too few starts exist, the engine generates passable random fallback starts.
- `AssignStartingPoints` builds a 16-byte occupied array from
  `ScenarioClass+0x1180`, the `mp_start_waypoints[16]` table.
- Human houses are assigned first.
- AI houses are assigned second.
- If a house is already present in `ScenarioClass+0x1180`, it receives that
  start slot directly.
- Otherwise the start selection helper chooses a slot using the known
  random/distance algorithm from `SPAWN_POINT_ASSIGNMENT_SYSTEM.md`.

Implication: the menu's start combo does not directly place MCVs. It stores
slot preferences that become `House+0x1605C` / scenario preassignment state,
then scenario init maps those preferences to waypoint cells.

## UX Model to Match

Original YR shell UX for start positions is:

1. Player opens Skirmish or a hosted/guest multiplayer game setup screen.
2. The player sees rows/slots with player/AI, country, color, start, and team
   controls. Skirmish reuses Host tooltip strings for start/team.
3. The map thumbnail displays numbered starting markers.
4. The start combo lets a slot choose random/none/numbered start values, with
   normal starts presented as positions 1-8.
5. On launch, slot data is resolved into House fields and scenario preassignment.
6. The scenario start assignment code respects explicit starts before applying
   automatic assignment.

Not part of this UX:

- The tactical renderer is not responsible for this menu selection. The visible
  selection surface is a Win32 shell dialog with owner-drawn controls and a map
  thumbnail.
- There is no evidence that the original asks the player to click an in-game
  full tactical map before starting. Our current spawn-pick phase is useful, but
  it is not the original shell UX.

## Current Rust Status

Observed files:

- `src/map/waypoints.rs`
- `src/ui/main_menu.rs`
- `src/app_skirmish.rs`
- `src/app_spawn_pick.rs`
- `src/app_init.rs`

Implemented today:

- `[Waypoints]` parsing exists.
- Multiplayer start waypoints 0-7 are filtered and sorted.
- `SkirmishSettings` already has a `start_position: StartPosition`.
- `StartPosition` supports `Auto` and `Position(u8)`.
- `seed_skirmish_opening_if_needed` can swap the chosen start into local-player
  position 0 before seeding MCVs.
- A full-map clickable `SpawnPick` phase exists and draws markers.

Not at feature parity:

- The main menu currently exposes map selection, credits, zoom, and Start Game,
  but not per-slot country/color/start/team controls.
- The current code has one player country and one AI country, not a full
  player/AI slot table.
- `start_position` is present in settings but there is no visible start combo
  in `src/ui/main_menu.rs`.
- `spawn_pick_pending` is forced false in `src/app_init.rs`, so the clickable
  full-map spawn picker is disabled in the normal launch path.
- The Rust seeding path handles a simple first-two-starts setup, not the full
  original per-slot start reservation flow.

## Implementation Implications

For a first faithful 1v1 goal, the smallest parity-aligned UX target is:

- Add a compact Skirmish slot table before Start Game.
- Rows: local player and one AI opponent.
- Columns at minimum: Player/AI, Country, Color, Start.
- Team can be added beside Start because original Skirmish has team control IDs
  too, but it can stay neutral/default for the first 1v1 if alliance logic is
  not ready.
- Start combo values should include Random and numbered starts 1-8 based on the
  selected map's start waypoints.
- The map preview should show numbered start markers. A full tactical-map
  spawn-pick mode is optional/non-original.
- Launch should store/consume the selected start per slot, then seed player and
  AI at those waypoint cells.

This should be implemented as shell/app-layer state. `sim/` should not depend on
menu rendering or `egui`.

## Open Questions

1. Exact Win32 dialog resource pixel layout was not extracted. The functional UX
   is clear, but pixel placement/spacing would need dialog-template extraction
   or screenshot verification.
2. The combo population handler for `0`, `A`-`D`, Random, and None needs a
   focused trace if we want exact mode-specific lists.
3. The handoff from `House+0x1605C` start location into
   `ScenarioClass+0x1180` was not re-decompiled in this pass because the
   existing spawn report already covers `+0x1180` use. A future verify pass
   can trace that write specifically if we need absolute end-to-end proof.
4. Whether all Skirmish rows are always visible or hidden/disabled based on AI
   count should be verified from the dialog message handler, not only from
   tooltip mappings.

## Source Evidence

Ghidra functions decompiled/rechecked:

- `FUN_006040B0` - shell dialog control ID to tooltip key dispatcher.
- `FUN_004E4F30` - anchor reference to `D:\ra2mdpost\GDlgSupp.cpp` string table.
- `SessionClass::ReadSkirmishSettings` at `0x00697F10`.
- `SessionClass::WriteSkirmishSettings` at `0x00698F90`.
- `FUN_00477440` - comma-triple INI reader.
- `FUN_00477510` - comma-triple INI writer.
- `ScenarioClass::Create_Houses` at `0x00687F10`.
- `DrawStartPositions` at `0x00640710`.
- `ScenarioClass::AssignStartingPoints` at `0x005EE9D0`.

Binary/string evidence:

- `STT:HostComboStart` xrefs: `FUN_006040B0`, `FUN_004E4F30`.
- `STT:GuestComboStart` xref: `FUN_006040B0`.
- `STT:SkirmishComboAIPlayer`, `STT:SkirmishComboCountry`,
  `STT:SkirmishComboColor` xrefs: `FUN_006040B0`.
- `GDlgSupp.cpp` string cluster includes `GUI:RandomAsSymbols`,
  `STT:HostComboStart`, numeric entries `0` and `1`-`8`, `LETTER_A`-`LETTER_D`,
  and `GUI:NoneAsSymbols`.
- `langmd.mix` string data contains start/team GUI and tooltip labels used by
  these controls.

Existing docs used:

- `docs/research/SPAWN_POINT_ASSIGNMENT_SYSTEM.md`
- `docs/research/HOUSE_CREATION_COLOR_SYSTEM.md`
- `docs/research/SESSIONCLASS_GHIDRA_REPORT.md`
- `docs/research/LOBBY_SESSION_HOUSE_CREATION_GHIDRA_REPORT.md`
- `docs/research/GADGET_UI_FRAMEWORK_GHIDRA_REPORT.md`
