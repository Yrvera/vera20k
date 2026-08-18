# Refinery Dock Cell + Anim Helpers — Verified

> **Correction 2026-05-21 - DAT_0089F6A0 source**
>
> Later focused work supersedes this report's older OQ-2 conclusion. `DAT_0089F6A0`
> is the hardcoded west-neighbor direction-table entry `(-1,0)` initialized by
> `Foundation_direction_table_init @ 0x0049F2F0`; it is not written by
> `DockingOffset%d` parsing. `GetDockCellForObject` remains unrelated to the
> stock unload rediscovery lookup.

**Addresses:** 0x0044EFB0, 0x00451890, 0x00451750, 0x00451E40  
**Confidence:** HIGH — all four functions decompiled directly in this session via
`decompile_function`. Slot table offsets cross-checked against BuildingClass struct
(`get_struct_layout`) and prior verified docs. Vtable base confirmed via `read_memory`.  
**Active in YR:** Yes  

---

## 1. Overview

This report covers the four helper functions invoked from refinery dock code paths:

| Function | Address | Role |
|----------|---------|------|
| `BuildingClass::GetDockCellForObject` | 0x0044EFB0 | Returns exit/dock cell for a unit leaving a building |
| `BuildingClass::CreateAnimForSlot` | 0x00451890 | Instantiates (or replaces) the AnimClass* in a building's slot |
| `BuildingClass::SetAnimSlotImage` | 0x00451750 | Selects art name by health/flag, then calls CreateAnimForSlot |
| `BuildingClass::ClearAnimSlot` | 0x00451E40 | Destroys the AnimClass* in a slot and nulls the pointer |

Key structural fact (verified via `get_struct_layout`):
- `BuildingClass::Anims_0` = offset **0x55C** (1372 decimal).
- 21 slots total (0x15), each a 4-byte `AnimClass*` pointer.
- Slot `N` lives at `building + 0x55C + N*4`.
- Slot 10 = `building + 0x55C + 0x28 = building + 0x584`. ← **This confirms prior doc's `building+0x584 = slot-10 anim pointer`.**

BuildingClass vtable base: **0x007E3EBC** (verified: vtable+0x48 = `0x00447AC0` =
`BuildingClass::GetCoords`; vtable+0x1B8 = `0x0041BEA0` = `ObjectClass::Get_Cell_Packed`,
both read via `read_memory`).

---

## 2. GetDockCellForObject (0x0044EFB0) — Cell Math

**Vtable slot:** `vtable + 0x2D4` = address `0x007E4390`
(verified: `read_memory 0x007E4390` returns `0xB0EF4400` = `0x0044EFB0`;
`get_xrefs_to 0x0044EFB0` returns only `007e4390 [DATA]`).

**Callers:** Zero direct callers — only invoked via vtable dispatch.
(`get_function_callers 0x0044EFB0` → no results).

**Purpose:** Given a building (`this`), a candidate docker unit (`param_2`),
and a fallback cell (`param_3`), return the first passable cell for an exiting
or docking unit. This is the **production exit / dock placement oracle** — not
the refinery harvester pad cell (that comes from the hardcoded `+3,+1` in
Receive_Radio case 0x0E).

**Active in YR: Yes** — called via vtable whenever a unit exits a building.

### Reference Frame

All coordinates in this function are **NW-cell-indexed (cells, not leptons)**,
obtained by calling `vtable+0x1B8` = `ObjectClass::Get_Cell_Packed @ 0x0041BEA0`
which returns `(building.Location_X >> 8, building.Location_Y >> 8)` — the building's
NW-corner cell. All arithmetic is in cell-space. Per CLAUDE.md frame taxonomy: **Frame 2
(Get_Cell_Packed / NW cell)**.

`param_1[0x148]` in the decompile = `param_1 + 0x148*4 = param_1 + 0x520` =
`BuildingClass.Type` pointer (BuildingTypeClass*). All type flag reads use this base.

### Per-Docker-Type Branches (in order, checked top-to-bottom)

The function tries branches in strict priority order, returning immediately on the
first valid passable cell. `Cell_in_bounds + vtable+0x1AC (CanEnterCell)` is the
passability check at each candidate.

