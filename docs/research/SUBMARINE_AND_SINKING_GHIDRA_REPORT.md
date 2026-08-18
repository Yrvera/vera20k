# Submarine Dive/Surface FSM and Ship Sinking — Ghidra Report

Research for naval-plan **Phase 5b (Submarine system)** and **Phase 5c (Ship sinking)**.
Source: live Ghidra MCP decompilation of `gamemd.exe`, cross-referenced with
`ini/rulesmd.ini`. Cross-check against `NAVAL_SYSTEM_RESEARCH.md`,
`NAVAL_IMPLEMENTATION_PLAN.md`, and `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md`.

---

## 0. Overview

**Confidence:** HIGH for the Naval-flag + Weight-threshold death branch; HIGH for
the per-tick sinking Z-descent loop; HIGH for all TechnoTypeClass field offsets
(Weight, Naval, Underwater, Organic, SinkingSound, VoiceSinking, NavalTargeting,
CloakingSpeed) because they were located directly in `TechnoTypeClass::ReadINI`.
HIGH for SplashList selection logic in `Warhead__SelectExplosionAnim`.
MEDIUM for a few secondary details (e.g., exact semantics of
TechnoClass +0x8C OnBridge state used in the dive gate) — called out inline.
LOW for the Dolphin's exact place in the class hierarchy (see §2.7).

**Scope:** this doc specifically covers:
- **A**: Submarine dive/surface finite-state machine.
- **B**: Ship sinking wreck/animation sequence triggered when a naval unit dies
  on water at Weight >= `ShipSinkingWeight`.

**Big picture (spoiler):** There is no dedicated submarine FSM. Submarines
reuse the generic cloak pipeline in `TechnoClass::CloakingTick`
(see `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md`). "Dove" is identical to
"Cloaked"; there is no separate submerged state, no separate depth timer,
no separate weapon gate. Submarines get 9-frame transitions with
`CloakingSpeed=1` and `CloakingStages=9` like any other cloakable unit.

**TS-legacy check:** Submarines and ship sinking are both LIVE in normal YR
skirmish (all stock naval maps trigger them). The bridge/Z-height paths in
the sinking loop are not gated behind SpecialFlags. No TS ghost path found on
the sinking code — it runs unconditionally when `IsSinking` is set.
Cloaking mechanics are TS-legacy-safe too (see
CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md §10).

---

## Part A — Submarine Dive/Surface FSM

### A.1 There is no dedicated submarine state machine

Finding: no function in `gamemd.exe` named `Dive`, `Surface`, `Submerge`,
`Submarine_*`, or similar. The string `Underwater` appears **once**, at
`0x00843848`, and has exactly one xref from `TechnoTypeClass::ReadINI`
(at 0x714d74) where it is parsed into a boolean flag at `TypeClass+0xD69`.
No code anywhere reads +0xD69 and switches on a "submerged" state.

The submerge behavior comes purely from the generic `CloakState` pipeline
at `TechnoClass+0x220`. All four YR submarines (`[SUB]`, `[BSUB]`,
`[DLPH]`, `[SQD]`) set `Cloakable=yes` (or are treated as such) and
use the 9-stage cloak animation.

```
[Uncloaked=0] --StartCloaking--> [Cloaking=1] --(visual=5)--> [Cloaked=2]
     ^                                                             |
     |                                                             | ShouldUncloak
     +----(visual=0)-- [Uncloaking=3] <--StartUncloaking-----------+
```

See `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md` §1 for the full state
machine, decompilation source, and offsets.

### A.2 Dive state — where it lives (HIGH)

**TechnoClass (instance) fields driving "submerged":**

| Offset | Size | Field | Notes |
|---|---|---|---|
| +0x220 | 4 | CloakState | 0=Surfaced, 1=Diving, 2=Submerged, 3=Surfacing |
| +0x224 | 4 | CloakProgress | 0..CloakingStages counter |
| +0x228 | 1 | CloakDirty | set when progress changes |
| +0x22C | 12 | CloakStepTimer | CDTimerClass between steps |
| +0x238 | 4 | CloakingSpeed | runtime copy of TypeClass+0x310 |
| +0x23C | 4 | CloakStepDelta | +1 diving, -1 surfacing |
| +0x240 | 12 | ReCloakDelayTimer | cooldown before re-dive after forced surface |
| +0x3D2 | 1 | HasStealthAbility | runtime cloakable flag |

There is **no `IsSubmerged` boolean**. "Submerged" = `CloakState == 2`.
"Currently transitioning" = `CloakState == 1` or `3`. "Surfaced" =
`CloakState == 0`.

### A.3 Dive triggers (HIGH)

Auto-dive is driven by `TechnoClass::CloakingTick` (0x006FB740), per-tick from
`TechnoClass::AI`. The gate for starting a dive is `CanAutoCloak()`
(0x006FBDC0). A submarine will start diving when ALL of the following hold
(from the decompiled function):

1. `IsCloakable()` returns true — for submarines this is true by construction:
   UnitClass constructor copies `TechnoTypeClass+0xCD0 (Cloakable=yes)` into
   TechnoClass+0x3D2 (`HasStealthAbility`), then `FootClass::IsCloakable()`
   returns true unless `CloakStop=yes` AND the unit is moving.
2. Cell is visible to the owner house (or HasStealthAbility is true — so
   submarines can re-cloak even outside LOS, since HasStealthAbility is set).
3. `CloakState != 2` — not already fully submerged.
4. `ReCloakDelayTimer` has expired (i.e., `CloakDelay` minutes have passed
   since the last surface). Default `CloakDelay=0.02` minutes → ~18 ticks at
   15 fps ≈ 1.2 s.
