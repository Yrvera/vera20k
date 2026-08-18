# SessionClass & Game Settings — Ghidra Analysis

**Source:** Live Ghidra decompilation of `gamemd.exe` (YR 1.001)
**Confidence:** HIGH for game option offsets (verified from `ReadMultiplayerDialogSettings`
decompilation with visible INI string references). MEDIUM for SessionClass struct layout
(some fields inferred from context).

---

## Overview

The "session" in gamemd.exe is split across **two** structures:

1. **RulesClass** (singleton at `DAT_008871e0`) — stores game options as fields at offsets
   +0x1480..+0x14BB. These are defaults from `[MultiplayerDialogSettings]` in `rulesmd.ini`,
   overridable per-game.

2. **SessionClass** (various globals at `DAT_00a8b230` area) — stores player slot data,
   network settings, and per-game state. Read from `RA2MD.INI` sections `[Skirmish]`,
   `[MultiPlayer]`, `[SerialDefaults]`.

The game options (Crates, ShortGame, etc.) live on RulesClass, NOT on a separate SessionClass
struct. SessionClass manages the lobby slots and network layer.

---

## Part 1: Game Options on RulesClass

All offsets relative to `g_RulesClass_Instance` (DAT_008871e0).

Parsed by `RulesClass__ReadMultiplayerDialogSettings` (0x00671ea0) from INI section
`[MultiplayerDialogSettings]`. All verified from live Ghidra decompilation with visible
string references.

### Integer Settings (4 bytes each)

| Offset | INI Key | Type | Purpose |
|--------|---------|------|---------|
| +0x1480 | `MinMoney` | int | Minimum credits in lobby slider |
| +0x1484 | `Money` | int | Default starting credits |
| +0x1488 | `MaxMoney` | int | Maximum credits in lobby slider |
| +0x148C | `MoneyIncrement` | int | Slider step size for credits |
| +0x1490 | `MinUnitCount` | int | Minimum starting units |
| +0x1494 | `UnitCount` | int | Default starting unit count |
| +0x1498 | `MaxUnitCount` | int | Maximum starting units |
| +0x149C | `TechLevel` | int | Default tech level |
| +0x14A0 | `GameSpeed` | int | Default game speed |
| +0x14A4 | `AIDifficulty` | int | Default AI difficulty (0/1/2) |
| +0x14A8 | `AIPlayers` | int | Default number of AI opponents |

### Boolean Settings (1 byte each)

| Offset | INI Key | Purpose |
|--------|---------|---------|
| +0x14AC | `BridgeDestruction` | Bridges can be destroyed |
| +0x14AD | `ShadowGrow` | Shroud regrows (TS legacy, unused in YR) |
| +0x14AE | `Shroud` | Shroud enabled (black fog for unexplored) |
| +0x14AF | `Bases` | Construction yards / base building enabled |
| +0x14B0 | `TiberiumGrows` | Ore/gems regenerate on the map |
| +0x14B1 | `Crates` | Random crate spawning enabled |
| +0x14B2 | `CaptureTheFlag` | Capture-the-flag mode (TS legacy) |
| +0x14B3 | `HarvesterTruce` | Harvesters cannot be attacked |
| +0x14B4 | `MultiEngineer` | Engineers capture at reduced HP only |
| +0x14B5 | `AlliesAllowed` | Players can form alliances |
| +0x14B6 | `ShortGame` | Defeat when ConYard + all buildings lost |
| +0x14B7 | `FogOfWar` | Semi-transparent fog (TS legacy, default OFF in YR) |
| +0x14B8 | `MCVRedeploys` | MCV can unpack back into vehicle |
| +0x14B9 | `SuperWeaponsAllowed` | Superweapons can be built |
| +0x14BA | `BuildOffAlly` | Can build adjacent to allied buildings |
| +0x14BB | `AllyChangeAllowed` | Alliances can be changed mid-game |

### Key Function

```
RulesClass__ReadMultiplayerDialogSettings (0x00671ea0)
  param_1 = RulesClass* (g_RulesClass_Instance)
  Reads from [MultiplayerDialogSettings] in rulesmd.ini
  Returns 1 if section exists, 0 if not
```

---

## Part 2: Skirmish Settings (SessionClass)

The skirmish settings are stored in a small struct at `DAT_008870c0` (28 bytes + slot data).
Parsed by `SessionClass__ReadSkirmishSettings` (0x00697f10) from `[Skirmish]` section in
`RA2MD.INI`.

### Skirmish Struct Layout (at DAT_008870c0)

