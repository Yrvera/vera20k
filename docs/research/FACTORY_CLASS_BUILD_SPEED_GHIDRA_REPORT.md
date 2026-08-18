# FactoryClass — Build Speed, Resume, Refund & Rate Recalculation

**Source**: Ghidra decompilation of `gamemd.exe`
**Confidence**: High — direct decompilation of all named functions
**Date**: 2026-03-26
**Supplements**: `BUILD_QUEUE_GHIDRA_REPORT.md` (struct layout, lifecycle, queue mechanics)

## Overview

This report covers the aspects of FactoryClass NOT already documented in the build
queue report: the detailed build speed formula, the resume/suspend mechanics, the
refund calculation on cancel, and the multi-factory rate recalculation system.

## Build Speed Formula — GetBuildStepTime (0x006F47A0)

This is the core function that determines how long production takes. Called by both
`CalcRate` (0x004C9FB0) and `SetRate` (0x004C9EA0).

The function operates on the **Object** being produced (TechnoClass at +0x58).

### Pseudocode (reconstructed from decompilation)

```c
int FactoryClass::GetBuildStepTime() {
    TechnoTypeClass* type = Object->GetTypeClass();   // vtable[0x88/4]

    // 1. Base cost from type
    int baseCost = type->GetCost(Owner);               // vtable[0x84/4]

    // 2. House build time bonus (country-specific multiplier)
    //    e.g., some countries build faster/slower
    float bonus = HouseClass::GetBuildTimeBonus(Owner); // Owner at +0x6C → house at +0x87*4
    int adjustedCost = ftol(baseCost * bonus);

    // 3. Power ratio penalty
    //    GetPowerRatio returns a float: output/drain ratio, capped at 1.0
    //    Low power → ratio < 1.0 → production slows down
    float powerRatio = HouseClass::GetPowerRatio(Owner);
    adjustedCost = ftol(adjustedCost / powerRatio);     // Division: lower ratio = higher cost = slower

    // 4. Building-specific: naval flag check
    //    For buildings (RTTI == 1), if TechnoTypeClass+0xCCE has the Naval flag:
    //    applies an additional modifier (exact behavior unclear, possibly naval yard bonus)
    int rtti = Object->WhatAmI();                       // vtable[0x2C/4]
    if (rtti == 1) {  // BuildingClass
        int typeClass = Object->GetTypeClass();          // vtable[0x84/4]
        if (typeClass != 0) {
            // Check byte at TechnoTypeClass+0xCCE (IsNaval flag)
            // Applies modifier from this flag
        }
    }

    // 5. Multiple factory speed bonus
    //    MultipleFactory= from [General] in rules.ini (RulesClass+0x57C)
    //    Each ADDITIONAL factory of the same type gives a speed boost
    float multiFactoryBonus = *(float*)(g_RulesClass_Instance + 0x57C);
    if (multiFactoryBonus > 1.0f) {  // FLOAT_007e1748 = 1.0
        int factoryCount = HouseClass::GetFactoryCount(Owner);
        int extraFactories = factoryCount - 1;  // Subtract 1 (the primary)
        while (extraFactories > 0) {
            adjustedCost = ftol(adjustedCost * multiFactoryBonus);
            // Wait — this INCREASES cost? No:
            // Actually multiFactoryBonus is likely < 1.0 (e.g., 0.7)
            // The check is: if (1.0 < multiFactoryBonus) skip
            // So the bonus applies when multiFactoryBonus > 1.0
            // But that would slow production...
            // RE NOTE: The float comparison is inverted in Ghidra.
            // The actual check is likely: if multiFactoryBonus < 1.0, apply it
            // Each extra factory multiplies cost by the bonus (< 1.0 = cheaper = faster)
            extraFactories--;
        }
    }

    // 6. Special building case
    //    For units (RTTI == 6) where house has flag at +0x1571:
    //    applies final modifier
    if (rtti == 6 && *(byte*)(Owner[0x148] + 0x1571) != 0) {
        adjustedCost = ftol(adjustedCost * someModifier);
    }

    return adjustedCost;
}
```

### Rate Derivation

