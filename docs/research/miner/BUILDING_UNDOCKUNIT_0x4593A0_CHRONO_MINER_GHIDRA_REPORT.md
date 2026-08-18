# BuildingClass::UndockUnit (0x4593A0) — Ghidra Decompilation Report

**Target:** BuildingClass::UndockUnit at 0x4593A0  
**Date:** 2026-05-19  
**Confidence:** HIGH (all facts verified from live decompilation in this session)

---

> **Correction 2026-05-21 - stock DockUnload refinement**
>
> The `UndockUnit` body analysis below remains valid for sell/damage/temporal
> interrupt exits when `building+0x2E4` is nonzero. However, later reports show
> that stock `CMIN/HARV -> GAREFN/NAREFN` DockUnload normally does not create the
> reciprocal `unit/building +0x2E4` link, so its dump-complete path does not use
> either `UndockUnit` or `ReleaseDockedHarvester`. The stock zero-link exit is
> `UnitClass::Mission_Deploy_Building` state 4; see
> `CHRONO_MINER_FORCE_TRACK_0X47_EXIT_NAVCOM_STEP_GHIDRA_REPORT.md` and
> `STANDARD_REFINERY_0X2E4_WRITER_INVENTORY_GHIDRA_REPORT.md`.

## 1. Critical Correction: UndockUnit Is NOT the Normal Exit Path

`BuildingClass::UndockUnit` (0x4593A0) fires ONLY on building destruction/interrupt:
- `BuildingClass::ReceiveDamage` — damage result case 4 (building destroyed)
- `BuildingClass::Sell` — if unit is docked when building is sold
- `TemporalClass::Update` — when building is chronosphered/temporaled while unit docked;
  specifically when the target object's type returns vtable+0x2c == 6 (BuildingClass RTTI id)
  and building's `+0xB9 int-slot` (= byte offset +0x2E4) is non-zero

**Superseded 2026-05-21:** the normal stock `CMIN/HARV -> GAREFN/NAREFN`
post-unload exit is the zero-link `UnitClass::Mission_Deploy_Building` state-4
path, not `ReleaseDockedHarvester`. `ReleaseDockedHarvester` is still a real
conditional helper for nonzero reciprocal-link contexts; it shares the same
general ejection vocabulary as `UndockUnit` but is not the standard stock
DockUnload completion path.

---

## 2. Verified Decompilation Summary

**Signature:** `void __fastcall BuildingClass__UndockUnit(int *param_1)` — `param_1` is `int*`
(BuildingClass pointer typed as int array; all field accesses multiply index by 4).

**Step-by-step (verified from decompile output):**

1. `piVar1 = (int*)param_1[0xB9]` → read docked-unit pointer from `building+0x2E4` (= 0xB9×4).
2. If null → return immediately.
3. Check `(*piVar1->vtable + 0x2C)()` — locomotion type query. Guard: proceed only if returns `1`
   (= DriveLocomotionClass RTTI/type identifier). Verified: same check appears in ReleaseDockedHarvester.
4. Check `piVar1[0x19D]` (= unit byte offset `0x674` = active locomotor ILocomotion*) is non-null;
   assert if zero.
5. `(*loco_vtable + 0x58)(loco)` — `ILocomotion::Stop()`.
6. Get building coords: `(*building_vtable + 0x48)(building, &stack_coord)` → `(iVar4, iVar2, iVar3)`.
7. `(*loco_vtable + 0x70)(loco, 0x47, iVar4 - 0x80, iVar2 + 0x80, iVar3)` — `ILocomotion::Head_To()`
   with facing/track-index 0x47, X offset −0x80 leptons, Y offset +0x80 leptons.
8. `(*unit_vtable + 0x544)(0, 0x3FF00000)` — speed setter, argument is IEEE 754 double 1.0
   passed as (lo=0, hi=0x3FF00000).
9. `piVar1[0xB9] = 0` — clear unit's dock-link field at unit+0x2E4.
10. `param_1[0xB9] = 0` — clear building's dock-link field at building+0x2E4.
11. `(*building_vtable + 0x274)(3)` — notify production system (RadioCommand CLEAR=3).

---

## 3. Specific Questions Resolved

### (a) Exact decompilation — CONFIRMED 0x4593A0
Function body spans 0x4593A0–0x004594xx. Signature and every field access verified above.
Ghidra label: `BuildingClass__UndockUnit`.

