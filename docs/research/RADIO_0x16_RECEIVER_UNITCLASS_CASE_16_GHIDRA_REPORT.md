# UnitClass::Receive_Radio — Case 0x16 (TIMING_SYNC Receiver) — Ghidra Report

**Date:** 2026-05-19
**Binary:** gamemd.exe
**Function:** `UnitClass__Receive_Radio` @ `0x00737430`, body ends `0x00737B51`
**Scope:** Case 0x16 receiver branch ONLY (harvester side). Sender side is out of scope.
**Active in YR:** YES — fires on every standard harvester→refinery dock, after ENTER_DOCK(0x18).
**Confidence:** HIGH on all core findings (verified by live disassembly and decompilation).

---

## 1. Five Load-Bearing Verified Facts

| # | Fact | Verification |
|---|------|-------------|
| 1 | Case 0x16 entry in `UnitClass__Receive_Radio` is at `0x007376AD` (switch dispatch landing) | `disassemble_function 0x00737430`, jump table at `0x00737455` |
| 2 | The value written to the facing timer is `0x4000`; the field written is `FootClass+0x388` (PrimaryFacing RateTimer), NOT a facing angle register | `disassemble_function 0x00737430` @ `007376d4` + `007376e6`; cross-confirmed by `TELEPORT_LOCOMOTION_DEEP_DIVE.md` §7 |
| 3 | Locomotor is NOT stopped in case 0x16; `ILocomotion::Is_Moving` is only checked AFTER the timer-set to gate the optional cascade — no stop/kill call exists in this path | `disassemble_function 0x00737430` case 0x16 body — no `Stop_Moving` call present |
| 4 | All exits from case 0x16 return `EAX = 1` (ROGER); the building checks this and proceeds — it does NOT block waiting for alignment | `disassemble_function 0x00737430` @ `0x0073770f`, `0x00737783` |
| 5 | The case 0x16 code path is fully live in YR; no TS-only gate surrounds it (no `SpecialFlags` check, no disabled-by-default INI key) | Caller context from `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md` §7 + direct function body inspection |

---

## 2. Complete Case 0x16 Logic (from Assembly + Decompile)

### 2.1 Inheritance Chain for Case 0x16

`UnitClass::Receive_Radio` (`0x00737430`) has an explicit `case 0x16`. It does NOT fall
through to `FootClass::Receive_Radio` — instead, it calls `FootClass::Receive_Radio` as
a sub-call first, then continues its own logic.

`FootClass::Receive_Radio` (`0x004D8FB0`) has **no case 0x16**. It falls through to
`TechnoClass::Receive_Radio` (`0x006F4AB0`) which has:

```
case 7:
case 9:
case 0x16:
    Transmit_Radio(0x18, sender);   // harvester sends ENTER_DOCK/TOGGLE_DOCK back to building
    RadioClass::Receive_Radio(...)
    return 1;
```

So when `UnitClass` calls `FootClass::Receive_Radio(0x16)`, the harvester sends
`TOGGLE_DOCK(0x18)` back to the building as a side effect of that sub-call.
After the sub-call returns, `UnitClass` continues its own facing-timer logic.

Verified: `UnitClass__Receive_Radio` @ `007376ba: CALL 0x004d8fb0` (FootClass sub-call),
then `007376bf: MOV AL, [ESI + 0x6af]` (the UnitClass-specific guard check).

---

### 2.2 Exact Assembly of the Case 0x16 UnitClass Body

Key instructions (all verified via `disassemble_function 0x00737430`):

```
007376ba: CALL 0x004d8fb0         ; FootClass::Receive_Radio(0x16) — side-effect: sends 0x18 to building

; Guard 1: check chrono-teleporting flag
007376bf: MOV AL, [ESI + 0x6AF]  ; UnitClass+0x6AF = chrono-teleporting flag
007376c7: JNZ 0x0073771b         ; if teleporting, skip timer-set and go to cascade-check

; Read current PrimaryFacing timer value
007376c9: LEA ECX, [ESP + 0x18]  ; out-buffer (stack)
007376ce: LEA ECX, [ESI + 0x388] ; this = UnitClass+0x388 = PrimaryFacing RateTimer
007376d4: CALL 0x004c93d0        ; RateTimer::Current(this, out) — reads current facing-timer value
007376d9: CMP word ptr [EAX], 0x4000  ; is current value already 0x4000?
007376de: JZ 0x0073771b          ; if already 0x4000, skip (go to cascade-check)

; Set PrimaryFacing timer to 0x4000
007376e0: MOV EAX, [ESI + 0x674] ; locomotor ILocomotion ptr (UnitClass+0x674)
007376e6: MOV word ptr [ESP + 0x38], 0x4000  ; value to set = 0x4000
007376fb: MOV ESI, [ESI + 0x674] ; ESI = locomotor ILocomotion ptr
00737705: PUSH EAX               ; push 0x4000
00737706: PUSH ESI               ; push ILocomotion ptr
00737707: MOV EDX, [ESI]         ; EDX = ILocomotion vtable
00737709: CALL [EDX + 0x4C]      ; ILocomotion vtable slot +0x4C = DriveLocomotionClass__Do_Turn
0073770f: MOV EAX, 1
          RET                    ; return 1 (ROGER) — timer-set path exits here

; Cascade-check (reached if already at 0x4000 OR if chrono-teleporting)
0073771b: MOV EAX, [ESI + 0x674]
00737738: CALL [ECX + 0x10]      ; ILocomotion vtable +0x10 = Is_Moving()
0073773d: JNZ 0x00737780         ; if still moving → skip cascade → return 1

; Cascade: if not moving AND has destination AND destination is a building on mission 7
0073777a: CALL [EDX + 0x278]     ; Transmit_Radio(0x15, destination-building)
00737783: MOV EAX, 1
          RET                    ; return 1
```

