# Build Queue System — Ghidra Research Report

## Overview

The RA2/YR build queue system centers on `FactoryClass`, which manages production of one
item at a time with a FIFO queue of pending items. Each HouseClass owns one factory per
production category. The sidebar click generates network commands that feed into the factory.

## FactoryClass Layout (0x74 = 116 bytes)

Verified from constructor at 0x004C98B0 and `operator_new(0x74)` in Begin_Production.

```
+0x00  vtable*           Main vtable (AbstractClass hierarchy, at 0x007E88D0)
+0x04  vtable*           IUnknown/IRTTIInfo vtable
+0x08  vtable*           IPersist vtable
+0x0C  vtable*           IPersistStream vtable
+0x10  vtable*           DynamicVectorClass vtable (for queue array memory)
+0x14  ???
+0x18  ???
+0x1C  ???
+0x20  ???
+0x24  int Progress      Stage counter: 0 → 54 (0x36). Production complete at 54.
+0x28  bool HasTicked    Set true when progress advances this frame
+0x2C  int TimerStart    CDTimerClass[0]: start frame (-1 = inactive)
+0x30  int TimerUnk      CDTimerClass[1]: unused/padding
+0x34  int TimerDuration CDTimerClass[2]: countdown duration (reset to Rate each step)
+0x38  int Rate          Frames per production step = GetBuildStepTime / 54, clamped [1, 255]
+0x3C  int StepIncrement Always 1 (initialized to 1 in constructor, never modified)
+0x40  vtable*           DynamicVectorClass vtable (for queue memory mgmt)
+0x44  ptr QueueArray    Pointer to array of TechnoTypeClass* (queued items)
+0x48  int QueueCapacity Current allocated capacity of queue array
+0x4C  byte ???          Some flag (initialized to 1)
+0x4D  bool IsDynamic    Whether queue array is heap-allocated
+0x50  int QueueCount    Number of items currently in the queue
+0x54  int GrowthIncr    Queue array growth increment (initialized to 10)
+0x58  ptr Object        TechnoClass* currently being produced (NULL if none)
+0x5C  bool NoFunds      True if production stalled due to insufficient funds
+0x5D  bool IsDifferent  "Changed" flag — set on state change, read+reset by sidebar
+0x60  int Balance       Remaining cost to deduct (decreases as production advances)
+0x64  int OrigBalance   Original cost at production start (from debug dump string)
+0x68  int SpecialItem   Building type heap index (-1 = none, for AI building queue)
+0x6C  ptr Owner         HouseClass* — set from Object->Owner (+0x21C) in StartProduction
+0x70  bool IsSuspended  True = production paused or completed (timer not running)
+0x71  bool IsManual     Suspension reason: true for player/manual suspension,
                        false for automatic prerequisite-loss suspension
```

Field names at +0x5D ("IsDifferent"), +0x60 ("Balance"), +0x64 ("OriginalBalance"),
+0x68 ("SpecialItem"), +0x70 ("IsSuspended") are confirmed from the debug dump function
at 0x004CA430 which prints them by name (format strings in .rdata).

## HouseClass Factory Pointers

Each HouseClass stores one FactoryClass* per production category. Verified from the
switch statements in Begin_Production, Place_Production, and the cancel handler:

```
+0x53AC  FactoryClass*  InfantryFactory    (RTTI 2,3 = InfantryClass/InfantryTypeClass)
+0x53B0  FactoryClass*  AircraftFactory    (RTTI 0xF,0x10 = AircraftClass/AircraftTypeClass)
+0x53B4  FactoryClass*  BuildingFactory    (RTTI 1,0x28 = BuildingClass, non-naval)
+0x53B8  FactoryClass*  NavalBuildFactory  (RTTI 1,0x28 = BuildingClass, naval)
+0x53BC  FactoryClass*  VehicleFactory     (RTTI 6,7 = UnitClass/UnitTypeClass, non-naval)
+0x53CC  FactoryClass*  NavalFactory       (RTTI 6,7 = UnitClass/UnitTypeClass, naval=5)
```