### (b) (-0x80, +0x80) offsets — lepton offsets, hardcoded literal
The building's `GetCoords()` returns leptons (cells × 0x100 + subcell 0x80 for center).
The offsets −0x80 and +0x80 are **hardcoded integer literals** in the binary instruction stream
(verified: bytes `81 EB 80 00 00 00` = `SUB EBX, 0x80` and `81 C5 80 00 00 00` = `ADD EBP, 0x80`).
They are NOT read from a BuildingTypeClass field.
Interpretation: −0x80 leptons in X = −0.5 cells west; +0x80 leptons in Y = +0.5 cells south.
For a GAREFN (4×3) refinery the pad center is at the east side; this offset places the exit point
at the bib row, south-east of the dock pad center.
`ReleaseDockedHarvester` uses identical offsets (verified in that function's decompile).

### (c) Speed = 1.0 — maximum speed multiplier
`(*unit_vtable + 0x544)(0, 0x3FF00000)` passes a 64-bit IEEE 754 double 1.0 (hex `3FF0000000000000`
split as lo=0, hi=0x3FF00000). This is a **speed multiplier**: 1.0 = full speed. It restores the
unit to its type-default speed after the dock imposed a speed override. Units: dimensionless fraction
(0.0–1.0 of the unit's MaxSpeed). Not leptons-per-tick directly.

### (d) Exit facing 0x47 — hardcoded, both functions
The literal `0x47` (decimal 71) appears as a hardcoded push in the binary
(`6A 47` confirmed in raw bytes at the Head_To call site, also in ReleaseDockedHarvester).
It is NOT read from a BuildingTypeClass field.
This is a **drive track index** (used by DriveLocomotionClass::Apply_Track_Delta via
`g_DriveTrackData_Array`), not a raw DirectionClass facing value, though it visually corresponds
to ESE. The CHRONO_MINER_SYSTEM_OVERVIEW §4 description of "exit facing 0x47 (ESE)" is confirmed
but the "facing" terminology is loose — it is a track index fed to `Head_To`, not a TechnoClass
facing field write.
TechnoClass facing write: no direct `unit->Facing = 0x47` assignment exists in UndockUnit.
The facing updates as the loco follows the track.

### (e) +0xB9 int-slot field (= byte offset +0x2E4)
Both `param_1[0xB9]` (building) and `piVar1[0xB9]` (unit) are **int-pointer indexing**,
so byte offset = 0xB9 × 4 = **0x2E4**.
Ghidra names this `field_0x2E4` on BuildingClass. In unit context the same offset holds the
docked-building back-pointer.
Semantics: this is the **mutual dock-link pointer** — building stores unit pointer, unit stores
building pointer, same offset on both classes. It is not named `IsDocked` (that is a separate
bool) nor `DockSlot` (that is a separate index). It is the paired object cross-reference for
the active dock relationship.
`ReleaseDockedHarvester` confirms: clears `piVar1[0xb9] = 0` (unit side) and
`param_1->field_0x2e4 = 0` (building side) — identical teardown.

### (f) Caller of UndockUnit — NOT FUN_006AF6C0
MINER_DOCK_GAPS referenced FUN_006AF6C0 as a DockManager caller. **This is incorrect.**
Ghidra identifies 0x006AF6C0 as `SlaveManagerClass__AI_Update`, which does not call UndockUnit.
Verified callers (3 total):
- `BuildingClass__ReceiveDamage` (0x00442230) — case 4 (building destroyed), fires if `field_0x2E4 != 0`
- `BuildingClass__Sell` (0x00449C30) — on sell if unit docked
- `TemporalClass__Update` (0x0071A760) — when building is chrono-wiped and target is BuildingClass
  type (vtable+0x2c == 6) and `building[0xB9] != 0`. FUN_00459470 handles the non-BuildingClass
  temporal branch (clears dock link without locomotion commands).
**Normal dock completion does NOT call UndockUnit at all.**

### (g) Chrono-miner-specific branches — NONE in UndockUnit
UndockUnit contains **no check for Teleporter=yes, +0xCD4, or any unit-type field**.
The only guard is the locomotion type check (`vtable+0x2C == 1`), which is true for
DriveLocomotionClass (the piggybacked loco active during dock). For the chrono miner:
- During dock: TeleportLoco is piggybacked under DriveLoco (DriveLoco is active, returns type 1)
- UndockUnit operates on DriveLoco identically to a regular harvester
- The TeleportLoco swap-back is handled by `FootClass::AI` (0x4DA530) after the miner moves away,
  per MINER_DOCK_GAPS_RESEARCH.md §Gap 1 (verified separately, not re-investigated here)

---

## 4. Corrections to Prior Documentation

| Prior claim (MINER_DOCK_GAPS) | Correction |
|-------------------------------|------------|
| "UndockUnit called from FUN_006AF6C0 (DockManager)" | FUN_006AF6C0 = SlaveManagerClass__AI_Update; UndockUnit has 3 callers, none is a DockManager |
| "Gets docked unit from this+0x2E4" | Correct, but stated as `this[0xB9]` in int* indexing — same field |
| "Clears `unit[0xB9] = 0` and `building[0xB9] = 0`" | Correct; byte offsets both = 0x2E4 |
| UndockUnit = normal exit handler | Incorrect. Superseded 2026-05-21: stock zero-link DockUnload exits through `Mission_Deploy_Building` state 4; `ReleaseDockedHarvester` is conditional nonzero-link release. |

---

## 5. Five Load-Bearing Verified Facts

1. **UndockUnit is an interrupt/destroy handler only** (3 verified callers: ReceiveDamage case-4,
   Sell, TemporalClass::Update). Normal exit = `ReleaseDockedHarvester` at 0x4595C0.

2. **`(-0x80, +0x80)` are hardcoded lepton deltas** applied to building center coords
   (bytes `81 EB 80 00 00 00` / `81 C5 80 00 00 00` confirmed in raw memory read).

3. **Speed argument `(0, 0x3FF00000)` = IEEE 754 double 1.0** — a speed multiplier restoring
   full unit speed, not a lepton-per-tick rate.

4. **0x47 is a hardcoded drive track index** (`6A 47` push literal), not a facing field write.
   Applies identically to chrono miner and regular harvester — no unit-type branch in UndockUnit.

5. **`[0xB9]` int-index = byte offset 0x2E4** on both building and unit; this is the mutual
   dock-link cross-reference pointer, cleared on both sides during teardown.

---

**Status: COMPLETE**
