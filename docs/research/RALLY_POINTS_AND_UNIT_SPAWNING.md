# Rally Points & Unit Spawning from Buildings — Ghidra RE Report

**Date:** 2026-03-28
**Source:** gamemd.exe live decompilation via Ghidra MCP
**Confidence:** HIGH — verified from binary, not inferred (exceptions noted)

---

## 1. Rally Point Storage

### Per-building rally target — `TechnoClass + 0x218`

- **Raw pointer to an AbstractClass-derived object** (typically CellClass for rally points)
- Multi-purpose field also used for: harvest destination, enter/dock target, teleport warp target
- Written by `TechnoClass__SetGhostCell` at `0x0070C610` (simple one-liner: `*(this + 0x218) = param`)
- Read via virtual dispatch: `building->field_0x218->vtable[0x48]()` to get target coordinates
- When non-null, exiting units receive `MoveTo(rally_target, 1)` + `SetMission(Guard)`

### Per-house rally point

- **`HouseClass + 0x53DC`** — CellClass pointer for the house-level rally point
- **`HouseClass + 0x53E0`** — packed cell coords (X:short, Y:short), used for overlay rendering

### AI-specific rally fields

| Offset | Purpose |
|--------|---------|
| `HouseClass + 0x54EC` | AI rally strategy mode (1 = standard, others = team-based) |
| `HouseClass + 0x54F0` | Explicit AI rally cell (overrides computed target when set) |
| `HouseClass + 0x5490` | Ally house base cell (fallback for strategy mode 1) |
| `HouseClass + 0x5494` | Ally house secondary cell |
| `HouseClass + 0x5498` | AI scatter radius (clamped 0x300–0x800, leptons) |

---

## 2. Setting Rally Points

### Command 0x1E Packet Structure

The rally point command is sent as a **0x6F byte (111 byte)** event packet with type "MEGAMISSION_F":

| Offset | Size | Field |
|--------|------|-------|
| 0x00 | 1 | Event type = **0x1E** |
| 0x01 | 1 | Flags |
| 0x02 | 1 | Player ID (house index, signed byte) |
| 0x03 | 4 | Frame counter (when event was created) |
| 0x07 | 5 | Source object ID (building) |
| 0x0C | 5 | Target object ID (rally cell/object) |
| 0x10+ | 1 | Additional flag |

**Object ID packing** (FUN_006e6ab0 at `0x006E6AB0`):
Each 5-byte object ID is: 4 bytes ID + 1 byte type tag:
- **Type tag `0x0B`** = CellClass: ID = `cellX + cellY * 1000` (packed cell coordinate)
- **Type tag `0x34`** = AbstractClass: ID = UniqueID from the object's heap entry
- **Type tag `0x00`** = null target

### Command 0x1E Handler

The main event dispatch function is at **`0x004C6CB0`**. Case 0x1E:
1. Resolves source building from `event[0x07]` via `FUN_006E6F20`
2. Validates source is alive via `FUN_006386E0`
3. Resolves target abstract from `event[0x0C]` via `FUN_006E6E20`
4. Calls `TechnoClass__SetGhostCell(source, target)` → writes target pointer to `source + 0x218`

### BuildingClass::SetRallyPoint — `0x00443860`

Player clicks map to set rally point:

1. Gets the CellClass at the clicked position
2. Determines speed type from building:
   - Aircraft (`Factory == 3`, +0xEB8) → SpeedType 4, Zone 9
   - Naval (`+0xCCE`) → SpeedType 6, Zone 4
   - Default → SpeedType 0
3. Calls `FootClass__Find_Nearby_Passable_Cell` to snap to a valid passable cell
4. If passable cell found: packs building + cell into IDs, queues **command 0x1E** into the lockstep command buffer
5. If no passable cell AND building is a `ConstructionYard` (`+0x16B9`): queues command from building's own cell instead
6. Plays `EVA_NewRallyPointEstablished` for human player (unless `ConstructionYard` or `+0x5ED` flag)

### HouseClass::Set_Rally_Point_Cell — `0x004FBF60`

Server-side command handler:

1. Validates cell is within map bounds
2. Clears old rally point via `Clear_Rally_Point`
3. Checks if target cell is passable for the house's primary factory speed type
4. If not passable: calls `Find_Nearby_Passable_Cell` with infantry speed type
5. Stores new CellClass* at `HouseClass + 0x53DC`

### HouseClass::Clear_Rally_Point — `0x004FBE40`