**Branch 1 — GDIBarracks (Type+0x16E4 ≠ 0):**

```
candidate = (NW.X + 1, NW.Y + 2)
CanEnterCell(candidate, -1, -1, 0, 1)   ← last arg=1: strict occupancy check
```

Allied barracks (GAPILE, 2×2) exits one east, two south of NW corner.

**Branch 2 — NODBarracks (Type+0x16E5 ≠ 0):**

```
candidate = (NW.X + 2, NW.Y + 2)
CanEnterCell(candidate, -1, -1, 0, 1)
```

Soviet barracks (NAHAND, 3×3) exits two east, two south.

**Branch 3 — YuriBarracks (Type+0x16E6 ≠ 0):**

```
candidate = (NW.X + 2, NW.Y + 1)
CanEnterCell(candidate, -1, -1, 0, 1)
```

Yuri barracks (YUBARX, 3×2) exits two east, one south.

**Branch 4 — Naval WeaponsFactory (Type+0xCCE ≠ 0 AND Type+0x16BD ≠ 0):**

```
exit_leptons = vtable+0xA8(unit)        ← GetExitCoord with unit param
cell.X = (exit_leptons.X + (exit_leptons.X >> 31 & 0xFF)) >> 8   ← sign-correct arithmetic shift
cell.Y = (exit_leptons.Y + (exit_leptons.Y >> 31 & 0xFF)) >> 8
try: (cell.X + 1, cell.Y + 1)   CanEnterCell(..., 0, 0)   ← last arg=0: less strict
try: (cell.X + 1, cell.Y    )   CanEnterCell(..., 0, 0)
try: (cell.X    , cell.Y + 1)   CanEnterCell(..., 0, 0)
```

Note: uses the building's exit coordinate (from `ExitCoord=` INI key, stored at
`BuildingTypeClass+0xEC8`), not purely the NW cell. Uses laxer passability check
(last arg 0) — naval unit is expected to go into water cells that might not block.

**Branch 5 — Fallback cell (param_3):**

```
if param_3 != INVALID_CELL (DAT_0089C818):
    CanEnterCell(param_3, -1, -1, 0, 0)   ← lax check
    return param_3 if passable
```

The `DAT_0089C818` sentinel is checked by comparing both the X and Y shorts.

**Branch 6 — Exit list or foundation perimeter scan (final fallback):**

Reads `Type+0xED4` (ExitList pointer — pointer into global foundation exit table).

*If ExitList pointer is NULL OR building is Hospital (Type+0x16C1 ≠ 0):*

Foundation perimeter scan in two passes:
```
// Pass A — top/bottom rows: X from -1 to GetFoundationWidth() inclusive
for x in (-1 ..= width):
    try: (NW.X + x, NW.Y + GetFoundationHeight())   CanEnterCell(..., 0, 1)
    try: (NW.X + x, NW.Y - 1                     )   CanEnterCell(..., 0, 1)

// Pass B — left/right columns: Y from -1 to GetFoundationHeight() inclusive
for y in (-1 ..= height):
    try: (NW.X + GetFoundationWidth(), NW.Y + y)   CanEnterCell(..., 0, 1)
    try: (NW.X - 1,                   NW.Y + y)   CanEnterCell(..., 0, 1)
```

*If ExitList pointer is non-NULL:*

Iterates `short[2]` pairs `{dx, dy}` terminated by `{0x7FFF, 0x7FFF}`:
```
for (dx, dy) in ExitList until (0x7FFF, 0x7FFF):
    candidate = (NW.X + dx, NW.Y + dy)
    CanEnterCell(candidate, -1, -1, 0, 0)   ← lax check
    return candidate if passable
```

**If nothing found:** Returns `DAT_0089C818` = INVALID_CELL sentinel.

### Concrete Fixture — GAREFN 4×3 at NW (10, 10)

