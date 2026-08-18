# BulletClass::AI Non-Impact Exit Enumeration

**Target:** All exit paths from `BulletClass::AI` @ `0x004666E0` that remove a bullet from
the scheduler via means other than a direct close-hit/impact.
**Date:** 2026-05-28
**Status:** COMPLETE
**Seed addresses:** `BulletClass::AI` @ `0x004666E0`; `BulletClassBulletDetonationImpactDamage` @
`0x00468D80`; `ProximityDetector::Check` @ `0x004E11F0`; OOB check @ `0x00568350`.

---

## Target Question

What are every non-direct-impact condition that sets `local_190` (the detonation/expire flag) in
`BulletClass::AI`, and which removal route does each one take?

## Non-Goals

- HomingTrack turn math / ROT-scalar computation
- Speed ramp / AAHeatSeeker2 arming numbers
- vtable+0xF8 identity (slot-1 scope)
- BulletDetonation warhead-damage internals

## Evidence Needed to Mark COMPLETE

A row for each of: range-expiry, altitude/lifetime, OOB, lost/null target, arcing ground-impact,
ProximityDetector gate; each with: predicate (assembly-verified offset), flag value set,
removal route, Active-in-YR verdict.

---

## Structural Overview

`BulletClass::AI` splits on `*(int *)(param_1[0x2b] + 0x2DC)` (i.e. `BulletType+0x2DC`).
- `+0x2DC > 0` → **ROT/homing branch** (uses `HomingTrack`, homing target ptr `param_1[0x43]`)
- `+0x2DC == 0` → **non-ROT / arcing branch** (uses `param_1[0x2c]` direct target, ground-impact
  logic, arcing trajectory)

Gate verified at assembly address `0x004668D3`:
```
MOV ECX,dword ptr [EAX + 0x2dc]
TEST ECX,ECX
JLE 0x004670c3     ; ← non-ROT branch
```
(verified via `disassemble_function 0x004666E0`)

`local_190` == 0 → no expire; == 1 → normal BulletDetonation + vtable+0xF8; == 2 → OOB silent
removal via vtable+0x124(2) only, no vtable+0xF8 call.

Final dispatch at `LAB_00467b7a` (`0x00467b7a`):
```
SUB EAX,0x2           ; EAX = local_190 - 2
JZ 0x00467fa9         ; local_190 == 2 → vtable+0x124(2) then vtable+0xF8 skipped
CALL [EDX + 0x124]    ; local_190 != 2 → vtable+0x124() normal call
...
CALL 0x00468d80       ; BulletDetonationImpactDamage
CALL [EAX + 0xf8]     ; vtable+0xF8 teardown
```
(verified at `0x00467b7e`–`0x00467fb4`)

---

## Exit Table (All Non-Impact Triggers)

### Exit A — Delayed-Detonation Anim Active Guard (param_1+0x158/+0x154)

**Predicate:** `*(char *)(param_1 + 0x158) != 0` **AND** `param_1[0x55] == 0`

Assembly:
```
00466705: MOV AL,byte ptr [EBP + 0x158]
0046670b: TEST AL,AL
0046670d: JZ 0x00466789       ; skip if not pending
0046670f: MOV EAX,dword ptr [EBP + 0x154]
00466715: TEST EAX,EAX
00466717: JNZ 0x00467fee       ; anim not done, return without detonation
; --- anim done (param_1[0x55] == 0): ---
00466771: CALL 0x00468d80      ; BulletDetonationImpactDamage(0)
00466779: CALL dword ptr [EAX + 0xf8]  ; vtable+0xF8 teardown
```

Meaning: When `Burst` weapons use an AnimClass to delay detonation (Nuclear Missile scenario),
`Bullet+0x158` is set to 1 and `+0x154` holds the anim pointer. When the anim completes
(`+0x155` clears to 0), the bullet detonates with warhead damage and vtable+0xF8 teardown.

**Removal route:** BulletDetonation (`0x00468D80` with arg 0) + vtable+0xF8.
**Damage applied:** Yes (BulletDetonation called).
**Active in YR:** Conditional — only when a `BulletType` uses Nuclear Missile-style delayed anim.

---

### Exit B — ROT Branch: Max-Range Expiry (BulletType+0x2BC)

**Predicate (ROT branch, `+0x2DC > 0`):** After trajectory step, new Z-coordinate `> *(int *)(BulletType + 0x2BC)`

