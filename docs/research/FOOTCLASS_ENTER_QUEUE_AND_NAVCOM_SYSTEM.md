# FootClass Enter Queue & NavCom System — Ghidra Deep Dive

## Overview

FootClass manages two critical queue/navigation systems for mobile units:
1. **NavCom queue** (DynamicVectorClass at 0x588) — queued movement destinations
2. **Enter queue** (DynamicVectorClass at 0x5AC) — queued enter-transport/building targets
3. **NavCom fields** (0x5A0-0x5A8) — current, auxiliary, and suspended navigation targets
4. **TarCom block** (0x5C4-0x5D1) — arrival target handler state

These systems coordinate the lifecycle of a unit moving to and entering a transport,
building (garrison), or refinery.

**2026-05-27 correction note:** Newer NavCom reports supersede several statements below. `NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` found no standard YR runtime player, TeamClass/AI, or trigger waypoint producer for Foot NavQueue; only save-load reconstruction positively populates entries. `NAVCOM_ONARRIVAL_TAIL_HOOKS_GHIDRA_REPORT.md` verifies `+0x687` is a deferred vtable `+0x174` hook, resolving to Scatter for stock Unit/Infantry, not an EVA/audio callback. `NAVCOM_POINTEREXPIRED_RETENTION_BRANCHES_GHIDRA_REPORT.md` verifies NavCom is not always cleared on pointer expiry.

## Confidence Level

**HIGH** — All offsets verified from FootClass::Constructor (0x4D31E0), FootClass::Load
(0x4DB3C0), FootClass::Save (0x4DB690), FootClass::ComputeChecksum (0x4DBAD0), and
multiple cross-referencing functions. param_1 type is `int*` in Constructor and most
functions, so `param_1[0x169]` = byte offset 0x5A4.

---

## FootClass Field Layout (Navigation Section)

### NavCom Queue — DynamicVectorClass<AbstractClass*> at +0x588

```
+0x588  VTable*           DVec vtable (PTR_FUN_007e91ec)
+0x58C  AbstractClass**   Buffer (heap-allocated array of pointers)
+0x590  int               Count (number of active entries — idx [0x164])
+0x594  ???               (alignment/padding — idx [0x165])
+0x595  bool              IsAllocated flag
+0x598  int               Count2 (active count for pop — idx [0x166])
+0x59C  int               Capacity (initialized to 10 — idx [0x167])
```

This queue is serialized, deserialized, consumed, and cleaned. When a nonzero queue
exists, `OnArrival` pops the first entry to set the next destination. However,
`NAVCOM_NAVQUEUE_PUSH_PRODUCERS_GHIDRA_REPORT.md` found no standard YR player,
TeamClass/AI, or trigger waypoint path that appends to this queue at runtime; treat
nonzero entries as save/load or legacy/unknown state until a producer is separately
verified.

### NavCom Navigation Fields

```
+0x5A0  AbstractClass*    NavCom_Aux (idx [0x168])
        Auxiliary/scratch NavCom pointer. Cleared in Set_Destination_Internal and
        FootClass__Stop_Moving. Purpose: temporary destination tracking, cleared
        at the start of each new navigation assignment.

+0x5A4  AbstractClass*    NavCom (idx [0x169])
        The CURRENT navigation target. This is the primary "where am I going"
        pointer. Set by Set_Destination_Internal (via TechnoClass::Set_Destination).
        Checked extensively in Mission_Enter, Mission_Move, AI, etc.

+0x5A8  AbstractClass*    SuspendedNavCom (idx [0x16A])
        Saved NavCom when a higher-priority mission temporarily overrides navigation.
        Set in Set_NavCom_With_Suspend (0x4D8F40):
            this[0x16A] = this[0x169];  // save current NavCom
            Override_Mission(params);    // set new mission
            Set_Destination(params);     // navigate to new target
        Cleared in PointerExpired when the suspended target is destroyed.
```

### Enter Queue — DynamicVectorClass<AbstractClass*> at +0x5AC