## Core Data Flow

### Queue Structure

The queue is a simple dynamic array of `TechnoTypeClass*` pointers:
- `QueueArray[0]` = next item to produce (FIFO front)
- `QueueArray[QueueCount-1]` = most recently queued item
- Items of DIFFERENT types can be interleaved (e.g., queue Conscript, Conscript, GI, GI)
- Each individual unit is one entry — queuing 5 Conscripts = 5 separate TechnoTypeClass* pointers

### MaximumQueuedObjects

From `rules(md).ini`: `MaximumQueuedObjects=29`
Stored at `RulesClass + 0xF0` (verified: INI string at 0x0083B454, parsed in
RulesClass::ReadGeneral at 0x00671D9F, reads/writes `[esi+0xF0]`).

Checked in `FactoryClass::StartProduction` at address 0x004C9C70:
```c
if (g_RulesClass_Instance->MaximumQueuedObjects <= factory->QueueCount ||
    house->CheckBuildLimit(type) != 0) {
    // Play error sound, reject queue
    return false;
}
```

## Network Command Types (verified from command handler at 0x004C6CB0)

| ID   | Name              | Handler |
|------|-------------------|---------|
| 0x0B | Place_Production  | `HouseClass::Place_Production` — place building or exit unit |
| 0x0E | Begin_Production  | `HouseClass::Begin_Production` — start producing or queue |
| 0x0F | Suspend           | `FUN_004fa910` — calls `FactoryClass::Suspend(isManual=true)` |
| 0x10 | Cancel (single)   | `FUN_004faa10` with removeAll=0 — cancel one matching queued or active item |
| 0x2E | Cancel (all)      | `FUN_004faa10` with removeAll=1 — remove all queued matches, then cancel a matching active item |

## Production Lifecycle

### 1. Sidebar Click → Network Command

`SelectClass::Action` (0x006AAD00) handles cameo clicks:
- **Left-click on idle cameo**: Creates network command 0x0E (Begin Production)
- **Right-click on active production**: Creates network command 0x0F (Suspend)
- **Right-click on idle/queued production**: Creates command 0x10 (Cancel single)
- **Left-click on completed building**: Creates command 0x0B (Place) with coordinates
- **Left-click on completed unit**: StripClass::AI auto-creates command 0x0B
- All actions go through network commands for multiplayer lockstep

### 2. HouseClass::Begin_Production (0x004FA350)

Called when command 0x0E is received:

1. Gets the TechnoTypeClass for the requested item
2. Checks `CanBuild()` prerequisites
3. Gets or creates the FactoryClass for the appropriate category (allocates 0x74 bytes)
4. Calls `FactoryClass::StartProduction(technoType, house, isResume)`
5. Calls `FactoryClass::SetRate()` to calculate and start the production timer
6. For AI players: gives an initial Progress headstart (up to 53/54)

### 3. FactoryClass::StartProduction (0x004C9C70)

Two paths based on factory state:

**Path A — No active production (first item or resume):**
Entered when: RTTI == 7 (naval), OR (Rate==0 OR IsSuspended) with empty queue and
(Object==NULL OR not suspended), OR isResume flag is set.
- Sets `IsSuspended = true`, `IsDifferent = true`
- Resets timer and progress to 0
- Creates the actual TechnoClass object via `TechnoTypeClass::CreateInstance()` → stored at +0x58
- Reads cost via `TechnoTypeClass::GetCost(owner)` → stored in `Balance` (+0x60)
- Sets `Owner` (+0x6C) from `Object->Owner` (+0x21C)
- Returns true if object creation succeeded

**Path B — Production already active (queuing):**
- Checks `MaximumQueuedObjects` limit from RulesClass+0xF0
- Also calls `HouseClass::CheckBuildLimit(type)`; either a full queue or a nonzero
  build-limit result plays the human-player error sound and returns false
