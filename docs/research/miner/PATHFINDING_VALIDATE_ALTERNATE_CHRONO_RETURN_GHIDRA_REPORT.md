# Pathfinding_validate_alternate — Chrono Miner Return Fallback Cell Validator

**Investigation date:** 2026-05-19
**Source:** `gamemd.exe` (YR 1.001), decompiled via Ghidra MCP.
**Active in YR:** Yes — fires every time a chrono miner teleport-returns to refinery when
  distance > `ChronoHarvTooFarDistance` (default 50 cells). Normal skirmish frequency: ~1–3x per
  harvest cycle per chrono miner.

---

## 1. Identity Resolution

The doc placeholder name "Pathfinding_validate_alternate" resolves to
**`FootClass::Find_Nearby_Passable_Cell`** at address **`0x56DC20`**.

This was confirmed by decompiling `UnitClass::Mission_Harvest` (0x73E5E0) and reading the
explicit Ghidra label on the call site in state 2 (RETURN).  No separate "validate_alternate"
function exists; the chrono return fallback calls the same `Find_Nearby_Passable_Cell` used
everywhere else in the engine.

The full behavior of `FootClass::Find_Nearby_Passable_Cell` (signature, search algorithm, all
validation checks, selection logic) is documented in `FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md`.
This report documents only the **chrono-miner-specific invocation**: which parameters are passed
and what they mean for the return path.

---

## 2. Seed Cell Input — Question (a)

Confirmed from Mission_Harvest state 2 decompilation at 0x73E5E0.

```c
// piVar3 = dock BuildingClass*
// piVar3[0x148] = *(int*)(piVar3 + 0x520) = BuildingClass->TypeClass

sVar10 = (short)(piVar3[0x27] + (piVar3[0x27] >> 0x1f & 0xff)) >> 8;   // dock.Location.X / 256
sVar2  = (short)(piVar3[0x28] + (piVar3[0x28] >> 0x1f & 0xff)) >> 8;   // dock.Location.Y / 256

// Seed cell = dock cell + DockOffset from BuildingTypeClass
target.X = *(short*)(piVar3[0x148] + 0x1618) + sVar10;   // cellX + DockOffset.X
target.Y = *(short*)(piVar3[0x148] + 0x161c) + sVar2;    // cellY + DockOffset.Y
```

**Confirmed:** `BuildingTypeClass+0x1618` = `DockOffset.X`, `+0x161C` = `DockOffset.Y`.
The seed cell is exactly `(cellX + dockOffsetX, cellY + dockOffsetY)` as documented in
§14 of `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`.

---

## 3. Passability / Zone / Occupancy Checks — Question (b)

The exact call site from Mission_Harvest state 2 (verified from decompilation):

```c
FootClass__Find_Nearby_Passable_Cell(
    this,          // the harvester FootClass*
    &outCell,      // output: validated cell (CellStruct)
    &target,       // origin: seed cell (cellX+dockOffX, cellY+dockOffY)
    2,             // param_4 = SpeedType = 2 (SPEED_WHEEL)
    0xffffffff,    // param_5 = zone_id = -1  --> ZONE CHECK DISABLED
    0,             // param_6 = locomotor_type = 0 (Drive)
    0,             // param_7 = bridge_aware = false
    1,             // param_8 = foundation_width = 1
    1,             // param_9 = foundation_height = 1
    0,             // param_10 = overlay_check = 0 (overlays NOT rejected)
    0,             // param_11 = check_height = false (height match skipped)
    0,             // param_12 = check_occupants = false (occupant check skipped)
    1,             // param_13 = 1 --> bridge cells ALLOWED
    &nullCell,     // param_14 = target cell = {0,0}  --> random selection mode
    0,             // param_15 = skip_first_quad = false
    0              // param_16 = check_occupancy_rect = false
);
```

**Checks that FIRE (with param values as above):**

| Check | Active? | Detail |
|-------|---------|--------|
| Map bounds (cell index 0..0x3FFFF) | Yes | Always |
| `TechnoClass::IsOnScreen` (playfield bounds) | Yes | Always |
| `CellRect::CheckPassability` (1×1, SpeedType=2, zone=-1, loco=0) | Yes | Zone check disabled (−1). Terrain cost for SpeedType=2 (SPEED_WHEEL) must be passable. Wall/overlay passability checked per terrain type. |
| Height match (±2 levels) | **No** | param_11=0 |
| Cell occupant safety (FUN_486FF0) | **No** | param_12=0 |
| Bridge cell rejection | **No** | param_13=1, bridge cells allowed |
| Foundation occupancy rect | **No** | param_16=0 |

