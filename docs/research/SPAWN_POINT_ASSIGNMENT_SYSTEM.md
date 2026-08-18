# Spawn Point Assignment System -- gamemd.exe Ghidra Analysis

Complete reverse-engineering of the spawn point assignment pipeline: how gamemd.exe
collects start positions from map waypoints, assigns them to players (human-first,
AI-second), and selects positions using distance-based algorithms.

Sources: Decompiled C files 085, 086, 109 from `C:\Users\enok\Documents\gidra\gidra c files\`,
reports 085, 086, 109, 110, 112 from `C:\Users\enok\Documents\gidra\reports\`,
and `docs/GAME_START_INITIALIZATION.md`.

---

## 1. Call Chain Overview

```
ScenarioClass::Full_Init() [0x00686b20]  (the 4.5KB monster function)
  -> ... loading terrain, overlays, triggers, houses ...
  -> ScenarioClass::Post_Map_Init() [0x00686890]
       -> Generate_Random_Units() [0x006886b0]  (if Bases=yes)
            -> Gather_Start_Positions() [0x00688380]
            -> per-house position assignment + MCV placement
       -> AssignStartingPoints() [0x005ee9d0]
            -> Gather_Start_Positions() [0x00688380]
            -> per-house FUN_005ee6f0() (position selection algorithm)
            -> FUN_0050e000() (SetPrimaryCenter)
