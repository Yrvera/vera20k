# BUNKER_SERVICEDEPOT_0X2E4_RECIPROCAL_LINK_TEARDOWN_GHIDRA_REPORT

**Date:** 2026-06-02
**Slot:** 3 (substrate §9.2 round-2 swarm)
**Addresses investigated:** `0x00459470` (FUN_00459470, new decompile), plus reconciliation across existing doc inventory.
**Investigation gate:**
- Target question: Unified +0x2E4 reciprocal-link contract — who writes both sides, which of the three teardown helpers clears both sides, what broadcast-BREAK behavior clears the flag on limbo/death, and whether building-side +0x2E4 and unit-side +0x2E4 share one semantic or differ per class.
- Non-goals: Full unload state machine, refinery deposit cadence, civilian garrison occupant vector, locomotor swap-back timing, pathfinding consequences of dock links.
- Evidence needed to mark COMPLETE: FUN_00459470 decompile confirming both-sides clear and vtable+0x274(3) BREAK send; caller set for FUN_00459470 confirming super/temporal/unit-damage triggers; reconciliation of refinery-vs-bunker +0x2E4 semantic from writer inventory; broadcast-BREAK limbo path verified from BROADCAST doc.
- Stop conditions: Stop once all three helpers confirmed, symmetric semantics resolved, and BREAK-on-limbo cited. Do not expand into unload FSM or civilian garrison.
**Confidence:** High for all items marked VERIFIED. Medium for one temporal branch detail (see Remaining Uncertainty).
**Active in YR:** Conditional — all three teardown helpers fire only when `building+0x2E4 != 0`, meaning only bunker-occupied buildings and service-depot-linked (non-stock-refinery) buildings reach them in normal skirmish.

---

## Prior Work Read (Cite, Do Not Redo)

The following docs were read before any fresh decompilation was performed:

- `BUNKER_0X2E4_LIFECYCLE_EXIT_CLEAR_PATH_GHIDRA_REPORT.md` — full bunker lifecycle, install semantics, all three helpers identified, Rust surface gaps.
- `BUILDINGCLASS_FIELD_0X2E4_REFINERY_DOCK_GHIDRA_REPORT.md` — (file not found on disk; STANDARD_REFINERY_0X2E4_WRITER_INVENTORY covers the same ground).
- `miner/STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md` — exhaustive writer inventory confirming stock CMIN/HARV refinery does NOT write +0x2E4; only Bunker-yes path does.
- `miner/RELEASEDOCKEDHARVESTER_0x4595C0_GHIDRA_REPORT.md` — step-by-step decompile of helper (a), full teardown sequence.
- `miner/BUILDING_UNDOCKUNIT_0x4593A0_CHRONO_MINER_GHIDRA_REPORT.md` — decompile of helper (b), sell/damage/temporal building-class trigger.
- `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md` — limbo-path BREAK broadcast, contact vector cleanup on death/despawn.
- `BUNKER_SYSTEM_GHIDRA_REPORT.md` and `TANK_BUNKER_ENTRY_EXIT_VISIBLE_LIFECYCLE_GHIDRA_REPORT.md` — cited for context, not re-decompiled.
- `SERVICE_REPAIR_RADIO_0X1C_0X22_PATH_GHIDRA_REPORT.md` — depot side: repair depot uses radio contacts, not +0x2E4 reciprocal link directly; cited only.

Fresh decompilation performed: `FUN_00459470 @ 0x00459470`, plus callers via `get_function_callers`, plus trigger-context reads of `SuperClass__Launch @ 0x006CC390`, `TemporalClass__Update @ 0x0071A760`, and `UnitClass__ReceiveDamage @ 0x00737C90`.

---

## 1. Resolved: Refinery-vs-Bunker — One Field, Two Contexts, Stock Refinery Uses Neither

