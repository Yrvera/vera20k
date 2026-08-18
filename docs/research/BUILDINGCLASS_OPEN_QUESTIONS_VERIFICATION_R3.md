---
name: BuildingClass Round-3 ExitObject Unknowns Verification
description: Binary verification of 5 ExitObject unknowns from Round 2 (Kind enum complete, Naval water-cell search, ClearBibArea, HasFreeRadioSlot precondition, AI build queue format at Owner+0x5704)
type: reference
---

# BuildingClass Round-3 — ExitObject Unknowns Resolved

**Date:** 2026-04-19
**Binary:** gamemd.exe
**Confidence:** HIGH — all 5 findings verified from direct decompilation
**Active in YR:** Yes — all findings apply to standard YR skirmishes

Resolves the 5 remaining ExitObject unknowns from
`BUILDINGCLASS_OPEN_QUESTIONS_VERIFICATION_R2.md` section 6.

---

## 1. Kind enum (vtable slot 11 / `What_Am_I`) — full enumeration

| Value | Class | Source |
|---|---|---|
| 1 | `UnitClass` | `0x00746E20` — `return 1;` |
| 2 | `AircraftClass` | (inferred from ExitObject case 2; not directly decompiled this round) |
| 6 | `BuildingClass` | `0x00459EC0` — `return 6;` |
| 0xF | `InfantryClass` | `0x00523340` — `return 0xF;` |

Slot 11 is the **class-kind tag** (`What_Am_I`), distinct from slot 8 which is
`AbstractClass::WhatAmI` returning an instance type-index. The slot 11 value
is a constant per class; ExitObject's top-level switch uses it as the
dispatch key.

**In ExitObject**: `case 1` (Unit) and `case 0xF` (Infantry) share the common
exit-cell tail; `case 2` has a dedicated aircraft path; `case 6` has a
dedicated building-from-building path.

## 2. Naval exit logic lives in `GetDockCellForObject` (0x0044EFB0)

**NOT** in ExitObject — the exit-cell selection is delegated to this helper
before ExitObject attempts the Unlimbo. Dispatch order inside
`GetDockCellForObject`:

| # | Condition | Cell(s) Tried |
|---|---|---|
| 1 | `Type+0x16E4` (GDI Barracks) | foundation_origin + (+1, +2) |
| 2 | `Type+0x16E5` (NOD Barracks) | foundation_origin + (+2, +2) |
| 3 | `Type+0x16E6` (Yuri Barracks) | foundation_origin + (+2, +1) |
| 4 | `Type+0xCCE` Naval AND `Type+0x16BD` WeaponsFactory | 3 adjacent water cells (see below) |
| 5 | Caller-provided fallback cell (`param_3`) if non-sentinel | that cell |
| 6 | `Type+0xED4` null OR `Type+0x16C1` Hospital | foundation perimeter scan (bottom → top → right → left) |
| 7 | Default: `Type+0xED4` exit list | walk `{dx, dy}` DWORD pairs until `0x7FFF, 0x7FFF` sentinel |

### Naval Yard (rule 4) — the 3 water cells

```
dock_coord = this->vtable[0xA8](out_coord, unit);   // get base dock pos
sVar5 = (dock.x + sign_adj) >> 8    // dock cell X
sVar3 = (dock.y + sign_adj) >> 8    // dock cell Y
sVar4 = sVar5 + 1                   // one cell east
```

Candidate cells (first free wins):

1. `(sVar4,     sVar3 + 1)` = (dock_x + 1, dock_y + 1)  — southeast
2. `(sVar4,     sVar3)`     = (dock_x + 1, dock_y)       — east
3. `(sVar5,     sVar3 + 1)` = (dock_x,     dock_y + 1)   — south

Each candidate is validated via `vtable[0x1AC]` (Is cell enterable by this
unit? returns 0 if yes).

### Fallback

If all candidates fail, function writes `DAT_0089C818` (the invalid/sentinel
coord) to the return slot. Caller (ExitObject) sees this and fails the exit
(returns 0).

### ExitObject's relationship to GetDockCellForObject

The WF ground exit path in ExitObject does NOT call GetDockCellForObject
directly. Instead:
- Naval Yard uses the 3-water-cells dispatch via other paths (likely the
  "fallback cell" flow or through `GetDockCoord`).
- Ground WF uses inline atan2 direction math + foundation-edge cell pick +
  Unlimbo.

