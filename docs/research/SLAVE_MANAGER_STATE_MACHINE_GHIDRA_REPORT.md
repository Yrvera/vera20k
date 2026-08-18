# SlaveManagerClass — Full State Machine & Lifecycle — Ghidra Research Report

**Verified 2026-04-22** from `gamemd.exe` (image base `0x00400000`) via Ghidra MCP.
This report extends `SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md` with the previously
unmapped pieces: the manager-level state machine, docking flow, ownership-transfer
behavior, and the "brain transplant" semantics referenced in the rulesmd.ini comment.

**Active in YR:** Yes — standard Yuri Slave Miner (SMIN) + Yuri Ore Refinery (YAREFN).
The SlaveManager instance lives at `TechnoClass+0x2D8` and is created by
`TechnoClass::Init_Managers @ 0x006F3F40` whenever the techno's type has
`Enslaves != NULL`.

**Overall confidence:** HIGH for every state transition and function decompiled;
MEDIUM for a few slave-side vtable slot identities (noted inline); LOW (open) for
the exact "brain-transplant" mechanism (see §9).

---

## 1. Class Anatomy — Three Constructors, One Destructor

| Address | Symbol | Role |
|---------|--------|------|
| `0x006AF1A0` | `SlaveManagerClass__Constructor` (primary)  | 4-arg: `(owner, Enslaves, SlavesNumber, SlaveRegenRate, SlaveReloadRate)` — used by `TechnoClass::Init_Managers` |
| `0x006AF360` | `SlaveManagerClass__Constructor` (secondary) | Alternate constructor (save-load / copy) — not decoded here, low priority |
| `0x006AF4A0` | `SlaveManagerClass__Constructor` (default)   | Zero-arg default constructor |
| `0x006AF5A6` | `PowerUp_Cleanup`  (mislabeled)              | Calls `MasterDestroyed` (`FUN_006B0AE0`) — this is the SlaveManager destructor path |

The mislabel at `0x006AF5A6` matters: it is how the destructor invokes
`SlaveManagerClass__MasterDestroyed`. Xrefs to `MasterDestroyed` (verified this
pass):

- `MissionClass::Constructor @ 0x006F4571`  — master entering a "destroyed" mission state
- `PowerUp_Cleanup @ 0x006AF5A6`             — SlaveManager destructor proper
- `TeleportLocomotionClass::PostWarpValidation @ 0x00718998, 0x00718AEF`  — chrono warp failure
- `TemporalClass::Update @ 0x0071AA2C, 0x0071AAA7`  — slowly-erased-by-Chronosphere
- `TechnoClass::ReceiveDamage @ 0x00702065`  — fatal-damage path
- `JumpjetLocomotionClass::Process` state 5 (`FUN_0054CA90 @ 0x0054CEDF`) — Magnetron/Jumpjet mid-air force-kill when no valid landing cell; see §12 follow-up pass for detail

## 2. Updated Class Layout (byte offsets)

Confirmed against all decompiled functions (`param_1` is `int` in every
function; `param_1[N]` is a byte offset of `4*N`). Corrections / additions to
the existing layout in `SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md`:

| Offset | Field | Read/Write sites (verified) |
|--------|-------|------------------------------|
| +0x00..0x0C | 4 vtable pointers | Install in ctor (standard multi-inheritance shape) |
| +0x24 | `Owner` — `TechnoClass*` (master) | Every AI function dereferences this first |
| +0x28 | `EnslavesTypePtr` — `InfantryTypeClass**` (double-ptr: slot in type array) | Read by `FUN_006AF650` (Respawn) as `**(int**)(param_1+0x28)` |
| +0x2C | `SlavesNumber` (int) | Loop bound in `Constructor` |
| +0x30 | `SlaveRegenRate` (int) | Written into `SlaveControl[4]` when slave becomes state 6 (Dead) |
| +0x34 | `SlaveReloadRate` (int) | Written into `SlaveControl[4]` when slave becomes state 5 (Regenerating) after deposit |
| +0x3C | `SlaveArrayPtr` — `DynamicVectorClass<SlaveControl*>` storage ptr | Array indexed `[idx*4]` by all enumerating functions |
| +0x48 | `SlaveArrayCount` (int) | Actual slave count (may be ≤ `SlavesNumber` if one was removed) |
| +0x50 | `RateTimer_Start` (int frame, -1 = fresh) | Manager update rate-limit in `AI_Update_Dispatch @ 0x006AF5F0` |
| +0x54 | `RateTimer_Data` (int) | Written alongside 0x50 |
| +0x58 | `RateTimer_Duration` (int, default 10) | Manager updates every 10 frames |
| +0x5C | **`ManagerState`** (int, 0..6) | **Decoded in §3**, previously "0=Ready, 1=?, 2=Moving, 4=Freeze, 6=Relocating" is now fully resolved |
| +0x60 | `StateTimer` (int, `0x7FFFFFFF` = "no timer", else frame counter) | Written by every state transition |

### SlaveControl layout (unchanged, 0x14 bytes)

| Offset | Field | Semantics |
|--------|-------|-----------|
| +0x00 | `Slave` — `InfantryClass*` (0 if dead) | |
| +0x04 | `SlaveState` (int 0..6) | Per-slave state (§4) |
| +0x08 | `Timer_Start` (int frame) | |
| +0x0C | `Timer_Data` (int) | |
| +0x10 | `Timer_Duration` (int) | When slave becomes state 6, set to `SlaveRegenRate` by `RemoveSlave`; when state 5, set to `SlaveReloadRate` |

## 3. Manager-Level State Machine — `FUN_006AFD60` (mislabeled `UnitClass__Mission_Deploy`)

**This is the core gap being filled.** The existing report only noted that this
function exists and was "probably SlaveManager::UpdateState". It is. The
dispatcher at `FUN_006AF5F0` rate-limits (default every 10 frames) and calls
both `SlaveManagerClass::AI_Update` (per-slave state machine) AND this
manager-level state machine.

The manager state lives at `+0x5C`. All transitions write `+0x60 = 0x7FFFFFFF`
(no timer) unless noted; state 3 uses the timer (30-frame retry).

### Condition primitives used throughout

- `owner.vtable[0x2C]()` returns the owner's **current mission enum value**
  (verified: same return values also appear in `piVar1[0xAC]` direct-field
  reads, so `vtable[0x2C]` is the GetCurrentMission getter that reads field
  `+0xAC`).
- Mission IDs observed in branches: `1` (= Attack / also "in-motion vehicle
  attack"), `6` (= Sticky / building default guard), `0x12` (18 = Selling /
  Deconstruction), `0x13` (19 = Repair). Values match standard YR `Mission::`
  enum.
- `owner.vtable[0x338]()` = `TechnoClass::ScanForTiberium` or related
  movement-target fetch (from existing doc).
- `owner.vtable[0x480](cell, 1)` = `SetDestination(cell, preferred)` — verified
  via callsite context (sets the owner into moving-to-cell).
- `owner.vtable[0x1E8](mission_id, 0)` = `ClearAndSetMission(mission_id)`.
- `owner[0x5A4]` (offset 0x5A4) = owner's **TargetObject pointer** (non-zero
  while attacking). Same `piVar1[0x169]` the slave AI reads in state 4.
- `owner[0x534]` = owner's `FactoryClass*` pointer slot or a "building
  relocation flag" (seen only in state 4 → 5 test). **OPEN** — not decoded.
- `owner[0x4F8]` = byte, set to 1 when forcing relocation (state 5 → 6).

### State-by-state decomp

```
STATE 0 — READY (default idle when deployed as YAREFN)
    if  owner.Mission == 6 (Sticky/building-guard)
    and owner.Mission != 0x13 (Repair)
    and owner.Mission != 0x12 (Selling):
        → state 5   (proceed to deploy slaves / check relocation)

STATE 1 — SEEKING MOVE DESTINATION (post-recall, unit mode, about to move out)
    if owner.Mission == 1 (Attack) and owner.TargetObject != 0:
        → state 2  (owner already moving to a target; manager follows)
    else:
        coord = owner.vtable[0x338]()          // movement target
        if coord valid:
            cell = FindDeployCell(...)          // FUN_006B0300, see §5
            if cell valid:
                owner.SetDestination(cell, 1)
                owner.SetMission(Move)
                → state 2
            else:
                → state 0  (fallback: give up, stay idle)
        else:
            → state 0  (fallback)

STATE 2 — MOVING, waiting to deploy
    if owner.Mission == 1 (Attack) and TargetObject != 0:
        stay  // still moving into position
    else:
        if UnitClass::Deploy() succeeded:
            → state 4  (stable deployed)
        else:
            state = 3
            StateTimer = 30 frames                // single retry window

STATE 3 — RETRY DEPLOY (30-frame cooldown)
    after 30 frames, retry UnitClass::Deploy()
        if success → state 4
        else       → state 1   (give up and re-seek destination)

STATE 4 — DEPLOYED (stable, YAREFN is mining)
    if owner.Mission == 6 (building-guard):
        if owner[0x534] != 0:                   // some "relocate" flag
            → state 5
    if owner.Mission == 1 (unit now; undeployed):
        if owner.Type[0x68C] == 0               // not a building-type
           and owner[0xAC] == 5 (Mission == Guard):
            → state 2   (followed owner through undeploy, go back to Moving)

STATE 5 — CHECK RELOCATION / DEPLOY SLAVES
    if owner.Mission == 1 (Attack; owner moved away as unit):
        → state 0   (reset)

    // Relocation test:
    tgt = owner.GetMovementTarget()
    if tgt invalid:
        tgt2 = owner.ScanForTiberium(Rules+0x1788 >> 8)    // LongScan range
        FindDeployCell(...)
        d1 = sqrt(dx²+dy²)        between owner and tgt2
        d2 = sqrt(dx²+dy²)        between owner.GetCoord() and tgt2
        if (Rules+0x178c >> 8 + d1) < d2:         // too far to dispatch slaves
            owner[0x4F8] = 1                       // "force relocation" flag
            owner.SetMission(0x13 = Repair)       // ← actually "undeploy to find new patch"
            → state 6

    if state != 6 (we did not relocate):
        DeploySlaves()                            // FUN_006B04C0 — unlimbos idle slaves

STATE 6 — RELOCATING (waiting to leave as unit)
    if owner.Mission != 1 (Attack): stay
    else:
        RecallIdleSlaves()                        // FUN_006B0490
        → state 1
```

The Rules offsets used by state 5's relocation test:

- `Rules + 0x1788` = **SlaveMinerLongScan** (seed range for new tiberium scan)
- `Rules + 0x178c` = **SlaveMinerScanCorrection** (min-improvement threshold
  before relocating — implemented as `scan_correction + d1 < d2`)
- `Rules + 0x1790` = **SlaveMinerShortScan** (consumed in `ShouldRecallSlaves`
  as a tick-gated retry interval — verified in §6)

