# Guardian GI MissileLauncher / AAHeatSeeker2 Projectile Lifecycle

**Date:** 2026-05-20
**Binary:** `gamemd.exe` (Yuri's Revenge)
**Investigation mode:** exhaustive slice for deployed Guardian GI
`MissileLauncher` firing `AAHeatSeeker2`; rendering internals are covered only
to the level needed to replace VERA20k's current app-side missile visual.
**Confidence:** HIGH for projectile creation, launch, homing update, arming,
detonation timing, INI key consumers, and current VERA20k gap classification.
MEDIUM for the exact object-notification path that clears `BulletClass.Target`
when the target object is destroyed; the live AI behavior for null/sentinel
targets is verified.
**Active in YR:** Yes. The traced path is the normal stock-YR weapon fire path:
`InfantryClass::Fire_At_Target` -> `TechnoClass::Fire_At` ->
`BulletClass::Fire` -> `BulletClass::AI` -> `BulletClass::BulletDetonation`.

## Scope

This report answers one narrow parity question: what does original YR do after a
deployed Guardian GI selects `MissileLauncher` and launches the
`AAHeatSeeker2` projectile?

Included:

- bullet allocation and launch for `[GGI]` deployed secondary fire
- when real damage is applied
- `AAHeatSeeker2` INI key consumers
- `WeaponType.Speed=30` effect on missile movement
- homing against ground vehicles and Rocketeer-style targets
- invalid/lost target behavior that is visible to gameplay
- projectile rendering facts needed by VERA20k's next implementation slice

Non-scope:

- full infantry deploy/fire animation frame audit; see `GGI_GHIDRA_REPORT.md`
- full pixel-perfect line-trail renderer; see `LINE_TRAIL_CLASS_GHIDRA_REPORT.md`
- complete warhead damage math; see existing warhead/damage reports

## 1. Summary Findings

Deployed Guardian GI does not produce a render-only effect. In stock YR,
`MissileLauncher` creates a real `BulletClass` object whose type is
`AAHeatSeeker2`. The missile exists in the object/display system, has a world
position and velocity, turns every game tick, can continue after the original
target moves, and applies its `GUARDWH` warhead only when the bullet detonates.

Actual damage is not applied at the fire tick. `TechnoClass::Fire_At` may update
an estimated-health bookkeeping field on the target, but real damage is delayed
until `BulletClass::BulletDetonation @ 0x00468D80` calls
`WarheadTypeClass::Detonate`.

For `AAHeatSeeker2`, `Proximity=no` is not a live disable switch for impact
checking. The field is parsed into `BulletTypeClass`, but the live homing path
does not test it. Because `ROT=60` selects the ROT>0 homing branch,
`BulletClass::AI` still runs the proximity/overshoot detector after the `Arm`
delay.

Second-pass swarm reports on 2026-05-20 refined several details in this parent
report. Treat the following as the current authoritative slice:

- stock YR `[General] MissileROTVar=.25`, not `1.0`
- launch velocity for ROT>0 AAHeatSeeker2 normalizes to magnitude `1.0`
- weapon `Speed=30/40` is target speed, reached by default `Acceleration=3`
- close-range ROT multiplies the computed turn integer by `1.5`
- the proximity detector reference coordinate is fixed at launch
- destroyed non-high-flying ground targets can retarget the bullet to a
  `CellClass*` at the last valid target cell
- DRAGON frame mapping is velocity-derived via the BulletClass draw helper
- GUARDWH detonation is one normal non-airburst warhead detonation with
  default `Cluster=1`

## 2. INI Inputs

### 2.1 Guardian GI and Weapon

`rulesmd.ini`:

```ini
[GGI]
Primary=M60
Secondary=MissileLauncher
OpenTransportWeapon=1
Deployer=yes
DeployFire=yes
DeployedCrushable=no
```

`MissileLauncher`:

```ini
[MissileLauncher]
Damage=40
ROF=40
Range=8
Burst=1
Projectile=AAHeatSeeker2
Speed=30
Warhead=GUARDWH
Report=GuardianGIDeployedAttack
MinimumRange=1
```

Elite variant:

```ini
[MissileLauncherE]
Damage=50
ROF=20
Range=8
Burst=1
Projectile=AAHeatSeeker2
Speed=40
Warhead=GUARDWH
Report=GuardianGIDeployedAttack
MinimumRange=1
```

### 2.2 Projectile

`AAHeatSeeker2` is identical in `rules.ini` and `rulesmd.ini` for this slice:

```ini
[AAHeatSeeker2]
Arm=2
Shadow=no
Proximity=no
Ranged=yes
AA=yes
AG=yes
Image=DRAGON
ROT=60
SubjectToCliffs=no
SubjectToElevation=no
SubjectToWalls=no
```

`artmd.ini` / `art.ini` `[DRAGON]`:

```ini
[DRAGON]
;Trailer=SMOKEY2
UseLineTrail=yes
LineTrailColor=216,216,255
LineTrailColorDecrement=16
Rotates=yes
```

`GUARDWH`:

```ini
[GUARDWH]
Wall=yes
Wood=yes
Verses=20%,20%,20%,100%,50%,100%,10%,10%,10%,100%,100%
Conventional=yes
InfDeath=3
AnimList=XGRYSML1,XGRYSML2,EXPLOSML,XGRYMED1,XGRYMED2,EXPLOMED,EXPLOLRG,TWLT070
ProneDamage=50%
CellSpread=.5
PercentAtMax=.5
```

## 3. Address Map

| Function / object | Address | Verified role |
|---|---:|---|
| `InfantryClass::Fire_At_Target` | `0x005206B0` | Deployed GGI fire driver; dispatches selected weapon fire |
| `InfantryClass::SelectWeapon` | `0x005218E0` | Returns deployed secondary for GGI when deployed |
| `TechnoClass::GetFireError` | `0x006FC0B0` | Uses projectile `AA`/`AG` gates for target legality |
| `TechnoClass::Fire_At` | `0x006FDD50` | Normal weapon fire implementation |
| `BulletClass::Allocate` | `0x0046B050` | Creates bullet and calls `BulletClass::Init` |
| `BulletClass::Init` | `0x004664C0` | Stores type, owner, target, warhead, speed, damage payload |
| `BulletClass::SetWeapon` | `0x0046B260` | Ghidra label may say `SetOwner`; writes weapon pointer |
| `BulletClass::Fire` | `0x00468670` | Reveals bullet, initializes velocity and arming detector |
| `BulletClass::AI` | `0x004666E0` | Per-tick projectile movement and detonation checks |
| `BulletClass::HomingTrack` | `0x005B20F0` | Turns missile velocity toward current target coordinate |
| `BulletClass::BulletDetonation` | `0x00468D80` | Calls warhead detonation and removes bullet |
| `BulletTypeClass::ReadINI` | `0x0046BEE0` | Parses projectile keys and DRAGON art keys |
| `ProximityDetector::Init` | `0x004E1100` | Clears detector state |
| `ProximityDetector::Set/Arm` | `0x004E1130` | Stores arm delay, reference target coordinate, closest distance |
| `ProximityDetector::Check` | `0x004E11F0` | Returns impact/overshoot status after arm delay |

## 4. Bullet Creation Path

### Verified binary facts

The deployed Guardian GI route is:

1. `InfantryClass::Fire_At_Target @ 0x005206B0` selects and fires the deployed
   weapon.
2. The selected weapon is `MissileLauncher` because GGI has
   `DeployFire=yes` and `DeployFireWeapon=1` behavior documented in
   `GGI_GHIDRA_REPORT.md`.
3. The infantry fire driver dispatches to `TechnoClass::Fire_At @ 0x006FDD50`.
4. `TechnoClass::Fire_At` reaches `BulletClass::Allocate @ 0x0046B050` from
   call site `0x006FE55D`.
5. `BulletClass::Allocate` creates a `BulletClass` and calls
   `BulletClass::Init @ 0x004664C0`.
6. `TechnoClass::Fire_At` calls the weapon-pointer setter
   `BulletClass::SetWeapon @ 0x0046B260`, conceals the object, computes the
   launch trajectory, then calls `BulletClass::Fire @ 0x00468670`.
7. `BulletClass::Fire` calls `ObjectClass::Reveal`, initializes velocity and
   arming state, then submits the bullet to the display/object system.

This proves the answer to the first investigation question: yes, deployed GGI
`MissileLauncher` spawns a real `BulletClass` projectile entity in standard YR.

### Key BulletClass fields used by this projectile

Offsets are from the existing `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` and live
decompilation of `BulletClass::Init`, `BulletClass::Fire`, and `BulletClass::AI`.

| Offset | Field | Relevance to AAHeatSeeker2 |
|---:|---|---|
| `+0x6C` | damage payload | `40` for normal `MissileLauncher`, later consumed at detonation |
| `+0x9C/+0xA0/+0xA4` | current world location | drives movement, render position, detonation coordinate |
| `+0xA8` | line-trail pointer | populated through Object/LineTrail path when `UseLineTrail=yes` |
| `+0xAC` | `BulletTypeClass*` | `AAHeatSeeker2` |
| `+0xB0` | owner/firer | Guardian GI |
| `+0xB8..+0xDF` | proximity detector | arming and overshoot/close-impact state |
| `+0xE8/+0xF0/+0xF8` | velocity vector | homing movement updates this every tick |
| `+0x105` | course-lock flag | used in homing path |
| `+0x108` | course-lock counter | used if `CourseLockDuration` is nonzero |
| `+0x10C` | target pointer | re-read every homing tick when non-null |
| `+0x110` | target speed | weapon `Speed` value, `30` for normal GGI |
| `+0x118` | approach sample count | used to detect no-longer-closing homing missiles |
| `+0x120` | approach sum | low closure eventually forces detonation |
| `+0x128` | warhead pointer | `GUARDWH` |
| `+0x12C` | animation frame | only cycles if `AnimLow/AnimHigh` are nonzero |
| `+0x12D` | animation timer | initialized from `AnimRate`; DRAGON does not set active cycle keys |
| `+0x130` | weapon pointer | `MissileLauncher` / `MissileLauncherE` |

## 5. Damage Timing

### Verified binary facts

Real damage is applied by the bullet detonation path, not the fire call.

`TechnoClass::Fire_At @ 0x006FDD50` allocates and launches the bullet. During
that routine, when the target is valid and the projectile is not an inaccurate
special case, YR updates a target-side estimated-health bookkeeping value. This
is pre-impact targeting bookkeeping, not actual damage application.

The actual damage call is reached later:

```text
BulletClass::AI @ 0x004666E0
  -> detonation condition becomes true
  -> BulletClass::BulletDetonation @ 0x00468D80
      -> WarheadTypeClass::Detonate(...)
      -> BulletClass::UnInit / removal
```

For `AAHeatSeeker2`, normal impact can be triggered by several live conditions:

- close enough to the target/adjusted coordinate in the ROT>0 homing path.
  Earlier notes described this as `distance <= current_speed * 90.0`, but the
  later exact HomingTrack report narrows this path and should be treated as the
  stronger source for scalar math.
- projectile height reaching ground/impact conditions
- proximity detector returning close impact or overshoot after arming
- lost/sentinel target coordinate at sufficient flight height
- approach history showing the missile is no longer closing
- bridge-crossing collision logic
- map/out-of-bounds or related object placement failure paths

`Arm=2` is not "explode after two ticks." It is the minimum age before the
proximity detector can report close impact or overshoot. Other detonation
conditions in `BulletClass::AI` can also apply.

## 6. AAHeatSeeker2 INI Key Consumers

### `Image=DRAGON`

`BulletTypeClass::ReadINI @ 0x0046BEE0` reads `Image` into the inherited
object-type image/art name field. Because `AAHeatSeeker2` is not `Inviso`, the
read path demand-loads the image. The art section `[DRAGON]` then supplies
projectile visuals such as `Rotates=yes` and `UseLineTrail=yes`.

Implementation implication: using `DRAGON.SHP` is correct, but it must be the
image of a sim-owned bullet, not an app-only transient effect.

### `ROT=60`

`ROT=60` selects the ROT>0 homing branch in `BulletClass::AI`.

The existing `GGI_GHIDRA_REPORT.md` verified the YR sidewinder scaling before
`HomingTrack`:

```text
sidewinder = cos(((bullet_id_like_value + frame) % 15) * 2*pi / 15) * Rules.MissileROTVar
           + Rules.MissileROTVar
           + 1.0
delta_far = ftol(sidewinder * ROT_INI)
if distance < 256 leptons, delta = ftol(delta_far * 1.5)
else delta = delta_far
ROT_BAM_per_tick = ((uint16)(delta & 0xFF)) << 8
```

With stock `MissileROTVar=.25`, `AAHeatSeeker2` uses a far-range turn integer
of roughly 60..90 before the final low-byte/BAM shift. At close range
(`< 256` leptons), the binary multiplies that integer by `1.5`, producing
roughly 90..135 before signed 16-bit wrap effects. In gameplay terms, this
missile is a sharply turning homing projectile, not a straight visual line.

### `Arm=2`

`BulletClass::Fire @ 0x00468670` passes the projectile `Arm` value to
`ProximityDetector::Set/Arm @ 0x004E1130`. The detector stores the current
global frame, the arm delay, the reference target coordinate, and the initial
closest distance.

`ProximityDetector::Check @ 0x004E11F0` will not report impact until:

```text
g_CurrentFrameCounter - detector_start_frame >= Arm
```

After arming, the detector compares the projectile's current coordinate against
the stored reference coordinate from launch. It reports close impact when half
the current distance is below `0x20` leptons, and reports an overshoot-style
impact when half the current distance is below `0x100` and the distance is now
increasing relative to the previously stored closest distance.

For one special case, `BulletClass::Fire` overrides this value to zero: if the
target's runtime type returned by `WhatAmI` is `2` (`AircraftClass` in existing
class mapping), the arm delay passed to the detector is `0`.