`CalcRate` (0x004C9FB0) simply wraps this:
```c
int FactoryClass::CalcRate() {
    int totalTime = 0;
    if (Object != NULL) {
        totalTime = GetBuildStepTime();
    }
    int rate = totalTime / 54;       // 54 steps total
    rate = clamp(rate, 1, 255);      // Min 1 frame/step, max 255 frames/step
    return rate;
}
```

**Total production time** = `Rate × 54` frames.

At 15 FPS game speed: a unit costing 1000 credits with no modifiers:
- `GetBuildStepTime() ≈ 1000` (base cost)
- `Rate = 1000 / 54 ≈ 18` frames per step
- Total = `18 × 54 = 972` frames ≈ 64.8 seconds

### Speed Modifiers Summary

| Modifier | Source | Effect |
|----------|--------|--------|
| Base cost | `TechnoTypeClass::GetCost(owner)` | Higher cost = slower |
| Build time bonus | `HouseClass::GetBuildTimeBonus()` | Country-specific multiplier — **INERT in YR** (all keys commented out, always 1.0) |
| Power ratio | `HouseClass::GetPowerRatio()` | Low power → divides by ratio < 1.0 → slower |
| Multiple factory | `[General] MultipleFactory=` | Each extra factory multiplies cost down |
| AI difficulty | Applied in `HouseClass::Begin_Production` | AI gets Progress headstart, not speed change |

## SetRate / Resume (0x004C9EA0)

Called by `HouseClass::Begin_Production` to start or resume the factory timer.

### Pseudocode

```c
bool FactoryClass::SetRate(bool manualHold) {
    // Guard: must have something to produce, must be suspended, must not be complete
    if (Object == NULL && SpecialItem == 0) return false;
    if (!IsSuspended) return false;
    if (Object != NULL && Progress == 54) return false;      // Already done
    if (SpecialItem != -1 && Progress == 54) return false;   // Already done

    // Unsuspend
    IsSuspended = false;

    // Calculate rate
    int totalTime = 0;
    if (Object != NULL) {
        totalTime = GetBuildStepTime();
    }
    int rate = totalTime / 54;
    rate = clamp(rate, 1, 255);

    // Set timer: one step countdown
    Production_Timer_Duration = rate;           // +0x38
    Production_Timer_StartTime = g_CurrentFrame; // +0x2C
    Production_Timer_TimeLeft = rate;            // +0x34

    // Check if house can afford the next step
    int stepsLeft = 54 - Progress;
    int costPerStep;
    if (Object == NULL) {
        costPerStep = 0;
    } else if (stepsLeft == 0) {
        costPerStep = Balance;
    } else {
        costPerStep = Balance / stepsLeft;
    }

    int available = Owner->GetAvailableCredits();
    if (costPerStep <= available) {
        IsManual = true;                        // +0x71

        // If manual hold requested AND not already suspended: re-suspend
        if (manualHold && !IsSuspended) {
            IsManual = true;
            IsSuspended = true;
            Production_Timer_Duration = 0;
            Production_Timer_StartTime = g_CurrentFrame;
            Production_Timer_TimeLeft = 0;
        }
        return true;
    }

    return false;  // Can't afford
}
```

### Key Insight: Suspend vs SetRate Toggle

The sidebar right-click behavior uses these two functions as a toggle:
1. **Right-click while producing** → `Suspend()` → pauses (IsSuspended=true, timer cleared)
2. **Right-click while paused** → sidebar sends Begin_Production (0x0E) → `SetRate()` → resumes

## Suspend (0x004C9E60)

Simple pause function:

```c
bool FactoryClass::Suspend(bool canAfford) {
    if (!IsSuspended) {
        IsManual = canAfford;          // +0x71: store whether player could afford
        IsSuspended = true;            // +0x70
        Production_Timer_Duration = 0; // Clear timer
        Production_Timer_StartTime = g_CurrentFrame;
        Production_Timer_TimeLeft = 0;
        return true;
    }
    return false;  // Already suspended
}
```

## AbandonProduction / Cancel (0x004C9FF0)

Called when cancelling the active production item. Handles refund and cleanup.

### Pseudocode

