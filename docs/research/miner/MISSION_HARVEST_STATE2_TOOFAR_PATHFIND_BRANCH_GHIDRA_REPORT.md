# Mission_Harvest State 2 — "Too Far" Pathfinding Branch
**Ghidra RE Report — `UnitClass__Mission_Harvest` @ 0x0073E5E0, State 2 sub-branch**
Date: 2026-05-19
Scope: The exact "too far to dock directly" pathfind setup in RETURN_TO_REFINERY state,
including the 2nd Find_Docking_Bay call, the 0x300-lepton threshold, QueueingCell usage,
FootClass__Find_Nearby_Passable_Cell arguments, and which unit field receives the result.

---

## 1. Context — Where This Branch Lives

`UnitClass::Mission_Harvest` at 0x0073E5E0 is a 5-state machine keyed on
`param_1[0x2F]` (byte offset 0xBC = MissionSubState). State 2 = RETURN_TO_REFINERY.

Active in YR: **Yes**. Fires every time a harvester finishes loading and returns to dock.
Frequency: constant throughout normal play. Very high — fires once per ore-run per harvester.

---

## 2. Full State 2 Control Flow

State 2 begins with a check for an existing destination:

```c
// cVar1 = *(char *)(TypeClass + 0xCD4)  — Teleporter flag (bool: chrono miner)
// param_1[0x169] = Destination (0 = no current destination)

// Step A: if already moving to a dock and chrono miner, try to stop early
if ((param_1[0x169] != 0) && (cVar1 != '\0')) {
    iVar8 = Find_Docking_Bay(TypeClass+0x3E8, 0, 0);  // arg3=0 normal search
    if (iVar8 != 0) {
        FootClass__Stop_Moving();
    }
}

// Step B: if already has a destination, skip to default timer
if (param_1[0x169] != 0) goto switchD_default;

// Step C: no destination yet — try to dock
piVar3 = Find_Docking_Bay(TypeClass+0x3E8, 0, 0);  // first search, arg3=0
```

Then the logic splits on Teleporter flag:

**Normal harvester path (Teleporter=no, cVar1 == '\0'):**
```c
if (piVar3 != NULL) {
    // compute distance (euclidean in leptons)
    dist = sqrt(dx^2 + dy^2 + dz^2);  // via Sqrt_Approx + Math__ftol
    if (dist <= RulesClass+0xD78 * 0x100) {  // HarvesterTooFarDistance * 256
        iVar8 = vtable+0x278(Move=2, piVar3);   // Can_Enter_Building(Move, refinery)
        if (iVar8 == 1) {
            param_1[0x2f] = 3;                  // advance to state 3 (ENTER_REFINERY)
            goto switchD_default;
        }
    }
}
```

**Chrono miner path (Teleporter=yes, cVar1 != '\0'):**
```c
else if (piVar3 != NULL) {
    dist = sqrt(dx^2 + dy^2 + dz^2);
    if (dist <= RulesClass+0xD7C * 0x100) {  // ChronoHarvTooFarDistance * 256
        iVar8 = vtable+0x278(Move=2, piVar3);   // Can_Enter_Building(Move, refinery)
        if (iVar8 == 1) {
            param_1[0x2f] = 3;
            goto switchD_default;
        }
    }
}
```

**If the above checks fail (distance > threshold or Can_Enter rejected), the "too far" branch runs:**

---

## 3. The "Too Far" Branch — Exact Code