5. No enemy actively targeting via a cell/targeting callback
   (`vtable+0x3ac(param_1[0xad])`).
6. Mission allows it. Concretely the RTTI/mission gate
   `(WhatAmI == 6 || param_1[0x89] == 0)` — for a unit this means the
   mission/state flag at +0x224 must be clear. Empirically subs dive during
   Guard, AreaGuard, Attack, Move.
7. Mission-state timer at +0x248 (copied from +0x92) expired.
8. MindControl master check clears (not mind-controlled with target).
9. `vtable+0x1c8()` (current Z elevation vs water plane) returns < 1.

Trigger: **passive auto-cloak each tick** — there is no user command to
"dive." Subs attempt to dive every tick they satisfy eligibility.
In particular no idle timer is used beyond `ReCloakDelayTimer`. Stock YR
submarines have `CloakStop=no` so they can dive while moving.

**Damage-gated dive:** If the sub is below `RulesClass.ConditionYellow`
(RulesClass+0x1708), there is only a **4% random chance per tick** to start
cloaking (CloakingTick state 0 path). Healthy subs dive at the first tick
they're eligible.

### A.4 Surface triggers (HIGH)

Per-tick from CloakingTick state 2 (fully submerged), the sub calls
`ShouldUncloak()` (0x006FBC90). Decompilation shows it returns true when:

- Unit lacks cloak ability (does not apply to intrinsic submarines).
- Unit is firing (`IsFiring`, vtable+0x37C), teleporting, or warping
  (unless veteran/elite with CLOAK ability — not relevant here).
- Current cell is not visible to the owning house (subs at fog edge uncloak
  briefly — this reproduces the retail quirk where subs can flash at map
  edges).

Additional decloak causes (outside `ShouldUncloak`):

- **Firing a weapon with `DecloakToFire=yes` (default).** Checked in
  `TechnoClass::GetFireError` (0x006FC0B0). Retail SubTorpedo,
  BoomerTorpedo, SonicZap, and SquidGrab all set `DecloakToFire=no`, so
  they fire submerged. The **Boomer's `CruiseLauncher`** (secondary) does
  NOT set `DecloakToFire=no`, so firing cruise missiles forces the Boomer
  to surface before launch. This is the single intra-unit exception.
- **Sensor detection.** When a unit with `SensorsSight > 0` covers the
  submerged unit's cell, the sensor-add function calls `DoUncloak()` on
  all cloaked occupants. See `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md`
  §4.1. Detector units in retail: DEST, AEGIS, DLPH, BSUB, SQD, SUB, and
  infantry like the Allied Spy Satellite and Soviet Psychic Sensor
  (building-level). All six underwater-capable ships have Sensors=yes.
- **Damage below ConditionYellow.** 10% chance per tick in CloakingTick
  state 1 to abort the dive and restart surfacing.
- **Entering/exiting transports.** On Unlimbo, `HasStealthAbility && !+0x3D5`
  sets `CloakState = 2` directly (skip animation) — if a cloakable sub is
  spawned from a naval yard, it appears instantly submerged without the
  9-frame fade-in.

### A.5 Cloak interaction / visibility pipeline (HIGH)

Submarines use the **same visibility pipeline** as Mirage Tanks and other
cloakables — there is no separate underwater rendering path in the sim.

- **Sensor counters:** per-cell `short SensorCount[MaxHouses]` at
  CellClass+0x7C. When this goes > 0 for a house, cloaked enemies in that
  cell receive `DoUncloak()`.
- **Visual state:** `TechnoClass::GetVisualState` (0x00703860) returns
  0..5 based on `CloakProgress / CloakingStages`. Renderer blends alpha
  per state (see CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md §7).
- **Allied shimmer:** owned/allied cloaked units get a pulsing shimmer via
  `ModifyCloakDrawFlags` (0x0070ED80) so the local player can track their
  own subs. Enemy subs at visual state 5 are not drawn.

Submarines do NOT get a separate per-unit "is-drawn-underwater" routine;
the water tinting you see in retail comes from:
1. The voxel blitter's 50/50 alpha blend at visual state 2-3.
2. The fact that the Wake animation stops once the sub cell check path
   triggers the CloakProgress → visual-state=5 → skip-draw branch.

No alternate palette is applied — the `Is_Surfacing` vtable entry at
slot 38 (`0x4B4C80`) is about **transport cargo surfacing**, NOT submarine
surfacing (see `NAVAL_SYSTEM_RESEARCH.md` §14 — verified in
`NAVAL_SYSTEM_RESEARCH.md` table of Ship vtable).

### A.6 Sight radius when dove vs surfaced

No separate field. Submerged subs use the same `Sight=` value as surfaced
subs. From `ini/rulesmd.ini`:

| Unit | Sight= | SensorsSight= |
|---|---|---|
| SUB | 4 | 7 |
| BSUB | 8 | 8 |
| DLPH | 4 | 8 |
| SQD | 5 | 8 |
| DEST | — (see INI) | yes |

Sensor coverage (the detector radius that forces enemy cloaks to drop)
is separate from standard sight — controlled by `SensorsSight=` at
`TechnoTypeClass+0x5F0`. Both Sight and SensorsSight are applied whether
the unit is surfaced or submerged. There is no "when dove the sub sees
less" system.

### A.7 Combat when dove (HIGH)

