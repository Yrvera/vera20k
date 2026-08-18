# TechnoClass Body Rocking System — Ghidra Research Report

**Primary address:** `TechnoClass::RockingUpdate @ 0x0070B570` (vtable slot offset **0x41C** = slot 263)
**Impulse dispatcher:** `TechnoClass::ApplyRocker @ 0x0070B280` (vtable slot offset **0x3D8** = slot 246)
**Per-tick caller:** `TechnoClass::AI_Update @ 0x006F9E50`, call site at `0x006FA236`
**Confidence:** HIGH — full disassembly of RockingUpdate (953 bytes), ApplyRocker, both impulse paths (`Apply_area_damage`, `WarheadTypeClass::Detonate`, `FootClass::ReceiveEMP`), all referenced floating-point constants read from binary memory, and Rules-side INI defaults verified.
**Active in YR:** Yes — runs every tick on every TechnoClass instance (vehicle / infantry / ship / aircraft / building) whose `vtable[0x298]` returns true. Constants are not gated behind any TS-era flag. Active in every standard YR skirmish.

---

## 1. Overview

Body rocking is a per-tick spring-damper that drives two float angles on each TechnoClass
instance — `AngleRotatedSideways` (roll) and `AngleRotatedForwards` (pitch) — toward zero
through a per-frame angular velocity pair (`RockingSidewaysPerFrame`, `RockingForwardsPerFrame`).
External events (weapon impacts via `Apply_area_damage` and `WarheadTypeClass::Detonate`'s
DirectRocker branch; EMP wobble via `FootClass::ReceiveEMP`; sinking via the `IsSinking` branch
inside `RockingUpdate` itself) push impulses into the velocity fields; `RockingUpdate` then
integrates angle, clamps angle to ±π/4, and decays velocity toward zero each frame using a
small base rate (`0.002 rad/tick`) optionally scaled by `Rules.FallBackCoefficient`.

The angles feed the VXL renderer's body-matrix composition path (see
`VXL_DRAW_MATRIX_GHIDRA_REPORT.md §13–§15`) — when either angle exceeds the 0.005 rad
deadband, the renderer takes the "tilt path" and applies `Rx(roll) × Ry(pitch)` plus
shear translation offsets to the body matrix.

Three orthogonal paths exist inside `RockingUpdate`:

1. **Sinking path** (`IsSinking != 0`) — pitches the body forward/back via a +0.01/-0.01
   rad/tick toggle keyed off the rate timer; ignores velocity fields entirely; returns early.
2. **Ship-rocking path** (`field_0x425 != 0`) — `angle += velocity` each tick, then a
   one-sided clamp to ±π/4; **no damping** of velocity. Used by `FootClass::ReceiveEMP`
   to drive continuous wobble.
3. **Normal spring-damper path** — the usual physics: integrate velocity → angle,
   detect saturation at ±π/4 (zero the velocity at the cap), then decay velocity toward
   zero by ±0.002 × FallBackCoefficient per tick. Wide-amplitude (|angle| > π) on either
   axis triggers `vtable[0x16C]` callback to spawn an effect / sound at `Rules+0xFA8`.

---

## 2. TechnoClass field layout (rocking-relevant)

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x14 | byte (bit 2) | State flag (semantic TBD — possibly "on-ground-and-engaged-with-terrain", co-occurs with crushing per §3.3.5) | Checked in the **spring-damper forwards path** (NOT ship-rocking) at `0x0070BA05-0x0070BA0B` to choose alternate forwards threshold (π/10 instead of ±π/4) |
| +0x2A8 | int | **IsMoving** | When 0, dampening rate is `±0.002`; when nonzero, rate is `±FallBackCoefficient × 0.002` (decay scales with motion state) |
| +0x328 | float | **AngleRotatedSideways** | Current sideways tilt (roll), radians |
| +0x32C | float | **AngleRotatedForwards** | Current forward tilt (pitch), radians |
| +0x330 | float | **RockingSidewaysPerFrame** | Angular velocity for sideways, radians/tick |
| +0x334 | float | **RockingForwardsPerFrame** | Angular velocity for forwards, radians/tick |
| +0x388 | (RateTimer) | Rate timer used by sinking path | `RateTimer__Current(&local_4)` reads this; sinking extracts bits `((timer>>12)+1)>>1 & 7` |
| +0x3CD | byte | **IsSinking** (sinking-branch gate) | Nonzero ⇒ enter sinking path (forwards oscillation toggle), return early before ship-rock / spring-damper. Zero ⇒ fall through. Does NOT globally disable rocking — gates only the sinking branch entry. (Doc previously called this "IsRocking" — that name is misleading; Ghidra labels it `IsSinking`.) |
| +0x425 | byte | **IsShipRocking** | Nonzero ⇒ take ship-rocking path. Set to 1 in `FootClass::ReceiveEMP`. Likely cleared by external code when EMP wears off (clear-site not located in this investigation) |
| +0x6B5 | byte | **"Currently crushing" flag** — set to 1 in `DriveLocomotionClass::Process_Drive_Track` when vehicle drives onto a crushable building (conditions: locomotor +0x64 != 0 AND `CellClass::FindFirstBuilding` returns non-null AND vehicle TypeClass+0x5B4 == 12). Co-checked with +0x14 bit 2 to swap forwards threshold to ±π/10 in the spring-damper. NOT a deploy/garrison flag (prior labeling was wrong — Process_Drive_Track also writes `RockingForwardsPerFrame = -0.05f` at the same site if TypeClass+0xD2B is set, producing the visible "tilt-back-while-crushing" rock). Note: doc previously also listed this as `this[1].field_0x195` — same byte, two C++ access notations; the byte is at TechnoClass+0x6B5. |

Type-class (vtable[0x84]) reads:

| TypeClass offset | Purpose |
|------------------|---------|
| +0xA0 | int — **`Strength` (max HP)** (verified 2026-05-11 against `TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md:194`). Used in §3.4 wide-amplitude callback as the damage value passed to `TechnoClass::ReceiveDamage` — so the body-tipover kill deals damage equal to the unit's max HP, which combined with `force_kill = 1` is functionally always lethal. |
| +0x370 | double — **`Weight` (default 2.0, retail range 0.5–5)** (verified 2026-05-11 against `TECHNOTYPECLASS_BASE_ADDENDUM.md:256`). Used in `ApplyRocker` as the divisor in `force_scaled = (0.04 − dist × 2.5e-5) × force / Weight`. Heavier units rock proportionally less per equivalent impulse — an Apocalypse (Weight=5) rocks ~2.5× less than a Grizzly (Weight=2) under the same force. Player-visible across mixed-armor compositions. |
| +0xB0 | pointer (or interpreted as int via Ghidra). The byte at `*(TypeClass+0xB0)` is the master rocking-enable gate used by both `vtable[0x298]` and `ApplyRocker`: body runs only when this byte is **zero**. Per the audit, the byte appears to act as "is-immune-to-rocking" (0 = not immune = rock enabled). The exact INI key behind this byte is still unidentified — see §10 Q10. |
| +0xD6A | byte — gates the IsShipRocking path early-return after the integrate step. **If zero, the ship-rocking unit does not clamp or saturate — just integrates and returns.** (Likely the per-unit "can ship-rock" capability bit.) |
| +0xCD5 | byte — gates a separate AI-Update branch unrelated to rocking |

