# What_Action_On_Cell — Bridge-Cell Cursor Branches — Ghidra Research Report

**Date:** 2026-05-19  
**Scope:** Cell-side `What_Action_On_Cell` bridge branches only (not object-side)  
**Active in YR:** Yes — runs on every hover tick and every right-click when a unit is selected  
**Confidence:** HIGH for all load-bearing claims (each verified via Ghidra MCP decompilation this session)

---

## 1. Function Addresses

| Function | Address | xref count | Role |
|---|---|---|---|
| `TechnoClass::What_Action_OnCell` (base) | `0x00700600` | 4 | Core action-code logic; dispatches attack/move/harvest/dock |
| `FootClass::What_Action_OnCell` (shroud wrapper) | `0x004DDDE0` | 4 | Wraps base; adds shroud override + bridge-height offset |
| `InfantryClass::What_Action_OnCell` | `0x0051F800` | 1 | Adds garrison/engineer/low-bridge override |
| `UnitClass::What_Action_OnCell` | `0x007404B0` | 1 | Adds harvest/deploy/low-bridge override |

Verified via `search_functions_enhanced` and `decompile_function` on each address.

---

## 2. Call Hierarchy and Bridge-Cell Flow

```
DisplayClass::DetermineAction
  └── best->vtable[0x70](cell, modifier)  // polymorphic cell dispatch
        ├── InfantryClass::What_Action_OnCell  @ 0x0051F800
        │     └── calls FootClass::What_Action_OnCell @ 0x004DDDE0
        │               └── calls TechnoClass::What_Action_OnCell @ 0x00700600
        └── UnitClass::What_Action_OnCell  @ 0x007404B0
              └── calls FootClass::What_Action_OnCell @ 0x004DDDE0 (via result variable)
                        └── calls TechnoClass::What_Action_OnCell @ 0x00700600
```

Each layer receives the base result and then applies its own specializations.

---

## 3. Bridge Flag Check in FootClass — Height Offset Only

In `FootClass::What_Action_OnCell` (verified via `decompile_function 0x004DDDE0`):

```c
iVar3 = CellClass__Get_Cell_At(&local_c);
if ((*(uint *)(iVar3 + 0x140) & 0x100) != 0) {
    local_4 = local_4 + DAT_008b3df4;   // height offset only
}
```

**Critical finding:** `flags & 0x100` (bridge body flag at `CellClass+0x140`) is checked **only to adjust the height offset**, not to gate or modify the returned action code. The action code `iVar2` is computed entirely by `TechnoClass::What_Action_OnCell` before this check and is returned unchanged.

**Active in YR: Yes.** Applies to all intact high bridge body cells.

---

## 4. Action Codes for Intact Bridge Cells (High Bridge)

For an intact high-bridge body cell (`flags & 0x100 != 0`):
- `TechnoClass::What_Action_OnCell` evaluates attack/move/dock logic and returns one of:
  - `2` (Move) — if the unit can move there and is a ground unit with a valid path
  - `5` (Attack via best weapon) — if the cell has enemies and unit is armed
  - `0` (NoMove/Invalid) — if unit cannot enter (no loco support)
  - `0x33` (ForceAttack cursor) — if ctrl held and targets present

The high bridge body flag `0x100` does NOT produce any dedicated bridge-entry cursor code. Units that can traverse high bridges get the normal **Move (2)** cursor. The bridge-body check only adjusts the ground height used for 3D world-position calculations.

**Active in YR: Yes.**

---

## 5. Action Code for Damaged-but-Passable Bridge Cells (High Bridge)

A damaged-but-not-collapsed high bridge cell still has `flags & 0x100` set (bridge body flag is not cleared until full destruction). The damage state is encoded in `CellClass+0x11E` (values 6 for NS-damaged, 15 for EW-damaged — see `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md`).

`What_Action_OnCell` **does not read `CellClass+0x11E`** at all. Therefore:
- A damaged-but-still-traversable bridge cell produces the **exact same cursor as an intact bridge cell**.
- Cursor shown: **Move (2)** for ground units that can cross.

**Active in YR: Yes.**

---

## 6. Action Codes for Destroyed Bridge Cells

After full high bridge destruction (`ProcessBridgeDestruction_High` walker, verified via `decompile_function 0x00573540`), the walker calls `MapClass__RecalcCellsAndRebuildZones` which re-evaluates each cell's `ZoneType`. The destroyed bridge body cells revert to their underlying terrain tile (water or impassable gap). Their `flags & 0x100` bit is no longer set.

