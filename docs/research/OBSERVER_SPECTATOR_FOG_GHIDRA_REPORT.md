# Observer/Spectator Mode & Fog of War — Ghidra Research Report

## Summary

The original engine has a **limited observer/spectator system** that only exists in
multiplayer (g_GameMode 3 or 4). There is no dedicated "spectator client" — observers
are implemented as **defeated players who continue watching with the map fully revealed**.
A lobby setting called `ObserverMode` controls whether defeated players stay in the
game as observers or are shown the resign/defeat dialog.

**Key finding:** There is NO separate spectator house. Observers are normal players
whose `IsDefeated` flag is set and whose `Visionary`/`MapIsClear` flags cause the
shroud to be fully revealed. The game simply stops processing their commands and reveals
all cells.

---

## Global Variables

| Address      | Name                   | Type   | Description |
|-------------|------------------------|--------|-------------|
| `0x00a8b238` | `g_GameMode`          | int    | 0=singleplayer, 3=LAN, 4=WOL/Internet, 5=skirmish |
| `0x00a8b23c` | `g_MultiplayerGameMode` | ptr  | Current `MultiplayerGameMode*` (NOT observer-specific) |
| `0x00ac10c8` | `g_ObserverMode`      | int    | Lobby setting: 0=disabled, nonzero=enabled |
| `0x00a83d4c` | `g_PlayerPtr`         | ptr    | Local player's HouseClass* |
| `0x00a8da84` | `g_PlayerCount`       | int    | Number of players in the game |
| `0x00a8da90` | `g_PlayerSlots[8]`    | int[8] | Array of player data pointers (slots 0-7) |

---

## HouseClass Fields Related to Observer/Fog

| Offset   | Name           | Type  | Description |
|----------|---------------|-------|-------------|
| `+0x1EC` | `CurrentPlayer` | bool | Player is human-controlled |
| `+0x1ED` | `PlayerControl` | bool | Player has player control (singleplayer) |
| `+0x1F5` | `IsDefeated`   | bool  | Player has been defeated |
| `+0x1F6` | `IsGivingUp`   | bool  | Player is surrendering |
| `+0x1F7` | `IsGivingUp2`  | bool  | Second surrender flag |
| `+0x1F8` | `HasWon`       | bool  | Player has won the game |
| `+0x240` | `Visionary`    | bool  | Map has been fully revealed (shroud cleared) |
| `+0x241` | `MapIsClear`   | bool  | Set when a player's map should be clear (observer state) |
| `+0x24A` | (unnamed)      | bool  | Set for AI houses when post-defeat alliance logic runs |

---

## RulesClass Multiplayer Settings (from `[MultiplayerDialogSettings]`)

| Offset     | INI Key              | Type |
|-----------|----------------------|------|
| `+0x14AC` | `BridgeDestruction`  | bool |
| `+0x14AD` | `ShadowGrow`         | bool |
| `+0x14AE` | `Shroud`             | bool |
| `+0x14AF` | `Bases`              | bool |
| `+0x14B0` | `TiberiumGrows`      | bool |
| `+0x14B1` | `Crates`             | bool |
| `+0x14B5` | `AlliesAllowed`      | bool |
| `+0x14B6` | `ShortGame`          | bool |
| `+0x14B7` | `FogOfWar`           | bool |
| `+0x14B8` | `MCVRedeploys`       | bool |

---

## How Observer Mode Works

### 1. Lobby Setup (Pre-Game)

The `ObserverMode` setting (`0x00ac10c8`) is read from `[MultiPlayer]` section of
`RA2MD.INI`:

```
Address: 0x005ee190
  DAT_00ac10c8 = CCINIClass__ReadInt("MultiPlayer", "ObserverMode", 0);
```

In the multiplayer lobby, when `ObserverMode` is enabled and the local player is
assigned as observer:
- The player gets **slot 7** (8th position, 0-indexed) — see string
  `"Sending observer slot of 7"` at `0x00831d50`
- The player's team index is set to **-1** (checked at `+0x6B` in player data),
  which marks them as having no team — i.e., observer

### 2. Game Start — Spawn Point Assignment

During `New_Scenario` → `FUN_005d6690` → `FUN_005d6890`:

For observers (teamIndex == -1), the spawn logic at `0x005d6890` either:
- Places them at a specific observed player's start cell
  (`"Observer Observing cell: %d (%d,%d)"`)
- Or assigns a random cell
  (`"Observer observing random cell: %d (%d,%d)"`)

### 3. Scenario Load — Shroud Disable Check

In the scenario loading function at `0x00684620`:

```c
// At 0x006849D8:
if (RulesClass->Shroud == false) {   // offset 0x14AE
    RevealEntireMap(NULL);            // FUN_00577f30(0)
}
```