---

## 3. The RockingUpdate algorithm (verified from disassembly at 0x0070B570)

### 3.1 Entry-condition fan-out

```
DL = (AngleRotatedSideways > +2e-5 ? 1 : 0)                ; "side angle above + deadband?"
byte_ESPa = (AngleRotatedSideways < -2e-5 ? 1 : 0)          ; "side angle below - deadband?"

if (this->IsSinking == 0):                                   ; +0x3CD == 0 → skip sinking
    goto BRANCH_NORMAL

; sinking-branch body (entered when IsSinking != 0):
fVar1 = |AngleRotatedForwards|
if (fVar1 >= 0.7853982):                                    ; π/4 saturation — return without update
    return
timer_bits = ((RateTimer >> 12) + 1) >> 1 & 7                ; 3-bit window from rate timer
if (timer_bits == 0 || (5 < timer_bits && timer_bits < 8)):  ; timer_bits ∈ {0, 6, 7}
    AngleRotatedForwards -= 0.01                             ; sinking oscillation: subtract
else:                                                          ; timer_bits ∈ {1, 2, 3, 4, 5}
    AngleRotatedForwards += 0.01                             ; sinking oscillation: add
return                                                        ; sinking ALWAYS returns here

BRANCH_NORMAL:
if (this->field_0x425 != 0):                                 ; IsShipRocking
    goto BRANCH_SHIPROCK
else:
    goto BRANCH_SPRINGDAMPER                                  ; ← the usual physics path
```

**The +0x3CD gate is the IsSinking gate, NOT a global rocking switch.** Disassembly at
`0x0070B5AE` (`CMP byte ptr [ESI + 0x3cd],BL`) followed by `JZ 0x0070B63D`: when `+0x3CD`
is zero the function jumps to the IsShipRocking check at 0x70B63D, bypassing only the
sinking arm. When `+0x3CD` is nonzero, the sinking arm runs and returns (the sinking arm
never falls through to ship-rock / spring-damper). All other writes to AngleRotated* and
RockingPerFrame fields still process every tick when `+0x3CD == 0`. Ghidra labels this
field `IsSinking`, consistent with §1.

**Sinking-bits formula precedence.** The bit-extraction is `((RateTimer >> 12) + 1) >> 1
& 7` (left-to-right with `+` bound tighter than `>>` per C precedence — see asm at
`0x0070B5F8-0x0070B5FE`: `SHR EAX,0xC ; INC EAX ; SHR EAX,1 ; AND EAX,7`). An earlier
revision wrote `(RateTimer >> 12 + 1) >> 1 & 7` which parses as `>> 13` — that form is
wrong; do not use it.

### 3.2 Ship-rocking path (BRANCH_SHIPROCK)

```
AngleRotatedForwards += RockingForwardsPerFrame
AngleRotatedSideways += RockingSidewaysPerFrame

TypeClass = vtable[0x84]()
if (TypeClass+0xD6A == 0):                                   ; type doesn't support ship-rock
    return                                                    ; ← angles already updated; no clamp

; One-sided clamps to lower bound only (-π/4):
AngleRotatedForwards = max(AngleRotatedForwards, -0.7853982)
AngleRotatedSideways = max(AngleRotatedSideways, -0.7853982)

; Upper-bound clamp on sideways only:
if (AngleRotatedSideways >= +0.7853982):
    AngleRotatedSideways = +0.7853982
    return
return                                                        ; no velocity damping in this path
```

Two important quirks of ship-rocking path:
- **There is NO upper-bound clamp on forwards in this path.** Only sideways gets a +π/4
  cap. The forwards angle can drift above +π/4 if the impulse velocity is large enough.
  This is the only legitimate way the wide-amplitude callback at the end of RockingUpdate
  (see §3.4 below) can fire under normal play.
- **Velocity is never zeroed or decayed in this path.** Once set by `FootClass::ReceiveEMP`,
  the velocity persists tick after tick until externally cleared. The EMP wobble runs at
  constant velocity for the duration of the EMP lock.

### 3.3 Spring-damper path (BRANCH_SPRINGDAMPER) — the "normal" physics

This is the path almost all ground vehicles take in normal play. It runs twice — once for
sideways, once for forwards — with structurally identical logic. Below is the sideways
half; the forwards half is the same with a few extra branches for the "deployed building"
threshold swap (see §3.3.5).

#### 3.3.1 Out-of-range flags

```
bVar13 = (AngleRotatedSideways > +π/2)                         ; angle above +90°
bVar6  = (AngleRotatedSideways < -π/2)                         ; angle below -90°
bVar14 = bVar6 || bVar13                                         ; "out of normal range"
```

Normal play never reaches |angle| > π/4 (the saturation cap), let alone π/2. These flags
exist to detect catastrophic out-of-envelope state and switch dampening signs accordingly.
Likely a TS-legacy safety net.

#### 3.3.2 Zero-velocity short-circuit

```
if (RockingSidewaysPerFrame == 0.0):
    AngleRotatedSideways = 0.0                                  ; snap to zero
    goto deadband_check                                          ; skip integrate + saturate, but still run velocity-dampen
```

If velocity is identically zero, the angle is force-zeroed and the integrate/saturate steps
are skipped. **Note: this is a strict `== 0.0` compare, not a small-velocity check.** Any
non-zero velocity bypasses the snap.

#### 3.3.3 Integrate + saturation clamp

```
new_angle = RockingSidewaysPerFrame + AngleRotatedSideways
prev_angle = AngleRotatedSideways
AngleRotatedSideways = new_angle                                ; provisional store

; Positive saturation at +π/4 — only fires when crossing the boundary from below
;  while NOT moving AND in normal range:
if (new_angle > +π/4 && IsMoving == 0 && !bVar14 && prev_angle < +π/4):
    AngleRotatedSideways = +0.7853982                            ; literal +π/4 (constant 0x3F490FDB)
    RockingSidewaysPerFrame = 0.0                                ; zero the velocity
    goto deadband_check

; Negative saturation at -π/4 — symmetric:
if (new_angle < -π/4 && IsMoving == 0 && !bVar14 && prev_angle > -π/4):
    AngleRotatedSideways = -0.7853982                            ; literal -π/4 (constant 0xBF490FDB)
    RockingSidewaysPerFrame = 0.0
```