```
+0x5AC  VTable*           DVec vtable (PTR_FUN_007e91ec)
+0x5B0  AbstractClass**   Buffer (heap-allocated array of pointers)
+0x5B4  int               Count (idx [0x16D]) — tracks buffer fill
+0x5B8  ???               (alignment/padding)
+0x5B9  bool              IsAllocated flag
+0x5BC  int               ActiveCount (idx [0x16F]) — number of queued entries
+0x5C0  int               Capacity (initialized to 10 — idx [0x170])
+0x5C3  (end of DVec)
```

This queue stores **targets to enter** — transports, garrison buildings, refineries.
Units queue up their enter-targets here before physically entering them.

### TarCom Block

```
+0x5C4  int               TarCom_Type (idx [0x171])
        Initialized to -1 (0xFFFFFFFF). Set by Assign_Target_Command to command
        type (e.g., 0x1D). Used by Arrival_Target_Handler.
        Cleared to -1 by Clear_All_TarCom.

+0x5C8  AbstractClass*    TarCom_Primary (idx [0x172])
        Primary arrival target. Set by Assign_Target_Command when cVar1 != 0
        (queued-as-cell reference). In Arrival_Target_Handler, this is checked
        first; if non-null and the unit can attack it (vtable+0x3B4 check),
        the handler returns early (target still being pursued).
        In PointerExpired: if target == expired, converts to cell coords and
        stores in TarCom_AuxCell, then clears.

+0x5CC  AbstractClass*    TarCom_Aux (idx [0x173])
        Secondary/auxiliary arrival target. Checked in Arrival_Target_Handler
        when TarCom_Primary is null. Set by Assign_Target_Command when
        bVar4 (second queued target).

+0x5D0  ???               (padding byte)

+0x5D1  bool              TarCom_Active (single byte)
        Flag indicating the arrival target system is engaged. Set to 1 in
        Arrival_Target_Handler when the unit actually starts pursuing.
        Cleared by Clear_All_TarCom and Assign_Target_Command.
        When set, Arrival_Target_Handler checks if it can attack the target
        at field +0x2B4 (base TechnoClass target).
```

---

## Key Functions

### FootClass::Enter_Destination (0x4DA0E0)

**Adds a target to the Enter Queue.** Called when a unit is told to enter a
transport/building.

```
void Enter_Destination(FootClass* this, AbstractClass* target)
{
    if (target == NULL) return;

    // Self-enter case: if target == this AND enter queue has entries,
    // set flag at +0x6B1 = 1 (signals "entering self" / unload mode)
    if (target == this && this->EnterQueue.ActiveCount > 0) {
        this->field_0x6B1 = 1;
        return;
    }

    // If queue is empty, clear the self-enter flag
    if (this->EnterQueue.ActiveCount == 0) {
        this->field_0x6B1 = 0;
    }

    // Add to enter queue if capacity allows
    int count = this->EnterQueue.Count;  // [0x16D]
    if (count < this->EnterQueue.ActiveCount ||   // [0x16F]
        ((this->EnterQueue.IsAllocated || count == 0) &&
         this->EnterQueue.Capacity > 0 &&          // [0x170]
         DVec::Grow(capacity + count)))
    {
        int idx = this->EnterQueue.ActiveCount++;  // [0x16F]++
        this->EnterQueue.Buffer[idx] = target;     // store at [0x16C] + idx*4
    }

    // If NavCom is null and this is an AircraftClass (RTTI==5),
    // and (no destination OR destination is not a building with Helipad flag),
    // then assign Enter mission
    if (this->NavCom == NULL &&
        this->GetAbstractType() == 5 &&  // AircraftClass
        (GetDestination() == NULL ||
         dest->RTTI != Building || !dest->Type->HasHelipad))
    {
        this->Assign_Destination(NULL, true);  // vtable+0x484
    }
}
```

**Key insight**: The Enter Queue is a FIFO-like buffer. Items are appended at
ActiveCount and consumed from index 0 (see OnArrival).

### FootClass::OnArrival (0x4D82B0)

**Called when a unit arrives at its NavCom destination. This is where the NavCom
queue gets popped.**