---

## 3. What "Set Facing to 0x4000" Actually Means

### 3.1 Field Written: FootClass+0x388

`UnitClass+0x388` = `FootClass+0x388` = **PrimaryFacing RateTimer** — the rate-interpolated
body-facing timer. This is **not** a raw facing register (not a compass-direction byte).
It is a `RateTimer` (interpolation timer) whose 16-bit current value encodes the facing
angle in YR's 0x0000–0xFFFF heading convention.

Evidence:
- Assembly @ `007376ce: LEA ECX, [ESI + 0x388]` then `CALL RateTimer::Current`
- `TELEPORT_LOCOMOTION_DEEP_DIVE.md` §7: "FootClass+0x388 is a heading/facing timer — this
  function updates the unit's facing direction…"
- `TECHNOCLASS_EXPANDED_STRUCT_LAYOUT.md`: `TechnoClass+0x388 = TurretFacing`; for
  FootClass/UnitClass the same offset is the PrimaryFacing rate control field.

### 3.2 The Vtable Slot: ILocomotion +0x4C = `DriveLocomotionClass__Do_Turn`

The call `[EDX + 0x4C]` on the ILocomotion vtable dispatches to `DriveLocomotionClass__Do_Turn`
at `0x004B0EF0`. Despite the label, this function does NOT perform a turn — it calls
`RateTimer::Set(FootClass+0x388, 0x4000)`.

Disassembly of `DriveLocomotionClass__Do_Turn` (`0x004B0EF0`):
```
004b0ef0: MOV EDX, [ESP+0x4]      ; EDX = ILocomotion ptr (= locomotor+4)
004b0ef8: LEA ECX, [ESP + 0x8]    ; ECX = &(0x4000) on stack
004b0f01: MOV ECX, [EDX + 0x8]    ; ECX = *(ILocomotion+8) = *(locomotor+0xC) = LinkedTo (FootClass*)
004b0f04: ADD ECX, 0x388           ; ECX = FootClass + 0x388 (PrimaryFacing RateTimer this)
004b0f0a: CALL 0x004c9220          ; RateTimer::Set(this=FootClass+0x388, new_value=&0x4000)
```

Verified via `disassemble_function 0x004b0ef0`.

### 3.3 What `RateTimer::Set(0x4000)` Does

`RateTimer::Set` (`0x004C9220`, verified via `search_functions RateTimer__Set`) updates the
timer's **target value** to `0x4000` and begins interpolating toward it. `0x4000` in YR's
16-bit heading convention = **90° / East** — a fixed dock-alignment heading, not tied to
any runtime direction.

If the current value already equals the target, `RateTimer::Set` returns 0 (no-op).
The guard at `007376d9: CMP word ptr [EAX], 0x4000` avoids the vtable call on a no-op.

---

## 4. Is the Locomotor Stopped in Case 0x16?

**No.** There is no `Stop_Moving` call, no `Set_Speed(0)`, no `Power_Off`, no locomotor
mission abort in the entire case 0x16 body of `UnitClass::Receive_Radio`.

The only locomotor interaction is:
1. **Read** the ILocomotion vtable slot +0x4C (to set the facing timer).
2. **Read** the ILocomotion vtable slot +0x10 (`Is_Moving`) in the cascade-check path.

Movement is not terminated here. The harvester continues under its current mission. If
the building sent a prior `MOVE_TO_CELL(0x12)` that already stopped the unit at the queue
cell, it would already be stationary — but case 0x16 itself does not enforce that.

---

## 5. Return Value Semantics

Case 0x16 always returns `1` (ROGER) — verified at `0x0073770f` and `0x00737783`.

In the building's `CAN_DOCK(0x0E)` handler (`BuildingClass::Receive_Radio @ 0x0043C2D0`):
```c
iVar10 = Transmit_Radio(0x16, harvester);   // TIMING_SYNC
if (iVar10 == 1) return 1;                  // building continues
PlaySound(...)
return 1;
```
(Source: `RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md` §7)

The building does **not block** on case 0x16. It checks the ROGER reply and proceeds
immediately. The TIMING_SYNC/facing-set is a one-shot setup, not a synchronization barrier.

