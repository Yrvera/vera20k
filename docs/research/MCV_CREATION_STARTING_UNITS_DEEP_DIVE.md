# MCV Creation & Starting Unit System — gamemd.exe Deep Dive

Complete reverse-engineering of the starting unit generation pipeline: MCV creation,
placement, auto-deploy, starting unit budget, filtering, and spiral placement.

Sources: Ghidra decompiled C files (109_006829a0_00689d30.c, 048_*, 049_*, 051_*,
105_*, 143_*), reports 082, 105, 117, and existing docs (GAME_START_INITIALIZATION.md,
MCV_DEPLOY_GHIDRA_REPORT.md, HOUSECLASS_GHIDRA_REPORT.md).

---

## 1. Post_Map_Init (0x00686890) — Full Annotated Pseudocode

Called from `Full_Init` (0x00686b20) and `Read_Scenario` (0x00684620) after all map
data and houses are loaded. Size: 648 bytes.

```c
void Post_Map_Init(ScenarioClass* this) // 0x00686890
{
    // --- Phase 1: Add AI players if needed ---
    // DAT_00a8da84 = human player count
    // DAT_00a8b274 = AI player count
    // RulesClass+0x14d0 = max players allowed
    if (humanPlayerCount + aiPlayerCount < Rules->MaxPlayers
        && !isEditorMode)
    {
        for (int i = 0; i < HouseClass::Array.Count; i++) {
            HouseClass* house = HouseClass::Array[i];
            // If house is AI (!IsHuman) and not a special house (!IsObserver)
            // and no AI players configured yet
            if (!house->IsHuman && !house->TypeClass->IsMultiplayOnly
                && aiPlayerCount < 1)
            {
                // corrected 2026-05-28: was "Add AI player to fill slot";
                // binary shows HouseClass__Destroy_All_Owned @ 004fb920
                // via get_function_by_address + decompile_function 0x00686890
                // ROOT_CAUSE: RTTI_LABEL_DRIFT
                HouseClass__Destroy_All_Owned();  // 0x004fb920
            }
        }
    }

    // --- Phase 2: Network service loop ---
    // corrected 2026-05-28: was "Timer checkpoint / FUN_0048d080";
    // binary label is Network_ServiceLoop @ 0048d080
    // via get_function_by_address 0x0048d080 — ROOT_CAUSE: RTTI_LABEL_DRIFT
    Network_ServiceLoop();  // 0x0048d080

    // --- Phase 3: Generate starting units OR network sync ---
    // corrected 2026-05-28: was "int savedTimerState = DAT_00a8e7ac" / "Pause timer";
    // binary saves/restores g_MapEditorMode (not a timer); DAT_00a8e7ac is g_MapEditorMode
    // via decompile_function 0x00686890 — ROOT_CAUSE: RTTI_LABEL_DRIFT
    int savedEditorMode = g_MapEditorMode;
    if (!isEditorMode) {
        g_MapEditorMode = 0;  // Suppress editor-mode checks during unit generation

        if (NetworkManager == NULL) {
            // Offline/skirmish: generate starting units directly
            Generate_Random_Units();  // 0x006886b0
        } else {
            // Multiplayer: let network manager handle it
            NetworkManager->vtable->GenerateUnits(this);   // vtable+0x84
            FUN_005d6d80();  // Network sync starting units
        }
    }
    g_MapEditorMode = savedEditorMode;

    // --- Phase 4: Tech share crate spawning ---
    FUN_0069ae90(0x5d);  // TechShare event (93 = 0x5d)
    FUN_0048d080();      // Timer checkpoint

    // --- Phase 5: Spawn initial crates if Crates=yes ---
    if (DAT_00a8b261 != 0) {  // Crates option enabled
        // Determine crate count: clamp between Rules min/max
        int crateCount = Rules->InitialCrateCount;  // +0x1470
        if (crateCount <= DAT_00a8b54c) crateCount = DAT_00a8b54c;
        if (Rules->MaxCrateCount <= crateCount)      // +0x1474
            crateCount = Rules->MaxCrateCount;

        for (int i = 0; i < crateCount; i++) {
            FUN_0056bd40();  // Spawn random crate
        }
    }

    // --- Phase 6: Final per-house initialization ---
    FUN_0048d080();      // Timer checkpoint
    FUN_004ac4f0();       // Initialize subsystem
    FUN_005117d0();       // Find special house type
    int neutralHouse = FUN_00502d30();  // Get/create Neutral house

    for (int i = 0; i < HouseClass::Array.Count; i++) {
        HouseClass* house = HouseClass::Array[i];

        // Self-reference for ownership tracking
        house->SelfPtr = house;  // +0x5774

        // For AI non-special houses: set starting timestamps and tech
        if (!house->IsHuman && !house->TypeClass->IsMultiplayOnly) {
            // Get tech level for this side from Rules
            int techLevel = Rules->SideTechLevels[house->SideIndex];
            house->StartFrame     = CurrentFrame;     // +0x5640
            house->StartTimestamp = ???;               // +0x5644
            house->StartTechLevel = techLevel;         // +0x5648

            // Initialize house via vtable (recalc power etc)
            house->SomeInit->vtable->Init();  // +0x24, vtable+0x18

            // Set initial money
            int startMoney = FUN_007c5f00();
            FUN_004f9950(house, startMoney);
        }

        // Set multiplayer visibility flag on house type
        if (!house->TypeClass->IsMultiplayOnly) {  // +0x1a6
            house->TypeClass->ShouldBeVisible = 1;  // +0x1a7
        } else {
            house->TypeClass->ShouldBeVisible = 0;
        }

        // Set ally/enemy flags with neutral house
        if (house != neutralHouse) {
            FUN_004f9b70(neutralHouse, 0);  // Set diplomacy
            FUN_004f9b70(house, 0);
        }
    }

    // --- Phase 7: Network manager post-init ---
    if (NetworkManager != NULL) {
        NetworkManager->vtable->PostInit1();  // +0x88
        NetworkManager->vtable->PostInit2();  // +0x8c
    }

    // --- Phase 8: Mark all houses as initialized ---
    FUN_0068c050();
    DAT_00a8d108 = 0x01010101;  // 4 houses initialized flags
    DAT_00a8d10c = 0x01010101;  // 4 more houses
}
```