Ground vehicles keep `Arm=2`.

Rocketeer-style targets are combat-classified as air by INI behavior, but the
object class remains infantry for this homing branch. The verified binary check
is `WhatAmI == 2`, not the projectile legality predicate. Therefore the
Rocketeer path should keep the normal `Arm=2` detector delay unless another
untraced object-class override changes `WhatAmI`, which existing GGI/Rocketeer
research does not indicate.

### `Ranged=yes`

`BulletClass::AI` uses `Ranged` only in the gate that skips the proximity
detector for non-homing projectiles:

```text
if ROT < 1 && Ranged == false:
    skip ProximityDetector::Check
else:
    run ProximityDetector::Check
```

Because `AAHeatSeeker2` has `ROT=60`, the detector runs regardless of
`Ranged=yes`. The key is still parsed and preserved, but it is not the decisive
reason the GGI missile checks proximity.

### `Proximity=no`

`BulletTypeClass::ReadINI` parses `Proximity` into `BulletTypeClass+0x29F`.
The live launch, homing, and detonation paths checked for this report do not
consume that flag for `AAHeatSeeker2`. The ROT>0 branch still calls
`ProximityDetector::Check`.

Implementation implication: VERA20k should not treat `Proximity=no` as
"homing missile never does proximity/overshoot impact." For parity, the
proximity detector is part of the live AAHeatSeeker2 path.