```c
// 2nd Find_Docking_Bay: fog/reservation-ignoring fallback
g_MapEditorMode = g_MapEditorMode + 1;                              // bypass Is_Enemy gate
piVar3 = Find_Docking_Bay(TypeClass + 0x3E8, /*arg2=*/0, /*arg3=*/1);  // reservation-free search
g_MapEditorMode = g_MapEditorMode + -1;

if (piVar3 != NULL) {
    // Recompute distance to the dock found by the fallback search
    piVar4 = (int *)(*piVar3->vtable+0x48)();     // refinery->GetCoord()
    piVar5 = (int *)(this->vtable+0x48)(local_20); // this->GetCoord()
    iStack_34 = piVar5[0] - piVar4[0];            // dx in leptons
    iStack_30 = piVar5[1] - piVar4[1];            // dy in leptons
    iStack_2c = piVar5[2] - piVar4[2];            // dz in leptons
    Sqrt_Approx(dy*dy + dz*dz + dx*dx);
    iVar8 = Math__ftol();                          // integer distance in leptons

    if ((0x300 < iVar8) || (cVar1 != '\0')) {      // 768 leptons OR is chrono miner

        // --- Compute dock entrance cell ---
        aiStack_10[2] = piVar3[0x29];              // refinery Z (height level)

        // Convert leptons → cells (arithmetic: (lepton_coord + sign_ext) >> 8)
        sVar10 = (short)(piVar3[0x27] + (piVar3[0x27] >> 0x1f & 0xff) >> 8);  // refinery cell X
        sVar2  = (short)(piVar3[0x28] + (piVar3[0x28] >> 0x1f & 0xff) >> 8);  // refinery cell Y

        // Apply QueueingCell offset from BuildingTypeClass:
        //   piVar3[0x148] = building TypeClass ptr (= instance_byte_offset 0x520)
        //   TypeClass+0x1618 = QueueingCell X offset (short, art.ini key)
        //   TypeClass+0x161C = QueueingCell Y offset (short, art.ini key)
        uStack_54 = CONCAT22(
            sVar2 + *(short *)(piVar3[0x148] + 0x161c),   // target Y = refinery_Y + QCell.Y
            *(short *)(piVar3[0x148] + 0x1618) + sVar10   // target X = refinery_X + QCell.X
        );
        uStack_4c = 0;   // out-param: flags
        uStack_4a = 0;   // out-param: secondary

        // --- Find passable cell near dock entrance ---
        puVar6 = FootClass__Find_Nearby_Passable_Cell(
            auStack_3c,    // output buffer (result cell packed)
            &uStack_54,    // center: refinery_cell + QueueingCell offset
            2,             // search radius: 2 cells
            0xffffffff,    // exclude_object: -1 (none excluded)
            0,             // param_5: 0
            0,             // param_6: 0
            1,             // param_7: 1 — include bridge cells
            1,             // param_8: 1 — require clear of obstacles
            0, 0, 0,
            1,             // param_12: 1 — require occupancy-clear
            &uStack_4c,    // preferred cell hint (initially INVALID = DAT_00abd480)
            0,             // param_14
            0              // param_15
        );

        // --- Set (or clear) destination ---
        uStack_54 = *puVar6;   // result packed cell coord
        if (result_cell == INVALID) {
            (*vtable+0x480)(0, 1);             // clear destination
        } else {
            CellClass* cell = MapClass__Get_CellClass(&result_cell);
            (*vtable+0x480)(cell);             // SET destination to passable cell
        }
    }
}
goto switchD_default;   // return with default mission timer (no state change)
```

State is NOT advanced (stays at 2). The harvester moves toward the intermediate cell,
and state 2 re-evaluates next tick.

---

## 4. Verified Field: RulesClass Distance Thresholds

| Offset | INI Key | Default | Unit | Meaning |
|--------|---------|---------|------|---------|
| Rules+0xD78 | `HarvesterTooFarDistance` | **5** | cells | Normal harvester max dock distance |
| Rules+0xD7C | `ChronoHarvTooFarDistance` | **50** | cells | Chrono miner max direct-teleport distance |

**Verification:**
- String "HarvesterTooFarDistance" at 0x0083c480, xref from `RulesClass__ReadGeneral` at 0x0066FFE3 [DATA]. ✓
- Disassembly at 0x0066FFF0: `89 86 78 0d 00 00` = `mov [esi+0xD78], eax` (store result back). ✓
- String "ChronoHarvTooFarDistance" at 0x0083c464, xref from `RulesClass__ReadGeneral` at 0x00670003 [DATA]. ✓
- Disassembly at 0x0066FFF6: `8b 8e 7c 0d 00 00` = `mov ecx, [esi+0xD7C]` (load current value as default). ✓
- Values confirmed in retail INI: `rules.ini` and `rulesmd.ini` both set `HarvesterTooFarDistance=5`
  and `ChronoHarvTooFarDistance=50` in the `[General]` section. ✓
- Distance comparison in binary: `*(int *)(g_RulesClass_Instance + 0xd78) * 0x100` (multiplied by 256
  leptons/cell to convert cells → leptons for the lepton-space distance comparison). ✓

**INI comment from rulesmd.ini** (verbatim, explains design intent):
```
; HarvesterTooFarDistance: If a harvester is farther than this from the refinery
;   it wants, it will move next to it instead of reserving it and refigure things
;   out when it stops. Should be small to approximate wait time concern vs. driving
;   to next refinery.
; ChronoHarvTooFarDistance: Same but for Chrono harvesters. Rather than have them
;   teleport super far and then repick an ore patch (or teleport super far and
;   drive super far back), they will stay on their side of the map.
```