```
int OnArrival(FootClass* this, ?, bool param_3)
{
    if (this->field_0x6B3 != 0) return 0;  // already processing arrival
    this->field_0x6B3 = 1;  // mark arrival in progress

    // Handle direction normalization
    FUN_00709a40(...);

    if (this->field_0x687 != 0) {
        this->field_0x687 = 0;
        this->vtable_0x174(&DAT_008B3DA8, 1, 0);
    }

    // IPiggyback locomotor check
    ...

    // *** POP FROM NAVCOM QUEUE ***
    if (this->NavQueue.Count2 > 0) {  // [0x166] > 0
        // Set destination to first entry in queue
        Set_Destination(this->NavQueue.Buffer[0], false);  // vtable+0x480

        // Shift queue entries left (pop front)
        if (this->NavQueue.Count2 > 0) {
            int newCount = --this->NavQueue.Count2;  // [0x166]--
            for (int i = 0; i < newCount; i++) {
                this->NavQueue.Buffer[i] = this->NavQueue.Buffer[i+1];
            }
        }
        return 1;  // consumed a queued destination
    }

    // No more queued destinations — handle path following, etc.
    ...
    return 0;
}
```

**Critical flow**: NavCom queue is consumed FIFO — index 0 is popped and the rest
shifts left. This happens exactly on arrival at the current NavCom destination.

### FootClass::Set_NavCom_With_Suspend (0x4D8F40)

**Suspends current NavCom and overrides with a new mission/destination.**

```
void Set_NavCom_With_Suspend(FootClass* this, mission, p3, p4)
{
    this->SuspendedNavCom = this->NavCom;  // [0x16A] = [0x169] — save
    ArchiveTarget = Target;                // newer correction: no Target=NavCom write
    Override_Mission(mission, p3, p4);
    Set_Destination(p4, true);             // vtable+0x480
}
```

Used by AircraftClass::Set_NavCom_Override (0x41BB30) to temporarily redirect
aircraft during specific mission types (4=attack, 0x1A-0x1F=special).

### FootClass::Stop_Moving (0x4DF0D0)

**Clears both NavCom_Aux and NavCom.**

```
void Stop_Moving(FootClass* this)
{
    this->NavCom_Aux = NULL;  // +0x5A0 = 0
    this->NavCom = NULL;      // +0x5A4 = 0
}
```

Simple and direct — does NOT clear the queues, only the active navigation pointers.

### FootClass::Set_Destination_Internal (0x4D94B0)

**The low-level NavCom setter.** Called at the end of TechnoClass::Set_Destination.

```
void Set_Destination_Internal(FootClass* this, AbstractClass* target)
{
    this->NavCom_Aux = NULL;  // [0x168] = 0 — always clear aux

    // Guard conditions: don't navigate if immobilized
    if (this->field_0x6AD && target != NULL) return;  // being warped
    if (this->IsBeingControlled && target != NULL) return;
    if (this->WarpedOutOf != 0 && target != NULL) return;
    if (this->ChronoTarget != 0 && target != NULL) {
        BuildingClass::DeployUnit_ChronoWarp(true);
    }

    this->NavCom = target;  // [0x169] = target

    if (target == NULL && this->field_0x6AD) {
        // Warp-out cleanup
        ...
    }

    if (this->NavCom == NULL) {
        // Stop locomotor
        locomotor->Stop();
        this->NavCom = target;
    } else {
        // Destroy tethered object if any
        if (this->TetheredTo != NULL) {
            this->TetheredTo->Destroy();
            this->TetheredTo = NULL;
        }

        // Get locomotor CLSID for WalkLocomotion special handling
        ...

        // Tell locomotor where to go
        if (!this->field_0x6AC) {
            CoordStruct dest = target->GetDockCoords(this);
            locomotor->Head_To_Coord(dest);
        } else {
            this->field_0x6AC = false;
        }
    }

    // Reset path timers
    this->field_0x6B7 = false;
    this->PathRetryTimer = RulesClass->PathDelay;
    this->PathRetryFrame = CurrentFrame;
    this->MoveCounter = 0;
}
```

### FootClass::TryEnterTransport (0x70D7E0)

**Called every tick from FootClass::AI. Attempts to physically enter the contact
transport.**