If the `Shroud` multiplayer setting is disabled, the entire map is revealed for
everyone at load time.

### 4. Player Defeat — Transition to Observer

When a player is defeated (`HouseClass::MPlayer_Defeated` at `0x004fc0b0`):

**Step 1:** Set defeated flag
```c
house->IsDefeated = true;   // offset +0x1F5
```

**Step 2:** Handle based on local vs remote player

**If this IS the local player (g_PlayerPtr):**
```c
// At 0x004fc1a9:
FUN_00577f30(this);   // Reveal entire map
FUN_00656df0(1);      // Enable radar/minimap for observation
FUN_004f42f0(2);      // Set display mode to "observer"
// Plus: play defeat EVA, show notification, etc.
```

**If this is a REMOTE player:**
```c
// At 0x004fc283:
g_HouseClass_Array[house->ID]->MapIsClear = true;  // offset +0x241
```
This sets MapIsClear for the defeated house, which is used for the debug sync
dump and post-defeat alliance changes.

### 5. Map Reveal Function — FUN_00577f30 (0x00577f30)

This is the core "reveal entire map" function:

```c
void RevealEntireMap(int defeatedHouse) {
    if (defeatedHouse != NULL) {
        g_HouseClass_Array[defeatedHouse->ID]->MapIsClear = true;  // +0x241
    }

    // Must be the local player to reveal
    if (defeatedHouse != g_PlayerPtr && defeatedHouse != NULL) return;

    // Determine if we should skip map edge cells
    bool skipEdgeCells = false;
    if ((g_GameMode == 3 || g_GameMode == 4) && g_MultiplayerGameMode != NULL) {
        if (g_MultiplayerGameMode->vfunc_1() == false) {
            skipEdgeCells = true;  // Non-cooperative MP: skip edge tiles
        }
    }

    if (g_PlayerPtr->Visionary == false) {
        g_PlayerPtr->Visionary = true;   // +0x240

        // Initialize cell iterator
        // Iterate ALL cells on the map:
        for each cell {
            if (skipEdgeCells) {
                // Skip certain map-edge cells (tiles at row 7, 13, etc.)
                // This prevents revealing the invisible border area
            }
            RevealCell(cell, g_PlayerPtr);
        }

        RadarClass__RefreshRadar();
        FUN_004f42f0(1);  // Refresh display
    }
}
```

### 6. State Machine — Observer vs Resign Dialog

In `State_Machine` at `0x0048c860`, game state 2 (defeat/end):

```c
case 2:
    if (g_ObserverMode == 0) {
        // ObserverMode disabled: show resign dialog
        if (player not defeated/won/lost) {
            // FUN_005c60d0() - resign confirmation
        }
    } else {
        // ObserverMode enabled: skip dialog, player stays in game observing
        FUN_006471a0();
    }
```

When ObserverMode is enabled, the defeated player seamlessly transitions to
watching the game with the map fully revealed, instead of being shown a dialog.

### 7. Post-Defeat Alliance Changes (FUN_00501640)

After defeat, if `AlliesAllowed` (RulesClass+0x14B5) is enabled:

```c
// At 0x00501640:
if ((g_MultiplayerGameMode != NULL && g_MultiplayerGameMode->vfunc_1()) ||
    AlliesAllowed) {
    // For all non-human, non-defeated houses:
    //   - Ally with all human houses
    //   - Break alliance with all non-human houses
    // This effectively makes AI houses allied with human observers
}
```

---

## MultiplayerGameMode Virtual Table

`DAT_00a8b23c` stores a `MultiplayerGameMode*`. This is the currently active game
mode object, NOT specifically an observer object.

### Base class vtable at `0x007eed60`:
| Offset | Function     | Description |
|--------|-------------|-------------|
| `+0x00` | Destructor  | |
| `+0x04` | `vfunc_1`   | Returns bool. Base returns FALSE (`XOR AL,AL; RET`) |
| `+0x08` | `vfunc_2`   | |
| `+0x0C` | `vfunc_3`   | |
| ...    | ...          | Many more vtable entries |

### vfunc_1 Overrides:
| Class                     | Returns | Calling Convention |
|--------------------------|---------|-------------------|
| MultiplayerGameMode (base) | FALSE  | plain `RET` |
| MultiplayerBattle         | FALSE  | plain `RET` (inherited) |
| FreeForAll                | FALSE  | plain `RET` (inherited) |
| UnholyAlliance            | FALSE  | plain `RET` (inherited) |
| **MPCooperative**         | **TRUE** | plain `RET` (`MOV AL,1; RET` at 0x005c4ee0) |