**Key insight**: In offline/skirmish mode, `Generate_Random_Units` is called directly.
In multiplayer mode, the network manager's vtable+0x84 handles it, followed by
`FUN_005d6d80` (the network variant of starting unit generation that handles
per-side bitmask filtering differently).

**Confidence: HIGH** — clear decompilation with identifiable globals and string refs.

---

## 2. Generate_Random_Units (0x006886b0) — Full Annotated Pseudocode

The core function for creating MCVs and starting units at game start. Size: 2076 bytes.
Source file: `D:\ra2mdpost\Scenario.CPP` (inferred from caller).

```c
void Generate_Random_Units(void)  // 0x006886b0
{
    // =============================================
    // STEP 1: Calculate starting unit budget
    // =============================================

    int unitCount = DAT_00a8b270;  // [MultiplayerDialogSettings] UnitCount

    if (DAT_00a8b258 != 0) {      // If Bases=yes
        unitCount = unitCount - 1; // MCV counts as 1 unit, subtract from budget
    }

    // Copy SpecialFlags (0xfd dwords = 253 ints = 1012 bytes starting at +0x218)
    memcpy(localFlags, DAT_00a8b230 + 0x86, 0xfd * 4);

    Debug_Printf("Creating %d units - Random seed is %08x\n", unitCount, ...);
    Debug_Printf("UniqueID is %08x\n", DAT_00a8b230[0x85]);

    // --- Compute average unit cost across all spawnable types ---
    // corrected 2026-05-28: was "Loop 1: Infantry, Loop 2: Vehicles";
    // binary iterates UnitTypeClass (vehicles) FIRST, then InfantryTypeClass.
    // Also corrected: forbidden-list (BaseUnitVector) check applies to VEHICLES
    // in this loop, NOT infantry. Infantry have no forbidden-list check here.
    // via decompile_function 0x006886b0 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT
    int totalCost = 0;
    int typeCount = 0;

    // Loop 1: UnitTypeClass/VehicleTypeClass array — checked against BaseUnitVector
    for (int i = 0; i < UnitTypeClass::Array.Count; i++) {
        TypeClass* type = UnitTypeClass::Array[i];

        if (type->Spawnable) {  // +0x6d5 != 0
            // Check if this type is in the forbidden/BaseUnit list
            // (RulesClass+0xb24 = BaseUnit vector data ptr, +0xb30 = count)
            bool isForbidden = false;
            for (int j = 0; j < Rules->BaseUnitVector.Count; j++) {
                if (Rules->BaseUnitVector.Items[j] == type) {
                    isForbidden = true;
                    break;
                }
            }
            if (!isForbidden) {
                int cost = type->vtable->GetCost();  // vtable+0xac
                totalCost += cost;
                typeCount++;
            }
        }
    }

    // Loop 2: InfantryTypeClass array — NO forbidden list check
    for (int i = 0; i < InfantryTypeClass::Array.Count; i++) {
        TypeClass* type = InfantryTypeClass::Array[i];

        if (type->Spawnable) {  // +0x6d5 != 0
            // NOTE: No forbidden list check for infantry in this loop!
            int cost = type->vtable->GetCost();  // vtable+0xac
            totalCost += cost;
            typeCount++;
        }
    }

    int averageCost = totalCost / typeCount;
    int totalBudget = averageCost * unitCount;
    // totalBudget = how much "money" worth of units to create

    // =============================================
    // STEP 2: Gather available start positions
    // =============================================
    Gather_Start_Positions(&startPositions);  // 0x00688380
    // Returns a DynamicVector of CellStruct (packed short x, short y)

    // Track which positions are assigned
    char assigned[16] = {0};  // max 16 start positions
    int assignedCount = 0;

    // =============================================
    // STEP 3: Per-house loop — assign position, create MCV, create units
    // =============================================
    for (int houseIdx = 0; houseIdx < HouseClass::Array.Count; houseIdx++)
    {
        HouseClass* house = HouseClass::Array[houseIdx];

        // Skip special/observer houses
        if (house->TypeClass->IsMultiplayOnly)  // +0x1a6
            continue;

        Debug_Printf("Generating units for house %d (%s)\n",
                      houseIdx, house->TypeClass->InternalName);

        // --- Build filtered type lists for this house ---
        // houseMask = bitmask for this house's side
        uint houseMask = 1 << (house->TypeClass->SideIndex & 0x1f);  // +0xb4

        // corrected 2026-05-28: was "infantry list first, then vehicles";
        // binary builds vehicleList (UnitTypeClass) FIRST, then infantryList.
        // Also corrected: forbidden-list check is on VEHICLES, NOT infantry.
        // via decompile_function 0x006886b0 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT

        // --- vehicleList: vehicle types this house can build ---
        DynamicVector vehicleList;
        for (int i = 0; i < UnitTypeClass::Array.Count; i++) {
            TypeClass* type = UnitTypeClass::Array[i];

            if (type->Spawnable                             // +0x6d5
                && type->TechLevel <= house->TechLevel      // +0x634
                && (type->HouseMask & houseMask) != 0)      // +0x6cc
            {
                // Check forbidden list (BaseUnitVector) for vehicles
                bool isForbidden = false;
                for (int j = 0; j < Rules->BaseUnitVector.Count; j++) {
                    if (Rules->BaseUnitVector.Items[j] == type) {
                        isForbidden = true; break;
                    }
                }
                if (!isForbidden) {
                    vehicleList.Add(type);
                }
            }
        }

        // --- infantryList: infantry types this house can build ---
        DynamicVector infantryList;
        for (int i = 0; i < InfantryTypeClass::Array.Count; i++) {
            TypeClass* type = InfantryTypeClass::Array[i];

            if (type->Spawnable                             // +0x6d5
                && type->TechLevel <= house->TechLevel      // +0x634 <= house+0x1d4
                && (type->HouseMask & houseMask) != 0)      // +0x6cc & mask
            {
                // NOTE: No forbidden list check for infantry here!
                infantryList.Add(type);
            }
        }

        // =============================================
        // STEP 4: Assign start position to this house
        // =============================================
        CellStruct startCell;
        if (assignedCount == 0) {
            // FIRST house: pick random position
            int idx = Random(0, startPositions.Count - 1);
            assigned[idx] = 1;
            assignedCount = 1;
            startCell = startPositions[idx];
        } else {
            // SUBSEQUENT houses: pick position FARTHEST from all assigned
            int distances[26];  // max start positions
            memset(distances, 0, sizeof(distances));

            // Sum distances from each unassigned position to all assigned positions
            for (int i = 0; i < startPositions.Count; i++) {
                if (assigned[i] == 0) {
                    for (int j = 0; j < startPositions.Count; j++) {
                        if (assigned[j] != 0) {
                            short dx = startPositions[i].x - startPositions[j].x;
                            short dy = startPositions[i].y - startPositions[j].y;
                            double dist = sqrt((double)(dx*dx + dy*dy));
                            distances[i] += (short)dist;
                        }
                    }
                }
            }

            // Find position with maximum total distance
            int bestDist = 0;
            int bestIdx = 0;
            for (int i = 0; i < startPositions.Count; i++) {
                if (distances[i] > bestDist || bestDist == 0) {
                    bestDist = distances[i];
                    bestIdx = i;
                }
            }

            assignedCount++;
            assigned[bestIdx] = 1;
            startCell = startPositions[bestIdx];
        }

        // =============================================
        // STEP 5: Set house primary center
        // =============================================
        HouseClass__SetPrimaryCenter(house, startCell);  // 0x0050e000

        // =============================================
        // STEP 6: Create MCV (if Bases=yes)
        // =============================================
        if (DAT_00a8b258 != 0) {  // Bases=yes
            // Allocate UnitClass (0x8e8 bytes)
            void* mem = operator_new(0x8e8);
            UnitClass* mcv = NULL;

            if (mem != NULL) {
                // Pick the correct BaseUnit for this house's side
                UnitTypeClass* baseUnitType =
                    FUN_00505310(Rules + 0xb20, house);
                // ^^^ Iterates RulesClass::BaseUnitVector (offset 0xb20)
                //     Returns first entry whose HouseMask (+0x6cc) matches
                //     the house's side bit. This picks AMCV/SMCV/YMCV.

                mcv = UnitClass::Constructor(mem, baseUnitType, house);
                // FUN_007353c0 — UnitClass constructor
            }

            // Convert cell to lepton coordinates (cell * 256 + 128)
            int leptonX = startCell.x * 256 + 128;
            int leptonY = startCell.y * 256 + 128;
            int leptonZ = 0;

            // Try to place MCV at starting cell
            bool placed = mcv->vtable->Place({leptonX, leptonY, leptonZ});
            // vtable+0xD8

            if (!placed) {
                // Spiral search for alternate placement
                int result = SpiralSearch(mcv, &startCell, 1);  // 0x00688ed0
                if (result == 0) {
                    // Total failure — destroy the MCV
                    if (mcv != NULL) {
                        mcv->vtable->Delete(1);  // vtable+0x20
                    }
                    goto skip_to_starting_units;
                }
            }

            // MCV placed successfully
            if (mcv != NULL) {
                // Clear house's primary factory pointer
                house->PrimaryFactory = 0;            // +0x53dc
                house->PrimaryFactoryCell = SENTINEL;  // +0x53e0

                // --- MCVDeploy: Force immediate deploy if flag set ---
                if (*SpecialFlags & 0x10) {  // MCVDeploy bit
                    // corrected 2026-05-28: function is Force_MCV_Deploy (not AssignPrimaryAndDeploy).
                    // Step 1: Clears rally on old primary (HouseClass__Clear_Rally_Point).
                    // Step 2: Calls UnitClass__AttachFlag(TypeClass+0xb8) @ 0x00740df0
                    //         which sets mcv[0x1b3] = deploy target and queues MISSION_DEPLOY
                    //         via vtable+0x124(2).
                    // Step 3: Sets house->PrimaryFactory = mcv  (+0x53dc)
                    // via decompile_function 0x004fc060 — ROOT_CAUSE: RTTI_LABEL_DRIFT
                    Force_MCV_Deploy(house, mcv, 1);  // 0x004fc060
                }
            }
        }

skip_to_starting_units:
        // =============================================
        // STEP 7: Create remaining starting units
        // =============================================
        int spentBudget = 0;
        DynamicVector placedUnits;  // tracks placed unit pointers

        while (spentBudget < totalBudget)
        {
            TypeClass* unitType = NULL;

            // corrected 2026-05-28: was "first 2/3 = infantry, last 1/3 = vehicles";
            // binary: first 2/3 of budget draws from vehicleList (iStack_c8 = vehicle count),
            // last 1/3 draws from infantryList (iStack_b0 = infantry count).
            // via decompile_function 0x006886b0 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT

            // --- 2/3 vehicles, 1/3 infantry split ---
            if (spentBudget < (totalBudget * 2) / 3 && vehicleList.Count > 0) {
                // First 2/3 of budget: pick random vehicle type
                int idx = Random(0, vehicleList.Count - 1);
                unitType = vehicleList[idx];
            }
            else if (infantryList.Count > 0) {
                // Last 1/3 of budget: pick random infantry type
                int idx = Random(0, infantryList.Count - 1);
                unitType = infantryList[idx];
            }

            // Create the unit object
            TechnoClass* unit = unitType->vtable->CreateObject(house);
            // vtable+0x8C

            // Get it as a placeable type (validates RTTI as unit/infantry/aircraft)
            TechnoClass* placeable = FUN_0040dd70(unit);
            // Returns unit if RTTI is 1 (unit), 2 (infantry), 6 (aircraft), or 15

            // Place via spiral search around start position
            int placed = SpiralSearch(placeable, &startCell, 3);
            // 0x00688ed0, initial radius = 3

            if (placed == 0) {
                // Placement failed — destroy the unit
                if (unit != NULL) {
                    unit->vtable->Delete(1);
                }
            }
            else {
                Debug_Printf("House %s deployed object %s\n",
                             house->TypeClass->InternalName, unitType->Name);

                // Accumulate cost toward budget
                int cost = unitType->vtable->GetCost();  // vtable+0xac
                spentBudget += cost;

                // Track placed unit
                placedUnits.Add(placeable);

                // --- InitialVeteran flag ---
                if (*SpecialFlags & 0x200) {
                    // corrected 2026-05-28: was "Set unit to Veteran rank";
                    // 0x007500b0 is VeterancyStruct__SetElite (not Veteran).
                    // SetElite writes 0x40000000 to veterancy; SetVeteran @ 0x00750090.
                    // via get_function_by_address + decompile_function 0x007500b0
                    // ROOT_CAUSE: RTTI_LABEL_DRIFT
                    VeterancyStruct__SetElite(placeable, 1);  // 0x007500b0
                    // Writes 0x40000000 to unit's veterancy field = Elite rank
                }

                // --- Assign initial mission ---
                bool isHuman = HouseClass__IsHumanControlled(house);
                if (!isHuman) {
                    placeable->vtable->AssignMission(MISSION_AREA_GUARD);
                    // vtable+0x1f0, AI units get Area Guard
                } else {
                    placeable->vtable->AssignMission(MISSION_GUARD);
                    // vtable+0x1f0, Human units get Guard
                }
            }
        }  // end while (budget loop)

        // Cleanup per-house dynamic vectors
        vehicleList.Destroy();
        infantryList.Destroy();

    }  // end for (each house)

    // Final random number log for sync verification
    int syncRandom = Random(0, 0x7FFFFF);
    Debug_Printf("Finished unit generation. Random number is %d\n", syncRandom);

    // Cleanup
    startPositions.Destroy();
    placedUnits.Destroy();
}
```

