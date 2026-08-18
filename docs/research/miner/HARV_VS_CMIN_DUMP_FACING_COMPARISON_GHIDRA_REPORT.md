# HARV vs CMIN Dump-Time Facing Comparison — Ghidra Research Report

**Date:** 2026-05-19
**Binary:** gamemd.exe (YR 1.001)
**Confidence:** HIGH (all key claims verified directly from live decompilation)
**Active in YR:** YES — fires every harvest cycle for both HARV and CMIN
**Scope:** Radio-0x16 dispatch at dock arrival; chrono vs standard gating; dump-time facing value.

---

## Supersession 2026-05-26

This report remains useful for one point: the radio `0x16` path is not
chrono-specific, and HARV/CMIN both reach the same unit-side receiver when the
stock refinery protocol sends it.

Do not use this report's "FACE_DOCK", "facing setter", or "pivoted to facing
East" wording as the current mechanism contract. Later audits and fresh
mission-deploy verification refine the model:

- radio `0x16` calls the active locomotor vtable `+0x4C(0x4000)` under the
  ordinary sync branch, but it does not directly write unit body facing, call
  `GetDockCoord`, set destination, write position, or start unload;
- later/already-synced `0x16` can send `0x15` only under the stopped,
  building-destination, contact-entered, mission-7 predicates;
- mission `0x10` / `UnitClass::Mission_Deploy_Building` owns the deploy-facing
  gate before unload display starts, using the `RateTimer` accept expression
  `((current >> 7) + 1) & 0x1FE == 0x80`;
- accepted unload-start does not directly snap body facing to East.

Current authorities:
`DOCK_ARRIVAL_PIVOT_SEQUENCE_DOC_CONFLICT_AUDIT_GHIDRA_REPORT.md`,
`DOCK_0X16_DOTURN_RATETIMER_UNLOAD_GATE_RECHECK_20260526.md`, and
`MISSION_DEPLOY_BUILDING_DEPLOY_FACING_VISIBLE_DUMP_FACING_GHIDRA_REPORT.md`.

---

## Key Question

Does radio-0x16 (the "face the dock" pivot) apply to both HARV (War Miner,
DriveLocomotionClass) and CMIN (Chrono Miner, TeleportLocomotionClass piggybacking
DriveLocomotionClass), or is it gated on a chrono-specific flag?

**Answer: The pivot is HARVESTER-GENERAL. There is NO chrono-specific gate in the
radio-0x16 receiver. Both HARV and CMIN go through the identical code path.**

---

## 1. Radio-0x16 Receiver — `UnitClass::Receive_Radio` case 0x16

**Verified via:** `decompile_function 0x737430`
**Function:** `UnitClass__Receive_Radio` at 0x737430–0x737B51

Full case 0x16 body (pseudocode from live decompile):

```c
case 0x16:
    FootClass__Receive_Radio(param_2, param_3, param_4);  // forward to base

    // Gate 1: field at unit+0x6af — locomotion "is moving" / path-lock flag
    // Gate 2: current heading != 0x4000
    if ((*(char *)((int)param_1 + 0x6af) == '\0') &&
        (psVar4 = (short *)RateTimer__Current(local_1c), *psVar4 != 0x4000)) {

        // SET FACING to 0x4000 via ILocomotion vtable+0x4c (Head_To facing setter)
        (**(code **)(*(int *)param_1[0x19d] + 0x4c))((int *)param_1[0x19d], 0x4000);
        return 1;
    }

    // Secondary branch: if locomotion not moving, destination is a building (RTTI==6),
    // and current mission is Enter (7) → send radio 0x15 (DOCK_NOW) to the building
    cVar1 = (**(code **)(*(int *)param_1[0x19d] + 0x10))((int *)param_1[0x19d]);
    if (((cVar1 == '\0') &&
         (piVar5 = (int *)FootClass__GetDestination(0), (char)param_1[0x106] != '\0')) &&
        ((piVar5 != (int *)0x0 &&
          ((iVar7 = (**(code **)(*piVar5 + 0x2c))(), iVar7 == 6 &&
            (iVar7 = (**(code **)(*param_1 + 0x184))(), iVar7 == 7)))))) {
        (**(code **)(*param_1 + 0x278))(0x15, piVar5);  // radio 0x15 DOCK_NOW
    }
    return 1;
```