**Gating subtlety:** the saturation only fires when **`IsMoving == 0`** (vehicle stationary)
AND `bVar14 == 0` (angle in normal range). A *moving* vehicle's angle is allowed to drift
past ±π/4 without being clamped. This is likely because the rocking impulse from a weapon
hit on a stopped tank is meant to "stick" at the cap, while a moving vehicle's body
oscillation should pass through (and be damped down naturally).

#### 3.3.4 Velocity dampening (the "spring" part)

This is a 16-way decision tree on three booleans: `sign(velocity)`, `IsMoving`,
and `out_of_range (bVar6, bVar13)`. The base rate is `±0.002 rad/tick` (constant at
`0x007F4E70`, float). The Rules-side `FallBackCoefficient` (Rules+0x18B8, double, default
**0.1** per `rulesmd.ini:621`) is multiplied in when `IsMoving != 0`.

```
if (velocity > 0):
    if (IsMoving == 0):
        if (in_range):    velocity -= 0.002                       ; basic decay toward zero
        else:             velocity += 0.002                       ; out-of-range — push back inward
    else:  ; IsMoving != 0
        if (in_range):    velocity -= FallBackCoefficient × 0.002 ; scaled decay (with FallBack=0.1 → -0.0002/tick)
        else:             velocity += 0.002                       ; out-of-range push back

if (velocity < 0):  ; symmetric — same structure with signs flipped
    if (IsMoving == 0):
        if (in_range):    velocity += 0.002                       ; basic decay toward zero
        else:             velocity -= 0.002                       ; out-of-range push back
    else:
        if (in_range):    velocity += FallBackCoefficient × 0.002 ; scaled decay
        else:             velocity -= 0.002

if (velocity == 0 case):
    if (in_range):                                                 ; smooth-decay branch
        velocity += 0.005 (positive path) or -= 0.005 (negative)   ; uses _DAT_007f4e68 (double)
```

**Important detail: there is an alternative `0.005` (double) decay rate** (`_DAT_007f4e68`)
used in a specific sub-branch of the dampening tree, alongside the `0.002` (float) rate
(`_LAB_007f4e70`). I traced it firing in the "velocity is on the same side of zero as the
angle, and out_of_range==1" combination (`LAB_0070b868`, `LAB_0070b90D`) — the engine
applies a faster snap-back when the rocking is *fighting itself* (velocity pushing further
from zero while angle is already wildly out). For a moving vehicle in normal play this
never fires; for an EMP'd unit in ship-rock mode this path is unreachable because
ship-rock doesn't run dampening at all.

#### 3.3.5 Forwards-half threshold swap for deployed buildings

The forwards half of the dampening (post-integrate, post-saturation) reads `+0x14 bit 2`
and the `+0x6B5` byte to select a tighter ±π/10 saturation:

```
threshold = +π/4                                                    ; default
if ((this+0x14 >> 2) & 1 != 0 && this+0x6B5 != 0):
    threshold = π/10 = 0.31416                                       ; constant 0x3EA0D97C
```

This is a single check in the binary (asm `0x0070BA05-0x0070BA19`). A prior version of
this doc presented it as two separate `if` blocks (one with `this[+0x195]`, one with
`+0x6B5`); those are the same byte accessed two ways (`this[1].field_0x195` evaluates to
`this + 0x520 + 0x195 = this + 0x6B5`).

**The π/10 saturation activates during a vehicle crushing a building, not deployment.**
The `+0x6B5` byte is set to 1 in `DriveLocomotionClass::Process_Drive_Track` when a
crushable vehicle drives onto a building (see §2 row for +0x6B5). At the same site,
`RockingForwardsPerFrame` is also written to `-0.05f` (if TypeClass+0xD2B is set), producing
the visible backward tilt as the vehicle climbs the building's debris. The tighter ±π/10
cap on the forwards half ensures the tilt stays in a small range while the crush plays out.

The sideways half does not use this swap — it always uses ±π/4. The semantics of `+0x14
bit 2` are not yet fully traced; an earlier "Alive" label is a guess.

#### 3.3.6 Deadband snap-to-zero

After each axis's dampening, the engine checks whether the angle has re-entered the
deadband:

```
if (DL_flag_was_set && (AngleRotatedSideways <= +2e-5)):           ; was-positive AND now near zero
    RockingSidewaysPerFrame = 0
    AngleRotatedSideways = 0
if (sign_flag_set && (AngleRotatedSideways <= -2e-5)):
    RockingSidewaysPerFrame = 0
    AngleRotatedSideways = 0
```

The deadband is **±2e-5 rad** (stored as a double at `0x007EC0B0` (+2e-5) and
`0x007F4E78` (-2e-5)). When the angle decays into the deadband, both the angle and the
velocity are zeroed in a single step — clean termination, no jitter.

### 3.4 Wide-amplitude self-destruct callback (end of function)

```
if (AngleRotatedSideways > +π || AngleRotatedSideways < -π ||
    AngleRotatedForwards > +π || AngleRotatedForwards < -π):
    TypeClass = vtable[0x84]()
    local_4 = TypeClass[+0xA0]                                       ; (likely max-HP-class scalar — passed as damage)
    vtable[0x16C](&local_4, 0, Rules+0xFA8, 0, 1, 0, 0)              ; TechnoClass::ReceiveDamage — self-damage call
```

**`vtable[0x16C]` is `TechnoClass::ReceiveDamage @ 0x00701900`** (verified 2026-05-11
audit). The 7-arg signature maps to ReceiveDamage's parameters:

| Arg | Value | Meaning |
|-----|-------|---------|
| arg1 | `&local_4` (=TypeClass+0xA0) | pointer to **damage** value |
| arg2 | 0 | source house |
| arg3 | `Rules+0xFA8` | **Warhead** reference (NOT an AnimType as earlier guessed) |
| arg4 | 0 | source object |
| arg5 | 1 | `force_kill` flag — set, bypasses armor multiplier and veterancy adjustments |
| arg6/7 | 0, 0 | trailing nulls |