```

**Key insight**: `Gather_Start_Positions` is called TWICE -- once from `Generate_Random_Units`
(for MCV placement) and once from `AssignStartingPoints` (for formal position assignment).
Both share the same waypoint pool and selection logic.

Confidence: HIGH -- confirmed by call graph in decompiled C files + debug string references.

---

## 2. AssignStartingPoints (0x005ee9d0) -- Full Annotated Pseudocode

Address: `0x005ee9d0` | Size: 475 bytes | Called by: `Full_Init` (0x00686b20)

```c
void AssignStartingPoints(void) {
    // ---- Phase 0: Collect spawn pool ----
    DynamicVector<CellStruct> spawnPool;
    Gather_Start_Positions(&spawnPool);   // FUN_00688380

    // ---- Phase 1: Build occupancy array from pre-assigned positions ----
    // ScenarioClass+0x1180 stores a 16-slot table:
    //   [index] = house_index   (-1 means free)
    // Each slot corresponds to a spawn position index.
    char occupied[16];                    // local_28 in decompilation
    for (int i = 0; i < 16; i++) {
        int* table = (int*)(DAT_00a8b230 + 0x1180);  // ScenarioClass->mp_start_waypoints
        if (table[i] == -1) {
            occupied[i] = 0;  // free
        } else {
            occupied[i] = 1;  // already assigned to some house
        }
    }

    // ---- Phase 2: Assign HUMAN players first ----
    // Iterates all houses in DAT_00a8022c (HouseClass::Array), count = DAT_00a80238
    for (int houseIdx = 0; houseIdx < DAT_00a80238; houseIdx++) {
        HouseClass* house = DAT_00a8022c[houseIdx];
        HouseTypeClass* houseType = house->Type;  // +0x34

        // Skip spectators: HouseTypeClass+0x1a6 == observer flag
        if (houseType->IsObserver != 0) continue;

        // Only process HUMAN players: HouseClass+0x1ec == IsHuman
        if (house->IsHuman == 0) continue;

        // Check if this house already has a pre-assigned position
        // Scan the 16-slot table for an entry matching this house index
        bool preAssigned = false;
        int preAssignedSlot = -1;
        for (int slot = 0; slot < 16; slot++) {
            int* table = (int*)(DAT_00a8b230 + 0x1180);
            if (table[slot] == houseIdx) {
                preAssigned = true;
                preAssignedSlot = slot;
            }
        }

        if (preAssigned) {
            // Use the pre-assigned position directly
            CellStruct cell = spawnPool.data[preAssignedSlot];
            House::SetPrimaryCenter(house, cell);   // FUN_0050e000
        } else {
            // No pre-assignment -- pick via algorithm
            Debug("Assigning Starting Points for house %d (%s)", houseIdx, houseType->Name);

            CellStruct cell;
            // isHuman=1 means "first player gets random" preference
            SelectSpawnPoint(&cell, houseIdx, &spawnPool, occupied, /*isFirstPlayer=*/1);
            House::SetPrimaryCenter(house, cell);

            CellStruct center = House::GetBaseCenter(house);
            Debug("Starting point for house %d is x=%d y=%d",
                  center.X, center.Y);
        }
    }

    // ---- Phase 3: Assign AI players second ----
    for (int houseIdx = 0; houseIdx < DAT_00a80238; houseIdx++) {
        HouseClass* house = DAT_00a8022c[houseIdx];
        HouseTypeClass* houseType = house->Type;

        // Skip spectators
        if (houseType->IsObserver != 0) continue;

        // Only process AI players: HouseClass+0x1ec == 0
        if (house->IsHuman != 0) continue;

        Debug("Assigning Starting Points for house %d (%s)", houseIdx, houseType->Name);

        CellStruct cell;
        // isFirstPlayer=0: AI never gets the "random first pick" privilege
        SelectSpawnPoint(&cell, houseIdx, &spawnPool, occupied, /*isFirstPlayer=*/0);
        House::SetPrimaryCenter(house, cell);

        CellStruct center = House::GetBaseCenter(house);
        Debug("Starting point for house %d is x=%d y=%d",
              center.X, center.Y);
    }

    // Cleanup: destroy the local DynamicVector
    spawnPool.~DynamicVector();
}
```

### Key Design Decisions

1. **Human-first, AI-second**: Humans are assigned positions before AI. This ensures
   human players get their preferred/pre-assigned positions. AI fills remaining slots.

2. **Pre-assigned position check**: The 16-slot table at `ScenarioClass+0x1180` stores
   lobby-selected spawn positions. When a player picks "Start Location 3" in the lobby,
   their house index is written to `table[3]`. The code scans this table first.

3. **isFirstPlayer flag**: Passed as `1` for human players, `0` for AI. This controls
   whether the "first player random" path is taken in the selection algorithm.

Confidence: HIGH -- decompiled from raw C with debug string references confirming
house index and coordinate logging.

---

## 3. Gather_Start_Positions (0x00688380) -- Full Annotated Pseudocode

Address: `0x00688380` | Size: 813 bytes | Called by: `AssignStartingPoints`, `Generate_Random_Units`

```c
DynamicVector<CellStruct>* Gather_Start_Positions(DynamicVector<CellStruct>* result) {
    // Init a temporary working vector
    DynamicVector<CellStruct> tempVec;
    tempVec.Init(/*capacity=*/10, /*growIncrement=*/10);

    // ---- Step 1: Count valid waypoints 0-7 ----
    int numValidWaypoints = 0;
    short* waypointArray = (short*)(DAT_00a8b230 + 0x632);  // ScenarioClass->waypoints

    for (int i = 0; i < 8; i++) {
        // Waypoint validity check:
        //   - Index must be in range [0, 701]
        //   - Both X and Y must differ from sentinel values
        if (i > 0x2bd || i < 0) break;  // range check (always passes for 0-7)

        // Each waypoint is 4 bytes: packed (short X, short Y)
        if (waypointArray[i*2] == DAT_00b05458 &&
            waypointArray[i*2+1] == DAT_00b0545a) {
            break;  // hit sentinel = end of valid waypoints
        }
        numValidWaypoints++;
    }

    // ---- Step 2: Count how many players need positions ----
    int observerCount = 0;
    for (int i = 0; i < DAT_00a8da84; i++) {    // DAT_00a8da84 = human player count
        PlayerData* player = DAT_00a8da78[i];     // DAT_00a8da78 = human player list
        if (*(int*)(player + 0x6b) == -1) {       // spawn position == -1 means observer
            observerCount++;
        }
    }

    // Total positions needed = (human players - observers) + AI count
    int numNeeded = (DAT_00a8da84 - observerCount) + DAT_00a8b274;  // DAT_00a8b274 = AI count

    // Use the larger of (needed, available waypoints)
    if (numNeeded < numValidWaypoints) {
        numNeeded = numValidWaypoints;
    }

    // ---- Step 3: Collect valid waypoints into the vector ----
    for (int i = 0; i < numNeeded; i++) {
        // Only add if waypoint index is valid and non-sentinel
        if (i < 702 && i >= 0) {
            CellStruct cell = *(CellStruct*)(DAT_00a8b230 + 0x632 + i * 4);
            if (cell.X != DAT_00b05458 || cell.Y != DAT_00b0545a) {
                // Add to vector (with automatic grow)
                tempVec.Add(cell);

                Debug("Multiplayer start waypoint found at cell %d,%d",
                      cell.X, cell.Y);
            }
        }
    }

    // ---- Step 4: Handle waypoint deficiency ----
    // If we don't have enough positions, generate random ones
    if (numNeeded != tempVec.count && (numNeeded - tempVec.count) >= 0) {
        Debug("Multiplayer start waypoint deficiency - looking for more start positions");

        while (tempVec.count < numNeeded) {
            // Generate random cell coordinates within map bounds
            // DAT_0087f90c..DAT_0087f918 are map boundary globals
            short randY = Random(0, DAT_0087f918 - 10) + 10 + DAT_0087f910;
            short randX = Random(10, DAT_0087f914 - 10) + DAT_0087f90c;
            CellStruct candidate = { randX, randY };

            // FUN_0056dc20: Find passable cell near candidate
            //   params: (out, &candidate, 1, -1, 0, 0, 8, 8, 0, 0, 0, 1, &flag, 0, 0)
            //   Requires 8x8 clearance, passable terrain
            CellStruct validCell;
            FindOpenCell(&validCell, &candidate, /*clearanceW=*/8, /*clearanceH=*/8);

            if (validCell.X != SENTINEL_X || validCell.Y != SENTINEL_Y) {
                tempVec.Add(validCell);
                Debug("Random multiplayer start waypoint added at cell %d,%d",
                      validCell.X, validCell.Y);
            }
        }
    }

    // ---- Step 5: Copy result to output parameter ----
    // Uses DynamicVector copy-assignment (FUN_0068c2b0)
    *result = tempVec;

    // Cleanup temp vector
    tempVec.~DynamicVector();

    return result;
}
```

### Key Observations

1. **Waypoints 0-7 only**: Only the first 8 waypoints are checked as start positions.
   The waypoint array has 702 total slots, but only 0-7 are used for multiplayer spawns.

2. **Early termination**: The waypoint scan breaks on the FIRST sentinel value. So if
   waypoint 3 is invalid, waypoints 4-7 are NOT checked even if they are valid.
   This means map authors must define waypoints contiguously starting from 0.

3. **Deficiency fallback**: When there aren't enough defined waypoints, random positions
   are generated. The random position finder requires 8x8 cell clearance, ensuring
   spawns aren't placed on cliffs or water.

4. **Player count calculation**: The number of positions needed accounts for observers
   (they don't need positions) and includes both human and AI players.

Confidence: HIGH -- decompiled C with debug strings. The sentinel check pattern
(`DAT_00b05458` / `DAT_00b0545a`) is confirmed by 8+ other functions.

---

## 4. SelectSpawnPoint / FUN_005ee6f0 -- Full Annotated Pseudocode

Address: `0x005ee6f0` | Size: 721 bytes | Called by: `AssignStartingPoints` only

This is `__fastcall` (ECX = param_1, stack = rest):

```c
// param_1 = output CellStruct* (return value)
// param_2 = house index being assigned
// param_3 = pointer to spawn pool DynamicVector:
//           +0x04 = data array pointer (CellStruct*, 4 bytes each: packed short X, short Y)
//           +0x10 = count of positions
// param_4 = occupancy boolean array (1 byte per position: 0=free, 1=occupied)
// param_5 = isFirstPlayer flag (1 for human pass, 0 for AI pass)