**Purpose of vfunc_1:** Indicates whether the game mode is cooperative. Used in
shroud reveal to determine whether to skip edge cells (non-cooperative modes skip
map border tiles to prevent revealing the invisible boundary area).

### MultiplayerObserverTeam (separate hierarchy)

`MultiplayerObserverTeam` inherits from `MultiplayerTeam`, NOT `MultiplayerGameMode`.
It's constructed at `0x005c9470` and is part of the team management system, not the
game mode system.

- RTTI: `.?AVMultiplayerObserverTeam@@`
- Source: `D:\ra2mdpost\MPObserver.cpp`
- Vtable at `0x007ee6c8`
- Constructor calls `MultiplayerTeam::Constructor` with teamIndex = -1

---

## OBSERVER.PAL

The string `"OBSERVER.PAL"` at `0x008453f4` is referenced from a table of player
color palette filenames. This is the color palette assigned to observer players in
the lobby — it gives them a neutral/distinct color scheme.

---

## Multiplayer Replay / Recording

The game supports recording and playback via `_DAT_00a8d5f8` flags:
- Bit 0 (`& 1`): Recording mode — writes commands to file
- Bit 1 (`& 2`): Playback mode — reads commands from file

During replay playback, the viewer sees the game from a specific player's perspective.
There is no special "omniscient replay" mode — the replay viewer sees whatever fog
state the recorded player had. The recording saves frame data, commands, and sync
checksums. Playback feeds them back through the same simulation path.

---

## Key Addresses Summary

| Address      | Function |
|-------------|----------|
| `0x004fc0b0` | `HouseClass::MPlayer_Defeated` — defeat handler, reveals map |
| `0x00577f30` | `RevealEntireMap` — iterates all cells, calls RevealCell |
| `0x004aa050` | `RevealCell` — reveals a single cell (sets explored flag) |
| `0x005673a0` | `MapClass::RevealShroud` — per-unit shroud reveal with radius |
| `0x005678e0` | Related fog-of-war reveal function |
| `0x004a9dd0` | `MapClass::UpdateFogOfWarCell` |
| `0x004876f0` | `CellClass::RevealShroudFlags` — sets cell explored + fog bits |
| `0x00586360` | `IsShrouded` — checks if a cell is in shroud |
| `0x005864a0` | `IsFogged` — checks if a cell is in fog-of-war |
| `0x005ee190` | Reads `ObserverMode` from INI |
| `0x005ee200` | Updates lobby UI based on ObserverMode |
| `0x005ec3a0` | Lobby slot management — assigns observer to slot 7 |
| `0x005d6890` | Observer spawn point assignment |
| `0x00501640` | Post-defeat alliance changes |
| `0x00684620` | Scenario load — checks Shroud setting for full reveal |
| `0x00671ea0` | Reads multiplayer dialog settings from INI |

---

## Confidence Levels

- **g_GameMode values (0,3,4,5):** HIGH — verified from New_Scenario switch cases
  and debug strings ("GAME_INTERNET")
- **HouseClass field offsets (0x1F5, 0x240, 0x241):** HIGH — verified from debug
  strings ("MapIsClear set to true", sync dump format string with "Visionary:%d MapIsClear:%d")
- **ObserverMode (0x00ac10c8):** HIGH — verified from INI read ("ObserverMode" key)
  and State_Machine usage
- **RulesClass multiplayer settings offsets:** HIGH — verified from sequential
  CCINIClass::ReadBool calls with string literals
- **MultiplayerGameMode::vfunc_1 meaning:** MEDIUM-HIGH — cooperative returns TRUE,
  all others return FALSE. Used for edge-cell skipping during map reveal. Could have
  additional semantic meaning not yet discovered.
- **Observer slot assignment (slot 7):** HIGH — verified from debug string and code
- **No separate spectator house:** HIGH — no evidence of a dedicated spectator
  HouseClass; observers use defeated player path

---

## Implications for Rust Implementation

1. **No special observer entity type needed.** Observer mode is just a combination
   of flags on the existing HouseClass: `IsDefeated=true` + `Visionary=true`.

2. **Map reveal is a one-time operation.** When transitioning to observer, iterate
   all cells and set their explored/revealed flags. No per-tick fog update needed
   for observers.

3. **ObserverMode is a lobby/game setting**, not a per-house flag. It controls whether
   defeated players transition to observer view or see the defeat dialog.

4. **The `Shroud` multiplayer setting** (RulesClass+0x14AE) can disable shroud
   entirely at game start, revealing everything for all players.

5. **Edge-cell skipping** during map reveal in non-cooperative multiplayer prevents
   revealing the invisible map border area. This is controlled by
   `MultiplayerGameMode::vfunc_1()` returning false for battle modes.