---

## 6. Optional Cascade: PREPARE(0x15) Back to Building

After the timer-set branch (or if already at 0x4000 / if chrono-teleporting), the code
checks whether to send `PREPARE(0x15)` back to the building:

**Conditions (all must hold):**
1. `ILocomotion::Is_Moving()` → false (locomotor idle)
2. `FootClass::GetDestination()` returns non-null (`param_1[0x106] != 0` in Ghidra pseudocode,
   confirmed as the has-destination flag from assembly @ `0073774a: MOV AL, [ESI + 0x418]`)
3. Destination exists and is a building (vtable+0x2C returns 6 = "building type")
4. Building mission == 7 (dock mission)
5. Harvester mission == 7 (harvester's own mission is dock)

If all conditions hold: `Transmit_Radio(0x15, building)`.

The building's `Receive_Radio case 0x15` then sets its dock-animation slot and advances the
dock-handshake state machine.

If ANY condition is false: cascade is silently skipped; case 0x16 still returns 1 (ROGER).

---

## 7. FACE_DOCK vs TIMING_SYNC — Name Resolution

The existing docs use two names for 0x16:
- `HARVESTER_DOCK_UNLOAD.md` calls it `FACE_DOCK` with description "Stop, set facing to 0x4000"
- `REFINERY_RADIO_DOCKING_ACCEPTANCE_QUEUE_GHIDRA_REPORT.md` calls it `TIMING_SYNC`

**Verdict:** Neither name is precisely correct.

- **"FACE_DOCK" is partially correct:** the effect IS setting a facing target of 0x4000 on the
  PrimaryFacing timer. However, the unit is NOT stopped (the "Stop" in HARVESTER_DOCK_UNLOAD.md
  is wrong for this case).
- **"TIMING_SYNC" captures the cascade:** the real purpose beyond the facing-set is to gate
  the `PREPARE(0x15)` back to the building — i.e., the building waits for the harvester to
  confirm it is idle and on-mission before playing the dock-arrival animation.
- **Accurate name for the effect:** `FACE_AND_SYNC` — set the arrival facing (0x4000) and,
  once stationary and on-mission, notify the building to proceed to PREPARE state.

The unit is NOT stopped here — stopping happened via the prior `MOVE_TO_CELL(0x12)` handshake.

**Disparity note:** `HARVESTER_DOCK_UNLOAD.md` line 252 incorrectly attributes "Stop" to this
case. The stop happens via MOVE_TO_CELL/mission state, not via case 0x16.

---

## 8. YR-Active Confirmation

Case 0x16 is reached only through the standard `DockUnload=yes` refinery path in
`BuildingClass::Receive_Radio case 0x0E`, which is confirmed YR-active by
`RECEIVE_RADIO_CASE_0x0E_CAN_DOCK_GHIDRA_REPORT.md`.

No TS-only flag gates the case 0x16 body. No `SpecialFlags` check. No `Fog=` or other
disabled-by-default YR setting guards this path.

**Active in YR: CONFIRMED.**

---

## 9. Inheritance Chain Summary

```
UnitClass::Receive_Radio case 0x16 (0x00737430):
  → calls FootClass::Receive_Radio(0x16) (0x004D8FB0):
      → no case 0x16 → falls to TechnoClass::Receive_Radio(0x16) (0x006F4AB0):
          → Transmit_Radio(0x18, building)   ← harvester ACKs with TOGGLE_DOCK
          → RadioClass::Receive_Radio(...)
          → return 1
  → checks UnitClass+0x6AF (chrono-teleporting flag)
  → reads RateTimer::Current at FootClass+0x388 (PrimaryFacing timer)
  → if current != 0x4000: calls ILocomotion vtable+0x4C → RateTimer::Set(FootClass+0x388, 0x4000)
  → if motionless + on-dock-mission: Transmit_Radio(0x15, building)
  → return 1
```

---

## 10. Struct Field Quick-Reference

| Field | Offset | Type | Value | Notes |
|-------|--------|------|-------|-------|
| Chrono-teleporting flag | `UnitClass+0x6AF` | bool | guards entire body | Set during warp animation |
| PrimaryFacing RateTimer | `FootClass+0x388` | RateTimer (16-bit) | target set to 0x4000 | Smooth body-facing interpolation |
| Locomotor ptr | `UnitClass+0x674` | ILocomotion* | used for vtable call | Locomotor ILocomotion COM interface pointer |
| Has-destination flag | `FootClass+0x418` | bool | gate for cascade | Set when harvester has an active destination |
| ILocomotion vtable slot +0x10 | — | fn ptr | `DriveLocomotionClass__ILocomotion_Is_Moving` | Cascade gate check |
| ILocomotion vtable slot +0x4C | — | fn ptr | `DriveLocomotionClass__Do_Turn` @ `0x004B0EF0` | Sets RateTimer at FootClass+0x388 to 0x4000 |

All offsets verified via `disassemble_function 0x00737430` and `disassemble_function 0x004b0ef0`.