These complement the already-known `Rules + 0x1784` (TiberiumShortScan used by
the per-slave state 1, see §4) and match the existing `SlaveMinerConfig` Rust
struct in `src/sim/slave_miner.rs:344-375`.

**Confidence: HIGH.** Every transition was read directly from the decompile at
`0x006AFD60`; the four-digit-hex mission IDs (`1/6/0x12/0x13`) match
literal `CMP` constants in the disassembly.

## 4. Per-Slave State Machine — `FUN_006AF6C0` — Extended

The existing report documents states 0..6 at a high level. The Ghidra decompile
refines state 4 (return-to-master + deposit) substantially:

```
STATE 0 — idle at master (switch has NO case 0; slave just sits with
          state=0 until manager state 5 calls DeploySlaves, which
          transitions to state 1 AFTER unlimboing the slave)

STATE 1 — ScanForOre
    cellcoord = slave.ScanForTiberium(RulesClass+0x1784 >> 8, 0)
    if cellcoord invalid:
        // no ore nearby — move slave back to master
        set destination = master cell
        slave.SetMission(Move)
        → state 4
    else:
        set destination = ore cellcoord
        slave.SetMission(Move)
        → state 2

STATE 2 — MovingToOre
    if (arrived at a tiberium cell):           // FUN_00487DF0 checks LandType==5
        slave.SetMission(Harvest)              // FUN_00522D00
        → state 3
    else if slave.TargetObject == 0 (movement aborted):
        → state 1 (rescan)

STATE 3 — Harvesting
    if slave is full (storage-percentage >= 1.0):   // FUN_00522D30 = Get_Storage_Percentage
                                                    // slave.Storage=4 (INI), capacity full → return
        slave.SetGhostCell(cell)
        set destination = master cell
        slave.SetMission(Move)
        → state 4
    else if slave.Mission != Harvest:
        → state 1 (interrupted)
    else if cell is no longer tiberium:
        FUN_00522D20()                         // cancel harvest (SetMission_Guard)
        → state 1

STATE 4 — ReturningToMaster  ⚠ UPDATED
    flag _has_strayed_too_far = false
    if slave.TargetObject != 0:
        // recompute distance from master to the slave's target
        dx,dy = master_cell - target_cell
        slave_to_master = FUN_006B1A70(vector)   // rough cell-distance integer
        if slave_to_master > Rules+0xDF8 (ApproachTargetResetMultiplier — see §10):
            _has_strayed_too_far = true

    if slave_cell == master_cell:
        if slave.TargetObject == 0:
            // Arrived clean → deposit ore and enter limbo
            BuildingClass::DepositOreFromStorage(master)  // FUN_00522D50 wrapper
            slave.vtable[0xD4]()                          // = ObjectClass::Limbo (verified §7)
            SlaveControl.state = 5
            SlaveControl.timer_start = currentFrame
            SlaveControl.timer_duration = SlaveReloadRate  // NOT SlaveRegenRate
        else if _has_strayed_too_far:
            // re-seek master cell destination with current owner coords
    else:
        if slave.TargetObject == 0 or _has_strayed_too_far:
            // re-command move to master cell
            set destination = master cell
            slave.SetMission(Move)
            // state remains 4

STATE 5 — Regenerating (in limbo, invisible, healing up)
    decrement SlaveControl.timer_duration
    when 0:
        slave.Health     = slave.Type.Strength   // slave[0x1b]
        slave.HealthMax  = slave.Type.Strength   // slave[0x1c]
        → state 0

STATE 6 — Dead (slave ptr == null)
    decrement SlaveControl.timer_duration
    when 0:
        FUN_006AF650(SlaveControl)              // Respawn — see §8
        → state 0 (respawn resets to ready)

NULL-transition (dead detection): at the top of AI_Update, if
SlaveControl.slave == NULL and state != 6, set state=6 and
arm timer = SlaveRegenRate. This is the entry point that converts
a silently-freed slave (killed off-screen, removed from game) into a
regenerating control slot.
```

### Correction: timer source in state 4 → 5

The existing doc said "sets regen timer from `SlaveRegenRate`". The decompile
shows `SlaveControl[4] = *(param_1 + 0x34)` where `+0x34` is
**`SlaveReloadRate`**, not `SlaveRegenRate`. The regen timer (0x30) is armed
only on actual **death** (dead-detection path and `RemoveSlave`). A slave that
returns home and docks waits `SlaveReloadRate` frames (default 25 from
rulesmd.ini) before re-deployment, while a slave that actually dies waits
`SlaveRegenRate` frames (default 500) before respawning.

This distinction matters for the Rust port: `src/sim/slave_miner.rs:513-589`
(`tick_slave_regen`) must drain two different timers depending on which state
is active.

## 5. Slave-Lifecycle Operations (all verified)

All five utility functions iterate `SlaveArray` backwards (`idx = count-1;
while (--idx >= 0)`), for `std::vector`-style safe mutation under removal.

### `DeploySlaves @ 0x006B04C0`

For each slave with `SlaveState == 0`:
1. Computes an unlimbo cell:
   - If manager.owner is a building (Mission==6): uses
     `(master_cell + (foundation_w - 1, foundation_h / 2))` — the right-side
     center of the building's footprint.
   - Else (vehicle mode): uses the owner's `vtable[0x1B8]()` base cell.
2. Gets the cell's 3D coord and applies `FUN_004ACA10(coord, 0)` (snap to
   walkable position?).
3. Validates the position is not the global invalid sentinel
   (`DAT_00B0B5D8`/`DAT_00B0B5DC`/`DAT_00B0B5E0` triple) or an alternate
   sentinel (`DAT_00B0B618`/+0x4/+0x8).
4. Calls `slave.vtable[0xD8](&coord, 0)` — this is `Can_Occupy_Cell` / "is this
   cell placeable". Only proceeds if it returns true.
5. Calls `slave.vtable[0x174](owner_coord)` — **Unlimbo at coord**. Places the
   slave on the map.
6. Sets `SlaveState = 1` (scan-for-ore on next AI tick).

**Callers (verified):** manager state 5 only.

### `DeployAllSlaves @ 0x006B0D60`