- Grows the queue array if needed (by `GrowthIncrement` = 10). The native dynamic-vector
  refusal/failure paths return true **without appending**; append occurs only when capacity
  is already available or growth succeeds
- Appends the TechnoTypeClass* pointer to `QueueArray[QueueCount]`
- Increments `QueueCount`
- Returns true

(corrected 2026-07-10: the prior summary made every successful return imply an append and
omitted the build-limit rejection; `decompile_function 0x004C9C70` shows the non-appending
true returns and `get_function_call_graph 0x004C9C70 depth=1 direction=callees` identifies
`HouseClass__CheckBuildLimit@0x0050B370` — INFERENCE_HARDENED.)

The key insight: **the queue stores TechnoTypeClass pointers (the type blueprint),
while the actively-producing item creates a real TechnoClass object.**

### 4. Production Rate Calculation

`FactoryClass::CalcRate` (0x004C9FB0):
```
Rate = GetBuildStepTime(Object) / 54
Rate = clamp(Rate, 1, 255)
```

`FactoryClass::GetBuildStepTime` (0x006F47A0) is a total build-time producer, not a cost
getter. Its body starts from object/type cost inputs and applies the owner's build-time
bonus, the type multiplier at TypeClass+0x608, power-ratio clamps, factory-count /
`MultipleFactory` scaling, and the applicable unit-production modifier before returning
the value divided by 54 above. (corrected 2026-07-10: was `GetBuildCost`; verified via
`disassemble_function 0x006F47A0` and `get_function_call_graph 0x004C9FB0 depth=2
direction=callees` — RTTI_LABEL_DRIFT.)

The rate represents **frames per production step**. Total production time ≈ `Rate × 54` frames.

### 5. AI Headstart for Computer Players

In `HouseClass::Begin_Production`, after calling SetRate, AI players get a Progress headstart:
```c
if (!wasSuspended && isMultiplayer && house->IsAI) {
    int headstart = (buildSpeed * multiplier / 60) * 54 / cost;
    if (headstart >= 53) headstart = 53; // CMP 0x35 / JL: cap at one step short of done
    factory->Progress = headstart;       // +0x24 set directly
}
```
This gives AI players an instant boost to Progress, NOT a faster step rate.
StepIncrement (+0x3C) remains 1 for all players.
(corrected 2026-07-10: was `headstart > 53`; raw `disassemble_bytes 0x004FA620`
shows `CMP EAX,0x35; JL 0x004FA687; MOV EAX,0x35` — OPERATOR_OR_ORDER_DRIFT.)

### 6. FactoryClass::AI — The Production Tick (0x004C9B20)

Called every game frame from `LogicClassPerTickUpdateLiveVector` (0x0055AFB0) via vtable[23] (offset 0x5C).
(corrected 2026-05-28: was "LogicClass::AI"; Ghidra label at 0x0055AFB0 is LogicClassPerTickUpdateLiveVector — ROOT_CAUSE: RTTI_LABEL_DRIFT; verified via get_function_by_address 0x0055AFB0)
This function iterates all FactoryClass instances in the global array at DAT_00A83E34.