- NW cell = (10, 10)
- Type+0x16E4 = 0 (not GDI barracks) → Branch 1 skipped
- Type+0x16E5 = 0 → Branch 2 skipped
- Type+0x16E6 = 0 → Branch 3 skipped
- Type+0xCCE = 0, Type+0x16BD = 0 → Branch 4 skipped
- `param_3` = INVALID_CELL for most production calls → Branch 5 skipped
- Type+0xED4: GAREFN has a non-null ExitList (foundation exit table entry for 4×3)
- Hospital = 0 → uses ExitList path
- Returns first passable cell from the ExitList `{dx, dy}` table

**For a refinery, GetDockCellForObject is the production exit oracle (used when
a unit leaves the refinery after being built there, not for harvester docking).
Harvester pad placement uses the hardcoded `(NW.X+3, NW.Y+1)` from
Receive_Radio case 0x0E directly — see §3 below.**

### Cross-Check with Receive_Radio case 0x0E Queue Cell

The `(NW.X+3, NW.Y+1)` queue cell formula in Receive_Radio case 0x0E is
**NOT computed by GetDockCellForObject**. It is inline in case 0x0E:

```c
// From RECEIVE_RADIO_CASE_0x0E doc (verified prior session):
uStack_8 = (int *)CONCAT22(psVar5[1] + 1,   // Y += 1
                            *psVar5 + 3);    // X += 3
MapClass__Get_CellClass(&uStack_8);
```

`GetDockCellForObject` is called via vtable dispatch from production code paths
(ExitObject, GrandOpening, etc.). It is NOT involved in harvester docking.

### DAT_0089F6A0 Connection

`DAT_0089F6A0` does NOT appear in GetDockCellForObject. It appears in
`Mission_Deploy_Building` (0x0073D630) as the dock-cell offset used to locate
the refinery from the harvester's current cell position (see
MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md §3a/3c):

```
0073E013: GetMapCell() + offset DAT_0089F6A0 → dock offset cell
0073E2C8: GetMapCell() + DAT_0089F6A0 offset → dock cell
```

Superseded 2026-05-21: the prior Phase 1 doc's claim that `DAT_0089F6A0`
comes from `[GAREFN] DockingOffset0=` / artmd.ini load-time parsing was wrong.

**OQ-2 superseded resolution:** `DAT_0089F6A0` is initialized by
`Foundation_direction_table_init @ 0x0049F2F0` as the west-neighbor offset
`(-1,0)`. It is not computed inside GetDockCellForObject and not loaded from
`Type+0x1788` / `DockingOffset%d`.

### DockingOffset Array (Type+0x1788) — Does GetDockCellForObject Use It?

**No.** The decompile of GetDockCellForObject contains no reference to
`Type+0x1788`. The DockingOffset array is consumed only by `GetDockCoord`
(0x447B20) for Helipad/UnitRepair buildings. This is consistent with the
NUMBEROFDOCKS_VS_DOCKOFFSET_RECONCILE_GHIDRA_REPORT.md verdict: the
`+0x1788` array is never touched by harvester or refinery code.

---

## 3. CreateAnimForSlot (0x00451890)

**Verified via** `decompile_function 0x00451890`.
**Active in YR: Yes** (24 callers confirmed via `get_function_callers 0x00451890`).

**Signature (reconstructed):**
```c
void BuildingClass::CreateAnimForSlot(
    BuildingClass *this,    // ECX (thiscall)
    int slot_index,         // in_stack_00000004 (first stack arg)
    int type_slot_index,    // in_stack_00000008 (second stack arg — same as slot_index in practice)
    bool low_health,        // in_stack_0000000c (bool: select damaged art variant)
    undefined4 extra_arg    // in_stack_00000010 (passed to AnimClass constructor)
);
```

Ghidra decompiled this as `__thiscall BuildingClass__CreateAnimForSlot(BuildingClass *this)` with
the remaining args as `in_stack_*` because the function uses a non-standard calling convention
or the decompiler lost type info on the stack args. The two slot-index args appear to be the
same value passed twice for different purposes (art lookup vs. slot array indexing).

### Slot Index Range

The loop `while (iVar9 < 0x15)` confirms **21 slots (0..20 inclusive)**. This matches
`Anims_0` through `Anims_20` (21 × 4 bytes = 84 bytes starting at 0x55C).

### Slot Field Layout (BuildingTypeClass)