Subs can fire while cloaked if their weapon sets `DecloakToFire=no`.
The fire-gate decompilation (`TechnoClass::GetFireError`, 0x006FC0B0,
see cloaking doc §3) returns `FIRE_MUST_DECLOAK (9)` when:
- `WeaponTypeClass+0x133 (DecloakToFire) == true` AND `CloakState != 0`.
- Exception: vehicles in transition states (1 or 3) CAN fire;
  only fully cloaked (state 2) vehicles hit the gate.

Retail behavior per unit:

| Unit | Primary Weapon | Primary DecloakToFire | Can fire submerged? |
|---|---|---|---|
| SUB | SubTorpedo | no | yes |
| BSUB | BoomerTorpedo | no | yes |
| BSUB | CruiseLauncher (secondary) | default YES | **no — forces surface** |
| DLPH | SonicZap | no | yes |
| SQD | SquidGrab | no | yes |

**Torpedo projectile (`[Torpedo]`):** `Image=SUBT`, `AG=yes`, `Level=yes`,
`ROT=12`. No `AntiUnderwater=` flag — but the commented-out `AntiUnderwater=yes`
lines in SubTorpedo/BoomerTorpedo in rulesmd.ini (via `Verses=` vs
armor types) are what implement anti-sub capability. There is no separate
"torpedo requires surface" engine rule; torpedoes are ordinary projectiles
that happen to travel at ROT=12 with no shadow.

### A.8 Detection / "depth charges" (HIGH)

RA2/YR does not have explicit depth-charge mechanics — it has the
**sensor system**. Three distinct detection systems exist
(see cloaking doc §4):

1. **SensorsSight** (unit-level): `CellClass+0x7C[house]` counter.
   When > 0 for a house, enemy cloaked units in that cell get forcibly
   uncloaked via `DoUncloak()`.
2. **SensorArray** (building-level): same cell counter, same mechanism.
   Examples: Psychic Sensor (building), Spy Satellite.
3. **PsychicDetectionRadius**: takes priority over SensorArray ranges.

The detector units for submarines in stock YR are:
- **Destroyer (DEST)** — `Sensors=yes` (specifically to hunt subs).
- **Dolphin (DLPH)** — `Sensors=yes, SensorsSight=8`.
- **Aegis Cruiser (AEGIS)** — (check rulesmd for specifics).
- **Submarine-to-submarine** — all four underwater-capable units have
  `Sensors=yes`, so submarines can see other submarines.

**No `DetectSubmarines=` or `Sonar=` INI key exists in gamemd.exe.**
Those are ModEnc myths; the binary parses `Sensors=` and `SensorsSight=`
only.

### A.9 Dolphin — clarification (MEDIUM confidence)

Dolphin (`[DLPH]`) is a **UnitClass** with `Voxel=no` (uses SHP sprites,
8 facings, WalkFrames=6, FiringFrames=6). From decompilation of
`UnitClass::AI`, Dolphin uses the same path as vehicles. The
`NonVehicle=yes` flag at TypeClass+0xE1B (verified from
`UnitTypeClass::ReadINI` at 0x7478a3) is off for DLPH (stock INI does not
set NonVehicle). Dolphin is NOT an InfantryClass — it remains a
UnitClass but with an SHP render path instead of voxel.

Dolphin uses the **Ship locomotor** (CLSID `{2BEA74E1-...}`) with
`Underwater=yes, Cloakable=yes, Sensors=yes, SensorsSight=8`. All
submerge/surface behavior reuses the generic cloak state machine
described above. The Sonic weapon's `DecloakToFire=no` is what keeps the
Dolphin invisible while attacking.

### A.10 Sub-specific INI keys actually consumed (HIGH)

| INI Key | Location | Type | Address | Notes |
|---|---|---|---|---|
| `Underwater=` | TypeClass+0xD69 | bool | parsed at 0x714d74 | Read but **not branched on anywhere in the combat/movement hot paths** — used only as metadata and by the tile-rendering layer. Most behavior comes from Cloakable + SensorsSight. |
| `Cloakable=` | TypeClass+0xCD0 | bool | parsed (see cloak doc) | Grants cloak ability. |
| `CloakingSpeed=` | TypeClass+0x310 | int | param_1[0xc4] | Frames per cloak step. Subs use 1. |
| `CloakStop=` | TypeClass+0xC93 | bool | parsed at 0x713105 | If true, must stop moving to cloak. All YR subs have CloakStop=no (can dive while moving). |
| `Sensors=` | TypeClass+0xC9D | bool | parsed at 0x713ff2 | Unit is a cloak detector. |
| `SensorsSight=` | TypeClass+0x5F0 | int | param_1[0x17c] | Sensor range in cells. |
| `Naval=` | TypeClass+0xCCE | bool | parsed at 0x714a59 | Naval classification. |
| `NavalTargeting=` | TypeClass+0x600 | int | param_1[0x180] | Weapon preference for naval AI. |
| `TooBigToFitUnderBridge=` | TypeClass+0xE16 | bool | parsed in UnitTypeClass ReadINI | All big ships have this (except DLPH/HYD). |
| `DecloakToFire=` | WeaponTypeClass+0x133 | bool | parsed elsewhere | Weapon-level gate. Default = true. All stock torpedo/sonic weapons override to false. |

---

## Part B — Ship Sinking Wreck Sequence

### B.1 Trigger (HIGH)

The sink vs explode decision lives in `UnitClass::ReceiveDamage`
(0x00737c90) at the `case 4` (unit died) branch. The key condition is:

```c
// param_1[10].vtable_INoticeSource = TechnoTypeClass*
// puVar10[0xCCE] = Naval flag
// puVar10[0xD69] = Underwater flag
// puVar10[0xD97] = Organic flag
// *(double*)(puVar10 + 0x370) = Weight (double, at TypeClass+0x370)
// *(double*)(g_RulesClass + 0x630) = ShipSinkingWeight (double, default 3.0)
// vtable+0x1bc => CellClass at unit's position
// cell+0xEC == 2 means LandType == Water
// &param_1[3].Health+1 = TechnoClass+0xCD (byte)  — some state guard
// &param_1[8].IsOnMap = TechnoClass+0x320 (int)   — "alternate death" override

if (*(int*)(puVar10 + 0xE20) < 1) {  // DeathFrames < 1 (not the infantry-DeathFrames path)
    if ((puVar10[0xCCE] == 0)                                // NOT Naval
        || (puVar10[0xD69] != 0)                              // OR Underwater (submerged)
        || (puVar10[0xD97] != 0)                              // OR Organic (Squid)
        || (*(double*)(puVar10 + 0x370)                       // OR Weight < ShipSinkingWeight
              < *(double*)(g_RulesClass + 0x630))
        || ((cell = vtable+0x1bc())->LandType != 2)           // OR not on Water cell
        || (*(char*)((int)&param_1[3].Health + 1) != 0))      // OR in-limbo/internal state
    {
        // NORMAL EXPLOSION PATH
        vtable+0x3B8();                 // clear target / release linked units
        cell_landtype = vtable+0x1c8(); // get Z height
        if (cell_landtype < 0xb && IsABomb) {
            if (cell->LandType == 2) {
                // ABomb-on-water special path: spawn Wake anim + selected splash
                // See note on ABomb SplashList below.
            }
        }
        UnitClass::Death_Explosion();
    } else {
        // ...DeathFrames > 1 fallthrough (not relevant here)
    }
}
// ... code falls through to cleanup (passengers eject, etc) ...
```

**Inverted-NOT reading** (so the sink path activates when all of):
1. `Naval=yes` (TypeClass+0xCCE)
2. `Underwater=no` (TypeClass+0xD69) — submerged things don't visually sink
3. `Organic=no` (TypeClass+0xD97) — Squid is organic, explodes instead
4. `Weight >= RulesClass.ShipSinkingWeight` (defaults to 3.0)
5. Unit is currently on LandType=Water (cell+0xEC == 2)
6. Internal state guard byte at TechnoClass+0xCD is clear

If any of those fails, the unit runs `UnitClass::Death_Explosion()`
(0x00738680) — the standard explode-into-wreck path.

### B.2 Setting IsSinking (HIGH)

When the sink branch is taken, control falls through to a block we did NOT
explicitly capture in the decompile of ReceiveDamage — but multiple
lines in `UnitClass::AI` (0x007360c0) confirm the state variable. After
the branch, the sinking state bit is set at `TechnoClass+0x3CD` (byte).
`CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md` does not cover this field;
cross-reference against `NAVAL_IMPLEMENTATION_PLAN.md` §5c.

IsSinking is a **per-unit byte flag**, NOT a mission state and NOT a
cloak state. It is independent of the cloak pipeline. Once set, the AI
path described in §B.3 takes over each tick.

| Offset | Size | Name | Notes |
|---|---|---|---|
| +0x3CD | 1 | IsSinking | master sink flag |
| +0x3CE | 1 | IsSinking_prev | edge detection for one-shot SinkingSound |
| +0x3CA | 2 | WaterlineY | screen-Y clipping for voxel (ship-only) |
| +0x328 | 4 | AngleRotatedSideways (float) | for tilt |
| +0x32C | 4 | AngleRotatedForwards (float) | for tilt |
| +0x9C/A0/A4 | 12 | Location XYZ | position updated each tick while sinking |

### B.3 Per-tick sinking loop (HIGH — verified from UnitClass::AI)

Located in `UnitClass__AI` (0x007360c0), early in the function right after
`TurretAI()`. Decompiled block:

```c
if (*(char*)((int)param_1 + 0x3cd) != '\0') {   // IsSinking == 1
    iStack_18 = param_1[0x27];   // Location_X  (TechnoClass+0x9C)
    iStack_14 = param_1[0x28];   // Location_Y  (TechnoClass+0xA0)
    iStack_10 = param_1[0x29];   // Location_Z  (TechnoClass+0xA4)
    local_4 = iStack_10 + -5;    // Z - 5 leptons
    iStack_c = iStack_18;
    iStack_8 = iStack_14;
    (**(code**)(*param_1 + 0x1b4))(&iStack_c);  // Set_Coord(x,y,z-5)

    iVar7 = (**(code**)(*param_1 + 0x1c8))();    // get ground height / altitude
    if (iVar7 < -400) {
        // Ship has sunk deep enough: remove from map
        (**(code**)(*param_1 + 0xe0))(0);        // Limbo
        (**(code**)(*param_1 + 0xf8))();         // delete/expire
        return;
    }

    if ((g_CurrentFrameCounter & 3) == 0) {      // every 4 frames
        iVar7 = Random__RandomRanged(-0xAA, 0xAA);  // ±170 leptons X
        iVar8 = Random__RandomRanged(-0xAA, 0xAA);  // ±170 leptons Y
        iStack_10 = (**(code**)(*param_1 + 0x1c8))();  // elev
        iStack_18 = param_1[0x27] + iVar8;   // NOTE: X <- X+iVar8, Y <- Y+iVar7
        iStack_14 = iVar7 + param_1[0x28];   //   (decompiler's vars vs code flow)
        iStack_10 = param_1[0x29] - iStack_10;
        pvVar5 = operator_new(0x1c8);
        if (pvVar5 != (void*)0x0) {
            AnimClass::Constructor(
                *(undefined4*)(g_RulesClass + 0x94),    // [General]Wake (WAKE1)
                &iStack_18, 0, 1, 0x600, 0, 0);
        }
    }
}
```

