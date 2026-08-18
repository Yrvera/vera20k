# HoverLocomotionClass — Ghidra Research Report

**Status:** VERIFIED (re-verified 2026-07-19). This report supersedes the 2026-04-19
version, which had two load-bearing errors: it mapped `HoverBob` to the wrong Rules
offset (`+0x16B8`) and used the wrong acceleration/brake ramp divisor (`*60` instead of
`*900`). Both are corrected below with binary citations.

**Primary Address:** `0x00514310` (Move/Process per tick), `0x00513C20` (Constructor),
`0x00515ED0` (SpeedUpdate), `0x00513D20` (HoverBob/lift height controller).
**IUnknown VTable:** `0x007EADC8`   **ILocomotion VTable:** `0x007EACFC`
**CLSID:** `{4A582742-9839-11d1-B709-00A024DDAFD1}`
**RulesClass instance pointer global:** `0x008871E0` (i.e. `Rules = *(void**)0x008871E0`).
**Global frame counter:** `0x00A8ED84`.
**Active in YR:** Yes — used by 4 live stock unit types (LCRF, ROBO, SAPC, YHVR).

---

## 0. TL;DR — the corrected model

Hover is **NOT drive-track based**. It is a continuous XY controller that steers toward
the current one-cell path waypoint (from the same `FootClass::NextCellPath[]` A* queue
Drive uses), plus a **separate vertical lift/damped-spring controller** that holds the
unit at cruise altitude `HoverHeight` and adds a small cosine bob. Per tick:

1. `SpeedUpdate` (`0x00515ED0`) computes a target speed in `[0,1]` from goal proximity +
   a straightaway boost, then ramps `SpeedCurrent` toward it at accel/brake limits.
2. `Move` (`0x00514310`) turns the body facing toward the waypoint, steps XY by
   `SpeedCurrent` along that facing, then calls the vertical controller.
3. The vertical controller (`0x00513D20`) drives the unit's `Get_Height()` altitude
   toward `HoverHeight` via a damped spring using `Gravity` and `HoverDampen`, and adds a
   `2·cos(phase)` visible bob whose period is set by `HoverBob`.

### Verified Rules offsets (all in the `RulesClass` singleton at `*0x008871E0`)

| Key | INI section | Rust type | Rules offset | Stock default | Parse fn |
|-----|-------------|-----------|--------------|---------------|----------|
| `HoverHeight` | `[General]` | **int** (leptons) | `+0x5CC` | 120 | Get_Int `0x005276D0` |
| `HoverBob` | `[General]` | **double** | `+0x5D0` | 0.04 | Get_Double `0x005283D0` |
| `HoverBoost` | `[General]` | **double** | `+0x5D8` | 1.50 (150%) | Get_Double |
| `HoverAcceleration` | `[General]` | **double** | `+0x5E0` | 0.02 | Get_Double |
| `HoverBrake` | `[General]` | **double** | `+0x5E8` | 0.03 | Get_Double |
| `HoverDampen` | `[General]` | **double** | `+0x5F0` | 0.40 (40%) | Get_Double |
| `Gravity` | `[AudioVisual]` | **int** | `+0x16B8` | code 3, **stock ini = 6** | Get_Int `0x005276D0` |

All seven verified from the parse sites in `RulesClass__ReadGeneral` and
`RulesClass__ReadAudioVisual` (see §2). The old doc's `HoverBob=+0x16B8` was wrong:
`+0x16B8` is `Gravity`, and the bob **amplitude** scales off `Gravity`, while `HoverBob`
(`+0x5D0`) sets the bob **period** (matching the INI comment "time between hover 'bobs'").

INI cross-check (`ini/rulesmd.ini`): HoverHeight=120 (L349), HoverBob=.04 (L351),
HoverBoost=150% (L352), HoverAcceleration=.02 (L353), HoverBrake=.03 (L354),
HoverDampen=40% (L350), Gravity=6 (L756). (There is also a parallel `Balloon*` key set,
L357-362, used by `BalloonHover=yes` jumpjet-style units — separate offsets, out of scope
here.)

---

## 1. Locomotor identity & class shape

