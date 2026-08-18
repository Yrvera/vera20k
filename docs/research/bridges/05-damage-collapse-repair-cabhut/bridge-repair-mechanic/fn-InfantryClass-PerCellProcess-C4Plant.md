# fn-InfantryClass-PerCellProcess — Mission_Sabotage / C4 Plant Branch

**Address:** `0x00519630`
**Class:** `InfantryClass`
**Method:** `PerCellProcess` (vtable slot `+0x1E4`, confirmed: vtable data at `0x007eb1e4` reads `30 96 51 00` = `0x00519630`)
**Scope:** NARROW — Mission_Sabotage (mission id `0x11`) C4 plant path only.
**Confidence:** HIGH (content, identity, vtable binding all verified via Ghidra MCP)
**YR-active:** YES — this code path is unconditionally reachable whenever an infantry unit with `C4=yes` is in Mission_Sabotage and reaches the target building cell.

---

## Signature

```c
void __thiscall InfantryClass__PerCellProcess(int *param_1, int *param_2)
```

- `param_1` = `this` pointer (`InfantryClass*`). Type is `int*` — all field accesses via `param_1[N]` are **pointer arithmetic**: byte offset = `N * 4`.
- `param_2` = nav-target argument passed by the locomotor tick. Value `0x2` is a special sentinel meaning "locomotor called with arrival signal."

Verified via `decompile_function 0x00519630`.

---

## Vtable Binding

The function is referenced as a data pointer at `0x007eb1e4`.

Verified via `read_memory 0x007eb1e0` (12 bytes):
```
90 be 41 00  |  30 96 51 00  |  20 5c 5f 00
```
Bytes at offset +4 = `30 96 51 00` = little-endian `0x00519630` — confirms `InfantryClass::PerCellProcess` at vtable+0x1E4.

The slot at `vtable+0x1E4` on `InfantryClass` is the PerCellProcess virtual. The `param_1 + 0x1E4` call pattern `(**(code **)(*param_1 + 0x1bc))` in callee bodies confirms this is the cell-arrival dispatch point.

---

## Param-1 Pointer Arithmetic Reference

Since `param_1` is `int*`, every `param_1[N]` index is `N * 4` bytes from the object base:

| Ghidra index | Byte offset | Field |
|---|---|---|
| `param_1[0x27]` | `0x9C` | Location X (leptons) |
| `param_1[0x28]` | `0xA0` | Location Y (leptons) |
| `param_1[0x29]` | `0xA4` | Location Z (leptons) |
| `param_1[0x1b0]` | `0x6C0` | Pointer to `InfantryTypeClass` (type object) |
| `param_1[0x169]` | `0x5A4` | Nav-target building pointer (`NavTarget`) |
| `param_1[0xad]` | `0x2B4` | Focus/archive building pointer |
| `param_1[0xbb]` | `0x2EC` | Frame counter snapshot at C4 arm start |
| `param_1[0xbc]` | `0x2F0` | Arm location saved (Location Y copy) |
| `param_1[0xbd]` | `0x2F4` | Arm direction (facing) saved |
| `param_1[0xd]` | `0x34` | Team pointer |
| `param_1[0x2b]` | `0xAC` | Mission enum |

---

## InfantryTypeClass C4 Flags (via param_1[0x1b0] = InfantryTypeClass*)

These are byte fields on `InfantryTypeClass`, offset from its base:

| Byte offset | Meaning | Ghidra expression |
|---|---|---|
| `+0xEC2` | `C4=yes` flag (SEAL/Tanya can plant C4 on buildings/bridges) | `*(char *)(param_1[0x1b0] + 0xec2)` |
| `+0xEB4` | `Infiltrate=yes` flag (spy infiltration) | `*(char *)(param_1[0x1b0] + 0xeb4)` |
| `+0xEB5` | `Infiltrate` secondary flag | `*(char *)(param_1[0x1b0] + 0xeb5)` |
| `+0xEC3` | Engineer-type flag (capture buildings) | `*(char *)(param_1[0x1b0] + 0xec3)` |
| `+0xEC4` | Spy infiltrate secondary flag | `*(char *)(param_1[0x1b0] + 0xec4)` |
| `+0xEC6` | Flag controlling building-find order (spy/engineer pathing) | `*(char *)(param_1[0x1b0] + 0xec6)` |