1. For UnitClass rally targets: calls `DetachFlag`
2. For other types: converts object coords to cell, checks cell passability
3. If the stored rally cell matches the building being cleared: nulls `+0x53DC`
4. If `param_3` flag set: clears overlay marker at `+0x53E0` (sets cell's `+0x44` to -1 and resets to InvalidCell)

---

## 3. BuildingTypeClass INI Fields

### Factory & Exit Configuration

| Offset | INI Key | Type | Purpose |
|--------|---------|------|---------|
| +0xEB8 | `Factory=` | int (enum) | Factory type: InfantryType=0xF/0x10, UnitType=1/0x28, AircraftType=2/3, BuildingType=6/7 |
| +0xEC8 | `ExitCoord=X` | int | Exit sub-cell X offset in leptons (256 = 1 cell) |
| +0xECC | `ExitCoord=,Y` | int | Exit sub-cell Y offset in leptons |
| +0xED0 | `ExitCoord=,,Z` | int | Exit sub-cell Z offset in leptons |
| +0xED4 | *(ExitList)* | short* | Array of (dx,dy) cell offset pairs, terminated by (0x7FFF, 0x7FFF) |
| +0xF00 | `DoorStages` | int | Number of animation stages for the building door |

### Building Role Flags

| Offset | INI Key | Type | Effect on Exit |
|--------|---------|------|----------------|
| +0xCCE | *(Naval)* | bool | Building spawns naval units; uses water cell exit path |
| +0x16A9 | `UnitRepair=` | bool | Building can repair units (WF has this=true, enables MissionRepairAndProduce) |
| +0x16AA | `UnitReload=` | bool | Building can reload ammo |
| +0x16AB | `Bunker=` | bool | Building is a bunker |
| +0x16AC | `Cloning=` | bool | Cloning vat: receives cloned unit from barracks production |
| +0x16BB | `Refinery=` | bool | Refinery exit: special harvester spawn with fixed offset |
| +0x16BC | `Weeder=` | bool | Weeder (same category as refinery) |
| +0x16BD | `WeaponsFactory=` | bool | War factory: uses door anim + locomotive drive-out |
| +0x16C1 | `Hospital=` | bool | Hospital/promotion building; affects exit list fallback |
| +0x16C2 | `Armory=` | bool | Armory/promotion building |
| +0x16E4 | `GDIBarracks=` | bool | Allied barracks exit style: cell (x+1, y+2) |
| +0x16E5 | `NODBarracks=` | bool | Soviet barracks exit style: cell (x+2, y+2) |
| +0x16E6 | `YuriBarracks=` | bool | Yuri barracks exit style: cell (x+2, y+1) |
| +0x16B9 | `ConstructionYard=` | bool | ConYard: no rally EVA, special MCV handling |

### Sound Fields

| Offset | INI Key |
|--------|---------|
| +0xE6C | `BuildupSound=` |
| +0xE70 | `PackupSound=` |
| +0xE74 | `CreateUnitSound=` |
| +0xE78 | `UnitEnterSound=` |
| +0xE7C | `UnitExitSound=` |
| +0xE80 | `WorkingSound=` |
| +0xE84 | `NotWorkingSound=` |

### Building Animation Slot Table

Each building has **21 animation slots** (indices 0–20). Each slot in BuildingTypeClass is **0x44 bytes (68 bytes)**:

| Field | Offset within slot | Size | Type |
|-------|-------------------|------|------|
| AnimName | +0x00 | 16 | char[16] |
| AnimNameDamaged | +0x10 | 16 | char[16] |
| AnimNameGarrisoned | +0x20 | 16 | char[16] |
| X | +0x30 | 4 | int |
| Y | +0x34 | 4 | int |
| ZAdjust | +0x38 | 4 | int |
| YSort | +0x3C | 4 | int |
| Powered | +0x40 | 1 | bool |
| PoweredLight | +0x41 | 1 | bool |
| PoweredEffect | +0x42 | 1 | bool |
| PoweredSpecial | +0x43 | 1 | bool |

**Complete slot index → INI key mapping:**

| Slot | Base Offset | INI Key | Usage in WF |
|------|-------------|---------|-------------|
| 0 | 0x0F4C | `PowerUp1Anim` | — |
| 1 | 0x0F90 | `PowerUp2Anim` | — |
| 2 | 0x0FD4 | `PowerUp3Anim` | — |
| 3 | 0x1018 | `ActiveAnim` | Restored after door opens |
| 4 | 0x105C | `ActiveAnimTwo` | — |
| 5 | 0x10A0 | `ActiveAnimThree` | — |
| 6 | 0x10E4 | `ActiveAnimFour` | — |
| 7 | 0x1128 | `PreProductionAnim` | — |
| 8 | 0x116C | `ProductionAnim` | Cleared during WF exit |
| 9 | 0x11B0 | `TurretAnim` | — |
| 10 | 0x11F4 | **`SpecialAnim`** | **WF door opening anim** |
| 11 | 0x1238 | `SpecialAnimTwo` | Cleared during WF exit |
| 12 | 0x127C | **`SpecialAnimThree`** | **WF "production running" door anim** |
| 13 | 0x12C0 | `SpecialAnimFour` | — |
| 14 | 0x1304 | `SuperAnim` | — |
| 15 | 0x1348 | `SuperAnimTwo` | — |
| 16 | 0x138C | `SuperAnimThree` | — |
| 17 | 0x13D0 | `SuperAnimFour` | — |
| 18 | 0x1414 | `IdleAnim` | Cleared when WF door opens |
| 19 | 0x1458 | `LowPower` | — |
| 20 | 0x149C | `SuperLowPower` | — |

Each slot has healthy / damaged / garrisoned variants at offsets +0x00, +0x10, +0x20 within the slot.
Damage state selected by comparing health ratio against `RulesClass + 0x1700` (ConditionYellow).

**Additional non-slot anim fields:**
| Offset | INI Key | Format |
|--------|---------|--------|
| 0x0F10 | `AnimIdle=` | 3 ints: start,count,rate |
| 0x0F1C | `AnimActive=` | 3 ints: start,count,rate |
| 0x0F34 | `AnimAux1=` | 3 ints: start,count,rate |
| 0x0F40 | `AnimAux2=` | 3 ints: start,count,rate |

---

## 4. War Factory State Machine

### CORRECTION: The War Factory uses a separate mission handler, NOT MissionRepairAndProduce

**`MissionRepairAndProduce`** (`0x0044B780`) handles hospitals, armories, cloning vats, and veterancy buildings.
The **War Factory** uses the slot-26 mission handler at **`FUN_0044D880`** for its door/drive-out sequence.
This function also hosts the Slave-Miner slave-deployment subsystem (Type+0x16AE/+0x16AF) — both are
"unload cargo" semantics, hence the shared slot. MissionRepairAndProduce's door timer logic
(URepairRate * 900.0) applies to hospitals/armories only.

> **Note:** Earlier drafts of this doc named this function "MissionUnload at `0x0044DCB9`". `0x0044DCB9`
> is **not a function entry** — it is an internal jump target inside `FUN_0044D880` that Ghidra
> mis-promoted to a function symbol. Decompiling at `0x0044DCB9` yields a fragment with type-propagation
> warnings; use `0x0044D880` for the full function. The exact YR mission enum index for slot 26
> ("Unload"?) is unconfirmed.

The WF enters this mission when ExitObject calls `building->SetMission(mission_id, 0)` after placing the unit.

### BuildingClass Fields Used

| Offset | Type | Name | Purpose |
|--------|------|------|---------|
| `0xBC` | int | MissionState | 0=setup, 1=unit driving out, 2=door animating |
| `0x218` | ptr | RallyTarget | AbstractClass* for rally point destination |
| `0x2FC` | int | DoorFrameCounter | Decremented when building has DoorStages |
| `0x534` | int | ProductionSlot1 | First production slot |
| `0x538` | int | ProductionSlot2 | Second production slot |
| `0x55C` | ptr[21] | Anims[0..20] | AnimClass* pointers for 21 anim slots |
| `0x57C` | int | ProductionFlag1 | Production-related flag |
| `0x588` | int | ProductionFlag2 | Production-related flag |
| `0x58C` | int | ProductionFlag3 | Additional production state |
| `0x620` | int | DoorProgress | Accumulates door animation progress (leptons * ticks) |
| `0x624` | byte | DoorAnimChanged | 1 if door anim advanced this tick, 0 otherwise |
| `0x628` | int | CDTimer_StartFrame | CDTimerClass start frame |
| `0x62C` | int | CDTimer_Internal | CDTimerClass internal field |
| `0x630` | int | CDTimer_Duration | CDTimerClass duration (ticks between door steps) |
| `0x634` | int | DoorStepDirection | +1 = opening, -1 = closing, 0 = stopped |
| `0x638` | int | DoorStepAmount | Amount added to DoorProgress each step |
| `0x6DD` | byte | ProductionComplete | Set to 1 when door is fully open/unit should exit |

### RulesClass Timing Values

| Offset | INI Key | Type | Purpose |
|--------|---------|------|---------|
| `0x16E8` | `URepairRate` | double | **WF door open threshold** = `URepairRate * 900.0` |
| `0x16F0` | `IRepairRate` | double | Hospital/Armory door threshold = `IRepairRate * 900.0` |
| `0x16F8` | *(hardcoded 1.0)* | double | ConditionGreen (full health threshold) |
| `0x1700` | `ConditionYellow` | double | Below this = damaged state (affects anim selection) |

**Hardcoded constant: `DAT_007E27F8` = 900.0** (IEEE 754 double `0x408C200000000000`).

### WF Mission State Machine (FUN_0044D880, slot 26)

The WF mission handler has **5 states** (field_0xBC):

#### State 0 — Door Opening Init

- Play door-open sound (`FUN_004A51F0(Type+0x3C8, Type+0x3CC)` — sound IDs)
- Clear an anim slot via `BuildingClass__ClearAnimSlot`
- Create the **ProductionAnim (slot 8, +0x116C)** — or its damaged variant at `+0x117C`
  when `health_ratio <= ConditionYellow` (Rules+0x1700). This is the door-opening anim;
  the slot 10 SpecialAnim is NOT used here.
- Set `field_0xBC = 1` (non-naval) or `field_0xBC = 4` (naval)
- Set `field_0x80 = 1` (building dirty flag)

#### State 1 — Bib Clearing

- Calls `FUN_00449540` to scatter any units blocking the WF bib area (see §4a below)
- If bib is clear or clearing fails: logs "kicking out unit" and advances to **State 2**

#### State 2 — Locomotion Setup + Drive-Out Start

- Gets the produced unit from the radio link
- Checks unit's locomotion CLSID via `IPersist::GetClassID()`:
  - **HoverLocomotion or TeleportLocomotion**: replace with DriveLocomotion, piggyback original
  - **WalkLocomotion or MechLocomotion**: replace with DriveLocomotion, piggyback original
  - **DriveLocomotion (native)**: no piggybacking needed, just send `Force_Track` (vtable 0x70, param=0x42)
- Sets unit speed to **0.5** (`0x3FE00000` as float)
- Transitions to **State 3**

#### State 3 — Monitoring Drive-Out (radio-tether gated)

- Stays in State 3 while `building->field_0x418 != 0`
- When `building->field_0x418 == 0`: call `FUN_004A5240(Type+0x3C8, Type+0x3CC)` (play door-close sound) and transition to **State 4**

**What `field_0x418` actually is:** a 1-byte **radio tether flag** on TechnoClass
(present on both BuildingClass and FootClass instances, since they both derive from
TechnoClass). It is set/cleared by `TechnoClass::Receive_Radio` (`0x006F4AB0`) as a
side effect of the radio-link handshake — not by any distance check.

| Address | Site | Effect |
|---|---|---|
| `0x004492B7` | BuildingClass activation/register loop (range `0x00448E30` – `0x004493FB`) | After sending radio cmd `2` and getting a positive ack, sets both `building+0x418 = 1` AND `target+0x418 = 1` |
| `0x006F4B72` | `Receive_Radio` case **`0x18`** ("HELLO / establish link") | `this+0x418 = 1`, forwards `0x18` |
| `0x006F4BA6` | `Receive_Radio` case **`0x19`** ("BYE / break link") | `this+0x418 = 0`, forwards `0x19` |
| `0x004C7342` | `EventClass::Execute` lockstep handler | `target+0x418 = 0` (network-event variant of `0x19`) |

`Receive_Radio` case **`8`** is the originator of a tether break: when received, it
transmits `0x19` to the other party (which clears that party's flag and propagates),
then transmits `3` ("over and out").

So State 3 → State 4 fires exactly when the WF's tether flag has been cleared. In the
WF flow, the building set the flag during ExitObject (via `Transmit_Radio(2)` +
`Transmit_Radio(0x18)`) — see §22. The break is initiated when the unit signals
"I'm done driving out" (originating party is one hop further out than this report
traces, but the radio path is `Receive_Radio` case `8` → forwards `0x19` → flags clear).

Earlier drafts of this doc described the State 3 gate as "Distance < 100 leptons from
exit point". That is wrong — no distance constant is involved at this site. The gate
is purely event-driven through the radio protocol.

#### State 4 — Door Closing

- Waits for door close animation to complete
- Clears radio contact with the unit
- Restores building dirty flag
- Returns to Guard mission

### 4a. Bib Clearing (FUN_00449540 at `0x00449540`)

Called in State 1 to scatter units from the WF bib (the area in front of the door):

1. Checks `Type[0x16BD]` (WeaponsFactory) — exits early if not WF
2. Gets the **bib offset** from `*(int *)(Type+0xED4) + 0x28` — i.e., the entry at byte
   offset `+0x28` (entry **10** when entries are 4-byte `(dx:short, dy:short)` pairs)
   of the ExitList array. NOT the first entry. The same offset-`+0x28` read appears in
   `FUN_0044D880` State 0's bib-cell setup. The magic 10 index appears to be a WF-specific
   convention placing the bib (door-front) cell at a fixed slot in the ExitList.
3. Adds offset to building's top-left cell with an additional `-1` on the X delta:
   `bib_cell = (top_left.x + dx - 1, top_left.y + dy)` → **bib cell** (cell in front of WF door)
4. Calls `CellClass__Find_Nearest_Object` on the bib cell
5. **If a unit is blocking:**
   - Logs: `"Weapons factory clearing %s from bib\n"`
   - Calls `CellClass__Scatter_Objects(force=1, force_move=1)` to push unit away
   - **Retries up to 8 times** (loop `iVar4 < 8`), calling `Pathfinding_update_continued()` each iteration
   - If still blocking after retries: logs `"Weapons factory clearing %s from bib area\n"`, scatters again
6. Returns true if unit was found and scattered, false if bib was clear

### MissionRepairAndProduce (Hospitals/Armories/Cloning Vats)

**`BuildingClass::MissionRepairAndProduce`** at `0x0044B780` handles non-WF buildings only:
- `Type[0x16C1]` (Hospital): veteran promotion with door animation
- `Type[0x16C2]` (Armory): same as hospital
- `Type[0x16B9]` (ConstructionYard): MCV deploy handling
- `Type[0x16A9]` (UnitRepair but not WeaponsFactory): service depot

The door timer for these uses: `DoorProgress >= IRepairRate * 900.0` (hospitals) or `URepairRate * 900.0` (service depots).

---

## 5. Locomotive Piggybacking (WF Drive-Out)

When a unit that normally walks (infantry-style) or mechs is produced from a War Factory, the engine temporarily replaces its locomotion with a DriveLocomotion so it can drive out the factory door.

### Locomotion CLSIDs

| Address | CLSID | Locomotion |
|---------|-------|------------|
| `0x007E9A30` | `{4A582741-9839-11D1-B709-00A024DDAFD1}` | **DriveLocomotion** |
| `0x007E9A40` | `{4A582742-9839-11D1-B709-00A024DDAFD1}` | **WalkLocomotion** |
| `0x007E9A50` | `{4A582743-9839-11D1-B709-00A024DDAFD1}` | HoverLocomotion |
| `0x007E9A60` | `{4A582744-9839-11D1-B709-00A024DDAFD1}` | FlyLocomotion |
| `0x007E9A70` | `{4A582745-9839-11D1-B709-00A024DDAFD1}` | RocketLocomotion |
| `0x007E9A80` | `{4A582746-9839-11D1-B709-00A024DDAFD1}` | DropPodLocomotion |
| `0x007E9A90` | `{4A582747-9839-11D1-B709-00A024DDAFD1}` | TeleportLocomotion |
| `0x007E9AA0` | `{55D141B8-DB94-11D1-AC98-0060080055B5}` | ShipLocomotion |
| `0x007E9AB0` | `{2BEA74E1-7CCA-11D3-BE14-00104B62A16C}` | **MechLocomotion** (YR-added) |

### IPiggyback Interface

**IID_IPiggyback:** `{92FEA800-A184-11D1-B70A-00A024DDAFD1}` at `0x007E9B10`

**IPiggyback vtable** (DriveLocomotionClass, at `0x007E7E88`):

| Offset | Method | Address |
|--------|--------|---------|
| 0x00 | QueryInterface | 0x004B4DC0 |
| 0x04 | AddRef | 0x004B4DD0 |
| 0x08 | Release | 0x004B4DE0 |
| 0x0C | **Begin_Piggyback** | 0x004AF8E0 |
| 0x10 | **End_Piggyback** | 0x004AF930 |
| 0x14 | Is_Ok_To_End | 0x004AF970 |
| 0x18 | Piggybacker_CLSID | 0x004AF610 |
| 0x1C | Is_Piggybacking | 0x004B4CD0 |

### Begin_Piggyback (0x004AF8E0)
```c
HRESULT Begin_Piggyback(ILocomotion* original_loco) {
    if (original_loco == NULL) return E_POINTER;  // 0x80004003
    if (this->PiggybackedLoco != NULL) return 0x80004005;  // already piggybacking
    this->PiggybackedLoco = original_loco;  // DriveLocoClass + 0x50
    original_loco->AddRef();
    return S_OK;
}
```

### End_Piggyback (0x004AF930)
```c
HRESULT End_Piggyback(ILocomotion** out_original) {
    if (out_original == NULL) return E_POINTER;
    if (this->PiggybackedLoco != NULL) {
        *out_original = this->PiggybackedLoco;
        this->PiggybackedLoco = NULL;
        return S_OK;
    }
    return 1;  // nothing to unpiggyback
}
```

### Which units get piggybacked?

The WF mission handler (`FUN_0044D880`, State 2) compares the unit's native locomotion CLSID against:
- **WalkLocomotion** (`0x007E9A40`)
- **MechLocomotion** (`0x007E9AB0`)

These are the two locomotion types that get a DriveLocomotion piggybacked for WF drive-out.
Units with DriveLocomotion natively do NOT need piggybacking.

### Drive-out piggybacking sequence

1. **Unit spawned in WF** — locomotion is checked via `IPersist::GetClassID()`
2. If Walk or Mech: a DriveLocomotion is created, `Begin_Piggyback(original_loco)` stores the original
3. Unit assigned to `FootClass + 0x674` (Locomotor) gets the DriveLocomotion
4. Unit drives out of WF using Drive behavior
5. **State 3 → State 4 gated by `building->field_0x418 == 0`** (a 1-byte radio-tether
   flag on TechnoClass). It is set to 1 during ExitObject by the building's
   `Transmit_Radio(2)` + `Transmit_Radio(0x18)` handshake, and cleared to 0 when the
   tether is broken via `Receive_Radio` case `0x19`. See §4 State 3 for the full
   site-by-site write map (`0x004492B7`, `0x006F4B72`, `0x006F4BA6`, `0x004C7342`).
   Earlier drafts called this a "Distance < 100 leptons" check — there is no distance
   constant at this site; the gate is event-driven through the radio protocol.
6. Building transitions `field_0xBC = 1`, returns timer = 3 (fast tick)

**Note (MEDIUM confidence):** The piggybacking creation (step 2) likely happens inside
ExitObject or the production placement code, not in `FUN_0044D880` itself. The state
machine in `FUN_0044D880` only handles the monitoring and unpiggyback phase.

---

## 6. ExitObject — The Unit Spawning Function

**`BuildingClass::ExitObject`** — vtable +0x100, address `0x00443C60`
(Ghidra label: `BuildingClass__ExitObject_Main`, 1034 lines of decompiled code.)

Called by `HouseClass::Place_Production` as:
```c
result = building->vt->ExitObject(building, produced_object, InvalidCell);
// Returns: 0 = failed, 1 = building placed, 2 = unit/infantry exited
```

### Dispatch by RTTI type of produced object

#### Infantry (RTTI == 2)

1. Calls `HouseClass__AI_EconomyStateMachine(2)` to update economy tracking
2. Resets `HouseClass::Primary_ForInfantry` (`House+0x5658`) to `-1`
3. **Dispatch check:** calls `FUN_0065adf0(building, produced_infantry)`. This iterates
   the building's `Comm_With` array at `+0xE4` (count at `+0xE8`) and returns 1 if any
   entry is `NULL` or matches the produced infantry, else 0. **`RadioClass::Constructor`
   at `0x0065A750` allocates a 1-slot array and writes `count=1, entry[0]=NULL`**, so
   for any building whose Comm_With is currently unused (the normal case for a barracks
   that just finished a production cycle), this function returns 1.
4. **If `FUN_0065adf0` returns 1 (normal case): alt-path infantry exit.** Jumps to
   `0x443F54`. This is the path actually taken for vanilla barracks production:
   - Call `FUN_005F6060(infantry, Z)` — adjusts the infantry's Z lepton coord
     (`+0xA4`), re-Mark()ing if the unit is already on the map
   - Call `building->GetDockCoord()` (vtable +0xA8, address `0x00447B20`). For a default
     barracks (no Weeder/Refinery/Bunker/UnitRepair/+0x16CB flag), this falls through
     to `FUN_005F6C80` which simply calls `building->GetCoord()` (vtable +0x48) —
     **the building's center lepton coord** (the value at `building+0x9C..0xA4`)
   - Call `infantry->Unlimbo(coord)` (vtable +0xD8) at that center coord. Infantry
     can occupy building-foundation cells (only vehicles are hard-blocked there), so
     the Unlimbo succeeds.
   - Establish radio tether: `building->Transmit_Radio(2, infantry)` (QUERY) followed
     by `building->Transmit_Radio(0x18, infantry)` (HELLO). The handshake sets both
     `building+0x418 = 1` and `infantry+0x418 = 1` (see §4 State 3 for the tether-flag
     mechanism). Also sets `infantry+0x6CC = building` (parent link).
   - Call `building->GetDockCoord()` again (cached), then `infantry->Set_Location(coord)`
     (vtable +0x1B4) — re-asserts position after the radio handshake
   - If `building->RallyTarget` (`+0x86`, ground rally) is non-NULL AND infantry is not
     Amphibious: `infantry->MoveTo(rally_target, 1)` (vtable +0x480) +
     `infantry->SetMission(MISSION_MOVE = 2)` (vtable +0x1E8).
   - The unit then pathfinds out of the foundation cells to the rally point — **this
     is the visible "exits through the door" behavior**. The pathfinder treats
     infantry-on-foundation as passable but suboptimal, so it walks straight to the
     nearest exterior cell on the way to the rally.

5. **If `FUN_0065adf0` returns 0 (edge case): "scatter on map" fallback path at
   `0x443D18`.** Fires only when the building's Comm_With slot is occupied by some
   *other* entity at the moment of production (e.g., the barracks is already in radio
   contact with an unrelated unit). The placement coord is computed from radar-map
   bounds + random scatter:
   - `cell_X = DAT_0087F8E4 + DAT_0087F8E8 + scatter` (or `+ DAT_0087F8EC` if the
     building is past a diagonal threshold)
   - `cell_Y = DAT_0087F8E8 - DAT_0087F8E4 + scatter` (or `- DAT_0087F8EC`)
   - `scatter = Random__RandomRanged(0, DAT_0087F8F0)`
   - The four `DAT_0087F8E4..F0` globals are the **radar-projected map rectangle**,
     written via `RadarClass::ComputeRadarMapBounds(&DAT_0087F8E4)` inside `FUN_00655990`
     during scenario init.
   - Then `Unlimbo` at `(cell * 256 + 128, cell * 256 + 128, 0)`, MoveTo rally, etc.
   - This branch produces a map-wide scatter result inconsistent with normal "infantry
     exits the barracks" behavior — it is practically unreachable for a vanilla barracks
     and likely a leftover for an edge case.

6. **Amphibious produced infantry (`Type+0xE0D` set):** if the building does NOT also
   set up that path (the secondary check at `0x443CFC`), `ExitObject` returns 0 (no
   placement attempted by this case). Amphibious infantry from non-amphibious-producing
   buildings is unsupported.

> **What does NOT happen for infantry exit:**
>
> - `BuildingTypeClass + 0xEC8/0xECC/0xED0` (`ExitCoord=`) is **never read** for infantry.
> - The GDI/NOD/Yuri Barracks-flag exit cells `(x+1,y+2)` / `(x+2,y+2)` / `(x+2,y+1)`
>   from `+0x16E4/0x16E5/0x16E6` are **never used** for infantry — they are consumed by
>   `GetDockCellForObject` (§8), which is called only by the vehicle/aircraft post-switch
>   code, never by case 2. The doc table at §3 lists those flags for completeness, but
>   their effect is on vehicle/aircraft exit from barracks-flagged buildings.
> - `GetDockCellForObject` (vtable +0x4D4) is **never called** in the infantry path.
> - The `ExitList` array (`Type+0xED4`) is **never iterated** for infantry.

#### Vehicles (RTTI == 1) — non-WeaponsFactory path

For barracks-type buildings producing vehicles, or generic unit factories:

1. Calls `HouseClass__AI_EconomyStateMachine(unit_type)`
2. Resets `HouseClass::Primary_ForVehicles` (0x5650) to -1
3. Checks building flags: `Hospital`, `Armory`, `WeaponsFactory`, `Refinery`
4. **Standard exit path** (not WF, not Hospital, not Armory, not Cloning):
   - Calls **`GetDockCellForObject`** (vtable 0x4D4) to find exit cell
   - If InvalidCell returned → return 0 (failed)
   - Calculates facing: `atan2(building_center - exit_cell)` → converts to 256-unit facing, then `(facing >> 7) + 1 >> 1` gives an 8-bit direction
   - Gets building top-left cell from vtable 0x1B8
   - **Adjusts exit toward building edge:**
     - If exit X < foundation_left: `exit_x + 1`
     - If exit X >= foundation_right: `exit_x - 1`
     - Same for Y axis
   - **ExitCoord application** (only when exit matches barracks-specific cells):
     - If `GDIBarracks` (+0x16E4) AND exit == `(foundation_x+1, foundation_y+2)`:
       ```
       final_x += ExitCoord.X    (+0xEC8)
       final_y += ExitCoord.Y    (+0xECC)
       final_z += ExitCoord.Z    (+0xED0)
       ```
     - If `NODBarracks` (+0x16E5) AND exit == `(foundation_x+2, foundation_y+2)`: same
     - If `YuriBarracks` (+0x16E6) AND exit == `(foundation_x+2, foundation_y+1)`: same
   - Calls **`Unlimbo`** at final position with computed facing
   - `SetMission(Guard)` + `MoveTo(exit_cell, 1)`
   - **AI dispatch:** if not player-controlled:
     - `SetMission(Area Guard)` (0xB)
     - Calls `FUN_00500200` → finds rally/scatter target
     - Sets `GhostCell` and `Enter_Destination` to the target

#### Aircraft (RTTI == 0xF) — uses the SAME path as vehicles

Aircraft (Helipad, Air Force Command, Allied Battle Lab spawner, etc.) hit the switch
at `case 1: case 0xf: break;` — the case 0xF arm just `break`s, falling through to the
post-switch code. After the switch:

1. The "FUN_0065adc0" / not-Hospital / not-Armory / not-WF / Factory != 0x10 conditional
   evaluates to true for Helipad → enters the **Standard exit path above**.
2. `GetDockCellForObject` (vtable 0x4D4) is called. For a Helipad/AirForceCommand, none
   of the special-cased tiers match (it isn't Barracks, isn't `WF+Naval`, and `target_cell`
   is `InvalidCell` because `ExitObject` was called from `Place_Production` with no cell),
   so the **ExitList iteration** tier fires and returns the first passable cell from
   `BuildingTypeClass + 0xED4` — typically the pad cell.
3. Foundation edge adjustment runs (the same `if exit_x < foundation_left: exit_x + 1`
   logic). For a typical Helipad whose ExitList[0] is already on the pad, this is a no-op.
4. The barracks `ExitCoord` add does NOT apply (Helipad isn't GDIBarracks/NODBarracks/
   YuriBarracks, and the (foundation_x+N, foundation_y+M) cell-match conditions wouldn't
   line up anyway). The cell-level coord goes through unmodified.
5. `Unlimbo` at the resulting cell with the `atan2`-computed facing.
6. Player-controlled: `Queue_Mission(2 = MOVE)` + `MoveTo(exit_cell, 1)` — since the
   aircraft is already on the pad cell, this transitions it through MISSION_MOVE
   trivially and lands in its idle state via `AircraftClass::Enter_Idle_Mode`.
7. AI: `Queue_Mission(0xB = AREA_GUARD)` + `FUN_00500200` scatter, just like vehicles.

There is **no aircraft-specific branch** in `ExitObject`. The doc's §16 `HasPadAvailable`
note refers to `FindFactory` (the factory-selection step that runs BEFORE `ExitObject`),
not to a separate aircraft exit path. Aircraft spawned from Helipads are placed via the
same RTTI-1 vehicle pipeline; they just happen to use a building that defines `Factory=
AircraftType` and an `ExitList` whose entry 0 points at the pad cell.

**Carriers / Dreadnought / Boomer / V3 aircraft are NOT placed through this path** —
those are children of a `SpawnManagerClass` attached to the parent ship/launcher when
the parent itself unlimbos. See §12 and the standalone
[SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](./SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md) for that
mechanism.

#### WeaponsFactory exit (`+0x16BD`, Naval=false)

Same as vehicle exit above, plus:
- If `Factory == 0x10` (infantry factory) AND not Cloning: finds sibling building with same type in Guard mission and transfers the produced unit to it (for cloning vats)

> **Note:** Earlier drafts of this doc claimed that the WF exit path calls
> `FUN_006B0D60` ("SpawnManagerClass::DeployAllSpawns") on units with `+0xB6 != 0`.
> That is incorrect: the call to `FUN_006B0D60` (actually
> `SlaveManagerClass::DeployAllSlaves`) lives in the **case 6 / Building placement** branch
> of `ExitObject_Main`, gated by the produced *building*'s SlaveManager pointer at byte
> offset `+0x2D8`, not on a unit's `+0xB6` field. See §12 for the corrected version.
> Aircraft-Carrier-style spawn release on WF exit is NOT handled by this function and is
> not traced in this report.

#### Naval factory (`+0xCCE` set)

1. Gets building center cell
2. Walks outward using 8-direction offsets to find a water cell
3. Checks if cell at each step is water (cell type == 2) or has no blocking object
4. Falls back to `FootClass__Find_Nearby_Passable_Cell` with the naval unit's speed type
5. `Unlimbo` on water cell → `MoveTo(rally_target)` if set

#### Refinery exit (`+0x16BB` or `+0x16BC`)

1. Special harvester spawn using direction 5 (SW) offsets:
   - `DAT_0089F69C` — X cell offset (short, direction 5 x component)
   - `DAT_0089F69E` — Y cell offset (short, direction 5 y component)
2. Final position (binary applies an additional `DAT_0089F698` offset during the
   cell→lepton conversion):
   ```
   cell_x = center.cell_x + DAT_0089F69C
   cell_y = center.cell_y + DAT_0089F69E
   lepton_x = (DAT_0089F698.low  + cell_x) * 256 + 128
   lepton_y = (DAT_0089F698.high + cell_y) * 256 + 128
   ```
3. `Unlimbo` → `FacingClass__UpdateFacing(0x8000)` (face south)
4. `Queue_Mission(mission_id=10, commence=0)` — mission 10 (`0xA`) is **MISSION_HARVEST**,
   not Guard. The harvester immediately begins its harvest behavior on spawn — that is
   why it ignores the rally point and goes hunting for ore on its own.

#### Building placement (RTTI == 6)

Uses player-specified cell, calls `CanBePlacedAt`, then `Unlimbo`. Not rally-relevant.

---

## 7. Infantry Sub-Cell Positions

### Sub-Cell Offset Table

**Address:** `0x0089E9F0` (runtime-populated by `CellClass__InitSubCellOffsets` at `0x0048E480`)

5 entries, each 3 ints (X, Y, Z) in **leptons** (cell = 256 leptons):

| Index | X | Y | Z | Position |
|-------|---|---|---|----------|
| 0 | 128 (0x80) | 128 (0x80) | 0 | **Center** |
| 1 | 64 (0x40) | 64 (0x40) | 0 | **Top-Left** |
| 2 | 192 (0xC0) | 64 (0x40) | 0 | **Top-Right** |
| 3 | 64 (0x40) | 192 (0xC0) | 0 | **Bottom-Left** |
| 4 | 192 (0xC0) | 192 (0xC0) | 0 | **Bottom-Right** |

### Sub-Cell Selection Algorithm (CellClass::GetSubCell at `0x004810A0`)

Given a lepton position within a cell:
1. Compute distance from cell center (128, 128)
2. If distance < 60 leptons (0x3C): return **0** (center)
3. Otherwise: `bit0 = (X > 128)`, `bit1 = (Y > 128)`, result = `(bit1 << 1 | bit0) + 1` if nonzero, else 0

### Sub-Cell Preference Tables

**Address:** `0x0081CC84` — priority order for trying sub-cells based on approach direction:

```
Index 1 (TL): [1, 2, 3, 4]  → try TL first, then TR, BL, BR
Index 2 (TR): [0, 2, 3, 4]  → try center, then TR, BL, BR
Index 3 (BL): [0, 1, 4, 3]  → try center, TL, BR, BL
Index 4 (BR): [0, 1, 4, 2]  → try center, TL, BR, TR
              [0, 2, 3, 1]  → center, TR, BL, TL
```

**Random center table** at `0x0081CC98`:
```
[1, 2, 3, 4]  → rotating priority:
[2, 3, 4, 1]
[3, 4, 1, 2]
[4, 1, 2, 3]
```

Used in `CellClass__PlaceInfantryInCell` to find an unoccupied sub-cell. The cell tracks occupancy via two bitfields at `CellClass + 0x124` (ground) and `+0x128` (bridge), where bit N = sub-cell N is occupied.

---

## 7a. GetExitCoord — WF Door Position

**`BuildingClass::GetExitCoord`** — vtable +0xB4, address `0x0044F640`

This is the function the WF state machine (`FUN_0044D880`) and `ExitObject`'s WF path
use to compute the **door coordinate in leptons** for spawning the produced unit. It is
NOT used by the standard vehicle / barracks / refinery / naval / aircraft exit paths —
those use `GetDockCellForObject` (vtable +0x4D4) for cell-level selection.

### Algorithm

```c
void BuildingClass::GetExitCoord(BuildingClass* this, CoordStruct* out) {
    auto Type = this->Type;

    // Sentinel: ExitCoord = (0, 0, 0) means "not set in INI" → fall back to center.
    // DAT_0089c848/0089c84c/0089c850 hold a 12-byte all-zeros sentinel.
    if (Type->ExitCoord.X == 0 &&
        Type->ExitCoord.Y == 0 &&
        Type->ExitCoord.Z == 0) {
        CoordStruct c = this->GetCoord();    // vtable +0x48
        *out = c;
        return;
    }

    // Else: building's lepton origin + ExitCoord delta (all leptons).
    out->X = Type->ExitCoord.X + this->Position.X;   // this+0x9C
    out->Y = Type->ExitCoord.Y + this->Position.Y;   // this+0xA0
    out->Z = Type->ExitCoord.Z + this->Position.Z;   // this+0xA4
}
```

### Notes

- `ExitCoord` is stored in **leptons** (256 leptons = 1 cell). It's a delta from the
  building's TopLeft anchor, NOT an absolute world coord. INI parses three ints
  `ExitCoord=X,Y,Z` into `BuildingTypeClass + 0xEC8 / 0xECC / 0xED0` (see §3).
- The "unset" sentinel is **`(0, 0, 0)`**, not `0xFFFFFFFF`. This means a Helipad with
  literal `ExitCoord=0,0,0` is indistinguishable from one that omits the key — both
  resolve to the building center. (`DAT_0089c848/4C/50` is the 12-byte sentinel; static
  memory holds zeros and the function compares against it.)
- The `Z` component matters for jumpjet / helipad / cliff-edge buildings where the
  door is elevated above the foundation.
- `this->Position` is the building's **lepton coords** (`this+0x9C / +0xA0 / +0xA4`),
  not its cell coords. Building::Unlimbo sets these from the placement cell × 256.

### Call sites

| Caller | Purpose |
|---|---|
| `BuildingClass::ExitObject` WF path (§22) | `Unlimbo(exit_coord, facing=0x40)` — drop the unit at the door |
| `FUN_0044D880` State 0 | Bib-cell setup for door anim positioning |
| Numerous building anim/SHP placement sites | Door-coord referenced by anim creation |

### Confidence

HIGH — function decompiled in one pass, sentinel verified by `read_memory` at
`0x0089c848` (returns 12 bytes of zero), call site in `ExitObject` decompile matches
the vtable +0xB4 dispatch.

---

## 8. GetDockCellForObject — Exit Cell Selection Algorithm

**`BuildingClass::GetDockCellForObject`** — `0x0044EFB0`

Takes: building, produced object, optional target cell
Returns: best exit cell (or InvalidCell if none found)

### Priority order

**1. Barracks-specific hardcoded exits** (checked first, each verified for passability):

| Flag | Exit Cell (relative to building top-left) |
|------|-------------------------------------------|
| `GDIBarracks` (+0x16E4) | `(top_left_x + 1, top_left_y + 2)` |
| `NODBarracks` (+0x16E5) | `(top_left_x + 2, top_left_y + 2)` |
| `YuriBarracks` (+0x16E6) | `(top_left_x + 2, top_left_y + 1)` |

Each checks `Cell_in_bounds_check` and then the unit's `CanEnterCell` (vtable 0x1AC) with `force=1`. First passable one wins.

**2. WeaponsFactory + Naval** (`+0x16BD` AND `+0xCCE`):

Uses the building's center coords (via vtable 0xA8) and tries:
- `(center_x + 1, center_y + 1)` — force=0
- `(center_x + 1, center_y)` — force=0
- `(center_x, center_y + 1)` — force=0

**3. Explicit target cell** (if passed and != InvalidCell):

Checks passability with force=0. If passable, returns it.

**4. ExitList** (`+0xED4`, if non-null AND NOT `Hospital`):

Iterates the array of `(dx:short, dy:short)` pairs:
```
exit_cell = (top_left_x + dx, top_left_y + dy)
```
Terminated by `(0x7FFF, 0x7FFF)`. Each checked for passability with force=0.

**5. Foundation edge scan** (fallback when no ExitList, or Hospital=true):

Scans cells around the building perimeter:

- **Bottom + top edges:** for x from -1 to foundation_width:
  - Check `(top_left_x + x, top_left_y + foundation_height)` with force=1
  - Check `(top_left_x + x, top_left_y - 1)` with force=1

- **Right + left edges:** for y from -1 to foundation_height:
  - Check `(top_left_x + foundation_width, top_left_y + y)` with force=1
  - Check `(top_left_x - 1, top_left_y + y)` with force=1

Returns **InvalidCell** if no passable exit found at any stage.

---

## 9. 8-Direction Cell Offset Table

**Address:** `0x0089F688` (runtime-populated, 8 entries of `short x, short y`)

| Index | Direction | Cell Offset (dx, dy) |
|-------|-----------|---------------------|
| 0 | N | (0, -1) |
| 1 | NE | (1, -1) |
| 2 | E | (1, 0) |
| 3 | SE | (1, 1) |
| 4 | S | (0, 1) |
| 5 | SW | (-1, 1) |
| 6 | W | (-1, 0) |
| 7 | NW | (-1, -1) |

Used in the naval exit path to walk from building center outward toward water.
Refinery exit uses direction 5 (SW) offsets specifically.

**Confidence:** MEDIUM — address verified from binary, values are standard RA2/TS convention (runtime-initialized, not static in .rdata).

---

## 10. Post-Exit Rally Dispatch

### Player-controlled buildings

- Unit placed with `SetMission(Guard)` + `MoveTo(exit_cell, 1)`
- If building has rally target (+0x218): unit gets `MoveTo(rally_target, 1)` immediately after unlimbo
- Unit auto-walks to rally point

### AI-controlled buildings

**`FUN_00500200`** @ `0x500200` — computes AI rally/scatter destination:

1. Counts unit weapons (primary + secondary + special via vtables 0x2DC/0x2D8/0x2D4)
2. If armed: picks random mode 1–4; if unarmed: mode 0
3. Calls **`FUN_00501AC0`** with mode to compute destination:

| Mode | Behavior |
|------|----------|
| 0 | Short range random near enemy base center |
| 1 | Medium range, directional bias (cos/sin from random angle) |
| 2 | Medium range, opposite directional bias |
| 3+ | Long range, random direction |

- Radius based on `HouseClass + 0x5498`, clamped to 0x300–0x800 leptons
- `iVar1 = radius * 2` for medium/long range modes

4. Final cell: `Find_Nearby_Passable_Cell` near the computed target
5. Sets `GhostCell` (pathfinding reservation) + `Enter_Destination`

### AI rally point functions

**`HouseClass::AI_GroundRallyPoint`** @ `0x509CD0`:
- If explicit rally cell (`+0x54F0`) set → use it
- If strategy mode == 1: find passable cell near ally base `+0x5490/0x5494`, offset by +2 cells
- Otherwise: call `AI_FindTeamTarget` for team-based rally

**`HouseClass::AI_NavalRallyPoint`** @ `0x509E00`:
- Same pattern for naval units
- Strategy mode 1: calls `AI_FindBestRallyTarget`

**`HouseClass::AI_FindBestRallyTarget`** @ `0x50CBF0`:
- Iterates ALL technos of the allied house
- Scores each as potential rally target based on:
  - Unit type and class (vehicles, buildings by factory type, aircraft)
  - Reads from `RulesClass` side-specific priority arrays at various offsets
  - Whether unit is currently in production (boosted priority)
  - Random tiebreaker for equal scores
- Collects candidates at the highest score level, picks randomly

---

## 11. HouseClass Factory Indices

These store **type class array indices** (not pointers), with -1 meaning "none assigned."

| Offset | Field | RTTI Cases | Type Array |
|--------|-------|-----------|------------|
| 0x5650 | Primary_ForVehicles | 1 (UnitClass), 0x28 | g_UnitTypeClass_Array |
| 0x5654 | Primary_ForAircraft | 0xF (AircraftClass), 0x10 | g_AircraftTypeClass_Array |
| 0x5658 | Primary_ForInfantry | 2 (InfantryClass), 3 | g_InfantryTypeClass_Array |
| 0x564C | Primary_ForBuildings | 6 (BuildingClass), 7 | g_BuildingTypeClass_Array |

Set by AI choose functions (`HouseClass__AI_Choose_Unit` at `0x4FEA60`, etc.).
All initialized to -1 in `HouseClass__Constructor` at `0x4F54A0`.

---

## 12. Manager-Class Pointers on TechnoClass (verified)

`TechnoClass::Init_Managers` at `0x006F3F40` is the canonical attach site for every
manager subsystem. It runs during the TechnoClass-side of `Unlimbo` (so the manager
is constructed when the techno first appears on the map, NOT when it's produced from
a factory). The function inspects flags on `TechnoTypeClass` and allocates the
matching manager, storing the pointer at a fixed offset:

| Manager | TechnoClass byte offset | `int *` index | Gating condition (on TechnoTypeClass) |
|---|---|---|---|
| `CaptureManagerClass` | `+0x2BC` | `[0xAF]` | `TypeClass + 0x155 != 0` (MindControl) |
| `SpawnManagerClass` | `+0x2D0` | `[0xB4]` | `TypeClass + 0xD58 != 0` (Spawns count) |
| `SlaveManagerClass` | `+0x2D8` | `[0xB6]` | `TypeClass + 0xD40 != 0` (SlavesNumber) |
| `AirstrikeClass` | `+0x294` | `[0xA5]` | `TypeClass + 0x61C > 0` |
| `ParasiteClass` | `+0x69C` | `[0x1A7]` | `TypeClass + 0x159 != 0` (parasite weapon) |
| `TemporalClass` | `+0x274` | `[0x9D]` | `TypeClass + 0x15A != 0` (temporal) |

The `[index]` column shows how `int *param_2` indexing in `ExitObject_Main` etc. resolves
to the byte offset above. The dual notation is a recurring source of confusion (see
CLAUDE.md "param_1 pointer arithmetic" pitfall) — both columns describe the same field.

### SpawnManagerClass construction parameters

```c
new SpawnManagerClass(
    /* owner       */ techno_instance,
    /* count       */ TypeClass + 0xD58,   // SpawnsNumber=
    /* type_class  */ TypeClass + 0xD5C,   // Spawns= (TechnoType ptr)
    /* regen_rate  */ TypeClass + 0xD60,   // SpawnRegenRate=
    /* regen_delay */ TypeClass + 0xD64    // SpawnReloadRate=
)
```

The actual deploy-aircraft logic lives in `SpawnManagerClass::AI` (`0x006B7230`), a
per-tick state machine — there is **no `DeployAllSpawns` method**. Deploy is
event-driven on the carrier acquiring a target (`+0x68` on the manager). See the
standalone [SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](./SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md)
for the full 7-state-per-slot machine, vtable layout, and YR consumers (Aircraft
Carrier, Destroyer, Dreadnought, Boomer Sub, V3 Launcher, Meteor Shower SW).

---

## 12a. SlaveManager Trigger on Building Placement

**`SlaveManagerClass::DeployAllSlaves`** @ `0x006B0D60` — called from
**case 6 (Building)** of `ExitObject_Main`, after a building's `Unlimbo` succeeds.
The trigger field is read as `param_2[0xB6]` where `param_2` is the produced object typed
`int *` — i.e., the **`SlaveManager*` pointer at byte offset `0x2D8`** on the
produced TechnoClass instance (see §12 table).

```c
void SlaveManagerClass__DeployAllSlaves(SlaveManagerClass* this) {
    if (this->State == 0) {          // +0x5C
        this->State = 4;             // "deploying"
        this->Timer = 0x7FFFFFFF;    // +0x60, max int
        for (i = this->SlaveCount - 1; i >= 0; i--) {   // +0x48 = count, +0x3C = array ptr
            SlaveNode* node = this->Slaves[i];
            if (node->Status != 6 && node->SlavePtr != NULL) {
                node->SlavePtr->vtable[0x3D0]();   // Deploy
            }
        }
    }
}
```

### What this actually fires for

This is the **Slave Miner** deploying its slaves on placement — when a Slave Miner
building is placed and successfully unlimbos, its pre-instantiated slave units are
released. It is NOT a generic carrier/spawner mechanism, and it does NOT fire when a
War Factory ejects a unit. Aircraft Carriers, Dreadnoughts, etc. use a separate
SpawnManager mechanism (not traced in this report).

The internal layout (`+0x5C` state, `+0x60` timer, `+0x48` count, `+0x3C` array,
`vtable[0x3D0]` Deploy on each node, `state==0` gate, `state := 4`, `timer := MAX_INT`)
is a generic "manager class" template — but in this binary's xref graph, the consumer
at `0x006B0D60` is exclusively the Slave Miner. The Ghidra label
`SlaveManagerClass::DeployAllSlaves` reflects this. Earlier drafts of this doc named
the function `SpawnManagerClass::DeployAllSpawns` and described the trigger as "carrier
exits War Factory" — both are wrong.

> **Implementation note:** if you need to support Aircraft-Carrier-style spawned aircraft
> in a Rust port, do NOT use this function as the reference. Trace the actual carrier
> spawn-deploy path separately.

---

## 13. Production Pipeline Summary

```
Player clicks "Build Unit" → FactoryClass::StartProduction
  → FactoryClass::AI (per-tick) advances production timer
  → FactoryClass::IsComplete → true when Production_Value == 0x36

HouseClass::Place_Production (called by sidebar/AI)
  ├── Gets FactoryClass by production type:
  │   ├── RTTI 1/0x28 → Primary_ForVehicles / Primary_ForShips
  │   ├── RTTI 2/3    → Primary_ForAircraft
  │   ├── RTTI 6/7    → Primary_ForBuildings / Primary_ForDefenses
  │   └── RTTI 0xF/10 → Primary_ForInfantry
  │
  ├── Gets produced object: FactoryClass::GetObject
  │
  ├── If placement cell specified (buildings):
  │   └── Direct Unlimbo at cell → done
  │
  └── If auto-exit (units/infantry):
      ├── Find primary building: object->FindFactory (vtable 0x190)
      ├── Call BuildingClass::ExitObject (vtable 0x100)
      │   ├── GetDockCellForObject → exit cell
      │   ├── Apply ExitCoord offsets (barracks types only)
      │   ├── Unlimbo at final position
      │   ├── MoveTo(exit_cell or rally_target)
      │   ├── WF: piggybacked DriveLocomotion for Walk/Mech units
      │   └── AI: FUN_00500200 → scatter destination
      │
      │   (Building case 6: after Unlimbo, if Type has SlaveManager pointer at
      │   byte offset +0x2D8, calls SlaveManagerClass__DeployAllSlaves —
      │   Slave Miner releases its slaves. See §12.)
      │
      ├── FactoryClass::CompletedProduction
      ├── Play CreateUnitSound (if building type has it at +0xE74)
      └── Radar event + EVA voice (player only)
```

---

## 14. Key Addresses Reference

| Address | Function |
|---------|----------|
| 0x00443860 | `BuildingClass::SetRallyPoint` |
| 0x00443C60 | `BuildingClass::ExitObject_Main` |
| 0x0044B780 | `BuildingClass::MissionRepairAndProduce` (hospital/armory/cloning-vat state machine) |
| 0x0044D880 | `FUN_0044D880` — WF mission slot 26 (slave-deploy + WF vehicle-eject state machine) |
| 0x0044EFB0 | `BuildingClass::GetDockCellForObject` |
| 0x00447780 | `BuildingClass::GrandOpening` |
| 0x00451890 | `BuildingClass::CreateAnimForSlot` |
| 0x0045AEA0 | `LocomotionClass::QueryInterface_IPiggyback` |
| 0x004AF8E0 | `DriveLocomotionClass::Begin_Piggyback` |
| 0x004AF930 | `DriveLocomotionClass::End_Piggyback` |
| 0x004C6CB0 | Event dispatch (command handler, switch on event type) |
| 0x004C6780 | Event constructor (MEGAMISSION_F) |
| 0x004C9B20 | `FactoryClass::AI` (production tick) |
| 0x004CA1A0 | `FactoryClass::CompletedProduction` |
| 0x004FB0E0 | `HouseClass::Place_Production` |
| 0x004FBD80 | `HouseClass::GetPrimaryFactoryBuilding` |
| 0x004FBE40 | `HouseClass::Clear_Rally_Point` |
| 0x004FBF60 | `HouseClass::Set_Rally_Point_Cell` |
| 0x004810A0 | `CellClass::GetSubCell` |
| 0x00481180 | `CellClass::PlaceInfantryInCell` |
| 0x0048E480 | `CellClass::InitSubCellOffsets` |
| 0x00500200 | AI rally/scatter destination calculator |
| 0x00501AC0 | AI scatter position generator |
| 0x00509CD0 | `HouseClass::AI_GroundRallyPoint` |
| 0x00509E00 | `HouseClass::AI_NavalRallyPoint` |
| 0x0050CBF0 | `HouseClass::AI_FindBestRallyTarget` |
| 0x005F5C20 | `TechnoClass::vtable_0x190` (FindFactory relay) |
| 0x005F7900 | `TechnoTypeClass::FindFactory` (actual implementation) |
| 0x006B0D60 | `SlaveManagerClass::DeployAllSlaves` (called from ExitObject case 6, Building) |
| 0x006E6AB0 | Object ID packer (5-byte: 4 ID + 1 type tag) |
| 0x006E6E20 | Abstract ID resolver |
| 0x006E6F20 | Object ID resolver |
| 0x0070C610 | `TechnoClass::SetGhostCell` (writes +0x218) |
| 0x005F4EC0 | `ObjectClass::Unlimbo` (base map placement) |
| 0x006F6CA0 | `TechnoClass::Unlimbo` (vision, house tracking, facing) |
| 0x004D7170 | `FootClass::Unlimbo` (locomotor init, neighbor occupancy) |
| 0x00737BA0 | `UnitClass::Unlimbo` (body facing, SHP frame) |
| 0x00741970 | `TechnoClass::Set_Destination` (MoveTo, vtable 0x480) |
| 0x004D94B0 | `FootClass::Set_Destination_Internal` (writes NavCom +0x5A4) |
| 0x005B35E0 | `MissionClass::Queue_Mission` (SetMission, vtable 0x1E8) |
| 0x004DA0E0 | `FootClass::Enter_Destination` (dock/enter queue) |
| 0x004C9300 | `FacingClass::UpdateFacing` |
| 0x005657A0 | `MapClass::Get_CellClass` (Y*512+X indexing) |
| 0x0073F0A0 | `UnitClass::CanEnterCell` |
| 0x004C9B20 | `FactoryClass::AI` (production tick) |
| 0x004C9C70 | `FactoryClass::StartProduction` |
| 0x004C9EA0 | `FactoryClass::SetRate` |
| 0x00449540 | `BuildingClass::ClearBibArea` (WF bib clearing function) |
| 0x0044F640 | `BuildingClass::GetExitCoord` (vtable +0xB4 — WF door lepton coord) |
| 0x006F3F40 | `TechnoClass::Init_Managers` (allocates SpawnManager/SlaveManager/Capture/Parasite/Temporal/Airstrike during Unlimbo) |
| 0x006F4AB0 | `TechnoClass::Receive_Radio` (handles 0x18/0x19 tether handshake → field_0x418) |
| 0x006B6C90 | `SpawnManagerClass::Constructor` |
| 0x006B7230 | `SpawnManagerClass::AI` (per-tick spawn-deploy state machine — Aircraft Carrier, V3, etc.) |
| 0x006B7B90 | `SpawnManagerClass::SetTarget` (queues target into +0x6C) |
| 0x00447B20 | `BuildingClass::GetDockCoord` (vtable +0xA8 — dock/exit lepton coord; default = building center) |
| 0x005F6C80 | Default `GetDockCoord` fallback — just returns `GetCoord()` (vtable +0x48) |
| 0x005F6060 | Infantry-exit Z-coord adjuster (called by alt-path before Unlimbo) |
| 0x0065ADF0 | Comm_With slot check — returns 1 if any slot is NULL or matches arg, else 0 |
| 0x0065A750 | `RadioClass::Constructor` (allocates 1-slot Comm_With array, initializes count=1, entry[0]=NULL) |
| 0x00655990 | Calls `RadarClass::ComputeRadarMapBounds(&DAT_0087F8E4)` — fills the radar-map-bounds globals |
| 0x006E21E0 | TS-legacy "Resize Playable Map" — writes `DAT_0087F8E4..F0` from a MapClass-like struct |
| 0x0050DCC6 | `HouseClass::DetermineEdge` — uses `DAT_0087F8EC`/`F0` to score N/E/S/W map edges |
| 0x00523B10 | `InfantryTypeClass::CreateInstance` (clone creation) |

---

## 15. FactoryClass Struct & Production Math

### FactoryClass Struct Layout (0x74 = 116 bytes)

| Offset | Size | Type | Field | Init Value |
|--------|------|------|-------|-----------|
| 0x00 | 4 | ptr | vtable | vtable_FactoryClass |
| 0x04 | 4 | ptr | vtable_IPersist | |
| 0x08 | 4 | ptr | vtable_IRTTIInfo | |
| 0x0C | 4 | ptr | vtable_INoticeSink | |
| 0x10 | 4 | int | UniqueID | (from ctor) |
| 0x14 | 4 | int | AbstractFlags | (from ctor) |
| 0x18 | 4 | ? | (padding) | |
| 0x1C | 4 | int | RefCount | |
| 0x20 | 1 | bool | Dirty | |
| **0x24** | **4** | **int** | **Production_Value** | **0** |
| 0x28 | 1 | bool | Production_HasChanged | false |
| 0x2C | 4 | int | CDTimer_StartFrame | g_CurrentFrameCounter |
| 0x30 | 4 | int | CDTimer_Internal | (stack) |
| 0x34 | 4 | int | CDTimer_TimeLeft | 0 |
| 0x38 | 4 | int | CDTimer_Duration | 0 |
| **0x3C** | **4** | **int** | **Production_Step** | **1** |
| 0x40 | 4 | ptr | Queue_Vtable | PTR_FUN_007E8934 |
| 0x44 | 4 | ptr | Queue_Items | 0 |
| 0x48 | 4 | int | Queue_Capacity | 0 |
| 0x4C | 1 | bool | Queue_IsInit | true |
| 0x4D | 1 | bool | Queue_IsAlloc | false |
| 0x50 | 4 | int | Queue_Count | 0 |
| 0x54 | 4 | int | Queue_CapIncr | 10 |
| **0x58** | **4** | **ptr** | **Object** | **0 (null)** |
| 0x5C | 1 | bool | OnHold | false |
| 0x5D | 1 | bool | IsDifferent | false |
| **0x60** | **4** | **int** | **Balance** | **0** |
| 0x64 | 4 | int | OriginalBalance | 0 |
| **0x68** | **4** | **int** | **SpecialItem** | **-1** |
| **0x6C** | **4** | **ptr** | **Owner** | **0 (null)** |
| **0x70** | **1** | **bool** | **IsSuspended** | **true** |
| 0x71 | 1 | bool | IsManual | true |

### Production Tick (FactoryClass::AI at `0x004C9B20`)

```
if IsSuspended: return
if Object == null AND SpecialItem == 0: return
if Object != null AND Production_Value == 0x36: return  // already complete

// Check CDTimer expiry
time_remaining = CDTimerClass_GetTimeRemaining()
if time_remaining != 0 OR CDTimer_Duration == 0:
    Production_HasChanged = false
    return  // timer not expired

// --- TIMER EXPIRED: advance one step ---
Production_Value += Production_Step    // normally += 1
Production_HasChanged = true
IsDifferent = true

// Reset timer for next step
CDTimer_StartFrame = g_CurrentFrameCounter
CDTimer_TimeLeft = CDTimer_Duration

// Calculate cost per remaining step
remaining = 0x36 - Production_Value
cost_per_step = (remaining == 0) ? Balance : Balance / remaining
cost_per_step = min(cost_per_step, Balance)

// Can we afford it?
if House->GetAvailableMoney() < cost_per_step:
    OnHold = true
    Production_Value -= 1              // UNDO the step
else:
    House->Spend_Money(cost_per_step)
    OnHold = false
    Balance -= cost_per_step

// Check completion
if Production_Value == 0x36:
    IsSuspended = true
    CDTimer_Duration = 0
    CDTimer_TimeLeft = 0
    House->Spend_Money(Balance)        // spend any remainder
    Balance = 0
```

### Key Constants

- **Production completes at exactly `Production_Value == 0x36` (54).** This is hardcoded, not from INI.
- **54 steps** from 0 to completion. Each step advances by `Production_Step` (default 1).
- Sidebar progress bar: `ratio = Production_Value / 54.0`

### StartProduction (`0x004C9C70`)

1. If factory is idle (IsSuspended or Duration=0, queue empty):
   - `Production_Value = 0`, `IsSuspended = true`
   - `Object = TypeClass->CreateInstance()` (vtable 0x8C) — allocates the unit
   - `Balance = TypeClass->GetCost(Owner)` (vtable 0x84)
   - `Object->field_0x300 = Balance`
   - For AI: sets `Object->field_0x6CA = 1`
2. If factory is busy: adds to queue (max `Rules+0xF0` = MaximumQueuedObjects)

### SetRate (`0x004C9EA0`) — Starts the Timer

```
step_time = GetBuildStepTime(Object)   // total build time in frames
rate = step_time / 54                  // frames per step
rate = clamp(rate, 1, 255)

IsSuspended = false                    // UNPAUSE
CDTimer_Duration = rate
CDTimer_StartFrame = g_CurrentFrameCounter
CDTimer_TimeLeft = rate
```

### Build Time Calculation

```
base_time = TypeClass->Cost * BuildSpeed     // Rules+0x1748 (double)
time *= HouseTypeClass->BuildTimeBonus       // side-specific multiplier

// Power penalty (if PowerOutput < PowerDrain):
powerRatio = PowerOutput / PowerDrain
// Interpolated via MinLowPowerProductionSpeed, MaxLowPowerProductionSpeed,
// LowPowerPenaltyModifier (Rules+0x570, 0x574, 0x578)

// MultipleFactory bonus (Rules+0x57C):
if MultipleFactory > 0.0 AND factoryCount > 1:
    time *= MultipleFactory ^ (factoryCount - 1)
    // e.g., 3 WFs with MultipleFactory=0.7 → time *= 0.49
```

**Factory count tracked per RTTI per house:**

| RTTI | HouseClass offset |
|------|------------------|
| Infantry (1, 0x28) | +0x5380 (ground), +0x5388 (naval) |
| Unit (2, 3) | +0x5378 |
| Building (6, 7) | +0x5384 |
| Aircraft (0xF, 0x10) | +0x537C |

### Queue System

- **Queue_Items** at +0x44: dynamic array of TechnoTypeClass pointers
- **FIFO**: first queued type is built next after CompletedProduction
- **Max size**: `Rules+0xF0` (MaximumQueuedObjects)
- **StartNextQueued** (`0x004CA5A0`): after completion, shifts queue left by 1, starts building the front item

---

## 16. FindFactory — How the Engine Finds the Exit Building

### Call Chain

```
producedObject->vtable[0x190](needsPrimary, mustBeAvailable)  // on the PRODUCED OBJECT
  → relay at 0x005F5C20:
      type = this->GetType()           // vtable+0x88
      owner = this->GetOwnerHouse()    // vtable+0x3C
      return type->FindFactory(needsPrimary, mustBeAvailable, 0, owner)
  → TechnoTypeClass::FindFactory at 0x005F7900
```

All type classes (UnitTypeClass, InfantryTypeClass, AircraftTypeClass) use the **same implementation** at `0x005F7900`.

### FindFactory Implementation (`0x005F7900`)

```c
BuildingClass* FindFactory(TechnoTypeClass* type,
                           char needsPrimary,
                           char mustBePrimary,
                           char mustBeCanBuild,    // always 0 from relay
                           HouseClass* owner)
{
    BuildingClass* fallback = NULL;

    for (each building in owner->Buildings) {       // +0x6C array, +0x78 count
        if (building->InLimbo) continue;            // +0x81
        if (building->Type->Factory != type->WhatAmI()) continue;  // factory type mismatch
        if (mustBePrimary && !building->IsPrimary) continue;       // +0x660
        if (building->CurrentMission == 0x13) continue;            // selling
        if (building->Mission == 0x13) continue;                   // +0xB4
        if (!(type->NavalFlags & building->Type->NavalFlags)) continue;

        // Aircraft: check pad availability
        if (!needsPrimary && type->WhatAmI() == Aircraft) {
            if (building->HasPadAvailable()) return building;
            fallback = building;
        }
        // Naval units: prefer naval buildings
        else if (type->WhatAmI() == 0x28 && type->IsNaval) {
            if (building->Type->IsNaval) {
                if (building->IsIdle) return building;  // +0x3D3
                fallback = building;
            }
        }
        // Standard: prefer idle buildings
        else if (!building->Type->IsNaval) {
            if (building->IsIdle) return building;
            fallback = building;
        }
    }
    return fallback;
}
```

### Place_Production Call Patterns

```c
// First attempt: find primary factory
building = producedObject->FindFactory(0, 1);
// param1=0: normal search (aircraft uses pad check)
// param2=1: must have IsPrimary flag (BuildingClass+0x660)

if (building == NULL && rtti == Aircraft) {
    building = producedObject->FindFactory(1, 1);
    // param1=1: skip aircraft pad check (any valid factory)
}

if (building == NULL) return 0;  // production stays queued
```

If ExitObject fails: `"Failed to exit object from factory - refunding money\n"` → money refunded, production re-queued.

---

## 17. Cloning Vat Mechanism

### Storage

`HouseClass + 0xF8` — DynamicVectorClass of BuildingClass* (all buildings with `Cloning=true`):
- `+0xFC`: items array pointer
- `+0x108`: count

Updated when buildings change owner (`BuildingClass::ChangeOwner` at `0x00448260`).

### Trigger

In `BuildingClass::ExitObject` (0x00443C60), after infantry successfully exits a barracks:

```c
if (building.Type.Factory == 0x10 && !building.Type.Cloning) {
    InfantryTypeClass* infType = producedInfantry->GetType();
    HouseClass* owner = building.Owner;

    for (int i = 0; i < owner->CloningCount; i++) {
        BuildingClass* cloningBldg = owner->CloningBuildings[i];
        InfantryClass* clone = infType->CreateInstance(cloningBldg->Owner);
        cloningBldg->ExitObject(clone, InvalidCell);
    }
}
```

- `InfantryTypeClass::CreateInstance` at `0x00523B10`: allocates 0x6F0 bytes, constructs InfantryClass
- **ALL** cloning buildings get a clone (loop iterates entire vector, no early exit)
- Each clone exits from its own cloning building via that building's own exit cell logic

---

## 18. Unlimbo Call Chain — How Units Are Placed on the Map

### ObjectClass::Unlimbo (`0x005F4EC0`) — Base

1. Rejects if coordinates match the global invalid sentinel
2. Clears `InLimbo = false`, `Discovered = false`
3. Calls `CanEnterCell` (vtable 0x1AC) to verify cell is passable — if blocked, restores InLimbo and returns 0
4. Snaps position to cell grid, calls `Set_Location` (vtable 0x1B4)
5. Calls `Mark(1)` (vtable 0x124) to register in cell occupancy
6. Calls `DisplayClass::Submit_Object` to add to render list
7. Returns 1

### TechnoClass::Unlimbo (`0x006F6CA0`)

1. Calls `ObjectClass::Unlimbo`
2. Converts leptons → cell coords: `cell = lepton / 256` (signed)
3. Checks cell in playfield → stores at `+0x3D5`
4. Calls `HouseClass::Added_To_Game` to update house unit count
5. **Sets body facing**: `FacingClass::UpdateFacing(facing_byte << 8)`
6. **Sets turret facing** to `0x4000` (south initially)
7. Sets ROT: `(-ROT * 256 + 0x4000)` → `RateTimer::Set`
8. Sets scatter state: `+0x127 = 1`, `+0x128 = 0`
9. Calls `Commence()` to activate queued mission
10. Stores veteran rank from coord Z / `RulesClass + 0x16BC`

### FootClass::Unlimbo (`0x004D7170`)

1. Calls `TechnoClass::Unlimbo`
2. Initializes locomotion via TypeClass vtable calls
3. Invalidates cached path cost: `+0x178 = -1`
4. **Neighbor cell occupancy**: increments `CellClass + 0x122` for all 8 adjacent cells (pathfinding zone weight)
5. Stores cell index at `+0x157` (non-aircraft)
6. Copies speed type pair from `TypeClass + 0x2F0`
7. If a spawner/manager pointer is present on the FootClass: triggers via vtable `0x4E8`
   (this is a separate code path from the §12 SlaveManager call in ExitObject; the
   specific manager class invoked here was not traced in this report)

### UnitClass::Unlimbo (`0x00737BA0`)

1. Calls `FootClass::Unlimbo`
2. **Body facing**: `FacingClass::UpdateFacing(facing_byte << 8)` — sets initial direction
3. Deploy state: if `+0x3D2` (IsDeployed) and not in playfield → `+0x220 = 2`
4. **SHP frame init**: if not voxel (`TypeClass+0xE18 == 0` AND `+0xE19 == 0`):
   - Frame = `Random(0, 0x1D)`, `+0x10C = 1` (animating)
   - Else (voxel): frame = 0, `+0x10C = 0` (no anim)

---

## 19. MoveTo, SetMission, and Enter_Destination

### MoveTo — `TechnoClass::Set_Destination` at `0x00741970` (vtable 0x480)

Called as `unit->MoveTo(target, 1)`. The second parameter controls bridge-aware pathing.

1. Rejects if unit is parasited, frozen, or immobilized
2. Resolves target: if target is a BuildingClass, extracts its CellClass
3. Ultimately calls `FootClass::Set_Destination_Internal` (`0x004D94B0`):
   - Writes target to `FootClass + 0x5A4` (**NavCom**)
   - Calls `ILocomotion::Head_To_Coord` (locomotor vtable 0x48) with target coordinates

### SetMission — `MissionClass::Queue_Mission` at `0x005B35E0` (vtable 0x1E8)

Called as `unit->SetMission(mission_id, commence_flag)`.

1. Blocked if current mission is Sell (0x1C) or Harmless (0x13)
2. Sets `QueuedMission` at `+0x2D` = mission_id
3. If `commence_flag != 0`: calls `Commence()` to activate immediately

### Guard (2) vs Area Guard (0xB)

| | Guard (2) | Area Guard (0xB) |
|---|---|---|
| Behavior | Stand and defend | Patrol/scan larger area |
| Chase | Immediate threats only | Chases to guard range, returns |
| Used for | Player-produced units | AI-produced units |
| Typical combo | + `MoveTo(rally)` | + `Enter_Destination(rally)` |

### Enter_Destination — `FootClass::Enter_Destination` at `0x004DA0E0`

Manages a **queue of dock/enter targets** at `FootClass + 0x5B4` (separate from NavCom):

1. Appends target to destination queue
2. If NavCom is null AND mission is Guard(5): starts `SetDestination` to the first queued target
3. Queue supports multiple destinations (unlike NavCom which is single-target)

### Difference: MoveTo vs Enter_Destination

| | MoveTo (vtable 0x480) | Enter_Destination |
|---|---|---|
| Sets | NavCom (`+0x5A4`) + locomotor | Enter queue (`+0x5B4` array) |
| Purpose | Travel to point/cell | Enter/dock with building |
| Queue | Single target | Multiple targets |
| Used for | Rally points, move orders | Refineries, repair, transports |

---

## 20. Facing System

### FacingClass Struct (at `0x004C9300`)

| Offset | Size | Field |
|--------|------|-------|
| 0x00 | 4 | CurrentFacing (16-bit used) |
| 0x04 | 4 | DesiredFacing |
| 0x08 | 4 | TimerStart |
| 0x0C | 4 | TimerRemaining |
| 0x10 | 2 | TurnRate |
| 0x14 | 2 | ROT |

### 256-Unit Facing System

| Value | Direction |
|-------|-----------|
| 0 | North |
| 32 | NE |
| 64 | East |
| 96 | SE |
| 128 | South |
| 160 | SW |
| 192 | West |
| 224 | NW |

### Facing Conversion in ExitObject

```c
// atan2 returns radians, scaled to 16-bit facing
angle = atan2(building_center_y - exit_y, exit_x - building_center_x);
facing_16bit = ftol(angle);  // already scaled internally
facing_256 = (facing_16bit >> 7) + 1 >> 1 & 0xFF;  // round to 256 system
```

### UpdateFacing Behavior

When called with a new facing value and TurnRate == 0 (initial placement):
- Snaps both CurrentFacing and DesiredFacing to the new value instantly
- No interpolation — unit faces the direction immediately

When TurnRate > 0: smoothly interpolates from current to desired facing over time.

---

## 21. Cell System and CanEnterCell

### Cell Coordinate System

- **Cell array**: `MapClass + 0x13C`, indexed by `Y * 512 + X`
- **Max**: 512 × 512 = 262,144 cells
- **1 cell = 256 leptons**
- **Cell center**: `cell * 256 + 128`
- **Lepton → cell**: `cell = lepton / 256` (signed integer division)
- **Out-of-bounds**: returns static dummy cell at `0x00ABDC50` (prevents crashes)

### CanEnterCell Force Parameter

`GetDockCellForObject` calls `CanEnterCell(cell, -1, -1, 0, force)`.

| force | Terrain check | Friendly units |
|-------|--------------|----------------|
| **0** | Checks SpeedType/LandType table (blocks impassable terrain) | Returns FRIENDLY_OCCUPIED (6) — blocks |
| **1** | **Skips terrain check** (assumes buildable = passable) | Checks if friendly can scatter; allows passage if moving same direction |

Usage in GetDockCellForObject:
- **force=1**: barracks-specific exits + foundation edge scan (known passable cells)
- **force=0**: ExitList cells + custom target cells + WeaponsFactory+Naval cells

---

## 22. WF-Specific Exit Path in ExitObject

The non-naval WeaponsFactory path in ExitObject differs significantly from the barracks path:

### Barracks Path (non-WF, non-Hospital, non-Armory)
1. `GetDockCellForObject` → exit cell
2. `atan2(center - exit)` → facing
3. Adjust toward building edge
4. Apply ExitCoord offsets (for GDI/NOD/Yuri barracks only)
5. `Unlimbo` at final position
6. `SetMission(Guard)` + `MoveTo(exit_cell, 1)`
7. AI: rally scatter via `FUN_00500200`

### WeaponsFactory Path (WF, non-naval)
1. Clears existing GhostCell
2. **Multi-WF handoff**: if building is already in the WF "unload" mission, iterates all same-type buildings. If a sibling WF is in Guard with no factory assigned → temporarily transfers factory and calls sibling's ExitObject. This is **multi-WF queue balancing**.
3. Gets **factory door position** via `building->GetExitCoord()` (vtable 0xB4) — not GetDockCellForObject
4. `Unlimbo(exit_coord, facing=0x40)` — facing 0x40 = East
5. `SetFacing(0)` + `SetMoveTo(exit_coord)`
6. `SetMission(Guard)`
7. **Establishes radio link**: `RadioCommand(2, unit)` + `RadioCommand(0x18, unit)` ("RADIO_NEED_TO_MOVE")
8. **Transitions building to the WF mission slot 26** (the FUN_0044D880 state machine): `building->SetMission(mission_id, 0)`. The exact mission enum index for slot 26 is unconfirmed; earlier drafts of this doc named it "MissionUnload (0x10)".
9. The slot-26 handler (`FUN_0044D880`, §4) then handles door animation + bib clearing + locomotive piggybacking + drive-out monitoring