HoverLocomotionClass is the COM locomotion controller (`ILocomotion` + `IUnknown`) bound
to units whose `Locomotor={4A582742-9839-11d1-B709-00A024DDAFD1}`. It reuses the FootClass
NextCellPath A* queue — it has **no** private pathfinder and consumes the path one cell at
a time (identical stepping discipline to Drive), but the per-frame motion is a continuous
`cos/sin * SpeedCurrent` integrator, **not** a drive-track curve table. Facing bytes are
NOT used as drive-track indices.

### Constructor `0x00513C20` (verified via decompile_function 0x00513C20)

```c
LocomotionClass__Constructor();              // base ctor (sets IsPowered=1, IsLockedDown=1)
param_1[6..8]  = DAT_00a8f180/184/188;       // +0x18 StepStartCoord  = SENTINEL
param_1[9..b]  = DAT_00a8f180/184/188;       // +0x24 NextCellWaypoint = SENTINEL
FUN_004c91e0(0);                             // FacingClass::Set(this+0x30, 0)
param_1[0x12]=0; param_1[0x13]=0;            // +0x48 SpeedRequest = 0.0 (double)
param_1[0x14]=0; param_1[0x15]=0;            // +0x50 SpeedCurrent = 0.0
param_1[0x16]=0; param_1[0x17]=0x3ff00000;   // +0x58 SpeedMult   = 1.0
param_1[0x18]=0; param_1[0x19]=0;            // +0x60 BobHeightOffset = 0.0
*(byte*)(this+0x68)=0; this+0x6C=0; *(byte*)(this+0x70)=0;
```