Each slot entry is `0x44` bytes in BuildingTypeClass, starting at offset `0xF4C`.
Slot N starts at `Type + 0xF4C + N * 0x44`.

| Slot-relative offset | Content | How used in CreateAnimForSlot |
|---------------------|---------|-------------------------------|
| +0x00 (= Type+0xF4C+N*0x44) | Undamaged anim name (char*) | Selected when low_health=false, extra=false |
| +0x10 (= +0xF5C+N*0x44) | Damaged anim name | Selected when low_health=true |
| +0x20 (= +0xF6C+N*0x44) | Firing anim name | Selected when extra=true |
| +0x30 (= +0xF7C+N*0x44) | AnimTypeClass* (resolved at load) | `this->Type + iVar10 + 0xF7C` read to get type |
| +0x34 (= +0xF80+N*0x44) | Draw X offset | `this->Type + iVar10 + 0xF80` → `puVar4+0x100` |
| +0x38 (= +0xF84+N*0x44) | Draw Y offset | `this->Type + iVar10 + 0xF84` → `puVar4+0x100` (packed) |
| +0x3C (= +0xF88+N*0x44) | ZAdjust/YSort | `this->Type + iVar10 + 0xF88` → `puVar4+0x104` |
| +0x40 (= +0xF8C+N*0x44) | XXXPoweredEffect flag | `this->Type[iVar10 + 0xF8C]` — if set, calls FUN_00425260 |

(Cross-confirmed with REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md which lists `+0x34=X,Y offset`,
`+0x38=ZAdjust`, `+0x3C=LoopCount/YSort` — prior doc had a slight offset numbering difference,
using +0x34/+0x38 for the pair; `this->Type + iVar10 + 0xF7C` = slot+0x30 for AnimTypeClass*,
slot+0x34 for draw offset, slot+0x38 for extra, confirmed.)

### Damaged-State Mass Swap (preamble)

**Before** creating the new anim for the requested slot, CreateAnimForSlot checks
if the building's damage state changed (`this->IsDamaged != low_health`). If so,
it iterates all 21 slots and calls `SetAnimSlotImage(i, low_health, 0, 0)` for
every occupied slot. This ensures all visible anims swap to their damaged/undamaged
variants atomically when health crosses `ConditionYellow`.

```c
if (this->IsDamaged != low_health) {
    this->IsDamaged = low_health;
    for (int i = 0; i < 21; i++) {
        if (this->Anims_0[i] != NULL)
            BuildingClass__SetAnimSlotImage(i, low_health, 0, 0);
    }
}
```

### Existing Slot Replacement

```c
if (this->Anims_0[slot_index] != NULL) {
    new_anim->field_0xac = old_anim->field_0xac;   // preserve storage-tier field
    this->Anims_0[slot_index] = NULL;
    old_anim->vtable->Destroy(1);                  // vtable+0x20 = Destroy
}
this->Anims_0[slot_index] = new_anim;
```

**If the slot is already occupied: the old anim is destroyed (not skipped, not
crashed).** The `field_0xac` value is preserved from old to new — this carries over
the storage-tier index when slot-10 is recycled on each bale pulse.

### Anim Construction

```c
pvVar3 = operator_new(0x1C8);    // 456 bytes = AnimClass size
puVar4 = AnimClass__Constructor(
    g_AnimTypes_Array[AnimTypeClass__FindByIndex()],
    &position_struct,
    extra_arg,
    1,        // visible
    0x1600,   // layer/flags
    0, 0
);
```

### Translucency Propagation

After creating the new anim, iterates all 21 slots and sets `anim->field_0x178 =
this->Translucency` on every occupied slot. If `Translucency == 0xF` and house
side == 5 (Yuri), overrides to `0x10`. This ensures cloaking translucency is
applied to all building anims uniformly whenever any slot changes.

### Slot-9 Special Case (TurretAnim)

```c
if (slot_index == 9 && this->Type[0x16C6] != 0) {
    new_anim->field_0x19D = 1;
}
```

Slot 9 = TurretAnim. `Type+0x16C6` is an unidentified flag (possibly `HasTurretAnim`).
When set, the new anim gets `+0x19D = 1`.