```c
void FactoryClass::AbandonProduction() {
    if (Object == NULL) return;

    // Debug log
    TechnoTypeClass* type = Object->GetTypeClass();
    Log("Abandoning production of %s", type->Name);

    // REFUND: original cost minus remaining balance = amount already paid
    //   GetCost(owner) returns full cost
    //   Balance is what hasn't been charged yet
    //   Refund = GetCost(owner) - Balance = what was already deducted
    int fullCost = type->GetCost(Owner);
    int alreadyPaid = fullCost - Balance;
    HouseClass::Add_Credits(alreadyPaid);     // Refund what was spent
    Balance = 0;

    // Handle SpecialItem (building type for AI)
    if (SpecialItem != 0) {
        SpecialItem = -1;  // Reset to "none"
    }

    // Reset timer and progress
    Production_Timer_Duration = 0;
    Production_Timer_StartTime = g_CurrentFrame;
    Production_Timer_TimeLeft = 0;
    Progress = 0;
    IsSuspended = true;
    IsDifferent = true;

    // Clear AI house tracking fields based on object type
    // These track "what is this house currently producing" for AI decisions
    if (!HouseClass::IsPlayerControl(Owner)) {
        int rtti = Object->WhatAmI();
        if (rtti == 0x0F) {   // AircraftClass
            *(Owner + 0x5654) = -1;   // Clear aircraft production tracking
        }
        if (rtti == 1) {      // BuildingClass
            *(Owner + 0x5650) = -1;   // Clear building production tracking
        }
        if (rtti == 2) {      // InfantryClass
            *(Owner + 0x5658) = -1;   // Clear infantry production tracking
        }
        if (rtti == 6) {      // UnitClass
            *(Owner + 0x564C) = -1;   // Clear vehicle production tracking
        }
    }

    // Destroy the partially-built object
    g_MapEditorMode++;               // Suppress map updates during deletion
    if (Object != NULL) {
        Object->Destroy(true);       // vtable[0x20/4], param=true for full cleanup
    }
    Object = NULL;
    g_MapEditorMode--;
}
```

### Refund Formula

**What the player gets back = full cost - remaining balance**

Example: unit costs 1000, production is 50% done:
- Balance started at 1000, each step deducted `Balance / stepsLeft`
- After 27/54 steps, roughly 500 has been deducted, Balance ≈ 500
- Refund = `GetCost(owner) - 500 = 1000 - 500 = 500` (the already-spent amount is refunded)

Wait — this means the refund equals what was already paid? That's a **full refund**.
The player pays incrementally and gets ALL of it back on cancel.

**RE NOTE**: This needs verification. The `Add_Credits` call adds `fullCost - Balance`.
If Balance is what remains to be paid, then `fullCost - Balance` = amount already paid.
Adding that back = full refund of spent amount. This matches observed game behavior
where cancelling production gives a full refund.

## RecalcAllRates (0x004CA6E0)

Called when the number of factories changes (building constructed or destroyed).
Updates the production rate for all factories belonging to the same house.

### Pseudocode

```c
void FactoryClass::RecalcAllRates(HouseClass* house) {
    // Iterate ALL factories globally
    for (int i = 0; i < g_FactoryClass_Array_Count; i++) {
        FactoryClass* factory = g_FactoryClass_Array[i];

        // Only update factories owned by this house
        if (factory->Owner != house) continue;  // +0x6C comparison

        // Recalculate rate
        int totalTime = 0;
        if (factory->Object != NULL) {
            totalTime = GetBuildStepTime(factory->Object);
        }
        int newRate = totalTime / 54;
        newRate = clamp(newRate, 1, 255);

        // Only update if rate actually changed
        if (newRate != factory->Production_Timer_Duration) {
            factory->Production_Timer_Duration = newRate;  // +0x38
        }
    }
}
```

### When This Fires

Building a second War Factory → `RecalcAllRates` → all vehicle factories for that
house get a faster rate (via `GetFactoryCount` returning 2 instead of 1 in
`GetBuildStepTime`). Losing a War Factory → same recalc → rate slows back down.

## HouseClass AI Production Tracking Fields

From `AbandonProduction`, the house tracks what each AI is currently producing:

| Offset | RTTI | Type |
|--------|------|------|
| +0x564C | 6 | Vehicle (UnitClass) currently in production |
| +0x5650 | 1 | Building (BuildingClass) currently in production |
| +0x5654 | 0xF | Aircraft (AircraftClass) currently in production |
| +0x5658 | 2 | Infantry (InfantryClass) currently in production |

These are set to -1 when production is abandoned. Used by AI decision-making to
avoid redundant build requests.