```
int TryEnterTransport(FootClass* this)
{
    AbstractClass* contact = this->Contact;  // [0x140] = TechnoClass+0x500

    if (contact == NULL) return 0;

    if (!contact->IsAlive) {
        this->Contact = NULL;
        return 0;
    }

    if (contact != NULL && contact->IsActive) {
        // Try RADIO_CAN_ENTER (radio command 2)
        int result = this->Transmit_Radio(RADIO_CAN_ENTER, contact);

        if (result == RADIO_ROGER) {
            // Try RADIO_ENTER (radio command 0xF)
            result = this->Transmit_Radio(RADIO_ENTER, contact);

            if (result == RADIO_ROGER) {
                // SUCCESS: set mission to Enter (7), set destination to contact
                this->Set_Mission(MISSION_ENTER, true);
                this->Set_Destination(contact, true);
                this->Contact = NULL;
                return 1;
            }

            // Enter rejected — abort
            this->Set_Mission(-1, false);   // clear mission
            this->Set_Destination(NULL, true);
            this->Contact = NULL;
            this->Transmit_Radio(RADIO_OVER_AND_OUT, contact);
            return 1;
        }

        // Special case: contact is a Building and this is Infantry,
        // and building has Passengers but not InfantryOnly
        if (contact->RTTI == Building && this->RTTI == Infantry &&
            contact->Type->Passengers && !contact->Type->InfantryOnly)
        {
            this->Transmit_Radio(RADIO_QUEUE_ENTER, contact);  // 0xE
            return 0;
        }
    }
    return 0;
}
```

**Called from AI loop**: In FootClass::AI (0x4DA530), near the end:
```
if (!this->IsInAir()) {
    FootClass::TryEnterTransport();
}
```

### FootClass::Arrival_Target_Handler (0x4DF3A0)

**Manages the "attack on arrival" system.** When a unit is given a target while
en route, this handler processes it when the unit arrives.

```
void Arrival_Target_Handler(FootClass* this)
{
    if (this->TarCom_Primary == NULL) {  // +0x5C8
        // Check TarCom_Aux (+0x5CC)
        if (this->TarCom_Aux != NULL &&
            this->Can_Attack(this->TarCom_Aux))  // vtable+0x3B4
            return;  // still pursuing auxiliary target

        // Check TarCom_Active flag
        if (this->TarCom_Active &&      // +0x5D1
            this->Can_Attack(this->Target))  // check base target +0x2B4
            return;

        // Clear base target, try to enter current cell
        this->Target = NULL;  // +0x2B4
        CoordStruct pos = this->GetCoords();
        if (!this->Try_Enter_Cell(pos, true)) {
            this->Target = this->TarCom_Aux;  // +0x2B4 = +0x5CC
            return;
        }
    } else {
        // TarCom_Primary is set
        if (this->TarCom_Active) {
            if (!this->Can_Attack(this->Target))
                this->Target = NULL;
        }

        CoordStruct pos = this->GetCoords();
        if (!this->Try_Enter_Cell(pos, true))
            return;
    }

    // Cell entered successfully — begin mission
    this->Set_Mission(MISSION_GUARD, true);  // vtable+0x1E8 with arg 1
    this->TarCom_Active = true;  // +0x5D1 = 1
}
```

### FootClass::Clear_All_TarCom (0x4DF1A0)

```
void Clear_All_TarCom(FootClass* this)
{
    this->TarCom_Type = -1;      // +0x5C4 = 0xFFFFFFFF
    this->TarCom_Primary = NULL;  // +0x5C8 = 0
    this->TarCom_Aux = NULL;      // +0x5CC = 0
    this->TarCom_Active = false;  // +0x5D1 = 0
}
```

### FootClass::Assign_Target_Command (0x4DF0E0)

**Parses a target command and populates TarCom fields.**