### Shroud/Visibility Propagation

The final block checks if the anim type has a shroud-related flag
(`*(anim_type + 0x35C) != 0`). If set, copies visibility/owner state from the
building to the new anim, using vtable+0x1E4 (GetOwnerMask?), vtable+0x1BC
(GetCell?), and vtable+0x464 (ShroudLevelAt?). This propagates shroud level so
new anims respect the current fog state at the building's position.

### Callers (from `get_function_callers 0x00451890`)

24 callers including: `SetAnimSlotImage`, `UpdateAnimation`, `GrandOpening`,
`OnConstructionComplete`, `OnPowerOn`, `OnPowerOff`, `ReleaseDockedHarvester`,
`Receive_Radio`, `AddUpgrade`, `SetDamagedState`, `UpdateGapAndSpecialEffects`,
`UpdateRepairAndPower`, `TriggerSpecialAnims`, and 10 `FUN_*` helpers.

---

## 4. SetAnimSlotImage (0x00451750)

**Verified via** `decompile_function 0x00451750`.
**Active in YR: Yes.**

**Full decompile (small function — showing complete logic):**

```c
void __thiscall BuildingClass__SetAnimSlotImage(
    BuildingClass *param_1,
    int param_2,    // slot index (0..20)
    char param_3,   // low_health flag (true = select damaged art)
    char param_4    // firing flag (true = select firing art)
) {
    char *pcVar1;
    if (param_3 == '\0') {
        if (param_4 == '\0') {
            pcVar1 = param_1->Type + param_2 * 0x44 + 0xF4C;  // undamaged name
        } else {
            pcVar1 = param_1->Type + param_2 * 0x44 + 0xF6C;  // firing name
        }
    } else {
        pcVar1 = param_1->Type + param_2 * 0x44 + 0xF5C;      // damaged name
    }
    if ((pcVar1 != NULL) && (*pcVar1 != '\0')) {
        BuildingClass__CreateAnimForSlot(param_1, ...);
    }
}
```

**Key findings:**

1. **Art variant selection:** `param_3` (low_health) picks the damaged name (+0x10 in slot),
   `param_4` (firing) picks the firing name (+0x20 in slot), default picks undamaged (+0x00).
   When both are false, selects undamaged = `Type + slot*0x44 + 0xF4C`.

2. **Empty-name gate:** Checks `pcVar1 != NULL && *pcVar1 != '\0'`. An empty anim name
   string causes early return — this is the mechanism that makes slot 7 and slot 8 calls
   no-ops on stock refineries (which define no `PreProductionAnim` or `ProductionAnim`).

3. **Difference from CreateAnimForSlot:** SetAnimSlotImage is the **art-variant selector**.
   It does NOT directly manipulate the slot array — it just resolves which name string to
   use and delegates to CreateAnimForSlot. CreateAnimForSlot does the actual allocation,
   replacement, and array write.

4. **Callers** (from `get_function_callers 0x00451750`):
   `CreateAnimForSlot` (recursive — the damage mass-swap preamble), `FUN_00519880`,
   `UnitClass::Mission_Deploy_Building`, `UnitClass::PerCellProcess`.

### Confirmation of Phase 1 slot 4 Claims

Slot calls from `Mission_Deploy_Building`:
- Slot 7 (PreProductionAnim): `PUSH 0x7; CALL 0x00451750` — confirmed.
- Slot 10 (SpecialAnim): `PUSH 0xA; CALL 0x00451750` — confirmed.
- Slot 8 (ProductionAnim): `PUSH 0x8; CALL 0x00451750` — confirmed.

All calls pass `param_3 = health <= ConditionYellow` (low_health flag) and `param_4 = 0`.

---

## 5. ClearAnimSlot (0x00451E40)

**Verified via** `decompile_function 0x00451E40`.
**Active in YR: Yes** (20 callers confirmed).

**Full decompile:**