```c
void FactoryClass::AI() {
    if (IsSuspended) return;               // Paused or completed
    // When Object==NULL and SpecialItem==0 (heap index 0 = no special item), bail.
    // Note: SpecialItem==-1 is the "none/completed" sentinel; SpecialItem==0 is the
    // check in the binary (corrected 2026-05-28: prior comment said "Nothing to produce"
    // implying -1 check; binary shows ==0; ROOT_CAUSE: INFERENCE_HARDENED; verified via decompile_function 0x004C9B20).
    if (Object == NULL) {
        if (SpecialItem == 0) return;
    } else if (Progress == 54) {
        return;
    }
    if (SpecialItem != -1 && Progress == 54) return;

    int timeRemaining = CDTimer.GetTimeRemaining();  // FUN_00426630 at +0x2C
    // GetTimeRemaining consumes Timer.Duration (+0x34), then the separate guard checks Rate (+0x38).
    // (corrected 2026-07-10: the 2026-05-28 correction moved this guard to +0x34, but raw
    //  bytes show MOV EAX,[ESI+0x38]; TEST EAX,EAX after the timer call; verified via
    //  disassemble_function 0x004C9B20 and read_memory 0x004C9B63 length=24
    //  — GHIDRA_STRUCT_FIELD_LABEL_DRIFT)
    if (timeRemaining != 0 || Rate == 0) {
        HasTicked = false;
        return;  // Timer not expired yet, or timer duration not set
    }

    // === ADVANCE PRODUCTION ===
    Progress += StepIncrement;     // Always +1
    HasTicked = true;

    // Reset timer for next step
    Timer.Start = g_CurrentFrameCounter;   // +0x2C
    Timer.Duration = Rate;                 // +0x34 = +0x38
    IsDifferent = true;                    // +0x5D — signal sidebar

    // Calculate per-step cost deduction
    // Note: recalculated each step as remaining_balance / remaining_steps
    int stepsLeft = 54 - Progress;
    int costThisStep;
    if (Object == NULL) {
        costThisStep = 0;
    } else if (stepsLeft == 0) {
        costThisStep = Balance;    // Final step: pay all remaining
    } else {
        costThisStep = Balance / stepsLeft;  // Integer division
    }
    costThisStep = min(costThisStep, Balance);

    // Check if house can afford this step
    // Owner (+0x6C) → sub-object at +0x24 → vtable[6] = GetAvailableCredits
    int available = Owner->GetAvailableCredits();
    if (available < costThisStep) {
        // INSUFFICIENT FUNDS — roll back progress by 1
        NoFunds = true;           // +0x5C
        Progress -= 1;            // Stall: net change = 0
    } else {
        // Deduct credits via HouseClass::SpendMoney (0x004F9790)
        Owner->SpendMoney(costThisStep);
        NoFunds = false;
        Balance -= costThisStep;
    }

    // Check completion
    if (Progress == 54) {
        IsSuspended = true;        // Mark as done, stop AI processing
        Rate = 0;                  // +0x38 = 0
        Timer.Start = g_CurrentFrameCounter;
        Timer.Duration = 0;
        Owner->SpendMoney(Balance); // Pay any integer-rounding remainder
        Balance = 0;
    }
}
```

The special-item completion gates above are not equivalent to an unconditional
`Progress == 54` return: object-backed production returns immediately at 54, while the
object-null path distinguishes `SpecialItem == 0`, `-1`, and other nonzero values.
(corrected 2026-07-10: the prior pseudocode collapsed those branches; verified via
`disassemble_function 0x004C9B20` at 0x004C9B33-0x004C9B63 — OPERATOR_OR_ORDER_DRIFT.)

**Key behaviors verified from binary:**
- Cost is deducted incrementally: each step pays `remaining_balance / remaining_steps`
- If player can't afford a step, progress rolls back by exactly 1 (net 0 with StepIncrement=1)
- Production completes at Progress == 54 (0x36)
- When complete, `IsSuspended = true` and `Rate = 0` stop the AI from further processing
- `IsDifferent` flag tells the sidebar to redraw the cameo progress bar

### 7. Completion → Delivery → Queue Advance

After production completes, the item must be placed/delivered:

**For units (Infantry/Aircraft):**
`StripClass::AI` (0x006A8B30) calls `HasChanged()` and `IsComplete()` each frame.
When complete, auto-creates network command 0x0B (Place_Production) with auto-calculated
exit coordinates, triggering the unit to exit the factory building.

**For buildings:**
`StripClass::AI` detects completion. Player clicks to place.
Creates network command 0x0B with the chosen coordinates.