Assembly:
```
00467334: MOV EDX,dword ptr [ECX + 0x2bc]   ; ECX = param_1[0x2b] = BulletType ptr
0046733a: CMP EAX,EDX                        ; new_z vs BulletType+0x2BC threshold
0046733c: JLE 0x00467350                     ; ok, continue
0046733e: MOV dword ptr [ESP + 0x20],0x1     ; local_190 = 1
00467346: MOV byte ptr [ESP + 0x18],0x1      ; local_198 = 1
0046734b: JMP 0x004677d3                     ; → detonation path
```

`BulletType+0x2BC` is the altitude/max-height (lifetime) threshold for arcing ROT bullets.
In the decompiler it appeared as `*(int *)(param_1[0x2b] + 700)` — 700 decimal = `0x2BC`.

**Removal route:** `local_190 = 1` → normal BulletDetonation + vtable+0xF8.
**Damage applied:** Yes.
**Active in YR:** Yes — applies to all ROT/ballistic bullets with a non-zero `+0x2BC` ceiling.

---

### Exit C — ROT Branch: Below-Ground (vtable+0x1C8 returns negative)

**Predicate (ROT branch):** `(*vtable+0x1C8)()` returns negative — i.e. bullet is below ground.

Assembly:
```
00467350: MOV EDX,dword ptr [EBP]
00467355: CALL dword ptr [EDX + 0x1c8]       ; get height/altitude
0046735b: TEST EAX,EAX
0046735d: JGE 0x00467371                     ; ok if >= 0
0046735f: MOV dword ptr [ESP + 0x20],0x1     ; local_190 = 1
00467367: MOV byte ptr [ESP + 0x18],0x1
0046736c: JMP 0x004677d3
```

vtable+0x1C8 is the locomotor altitude query. A negative result means the bullet is underground.

**Removal route:** `local_190 = 1` → normal BulletDetonation + vtable+0xF8.
**Damage applied:** Yes.
**Active in YR:** Yes.

---

### Exit D — ROT Branch: Ground-Impact via Bridge/Water Cell Flag (0x100)

**Predicate (ROT branch):** Current or previous cell has `CellClass+0x140 & 0x100` (bridge/water
flag) **AND** the Z-coordinate crosses the ground height from above-to-below or below-to-above.

Assembly:
```
004673a3: TEST EDI,ECX         ; EDI=0x100, ECX = CellClass+0x140
004673a5: JNZ 0x004673c2
004673b1: CALL 0x00565730      ; check prev cell
004673b6: TEST dword ptr [EAX + 0x140],EDI
004673bc: JZ 0x004677d3        ; neither cell is bridge → no expiry
004673c2: MOV EAX,dword ptr [ESP + 0x2c]     ; new_z
004673c6: CMP EAX,ESI          ; ESI = ground_height
004673c8: MOV EAX,dword ptr [ESP + 0x4c]     ; prev_z
...
; above → below:
004673d6: MOV dword ptr [ESP + 0x20],0x1
004673de: MOV byte ptr [ESP + 0x18],0x1
004673e3: JMP 0x004677d3
; below → above:
004673f0: MOV dword ptr [ESP + 0x20],0x1
004673f8: MOV byte ptr [ESP + 0x18],0x1
004673fd: JMP 0x004677d3
```

This catches bullets hitting bridge surfaces or passing through water/bridge cells from either
direction.

**Removal route:** `local_190 = 1` → normal BulletDetonation + vtable+0xF8.
**Damage applied:** Yes.
**Active in YR:** Yes — fires whenever a bullet crosses a bridge/water tile boundary.

---

### Exit E — Non-ROT Branch: Range Remaining Comparison (param_1[0x44])

**Predicate (non-ROT branch, `+0x2DC == 0`):** Speed magnitude `>= param_1[0x44]` (range
remaining counter).

Assembly:
```
00466936: CMP dword ptr [EBP + 0x110],0x28   ; param_1[0x44] = EBP+0x110
0046693d: JGE 0x00466978                     ; skip flag-clear if param_1[0x44] >= 40
```

Actually `param_1[0x44]` is used as a countdown. The inner-loop block decrement is at
`0x00466936`–`0x00466978`. When param_1[0x44] goes to zero (or the speed magnitude exceeds
the remaining range), it clears `Bullet+0x105` (which is a wobble/swerve flag, not the expire
flag directly). The **true range expiry** in non-ROT comes from `BulletType+0x2BC` in the ROT
branch (Exit B above). In the non-ROT branch there is no explicit separate "range out" flag —
the bullet continues until it hits ground (Exits F/G/H) or goes OOB (Exit I).