```c
void __thiscall BuildingClass__ClearAnimSlot(BuildingClass *this, int slot_index) {
    if (slot_index == -2) {
        // Clear ALL 21 slots
        for (int i = 0; i < 21; i++) {
            AnimClass *anim = this->Anims_0[i];
            if (anim != NULL) {
                this->Anims_0[i] = NULL;
                anim->vtable->Destroy(1);   // vtable+0x20
            }
        }
        return;
    }
    // Clear single slot
    AnimClass *anim = this->Anims_0[slot_index];
    if (anim != NULL) {
        this->Anims_0[slot_index] = NULL;
        anim->vtable->Destroy(1);
    }
}
```

**Key findings:**

1. **Slot index semantics:** Normal range 0..20. Special value **-2 clears all 21 slots**.
   (Not -1 — the prior Phase 1 doc mentioned `PUSH 0xA` = slot 10; that confirms slot 10
   for SpecialAnim clearing. The -2 sentinel is the "clear everything" path, used by
   Destructor and power-related teardown.)

2. **Null-slot safety:** If `Anims_0[slot_index] == NULL`, returns silently — no crash.

3. **Destroy semantics:** Always calls `Destroy(1)` (not 0). The `1` argument means
   "delete self" — the AnimClass self-destructs. After Destroy, the slot pointer is
   already nulled by the caller (the NULL write precedes the Destroy call).

4. **Callers** (from `get_function_callers 0x00451E40`): `Destructor`, `OnPowerOff`,
   `OnPowerOn`, `UpdateAnimation`, `ReleaseDockedHarvester`, `Receive_Radio`,
   `RemoveLastUpgrade`, `AddUpgrade`, `UpdateGapAndSpecialEffects`, `TechnoClass::Set_Destination`,
   `Mission_Deploy_Building`, `PerCellProcess`, and others.

---

## 6. Definitive Slot Index → Field Offset → INI Key Table

**BuildingTypeClass slot table base:** `Type + 0xF4C`. Each slot = 0x44 bytes.
Slot N starts at `Type + 0xF4C + N * 0x44`.

BuildingClass `Anims_0` array base: `building + 0x55C`. Slot N AnimClass* at `building + 0x55C + N*4`.

Sources for slot↔INI key: REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md (verified via
`LEA EDX,[EBP + offset]` instructions in BuildingTypeClass::ReadINI at 0x45FE50).
Sources for AnimClass pointer offsets: derived from `Anims_0 = 0x55C` + slot*4 (get_struct_layout).

| Slot | Type offset (0xF4C+N*0x44) | Anim ptr (building+0x55C+N*4) | INI Key | Semantic |
|------|---------------------------|-------------------------------|---------|----------|
| 0 | 0x0F4C | building+0x55C | (unnamed) | ActiveAnim base (slot 0) |
| 1 | 0x0F90 | building+0x560 | (unnamed) | — |
| 2 | 0x0FD4 | building+0x564 | (unnamed) | — |
| 3 | 0x1018 | building+0x568 | `ActiveAnim` | Looping conveyor/primary active |
| 4 | 0x105C | building+0x56C | `ActiveAnimTwo` | Looping active 2 |
| 5 | 0x10A0 | building+0x570 | `ActiveAnimThree` | Looping active 3 |
| 6 | 0x10E4 | building+0x574 | `ActiveAnimFour` | Looping active 4 |
| 7 | 0x1128 | building+0x578 | `PreProductionAnim` | Dock arrival one-shot |
| 8 | 0x116C | building+0x57C | `ProductionAnim` | Cargo-empty one-shot |
| 9 | 0x11B0 | building+0x580 | `TurretAnim` | Turret rotation anim |
| 10 | 0x11F4 | building+0x584 | `SpecialAnim` | Per-bale one-shot; storage tier display |
| 11–17 | 0x1238–0x13D0 | 0x588–0x5A0 | (various) | — |
| 18 | 0x1414 | building+0x5A4 | `IdleAnim` | Idle (e.g., Yuri slave miner) |
| 19–20 | 0x1458–0x149C | 0x5A8–0x5AC | (unnamed) | — |

Slots 0–2 and 11–17 are present in the 21-slot array but not documented in prior
research. INI keys for those slots are unknown (not investigated in this session).