The atan2 + foundation-edge logic in ExitObject:
```c
dock = this->GetCoords();                      // building center (lepton)
target = someTargetCell;                       // usually from ExitCoord
rad = atan2(target.y - dock.y, target.x - dock.x);
facing = (ftol(rad) >> 7 + 1) >> 1 & 0xFF;    // 0-255 facing

// Step the chosen cell from foundation-origin toward target:
//   if target.x >= foundation.x + width: cell.x -= 1
//   else if target.x < foundation.x:     cell.x += 1 (clamp within ±1 step)
// (same for y)

// Apply GDI/NOD/Yuri ExitCoord offset if building has that flag AND
// target is in the specific foundation quadrant (1,2)/(2,2)/(2,1):
cell.x += Type+0xEC8 (ExitCoord.X);
cell.y += Type+0xECC (ExitCoord.Y);
cell.z += Type+0xED0 (ExitCoord.Z);

Unlimbo(unit, cell, facing);
```

## 3. `ClearBibArea` (0x00449540) — WF bib scatter helper

Called from ExitObject before attempting vehicle exit. Purpose: scatter any
units currently standing in the WF's bib zone so the exiting vehicle has a
clear path.

**Gated by:** `Type+0x16BD` (WeaponsFactory flag). Returns 0 for non-WF.

**Algorithm:**
```c
bib_cell = this->GetFoundationOrigin() + Type->ExitList[0x28];
iVar = CellClass::Find_Nearest_Object(bib_cell, 0, this);
if (iVar == NULL) return 0;   // bib clear

CellClass::Scatter_Objects(&DAT_0089C848, 1, 1, 0);   // scatter
for (i = 0; i < 8; i++) {
    Pathfinding_update_continued(i);
    iVar = CellClass::Find_Nearest_Object(bib_cell, 0, this);
    if (iVar != NULL) CellClass::Scatter_Objects(bib_cell, 1, 1, 0);
}
return 1;
```

Up to 8 scatter retries with pathfinding steps interleaved. Returns 1 if any
scatter occurred (bib was occupied).

## 4. `FUN_0065ADC0` = `RadioClass::HasFreeSlot` (0x0065ADC0)

Simple polymorphic-array slot check:

```c
uint RadioClass__HasFreeSlot(int *this) {
    int count = this[0xE8/4];         // +0xE8 = Contact count
    int *items = this[0xE4/4];         // +0xE4 = Contacts array ptr
    for (int i = 0; i < count; i++) {
        if (items[i] == NULL) return 1;
    }
    return 0;
}
```

Returns 1 if at least one radio slot is null (available), 0 if all occupied.

**In ExitObject's precondition:**
```c
if (!Type.Hospital && !Type.Armory && !Type.WeaponsFactory) {
    if (!RadioClass::HasFreeSlot(this)) return 1;   // retry next tick
}
```

Non-Hospital / non-Armory / non-WF buildings need a free radio slot to pair
with the exiting unit (for the MOVE/DOCK/APPROACH radio contract). If all
slots are full, bail and retry next tick.

**Recommended Ghidra rename:** `RadioClass__HasFreeSlot` at `0x0065ADC0`.

## 5. AI Build Queue at `Owner+0x5704` — `DynamicVector<BuildOrder>`

### Vector layout (standard polymorphic DynamicVector)

| Offset | Size | Purpose |
|---|---|---|
| `Owner+0x5704` | 4 | vtable pointer (DynamicVector type) |
| `Owner+0x5708` | 4 | Items array pointer (base of 16-byte entries) |
| `Owner+0x570C` | 4 | Capacity |
| `Owner+0x5710` | 1 | IsAllocated flag (byte; +0x3 padding to +0x5714) |
| `Owner+0x5714` | 4 | **Count** (number of valid entries) |

### BuildOrder entry — 16 bytes (`0x10`)

| Offset | Size | Purpose |
|---|---|---|
| +0x0 | 4 | `BuildingType ID` (matched against `BuildingTypeClass+0xDF8` — likely array index or unique ID) |
| +0x4 | 4 | Packed cell coord (`short x` low, `short y` high) |
| +0x8 | 4 | Unknown — not touched in ExitObject or FUN_0050A490 |
| +0xC | 4 | Unknown — not touched |

### Usage — lookup and remove in ExitObject

```c
// Find index of matching order:
int idx = (*Owner+0x5704[0x14/4])(this);   // polymorphic find

// Remove (shift-left by 16 bytes):
Owner+0x5714 -= 1;                          // decrement count
for (int i = idx; i < count; i++) {
    Items[i] = Items[i + 1];                // memcpy 16 bytes
}
```

### Special case — IsBaseDefense (`Type+0x1706`)

Instead of removing the entry, ExitObject **updates the cell coord** to the
actual placement cell:

```c
if (Type+0x1706) {
    Items[idx + 4bytes] = pack(actual_cell.x, actual_cell.y);
    // Entry stays in queue so AI knows this slot is taken
}
```

### Entry cleanup on building destruction (`FUN_0050A490` / "OnBuildingDestroyed")

Called from `BuildingClass::OnDestroyed` at `0x0050A490`:

```c
void OnBuildingDestroyed(HouseClass *owner, BuildingClass *building) {
    if (g_MapEditorMode) return;
    for (int i = 0; i < count; i++) {
        BuildOrder *entry = &Items[i];
        if (entry.type_id == building.Type+0xDF8 &&
            entry.cell.x == building.location.x &&
            entry.cell.y == building.location.y) {
            // Matching entry found — invalidate duplicate entries at same cell
            packed_cell = entry.cell;
            for (int j = 0; j < count; j++) {
                if (j != i && Items[j].cell == packed_cell) {
                    Items[j].cell = g_InvalidCell;
                }
            }
            if (Type+0x1706 IsBaseDefense && g_GameMode != 0) {
                // Mark defensive slot as available for rebuild
                Items[i].type_id = 0xFFFFFFFF;
                Items[i].cell = g_InvalidCell;
            }
            return;
        }
    }
}
```

Purpose: when a planned/built structure is destroyed, the queue is updated so
the AI knows to rebuild. For base defenses, the slot is kept but zeroed
(ready for rebuild). For others, matching entries are invalidated.

### Consumers

- `HouseClass::AI_Manage_Build_Queue` at `0x004FDD10` — adds entries
- `HouseClass::AI_ChooseNextProduction` at `0x00506EF0` — reads entries to
  pick next build
- `BuildingClass::ExitObject_Main` — removes/updates entries on spawn
- `FUN_0050A490` (OnBuildingDestroyed hook) — invalidates on destruction

---

## Summary of Round-3 Findings

| # | Question | Status |
|---|---|---|
| 1 | Kind enum 0xF = InfantryClass | ✓ Verified (return 0xF) |
| 2 | Naval exit water-cell search | ✓ In `GetDockCellForObject`, 3 cells east/south |
| 3 | WF exit direction math | ✓ atan2 + foundation-edge step + ExitCoord offset |
| 4 | FUN_0065ADC0 = RadioClass::HasFreeSlot | ✓ Verified |
| 5 | AI build queue format at Owner+0x5704 | ✓ DynamicVector<BuildOrder>, 16-byte entries |

### Still unknown (deferred)

- `AircraftClass::What_Am_I` return value (case 2 observed in ExitObject
  but not directly decompiled — low priority, clearly = 2)
- BuildOrder fields `+0x8` and `+0xC` (16-byte entry has 2 unused-looking
  DWORDs; may be priority, timestamp, or placement state)
- `ClearBibArea`'s exact interaction with WF gate timing (currently
  undocumented — probably just "scatter bib before gate opens")

### Master-doc updates from this round

1. **Section 4 (Vtable)**: add row for slot 11 → "What_Am_I (returns class
   kind: 1/2/6/0xF for Unit/Aircraft/Building/Infantry)"
2. **Section 10 (Docking System)**: expand Naval Yard rule with the 3
   specific water cells and the fallback flow.
3. **Section 10**: document the ExitObject precondition =
   `RadioClass::HasFreeSlot` for non-Hospital/Armory/WF buildings.
4. **New section / HouseClass doc**: add AI build queue
   `DynamicVector<BuildOrder>` at Owner+0x5704 with its 16-byte entry
   format. Useful for any AI-adjacent implementation work.

---

## Sources

### Functions decompiled this round

- `0x00459EC0` — BuildingClass::What_Am_I (returns 6)
- `0x00746E20` — UnitClass::What_Am_I (returns 1)
- `0x00523340` — InfantryClass::What_Am_I (returns 0xF)
- `0x0044EFB0` — BuildingClass::GetDockCellForObject (full dispatch)
- `0x00449540` — BuildingClass::ClearBibArea
- `0x0065ADC0` — RadioClass::HasFreeSlot (previously FUN_0065ADC0)
- `0x0050A490` — HouseClass::OnBuildingDestroyed queue cleanup
- (context from) `0x004FDD10` — HouseClass::AI_Manage_Build_Queue
- (context from) `0x00506EF0` — HouseClass::AI_ChooseNextProduction

### Ghidra reads

- Direct call-site analysis of ExitObject (0x00443C60) for queue
  removal/update patterns
- Byte pattern `08 57 00 00` (Owner+0x5708 access) to find all queue consumers