### Key Observations

1. **Budget calculation**: `averageCost = totalInfVehCost / totalInfVehCount`, then
   `budget = averageCost * unitCount`. If Bases=yes, unitCount is decremented by 1
   (the MCV counts as one unit). Budget is tracked by cost, not unit count -- so
   expensive units consume more budget.

2. **The 2/3 vs 1/3 split is vehicles vs infantry** (corrected 2026-05-28: was
   "infantry vs vehicles" — reversed). The first 2/3 of the cost budget draws from
   UnitTypeClass (vehicles), the last 1/3 from InfantryTypeClass. Loop order in both
   the average-cost pass and the per-house filtering pass is vehicles first, infantry
   second. via decompile_function 0x006886b0 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT.

3. **Forbidden list**: Vehicle types are checked against `RulesClass::BaseUnitVector`
   (the BaseUnit= list) to exclude them from the random pool. Infantry are NOT
   checked against this list (corrected 2026-05-28: was "infantry checked, vehicles
   not" — the direction was reversed). This is correct because MCV types
   (AMCV/SMCV/YMCV) are vehicles, so they need the forbidden-list gate; infantry can
   never be MCVs. In the average cost calculation (Step 1), vehicles ARE checked
   against the forbidden list, but infantry are not.
   via decompile_function 0x006886b0 — ROOT_CAUSE: OPERATOR_OR_ORDER_DRIFT.

4. **HouseMask filtering** (`+0x6cc`): Each type has a bitmask of which sides can
   build it. The house's side index (`TypeClass+0xb4`) is used to compute
   `1 << (sideIndex & 0x1f)`, which is AND'd against the type's mask. This ensures
   Allied houses only get Allied units, etc.

5. **The sync random at the end** is critical for multiplayer lockstep: all players
   must arrive at the same random state after unit generation. The printed value
   lets debugging verify sync.

**Confidence: HIGH** — clear decompilation with debug strings confirming function purpose.

---

## 3. Spiral Placement Function (0x00688ed0) — Full Algorithm

Size: 1045 bytes. Called for MCV placement and all starting unit placement.

```c
// Returns 1 on success, 0 on failure
int SpiralSearch(
    TechnoClass* unit,    // param_1 (ecx) - the unit to place
    CellStruct* target,   // param_2 (edx) - target cell (short x, short y)
    int startRadius       // param_3 (stack) - initial search radius
)
{
    // =============================================
    // PHASE 1: Try exact target cell first
    // =============================================
    bool passable = FUN_00578460(target, 1);  // Check cell passability
    if (passable) {
        CellStruct queryCell = {0, 0};
        FUN_005657a0(target);  // Get cell info

        // Check occupancy at target
        TechnoClass* occupant = FUN_0047c3d0(&queryCell, 0, 0);

        if (occupant == NULL
            || (occupant->GetAbstractType() == 0xF   // both are "ground" type
                && unit->GetAbstractType() == 0xF))
        {
            // Cell is free (or compatible type) — place directly
            int leptonX = target->x * 256 + 128;
            int leptonY = target->y * 256 + 128;
            int leptonZ = 0;
            int groundZ = FUN_00578080({leptonX, leptonY, leptonZ});  // Get ground height

            Coord3D coords = {
                target->x * 256 + 128,
                target->y * 256 + 128,
                groundZ  // was 0, but now corrected
            };

            bool placed = unit->vtable->Place(coords, 0);  // vtable+0xD8
            if (placed) return 1;
        }
    }

    // =============================================
    // PHASE 2: Spiral search with increasing radius
    // =============================================
    // The search tries all 8 compass directions at each radius,
    // then does a second pass with random jitter.

    while (startRadius <= 31) {  // Max radius = 31 cells
        int startDir = Random(0, 7);  // Random starting direction
        int pass = 0;  // 0 = no jitter, 1 = with jitter

        while (pass < 2) {
            int dirsTried = 0;
            int currentDir = startDir;

            while (dirsTried < 8) {
                // Compute candidate cell from target + direction offset
                int cellX = target->x;
                int cellY = target->y;

                // Map bounds
                int mapLeft   = DAT_0087f90c;  // Map left edge
                int mapTop    = DAT_0087f910;  // Map top edge
                int mapRight  = DAT_0087f90c - 1 + DAT_0087f914;  // right edge
                int mapBottom = DAT_0087f910 - 1 + DAT_0087f918;  // bottom edge

                switch (currentDir) {
                    case 0: cellY -= startRadius; break;                         // N
                    case 1: cellX += startRadius; cellY -= startRadius; break;   // NE
                    case 2: cellX += startRadius; break;                         // E
                    case 3: cellX += startRadius; cellY += startRadius; break;   // SE
                    case 4: cellY += startRadius; break;                         // S
                    case 5: cellX -= startRadius; cellY += startRadius; break;   // SW
                    case 6: cellX -= startRadius; break;                         // W
                    case 7: cellX -= startRadius; cellY -= startRadius; break;   // NW
                }

                // Clamp to map bounds
                if (cellX > mapRight)  cellX = mapRight;
                if (cellX < mapLeft)   cellX = mapLeft;
                if (cellY > mapBottom) cellY = mapBottom;
                if (cellY < mapTop)    cellY = mapTop;

                // --- Pass 1 (jitter): add random offset ---
                if (pass > 0) {
                    int jitterAmount = Random(0, 1);  // 0 or 1 cell

                    // X jitter: 50% chance add, 50% chance subtract
                    if (Random(0, 99) < 50)
                        cellX = min(cellX + jitterAmount, mapRight);
                    else
                        cellX = max(cellX - jitterAmount, mapLeft);

                    // Y jitter: same
                    int jitterY = Random(0, 1);
                    if (Random(0, 99) < 50)
                        cellY = min(cellY + jitterY, mapBottom);
                    else
                        cellY = max(cellY - jitterY, mapTop);
                }

                CellStruct candidate = {(short)cellX, (short)cellY};

                // Skip if candidate == target (already tried)
                bool isTarget = (candidate.x == target->x
                                && candidate.y == target->y);

                // Check passability
                bool passable = FUN_00578460(&candidate, 1);

                if (passable && !isTarget) {
                    // Check occupancy
                    CellStruct queryCell = {0, 0};
                    FUN_005657a0(&candidate);
                    TechnoClass* occupant = FUN_0047c3d0(&queryCell, 0, 0);

                    if (occupant == NULL
                        || (occupant->GetAbstractType() == 0xF
                            && unit->GetAbstractType() == 0xF))
                    {
                        // Compute lepton coords with ground height
                        FUN_0041c230(candidate.x * 256 + 128,
                                     candidate.y * 256 + 128, 0);
                        int groundZ = FUN_00578080(leptonCoords);
                        FUN_0041c230(candidate.x * 256 + 128,
                                     candidate.y * 256 + 128, groundZ);

                        bool placed = unit->vtable->Place(coords, 0);
                        if (placed) return 1;  // SUCCESS
                    }
                }

                currentDir = (currentDir + 1) % 8;  // Next direction
                dirsTried++;
            }  // end 8 directions

            pass++;
        }  // end pass (no jitter, then with jitter)

        startRadius++;  // Increase radius
    }  // end radius loop

    return 0;  // FAILURE: no valid position within 31 cells
}
```

### Algorithm Summary

1. **Try exact cell first** (if passable and unoccupied)
2. **For each radius** from `startRadius` to 31:
   - Pick random starting direction (0-7 = N/NE/E/SE/S/SW/W/NW)
   - **Pass 0**: Try all 8 directions at exact offset
   - **Pass 1**: Try all 8 directions again with random jitter (0-1 cells, 50/50 add/subtract per axis)
   - Each try: check passability, check occupancy, attempt Place()
3. **Clamp** all candidates to map bounds
4. Return 0 if nothing worked within radius 31

**Confidence: HIGH** — clear algorithmic structure, direction switch cases match compass directions.

---

## 4. SetPrimaryCenter (0x0050e000) — Trivial Setter

Size: 13 bytes. One of the simplest functions in the engine.

```c
void HouseClass::SetPrimaryCenter(HouseClass* this, CellStruct cell)
{
    this->BaseCenterPrimary = cell;  // offset +0x5490
    // CellStruct is a packed (short x, short y) = 4 bytes total
}
```

Called by:
- `AssignStartingPoints` (0x005ee9d0) — sets base center for each player
- `Generate_Random_Units` (0x006886b0) — sets base center before MCV creation
- `UnitClass::Deploy` (0x007393c0) — updates base center when MCV deploys
- `FUN_006dd8b0` — AI base center update

**HouseClass base center offsets:**
| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| +0x5490 | 4 | BaseCenterPrimary | CellStruct (packed short x, short y) |
| +0x5494 | 4 | BaseCenterAlternate | Secondary base center |

**Confidence: HIGH** — trivial function, fully decompiled.

---

## 5. BaseUnit in RulesClass — DynamicVector at Offset 0xb20

### How BaseUnit is Parsed

In `FUN_0066d530` (the [General] section parser, 18,793 bytes), BaseUnit is read
as follows:

```c
// Address: ~0x0066F8C0 (within FUN_0066d530)

// Read "BaseUnit" as a string from [General]
int found = ReadString("[General]", "BaseUnit", defaultBuf, outBuf, 0x80);

if (found == 0) {
    // No BaseUnit key — copy default vector
    CopyVector(Rules->BaseUnitVector);
} else {
    // Parse comma-separated list of UnitTypeClass names
    DynamicVector tempVec;
    InitVector(&tempVec, 0, 0);

    char* token = strtok(outBuf, ",");
    while (token != NULL && *token != '\0') {
        UnitTypeClass* unitType = UnitTypeClass::FindOrCreate(token);
        // FUN_007480d0 — looks up existing or creates new
        if (unitType != NULL) {
            tempVec.Add(unitType);
        }
        token = strtok(NULL, ",");
    }

    // Assign to RulesClass
    CopyVector(Rules->BaseUnitVector);
}

// Final vector stored at RulesClass offsets:
// +0xb20: vtable pointer (DynamicVectorClass vtable)
// +0xb24: Items pointer (UnitTypeClass** — array of pointers)
// +0xb28: (internal allocator state)
// +0xb2c: (internal allocator state)
// +0xb30: Count (number of entries)
// +0xb34: Capacity
// +0xb38: (growth flag)
```

### How BaseUnit is Used at Game Start

In `Generate_Random_Units`, the BaseUnit is selected per-house:

```c
// At address 0x00688B5C:
UnitTypeClass* baseUnit = FUN_00505310(Rules + 0xb20, house);
```

`FUN_00505310` (79 bytes) iterates the BaseUnit DynamicVector and returns the
**first entry whose `HouseMask` (+0x6cc) matches the current house's side bit**:

```c
int FindMatchingBaseUnit(DynamicVectorClass* vec)  // 0x00505310
{
    byte sideIndex = FUN_005117d0();  // Get current processing house's side

    for (int i = 0; i < vec->Count; i++) {       // vec+0x10 = count
        TypeClass* type = vec->Items[i];           // vec+0x4 = data ptr
        if ((type->HouseMask & (1 << (sideIndex & 0x1f))) != 0) {
            return vec->Items[i];  // Return first match
        }
    }
    return 0;  // No match
}
```

### INI Configuration

In `rulesmd.ini` under `[General]`:
```ini
BaseUnit=AMCV,SMCV,YMCV
```

This creates a 3-element vector:
- AMCV (Allied MCV) — HouseMask includes Allied side bits
- SMCV (Soviet MCV) — HouseMask includes Soviet side bits
- YMCV (Yuri MCV) — HouseMask includes Yuri side bits

The `FUN_00505310` call then picks the correct one based on side.

**Also stored nearby in RulesClass:**
| Offset | INI Key | Type |
|--------|---------|------|
| +0xb20 | BaseUnit | DynamicVector\<UnitTypeClass*\> |
| +0xb3c | (gap/padding) |  |
| +0xb4c | HarvesterUnit | DynamicVector\<UnitTypeClass*\> (count at +0xb4c) |
| +0xb58 | PadAircraft | DynamicVector\<AircraftTypeClass*\> |

**Confidence: HIGH** — traced from INI parsing through to usage in Generate_Random_Units.

---

## 6. TypeClass Fields Used for Starting Unit Filtering

These fields on InfantryTypeClass/UnitTypeClass control eligibility:

| Offset | Size | Field | Filter Role |
|--------|------|-------|-------------|
| +0x634 | 4 | TechLevel | Must be <= house's TechLevel (+0x1d4) |
| +0x6cc | 4 | HouseMask | Bitmask; must have house's side bit set |
| +0x6d5 | 1 | Spawnable | Must be != 0 to be eligible for random start |

The `Spawnable` flag (`+0x6d5`) is the key gate. Only types marked `Spawnable=yes`
in rules.ini can appear as starting units. The `TechLevel` check uses the house's
current tech level, which comes from the multiplayer dialog settings or the map's
per-house configuration.

**Confidence: HIGH** — offsets confirmed from both the filtering code and cross-reference
with other ReadINI reports.

---

## 7. SpecialFlags Bit Layout (from 0x006b8b30 / 0x006b8ca0)

The SpecialFlags bitfield is at `DAT_00a8b230` (first dword of ScenarioClass or
a separate SpecialClass structure).

| Bit | Hex Value | Flag | Used In Generate_Random_Units |
|-----|-----------|------|-------------------------------|
| 4 | 0x10 | MCVDeploy | Yes — forces MCV auto-deploy at game start |
| 5 | 0x20 | Inert | No |
| 6 | 0x40 | TiberiumGrows | No |
| 7 | 0x80 | TiberiumSpreads | No |
| 9 | 0x200 | InitialVeteran | Yes — grants Veteran rank to starting units |
| 10 | 0x400 | FixedAlliance | No |
| 11 | 0x800 | HarvesterImmune | No |
| 12 | 0x1000 | FogOfWar | No |
| 14 | 0x4000 | TiberiumExplosive | No |
| 15 | 0x8000 | DestroyableBridges | No |

**NOTE on bit numbering**: The save/load functions (0x006b8b30/0x006b8ca0) use
`>> 8` for MCVDeploy when serializing to INI. However, in the runtime bitfield
accessed by Generate_Random_Units, the check is `& 0x10` (bit 4). This difference
exists because the save function extracts individual bytes and shifts them, while
the runtime bitfield may have a different memory layout or the SpecialClass structure
has additional leading bytes before the flags. The functional behavior is consistent:
when MCVDeploy is enabled in the lobby, `*SpecialFlags & 0x10` is set, and MCVs
auto-deploy.

**Confidence: MEDIUM** — the bit position discrepancy between serialization (>>8) and
runtime check (& 0x10) needs further investigation. The functional behavior is confirmed.

---

## 8. Force_MCV_Deploy (0x004fc060) — Primary + Deploy

Size: 78 bytes. Called when MCVDeploy flag is set.

> corrected 2026-05-28: function name was "HouseClass::AssignPrimaryAndDeploy";
> Ghidra label is `Force_MCV_Deploy`. Also corrected: this does NOT call
> `FUN_00740df0(unit, deploysIntoIdx)`. Instead it calls
> `UnitClass__AttachFlag(TypeClass+0xb8)` which internally sets `unit[0x1b3]`
> and calls `vtable+0x124(2)` (QueueMission MISSION_DEPLOY). The old pseudocode
> attributed the wrong caller for the deploy-index argument.
> via decompile_function 0x004fc060 — ROOT_CAUSE: RTTI_LABEL_DRIFT + INFERENCE_HARDENED

```c
int Force_MCV_Deploy(
    HouseClass* this,     // ecx  (param_1)
    TechnoClass* unit,    // param_2 — the MCV unit
    undefined4 param_3    // rally-point clear arg
)
{
    if (unit == NULL || unit->InLimbo)  // unit+0x81
        return 0;

    // Clear rally point on old primary factory
    HouseClass__Clear_Rally_Point(this->PrimaryFactory, param_3);  // +0x53dc

    // Attach deploy flag: sets deploy-index from TypeClass+0xb8, queues MISSION_DEPLOY
    // UnitClass__AttachFlag @ 0x00740df0:
    //   if (unit[0x1b3] == -1) {
    //       unit[0x1b3] = param_2  (TypeClass+0xb8 value);
    //       unit->vtable->QueueMission(MISSION_DEPLOY);  // vtable+0x124, arg=2
    //   }
    UnitClass__AttachFlag(*(TypeClass+0xb8));  // 0x00740df0

    // Set this unit as the house's primary factory
    this->PrimaryFactory = unit;  // +0x53dc

    return 1;
}
```

This is the link between Generate_Random_Units and the MCV deploy path documented
in MCV_DEPLOY_GHIDRA_REPORT.md. After this function, the MCV will process its
Deploy mission on the next sim tick, calling `UnitClass::Deploy` (0x007393c0)
which creates the Construction Yard building.

**Confidence: HIGH** — fully decompiled, small function with clear purpose.

---

## 9. Gather_Start_Positions (0x00688380) — Full Algorithm

Size: 813 bytes. Collects waypoints 0-7 from the map into a position vector.

```c
DynamicVector* Gather_Start_Positions(DynamicVector* outVec)  // 0x00688380
{
    // Count valid waypoints (0 through 7)
    int validWaypoints = 0;
    short* waypointPtr = (short*)(ScenarioClass + 0x632);  // Waypoint array

    for (int i = 0; i < 8; i++) {
        // Check if waypoint i is past the array or is sentinel
        if (i > 701 || i < 0) break;
        if (waypointPtr[0] == SENTINEL_X && waypointPtr[1] == SENTINEL_Y) break;
        validWaypoints++;
        waypointPtr += 2;  // Each waypoint = 4 bytes (2 shorts)
    }

    // Count how many positions we need
    // = (human players - observers) + AI players
    int observers = 0;
    for (int i = 0; i < humanPlayerCount; i++) {
        if (PlayerList[i]->SpawnLocation == -1)  // +0x6b
            observers++;
    }
    int needed = (humanPlayerCount - observers) + aiPlayerCount;
    if (needed < validWaypoints) needed = validWaypoints;

    // Collect valid waypoints into output vector
    for (int i = 0; i < needed; i++) {
        if (i < 702 && i >= 0) {
            CellStruct wp = *(CellStruct*)(ScenarioClass + 0x632 + i * 4);
            if (wp.x != SENTINEL_X || wp.y != SENTINEL_Y) {
                outVec->Add(wp);
                Debug_Printf("Multiplayer start waypoint found at cell %d,%d\n",
                             wp.x, wp.y);
            }
        }
    }

    // If not enough waypoints, generate random positions
    if (needed != outVec->Count && (needed - outVec->Count) >= 0) {
        Debug_Printf("Multiplayer start waypoint deficiency - "
                     "looking for more start positions\n");

        while (outVec->Count < needed) {
            // Generate random position within map bounds (with 10-cell margin)
            short randY = Random(0, mapHeight - 10);
            short randX = Random(10, mapWidth - 10);

            CellStruct candidate = {
                randX + mapLeft,
                randY + 10 + mapTop
            };

            // Find open cell with 8x8 clearance
            CellStruct result;
            FUN_0056dc20(&result, &candidate, 1, -1, 0, 0, 8, 8, 0, 0, 0, 1, ...);

            if (result.x != SENTINEL_X || result.y != SENTINEL_Y) {
                outVec->Add(result);
                Debug_Printf("Random multiplayer start waypoint added at cell %d,%d\n",
                             result.x, result.y);
            }
        }
    }

    // Transfer vector data to output parameter
    // (DynamicVector move semantics)
    return outVec;
}
```

**Key detail**: The fallback random position finder (`FUN_0056dc20`) requires an
**8x8 cell clearance** — enough space for a Construction Yard foundation. This
ensures auto-generated spawn points always have room for an MCV to deploy.

**Confidence: HIGH** — debug strings confirm every branch.

---

## 10. Complete Call Chain Summary

```
ScenarioClass::Full_Init (0x00686b20)
  |
  +-- Post_Map_Init (0x00686890)
        |
        +-- [if offline] Generate_Random_Units (0x006886b0)
        |     |
        |     +-- Gather_Start_Positions (0x00688380)
        |     |     +-- Reads waypoints 0-7 from ScenarioClass+0x632
        |     |     +-- FUN_0056dc20 for random fallback positions (8x8 clearance)
        |     |
        |     +-- For each house:
        |     |     +-- FUN_0050e000 (SetPrimaryCenter at house+0x5490)
        |     |     +-- FUN_00505310 (pick BaseUnit by side from RulesClass+0xb20)
        |     |     +-- operator_new(0x8e8) + UnitClass__Constructor (FUN_007353c0)
        |     |     +-- vtable+0xD8 (Place at cell) or FUN_00688ed0 (spiral fallback)
        |     |     +-- [if MCVDeploy] Force_MCV_Deploy(0x004fc060)
        |     |           → UnitClass__AttachFlag(0x00740df0) (sets deploy-index + queues MISSION_DEPLOY)
        |     |     +-- Starting unit loop (vehicles first 2/3, infantry last 1/3):
        |     |           +-- vtable+0x8C (CreateObject)
        |     |           +-- Filter_AbstractType_InMap (FUN_0040dd70, RTTI type check)
        |     |           +-- FUN_00688ed0 (spiral placement, radius 3)
        |     |           +-- [if InitialVeteran] VeterancyStruct__SetElite(0x007500b0) (Elite rank)
        |     |           +-- vtable+0x1F0 (assign Guard/AreaGuard mission)
        |     |
        |     +-- Sync random verification
        |
        +-- [if multiplayer] NetworkManager->GenerateUnits + FUN_005d6d80
        |
        +-- Crate spawning (if Crates=yes)
        +-- Per-house final init (tech level, diplomacy, timers)
```

---

## Confidence Summary

| Area | Confidence | Notes |
|------|-----------|-------|
| Post_Map_Init flow | HIGH | Clear decompilation; function names corrected 2026-05-28 |
| Generate_Random_Units overall | HIGH | Debug strings confirm every phase |
| Budget calculation (avg cost * count) | HIGH | Arithmetic is unambiguous |
| 2/3 vehicles / 1/3 infantry split | HIGH | Corrected 2026-05-28: direction was reversed; binary confirmed |
| Vehicle forbidden-list check (not infantry) | HIGH | Corrected 2026-05-28: direction was reversed; binary confirmed |
| Spawnable/TechLevel/HouseMask filtering | HIGH | Field offsets confirmed from multiple sources |
| BaseUnit DynamicVector at RulesClass+0xb20 | HIGH | Traced from INI parsing to usage |
| FUN_00505310 side-matching lookup | HIGH | Fully decompiled, clear logic |
| Spiral placement algorithm | HIGH | Complete algorithm with all 8 directions |
| SetPrimaryCenter trivial setter | HIGH | 13-byte function, fully clear |
| MCVDeploy bit position (0x10) | HIGH | Confirmed via decompile_function 0x006886b0; serialization discrepancy is separate concern |
| Force_MCV_Deploy (0x004fc060) | HIGH | Corrected 2026-05-28: calls UnitClass__AttachFlag not FUN_00740df0 directly |
| InitialVeteran grants Elite (not Veteran) | HIGH | Corrected 2026-05-28: VeterancyStruct__SetElite @ 0x007500b0 |
| Gather_Start_Positions | HIGH | Debug strings confirm branches |