The call **damages the unit on itself** with `damage = TypeClass+0xA0` (a scalar from the
TechnoTypeClass that, in this context, is sized to be at least max-HP-class — the
force-kill flag makes the hit effectively lethal). The AI_Update site immediately checks
the "still alive" byte at `[this + 0x90]` after RockingUpdate (asm 0x006FA23C–0x006FA244)
and bails if zero, confirming the framework expects the callback to potentially kill.

**Trigger reachability in retail YR:**

|angle| > π is outside the normal envelope (saturation caps are ±π/4 = 45°). Three paths
can reach it:

1. **EMP'd unit in ship-rocking mode whose `TypeClass+0xD6A == 0`.** Ship-rocking path has
   no damping; with TypeClass+0xD6A unset, no clamps either. The angle integrates linearly
   from the 0.1–0.25 rad/tick EMP velocity, reaching π in ~13–31 ticks (~1–2 seconds at 15
   FPS). Would self-destruct any EMP'd unit lacking this gate. Strong evidence retail tunes
   TypeClass+0xD6A on every EMP-able type to avoid this. (See §10 Q7.)
2. **Moving vehicle (IsMoving != 0) hit with stacked rocker impulses.** Spring-damper
   saturation at ±π/4 only fires when `IsMoving == 0`. A moving vehicle hit hard skips that
   clamp; with FallBack=0.1 (slow decay), single-impulse peak is ~π/2 before out-of-range
   damping reverses it. Several near-simultaneous V3-class impacts could plausibly stack
   past π.
3. **External writes to the angle fields.** Rare/non-standard.

In typical YR skirmish play the callback essentially never fires — the constants and
per-type flags are tuned so legitimate impulses stay within ±π/4. It exists as a safety
net for catastrophic state. A faithful Rust port should still implement it; an
implementation that omits it would diverge for edge cases (sustained EMP on an
unprotected type, stacked impulses, modded warheads with extreme force).

`Rules+0xFA8` is the `[CombatDamage] C4Warhead=` INI key (verified 2026-05-11 — parsed
at 0x0066C31F via "C4Warhead" string xref to 0x0083B1D4, stored at 0x0066C346). Adjacent
slots: `Rules+0xFAC = CrushWarhead`, `Rules+0xFB0 = V3Warhead`.

**In retail rulesmd.ini line 818:** `C4Warhead=Super` with a Westwood designer comment
explicitly stating *"This warhead is used throughout the code to mean 'Absolute damage'"*.
So the engine intentionally reuses `Super` as the universal "no-armor-saves-you" kill
warhead — for SEAL/Tanya/Engineer C4, for the body-rocking tip-over self-destruct, and
likely a few other catastrophic-state code paths. Implementations of this engine must
load `[CombatDamage] C4Warhead=` into a Ruleset slot and use it for the rocking
self-destruct (not introduce a separate Warhead).

---

## 4. Impulse sources (every code path that writes to +0x330 / +0x334)

Exhaustive search of the binary for stores to `+0x330` (sideways velocity) and `+0x334`
(forwards velocity) on a TechnoClass pointer:

| Source function | Address | Writes | Notes |
|-----------------|---------|--------|-------|
| `TechnoClass::ApplyRocker` (FUN_0070B280) | 0x0070B280 | Both (0x70B561, 0x70B555) | The "real" impulse path — called via vtable[0x3D8]. Computes direction-aware force and writes velocity components. |
| `RockingUpdate` itself | 0x0070B570 | Many | Decay / zero / clamp — not an impulse, but writes velocity for completeness. |
| `FootClass::ReceiveEMP` | 0x004DECF0 | Both (0x4DECF0, 0x4DED13, 0x4DED45) | EMP "cosmetic wobble" — random velocity, sets IsShipRocking=1. |
| `BulletTypeClass::ReadINI_Part2` | 0x00428682 | Both | **False positive** — writes BulletType+0x330/+0x334, not TechnoClass. Skip. |
| `RulesClass::ReadGeneral` | 0x0066D530 region | Both | **False positive** — writes RulesClass fields at the same offset. Skip. |
| `FUN_00678850` | 0x00678D52/0x00678D60 | Both | **False positive** — different struct, same byte offsets. Skip. |

Real impulse sites (after filtering): **ApplyRocker** and **ReceiveEMP**. ApplyRocker is
itself called from at least two places:

### 4.1 `Apply_area_damage @ 0x00489280` (area-damage rocking — "Rocker=yes")

Called by `WarheadTypeClass::Detonate` when no special-effect flag matches. Iterates the
3×3 cell grid centered on the impact and for each TechnoClass in those cells:

```
counter = (some-per-target-accumulator)                          ; built earlier in the function
force = counter × 0.01                                            ; _g_ImpassableSpeedThreshold_0_01 (yes, same 0.01 constant the sinking path uses)
if (force >= 0x007E3CC8):
    force = 4.0                                                   ; saturate at 4.0
if (Warhead[+0x14E] != 0 && force > 0x007E5138):                  ; Warhead.Rocker=yes
    for target in 3x3_grid:
        target.ApplyRocker(direction_from_impact, force)
```

The **Rocker** flag is at `Warhead+0x14E`. (Distinct from DirectRocker — see §4.2.)
Searching `rulesmd.ini` confirms a `Rocker=yes` per-warhead boolean (default `no`) used by
SonicWarhead, V3 warheads, KirovHE, Parasite, etc. See §6 for the full set.

### 4.2 `WarheadTypeClass::Detonate @ 0x004690B0` (direct-hit rocking — "DirectRocker=yes")

When a bullet hits a single target directly, before falling through to area damage:

```
if (Warhead[+0x14F] != 0 && target != NULL && target.WhatAmI() == 1):
    ; conditions: Warhead.DirectRocker=yes AND target exists AND target is a vehicle
    ; (WhatAmI()==1 is UnitClass; infantry=2, aircraft=3, building=0xF)
    if (target+0x14 bit 1 set && target.vtable[0x160]() == 0):    ; some "not in transition" gate
        force = (BulletClass.RockerScale × BulletClass.Damage >> 8)
              × Rules.DirectRockingCoefficient                     ; float multiplier
              / 100.0                                              ; _DAT_0081AEF8 = 100.0 (double)
        if (force >= 4.0):                                          ; _DAT_007E3CC8 = 4.0 (double)
            force = 4.0                                             ; saturate at 4.0 (same as Apply_area_damage)
        ; Compute offset = normalized(target_pos - bullet_target_pos) × 10.0  (_g_BridgeDiag_NonBridge_10_0)
        target.ApplyRocker(target_pos + offset, force)
        target[+0xAA] = bullet                                       ; cross-link bullet → target
        bullet[+0x2A8] = target                                      ; bullet → IsMoving back-link (?!)
```