Active in YR: **Yes**. Both keys present and set in rulesmd.ini.

---

## 5. The 0x300-Lepton Threshold

**Value:** `0x300` = 768 leptons = **3 cells** (at 256 leptons/cell).

**Origin:** Hardcoded constant in `UnitClass__Mission_Harvest` at 0x0073E5E0. It does NOT
come from RulesClass and has no INI key.

**Purpose:** After the 2nd (fallback) Find_Docking_Bay finds a refinery, the "too far"
sub-branch only pathfinds to an intermediate cell if `distance > 3 cells OR unit is chrono miner`.
If the refinery is ≤ 3 cells away and the unit is a normal harvester, no intermediate cell
is set — the harvester simply returns with the default timer and will retry Can_Enter_Building
next state-2 tick.

**Verified from decompilation:** `if ((0x300 < iVar8) || (cVar1 != '\0'))` ✓

Active in YR: **Yes**. Hardcoded branch in live code path.

---

## 6. 2nd Find_Docking_Bay Call — Arg3 Semantics

**Function:** `FootClass__Find_Docking_Bay` at 0x004DF040, vtable slot 0x528.
**Call in state 2:** `Find_Docking_Bay(TypeClass+0x3E8, 0, 1)` — arg3=1.

**Arg3=1 semantics** (from FIND_DOCKING_BAY_FALLBACK_ARG3_GHIDRA_REPORT.md, verified):
- The inner evaluator at 0x004DEE80 checks `arg3 == 1` at ~0x4DEF01: `cmp [esp+0x48],1; je SKIP`
- When arg3=1: the reservation-list check (`FUN_0065ADF0`) is **skipped** entirely.
- `FUN_0065ADF0` checks whether the dock has a free slot or a slot pre-reserved for this unit.
  When skipped, a dock whose reservation slots are all taken by OTHER harvesters is still returned.
- All other filters (CanDock, on-map, state, ownership) still apply.

**g_MapEditorMode bracket:**
- `g_MapEditorMode` global at 0x00A8E7AC.
- Incremented before the call, decremented after.
- Makes `HouseClass__Is_Enemy` return true unconditionally (verified at 0x0050157C).
- Practical effect: defensive bypass for any Is_Enemy-gated alliance checks downstream.
  For a harvester seeking its own house's refinery (the normal case), `CanDock` takes the
  same-house path which does NOT call `Is_Enemy`, so g_MapEditorMode has no observable
  effect on the result. The bracket is a safety measure.

Active in YR: **Yes**. Fires every state-2 tick where no directly-dockable refinery was found.

---

## 7. FootClass__Find_Nearby_Passable_Cell (0x0056DC20)