**For vehicles (RTTI 6):**
`StripClass::AI` does not call the factory exit function directly. It plays the
ready EVA and calls `FUN_00734250`, which only stores the produced unit pointer in
the pending delivery global: `DAT_00B0FE5C` for non-naval vehicles or
`DAT_00B0FE60` for naval vehicles. Later command/UI action readers consume those
globals and route to placement/delivery.

All paths eventually call `HouseClass::Place_Production` (0x004FB0E0), which:
1. Gets the factory for the production category
2. Checks `IsComplete()` — must be true
3. Gets the Object via `GetObject()`
4. Attempts to exit/place the object at the target cell
5. On accepted success: calls `CompletedProduction` + queue handler
6. On blocked vehicle exit: returns failure without `CompletedProduction`, without
   `FUN_004FAA10`, and without starting the next queued item

#### FactoryClass::CompletedProduction (0x004CA1A0)
```c
if (Object != NULL && Progress == 54) {
    Object = NULL;          // Clear produced item
    IsSuspended = true;
    IsDifferent = true;
    Progress = 0;           // Reset for next item
    Timer.Reset();          // Start = currentFrame, Duration = 0
    return true;
}
// Also handles SpecialItem path similarly
```

#### Queue Handler — FUN_004FAA10

> **Correction 2026-05-21 - normal completion restart**
>
> Normal completed-item placement commands use `heapId = -1`. After successful
> delivery, `Place_Production` calls `CompletedProduction`, then `FUN_004FAA10`.
> Because `heapId < 0`, the helper skips the non-naval cancel/remove-one branch
> and reaches `StartNextQueued` in the same command execution if queued items
> remain. Queue restart after successful delivery is not sidebar-driven.
>
> The `heapId >= 0` remove-one branch described below is cancel/remove behavior,
> not normal completed-item delivery.
>
> A blocked stock land war-factory vehicle exit is a separate boundary: if
> `ExitObject` fails for produced `WhatAmI() == 6`, `Place_Production` returns
> before `CompletedProduction` and before `FUN_004FAA10`. The completed vehicle
> remains pending and the queue does not advance.

This function serves dual purpose: it's called after successful placement
(from Place_Production, 3 call sites) and as the Cancel handler (from network
commands 0x10/0x2E via the command handler at 0x004C6CB0). Also called from
BuildingClass destruction (0x0044EBF0, 5 call sites) to cancel production
when prerequisites are lost.

**Behavior depends on naval index (param_2 after RTTI-based override):**

For all non-vehicle types (infantry, buildings, aircraft): `param_2 = 0`
For non-naval vehicles: `param_2 = 0`
For naval vehicles: `param_2 = TechnoTypeClass->NavalIndex = 5`

**When `heapId >= 0`, param_2 == 0 (cancel/remove for non-naval types), and queue has items:**
1. Looks up TechnoTypeClass* via `FUN_0048dcd0(RTTI, heapID)` from global type arrays
2. Calls `RemoveFromQueue(type)` — finds and removes ONE matching pointer from queue
3. If `removeAll` is false and one queued match was removed: refreshes the sidebar and
   **returns early**, before the `StartNextQueued` call
4. If `removeAll` is true: keeps removing queued matches until none remain, refreshes the
   sidebar, and then continues to the active-item heap-ID check; a matching active item is
   abandoned and the next surviving queue entry can start

(corrected 2026-07-10: the prior steps applied the cancel-one early return to cancel-all;
`decompile_function 0x004FAA10` shows `param_5 == 0` is required for that return, while
the true branch loops and falls through — OPERATOR_OR_ORDER_DRIFT.)

**When param_2 != 0 (naval types):**
- Checks `Object == NULL` → falls through to LAB_004FAB64
- Calls `AbandonProduction()`
- Checks `QueueCount != 0` → `JNZ 0x4fac94` → `StartNextQueued()` (verified from raw
  assembly: MOV ECX,ESI + CALL 0x4ca5a0 at addresses 0x4fac94-0x4fac9a)