CellStruct* SelectSpawnPoint(CellStruct* out, int houseIndex,
                             DynVec* pool, char* occupied, char isFirstPlayer) {
    int totalPositions = pool->count;   // *(param_3 + 0x10)

    // ---- Count currently occupied positions ----
    int numOccupied = 0;
    for (int i = 0; i < totalPositions; i++) {
        if (occupied[i] != 0) {
            numOccupied++;
        }
    }

    // ========================================
    // STRATEGY 1: First player, nothing occupied
    // ========================================
    if (numOccupied == 0 && isFirstPlayer != 0) {
        // Pick a completely random position from the entire pool
        int pick = Random(0, totalPositions - 1);   // FUN_0065c7e0

        occupied[pick] = 1;
        ScenarioClass->mp_start_waypoints[pick] = houseIndex;  // DAT_00a8b230+0x1180
        *out = pool->data[pick];
        return out;
    }

    // ========================================
    // STRATEGY 2: Exactly 2 occupied, AI player (isFirstPlayer==0)
    // ========================================
    if (numOccupied == 2 && isFirstPlayer == 0) {
        // Pick a random UNOCCUPIED position
        // Random(0, totalPositions - 3) gives index among free positions
        int pick = Random(0, totalPositions - 3);

        int freeCount = 0;
        for (int i = 0; i < totalPositions; i++) {
            if (occupied[i] == 0) {
                freeCount++;
                if (freeCount == pick + 1) {
                    occupied[i] = 1;
                    ScenarioClass->mp_start_waypoints[i] = houseIndex;
                    *out = pool->data[i];
                    return out;
                }
            }
        }
        // Falls through to strategy 3/4 if somehow fails
    }

    // ========================================
    // STRATEGY 3: Exactly 1 occupied -- MINIMIZE distance
    // ========================================
    if (numOccupied == 1) {
        int bestIndex = -1;
        int bestDist = -1;    // starts at -1 to accept first candidate

        for (int i = 0; i < totalPositions; i++) {
            if (occupied[i] != 0) continue;  // skip occupied positions

            // Sum Euclidean distances to ALL occupied positions
            int totalDist = 0;
            for (int j = 0; j < totalPositions; j++) {
                if (occupied[j] == 0) continue;  // only measure to occupied

                short* coords = pool->data;  // array of packed (short X, short Y)
                short dx = coords[i*2] - coords[j*2];       // X difference
                short dy = coords[i*2+1] - coords[j*2+1];   // Y difference

                double dist = sqrt((double)(dx*dx) + (double)(dy*dy));
                totalDist += (short)dist;   // truncated to short via FUN_007c5f00
            }

            // MINIMIZE: pick position with smallest total distance
            if (totalDist < bestDist || bestDist < 0) {
                bestDist = totalDist;
                bestIndex = i;
            }
        }

        occupied[bestIndex] = 1;
        ScenarioClass->mp_start_waypoints[bestIndex] = houseIndex;
        *out = pool->data[bestIndex];
        return out;
    }

    // ========================================
    // STRATEGY 4: 2+ occupied -- MAXIMIZE distance
    // ========================================
    // (also used when numOccupied==0 and isFirstPlayer==0, though unusual)
    {
        int bestIndex = -1;
        int bestDist = -1;  // will be beaten by any positive value

        for (int i = 0; i < totalPositions; i++) {
            if (occupied[i] != 0) continue;

            int totalDist = 0;
            for (int j = 0; j < totalPositions; j++) {
                if (occupied[j] == 0) continue;

                short* coords = pool->data;
                short dx = coords[i*2] - coords[j*2];
                short dy = coords[i*2+1] - coords[j*2+1];

                double dist = sqrt((double)(dx*dx) + (double)(dy*dy));
                totalDist += (short)dist;
            }

            // MAXIMIZE: pick position with greatest total distance
            if (totalDist > bestDist) {
                bestDist = totalDist;
                bestIndex = i;
            }
        }

        occupied[bestIndex] = 1;
        ScenarioClass->mp_start_waypoints[bestIndex] = houseIndex;
        *out = pool->data[bestIndex];
        return out;
    }
}
```

### The Four Strategies Summarized

| # | Condition | Strategy | Behavior |
|---|-----------|----------|----------|
| 1 | `numOccupied == 0` AND `isFirstPlayer == 1` | **Random** | Pick any position at random |
| 2 | `numOccupied == 2` AND `isFirstPlayer == 0` | **Random unoccupied** | Pick randomly among free positions |
| 3 | `numOccupied == 1` | **Minimize distance** | Pick the CLOSEST free position to occupied ones |
| 4 | `numOccupied >= 2` (general case) | **Maximize distance** | Pick the FARTHEST free position from occupied ones |

### Distance Formula

```
distance(A, B) = sqrt( (Ax - Bx)^2 + (Ay - By)^2 )
```

- Coordinates are `short` (int16) cell coordinates packed as `(X, Y)` in 4 bytes
- `sqrt` via `FUN_004cac40` (standard math library sqrt)
- Result truncated to `short` via `FUN_007c5f00` (float-to-int16 conversion)
- For multi-occupied scoring: distances are SUMMED across all occupied positions

### Strategy 3 Analysis: The "Minimize" Case

This is counterintuitive at first glance. When exactly ONE position is occupied,
the algorithm picks the CLOSEST free position. This makes sense in the context of
the two-pass system:

- **Pass 1** (humans): First human gets random. If only one human, strategy 3 never
  triggers (numOccupied goes from 0 to 1, then AI pass starts).
- **Pass 2** (AI): If there's 1 human and the AI is the next player, numOccupied==1.
  The AI picks the CLOSEST position. But wait -- this is the "minimize" case for
  exactly 1 occupied only. When a second position gets occupied, all subsequent
  players use strategy 4 (maximize).

In practice, this "minimize for 1 occupied" case is unusual and mostly applies to
2-player games where the second player wants a nearby start (not maximally distant).
For 3+ player games, strategy 4 (maximize) dominates.

**CORRECTION/CLARIFICATION from the decompiled code**: Looking at the raw decompilation
more carefully, the code structure is:

```
if (numOccupied == 0 && isFirstPlayer) { STRATEGY 1: random }
if (numOccupied == 2 && !isFirstPlayer) { STRATEGY 2: random unoccupied; goto fallthrough }
// Falls through to:
if (numOccupied != 1) { STRATEGY 4: maximize distance }
else { STRATEGY 3: minimize distance }
```

So the flow is:
- Strategy 2 tries random for the exact case of 2 occupied + AI, but can fall through
- Strategy 3 (minimize) ONLY fires when exactly 1 position is occupied
- Strategy 4 (maximize) is the default for 2+ occupied positions

Confidence: HIGH -- directly from raw Ghidra decompilation with annotated variable tracking.

---

## 5. SetPrimaryCenter (0x0050e000)

Address: `0x0050e000` | Size: 10 bytes

```c
void House::SetPrimaryCenter(HouseClass* this, CellStruct cell) {
    this->BaseCenterPrimary = cell;   // HouseClass+0x5490
}
```

Trivially simple: stores the 4-byte packed cell coordinate at offset +0x5490.

The companion function `GetBaseCenter` (0x0050def0) returns +0x5494 (alternate center)
if valid, otherwise falls back to +0x5490 (primary center).

Confidence: HIGH -- confirmed from decompilation and 4 callers.

---

## 6. ScenarioClass Waypoint Functions

### 6.1 GetWaypoint (0x0068bcc0)

Size: 20 bytes | Called by: 35 functions

```c
void ScenarioClass::GetWaypoint(CellStruct* out, int index) {
    // Waypoint array at ScenarioClass+0x632
    // Each entry is 4 bytes (packed short X, short Y)
    *out = this->waypoints[index];   // *(this + 0x632 + index * 4)
}
```

### 6.2 GetWaypointCell (0x0068bce0)

Size: 25 bytes | Called by: 4 functions

```c
CellClass* ScenarioClass::GetWaypointCell(int index) {
    CellStruct cell = this->waypoints[index];
    return MapClass::GetCellAt(cell);    // FUN_005657a0
}
```

### 6.3 GetWaypointCoord3D (0x0068bd00)

Size: 88 bytes | Called by: 5 functions

```c
CoordStruct ScenarioClass::GetWaypointCoord3D(int index) {
    CellClass* cell = MapClass::GetCellAt(this->waypoints[index]);
    CoordStruct coords = cell->GetCoords();   // vtable+0x48

    // If cell has bridge flag (0x500), add bridge height
    if (cell->flags & 0x500) {
        coords.Z += DAT_00b054bc;   // bridge height offset constant
    }

    return coords;  // {X, Y, Z}
}
```

### 6.4 ClearAllWaypoints (0x0068bd60)

Size: 26 bytes | Called by: 1 function

```c
void ScenarioClass::ClearAllWaypoints() {
    // Fill all 702 waypoint slots with the sentinel value
    for (int i = 0; i < 702; i++) {
        this->waypoints[i] = SENTINEL;   // DAT_00b05458
    }
}
```

The waypoint array has 702 (0x2BE) entries, not 8. Waypoints 0-7 are multiplayer
starts. Waypoints 8+ are used for triggers, AI scripts, etc. The special waypoint
IDs 0x117b-0x1182 (4475-4482 decimal) correspond to the 8 player start positions
in map files using the `<Player @ A>` through `<Player @ H>` naming convention.

### 6.5 IsWaypointValid (0x0068bd80)

Size: 59 bytes | Called by: 8 functions

```c
bool ScenarioClass::IsWaypointValid(int index) {
    if (index < 0 || index > 701) return false;

    CellStruct wp = this->waypoints[index];
    if (wp.X == DAT_00b05458 && wp.Y == DAT_00b0545a) {
        return false;   // sentinel = invalid
    }
    return true;
}
```

### 6.6 SetWaypoint (0x0068bf50)

Size: 18 bytes | Called by: 4 functions

```c
void ScenarioClass::SetWaypoint(int index, CellStruct value) {
    this->waypoints[index] = value;   // direct 4-byte store
}
```

### Waypoint Data Layout

```
ScenarioClass+0x632:  waypoints[702]    -- 4 bytes each = 2808 bytes total
                      Each entry: packed (short X, short Y)
                      Sentinel: (DAT_00b05458, DAT_00b0545a) = "invalid/unused"