| Offset | Size | INI Key | Purpose |
|--------|------|---------|---------|
| +0x00 | 4 | `GameMode` | 1=Skirmish, 6=default |
| +0x04 | 4 | `ScenIndex` | Selected map index |
| +0x08 | 4 | `GameSpeed` | Override of Rules default |
| +0x0C | 4 | `Credits` | Override of Rules default |
| +0x10 | 4 | `UnitCount` | Override of Rules default |
| +0x14 | 1 | `ShortGame` | Override boolean |
| +0x15 | 1 | `SuperWeaponsAllowed` | Override boolean |
| +0x16 | 1 | `BuildOffAlly` | Override boolean |
| +0x17 | 1 | `MCVRepacks` | Override boolean |
| +0x18 | 1 | `CratesAppear` | Override boolean |
| +0x1C..+0x58 | 12×7 | `Slot01`..`Slot07` | Per-slot: side, color, start_location (3 ints each) |

### Slot Layout

Slots 1-7 (index 1-based), 3 ints per slot:
```
param_1[slot * 3 + 7] = side/country index
param_1[slot * 3 + 8] = color index
param_1[slot * 3 + 9] = start location
```

Slot 1 uses `param_5` (special handling for local player), slots 2-7 use `param_4`.
Value -2 (0xFFFFFFFE) means "random" for all three fields.

### Key Function

```
SessionClass__ReadSkirmishSettings (0x00697f10)
  Called as: FUN_00697f10(&DAT_008870c0, "Skirmish", 1, 6)
  param_1 = skirmish struct pointer
  param_2 = INI section name
  param_3 = default GameMode (1)
  param_4 = default slot value (6 = AI)
  param_5 = overridden for slot 1 (local player)
```

---

## Part 3: Player Slot Globals

These globals manage the player slots before houses are created.

### Human Player Slots

| Address | Type | Purpose |
|---------|------|---------|
| `DAT_00a8da78` | int*[8] | Array of pointers to NodeNameTag structs (one per human) |
| `DAT_00a8da84` | int | Number of human players |
| `DAT_00a8b394` | int | Local player's color index (saved during random assignment) |

### AI Player Slots (Parallel Arrays)

| Address | Type | Offset from DAT_00a8b29c | Purpose |
|---------|------|--------------------------|---------|
| `DAT_00a8b27c` | int[8] | -0x20 | AI difficulty per slot (0=Easy, 1=Normal, 2=Hard) |
| `DAT_00a8b29c` | int[8] | +0x00 | AI country/side index per slot (-1=empty, -3=observer) |
| `DAT_00a8b2bc` | int[8] | +0x20 | AI color index per slot |
| `DAT_00a8b2dc` | int[8] | +0x40 | AI team index per slot |
| `DAT_00a8b2fc` | int[8] | +0x60 | AI start location per slot (-1=random) |
| `DAT_00a8b274` | int | — | Total AI player count |

### NodeNameTag Structure (per human player)

| Offset | Size | Type | Field |
|--------|------|------|-------|
| +0x4B | 4 | int | CountryIndex (-3=observer, -2=random) |
| +0x4F | 4 | int | CountryRandom (-2=random, -1=assigned) |
| +0x53 | 4 | int | ColorIndex (0-7 lobby slot, used as sort priority) |
| +0x57 | 4 | int | ColorRandom (-2=random, -1=assigned) |
| +0x5B | 4 | int | TeamIndex |
| +0x5F | 4 | int | TeamRandom |
| +0x63 | 4 | int | SpawnLocation (0-7 map position, -1=random) |
| +0x67 | 4 | int | SpawnRandom |
| +0x6B | 4 | int | ObserverFlag (-1=observer, else=player) |
| +0x6F | 4 | int | HouseIndex (written AFTER house creation) |

---

## Part 4: Random Assignment Resolution

`SessionClass__ProcessRandomAssignments` (0x0069b8c0) runs before `Create_Houses`.
Resolves all "random" (-2) selections to concrete values.

### For each human player:
1. If `CountryIndex == -3` (observer): set side=-3, color=8, both randoms=-1
2. If `CountryRandom == -2`: pick random country 0-9 via `Random__RandomRanged(0,9)`
3. If `ColorRandom == -2`: pick random color 0-7, loop until unique (checked via `FUN_0069b600`)
4. Player 0's color saved to `DAT_00a8b394` (local player color)

### For each AI slot (8 iterations over DAT_00a8b29c):
1. If country == -2: pick random 0-9
2. If color == -2: pick random 0-7, collision-check against all human colors AND other AI colors
3. Log: `"AI %i: Side = %i, Color = %i"`

---

## Part 5: Network/Multiplayer Globals