**When queue is empty AND param_3 != -1:**
- Checks `Object == NULL` → returns 0 (also jumps to 0x4FAC9B epilogue)

**Correction 2026-05-21:** The early-return statement applies to the
real-heap-id cancel/remove branch above. Normal completion passes `heapId = -1`,
skips that branch, and can reach the `AbandonProduction`/object-null no-op path
that then calls `StartNextQueued` if the queue is non-empty.

**SUPERSEDED 2026-05-21 — Non-naval queue restart:**
The paragraph below is retained as investigation history only. It incorrectly
treated normal completion as a real-heap-id cancel/remove call; verified normal
completion uses `heapId = -1` and restarts in `FUN_004FAA10` during the same
`Place_Production` command after successful delivery.
Superseded conclusion: no extra per-frame/sidebar restart mechanism is needed for
normal delivery. The only remaining caution is to keep cancel/remove calls with
real heap ids separate from normal `heapId = -1` completion.

#### FactoryClass::StartNextQueued (0x004CA5A0)
```c
if (QueueCount != 0 && Object == NULL && (Rate == 0 || IsSuspended)) {
    TechnoTypeClass* next = QueueArray[0];  // Take from front (FIFO)

    // Remove from queue (shift array left by one)
    QueueCount--;
    for (int i = 0; i < QueueCount; i++)
        QueueArray[i] = QueueArray[i+1];

    // Get type info from the dequeued TechnoTypeClass
    int heapId = next->GetHeapID();      // vtable[0x40/4]
    bool isNaval = next->IsNaval;        // byte at TechnoTypeClass+0xCCE
    int rtti = next->WhatAmI();          // vtable[0x2C/4]

    HouseClass::Begin_Production(rtti, heapId, isNaval, /*isResume=*/true);
}
```

The guard's rate test is byte-exact: `MOV EAX,[ESI+0x38]` at 0x004CA5B2, followed by
the `IsSuspended` fallback. There are two direct active-YR callers: `FUN_004FAA10` at
0x004FAC96 (cancel/completion queue handler) and `UpdateRadar` at 0x00509223 (after
automatic abandonment during prerequisite revalidation). (added 2026-07-10: verified
via `disassemble_function 0x004CA5A0` and `get_xrefs_to 0x004CA5A0` — STALE_OMISSION.)

## Queue Management Functions

| Address    | Name                | Description |
|------------|---------------------|-------------|
| 0x004C9B20 | AI                  | Per-frame production tick (vtable[23]) |
| 0x004C9C60 | HasChanged          | Read+reset IsDifferent flag (+0x5D) |
| 0x004C9C70 | StartProduction     | Start item or append to queue |
| 0x004C9E60 | Suspend             | Store manual flag at +0x71, set +0x70, clear +0x38 rate and +0x34 timer duration; preserve +0x24 progress |
| 0x004C9EA0 | SetRate/Resume      | Clear +0x70, calculate +0x38 rate, seed +0x34 timer duration, then test affordability |
| 0x004C9FB0 | CalcRate            | Calculate rate: GetBuildStepTime/54, clamped [1,255] |
| 0x004C9FF0 | AbandonProduction   | Cancel current production, refund |
| 0x004CA120 | GetProgress         | Returns Progress (+0x24), range 0-54 |
| 0x004CA130 | IsComplete          | Returns Progress==54 with Object!=NULL or SpecialItem!=-1 |
| 0x004CA160 | GetObject           | Returns Object ptr (+0x58) |
| 0x004CA1A0 | CompletedProduction | Clear object, reset progress to 0 |
| 0x004CA5A0 | StartNextQueued     | Pop front of queue, call Begin_Production |
| 0x004CA620 | RemoveFromQueue     | Find and remove one instance of a type |
| 0x004CA6B0 | IsInQueue           | Check if a TechnoTypeClass is in queue |
| 0x004CA6E0 | RecalcAllRates      | Update rate for all factories of a house |

## CDTimerClass (embedded at FactoryClass +0x2C, 12 bytes)

