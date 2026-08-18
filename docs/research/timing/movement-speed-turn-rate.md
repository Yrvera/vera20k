# Movement Speed / Turn Rate / Acceleration

## Overview

**Player-visible effect:** every unit on the map moves at its own pace.
A Conscript walks slowly across a tile while a Harrier zips across the
screen. When a tank turns to face a new direction, it pivots smoothly;
when an aircraft turns, it banks gradually instead. Hitting a slope
slows units down; downhill gives a small boost. Country-specific
speed bonuses (Allied Cavalier vs. Soviet Rhino) make the same unit
class feel different across factions. Veterancy gives a small SPEED
multiplier when a unit has earned the veterancy SPEED ability.

**Mechanism in plain terms:** every TechnoType has a per-type `Speed=`
field (INI integer, typically 1–14 in `rulesmd.ini`). This value
becomes the "max distance the unit can move in one game tick"
(measured in leptons, where 1 cell = 256 leptons). The unit's
locomotor (Drive, Walk, Hover, Fly, Ship, Jumpjet, Rocket, Teleport,
Mech, DropPod) consumes that budget per tick to advance along its
current track/path. Acceleration ramps current_speed up to the target
over multiple ticks; deceleration is always 1.5× faster than
acceleration. Turning is handled separately via `TurnRate=`/`ROT=` (a
per-frame angular delta) that's also locomotor-specific.

**Three multipliers** stack on top of the per-type `Speed=`:

1. **`GameSpeedBias=1.6`** (rules-global) — multiplies *all* unit
   movement speeds. Lives at `RulesClass + 0x1418` as a `double`.
2. **`HouseClass::GetSpeedBonus()`** — country-specific per-unit-class
   multiplier (different for aircraft vs. vehicles vs. infantry).
   Read from `HouseTypeClass` fields at offsets `+0x128/+0x12C/+0x130`.
3. **Veterancy SPEED ability** (`+0x29E` / `+0x2B0` bitmask on
   TechnoType) — extra multiplier from `RulesClass.VeteranSpeed` /
   `EliteSpeed` when promoted.

The cooldown for "next tick of movement" is the **master game-tick
clock** — `g_CurrentFrameCounter` advances by 1 per tick, and each
locomotor's per-tick budget is consumed once per advance. So unit
movement is on the same clock as ROF, animation, and everything else;
it scales with GameSpeed slider.

**Per-locomotor variance:** Drive/Walk/Ship use linear-accel +
1.5×-decel ramping. Fly/Hover use symmetric accel/decel with a
distance-driven target speed. Jumpjet adds vertical climb and wobble.
Teleport does not move continuously — it warps in discrete steps with
a piggybacked Drive for the ground portion. Rocket is one-shot launch
+ ballistic; no acceleration tuning. Each locomotor's specific math is
in its own existing report (cross-referenced).

---

## INI surface

### `rulesmd.ini` — per-`[TechnoType]` (unit movement keys)

```ini
[GRIZZLY]                   ; Allied Grizzly Tank
Speed=8
...
```

```ini
[E1]                        ; GI
Speed=4
...
```

```ini
[CIVA]                      ; civilian air?
Speed=14                    ; quick civilian aircraft
...
```

```ini
[SHAD]                      ; Soviet Apocalypse turret
ROT=5                       ; turret rotation rate
...
```

```ini
[REFINERY-RELATED]
SpeedType=Amphibious
MovementZone=AmphibiousDestroyer
```

| Key | Type | Default | TechnoType byte offset | Notes |
|---|---|---|---|---|
| `Speed=` | int | `-1` (use fallback) | `+0x678` | INI value clamped to `[0, 100]`, then converted via `val * 256 / 100` and clamped to `[0, 255]`. Stored as leptons-per-tick. |
| `TurnRate=` | int | (per-locomotor default) | (per-locomotor field — Jumpjet at JumpjetControls struct) | Angular delta per tick for **non-turreted** units (the body itself rotates). |
| `ROT=` | int | (per-type field) | (TechnoType field; offset deferred) | "Rate of Turn" — angular delta per tick for the **turret** (separate from body). Used by vehicles with independent turrets and by missiles for homing-rate. |
| `Accelerates=` | bool | `true` | (TechnoType field; offset deferred) | When `false`, the unit doesn't ramp — jumps straight to max_speed (used by infantry walk). |
| `SpeedType=` | enum | `Track`/`Wheel`/etc. | (TechnoType field) | Terrain class for slope penalty / passability — see `Land=` reciprocal under `[LandType]` |
| `MovementZone=` | enum | (per-locomotor) | (TechnoType field) | Pathfinding zone class (Infantry/Vehicle/Aircraft/Amphibious/Hover/etc.) — determines which cells the unit is allowed to enter |
| `Locomotor=` | CLSID | (none) | (TechnoType field) | COM CLSID of the locomotor (e.g., `{4A582741-...}` for Drive). Maps to one of the 11 ILocomotion implementations. |

**Confidence (Speed at +0x678):** HIGH. `TechnoTypeClass::ReadINI` at
`0x00714699` executes `MOV [EBP + 0x678], EDX` after the `ReadInt` call
for the bare `"Speed"` key (string at `0x0081d9cc`); the value passes
through a clamp-to-`[0,100]`, then `val * 256 / 100` (via the classic
`IMUL 0x51eb851f / SAR 5` divide-by-100 idiom), and a final clamp to
`[0, 255]` before the store.

**Correction (2026-05-19):** an earlier iteration claimed `Speed` lived
at `+0x618` and that the function at `0x00717800` was
`TechnoTypeClass::GetSpeed`. Direct disassembly proved both wrong.
`+0x618` is `FlightLevel` (write at `0x0071234a` in
`TechnoTypeClass::ReadINI` after `PUSH 0x83c854` = `"FlightLevel"`), and
the function at `0x00717800` reads `[ECX + 0x618]` with fallback
`RulesClass + 0x7b4` — both FlightLevel fields. `RulesClass + 0x7b4` is
the global `FlightLevel` default (written by `RulesClass::ReadGeneral`
at `0x0066f308`), not the Speed default. The function has been renamed
in Ghidra to `TechnoTypeClass::GetFlightLevel`. The canonical per-instance
speed reader lives behind the FootClass vtable slot at `+0x38c` (called
by `FootClass::GetCurrentSpeed @ 0x004db1a0` and `ftol`'d from a float —
that pipeline is still to be documented).

### `rulesmd.ini` — `[General]` (movement-related globals)

```ini
GameSpeedBias=1.6       ; multiplier to overall game object movement speed
;was 1.2

MissileSpeedVar=.25     ; speed flucuation percentage that guided missiles have
MissileROTVar=.25       ; rate of turn fluctuation percentage that guided missiles have
MissileSafetyAltitude=750 ;gs this is the altitude a missile fired at an air target that dies will fly to before detonating.
```