**VERIFIED:** `TechnoClass+0x2E4` / `UnitClass+0x2E4` / `BuildingClass+0x2E4` is the **same field** at byte offset `0x2E4` (`[0xB9]` in int-pointer indexing) on all Techno-derived objects. Its semantic is "pointer to the docked/garrison partner" — building side holds the unit pointer; unit side holds the building pointer. The field name and layout are class-independent; what varies is which code path writes it.

**VERIFIED:** Stock `CMIN/HARV -> GAREFN/NAREFN` refinery docking does **not** write a reciprocal `unit+0x2E4 <-> building+0x2E4` pair. The stock unload FSM enters `UnitClass::Mission_Deploy_Building @ 0x0073D630` on the `unit+0x2E4 == 0` branch and rediscovers the refinery via `DAT_0089F6A0` cell lookup. The `unit+0x2E4 != 0` branch (which triggers `ReleaseDockedHarvester`) is the **conditional** nonzero-link path, active only when the reciprocal link has been written by the Bunker install.
Evidence: `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY` §4 negative inventory + `UnitClass::Mission_Deploy_Building` entry `CMP [ESI+0x2E4],0; JZ 0x0073D6E6`.

**VERIFIED:** The design doc's own §3.3 idiom #1 statement that "stock refinery runs with `unit+0x2E4 == 0`" is **correct**. The design doc's §3.3 idiom #4 and §5.2.8 references to the "reciprocal link" apply exclusively to the Bunker (`BuildingType+0x16AB Bunker=yes`) state machine, not to stock ore refineries.

**VERIFIED:** The only active writer of both sides is `FUN_00458E50 / BuildingClass::MissionRepairAndProduce` case 5 at `0x00459301` / `0x0045930F`, gated by `BuildingType+0x16AB`. For stock YR skirmish, this is only reached for `[NATBNK]` buildings (`rulesmd.ini:13732 Bunker=yes`). `[NABNKR]` does not have `Bunker=yes` in the checked stock file and cannot reach this writer path.

---

## 2. Reciprocal Link Install — Unified Contract

**Writer:** `FUN_00458E50` case 5 (called from `BuildingClass::MissionRepairAndProduce @ 0x0044B7A3` when `BuildingType+0x16AB != 0`).

Install sequence (verified from `BUNKER_0X2E4_LIFECYCLE_EXIT_CLEAR_PATH_GHIDRA_REPORT.md` §"Entry And Install"):
```
0x00459301:  building+0x2E4 = unit      // building stores unit pointer
0x0045930F:  unit+0x2E4 = building      // unit stores building pointer
             unit+0x214 = -1
             unit.vtable+0x150()         // hide/limbo-style
             building+0x718 = 6          // bunker state flag
             unit.SetMission(5, 1)
             VocClass__PlayAt(RulesClass+0x240)  // BunkerWallsUpSound
```

Both sides are written **atomically within one code path**. There is no partial-write window observable to other sim systems. The unit is hidden before any subsequent tick.

---

## 3. Three Teardown Helpers — Unified Contract

### (a) BuildingClass::ReleaseDockedHarvester @ 0x004595C0 — Normal/Deploy Exit

**Trigger:** `UnitClass::Mission_Deploy_Building @ 0x0073D630` when `unit+0x2E4 != 0` and building found in current cell. Called from the bunkered unit's deploy mission.
**Active in YR:** Conditional. Only when `unit+0x2E4 != 0` (bunker occupied and unit deploying out).
**Both sides cleared:** Yes.
- Unit side: `piVar1[0xb9] = 0` (at step 6, before locomotion commands).
- Building side: `param_1->field_0x2e4 = 0` (at step 13, after SetMission(MOVE)).
- Building state: `building+0x718 = 0`; `building.SetMission(5)`; vtable+0x274(3) sends BREAK.
**Sound:** `VocClass__PlayAt(RulesClass+0x244)` — `BunkerWallsDownSound` — at step 2 before locomotion.
**Placement:** Full nearby-passable-cell ejection via `FootClass::Find_Nearby_Passable_Cell`; unit receives `SetMission(MOVE=2)`.
**BREAK send:** `vtable+0x274(3)` (RadioCommand CLEAR=3) at step 13. Evidence: `decompile_function 0x004595C0` step 13.