Verified from `FUN_00426630` (GetTimeRemaining):

```c
struct CDTimerClass {       // 12 bytes
    int StartFrame;   // +0x00 (factory +0x2C): frame when timer started (-1 = inactive)
    int Unknown;      // +0x04 (factory +0x30): unused in GetTimeRemaining
    int Duration;     // +0x08 (factory +0x34): countdown duration in frames
};

// FUN_00426630: __fastcall, this = CDTimerClass*
int GetTimeRemaining() {
    int remaining = this->Duration;
    if (this->StartFrame != -1) {
        int elapsed = g_CurrentFrameCounter - this->StartFrame;
        if (elapsed < remaining)
            return remaining - elapsed;
        return 0;  // Timer expired
    }
    return remaining;  // Inactive: return duration as-is
}
```

The `Rate` field at +0x38 is SEPARATE from the CDTimerClass. After each production step,
`Timer.Duration` (+0x34) is reset to `Rate` (+0x38).

`FactoryClass::Suspend(bool)` copies its argument to +0x71 before setting +0x70 and
zeroing both +0x38 and the embedded timer's +0x34; it does not write Production.Value
at +0x24. The command-0x0F path passes true, while `UpdateRadar` passes false for
automatic prerequisite-loss suspension and later resumes only when +0x71 is false.
(corrected 2026-07-10: +0x71 was documented as `CanAfford`; verified via
`disassemble_function 0x004C9E60`, `disassemble_bytes 0x004FA998`,
`disassemble_bytes 0x00509240`, and `decompile_function 0x00509140`
— INFERENCE_HARDENED.)

## Rust Implementation Handoff (audited 2026-07-10)

`src/sim/production/factory.rs` already carries distinct `step_rate_frames`,
`step_timer`, `suspended`, `manual`, and `progress` fields, which are the required Rust
surfaces for +0x38, +0x34, +0x70, +0x71, and +0x24 respectively. The remaining
player-visible mismatch is pause routing: `FactoryRegistry::toggle_pause` flips `manual`
both ways, but active-YR command 0x0F is a one-way `Suspend(true)` and a click on an
already rate-zero/suspended cameo emits cancel command 0x10 rather than an unpause.
This is **DRIFT**, not an equivalent internal representation. The Rust acceptance path
must also retain the automatic prerequisite-loss `Suspend(false)` → revalidation →
`StartNextQueued`/resume distinction and the +0x38 rate guard. (binary requirements
verified via `disassemble_function 0x004C9E60`, `decompile_function 0x00509140`,
`disassemble_function 0x004CA5A0`, and `disassemble_function 0x006AAD00`; current Rust
touchpoints read at `factory.rs` `Factory`, `set_rate`, `toggle_pause`, and
`start_next_queued` — STALE_RUST_HANDOFF.)

## Sidebar Display Integration

`StripClass::Draw` (0x006A9540) calls `GetProgress()` three times to draw the
cameo progress bar. The progress value 0-54 is mapped to the cameo height for
the green fill overlay.

`StripClass::AI` (0x006A8B30) calls `HasChanged()` each frame per strip entry.
If true, checks `IsComplete()` and handles auto-delivery for units. Also maintains
a local progress bar animation timer that mirrors the factory's production state.

## Queue Example: "10 Conscript then 5 GI"

1. Click Conscript → `Begin_Production` creates factory, starts producing Conscript #1
   - Object = new InfantryClass("CONS"), Progress = 0, queue empty
2. Click Conscript 9 more times → each calls StartProduction Path B
   - QueueArray = [CONS, CONS, CONS, CONS, CONS, CONS, CONS, CONS, CONS]
   - QueueCount = 9
3. Click GI 5 times → appends after conscripts
   - QueueArray = [CONS×9, GI×5]
   - QueueCount = 14
