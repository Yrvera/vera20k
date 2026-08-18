# FootClass::Find_Docking_Bay — Arg3 Fallback Semantics
**Ghidra RE Report — `FUN_004DF040`**
Date: 2026-05-19
Scope: arg3=0 vs arg3=1 behavioral difference; g_MapEditorMode toggle semantics; DockList struct layout; BuildingTypeClass dock offsets.

---

## 1. Function Identity

| Item | Value |
|---|---|
| Address | 0x004DF040 |
| Ghidra name | `FootClass__Find_Docking_Bay` |
| Calling convention | `__thiscall` (ecx = this = FootClass/UnitClass instance) |
| Vtable slot | 0x528 in FootClass vtable (base 0x7E8C94) and UnitClass vtable (base 0x7F5C70) |
| DATA xrefs | 0x7E91BC (FootClass slot 0x528), 0x7EB580 (near UnitClass), 0x7F6198 (UnitClass slot 0x528) |
| Direct CALL xref | 0x0041BC17 in `AircraftClass__FindBuildingToDock` |

Active in YR: **Yes**. Called every time a harvester/chrono miner needs to find a refinery. Fires in normal skirmish.

---

## 2. Signature

```c
int __thiscall FootClass__Find_Docking_Bay(
    int *this,        // ecx — the harvester/unit instance
    int   param_2,    // arg1 — pointer to DockList struct (see §5)
    int   param_3,    // arg2 — always 0 in observed calls; passed through to evaluator
    int   param_4     // arg3 — 0 = normal path, 1 = fallback path
)
// Returns: building instance pointer of best matching dock, or 0 (no dock found)
```

---

## 3. Top-Level Algorithm

```
best_dist = -1
best_building = 0
for each TypeClass entry in DockList:
    call evaluator (vtable slot 0x52C) with (TypeClass_entry, param_3, param_4, &dist)
    if result != 0:
        if dist < best_dist OR best_dist == -1:
            best_dist = dist
            best_building = result
        elif building+0x3D3 != 0:   // prefer already-queued building (byte flag)
            best_building = result
return best_building
```

Confidence: HIGH (content). All logic read directly from decompiled output at 0x4DF040.

---

## 4. The arg3=0 vs arg3=1 Behavioral Difference

### arg3 flows into the inner evaluator at vtable slot 0x52C (= 0x4DEE80)

In Find_Docking_Bay, `param_4` (arg3) is passed unchanged as the 3rd argument to the evaluator:

```c
evaluator(typeclass_entry, param_3 /*=0*/, param_4 /*=0 or 1*/, &dist_out)
```

### What the evaluator at 0x4DEE80 does with arg3

The evaluator iterates all building instances of the specified TypeClass owned by the unit's house.
For each candidate building, it runs a series of filters in order:

1. **Building validity check** — `HouseClass__CountOwnedInstances` call ensures non-zero count
2. **Type match** — `building.TypeClass == DockList_entry` (byte comparison at +0x520 on building)
3. **On-map check** — `building[0x81] == 0` (ObjectClass limbo flag; 0 = placed on map, not limbo'd)
4. **Dock reservation check (SKIPPED when arg3 == 1)**:

```asm
; address ~0x4DEF01
cmp  dword [esp+0x48], 1   ; arg3 == 1?
je   SKIP_RESERVATION_CHECK ; if yes, skip
push esi                    ; building ptr
mov  ecx, edi               ; unit ptr (this)
call FUN_0065ADF0            ; dock reservation/contact-list check
test eax, eax
je   REJECT_BUILDING         ; if check fails, skip this building
SKIP_RESERVATION_CHECK:
```

5. **CanDock check** — calls `BuildingClass__CanDock` (0x457CE0); see §6

### FUN_0065ADF0 — the skipped reservation check

Address: 0x0065ADF0. Signature: `uint(building_ptr, unit_ptr)`.

Iterates the building's contact list at `building[+0xE4]` (data ptr) / `building[+0xE8]` (count).
Returns 1 (accept) if:
- list is empty (count == 0) → dock has no assigned unit (free)
- OR: one list element is null (free slot)
- OR: one list element == unit_ptr (unit is already assigned to this dock)

Returns 0 (reject) if all slots are filled by OTHER units.

**Effect of skipping (arg3=1):**
The chrono miner can target a dock building even when all its reservation slots are
occupied by other harvesters. Under arg3=0, only docks with a free slot or a slot
pre-reserved for this specific miner are accepted. Under arg3=1, every building of
the right type (that passes the map/CanDock checks) is a candidate regardless of
whether its reservation list is full.

---

## 5. g_MapEditorMode Toggle Semantics

### Location

`g_MapEditorMode` global: **0x00A8E7AC** (verified via list_globals).

### Bracket in Mission_Harvest state 2 (UnitClass__Mission_Harvest @ 0x73E5E0)

```c
// Normal first attempt (always tried first):
piVar3 = Find_Docking_Bay(this, TypeClass + 0x3E8, 0, 0);  // arg3=0
if (piVar3 != NULL) { ... }

// Fallback: tries AGAIN with relaxed filters
g_MapEditorMode++;
piVar3 = Find_Docking_Bay(this, TypeClass + 0x3E8, 0, 1);  // arg3=1
g_MapEditorMode--;
```

Both calls are in **state 2** of Mission_Harvest, which is the "full load, need to return to refinery" state. The normal (arg3=0) call is tried first in the same state. Only when it returns NULL does the fallback fire.

### What g_MapEditorMode != 0 does inside Find_Docking_Bay

`g_MapEditorMode` is NOT directly read inside Find_Docking_Bay at 0x4DF040 or the
evaluator at 0x4DEE80. Its effect is through downstream callers:

**`HouseClass__Is_Enemy` @ 0x00501540:**
```c
if (g_MapEditorMode != 0) {
    return true;   // all houses are "enemies" when MapEditorMode is set
}
```
(Verified at address 0x0050157C; `g_MapEditorMode` read confirmed at 0x00501595.)

**Effect in the Find_Docking_Bay chain:** `HouseClass__Is_Enemy` is called from alliance checks
in `CanDock` and the evaluator. However, the evaluator primarily uses the unit's OWN house
building list (via `house + 0x5500` indexed by building type), not an ally-expansion loop.
For the chrono miner (`Teleporter=yes`, `Harvester=yes`), `CanDock` takes the same-house
ownership path (not the `HouseClass__Is_Ally_ByObject` path), which does NOT call `Is_Enemy`.

**Practical effect of g_MapEditorMode bracket:** The incremented MapEditorMode primarily acts
as a safety bypass for any Is_Enemy-gated checks that might inadvertently block the fallback
search. In the common case (unit searching for its own house's refineries), it has no direct
observable effect — the arg3=1 flag is the operative change. The g_MapEditorMode bracket is
defensive: it ensures that any alliance-check gating downstream of Find_Docking_Bay cannot
accidentally reject valid candidates during the fallback path.

**Active in YR:** Yes. The increment/decrement is in `UnitClass__Mission_Harvest` state 2,
which fires whenever a fully-loaded harvester tries to return. In skirmish with a chrono miner,
this fires every time the miner loads up and no reserved dock is found.

---

## 6. BuildingClass::CanDock — Filters That Always Apply (arg3=0 AND arg3=1)

Address: 0x00457CE0. Called by evaluator regardless of arg3.

```c
bool CanDock(building, unit) {
    if (unit == NULL) return false;
    if (building.TypeClass[+0x157B] == 0) return false;   // building is not a dock type
    if (building.State == 0x12 || building.State == 0x13) return false;  // under construction or sold
    if (!MapClass__IsCoordsInPlayfield(building.coords)) return false;    // off-map
    if (TechnoClass__IsWarpingOut(building)) return false;                // temporal warped
    
    occupants = BuildingClass__GetOccupantCount(building);  // building[+0x694]
    max_dockers = building.TypeClass[+0x1580];              // BuildingTypeClass max dock count

    if (unit.TypeClass[+0xEB4]):    // Teleporter flag path (Teleporter=yes -> uses 0xEB4/0xEB5)
        // ... ally check via HouseClass__Is_Ally_ByObject AND dock not full
    else:                           // same-house ownership path
        if (building.House == unit.House || building.House.allied_flag):
            if (occupants != max_dockers && !building.IsRedHP && !building.IsMindControlled):
                return true
}
```

**Fields verified from decompilation at 0x457CE0:**
- `building.TypeClass` at instance offset 0x520 bytes (= `param_1[0x148]` in int-ptr indexing)
- `building.State` at instance offset 0xC (via `param_1[1].vtable` cast; states 0x12/0x13 = under construction/sold)
- `building[+0x694]` = current occupant count (via `BuildingClass__GetOccupantCount @ 0x4581F0`)
- `BuildingTypeClass[+0x1580]` = max dock count

Note on UnitTypeClass field 0xEB4/0xEB5: These are TechnoTypeClass-level flags, offset from the class base. `Teleporter=` from rulesmd.ini maps to `TechnoTypeClass[+0xCD4]` (verified: TechnoTypeClass__ReadINI @ 0x712170 writes it at `param_1 + 0x335` where param_1 is `int*`, so byte offset 0x335*4=0xCD4). The 0xEB4/0xEB5 offsets in CanDock are relative to the unit INSTANCE's type field at `instance+0x6C0`; the exact mapping to named INI keys requires further investigation.

---

## 7. DockList Struct Layout

From Find_Docking_Bay decompilation:

```
DockList struct (at UnitTypeClass + 0x3E8):
  +0x00:  ? (unused in this function)
  +0x04:  int*  — pointer to array of BuildingTypeClass* entries
  +0x08:  ?
  +0x0C:  ?
  +0x10:  int   — count of entries in the array
```

The `param_1[0x1b1] + 1000` expression in Mission_Harvest:
- `param_1[0x1b1]` = UnitTypeClass* (at UnitClass instance byte offset 0x6C4)
- `+ 1000` = `+ 0x3E8` decimal → DockList struct starts at **UnitTypeClass byte offset +0x3E8**

**Active in YR:** Yes. The DockList is populated from `DockWith=` INI key at load time.

Confidence: HIGH (content) — read directly from decompilation.

---

## 8. BuildingTypeClass Dock Offsets (used after dock is found)

In Mission_Harvest state 2, AFTER Find_Docking_Bay returns a dock building `piVar3`:

```c
// piVar3[0x148] = piVar3[0x520 bytes] = building's TypeClass (BuildingTypeClass*)
sVar10 = piVar3[0x27] >> 8;   // building cell X
sVar2  = piVar3[0x28] >> 8;   // building cell Y

// Dock landing offset from BuildingTypeClass:
dock_X = sVar10 + *(short *)(piVar3[0x148] + 0x1618);
dock_Y = sVar2  + *(short *)(piVar3[0x148] + 0x161C);
```

- **BuildingTypeClass+0x1618** = X-axis dock offset (signed 16-bit, cell coordinate delta)
- **BuildingTypeClass+0x161C** = Y-axis dock offset (signed 16-bit, cell coordinate delta)

These are used to compute the cell where the chrono miner should teleport-to
(adjacent to the refinery's dock entrance). This is then passed to
`FootClass__Find_Nearby_Passable_Cell` to find the actual passable landing cell.

Active in YR: Yes. Fires every time a loaded chrono miner successfully finds a dock.
Confidence: HIGH (content) — read from decompilation of UnitClass__Mission_Harvest @ 0x73E5E0.

---

## 9. Player-Visible Difference: arg3=0 vs arg3=1

| Condition | arg3=0 (normal) | arg3=1 (fallback) |
|---|---|---|
| Dock must have free reservation slot | YES | NO |
| Dock must be of correct type (DockList match) | YES | YES |
| Dock must be on map (not limbo'd) | YES | YES |
| CanDock checks (state, capacity, ownership) | YES | YES |
| Dock reservation must include this unit | YES (or slot free) | Ignored |

**Observable player behavior:**

- **arg3=0 returns a dock**: The chrono miner has a valid reserved or free dock.
  Mission_Harvest state 2 proceeds to teleport the miner to the dock location.

- **arg3=0 returns NULL**: No dock was found with a free or pre-reserved slot.
  The fallback fires: arg3=1 is tried.

- **arg3=1 returns a dock**: A dock building of the right type exists on the map and
  is in a valid state (not under construction, not warping, etc.), but all reservation
  slots are taken by other harvesters. The chrono miner teleports to this dock anyway
  (overrides reservation system). Player sees the miner teleport home even when the
  refinery is already "fully booked" by other harvesters.

- **arg3=1 returns NULL**: No dock building of the right type exists at all (all
  destroyed, all under construction, all limbo'd). Miner cannot return; Mission_Harvest
  state 2 falls through to default timer behavior.

**Frequency:** This fires every time a chrono miner finishes loading. In any game with a
chrono miner AND multiple active harvesters simultaneously queued, the arg3=1 fallback can
trigger. With the default `HarvestersPerRefinery=2`, two harvesters compete for one refinery's
reservation slots — the second one will use the arg3=1 path. High frequency in normal gameplay
with CMIN active.

---

## 10. Summary of Callers

| Caller | Address | arg3 value | Context |
|---|---|---|---|
| `UnitClass__Mission_Harvest` state 2 (normal) | 0x73E5E0 | 0 | First attempt: need free/reserved slot |
| `UnitClass__Mission_Harvest` state 2 (fallback) | 0x73E5E0 | 1 | Second attempt (g_MapEditorMode bracket): ignore reservation |
| `AircraftClass__FindBuildingToDock` | 0x41BC17 | param_4 (forwarded) | Aircraft dock search; indirectly via `FootClass__Find_Docking_Bay` |

AircraftClass__FindBuildingToDock (0x41BBD0) calls `FootClass__Find_Docking_Bay` directly
(not via vtable), forwarding param_4 as-is. Aircraft use a separate "last known dock" cache
(`param_1[0x1b3]`) before calling Find_Docking_Bay.

Active in YR: Yes (all three paths are reachable in a normal skirmish).

---

## 11. Confidence Summary

| Claim | Confidence | Evidence |
|---|---|---|
| arg3=1 skips reservation check in evaluator | HIGH | Direct decompilation at 0x4DEF01: `cmp [esp+0x48],1; je skip` |
| Reservation check = FUN_0065ADF0 contact-list scan | HIGH | Call at 0x4DEF0B, decompile at 0x65ADF0 |
| g_MapEditorMode makes Is_Enemy return true | HIGH | Decompilation of HouseClass__Is_Enemy @ 0x501540, read at 0x50157C |
| DockList at UnitTypeClass+0x3E8 | HIGH | Mission_Harvest decompile: `param_1[0x1b1] + 1000` = TypeClass + 0xE8 |
| BuildingTypeClass+0x1618/+0x161C = dock XY offsets | HIGH | Mission_Harvest decompile: `*(short *)(piVar3[0x148] + 0x1618/0x161C)` |
| Vtable slot 0x528 = FootClass__Find_Docking_Bay | HIGH | Memory read at 0x7F6198 = 0x004DF040 |
| Vtable slot 0x52C = evaluator at 0x4DEE80 | HIGH | Memory read at 0x7F619C = 0x004DEE80 |
| CanDock always applies regardless of arg3 | HIGH | Evaluator hex: CanDock call is AFTER the je-skip block |
| g_MapEditorMode practical effect in Find_Docking_Bay is defensive/minimal for CMIN | MEDIUM | Is_Enemy not called in CanDock's Teleporter path; no MapEditorMode read in evaluator itself |
| UnitTypeClass+0xEB4/0xEB5 exact INI key mapping | LOW | Only field offsets seen in CanDock decompile; not yet cross-referenced to ReadINI |