### Key observations

| Field / Call | Byte Offset | Value | Meaning |
|---|---|---|---|
| `param_1[0x19d]` | unit+0x674 | locomotion ptr | ILocomotion COM pointer (same slot for HARV and CMIN) |
| `*(int *)param_1[0x19d] + 0x4c` | loco vtable slot 0x4c | function ptr | ILocomotion::Head_To (facing setter) |
| `0x4000` | — | 16384 decimal | Target facing value — see §2 |
| `*(char *)((int)param_1 + 0x6af)` | unit+0x6af | bool flag | Locomotion-path-lock / "already pivoted" flag |
| Gate check | — | no `+0xCD4` | **NO Teleporter flag check anywhere in case 0x16** |

**The `Teleporter` flag (TechnoTypeClass+0xCD4) is never read in case 0x16.**
**The `Harvester` flag (TechnoTypeClass+0xE0E) is never read in case 0x16.**
**The only gate is the locomotion-moving flag at unit+0x6af and whether facing is already 0x4000.**

---

## 2. Facing Value: 0x4000

The target facing set by radio-0x16 is **0x4000 (16384)** in the 16-bit facing system
(0x0000–0xFFFF for 0–360°, wrapping).

Converting to the 8-bit facing system (0–255 used elsewhere in gamemd.exe):
- 0x4000 in 16-bit = 16384
- 16384 / 65536 × 256 = **64** in 8-bit facing
- 8-bit 64 = **East** (0=North, 64=East, 128=South, 192=West)

Both HARV and CMIN are pivoted to facing **East (64)** during dock arrival, not "south" or
any chrono-specific direction. This is consistent across both unit types because the same
locomotion vtable slot is called with the same argument.

---

## 3. Who Sends Radio-0x16 and When?

**Sender:** `BuildingClass::Receive_Radio` case 0xE (CAN_DOCK), verified via
`decompile_function 0x43C2D0`.

From the BuildingClass side (case 0xE, DockUnload/Weeder refinery branch):

```c
// After DOCK_LINK (radio 2) and ENTER_DOCK (radio 0x18):
iVar10 = (**(code **)(param_1->vtable + 0x278))(0x16, param_2);  // radio 0x16 → unit
if (iVar10 == 1) {
    return 1;
}
// If radio 0x16 not accepted, scatter the unit
(**(code **)(param_2->vtable + 0x174))(&DAT_0089c848, 1, 1);
return 1;
```

The building sends radio 0x16 to `param_2` = the requesting harvester. `param_2` is typed
as `TechnoClass*` with no Teleporter check. The building does not inspect whether the
sender is HARV or CMIN — it sends 0x16 to any unit that has passed the DockUnload/Weeder
gate (which requires the unit to have `Harvester=yes` via `TechnoTypeClass+0xE0E`).

**Both HARV and CMIN reach this radio-0x16 dispatch because both have `Harvester=yes`.**

---

## 4. HARV vs CMIN Locomotion at Dock Arrival

At the moment radio-0x16 is received, both miners are using **DriveLocomotionClass** as
the active locomotor:

- **HARV (War Miner):** DriveLocomotionClass is the permanent primary locomotor.
  `unit+0x674` always points to a DriveLocomotionClass instance.

- **CMIN (Chrono Miner):** By the time the miner arrives at the dock cell, `FootClass::AI`
  (0x4DA530) has swapped in DriveLocomotionClass as the active locomotor (piggybacked over
  TeleportLocomotionClass, which is suspended). The swap happens when
  `TeleportLocomotionClass::Is_Ok_To_End` returns true after the warp completes.
  At dock arrival, `unit+0x674` therefore also points to a **DriveLocomotionClass**
  instance for CMIN.