ScenarioClass+0x1180: mp_start_waypoints[16]  -- 4 bytes each = 64 bytes
                      Each entry: house_index (int)
                      -1 = unassigned/free
                      Used by AssignStartingPoints for pre-assignment
```

Confidence: HIGH -- 35 callers for GetWaypoint, sentinel pattern confirmed across
8+ functions, waypoint count 702 confirmed from ClearAllWaypoints loop.

---

## 7. ReadWaypointsFromINI (0x0068bdc0)

Size: 207 bytes | Called during scenario loading

```c
void ScenarioClass::ReadWaypointsFromINI(INIClass* ini) {
    // Reads [Waypoints] section
    for (int i = 0; i < 702; i++) {
        char key[16];
        sprintf(key, "%d", i);

        int value = ini->ReadInt("[Waypoints]", key, 0);

        if (value == 0) {
            this->waypoints[i] = SENTINEL;
        } else {
            // Standard RA2 waypoint encoding: value = X + Y*1000
            short x = value % 1000;
            short y = value / 1000;
            this->waypoints[i] = pack(x, y);

            // Mark cell as having a waypoint (flag 0x4 at CellClass+0x140)
            CellClass* cell = MapClass::GetCellAt(this->waypoints[i]);
            cell->flags_140 |= 0x4;
        }
    }
}
```

The companion `WriteWaypointsToINI` (0x0068be90) reverses this: converts `(X, Y)`
back to `X + Y*1000` format.

---

## 8. SessionClass Structure

`DAT_00a8b230` is actually the **ScenarioClass** singleton pointer (confirmed by
constructor at 0x006832c0 which allocates 0x3740 bytes). The SessionClass is a
separate singleton that stores multiplayer lobby state.

### SessionClass Game Mode (DAT_00a8b238)

This is the game type / session type:

| Value | Mode | Description |
|-------|------|-------------|
| 0 | Campaign | Single-player campaign missions |
| 1 | Serial/Modem | Direct serial/modem connection (NullModemClass) |
| 2 | LAN (IPX) | IPX network LAN game |
| 3 | LAN (UDP) | UDP/TCP LAN game |
| 4 | WOL/Internet | Westwood Online / Internet game |
| 5 | Skirmish | Offline skirmish vs AI |

Confirmed from multiple decompiled functions:
- Report 012: "DAT_00a8b238 -- game mode (0=singleplayer, nonzero=multiplayer, 5=skirmish)"
- Report 041: "DAT_00a8b238 = game mode/state"
- Report 083: "DAT_00a8b238 -- Network mode (3 = LAN/IPX, 4 = WOL/Internet)"
- Report 109: Skirmish check `DAT_00a8b238 == 5`, campaign check `DAT_00a8b238 == 0`
- Report 112: `ProcessRandomAssignments` uses mode to select delegate functions

### Lobby Option Globals

These are stored near DAT_00a8b230 as part of the session/game state area:

| Address | Name | Description |
|---------|------|-------------|
| DAT_00a8b230 | ScenarioClass* (Scen) | Master scenario singleton pointer |
| DAT_00a8b238 | SessionClass::GameMode | See table above |
| DAT_00a8b23c | GameModeObject* | Pointer to game mode object (vtable-driven) |
| DAT_00a8b250 | MapIndex | Currently selected map index |
| DAT_00a8b254 | ScenarioIndex | Currently selected scenario index |
| DAT_00a8b258 | Bases | Bases=yes option (ConYard/MCV enabled) |
| DAT_00a8b270 | UnitCount | Starting unit count setting |
| DAT_00a8b274 | AIPlayerCount | Number of AI players |
| DAT_00a8b278 | Difficulty | Difficulty setting (skirmish) |

### Player Slot Arrays

8 parallel arrays for multiplayer lobby slot data:

| Address | Size | Content |
|---------|------|---------|
| DAT_00a8da90[8] | 4 bytes each | Player slot state (sentinel value OR connection object pointer) |
| DAT_00a8b27c[8] | 4 bytes each | AI difficulty index per slot (0=Hard, 1=Normal, 2=Easy) |
| DAT_00a8b29c[8] | 4 bytes each | Country/side selection per slot (-1=none, -2=random, -3=observer) |
| DAT_00a8b2bc[8] | 4 bytes each | Color selection per slot (0-7, -2=random) |
| DAT_00a8b2dc[8] | 4 bytes each | Team selection per slot |
| DAT_00a8b2fc[8] | 4 bytes each | Start location per slot |

### Slot State Sentinel Values

| Address | Sentinel | State |
|---------|----------|-------|
| DAT_00ac119c | value at addr | Open (accepts players) |
| DAT_00ac11a0 | value at addr | Closed |
| DAT_00ac11a4 | value at addr | AI Easy |
| DAT_00ac11a8 | value at addr | AI Normal |
| DAT_00ac11ac | value at addr | AI Hard |
| DAT_00ac11b0 | value at addr | Waiting (player joining) |
| DAT_00ac11b4 | value at addr | Open Observer |

Any other non-sentinel value in `DAT_00a8da90[slot]` is a pointer to the player's
connection/network object.

### Player List Globals

| Address | Content |
|---------|---------|
| DAT_00a8da78 | Pointer to human player connection object array |
| DAT_00a8da84 | Human player count |
| DAT_00a8da90[8] | Player slot states (see above) |
| DAT_00a8b394 | Local player's color index |
| DAT_00a8da8c | Local player's connection object pointer |

### Team Storage

Teams are stored per-slot in `DAT_00a8b2dc[8]`. The team index is a simple integer
(0 = no team, 1-4 = team number). Teams are enforced during gameplay via the alliance
system (FUN_004f9b70 = force ally).

For WOL clan/squad games, teams are parsed from comma-separated squad strings at
`DAT_00b77a04` (Squad 1) and `DAT_00b76568` (Squad 2) by `FUN_0069b170`.

Confidence: HIGH -- all addresses cross-referenced across 10+ decompiled functions
and reports.

---

## 9. ScenarioClass Spawn-Related Field Map

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| +0x632 | 2808 | waypoints[702] | All waypoints (4 bytes each: packed short X, short Y) |
| +0x112c | 4 | StartX | Map visible area left edge |
| +0x1130 | 4 | StartY | Map visible area top edge |
| +0x1134 | 4 | Width | Map visible area width |
| +0x1138 | 4 | Height | Map visible area height |
| +0x113c | 4 | NumberStartingPoints | Count of defined spawn waypoints |
| +0x1140 | 128 | headerWaypoints[16] | Header waypoints from [Header] section (8 bytes each) |
| +0x1180 | 64 | mp_start_waypoints[16] | House-to-position assignment table (4 bytes each) |
| +0x11e4 | 4 | NumCoopHumanStartSpots | Co-op specific start position count |

### Sentinel Values

| Global | Value | Purpose |
|--------|-------|---------|
| DAT_00b05458 | (short) | Waypoint X sentinel -- marks "unused/invalid" waypoint |
| DAT_00b0545a | (short) | Waypoint Y sentinel -- paired with X sentinel |
| DAT_00b054bc | (int) | Bridge height offset (added to Z for bridge waypoints) |

---

## 10. Interaction with Generate_Random_Units (0x006886b0)

When `Bases=yes`, `Generate_Random_Units` is called BEFORE `AssignStartingPoints`.
It has its own position assignment logic:

```c
void Generate_Random_Units() {
    DynamicVector<CellStruct> positions;
    Gather_Start_Positions(&positions);  // same function!

    for (each house) {
        if (first house) {
            // Pick random position
            int pick = Random(0, positions.count - 1);
            assignedPosition = positions.data[pick];
        } else {
            // Pick position FARTHEST from all already-assigned positions
            // Euclidean distance with sqrt, maximize sum-of-distances
            int bestIdx = -1;
            int bestDist = -1;
            for (each unassigned position) {
                int totalDist = sum of sqrt(dx^2+dy^2) to all assigned positions;
                if (totalDist > bestDist) {
                    bestDist = totalDist;
                    bestIdx = positionIndex;
                }
            }
            assignedPosition = positions.data[bestIdx];
        }

        House::SetPrimaryCenter(house, assignedPosition);  // FUN_0050e000
        // Create MCV, deploy if MCVDeploy flag set, create starting units...
    }
}
```

This is SIMILAR to the `SelectSpawnPoint` algorithm but not identical:
- `Generate_Random_Units` always maximizes distance (no minimize case)
- `SelectSpawnPoint` has the special minimize-for-1-occupied case
- `Generate_Random_Units` does NOT use the pre-assignment table (+0x1180)
- Both use the same `Gather_Start_Positions` function for waypoint collection

---

## Confidence Summary

| Component | Confidence | Basis |
|-----------|-----------|-------|
| AssignStartingPoints full flow | HIGH | Raw decompiled C + debug strings |
| Gather_Start_Positions full flow | HIGH | Raw decompiled C + debug strings |
| SelectSpawnPoint 4 strategies | HIGH | Raw decompiled C, all branches traced |
| Distance formula (Euclidean) | HIGH | sqrt call + packed coordinate math visible |
| Waypoint accessors (6 functions) | HIGH | 35+ callers, trivial implementations |
| Pre-assignment table at +0x1180 | HIGH | Used by both AssignStartingPoints and GetMultiplayerStartingWaypoint |
| SessionClass game modes | HIGH | Cross-referenced across 10+ reports |
| Player slot arrays | HIGH | Confirmed from lobby UI code + network serialization |
| Human-first/AI-second ordering | HIGH | Two separate loops in decompiled code with IsHuman check |
| Waypoint sentinel values | HIGH | Used consistently across 8+ functions |
| ScenarioClass size 0x3740 | HIGH | Constructor allocation + serialization confirm |