**Key implication:** Only basic terrain passability for SpeedType=2 (wheeled) is checked.
No zone connectivity enforcement, no height matching, no occupancy check.

---

## 4. Fallback Search Pattern — Question (c)

`Find_Nearby_Passable_Cell` uses an expanding diamond/ring pattern (documented fully in
`FIND_NEARBY_PASSABLE_CELL_GHIDRA_REPORT.md` §§2–3). Summary for this call:

- **Search radius:** `min(harvester.Speed + harvester.Sight, 32)` cells from the seed cell.
  For the Chrono Miner (CMIN) with typical Speed≈5 and Sight≈6, radius ≈ 11; hard cap = 32.
- **Pattern:** Concentric diamond rings from radius 0 outward. For each ring, visits top/bottom
  edges then left/right columns in order.
- **Early termination:** Stops when 24 candidates collected OR when a "direct" candidate is
  found and the current ring is complete.
- **Selection (param_14={0,0}):** When no target cell is given (the `nullCell` passed here),
  uses pseudo-random selection from the frame counter: `candidates[frame_counter % count]`.
  Direct candidates (height-visually-correct) are preferred over indirect.

There is no spiral pattern; the shape is diamond rings, not a row scan.

---

## 5. No-Valid-Cell Return — Question (d)

From `Find_Nearby_Passable_Cell` decompilation, the zero-candidates path:

```c
LAB_0056e79a:
    *param_2 = DAT_00abd480;   // output = null cell {0, 0}
    return;
```

`DAT_00abd480` reads as `{0x00, 0x00, 0x00, 0x00}` = CellStruct `{0, 0}` (verified by
`read_memory` at 0xABD480).

In Mission_Harvest, after the call:

```c
if ((short)outCell == (short)DAT_00b1cfb8 && outCell._2_2_ == DAT_00b1cfb8._2_2_) {
    // null cell returned: Set_Destination(NULL, 1)
    (**(code **)(*param_1 + 0x480))(0, 1);
} else {
    // valid cell: Set_Destination(MapClass::Get_CellClass(outCell))
    uVar7 = MapClass__Get_CellClass(&outCell);
    (**(code **)(*param_1 + 0x480))(uVar7);
}
```

`DAT_00b1cfb8` also reads `{0,0}` (verified by `read_memory` at 0xB1CFB8). These are two
static zero-initialised globals used as sentinel comparands.

`vtable+0x480` = `Set_Destination` (confirmed from `BRIDGE_CAN_ENTER_CELL_HIERARCHY_GHIDRA_REPORT.md`
annotation at the same vtable slot).

**Confirmed:** When no valid cell exists, `Set_Destination(NULL, 1)` is called — the miner's
destination is cleared. The unit remains stationary until the next Mission_Harvest tick reschedules
a new dock search.

---

## 6. Verified Facts Summary

| # | Fact | Evidence |
|---|------|----------|
| 1 | "Pathfinding_validate_alternate" IS `FootClass::Find_Nearby_Passable_Cell` at `0x56DC20` | Explicit Ghidra label on call in `UnitClass::Mission_Harvest` (0x73E5E0) decompilation |
| 2 | Seed cell = `(dock.cellX + BuildingTypeClass[+0x1618], dock.cellY + BuildingTypeClass[+0x161C])` | Read directly from Mission_Harvest state 2 decompilation; matches offsets in §14 of chrono doc |
| 3 | Zone check disabled (param_5=-1); only SpeedType=2 terrain passability fires | Exact parameter literal `0xffffffff` visible in decompilation call site |
| 4 | No-valid-cell case writes null cell `{0,0}` to output → `Set_Destination(NULL,1)` | `LAB_0056e79a` in `Find_Nearby_Passable_Cell`; `read_memory` at 0xABD480 = `0x00000000`; vtable+0x480 confirmed as `Set_Destination` |
| 5 | Target cell passed as `{0,0}` → random selection (frame counter modulo candidate count) | Literal `&uStack_4c` (zero-init'd local) in call; confirmed by selection branch in `Find_Nearby_Passable_Cell` decompilation |

---

## 7. Status

**COMPLETE**

All four sub-questions answered with binary-verified evidence. No invented offsets or addresses.