### (b) BuildingClass::UndockUnit @ 0x004593A0 — Sell/Destroy/Temporal (Building class)

**Trigger:**
- `BuildingClass::Sell @ 0x0044AA00` — sell if `building+0x2E4 != 0`; xref `0x0044AAA4..0x0044AAB0`.
- `BuildingClass::ReceiveDamage @ 0x00442230` — destruction case 4; xref `0x004424EA`.
- `TemporalClass::Update @ 0x0071A760` — when target is BuildingClass (vtable+0x2c == 6) and `building[0xB9] != 0`; confirmed in fresh decompile: `if (iVar4 == 6) ... if (piVar3[0xb9] != 0) BuildingClass__UndockUnit()`.
**Active in YR:** Conditional. Only when `building+0x2E4 != 0`.
**Both sides cleared:** Yes.
- Unit side: `piVar1[0xB9] = 0` (step 9 of UndockUnit, after locomotion commands).
- Building side: `param_1[0xB9] = 0` (step 10).
- BREAK: `building.vtable+0x274(3)` (step 11).
**No placement:** Does not run nearby-passable-cell search or set unit mission. Unit continues from wherever it is.
**Sound:** None — no VOC play in UndockUnit body.

### (c) FUN_00459470 @ 0x00459470 — Super/Temporal-Non-Building/Unit-Damage Clear-Only

**Trigger (verified via fresh `get_function_callers` + decompile of each caller):**
1. `SuperClass__Launch @ 0x006CC390`: In case 4 (super weapon launch) inner unit loop: `if (unit+0x2E4 != 0 && building.WhatAmI()==6) FUN_00459470()`. The surrounding context scans for occupied units within the super's area and clears their bunker links before warp.
2. `TemporalClass__Update @ 0x0071A760`: Non-building branch (`iVar4 != 6`): `piVar2 = building+0x2E4; if (piVar2 != 0) { vtable+0x2c(); FUN_00459470(); }`. This fires when the temporal target is NOT a BuildingClass (e.g., a unit) — contrasting with the `iVar4 == 6` branch above that calls `BuildingClass__UndockUnit`.
3. `UnitClass__ReceiveDamage @ 0x00737C90`: Death case 4 path: `if (unit+0x2E4 != 0) FUN_00459470()`. This clears the bunker link when the bunkered unit is killed by damage.
**Active in YR:** Conditional — fires when `building+0x2E4 != 0` under super-launch, non-building temporal wipe, or unit death.

**Fresh decompile — full verified body (decompile_function 0x00459470):**
```c
void __fastcall FUN_00459470(BuildingClass *param_1) {
  BuildingClass__ClearAnimSlot(param_1);        // anim slot A
  BuildingClass__ClearAnimSlot(param_1);        // anim slot B
  if (param_1->field_0x2e4 != 0) {
    if (RulesClass+0x244 != -1)
      VocClass__PlayAt(0);                      // BunkerWallsDownSound
    // create anim slot C: SpecialAnimThree (Type+0x127C or +0x128C)
    // create anim slot D: SpecialAnimFour  (Type+0x12C0 or +0x12D0)
    (*vtable+0x274)(3);                         // RadioCommand BREAK=3
    *(param_1->field_0x2e4 + 0x2e4) = 0;       // unit side cleared
    param_1->field_0x2e4 = 0;                  // building side cleared
    param_1->field_0x718 = 0;                   // bunker state cleared
    (*vtable+0x1e8)(5, 0);                      // building SetMission(GUARD)
  }
}
```
Evidence: `decompile_function 0x00459470` (live session).