**Ghidra label:** `FootClass__Find_Nearby_Passable_Cell`
**Address:** `0x0056DC20`
**Body:** `0x0056DC20` – `0x0056E7B5`
**Calling convention:** `__thiscall` (this = FootClass instance in ecx, but the first
formal param is `int param_1` = this after Ghidra's representation).
**Also known as in ADDRESS_MAP.md:** "Pathfinding validate (alternate)" @ 0x0056DC20
(distinct from 0x0056D230 "Pathfinding validate (rally point)").

**Actual arguments as called from Mission_Harvest state 2:**
```
param_1  (this)    = harvester unit instance
param_2            = &uStack_54  (output: receives the chosen cell packed coord)
param_3  (center)  = &uStack_54 initially = refinery_cell + QueueingCell offset (packed short,short)
param_4            = 2           (search radius in cells; scans up to 2 cells from center)
param_5            = 0xffffffff  (exclude_object: no specific object excluded)
param_6            = 0           
param_7            = 0          
param_8            = 1           (bridge_cells: include bridge cells in search)
param_9            = 1           (obstacle_free: require cell free of obstacles)
param_10           = 0, 0, 0
param_13           = 1           (occupancy_clear: require no current occupant)
param_14           = &uStack_4c  (preferred cell hint, initialized to INVALID)
param_15           = 0
param_16           = 0
```

**Algorithm summary** (from decompilation at 0x0056DC20):
1. Starts at center cell (refinery + QueueingCell offset).
2. Expands outward in a square ring pattern up to radius=2 (up to 24 candidates, cap at local_c0[24]).
3. For each candidate: checks `CellRect__CheckPassability` + obstacle-free + occupancy-free.
4. Splits candidates into two lists: on-same-cell (FUN_006d6410 locking check) vs off-cell.
5. If a preferred cell hint is provided and non-INVALID, picks the closest candidate to that hint.
6. Otherwise picks the candidate at `frame_counter % candidate_count` (deterministic-but-rotating selection).
7. Writes the chosen cell to `*param_2` (uStack_54).
8. If no valid cell found, writes `DAT_00abd480` (the INVALID sentinel).

**Active in YR:** Yes. Fires whenever a harvester is too far from its refinery.

---

## 8. What Unit-Side Field Changes

After `FootClass__Find_Nearby_Passable_Cell` returns:

```c
if (result_cell == INVALID_CELL) {           // == DAT_00b1cfb8 sentinel
    (*vtable+0x480)(0, 1);                   // clear destination
} else {
    CellClass* cell = MapClass__Get_CellClass(&result_cell);
    (*vtable+0x480)(cell);                   // SET destination to passable cell near refinery
}
```

**`vtable+0x480`** sets the unit's movement destination. This is the standard
`FootClass::Set_Primary_Target` / "Set destination" slot, writing to `param_1[0x169]` (instance
byte offset 0x5A4 = Destination pointer).

**State does NOT change.** `param_1[0x2f]` (MissionSubState at 0xBC) remains 2 (RETURN_TO_REFINERY).
On the next state-2 tick, the harvester's new destination causes `param_1[0x169] != 0` to be true,
sending control to `switchD_0073e6ea_default` immediately (step A triggers StopMoving for chrono
miners when they arrive, step B/goto fires for normal harvesters). The harvester moves toward the
intermediate cell and tries docking again each tick.

**Only field changed:** Destination (instance+0x5A4) — set to passable cell adjacent to refinery's
QueueingCell offset.

---

## 9. Chrono Miner vs Normal Harvester Differences

| Condition | Normal Harvester | Chrono Miner |
|-----------|-----------------|--------------|
| Direct-dock threshold | 5 cells (HarvesterTooFarDistance) | 50 cells (ChronoHarvTooFarDistance) |
| "Too far" sub-branch condition | `dist > 3 cells (0x300 leptons)` | ALWAYS (cVar1 != '\0') |
| Pathfind to intermediate cell | Yes when dist > 3 cells | Yes unconditionally |
| State 3 (Can_Enter_Building) fires | When dist ≤ 5 cells AND can dock | When dist ≤ 50 cells AND can dock |

For the chrono miner, the `cVar1 != '\0'` override in `if ((0x300 < iVar8) || (cVar1 != '\0'))`
means the pathfind-to-intermediate-cell sub-branch **always fires** when a dock is found but
Can_Enter_Building wasn't called (e.g., dist is > 50 cells). This is the Teleporter=yes path.

**Active in YR:** Yes for both paths. The Teleporter=yes path is CMIN-specific (chrono miner
has `Teleporter=yes` in rulesmd.ini). Fires in any skirmish map where CMIN is built.

---

## 10. QueueingCell Offset (BuildingTypeClass+0x1618/+0x161C)

From BUILDINGCLASS_MASTER_GHIDRA_REPORT.md and BUILDINGTYPECLASS_CTOR_DEFAULTS.md (verified):

| TypeClass Offset | Field | Type | Default | INI Key |
|-----------------|-------|------|---------|---------|
| +0x1618 | QueueingCell.X | short | UNINIT (ctor bug — only max written at 0x161C) | art.ini `QueueingCell=` |
| +0x161C | QueueingCell.Y | short | 0 | art.ini `QueueingCell=` |

In Mission_Harvest state 2 "too far" branch:
```c
// piVar3[0x148] = piVar3[0x520 bytes] = refinery->TypeClass (BuildingTypeClass*)
dock_X = refinery_cell_X + *(short *)(piVar3[0x148] + 0x1618);
dock_Y = refinery_cell_Y + *(short *)(piVar3[0x148] + 0x161C);
```

This computes the cell offset from the refinery's center where harvesters should queue.
The result is the center for `FootClass__Find_Nearby_Passable_Cell`.

**Active in YR:** Yes. Any refinery with `QueueingCell=` in art.ini shifts the target.
Stock refineries use the default (which due to the ctor bug is uninit for X, 0 for Y).

---

## 11. Active-in-YR Summary

| Finding | Active in YR |
|---------|-------------|
| Rules+0xD78 HarvesterTooFarDistance (default 5) | Yes |
| Rules+0xD7C ChronoHarvTooFarDistance (default 50) | Yes |
| 0x300-lepton hardcoded threshold | Yes |
| 2nd Find_Docking_Bay with arg3=1 (skip reservation) | Yes |
| g_MapEditorMode bracket (defensive Is_Enemy bypass) | Yes |
| FootClass__Find_Nearby_Passable_Cell at 0x0056DC20 | Yes |
| QueueingCell offset (BuildingTypeClass+0x1618/0x161C) | Yes |
| Destination written via vtable+0x480 | Yes |
| State stays at 2 after branch (no advance to 3) | Yes |
| Chrono miner always enters pathfind sub-branch | Yes (Teleporter=yes path) |

---

## 12. Confidence Summary

| Claim | Confidence | Evidence |
|-------|-----------|----------|
| Rules+0xD78 = HarvesterTooFarDistance, default 5 | HIGH | Binary disasm 0x0066FFF0 `mov [esi+0xD78],eax`; INI xref at 0x0083c480; retail INI value |
| Rules+0xD7C = ChronoHarvTooFarDistance, default 50 | HIGH | Binary disasm 0x0066FFF6 `mov ecx,[esi+0xD7C]`; INI xref at 0x0083c464; retail INI value |
| 0x300 threshold is hardcoded (not from Rules) | HIGH | Literal in decompiled Mission_Harvest; no INI read trace to this constant |
| 0x300 = 768 leptons = 3 cells | HIGH | 0x300/0x100 = 3; verified lepton-per-cell constant from distance multiplier |
| 2nd Find_Docking_Bay arg3=1 skips reservation check | HIGH | FIND_DOCKING_BAY_FALLBACK_ARG3_GHIDRA_REPORT.md (verified) |
| FootClass__Find_Nearby_Passable_Cell at 0x0056DC20 | HIGH | Ghidra label + body bounds 0x0056DC20–0x0056E7B5; ADDRESS_MAP.md entry |
| Find_Nearby_Passable_Cell search radius = 2 cells | HIGH | Literal arg `2` in call: `FootClass__Find_Nearby_Passable_Cell(..., 2, 0xffffffff, ...)` |
| Destination (instance+0x5A4) updated by vtable+0x480 | HIGH | Decompiled call: `(*vtable+0x480)(MapClass__Get_CellClass(&result_cell))` |
| State stays at 2 (no MissionSubState change) | HIGH | No `param_1[0x2f] = N` assignment in this branch; falls through to default timer |
| QueueingCell offsets at BuildingTypeClass+0x1618/0x161C | HIGH | Confirmed by 3 docs: BUILDINGCLASS_MASTER, BUILDINGTYPECLASS_CTOR_DEFAULTS, FIND_DOCKING_BAY_FALLBACK_ARG3 |
| Chrono miner always enters sub-branch | HIGH | `cVar1 = *(TypeClass+0xCD4)` (Teleporter flag); `(0x300 < iVar8) || (cVar1 != '\0')` |

---

## 13. Rust Port Implications

1. **Two separate thresholds:** Implement `if teleporter { chrono_thresh } else { normal_thresh }` —
   HarvesterTooFarDistance (5) and ChronoHarvTooFarDistance (50) are read from RulesClass.
   Multiply by 256 (leptons/cell) before comparing against euclidean lepton distance.

2. **0x300 hardcoded gate:** After the fallback dock search, pathfind to intermediate cell
   ONLY if `dist > 768 leptons OR is_chrono_miner`. This prevents redundant pathfinding when
   the harvester is already close enough to dock on the next tick.

3. **QueueingCell:** The intermediate cell center is `refinery_cell + (BuildingTypeClass.queueing_cell_x, queueing_cell_y)`.
   Note the known ctor bug: `.x` (offset 0x1618) is uninitialized if no INI value set; initialize to 0 in Rust.

4. **Find_Nearby_Passable_Cell radius=2:** The passable cell search spans 2 cells from the
   QueueingCell-adjusted dock entrance.

5. **State stays at 2:** Do not advance MissionSubState to 3 in this branch. Only `vtable+0x480`
   (Set_Destination) changes; state advances to 3 on a LATER tick via Can_Enter_Building.

6. **Reservation bypass:** The 2nd dock search (arg3=1) must skip reservation slot checks.
   A harvester can target an already-fully-booked refinery as its intermediate destination.