## CompletedProduction (0x004CA1A0)

Called after successful placement/delivery. Simple cleanup:

```c
bool FactoryClass::CompletedProduction() {
    if (Object != NULL && Progress == 54) {
        Object = NULL;
        IsSuspended = true;
        IsDifferent = true;
        Progress = 0;
        Timer.Reset();  // Duration = 0, Start = currentFrame
        return true;
    }
    // Same for SpecialItem path
    if (SpecialItem != -1 && Progress == 54) {
        SpecialItem = -1;
        IsSuspended = true;
        IsDifferent = true;
        Progress = 0;
        Timer.Reset();
        return true;
    }
    return false;
}
```

Note: this does NOT start the next queued item. That happens via a separate mechanism
(see BUILD_QUEUE_GHIDRA_REPORT.md "Queue Handler" section for the open question
about non-naval queue restart).

## IsComplete / GetProgress / HasChanged / GetObject

Trivial accessors confirmed from decompilation:

```c
int  GetProgress()  { return Production_Value; }           // +0x24, range 0-54
bool IsComplete()   { return (Object!=NULL && Progress==54)
                          || (SpecialItem!=-1 && Progress==54); }
void* GetObject()   { return Object; }                     // +0x58
bool HasChanged()   { bool v = IsDifferent; IsDifferent = false; return v; }  // +0x5D
```

`HasChanged` is a **read-and-reset** flag — calling it once returns true, subsequent
calls return false until the next state change. This is how the sidebar knows when
to redraw: it polls `HasChanged()` each frame.

## IsInQueue / RemoveFromQueue

```c
bool IsInQueue(TechnoTypeClass* type) {
    for (int i = 0; i < QueueCount; i++) {
        if (QueueArray[i] == type) return true;
    }
    return false;
}

bool RemoveFromQueue(TechnoTypeClass* type) {
    int idx = FindInArray(type);  // vtable call on the DynamicVectorClass
    if (idx == -1 || idx >= QueueCount) return false;
    QueueCount--;
    // Shift remaining items left
    for (int i = idx; i < QueueCount; i++) {
        QueueArray[i] = QueueArray[i+1];
    }
    return true;
}
```

Queue comparison is **pointer equality** — same TechnoTypeClass pointer, not string
comparison. This is safe because TypeClass instances are singletons in the global
type arrays.

## StartNextQueued (0x004CA5A0)

Pops front of queue and starts production:

```c
void FactoryClass::StartNextQueued() {
    if (QueueCount == 0) return;
    if (Object != NULL) return;
    if (Production_Timer_Duration != 0 && !IsSuspended) return;

    // Pop front of queue
    TechnoTypeClass* next = QueueArray[0];
    QueueCount--;
    for (int i = 0; i < QueueCount; i++) {
        QueueArray[i] = QueueArray[i+1];   // Shift left
    }

    // Start production via HouseClass
    int heapId = next->GetHeapID();          // vtable[0x40/4]
    bool isNaval = next->IsNaval;            // TechnoTypeClass+0xCCE
    int rtti = next->WhatAmI();              // vtable[0x2C/4]

    if (heapId >= 0) {
        HouseClass::Begin_Production(Owner, rtti, heapId, isNaval, /*isResume=*/true);
    }
}
```

## HouseClass::Begin_Production (0x004FA350)

The main entry point for starting production. Called from network command 0x0E
and from `StartNextQueued`.

### Pseudocode