| Address | Type | Purpose |
|---------|------|---------|
| `DAT_00a8b238` | int | GameMode (0=campaign, 3=LAN, 4=WOL, 5=skirmish) |
| `DAT_00a8b25c` | int | StartingCredits (global, used by Create_Houses) |
| `DAT_00a8b550` | int | MaxAhead / NetworkFrameBudget (lockstep frame budget) |
| `DAT_00a8b262` | byte | Special defeat mode flag (alt defeat detection in Update) |
| `DAT_00a8b538` | byte | GameEnding flag (set when local player defeated) |
| `DAT_00ac1198` | ptr | ObserverHouse pointer (HouseClass* of observer, if any) |
| `DAT_00a83d4c` | ptr | PlayerPtr (local player's HouseClass*) |
| `DAT_00a83d49` | byte | GameWon flag (triggers game-over screen) |
| `DAT_00a8ecd0` | byte | GameLost flag |
| `DAT_00a8b230` | ptr | ScenarioClass* / SpecialFlags bitfield |

### SpecialFlags Bits (DAT_00a8b230 dereferenced)

| Bit | Mask | Purpose |
|-----|------|---------|
| 4 | 0x10 | Rally point clearing on defeat |
| 11 | 0x800 | Garrison ejection on defeat |
| 12 | 0x1000 | Fog of war active (TS legacy gate) |

---

## Part 6: SessionClass Settings from RA2MD.INI [MultiPlayer]

Read by `FUN_006980c0` from `RA2MD.INI`:

| Offset (param_1) | INI Key | Purpose |
|-------------------|---------|---------|
| +0x17C | `Color` | Last used color |
| +0x180 | `ColorEx` | Extended color |
| +0x184 | (via FUN_00475540) | Country/side selection |
| +0x188 | `SideEx` | Extended side selection |
| +0x308 | `WOLLimitResolution` | Resolution limit for Westwood Online |
| +0x30C | `LastNickSlot` | Last used nickname slot |
| +0x1ED8 | `LANTaunts` | Taunts enabled on LAN |
| +0x1ED9 | `WOLTaunts` | Taunts enabled on WOL |
| +0x1FC0 | `LANScrollText` | Scroll text on LAN |
| +0x1FC1 | `WOLScrollText` | Scroll text on WOL |
| +0x2884 | `PhoneIndex` | Modem phone book index |
| +0x2FF0 | `PortBase` | Network port base (default 0x4E2 = 1250) |
| +0x2FF4 | `ForcePortBase` | Forced port override |
| +0x30D0 | `CheckHeap` | Debug heap checking |

---

## Part 7: Flow from Lobby to Game Start

```
1. Game launch
   → Load rulesmd.ini
   → RulesClass__ReadMultiplayerDialogSettings() fills Rules+0x1480..+0x14BB

2. Lobby screen
   → Player picks country, color, team, map, options
   → Options override Rules defaults into SessionClass skirmish struct
   → AI slots configured in parallel arrays

3. Click "Start"
   → SessionClass__ProcessRandomAssignments() resolves all randoms
   → ScenarioClass__Full_Init() begins
     → ScenarioClass__Create_Houses() reads slot data, creates HouseClass instances
     → ScenarioClass__Post_Map_Init() spawns MCVs, starting units, crates
     → ScenarioClass__AssignStartingPoints() places houses at spawn positions

4. Gameplay
   → Each HouseClass::Update() runs every frame
   → Options like Crates, ShortGame, BridgeDestruction read from Rules+0x14XX
   → Network uses MaxAhead for lockstep sync
```

---

## Part 8: Relevance to ra2-rust-game

### What We Currently Have
- `app_skirmish.rs` — handles skirmish setup but doesn't read from INI game options
- `rules/ruleset.rs` — parses rules.ini but doesn't parse [MultiplayerDialogSettings]
- `map/houses.rs` — parses per-map [Houses] section
- No SessionClass equivalent

### What We Need
1. **GameOptions struct** in `rules/` — parse [MultiplayerDialogSettings] for all boolean/int options
2. **SkirmishSettings struct** — persist last-used settings (credits, unit count, AI count, etc.)
3. **PlayerSlot struct** — represent a lobby slot (country, color, team, spawn, difficulty)
4. **Random assignment resolution** — resolve "random" picks before game start
5. The options feed into `Create_Houses` equivalent and simulation tick behavior

### Key Design Decision
The original engine stores game options on RulesClass (mixed with other rules data). In our
engine, these should be a separate `GameOptions` struct passed to the simulation, NOT mixed
into the ruleset. This keeps the separation clean: rules = static game data from INI,
options = per-game lobby choices.
