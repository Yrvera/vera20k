# Lobby / Session → HouseClass Creation — Ghidra Report

**Source**: Ghidra decompilation of `gamemd.exe`
**Confidence**: High — corroborated by debug strings (`"Player %i %s: Start %i, Country %i, Color %i"`, `"AI %i: Difficulty %i, Country %i, Color %i, Start %i, Ally %i"`)
**Date**: 2026-03-26

## Overview

In multiplayer/skirmish, houses are NOT created from the map's `[Houses]` section.
They are created entirely from **lobby/session slot data**. The map only provides
waypoints for starting positions.

## Session Data Structures

### Skirmish Settings (SessionClass)

`SessionClass::ReadSkirmishSettings` (0x00697f10) reads from INI:

| Key | Type | Default |
|-----|------|---------|
| GameMode | int | from session |
| ScenIndex | int | 0 |
| GameSpeed | int | from `[General]` rules +0x14A0 |
| Credits | int | from `[General]` rules +0x1484 |
| UnitCount | int | from `[General]` rules +0x1494 |
| ShortGame | bool | from rules +0x14B6 |
| SuperWeaponsAllowed | bool | from rules +0x14B9 |
| BuildOffAlly | bool | from rules +0x14BA |
| MCVRepacks | bool | from rules +0x14B8 |
| CratesAppear | bool | from rules +0x14B1 |

Per-slot (`Slot01`–`Slot07`): 3 values each — Country, Color, Team.

`SessionClass::WriteSkirmishSettings` (0x00698F90) writes the same fields back.

### Human Player Nodes (NodeNameTag)

Pointer array at `DAT_00a8da78`, count at `DAT_00a8da84`.

Each node struct has:

| Offset | Field | Values |
|--------|-------|--------|
| +0x4B | Country index | 0-9 = specific country, -2 = Random, -3 = Observer/Closed |
| +0x4F | Country resolved flag | -2 = needs random, -1 = already resolved |
| +0x53 | Color priority | 0-7 = specific color, -2 = Random |
| +0x57 | Color resolved flag | -2 = needs random |
| +0x5B | Team | alliance group, -2 = no team |
| +0x63 | Start position / Ally | |
| +0x6B | Assigned start | -1 = random assignment |
| +0x6F | → HouseClass ID | set after house creation |

### AI Slots (parallel arrays, 8 entries max)

| Global Address | Field |
|----------------|-------|
| `0xa8b29c + i*4` | Country index |
| `0xa8b2bc + i*4` | Color priority |
| `0xa8b27c + i*4` | Difficulty (0=Easy, 1=Normal, 2=Hard) |
| `0xa8b2dc + i*4` | Team |
| `0xa8b2fc + i*4` | Ally |

AI count at `DAT_00a8b274`.

## Random Resolution

`SessionClass::ProcessRandomAssignments` (0x0069B8C0) runs once before house creation.

For each **human** player node:
- Country == -2 → pick random 0-9 (via network callback if multiplayer, else `Random::RandomRanged`)
- Color == -2 → pick random 0-7, **collision-checked** against all other players (retry until unique)
- Observer (-3) → forced to country=-3, color=-1, team=8, start=-1

For each **AI** slot:
- Country == -2 → pick random 0-9 (via network callback if available)
- Color == -2 → pick random 0-7, collision-checked against ALL players (human + AI + other AI)

Debug output: `"Player[%i]: %s, Side = %i, Color = %i"`

## Color Priority → Scheme Mapping

`SessionClass::PriorityToColorScheme` (0x0069A310):

Colors in the lobby are **priority numbers** (0-8). These map through a lookup table
at `0x0083ed14` to ColorScheme indices used by the rendering palette remap system:

```
Bytes at 0x0083ed14: 03 0B 15 1D 0D 19 11 0F 05

Priority 0 → Scheme 0x03 (Gold/Yellow)
Priority 1 → Scheme 0x0B (Red)
Priority 2 → Scheme 0x15 (Blue)
Priority 3 → Scheme 0x1D (Green)
Priority 4 → Scheme 0x0D (Orange)
Priority 5 → Scheme 0x19 (SkyBlue)
Priority 6 → Scheme 0x11 (Purple)
Priority 7 → Scheme 0x0F (Pink)
Priority 8 → Scheme 0x05 (Observer)
```

If priority == -2 (unresolved random), falls back to `DAT_0083ed1c` (default).

String table entries confirm order: `STT:PlayerColorGold`, `STT:PlayerColorRed`,
`STT:PlayerColorBlue`, `STT:PlayerColorGreen`, `STT:PlayerColorOrange`,
`STT:PlayerColorSkyBlue`, `STT:PlayerColorPurple`, `STT:PlayerColorPink`,
`STT:PlayerColorObserver`.

## ScenarioClass::Create_Houses (0x00687F10)

This is the core function that turns lobby slots into runtime HouseClass objects.

### Phase 1: Human Players

Iterates human player nodes sorted by **color priority** (lowest first):