**Confirmed:** `building+0x584 = Anims_0[10]` = SpecialAnim AnimClass pointer.
The `Mission_Deploy_Building` check `building+0x584 == 0` tests whether slot 10
is currently unoccupied before triggering per-bale SetAnimSlotImage(10).

---

## 7. Tiny Details

### Anim Slot Bounds

- 21 slots (0..20). No bounds check inside ClearAnimSlot or CreateAnimForSlot —
  caller is responsible for passing a valid index. Passing slot > 20 would corrupt
  adjacent memory (out-of-bounds Anims_0 write).

### Null Handling

- `SetAnimSlotImage`: returns without calling CreateAnimForSlot if the art name pointer
  is null or the name string is empty. Safe to call for slots with no defined art.
- `ClearAnimSlot`: silently no-ops if slot pointer is already NULL.
- `CreateAnimForSlot`: early-returns if `AnimTypeClass__FindByIndex()` returns -1
  (anim type not registered). This guards against undefined anim names crashing.

### building+0x584 (Slot 10 Pointer)

`building + 0x584 = Anims_0[10]` (4-byte AnimClass* pointer). VERIFIED via struct layout:
`0x55C + 10*4 = 0x55C + 0x28 = 0x584`. The prior doc's RESOLVED finding (OQ 9.2) is correct.

The `Mission_Deploy_Building` per-bale gate:

```asm
0073E384: MOV EAX, [EDI + 0x584]   ; read slot-10 pointer
0073E38C: JNZ 0x0073E3BF            ; if non-null → skip slot-10 trigger
```

Meaning: slot-10 SpecialAnim call is skipped if there is already an anim in slot 10.
Per prior doc (OQ 9.2): `UpdateAnimation` can populate slot 10 for storage-tier display
on refineries (gated on `Type+0x16A8`). If that path populates slot 10, the per-bale
trigger is suppressed. For stock refineries where `Type+0x16A8` = 0 (not HasStorage or
not the relevant flag), slot 10 is null between bales and the trigger fires normally.

### building+0x57C (Slot 8 Pointer — ProductionAnim)

`building + 0x57C = Anims_0[8]`. Not explicitly discussed in prior docs but follows
the same layout. No special treatment observed.

### field_0x6E7 (Shroud/Cloaked Flag)

`this->field_0x6e7` is checked in CreateAnimForSlot:
```c
if (this->field_0x6e7 != '\0') {
    new_anim->field_0x199 = 1;
}
```
Sets the new anim's `+0x199` flag when the building is cloaked/hidden. Byte at
`building + 0x6E7`. Semantic: "cloaked" — the new anim inherits the invisibility
state so it doesn't flash visible on slot replacement.

### AnimClass Constructor Args (0x1C8 = 456 bytes)

```c
AnimClass__Constructor(anim_type_ptr, &position, extra_arg, 1, 0x1600, 0, 0)
```

Arg 4 = `1` (visible), arg 5 = `0x1600` (layer/z-flags), arg 6-7 = 0. The `0x1600`
layer value is a constant for all building slot anims — active in YR: Yes.

### Damaged Mass-Swap Trigger

CreateAnimForSlot's preamble compares `this->IsDamaged` (verified field at
`building + 0x6E6` from struct — `IsDamaged` is `bool` at offset 1766 in the struct layout)
against `low_health`. On mismatch, ALL 21 occupied slots are re-created with the new health
state before the requested slot is created. This means a single `SetAnimSlotImage(10, true, 0)`
call can trigger 20 additional anim recreations (one per occupied slot). The total cost is
bounded by occupied-slot count, not a constant.

---

## 8. Diffs vs Prior Docs