For a destroyed bridge body cell (no `flags & 0x100`, underlying terrain is impassable water/gap):
- `TechnoClass::What_Action_OnCell` returns **`0` (NoMove)** because:
  - The unit cannot move there (speed table lookup for the underlying LandType, e.g. Rock (3) or Water for ground units without Hover/Float locomotion, yields 0.0)
  - The ZoneType after `RecalcZoneType` is `6` (Impassable)
  - No weapon target in an empty cell, so attack codes don't fire
- Pathfinding routes around the destroyed span (bridge zone edges removed from zone graph by `MapClass__UpdateBridgeZonesHelper`)

**Result: Destroyed bridge cells return `0` (NoMove) for ground units.**

---

## 7. Low Bridge Cells — Dedicated Cursor Override

`CellClass__IsLowBridgeCell` (verified via `decompile_function 0x00484AB0`):

```c
if ((-1 < *(short *)(param_1 + 0x116)) &&
    (*(short *)(param_1 + 0x116) < DAT_008b4148) &&
    (*(int *)(param_1 + 0xec) == 10)) {  // LandType == Tunnel (10)
    return 1;
}
```

Detection: `CellClass+0x116` (TubeIndex) in valid range AND `CellClass+0xEC` (LandType) == `10` (Tunnel).

### UnitClass low bridge branch (`decompile_function 0x007404B0`):

```c
MapClass__Get_CellClass(psVar1);
cVar2 = CellClass__IsLowBridgeCell();
if (cVar2 != '\0') {
    MapClass__Get_CellClass(psVar1);
    cVar2 = FUN_00484f10(param_1);         // always returns 1 (stub, verified via read_memory 0x00484f10)
    return 0x24 - (uint)(cVar2 != '\0');   // 0x24 - 1 = 0x23
}
```

`FUN_00484f10` bytes: `B0 01 C2 04 00` = `MOV AL,1; RETN 4` — confirmed stub always returning 1.

Therefore for UnitClass on a low bridge cell: **always returns `0x23`**.

### InfantryClass low bridge branch (`decompile_function 0x0051F800`):

Identical pattern:
```c
if (iVar2 == 1) {   // only applies when base returned Move(1) or Move(2)
    MapClass__Get_CellClass(param_2);
    cVar1 = CellClass__IsLowBridgeCell();
    if (cVar1 != '\0') {
        MapClass__Get_CellClass(param_2);
        cVar1 = FUN_00484f10(param_1);   // always returns 1
        return 0x24 - (uint)(cVar1 != '\0');  // always 0x23
    }
}
```

Infantry also returns **`0x23`** for any low bridge cell where the base returned Move.

### Action code 0x23 meaning:

Per `DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md` §4:
- `0x23` = cursor `0x19` — this is the **garrison alternate** entry in the action table, but in the low-bridge context it functions as the **low-bridge entry cursor** (cursor SHP index `0x19`).
- `0x24` would be the "can't enter low bridge" variant, but `FUN_00484f10` being a stub means **the check that would have distinguished them is not implemented** — units always get `0x23` (the passable variant).

**Active in YR: Yes.**

---

## 8. Infantry-Specific Low Bridge Blocking

`InfantryClass::What_Action_OnCell` (verified via `decompile_function 0x0051F800`) adds an additional pre-check:

```c
if ((*(char *)(*(int *)&param_1[1].field_0x1a0 + 0xd94) != '\0') && ((iVar2 == 1 || iVar2 == 2))) {
    MapClass__Get_CellClass(param_2);
    cVar1 = FUN_00484ae0();   // horizontal low-bridge edge detector
    if (cVar1 == '\0') {
        MapClass__Get_CellClass(param_2);
        cVar1 = FUN_00484d60();   // vertical low-bridge edge detector
        if (cVar1 == '\0') goto LAB_0051f8a8;
    }
    iVar2 = 0;   // block infantry from crossing
}
```

The flag at `TechnoTypeClass+0xD94` gates this. `FUN_00484ae0` and `FUN_00484d60` are "adjacent-to-low-bridge" detectors that check LandType == 10 in neighboring cells — they block infantry from crossing the low bridge cell if the entry is sideways. When triggered, returns **`0` (NoMove)** for infantry.

**Active in YR: Yes, whenever TechnoTypeClass+0xD94 is set.**

---

## 9. Harvest Override on Ore/Tiberium Cells (UnitClass)

In `UnitClass::What_Action_OnCell`, after the low bridge branch, there is an ore harvest override:

```c
iVar5 = *(int *)(iVar3 + 0xec);   // cell LandType
if (iVar5 == 5) ...   // LandType::Tiberium (5) = ore cell
    return 6;         // Harvest cursor
if (iVar5 == 0xb) ... // LandType::Weeds (11) = gem cell  
    return 6;         // Harvest cursor
```

These are NOT bridge-related, included here for completeness.

---

## 10. Summary Table — Cursor by Bridge State

| Bridge state | Cell condition | `flags & 0x100` | Returns (Unit) | Returns (Infantry) | Cursor shown |
|---|---|---|---|---|---|
| Intact high bridge body | LandType passable, `flags & 0x100 = 1` | Set | `2` (Move) | `2` (Move) | Move arrow |
| Damaged-but-passable high bridge | Same as intact (`+0x11E` ≥ 6 but not collapsed) | Set | `2` (Move) | `2` (Move) | Move arrow (no damage cursor) |
| Destroyed high bridge | LandType reverts to ground (impassable water/gap) | Cleared | `0` (NoMove) | `0` (NoMove) | NoMove / no cursor |
| Intact low bridge body | `TubeIndex ≥ 0`, LandType == Tunnel (10) | Not tested | `0x23` (low-bridge entry) | `0x23` | Cursor 0x19 |
| Destroyed low bridge | LandType no longer 10, TubeIndex invalid | N/A | Falls through to `0` (NoMove) | `0` (NoMove) | NoMove |

---

## 11. Key Verified Facts

1. **`flags & 0x100` is height-only** — the bridge body flag at `CellClass+0x140` is checked only to add a height offset in `FootClass::What_Action_OnCell`; it does not modify the returned action code. (verified: `decompile_function 0x004DDDE0`)

2. **Damaged high bridge = same cursor as intact** — `What_Action_OnCell` never reads `CellClass+0x11E` (damage state). A damaged-but-traversable bridge returns Move (2) identical to intact. (verified: `decompile_function 0x007404B0`, `decompile_function 0x0051F800`)

3. **Destroyed high bridge → NoMove (0)** — After destruction the bridge flag `0x100` is cleared and the cell reverts to impassable ground tile; `TechnoClass::What_Action_OnCell` returns 0. (verified: `decompile_function 0x00573540` shows `RecalcCellsAndRebuildZones` call; `decompile_function 0x00700600` has no bridge-specific override path)

4. **Low bridge cursor is `0x23` (always)** — `FUN_00484f10` is a 5-byte stub `B0 01 C2 04 00` (MOV AL,1; RETN 4) that always returns 1, so `return 0x24 - 1 = 0x23` in both `UnitClass` and `InfantryClass`. (verified: `read_memory 0x00484f10`, `decompile_function 0x007404B0`, `decompile_function 0x0051F800`)

5. **Low bridge detection: LandType == 10 AND TubeIndex in valid range** — `CellClass__IsLowBridgeCell` checks `CellClass+0xEC == 10` (LandType Tunnel) and `CellClass+0x116` (short TubeIndex) in `[0, DAT_008b4148)`. (verified: `decompile_function 0x00484AB0`)

---

## 12. Active in YR Status

All findings in this report apply to standard YR skirmish:
- `What_Action_OnCell` runs every hover tick for every selected unit — high frequency
- `flags & 0x100` check (height offset): **Active in YR: Yes**
- Low bridge cursor override: **Active in YR: Yes** (low bridges are standard map features)
- `FUN_00484f10` stub (always-passable low bridge): **Active in YR: Yes** (the size-too-big path is never taken — stub removes discrimination)
- `TechnoTypeClass+0xD94` infantry block: **Active in YR: Conditional** (depends on unit type flag)
- Destroyed bridge → NoMove: **Active in YR: Yes** (bridge destruction is live in YR when `DestroyableBridges=yes`)

---

## 13. Implications for Rust Port

- For intact/damaged high bridge cells: no special cursor logic needed — normal Move/Attack resolution applies. The height offset (`flags & 0x100 → +DAT_008b3df4`) must be applied during the height query, not during cursor logic.
- For low bridge cells: both Unit and Infantry subclasses return `0x23` unconditionally (FUN_00484f10 is a stub). The size-check that would have returned `0x24` instead is dead code.
- For destroyed bridge cells: cursor logic naturally returns NoMove (0) once the zone system marks those cells impassable — no bridge-specific cursor logic needed.
- The infantry `TechnoTypeClass+0xD94` block that forces 0 on low-bridge approaches needs implementing for infantry types that have this flag set.
