# Per-Class Can_Enter_Cell Hierarchy — Ghidra Research Report

**Phase:** Phase 2 of approved plan `docs/plans/2026-05-13-bridge-pathfinding-locomotion-investigation-plan.md`
**Plan items covered:** #10 (UnitClass), #11 (InfantryClass), #12 (AircraftClass), #13 (BuildingClass), #14 (LocomotionClass stub)
**Companion doc:** `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md` (items #15 CheckBridgeTraversal, #16 CellClass offsets, #17 flag-bit semantics)
**Date:** 2026-05-13
**Active in YR:** All live except the empty `LocomotionClass` base stub (still reachable via dispatch, returns 0).

> Every claim cites a Ghidra address + decompilation or `read_memory` byte dump.
> Confidence axes: **C**=content / **I**=identity / **B**=binding.

---

## 1. The two-slot virtual layout (this is the key shape)

The user's plan said "vtable+0x1B0 is Can_Enter_Cell". **That's half-right.** There are TWO slots, both load-bearing:

| Vtable slot | Role | Called from |
|-------------|------|-------------|
| **`+0x1AC`** | **A* Can_Enter_Cell entry** — returns code 0-7 | `AStar_main_loop @ 0x429F54` (`CALL [EDX+0x1ac]`) |
| **`+0x1B0`** | **Bridge traversal sub-check** — returns 0/7, writes `path_height` and `bridge_entered` outputs | Inside the +0x1AC handler of each class |

Both slots are polymorphic and each derived class overrides one or both. The 2026-05-12 correction note in MEMORY (`feedback_vtable_binding_verification`) which said "vtable[0x1B0] = CheckBridgeTraversal NOT parent virtual" was correct for FootClass/UnitClass/InfantryClass — but did not call out that +0x1AC is a *separate* slot containing the actual A* entry.

### 1.1 Verified vtable bases (read from class constructors)

| Class | Vtable base | Evidence (constructor write site) |
|-------|-------------|----------------------------------|
| `UnitClass` | `0x007F5C70` | `0x735794: MOV [ESI],0x7F5C70` (in `UnitClass::Destructor` at 0x735780; same value used in constructor pair) |
| `InfantryClass` | `0x007EB058` | `0x517ACC: MOV [ESI],0x7EB058` |
| `AircraftClass` | `0x007E21F8` | (computed: `0x415B10` is at slot `0x7E23A4`, which is `base + 0x1AC`) |
| `BuildingClass` | `0x007E3EBC` | `0x43B71F: MOV [ESI],0x7E3EBC` |
| `FootClass` | `0x007E8C94` | `0x4D345D: MOV [ESI],0x7E8C94` |

### 1.2 Per-class slot resolution (read directly from memory)

Reading each vtable at `base + 0x1A8..0x1B7` (16 bytes covering both slots and immediate neighbours):

#### UnitClass — `0x7F5E14..0x7F5E23`
```
+0x1A4 (0x7F5E14): 0x004DC810       (FUN_004dc810)
+0x1A8 (0x7F5E18): 0x005F4410       ObjectClass::UpdatePosition
+0x1AC (0x7F5E1C): 0x0073F0A0   ←   UnitClass::Can_Enter_Cell  (A* entry)
+0x1B0 (0x7F5E20): 0x004D9C60   ←   CheckBridgeTraversal       (bridge sub-check)
```

#### InfantryClass — `0x7EB200..0x7EB20F`
```
+0x1A8 (0x7EB200): 0x005F4410       ObjectClass::UpdatePosition
+0x1AC (0x7EB204): 0x0051BF90   ←   InfantryClass::Can_Enter_Cell  (unlabeled — `FUN_0051BF90` in Ghidra)
+0x1B0 (0x7EB208): 0x004D9C60   ←   CheckBridgeTraversal           (same as UnitClass)
+0x1B4 (0x7EB20C): 0x004DB810       (FUN_004db810)
```

#### AircraftClass — `0x7E239C..0x7E23AB`
```
+0x1A4 (0x7E239C): 0x004DE5D0       (FUN_004de5d0)
+0x1A8 (0x7E23A0): 0x00703850       (FUN_00703850 — 0xA-byte stub)
+0x1AC (0x7E23A4): 0x00415B10   ←   AircraftClass::Can_Enter_Cell  (the 8-direction landing-pad scanner)
+0x1B0 (0x7E23A8): 0x005F4B10   ←   ObjectClass::DrawIt            (NOT a bridge check — inherited from ObjectClass!)
```

#### BuildingClass — `0x7E4064..0x7E4073`
```
+0x1A8 (0x7E4064): 0x005F4410       ObjectClass::UpdatePosition
+0x1AC (0x7E4068): 0x00449440   ←   BuildingClass::Can_Enter_Cell  (unlabeled — `FUN_00449440`)
+0x1B0 (0x7E406C): 0x004264D0   ←   AnimClass__Click_stub (returns 0 — Ghidra label is misleading; this is a generic "return 0" stub)
+0x1B4 (0x7E4070): 0x005F6940       (FUN_005f6940)
```

#### FootClass — `0x7E8E3C..0x7E8E4B`
```
+0x1A8 (0x7E8E3C): 0x005F4410       ObjectClass::UpdatePosition
+0x1AC (0x7E8E40): 0x004D9C10   ←   FootClass::LocomotorPassabilityCheck  (base — thin function)
+0x1B0 (0x7E8E44): 0x004D9C60   ←   CheckBridgeTraversal
+0x1B4 (0x7E8E48): 0x004DB810       (FUN_004db810)
```

### 1.3 Polymorphism table (the actual dispatch on `this`)

| Concrete `this` | A* entry (+0x1AC) | Bridge sub-check (+0x1B0) | Notes |
|-----------------|-------------------|---------------------------|-------|
| UnitClass (vehicle) | `0x73F0A0` UnitClass::Can_Enter_Cell | `0x4D9C60` CheckBridgeTraversal | full pipeline; 8 return codes |
| InfantryClass | `0x51BF90` InfantryClass::Can_Enter_Cell | `0x4D9C60` CheckBridgeTraversal | similar pipeline, infantry-specific blocker logic |
| AircraftClass | `0x415B10` Aircraft 8-dir landing scanner | `0x5F4B10` **ObjectClass::DrawIt — NOT a bridge check** | Aircraft don't use the +0x1B0 bridge logic at all. Inherited slot from ObjectClass that points to DrawIt. |
| BuildingClass | `0x449440` BuildingClass::Can_Enter_Cell | `0x4264D0` stub `return 0` (Ghidra labels it `AnimClass__Click_stub` because the generic stub is shared) | Buildings return only 0/7. No soft codes. |
| FootClass (base — abstract, never instantiated alone) | `0x4D9C10` FootClass::LocomotorPassabilityCheck | `0x4D9C60` CheckBridgeTraversal | only reached for objects that didn't override — i.e. unreachable in practice |
| LocomotionClass | n/a (different hierarchy) | n/a | `LocomotionClass::Can_Enter_Cell @ 0x55ABF0` is a separate 4-byte stub returning 0. See §6. |

**Confidence:** C=HIGH (slot values read live), I=HIGH (function bodies decompiled), B=HIGH (each slot has one DATA xref from its vtable — confirmed exclusive).

---

## 2. UnitClass::Can_Enter_Cell (0x73F0A0) — full pipeline

Function size: 0x73F0A0..0x73FD45 (~3.2 KB). Signature:

```c
int __thiscall UnitClass::Can_Enter_Cell(
    int *this,           // FootClass*/UnitClass*
    int cell_ptr,        // dest CellClass*
    int direction,       // 0..7 = cardinal/diagonal; 8 = tube
    int path_height,     // current height in path; -1 = "compute it"
    undefined4 flags     // packed byte flags from caller (low byte = "include crushable")
);
```

### 2.1 Phase 1 — pre-vtable layer pre-decision

```c
// uStack_80 byte 3 — "this is a BRIDGE-layer move" (1) or GROUND (0)
if ((cell.Flags & 0x100) == 0 ||                                       // not bridge cell
    (path_height != -1 && abs(path_height - cell.Level) < 2)) {        // OR height-diff < 2
  uStack_80.byte3 = 0;  // GROUND layer
} else {
  uStack_80.byte3 = 1;  // BRIDGE layer
}
```

Verified at `0x73F0BD..0x73F0EB` (the **pre-vtable** decision). Identical gate to AStar_main_loop §4 (height-diff ≥ 2).

### 2.2 Phase 2 — pre-vtable ground-occupancy snapshot

```c
local_74 = cell.+0x54;                                                 // some cell field (radar pip?)
local_7c = (cell.+0x124 & 0xFF) | ((cell.+0x124 >> 5) & 0xFF) << 8;   // pack {OccBits[0], OccBits[5..7]}
                                                                       // ... masked to 0x1FF
```

Snapshots the **ground occupancy flags** (`cell.+0x124 = OccupationFlags`) into `local_7c` BEFORE the vtable+0x1B0 call. Verified at `0x73F0ED..0x73F109`.

### 2.3 Phase 3 — tube case (direction == 8)

```c
tube = CellClass::GetTubeAtCell();
if (direction == 8) {
  if (tube == NULL) return 7;                                          // no tube here → blocked
  if (tube.+0x28 (low word) == 0 && tube.+0x28 (high word) == 0)
    return 7;                                                          // tube endpoint zeroed → blocked
  return 0;
}
```

Tube-cell early termination. Direction code 8 is the synthetic "I'm entering this cell via a tube" direction.

### 2.4 Phase 4 — adjacent-cell tube collision check

```c
if (tube != NULL && abs(direction - tube.+0x2C) ∈ (2..6) && direction != -1)
  return 7;
```

If the current cell has a tube and the direction we want to move clashes with the tube's "natural" direction (`tube.+0x2C` is some angular field), blocked. This stops units from cutting across tube cells perpendicularly.

A second copy of the same check at `0x73F1A8..0x73F1C9` uses `(direction - 4) & 7` (= 180° opposite direction). Belt-and-suspenders.

### 2.5 Phase 5 — vtable+0x1B0 dispatch (the bridge sub-check)

```c
iVar6 = this->vtable[0x1B0](cell_ptr, direction, &path_height, &uStack_80.byte3);
if (iVar6 == 7) return 7;
```

For UnitClass, this dispatches to **`CheckBridgeTraversal @ 0x4D9C60`** — covered in the companion report. The sub-check **may UPDATE `path_height`** (the 3rd arg is a pointer, not a value). After this call, `path_height` may differ from the caller's expectation.

### 2.6 Phase 6 — post-vtable occupancy re-snapshot (the "two-pass" mechanism)

```c
if (path_height != -1 && (cell.Flags & 0x100) != 0 && path_height == cell.Level + 4) {
  unaff_EBP = cell.+0x58;                                              // bridge-layer field
  unaff_EDI = (cell.+0x128 & 0xFF) | ((cell.+0x128 >> 5) << 8);       // BRIDGE occupancy snapshot
                                                                       // ... masked to 0xFF01FF
}
```

This is the **bounded parity divergence** the 2026-05-12 audit identified. After the vtable+0x1B0 call returned `path_height = cell.Level + 4` (= bridge-deck level), the function re-reads the **bridge** occupancy field at `cell.+0x128` (AltOccupationFlags) — overwriting the earlier ground-list snapshot stored in `local_7c`/`unaff_EDI`.

But the **occupier-list iteration** later (Phase 8) still uses the layer chosen pre-vtable in Phase 1 (the `uStack_80.byte3` bit), which may now disagree with the height-derived layer chosen here. Two-pass divergence at the bridgehead-exit boundary tick.

### 2.7 Phase 7 — screen visibility + map-editor gating

```c
if (g_MapEditorMode == 0 &&
    !TechnoClass::IsOnScreen(cell, 1) &&
    !this->vtable[800]() &&
    *(char *)(this + 0x3D5) != 0) {
  return 7;
}
```

Hidden cells outside the visible viewport with `+0x3D5` flag set return blocked. `+0x3D5` is the **fog/shroud-restricted byte** (gates whether AI can plan into unobserved cells). This is **TS-legacy in YR** — `FogOfWar` defaults off, so `+0x3D5` is typically not set in standard YR.

### 2.8 Phase 8 — locomotor passability check

```c
iVar6 = FootClass::LocomotorPassabilityCheck(cell_ptr, direction, path_height, ...);
if (iVar6 == 7) return 7;
```

Calls `FootClass::LocomotorPassabilityCheck @ 0x4D9C10` — base implementation. (Different from the vtable+0x1AC slot at the same address; this is a *direct* call.) Returns 0/7. This checks SpeedType vs LandType matrix.

### 2.9 Phase 9 — overlay check (wall handling)

```c
if (cell.OverlayTypeIndex != -1) {
  overlay = g_OverlayTypeClass_Array[cell.OverlayTypeIndex];
  
  if (overlay.+0x2AA != 0 &&                                           // "OverlayHostile" or similar
      !HouseClass::IsPlayerControl() && g_GameMode == 0)
    return 7;
  
  if (overlay.+0x2A8 != 0) {                                           // "is a wall"
    // ... allied-wall logic, returns code 4 (FriendlyWall) or 5 (EnemyBlock)
  }
}
```

Walls/overlays here. Allied walls return code 4 (cost 60); enemy walls return code 5 (cost 20). The exact branching is intricate — see decomp for nuances.

### 2.10 Phase 10 — occupancy-list walk (the soft-block codes)

The post-vtable layer pick chooses which list to walk:

```c
if (uStack_80.byte3 == 0) {                                            // GROUND layer pre-decision
  piVar15 = cell.+0xE4;                                                // FirstObject (ground list head)
} else {
  piVar15 = cell.+0xE8;                                                // AltObject (bridge list head)
}

while (piVar15 != NULL) {
  if (piVar15 == this) {
    // We found ourselves in the cell — skip
    continue;
  }
  
  // ... pile of friendly-vs-enemy + transport-pickup + bunker-garrison + slave-tracker checks
  // Result: iVar6 set to 1 (Crushable), 2 (TemporaryBlock), 3 (ScatterRequired), 5 (EnemyBlock), or 6 (FriendlyStationary)
  // Pipeline keeps the MAX of all iVar6 values encountered
}
```

Eventually returns one of the codes 0-7 based on what was found. Civilian/crushable infantry → 1. Moving allied → 2. Allied building you can scatter → 3. Allied wall → 4. Enemy/cloaked enemy → 5. Stationary allied non-building → 6. Anything completely blocking → 7.

### 2.11 Verified return codes (from the cost-table consumer)

The 8 codes correspond to `g_AStar_EdgeCost_BaseTable @ 0x81870C` indexes (full table in companion costs report):

| Code | Cost | Meaning |
|------|------|---------|
| 0 | 1.0 | Clear |
| 1 | 1000.0 | Crushable (civilian, dog) |
| 2 | 1.0 | TemporaryBlock (moving friendly) — triggers blocker prediction in compute_edge_cost |
| 3 | 1.0 | ScatterRequired (allied building bump) |
| 4 | 60.0 | FriendlyWall |
| 5 | 20.0 | EnemyBlock |
| 6 | 8.0 | FriendlyStationary |
| 7 | 10000.0 | Impassable |

---

## 3. InfantryClass::Can_Enter_Cell (0x51BF90, unlabeled FUN_0051BF90)

Function size: 0x51BF90..0x51C882 (~2.3 KB). Signature mirrors UnitClass:

```c
int __thiscall FUN_0051BF90(TechnoClass *this, int cell_ptr, int direction, int path_height, uint flags);
```

### 3.1 Same pre-vtable layer pre-decision

The function opens with the **identical** layer pre-decision pattern from UnitClass §2.1 — copying the height-diff ≥ 2 + 0x100 gate. Same byte position in the `uStack_24` packed state.

### 3.2 Same tube case at direction == 8

Identical except the dummy-cell check is slightly different: `tube.+0x28.lo == tube.+0x24.lo && tube.+0x28.hi == tube.+0x24.hi` (test if start == end coord — degenerate tube).

### 3.3 Same vtable+0x1B0 dispatch

```c
iVar8 = (**(code **)(this->vtable + 0x1b0))(cell_ptr, direction, &path_height, &uStack_24.byte1);
```

Calls `CheckBridgeTraversal @ 0x4D9C60` (same as UnitClass for InfantryClass instances). Identical sub-check semantics.

### 3.4 Infantry-specific behaviour: the **early-return for high path bumps**

```c
if (path_height - cell.Level > 4) return 0;
```

Verified at `0x51C055..0x51C062`. **Infantry uniquely accept any cell where the path-height is more than 4 above the cell's ground level.** Vehicles never get this shortcut. Player-observable: infantry can "magically" path across high gaps where the path-height jumped above the deck (e.g., on bridge-collapse transitions); vehicles cannot.

### 3.5 Infantry-specific: garrison entry check

```c
if (target_cell_has_garrisonable_building) {
  cVar = BuildingClass::CanGarrison();
  if (!cVar) {
    if (allied) iVar8 = max(iVar8, 3);   // ScatterRequired
    else        iVar8 = max(iVar8, 5);   // EnemyBlock
  }
}
```

Infantry treat garrisonable buildings differently from vehicles. UnitClass also has garrison logic but routes it through a different code path.

### 3.6 Infantry-specific: weapon-range gate

```c
if (HouseClass::Is_Ally_ByIndex(unaff_EDI) == false) {
  iVar14 = TechnoClass::GetWeaponRange(this, -1);
  if (iVar14 < 1) return 7;                                              // ranged weapon required for hostile cell
  if (iVar8 < 5) return 5;
}
```

Infantry without a weapon (cf. engineer, civilian) get hard-blocked at enemy cells. Vehicles use a different mechanism.

### 3.7 Confidence

**C=HIGH** (full decompilation; structural correspondence with UnitClass is high), **I=MEDIUM** (function is `FUN_0051BF90` — name inferred from vtable position; no symbol), **B=HIGH** (single DATA xref from InfantryClass vtable+0x1AC at 0x7EB204; confirmed by reading memory).

---

## 4. AircraftClass::Can_Enter_Cell (0x415B10)

**The label is misleading.** This is NOT a per-cell passability predicate in the A* sense — it's an **8-direction landing-pad scanner with blocker-eviction**.

```c
undefined4 __thiscall AircraftClass::Can_Enter_Cell(int *this, int *target) {
  // Loop over the 8 directions (table at DAT_00817A58..0x817A78, 8 dword entries):
  for (int dir = 0; dir < 8; dir++) {
    coord = this->Get_Coord();                                            // vtable+0x1B8
    cell = MapClass::Get_CellClass(coord + g_DirectionOffsets[dir]);
    result = cell->vtable[0x1AC](...);                                    // recurse Can_Enter_Cell on neighbor
    if (result == 0) break;                                               // found a passable neighbor
  }

  // If we found a passable cell, command `target` to move out of the way:
  if (target->vtable[0xD8](pos_args, direction)) {
    target->vtable[0x1E8](2, 0);                                          // Set_Mission(MOVE)
    MapClass::Get_CellClass(target_coord);
    this->vtable[0x480](target_cell, 1);                                  // Set_Destination
    if (this->vtable[0x278](2, target) == 1) {
      this->vtable[0x274](9);                                             // Reset something
    }
    return 1;
  }
  return 0;
}
```

This function is called from helicopter/aircraft mission code, not from `AStar_main_loop` (aircraft don't use the same ground-A* path). It's used for "bump the unit blocking my landing pad". The semantics are entirely different from UnitClass::Can_Enter_Cell.

**Aircraft don't have a real Can_Enter_Cell in the A* sense** — they use altitude-based pathing through `FlyLocomotionClass` and a separate landing-cell selector via `FootClass::Find_Path`'s code-6/7 branches.

**The +0x1B0 slot for AircraftClass = `0x5F4B10` = `ObjectClass::DrawIt`** — inherited from ObjectClass, NOT overridden, and irrelevant to bridge logic. Aircraft never invoke a bridge sub-check.

**Confidence:** C=HIGH (decompiled), I=HIGH (Ghidra label "Can_Enter_Cell" placed but misleading; real semantic is landing-pad scanner), B=HIGH (single DATA xref from AircraftClass vtable+0x1AC at 0x7E23A4).

---

## 5. BuildingClass::Can_Enter_Cell (0x449440, unlabeled FUN_00449440)

Function size: 0x449440..0x4494B4 (only ~117 bytes). Signature:

```c
int __thiscall FUN_00449440(int this, int args_ptr);
```

Full decomp:

```c
param_2 = args_ptr.+0x24;                                                  // dest coord
BuildingTypeClass *type = *(int **)(this + 0x520);                         // BuildingClass.Type

if (type->+0x408 != 0 && this->+0x74 != 0) {
  // Specific code path for active-construction buildings
  speed_or_zone = *(uint *)(this + 0x21C);
  zone_ptr = type->+0x67C;
  MapClass::Get_CellClass(coord);
  passable = Cell_passability_building_placement(zone_ptr, type, speed_or_zone);
  return passable ? 0 : 7;
}

// Default path
passable = type->vtable[0xA8](coord, this->+0x21C);                        // BuildingTypeClass::CanPlaceAt
return passable ? 0 : 7;
```

**Buildings return only 0 (OK) or 7 (Impassable).** Never any of the soft codes 1-6 — buildings don't block based on friendly/enemy occupancy distinctions, they're either placeable or not.

This function isn't reached via standard A* (buildings don't path); it's reached when a building "Can_Enter_Cell" probe is needed for placement validation, which happens in:
- MCV deploy spots
- Engineer-capturable structures
- Some construction-yard footprint extension code

**Confidence:** C=HIGH, I=MEDIUM (function name inferred from vtable position; no symbol), B=HIGH (DATA xref from BuildingClass vtable+0x1AC at 0x7E4068).

---

## 6. LocomotionClass::Can_Enter_Cell (0x55ABF0) — confirmed stub

```c
undefined4 LocomotionClass::Can_Enter_Cell(void) {
  return 0;
}
```

**4-byte function** — `xor eax, eax; ret`. Returns 0 (= OK / Clear) unconditionally.

### 6.1 11 DATA xrefs — all to locomotor vtables

| Xref site | Locomotor (inferred) | Offset within vtable (Can_Enter_Cell slot for ILocomotion COM interface) |
|-----------|----------------------|---------------------------------------------------------------------------|
| `0x7E7ECC` | DriveLocomotionClass (vtable @ 0x7E7EB0) | +0x1C |
| `0x7E8294` | (Walk?) | +0x1C |
| `0x7E8A10` | (Hover?) | +0x1C |
| `0x7EAD18` | (JumpJet?) | +0x1C |
| `0x7ECD84` | (Teleport?) | +0x1C |
| `0x7EAE10` | (Fly?) | +0x1C |
| `0x7EDB88` | — | +0x1C |
| `0x7F0B38` | — | +0x1C |
| `0x7F2DA8` | — | +0x1C |
| `0x7F501C` | — | +0x1C |
| `0x7F6A14` | — | +0x1C |

All 11 locomotor classes inherit the empty stub. None override.

### 6.2 Is it really TS-dead, or just "always-true"?

The 2026-05-13 plan said "re-verify TS-dead claim." The body is `return 0` — but `0` is the **"OK / passable"** code in the Can_Enter_Cell convention. So the stub's effect when called is "this cell is fine, go ahead."

**Conclusion**: The function is **NOT TS-dead** — it's reachable via vtable dispatch from any locomotor's COM-interface Can_Enter_Cell call. It is, however, a **silent always-true gate** — the actual decision lives in the Unit/Infantry/Aircraft/Building class hierarchy's vtable+0x1AC slot (the Cells walk through a DIFFERENT vtable path).

This explains the name confusion: there are TWO different Can_Enter_Cell-shaped virtual functions:
- **TechnoClass-side `vtable+0x1AC`** = the A* per-cell predicate, returns 0-7.
- **LocomotionClass-side `vtable+0x1C` (ILocomotion COM)** = an unused always-OK stub.

The ILocomotion slot exists in the binary as a COM-interface contract requirement (every COM object must implement all its interface slots) but is **never overridden by any concrete locomotor**.

**Action:** Do NOT port this to Rust. It's pure COM-contract cruft. The "always OK" behaviour falls out for free if there's no locomotor-side cell-entry check at all.

**Confidence:** C=HIGH (4-byte body is unambiguous), I=HIGH (Ghidra label "LocomotionClass::Can_Enter_Cell"; matches usage), B=HIGH (11 DATA xrefs to locomotor vtables enumerated above; no caller in any function body found via callers).

---

## 7. Call chain summary (per object type)

For a **vehicle** path expansion:

```
AStar_main_loop @ 0x429F54
  CALL [vehicle->vtable + 0x1AC]
    → UnitClass::Can_Enter_Cell @ 0x73F0A0
        CALL [vehicle->vtable + 0x1B0]
          → CheckBridgeTraversal @ 0x4D9C60
            (returns 0/7, may update path_height)
        CALL FootClass::LocomotorPassabilityCheck @ 0x4D9C10
          (returns 0/7)
        ... walk cell+0xE4 / cell+0xE8 occupancy list ...
      returns 0/1/2/3/4/5/6/7
```

For an **infantry** path expansion:

```
AStar_main_loop @ 0x429F54
  CALL [infantry->vtable + 0x1AC]
    → InfantryClass::Can_Enter_Cell @ 0x51BF90 (unlabeled)
        CALL [infantry->vtable + 0x1B0]
          → CheckBridgeTraversal @ 0x4D9C60        (SAME as Unit)
        ... infantry-specific blocker walk ...
      returns 0/1/2/3/5/6/7
```

For a **building** placement probe:

```
BuildingTypeClass::CanPlaceAt or MCV-deploy check
  CALL [building->vtable + 0x1AC]
    → BuildingClass::Can_Enter_Cell @ 0x449440 (unlabeled)
        type->CanPlaceAt() or Cell_passability_building_placement
      returns 0 or 7 only
```

For an **aircraft** landing approach:

```
Aircraft mission code (Find_Approach_Cell, Landing dock select)
  CALL [aircraft->vtable + 0x1AC]
    → AircraftClass::Can_Enter_Cell @ 0x415B10 (the 8-dir landing scanner)
        Loops 8 dirs, calls neighbor.vtable[0x1AC]
        On hit: commands target to move out
      returns 0 or 1
  (NEVER calls vtable+0x1B0 — that slot inherits ObjectClass::DrawIt; aircraft don't do bridge sub-check)
```

---

## 8. Cross-doc contradictions resolved

### 8.1 Prior MEMORY entry `feedback_vtable_binding_verification`

The memory said "RE: every vtable-override claim must be confirmed by live read_memory; Ghidra labels alone are not sufficient (a wrong label survived for weeks in MISSION_ENTER docs)". **Applied here**: every claim about vtable+0x1AC / +0x1B0 in this report is backed by a live `read_memory` call. Verified:

- UnitClass `+0x1AC = 0x73F0A0`, `+0x1B0 = 0x4D9C60` (direct read at 0x7F5E1C / 0x7F5E20)
- InfantryClass `+0x1AC = 0x51BF90`, `+0x1B0 = 0x4D9C60` (direct read at 0x7EB204 / 0x7EB208)
- AircraftClass `+0x1AC = 0x415B10`, `+0x1B0 = 0x5F4B10` (direct read at 0x7E23A4 / 0x7E23A8)
- BuildingClass `+0x1AC = 0x449440`, `+0x1B0 = 0x4264D0` (direct read at 0x7E4068 / 0x7E406C)
- FootClass `+0x1AC = 0x4D9C10`, `+0x1B0 = 0x4D9C60` (direct read at 0x7E8E40 / 0x7E8E44)

### 8.2 Plan item #11 "InfantryClass::Can_Enter_Cell — find via vtable+0x1B0"

The plan said vtable+0x1B0 was the A* entry. **Refuted.** vtable+0x1B0 is the *bridge sub-check*; vtable+0x1AC is the A* entry. For InfantryClass:
- vtable+0x1AC = InfantryClass-specific A* entry at 0x51BF90 (unlabeled, full Can_Enter_Cell pipeline)
- vtable+0x1B0 = CheckBridgeTraversal (same as UnitClass — shared bridge logic across both)

### 8.3 Plan item #12 "AircraftClass::Can_Enter_Cell suspected to be landing-pad finder, not vtable Can_Enter_Cell"

**Confirmed.** AircraftClass.vtable+0x1AC IS the 8-direction landing-pad scanner. It's labeled "Can_Enter_Cell" in Ghidra but semantically does something quite different from the Unit/Infantry pipeline. **The label is misleading but the slot is correct.**

### 8.4 Plan item #14 "LocomotionClass::Can_Enter_Cell — re-verify TS-dead claim"

**Refined.** Not TS-dead per se — reachable via 11 locomotor vtables — but functionally an always-OK stub. No locomotor overrides it. Action for Rust: do not port.

---

## 9. Open Questions

1. **InfantryClass::Can_Enter_Cell formal name** — `FUN_0051BF90` should be relabeled in Ghidra to `InfantryClass__Can_Enter_Cell` after verification.
2. **BuildingClass::Can_Enter_Cell formal name** — `FUN_00449440` should be relabeled.
3. **AircraftClass.vtable+0x1B0 = ObjectClass::DrawIt** — confirm via ObjectClass vtable layout that this is genuine inheritance (not a Ghidra label mistake on the AnimClass__Click_stub-style misnaming).
4. **Cell field `+0x122`** — used in UnitClass::Can_Enter_Cell as `*(char *)(cell + 0x122) == '\0'` with `param_7 != '\0'` causing skip. Semantic unknown. Possibly "is non-water terrain" gate for amphibious checks.
5. **`Cell_passability_building_placement` body** — used by BuildingClass::Can_Enter_Cell. Not decompiled in this phase.
6. **`AircraftClass+0x817A58 table`** of 8 dword entries — verify these are pointers to DirectionOffsets or some other 8-element array.
7. **Phase-6 two-pass divergence** — exact reproduction conditions: which tick does the cell occupancy switch between layers? Requires step-trace in actual gameplay or sim test.
8. **TechnoClass+0xC94 ("TooBig"-ish flag)** — used at multiple sites in UnitClass::Can_Enter_Cell to gate cell-walk behaviour. Semantic confirmation needed (Phase 5 item #57 covers this).
9. **`field_0x3D5`** (fog/visibility gate) — confirm this is actually fog-related and TS-legacy.

---

## 10. Current Rust Implementation Status

| Binary feature | Rust file | Status |
|----------------|-----------|--------|
| Two-slot virtual structure (+0x1AC vs +0x1B0) | [src/sim/pathfinding/cell_entry.rs](../../ra2-rust-game/src/sim/pathfinding/cell_entry.rs) | Implemented as a single function rather than two-virtuals. Observable behaviour can be matched without porting the split, as long as the layer-decision gate (height-diff ≥ 2 + 0x100) and the sub-check (CheckBridgeTraversal logic) are correct. |
| UnitClass-specific 12-phase pipeline | [src/sim/pathfinding/cell_entry.rs](../../ra2-rust-game/src/sim/pathfinding/cell_entry.rs) | Partial — covers terrain + occupancy. Missing phases: tube collision check (#4), screen visibility gate (#7), specific overlay/wall handling (#9). |
| InfantryClass `path_height > cell.Level + 4` shortcut | none | **Missing.** Infantry parity bug: cannot path across high gaps where path_height jumped above deck. Triggers on bridge-collapse transitions. |
| InfantryClass weapon-range hostile-cell gate | none | **Missing.** Affects civilian/engineer behaviour at enemy positions. |
| Aircraft 8-direction landing-pad scanner with blocker-eviction | partial | `air_movement.rs` handles altitude, but the "scan 8 directions and command blocker to move" behaviour is not replicated. |
| Building-only 0/7 return | n/a | Buildings don't path in Rust — no equivalent function needed. |
| LocomotionClass return-0 stub | n/a | Don't port. |
| 8 return codes mapping to cost table | partial | Codes 0/1/2/5/6/7 mapped; codes 3 (ScatterRequired) and 4 (FriendlyWall) less clear. |

---

## 11. Sources

**Ghidra functions decompiled:**
- `UnitClass::Can_Enter_Cell` @ 0x0073F0A0 (~3.2 KB body)
- `InfantryClass::Can_Enter_Cell` @ 0x0051BF90 (FUN_0051BF90, ~2.3 KB body)
- `AircraftClass::Can_Enter_Cell` @ 0x00415B10
- `BuildingClass::Can_Enter_Cell` @ 0x00449440 (FUN_00449440, 117 bytes)
- `LocomotionClass::Can_Enter_Cell` @ 0x0055ABF0 (4-byte stub)
- `ObjectClass::UpdatePosition` @ 0x005F4410 (incidental — vtable+0x1A8)
- `ObjectClass::DrawIt` @ 0x005F4B10 (incidental — Aircraft vtable+0x1B0)
- `AnimClass__Click_stub` @ 0x004264D0 (4-byte stub, used as BuildingClass vtable+0x1B0)
- `FootClass::LocomotorPassabilityCheck` @ 0x004D9C10 (vtable+0x1AC for base FootClass)

**Memory reads (vtable slot verification):**
- 0x007F5E14..0x7F5E23 (UnitClass vtable +0x1A4..+0x1B7)
- 0x007EB200..0x7EB20F (InfantryClass vtable +0x1A8..+0x1B7)
- 0x007E239C..0x7E23AB (AircraftClass vtable +0x1A4..+0x1B3)
- 0x007E4064..0x7E4073 (BuildingClass vtable +0x1A8..+0x1B7)
- 0x007E8E3C..0x7E8E4B (FootClass vtable +0x1A8..+0x1B7)

**Constructor disassembly (vtable base discovery):**
- `InfantryClass::Constructor` @ 0x517A50 writes vtable=0x7EB058 at 0x517ACC
- `BuildingClass::Constructor` @ 0x43B680 writes vtable=0x7E3EBC at 0x43B71F
- `FootClass::Constructor` @ 0x4D31E0 writes vtable=0x7E8C94 at 0x4D345D
- `UnitClass::Destructor` @ 0x735780 writes vtable=0x7F5C70 at 0x735794

**Function search:**
- `search_functions("Can_Enter_Cell")` — 3 labelled functions only
- `search_functions("InfantryClass")` / `BuildingClass` / `AircraftClass` / `UnitClass` — verified each class has its constructor and key methods named

**Companion doc:**
- `BRIDGE_CHECK_TRAVERSAL_AND_CELL_OFFSETS_GHIDRA_REPORT.md` (CheckBridgeTraversal full decomp + CellClass offsets + cell-flag bit semantics)
- `BRIDGE_ASTAR_DUAL_CLOSED_LIST_GHIDRA_REPORT.md` (Phase 1 — A* spine + how vtable+0x1AC is dispatched from main loop)