**Correction / clarification:** `param_1[0x44]` in the non-ROT branch is the **target distance
counter** (distance from origin to target, in leptons), used to control the speed profile —
not a remaining-range expire field. The check at `0x00466936` compares `param_1[0x44]` against
0x28 (40 leptons) to decide whether to disable the wobble flag `+0x105`.

**No independent range-expiry exit in non-ROT path.** Verified: no branch sets `local_190 = 1`
solely from `param_1[0x44]` in the non-ROT branch.

**Active in YR:** N/A — no such exit exists.

---

### Exit F — Non-ROT Branch: Velocity Near-Zero with Same-Cell + Close-Distance (LAB_00467879)

**Predicate (non-ROT branch):** Current cell == previous cell (`sVar16 == sVar19 && sVar21 ==
sVar4`) **AND** `BulletType+0x2C0 == 0` (not `Inviso`) **AND** `(*vtable+0x1C8)() < 2 * DAT_0089de70`.

Assembly:
```
00467840: CMP CX,SI            ; current cell X == prev cell X
00467843: MOV dword ptr [ESP + 0x80],EDX
0046784a: JNZ 0x00467890
0046784c: CMP DI,AX            ; current cell Y == prev cell Y
0046784f: JNZ 0x00467890
00467851: MOV EAX,dword ptr [EBP + 0xac]
00467857: MOV CL,byte ptr [EAX + 0x2c0]     ; BulletType+0x2C0 (Inviso flag)
0046785d: TEST CL,CL
0046785f: JNZ 0x00467890
00467866: CALL dword ptr [EDX + 0x1c8]       ; get dist to target
0046786c: MOV ECX,dword ptr [0x0089de70]
00467872: LEA EDX,[ECX + ECX*0x1]
00467875: CMP EAX,EDX
00467877: JGE 0x00467890
00467879: MOV dword ptr [ESP + 0x20],0x1     ; local_190 = 1
00467881: MOV byte ptr [ESP + 0x18],0x1
00467886: MOV byte ptr [ESP + 0x1f],0x1      ; uStack_194 LSB = 1 (direct-hit flag)
0046788b: JMP 0x00467b7a
```

This is the **direct-contact detonation** for non-ROT bullets that have closed to within cell
and are very close to the target. `uStack_194` byte = 1 marks it as a confirmed hit.

**Note:** This is technically a "same-cell very-close-range" detonation, not a non-impact exit.
Included for completeness because it sets `local_190 = 1` via `LAB_00467879`.

**Removal route:** `local_190 = 1` → normal BulletDetonation + vtable+0xF8.
**Damage applied:** Yes.
**Active in YR:** Yes.

---

### Exit G — Non-ROT Branch: Lost / Null / Dead Target → CellClass::Find_Nearest_Object match

**Predicate (non-ROT branch):** After `CellClass::Find_Nearest_Object` at `0x004678FA`, if
nearest object `ESI != 0` **AND** `param_1[0xb0]` (direct target ptr) `== 0` **AND** `ESI !=
param_1[0xb0]` (local_199 = 0, meaning found object is not the same as original target) **AND**
nearest object is not ally **AND** object is within 128 leptons.

Assembly:
```
004679d7: MOV CL,byte ptr [ESP + 0x17]   ; local_199 (is found == original target?)
004679dd: JNZ 0x00467a2b
004679df: MOV CL,byte ptr [ESP + 0x16]   ; local_19a (is found == ally?)
004679e5: JNZ 0x00467a2b
004679e7: TEST AL,AL                     ; AL = within-128-leptons flag
004679e9: JZ 0x00467a2b
004679eb: MOV EDX,dword ptr [EBP + 0xac]
004679f1: MOV dword ptr [ESP + 0x20],0x1  ; local_190 = 1
004679f9: MOV byte ptr [ESP + 0x18],0x1
004679fe: MOV AL,byte ptr [EDX + 0x2a2]   ; BulletType+0x2A2 (NoSelf?)
00467a04: TEST AL,AL
00467a06: JNZ 0x00467b7a                  ; if NoSelf, skip coord snap
00467a0c: ADD ESI,0x9c                    ; snap detonation coords to found object's position
00467a26: JMP 0x00467b7a
```