| Key | Type | Default | RulesClass byte offset | Notes |
|---|---|---|---|---|
| `GameSpeedBias=` | double | `1.6` | `+0x1418` | Multiplier applied to all unit movement |
| `MissileSpeedVar=` | double | `0.25` | (RulesClass field) | ±25% random variance in missile speed at spawn |
| `MissileROTVar=` | double | `0.25` | (RulesClass field) | ±25% random variance in missile ROT at spawn |
| `MissileSafetyAltitude=` | int | `750` | (RulesClass field) | Altitude (leptons) a missile climbs to when target dies |

`GameSpeedBias=1.6` was bumped from `1.2`. The `;was 1.2` comment is
preserved in the INI — the bump speeds up the *entire game* by ~33%
without changing the GameSpeed slider or any per-unit `Speed=`.

### `rulesmd.ini` — `[JumpjetControls]` (Jumpjet locomotor defaults)

```ini
; Jumpjet movement controls
[JumpjetControls] ;gs These are now merely defaults and units can define their own
TurnRate=4
Speed=14
Climb=5
CruiseHeight=500	; cruiseheight should be higher than a bridge, just to be safe
Acceleration=2
WobblesPerSecond=.15 ; was .25
WobbleDeviation=40 ; was 40
```

Read by `RulesClass::ReadJumpjetControls` @ `0x006743D0`. Per-unit
overrides are possible (a TechnoType can set its own `JumpjetTurnRate=`,
`JumpjetSpeed=`, etc.). Detailed semantics in
[JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md).

| Key | Type | Default | Notes |
|---|---|---|---|
| `TurnRate=` | int | `4` | Default per-tick turn delta for Jumpjet units (Rocketeer) |
| `Speed=` | int | `14` | Default max horizontal speed |
| `Climb=` | int | `5` | Default vertical climb rate (per-tick altitude delta) |
| `CruiseHeight=` | int | `500` | Default cruise altitude in leptons (≈1.95 cells) |
| `Acceleration=` | int | `2` | Default per-tick horizontal accel ramp delta |
| `WobblesPerSecond=` | double | `0.15` | Frequency of the in-flight wobble visual |
| `WobbleDeviation=` | int | `40` | Amplitude of wobble in leptons |

### `rulesmd.ini` — `[LandType]` blocks (slope speed / passability table)

```ini
[Clear]
Foot=80%
Light=100%
Heavy=100%
Wheel=100%
Track=100%
Hover=100%
Float=0%
Amphibious=100%
```