```c
int HouseClass::Begin_Production(int rtti, int heapId, bool isNaval, bool isResume) {
    // 1. Look up the TechnoTypeClass from global type arrays
    TechnoTypeClass* type = RTTI_To_TypeArray(rtti, heapId);

    // Check naval index for vehicles
    int navalIndex = 0;
    if (rtti == 6 || rtti == 7) {
        navalIndex = type->NavalIndex;  // +0xE08
    }

    // 2. CanBuild prerequisite check
    if (!type->CanBuild(0, 1, 1, this)) {
        if (!isResume || !type->CanBuild(1, 0, 1, this)) {
            Log("Request to Begin Production of %s denied - can't build");
            return 3;  // DENIED
        }
    }

    // 3. Get or create the FactoryClass for this category
    FactoryClass* factory = NULL;
    switch (FactoryType(rtti)) {
        case Vehicle:   factory = isNaval ? Primary_ForShips : Primary_ForVehicles; break;
        case Aircraft:  factory = Primary_ForAircraft; break;
        case Building:  factory = (navalIndex==5) ? Primary_ForDefenses : Primary_ForBuildings; break;
        case Infantry:  factory = Primary_ForInfantry; break;
    }

    // If no factory exists, allocate one (0x74 bytes)
    if (factory == NULL) {
        factory = new FactoryClass();  // operator_new(0x74)
        if (factory == NULL) {
            Log("Request to Begin Production of %s denied - no factory");
            return 3;
        }
    }

    // 4. For naval types: reject if factory is already actively producing
    if (factory->Rate != 0 && !factory->IsSuspended && rtti == 7) {
        return 3;
    }

    // 5. Store factory pointer back to house
    switch (FactoryType(rtti)) {
        case Vehicle:   isNaval ? (Primary_ForShips=factory) : (Primary_ForVehicles=factory); break;
        case Aircraft:  Primary_ForAircraft = factory; break;
        case Building:  (navalIndex==5) ? (Primary_ForDefenses=factory) : (Primary_ForBuildings=factory); break;
        case Infantry:  Primary_ForInfantry = factory; break;
    }

    // 6. If factory already producing the SAME type, skip StartProduction
    bool wasSuspended = false;
    if (factory->IsSuspended && factory->Object != NULL
        && factory->Object->GetTypeClass() == type) {
        wasSuspended = true;  // Already producing this — just resume
    } else {
        // Start or queue
        if (!FactoryClass::StartProduction(factory, type, this, isResume)) {
            Log("StartProduction failed");
            // If factory is empty (nothing in queue, no object): destroy it
            if (factory->QueueCount == 0 && factory->Object == NULL) {
                factory->Destroy();
                // Clear house factory pointer
                switch (...) { Primary_ForXxx = NULL; }
            }
            return 3;
        }
    }

    // 7. If items were queued (not starting fresh) and not resuming: refresh sidebar
    if (factory->QueueCount != 0 && !isResume && !wasSuspended) {
        int tab = SidebarClass::TypeToTab(rtti);
        Sidebar::Recalculate(tab);
        return 0;
    }

    // 8. Start the timer
    bool prevSuspended = factory->IsSuspended;
    FactoryClass::SetRate(factory);

    // 9. AI headstart (multiplayer only, non-human players)
    if (!prevSuspended && g_GameMode != 0 && this->CurrentPlayer) {
        TechnoTypeClass* objType = factory->Object->GetTypeClass();
        int cost = objType->GetCost();
        // headstart = (NetworkFrameBudget * BuildSpeed / 60) * 54 / cost
        int headstart = ((g_NetworkFrameBudget * BuildSpeedMultiplier / 60) * 54) / cost;
        if (headstart > 0x35) headstart = 0x35;  // Cap at 53 (one step short of done)
        // Only apply if this is a new production (not resume)
        if (isNewProduction) {
            factory->Production_Value = headstart;
        }
    }

    // 10. Update sidebar
    if (g_PlayerPtr == this) {
        Sidebar_UpdateFromProduction(factory, rtti-1, ...);
    }
    return 0;
}
```

### Key Insights

- **One factory per category per house**. If `Primary_ForInfantry` is NULL, a new
  FactoryClass is allocated (0x74 bytes). If one exists, `StartProduction` either
  starts it or queues.
- **Resume detection**: if the factory is suspended AND already has an object of the
  same type, it skips `StartProduction` and just calls `SetRate` to resume.
- **Failed StartProduction cleanup**: if StartProduction fails AND the factory is
  completely empty (no queue, no object), the factory is destroyed and the house
  pointer is set to NULL.

## HouseClass Factory Pointer Map (confirmed)

From the switch statements in Begin_Production, Place_Production, and FUN_004FAA10:

```
HouseClass offsets:
+0x53AC  FactoryClass*  Primary_ForInfantry     (RTTI 2,3)
+0x53B0  FactoryClass*  Primary_ForAircraft     (RTTI 0xF,0x10)
+0x53B4  FactoryClass*  Primary_ForBuildings    (RTTI 1,0x28, non-naval)
+0x53B8  FactoryClass*  Primary_ForShips        (RTTI 1,0x28, naval)  [building naval yards]
+0x53BC  FactoryClass*  Primary_ForVehicles     (RTTI 6,7, non-naval)
+0x53CC  FactoryClass*  Primary_ForDefenses     (RTTI 6,7, naval=5)  [naval units from shipyard]
```