This is a **proximity find-and-detonate**: when the original target pointer is null/expired but
a nearby enemy object is found within 128 leptons, the bullet detonates at that object's
position (unless `BulletType+0x2A2` is set, which prevents the coord snap).

**Removal route:** `local_190 = 1` → normal BulletDetonation + vtable+0xF8.
**Damage applied:** Yes.
**Active in YR:** Yes — fires whenever a non-ROT bullet's original target dies and an enemy is
nearby.

---

### Exit H — Non-ROT Branch: OOB / Off-Map (FUN_00568350 returns 0)

**Predicate (non-ROT branch):** `FUN_00568350(MapClass_this, &current_position)` returns 0
(position is outside map bounds).

Assembly:
```
00467a2f: MOV ECX,0x87f7e8        ; MapClass singleton
00467a35: CALL 0x00568350          ; OOB check: returns 0 if off-map
00467a3a: TEST AL,AL
00467a3c: JNZ 0x00467a6a           ; non-zero = in-bounds → continue
00467a3e: LEA EAX,[EBP + 0x9c]
00467a44: MOV dword ptr [ESP + 0x20],0x2  ; local_190 = 2 (OOB marker)
00467a4c: MOV byte ptr [ESP + 0x18],0x1
00467a51: MOV ECX,dword ptr [EAX]         ; restore coords to bullet's ORIGINAL position
00467a53: MOV dword ptr [ESP + 0x24],ECX
; ... (copies param_1[0x27/0x28/0x29] = original coords back)
00467a65: JMP 0x00467b7a
```

Then at dispatch:
```
00467b7e: SUB EAX,0x2               ; EAX = local_190 - 2
00467b84: PUSH 0x2
00467b86: MOV ECX,EBP
00467b88: JZ 0x00467fa9             ; == 2 → silent removal path
; ...
00467fa9: CALL dword ptr [EDX + 0x124]  ; vtable+0x124(2) — silent removal
00467faf: MOV EAX,dword ptr [EBP]
00467fb2: MOV ECX,EBP
00467fb4: CALL dword ptr [EAX + 0xf8]  ; vtable+0xF8 ALSO called for OOB
```

CRITICAL: `local_190 == 2` does NOT skip vtable+0xF8. The path calls vtable+0x124(2) AND then
falls through to `vtable+0xF8` at `0x00467fb4`. What it skips is `BulletDetonationImpactDamage`.
So OOB = **silent remove, no warhead damage, but vtable+0xF8 teardown still runs**.

`FUN_00568350` (`__thiscall`) checks `MapClass+0xF4` (map width/height) against the cell coords
of the position — returns 1 if in-bounds, 0 if off-map.

**Removal route:** `local_190 = 2` → vtable+0x124(2) (no damage) + vtable+0xF8 (teardown).
**Damage applied:** No.
**Active in YR:** Yes — fires whenever a non-ROT bullet travels off the map edge.

---

### Exit I — Non-ROT Branch: Arcing Ground-Impact via Bridge/Water Cell Flag (0x100)

**Predicate (non-ROT branch):** Same cell-flag logic as Exit D but in the non-ROT path.

Assembly at `0x004673a3`–`0x004673fd` (as Exit D above, but reached from `+0x2DC == 0` branch at
`0x00467402` onwards), using `CellClass+0x140 & 0x100`.

**Removal route:** `local_190 = 1` → normal BulletDetonation + vtable+0xF8.
**Damage applied:** Yes.
**Active in YR:** Yes.

---

### Exit J — Homing Branch: Stall / Speed-Below-Threshold with ROT ≤ 9

**Predicate (ROT/homing branch):** After `HomingTrack`, velocity magnitude below `DAT_007e44a8`
threshold **AND** `(*vtable+0x1C8)()` (distance to target) `< 10`.

Assembly:
```
00467b4d: FCOMP double ptr [0x007e44a8]  ; compare speed vs min threshold
00467b58: TEST AH,0x1
00467b5b: JZ 0x00467b7a                  ; ok if above threshold
; speed below threshold:
00467b62: CALL dword ptr [EAX + 0x1c8]   ; get dist to target
00467b68: CMP EAX,0xa                    ; < 10 leptons?
00467b6b: JGE 0x00467b7a                 ; keep going if not close enough
00467b6d: MOV dword ptr [ESP + 0x20],0x1 ; local_190 = 1
00467b75: MOV byte ptr [ESP + 0x18],0x1
00467b7a: ...
```