### `AA=yes` / `AG=yes`

`TechnoClass::GetFireError @ 0x006FC0B0` gates fire legality using projectile
`AA` and `AG`.

- If the target is considered air and projectile `AA` is false, the weapon
  cannot fire.
- If the target is not considered air and projectile `AG` is false, the weapon
  cannot fire.

`AAHeatSeeker2` has both true, so deployed GGI can legally fire at both ground
vehicles and air-classified targets such as Rocketeers.

The homing motion branch does not use `AA`/`AG` as the main target-type
selector. The aircraft-special homing/arming flag is driven by target
`WhatAmI == 2`.

### `SubjectToCliffs=no`, `SubjectToElevation=no`, `SubjectToWalls=no`

These keys are parsed by `BulletTypeClass::ReadINI`. For this projectile they
disable terrain/wall/elevation constraints that would otherwise affect some
projectile collision/range paths. In the verified `AAHeatSeeker2` homing path,
there is no wall/cliff stop for the DRAGON missile.

Implementation implication: the first sim projectile slice should not collide
the GGI missile with walls or cliffs.

### `Shadow=no`

`Shadow=no` overrides the bullet type default shadow behavior. The projectile is
visible through `DRAGON.SHP` and its line trail, but it should not draw a normal
projectile shadow.

## 7. Speed and Travel