```
int Assign_Target_Command(FootClass* this, CommandStruct* cmd)
{
    char cmdType = cmd->Type;  // +0xC

    if (cmdType != 0x1D) {
        this->Clear_TarCom();  // vtable+0x4A8
        return cmdType;
    }

    bool isQueued = cmd->IsQueued;  // +0x12
    char isAlt = cmd->AltTarget;   // +0x17

    if (!this->Can_Assign_Target()) return 2 - isQueued;

    this->TarCom_Active = false;   // +0x5D1 = 0
    this->TarCom_Type = 0x1D;     // +0x5C4 (idx [0x171])

    if (isAlt) {
        this->TarCom_AuxCell = ResolveTarget();  // +0x5C8 (idx [0x172])
        return 2;
    }

    if (isQueued) {
        this->TarCom_Aux = ResolveTarget();  // +0x5CC (idx [0x173])
        return 1;
    }

    this->Clear_TarCom();
    return 0x1D;
}
```

### FootClass::PointerExpired (0x4D9960)

**Cleans up all queues and pointers when a referenced object is destroyed.**

Key queue cleanup sections:

```
// NavCom cleanup
if (this->SuspendedNavCom == expired) this->SuspendedNavCom = NULL;  // [0x16A]
if (this->NavCom == expired) {
    // Complex logic: sensor-visible expired targets and a strict infantry
    // Occupier mission-8 branch can retain NavCom. Only the final clear path
    // zeros NavCom_Aux and NavCom.
    if (should_clear) {
        this->NavCom_Aux = NULL;  // [0x168]
        this->NavCom = NULL;      // [0x169]
    }
}

// TarCom cleanup
if (this->TarCom_Aux == expired) {
    if (expired != NULL) {
        // Convert to cell reference before clearing
        CellStruct cell = CellClass::Get_Cell_At(expired->GetCoords());
        this->TarCom_AuxCell = cell;
    }
    this->TarCom_Aux = NULL;  // [0x173]
}
if (this->TarCom_AuxCell == expired) this->TarCom_AuxCell = NULL;  // [0x172]

// ENTER QUEUE cleanup — iterates and removes matching entries with shift-left
for (int i = 0; i < this->EnterQueue.ActiveCount; i++) {
    if (this->EnterQueue.Buffer[i] == expired) {
        int newCount = --this->EnterQueue.ActiveCount;
        for (int j = i; j < newCount; j++) {
            this->EnterQueue.Buffer[j] = this->EnterQueue.Buffer[j+1];
        }
        i--;  // re-check this index since items shifted
    }
}

// NAVCOM QUEUE cleanup — same pattern
for (int i = 0; i < this->NavQueue.Count2; i++) {
    if (this->NavQueue.Buffer[i] == expired) {
        int newCount = --this->NavQueue.Count2;
        for (int j = i; j < newCount; j++) {
            this->NavQueue.Buffer[j] = this->NavQueue.Buffer[j+1];
        }
        i--;
    }
}
```

---

## State Machine Flow

### Complete Enter Transport Sequence

```
1. Player orders unit to enter transport
   -> TechnoClass::Set_Destination() called with transport as target
   -> FootClass::Enter_Destination() adds transport to Enter Queue
   -> Unit assigned MISSION_ENTER (7) if AircraftClass

2. Unit pathfinds toward transport
   -> NavCom = transport (or nearby cell)
   -> Locomotor drives movement each tick
   -> FootClass::AI calls TryEnterTransport each tick

3. Unit arrives at transport's cell
   -> OnArrival fires
   -> If NavCom queue has entries, pops next waypoint
   -> Otherwise processes arrival at destination

4. TryEnterTransport succeeds
   -> Sends RADIO_CAN_ENTER (2) to transport -> ROGER
   -> Sends RADIO_ENTER (0xF) to transport -> ROGER
   -> Sets MISSION_ENTER (7), destination = transport
   -> Clears Contact pointer

5. Mission_Enter executes (InfantryClass or UnitClass override)
   -> Checks unit is in same cell as transport
   -> Sends RADIO_ENTER (0x15) radio command
   -> On success: Limbo(), AddPassenger(), stop moving

6. For buildings (garrison):
   -> InfantryClass::Mission_Enter checks building type
   -> If garrison-capable: BuildingClass::AddGarrisonOccupant()
   -> If refinery: harvest/dock logic
   -> If spy: infiltration logic
   -> If engineer: capture logic
```

### NavCom Queue vs Enter Queue Interaction

The NavCom Queue and Enter Queue serve different purposes:

- **NavCom Queue** (0x588): Stores serialized/loaded queued navigation targets and
  supports FIFO consumption. Newer producer coverage found no standard runtime
  player, TeamClass/AI, or trigger waypoint append path, so do not model normal
  shift-click movement as a Foot NavQueue push without separate evidence.

- **Enter Queue** (0x5AC): Stores things to enter. This is used when a transport
  receives multiple load commands, or when queuing units to enter buildings.
  The Enter_Destination function specifically handles the case where target == self
  (indicating unload/deploy). The queue is **not popped by OnArrival** — it's consumed
  by the entering logic itself.

### NavCom Suspension Flow

```
1. Unit is moving to NavCom destination A
2. Higher-priority mission arrives (e.g., aircraft attack run)
3. Set_NavCom_With_Suspend called:
   - SuspendedNavCom = NavCom (saves A)
   - Override_Mission(new mission)
   - Set_Destination(new target)
4. Unit performs new mission
5. After completion, SuspendedNavCom can be restored as NavCom
   (specific restoration logic varies by mission type)
6. If suspended target is destroyed, PointerExpired clears SuspendedNavCom
```

---

## Serialization Verification

From FootClass::Load (0x4DB3C0):
1. Reads NavCom Queue count, then reads that many AbstractClass* entries into DVec at +0x588
2. Reads Enter Queue count, then reads that many AbstractClass* entries into DVec at +0x5AC
3. Loads Locomotor via OleLoadFromStream at +0x674
4. Clears NavCom_Aux (+0x5A0 = 0)
5. Resolves swizzled pointers for:
   - +0x5D4, +0x5D8 (TarCom-related pointers beyond the main block)
   - +0x5A4 (NavCom), +0x5A8 (SuspendedNavCom)
   - +0x5C8 (TarCom_Primary), +0x5CC (TarCom_Aux)
   - +0x5DC, +0x694, +0x69C (other FootClass pointers)
   - All Enter Queue entries (loops over +0x5BC count)
   - All NavCom Queue entries (loops over +0x598 count)

From FootClass::Save (0x4DB690):
1. Writes NavCom Queue count ([0x166]), then each entry
2. Writes Enter Queue count ([0x16F]), then each entry
3. Serializes Locomotor via OleSaveToStream

From FootClass::ComputeChecksum (0x4DBAD0):
- Checksums +0x5A4 (NavCom) and +0x5A8 (SuspendedNavCom) via RTTI->UniqueID
- Checksums +0x5D4 and +0x5D8 (extended TarCom pointers) via RTTI->UniqueID
- Does NOT checksum the queue contents directly (only counts matter for determinism)

---

## Additional Fields Beyond TarCom Block

```
+0x5D4  AbstractClass*    Extended target pointer 1 (checksummed, serialized)
+0x5D8  AbstractClass*    Extended target pointer 2 (checksummed, serialized)
+0x5DC  AbstractClass*    (serialized, swizzled)
```

These are likely related to the attack-move or force-fire systems but were not the
focus of this investigation.

---

## Summary Table

| Offset | Size | Field               | Init  | Purpose |
|--------|------|---------------------|-------|---------|
| 0x588  | 24   | NavCom Queue (DVec) | cap=10| Serialized/loaded queued targets, popped on arrival; no standard runtime command producer verified |
| 0x5A0  | 4    | NavCom_Aux          | 0     | Scratch/temp dest, cleared on new dest |
| 0x5A4  | 4    | NavCom              | 0     | Current movement target |
| 0x5A8  | 4    | SuspendedNavCom     | 0     | Saved NavCom during mission override |
| 0x5AC  | 24   | Enter Queue (DVec)  | cap=10| Queue of things to enter |
| 0x5C4  | 4    | TarCom_Type         | -1    | Command type for arrival target |
| 0x5C8  | 4    | TarCom_Primary      | 0     | Primary arrival attack target |
| 0x5CC  | 4    | TarCom_Aux          | 0     | Secondary arrival attack target |
| 0x5D0  | 1    | (padding)           | 0     | |
| 0x5D1  | 1    | TarCom_Active       | 0     | Arrival target handler engaged flag |