Findings:

- **Descent rate:** 5 leptons per tick (Z -= 5 each call to Set_Coord).
  At 15 fps that is 75 leptons/sec. Standard cell is 256 leptons, so a
  ship sinks ~30% of a cell's worth per second vertically.
- **Termination:** when `vtable+0x1c8()` (altitude-over-ground) returns
  less than `-400`, the ship Limbos itself (`vtable+0xE0`) and then
  expires (`vtable+0xF8`). This gives a total Z-drop of 400+ leptons ≈
  **80 ticks ≈ 5.3 seconds at 15 fps** before the entity is removed.
  (400/5 = 80 exact lower bound; the initial altitude over water can
  tack on a few ticks.)
- **Ambient spawns:** every 4 frames a `Wake=` anim (`WAKE1` from
  `[General]Wake` at RulesClass+0x94) is created at a random offset
  of ±170 leptons around the sinking ship. This is the rippling "water
  surface disturbance" you see around the sinking ship in retail — NOT
  the `SplashList` entries.
- **No SplashList consumption here.** `SplashList` (H2O_EXP3, H2O_EXP2,
  H2O_EXP1) is spawned by `Warhead::Detonate` via
  `Warhead__SelectExplosionAnim` (§B.5) — it is the *impact* splash on
  water, not the sinking visual. See next section for the full selection
  logic.

### B.4 Tilt + waterline clipping (MEDIUM — not re-verified here)

Not freshly re-decompiled in this pass. `NAVAL_IMPLEMENTATION_PLAN.md`
§5c documents:
- TechnoClass+0x32C (AngleRotatedForwards) increments by 0.01 rad/frame.
- Max tilt = PI/4 (≈ 0.785 rad).
- Direction = sign based on facing octant (octants 0,6,7 → negative;
  octants 1,2,3,4,5 → positive).
- TechnoClass+0x3CA (WaterlineY, short) clips the voxel image Y in
  screen space.

These are consumed in `Draw_Matrix` (`0x69F670`), specifically the
`AngleRotatedSideways/Forwards` read at `techno+0x328` / `+0x32C` noted in
`NAVAL_SYSTEM_RESEARCH.md` §13. This research confirms the UnitClass::AI
loop only modifies Location_Z directly; the tilt is applied separately
somewhere near WhereFrom the tilt rate of 0.01 rad/frame comes. **Open
Question:** we did not re-verify the 0.01 rad/frame figure in this pass —
flagged as MEDIUM.

### B.5 SplashList — when it fires (HIGH)

`SplashList` is a RulesClass field storing a DynamicVector of AnimTypes.
Parsed in `RulesClass::ReadCombatDamage` at 0x0066BFA0 (line in that
function for `s_SplashList_0083b1fc`). Stored at:

| RulesClass Offset | Field |
|---|---|
| +0xBC4 | AnimType ptr array base |
| +0xBD0 | AnimType count |
| +0xBD4..0xBD8 | vector tail pointers |

Selected by `Warhead__SelectExplosionAnim` (0x0048A4F0):

```c
// param_1 = impact damage; param_2 = WarheadTypeClass*
// param_3 = LandType of impact cell; param_4 = impact coord struct

if (param_3 == 2 /*Water*/                         // impact on water
    && warhead+0x14D (Conventional=yes)            // normal explosive warhead
    && (cell.flags & 0x100) == 0                   // not on a bridge
    && impact_z < ground_z + DAT_0089E870 * 2)     // near surface
{
    if (RulesClass+0xBD0 == 0) return 0;           // no SplashList
    idx = min(damage, 0x23 * count - 1) / 0x23;    // damage-scaled bucket
    return RulesClass+0xBC4[idx];                  // pick anim type
}
// ...else fall through to warhead AnimList selection (non-water)
```

**Key insight:** the SplashList triggers on **any conventional weapon
impact on water**, not specifically on ship death. When a ship dies
from a Conventional warhead hit and is over water, both things happen:
1. The warhead plays a SplashList anim at the impact point.
2. The ship transitions through Death_Explosion OR into IsSinking.

The SplashList is selected by **damage bucket**: each index covers
0x23 (35) damage, so higher-damage impacts pick later (bigger) splash
animations. Stock YR: `SplashList=H2O_EXP3,H2O_EXP2,H2O_EXP1` — indices
0,1,2 with 0 being the smallest.

**Suppression gate:** In `Warhead::Detonate` (0x004690B0), there is an
*additional* check before calling SelectExplosionAnim:

```c
if (impact_z < DAT_0089DE70 * 2) {
    cell = Get_Cell_At(impact);
    if (cell.LandType == 2 /*Water*/ && areaDamage != 2) {
        // Suppress water SplashList if a Naval unit is on the cell
        // (and it's not underwater/submerged and not in limbo)
        first_occupant = cell.FirstObject;
        if (first_occupant &&
            first_occupant.WhatAmI == 1 &&               // UnitClass
            first_occupant.Type.Naval != 0 &&            // Naval=yes
            first_occupant.Type.Underwater == 0)         // not submerged
        {
            iVar12 = -1;  // fall back to warhead's own AnimList
        }
    }
}
iVar12 = Warhead__SelectExplosionAnim(iVar12, ...);
```