### Verified binary facts

`WeaponType.Speed` is stored into the bullet as target speed during
`BulletClass::Init`; for normal deployed GGI this is `30`, and for elite it is
`40`.

In the ROT>0 homing branch, `BulletClass::AI` computes current velocity length
and adjusts it toward the target speed using the projectile's acceleration.
`AAHeatSeeker2` does not specify `Acceleration`, so it uses the
`BulletTypeClass` default of `3`.

The verified movement behavior is:

- if current speed is below target speed, add acceleration and cap at target
- if current speed is above target speed, reduce by roughly half acceleration
  and clamp at zero
- normalize the velocity vector to the updated speed
- turn the vector toward the current target coordinate through
  `BulletClass::HomingTrack`
- integrate position and run collision/detonation checks

Therefore `Speed=30` is a simulation speed in world units per game tick, not a
render-only duration. A VERA20k implementation that picks a fixed visual
lifetime from muzzle-to-target distance is only an approximation and cannot
match moving targets or delayed damage.

## 8. Target Tracking and Invalid Targets

### Moving targets

`BulletClass::AI` re-reads the target coordinate every homing tick when
`BulletClass.Target` is non-null. The call uses the target object's coordinate
vtable entry and, for some object states, a center/alternate coordinate query.
That current coordinate is then passed to `BulletClass::HomingTrack`.