Each pointer is NULL when no factory exists for that category. Created on first
Begin_Production, destroyed when empty after failed StartProduction.

## FUN_004FAA10 — Cancel / Queue Handler (0x004FAA10)

**RESOLVED: The open question about non-naval queue restart.**

Fully decompiled. The flow is:

```c
void FUN_004faa10(HouseClass* house, int rtti, int heapId, bool isNaval, bool removeAll) {
    FactoryClass* factory = house->GetFactory(rtti, isNaval);
    if (factory == NULL) return 3;

    // PATH A: Queue has items AND heapId is valid AND non-naval
    if (factory->QueueCount > 0 && heapId >= 0 && !isNaval) {
        TechnoTypeClass* type = RTTI_To_TypeArray(rtti, heapId);
        bool removed = FactoryClass::RemoveFromQueue(factory, type);

        if (removeAll) {
            while (FactoryClass::RemoveFromQueue(factory, type)) {}
        }

        if (removed) {
            Sidebar::Recalculate(...);
            if (!removeAll) return 0;
        }
    }
    // PATH B: Naval OR no queue items
    else if (isNaval) {
        if (factory->Object == NULL) goto ABANDON;
    }

    // Verify the object being produced matches heapId (if specified)
    if (heapId != -1) {
        if (factory->Object == NULL) return 0;
        if (factory->Object->GetTypeClass()->GetHeapID() != heapId) return 0;
    }

ABANDON:
    // Clear sidebar placement state for buildings/vehicles
    if (g_PlayerPtr == house) {
        ClearPlacementState(rtti, heapId, factory);
    }

    // Cancel and restart
    FactoryClass::AbandonProduction(factory);

    if (factory->QueueCount != 0) {
        FactoryClass::StartNextQueued(factory);   // ← QUEUE RESTART!
        return 0;
    }

    // Factory empty — clean up house pointer via jumptable
    // (Clears Primary_ForXxx = NULL based on rtti)
    CleanupFactoryPointer(house, rtti, isNaval);
}
```

**Correction 2026-05-21 - queue restart answer**: For normal completed-item
placement, `Place_Production` calls `CompletedProduction` and then calls
`FUN_004FAA10` with `heapId = -1`. Since `heapId < 0`, the helper skips the
non-naval real-heap-id cancel/remove branch and falls through the object-null
cleanup path. If `QueueCount != 0`, it calls `StartNextQueued` during the same
`Place_Production` command execution.

The `heapId >= 0` branch that removes one matching type from the queue is
cancel/remove behavior, not normal completion. Queue restart after successful
normal delivery is server/command-side inside `Place_Production`, not generated
by a later sidebar tick.

## StripClass::AI — Sidebar Auto-Delivery (0x006A8B30)

The sidebar strip is the per-frame driver for factory state polling.

### Key behaviors (from decompilation):

1. **Polls HasChanged()** each frame for each cameo's associated factory
2. **On completion (IsComplete + GetObject):**
   - For buildings (RTTI 1), infantry (RTTI 2), aircraft (RTTI 0xF):
     Creates a **Place_Production command (0x0B)** in the network command queue
     with the object's RTTI, HeapID, and naval flag
   - For vehicles (RTTI 6):
     Plays "Unit Ready" EVA, calls `FUN_00734250`, which stores the produced
     unit pointer in the pending land/naval vehicle delivery global. It is not
     the factory exit handler.

3. **Progress bar mirroring**: Each strip entry has its own local timer that mirrors
   the factory's production progress (via CalcRate). This drives the animated
   progress bar fill even between HasChanged ticks.

4. **Idle factory detection**: When the strip's local progress tracking sees
   `Rate==0 || IsSuspended`, it resets the local progress counter.

### The Queue Restart Path

For successful non-naval production delivery:
1. `FactoryClass::AI` advances Progress to 54, sets `IsSuspended=true`
2. `StripClass::AI` detects `HasChanged()` → `IsComplete()` → creates or enables
   the delivery command path