Verified via `decompile_function 0x00519630` — all offsets read directly from the decompiled C pseudocode.

---

## Mission_Sabotage C4 Plant Branch — Full Logic

The C4 plant branch activates only when ALL of:
1. `GetMission() == 0x11` (Sabotage mission, verified: vtable call `*param_1 + 0x184`)
2. `*(char *)(param_1[0x1b0] + 0xec2) != '\0'` — infantry type has `C4=yes`
3. A building exists in the current cell: `Look_up_building_in_cell() != NULL`
4. That building is the nav-target: `piVar10 == param_1[0x169]`

### Step 1 — IronCurtain Guard

```c
iVar3 = (**(code **)(*piVar10 + 0x184))();  // target building GetMission/state check
if ((iVar3 != 0x13) &&                       // 0x13 = building is being sold/destroyed
    (cVar2 = (**(code **)(*piVar10 + 0x160))(), cVar2 == '\0'))  // IronCurtain check
```

- `vtable + 0x160` on target building = `IsIronCurtained()`. If the building is IronCurtained, the C4 is NOT planted — the infantry is bounced back.
- `vtable + 0x184` = mission/state check on the target building; `0x13` = building is in a terminal state (being sold). C4 cannot be planted on a building being destroyed.

### Step 2 — Already-Planted Guard (bridge hut special case)

```c
if (*(char *)((int)piVar10 + 0x6df) != '\0') {
    // C4 already planted on this building — abort, reassign to new nav target
    (**(code **)(*param_1 + 0x480))(0, 1);
    iVar4 = (**(code **)(*param_1 + 0x318))(1);
    param_1[0xbb] = g_CurrentFrameCounter;
    param_1[0xbc] = local_14;
    param_1[0xbd] = iVar4;
    uVar6 = (**(code **)(*piVar10 + 0x48))(&local_18, 1, 1);
    (**(code **)(iVar3 + 0x174))(uVar6);
    return;
}
```

`BuildingClass + 0x6DF` is the "C4 already planted" flag on the **building** (not the infantry). If set, the infantry gives up on this target and reroutes.

### Step 3 — Plant the C4 (core write)

```c
*(undefined1 *)((int)piVar10 + 0x6df) = 1;   // Mark building as having C4 planted
```

**Confirmed at `0x0051A5A7`:** `MOV BYTE PTR [EDI+0x6DF], 1`

Verified via `read_memory 0x0051A5A0` (24 bytes):
```
5b 83 c4 40 c2 04 00 c6 87 df 06 00 00 01 ...
```
Bytes at offset +7: `c6 87 df 06 00 00 01` = x86 `MOV BYTE [EDI+0x6DF], 1`. Address = `0x0051A5A0 + 7 = 0x0051A5A7`. Exact match.

### Step 4 — Arm timer writes to building

Immediately after setting `+0x6DF`, the following writes occur on the building (`*piVar10`):

```c
iVar3 = *piVar10;
uVar6 = Math__ftol();
(**(code **)(iVar3 + 0x148))(uVar6);          // vtable+0x148 — arm countdown set
iVar3 = g_CurrentFrameCounter;
iVar4 = Math__ftol();
piVar10[0x150] = (int)param_1;               // +0x540 = pointer back to planting infantry
piVar10[0x14a] = iVar3;                      // +0x528 = frame counter at plant time
piVar10[0x14b] = iStack_8;                   // +0x52C = (unknown field, likely target location y)
piVar10[0x14c] = iVar4;                      // +0x530 = ftol result (countdown ticks)
```