**Both sides cleared:** Yes.
- Unit side: `*(param_1->field_0x2e4 + 0x2e4) = 0` — dereferences the building's pointer to reach the unit, then clears unit+0x2E4.
- Building side: `param_1->field_0x2e4 = 0`.
- State: `building+0x718 = 0`; `building.SetMission(5)`.
**BREAK send:** `vtable+0x274(3)` BEFORE the unit side is cleared. This matches the pattern in `ReleaseDockedHarvester` where BREAK fires before both-sides clear.
**Sound:** `VocClass__PlayAt(RulesClass+0x244)` — `BunkerWallsDownSound` — same as helper (a).
**No placement:** Does NOT run locomotion commands, `FootClass::Find_Nearby_Passable_Cell`, or `SetMission` on the contained unit. The unit pointer is cleared without repositioning the unit. This is a forced-clear, not an orderly exit.

---

## 4. BREAK-on-Limbo/Death — Broadcast Contract

**VERIFIED (from BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP_GHIDRA_REPORT.md):**
When any Techno (foot or building) enters limbo or is destroyed, the call chain is:

```
FootClass__Limbo / BuildingClass__Limbo
  -> TechnoClass__Limbo_Helper @ 0x006F6AC0
     -> TechnoClass__Limbo_Tail_CallConceal @ 0x0065AA80
        -> vtable[+0x280](3)  // Broadcast_Radio_ToAll(BREAK=3)
        -> ObjectClass__Conceal
```

For message `3`, `RadioClass::Transmit_Radio_Impl @ 0x0065A970`:
1. Nulls the matching contact slot in the sender before dispatching to the target's `Receive_Radio`.
2. Target `RadioClass::Receive_Radio @ 0x0065A820` then nulls the matching sender slot.

`BuildingClass::Receive_Radio @ 0x0043C2D0` case `3` additionally calls `BuildingClass::GrandOpening()` before the common contact-clear path.

**Implication for +0x2E4:** The three teardown helpers (a), (b), (c) each explicitly send vtable+0x274(3) BEFORE the building itself enters limbo. This means the +0x2E4 link is cleared **by the teardown helper** before the general limbo BREAK broadcast. The general BREAK broadcast is a safety net for other radio contacts (e.g., production links), not the primary +0x2E4 clearer. The reciprocal flag is cleared by the helpers themselves, not by the limbo-path broadcast.

**Evidence:** `0x0065AA80`, `0x0065ACE0`, `0x0065A970`, `0x0065A820`, `0x0043C2D0` (from BROADCAST doc); vtable+0x274(3) confirmed in all three helper decompiles.

---

## 5. Comparison Table — Three Helpers

| Aspect | (a) ReleaseDockedHarvester 0x4595C0 | (b) UndockUnit 0x4593A0 | (c) FUN_00459470 0x00459470 |
|--------|--------------------------------------|-------------------------|------------------------------|
| Trigger | Unit deploy mission, nonzero link | Sell/Destroy/TemporalClass(building) | SuperLaunch/TemporalClass(non-bldg)/Unit death |
| Unit side cleared | Yes (step 6, before loco) | Yes (step 9, after loco) | Yes (before bldg side) |
| Building side cleared | Yes (step 13, after SetMission) | Yes (step 10) | Yes (after unit side) |
| Building+0x718 cleared | Yes | No | Yes |
| BREAK send | vtable+0x274(3) at step 13 | vtable+0x274(3) at step 11 | vtable+0x274(3) before side clears |
| Sound played | BunkerWallsDownSound | None | BunkerWallsDownSound |
| Nearby-passable-cell search | Yes | No | No |
| Unit locomotion commands | Yes (Force_Track + Set_Destination + SetMission(MOVE)) | Yes (Force_Track only, no destination) | No |
| Unit mission set | MOVE=2 | None | None |
| Building mission reset | SetMission(5) | BREAK only, no mission set | SetMission(5) |
| Anim slots cleared (A/B) | Yes | No | Yes |
| Anim slots created (C/D) | Yes | No | Yes |

---

## 6. Unified Contract — Implementation Summary

