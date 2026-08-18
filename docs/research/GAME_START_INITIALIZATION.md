# Game Start Initialization — gamemd.exe Ghidra Analysis

Complete reverse-engineering of how gamemd.exe sets up a new game: player creation,
color/side assignment, spawn point allocation, MCV deployment, and starting credits.

Sources: Reports 047, 048, 051, 056, 057, 085, 086, 105, 109, 110, 112 from
`<local>/Documents/gidra/reports/`, and `docs/MCV_DEPLOY_GHIDRA_REPORT.md`,
`docs/HOUSECLASS_GHIDRA_REPORT.md`.

---

## Overview: The Full Initialization Pipeline

When a game starts, the engine executes a deep call chain. Here is the complete
sequence from top to bottom:

```
Main_Game() [0x0052d9a0]
  → ScenarioClass::Start_Scenario() [0x00683ab0]
      → ScenarioClass::Read_Scenario() [0x00684620]
          → ScenarioClass::Read_Scenario_INI() [0x00686730]
              → ScenarioClass::Full_Init() [0x00686b20]    ← THE MONSTER FUNCTION (4.5KB)
                  Phase 1: Clear_All() [0x006851f0]
                  Phase 2: Game mode setup, difficulty, flags
                  Phase 3: Map header parsing (StartX/Y, Width, Height, NumberStartingPoints)
                  Phase 4: Theater setup, terrain, overlays, triggers
                  Phase 5: Create_Houses() [0x00687f10]    ← PLAYER CREATION
                  Phase 5b: AssignStartingPoints() [0x005ee9d0]  ← SPAWN ASSIGNMENT (called from Full_Init, NOT from Post_Map_Init; conditioned on DAT_00a8b244==2; corrected 2026-05-29: was listed under Post_Map_Init; binary shows single caller ScenarioClass__Full_Init via get_function_callers 0x005ee9d0 — OPERATOR_OR_ORDER_DRIFT)
                  Phase 6: Object loading (buildings, units, infantry, aircraft from map INI)
                  Phase 7: Post_Map_Init() [0x00686890]    ← STARTING UNITS + CREDITS
                      → Generate_Random_Units() [0x006886b0]
                  Phase 8: Post_Load_Init() [0x00684c30]   ← TERRAIN + VISION + AI
```

**Confidence: HIGH** — sourced from multiple decompiled functions with debug strings
confirming file/function names.

---

## 1. Spawn Positions: How the Game Reads Them from the Map

### ScenarioClass Waypoint Array

- **Location**: `ScenarioClass + 0x632` (702 entries, each 4 bytes = packed cell coordinate)
- **Sentinel value**: `DAT_00b05458` / `DAT_00b0545a` (marks "unused" waypoints)
- **Waypoints 0–7** are the 8 multiplayer start positions (max 8 players)
- **Size**: ScenarioClass is 0x3740 bytes (14,144 bytes total)

### Map INI [Header] Section Parsing (0x00689e90)

The `ScenarioClass::ReadINI` function reads:

| INI Key | ScenarioClass Offset | Description |
|---------|---------------------|-------------|
| `StartX` | +0x112c | Map visible area left edge |
| `StartY` | +0x1130 | Map visible area top edge |
| `Width` | +0x1134 | Map visible area width |
| `Height` | +0x1138 | Map visible area height |
| `NumberStartingPoints` | +0x113c | How many spawn positions are defined |
| `NumCoopHumanStartSpots` | +0x11e4 | Co-op specific human start count |
| `Waypoint0`..`Waypoint7` | +0x1140 + i*8 | Starting positions (pair of 4-byte values per waypoint) |

Each waypoint is read with `sprintf("Waypoint%d", i)` and stored as a coordinate pair.

### Waypoint Accessors

| Address | Function | Description |
|---------|----------|-------------|
| 0x0068bcc0 | GetWaypoint | Core accessor: `*out = this->waypoints[index]` |
| 0x0068bce0 | GetWaypointCell | Returns CellClass* for waypoint |
| 0x0068bd00 | GetWaypointCoord3D | Returns (X, Y, Z) with bridge height adjustment |
| 0x0068bd60 | ClearAllWaypoints | Fills all 702 slots with sentinel |
| 0x0068bd80 | IsWaypointValid | Returns 1 if in range 0–701 and != sentinel |
| 0x0068bf50 | SetWaypoint | Stores coordinate at index |