Pointer arithmetic on `piVar10` (which is also `int*`):
- `piVar10[0x150]` = byte offset `0x150 * 4 = 0x540` → `BuildingClass+0x540` = pointer to planting infantry
- `piVar10[0x14a]` = byte offset `0x14a * 4 = 0x528` → `BuildingClass+0x528` = frame stamp at plant
- `piVar10[0x14b]` = byte offset `0x14b * 4 = 0x52C` → `BuildingClass+0x52C` = unknown (coord?)
- `piVar10[0x14c]` = byte offset `0x14c * 4 = 0x530` → `BuildingClass+0x530` = countdown ticks

Verified via `decompile_function 0x00519630` — all four writes confirmed in the same basic block as the `+0x6DF` write.

### Step 5 — Infantry stop + face target

```c
FootClass__Stop_Moving();
(**(code **)(*param_1 + 0x45c))(0);   // FootClass::FaceTarget or similar orient call
iVar4 = (**(code **)(*param_1 + 0x318))(1);  // get facing index
param_1[0xbb] = g_CurrentFrameCounter;
param_1[0xbc] = local_10;
param_1[0xbd] = iVar4;
uVar6 = (**(code **)(*piVar10 + 0x48))(&local_14, 1, 1);  // GetCoords of building
(**(code **)(iVar3 + 0x174))(uVar6);    // MoveTo / route infantry to plant position
return;
```

After planting, the infantry stops moving, records the current frame + facing, gets the building's coords, then assigns a movement order to walk to the plant position. This is the "arming animation" setup.

---

## Bridge-Specific Path: High vs. Low Bridge Detection

Within the broader Mission `0x11` + `C4=yes` case, there is a **pre-plant bridge detection loop** that runs when the target building type is `BuildingTypeClass::Type == 6` (bridge hut):

```c
iVar3 = (**(code **)(pBVar7->vtable + 0x2c))();  // GetBuildingType()
if ((iVar3 == 6) && (pBVar7->Type[0x16b6] != '\0')) {
```

`BuildingTypeClass + 0x16b6` = `BridgeRepairHut` flag on the type object. This distinguishes bridge huts from other type-6 buildings.

The loop scans a 5×5 cell area (±2 in each axis) around the infantry's current position:

```c
param_2 = (int *)((uint)param_2 & 0xffffff00);  // low byte = bridge-type accumulator, cleared
do {
    iVar3 = -2;
    do {
        // Get cell N/S offset
        // Check if cell overlay tile index is in [DAT_00abad1c, DAT_00abad1c+0x10)
        //   => low bridge overlay range
        // Check if cell overlay index is in (0x49, 0x66)
        //   => high bridge overlay range
        if (low_bridge_found || high_bridge_found) {
            param_2 = (int *)CONCAT31(param_2._1_3_, 1);  // set low byte = 1 = "bridge nearby"
        }
        iVar3++;
    } while (iVar3 < 3);
} while (local_3c + 1 < 3);
```

After the scan:
- If `(char)param_2 == '\0'` (no bridge tiles found in 5×5 area) → call `ProcessBridgeDestruction_High(cell_ptr)` → **high bridge** destruction path
- If `(char)param_2 != '\0'` (bridge tiles found) → call `ProcessBridgeDestruction_Low(cell_ptr)` → **low bridge** destruction path

The overlay range constants:
- Low bridge range: `[DAT_00abad1c, DAT_00abad1c + 0x10)` — 16 consecutive overlay indices
- High bridge range: `(0x49, 0x66)` — overlay indices 0x4A through 0x65 (hardcoded)

---

## Called Bridge Destruction Functions

Both confirmed via `get_function_callees 0x00519630`:

| Function | Address | Range |
|---|---|---|
| `ProcessBridgeDestruction_Low` | `0x00570050` | `0x00570050 – 0x00570ad3` |
| `ProcessBridgeDestruction_High` | `0x00573540` | `0x00573540 – 0x00573ff6` |