```
for each human node (sorted by color priority ascending):
    1. operator_new(0x160B8)     // 90,296 bytes per HouseClass
    2. HouseClass::Constructor(country_type_ptr)
    3. Copy player name to HouseClass+0x1602A (21 chars)
       - Multiplayer: actual player name from network
       - Singleplayer: "<human_player>"
    4. HouseClass+0x1EC = 1      // is_active
    5. Set_Credits_And_Color(color_priority, country_index, starting_credits)
       - +0x1DC = credits, +0x30C = credits (both set)
       - CountryType+0xC0 = color_priority
       - +0x16054 = color_priority
    6. PriorityToColorScheme(color_priority) → store at +0x16054
    7. InitColor()               // initialize color state
    8. ComputeRemap()            // build palette remap table
    9. Store team at +0x16058
   10. Store start_position at +0x1605C
   11. If first player (index 0):
       - g_PlayerPtr = this house
       - +0x1ED = 1              // is_player_controlled
   12. If start_position == -1:
       - DAT_00AC1198 = this     // "spectator" house
   13. +0x1D4 = base IQ          // from rules default
   14. SetDifficulty(1)          // humans always difficulty 1
   15. node+0x6F = house+0x30    // link node back to house array index
```

### Phase 2: AI Players

Iterates AI slot array (`0xa8b29c` through `0xa8b2bc`):

```
for each AI slot where country != -1 and country != -3:
    1. operator_new(0x160B8)
    2. HouseClass::Constructor(country_type_ptr)
    3. +0x1EC = 0                // NOT human-active
    4. Set_Credits_And_Color(color, country, starting_credits)
    5. PriorityToColorScheme(color) → +0x16054
    6. InitColor() + ComputeRemap()
    7. Store team at +0x16058
    8. Store ally at +0x1605C
    9. Name = "Computer"
   10. +0x1D4 = base IQ from rules
   11. Difficulty from AI slot array
       - If MultiPlayerAIDifficultyModifier=yes AND >1 human: difficulty -= 1
   12. SetDifficulty(difficulty)
```

### Phase 3: Neutral Houses

Two additional HouseClass objects created for civilian/neutral structures:

```
1. operator_new(0x160B8) → HouseClass::Constructor(neutral_country)
   - FindColorSchemeIndex() → +0x16054
   - InitColor()
2. operator_new(0x160B8) → HouseClass::Constructor(neutral_country)
   - FindColorSchemeIndex() → +0x16054
   - InitColor()
```

## Starting Point Assignment

`ScenarioClass::AssignStartingPoints` (0x005EE9D0) runs after Create_Houses.

1. `Gather_Start_Positions()` — collects waypoints 0-15 from map
2. First pass: Human players that have explicit start positions get assigned
3. Second pass: Remaining humans get the **farthest available** waypoint from already-assigned points
4. Third pass: AI houses get assigned to remaining waypoints (also maximizing distance)

## Unit Generation

`ScenarioClass::Generate_Random_Units` (0x006886B0) runs during `Post_Map_Init`:

1. Calculate budget: `UnitCount * average_unit_cost` across all spawnable unit types
2. For each house:
   - First house gets a **random** start position; subsequent houses get **maximally distant** positions
   - If `MCVRepacks` enabled: spawn MCV at start position, optionally auto-deploy
   - Spawn random units up to budget (2/3 ground vehicles, 1/3 aircraft)
   - If `AllVeteran` flag set: all spawned units get elite veterancy
3. Starting credits added from session settings via `HouseClass::Add_Credits()`

## HouseClass Key Fields (relevant to lobby)

| Offset | Size | Field |
|--------|------|-------|
| +0x30 | 4 | Array index |
| +0x34 | 4 | → HouseTypeClass (country) |
| +0x1D4 | 4 | Base IQ level |
| +0x1DC | 4 | Credits (balance) |
| +0x1EC | 1 | is_active |
| +0x1ED | 1 | is_player_controlled |
| +0x30C | 4 | Credits (duplicate/display?) |
| +0x16054 | 4 | ColorScheme index |
| +0x16058 | 4 | Team |
| +0x1605C | 4 | Start position / Ally |
| +0x15FF4 | 21 | Player name string |
| +0x1602A | 21 | Node name string |

## Full Init Sequence (multiplayer)

`ScenarioClass::Full_Init` (0x00686B20):

```
1. Clear old scenario state
2. Set special flags from session (fog, MCV redeploy, etc.)
3. Load map INI, read rules
4. Read all country/house type classes from rules
5. *** Create_Houses() ***           ← lobby slots → HouseClass objects
6. Load map terrain, theaters
7. AssignStartingPoints()            ← match houses to waypoints
8. Load map objects (pre-placed units/buildings/terrain)
9. Post_Map_Init():
   a. Remove excess AI houses if fewer players than expected
   b. Generate_Random_Units()        ← spawn MCVs + starting units
   c. Make neutral house ally with everyone
   d. Add starting credits to each house
10. Initialize radar, fog of war, overlays
11. Game begins
```

## Implications for Rust Implementation

1. **Lobby is flat data**: country index + color priority + team + start position per slot.
   No complex state — just a `Vec<PlayerSlot>`.

2. **Map `[Houses]` is irrelevant in multiplayer**. All houses come from session slots.
   Map only provides waypoints.

3. **Color is indirect**: lobby color (0-7) → priority-to-scheme table → ColorScheme index.
   Need to replicate or simplify this mapping for palette remap.

4. **Random resolution is separate**: resolve random country/color before creating houses,
   with collision detection for colors.

5. **Two neutral houses always exist**: needed for civilian/tech structures on the map.

6. **Start point assignment is a post-step**: houses created first, then matched to
   waypoints by distance maximization.

7. **UnitCount spawning is budget-based**: not a fixed number of units, but a cost budget
   filled with random eligible unit types.