So: when a weapon hits a (surfaced) ship, the regular warhead AnimList
(explosion) plays — **not** the SplashList. SplashList only plays on open
water or on submerged (Underwater=yes) targets.

### B.6 SinkingSound + VoiceSinking (HIGH)

Parsed in `TechnoTypeClass::ReadINI`:

| Field | TypeClass Offset (int*) | Byte Offset | INI Key |
|---|---|---|---|
| `SinkingSound=` | param_1[0x151] | +0x544 | `SinkingSound=GenLargeWaterDie` on all naval units |
| `VoiceSinking=` | param_1[0x154] | +0x550 | `VoiceSinking=` — commonly empty; unit-specific crew voice |

Both stored as `VocClass` indices (int; -1 = none). From the decompile,
the value is the result of `VocClass::FindByName()` on the string. If the
string doesn't resolve to a VocClass, the previous value is kept.

Stock usage: `SinkingSound=GenLargeWaterDie` appears on 9 naval unit
sections (DEST, CARRIER, DLPH, SUB, HYD, DRED, BSUB, SQD, AEGIS — and
their YR equivalents). The sound is played **once** at the IsSinking 0→1
transition (edge-detected via TechnoClass+0x3CE = IsSinking_prev).
The edge-detection emit site is not in UnitClass::AI (which only
advances the tick), so the write to +0x3CD probably goes through a setter
that pushes the sound — we did not re-verify the exact emit point here.
Flagged as **MEDIUM** — see Open Questions.

### B.7 Final cleanup & occupancy release (HIGH)

When `vtable+0x1c8()` returns < -400 in the sinking loop:

1. `vtable+0xE0(0)` — Limbo the unit (removes from map, calls
   `Mark_All_Occupation_Bits` to clear cell occupation).
2. `vtable+0xF8()` — expire/destroy. This calls the destructor chain
   that deallocates memory and removes from global DynamicVector.

So occupancy is released **at end-of-sink**, not at IsSinking-set.
During the ~80-tick sink, the ship still occupies its cells (ships cannot
drive through each other while one is mid-sink). No wreck, no corpse,
no debris — the ship entity is simply gone after ~5.3 seconds.

### B.8 Comparison vs ground-unit death (HIGH)

| Phase | Ground unit (tank) | Naval unit (ship, sink path) |
|---|---|---|
| 0 | ReceiveDamage case 4, HP≤0 | Same |
| 1 | UnitClass::Death_Explosion (0x738680) — spawns explosion anim from `ExplosionAnim=`, applies Scorch, drops Crate if lucky | Branch **skipped**; Death_Explosion is NOT called |
| 2 | Parachute infantry (Survivor) eject via `TechnoTypeClass+0xccd` if random < `rules+0x5C0` | Skipped (ships have no Passengers typically; passengers dropped via OpenTransport path if Opentopped) |
| 3 | Entity removed immediately (no sink) — VXL wreck spawned if Crusher/CrushableLevel rules say so | IsSinking set; entity persists on map for ~80 ticks |
| 4 | — | Per-tick Z-descent + Wake spawns (§B.3) |
| 5 | — | At Z < -400: Limbo + expire |

Ground units do **NOT** have a "wreck anim timer" in gamemd.exe; the
explosion animation is just a spawned AnimClass and the entity is
deleted the same frame. Ships differ because the sinking sequence keeps
the ship entity alive for the visual drop, and `UnitClass::Death_Explosion`
is bypassed entirely. No secondary explosion plays when a ship transitions
to sinking — only `SinkingSound` and the falling Wake animations.

---

## Class Layouts / Key Offsets (Consolidated)

### TechnoTypeClass (INI-backed type data)

| Byte Offset | int* idx | Size | INI Key | Default | Notes |
|---|---|---|---|---|---|
| 0x310 | [0xC4] | 4 | `CloakingSpeed=` | — | frames/step |
| 0x370 | [0xDC] | 8 | `Weight=` | 1.0 | double, decides sink vs explode |
| 0x544 | [0x151] | 4 | `SinkingSound=` | — | VocClass index (int) |
| 0x550 | [0x154] | 4 | `VoiceSinking=` | — | VocClass index |
| 0x5F0 | [0x17C] | 4 | `SensorsSight=` | 0 | cells |
| 0x600 | [0x180] | 4 | `NavalTargeting=` | 0 | AI preference |
| 0xC93 | byte | 1 | `CloakStop=` | no | must-stop-to-cloak |
| 0xC9D | byte | 1 | `Sensors=` | no | has sensor coverage |
| 0xCCE | byte | 1 | `Naval=` | no | sink classification |
| 0xCD0 | byte | 1 | `Cloakable=` | no | unit can cloak |
| 0xD69 | byte | 1 | `Underwater=` | no | submerged by default (rendering hint) |
| 0xD97 | byte | 1 | `Organic=` | no | explodes instead of sinks |
| 0xE16 | byte | 1 | `TooBigToFitUnderBridge=` | no | bridge gate |
| 0xE1B | byte | 1 | `NonVehicle=` | no | SHP infantry-like render (not set for DLPH) |

### TechnoClass (runtime instance)