This verifies that `AAHeatSeeker2` tracks moving targets after launch.

Ground vehicle target:

- tracks the vehicle's updated coordinate each tick
- uses normal `Arm=2`
- passes the non-aircraft homing flag to `HomingTrack`

Rocketeer / `ConsideredAircraft=yes` target:

- is legal to attack because the fire-error/legality path sees it as air
- still tracks the target's updated coordinate each tick
- does not take the `WhatAmI == 2` aircraft-special arming branch if the object
  remains `InfantryClass`

Actual `AircraftClass` target:

- tracks updated coordinate each tick
- `BulletClass::Fire` passes arm delay `0`
- `BulletClass::AI` passes the aircraft-special flag to `HomingTrack`

### Target dies or becomes invalid

The homing AI has explicit guards for null target pointers and sentinel target
coordinates.

Verified behavior once the bullet sees no usable target coordinate:

- if the target pointer is null, the homing branch uses a sentinel target
  coordinate
- if target coordinate resolution returns the sentinel coordinate, the missile
  follows the lost-target branch
- if the target coordinate is sentinel and projectile height is at or above the
  global flight level, the bullet detonates
- otherwise the bullet continues through the same movement, approach, and
  proximity checks until another detonation condition is reached

Deferred internal detail: this pass did not fully trace the object-notification
or reference-clearing path that changes `BulletClass+0x10C` when a target object
is destroyed. The live `BulletClass::AI` behavior after the target is null or
returns sentinel coordinates is verified and is sufficient for the first
projectile pipeline implementation.

## 9. Rendering Facts

### Verified binary facts

The projectile is an object, not a one-shot render event.

`BulletClass::Fire @ 0x00468670` calls `ObjectClass::Reveal`, and the bullet is
submitted to the display/object system. `Image=DRAGON` supplies the projectile
image. `[DRAGON] Rotates=yes` means facing-driven SHP frame selection is part of
the object render path. DRAGON does not set `AnimLow`, `AnimHigh`, or an active
projectile animation frame range, so the visible missile is driven by facing and
position rather than by a looping animation sequence.

`[DRAGON] UseLineTrail=yes` creates the pale blue line trail through the inherited
ObjectType/LineTrail path. `LineTrailColor=216,216,255` and
`LineTrailColorDecrement=16` are art-section rendering values. `Trailer=SMOKEY2`
is commented out, so `BulletClass::AI` does not spawn a recurring SMOKEY2
`AnimClass` trailer for this projectile.

`Shadow=no` suppresses projectile shadow rendering.

The detonation visual comes from the warhead (`GUARDWH AnimList=...`) at the
bullet detonation coordinate, not from `DRAGON.SHP`.

Altitude is projectile state. `BulletClass.Location.Z`, terrain/height checks,
and target coordinate Z participate in movement, render placement, and
detonation decisions. It cannot be reconstructed correctly from a 2D app-side
fire event alone.

## 10. TS Legacy Check

No TS-only or dead legacy branch is required for the observed deployed GGI
missile lifecycle. The verified path is the standard YR `TechnoClass::Fire_At`
and `BulletClass::AI` path.

The one caution is `Proximity=`: it is a parsed projectile key, but for this
YR-live homing path it does not control whether `ProximityDetector::Check` runs.
This is not a TS legacy behavior to implement; it is a parsed-but-not-consumed
field for this path.