Per-terrain `LandType` block with per-`SpeedType` percentage. Parsed
by `RulesClass::ReadSpeedTypeLandTypeTable` @ `0x00674000`. Default
9-entry per-slope lookup at `DAT_0089ea40` is the runtime fallback if
the rules-side table is missing. Used by
`FootClass::Get_Slope_Speed_Factor` @ `0x004dc760` (see "Hardcoded
constants").

Detail of every `[LandType]`-`[SpeedType]` cross-reference is owned
by a future terrain-cost doc; per-tick consumption math is owned here.

### `rulesmd.ini` — per-`[HouseType]` (country-specific multipliers)

```ini
[Americans]
SideName=GDI
...
```

The per-house speed multipliers are NOT loaded from string-named INI
keys — they're hardcoded into the binary's `HouseTypeClass` constructor
and overlaid into byte offsets:

| HouseType byte offset | Subject | Read by |
|---|---|---|
| `+0x128` | Building movement bonus (for things like Allied MCV / Soviet War Miner deploy/undeploy?) | `HouseClass::GetSpeedBonus` on `WhatAmI() == 0x10` |
| `+0x12C` | Infantry movement bonus | `HouseClass::GetSpeedBonus` on `WhatAmI() == 0x28` |
| `+0x130` | Aircraft movement bonus | `HouseClass::GetSpeedBonus` on `WhatAmI() == 3` |
| (default `DAT_007e2ac8 = 1.0`) | Vehicle / fallback | `HouseClass::GetSpeedBonus` on other |

The semantics of "house speed bonus" in YR appear to be **largely
constant 1.0** in shipping play (the value at `_DAT_007e2ac8` is 1.0,
returned for non-matched type classes). Specific house types may
override only for narrow special cases. Detailed enumeration of which
HouseType sets which offsets is deferred to a future country-bonus
doc.

---

## Hardcoded constants

### TechnoType `Speed=` field (per-type, at +0x678)

Per-type `Speed=` writes to `TechnoTypeClass + 0x678` after the
`ReadINI` conversion `clamp(val, 0, 100) * 256 / 100`, then clamped
to `[0, 255]`. See [game-speed-master-clock.md](game-speed-master-clock.md)
for the offset confirmation block and the prior-misclaim correction.

**There is no dedicated `TechnoTypeClass::GetSpeed` function.** The
function at `0x00717800` previously labeled that way is actually
`TechnoTypeClass::GetFlightLevel` — it reads byte `+0x618`, which is
the FlightLevel field, with fallback `RulesClass + 0x7b4` (also the
FlightLevel default, not the Speed default). Renamed in Ghidra
2026-05-19. See the prior-correction block in this doc's INI surface
section.

### `TechnoClass::GetTypeSpeed` (the per-instance base-speed reader)

`0x0070efe0`. Bound to vtable slot `+0x38c` on all three FootClass
subclasses (`vtable__UnitClass + 0x38c`, `vtable__InfantryClass + 0x38c`,
`vtable__AircraftClass + 0x38c` — all three point to the same
function; inherited implementation, no overrides).

```asm
0070efe0: MOV  EAX, [ECX]            ; vtable
0070efe2: CALL [EAX + 0x84]          ; virtual GetType() → TechnoTypeClass* in EAX
0070efe8: TEST EAX, EAX
0070efea: JZ   .null
0070efec: MOV  EAX, [EAX + 0x678]    ; TechnoType.Speed
0070eff2: RET
.null:
0070eff3: XOR  EAX, EAX
0070eff5: RET
```

Returns the raw per-type `Speed` value as int. **No multipliers are
applied here** — they layer in `FootClass::GetCurrentSpeed`. Named
`TechnoClass__GetTypeSpeed` in Ghidra 2026-05-19 (HIGH confidence:
direct disassembly + all three derived-class vtable slots verified
to point here).

### `FootClass::GetCurrentSpeed` (the full per-tick effective speed)

`0x004db1a0`. Full disassembly as decoded this session:

```asm
004db1a7: MOV  EAX, [ESI]                       ; FootClass vtable
004db1a9: CALL [EAX + 0x84]                     ; virtual GetOwner() → HouseClass* in EAX
004db1af: MOV  ECX, [ESI + 0x21c]               ; FootClass+0x21c → ECX (next thiscall 'this')
004db1b5: PUSH EAX                               ; arg2 for GetSpeedBonus
004db1b6: CALL HouseClass::GetSpeedBonus        ; returns SpeedBonus double on FP stack
004db1bf: FSTP qword [ESP + 0xc]                ; stash SpeedBonus locally
004db1c3: CALL [EDX + 0x38c]                    ; TechnoClass::GetTypeSpeed → int EAX
004db1c9: MOV  [ESP + 0x8], EAX
004db1cd: FILD dword [ESP + 0x8]                ; load BaseSpeed as float
004db1d1: FMUL qword [ESP + 0xc]                ; × SpeedBonus
004db1d5: FMUL qword [ESI + 0x580]              ; × FootClass+0x580 (unidentified runtime double — see below)
004db1db: CALL Math::ftol                        ; truncate to int
004db1e0: PUSH 0                                 ; ability_index = 0 = FASTER
004db1e8: CALL TechnoClass::HasWeaponAbility
004db1ed: TEST AL, AL
004db1ef: JZ   .no_veteran_speed
004db1f1: FILD dword [ESP + 0x8]                ; reload as float
004db1f5: MOV  EAX, [0x008871e0]                ; g_RulesClass
004db1fa: FMUL qword [EAX + 0x678]              ; × RulesClass.VeteranSpeed (= 1.2 in shipping YR)
004db200: CALL Math::ftol
004db205: MOV  [ESP + 0x8], EAX
.no_veteran_speed:
004db209: FILD dword [ESP + 0x8]
004db20d: FMUL qword [ESI + 0x578]              ; × SetSpeedFraction (current/max ratio, set by SetSpeedFraction)
004db213: CALL Math::ftol
004db21e: CALL [EDX + 0x2c]                     ; virtual WhatAmI() — UnitClass returns 1
004db221: CMP  EAX, 0x1
004db224: JNZ  .return_full
004db226: MOV  EAX, [ESI + 0x6cc]               ; half-speed flag
004db22c: CMP  EAX, -1
004db231: JZ   .return_full
                                                 ; else: EAX = EDI / 2 (half speed)
```

Composition (in order applied):

| Step | Operation | Source / value |
|------|-----------|----------------|
| 1 | Multiply by SpeedBonus (double) | `HouseClass::GetSpeedBonus` — per-house, per-TechnoType-subclass |
| 2 | Read BaseSpeed (int) | `TechnoClass::GetTypeSpeed` — `TechnoType.Speed` at `+0x678` |
| 3 | Multiply by FootClass+0x580 (double) | unidentified runtime multiplier (see below) |
| 4 | If `HasWeaponAbility(FASTER)`: multiply by `RulesClass[+0x678]` | `VeteranSpeed`, **1.2 in shipping YR** (1.0 default) |
| 5 | Multiply by FootClass+0x578 (double) | `SetSpeedFraction` result — `current_speed / max_speed` ratio |
| 6 | If `WhatAmI() == 1` AND `+0x6cc != -1`: halve | half-speed flag, **effectively dead in YR** (see below) |

**Critical finding: `GameSpeedBias` is NOT applied in this pipeline.**
A byte-pattern search across the entire binary for any FMUL/FLD of
`[reg + 0x1418]` returned 6 hits, all inside `HouseClass::SetDifficulty`
@ `0x004f6ec0`. The shipping-YR INI comment "multiplier to overall
game object movement speed" is misleading; see "`GameSpeedBias` storage
and consumers" below.

**Confidence: HIGH** — every instruction in the pipeline was read from
the binary this session.

### `FootClass::Get_Slope_Speed_Factor`

`0x004dc760`:

```c
float10 __fastcall FootClass__Get_Slope_Speed_Factor(int param_1) {
    if ((*(int *)(param_1 + 0x5d4) != 0) &&
        (*(char *)(*(int *)(*(int *)(param_1 + 0x5d4) + 0x24) + 0xf2) != '\0')) {
        return (float10)_g_Const_1_0;       // 1.0 = no slope penalty (bridge or similar override)
    }
    return (float10)*(double *)(param_1 + 0x530);   // cached slope factor
}
```

Reads a cached per-unit slope factor at `FootClass + 0x530` (a
`double`). The cache is populated during `Process_Movement` from a
9-entry lookup table at `DAT_0089ea40` (per [LOCOMOTION_MATH_AND_CONSTANTS.md](../LOCOMOTION_MATH_AND_CONSTANTS.md)).
Bridge-cells skip the penalty (return 1.0).

### `HouseClass::GetSpeedBonus`

`0x0050c050`:

```c
float10 __thiscall HouseClass__GetSpeedBonus(int param_1, int *param_2) {
    int iVar1 = (**(code **)(*param_2 + 0x2c))();   // WhatAmI()
    if (iVar1 == 3) {
        return (float10)*(float *)(*(int *)(param_1 + 0x34) + 0x130);   // AirSpeedBonus
    }
    if (iVar1 != 0x10) {
        if (iVar1 != 0x28) {
            return (float10)_DAT_007e2ac8;            // default 1.0
        }
        return (float10)*(float *)(*(int *)(param_1 + 0x34) + 300);    // InfantrySpeedBonus (300 = 0x12C)
    }
    return (float10)*(float *)(*(int *)(param_1 + 0x34) + 0x128);     // BuildingSpeedBonus
}
```

`WhatAmI()` return values mapped to HouseType offsets:
- `3` = Aircraft → `HouseType + 0x130`
- `0x10` = `Building` → `HouseType + 0x128`
- `0x28` = `InfantryClass` → `HouseType + 0x12C`
- anything else (vehicles, ships, etc.) → `1.0`

So **vehicles do not get country speed bonuses** in this function;
only aircraft, infantry, and buildings do. Cross-ref deferred to a
future country-bonus doc.

### `GameSpeedBias` storage and consumers

`RulesClass + 0x1418` (double). Verified via direct memory dump of
`RulesClass::ReadGeneral` at the `GameSpeedBias` parse block:

```
8b 96 1c 14 00 00      mov edx, [esi+0x141c]    ; default high dword
8b 86 18 14 00 00      mov eax, [esi+0x1418]    ; default low dword
52 8b 0d 9c 0c 7f 00   push edx, mov ecx, ...
50                     push eax
68 24 bf 83 00         push 0x83bf24            ; "GameSpeedBias"
51                     push ecx
8b cf                  mov ecx, edi
e8 9d 78 eb ff         call CCINIClass::ReadDouble
dd 9e 18 14 00 00      fstp qword [esi+0x1418]  ; store double to RulesClass+0x1418
```

`RulesClass + 0x1418` = `GameSpeedBias` (double, default `1.6` in
shipping YR, bumped from `1.2`).

**Consumer enumeration (verified 2026-05-19):** byte-pattern search
across the entire binary for `FMUL`/`FLD`/`FADD`/`FDIV` of
`[reg + 0x1418]` (all 8 base-register ModR/M variants) returned **6
total hits, all inside `HouseClass::SetDifficulty` @ `0x004f6ec0`**.
No other function reads this field.

`SetDifficulty` writes 3 GameSpeedBias-scaled per-house cached doubles:

| Per-difficulty source `[RulesClass + 0x1538 + diff*0x50 + off]` | Cached at `HouseClass +` | SP path | MP path |
|---|---|---|---|
| `off = +0x08` | `+0x190` | × `GameSpeedBias` | × `GameSpeedBias` × `HouseType+0xd0` |
| `off = +0x10` | `+0x198` | × `GameSpeedBias` | × `GameSpeedBias` × `HouseType+0xd8` |
| `off = +0x30` | `+0x1b8` | × `GameSpeedBias` | × `GameSpeedBias` × `HouseType+0xf8` |

The per-difficulty source values come from the `[Easy] / [Normal] /
[Difficult]` sections in `rulesmd.ini` (each section reads as 10
doubles into a contiguous 0x50-byte slot at `RulesClass + 0x1538 +
diff * 0x50`).

**Player-visible implication:** the YR INI comment _"multiplier to
overall game object movement speed"_ is misleading. The `1.2 → 1.6`
shipping bump does **not** uniformly scale every unit's movement.
`FootClass::GetCurrentSpeed` — the per-tick per-unit speed pipeline
— does not consult `RulesClass + 0x1418` at all. Instead,
`GameSpeedBias` only scales three per-house AI-difficulty caches at
`HouseClass + 0x190 / +0x198 / +0x1b8`, regenerated on every
`SetDifficulty` call. Whether (and which) units feel the bump
depends on which consumers read those cached fields — most likely
AI-driven movement-related decisions (production rates, threat
evaluation, attack tempo), not the per-unit movement pipeline.

**Confidence: HIGH** on "consumed only by `SetDifficulty`" (exhaustive
byte-pattern search). **MEDIUM** on the player-visible interpretation
— downstream consumers of `HouseClass + 0x190 / +0x198 / +0x1b8`
have not been traced.

### `CCINIClass::ReadSpeed` (used by weapon/projectile speed; NOT unit movement)

`0x00474810`:

```c
int CCINIClass__ReadSpeed(undefined4 param_1, undefined4 param_2, int param_3) {
    uint uVar1 = CCINIClass__ReadInt(param_1, param_2, 0xffffffff);
    if (uVar1 == 0xffffffff) return param_3;
    if (99 < (int)uVar1) uVar1 = 100;
    int iVar2 = (int)((((int)uVar1 < 1) - 1 & uVar1) << 8) / 100;
    if (0xfe < iVar2) iVar2 = 0xff;
    return iVar2;
}
```

INI 0–100 → internal 0–255 via `value × 256 / 100`, clamped. **This is
used for `[Weapon] Speed=` (projectile launch speed) — NOT for
`[TechnoType] Speed=` (unit movement speed).** The TechnoType Speed
keeps its raw INI integer value (1–14 range typical). Two different
parsers for the same key name "Speed" in different sections.

### Drive locomotor: linear accel, 1.5× decel

Per [LOCOMOTION_MATH_AND_CONSTANTS.md](../LOCOMOTION_MATH_AND_CONSTANTS.md) §3.1:

```c
if (current_speed < target_speed)
    current_speed += acceleration;
if (current_speed > target_speed)
    current_speed -= acceleration * 1.5;        // DAT_007e48f0 = IEEE 754 double 1.5
current_speed = clamp(current_speed, 0, max_speed);
```

`DAT_007e48f0 = 1.5` is the universal **deceleration multiplier**
across Drive, Walk, Ship — confirmed via direct IEEE 754 inspection.
So braking is always 1.5× faster than accelerating, which gives the
characteristic feel of RA2 tanks coming to a stop quickly but
starting up slowly.

### Drive locomotor: movement budget = 7 leptons per track step

Per [LOCOMOTION_MATH_AND_CONSTANTS.md](../LOCOMOTION_MATH_AND_CONSTANTS.md) §4.1:

> **Movement budget**: subtract 7 per track step consumed per frame.

Each "track step" in the drive's track table is `7 leptons`. The unit
consumes ⌊current_speed / 7⌋ track steps per tick (with `residual_ticks`
at `DriveLocomotionClass + 0x4C` holding the remainder for the next
tick). So `Speed=14` ≈ 2 track steps per tick.

### Fly locomotor: distance-driven target speed, symmetric accel

Per [LOCOMOTION_MATH_AND_CONSTANTS.md](../LOCOMOTION_MATH_AND_CONSTANTS.md) §5:

```c
target_speed = computed_from_distance;
delta = max_speed / (accel_factor * 60);

if (current_speed < target_speed)
    current_speed += delta;
if (current_speed > target_speed)
    current_speed -= delta;    // symmetric (NOT 1.5×)
```

Per-frame delta is `max_speed / (accel_factor * 60)` where 60 is the
hardcoded "ramp-up duration in ticks". So an aircraft with
`Acceleration=2` ramps from 0 to max in 120 ticks (~6 s wall-clock at
20 ticks/sec Medium). `RulesClass + 0x5F0` holds the acceleration
factor; `+0x16B8` is the speed cap.

### Descent slowdown zones

Per [LOCOMOTION_MATH_AND_CONSTANTS.md](../LOCOMOTION_MATH_AND_CONSTANTS.md) §5:

- **20 ticks** before destination: aircraft begin decelerating
- **50 ticks** for further approach: secondary descent zone

Cross-ref Fly/Hover docs for the full state-machine.

### Walk locomotor: 7-state machine, same accel pattern

Per [LOCOMOTION_MATH_AND_CONSTANTS.md](../LOCOMOTION_MATH_AND_CONSTANTS.md) §6:

> 7-state machine. Speed ramping identical to drive (linear accel, 1.5x decel).
> Acceleration constant at `DAT_007e48f0`.

Infantry walking uses the same 1.5×-decel ramp. The 7 states are
typically Idle/StartWalk/Walk/Stop/Run/Crouch/Standup or similar —
detailed state machine in [WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md).

### `Accelerates=` flag (per-TechnoType)

When `Accelerates=false` (rare; default `true`), the locomotor skips
the ramp entirely and jumps directly to `target_speed`. Used by some
infantry variants and possibly tunnel-creature TS legacy units (not in
shipping YR units). Identification of the exact byte-offset for this
bool deferred.

### Direction tables (8-way movement)

Per [LOCOMOTION_MATH_AND_CONSTANTS.md](../LOCOMOTION_MATH_AND_CONSTANTS.md) §3.3:

| Symbol | Description |
|---|---|
| `DAT_0089f6d8` / `DAT_0089f6dc` | 8 entries of `(dx, dy)` in leptons, 8 bytes each |
| `DAT_0089f688` | 8 entries of `(dx, dy)` in cells (shorts) |
| `DAT_007e7b30` | Track-transformation flags (mirror/flip per direction) |

These tables encode the 8-direction movement offsets used by every
locomotor. The 8 facings correspond to N/NE/E/SE/S/SW/W/NW; each
facing has both a per-cell offset and a per-lepton offset.

### Locomotor CLSID table

Per [LOCOMOTION_MATH_AND_CONSTANTS.md](../LOCOMOTION_MATH_AND_CONSTANTS.md) §1
(reproduced for cross-reference):

| Class | CLSID | Units |
|---|---|---|
| `DriveLocomotionClass` | `{4A582741-...}` | Grizzly, Prism Tank, all tanks |
| `HoverLocomotionClass` | `{4A582742-...}` | Robot Tank |
| `TunnelLocomotionClass` | `{4A582743-...}` | (TS legacy — unused in YR) |
| `WalkLocomotionClass` | `{4A582744-...}` | GI, Dog, Engineer, all infantry |
| `DropPodLocomotionClass` | `{4A582745-...}` | (TS legacy — unused in YR) |
| `FlyLocomotionClass` | `{4A582746-...}` | Harrier, Kirov |
| `TeleportLocomotionClass` | `{4A582747-...}` | Chrono Legionnaire, Chrono Miner |
| `MechLocomotionClass` | `{55D141B8-...}` | (TS legacy — unused in YR) |
| `ShipLocomotionClass` | `{2BEA74E1-...}` | Destroyer, Aegis |
| `JumpjetLocomotionClass` | `{92612C46-...}` | Rocketeer |
| `RocketLocomotionClass` | `{B7B49766-...}` | V3 Rocket |

Each locomotor has its own `ILocomotion::Process` per-tick method;
`LocomotionClass::Apparent_Speed` at `0x0055ad10` is a trampoline to
the locomotor's `vtable + 0x538` "GetApparentSpeed" virtual.

### Turn rate (ROT) — per-TechnoType field

`ROT=` (Rate of Turn) is a per-TechnoType field used by both:
- **Turreted units** — turret rotation speed (independent of body
  rotation, separate `TurnRate` for the body)
- **Missiles** — homing turn rate per tick

The byte offset on TechnoType is not extracted in this iteration —
flagged for follow-up. The conversion is `internal_ROT = INI_ROT *
some_constant` to convert from "tenths of facing-units per tick" to
the engine's 256-unit facing system.

### `Mission_Move` and per-tick movement dispatch

The per-tick movement dispatch happens inside `LogicClass::PerTickUpdate`
(see [logic-vs-render-loop.md](logic-vs-render-loop.md)) via the
per-entity vtable-`+0x5c` loop. Each FootClass entity's `+0x5c` slot
ultimately calls `ILocomotion::Process` on its bound locomotor, which
advances `current_speed` and moves the unit along its track.

The chain: `FootClass::AI` → `Locomotor::Process` → consume
`movement_budget` → advance `current_position` toward `head_to` → emit
animation frame update → on arrival at `head_to`, advance to next
waypoint from path queue.

---

## Tick / frame topology

| Stage | Clock | Where |
|---|---|---|
| Speed/ROT lookup at construction | (cached per-unit) | TechnoType field read once and cached |
| Per-tick locomotor process | game-tick | `ILocomotion::Process` (via vtable+0x538/0x53C) |
| Per-tick speed ramp | game-tick | inside `Process_Drive_Track` / equivalent |
| Per-tick movement budget consumption | game-tick (7 leptons per track step for Drive) | `residual_ticks` carryover |
| `GameSpeedBias` multiplier | applied per-frame in `GetCurrentSpeed` | every consumer of `FootClass::GetCurrentSpeed` |
| Slope factor | cached, updated on slope transition | `FootClass + 0x530` cache |
| House speed bonus | applied per-frame in `GetCurrentSpeed` | per-house-type-class table |
| Veterancy SPEED multiplier | applied per-frame (one flag check) | `TechnoClass::HasWeaponAbility(0)` |
| Turret rotation (ROT) | game-tick | per-frame angular delta toward target facing |
| Aircraft accel ramp | game-tick (delta = max_speed / (accel × 60)) | Fly locomotor process |

### Clock binding

All movement is on the **master game-tick clock**. Wall-clock
movement-per-second therefore scales linearly with GameSpeed:
- GameSpeed=Fastest (uncapped): ~60 ticks/sec → Speed=8 unit moves
  ~8 * 60 = 480 leptons/sec ≈ 1.88 cells/sec
- GameSpeed=Slowest (≈10 ticks/sec): same unit moves ~8 * 10 = 80
  leptons/sec ≈ 0.31 cells/sec

`GameSpeedBias=1.6` multiplies all these wall-clock rates by 1.6.
**It does NOT change the slot mapping** — Speed=8 still consumes 8
leptons of budget per tick; the multiplier scales the budget itself.

### Per-tick movement composition (for ground vehicle on flat terrain)

```
budget_per_tick = TechnoType.Speed
                * GameSpeedBias
                * HouseSpeedBonus    (1.0 for vehicles in shipping YR)
                * VeteranSpeedMult    (1.0 if not veteran)
                * SlopeFactor         (1.0 on flat ground)
                * (0.5 if cell penalty active)
```

Then `current_speed` ramps toward `budget_per_tick` at `+accel`/tick
(or `-accel × 1.5`/tick if decelerating), clamped to `[0,
budget_per_tick]`.

---

## Multipliers and modifiers

### `GameSpeedBias=1.6` (rules-global) — **not in the per-unit pipeline**

**Correction (2026-05-19):** prior iteration claimed `GameSpeedBias`
"multiplies every unit's effective speed, always applied". Direct
evidence refutes this. `RulesClass + 0x1418` is read in **exactly
one function** in the binary: `HouseClass::SetDifficulty` @ `0x004f6ec0`,
where it scales 3 of the per-difficulty per-house AI caches
(`HouseClass + 0x190 / +0x198 / +0x1b8`). `FootClass::GetCurrentSpeed`
— the per-tick per-unit speed pipeline — does **not** touch
`RulesClass + 0x1418`. See "`GameSpeedBias` storage and consumers"
above for full enumeration.

### `HouseClass::GetSpeedBonus` (country-specific)

Aircraft / Infantry / Buildings get per-house-type-class multipliers
from HouseType `+0x128 / +0x12C / +0x130`. Vehicles, ships, and most
others get **`1.0`** (the default `DAT_007e2ac8`). So in shipping YR,
ground vehicles **do not** have a country speed advantage; the
visible "Soviet tanks are slower than Allied tanks" difference comes
from per-unit `Speed=` differences (e.g., Grizzly Speed=8 vs Rhino
Speed=5), not from house bonuses.

### Veterancy SPEED ability

`TechnoClass::HasWeaponAbility(this, 0)` (ability index 0 = FASTER)
checks the per-unit veterancy state combined with the
`VeteranAbilities` / `EliteAbilities` bitmask. When set,
`FootClass::GetCurrentSpeed` at `0x004db1fa` does
`FMUL qword [g_RulesClass + 0x678]` — applying `RulesClass.VeteranSpeed`
(double, default `1.0`, **shipping YR = `1.2`** per
[VETERANCY_SYSTEM_GHIDRA_REPORT.md](../VETERANCY_SYSTEM_GHIDRA_REPORT.md)).

`RulesClass.EliteSpeed` at `+0x680` is parsed and stored but **not
applied here** — the FASTER check returns true for both veteran and
elite, and the pipeline only multiplies by `VeteranSpeed`. Whether
`EliteSpeed` is consulted elsewhere is deferred.

### Slope factor (per-cell terrain)

9-entry lookup at `DAT_0089ea40` per locomotion type, indexed by cell
slope class (`CellClass + 0x11C`). Cached at `FootClass + 0x530`.
Bridges (`*(int *)(param_1 + 0x5d4) != 0 && bridge-related-flag !=
0`) skip the penalty.

Per [LOCOMOTION_MATH_AND_CONSTANTS.md](../LOCOMOTION_MATH_AND_CONSTANTS.md):

| RulesClass offset | Purpose |
|---|---|
| `+0x768` | Uphill speed multiplier |
| `+0x770` | Downhill speed bonus |
| `+0x778` | Additional uphill factor |
| `+0x780` | Additional downhill factor |

### Iron Curtain / EMP / Mind Control / Stasis movement freeze

- **Iron Curtain:** unit still moves at full speed during IC.
- **EMP:** unit cannot move (mission cleared, locomotor halts).
- **Stasis:** unit frozen completely.
- **Mind Control:** unit retargets but can still move.
- **Magnetron:** unit lifted off ground (Magnetron lift cycle owns
  this — see [magnetron-lift-cycle.md](magnetron-lift-cycle.md)).

Specific gating fields deferred to the respective effect docs.

### `FootClass + 0x6cc` half-speed flag — **effectively dead in YR**

When `WhatAmI() == 1` (UnitClass = vehicle) AND `FootClass + 0x6cc != -1`,
`GetCurrentSpeed` halves the result. The decompile rendered this as
`param_1[0x1b3]` (since Ghidra typed `param_1` as `int *` →
`0x1b3 * 4 = 0x6cc` byte offset).

Writer enumeration (verified 2026-05-19 — searched all 8 base-register
`MOV [reg+0x6cc], r32` and `MOV [reg+0x6cc], imm32` ModR/M variants):

| Site | Function | Value written |
|---|---|---|
| `0x00735884` | `UnitClass::Constructor` | `-1` (init) |
| `0x007440eb` | `UnitClass::Limbo` | `-1` (reset) |
| `0x00413d55` | `AircraftClass::Constructor` | (init, value unverified) |
| `0x00419ce5` | `AircraftClass::Mission_Enter` | target id (non-(-1)) |
| `0x00419d57` | `AircraftClass::Mission_Enter` | target id |
| `0x0041b67f` | `AircraftClass::Detach` | target id |
| `0x0041bbff` | `AircraftClass::FindBuildingToDock` | (imm32) |
| `0x0041bc1c` | `AircraftClass::FindBuildingToDock` | target id |

**No UnitClass code writes a non-(-1) value to this field.** All 4
active writers live in AircraftClass — but `AircraftClass::WhatAmI() == 2`,
so the half-speed gate (`== 1`) never fires for aircraft either. Net
result in vanilla YR: **the gate is dead code** for both classes —
vehicles never trip it because their `+0x6cc` is always `-1`, and
aircraft trip the field but not the gate.

Almost certainly TS-legacy plumbing (the original Tiberian Sun
likely had a half-speed-while-approaching-dock state that survived
into the binary but never gets the writer-class-matches-gate
combination YR needs). **Do not implement this gate** in the Rust
port unless evidence emerges of a code path that writes UnitClass's
`+0x6cc` to a non-(-1) value.

**Confidence: HIGH** (exhaustive writer enumeration).

### `FootClass + 0x580` unconditional multiplier (unidentified)

The disassembly shows `FMUL qword [ESI + 0x580]` at `0x004db1d5` —
applied unconditionally before the veterancy check and before the
SetSpeedFraction multiply. A `double` value, written by an FSTP at
`0x0048306c` (function not yet labeled, in the 0x0048 region).
The writer multiplies the existing `+0x580` by a stack-local double
— an accumulator pattern, not a one-shot init.

Plausible candidates (not yet confirmed):
- Damage-state speed penalty (units slow when low HP)
- Power-out speed factor
- Crawl-while-deploying multiplier
- A different terrain runtime factor than `+0x530` (the
  `Get_Slope_Speed_Factor` cache)

**Deferred for follow-up.** Until identified, treat as `1.0` in any
Rust-side implementation of `GetCurrentSpeed` — but this is a
known unknown that could account for visible slowdown behaviors in
specific situations.

### `FootClass + 0x578` SetSpeedFraction multiplier

The final unconditional multiplier in `GetCurrentSpeed`
(`FMUL qword [ESI + 0x578]` at `0x004db20d`). Written by
`TechnoClass::SetSpeedFraction` @ `0x004d3710` — the per-instance
"what fraction of max speed am I currently at" double, used by the
acceleration ramp. Just-spawned units start near `0.0` and ramp up
when `Accelerates=true`; full-speed units sit at `1.0`; braking
units sit between. Cross-reference acceleration math at
[ILOCOMOTION_COM_PROTOCOL_SPEC.md](../ILOCOMOTION_COM_PROTOCOL_SPEC.md).

### `Accelerates=false` (per-TechnoType)

Skips the ramp entirely. Rare in shipping units.

### `MovementZone` and `SpeedType`

Pathfinding-relevant only — they determine **which cells** the unit
can enter, not how fast it moves. Speed within an allowed cell is
governed by `Speed=` and slope. The interaction with `[LandType]`
percentages provides per-terrain slowdown for crossing different
ground types.

### Per-locomotor differences

| Locomotor | Accel pattern | Turn pattern | Cruise altitude | Notes |
|---|---|---|---|---|
| Drive | Linear accel + 1.5× decel | Snap to track waypoint heading | n/a | 72-entry TurnTrack, 16 RawTrack |
| Walk | Linear accel + 1.5× decel | 8-way facing snap | n/a | 7-state machine |
| Ship | Linear accel + 1.5× decel; **decel read from TypeClass+0x678** | 67-entry TurnTrack, 14 RawTrack | n/a | Spawns wake every 8 frames vs drive's dust every 10 |
| Hover | Linear accel | Continuous angle | small float Z | Robot Tank |
| Fly | Symmetric accel; target = f(distance) | Continuous angle (banking visual) | `CruiseHeight` from rules | 20 / 50-tick descent slowdown zones |
| Jumpjet | Constant accel | TurnRate=4 default; wobble | `CruiseHeight=500` | Rocketeer |
| Rocket | One-shot launch + ballistic | Ballistic trajectory only | escape to space | V3 rocket |
| Teleport | Discrete warp + piggybacked Drive | n/a (instantaneous) | n/a | Chrono Legionnaire, Chrono Miner |
| Mech | (TS-legacy, unused in YR) | | | |
| DropPod | (TS-legacy, unused in YR) | | | |
| Tunnel | (TS-legacy, unused in YR) | | | |

---

## Edge cases

### Pause behavior

Per [logic-vs-render-loop.md](logic-vs-render-loop.md): the per-entity
AI loop in `LogicClass::PerTickUpdate` runs unconditionally during
pause. So **units continue to advance through their locomotor
`Process` during pause** — but `Mission_Move` (which queues the next
path waypoint) is in `LogicClass::AI`'s input-dispatch chain, gated by
`g_GameState == 0`. So: a unit already moving along its current
waypoint will complete that waypoint during pause; a unit waiting for
a new destination will not get one.

**Player-visible effect:** open the in-game menu while units are
mid-move — they'll complete their current segment but won't accept
new orders. Confirmed against the partial-pause model.

### Save / load mid-move

`DriveLocomotionClass` (and equivalents) save their internal state
(`destination`, `head_to`, `track_index`, `point_index`, `is_reversed`,
`current_speed`, `residual_ticks`, `cached_slope_index`) via the
standard save/load mechanism. The unit resumes mid-track with the
exact remaining budget.

### Replay determinism

Movement involves no `Random__RandomRanged` calls in the steady-state
case. The only randomness is at path-acquisition time (path stuck
counter, scatter direction selection). Replays move identically.

### `MissileSpeedVar` / `MissileROTVar`

Missiles get a one-shot random ±25% variance at launch (per the INI
default). The variance is sampled deterministically via the seeded
RNG, so identical missiles fired on the same tick from the same source
to the same target launch with identical velocities across all
peers. Cross-ref to [animation-rate-delay.md](animation-rate-delay.md)
for `Random::RandomRanged` determinism.

### Stop on impassable cell

When the next planned cell becomes impassable (e.g., another unit
parked in the way), the locomotor's `Can_Enter_Cell` returns 0 and
the unit halts. After `path_stuck_counter` (initially 10) ticks of
unsuccessful retries, `Mission_Move` re-plans the path. The 10-tick
counter is at `FootClass + 0x64C` (per
[DRIVE_LOCOMOTION_CLASS.md](../DRIVE_LOCOMOTION_CLASS.md)).

### Bridge transitions

Bridge cells (`cell.flags & 0x100`) trigger:
- Height offset: add `DAT_008a07c4` when entering bridge
- Skip slope penalty
- Set `bridge_transition_flag` (`FootClass + 0x68B`) for one tick

### Aircraft landing

When `Mission_Move` arrives at landing target, Fly locomotor switches
to descent mode. 20-tick descent zone applies an extra deceleration
multiplier so the aircraft visibly slows before touching down.

### Magnetron lift

Vehicle under Magnetron beam has its `body_pitch` / altitude modified
externally (by `MagBeam` WaveClass), bypassing the normal locomotor.
See [magnetron-lift-cycle.md](magnetron-lift-cycle.md).

---

## TS-legacy filter

| Field / branch | TS-legacy? | Notes |
|---|---|---|
| `Speed=` (per-TechnoType) | **Live in YR** | All units. |
| `TurnRate=` / `ROT=` | **Live in YR** | Vehicles, turrets, missiles. |
| `Accelerates=` | **Live in YR (rare)** | Default `true`. |
| `MovementZone=` / `SpeedType=` | **Live in YR** | Pathfinding cell gating. |
| `Locomotor=` CLSID | **Live in YR** | All units. |
| `[General] GameSpeedBias=1.6` | **Live in YR** | Universal multiplier. |
| `[General] MissileSpeedVar` / `MissileROTVar` / `MissileSafetyAltitude` | **Live in YR** | Missile variance. |
| `[JumpjetControls]` block | **Live in YR** | Rocketeer + per-unit jumpjet overrides. |
| `[LandType]` per-`[SpeedType]` table | **Live in YR** | Terrain speed percentages. |
| `HouseClass::GetSpeedBonus` for Aircraft/Infantry/Building | **Live in YR** | Country-specific. |
| `HouseClass::GetSpeedBonus` for Vehicles | **Effectively dead — returns 1.0** | No per-house vehicle bonus in shipping YR. |
| `TunnelLocomotionClass` | **TS LEGACY** | Subterranean APC. Confirmed unused in YR per memory `feedback_no_tunnel_subterranean`. |
| `DropPodLocomotionClass` | **TS LEGACY** | Used only by superweapon drop pods; no normal unit uses it. |
| `MechLocomotionClass` | **TS LEGACY** | Cyborg / Titan Mech from TS. Not used in YR. |
| Tunnel/Mech/DropPod ROT or Speed fields | **TS LEGACY** | Parsed but dormant. |
| `Climb=` / `WobblesPerSecond=` / `WobbleDeviation=` (in JumpjetControls) | **Live in YR** | Rocketeer wobble. |

---

## Cross-references

- [game-speed-master-clock.md](game-speed-master-clock.md) — defines
  `g_CurrentFrameCounter` that movement budgets count in; defines
  GameSpeed slider
- [logic-vs-render-loop.md](logic-vs-render-loop.md) — per-entity
  movement Process runs in `LogicClass::PerTickUpdate` (unconditional);
  `Mission_Move` queues paths in the gameplay block (gated by pause)
- [animation-rate-delay.md](animation-rate-delay.md) — per-tick anim
  frame for walking infantry / track animation
- [LOCOMOTION_MATH_AND_CONSTANTS.md](../LOCOMOTION_MATH_AND_CONSTANTS.md)
  — comprehensive locomotor math reference (cited heavily here)
- [DRIVE_LOCOMOTION_CLASS.md](../DRIVE_LOCOMOTION_CLASS.md) — Drive
  locomotor full reference
- [SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../SHIP_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
  — Ship-vs-Drive deltas
- [SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md](../SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md)
  — 6 concrete Ship/Drive deltas
- [WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../WALK_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
  — Walk 7-state machine
- [FLY_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../FLY_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
  — Fly accel/descent
- [HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../HOVER_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
  — Hover continuous accel
- [JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
  — Jumpjet wobble + climb
- [ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
  — Rocket ballistic
- [TELEPORT_LOCOMOTION_DEEP_DIVE.md](../TELEPORT_LOCOMOTION_DEEP_DIVE.md)
  — Teleport piggyback + warp stages
- [TS_DORMANT_LOCOMOTORS_GHIDRA_REPORT.md](../TS_DORMANT_LOCOMOTORS_GHIDRA_REPORT.md)
  — Tunnel/Mech/DropPod dormancy
- [magnetron-lift-cycle.md](magnetron-lift-cycle.md) — vehicle lift
  overrides normal locomotor
- [emp-stun-duration.md](emp-stun-duration.md) — EMP movement freeze
- [iron-curtain-duration.md](iron-curtain-duration.md) — IC does not
  freeze movement (but does set the `field_0x2E4` damage modifier
  flag — cross-ref [weapon-rof-burst.md](weapon-rof-burst.md))
- [terrain-cost / pathfinding doc] (future) — `[LandType]` /
  `MovementZone` / `SpeedType` interactions

---

## Coverage audit

| Item | Disposition |
|---|---|
| `[TechnoType] Speed=` | Owned here |
| `[TechnoType] TurnRate=` / `ROT=` | Owned here (per-tick angular delta); detailed turret-vs-body separation deferred |
| `[TechnoType] Accelerates=` | Owned here (rare flag) |
| `[TechnoType] MovementZone=` / `SpeedType=` / `Locomotor=` | Cross-referenced to pathfinding doc; identity only owned here |
| `[General] GameSpeedBias` | Owned here (verified at `RulesClass + 0x1418` double) |
| `[General] MissileSpeedVar` / `MissileROTVar` / `MissileSafetyAltitude` | Owned here |
| `[JumpjetControls]` block (TurnRate, Speed, Climb, CruiseHeight, Acceleration, WobblesPerSecond, WobbleDeviation) | Owned here; detailed wobble math in [JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md) |
| `[LandType]` per-`[SpeedType]` percentages | Cross-referenced; full enumeration in future terrain-cost doc |
| HouseType `+0x128/+0x12C/+0x130` speed bonuses | Owned here (per-class lookup); full HouseType enumeration in future country-bonus doc |
| RulesClass `+0x7b4` (Speed fallback) | Owned here (flagged — INI key not identified) |
| RulesClass `+0x768/+0x770/+0x778/+0x780` (slope factors) | Owned here; details in [LOCOMOTION_MATH_AND_CONSTANTS.md](../LOCOMOTION_MATH_AND_CONSTANTS.md) |
| RulesClass `+0x5F0` (acceleration factor for Fly) | Owned here |
| RulesClass `+0x16B8` (speed cap for Fly) | Owned here |
| `DAT_007e48f0 = 1.5` (decel multiplier) | Owned here |
| `DAT_007e2ac8 = 1.0` (default speed bonus) | Owned here |
| `DAT_0089ea40` (9-entry slope lookup) | Cross-referenced |
| `DAT_0089f6d8 / DAT_0089f6dc / DAT_0089f688` (8-way direction tables) | Cross-referenced |
| `FootClass +0x530` (cached slope factor) | Owned here |
| `FootClass +0x5d4` (bridge-related override) | Owned here |
| `FootClass +0x1b3` (cell penalty flag) | Owned here (flagged — semantics unverified) |
| `TechnoType +0x618` (Speed field read by GetSpeed) | Owned here (verified read site; write site deferred) |
| `TechnoType +0x29E / +0x2B0` (VeteranAbilities / EliteAbilities) | Cross-referenced to veterancy doc |
| 11 locomotor CLSIDs | Cross-referenced; identities owned here |
| Drive `DriveLocomotionClass +0x4C/+0x50/+0x58` (residual/current_speed/track_index) | Cross-referenced to [DRIVE_LOCOMOTION_CLASS.md](../DRIVE_LOCOMOTION_CLASS.md) |

---

## Ghidra queries log (this iteration)

| Query | Result |
|---|---|
| Read [LOCOMOTION_MATH_AND_CONSTANTS.md](../LOCOMOTION_MATH_AND_CONSTANTS.md) lines 1–200 | Confirmed 11 locomotor CLSIDs; linear accel + 1.5× decel pattern (Drive/Walk/Ship); Fly symmetric accel; `DAT_007e48f0 = 1.5`; 9-entry slope lookup at `DAT_0089ea40`; 8-way direction tables; Drive 7-leptons-per-step budget; Fly descent zones (20/50 ticks); RulesClass slope offsets at `+0x768/+0x770/+0x778/+0x780`; RulesClass Fly fields `+0x5F0`/`+0x16B8` |
| `decompile_function 0x00717800` (TechnoTypeClass::GetSpeed) | Confirmed `Speed` field at `TechnoType + 0x618` (raw int); fallback at `RulesClass + 0x7b4` |
| `decompile_function 0x004db1a0` (FootClass::GetCurrentSpeed) | Confirmed multiplier chain: HouseSpeedBonus → TotalSpeed (virtual +0x38c) → HasWeaponAbility(0) → cell-penalty halving |
| `decompile_function 0x0050c050` (HouseClass::GetSpeedBonus) | Confirmed per-class lookup table: Aircraft (`WhatAmI=3`)→`+0x130`, Building (`WhatAmI=0x10`)→`+0x128`, Infantry (`WhatAmI=0x28`)→`+0x12C`, default `_DAT_007e2ac8 = 1.0` for vehicles/ships |
| `decompile_function 0x004dc760` (FootClass::Get_Slope_Speed_Factor) | Confirmed cached slope factor at `FootClass + 0x530`; bridge override at `+0x5d4` |
| `decompile_function 0x0055ad10` (LocomotionClass::Apparent_Speed) | Trampoline to `vtable+0x538` "GetApparentSpeed" virtual |
| `decompile_function 0x00474810` (CCINIClass::ReadSpeed) | Confirmed `INI [0..100] × 256 / 100 → internal [0..255]` for WEAPON Speed (NOT unit Speed) |
| `search_strings "Speed"` exact match | Single hit at `0x0081d9cc` |
| `get_xrefs_to 0x0081d9cc` | 4 hits: `RulesClass::ReadJumpjetControls` (the JumpjetControls Speed=14 default), `WeaponTypeClass::ReadINI` (Speed for projectile), `TechnoTypeClass::ReadINI` (per-type unit Speed) |
| `read_memory 0x00714630 len=96` | Decoded Speed parse: PUSH -1 (default), PUSH "Speed", CALL ReadInt, then clamp `[0..100]`, scale `× 256 / 100`, clamp `[0..255]` |
| `read_memory 0x0071469a len=96` | Speed conversion result stored at `+0x678` per disassembly — disagrees with GetSpeed's `+0x618` read; flagged for follow-up |
| `search_strings "GameSpeedBias"` | Single hit at `0x0083bf24` |
| `read_memory 0x00670b10 len=48` | Decoded `RulesClass::ReadGeneral` GameSpeedBias parse: stores to `RulesClass + 0x1418` as `double` (default 1.6) |
| `grep ^Speed= rulesmd.ini head -30` | Confirmed typical range (1–14); `[JumpjetControls] Speed=14`, infantry 3–4, tanks 5–8, aircraft 14 |
| `grep ^TurnRate=\|^ROT= rulesmd.ini head -10` | Confirmed `TurnRate=4` only in JumpjetControls (rest inherit); `ROT=5` common for vehicles, `ROT=40` for two units (likely missiles) |
| Read [DRIVE_LOCOMOTION_CLASS.md](../DRIVE_LOCOMOTION_CLASS.md) lines 1–100 | Confirmed Drive object layout including `current_speed @ +0x50`, `track_index @ +0x58`, `residual_ticks @ +0x4C`; FootClass interaction fields |