4. Conscript #1 completes (Progress reaches 54):
   - FactoryClass::AI sets IsSuspended=true, Rate=0
   - StripClass::AI detects HasChanged+IsComplete → creates Place event (0x0B)
   - Place_Production → CompletedProduction: Object=NULL, Progress=0
   - FUN_004FAA10(heapId=-1) skips cancel/remove and reaches StartNextQueued
   - StartNextQueued pops the next CONS from the queue during the same command
   - Object = new InfantryClass("CONS") for #2
5. Repeat for each conscript...
6. After all 10 conscripts, the first GI starts automatically from queue front

## Right-Click Behavior

From `SelectClass::Action` — right-click on a cameo:

- **If production is active** (Rate != 0 && !IsSuspended):
  Creates command 0x0F → `FactoryClass::Suspend(isManual=true)` — pauses production

- **If production is idle/suspended** (Rate == 0 || IsSuspended):
  Sends command 0x10. The handler removes one queued match first when present; otherwise
  it can abandon the matching active item and start the next queued item.
  The command-0x10 construction branch does not also emit a Begin_Production 0x0E event.

(corrected 2026-07-10: the previous right-click summary invented an additional 0x0E
event and misnamed the Suspend argument; `disassemble_function 0x006AAD00` confirms the
entry, while `disassemble_bytes 0x006AAF90` shows the 0x10 event branch and
`disassemble_bytes 0x006AAFE0`/`0x006AB0F0` show the distinct 0x0F branches
— INFERENCE_HARDENED.)

## Key Rules.ini Settings

- `MaximumQueuedObjects=29` — max items in the queue array (RulesClass+0xF0)
- `MultipleFactory` — speed bonus modifier (affects GetBuildStepTime calculation)
- Build speed / AI difficulty — affects Rate calculation and AI Progress headstart

## Confidence Levels

- **HIGH** (verified from multiple decompiled functions, debug strings, AND raw assembly):
  FactoryClass layout and size (0x74 bytes), all named fields (+0x24 Progress, +0x50
  QueueCount, +0x58 Object, +0x5D IsDifferent, +0x60 Balance, +0x68 SpecialItem,
  +0x70 IsSuspended — names from debug dump format strings at 0x004CA430).
  Queue stores TechnoTypeClass* pointers (verified from both StartProduction append
  and StartNextQueued dequeue). AI tick at vtable[23], progress 0-54 with cost per
  step. CDTimerClass layout (12 bytes, GetTimeRemaining at 0x00426630 reads [0] and
  [2]). MaximumQueuedObjects at RulesClass+0xF0 (traced from INI parser to
  StartProduction check). Network command IDs (0x0B/0x0E/0x0F/0x10/0x2E — from
  command handler switch at 0x004C6CB0). HouseClass factory offsets (+0x53AC through
  +0x53CC — from consistent switch statements in 6+ functions). StepIncrement (+0x3C)
  always 1 (constructor sets to 1, no write sites found). AI headstart modifies
  Progress directly, not StepIncrement.

- **HIGH** (corrected 2026-05-21):
  Normal successful completion passes `heapId = -1` into `FUN_004FAA10`, skips the
  real-heap-id cancel/remove branch, and reaches `StartNextQueued` if queued items
  remain. The early return behavior applies to cancel/remove calls with a real
  heap id, not normal completed-item placement.

- **MEDIUM** (verified from one source or partially traced):
  +0x64 OrigBalance (debug dump string but write site not traced).
  +0x6C Owner = HouseClass* (set from Object+0x21C, confirmed by debug dump "House->
  Fetch_ID" format string). +0x4C flag, +0x14-0x20 field purposes unknown.

- **RESOLVED 2026-05-21 - Queue restart mechanism for non-naval types:**
  Successful normal delivery calls `CompletedProduction` and then `FUN_004FAA10`
  with `heapId = -1`; `StartNextQueued` is reached from `FUN_004FAA10` in the same
  `Place_Production` command execution. The prior open question confused normal
  completion with the real-heap-id cancel/remove branch.