## 11. Current VERA20k Status

Current repo behavior, as inspected during this investigation:

- `src/rules/projectile_type.rs` parses the important projectile keys including
  `Image`, `ROT`, `Arm`, `Ranged`, `Proximity`, `AA`, `AG`, and
  `SubjectTo*`.
- `src/sim/combat/combat_weapon.rs` has target legality improvements for
  projectile `AA`/`AG`, including the recent `ConsideredAircraft=yes` combat
  classification fix.
- `src/sim/combat/mod.rs` still applies damage in the combat tick that emits
  `SimFireEvent`; there is no authoritative projectile lifetime between fire
  and damage.
- `src/app_fire_effects.rs` and `src/app_instances/overlays.rs` create a
  short-lived app-side `ProjectileVisual` from `SimFireEvent`, preload
  `DRAGON.SHP`, and draw the missile as a visual overlay.

What current VERA20k gets right:

- deployed GGI can select `MissileLauncher`
- `AAHeatSeeker2` is parsed as the projectile
- `Image=DRAGON` and the DRAGON asset are used for a visible missile
- `AA=yes` / `AG=yes` target legality is moving in the right direction
- `ConsideredAircraft=yes` matters for combat legality

What must be replaced:

- immediate damage on fire for projectile weapons
- app-owned missile lifetime as the only projectile state
- fixed visual duration derived from a fire event
- lack of projectile target tracking after launch
- lack of projectile arming/proximity/overshoot/approach detonation checks
- lack of sim-owned projectile position, velocity, facing, altitude, and removal
- detonation animation/sound timing tied to fire instead of impact

## 12. Minimal Safe Next Implementation Slice

The safest first VERA20k implementation slice is not a generic all-projectile
rewrite. Implement the narrow ROT>0 homing bullet path needed by
`AAHeatSeeker2`, while keeping the existing render overlay as a consumer of
sim projectile state.

Recommended slice:

1. Add deterministic sim projectile entities for weapons whose projectile has
   `ROT > 0`, starting with `AAHeatSeeker2`.
2. On weapon fire, create a projectile with:
   - owner entity id
   - target handle / last known target coordinate
   - projectile type id
   - weapon type id
   - warhead id
   - damage payload
   - current position and velocity
   - target speed from weapon `Speed`
   - arm/proximity detector state
   - line-trail/render metadata id, not render-owned lifetime
3. Delay damage until projectile detonation.
4. Each sim tick:
   - re-resolve target coordinate if target is still valid
   - adjust speed toward weapon `Speed` using projectile `Acceleration`
   - apply ROT homing turn, including the `ROT`/`MissileROTVar` sidewinder
     scaling before exact parity tuning
   - update position and facing
   - run arming/proximity/overshoot and close-target checks
   - detonate through normal warhead damage when impact conditions are met
5. Render DRAGON from projectile state:
   - facing-driven frame from `Rotates=yes`
   - no shadow
   - line trail from `[DRAGON] UseLineTrail=yes`
   - detonation anim from `GUARDWH AnimList` at impact, not at fire

Do not implement `Proximity=no` as a disable switch for the GGI missile. For
this YR-live path, the proximity detector is still active because `ROT=60`.

## 13. Coverage Ledger

| Question | Status | Evidence |
|---|---|---|
| Does deployed GGI spawn a BulletClass? | RESOLVED | `InfantryClass::Fire_At_Target` -> `TechnoClass::Fire_At` -> `BulletClass::Allocate` call site `0x006FE55D` |
| Is damage immediate? | RESOLVED | Real damage via `BulletClass::BulletDetonation @ 0x00468D80`; fire tick only updates estimated health bookkeeping |
| How is `Image=DRAGON` used? | RESOLVED | `BulletTypeClass::ReadINI @ 0x0046BEE0`, non-Inviso load, object render path |
| How are `ROT=60` and `Arm=2` used? | RESOLVED | ROT>0 homing branch in `BulletClass::AI`; arm passed to `ProximityDetector::Set/Arm` |
| Does `Proximity=no` disable the detector? | RESOLVED | Parsed but not consumed by this live homing path; detector still runs |
| How does `Speed=30` affect travel? | RESOLVED | Stored as bullet target speed and approached by acceleration in homing AI |
| Does it track moving targets? | RESOLVED | Target coordinate re-read every homing tick before `HomingTrack` |
| Ground vs Rocketeer behavior? | RESOLVED | Fire legality uses air classification; homing aircraft-special branch uses `WhatAmI==2` |
| Target destroyed/invalid behavior? | PARTIAL | Null/sentinel behavior verified; exact reference-clear notification path deferred |
| Rendering state source? | RESOLVED | Bullet object state drives position/facing/line trail; warhead drives impact anim |
| TS legacy risk? | RESOLVED | Normal YR path; no TS-only behavior required |