| Byte Offset | Size | Field | Notes |
|---|---|---|---|
| 0x9C | 12 | Location XYZ (3 ints) | |
| 0x220 | 4 | CloakState | 0..3 |
| 0x224 | 4 | CloakProgress | 0..CloakingStages |
| 0x22C | 12 | CloakStepTimer | CDTimerClass |
| 0x238 | 4 | CloakingSpeed | runtime copy of TypeClass+0x310 |
| 0x240 | 12 | ReCloakDelayTimer | CDTimerClass |
| 0x328 | 4 | AngleRotatedSideways | float |
| 0x32C | 4 | AngleRotatedForwards | float |
| 0x370 | 8 | (type weight echo — check) | double |
| 0x3CA | 2 | WaterlineY | screen-Y clip short |
| 0x3CD | 1 | IsSinking | master flag |
| 0x3CE | 1 | IsSinking_prev | edge detect |
| 0x3D2 | 1 | HasStealthAbility | runtime Cloakable |

### RulesClass

| Byte Offset | Size | Name | Default |
|---|---|---|---|
| 0x94 | 4 | Wake (AnimType*) | `WAKE1` |
| 0x628 | 4 | CloakingStages | 9 |
| 0x630 | 8 | ShipSinkingWeight (double) | 3.0 |
| 0xBC4 | 4 | SplashList base (AnimType*[]) | — |
| 0xBD0 | 4 | SplashList count | 3 (H2O_EXP3,2,1) |
| 0x1410 | 8 | CloakDelay (minutes, double) | 0.02 |
| 0x1708 | 8 | ConditionYellow (double) | — |

### WeaponTypeClass

| Offset | Size | INI Key | Notes |
|---|---|---|---|
| 0x133 | 1 | `DecloakToFire=` | **default YES**; overridden by torpedo weapons |

### WarheadTypeClass

| Offset | Size | INI Key | Notes |
|---|---|---|---|
| 0x14D | 1 | `Conventional=` | gates SplashList use for water impact |

### CellClass

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0x7C | 2*N | SensorCount[MaxHouses] | per-house cloaked detection |
| 0xE4 | 4 | first occupant (Object*) | used to suppress SplashList over ships |
| 0xEC | 4 | LandType | 2 = Water |
| 0x140 | 4 | cell flags (bit 0x100 = bridge) | |

---

## INI Keys Consumed (Full List for Phase 5b/5c)

**Per-unit (TechnoTypeClass / UnitTypeClass):**
- `Cloakable=`, `CloakStop=`, `CloakingSpeed=` — dive/surface behavior
- `Sensors=`, `SensorsSight=` — detect submerged enemies
- `Underwater=` — rendering-layer hint; NOT behavior-gated in sim
- `Naval=` — classification + sink gate
- `NavalTargeting=` — AI weapon selection
- `Weight=` — sink-threshold comparison
- `Organic=` — skips sink path (Squid explodes)
- `SinkingSound=` — one-shot voc at IsSinking transition
- `VoiceSinking=` — optional crew voice (rarely set)
- `TooBigToFitUnderBridge=` — big ships blocked by bridges (orthogonal to dive)

**Per-weapon (WeaponTypeClass):**
- `DecloakToFire=` — default YES; torpedo weapons set NO

**Per-warhead (WarheadTypeClass):**
- `Conventional=` — gates SplashList for conventional explosions on water

**[General] (RulesClass):**
- `ShipSinkingWeight=3.0` — default sink threshold
- `CloakingStages=9` — dive/surface animation steps
- `CloakDelay=.02` — minimum time between dives (minutes)
- `ConditionYellow=` — damage threshold for dive-chance reduction
- `Wake=WAKE1` — animation used for wake AND for sinking splash rings
- `SplashList=H2O_EXP3,H2O_EXP2,H2O_EXP1` — water-impact animations

---

## Integration Points (per-tick loop)

### Sim-tick call chain (submarine)

```
World::advance_tick
  └── combat/targeting phase
        └── TechnoClass::AI         (per-unit)
              └── TechnoClass::CloakingTick (0x006FB740)
                    ├── state 0 path: CanAutoCloak → StartCloaking
                    ├── state 1 path: advance progress → state 2 when visual==5
                    ├── state 2 path: ShouldUncloak → StartUncloaking
                    └── state 3 path: advance progress DOWN → state 0 when visual==0
```

### Sim-tick call chain (sinking)

```
World::advance_tick
  └── combat/targeting phase
        └── UnitClass::AI (0x007360c0)
              ├── ...
              └── if (TechnoClass.IsSinking) {
                    Location_Z -= 5
                    Set_Coord()
                    if (altitude < -400) Limbo + expire
                    if (frame & 3 == 0) spawn Wake anim at random offset
                  }
              └── ... more AI logic only if NOT sinking
```

### On death (ReceiveDamage case 4)

```
TechnoClass::ReceiveDamage
  └── ...delegates...
        └── UnitClass::ReceiveDamage (0x00737c90)
              ├── if (Naval && !Underwater && !Organic &&
              │       Weight >= ShipSinkingWeight && LandType==Water && guard OK)
              │     ├── vtable+0x3B8()   — disengage
              │     ├── set IsSinking = 1   (write to TechnoClass+0x3CD)
              │     │  (triggers one-shot SinkingSound via edge detect in a setter)
              │     └── (does NOT call Death_Explosion)
              └── else
                    └── UnitClass::Death_Explosion — normal explode + remove
```

---

## Current Rust Implementation Status

Grep of `src/sim/` and `src/rules/` (honest report):

- **`Naval=` and `WaterBound=` are parsed** in
  `src/rules/object_type.rs` (fields `naval`, `water_bound`). Used by
  production placement (`src/sim/production/production_placement.rs`).