**Confidence: HIGH** — 35 callers for GetWaypoint, sentinel values confirmed.

---

## 2. Player-to-Position Assignment

### Pre-Game: Random Assignment Resolution (0x0069b8c0)

Before houses are created, `ProcessRandomAssignments()` resolves all "Random" choices:

```
For each human player (DAT_00a8da78 array, count DAT_00a8da84):
  If side == -3 (observer): side=-3, color=8, both flags=-1
  If side == -2 (random):
    Generate random side via FUN_0065c7e0(0, 9)  // 0-9 = 10 countries
  If color == -2 (random):
    Loop: generate random color 0-7 via FUN_0065c7e0(0, 7)
    Check FUN_0069b600 (is color already taken?)
    Repeat until unique color found
  Store player 0's color in DAT_00a8b394 (local player color)
  Log "Player %i, %s: Side = %i, Color = %i"

For each AI player (DAT_00a8b29c, 8 slots):
  Same random side/color resolution with collision avoidance
  Log "AI %i: Side = %i, Color = %i"
```

**Key data structures for lobby slots**:

| Address | Content |
|---------|---------|
| DAT_00a8da90[8] | Player slot state pointers (sentinel or connection object) |
| DAT_00a8b27c[8] | AI difficulty index per slot |
| DAT_00a8b29c[8] | Country/side selection per slot |
| DAT_00a8b2bc[8] | Color selection per slot |
| DAT_00a8b2dc[8] | Team selection per slot |
| DAT_00a8b2fc[8] | Start location per slot |

**Sentinel values for slot states** (at DAT_00ac119c–DAT_00ac11b4):

| Global | Slot State |
|--------|-----------|
| DAT_00ac119c | Open |
| DAT_00ac11a0 | Closed |
| DAT_00ac11a4 | AI Easy |
| DAT_00ac11a8 | AI Normal |
| DAT_00ac11ac | AI Hard |
| DAT_00ac11b0 | Waiting |
| DAT_00ac11b4 | Open Observer |

### In-Game: Gather_Start_Positions (0x00688380)

Collects available start positions:

1. Iterates waypoints 0–7, counts non-sentinel entries
2. Counts how many players need start positions (total players + AI minus observers)
3. For each valid waypoint, adds cell coordinate to output vector
4. If not enough waypoints defined:
   - Logs `"Multiplayer start waypoint deficiency - looking for more start positions"`
   - Generates random positions using `FUN_0056dc20` (find open cell with 8x8 clearance)
   - Logs `"Random multiplayer start waypoint added at cell %d,%d"`

### In-Game: AssignStartingPoints (0x005ee9d0)

Called once during map initialization:

```
1. Call Gather_Start_Positions() to get spawn pool
2. Mark which positions are already occupied (-1 = free)
3. For each HUMAN house (flag +0x1ec != 0, not spectator +0x1a6 == 0):
   - Check if pre-assigned in the 16-slot table at DAT_00a8b230+0x1180
   - If found: use directly via FUN_0050e000 (SetPrimaryCenter)
   - If not: call FUN_005ee6f0 to pick a free position (preference flag = 1)
   - Log "Starting point for house %d is x=%d y=%d"
4. Second pass for AI houses (+0x1ec == 0):
   - Same mechanism but with preference flag = 0
```

### Spawn Point Selection Algorithm (0x005ee6f0)

Four strategies depending on how many positions are already occupied:

| Situation | Strategy |
|-----------|----------|
| First player, no occupied points | Random from all available |
| Second player, exactly 2 total | Random from unoccupied |
| 1 occupied (general case) | **Minimize** total distance to occupied (closest) |
| 2+ occupied (general case) | **Maximize** total distance to occupied (farthest) |

Distance: Euclidean `sqrt(dx*dx + dy*dy)` on packed (short X, short Y) coordinates.

### Alternative: Generate_Random_Units (0x006886b0) — Skirmish Start Position

In skirmish/multiplayer (called from `Post_Map_Init`):

```
1. First house gets a random position from the pool
2. Subsequent houses get the position FARTHEST from all already-assigned positions
   (maximizing inter-player distance — the classic "fair start" placement)
3. Distance uses Euclidean distance with sqrt (FUN_004cac40)
4. Each assigned position calls FUN_0050e000 (SetPrimaryCenter)
```

**Confidence: HIGH** — debug strings and log messages confirm this flow.

---

## 3. Starting Credits

### Per-House Credit Setup