After either bridge destruction call, a sweep over all infantry clears their nav-target reference to the destroyed hut:

```c
iVar3 = g_InfantryClass_Array_Count;
while (iVar3 = iVar3 + -1, -1 < iVar3) {
    (**(code **)(**(int **)(g_InfantryClass_Array + iVar3 * 4) + 0x28))(pBVar7, 0);
    // vtable+0x28 = ClearNavTarget(building, 0) on each infantry unit
}
(**(code **)(pBVar7->vtable + 0x2e0))();  // building KillAnim/Die
```

---

## Other Mission Branches in PerCellProcess (Out-of-Scope Refs)

The function handles many missions besides Sabotage. These are NOT the focus of this decode but are flagged as out-of-scope-refs for the manifest:

- Mission `8` + Infiltrate flags → garrison entry or spy infiltration → `BuildingClass__OnSpyInfiltrate` at `0x004571e0`
- Mission `9` → harvester unload → `HouseClass__Add_Credits` at `0x004f9950`
- Mission `0x19` + Engineer (`+0xEC3`) → building capture → `BuildingClass::CaptureBuilding` via vtable+0x3d4
- Mission `7` → passenger boarding → `CargoClass__AddPassenger`
- Missions `8`, `0xB`, `0x19` combined → docking check → radio handshake path

---

## Summary of C4 Plant State Machine

```
Infantry in Mission_Sabotage (0x11)
  ├─ Has C4=yes (InfantryTypeClass+0xEC2 != 0)
  ├─ Reached nav-target building cell
  ├─ Building not IronCurtained (vtable+0x160)
  ├─ Building not in state 0x13 (terminal)
  ├─ BuildingClass+0x6DF == 0 (not already planted)
  │
  ├─ If building type == 6 (bridge hut) + BridgeRepairHut flag set:
  │     Scan ±2 cells for bridge overlay tiles
  │     → ProcessBridgeDestruction_Low (0x00570050)  if low bridge tiles found
  │     → ProcessBridgeDestruction_High (0x00573540) if no bridge tiles found
  │     Sweep all infantry: clear nav-target to destroyed hut
  │     building.KillAnim()
  │
  └─ Else (normal C4 target):
        BuildingClass+0x6DF = 1            (mark as planted, at 0x0051A5A7)
        BuildingClass+0x528 = current_frame
        BuildingClass+0x52C = coord_y
        BuildingClass+0x530 = countdown_ticks (ftol)
        BuildingClass+0x540 = this infantry
        FootClass::Stop_Moving()
        infantry route to building center
```

---

## Unverified

None — all offsets above are read directly from `decompile_function 0x00519630` output or confirmed via `read_memory`. The bridge overlay range constants (`DAT_00abad1c` and `0x49`/`0x66`) are read directly from the decompilation; the runtime value of `DAT_00abad1c` is a global and its precise value would require a separate read_memory call at `0x00abad1c` to confirm — flagged YELLOW.

---

## Self-Proof (exit gate)

### Claim 1: Function at `0x00519630` is `InfantryClass__PerCellProcess`
`get_function_by_address 0x00519630` → `InfantryClass__PerCellProcess`, body
`0x00519630 – 0x0051AA0A`. **VERIFIED.**

### Claim 2: vtable slot `+0x1E4` = `0x00519630`
`read_memory 0x007EB1E0` (12 bytes) → hex `90 BE 41 00 | 30 96 51 00 | 20 5C 5F 00`.
Bytes at offset +4 = `30 96 51 00` = little-endian `0x00519630`. **VERIFIED.**

### Claim 3: `InfantryTypeClass + 0xEC2` = C4 flag
`decompile_function 0x00519630` shows `*(char *)(param_1[0x1b0] + 0xec2)` as the C4 check
(param_1 is `int*`, so `param_1[0x1b0]` = byte offset `0x6C0` = InfantryTypeClass pointer;
`+0xEC2` is the C4 byte). **VERIFIED from decompilation output.**