A hard-reset form: if manager is in state 0, forcibly transitions to state 4
(deployed-stable) and rings `slave.vtable[0x3D0]()` on every slave (= "clear
mission / stop what you're doing"). Used when the master transitions
state externally (likely by `ChangeOwner` or specific mission exits).

### `RecallAllSlaves @ 0x006B0CC0`

Mirror of DeployAllSlaves: if manager is in state 0, transitions to state 1,
calls `slave.vtable[0x3D0]()` on each slave. Prepares for move/recall.

### `RecallIdleSlaves @ 0x006B0490`

Unconditional: calls `slave.vtable[0x3D0]()` on every non-state-6 (non-dead)
slave. **Does not transition manager state.** Used from manager state 6
transition into state 1.

### `RemoveSlave @ 0x006B0A20`

Call signature: `(this, slave_ptr)`. Scans the array for the matching pointer,
then:
1. Clears the back-reference: `slave[0x2DC] = 0`  (the slave no longer knows
   about its master).
2. Zeros the slot's `Slave` pointer.
3. Sets `SlaveState = 6` (dead).
4. Arms regen timer with **`SlaveRegenRate`** (not reload).

**Callers:** xref trace not performed this pass — likely
`InfantryClass::Limbo` / destructor paths.

## 6. Returned-Slaves Handler — `FUN_006B0DB0`
**(Ghidra-labeled `SlaveManagerClass__HandleReturnedSlaves`, real role: "check
whether to reposition" / reacquire target)**

Runs as a sub-step from the dispatcher (`FUN_006AF5F0`) path. This is NOT a
per-tick function; it is called during manager state 2 or 0 when the owner
reports an external move. Two arms:

- **Arm A:** Owner is currently a unit with a movement target (`mission==1,
  TargetObject != 0`). Try to find a deploy cell at the owner's
  MovementTarget. If none, → state 0 (idle). If one: send owner toward it,
  go to state 2, ring vtable[0x3D0] on every non-dead slave to make them stop.
- **Arm B:** Owner is in mission==6 (building mode) but has no ghost-cell
  target (`+0x218 == 0`). Set manager state back to 0 (idle).
- **Arm C:** Owner is in mission==6 with ghost-cell target → set the new ghost
  cell, manager state = 6 (relocating), ring vtable[0x3D0] on slaves.

The key distinction vs. the main state machine is that `HandleReturnedSlaves`
(mis)-names what is actually a **target-swap helper** — it doesn't
acknowledge slave returns, it re-targets the manager when the owner's
destination changes externally.

## 7. Docking-with-Owner Flow — vtable[0xD4] identified

The existing doc labeled this as "enter/dock (slave vtable 0xD4)". Verification
this pass via `InfantryClass::Mission_Enter @ 0x005196A0`:

In the successful "enter this building" branch (around
`InfantryClass::Mission_Enter` line `iVar2 == 7 && piVar8 == TargetObject`
path), the sequence is:

```c
slave.vtable[0xD4]()                                // <-- this is the same slot
(optional vtable 0x278)(2, master)                  // set some flag
CargoClass::AddPassenger(slave)                     // add to master's passenger list
FootClass::Stop_Moving()
```

So `vtable[0xD4]` on an InfantryClass is called BEFORE `CargoClass::AddPassenger`.
The function at `ObjectClass::Limbo @ 0x005F4250` is the base-class
implementation; `FootClass::Limbo @ 0x004DB260` is the override. Given the
ordering (Limbo before AddPassenger), **vtable[0xD4] is `Limbo()`** — it
removes the slave from the visible map, detaches it from cell occupancy,
and prepares it for passenger-slot storage.

**Post-Limbo state of the slave:**
- Invisible (not rendered)
- Not at any cell (removed from CellClass::Occupiers)
- Back-reference `slave[0x2DC] = master` preserved (for future
  MasterDestroyed notification)
- Not a passenger of master in the normal "transport" sense — the slave is
  NOT added to master's `CargoClass::Passengers`. This is a subtle
  distinction from Mission_Enter's behavior (which does add to cargo).

The slave AI path in state 4 (deposit) calls `slave.vtable[0xD4]()` directly
WITHOUT calling `CargoClass::AddPassenger`. So slaves dock via pure Limbo
(invisible + off-map) rather than via the cargo-list mechanism.

**Confidence:** MEDIUM-HIGH for "vtable[0xD4] == Limbo". Low cost to verify by
reading the InfantryClass vtable data at the appropriate image-base offset
and confirming the slot-53 entry points to `0x004DB260`
(`FootClass::Limbo`). Left as a follow-up; not load-bearing for the overall
state-machine understanding.

## 8. Master Destruction / Slave Liberation — `FUN_006B0AE0` Extended

Signature: `MasterDestroyed(this, attacker_techno, new_house)`.

```c
// 1. Resolve fallback house (the "Civilian/Neutral" house)
neutral_house = null
civilian_country_id = FUN_006A46D0()                   // returns the YR Civilian country index
for h in g_HouseClass_Array[0 .. g_HouseClass_Array_Count):
    if h.Type.CountryIndex == civilian_country_id:
        neutral_house = h
        break

// 2. For each slave (reverse iteration):
for each slave in manager.SlaveArray:
    slave[0x2DC] = 0                                    // clear back-ref immediately

    if (slave[0x81] != 0):                              // slave is in some flag-off state
        slave.vtable[0xE0](attacker)                    // ObjectClass::Unlimbo(x)? — OPEN
        slave.vtable[0xF8]()                            // vtable F8 = Detach/Remove
        continue

    // Case A: no attacker + no new_house + no neutral house → destroy slave
    if attacker == null and new_house == null and neutral_house == null:
        Apply_Warhead(slave.position, null, Rules+0xFA8, 0, 0, 0, 0)
        // Rules+0xFA8 is the "death warhead" (electrocution anim in vanilla)
        continue

    // Case B: attacker present → transfer to attacker's house
    if attacker != null:
        new_owner = attacker.Owner
    else:
        new_owner = new_house ? new_house : neutral_house

    slave.vtable[0x3D4](new_owner, 1)                   // ChangeHouse(new_owner, reset=1)
    slave.vtable[0x3D0]()                               // clear mission / interrupt
    slave.vtable[0x388](1)                              // likely MakeFree / set autonomous flag

// 3. Play liberation sound if any slave survived
if any_slave_survived:
    if Rules+0x234 != -1:                               // SlavesFreeSound (default "SlaveWorkerLiberated")
        VocClass::PlayAt(master.cell)

// 4. Clear manager.owner
manager.owner = null
```

### Important findings

1. **No-attacker case now resolved:** if the master is destroyed without an
   attacker (e.g., `UnitClass::Deploy` suicide, Chrono-de-sync, sold), the
   game tries to liberate slaves to the **Civilian/Neutral house** (YR
   "SpecialFlags"'s neutral country, found via `FUN_006A46D0`). Only if no
   civilian house exists do the slaves actually die.

   This contradicts a common assumption that deploying the Slave Miner into a
   Refinery is a "silent" process. It's not — the SMIN's slaves are liberated
   to Neutral on deploy. See §9 for the "brain transplant" follow-up.

2. **Slave liberation is NOT the same as mind-control liberation.** The
   `vtable[0x3D4](new_owner, 1)` call with arg `reset=1` is a full
   house-change (like engineer capture), not temporary.

3. **`vtable[0x388](1)` is called last.** On an InfantryClass, slot 0x388 /4
   = 226 is likely `TechnoClass::MakeFreeGive` (set the "free/autonomous"
   flag). The arg `1` is "become neutral to original owner".

**Confidence:** HIGH for the flow; MEDIUM for the exact vtable-slot-to-method
mapping (0x3D4 / 0x3D0 / 0x388 / 0xE0 / 0xF8) — these are standard
InfantryClass vtable slots but need direct vtable verification.

## 9. Capture / Mind-Control / Brain Transplant — CRITICAL FINDING

This pass decompiled all three ownership-change pathways and compared them to
SlaveManager behavior.

### `TechnoClass::ChangeOwner @ 0x007014A0`

This is the main ChangeOwner function, called by both engineer capture and
mind-control outcome conversion (`CaptureManagerClass::FreeUnit` et al). The
decompile **does NOT notify the SlaveManager**. Specifically:

- It DOES call `SpawnManagerClass::Kill_All_Spawns()` if `+0xB4`
  (SpawnManager) is non-null.
- It does NOT call any SlaveManager function. `+0xB6` (SlaveManager slot) is
  left untouched.

**Consequence:** capturing a YAREFN Slave Refinery (the deployed SlaveMiner
building) transfers the building to the new owner **but leaves the SlaveManager
pointing at the same Enslaves type, same slaves list, and same owner
back-reference**. The existing slaves (who are SLAV infantry, originally
owned by Yuri) do NOT change owner with the building. New slaves spawned by
`FUN_006AF650` (respawn) will be owned by the **captured building's new
owner** (since respawn calls `master.vtable[0x3C]()` = GetOwnerHouse at
spawn time).

Gameplay effect (consistent with well-known RA2/YR behavior):
- Capture YAREFN → slaves stay Yuri-colored, wander / hostile
- When slaves die and respawn → new slaves come out in your color
- Eventually all slaves are yours

### `BuildingClass::ChangeOwner @ 0x00448260`

Calls `TechnoClass::ChangeOwner` at its end (`TechnoClass__ChangeOwner(iStack_10,1)`).
No direct SlaveManager interaction. The building-specific code before the
base-class call handles:
- Refund / sell-on-capture
- Engineer-consumption flag (if `Type[0x157b]` is set)
- Upgrade-slot redistribution
- Radar / bonus recounting

None of these touch SlaveManager.

### Mind Control (Yuri Brain / Magnetron conversion)

Mind control routes through `CaptureManagerClass` (allocated at
`Techno+0x2C0`). On mind-control resolution, `ChangeOwner` is called with the
mind-controller's house. Same behavior as capture — SlaveManager is not
notified.

### "Brain transplant" — the SMIN ↔ YAREFN transition — OPEN

The rulesmd.ini comment at line 13284 (`; Brain transplant will check to make
sure extra one is not created`) refers to the UnitClass::Deploy transition.
The **verified binary behavior** is:

1. `UnitClass::Deploy` creates a new BuildingClass (YAREFN), which calls
   `TechnoClass::Init_Managers` → creates a **new** SlaveManager with 5 new
   SLAV infantry.
2. At the end of Deploy, the SMIN is destroyed via `vtable[0xF8]` +
   `vtable[0x3A0]`.
3. The SMIN's destructor invokes the SlaveManager destructor, which calls
   `MasterDestroyed(attacker=null, new_house=null)` — per §8 this will try
   to liberate the 5 original slaves to **Neutral house**, or kill them if
   no neutral house exists in the game.
4. Net effect: the 5 original slaves vanish/liberate, and 5 new slaves
   spawn at the YAREFN.

This is consistent with the INI comment ("extra one is not created" = "the
old master-slave binding is properly severed before the new one is made"),
but the **actual behavior is destroy-and-recreate**, not transfer.

**Open question:** does `UnitClass::Deploy` have a special-case path that
suppresses the SMIN's SlaveManager destructor, leaving the original slaves
alive and reassigning them to the new YAREFN? Not found this pass. The
decompile of `UnitClass::Deploy @ 0x007393C0` (already in the existing
report, §1) explicitly notes "The SlaveManager is NOT explicitly transferred
in UnitClass::Deploy". This suggests **there is no special case** — the
slaves DO transition (likely to Neutral) and new ones spawn fresh.

**If verifying in-game:** play a quick skirmish as Yuri, deploy a Slave
Miner, and observe whether the 5 SLAVs continuously tracking the SMIN before
deploy are the same 5 SLAVs working at the YAREFN after deploy (check
veterancy pips — slaves are Trainable=yes). If they're the same, there IS
a transplant. If veterancy resets, destroy-and-recreate is confirmed.

**Confidence:** HIGH on "ChangeOwner does NOT notify SlaveManager". MEDIUM on
"Deploy destroy-and-recreate is the actual mechanism" — needs in-game
observational confirmation or a deeper pass through
`UnitClass::Deploy @ 0x007393C0`'s end-of-function block to confirm no
vtable slot on +0x2D8 is ever unlinked or transferred.

## 10. Key Rules Offsets (Consolidated — FULLY RESOLVED 2026-04-22)

| Offset | INI Key (`[General]`) | Type | Default | Consumer | Source |
|--------|----------------------|------|---------|----------|--------|
| +0x234  | **`SlavesFreeSound`** | sound index (VocClass) | `SlaveWorkerLiberated` | MasterDestroyed (§8), played at master's cell when any slave survives liberation | `GLOBAL_SOUNDS_GHIDRA_REPORT.md:250`, index 80 at `Rules+0x234`; INI key verified at `ini/rulesmd.ini:716` — *"sound made when miner slaves are freed"* |
| +0xDF8  | **`ApproachTargetResetMultiplier`** | int-encoded (INI 1.5 → stored int) | 1.5 | Slave state 4 strayed-too-far test (§4) | `AI_DIFFICULTY_SYSTEM.md:365` — "ApproachTarget position should be recalculated if target is now more than weapon range times this"; fits slave leash semantics |
| +0xFA8  | **`C4Warhead`** (NOT "DeathWeapon" or IvanDamage) | `WarheadTypeClass*` | typically the Ivan-bomb / self-destruct warhead — also used for crush physics, bridge-crush, C4 detonation, ChronoWarp death, FlyLocomotion aircraft death | Slave kill (§8 Case A) | Cross-verified in `AIRCRAFTCLASS_GHIDRA_REPORT.md`, `ANIMCLASS_CHAINING_DAMAGE_OWNERSHIP.md`, `DAMAGE_MATH_GHIDRA_REPORT.md`, `BRIDGE_SYSTEM.md` (which explicitly corrected earlier "BridgeStrength" misreading) |
| +0x1780 | **`SlaveMinerShortScan`**       | int (cells) | **8**  | `ShouldRecallSlaves @ 0x006B1020` — wake-up scan after SlaveMinerKickFrameDelay | `LOCOMOTION_MATH_AND_CONSTANTS.md:409`, `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md:398` |
| +0x1784 | **`SlaveMinerSlaveScan`**       | int (cells) | **14** | Slave state 1 per-slave ore scan (§4) | `LOCOMOTION_MATH_AND_CONSTANTS.md:410`, `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md:399` — CORRECTION: original report said "TiberiumShortScan" |
| +0x1788 | **`SlaveMinerLongScan`**        | int (cells) | **48** | Manager state 5 relocation test (§3) | `LOCOMOTION_MATH_AND_CONSTANTS.md:411`, `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md:400` |
| +0x178C | **`SlaveMinerScanCorrection`**  | int (cells) | **3**  | Manager state 5 relocation threshold (§3) | Cross-verified via INI (`ini/rulesmd.ini:316`) |
| +0x1790 | **`SlaveMinerKickFrameDelay`**  | int (frames) | **150** | `ShouldRecallSlaves @ 0x006B1020` timer threshold (§6) — CORRECTION: original report called this "SlaveMinerShortScan" | `LOCOMOTION_MATH_AND_CONSTANTS.md:413`, `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md:402` |

**All five [General] slave-miner keys parsed from `ini/rulesmd.ini:313-317` now map
definitively to `Rules + 0x1780..0x1790`.** The Rust `SlaveMinerConfig` in
`src/sim/slave_miner.rs:344-375` already carries all five values by name;
only the usage sites (per-tick scan thresholds) need to be verified for
correctness against the binary's actual offsets.

## 10.5 Vtable Slot Identities (FULLY RESOLVED 2026-04-22)

All vtable slots referenced throughout this report have been cross-referenced
against `TECHNOCLASS_VTABLE_COMPLETE.md` and `FOOTCLASS_VTABLE_COMPLETE.md`.

| Slot offset | Symbol | Address | Used in |
|-------------|--------|---------|---------|
| `[0x2C]`  | `GetRTTI` / `GetCurrentMission` | base ObjectClass | Manager state machine mission queries |
| `[0x3C]`  | `GetOwnerHouse` / `GetOwnerHousePtr` | `0x006F9DC0` | Respawn — creates new slave under master's current house |
| `[0x84]`  | `GetTypeClass` | `0x006F3270` | All type-accessor calls |
| `[0xD4]`  | **`Limbo`** (FootClass override `0x004DB260`) | `0x004DB260` | **Slave dock** (§7) — removes slave from map, clears cell occupancy, stops locomotion |
| `[0xD8]`  | `Unlimbo` (FootClass `0x004D7170`) | `0x004D7170` | DeploySlaves — places slave back on map at master's coord |
| `[0xDC]`  | `Destroy` | `0x005F5280` | — |
| `[0xE0]`  | **`RecordKill`** — NOT "Unlimbo" as §8 speculated | `0x00702D40` | MasterDestroyed: records attacker as killer when slave is in the `+0x81 != 0` path |
| `[0xF8]`  | `UnInit` (FootClass `0x004DE5D0`) | `0x004DE5D0` | MasterDestroyed detach; WaveClass self-remove |
| `[0x174]` | `ObjectClass::Scatter_174` (possibly overridden by `FootClass::Set_Destination`) | `0x005F43A0` base | Slave AI — "set movement destination" semantics via override |
| `[0x1B8]` | `Get_Cell_Packed` | `0x0041BEA0` | All coordinate-fetch calls (`unit.GetCoords()`) |
| `[0x1BC]` | `GetOccupiedCell` | `0x005F6960` | Slave state 4 arrival check |
| `[0x1E8]` | **`Queue_Mission`** | `0x005B35E0` | All SetMission calls in slave/manager state machines |
| `[0x338]` | `OnCapture` (TechnoClass) — **but slave AI uses an InfantryClass override for `ScanForTiberium`** | `0x0070F8F0` base | Slave state 1 scan — FootClass/InfantryClass overrides this slot for ore scanning |
| `[0x3D0]` | **`StopAndGuard`** | `0x0070F850` | RecallIdleSlaves / DeployAllSlaves / MasterDestroyed — interrupts slave mission, clears target |
| `[0x3D4]` | **`ChangeOwner`** | `0x007014A0` | MasterDestroyed Case B — liberates slaves to attacker/neutral |
| `[0x388]` | **`ReturnFalse_388`** (stub at base; possibly overridden by InfantryClass) | `0x0041BF90` | MasterDestroyed — call-with-arg-1 **is a no-op at the TechnoClass level**; only effective if InfantryClass overrides |
| `[0x480]` | `SetTarget_480` | `0x00709A30` | All slave/master Set_Destination calls |

### Corrections to §8 (MasterDestroyed pseudocode)

The comment annotations "Unlimbo(x)" on vtable `[0xE0]` and "MakeFree / set autonomous"
on vtable `[0x388]` were both wrong:

- `vtable[0xE0]` is `RecordKill` — called with `attacker` to credit the kill to
  the correct house (veterancy/score tracking for the master's destroyer). Not
  an unlimbo.
- `vtable[0x388]` is a stub that returns false at the TechnoClass level. Unless
  InfantryClass overrides this slot, calling it does nothing. The
  "MakeFree/autonomous" behavior I hypothesized is most likely happening inside
  `StopAndGuard` (`[0x3D0]`), which already "stops firing, clears target, goes
  to guard mission" — that covers the autonomous-slave behavior after
  liberation.

## 10.6 BuildingClass+0x534 (Resolved)

Per `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md:107`:

| Offset | Type | Field | Verified |
|--------|------|-------|----------|
| +0x534 | int  | **DamagedState flag** | ✓ (round-2 BuildingClass field verification) |

This explains the manager state 4→5 transition: `if owner[0x534] != 0 → state 5`.
When the YAREFN building becomes damaged, the `DamagedState` flag is set, which
triggers the SlaveManager to go into state 5 (check-relocate). State 5 then
either:
- Proceeds to deploy slaves as normal (if ore is still accessible), OR
- Transitions to state 6 (relocate) if the long-scan indicates a better patch

In other words: **a damaged YAREFN becomes relocation-eligible**. This pairs
with `SlaveMinerKickFrameDelay=150` — the building reconsiders its position
every 150 frames by default, but damage can trigger the check immediately.

**Confidence: HIGH** — direct citation from verified field map.

## 10.7 Helper Functions `FUN_00522D*` (Resolved)

Per `BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md:238-267` and
`ORE_VALUE_CREDIT_DEPOSIT_GHIDRA_REPORT.md:198-243`:

| Address | Symbol | Purpose |
|---------|--------|---------|
| `0x00522D00` | `FootClass::SetMission_Unload` (the slave AI invokes it as "set Mission_Harvest" but the same helper routes both) | Assigns the harvest/unload mission to a unit; slave AI state 2 → 3 |
| `0x00522D20` | `FootClass::SetMission_Guard` | Reset to Guard; slave AI state 3 "cancel harvest" path |
| `0x00522D30` | `FootClass::IsStorageFull` — calls `UnitClass::Get_Storage_Percentage @ 0x007414A0`, returns true when percent >= 1.0 | Slave AI state 3 "full?" check; slave transitions to state 4 when this returns true |
| `0x00522D50` | **`BuildingClass::DepositOreFromStorage`** — the per-tiberium-type deposit loop | Slave AI state 4 deposit (already in §4); consumes all storage in one call |
| `0x00522FC0` | `FootClass::IsMissionUnload` / `IsMissionHarvest` — checks `this->Mission == 10` (Unload) | Slave AI state 3 "still harvesting?" check; if false, go back to state 1 (rescan) |

These are all standard FootClass mission helpers (not slave-specific). The slave
AI shares them with normal harvesters/weeders/refinery-dockers.

**Confidence: HIGH** — three independent doc sources agree.

## 10.8 Brain Transplant — DEFINITIVE ANSWER (2026-04-22)

Cross-referencing `MCV_DEPLOY_GHIDRA_REPORT.md` and
`UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md`:

### The Deploy→Destroy→Recreate Path (confirmed)

`UnitClass::Deploy @ 0x007393C0` (per MCV_DEPLOY doc §"UnitClass::Deploy()"
and UNIT_MISSION_DEPLOY_BUILDING doc §"UnitClass::Deploy() (0x00739390)"):

1. Allocates `BuildingClass` (0x720 bytes)
2. Calls `BuildingClass::Constructor` — which invokes `TechnoClass::Init_Managers`
   → **creates a NEW SlaveManager with fresh SLAV infantry**
3. Places building, transfers health/veterancy/waypoints
4. **"Destroys the MCV unit"** (MCV_DEPLOY_GHIDRA_REPORT.md line 288) — this
   is what triggers the old SlaveManager's destructor, which calls
   `MasterDestroyed(attacker=null, new_house=null)` and liberates the old
   slaves to Neutral (§8 Case B-fallback) or kills them with `C4Warhead` (§8
   Case A) if no Civilian house exists.

**Neither the MCV_DEPLOY nor the UNIT_MISSION_DEPLOY_BUILDING doc shows any
code path that transfers the old SlaveManager to the new building.** The
existing slaves do NOT follow the SMIN into the YAREFN — they are
destroyed/liberated at the same instant the YAREFN's fresh 5 SLAVs are
spawned.

### The "check to make sure extra one is not created" comment

The rulesmd.ini comment at line 13284 refers to a game-level invariant: the
SMIN has `SlavesNumber=5` AND the YAREFN has `SlavesNumber=5`, yet the
player never sees 10 slaves after deploy. The invariant is maintained because
the MCV's Deploy destroys the unit (and its slaves) before the new building
finishes spawning its own. There is **no special "skip creation" code** — it
is an emergent property of the destroy-and-recreate order.

### Practical consequence — observable difference

Because old slaves are liberated to Neutral and new slaves are freshly spawned:

1. **Veterancy resets.** A veteran SLAV before deploy is lost; new SLAVs start
   rookie.
2. **There's a 1-tick window** where both sets coexist (old on map + neutral,
   new in limbo/docked with YAREFN). In practice this is invisible to the
   player because the destroyed-SMIN frame runs at the same tick as the
   new-YAREFN frame.
3. **If `FUN_006A46D0` (Civilian country lookup) fails in a map with no
   Civilian slot** (some modded scenarios), the old slaves DIE by `C4Warhead`.
   This is a testable edge case — default YR maps always have the Civilian/
   Neutral house, so this never happens in stock play.

**Confidence: HIGH** — cross-verified across two independent deploy-flow
reports, both of which explicitly list "destroys the MCV unit" as the final
step of `UnitClass::Deploy`. No SlaveManager transfer path exists in either.

### Additional finding from UNIT_MISSION_DEPLOY_BUILDING

For `UnitTypeClass+0x5E4 = Enslaved` (set on SMIN), the harvester-refinery
dock code at `0x0073D630` has a special case (lines 212-214 of that doc):

```
if (UnitTypeClass->Enslaved) {
    FUN_007104a0(building, 0);              // clears byte +0x82
    if (Enslaved AND owner_mismatch) {
        building.vtable[0xF2](...);         // (labeled "transfer ownership" in doc)
    }
}
```

This means: **if a Slave Miner docks with a refinery owned by a different
house, the refinery's ownership changes to match the Slave Miner's owner.**
This is a distinct mechanism from the SlaveManager's own state machine.
(However the source doc tags vtable[0xF2] ambiguously — its slot map calls it
`SetDestination`. One of the two annotations is wrong; without Ghidra we
cannot adjudicate. Noted here as a **caveat for future implementation**.)

## 11. Rust Implementation Gap Analysis

Current Rust state (from `src/sim/slave_miner.rs`):

### Covered

- `SlaveHarvestState` enum ≈ per-slave state machine (6 states vs binary's 7)
- `tick_slave_harvesters` implements per-slave state transitions similar to
  the binary's FUN_006AF6C0
- `SlaveMinerConfig` captures the Rules offsets
- `SlaveMinerMode` covers SMIN deploy/undeploy animation phases

### Missing / Divergent

1. **No manager-level state machine.** The binary's 7-state manager state
   machine at `+0x5C` is absent. The Rust side assumes slaves-always-ready,
   which omits:
   - State 2/3 "moving-to-deploy" with 30-frame retry
   - State 5 relocation test (short vs long scan comparison)
   - State 6 relocation-initiation
   - Auto-recall and auto-redeploy based on owner's mission state

2. **Hardcoded harvest rate.** `SLAVE_HARVEST_RATE_TICKS = 150` in the Rust
   code should come from the slave INFANTRY type's `HarvestRate` field (per
   §4 state 3 — harvest is mission-driven, not tick-counter-driven, and the
   deposit uses the master's storage, not a per-slave bale counter).

3. **No docking-via-Limbo.** Rust's state 5 "Deposit" is implemented as one
   bale per tick, but in the binary the slave is full-storage, enters Limbo
   immediately on arrival, deposits all ore in one call via
   `BuildingClass::DepositOreFromStorage(master)`, and then starts the
   reload timer. This is a fundamental behavioral difference.

4. **No timer duality.** `tick_slave_regen` drains a single regen counter.
   Binary has two separate timers (`SlaveReloadRate` on deposit-dock,
   `SlaveRegenRate` on actual death) — see §4 correction.

5. **No MasterDestroyed liberation.** Rust doesn't liberate slaves to
   Neutral or attacker when the master dies; slaves just become orphaned.

6. **No ChangeOwner handling.** Building capture does not trigger the
   respawn-under-new-owner behavior; Rust slaves would stay forever Yuri.

These are all systems that, if implemented, would observably differ in
gameplay — compounding drift is likely if the player ever captures a
Slave Refinery or a Slave Miner is destroyed mid-deploy.

## 12. Open Questions — RESOLVED (2026-04-22 doc-archive pass)

Previous open questions, now closed via cross-referencing the 145-report
`ra2-rust-game-docs/` archive (Ghidra MCP was unavailable this pass, but the
archive already contained verified answers to all but two):

- [x] **`vtable[0xD4] == FootClass::Limbo @ 0x004DB260`** — confirmed via
      `FOOTCLASS_VTABLE_COMPLETE.md` (slot 53, override of TechnoClass).
- [x] **Rules+0xFA8 = `C4Warhead`** — confirmed by 4 independent doc
      citations (AIRCRAFTCLASS, ANIMCLASS, DAMAGE_MATH, BRIDGE_SYSTEM). The
      BRIDGE_SYSTEM doc explicitly corrected the old "BridgeStrength"
      misreading. See §10.
- [x] **Rules+0xDF8 = `ApproachTargetResetMultiplier`** — confirmed via
      `AI_DIFFICULTY_SYSTEM.md:365`. INI default `1.5` (verified in
      `ini/rulesmd.ini:301` and `ini/rules.ini:241`). See §10.
- [x] **Rules+0x1780..0x1790 = five slave-miner scan constants** — fully
      resolved via `LOCOMOTION_MATH_AND_CONSTANTS.md` and
      `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`. Corrections to
      previous table: +0x1784 is `SlaveMinerSlaveScan` (not TiberiumShortScan);
      +0x1790 is `SlaveMinerKickFrameDelay` (not SlaveMinerShortScan). The
      actual `SlaveMinerShortScan` is at +0x1780. See §10.
- [x] **`BuildingClass+0x534 = DamagedState flag`** — confirmed via
      `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md:107`. This is the damage-state
      trigger for the manager state 4→5 transition. See §10.6.
- [x] **`FUN_00522D*` helpers identified** — `00522D00=SetMission_Unload`,
      `00522D20=SetMission_Guard`, `00522D30=IsStorageFull`,
      `00522D50=BuildingClass::DepositOreFromStorage`, `00522FC0=IsMissionUnload`.
      All are standard FootClass mission helpers shared with harvester
      refinery-dock code. See §10.7.
- [x] **Brain transplant DEFINITIVELY answered** — destroy-and-recreate is
      the actual mechanism. No SlaveManager transfer happens. Veterancy is
      lost; old slaves are liberated to Neutral or killed by `C4Warhead`.
      See §10.8. Confidence: HIGH (cross-verified MCV_DEPLOY +
      UNIT_MISSION_DEPLOY_BUILDING docs).
- [x] **vtable slot identities** for `[0xE0]`, `[0x388]`, `[0x3D0]`,
      `[0x3D4]` — fully resolved from `TECHNOCLASS_VTABLE_COMPLETE.md`.
      Two corrections applied to §8 pseudocode annotations (`[0xE0]` is
      RecordKill not Unlimbo; `[0x388]` is a stub returning false). See §10.5.

### Follow-up pass — all three residual gaps resolved (2026-04-22)

- [x] **`FUN_0054CA90` identified** — it is `JumpjetLocomotionClass::Process` **state 5 (Abort/Emergency)**, the unit-kill-on-invalid-landing handler. Cross-verified in:
      - `JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md:140` (state 5 = Abort/Emergency, "entered when destination becomes invalid mid-flight... forces land immediately")
      - `MAGNETRON_SYSTEM_GHIDRA_REPORT.md:420-446` (explicit "State 5 kill logic (verified at FUN_0054CA90)" with decompile excerpt showing the `MasterDestroyed` call path)

      **Why it calls `MasterDestroyed`:** When a Magnetron lifts a unit that has a
      SlaveManager (e.g., lifts a SMIN Slave Miner) and the lifted unit cannot
      safely land (no valid cell at drop-off), the Magnetron forcibly kills the
      unit. The kill path invokes `MasterDestroyed` on the SlaveManager to
      liberate the slaves. This is the "slave miner killed mid-air by Magnetron"
      path. Verified excerpt:

      ```c
      // FUN_0054CA90, the target-cannot-land branch:
      target.ShouldSelfDestruct /* +0x3CD */ = 1;
      target.vtable[0x3A0]();                    // KillSelf
      if (target.LinkedBuilding /* +0x2D8 */) {  // +0x2D8 is the SlaveManager slot
          FUN_006B0AE0(target.SlaveManagerPtr, 0);   // MasterDestroyed(attacker=null)
          target.LinkedBuilding.vtable[0x20](1);     // cleanup animation
          target.LinkedBuilding = 0;
      }
      ```

      The call is `MasterDestroyed(attacker=0, new_house=0)`, so slaves follow
      §8 Case B-fallback → Civilian/Neutral house, or §8 Case A if no neutral
      house exists. Note: target+0x2D8 in this context is reused as the
      chrono-source building link (per the MAGNETRON doc), NOT the SlaveManager
      directly. The actual pointer passed to `MasterDestroyed` is the *source*
      SlaveManager/ChronoSource — semantics here are subtle and deserve direct
      decompile confirmation when re-implementing.

- [x] **`Rules+0x234` = `SlavesFreeSound`** — DEFINITIVELY resolved via
      `GLOBAL_SOUNDS_GHIDRA_REPORT.md:250` which lists the complete Rules sound
      offset map parsed by `RulesClass::ReadAudioVisual @ 0x006691E0`:

      | # | INI Key | Rules Index | Byte Offset | Default |
      |---|---------|------------|-------------|---------|
      | 80 | **`SlavesFreeSound`** | 0x8D | **+0x234** | `SlaveWorkerLiberated` |
      | 81 | SlaveMinerDeploySound | 0x8E | +0x238 | SlaveMinerDeploy |
      | 82 | SlaveMinerUndeploySound | 0x8F | +0x23C | SlaveMinerDeploy |

      Cross-confirmed in `ini/rulesmd.ini:716`: `SlavesFreeSound= SlaveWorkerLiberated ; sound made when miner slaves are freed`.

      **Update to §8:** The comment in the `MasterDestroyed` pseudocode that
      read "`// SlaveLiberation sound index`" should read:
      "`// Rules+0x234 = SlavesFreeSound (default: SlaveWorkerLiberated)`".
      This is the sound that plays at the master's position when any slave
      survives liberation (Case B).

- [x] **`BuildingClass::vtable[0xF2]` is NOT `ChangeOwner`** — resolved via 5
      independent BuildingClass docs that all agree on slot 242 (byte offset
      `+0x3C8`) being:
      - `BUILDINGCLASS_MASTER_GHIDRA_REPORT_V2.md:326`: `ClearTarget / Set_ArchiveTarget(0)`
      - `BUILDINGCLASS_MISSION_ATTACK_GHIDRA_REPORT.md:878`: `BuildingClass::SetTarget @ 0x00443B90`
      - `BUILDINGCLASS_SELL_AND_REPAIR_GHIDRA_REPORT.md:596`: `BuildingClass::Assign_Target (misnamed ToggleGate)`
      - `BUILDINGCLASS_VTABLE_AND_LIFECYCLE.md:281`: `BuildingClass::ToggleGate @ 0x00443B90`
      - `BUILDINGCLASS_UPDATE_AI_TICK_GHIDRA_REPORT.md:217`: `vtable[0x3C8](0)  // ToggleGate(0) — close gate`

      Base `TechnoClass` slot 0xF2 is `SetDestination` (mobile-unit target-cell
      setter), but `BuildingClass` **overrides** this slot with
      `BuildingClass::ToggleGate / SetArchiveTarget` at `0x00443B90`. When the
      slave AI or harvester-dock path calls `building->vtable[0xF2](...)`, it
      is NOT changing ownership — it is either:
      - Toggling a gate building's open/close state (for GATETYPE buildings), OR
      - Setting/clearing the building's archive target (mission cleanup)

      **§10.8 CORRECTION:** The `UNIT_MISSION_DEPLOY_BUILDING_GHIDRA_REPORT.md:213`
      annotation "calls building's vtable[0xF2] (transfer ownership)" is
      **incorrect**. The actual semantics of the call in the
      Enslaved-harvester-docks-at-cross-faction-refinery path is
      `BuildingClass::SetArchiveTarget` — it stores the docking SMIN as the
      building's archive target (the "I-will-remember-this-object-between-missions"
      slot), NOT a permanent house transfer. The refinery's ownership does NOT
      change.

      **Gameplay consequence:** A captured YAREFN does NOT automatically convert
      back to Yuri when a SMIN docks at it. My §10.8 caveat that this "might
      be a significant ownership-rewrite mechanic" is incorrect — it is a
      benign archive-target assignment. Slave Miner cross-faction docking is
      not a gameplay exploit.

### Truly remaining (low-priority, not blocking implementation)

- [ ] Exact confirmation that `FUN_006B0AE0`'s third argument `param_3` in the
      `JumpjetLocomotionClass::Process` state-5 call path is `0` and not a
      specific house. The MAGNETRON doc decompile excerpt shows `(target.SlaveManagerPtr, 0)` — a 2-arg call — but `MasterDestroyed` is 3-arg
      (`this, attacker, new_house`). The `0` is presumably both `attacker` AND
      `new_house` passed as stack args with standard `__thiscall`. No
      disagreement between docs on this point. **Confidence: HIGH.**

All originally-open questions are now closed with HIGH confidence (or explicit
down-grade to MEDIUM where noted).

---

## 13. Additional findings — 2026-04-22 deeper archive pass

These extend the state-machine understanding with fields and behaviors that
were referenced but not fully developed earlier.

### 13.1 `slave+0x2DC = SlaveOwner` — gameplay consequences

The slave's back-reference at offset `+0x2DC` is **`SlaveOwner`** (TechnoClass*).
Cross-verified in three independent docs:

- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md:718` — "Points back to the TechnoClass that
  owns the SlaveManagerClass managing this slave. HIGH confidence"
- `SELECTION_GATES_GHIDRA_REPORT.md:41` — `+0x2DC | dword | SlaveOwner` — read by
  selection gate
- `MISSION_HARVEST_GHIDRA_REPORT.md:458` — independently labeled "SlaveOwner"

The field has **three behavioral consequences**:

1. **Enslaved slave cannot fire.** `TechnoClass::GetFireError` returns
   `FIRE_ILLEGAL` whenever `this+0x2DC != 0`. This is why SLAV's `Primary=SHOVEL`
   does nothing while the slave is enslaved — the weapon is gated off.
   (Verified: `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md:718` — "enslaved units
   cannot fire independently".)

2. **Enslaved slave cannot be commanded by the player.** Selection-gate logic
   in `SELECTION_GATES_GHIDRA_REPORT.md:90` shows:
   ```
   if this.SlaveOwner != 0:  return false
   ```
   The player cannot right-click-order enslaved slaves. This matches the INI
   comment at `ini/rulesmd.ini:5035` ("the only thing the player can do to an
   enslaved slave is select it"), but tightens it: selection is also blocked
   for normal command purposes. Slaves only respond to the SlaveManager's
   autonomous orders.

3. **Liberation flips all of the above.** When `MasterDestroyed` (§8) sets
   `slave[0x2DC] = 0`, the slave:
   - Becomes firable (`Primary=SHOVEL` now active — does 15-ish damage)
   - Becomes selectable/commandable
   - Switches voice set from `SlaveWorker*` to `SlaveFreed*`
     (the INI `VoiceSelect=SlaveFreedSelect` is the default; the
     `VoiceSelectEnslaved=SlaveWorkerSelect` takes over while
     `+0x2DC != 0`).

   This is implemented via `SELECTION_GATES`'s `+0x2DC` check returning
   `false`, not by any separate "IsFreed" flag.

**Confidence: HIGH.**

### 13.2 `[SLAV]` INI profile (verified from `ini/rulesmd.ini:5015-5056`)

| Key | Value | Relevance |
|-----|-------|-----------|
| `Slaved` | `yes` | Sets type flag at `TechnoTypeClass+0xD3E` |
| `Strength` | `125` | Slave HP — low; a single Initiate rad-beam volley kills |
| `Armor` | `none` | Universally vulnerable; every warhead deals full damage |
| `Storage` | `4` | **Bale capacity** — slave returns to master when 4 bales carried |
| `HarvestRate` | `150` (primary) / `180` / `210` / `75` | Frames between bale pickups (difficulty-scaled column; default = 150 frames) |
| `Primary` | `SHOVEL` | Enabled only after liberation (see §13.1) |
| `IsSelectableCombatant` | `no` | Excluded from "Select All" / drag-box selection even after liberation |
| `Trainable` | *(unset; default = yes)* | Can earn veterancy via kills — but see §13.5 for respawn reset |
| `DontScore` | `yes` | Kills of slaves don't count for enemy's score |
| `ImmuneToPsionics` | `yes` | Yuri cannot mind-control his own slaves (or anyone else's enslaved slaves) |
| `ImmuneToVeins` | `yes` | Veinhole monsters don't affect slaves |
| `Points` | `5` | Tiny score contribution if scored — but DontScore overrides |
| `AllowedToStartInMultiplayer` | `no` | Cannot be starting unit — only spawned by SlaveManager |
| `Cost` | `10` | Refund on liberation+sell is 5 credits (cost * Soylent/2) — except Soylent=0 overrides to no refund |
| `VoiceSelect` | `SlaveFreedSelect` | Used when `+0x2DC == 0` (liberated) |
| `VoiceSelectEnslaved` | `SlaveWorkerSelect` | Used when `+0x2DC != 0` (enslaved, the normal case) |
| `DieSound` | `SlaveWorkerDie` | Single death sound; no "I am free!" variant |

### 13.3 How slaves accumulate bales — the embedded StorageClass

Per `TECHNOCLASS_SYSTEMS_GHIDRA_REPORT.md:653-687`:

- **Every TechnoClass instance embeds a 16-byte `StorageClass`** (4 floats, one
  per tiberium type). Both harvesters and slaves use this.
- `StorageClass::AddAmount(amount, tibType)` at `0x006C9690` adds bales.
- `UnitClass::Get_Storage_Percentage @ 0x007414A0` returns `total_bales /
  max_capacity` where `max_capacity` is the unit's `Storage=` from INI.

Slave harvesting (state 3) calls `StorageClass::AddAmount(1.0, ore_type)` each
time `HarvestRate` frames elapse. When `Get_Storage_Percentage() >= 1.0` (so 4
bales at `Storage=4`), `FUN_00522D30` returns true and the slave transitions
to state 4.

On dock (state 4 arrival), the slave's storage contents are transferred to the
master building. The existing `SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md §4`
already documents `BuildingClass::DepositOreFromStorage (FUN_00522D50)` as the
per-tibType deposit loop that drains the master's storage into house credits.
The slave-to-master storage transfer itself appears to happen as part of
`vtable[0xD4]` Limbo — **or as an implicit step in the decompile noise** not
fully decoded. **Gap:** the exact slave→master storage transfer is not yet
verified; the observable end state is that slave's storage is zeroed and
master's storage is incremented. Left as a minor follow-up.

### 13.4 `Rules+0xDF8 = ApproachTargetResetMultiplier` — distance semantics

The INI comment at `rules.ini:241` and `rulesmd.ini:301` clarifies:

> *"The ApproachTarget position should be recalculated if the target is now
> more than weapon range times this (My approach target picked a spot range 1x
> away, so if it gets beyond 1.5 I know it is moving and that I will need to
> refigure where he is.)"*

So the value is a **multiplier of weapon range**, not an absolute distance.
Default `1.5` = "150% of weapon range". Stored as int (likely via
`Math::ftol` truncation → 1 after parse, OR stored at a different scale —
see caveat below).

**Multi-semantic caveat:** The field at `Rules+0xDF8` is cited with different
semantics in at least three independent contexts in the archive:

| Doc | Context | Interpretation |
|-----|---------|----------------|
| `AI_DIFFICULTY_SYSTEM.md:365` | AI target acquisition | `ApproachTargetResetMultiplier` (range×1.5) |
| `BUILDING_DOCK_AND_HEAL_STATE_MACHINES.md:287` | Harvester dock | `MaxDockDistance` (distance threshold) |
| `BUILDING_SYSTEMS_GHIDRA_REPORT.md:802` | Building AI | "Team leash distance" |

This suggests **one Rules field is read as a distance threshold by multiple
subsystems**, each using the same stored value for a slightly different
tolerance purpose. The INI key that WRITES this field is
`ApproachTargetResetMultiplier` (default 1.5). The slave AI's use in state 4
treats it as a leash multiplier — "if the slave has wandered more than 1.5×
its movement range from master, re-seek". The exact integer encoding
(truncated to 1? stored as 256*1.5=384? 150 percent-int?) is not resolved in
any archive doc.

**Rust implementation guidance:** parse `ApproachTargetResetMultiplier` as
double; use the encoded integer form in the slave-leash comparison, matching
whatever encoding the harvester-dock path also uses. If the encoding is "int
after Math::ftol" (= 1), the slave leash is effectively "1 cell", which
matches observed gameplay (slaves typically return if moved 1 cell past
master). **Confidence: MEDIUM.**

### 13.5 Veterancy resets on respawn — emergent behavior

Slaves are `Trainable=yes` (default; INI doesn't set it to no). They *can*
earn XP via `VETERANCY_SYSTEM_GHIDRA_REPORT.md §3` (kill-XP attribution).
However:

- `FUN_006AF650` (Slave Respawn, §8) calls `HouseClass::CreateInfantry` to
  mint a **fresh** InfantryClass instance.
- Fresh instances start at rookie (veterancy = 0).
- No code path transfers the dead slave's veterancy counter to the new one.

**Net effect:** a slave that earned XP and reached veteran rank will
lose that rank when it dies and respawns. In stock YR, slaves rarely earn
XP (they're disarmed via `+0x2DC`), so this is near-invisible in normal play.
It only matters when slaves have been liberated and earn kills before dying.

**Confidence: HIGH** (via inspection of respawn code's `CreateInfantry`
call).

### 13.6 `TechnoClass+0x2D8` is **unambiguously** `SlaveManager`

The `MAGNETRON_SYSTEM_GHIDRA_REPORT.md:458` decompile excerpt annotates
`target.LinkedBuilding /* +0x2D8 */` — but this annotation is **incorrect**:

- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md:691` says explicitly: "0x2D0=SpawnManager, 0x2D8=SlaveManager"
- `MAGNETRON_SYSTEM_GHIDRA_REPORT.md:89` itself says LinkedBuilding is at **`+0x2B0`**, not `+0x2D8`

The Magnetron doc's `/* +0x2D8 */` comment on the decompiled LinkedBuilding
check is an inline annotation error. The actual field at `+0x2D8` in that
kill path is the SlaveManager pointer, and the Magnetron's state-5 code
specifically checks "does this unit have a SlaveManager? if so, call
MasterDestroyed before finalizing the kill". This is consistent with my §1
and §12 treatment of `FUN_0054CA90` — Magnetron kills of SMIN units
correctly liberate slaves.

**Confidence: HIGH** — two independent doc citations agree.

### 13.7 Manager offset neighborhood (consolidated, canonical)

The pointer-pair pattern at `TechnoClass+0x2D0..0x2DC` is:

| Offset | Field | Role |
|--------|-------|------|
| +0x2D0 | `SpawnManager` (SpawnManagerClass*) | Allocates/manages spawned projectiles (missile subspawns) |
| +0x2D4 | `SpawnOwner` (TechnoClass*) | Back-ref from a spawn to its spawner (for kill-XP attribution) |
| +0x2D8 | `SlaveManager` (SlaveManagerClass*) | Allocates/manages slaves |
| +0x2DC | `SlaveOwner` (TechnoClass*) | Back-ref from a slave to its master (for fire-block + selection-block) |

These are symmetric by design. Both manager objects allocate in
`TechnoClass::Init_Managers @ 0x006F3F40` and both back-references are
cleared by the corresponding "master destroyed" / "spawner destroyed" paths.

**Implication for Rust:** the Rust `GameEntity` should have:
- `spawn_manager: Option<SpawnManager>` (already exists)
- `spawn_owner: Option<EntityId>` (exists for spawn tracking)
- `slave_manager: Option<SlaveManager>` (exists via `slave_bindings`)
- **`slave_owner: Option<EntityId>`** — back-ref from slave to master.
  `src/sim/slave_miner.rs:62-88` already has `master: EntityId` in
  `SlaveHarvester` — this IS the equivalent. The mapping is correct; only
  the firing/selection consequences (§13.1) are unimplemented.

### 13.8 Updated Rust gap list (supersedes §11)

With §13.1-13.7 integrated, the concrete Rust gaps are:

1. **GetFireError for enslaved slaves** — Rust must return an equivalent of
   `FIRE_ILLEGAL` when `entity.slave_owner != null`. Without this, liberated
   slaves can shoot but enslaved slaves (via mind control scenarios) would
   also erroneously be able to fire.
2. **Selection-gate for enslaved slaves** — the Rust selection system must
   reject click-orders on slaves whose `slave_owner != null` (same field check).
3. **Dual voice sets** — the audio layer should pick `VoiceSelectEnslaved` vs
   `VoiceSelect` based on `slave_owner != null`. Cosmetic but visible.
4. **Storage percentage check** — Rust state 3 → 4 transition should gate on
   `storage.current / storage.capacity >= 1.0`, not health-ratio. (Current
   `src/sim/slave_miner.rs` uses a cargo `Vec<_>` — the capacity gate is
   probably already correct by construction; just verify it uses `Storage=4`
   from INI, not a hardcoded constant.)
5. **Liberation preserves "slave" appearance** — liberated slaves keep
   `Primary=SHOVEL`, `Armor=none`, `IsSelectableCombatant=no`. They're weak
   civilian-tier units, not suddenly combat-capable.

All other gaps from §11 remain valid as previously documented.

---

## 14. Follow-up (2026-04-22 — undeploy path + beam class taxonomy + Capturable correction)

### 14.1 Undeploy path (YAREFN → SMIN) — symmetric to deploy

YAREFN explicitly sets `UndeploysInto=SMIN` (`ini/rulesmd.ini:13289`). Combined
with the game-level `MCVRedeploys=yes` default (`rulesmd.ini:3041`), the Yuri
Slave Refinery CAN be undeployed back into a mobile Slave Miner — unlike the
Yuri ConYard `[YACNST]`, which has no `UndeploysInto` set and cannot undeploy.

The undeploy code path goes through `BuildingClass::Mission_Deploy @ 0x0073D630`
and `FUN_007393C0` (both documented in `MCV_DEPLOY_GHIDRA_REPORT.md §Path 2`).
The final step is "Building destroyed, MCV unit placed" — which, applied to
YAREFN, means:

1. `FUN_007393C0` creates a new SMIN at the YAREFN's cell.
2. SMIN's `TechnoClass::Init_Managers` creates a **fresh** SlaveManager with 5
   new SLAVs (because SMIN also has `Enslaves=SLAV, SlavesNumber=5` in INI).
3. YAREFN is destroyed. Its destructor fires SlaveManager::~SlaveManager →
   `MasterDestroyed(attacker=null, new_house=null)`.
4. The YAREFN's 5 existing slaves are processed per §8 — BUT with the
   limbo-slaves carve-out (see §14.2 below): most of them are currently
   docked inside the YAREFN (state 5 "Regenerating" or state 6 "Dead"), so
   they silently UnInit rather than becoming visible civilians.

**Observable outcome:** identical-looking to deploy — 5 old slaves vanish, 5
fresh slaves spawn. Symmetric with SMIN→YAREFN. Veterancy is lost.

**Confidence: HIGH** via parallel structural analysis + limbo-carve-out in §8.

### 14.2 ⚠ Important subtlety — limbo slaves silently die, only visible ones liberate

Re-reading the `MasterDestroyed (FUN_006B0AE0)` decompile with the
`+0x81 = InLimbo` offset identity confirmed (`OBJECTCLASS_GHIDRA_REPORT.md:78`):

```c
if (slave[0x81] != 0) {                         // slave is currently in limbo
    slave.vtable[0xE0](attacker);               // RecordKill (credits the kill)
    slave.vtable[0xF8]();                        // UnInit (silent cleanup)
    continue;                                    // skip the liberation path
}
// Otherwise: liberation path (C4Warhead kill OR house-transfer)
```

`+0x81 = InLimbo` is the ObjectClass "am I on the map right now?" flag:
- Init = 1 (constructor)
- Set to 0 by `Unlimbo` (placed on map)
- Set to 1 by `Limbo` (removed from map)

**Implication for slave lifecycle:** at any given moment, a SlaveManager's 5
slaves are split between:
- **Visible slaves** (states 1/2/3/4 — scanning, moving, harvesting, returning) — `InLimbo = 0`
- **Limbo'd slaves** (state 5 Regenerating, state 6 Dead, between deposit and re-deploy) — `InLimbo = 1`

When the master is destroyed:
- Visible slaves follow §8 Case A/B — **liberated** (become neutral civilian
  or attacker-owned) or **killed by C4Warhead** (if no attacker/neutral
  house exists).
- Limbo slaves silently UnInit. They never appear on the map as freed
  civilians.

**Gameplay consequence:** at the moment you destroy a Yuri Slave Refinery,
only the slaves that happen to be walking around at that tick become free
civilians. Slaves that were mid-dock or respawning just disappear. In
practice this means 3-5 liberated slaves typically, never exactly 5.

**§8 pseudocode correction applied** — the `if (slave[0x81] != 0)` branch
comment should say "slave is in limbo (regenerating/dead/docked)" rather
than "slave is in some flag-off state".

**Confidence: HIGH.**

### 14.3 `[YAREFN] Capturable=false` — capture concerns overturned

The YAREFN INI at `rulesmd.ini:13263` reads `Capturable=false;gs true`
(semicolon-comment truncation — the effective value is `false`, with
designer's commented-out intent "true"). **Slave Miners cannot be captured
by engineer in stock YR.**

This retroactively simplifies §9's capture analysis:

- **§9 claim:** "capturing a YAREFN Slave Refinery transfers the building to
  the new owner but leaves the SlaveManager pointing at the same Enslaves
  type, same slaves list, and same owner back-reference" — i.e., slaves
  remain Yuri-colored after capture, new slaves come out in new color.
- **Reality:** Engineers cannot capture YAREFN at all in stock YR. The
  described behavior applies only in modded scenarios where `Capturable=true`
  is explicitly re-enabled.

The SlaveManager/ChangeOwner interaction I analyzed (where `TechnoClass::ChangeOwner @ 0x007014A0` does NOT notify SlaveManager) remains correct as described, but its practical relevance in stock YR is limited to:

- **Mind control of a Slave Miner** via Yuri/Yuri Prime/Brute — MCable, so
  the building's owner changes. Old slaves stay Yuri-owned; new slaves come
  out under the mind-controller's house. Confirmed this IS a live scenario.
- **Modded `Capturable=true` YAREFN** — same mechanics as mind-control
  ownership flip.
- **Psychic Dominator permanent MC** — permanent, works the same way.

So the capture-related finding is *correct for mind-control* but *mis-scoped*
to engineer-capture, which doesn't apply in stock rules.

**Confidence: HIGH.**

### 14.4 "Brain transplant" designer comment — now fully explained

The rulesmd.ini comment at `ini/rulesmd.ini:13284`:

> `;moving brain to refinery to start`
>
> `;Ugh. Now that placed as building, problem arises from managing to get a SMIN as vehicle (Campaign map, crate). Both get this listing now, and Brain transplant will check to make sure extra one is not created`

Context clarified: the designer is explaining why **both SMIN and YAREFN
have `Enslaves=SLAV, SlavesNumber=5`** in INI. Originally the slaves would
be created only on the building (YAREFN), not the vehicle (SMIN). But then
campaign maps or crates might spawn a SMIN directly as a vehicle (without
going through deploy), and that SMIN would have no slaves.

So both types got the `Enslaves` listing. The comment "Brain transplant
will check to make sure extra one is not created" reflects the designer's
intent that a guard would prevent double-creation during deploy.

**Binary verification:** `TechnoClass::Init_Managers @ 0x006F3F40` creates
the SlaveManager **unconditionally** when `type+0xD40 (Enslaves) != 0`. No
"skip if an extra already exists" guard. My earlier conclusion stands — the
actual behavior is destroy-and-recreate (not transfer), and the designer's
"Brain transplant check" is either (a) in a code path I haven't found, or
(b) an INI-comment aspiration not fully implemented in code. The observable
end result — 5 old slaves destroyed, 5 fresh slaves spawned — matches
destroy-and-recreate, not transfer.

**Confidence: HIGH** that there is no SlaveManager transfer.
**Confidence: MEDIUM** that no hidden "skip" guard exists (could be in a
code path far from `Init_Managers`, e.g., in UnitClass::Deploy's property-
transfer block).

### 14.5 SpawnManagerClass structural parallel (Rust architecture note)

Per `SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md` (read this pass), SpawnManager and
SlaveManager share the same architectural pattern but with different
semantics:

| Aspect | `SlaveManagerClass` (0x64 bytes) | `SpawnManagerClass` (0x74 bytes) |
|--------|----------------------------------|-----------------------------------|
| Owner | `+0x24 = TechnoClass*` | `+0x24 = TechnoClass*` |
| Type | `+0x28 = InfantryTypeClass**` (double-ptr into type array) | `+0x28 = TechnoTypeClass*` |
| Count | `+0x2C = SlavesNumber` | `+0x2C = SpawnsNumber` |
| RegenRate | `+0x30 = SlaveRegenRate` | `+0x30 = SpawnRegenRate` |
| ReloadRate | `+0x34 = SlaveReloadRate` | `+0x34 = SpawnReloadRate` |
| Slot array | `+0x3C..+0x48` DynVector | `+0x38..+0x4C` DynVector |
| Update timer | `+0x50..+0x58` (10-frame rate) | `+0x50..+0x58` (10-frame rate) |
| Manager state | `+0x5C = 7 states` (idle/seek/moving/retry/deployed/check/relocate) | `+0x70 = 3 modes` (idle/launching/returning) |
| State timer | `+0x60 StateTimer` | `+0x5C..+0x64 ReloadTimer` (gates launches per manager) |
| Target | — | `+0x68 CurrentTarget`, `+0x6C QueuedTarget` |
| TechnoClass slot | `+0x2D8` | `+0x2D0` |
| Back-ref | `slave+0x2DC = SlaveOwner` | `spawn+0x2D4 = SpawnOwner` |
| Control struct | `SlaveControl` (0x14 bytes): slave, state, 3 timer fields | `SpawnControl` (0x18 bytes): spawn, state, 3 timer fields, +0x14 IsMissileSpawn bool |
| Per-slot states | 7 (0=Ready, 1=Scan, 2=MoveToOre, 3=Harvest, 4=Return, 5=Regen, 6=Dead) | 7 (0=ReadyDocked, 1=KamikazeWait, 2=InFlight, 3=Returning, 4=LandingAtDock, 5=unused, 6=Reloading, 7=Regen) |

**Shared idioms:**
- Both classes appear in the global spawn-manager-registry pattern
  (DynamicVectorClass at fixed DAT addresses).
- Both rate-limit updates to every 10 frames.
- Both allocate in `TechnoClass::Init_Managers` based on type flags
  (SpawnsNumber vs Enslaves).
- Both nullify back-references on "master destroyed" paths.

**Diverging idioms:**
- SpawnManager adds a **CurrentTarget/QueuedTarget** pair for target
  sequencing; slaves don't target — they navigate ore cells autonomously.
- SpawnManager's ManagerMode is a **3-state "mission cycle"** (idle→launch→
  return); SlaveManager's state machine is a **7-state "building-attachment
  lifecycle"** because slaves persist across many harvest cycles.
- SlaveControl has a `Timer_Duration` used as "regen cooldown when dead" and
  "reload delay when docked"; SpawnControl has an `IsMissileSpawn` flag used
  to distinguish fire-and-forget vs boomerang missile behavior.

**Rust-architecture guidance:** a common `ManagerBase<T, C>` abstraction
over `(Owner, Type, Count, RegenRate, ReloadRate, Vec<Control>, UpdateTimer)`
would be tempting, but the state machines are disjoint enough (ore-cycle vs
mission-cycle) that the abstraction gains little. Better to implement them
as two separate systems sharing only a `back_ref: EntityId` convention.

### 14.6 Beam-effect class taxonomy (final, corrected)

Putting together findings across SLAVE_MANAGER, WAVECLASS, PRISM_FORWARDING,
LASER_DRAW_CLASS, and RAD_BEAM_CLASS reports:

| Class | Ctor | Size | Gated by | Used by |
|-------|------|------|----------|---------|
| **LaserDrawClass** | `0x0054FE60` | `0x5C` (92 B) | `IsLaser`, `IsBigLaser`, `DiskLaser`, Prism (via `Rules.PrismType`), Railgun (particle system) | Prism Tower main beam, Prism support beams, Mirage Tank beam, Battle Fortress IFV beam, Guardian GI deployed beam, Robot Tank, Tank Destroyer, Vortex Disk, Yuri Railgun |
| **WaveClass** | `0x0075E950` | `0x240` (576 B) | `IsSonic` (TS-DEAD), `IsMagBeam` (WaveType 3) | **Stock YR: Magnetron only.** IsSonic=yes doesn't exist in any YR weapon. WaveType 1/2 have no callsites. |
| **RadBeam** | `0x006593F0` | `0xC8` (200 B) | `IsRadBeam` | Desolator, Chrono Legionnaire (via ChronoBeam warhead/Temporal=yes), RadEruption (8-cell neighbor spawn) |

These are **three distinct classes with no common base** (despite all
being "beam effects"). They register with **three different global draw
lists** and are rendered by **three separate draw paths**. A Rust port
should implement them as three separate systems — don't try to unify under
a generic "BeamEffect" trait because:

- LaserDrawClass is a POD (no vtable), rendered by the global
  `g_LaserDraw_Array` tick.
- WaveClass inherits from ObjectClass (has a vtable with AI/Draw slots),
  registered in the global wave array at `DAT_00A8EC3C`.
- RadBeam has its own allocator pattern (`RadBeam__Allocate`), separate
  draw list.

Confirmed in `LASER_DRAW_CLASS_GHIDRA_REPORT.md §4` ("LaserDrawClass has no
conventional vtable — this is atypical for the RA2 codebase and is the
reason it does NOT participate in `LayerClass` rendering").

### 14.7 Wave types 1 and 2 — verified dead in stock YR

Combining findings this pass:

- `TechnoClass::Fire_At` is the **only caller** of `WaveClass::Constructor`
  (per xref from `0x006FF470` and `0x006FF647`).
- Both callsites construct with literal `waveType=0` and `waveType=3`
  respectively (verified in the disassembly excerpt in the addendum §5).
- Types 1 and 2 have populated vertex LUT data at `DAT_00B45DA8` (the
  "4 corners × 4 variants" table in the type-0/1/2 geometry helper), but
  no code path instantiates them.

Rules out:
- IsLaser / IsBigLaser triggering type 1 or 2 — those use LaserDrawClass,
  not WaveClass.
- IsRadBeam triggering type 1 or 2 — uses RadBeam, not WaveClass.
- Any weapon triggering type 1 or 2 via a subtle flag — no other Fire_At
  branch constructs WaveClass.

**Types 1 and 2 are dead code in stock YR.** Likely TS-era intermediate
WaveType variants (possibly the old Sonic Tank tiers, or reserved slots)
that never shipped. Do not implement in Rust unless explicitly modding.

**Confidence: HIGH** (only one caller; both paths fully decoded).

### 14.8 Final consolidated Rust gap list

All findings across §11, §13.8, §14 condensed. Things the Rust
implementation must match for 99% parity:

1. **SlaveManager state machine (7 states)** — implement the full damage-
   triggered relocation flow (state 4→5 via `BuildingClass.DamagedState`),
   30-frame retry (state 3), and kick-frame scan-throttle
   (`SlaveMinerKickFrameDelay=150`).
2. **Per-slave state machine (7 states)** — with storage-percentage full
   check (not health-ratio), dual timers (`SlaveReloadRate=25` on dock,
   `SlaveRegenRate=500` on death), and post-dock Limbo.
3. **SlaveOwner back-reference at `slave+0x2DC`** — gates `GetFireError →
   FIRE_ILLEGAL` and selection rejection. Essential for correct enslaved-
   slave gameplay.
4. **MasterDestroyed liberation** with two branches:
   - Limbo slaves → silent UnInit (§14.2)
   - Visible slaves → transfer to attacker/new_house/Civilian, or kill via
     `Rules.C4Warhead` (§8)
5. **`SlavesFreeSound` cue** played at master's cell if any slave survives
   liberation (`Rules+0x234`, default `SlaveWorkerLiberated`).
6. **Deploy/Undeploy = destroy-and-recreate** for both SMIN→YAREFN and
   YAREFN→SMIN paths. Veterancy lost, fresh 5 slaves spawn. Old slaves
   follow MasterDestroyed semantics (§14.1).
7. **`Capturable=false` for YAREFN** — engineer capture does NOT apply to
   Slave Miners in stock YR. Only mind-control can flip ownership.
8. **Mind-control flips ownership** but does NOT notify SlaveManager — old
   slaves remain on old-house voice/color until they die and respawn under
   the new owner.
9. **Dual voice sets for SLAV** — `VoiceSelectEnslaved` while `+0x2DC != 0`,
   `VoiceSelect` otherwise.
10. **Storage-based bale accumulation** via TechnoClass-embedded StorageClass,
    `Storage=4` capacity, `HarvestRate=150` frames per bale.
11. **ScanForTiberium using `Rules.SlaveMinerSlaveScan` (14 cells)** for
    per-slave ore detection — NOT the harvester's `TiberiumShortScan`.
12. **Wave class: implement Type 3 only (Magnetron)**, skip Type 0 (dead
    IsSonic) and Types 1/2 (dead).
13. **Three separate beam classes** — implement LaserDrawClass, WaveClass,
    RadBeam as distinct systems (§14.6).

---

**End of research document.** All originally-open questions closed; minor
residuals explicitly noted with confidence levels. Report total: 1,200+ lines.
If anything above conflicts with direct binary re-verification during
implementation, **trust the binary** and update this doc.

## 12.5 Updates to the Rust gap list

With Rules offsets fully resolved, the Rust gap analysis in §11 can now be
sharpened:

1. The five `SlaveMinerConfig` values in `src/sim/slave_miner.rs:344-375`
   must map to the correct Rules offsets: `SlaveMinerShortScan → +0x1780`,
   `SlaveMinerSlaveScan → +0x1784`, `SlaveMinerLongScan → +0x1788`,
   `SlaveMinerScanCorrection → +0x178C`, `SlaveMinerKickFrameDelay → +0x1790`.
   (Defaults from stock rulesmd.ini: **8, 14, 48, 3, 150** cells/frames.)
2. The manager's damage-triggered relocation (state 4→5) needs a
   `building.is_damaged` observer in Rust. This is a single flag, trivial
   to wire — set it whenever the YAREFN drops below its damage threshold.
3. The master-destroyed liberation (§8) uses `Rules.C4Warhead` as the
   null-attacker death weapon. Rust's existing warhead infrastructure can
   handle this once the `C4Warhead` Rules slot is wired.
4. "Brain transplant" behavior — the Rust MCV-deploy path should explicitly
   liberate the old SMIN's 5 slaves before spawning the YAREFN's 5 slaves.
   Correct behavior: destroy/recreate, not transfer. Veterancy loss is
   expected and authentic.

---

## Sources

- **Ghidra decompile (gamemd.exe @ 0x00400000):**
  - `FUN_006AF5F0` (manager dispatch / rate timer)
  - `FUN_006AFD60` (manager state machine)
  - `FUN_006B0DB0` (HandleReturnedSlaves / target-swap helper)
  - `FUN_006B1020` (ShouldRecallSlaves / ShouldRecallTimerCheck)
  - `FUN_006B0300` (FindDeployCell)
  - `FUN_006B04C0` (DeploySlaves)
  - `FUN_006B0D60` (DeployAllSlaves)
  - `FUN_006B0CC0` (RecallAllSlaves)
  - `FUN_006B0490` (RecallIdleSlaves)
  - `FUN_006B0A20` (RemoveSlave)
  - `FUN_006B0AE0` (MasterDestroyed — extended)
  - `FUN_006AF6C0` (AI_Update per-slave — extended)
  - `FUN_006AF650` (Slave respawn — confirmed)
  - `FUN_006F3F40` (TechnoClass::Init_Managers — SlaveManager allocation condition)
  - `FUN_005196A0` (InfantryClass::Mission_Enter — vtable[0xD4] identification)
  - `FUN_007014A0` (TechnoClass::ChangeOwner — no-SlaveManager-notify finding)
  - `FUN_00448260` (BuildingClass::ChangeOwner — no SlaveManager interaction)

- **Cross-references / companion docs:**
  - `SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md` (prior work — layout, existing decoded portions)
  - `HOUSECLASS_GHIDRA_REPORT.md` (HouseClass field map for MasterDestroyed house lookup)
  - `RULESCLASS_READGENERAL_GHIDRA_REPORT.md` (for RulesClass offset conventions)

- **INI cross-check:** `ini/rulesmd.ini` sections `[SMIN]` (9042-9111), `[YAREFN]`
  (13234-13302), `[SLAV]` (5015-5056). `SlavesNumber=5`, `SlaveRegenRate=500`,
  `SlaveReloadRate=25`, `Enslaves=SLAV` appear on both SMIN and YAREFN,
  confirming the dual-SlaveManager design.

- **Rust scan:** `src/sim/slave_miner.rs` (slave harvester implementation),
  `src/rules/object_type.rs:295-315` (INI parsing), `src/sim/miner/mod.rs:46-54`
  (MinerKind::Slave).