This catches a stalled/slow homing missile that is already very close (< 10 leptons) to target.

**Removal route:** `local_190 = 1` → normal BulletDetonation + vtable+0xF8.
**Damage applied:** Yes.
**Active in YR:** Yes — fires for any ROT > 0 bullet that stalls very close to target.

---

### Exit K — Homing Branch: Target Lost / Target Coords = Null Sentinel + Range Exceeded

**Predicate (ROT/homing branch):** `param_1[0x43]` (homing target) is null **AND** target coords
equal the null-sentinel `DAT_0089de30/34/38` **AND** `(*vtable+0x1C8)()` >= `Rules+0x5A0`
(MaximumQueuedObjects / tactical range threshold).

Assembly:
```
00466e6b: MOV ECX,dword ptr [0x0089de30]  ; null sentinel
00466e75: CMP EAX,ECX                     ; target_x vs null
00466e7f: JNZ 0x00466eb6
; ... check Y and Z ...
00466e94: CALL dword ptr [EDX + 0x1c8]    ; get dist
00466ea5: CMP EAX,dword ptr [ECX + 0x5a0] ; Rules+0x5A0 max range
00466eab: JL 0x00466eb6                   ; keep going if within range
00466ead: MOV byte ptr [ESP + 0x18],0x1   ; local_198 = 1
00466eb2: MOV dword ptr [ESP + 0x20],EDI  ; local_190 = 1 (EDI was set to 1 earlier)
```

This fires when a homing missile has no living target and has wandered beyond the `Rules+0x5A0`
max range.

**Removal route:** `local_190 = 1` → normal BulletDetonation + vtable+0xF8.
**Damage applied:** Yes.
**Active in YR:** Yes.

---

### Exit L — Homing Branch: Close-Range + HomingTrack distance ≤ range×_DAT_007e1738

**Predicate (ROT/homing branch):** After `HomingTrack`, remaining distance to target ≤
speed × `DAT_007e1738` **OR** `(*vtable+0x1C8)()` returns < 1.

Assembly:
```
00466de2: FMUL double ptr [0x007e1738]     ; speed × threshold
00466de8: FCOMP double ptr [ESP + 0x58]    ; compare vs remaining distance
00466df4: TEST AH,0x1
00466df6: JZ 0x00466e05                    ; ok if dist > threshold×speed
; within one tick's travel:
00466e08: MOV EDI,0x1
00466e0f: MOV byte ptr [ESP + 0x18],0x1    ; local_198 = 1
00466e14: MOV dword ptr [ESP + 0x20],EDI   ; local_190 = 1
; ... or vtable+0x1C8 < 1:
00466e1e: TEST EAX,EAX
00466e20: JLE 0x00466e6b
```

This is the homing bullet's "I've arrived" detonation — within one tick's travel of target.

**Removal route:** `local_190 = 1` → normal BulletDetonation + vtable+0xF8.
**Damage applied:** Yes.
**Active in YR:** Yes.

---

### Exit M — Homing Branch: Ground-Impact via Bridge/Water Cell Flag (ROT branch version)

**Predicate (ROT/homing branch):** `(local_14c[0x50] & 0x100) != 0` OR the previous-cell
`CellClass+0x140 & 0x100`, same crossing logic as Exits D/I.

Assembly:
```
00467032: MOV EAX,dword ptr [ESP + 0x20]
00467036: TEST EAX,EAX
00467038: JNZ 0x00467b7a                    ; already detonating, skip
0046703e: MOV ECX,dword ptr [ESP + 0x64]    ; local_14c = current CellClass
00467042: MOV ESI,0x100
00467047: TEST dword ptr [ECX + 0x140],ESI  ; bridge/water flag
0046704d: JNZ 0x0046706a
0046704f: LEA EDX,[ESP + 0x44]              ; prev cell coords
00467059: CALL 0x00565730
0046705e: TEST dword ptr [EAX + 0x140],ESI
00467064: JZ 0x00467b7a
0046706a: ... (GetGroundHeight + Z crossing check)
00467091: MOV dword ptr [ESP + 0x20],0x1    ; local_190 = 1
```