Credits are set during house creation via `FUN_004fce00` (House::Set_Credits_And_Color):

```c
void Set_Credits_And_Color(HouseClass* this, int sideId, int countryId, int credits) {
    this->StartingCredits = credits;  // +0x1dc
    this->AvailableCredits = credits; // +0x30c
    // Also sets color from sideId/countryId
}
```

### Credit Sources

**Multiplayer/Skirmish**: Credits come from the lobby setting.
- Read from `[MultiplayerDialogSettings]` in rules: `Money` (default), `MinMoney`, `MaxMoney`, `MoneyIncrement`
- Stored in DAT_00a8b258-area globals
- Passed to `Set_Credits_And_Color` during house creation

**Campaign/Single-player**: Credits come from the map INI.
- `[HouseName] Credits=` → HouseClass+0x1dc
- **Internal scaling**: INI `Credits=10000` → internal value 1,000,000 (multiplied by 100)
- Per `House::Read_Scenario_INI` at 0x500b40

**Difficulty bonus** (single-player only, when `PlayerControl=yes`):
- Easy (DAT_00a8eb64=0): credits += `RulesClass+0xdfc` (CampaignMoneyDeltaEasy)
- Hard (DAT_00a8eb64=2): credits += `RulesClass+0xe00` (CampaignMoneyDeltaHard)
- Medium: no bonus
- Credits clamped to >= 0

### HouseClass Credit Offsets

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x1dc | 4 | StartingCredits | INI `Credits=` × 100 |
| +0x30c | 4 | AvailableCredits | Current balance |
| +0x2dc | 4 | TotalCreditsSpent | Cumulative tracker |
| +0x310 | 4 | TrackedTiberiumBalance | Ore/tiberium accounting |

**Confidence: HIGH** — confirmed from multiple decompiled functions and INI parsing.

---

## 4. Initial MCV Creation and Deployment

### MCV Creation at Game Start (0x006886b0)

The `Generate_Random_Units` function handles MCV creation:

```
For each player house:
  1. Assign start position (see Section 2)
  2. If Bases=yes (DAT_00a8b258):
     a. Look up BaseUnit type from RulesClass (parsed from [General] BaseUnit=)
     b. Create MCV unit: FUN_007353C0(BaseUnitType, House)
     c. Place at starting cell coordinates
     d. Call Place(coords) — vtable+0xD8
     e. If placement fails: FUN_00688ED0 (find alternate cell with spiral search)
     f. Set as house's primary center via FUN_0050e000
     g. If MCVDeploy special flag is ON (bit 8 of SpecialFlags at DAT_00a8b230):
        Call FUN_004FC060 to force immediate deploy
  3. Create remaining starting units (from UnitCount option)
```

### BaseUnit Lookup

- `BaseUnit` is read from `[General]` section in rulesmd.ini
- Stored in RulesClass at an offset within the [General] parser (FUN_0066d530)
- Typically points to the MCV unit type (AMCV/SMCV/YMCV depending on side)

### MCV Auto-Deploy Flag

| Bit | Global | Flag | Effect |
|-----|--------|------|--------|
| 8 | DAT_00a8b230 (SpecialFlags) | `MCVDeploy` | Forces immediate MCV→ConYard at game start |

When set, the game creates an MCV unit then immediately calls the deploy function,
which creates a Construction Yard building at that cell. This is the standard behavior
in most multiplayer games.

### Starting Unit Generation

After MCV placement, additional starting units are generated:

```
1. Compute average unit cost across all InfantryType and VehicleType objects
2. Total budget = average_cost × configured_unit_count (DAT_00a8b270)
   (minus 1 if MCV was deployed, since MCV counts as one unit)
3. Build list of valid unit types filtered by:
   - Spawnable flag (TypeClass+0x6d5)
   - Tech level (TypeClass+0x634 <= house tech level)
   - House allowed mask (TypeClass+0x6cc & house bit)
   - Not in ForbiddenHouses list
4. Distribution: 2/3 from infantry/vehicle types, 1/3 from naval/aircraft
5. For each unit:
   - Create via vtable+0x8c (CreateObject)
   - Find placement via FUN_0040dd70 (Find_Place)
   - Place via FUN_00688ed0 (spiral search around start position)
   - Log "House %s deployed object %s"
```

### Spiral Placement Algorithm (0x00688ed0)

When placing units near a start position:

1. Try exact target cell first (check passability + occupancy)
2. If occupied, spiral search: 8 directions × increasing radii (1 to 31 cells)
3. After first pass, add random jitter (0–1 cells per axis)
4. Check passability via FUN_00578460
5. Check occupancy via FUN_0047c3d0
6. Returns 1 on success, 0 if no valid position within radius 31

**Confidence: HIGH** — debug strings and clear decompilation.

---

## 5. Player/House Creation

### Create_Houses (0x00687f10) — The Player Factory

Called from `Full_Init` during scenario loading:

```
Phase 1: Fix broken assignments
  - Iterate player slots
  - For any slot with country index -3 (observer/unassigned): call FUN_00696f90(0)

Phase 2: Create human player houses (sorted by priority)
  - Iterate player list, pick lowest priority value (+0x53) first
  - For each: allocate HouseClass (0x160B8 bytes = ~90KB)
  - Set name:
    - WOL mode: read from WOL player info structure
    - Other modes: "<human player>"
  - Copy internal name to house+0x1602a (max 21 bytes)
  - Set human flag: +0x1ec = 1
  - Call FUN_004fce00 (init with side ID, country ID, starting credits)
  - Call FUN_0069a310 (map priority to color scheme)
  - First player (local_4c == 0):
    - Becomes DAT_00a83d4c (the local player pointer)
    - Gets +0x1ed = 1 (PlayerControl flag)
  - Player with spawn position -1:
    - Becomes DAT_00ac1198 (spectator/special house)

Phase 3: Create AI player houses
  - Iterate 8 AI slots at DAT_00a8b29c
  - For each valid entry (country != -1 and != -3):
    - Allocate HouseClass (0x160B8 bytes)
    - Set name: "Computer"
    - Set display name: "TXT_COMPUTER"
    - Set +0x1ec = 0 (AI-controlled)
    - If allies bonus enabled and player count > 1: adjust difficulty

Phase 4: Create special houses
  - "Neutral" house (using FUN_005117d0 to find country index)
  - "Special" house (same lookup)
  - Color for neutral: "LightGrey"
```

### HouseClass Constructor (0x4f54a0) — Full Init

The full constructor (~90KB object):

1. Initializes ~12 DynamicVectorClass arrays (capacity 10 each)
2. Sets house index from `DAT_00a80238` (global house count)
3. Copies difficulty params from RulesClass scaled by house type modifiers
4. Registers `this` in 5 global arrays:
   - DAT_00b0f674, DAT_00b0f644, DAT_00b0f5f4, DAT_00b0f61c, DAT_00b0f724
   - And the master array at DAT_00a8022c
5. Cross-registers with each existing house's diplomacy arrays
6. Copies player name (20 chars)
7. Initializes ally bitfield at offset 0x76
8. Creates CellSpreadClass (0x34 bytes)
9. Zeros 0x4204-dword visibility/shroud array at offset 0x15f9
10. Sets veteran/elite rank percentages (0x4b = 75%)

### HouseClass Identity Offsets

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x30 | 4 | HouseIndex | Player slot 0–31 |
| +0x34 | 4 | HouseTypeClass* | Pointer to CountryTypeClass |
| +0x1d4 | 4 | TechLevel | From map `[HouseName] TechLevel=` |
| +0x1e0 | 4 | StartingEdge | Map edge index for spawning |
| +0x1e8 | 4 | SideIndex | 0=Allied, 1=Soviet, 2=Yuri |
| +0x1ec | 1 | IsHuman | Human-controlled flag |
| +0x1ed | 1 | PlayerControl | Local player flag |
| +0x15ff4 | 20 | PlayerName | 20-char name string |
| +0x16009 | 32 | UIName | Localized display name |
| +0x16054 | 4 | ColorSchemeIndex | Color assignment |
| +0x5490 | 4 | BaseCenterPrimary | CellStruct for base center |
| +0x5494 | 4 | BaseCenterAlternate | Alternate base center |

**Confidence: HIGH** — constructor decompiled, field offsets cross-referenced.

---

## 6. Color Assignment

### Color Flow

1. **Lobby**: Each player selects a color (0–7) or "Random" (-2)
2. **Pre-game**: `ProcessRandomAssignments` (0x0069b8c0) resolves randoms:
   - Generates candidates via `FUN_0065c7e0(0, 7)` (random int 0–7)
   - Checks `FUN_0069b600` for collision with existing assignments
   - Loops until unique color found
   - Observer gets color index 8

### Color Assignment Functions