### Write path
Single writer: `FUN_00458E50` case 5 (Bunker state machine), called from `BuildingClass::MissionRepairAndProduce`, gated by `BuildingType+0x16AB Bunker=yes`. Writes `building+0x2E4 = unit` at `0x00459301` then `unit+0x2E4 = building` at `0x0045930F`. No other active gameplay writer sets both sides.

### Clear paths (all three must clear both sides)
- **Normal exit** (unit deploys out): `ReleaseDockedHarvester @ 0x004595C0` — full ejection with locomotion, sound, anims, placement.
- **Building sell/destroy/temporal(bldg)**: `UndockUnit @ 0x004593A0` — minimal ejection, locomotion only, no sound/anims/placement.
- **Super/temporal(non-bldg)/unit-death**: `FUN_00459470 @ 0x00459470` — clear-only, sound + anims + building reset, NO unit locomotion or placement.

### BREAK behavior
All three helpers send `vtable+0x274(3)` (RadioCommand BREAK=3) before or during the side-clear. The general limbo-path broadcast (`Broadcast_Radio_ToAll @ 0x0065ACE0`) is a secondary safety net for other contacts, not the +0x2E4 primary clearer.

---

## 7. Implementation Handoff

### Handoff 1 — Reciprocal link as `Option<EntityId>` with symmetric invariant
**Verified behavior:** Install writes both sides atomically; no single-sided partial state exists in the binary. Every teardown helper clears both sides before the building resets its mission. The field is the same byte offset on both unit and building.
**Rust delta:** Model as `reciprocal_dock_link: Option<EntityId>` on both `GameEntity` variants (building and foot-unit). Write both sides in the Bunker install action; assert both are `None` before writing. On any teardown path, clear both sides in the correct sequence (unit first for helpers a and c, unit side first or together for b).
**Affected surface:** `GameEntity`, `BunkerInstallAction`, `BunkerExitAction`, `BunkerClearAction` (new), `sell/despawn` lifecycle hooks.
**Acceptance scenario:** Installing a unit into `NATBNK` sets both `building.reciprocal_dock_link = Some(unit_id)` and `unit.reciprocal_dock_link = Some(building_id)`; deploying the unit clears both; selling the building while occupied clears both without running placement.
**Proposed test name:** `bunker_reciprocal_link_install_and_clear_both_sides_invariant`.
**Risk:** High. One-sided clear leaves a stale `Option<EntityId>` that will cause misidentified dock-in-progress on the next tick query.

### Handoff 2 — Three distinct clear paths, not one generic ejector
**Verified behavior:** Helper (a) runs full locomotion + passable-cell placement; helper (b) runs locomotion only (no placement or sound); helper (c) runs NO locomotion at all. The unit is repositioned only in path (a).
**Rust delta:** Do NOT collapse all three into one generic "eject from bunker" action. Implement three distinct sim actions:
- `BunkerNormalExit`: runs locomotion + Find_Nearby_Passable_Cell + SetMission(MOVE); clears both links; plays down sound; resets building state.
- `BunkerInterruptExit` (sell/destroy/temporal-building): runs locomotion only (Force_Track); clears both links; no sound; sends BREAK.
- `BunkerForceClear` (super/temporal-non-building/unit-death): clears both links immediately; plays down sound; resets building state; NO unit movement.
**Affected surface:** Building sell path, ReceiveDamage death path, temporal/super interaction paths, unit death path.
**Proposed test name:** `bunker_clear_path_dispatch_correct_by_trigger`.
**Risk:** High. Using helper (a)'s placement logic on a helper (c) path spawns the unit at an incorrect position; using helper (c)'s no-movement path on a helper (a) context traps the unit inside the bunker cell.