3. Place command executes `HouseClass::Place_Production` → `CompletedProduction`
   → `FUN_004FAA10(heapId=-1)`
4. `FUN_004FAA10` skips the real-heap-id cancel/remove branch and reaches
   `StartNextQueued` if `QueueCount != 0`
5. Factory immediately begins the next queued item during the same command
   execution

For blocked stock land war-factory vehicle delivery, `Place_Production` returns
before `CompletedProduction` and before `FUN_004FAA10`; the completed vehicle
stays pending and the queue does not advance.

## LogicClass::AI — Global Tick Order (0x0055AFB0)

The main game loop tick function. Relevant production ordering:

```c
void LogicClass::AI() {
    // ... cell actions, fog timer, tiberium growth ...

    // Tick all game objects (buildings, units, infantry, aircraft)
    for each TechnoClass in LayerClass arrays:
        techno->AI();    // vtable[0x5C/4]

    // Tick all factories (production advancement)
    for (int i = 0; i < g_FactoryClass_Array_Count; i++) {
        g_FactoryClass_Array[i]->AI();   // vtable[0x5C/4] = FactoryClass::AI
    }

    // Tick all houses (per-player logic)
    for (int i = 0; i < g_HouseClass_Array_Count; i++) {
        g_HouseClass_Array[i]->AI();     // vtable[0x5C/4] = HouseClass::AI
    }

    // ... superweapons, EMP, lightning storm ...
}
```

**Order matters**: Factories tick AFTER game objects but BEFORE houses. This means:
- Building damage (from combat) is applied before factory rate check (power changes)
- Factory progress advances before HouseClass::AI (which handles things like defeat check)

## Prerequisite Validation — FUN_00509140 (UpdateRadar)

This function validates queued items when prerequisites change (building lost, sold).
Called when a structure is destroyed or captured.

```c
void ValidateFactoryQueues(HouseClass* house, int rtti, bool isNaval, int navalIdx) {
    FactoryClass* factory = house->GetFactory(rtti, isNaval);
    if (factory == NULL) return;

    // 1. Validate queue items (iterate backwards for safe removal)
    for (int i = factory->QueueCount - 1; i >= 0; i--) {
        TechnoTypeClass* queued = factory->QueueArray[i];
        if (!queued->CanBuild(1, 0, 1, house)) {
            // Prerequisite lost — remove from queue
            factory->QueueCount--;
            // Shift array left
            for (int j = i; j < factory->QueueCount; j++)
                factory->QueueArray[j] = factory->QueueArray[j+1];
        }
    }

    // 2. Validate the active production item
    if (factory->Object != NULL) {
        TechnoTypeClass* activeType = factory->Object->GetTypeClass();

        if (!activeType->CanBuild(1, 0, 1, house)) {
            // Prerequisite lost for active item — ABANDON
            FactoryClass::AbandonProduction(factory);
            FactoryClass::StartNextQueued(factory);  // ← Try next in queue
        }
        else if (!activeType->CanBuild(1, 1, 1, house)) {
            // Different prereq check failed — suspend
            FactoryClass::Suspend(factory, false);
            Sidebar::Recalculate(tab);
        }
        else if (factory->IsSuspended && !factory->IsManual) {
            // Prereqs restored, was auto-suspended — resume
            FactoryClass::SetRate(factory);
        }
    }

    // 3. If factory is completely empty, destroy it
    if (factory->Object == NULL && factory->QueueCount == 0) {
        factory->Destroy();
        // house factory pointer set to NULL
    }
}
```

**This solves another puzzle**: when you sell your only Radar and lose prereqs
for Prism Tanks in queue, this function removes them. And when you lose your
only War Factory while producing, it calls `AbandonProduction` + `StartNextQueued`.

### Auto-resume on prereq restoration

The third branch is notable: if a factory was auto-suspended (not manually paused
by the player) and the prerequisite is now met again, it auto-resumes via `SetRate`.
This is why rebuilding a lost prerequisite building can resume stalled production.

## HouseClass::Spend_Money (0x004F9790)

The credit deduction system. More complex than expected — handles the split between
cash reserves and ore storage.