**Normalization divisor is 100.0, not 256.** A previous revision speculated `_DAT_0081AEF8`
was `256` (Q8.8 unit normalization); the actual stored bytes at 0x0081AEF8 are
`00 00 00 00 00 00 59 40` = 0x4059000000000000 = 100.0 (double). Implementations dividing
by 256 will produce force impulses ~2.56× smaller than retail.

DirectRocker fires **only on vehicles**, not infantry or buildings. The bulletshot's
direction-from-bullet-to-target informs the rocker push direction.

**Verified retail defaults (rulesmd.ini lines 620–621):**
- `DirectRockingCoefficient = 1.5` (multiplier on the bullet-derived force)
- `FallBackCoefficient = 0.1` (with designer comment: "Used to reduce the amount the
  tank falls back between pushes. Smaller number = less fallback")

### 4.3 `FootClass::ReceiveEMP @ 0x004DECF0` (EMP cosmetic wobble)

When a non-building unit takes an EMP hit:

```
this[+0x425] = 1                                                   ; set IsShipRocking
vtable[0x274](3)                                                   ; some state-set
vtable[0x3A0]()                                                    ; some notification
FootClass__EMPPassengers(emp_dur)
if (WhatAmI() != 0xF && !MapEditor):                               ; non-building, in normal play
    r1 = Random(0, 0x7FFFFFFE)
    sideways_vel = r1 × _PTR_007E3570 × _DAT_007E9280 + _DAT_007E3860
    if (Random(0, 1) == 0): sideways_vel = -sideways_vel            ; random sign
    this[+0x330] = sideways_vel
    r2 = Random(0, 0x7FFFFFFE)
    forwards_vel = r2 × _PTR_007E3570 × _DAT_007E3860               ; (no second multiplier, positive only)
    this[+0x334] = forwards_vel
```

Sideways gets random sign; forwards is positive-only. The IsShipRocking flag stays set
until presumably cleared in the EMP-tick-down code (not located in this investigation;
likely in `RadiationEMP` or the EMP timer-expiry path).

### 4.4 `ApplyRocker` (FUN_0070B280) — the impulse-receiver internals

```
fn ApplyRocker(this, source_pos, force, no_dampen_flag):
    TypeClass = vtable[0x84]()
    if (TypeClass+0xB0 == 0): return                                  ; pointer null → cannot rock
    if (*(TypeClass+0xB0) != 0): return                               ; ← byte at *(TypeClass+0xB0) MUST be 0
                                                                       ;   for body to run; semantics is
                                                                       ;   "byte==0 → enable rocking"

    delta = source_pos - this.Location                                ; XYZ vector from impact to target
    timer = RateTimer__Current()
    angle = (timer - 0x3FFF) × _LAB_007E2810                          ; map rate timer to angle (≈ -π/32768)
    cos_a, sin_a = cos(angle), sin(angle)
    distance = length(delta)
    force_scaled = (_LAB_007F4E54 - distance × _LAB_007F4E58) × force / TypeClass[+0x370]
                                                                       ; (0.04 - distance × 2.5e-5) × force / Weight
                                                                       ; Weight (TypeClass+0x370) = INI "Weight=" (double, default 2.0)

    if (|distance| < 2e-5 || force_scaled < 0.01):                    ; 0x007F4E34 = 0.01f (too-weak gate)
        return                                                         ; too close or too weak — no rocking
    if (force_scaled > 0.05):                                          ; 0x007F4E50 = 0.05f
        force_scaled = 0.05                                            ; saturate the per-axis impulse

    ; Normalize delta horizontally (XY only, then push Z to zero)
    horizontal = sqrt(dx² + dy²)
    if (horizontal != 0):
        dx /= horizontal; dy /= horizontal; dz_offset = 0 / horizontal

    ; Rotate the delta by the timer-derived angle to add jitter
    fVar8 = dy × cos_a + dx × sin_a                                    ; forward component
    fVar9 = dx × cos_a - dy × sin_a                                    ; sideways component
    fVar14 = sqrt(fVar9² + (dz_offset × sin_a)² + (dz_offset × cos_a)²)

    ; Sign correction based on cross-product magnitude
    if (|sin_a × fVar8 - cos_a × fVar14 - dx| > 2e-4                   ; 0x007EC0A8 ≈ 2e-4 double
        || |cos_a × fVar8 + sin_a × fVar14 - dy| > 2e-4):
        fVar14 = -fVar14

    fVar8 = fVar8 × force_scaled
    if (no_dampen_flag == 0):
        fVar8 = fVar8 × 0.5                                            ; _DAT_007E5168 = 0.5f (verified)

    this[+0x334] = fVar8                                               ; RockingForwardsPerFrame
    this[+0x330] = -(fVar14 × force_scaled)                            ; RockingSidewaysPerFrame (negated)
```

**Critical: the second-level gate at `*(TypeClass+0xB0) == 0` is binary-verified.** Asm
at `0x0070B2B4: CMP byte ptr [EAX],0x0` followed by `0x0070B2B7: JNZ exit`. The body runs
only when the byte is **zero**, not non-zero. A previous version of this doc had the gate
written as "if (TypeClass+0xB0->byte_0 == 0): return" — that reading was inverted; the
binary returns early when the byte is NON-zero. Semantically the byte appears to act as
"is-immune-to-rocking" (0 = not immune = rock enabled), but the exact type-side meaning
needs separate verification. The same gate is used by `vtable[0x298]` (the AI_Update
rocking dispatch predicate, FUN_006F9E10) — confirming it's the data-driven master
"can-this-unit-rock" check.

The "cast to int" is a Ghidra reading artifact — these are floats stored via FSTP. The
function writes the velocity components such that **forwards velocity ∝ (delta-Y rotated
by timer-angle) × force, sideways velocity ∝ negative cross-component × force**. The
rate-timer-derived angle adds a small per-tick jitter so identical impacts in identical
spots don't produce identical rocking patterns (which would be visually robotic across an
army).

The `force_scaled` saturation at **0.05** means the per-axis velocity injected per impulse
is bounded — even a maximum-force hit produces a starting velocity of at most 0.05 rad/tick.
At the default 0.1 FallBackCoefficient that decays at 0.0002 rad/tick², the rocking
naturally damps to zero in roughly `0.05 / 0.0002 = 250 ticks = ~16.6 seconds at 15 fps`.
This is consistent with the long, slow rock you see on a Rhino tank that took a V3 hit.

---

## 5. Per-tick dispatch (TechnoClass::AI_Update)

Single call site at `0x006FA236` inside `TechnoClass::AI_Update`:

```
006FA224: EDX = this->vtable
006FA228: AL = vtable[0x298](this)                                    ; gate
006FA22E: TEST AL, AL
006FA230: JZ skip_rocking                                              ; if false, skip RockingUpdate
006FA236: vtable[0x41C](this)                                          ; RockingUpdate
006FA23C: if (this[+0x90] == 0): return from AI_Update                 ; "still alive after rocking?"
```

Two observations:

1. **The vtable[0x298] gate is data-driven, NOT per-class polymorphic** (verified
   2026-05-11). Slot 166 (offset 0x298 / 4). All six TechnoClass subclasses
   (TechnoClass / UnitClass / InfantryClass / AircraftClass / BuildingClass / VesselClass)
   share the same implementation at `0x006F9E10`:

   ```c
   TypeClass = this->vtable[0x84]();
   if (TypeClass+0xB0 == 0):  return 0;            // pointer null
   return (*(byte *)(TypeClass+0xB0) == 0);         // byte at *(+0xB0) is zero?
   ```

   So "should this unit rock?" is **data-driven via TypeClass+0xB0** — a pointer-to-byte
   where `byte == 0` means "this type rocks". This is the SAME gate used by ApplyRocker
   (§4.4), confirming the byte is the master rocking enable. Vehicles/Vessels presumably
   have the byte = 0; infantry/aircraft presumably have it != 0 (or have +0xB0 = null).
   The exact mapping needs a separate sweep of TechnoTypeClass struct.

2. **The +0x90 byte check** after RockingUpdate is the "early-out-if-dead" check. The
   wide-amplitude callback at the end of RockingUpdate can theoretically destroy the unit
   (via `vtable[0x16C]` → spawn anim → trigger something), and AI_Update bails to avoid
   running on a destroyed `this`.

RockingUpdate is called **once per AI tick** per TechnoClass. The AI tick rate is the
standard game tick (~15 FPS / 66.6 ms per tick in normal-speed YR).

---

## 6. INI keys

### 6.1 Per-warhead (WarheadTypeClass)

| INI key | Field offset | Default | Type | Purpose |
|---------|--------------|---------|------|---------|
| `Rocker` | Warhead+0x14E | `no` | bool | Enables area-damage rocking via `Apply_area_damage`. |
| `DirectRocker` | Warhead+0x14F | `no` | bool | Enables direct-hit rocking via `WarheadTypeClass::Detonate`. Vehicle-only. |

`Rocker=yes` warheads in retail (from `rulesmd.ini`): SonicWarhead, V3WH, V3EWH, BlimpHE,
BlimpHEEffect, KTSTLEXP (Kirov), MaverickHE, ARTYHE, SCHOPWH (Schop), IonWH (Ion Cannon),
TRexWH, RPG, Parasite (Terror Drone), ParasiteDog, ParasitePlus (SquidGrab), Smashing
(Brute), ORCAHE, GrandCannonWH. **All other warheads default to Rocker=no.**

DirectRocker is rarer — used on Apocalypse-class direct kinetic hits and a few specials.

### 6.2 Per-bullet (BulletTypeClass)

| INI key | Field offset | Default | Type | Purpose |
|---------|--------------|---------|------|---------|
| `RockerScale` | Bullet+0x150 | `0x100` (= 1.0 in Q8.8) | Q8.8 fixed | Per-bullet multiplier on DirectRocker force. Almost always defaulted in retail. |

### 6.3 Per-unit (TechnoTypeClass)

| INI key | Default | Type | Purpose |
|---------|---------|------|---------|
| `IsTilter` | `yes` | bool | Slope-tilt enable (NOT the rocking gate — slope and rocking are independent). Set `no` on infantry-like units that should never tilt. |

The vtable[0x298] gate that selects whether to run RockingUpdate appears to be hard-coded
per class type (not INI-driven). The `IsTilter` flag on TechnoTypeClass controls only the
slope-tilt half of the renderer's body-matrix path, not the rocking spring-damper. (Per
VOXEL_SLOPE_TILT_SYSTEM.md.)

### 6.4 Per-rules (RulesClass, [AudioVisual] section)

| INI key | Field offset | Default in rulesmd.ini | Type | Purpose |
|---------|--------------|------------------------|------|---------|
| `DirectRockingCoefficient` | Rules+0x18B4 | **1.5** | **float** | Multiplier on DirectRocker bullet-derived force. (Stored and loaded as `float` — asm at 0x66B89D/0x66B8B6 uses `FLD/FSTP float ptr`. The temporary double promotion on stack at the INI helper is calling-convention only.) |
| `FallBackCoefficient` | Rules+0x18B8 | **0.1** | **float** | Multiplier on dampening rate when `IsMoving != 0`. Smaller = slower decay = longer rocking persistence. Designer comment: "Used to reduce the amount the tank falls back between pushes. Smaller number = less fallback." Stored/loaded as `float` (asm uses `FLD float ptr [ECX+0x18b8]` in RockingUpdate). |

---

## 7. Constants reference (every magic number read from binary memory)