Consequently, the ILocomotion vtable read at `*(int *)param_1[0x19d] + 0x4c` resolves
to **DriveLocomotionClass**'s slot 0x4c for both miners. Same vtable function, same
argument (0x4000), same outcome.

---

## 5. BuildingClass Radio-0xE: No Teleporter Gate Confirmed

The complete BuildingClass case 0xE dispatch sequence for DockUnload/Weeder refineries
(from decompile of 0x43C2D0):

1. Compute queue cell: `(building_top_left_x + 3, building_top_left_y + 1)`
2. Send radio 0x12 (MOVE_TO_CELL) → if not accepted (0x14), return
3. Send radio 0x18 (ENTER_DOCK) to unit
4. **Send radio 0x16 (TIMING_SYNC/FACE_DOCK) to unit**
5. If radio 0x16 returns 1: accept and return
6. Otherwise: scatter the unit

No check on `TechnoTypeClass+0xCD4` (Teleporter) or any chrono-specific field at any
point in this sequence. The dispatch is identical for any `Harvester=yes` unit in the
dock queue.

---

## 6. Summary Table

| Aspect | HARV (War Miner) | CMIN (Chrono Miner) |
|--------|-----------------|----------------------|
| Radio-0x16 received? | YES | YES |
| Code path in `UnitClass::Receive_Radio` | Case 0x16, identical | Case 0x16, identical |
| Teleporter gate in case 0x16? | NO | NO |
| Harvester gate in case 0x16? | NO | NO |
| Active locomotor at dock arrival | DriveLocomotionClass | DriveLocomotionClass (Drive piggybacked over Teleport) |
| ILocomotion vtable slot called | +0x4c (Head_To facing) | +0x4c (Head_To facing) |
| Facing value set | **0x4000 (= East, 64 in 8-bit)** | **0x4000 (= East, 64 in 8-bit)** |
| Gate that prevents pivot | `unit+0x6af != 0` OR current facing already 0x4000 | same |

---

## 7. Implication for the Rust Engine Fix

The dock-arrival facing pivot (radio-0x16 → face 0x4000) is **harvester-general behavior**,
not a chrono-miner-specific behavior. Any fix to the pivot must apply to ALL units that
receive radio-0x16, which means any unit with `Harvester=yes` docking at a `DockUnload=yes`
or `Weeder=yes` refinery.

The fix is not a special-case for CMIN — it belongs in the generic
`UnitClass::receive_radio` handler for command 0x16.

---

## 8. Verification Citations

| Claim | Verified via |
|-------|-------------|
| `UnitClass__Receive_Radio` at 0x737430 | `get_function_by_address 0x737430` → confirmed label and body range |
| Case 0x16 body (facing set, no Teleporter gate) | `decompile_function 0x737430` — full switch decompile |
| Building sends radio 0x16 in CAN_DOCK (case 0xE) | `decompile_function 0x43C2D0` — case 0xE DockUnload branch |
| Locomotion pointer at unit+0x674 (`param_1[0x19d]`) | Confirmed from MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md §9.3, cross-checked in decompile of 0x737430 |
| CMIN uses DriveLocomotionClass at dock arrival | CHRONO_MINER_SYSTEM_OVERVIEW.md §8 step 10; MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md §9 |
| `Harvester=yes` at TechnoTypeClass+0xE0E required for DockUnload acceptance | `decompile_function 0x43C2D0` case 0xF, `*(char *)(*(int *)&param_2[1].field_0x1a4 + 0xe0e)` |

---

## 9. TS-Legacy Filter

No Tiberian Sun legacy code detected in the radio-0x16 path. The DockUnload refinery
docking protocol is live YR gameplay (refineries are central to YR economy).

The `RateTimer__Current` call in the gate check (`*psVar4 != 0x4000`) and the locomotion
vtable call are both in live YR code paths — verified by the fact that `BuildingClass`
unconditionally sends radio 0x16 to any docked harvester.