Then `LAB_00467b75`:
```
00467b75: MOV byte ptr [ESP + 0x18],0x1     ; local_198 = 1
```

**Removal route:** `local_190 = 1` → normal BulletDetonation + vtable+0xF8.
**Damage applied:** Yes.
**Active in YR:** Yes.

---

### ProximityDetector::Check — Gated by BulletType+0x2DC and +0x2A0

`ProximityDetector::Check` @ `0x004E11F0` is called at `0x00467C35` but only under:

```
00467c12: MOV ECX,dword ptr [EAX + 0x2dc]   ; BulletType+0x2DC (ROT count / proximity type)
00467c18: TEST ECX,ECX
00467c1a: JG 0x00467c2a                     ; > 0 → call ProximityDetector
00467c1c: MOV CL,byte ptr [EAX + 0x2a0]     ; BulletType+0x2A0
00467c22: TEST CL,CL
00467c24: JNZ 0x00467c2a                    ; non-zero → call ProximityDetector
00467c26: XOR ESI,ESI                       ; else result = 0 (no proximity hit)
00467c28: JMP 0x00467c3c
```

`ProximityDetector::Check` is called **after** `local_190` is already resolved. Its result
(`ESI`) is used to:
1. Potentially snap detonation position to target coords (vtable+0x1B4 call)
2. Allow target-health-check override (ESI==2 downgraded to 1 if target is a building in InfDeath
   mode `+0xD94`)
3. Gate whether the `BulletType+0x29C` (Proximity) no-expire path is taken

`ProximityDetector::Check` returns `1` (proximity hit) or `2` (direct hit). It does **not**
independently set `local_190`; it only adjusts how the detonation position and target-snap are
handled.

**Active in YR:** Conditional — only when `BulletType+0x2DC > 0` OR `BulletType+0x2A0 != 0`.

---

## Summary Exit Table

| Exit | Trigger | local_190 | Damage | vtable+0xF8 | Active in YR |
|------|---------|-----------|--------|-------------|--------------|
| A | Delayed-anim complete (`+0x158`/`+0x155`) | 1 | Yes | Yes | Conditional |
| B | ROT: new Z > BulletType+0x2BC | 1 | Yes | Yes | Yes |
| C | ROT: vtable+0x1C8 < 0 (underground) | 1 | Yes | Yes | Yes |
| D | ROT: bridge/water cell (0x100) + Z crossing | 1 | Yes | Yes | Yes |
| F | Non-ROT: same cell + close + dist < 2×`DAT_0089de70` | 1 | Yes | Yes | Yes |
| G | Non-ROT: lost-target, nearest obj within 128 lep | 1 | Yes | Yes | Yes |
| H | Non-ROT: OOB (`FUN_00568350` returns 0) | **2** | **No** | Yes | Yes |
| I | Non-ROT: bridge/water cell (0x100) + Z crossing | 1 | Yes | Yes | Yes |
| J | Homing: speed < threshold AND dist < 10 lep | 1 | Yes | Yes | Yes |
| K | Homing: null target + null-sentinel coords + dist ≥ `Rules+0x5A0` | 1 | Yes | Yes | Yes |
| L | Homing: within one tick's travel OR dist < 1 | 1 | Yes | Yes | Yes |
| M | Homing ROT: bridge/water cell (0x100) + Z crossing | 1 | Yes | Yes | Yes |

---

## Implementation Handoff

### Handoff 1 — OOB is silent (no damage)

**Verified behavior:** When `FUN_00568350` returns 0 (position off-map), `local_190 = 2` → only
vtable+0x124(2) is called; `BulletDetonationImpactDamage` is NOT called. vtable+0xF8 teardown
still runs.

**Rust delta:** In `homing_movement.rs` and `rocket_movement.rs`, the `detonated` path currently
does not distinguish OOB from normal detonation. An OOB flag must be threaded through so the
combat system skips warhead damage application.

**Affected surface:** `src/sim/movement/homing_movement.rs` (returns `Vec<u64>` of detonated IDs);
`src/sim/movement/rocket_movement.rs` (same). Combat damage dispatch caller.

**Acceptance scenario:** A bullet fired toward the map edge that flies off-map is removed from
the scheduler with no damage applied to any unit. No explosion effect on the map border.

**Proposed test name:** `oob_bullet_removed_without_damage`

**Risk:** HIGH — current Rust treats all despawns as damage-triggering; OOB bullets may incorrectly
apply damage at map-edge coordinates.