- **`Cloakable`, `CloakingSpeed`, `CloakStop`, `Sensors`, `SensorsSight`,
  `Underwater`, `Organic`, `Weight`, `SinkingSound`, `VoiceSinking`,
  `NavalTargeting`, `SplashList`, `ShipSinkingWeight`, `Wake`, `DecloakToFire`,
  `Conventional`** — **NOT parsed.** Grep returns only "Cloakable" as a string
  in an INI parser test case.
- **No `CloakState` or `IsSinking` fields** on any GameEntity variant.
- **No submarine/dive/surface logic** in any sim module. Combat does not
  consider cloak; all sub entities would be fully visible and firing as
  normal ships today.
- **No sinking tick path.** Ship destruction currently runs through the
  generic entity-death path; ships would disappear instantly on HP≤0.
- **Wake animation** is listed as Phase 5 in
  `NAVAL_IMPLEMENTATION_PLAN.md` but not implemented either.

So the full Phase 5b (submarine) and Phase 5c (sinking) feature surface
is **greenfield** in Rust. No cloak framework exists to build on; a
generic cloak pipeline will need to be created before submarines can be
added.

---

## Open Questions

1. **`TechnoClass+0xCD` state guard in the sink branch.** The ReceiveDamage
   decompile shows a byte at `(int)&param_1[3].Health + 1` which resolves
   to `TechnoClass+0xCD`. This guard skips the sink path. We did not
   identify which exact state this byte represents (candidates: "HasPayback,"
   "IsBeingWarped," "IsInAir") — needs xref analysis. LOW impact since
   for normal skirmish combat it is zero; only exotic scenarios flip it.

2. **Tilt rate 0.01 rad/frame not re-verified.** The figure appears in
   `NAVAL_IMPLEMENTATION_PLAN.md` §5c but was not re-decompiled in this
   pass. The code that increments `AngleRotatedForwards` while
   IsSinking=1 might live in `Draw_Matrix` or in a separate AI sub-helper
   we didn't locate. MEDIUM priority.

3. **Where IsSinking=1 is actually written.** The sink branch in
   ReceiveDamage clearly takes a different code path, but the
   decompile we have of 0x00737C90 does not directly show a
   `*(char*)(this+0x3CD) = 1` assignment. There is probably a
   `MarkForSinking()` helper or inline setter that also pushes the
   one-shot SinkingSound. Needs xref hunt on the 0x3CD byte offset.

4. **DLPH NonVehicle classification.** Dolphin is an SHP-sprite unit
   but NOT `NonVehicle=yes` in stock INI. Confirm whether the SHP
   selection is based on `Voxel=no` alone or on other flags.

5. **Boomer cruise missile decloak order.** Boomer has Primary
   `BoomerTorpedo` (DecloakToFire=no) and Secondary `CruiseLauncher`
   (default decloak=yes). When attack-moving, which weapon is chosen, and
   does the Boomer surface *before* starting the cruise launch or
   *during*? Verify the target-weapon decision vs the surface-trigger
   order in `TechnoClass::GetFireError`.

6. **Wake-around-sinking randomness determinism.** The loop uses
   `Random__RandomRanged(-0xAA, 0xAA)`. This has to be the deterministic
   lockstep RNG — confirm that `Random__RandomRanged` goes through the
   same RNG as other sim calls (likely yes, but we didn't trace it).

---

## Sources

- Live Ghidra MCP decompilation of `gamemd.exe` (2026-04):
  - `UnitClass::ReceiveDamage` — 0x00737C90
  - `UnitClass::Death_Explosion` — 0x00738680
  - `UnitClass::AI` — 0x007360C0 (contains the sinking per-tick loop)
  - `UnitClass::TubeMovement` — 0x00736060 (unrelated ref, same file)
  - `ShipLocomotionClass::Process` — 0x0069FC10 (confirms no dive logic here)
  - `TechnoClass::CloakingTick` — 0x006FB740 (via cloak doc)
  - `TechnoClass::CanAutoCloak` — 0x006FBDC0
  - `TechnoClass::ShouldUncloak` — 0x006FBC90
  - `Warhead::SelectExplosionAnim` — 0x0048A4F0
  - `WarheadTypeClass::Detonate` — 0x004690B0
  - `TechnoTypeClass::ReadINI` — searched for keys: `Naval`, `Underwater`,
    `Organic`, `Weight`, `SinkingSound`, `VoiceSinking`, `NavalTargeting`,
    `CloakingSpeed`, `CloakStop`, `Sensors`, `SensorsSight`, `Cloakable`
  - `UnitTypeClass::ReadINI` — 0x007478A3 (NonVehicle offset +0xE1B)
  - `RulesClass::ReadCombatDamage` — 0x0066BFA0 (SplashList parse)
  - `RulesClass::ReadGeneral` — 0x0066F16C (ShipSinkingWeight parse)
- `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini` — all naval
  unit sections, torpedo/sonic weapon sections, splash/wake general keys.
- Prior research:
  - `CLOAKING_STEALTH_SYSTEM_GHIDRA_REPORT.md` — full cloak pipeline
    (inherited without modification for submarines).
  - `NAVAL_SYSTEM_RESEARCH.md` §14 (Submarine & Underwater) — confirms
    `Is_Surfacing` at vtable 38 is about transport cargo, not sub surface.
  - `NAVAL_IMPLEMENTATION_PLAN.md` §5b/5c — previous planning doc; values
    in this report supersede where they differ.
  - `BUILDING_DAMAGE_DESTRUCTION_GHIDRA_REPORT.md` — ground-unit death
    pattern for comparison (§B.8).