| Address | Function | Description |
|---------|----------|-------------|
| 0x0069b600 | IsColorTaken | Scans AI list (+0x2840/+0x284C) and human slots (+0x84) |
| 0x0069b690 | GenerateUniqueColor | Random 0–7, loop until unique |
| 0x0069b7e0 | SetColorWithCollision | Sets color with collision avoidance |
| 0x0069b8c0 | ProcessRandomAssignments | Resolves all random side/color for all players and AI |

### Color Application to HouseClass

During `Create_Houses`, `FUN_004fce00` applies the color:
- Stores at `HouseTypeClass+0xc0` and `HouseClass+0x16054`

During `House::Read_Scenario_INI` (0x500b40), color is re-read:
- `[HouseName] Color=` → +0x16054
- Forces WHITE=5 if invalid (with debug warning)
- Reads palette scheme from `DAT_00b054d4[ColorSchemeIndex]`
- Extracts RGB, computes brightness normalization
- Stores base color at +0x56f9, bright remap at +0x56fc

### [Colors] and [ColorAdd] Sections

From rules parsing (0x0066d3a0, 0x0066d480):
- `[Colors]` defines named colors as RGB triplets
- `[ColorAdd]` defines per-color brightness adjustments (16 entries, 3 bytes each at RulesClass+0x1874)
- Colors are registered in the global color system via FUN_00626ab0 and FUN_0068c9c0

**Confidence: HIGH** — multiple cross-references confirm the flow.

---

## 7. Post-Map Initialization (0x00686890)

After all map data is loaded and houses created:

```
1. If total players + observers < max players in rules, and not in editor mode:
   Add AI players where needed

2. If network manager exists:
   Call network init methods (+0x84, +0x88, +0x8c)
   Call FUN_005d6d80 (sync state)
   Otherwise: call Generate_Random_Units (0x006886b0) — starting MCV + units

3. If TechShare flag is set:
   Create shared starting tech units

4. Per-house final setup:
   - Set power grid
   - Set starting money from rules
   - Set allies/enemies flags
   - Set +0x5774 to own pointer (self-reference for ownership)
   - Set +0x5640/+0x5644/+0x5648 (timestamp + starting tech level)

5. Set up neutral house visibility flags

6. Call FUN_0068c050 and set bitfield for all houses initialized
```

---

## 8. Key Globals Reference

| Global Address | Name | Purpose |
|---------------|------|---------|
| DAT_00a8b230 | ScenarioClass* (Scen) | Master scenario singleton (0x3740 bytes) |
| DAT_008871e0 | RulesClass* | Game rules singleton (0x18c0 bytes) |
| DAT_00a8022c | HouseClass::Array | Array of all HouseClass pointers |
| DAT_00a80238 | HouseClass::Array.Count | Number of active houses |
| DAT_00a83d4c | PlayerPtr | Pointer to local player's HouseClass |
| DAT_00ac1198 | HouseClass::Observer | Spectator/observer house |
| DAT_00a8b238 | SessionClass::GameMode | 0=campaign, 1-4=multiplayer, 5=skirmish |
| DAT_00a8b258 | Bases option | Bases=yes enables ConYard placement |
| DAT_00a8b270 | UnitCount | Starting unit count setting |
| DAT_00a8b394 | LocalPlayerColor | Local player's color index |
| DAT_00b054d4 | ColorSchemeArray | Array of color scheme pointers |
| DAT_00a8da78 | PlayerList | Array of human player connection objects |
| DAT_00a8da84 | PlayerList.Count | Number of human players |
| DAT_00a8da90[8] | PlayerSlots | Per-slot state (sentinel or connection ptr) |
| DAT_00a8b29c[8] | SlotCountry | Country selection per slot |
| DAT_00a8b2bc[8] | SlotColor | Color selection per slot |
| DAT_00a8b2dc[8] | SlotTeam | Team selection per slot |
| DAT_00a8ed84 | Scen->Frame | Current game frame number |
| DAT_00a8ed94 | RandomSeed | Pre-agreed multiplayer random seed |

---

## 9. [MultiplayerDialogSettings] — Game Lobby Options

Parsed from rules at 0x00671ea0:

| INI Key | Default | Description |
|---------|---------|-------------|
| MinMoney | — | Minimum credits slider value |
| Money | — | Default credits setting |
| MaxMoney | — | Maximum credits slider value |
| MoneyIncrement | — | Credits slider step |
| MinUnitCount | — | Minimum starting unit count |
| UnitCount | — | Default starting unit count |
| MaxUnitCount | — | Maximum starting unit count |
| TechLevel | — | Starting tech level |
| GameSpeed | — | Default game speed |
| AIDifficulty | — | Default AI difficulty |
| AIPlayers | — | Default number of AI players |
| BridgeDestruction | — | Can bridges be destroyed |
| Bases | — | Starting ConYard/MCV enabled |
| Crates | — | Random crates on map |
| ShortGame | — | Short game victory condition |
| SuperWeaponsAllowed | — | Allow superweapons |
| MCVRedeploys | — | Allow ConYard→MCV undeploy |
| MultiEngineer | — | Engineers capture at any health |
| BuildOffAlly | — | Build adjacent to ally buildings |
| FogOfWar | — | Enable fog of war |
| ShadowGrow | — | Shroud regrows |
| Shroud | — | Enable shroud |
| TiberiumGrows | — | Ore regeneration |
| HarvesterTruce | — | Harvesters are immune |
| AlliesAllowed | — | Alliance system enabled |
| AllyChangeAllowed | — | Can change alliances mid-game |
| CaptureTheFlag | — | CTF game mode |

---

## 10. Complete Initialization Order Summary

```
1. Main_Game selects game mode (skirmish/LAN/WoL/campaign)
2. ProcessRandomAssignments resolves random side/color for all players
3. Start_Scenario called with scenario filename
4. Read_Scenario opens and parses the map INI file
5. Full_Init (the 4.5KB monster function):
   a. Clear_All — destroy previous game state
   b. Parse [Basic] section (InitTime, Official, etc.)
   c. Parse [Header] section (map bounds, waypoint positions)
   d. Load theater (TEMPERAT/SNOW/URBAN MIX + palette)
   e. Load terrain, overlays, triggers from map INI
   f. Create_Houses — create HouseClass for each human + AI player
      - Human players sorted by priority, first = local player
      - AI players from 8-slot array
      - Neutral + Special houses always created
   g. AssignStartingPoints (human first, then AI) — called from Full_Init before Post_Map_Init,
      conditioned on DAT_00a8b244==2 (corrected 2026-05-29: was listed inside Post_Map_Init;
      binary shows single caller ScenarioClass__Full_Init — OPERATOR_OR_ORDER_DRIFT)
   h. Load objects (buildings, units, infantry, aircraft from map INI)
   i. Post_Map_Init:
      - If Bases=yes: Generate_Random_Units
        - Gather start positions from waypoints 0-7
        - Assign positions (random for first, maximize distance for rest)
        - Create MCV at each player's start position
        - If MCVDeploy flag: force immediate deploy to ConYard
        - Create additional starting units (from UnitCount setting)
      - Set starting credits, tech level, power grid per house
   i. Post_Load_Init:
      - Initialize per-house economy
      - Center view on player's start position (campaign)
      - Initialize AI threat evaluation
      - Terrain recalc, shadow recalc, subsystem inits
      - Create ambient particle systems
   j. Wait_For_Players (multiplayer only — sync all players)
6. Game timer starts, first tick begins
```

---

## Confidence Summary

| Area | Confidence | Source |
|------|-----------|--------|
| Waypoint reading from map INI | HIGH | Decompiled FUN_00689e90 with string refs |
| Spawn point assignment algorithm | HIGH | Decompiled FUN_005ee6f0 + FUN_00688380 |
| Player creation (Create_Houses) | HIGH | Decompiled FUN_00687f10 with string refs |
| Color assignment flow | HIGH | Decompiled FUN_0069b8c0 with logging |
| Starting credits setup | HIGH | Decompiled FUN_004fce00 + FUN_00500b40 |
| MCV creation and deploy | HIGH | Decompiled FUN_006886b0, confirmed by MCV_DEPLOY report |
| Starting unit generation | HIGH | Decompiled FUN_006886b0 with debug strings |
| Spiral placement algorithm | HIGH | Decompiled FUN_00688ed0, clear algorithmic structure |
| Full_Init call sequence | HIGH | Decompiled FUN_00686b20, 30+ sub-calls confirmed |
| BaseUnit RulesClass offset | MEDIUM | Confirmed parsed from [General] but exact offset not pinned |
| HouseClass size 0x160B8 | HIGH | Confirmed from operator_new allocation |
| ScenarioClass size 0x3740 | HIGH | Confirmed from constructor + serialization |