| Address | Value | Type | Used as |
|---------|-------|------|---------|
| 0x007E1748 | 0.0 | float | Comparison sentinel ("velocity is exactly zero") and "sign of forwards angle" probe in sinking path |
| 0x007E2810 | ≈ -π/32768 (-9.587e-5) | **double** | ApplyRocker rate-timer → jitter-angle multiplier. `angle = (timer - 0x3FFF) × this` |
| 0x007E3808 | 0.01 | **double** | Sinking-path tilt rate (rad/tick) — also `_g_ImpassableSpeedThreshold_0_01` reused as a factor in Apply_area_damage |
| 0x007E3CC8 | **4.0** | **double** | Force saturation: if `force >= 4.0`, clamp to 4.0. Used in both Detonate's DirectRocker and Apply_area_damage's Rocker paths. (Verified 2026-05-11: bytes `00 00 00 00 00 00 10 40`.) |
| 0x007E5138 | **0.3** | **double** | Apply_area_damage Rocker-loop force floor. The 3×3 cell loop fires only when `force > 0.3` (after the 4.0 saturation). (Verified 2026-05-11: bytes `33 33 33 33 33 33 D3 3F`.) |
| 0x007E5168 | **0.5** | **float** | ApplyRocker secondary forwards dampener — when `no_dampen_flag == 0`, `RockingForwardsPerFrame *= 0.5`. Asymmetric (sideways component is NOT halved). (Verified 2026-05-11: bytes `00 00 00 3F`.) |
| 0x007E897C | +π/2 = +1.5708 | float | "Out of normal range" upper bound (sideways/forwards) |
| 0x007E8980 | -π/2 = -1.5708 | float | "Out of normal range" lower bound |
| 0x007EC0A8 | ≈ 2e-4 | **double** | ApplyRocker sign-correction threshold (cross-product magnitude check) |
| 0x007EC0B0 | +2.0e-5 | **double** | Deadband (positive). Angles inside ±2e-5 are snapped to 0. |
| 0x007EF8F8 | +π/4 = +0.7853982 | float | Saturation cap (positive). Inline immediate `0x3F490FDB`. |
| 0x007F4E34 | 0.01 | float | ApplyRocker "force too weak" gate — if `force_scaled < 0.01`, return without rocking |
| 0x007F4E50 | 0.05 | float | ApplyRocker per-axis impulse cap. If `force_scaled > 0.05`, clamp to 0.05 (inline immediate `0x3D4CCCCD`) |
| 0x007F4E54 | 0.04 | float | ApplyRocker distance-attenuation offset: `(0.04 - distance × 2.5e-5) × force / TypeClass+0x370` |
| 0x007F4E58 | ≈ 2.5e-5 | float | ApplyRocker distance-attenuation slope (paired with 0x7F4E54) |
| 0x007F4E5C | -π = -3.1415927 | float | Wide-amplitude callback trigger (negative side) |
| 0x007F4E60 | +π = +3.1415927 | float | Wide-amplitude callback trigger (positive side) |
| 0x007F4E64 | π/10 = 0.31416 | float | Forwards-path saturation cap when vehicle-is-crushing (replaces ±π/4 — see §3.3.5) |
| 0x007F4E68 | ≈ 0.005 (actual bytes `…7AE158000000`) | **double** | Alt dampening rate — "snap-back" when velocity is fighting itself out of range. **Bit-precise note:** stored value is 0x3F747AE158000000 ≈ 0.005000000186, NOT the canonical 0x3F747AE147AE147B exactly-0.005. Treat as "≈ 0.005"; bit-perfect reproduction would need to replicate the exact bits. |
| 0x007F4E70 | 0.002 | float | Base dampening rate (per tick) |
| 0x007F4E74 | -π/4 = -0.7853982 | float | Saturation cap (negative). Inline immediate `0xBF490FDB`. |
| 0x007F4E78 | -2.0e-5 | **double** | Deadband (negative) |
| 0x0081AEF8 | **100.0** | **double** | DirectRocker force normalization divisor (NOT 256 — earlier guess was wrong). Formula: `force = … / 100.0`. (Verified 2026-05-11: bytes `00 00 00 00 00 00 59 40`.) |
| Rules+0xFA8 | (likely Warhead, not AnimType — see §10) | int (pointer/index) | Argument passed to vtable[0x16C] in wide-amplitude callback. Also passed to Apply_area_damage in the cell-corruption code path at the end of Apply_area_damage, suggesting it's a Warhead reference. |

---

## 8. Integration with the VXL renderer

Per `VXL_DRAW_MATRIX_GHIDRA_REPORT.md §13–§15`:
- The body matrix takes the "simple path" when **both** angles are within ±0.005 rad (the
  renderer-side deadband). Below threshold, only the facing rotation is applied — no tilt
  math at all.
- Above threshold (either axis): the "tilt path" applies `Rx(roll) × Ry(pitch)` plus
  translation shears (`combined_Z`, `partial_X/Y`, `remainder_X/Y`) so the rotated body
  visually stays grounded.