## 14. Open Questions Log - Final State

| ID | Question | Final state |
|---|---|---|
| OQ-GGI-AAH-001 | Does deployed GGI `MissileLauncher` spawn a projectile entity? | RESOLVED: yes, a `BulletClass` is allocated from `TechnoClass::Fire_At` call site `0x006FE55D`. |
| OQ-GGI-AAH-002 | Is damage applied at fire, impact, proximity detonation, arming expiry, or another event? | RESOLVED: real damage is applied at bullet detonation through `WarheadTypeClass::Detonate`; arming expiry only enables proximity/overshoot reporting. |
| OQ-GGI-AAH-003 | Are the listed projectile keys live for `AAHeatSeeker2`? | RESOLVED: `Image`, `ROT`, `Arm`, `Ranged`, `AA`, `AG`, `SubjectTo*`, and `Shadow` have live consumers; `Proximity` is parsed but not consumed by this homing path. |
| OQ-GGI-AAH-004 | What does `Weapon Speed=30` mean? | RESOLVED: it is the bullet target speed approached by the homing velocity update, not a render duration. |
| OQ-GGI-AAH-005 | Does the missile track moving targets after launch? | RESOLVED: yes, target coordinates are re-read every homing tick before `HomingTrack`. |
| OQ-GGI-AAH-006 | What differs between ground vehicles and Rocketeer/air targets? | RESOLVED: fire legality uses projectile air/ground gates; the homing aircraft-special branch uses `WhatAmI==2`, so Rocketeer legality and AircraftClass homing flags are separate concerns. |
| OQ-GGI-AAH-007 | What if the target dies or becomes invalid before impact? | PARTIAL: null/sentinel target behavior in `BulletClass::AI` is verified; the exact notice/reference-clear writer for `BulletClass+0x10C` was not traced in this slice. |
| OQ-GGI-AAH-008 | Which rendering facts are state-driven? | RESOLVED for pipeline needs: object position, velocity, facing, line trail, no shadow, altitude, and detonation animation timing are all projectile/warhead state, not fire-event-only state. |
| OQ-GGI-AAH-009 | Is this TS legacy? | RESOLVED: no, the traced path is live stock YR behavior. |

## Sources

- Live Ghidra decompilation of `gamemd.exe`
  - `InfantryClass::Fire_At_Target @ 0x005206B0`
  - `InfantryClass::SelectWeapon @ 0x005218E0`
  - `TechnoClass::GetFireError @ 0x006FC0B0`
  - `TechnoClass::Fire_At @ 0x006FDD50`
  - `TechnoClass::Resolve_ArchiveTarget_Coords @ 0x0070BCB0`
  - `BulletClass::Allocate @ 0x0046B050`
  - `BulletClass::Init @ 0x004664C0`
  - `BulletClass::Fire @ 0x00468670`
  - `BulletClass::AI @ 0x004666E0`
  - `BulletClass::HomingTrack @ 0x005B20F0`
  - `BulletClass::BulletDetonation @ 0x00468D80`
  - `BulletTypeClass::ReadINI @ 0x0046BEE0`
  - `ProximityDetector::Init @ 0x004E1100`
  - `ProximityDetector::Set/Arm @ 0x004E1130`
  - `ProximityDetector::Check @ 0x004E11F0`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`
- `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`
- `C:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini`
- `C:/Users/enok/Documents/ra2-rust-game/ini/art.ini`
- `BULLETCLASS_INIT_AND_FIRE_GHIDRA_REPORT.md`
- `BULLET_CLASS_AI_GHIDRA_REPORT.md`
- `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md`
- `BULLETTYPECLASS_GHIDRA_REPORT.md`
- `GGI_GHIDRA_REPORT.md`
- `LINE_TRAIL_CLASS_GHIDRA_REPORT.md`