| Prior claim | This report's finding | Delta |
|-------------|----------------------|-------|
| GetDockCellForObject computes the harvester pad cell | **WRONG** — it is the production exit oracle. Harvester queue cell is hardcoded in Receive_Radio case 0x0E. | Substantive correction |
| GetDockCellForObject consumes DockingOffset (Type+0x1788) | **WRONG** — it reads ExitList (Type+0xED4) and GetFoundationWidth/Height, not DockingOffset. | Substantive correction |
| DAT_0089F6A0 might be computed in GetDockCellForObject (OQ-2 in Phase 1 slot 4) | **RESOLVED / SUPERSEDED 2026-05-21** — DAT_0089F6A0 appears in Mission_Deploy_Building and is initialized by `Foundation_direction_table_init @ 0x0049F2F0` as `(-1,0)`, not by DockingOffset parse. GetDockCellForObject is unrelated. | OQ closed |
| building+0x584 = "anim invisible" (tentative) | **WRONG label** — it is `Anims_0[10]`, the slot-10 AnimClass pointer. Verified by struct layout + arithmetic. | Confirmed per OQ 9.2 resolution |
| Slot range is 0..10 or 0..15 (ambiguous) | **CONFIRMED: 0..20 (21 slots)** — `while (iVar9 < 0x15)` in ClearAnimSlot and CreateAnimForSlot. | Confirmed |
| ClearAnimSlot with `PUSH 0xA` clears slot 10 | **CONFIRMED**. Also: -2 is the "clear all 21" sentinel. | Extended |
| CreateAnimForSlot replaces occupied slot | **CONFIRMED** — preserves `field_0xac` (storage tier) from old to new. | Extended |

---

## 9. Open Questions — Final State

| OQ | Status | Finding |
|----|--------|---------|
| OQ-2 (Phase 1 slot 4): DAT_0089F6A0 source | **CLOSED / SUPERSEDED 2026-05-21** — used by Mission_Deploy_Building and initialized by `Foundation_direction_table_init @ 0x0049F2F0` as `(-1,0)`; not written by DockingOffset load-time parse. GetDockCellForObject is unrelated. |
| Type+0x16A8 identity (HasStorage vs HasTurretAnim) | **STILL OPEN** — decompile of CreateAnimForSlot does not resolve this. The field gates slot-10 creation in UpdateAnimation. Needs a targeted trace of BuildingTypeClass::ReadINI for +0x16A8 to identify its INI key. |
| Slots 0–2 and 11–17 INI key mapping | **OPEN** — not investigated. Not needed for refinery dock parity. |
| Type+0x16C6 identity (slot-9 special flag) | **OPEN** — set in CreateAnimForSlot slot-9 branch. Likely `HasTurretAnim`. Not blocking. |
| `field_0x6E7` exact semantic on BuildingClass | **OPEN** — used as "cloaked/hidden" flag here; not fully traced. |
| Per-bale timer increment site (`unit+0xF8`) | **ALREADY RESOLVED** in REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md §9.1. |

---

## Sources

- `decompile_function 0x0044EFB0` — GetDockCellForObject full decompile (this session)
- `decompile_function 0x00451890` — CreateAnimForSlot full decompile (this session)
- `decompile_function 0x00451750` — SetAnimSlotImage full decompile (this session)
- `decompile_function 0x00451E40` — ClearAnimSlot full decompile (this session)
- `get_struct_layout BuildingClass` — confirmed `Anims_0 = 0x55C`, `IsDamaged = 0x6E6` (this session)
- `read_memory 0x007E3F04` → `0x00447AC0` (BuildingClass::GetCoords at vtable+0x48) — vtable base confirmation
- `read_memory 0x007E4074` → `0x0041BEA0` (Get_Cell_Packed at vtable+0x1B8) — vtable base confirmation
- `read_memory 0x007E4390` → `0x0044EFB0` (GetDockCellForObject at vtable+0x2D4) — vtable slot confirmation
- `get_xrefs_to 0x0044EFB0` → only `007e4390 [DATA]` — confirms vtable-only dispatch
- `get_function_callers 0x00451890` — 24 callers listed
- `get_function_callers 0x00451750` — 4 callers listed
- `get_function_callers 0x00451E40` — 20 callers listed
- Prior docs read: REFINERY_DOCK_ANIM_SLOTS_GHIDRA_REPORT.md, BUILDING_DOCKING_SYSTEM_GHIDRA_REPORT.md,
  RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md, NUMBEROFDOCKS_VS_DOCKOFFSET_RECONCILE_GHIDRA_REPORT.md,
  MISSION_DEPLOY_BUILDING_REFINERY_UNLOAD_GHIDRA_REPORT.md