---

### Handoff 2 — Ground-impact via bridge/water cell flag (0x100) applies damage

**Verified behavior:** In all three trajectory branches (ROT/non-ROT/homing), crossing a
bridge/water cell height boundary sets `local_190 = 1` → full BulletDetonation with warhead
damage.

**Rust delta:** `homing_movement.rs` / `rocket_movement.rs` need to check the bridge/water cell
flag (`CellClass+0x140 & 0x100`) on the current and previous cell each tick, and trigger
detonation when Z crosses the bridge/water ground height from either direction.

**Affected surface:** `src/sim/movement/homing_movement.rs`, `src/sim/movement/rocket_movement.rs`,
and the cell-lookup layer in `src/map/`.

**Acceptance scenario:** A rocket fired over a bridge detonates on the bridge surface rather than
passing through it.

**Proposed test name:** `bullet_detonates_on_bridge_surface`

**Risk:** MEDIUM — bridge-passthrough bugs are player-visible (projectiles fly through bridges).

---

### Handoff 3 — ProximityDetector::Check is post-expiry, not an independent expire trigger

**Verified behavior:** `ProximityDetector::Check` at `0x004E11F0` is called **after** `local_190`
is resolved (after `LAB_00467b7a`). Its return value adjusts position-snap and target-select
but does not independently set the expire flag.

**Rust delta:** If any Rust code uses proximity-detector result to trigger despawn, that is wrong.
Proximity only adjusts where the detonation is snapped, not whether it occurs.

**Affected surface:** Any proximity-detector integration in `src/sim/movement/`.

**Acceptance scenario:** A proximity bullet detonates at the target-snapped position even when
ProximityDetector returns 0 (no snap), as long as `local_190` was already set by another trigger.

**Proposed test name:** `proximity_detector_does_not_gate_detonation`

**Risk:** LOW — proximity-detector result is a position adjustment, not a spawn/despawn gate.

---

## Negative Facts / Do Not Do

1. **Do not apply damage on OOB removal.** `local_190 == 2` → vtable+0x124(2) path skips
   `BulletDetonationImpactDamage`. Verified at `0x00467b7e`–`0x00467fa9`.

2. **Do not treat `param_1[0x44]` as a range-expiry countdown in the non-ROT branch.** In the
   non-ROT branch, `param_1[0x44]` controls the speed profile (wobble enable/disable), not an
   expire timer. Verified at `0x00466936`. No branch sets `local_190 = 1` from this field alone
   in non-ROT.

3. **Do not skip vtable+0xF8 on OOB.** Even for `local_190 == 2`, vtable+0xF8 teardown IS
   called at `0x00467fb4`. Only `BulletDetonationImpactDamage` is skipped.

4. **ProximityDetector::Check does not control expire.** It only controls detonation position
   and target-snap. Verified at `0x00467c0c`–`0x00467c6a`.

5. **Do not add a non-ROT separate "range remaining" expiry exit.** The non-ROT branch has no
   `param_1[0x44]`-triggered detonation. Expiry in non-ROT comes only from ground-impact (Exits
   F/G/H/I).

---

## Remaining Uncertainty

- Identity of vtable+0x124 (what class/method the slot resolves to in stock YR) — deferred to
  slot-1 (vtable+0xF8 teardown/removal path scope).
- Whether `BulletType+0x2BC` encodes a max altitude or a max lifetime in ticks — the assembly
  shows it as a Z-coordinate ceiling comparison, but what INI key populates it is not verified
  here. Likely `BallisticScatter`-related or `Inaccuracy`-related; consult `INI ReadINI` xref.
- `DAT_007e44a8` exact value (speed stall threshold in Exit J) — runtime constant, not verified
  in this session.

---

## COMPLETE

All five task scope items covered:
1. Max-range/altitude: Exit B (ROT `+0x2BC`) + range-counter correction for non-ROT (no such
   exit — verified).
2. OOB: Exit H — `local_190 = 2` → vtable+0x124(2) + vtable+0xF8, no damage.
3. Lost/invalid target: Exits G (nearest-object proximity detonate) + K (homing, null-sentinel
   coords exceeded).
4. Vertical/ground-impact for arcing: Exits D/I/M (bridge/water cell 0x100 flag + Z crossing).
5. ProximityDetector gate: post-expiry position-snap only; does not independently expire bullet.