**Correction to old doc:** the two coord vectors at `+0x18` and `+0x24` are initialised to
`DAT_00a8f180` (the engine's invalid-coord sentinel), NOT literal zero as a struct field.
`read_memory 0x00a8f180` = all-zero in the static image, so the runtime sentinel value is
effectively `(0,0,0)` and equality-compared against it — a Rust `Option<Coord>` (None =
sentinel) is the faithful model. Verified via decompile 0x00513C20 + read_memory 0x00a8f180.

### Struct layout (offsets absolute from object base)

| Offset | Type | Field | Notes |
|--------|------|-------|-------|
| +0x00 | ptr | IUnknown vtable | `vtable__HoverLocomotionClass` |
| +0x04 | ptr | ILocomotion vtable | `HoverLocomotionClass__ILocomotion_vtable` |
| +0x0C | FootClass* | LinkedTo (owner) | set by Link_To_Object |
| +0x10 | bool | IsPowered | base ctor = 1 |
| +0x11 | bool | IsLockedDown | base ctor = 1 |
| +0x18 | Coord(12) | StepStartCoord | sentinel = parked; distance-from-start gates slow-approach |
| +0x24 | Coord(12) | NextCellWaypoint | sentinel = no target; else current one-cell goal |
| +0x30 | FacingClass(24) | BodyFacing (locomotor-owned) | RateTimer at this offset; steers movement dir |
| +0x48 | double | SpeedRequest | 0.0 / 0.5 / 1.0 |
| +0x50 | double | SpeedCurrent | ramped; XY step magnitude (leptons/tick) |
| +0x58 | double | SpeedMult | 1.0 normally, HoverBoost on straightaways |
| +0x60 | double | BobHeightOffset | vertical spring state (visible altitude offset) |
| +0x68 | bool(byte) | TurnDecayActive | gates the per-tick facing-decay loop in Move |
| +0x6C | int | FacingDeltaPending | signed turn-steps remaining (decremented in Move) |
| +0x70 | bool(byte) | IsFacingChangeActive | SpeedUpdate gate: snap-vs-gradual facing + speed |

---

## 2. Rules parse evidence (RulesClass__ReadGeneral / __ReadAudioVisual)

The six `[General]` keys are parsed as one contiguous block into a 5-double + 1-int
cluster. Verified via `get_assembly_context` on the parse sites (0x0066ED66..0x0066EE90):

| Store instr | Rules offset | Key string | String addr | Parser |
|-------------|--------------|------------|-------------|--------|
| `MOV [ESI+0x5cc],EAX` @0x0066EE18 | +0x5CC | `HoverHeight` | 0x83CA10 | Get_Int 0x005276D0 |
| `FSTP [ESI+0x5d0]`   @0x0066EDF9 | +0x5D0 | `HoverBob` | 0x83CA1C | Get_Double 0x005283D0 |
| `FSTP [ESI+0x5d8]`   @0x0066EE3E | +0x5D8 | `HoverBoost` | 0x83CA04 | Get_Double |
| `FSTP [ESI+0x5e0]`   @0x0066EE64 | +0x5E0 | `HoverAcceleration` | 0x83C9F0 | Get_Double |
| `FSTP [ESI+0x5e8]`   (0x0066EE88) | +0x5E8 | `HoverBrake` | 0x83C9E4 | Get_Double |
| `FSTP [ESI+0x5f0]`   @0x0066EDD2 | +0x5F0 | `HoverDampen` | 0x83CA28 | Get_Double |

`Gravity` is `[AudioVisual]` (`RulesClass__ReadAudioVisual`): `MOV [ESI+0x16b8],EAX`
@0x0066B3DE, key string `Gravity` @0x83A34C, Get_Int 0x005276D0. Default 3 is written by
`RulesClass__Constructor` (`MOV [ESI+0x16b8],0x3` @0x006674D6); stock `rulesmd.ini` L756
overrides it to **6**. (Get_Int/Get_Double named by role from the two distinct parser
signatures used in the block — int vs double return path.)

**Time-unit insight (verified constants):** the shared constant `900.0`
(`double @0x007E27F8`) is **ticks-per-minute** (15 fps x 60). It divides all three
time-valued hover keys, so `HoverBob`, `HoverAcceleration`, `HoverBrake` are **times in
minutes**, exactly as the INI comments say:
- `HoverAcceleration=0.02` -> `0.02 x 900 = 18` ticks to ramp 0->full.
- `HoverBrake=0.03`        -> `0.03 x 900 = 27` ticks to brake full->0.
- `HoverBob=0.04`          -> bob period `~ 0.04 x 900 = 36` ticks (x a 1.0/1.1 scale, below).

## 3. SpeedUpdate 0x00515ED0 — throttle model (verified via decompile_function)

`SpeedCurrent` (+0x50) is a **[0,1] throttle fraction** of the unit base `Speed=`
(leptons/tick). Per tick:

```
Is_Moving = ILoco.vtable[4]()
if (!Is_Moving || NextCellWaypoint == sentinel):
    if (!Is_Moving) return
    if (NextCellWaypoint != sentinel) return
    LinkedTo.slot_0x544(0, 1.0); FUN_005164D0(0); FUN_00514F70(1)   // halt+arrive+next
    return

owner = LinkedTo.Get_Coord()
desired = atan2( owner.Y - waypoint.Y , waypoint.X - owner.X )       // facing toward waypoint
if (!IsFacingChangeActive[+0x70]) BodyFacing.RateTimer.Set(desired)  // snap
else                              BodyFacing.UpdateFacing(desired)   // gradual

# halt while turning hard
P = ILoco.vtable[0x18]()
turning_hard = (!P) || ( abs(BodyFacing.Current() - desired) > 0x2000 )   // >45 deg
if (turning_hard && !IsFacingChangeActive): SpeedRequest[+0x48] = 0.0 ; goto ACCEL

# proximity throttle
if (StepStartCoord[+0x18] == sentinel):
    if (Distance3D(owner - waypoint) <= 0xFF): SpeedRequest = 0.5    # final approach
    else: SpeedRequest = 1.0
else:
    if (StepStartCoord != sentinel && sqrt(|owner-StepStartCoord|^2) < 0x100): SpeedRequest = 0.5
    SpeedRequest = 1.0
    if (IsFacingChangeActive): SpeedCurrent = 1.0                    # peg speed mid-turn-follow

ACCEL:
SpeedMult[+0x58] = 1.0
if (SpeedRequest > 0):
    if (NextCellPath[0](+0x5E0) != -1 && NextCellPath[0] == NextCellPath[1](+0x5E4)):
        SpeedMult = HoverBoost                                      # 2 same-dir steps queued
target = min( SpeedMult * SpeedRequest , 1.0 )                      # <-- CLAMPED to 1.0
if (SpeedCurrent < target): SpeedCurrent = min(target, SpeedCurrent + 1/(HoverAccel*900))
if (target < SpeedCurrent): SpeedCurrent = max(0.0,   SpeedCurrent - 1/(HoverBrake*900))
```

**CONFIRMED surprising detail:** `target` is clamped to 1.0 *after* the boost multiply.
So at full cruise (SpeedRequest=1.0) `HoverBoost=1.5` is clamped away and has **no** speed
effect; the boost only bites while `SpeedRequest=0.5` (approach/near-start), raising that
throttle to `min(0.75, 1.0)=0.75`. Verified via decompile 0x00515ED0 (the
`if (1.0 < dVar2) dVar2 = 1.0;` immediately after `dVar2 = SpeedMult * SpeedRequest`). The
straightaway boost is effectively a no-op at cruise in stock YR.

- Boost condition: `LinkedTo+0x5E0 (NextCellPath[0]) != -1 && == LinkedTo+0x5E4 (NextCellPath[1])`.
- `0x2000` = 45 deg (of 0x10000 full circle). `0xFF`/`0x100` ~ 1 cell (256 leptons/cell).

## 4. Move 0x00514310 — continuous XY integrator (verified via decompile_function)

Hover is a **distinct continuous controller**, not a drive-track lookup. Per-tick XY step
(only when effective speed > 0):

```
speed = ftol(LinkedTo.slot_0x538())          # effective leptons/tick = base Speed x SpeedCurrent
f     = BodyFacing.RateTimer.Current()        # short facing [0..0xFFFF]
angle = (f - 0x3FFF) * (-2*PI/65536)          # constant -9.587672516830327e-05
dx    = ftol( cos(angle) * speed )
dy    = ftol( sin(angle) * speed )
LinkedTo.Set_Coord( owner.X+dx , owner.Y+dy , owner.Z )   # slot 0x1b4
```

Arrival: before stepping, if `sqrt(dx_wp^2 + dy_wp^2) <= speed` (within one tick of the
waypoint) the unit snaps to cell-arrival: sets `LinkedTo+0x6B6=1,+0x6B7=0`, clears
`IsFacingChangeActive`, runs `FUN_005164D0` (arrival/path-continuation), and either
consumes the next path step (`FUN_00514F70`) or fully stops (SpeedRequest/SpeedCurrent=0,
waypoint->sentinel, `slot 0xF4` cleanup). Bridge handling: on a cell change with
`cell.flags & 0x100` set and `Get_Height() >= DAT_00A8F1B4`, set owner `+0x8C=1`; cleared
when the flag drops. Over deep water (`cell.type == 2`) below `ground + DAT_00A8F1C0`,
calls `slot 0xEC` (force-float-up) to keep the skirt above water.

Facing-decay tail (gated by byte `+0x68`): while a turn is in progress it advances
`BodyFacing` by `(FacingDeltaPending[+0x6C] << 8) + Current()` via `UpdateFacing`, then
steps `|FacingDeltaPending|` one toward 0; when it hits 0, clears `+0x68`.

**Struct-flag correction to old doc:** there are three trailing fields, not two — byte
`+0x68` (turn-decay-active), int `+0x6C` (signed `FacingDeltaPending`), byte `+0x70`
(`IsFacingChangeActive`, SpeedUpdate gate). All three zeroed by the constructor.

## 5. Vertical / bob controller 0x00513D20 (verified via disassemble_function)

A damped-spring altitude controller that also writes the visible cosine bob. Per tick
(names corrected: amplitude = `Gravity`, period = `HoverBob`, damping = `HoverDampen`):

```
H    = LinkedTo.Get_Height()                 # altitude above ground (leptons), slot 0x1c8
Heff = H
if (moving && GroundHeight(nextCell) > GroundHeight(curCell)):   # climbing a slope
    Heff = H - HoverHeight

# --- visible height (cosine bob on top of current offset) ---
counter = frameCounter + 2*b                 # b = owner bool (period phase nudge)
Kscale  = (b ? 1.0 : 1.1)                     # 0x7E1718=1.0 / 0x7E9258=1.1
period  = ftol( Kscale * HoverBob * 900.0 )   # ~36 (moving) / ~40 (idle) ticks at defaults
phase   = (counter % period) * 6.283185307179586 / period          # 2*PI @0x7E3CC0
visible = ftol( 2*cos(phase) + H + BobHeightOffset )
if (visible < 0): BobHeightOffset = 0; visible = 0
LinkedTo.Set_Height(visible)                 # slot 0x1cc

# --- damped-spring update of BobHeightOffset toward cruise ---
if (Heff < HoverHeight):
    if (Is_Powered):
        BobHeightOffset += ((2*HoverHeight - Heff) / HoverHeight) * Gravity
    if (Heff < HoverHeight/4):
        BobHeightOffset += Gravity / 3        # integer /3 (0x55555556 magic-div)
BobHeightOffset = (BobHeightOffset - Gravity) * HoverDampen
```

Net: the offset is pulled up when the unit sits below `HoverHeight` (proportional lift,
strongest near the ground, extra `Gravity/3` kick below `HoverHeight/4`), pulled down by
`-Gravity` each tick, and multiplied by `HoverDampen` (0.4) so the system settles smoothly
toward cruise altitude. On top of that steady altitude, `2*cos(phase)` gives the visible
float wobble with period `~HoverBob*900` ticks. When unpowered (EMP/low power) the lift
term is skipped (`LocomotionClass::Is_Powered` @0x0055A930 gate) so the unit sinks and the
bob flattens, but damping still runs. `Gravity`, `HoverBob`, `HoverDampen`, `HoverHeight`
are all live every tick for every hover unit — none TS-legacy or flag-gated.

## 6. Facing model (answers Q3)

Hover keeps a **locomotor-owned body FacingClass** at `+0x30` (RateTimer form) that always
steers toward the *current one-cell waypoint*, not toward the final goal and not via a
drive-track curve. Two modes: `RateTimer.Set()` (snap) when already aligned / not
mid-turn, and `FacingClass::UpdateFacing()` (rate-limited gradual) while turning. The
movement vector in section 4 reads this facing directly. Forward motion stalls
(`SpeedRequest=0`) while the required turn exceeds `0x2000` (45 deg). The body ROT
(`ROT=5` for all four live hover units) is the rotation-rate source for the gradual
`UpdateFacing` (HIGH-inferred: ROT is the only body-rotation rate input; the exact
FacingClass rate-field write was not traced here — MEDIUM on the precise binding). Turret
facing, where present, is the standard TechnoClass turret and is unaffected by the
locomotor. Differs from Drive (which indexes drive-track curves by facing) but converges on
the same observable: smooth body rotation at ROT toward the movement heading.

## 7. Live hover units (answers Q4 — verified against ini/rulesmd.ini)

Hover CLSID `{4A582742-9839-11d1-B709-00A024DDAFD1}`. Four **active** stock units, all
`SpeedType=Hover`, `MovementZone=Amphibious`(-Destroyer), `ROT=5`:

| ID | Name= | Section (rulesmd.ini) | MovementZone | ROT |
|----|-------|-----------------------|--------------|-----|
| LCRF | Landing Craft | [LCRF] L7012, Locomotor L7056 | Amphibious | 5 |
| ROBO | Robot Tank | [ROBO] L7417, Locomotor L7453 | AmphibiousDestroyer | 5 |
| SAPC | Armored Transport | [SAPC] L7881, Locomotor L7933 | Amphibious | 5 |
| YHVR | Hover Transport Yuri | [YHVR] L8870, Locomotor L8918 | Amphibious | 5 |

**Corrections to old doc:** (1) YHVR is "Hover Transport Yuri" (Yuri amphibious transport,
UIName Name:SAPC), **not** a Boomer submarine. (2) YURIPR (Yuri Prime) does **not** use
Hover — its active `Locomotor={4A582744-...}` (L5288, `SpeedType=Amphibious`) is a
different locomotor; the `;Locomotor={4A582742-...}` at L5283 is commented-out and dormant.
So exactly four live hover units, not five.

## 8. Rust Implementation Handoff

**Current Rust surface being corrected:** `src/sim/movement/locomotor.rs:107-109`
approximates Hover as a flat `0.65 x Drive-speed`. None of the six `[General]` hover keys
(nor `Gravity`) are parsed. Replace the flat multiplier with the throttle + vertical model.

### 8.1 Parse these keys (INI -> fixed-point)
- `[General] HoverHeight` (int leptons, default 120)
- `[General] HoverBob` (double minutes, default 0.04)
- `[General] HoverBoost` (double, default 1.50)
- `[General] HoverAcceleration` (double minutes, default 0.02)
- `[General] HoverBrake` (double minutes, default 0.03)
- `[General] HoverDampen` (double, default 0.40)
- `[AudioVisual] Gravity` (int, default 3 -> stock 6) — bob amplitude; also used by ballistics.

### 8.2 Horizontal throttle (replaces the 0.65x stub)
- State: `speed_request in {0.0,0.5,1.0}`, `speed_current in [0,1]`, `speed_mult in {1.0, HoverBoost}`.
- Set `speed_request`: `0.0` while turning >45 deg (0x2000) and not mid-turn-follow; `0.5`
  within ~1 cell (<=255 lept) of the waypoint or <=255 lept of step-start; else `1.0`.
- `speed_mult = HoverBoost` iff `NextCellPath[0] == NextCellPath[1]` and `!= -1`, else `1.0`.
- `target = min(speed_mult * speed_request, 1.0)` — **clamp to 1.0** (boost is near-no-op at
  cruise; keep the clamp for exactness).
- Ramp: `+1/(HoverAccel*900)` toward target when below; `-1/(HoverBrake*900)` when above
  (clamp to target / to 0). `900` = ticks/minute; use it literally.
- Per-tick XY leptons = `speed_current x base_speed_leptons_per_tick` along the body facing:
  `dx=round(cos(angle)*speed)`, `dy=round(sin(angle)*speed)`,
  `angle=(facing16 - 0x3FFF) * (-2*PI/65536)`.
- Arrival to next path cell when `dist_to_waypoint <= speed`.

### 8.3 Vertical controller (new; visible float + bob)
Implement section 5 verbatim with fixed-point. `bob_offset` is a damped spring:
```
if Heff < HoverHeight:
    if powered: bob_offset += ((2*HoverHeight - Heff)/HoverHeight) * Gravity
    if Heff < HoverHeight/4: bob_offset += Gravity/3        # integer division
bob_offset = (bob_offset - Gravity) * HoverDampen
visible_height = max(0, round(2*cos(phase) + H + bob_offset_prev))
```
`Heff = H - HoverHeight` only while moving uphill (next-cell ground > current-cell ground),
else `Heff = H`. `phase = (frame_counter % period) * 2*PI / period`,
`period = round(Kscale * HoverBob * 900)`, `Kscale = 1.0` (moving) or `1.1` (idle).

### 8.4 Acceptance values
(defaults: HoverHeight=120, HoverBob=.04, HoverBoost=1.5, HoverAccel=.02, HoverBrake=.03,
HoverDampen=.4, Gravity=6)
- Accel ramp: `1/(0.02*900) = 1/18 ~ 0.05556` throttle/tick -> 18 ticks 0->full.
- Brake ramp: `1/(0.03*900) = 1/27 ~ 0.03704` throttle/tick -> 27 ticks full->0.
- Bob period: `round(1.0*0.04*900) = 36` ticks moving; `round(1.1*0.04*900)=40` idle.
- Cruise altitude target ~ `HoverHeight = 120` leptons above ground (spring equilibrium).
- Straightaway boost target at cruise = `min(1.5*1.0,1.0)=1.0` (no change); at approach =
  `min(1.5*0.5,1.0)=0.75`.
- All four live units (LCRF/ROBO/SAPC/YHVR) share ROT=5, MovementZone=Amphibious(-Destroyer).

### 8.5 Remaining uncertainty
- Exact `Set_Speed` plumbing between locomotor `speed_current` and the FootClass speed
  getter (slot 0x538): observable is `leptons/tick = speed_current x base Speed`; the
  intermediate write path not fully traced (LOW risk — output is what matters).
- Per-unit bool `b` selecting bob `Kscale` 1.0/1.1 and the `+2` counter nudge: cosmetic-only
  (bob phase), identity not confirmed (LOW risk).
- FacingClass rate field vs ROT binding (section 6): MEDIUM; body turn is visibly at ROT.
- `DAT_00A8F1B4` (bridge altitude threshold) / `DAT_00A8F1C0` (water-float threshold) are
  BSS runtime-init (static read = 0); documented in the bridge-hover reports.

---
**Verified:** 2026-07-19 via Ghidra MCP (decompile/disassemble/read_memory/get_assembly_context)
on gamemd.exe @ image base 0x00400000. Supersedes the 2026-04-19 revision.