The renderer's 0.005 rad threshold (`0x007E44E8` double per the VXL doc) is **larger** than
the RockingUpdate spring-damper's 2e-5 rad deadband. So there's a band between `2e-5` and
`0.005` where the spring-damper considers the rocking "still active" (won't snap to zero)
but the renderer treats the unit as flat (simple path). This produces a near-end-of-rock
"settling tick" where velocity is still nonzero but the rendered body has already returned
to upright. **This is correct gamemd behavior and the player won't notice — the band is
narrow and the velocity is decaying anyway.**

The lighting pipeline does NOT incorporate the rocking angles. The Blinn-Phong LUT is
pre-computed per facing only (see `VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md §6`). Rocking is
purely a geometric body transform — lighting "moves with" the body because the per-voxel
normals get transformed through the body matrix during rasterization, but the LUT itself
is unaware of the tilt. A unit that has rocked far enough to have visibly tilted faces
gets correct shading because the body matrix carries the rotation; the LUT just maps
normal-index → brightness independent of orientation.

---

## 9. Current Rust implementation status

**Not implemented in any form.** Verified by grep on the repo:
- No symbols `rocking`, `RockingUpdate`, `AngleRotatedSideways`, `AngleRotatedForwards`,
  `RockingSidewaysPerFrame`, `RockingForwardsPerFrame` anywhere in `src/`.
- No INI parsing for `Rocker`, `DirectRocker`, `RockerScale`, `DirectRockingCoefficient`,
  `FallBackCoefficient`.
- The voxel renderer's [`src/render/vxl_raster.rs`](../ra2-rust-game/src/render/vxl_raster.rs)
  applies world rotation from facing only (line 311). No body-rocking angles to feed it.
- The voxel renderer's per-facing Blinn-Phong LUT at
  [`src/render/vxl_normals.rs`](../ra2-rust-game/src/render/vxl_normals.rs) takes only
  facing — consistent with gamemd, no change needed once rocking is added.

Effort to reach parity:
- Add 4 float fields to whatever entity-component holds TechnoClass-equivalent state.
- Add 2 INI fields on TechnoType (or equivalent).
- Add 2 INI fields on Warhead, 1 on Bullet, 2 on Rules.
- Per-tick: integrate angle += velocity; saturate at ±π/4 (with deployed-building ±π/10
  override on forwards); dampen velocity by ±0.002 × FallBackCoefficient (or just ±0.002
  when IsMoving==0); snap to zero in deadband.
- Impulse: when a Rocker/DirectRocker warhead detonates, compute force per §4.1/§4.2,
  call `apply_rocker(source_pos, force)` on each eligible target.
- Renderer: feed angles into the body-matrix tilt path with the 0.005 rad threshold.

The slope-tilt SLERP transition and renderer-side tilt path are **separate gaps** — see
`VXL_DRAW_MATRIX_GHIDRA_REPORT.md` and the disparity scan at
`docs/gap-scans/2026-05-11-disparity-scan-voxel.md`. Rocking is the impulse source for
the tilt half of that path; slope-tilt is the terrain source. They share the same
output (final body matrix) but originate from different code.

---

## 10. Open questions

### Resolved (verified 2026-05-11 audit pass)

1. ~~**`_DAT_007E5168` value (ApplyRocker secondary dampener).**~~ **RESOLVED: 0.5f**.
   Used at `0x0070B54F: FMUL float ptr [0x007E5168]` to halve forwards velocity when
   `no_dampen_flag == 0`. Sideways velocity is NOT halved.
2. ~~**`_DAT_007E3CC8` value (force saturation threshold).**~~ **RESOLVED: 4.0 double**.
   Used in both Detonate (DirectRocker path, 0x004692xx) and Apply_area_damage (Rocker
   path) as the "force >= this → clamp to 4.0" threshold. The threshold and clamp value
   are the same constant (4.0).
3. ~~**`_DAT_0081AEF8` value (DirectRocker normalization).**~~ **RESOLVED: 100.0 double**.
   Earlier guess of 256 (Q8.8) was wrong. Implementations dividing by 256 will produce
   force impulses ~2.56× smaller than retail.
4. ~~**`_DAT_007E5138` value (Apply_area_damage force floor).**~~ **RESOLVED: 0.3 double**.
   The Rocker-yes 3×3-cell loop fires only when `force > 0.3` (after 4.0 saturation).
5. ~~**`vtable[0x298]` per-class implementations.**~~ **RESOLVED: NO per-class
   polymorphism.** All six subclass vtables share a single implementation at `0x006F9E10`
   that returns `(*(byte *)(TypeClass+0xB0) == 0)`. Whether a unit rocks is data-driven
   via TypeClass+0xB0, not virtual override. See §5 observation 1.
9. ~~**Confirmation that IsRocking (+0x3CD) gates only the sinking branch.**~~ **RESOLVED:
   CONFIRMED**. Asm `0x0070B5AE CMP byte [ESI+0x3cd], BL ; JZ 0x0070B63D` jumps to the
   IsShipRocking check at 0x70B63D (bypassing only the sinking arm). Field correctly
   renamed to IsSinking in §2.

### Still open

6. ~~**`vtable[0x16C]` wide-amplitude callback.**~~ **RESOLVED 2026-05-11:**
   `vtable[0x16C] = TechnoClass::ReceiveDamage @ 0x00701900`. The callback is NOT an
   animation — it self-damages the unit with `damage = TypeClass+0xA0` and
   `force_kill = 1`, killing it when its body tips past ±180°. `Rules+0xFA8` is the
   **Warhead** used for this self-damage hit (not an AnimType). See §3.4 for the
   reachability analysis. The exact INI key that populates Rules+0xFA8 is still
   unidentified — a focused xref pass on Rules+0xFA8 in RulesClass parser would
   resolve this.
7. **TypeClass+0xD6A "supports ship-rock" byte.** Default and INI mapping unknown. Per
   the ship-rocking path: if this is 0, the angle integrates but is never clamped.
   The fact that the ship-rocking path also skips the wide-amplitude callback means
   this byte is the gate for the entire IsShipRocking feature class.
8. **Where IsShipRocking (+0x425) gets cleared.** `FootClass::ReceiveEMP` sets it to 1
   but the EMP-tick-expiry / EMP-timer-resolution path that clears it back to 0 wasn't
   traced in this investigation. Likely in `RadiationEMP_GHIDRA_REPORT.md`'s EMP timer
   logic.
10. **Semantic of TypeClass+0xB0 (rocking master enable).** The pointer-to-byte is the
    data-driven master "can this type rock" check used by both `vtable[0x298]` and
    `ApplyRocker`. Body runs when the byte is **zero**. Likely points to an "Immune list"
    or similar struct, where byte=0 means "not immune to rocking". Needs separate sweep
    of TechnoTypeClass +0xB0 region to identify the actual semantic.
11. **Semantic of TechnoClass+0x14 bit 2.** Co-checked with `+0x6B5` in the spring-damper
    forwards path to select π/10 threshold. Earlier "Alive" label is a guess; given +0x6B5
    is set during crush, +0x14 bit 2 may be an "on-ground/engaged-with-terrain" or
    "deploy/garrison" flag. Needs separate verification.
12. **Bit-precise value at 0x007F4E68.** Stored bytes encode 0x3F747AE158000000 ≈
    0.005000000186, not the canonical IEEE-754 double for exactly 0.005
    (0x3F747AE147AE147B). Could be a long-double → double truncation artifact at compile
    time. For functional purposes 0.005 is fine; for bit-perfect replay this needs the
    exact stored bits.

---

## Sources

**Ghidra MCP decompilations:**
- `TechnoClass::RockingUpdate @ 0x0070B570` — full decompile + disassembly
- `TechnoClass::ApplyRocker (FUN_0070B280) @ 0x0070B280` — full decompile
- `WarheadTypeClass::Detonate @ 0x004690B0` — full decompile
- `Apply_area_damage @ 0x00489280` — full decompile
- `FootClass::ReceiveEMP @ 0x004DECF0` — full decompile
- `TechnoClass::AI_Update @ 0x006F9E50` — disassembly excerpt around 0x6FA224-0x6FA244
- `WarheadTypeClass::ReadINI @ 0x0075D5xx` region — flag offsets verified
- `RulesClass::ReadAudioVisual @ 0x006691E0` — coefficient INI loads verified (string xref)

**Binary memory reads (raw float / double values):**
- 0x007E1748, 0x007E3808, 0x007E897C, 0x007E8980, 0x007EC0B0, 0x007EF8F8, 0x007F4E5C..0x007F4E78

**Doc cross-references:**
- `VXL_DRAW_MATRIX_GHIDRA_REPORT.md` §13–§15 (rocking-to-render integration, threshold)
- `VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md` §6 (lighting interaction)
- `NAVAL_SYSTEM_RESEARCH.md` (field offset confirmation)
- `SUBMARINE_AND_SINKING_GHIDRA_REPORT.md` (sinking branch interaction)
- `TECHNOCLASS_STRUCT_LAYOUT.md`, `TECHNOCLASS_VTABLE_COMPLETE.md` (vtable + field layout
  cross-check; **note vtable docs had RockingUpdate at slot 246 / 0x3D8 which is actually
  ApplyRocker; real RockingUpdate is slot 263 / 0x41C — flag for correction**)
- `BULLETCLASS_LIFECYCLE_AND_TIER1_VERIFICATIONS_GHIDRA_REPORT.md` §1.1 (DirectRocker
  formula partial mention)

**INI files:**
- `ini/rulesmd.ini:620-621` (DirectRockingCoefficient=1.5, FallBackCoefficient=0.1)
- `ini/rulesmd.ini` (Rocker=yes warhead enumeration)
