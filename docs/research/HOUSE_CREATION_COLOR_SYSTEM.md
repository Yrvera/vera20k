# House/Player Creation and Color System -- Ghidra Deep Dive

Source: `D:\ra2mdpost\Scenario.CPP`, `D:\ra2mdpost\House.CPP`, `D:\ra2mdpost\Session.CPP`

This document covers the full pipeline from lobby player slots to in-game HouseClass
instances, including color scheme resolution, priority sorting, and the random
assignment system.

---

## Table of Contents

1. [Key Data Structures](#key-data-structures)
2. [ProcessRandomAssignments (0x0069b8c0)](#processrandomassignments-0x0069b8c0)
3. [Create_Houses (0x00687f10)](#create_houses-0x00687f10)
4. [HouseClass Constructor (0x004f54a0)](#houseclass-constructor-0x004f54a0)
5. [Set_Credits_And_Color (0x004fce00)](#set_credits_and_color-0x004fce00)
6. [Color Scheme Resolution Pipeline](#color-scheme-resolution-pipeline)
7. [House::Read_Scenario_INI (0x00500b40)](#houseread_scenario_ini-0x00500b40)

---

## Key Data Structures

### NodeNameTag (Player Node) -- Size 0x85 (133 bytes)

Each human player in the lobby has a NodeNameTag allocated via `operator_new(0x85)`.
This struct is what `DAT_00a8da78[]` points to (one per human player, count in
`DAT_00a8da84`).

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x00 | ~52 | PlayerName | Network player name string |
| +0x28 | 12 | NetworkAddress | IP/port (3 DWORDs) |
| +0x34 | 1 | IsActive | Has-joined flag |
| +0x35 | 4 | LastSeenFrame | Timeout tracking |
| +0x4b | 4 | CountryIndex | Side/faction selection (index into HouseTypeClass array) |
| +0x4f | 4 | CountryRandom | -2 = random, -1 = assigned. Guards CountryIndex |
| +0x53 | 4 | ColorIndex | Color scheme index (0-7) |
| +0x57 | 4 | ColorRandom | -2 = random, -1 = assigned. Guards ColorIndex |
| +0x5b | 4 | TeamIndex | Team/alliance group |
| +0x5f | 4 | TeamRandom | -2 = random, -1 = assigned |
| +0x63 | 4 | SpawnLocation | Start position on map (0-7, or -1) |
| +0x67 | 4 | SpawnRandom | -2 = random, -1 = assigned |
| +0x6b | 4 | ObserverFlag | -1 = observer, other = player slot index |
| +0x6f | 4 | HouseIndex | Set AFTER HouseClass created -- stores HouseClass.Index |
| +0x77 | 4 | ReadyState | 0=not ready, 1=accepted, 2=host |
| +0x7f | 4 | HasSentOptions | Network sync flag |

The getter/setter functions confirm this layout:

```c
// FUN_00696f10 -- GetCountry(node)
int GetCountry(NodeNameTag* node) {
    if (node->CountryRandom == -2) return -2;  // "Random"
    return node->CountryIndex;
}

// FUN_00696f20 -- GetColor(node)
int GetColor(NodeNameTag* node) {
    if (node->ColorRandom == -2) return -2;    // "Random"
    return node->ColorIndex;
}

// FUN_00696f50 -- GetTeam(node)
int GetTeam(NodeNameTag* node) {
    if (node->TeamRandom == -2 && node->TeamIndex == -1) return -2;
    return node->TeamIndex;
}
```

### AI Slot Arrays (parallel arrays, 8 entries each)

AI players are NOT stored in `DAT_00a8da78`. Instead they use parallel global arrays:

| Global | Content |
|--------|---------|
| `DAT_00a8b274` | AI player count |
| `DAT_00a8b29c[8]` | AI country/side index per slot (at +0x00 from base) |
| `DAT_00a8b2bc[8]` | AI color index per slot (at +0x20 from base, i.e. offset 8 ints) |
| `DAT_00a8b2dc[8]` | AI team index per slot (at +0x40) |
| `DAT_00a8b2fc[8]` | AI start location per slot (at +0x60) |
| `DAT_00a8b27c[8]` | AI difficulty index per slot |

In the decompilation, `piVar8 = &DAT_00a8b29c` and fields are accessed as:
- `piVar8[0]` = country (DAT_00a8b29c)
- `piVar8[8]` = color (DAT_00a8b2bc)
- `piVar8[0x10]` = team (DAT_00a8b2dc)
- `piVar8[0x18]` = start location (DAT_00a8b2fc)
- `piVar8[-8]` = difficulty (DAT_00a8b27c)

### Key Globals

| Address | Name | Purpose |
|---------|------|---------|
| DAT_00a83d4c | PlayerPtr | Pointer to the local player's HouseClass |
| DAT_00ac1198 | ObserverHouse | Pointer to the observer's HouseClass (set when ObserverFlag == -1) |
| DAT_00a83c9c | HouseTypeClass::Array | Array of CountryTypeClass pointers |
| DAT_00a83ca8 | HouseTypeClass::Array.Count | Number of country types |
| DAT_00b054d4 | ColorSchemeArray | Array of ColorSchemeClass pointers |
| DAT_00b054e0 | ColorSchemeArray.Count | Number of color schemes |
| DAT_00a8b238 | SessionClass::GameMode | 0=campaign, 3=LAN, 4=WOL |
| DAT_00a8b25c | StartingCredits | Global starting credits setting |
| DAT_00822cf4 | DefaultTechLevel | Default tech level from rules |
| DAT_0083ed14 | PriorityToColorMap | 9-byte array mapping color index to scheme index |
| DAT_0083ed1c | RandomColorScheme | Default color scheme for "random" (-2) color |

---

## ProcessRandomAssignments (0x0069b8c0)

**Called before Create_Houses.** Resolves all "Random" selections (side=-2, color=-2)
into concrete values. Called from `FUN_005dc350` (StartGame) and `FUN_006acee0`.

```c
// Address: 0x0069b8c0
// Source: D:\ra2mdpost\Session.CPP
// Size: 494 bytes
void ProcessRandomAssignments(SessionClass* session)
{
    DebugPrint("Processing Random Assignments...\n");

    int playerCount = DAT_00a8da84;  // human player count

    // === PHASE 1: Human players ===
    for (int i = 0; i < playerCount; i++) {
        NodeNameTag* node = DAT_00a8da78[i];

        // --- Observer handling ---
        // If CountryRandom == -3 OR CountryIndex == -3, this is an observer
        if (node->CountryRandom == -3 || node->CountryIndex == -3) {
            node->CountryIndex  = -3;    // side = observer sentinel
            node->CountryRandom = -1;    // mark as assigned
            node->ColorIndex    = 8;     // color 8 = observer gray
            node->ColorRandom   = -1;    // mark as assigned
        }
        else {
            // --- Random side resolution ---
            if (node->CountryRandom == -2) {
                node->CountryRandom = -1;  // mark as assigned
                if (session->GameModeObj == NULL) {
                    // No game mode object: pick random 0..8
                    node->CountryIndex = Random(0, 9);
                } else {
                    // Game mode provides its own random side logic
                    node->CountryIndex = session->GameModeObj->vtable->GetRandomSide();
                }
            }

            // --- Random color resolution with collision avoidance ---
            if (node->ColorRandom == -2) {
                node->ColorRandom = -1;  // mark as assigned
                node->ColorIndex  = -1;  // sentinel for "not yet picked"
                do {
                    int candidate = Random(0, 7);  // pick from 0..7 (8 colors)
                    bool collision = IsColorTaken(candidate);
                } while (collision);
                node->ColorIndex = candidate;
            }
        }

        // Save first player's color as "primary color"
        if (i == 0) {
            DAT_00a8b394 = node->ColorIndex;
        }

        DebugPrint("Player %i, %s: Side = %i, Color = %i\n",
                   i, GetPlayerName(), side, color);
    }

    // === PHASE 2: AI players ===
    int* aiSlot = &DAT_00a8b29c;  // iterate over AI slot array
    int aiIdx = 0;
    do {
        // Random side for AI
        if (*aiSlot == -2) {  // aiSlot[0] = country
            if (session->GameModeObj == NULL) {
                *aiSlot = Random(0, 9);
            } else {
                *aiSlot = session->GameModeObj->vtable->GetRandomAISide();
            }
        }

        // Random color for AI (with collision avoidance)
        if (aiSlot[8] == -2) {  // aiSlot[8] = color (offset 0x20)
            do {
                int candidate = Random(0, 7);
                if (candidate == -2) break;  // exhausted

                // Check against ALL human players
                bool taken = false;
                for (int j = 0; j < DAT_00a8da84; j++) {
                    int playerColor = GetEffectiveColor(DAT_00a8da78[j]);
                    if (playerColor == candidate) { taken = true; break; }
                }
                if (taken) continue;

                // Check against all previous AI slots
                int offset = 0x84;  // DAT_00a8b2bc relative to DAT_00a8b238
                while (offset <= 0xa3) {
                    if (*(int*)((int)&DAT_00a8b238 + offset) == candidate) {
                        taken = true; break;
                    }
                    offset += 4;
                }
                if (taken) continue;

                break;  // found unused color
            } while (true);
            aiSlot[8] = candidate;
        }

        DebugPrint("AI %i: Side = %i, Color = %i\n", aiIdx, *aiSlot, aiSlot[8]);
        aiSlot++;
        aiIdx++;
    } while ((int)aiSlot < 0xa8b2bc);  // iterate up to 8 AI slots
}
```

### IsColorTaken (0x0069b600) -- Color Collision Check

```c
// Address: 0x0069b600
// Size: 103 bytes
bool IsColorTaken(SessionClass* session, int candidate)
{
    if (candidate == -2) return false;  // "random" never collides

    // Check against human players
    for (int i = 0; i < session->PlayerCount; i++) {
        NodeNameTag* node = session->Players[i];
        int color;
        if (node->ColorRandom == -2 && node->ColorIndex == -1) {
            color = -2;  // still unresolved
        } else {
            color = node->ColorIndex;
        }
        if (color == candidate) return true;  // collision
    }

    // Check against AI color slots (8 entries at session + 0x84)
    for (int i = 0; i < 8; i++) {
        if (session->AIColors[i] == candidate) return true;
    }

    return false;
}
```

**Key observations:**
- Random sides: range 0..8 (9 sides total -- matches the CountryTypeClass array indices)
- Random colors: range 0..7 (8 color slots, indices 0-7)
- Observer gets: country=-3, color=8, which is a special 9th "observer gray" color
- Collision avoidance checks both human players AND AI slots
- The AI random side can use a game mode-specific virtual (vtable+0x6c for humans,
  vtable+0x70 for AI) which may restrict available sides

---

## Create_Houses (0x00687f10)

**The master function that creates all HouseClass instances.** Called during scenario
loading from `FUN_00686b20` (scenario init) and `FUN_00689e90`. This is where the
lobby data (NodeNameTag structs and AI arrays) becomes actual game state (HouseClass
instances).

```c
// Address: 0x00687f10
// Source: D:\ra2mdpost\Scenario.CPP
// Size: 1134 bytes
void Create_Houses()
{
    int humanCount = DAT_00a8da84;  // number of human players

    // === PHASE 0: Clear observer houses ===
    // Any player with side == -3 (observer) triggers FUN_00696f90(0)
    // which resets their country to 0 and marks it "random"
    for (int i = 0; i < humanCount; i++) {
        if (DAT_00a8da78[i]->CountryIndex == -3) {
            SetCountry(DAT_00a8da78[i], 0);  // reset observer's country
        }
    }

    // === PHASE 1: Create houses for HUMAN players ===
    // Priority-sorted: players with lowest Priority value go first
    char processed[8] = {0};  // marks which player slots have been processed
    DAT_00ac1198 = NULL;      // clear ObserverHouse pointer

    for (int count = 0; count < humanCount; count++) {
        // --- Find the unprocessed player with lowest priority ---
        int bestPriority = -1;
        int bestSlot = -1;
        for (int j = 0; j < humanCount; j++) {
            if (!processed[j]) {
                int priority = DAT_00a8da78[j]->ColorIndex;  // +0x53 = color priority
                if (bestSlot == -1 || priority < bestPriority) {
                    bestPriority = priority;
                    bestSlot = j;
                }
            }
        }

        NodeNameTag* node = DAT_00a8da78[bestSlot];
        processed[bestSlot] = 1;

        // --- Allocate HouseClass ---
        void* mem = operator_new(0x160b8);  // HouseClass is 0x160B8 bytes (~90KB)
        HouseClass* house;
        if (mem == NULL) {
            house = NULL;
        } else {
            // Look up CountryTypeClass from the country index
            int countryIdx = node->CountryIndex;  // +0x4b
            CountryTypeClass* countryType = DAT_00a83c9c[countryIdx];
            house = HouseClass::Constructor(mem, countryType);  // FUN_004f54a0
        }

        // --- Set player name ---
        if (DAT_00a8b238 == 4) {
            // WOL mode: get player name from network
            // (calls FUN_007b66d0..FUN_007b5400 chain to extract name)
            char name[21];
            GetNetworkPlayerName(node, name, 20);
            memcpy(house->PlayerName, name, 21);  // +0x15ff4
        } else {
            // LAN/Skirmish: use "<human player>" placeholder
            strncpy(name, "<human player>", 20);
            memcpy(house->PlayerName, name, 21);  // +0x15ff4
        }

        // --- Copy serialized node data into house ---
        memcpy(house->UINameBuffer, node, 0x15);  // +0x1602a -- copy 21 bytes of node
        house->UINameSuffix = 0;                   // +0x16052 = 0

        // --- Mark as human-controlled ---
        house->IsHuman = 1;  // +0x1ec = 1

        // --- Set credits and color ---
        Set_Credits_And_Color(house,
            node->ColorIndex,     // +0x53 = color priority index
            node->CountryIndex,   // +0x4b = country/side
            StartingCredits);     // DAT_00a8b25c

        // --- Map priority index to color scheme ---
        int colorScheme = PriorityToColorScheme(node->ColorIndex);  // FUN_0069a310
        house->ColorSchemeIndex = colorScheme;  // +0x16054

        // --- Initialize house color from scheme ---
        House_InitColor(house);   // FUN_0050b840
        House_ComputeRemap(house); // FUN_0050ba00

        // --- Get team/alliance index ---
        int team = GetTeam(node);  // FUN_00696f50
        house->TeamIndex = team;   // +0x16058

        // --- Get start location ---
        house->StartLocation = node->SpawnLocation;  // +0x6f -> +0x1605c (actually +99 = 0x63)

        // --- Set PlayerPtr for first processed player ---
        if (bestSlot == 0) {
            DAT_00a83d4c = house;   // PlayerPtr = this house
            house->PlayerControl = 1;  // +0x1ed = 1
        }

        // --- Set ObserverHouse if observer ---
        if (node->ObserverFlag == -1) {  // +0x6b
            DAT_00ac1198 = house;  // ObserverHouse pointer
        }

        // --- Set tech level ---
        house->TechLevel = DAT_00822cf4;  // +0x1d4 = default tech level

        // --- Set difficulty ---
        House_SetDifficulty(house, 1);  // FUN_004f6ec0(1)

        // --- Store house index back into node ---
        node->HouseIndex = house->HouseIndex;  // +0x6f = house->+0x30
    }

    // === PHASE 2: Create houses for AI players ===
    int aiCount = DAT_00a8b274;
    int aiCreated = 0;
    int* aiSlot = &DAT_00a8b29c;  // walks through AI slot array

    do {
        if (aiCreated < aiCount && *aiSlot != -1 && *aiSlot != -3) {
            // This AI slot is active (not empty, not observer)
            int aiCountry = *aiSlot;       // country index
            int aiColor = aiSlot[8];       // color index (offset +0x20)
            aiCreated++;

            void* mem = operator_new(0x160b8);
            HouseClass* house;
            if (mem == NULL) {
                house = NULL;
            } else {
                CountryTypeClass* countryType = DAT_00a83c9c[aiCountry];
                house = HouseClass::Constructor(mem, countryType);
            }

            house->IsHuman = 0;         // +0x1ec = 0 (AI controlled)
            house->TechLevel = DAT_00822cf4;

            Set_Credits_And_Color(house, aiColor, aiCountry, StartingCredits);

            int colorScheme = PriorityToColorScheme(aiColor);
            house->ColorSchemeIndex = colorScheme;  // +0x16054

            House_InitColor(house);
            House_ComputeRemap(house);

            house->TeamIndex = aiSlot[0x10];        // team
            house->StartLocation = aiSlot[0x18];    // start location

            // If start location != -1, enable spawner flag
            if (aiSlot[0x18] != -1) {
                *(byte*)(DAT_00a8b230 + 0x11e0) = 1;
            }

            // Set player name to "Computer"
            strncpy(name, "Computer", 20);
            memcpy(house->PlayerName, name, 21);

            // Set UI name from string table
            int csf_id = CSF_Resolve("D:\\ra2mdpost\\Scenario.CPP", 0xfc0);
            strcpy(house->UINameBuffer, CSF_String(csf_id));  // "TXT_COMPUTER"

            // Copy credits from multiplayer settings if applicable
            if (DAT_00a8b238 != 0) {
                house->CurrentIQ = *(int*)(DAT_008871e0 + 0x1434);
            }

            // Set difficulty from AI difficulty slot
            int difficulty = aiSlot[-8];  // DAT_00a8b27c
            if (DAT_00a8da84 > 1 && *(char*)(DAT_008871e0 + 0x17e3) != 0
                && difficulty > 0) {
                difficulty--;  // reduce difficulty by 1 in multi-human games
                               // when BerzerkAllowed is set
            }
            House_SetDifficulty(house, difficulty);
        }
        aiSlot++;
    } while ((int)aiSlot < 0xa8b2bc);  // process up to 8 AI slots

    // === PHASE 3: Create Neutral house ===
    void* mem = operator_new(0x160b8);
    HouseClass* neutralHouse;
    if (mem == NULL) {
        neutralHouse = NULL;
    } else {
        int neutralIdx = FindCountryByName("Neutral");  // FUN_005117d0
        CountryTypeClass* neutralType = DAT_00a83c9c[neutralIdx];
        neutralHouse = HouseClass::Constructor(mem, neutralType);
    }
    int neutralColor = FindColorSchemeByName("Neutral");  // FUN_0068cab0
    neutralHouse->ColorSchemeIndex = neutralColor;
    House_InitColor(neutralHouse);

    // === PHASE 4: Create Special house ===
    mem = operator_new(0x160b8);
    HouseClass* specialHouse;
    if (mem == NULL) {
        specialHouse = NULL;
    } else {
        int specialIdx = FindCountryByName("Special");  // FUN_005117d0 (different param)
        CountryTypeClass* specialType = DAT_00a83c9c[specialIdx];
        specialHouse = HouseClass::Constructor(mem, specialType);
    }
    int specialColor = FindColorSchemeByName("Special");  // FUN_0068cab0
    specialHouse->ColorSchemeIndex = specialColor;
    House_InitColor(specialHouse);
}
```

### Key Details

1. **Priority sorting**: Human players are processed in order of their `ColorIndex`
   (+0x53). The player with the lowest color index is processed first. This matters
   because the FIRST processed player (bestSlot == 0) becomes `PlayerPtr` and gets
   `PlayerControl = 1`.

2. **HouseClass allocation**: Every house is 0x160B8 bytes (~90KB). The constructor
   takes a `CountryTypeClass*` (looked up from `DAT_00a83c9c[countryIndex]`).

3. **Neutral and Special**: These are always created after all human + AI houses.
   They use `FindCountryByName()` to look up their CountryTypeClass, and
   `FindColorSchemeByName()` to look up their color scheme. They do NOT call
   `Set_Credits_And_Color` or `ComputeRemap` -- only `InitColor`.

---

## HouseClass Constructor (0x004f54a0)

**Size**: 4250 bytes. **Object size**: 0x160B8 bytes.

This is the full constructor with spawn/scenario data. Called from Create_Houses,
and also from scenario loading for campaign houses.

```c
// Address: 0x004f54a0
// Param: this (0x160B8 bytes), param_2 = CountryTypeClass*
HouseClass* HouseClass::Constructor(void* this, CountryTypeClass* countryType)
{
    // Call base class constructor (AbstractClass)
    AbstractClass::Init(this);  // FUN_00410170

    // === Identity ===
    this->HouseIndex = -1;           // +0x30 (will be set later by array insertion)
    this->CountryType = countryType; // +0x34

    // === DynamicVectorClass arrays (12 total) ===
    // Each initialized with vtable and capacity=10
    // Offsets 0x38..0x154 cover these arrays (6 DWORDs each: vtable, data, ?, count, cap, grow)
    //
    // Array at +0x38: vtable=007ea5a4 (OwnedAircraftArray)
    // Array at +0x50: vtable=007e9e24 (OwnedInfantryArray)
    // Array at +0x68: vtable=007e9e24 (OwnedUnitArray)
    // Array at +0x80: vtable=007e9e24 (OwnedBuildingArray)
    // Array at +0x98: vtable=007e9e24 (OwnedNavalArray)
    // ... and 7 more arrays with same vtable pattern
    // Plus special arrays at +0x16c (vtable=007ea944)
    //   and +0x254 (vtable=007ea4e4)

    // === Difficulty multipliers (doubles) ===
    // +0x188..+0x1cc: 7 double-precision values, all initialized to 1.0
    // (FirepowerMult, ArmorMult, SpeedMult, ROFMult, CostMult, RepairMult, BuildSpeedMult)
    this->FirepowerMult = 1.0;   // +0x188/+0x18c = 0x3ff00000 (IEEE 754 for 1.0)
    this->ArmorMult     = 1.0;   // +0x190
    this->SpeedMult     = 1.0;   // +0x198
    this->ROFMult       = 1.0;   // +0x1a0
    this->CostMult      = 1.0;   // +0x1a8
    this->RepairMult    = 1.0;   // +0x1b0
    this->BuildSpeedMult= 1.0;   // +0x1b8

    // === Flags ===
    this->AnnouncedReady = 0;      // +0x1c4
    this->ProductionChanged = 0;   // +0x1c8..+0x1d0
    this->IsAlly[self] = 1;       // +0x1d4 (always allied with self)
    this->PowerOutput = 0;         // +0x1e0
    this->PowerDrain = 0;          // +0x1e4
    this->DestroyedFlag = -1;      // +0x1e8
    this->ShroudState = 0;         // +0x1ec

    // +0x1ec: IsHuman = 0 (default, overridden by Create_Houses)
    // +0x1ed: PlayerControl = 0
    // +0x1ee: AIActive = 0
    // +0x1ef: AITriggersActive = 0
    // +0x1f0: IsRevealedToPlayer = 1  (default revealed)
    // +0x1f1..+0x1f7: various flags zeroed
    // +0x1f8: AlwaysReveal = 0
    // +0x1fc: HasSuperweapon = 1 (initially true)

    // === Economy ===
    this->Balance = 0;          // +0x200
    this->TotalSpent = 0;       // +0x204
    // +0x208: IQ-related flag = 0
    // +0x20c: FactoryIndex = -1

    // +0x240: BaseReady = 0
    // +0x241-+0x248: various flags zeroed
    // +0x24b: SidebarUpdatePending = 1

    // === Counters (via FUN_006c95e0) ===
    // Two counter objects initialized at +0x2ec and +0x310

    // === 10x FUN_00748fd0 calls ===
    // These initialize 10 timer/rate-tracking objects

    // === Tracking arrays ===
    // +0x5390..+0x539c: 5 floats initialized to 1.0 (build bonuses)
    // +0x53a4: AttackPower = 0
    // +0x53a8: DefensePower = 0
    // ... more zeroed fields ...

    // === Production queues (12x FUN_0049f9b0) ===
    // 12 timer slots at +0x54f0..+0x5600

    // === Build multiplier arrays (3x FUN_004b69b0) ===

    // === House color ===
    // +0x56f9..+0x56fb: RGB color bytes = 0 (black, will be set later)
    // +0x56fc..+0x56fe: Bright/remap RGB = 0xFF, 0xFF, 0xFF (white default)

    // === FUN_0042e6f0 call ===
    // Initializes some combat-related state

    // +0x5778: SpeechPending = 1
    // +0x5779: AnnouncementPending = 1

    // === Set vtable pointers (multiple inheritance) ===
    this->vtable[0] = &PTR_FUN_007ea8a0;  // primary vtable
    this->vtable[1] = &PTR_LAB_007ea884;  // IPublicHouse
    this->vtable[2] = &PTR_LAB_007ea87c;  // IHouse
    this->vtable[3] = &PTR_LAB_007ea874;  // IOther
    this->vtable[9] = &PTR_LAB_007ea834;
    this->vtable[10] = &PTR_LAB_007ea80c;
    this->vtable[11] = &PTR_LAB_007ea7f4;

    // === Copy UIName from CountryType ===
    strncpy(this->UIName, countryType->UIName, 31);  // +0x16009 from +0x24

    // === Register in AbstractClass array ===
    FUN_00410230(this + 1);  // base class registration

    // === Copy SideIndex from CountryType ===
    if (countryType != NULL) {
        this->SideFlags = countryType->DefaultFlags;  // +0x16060 from +0xc0
    }

    // === Register in global HouseClass array ===
    // this gets added to DAT_00a8022c[DAT_00a80238]
    // DAT_00a80238 (count) incremented
    this->HouseIndex = DAT_00a80238;  // +0x30 = current count (becomes index)

    // === Cross-register with all existing houses ===
    // For each existing house in DAT_00a8022c:
    //   - Add this to their AltHouseList (+0x5604..+0x5614)
    //   - Add this to their VisibleHouseList (+0x5620..+0x5630)
    //   - Add them to this->AltHouseList
    //   - Add them to this->VisibleHouseList

    // === Zero visibility/shroud array ===
    // 0x4204 DWORDs at offset +0x57e4 (the threat/visibility map)
    memset(this + 0x57e4, 0, 0x4204 * 4);

    // === Create CombatDamage objects ===
    // For each weapon type in DAT_00a8e334..DAT_00a8e340:
    //   Allocate 0x80-byte CombatDamageClass via FUN_006caf90
    //   Add to DynamicVector at +0x254

    // === Zero known-object arrays ===
    // 0x14 DWORDs at +0x53e4 and +0x5438

    // === Copy player name from CountryType ===
    if (countryType != NULL) {
        strncpy(this->CountryName, countryType->Name, 20);  // +0x15ff4 from +0x24
        strcpy(this->UINameBuffer, countryType->UINameCSF);  // +0x1602a from +0x60
    }

    // === Zero visibility map again (double-init) ===
    memset(this + 0x57e4, 0, 0x4204 * 4);

    // === Set ally bitmask (ally with self) ===
    this->AllyBitmask |= (1 << (this->HouseIndex & 0x1f));  // +0x1d8

    // === Allocate shroud/fog arrays ===
    // FUN_0065c7e0(0x1c2, 0x708) -- allocates visibility tracking structure

    // === Initialize tracking timers ===
    // 10 calls to FUN_00749060 with various type array counts

    // === Zero base plan arrays ===
    // 12 DWORDs at +0x210..+0x240

    // === Set SideIndex from CountryType ===
    if (countryType != NULL) {
        int side = countryType->SideIndex;  // +0xbc
        if (side == 0)      this->SideIndex = 0;  // Allied
        else if (side == 1) this->SideIndex = 1;  // Soviet
        else if (side == 2) this->SideIndex = 2;  // Yuri
    }

    // === Create SpreadClass for this house ===
    // FUN_004a0870 allocates 0x34-byte CellSpreadClass

    // === Register locomotor ===
    // FUN_004f6830 + FUN_004a0870 creates a locomotor binding

    return this;
}
```

### Key Points

- **Total object size**: 0x160B8 bytes (~90KB per house)
- **12 DynamicVectorClass arrays** for tracking owned objects by category
- **7 double-precision difficulty multipliers** (all initialized to 1.0)
- **0x4204 DWORDs** for the visibility/threat map (zeroed twice in constructor)
- **Cross-registration**: every existing house gets told about the new house,
  and vice versa, via diplomacy/visibility arrays
- **Self-ally**: the house always starts allied with itself via bitmask
- The constructor does NOT set credits, color, IsHuman, or PlayerControl --
  those are all set by Create_Houses after construction

---

## Set_Credits_And_Color (0x004fce00)

A very small function (38 bytes) that sets four fields on the HouseClass.

```c
// Address: 0x004fce00
// Size: 38 bytes
// Called only by Create_Houses (0x00687f10)
void HouseClass::Set_Credits_And_Color(
    int colorPriority,   // param_2: color index from lobby
    int countryIndex,    // param_3: country/side index (UNUSED in body!)
    int startCredits)    // param_4: starting credits amount
{
    this->StartingCredits  = startCredits;   // +0x1dc
    this->AvailableCredits = startCredits;   // +0x30c
    this->CountryType->DefaultColor = colorPriority;  // *(this->+0x34 + 0xc0)
    this->ColorSchemeIndex = colorPriority;  // +0x16054
}
```

**Critical note**: Despite receiving `countryIndex` as param_3, the function body
does NOT use it at all. It only stores:
1. Starting credits into both StartingCredits and AvailableCredits
2. The color priority index into both the CountryType's DefaultColor field AND
   the house's own ColorSchemeIndex

The `ColorSchemeIndex` (+0x16054) is then immediately overwritten by
`PriorityToColorScheme()` in Create_Houses, so the value stored here at +0x16054
is transient. The value stored in `CountryType->DefaultColor` (+0xc0) persists.

---

## Color Scheme Resolution Pipeline

The color index from the lobby (0-7 for normal players, 8 for observers) goes through
several transformations before becoming actual RGB values.

### Step 1: PriorityToColorScheme (0x0069a310)

Maps the lobby color index to a color scheme array index.

```c
// Address: 0x0069a310
// Size: 39 bytes
int PriorityToColorScheme(int colorIndex)
{
    if (colorIndex == -2) {
        // "Random" -- return the default random scheme
        return (int)DAT_0083ed1c;   // global default color scheme index
    }
    if (colorIndex < 9) {
        // Normal index (0-8): look up in mapping table
        return (signed char) DAT_0083ed14[colorIndex];
    }
    // Out of range: return as-is (passthrough)
    return colorIndex;
}
```

**DAT_0083ed14** is a 9-byte lookup table at address 0x0083ed14. It maps the lobby
color priority (0-7, plus 8 for observer) to an index into the ColorSchemeArray
(`DAT_00b054d4`). The exact values are initialized during game startup from the
`[Colors]` section of rules.ini.

### Step 2: ColorSchemeArray Lookup

The ColorSchemeArray at `DAT_00b054d4` is a `DynamicVectorClass<ColorSchemeClass*>`:
- `DAT_00b054d4` = pointer to array of ColorSchemeClass pointers
- `DAT_00b054e0` = count

Each ColorSchemeClass entry (constructed during INI parsing) has:

| Offset | Type | Purpose |
|--------|------|---------|
| +0x304 | char* | Color name string (e.g. "Gold", "Red", "DarkBlue") |
| +0x30c | ptr | Palette/ConvertClass pointer |
| +0x310 | int | Type flag (1 = special/excluded from normal lookups) |
| +0x330 | int | Remap index into palette |

### Step 3: House::InitColor (0x0050b840)

Extracts RGB from the color scheme's palette entry.

```c
// Address: 0x0050b840
// Size: 213 bytes
void House::InitColor(HouseClass* this)
{
    int schemeIdx = this->ColorSchemeIndex;  // +0x16054

    // Clamp negative to default (white = index 5)
    if (schemeIdx < 0) {
        this->ColorSchemeIndex = 5;
        schemeIdx = 5;
    }

    ColorSchemeClass* scheme = DAT_00b054d4[schemeIdx];

    // Fallback if scheme is NULL
    if (scheme == NULL) {
        DebugPrint("Forcing House %s [%s] to color WHITE (from %d)...", ...);
        this->ColorSchemeIndex = 5;
        scheme = DAT_00b054d4[5];  // WHITE is always at index 5
    }

    // Get palette data
    ConvertClass* convert = scheme->Convert;        // +0x30c
    byte* pixelData = convert->PixelData;           // +0x174
    int remapIdx = scheme->RemapIndex;              // +0x330

    // Read the pixel value at the remap index
    uint16_t pixel;
    if (convert->BitsPerPixel == 1) {               // +0x4 == 1 (8-bit mode)
        pixel = (uint16_t) pixelData[remapIdx];     // byte lookup
    } else {
        pixel = *(uint16_t*)(pixelData + remapIdx * 2);  // 16-bit lookup
    }

    // Extract RGB components using display surface bit masks
    // DAT_008a0dd0..008a0de4 define the bit positions and shift amounts
    // for R, G, B extraction from the 16-bit pixel value
    byte r = (byte)(pixel >> (DAT_008a0dd0 & 0x1f)) << (DAT_008a0dd4 & 0x1f);
    byte g = (byte)(pixel >> (DAT_008a0de0 & 0x1f)) << (DAT_008a0de4 & 0x1f);
    byte b = (byte)(pixel >> (DAT_008a0dd8 & 0x1f)) << (DAT_008a0ddc & 0x1f);

    this->ColorR = r;  // +0x56f9
    this->ColorG = g;  // +0x56fa
    this->ColorB = b;  // +0x56fb
}
```

### Step 4: House::ComputeRemap (0x0050ba00)

Normalizes the RGB to a "bright" remap color used for unit colorization.

```c
// Address: 0x0050ba00
// Size: 656 bytes
void House::ComputeRemap(HouseClass* this)
{
    byte r = this->ColorR;  // +0x56f9
    byte g = this->ColorG;  // +0x56fa
    byte b = this->ColorB;  // +0x56fb

    double dr = (double)r;
    double dg = (double)g;
    double db = (double)b;

    double magnitude = sqrt(dr*dr + dg*dg + db*db);

    double bright_r, bright_g, bright_b;
    if (magnitude == 0.0) {
        // Black color: default to bright white
        bright_r = bright_g = bright_b = 255.0;
    } else {
        // Normalize each component: (component / magnitude) * 255
        bright_r = clamp((dr * 255.0) / magnitude, 0.0, 255.0);
        bright_g = clamp((dg * 255.0) / magnitude, 0.0, 255.0);
        bright_b = clamp((db * 255.0) / magnitude, 0.0, 255.0);
    }

    this->BrightR = (byte)bright_r;  // +0x56fc
    this->BrightG = (byte)bright_g;  // +0x56fd
    this->BrightB = (byte)bright_b;  // +0x56fe
}
```

The "bright" color is the RGB vector normalized to length 255. This preserves the
hue but maximizes saturation. For example, dark red (128, 0, 0) becomes (255, 0, 0).

### Step 5: FindColorSchemeByName (0x0068cab0) -- for Neutral/Special

Used only for Neutral and Special houses. Searches the ColorSchemeArray by name string.

```c
// Address: 0x0068cab0
// Size: 86 bytes
int FindColorSchemeByName(char* name, int typeFilter)
{
    for (int i = 0; i < DAT_00b054e0; i++) {
        ColorSchemeClass* scheme = DAT_00b054d4[i];
        if (strcmp(name, scheme->Name) == 0       // +0x304 = name string
            && scheme->TypeFlag == typeFilter) {  // +0x310 = type filter
            return i;
        }
    }
    return -1;
}
```

The `typeFilter` parameter is passed from Create_Houses -- it comes from
`FUN_005117d0()` which returns the CountryTypeClass index. The Neutral house passes
the result of looking up "Neutral" in the country array; Special passes "Special".

---

## House::Read_Scenario_INI (0x00500b40)

**Called during campaign/scenario loading** (NOT multiplayer Create_Houses). This is
how campaign houses get their properties from the map's `[HouseName]` INI sections.

```c
// Address: 0x00500b40
// Source: D:\ra2mdpost\House.CPP
// Size: 1557 bytes
void HouseClass::Read_Scenario_INI(HouseClass* this, INIClass* ini)
{
    char section[20];
    strncpy(section, this->PlayerName, 20);  // +0x15ff4

    // Read tech level
    this->TechLevel = INI_ReadInt(section, "TechLevel",
                                  Rules->DefaultTechLevel);  // +0x1d4

    // Read starting credits (multiplied by 100 internally)
    int credits = INI_ReadInt(section, "Credits", 0);
    this->StartingCredits = credits * 100;  // +0x1dc

    // Read PlayerControl flag
    char playerControl = INI_ReadBool(section, "PlayerControl", false);
    this->PlayerControl = playerControl;  // +0x1ed

    // Compute AvailableCredits based on difficulty
    if (playerControl && GameMode == 0) {
        // Campaign with player control: add difficulty bonus
        if (DifficultyLevel == 0) {
            this->AvailableCredits = Rules->EasyBonus + this->StartingCredits;
        } else if (DifficultyLevel == 2) {
            this->AvailableCredits = Rules->HardBonus + this->StartingCredits;
        } else {
            this->AvailableCredits = this->StartingCredits;
        }
        // Clamp: if negative, set to 0
        if ((int)this->AvailableCredits < 0) this->AvailableCredits = 0;
    } else {
        this->AvailableCredits = this->StartingCredits;
    }

    // Read UIName
    INI_ReadString(section, "UIName", this->UIName, buffer, 32);
    if (buffer[0] != '\0') {
        memcpy(this->UIName, buffer, 32);  // +0x16009
    }
    // If UIName is empty, use CountryType name
    if (this->UIName[0] == '\0') {
        strcpy(this->UINameBuffer, this->CountryType->UINameCSF);
    }

    // Read AI ratio settings
    this->RatioAITriggerTeam = INI_ReadInt(section, "RatioAITriggerTeam", ...);
    this->RatioTeamAircraft = INI_ReadInt(section, "RatioTeamAircraft", 75);
    this->RatioTeamInfantry = INI_ReadInt(section, "RatioTeamInfantry", 75);
    this->RatioTeamUnits = INI_ReadInt(section, "RatioTeamUnits", 75);

    // Read SideIndex from CountryType
    this->SideIndex = this->CountryType->SideIndex;  // +0x1e8 from +0xbc
    if (this->SideIndex == -1) this->SideIndex = 0;

    // Read IQ level
    int iq = INI_ReadInt(section, ???, 0);
    if (iq > Rules->MaxIQ) iq = 1;
    this->IQLevel = iq;   // +0x1d0
    this->CurrentIQ = iq;  // +0x24c

    // Read starting edge
    this->StartingEdge = INI_ReadEdge(section, ???, -1);  // +0x1e0

    // === READ COLOR ===
    int colorIdx = INI_ReadColorIndex(section, "Color", this->ColorSchemeIndex);
    this->ColorSchemeIndex = colorIdx;  // +0x16054

    if (colorIdx < 0) {
        this->ColorSchemeIndex = 5;  // default to WHITE (index 5)
    }

    // Validate: look up scheme, force WHITE if invalid
    ColorSchemeClass* scheme = DAT_00b054d4[this->ColorSchemeIndex];
    if (scheme == NULL) {
        DebugPrint("Forcing House %s [%s] to color WHITE (from %d)...",
                   this->PlayerName, this->CountryType->Name + 100,
                   this->ColorSchemeIndex);
        this->ColorSchemeIndex = 5;
        scheme = DAT_00b054d4[5];
    }

    // Extract RGB from scheme (same logic as InitColor)
    uint16_t pixel = GetPalettePixel(scheme);
    this->ColorR = ExtractR(pixel);  // +0x56f9
    this->ColorG = ExtractG(pixel);  // +0x56fa
    this->ColorB = ExtractB(pixel);  // +0x56fb

    // Normalize to bright color
    NormalizeRGBVector(&r_double, &g_double, &b_double);
    this->BrightR = ftol(r_double);  // +0x56fc
    this->BrightG = ftol(g_double);  // +0x56fd
    this->BrightB = ftol(b_double);  // +0x56fe

    // Read Allies bitmask
    uint allyMask = INI_ReadUInt(section, "Allies", 0);
    ClearAllAlliances(this);
    for (int i = 0; i < HouseCount; i++) {
        HouseClass* other = DAT_00a8022c[i];
        if (allyMask & (1 << other->HouseIndex)) {
            MakeAlly(this, other, 0);
        }
    }

    // Compute base map position
    int cellMapAddr = *(Rules->CellMap + this->SpawnLocation * 4);
    this->BaseMapX = InvalidCell;    // +0x5798
    this->BaseMapY = brightness;     // +0x579c
    this->BaseMapOffset = cellMapAddr + this->HouseIndex * 0xaf;  // +0x57a0

    // Register in the per-scenario house tracking
    FUN_0042ebe0(ini, section);
    this->SelfPtr = this;  // +0x5774
}
```

### INI_ReadColorIndex (0x00474a90)

This function reads the "Color=" key from INI and resolves it to a ColorSchemeArray index.

```c
// Address: 0x00474a90
// Size: 138 bytes
int ReadColorIndex(char* section, char* key, int defaultIdx)
{
    // Get the default color name from the current scheme
    char* defaultName = DAT_00b054d4[defaultIdx]->Name;  // +0x304

    // Read the INI value (string), defaulting to the current scheme name
    char buffer[32];
    INI_ReadString(section, key, defaultName, buffer, 32);

    // Search the ColorSchemeArray for a matching name
    for (int i = 0; i < DAT_00b054e0; i++) {
        ColorSchemeClass* scheme = DAT_00b054d4[i];
        if (strcmp(scheme->Name, buffer) == 0      // +0x304
            && scheme->TypeFlag != 1) {            // +0x310 != 1 (exclude special types)
            return i;
        }
    }
    return defaultIdx;  // name not found, keep current
}
```

**Key observations:**
- The "Color=" INI key uses **string names** (e.g., "Gold", "Red", "DarkBlue"),
  not numeric indices
- The lookup excludes entries with `TypeFlag == 1` (these are special color schemes
  like "LightGrey" used for Neutral/Special houses)
- If the name isn't found, the default index is returned unchanged

---

## Summary of the Full Pipeline

### Multiplayer (Skirmish/LAN/WOL)

```
Lobby selection (color combo 0-7, country combo)
    |
    v
ProcessRandomAssignments()    -- resolves Random selections
    |                         -- observer: country=-3, color=8
    |                         -- collision avoidance for colors
    v
Create_Houses()
    |
    +-- Sort human players by ColorIndex (priority sorting)
    |
    +-- For each human player (priority order):
    |     1. operator_new(0x160B8)
    |     2. HouseClass::Constructor(CountryTypeClass*)
    |     3. Set_Credits_And_Color(color, country, credits)
    |     4. colorScheme = PriorityToColorScheme(colorIndex)  -- DAT_0083ed14 lookup
    |     5. house->ColorSchemeIndex = colorScheme
    |     6. InitColor(house)     -- extract RGB from palette
    |     7. ComputeRemap(house)  -- normalize to bright RGB
    |     8. First player -> PlayerPtr, PlayerControl=1
    |     9. ObserverFlag==-1 -> ObserverHouse
    |
    +-- For each AI slot:
    |     Same steps 1-7, but IsHuman=0, name="Computer"
    |     Difficulty from AI slot array, reduced by 1 in multi-human
    |
    +-- Create "Neutral" house (FindCountryByName + FindColorSchemeByName)
    +-- Create "Special" house (FindCountryByName + FindColorSchemeByName)
```

### Campaign (Single Player)

```
Map INI [Houses] section lists house names
    |
    v
For each [HouseName] section:
    1. operator_new(0x160B8)
    2. HouseClass::Constructor(FindCountryByName(houseName))
    3. Read_Scenario_INI(house, ini)
         - reads TechLevel, Credits, PlayerControl, Color, Allies
         - Color= is a string name resolved via ReadColorIndex()
         - RGB extracted and remap computed inline
```

### Color Index Reference

| Lobby Index | Typical Color | Color Scheme |
|-------------|---------------|--------------|
| 0 | Gold/Yellow | Mapped via DAT_0083ed14[0] |
| 1 | Red | Mapped via DAT_0083ed14[1] |
| 2 | Blue | Mapped via DAT_0083ed14[2] |
| 3 | Green | Mapped via DAT_0083ed14[3] |
| 4 | Orange | Mapped via DAT_0083ed14[4] |
| 5 | Teal/Cyan | Mapped via DAT_0083ed14[5] |
| 6 | Purple | Mapped via DAT_0083ed14[6] |
| 7 | Pink | Mapped via DAT_0083ed14[7] |
| 8 | Observer Gray | Mapped via DAT_0083ed14[8] |
| -2 | Random | Mapped via DAT_0083ed1c (default scheme) |

The actual mapping depends on how the `[Colors]` section is parsed during startup.
The byte array at `DAT_0083ed14` contains 9 bytes that index into the
ColorSchemeArray. The value at `DAT_0083ed1c` is used for unresolved random colors.

### HouseClass Color Fields

| Offset | Size | Field | Set By |
|--------|------|-------|--------|
| +0x16054 | 4 | ColorSchemeIndex | Set_Credits_And_Color, then PriorityToColorScheme |
| +0x56f9 | 1 | ColorR | InitColor (from palette) |
| +0x56fa | 1 | ColorG | InitColor (from palette) |
| +0x56fb | 1 | ColorB | InitColor (from palette) |
| +0x56fc | 1 | BrightR | ComputeRemap (normalized) |
| +0x56fd | 1 | BrightG | ComputeRemap (normalized) |
| +0x56fe | 1 | BrightB | ComputeRemap (normalized) |

---

## Confidence Assessment

| Finding | Confidence | Basis |
|---------|-----------|-------|
| NodeNameTag field layout (+0x4b thru +0x7f) | HIGH (90%) | Confirmed by 6+ getter/setter functions |
| Create_Houses flow | HIGH (90%) | Full decompilation with string refs |
| Set_Credits_And_Color fields | HIGH (95%) | Only 38 bytes, trivially verified |
| PriorityToColorScheme logic | HIGH (95%) | 39 bytes, simple table lookup |
| HouseClass constructor field offsets | MEDIUM (80%) | `param_1` is `int*` so offsets are x4; verified against known fields |
| ColorSchemeClass field layout | MEDIUM (75%) | +0x304, +0x30c, +0x310, +0x330 confirmed by multiple xrefs |
| ProcessRandomAssignments ranges | HIGH (90%) | Random(0,9) for sides, Random(0,7) for colors confirmed from decompilation |
| Observer handling (color=8, side=-3) | HIGH (95%) | Explicit in decompilation with clear sentinel checks |