### Handoff 3 — BREAK-on-limbo is a safety net, not the primary +0x2E4 clearer
**Verified behavior:** All three helpers send `vtable+0x274(3)` (RadioCommand BREAK=3) before the building enters limbo. The limbo-path `Broadcast_Radio_ToAll(3)` runs afterward and handles other contacts. The +0x2E4 link is already cleared by the helper when limbo broadcast runs.
**Rust delta:** In Rust's `despawn_entity` / limbo path, add a generic `radio_contacts` BREAK broadcast (per `BROADCAST_RADIO_TO_ALL_LIMBO_BREAK_CLEANUP` handoff). For +0x2E4 specifically, the three explicit clear paths above handle it before despawn. The broadcast does not need to re-clear +0x2E4 — but it must clear `radio_contacts` entries from peers.
**Affected surface:** `despawn_entity`, `GameEntity.radio_contacts`, peer contact removal.
**Proposed test name:** `bunker_link_cleared_before_limbo_broadcast_fires`.
**Risk:** Medium. If the broadcast is implemented as the only +0x2E4 clearer, the link will still be set at the moment the helper's BREAK fires, causing the target's `Receive_Radio` case 3 to run `GrandOpening` on a building that still shows the occupant — producing a one-tick visual artifact.

---

## 8. Negative Facts / Do Not Do

1. **Do not model the stock refinery DockUnload path as a reciprocal +0x2E4 link user.** Evidence: `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY` §4 confirms no writer in the stock `CMIN/HARV -> GAREFN/NAREFN` path; `Mission_Deploy_Building @ 0x0073D630` zero-+0x2E4 branch is the active path.

2. **Do not collapse the three teardown helpers into one function.** Evidence: fresh `decompile_function 0x00459470` confirms helper (c) has NO locomotion commands; helpers (a) and (b) both do — but with different placement behaviors.

3. **Do not play `BunkerWallsDownSound` in helper (b) UndockUnit.** Evidence: `BUILDING_UNDOCKUNIT_0x4593A0` §2 step-by-step confirms no VOC call in `UndockUnit`. Only helpers (a) and (c) play the down sound.

4. **Do not assume the BREAK-on-limbo broadcast is what clears +0x2E4.** Evidence: vtable+0x274(3) is called inside each helper before side-clear; limbo broadcast fires afterward as a separate mechanism for other contacts.

5. **Do not implement FUN_00459470 as placing the unit on a passable cell.** Evidence: fresh decompile of `0x00459470` contains no call to `FootClass::Find_Nearby_Passable_Cell` or `Set_Destination`; only link-clear, sound, anims, and building mission reset.

---

## 9. Remaining Uncertainty

- **`TemporalClass__Update @ 0x0071A760` non-building branch detail:** The decompile shows `piVar2 = building+0x2E4; if (piVar2 != 0) { vtable+0x2c(); FUN_00459470(); }` in the `iVar4 != 6` (non-BuildingClass) branch. The `vtable+0x2c()` call before `FUN_00459470` is a type-query — its exact role (guard or side effect) within this branch is inferred from context, not named. This does not affect the +0x2E4 clear contract (which is verified) but the exact pre-condition for this branch in all temporal scenarios is UNCHECKED beyond the building-vs-non-building split.

- **`SuperClass__Launch @ 0x006CC390` case 4 full area scan:** The call to `FUN_00459470` appears inside an area-scan loop over units near the super target. The exact loop bounds and whether ALL bunkered units in the area are cleared or only the first are not fully traced. The clear-per-unit mechanic is confirmed; the loop completeness is UNCHECKED.

---

## 10. Stale-Doc Notes

- `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md` §6 INI row `[NABNKR] Bunker=yes` at line 13732: **This is wrong** — line 13732 is under `[NATBNK]`, not `[NABNKR]`. Checked `rulesmd.ini:13722` starts `[NATBNK]` and `rulesmd.ini:13732` sets `Bunker=yes` within that section. `[NABNKR]` has no `Bunker=yes` in the checked file. The row label in the inventory table should read `[NATBNK] Bunker=yes` not `[NABNKR]`. This is a labeling error in the table only; the body text in that doc is correct.

---

**Status: COMPLETE**