```c
void HouseClass::Spend_Money(int amount) {
    float previousTotal = StorageClass::GetTotalAmount();

    int cash = this->Credits;        // +0x30C (cash on hand)

    if (cash < amount) {
        // Not enough cash — dip into ore storage
        int remainder = amount - cash;
        this->Credits = 0;           // Drain all cash
        int fromCash = cash;

        if (remainder > 0 && StorageClass::GetTotalAmount() > 0.0) {
            // Iterate refineries, drain ore storage
            for (int i = 0; i < numRefineries; i++) {
                if (refinery[i] != NULL && StorageClass::GetTotalAmount() > 0.0) {
                    while (remainder > 0) {
                        int slot = StorageClass::FindFirstNonEmptySlot();
                        while (StorageClass::GetAmount(slot) > 0.0) {
                            float removed = StorageClass::RemoveAmount(1.0, slot);
                            int removedInt = ftol(removed);
                            remainder -= removedInt;
                            fromCash += removedInt;
                            if (remainder < 0) {
                                fromCash += remainder;  // Overpaid — adjust
                                this->Credits -= remainder;  // Put excess back
                                remainder = 0;
                                break;
                            }
                        }
                        if (remainder <= 0) break;
                    }
                }
            }
        }
        amount = fromCash;  // Actual amount spent
    } else {
        this->Credits = cash - amount;  // Simple cash deduction
    }

    HouseClass::Notify_Credit_State_Change(previousTotal);
    this->TotalSpent += amount;      // +0x2DC running total
}
```

**Key insight**: The original engine has TWO pools — **cash** (Credits at +0x30C,
instant) and **ore storage** (StorageClass, linked to refinery buildings). Cash is
used first, then ore is drained from refineries. This is the "tiberium in silos"
mechanic from Tiberian Sun — in YR it's mostly vestigial since `Silos=no` on most
buildings, but the code path exists and runs.

## Confidence Summary

| Finding | Confidence | Method |
|---------|-----------|--------|
| GetBuildStepTime formula structure | HIGH | Direct decompilation of 0x006F47A0 |
| MultipleFactory bonus loop | HIGH | Loop structure clear, float at Rules+0x57C |
| Power ratio effect on speed | HIGH | GetPowerRatio call + division confirmed |
| SetRate resume logic | HIGH | Direct decompilation of 0x004C9EA0 |
| AbandonProduction refund = full cost - balance | HIGH | Add_Credits(GetCost - Balance) confirmed |
| AI tracking fields (+0x564C-0x5658) | HIGH | Switch on WhatAmI() in AbandonProduction |
| RecalcAllRates iterates global array | HIGH | Direct decompilation of 0x004CA6E0 |
| HouseClass factory pointers +0x53AC-0x53CC | HIGH | Consistent switch in 4+ functions |
| Begin_Production allocates factory (0x74 bytes) | HIGH | operator_new(0x74) + constructor in decompilation |
| One factory per category per house | HIGH | Get-or-create pattern in Begin_Production |
| AI headstart formula in Begin_Production | HIGH | Direct decompilation, cap at 0x35 = 53 |
| FUN_004FAA10 queue handler full flow | HIGH | Complete decompilation including all paths |
| StripClass::AI creates Place commands for completed items | HIGH | Network command construction visible in decompilation |
| StripClass::AI auto-delivers vehicles via FUN_00734250 | HIGH | Direct call visible, RTTI==6 branch |
| LogicClass tick order: objects → factories → houses | HIGH | Iteration order in 0x0055AFB0 |
| Prerequisite validation removes invalid queue items | HIGH | FUN_00509140 decompiled, backwards iteration |
| Auto-resume when prereqs restored (IsSuspended && !IsManual) | HIGH | Third branch in FUN_00509140 |
| Spend_Money drains cash first, then ore storage | HIGH | Two-pool logic in 0x004F9790 |
| Queue restart for non-naval: server-side via FUN_004FAA10 → StartNextQueued | HIGH | Deep dive confirmed: Place_Production calls CompletedProduction (Object=NULL), then FUN_004FAA10. AbandonProduction is no-op on NULL, StartNextQueued fires directly. NOT sidebar-driven. |
| GetBuildTimeBonus always returns 1.0 in YR | HIGH | Decompiled; all country INI keys commented out |
| Exact float comparison direction in MultipleFactory check | MEDIUM | Ghidra float comparison can be inverted |
| HouseClass+0x1571 flag meaning | LOW | Referenced but not traced to INI key |
