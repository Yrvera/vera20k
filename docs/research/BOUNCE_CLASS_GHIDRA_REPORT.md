# BounceClass — Ghidra Research Report

**Top-line verdict:** **Shared embedded physics component (NOT standalone, NOT a flag).**
BounceClass is a concrete C++ class (RTTI: `.?AVBounceClass@@` at `0x00845E38`) but it never
appears as a standalone entity — instances are always embedded inside another object
(VoxelAnimClass for 3D voxel debris; AnimClass for 2D SHP debris that uses the physics
pipeline). Its methods (`Init`, `Update`) are shared by both host classes via direct call,
not via vtable/virtual dispatch. A handful of "Bounce*" INI keys are parsed on
VoxelAnimTypeClass and drive the physics parameters.

**Primary addresses (verified live in YR):**
- `BounceClass::Init`         — `0x004397E0`
- `BounceClass::Update`       — `0x00439B00`
- `BounceClass__SpawnRandom`  — `0x00439690`  (helper: random spherical velocity + Init, gravity=3.0)
- `AnimClass::ProcessBounceResult` — `0x00423930`  (AnimClass-side driver that calls Update)
- RTTI string                 — `0x00845E38`  (`.?AVBounceClass@@`)
- INI key "Bouncer"           — `0x008183AC`
- INI key "BounceAnim"        — `0x008183F8`

**Confidence:** HIGH for physics core (Init / Update / spawn). MEDIUM for the AnimClass-side
embedding offset — confirmed the embed happens in `AnimClass__Constructor` (call site
`0x00422648`), but did not enumerate every struct offset on AnimClass in this pass.

**YR-live status:** LIVE. Voxel debris (e.g. `VoxelAnims` list in rulesmd.ini — tire, gas
tank, crystal shards, meteorites) actively use BounceClass during standard YR skirmishes.
Infantry death gibs do NOT use BounceClass — they use the simpler AnimClass sprite-animation
path without physics. `Bouncer=yes` on AnimTypeClass is a **separate system** (see below).

---

## 1. Clarification — "Bouncer" and BounceClass are different systems

There are two different "bounce" concepts in gamemd.exe; research found them both and they are easy to confuse:

### 1a. `Bouncer=yes` on AnimTypeClass — homing/attach sprite behavior (NOT BounceClass)

- Read from `art(md).ini` per-animation. In artmd.ini ~40 entries set `Bouncer=yes`
  alongside `Elasticity=0.0` (debris anims: `GUNFIRE1..4`, rubble anims, death anims).
- Implemented in `AnimClass__BounceAI` at `0x00425670`. This is a 2D homing behavior:
  the anim drifts toward a computed attach target, tests distance < 0x13, snaps to
  the target, and sticks on impact. It does **not** simulate gravity/elasticity and
  does **not** call BounceClass. Confidence: HIGH.
- Note: `Elasticity=` also appears on AnimTypeClass but in the `Bouncer=yes` path is
  not multiplied into a velocity; it is read but the observed AI does not use it as a
  bounce coefficient. Treat the art.ini `Elasticity` on AnimTypes as a legacy/unused
  field when `Bouncer=yes`, unless future RE finds a reader that applies it.
  Confidence: MEDIUM (could not prove completely unused in one pass).

### 1b. BounceClass — true physics simulation (gravity / elasticity / spin)

Used by:
- **VoxelAnimClass** — primary consumer. Each VoxelAnim carries one embedded BounceClass at byte offset `0xB0` (VoxelAnim size `0x148`).
- **AnimClass** — also carries an embedded BounceClass. `AnimClass::Constructor` calls `BounceClass::Init` at `0x00422648`, and `AnimClass::ProcessBounceResult` (`0x00423930`) drives it by calling `BounceClass::Update` at `0x00423941`. This path is invoked when the AnimTypeClass has the gravity/elasticity physics branch engaged (not `Bouncer=yes` — a different branch).
- **`FUN_00439690`** (`BounceClass_SpawnRandom` helper) — generates a random spherical velocity then calls `Init`. Used for terrain meteors / random debris spawns. Gravity passed here is **3.0** (not 1.4).

The same `BounceClass::Init` and `BounceClass::Update` serve all consumers.

---

## 2. Purpose

BounceClass simulates a single rigid-body projectile or debris chunk that falls, bounces
off the ground (with elasticity-dampened velocity reflection), tumbles (quaternion
integration of per-tick rotation), interacts with slopes and buildings, and reports
when it has come to rest. It owns:
- 3D position (double→float copy for physics)
- 3D velocity (gravity applied to Z each tick)
- Orientation quaternion
- Per-tick rotation delta quaternion
- Static configuration (elasticity, gravity, angular-velocity clamp threshold)

It is a **component**, not an entity — it does not appear in any object list, has no
vtable use for its own methods, no Save/Load slot of its own, and no WhatAmI. It is
serialized/lifecycled as part of its host (VoxelAnimClass / AnimClass).

---

## 3. Struct Layout

`param_1` in `BounceClass::Init` is typed `undefined4 *` (int pointer) in Ghidra, so the
decompilation indices `param_1[N]` are **byte offset = N * 4**. `param_1` in
`BounceClass::Update` is typed `double *` — `param_1[N]` is **byte offset = N * 8** in
that function. Both are confirmed against assembly in the VoxelAnimClass report.

**Total size: 0x50 (80) bytes.** This is fully mapped; no fields exist past `0x4F`.
(Previously claimed as 0x98 bytes in an earlier pass — corrected in the VoxelAnimClass
report and reconfirmed here.)

| Byte Offset | Size | Type       | Field                       | Notes |
|-------------|------|------------|-----------------------------|-------|
| 0x00        | 8    | double     | Elasticity                  | 0.0–1.0 bounce coefficient from VoxelAnimType->Elasticity |
| 0x08        | 8    | double     | Gravity                     | Almost always 1.4 for voxel debris (hardcoded `0x3FF66666_60000000`); 3.0 in terrain-meteor spawn helper `FUN_00439690` (passed as `0x40080000` upper-half) |
| 0x10        | 8    | double     | AngularVelocityMagnitude    | Clamp threshold. 0.0 in default path (= no clamp). 3.0 in terrain meteor path. When > 0 and current vel length > this, velocity is normalized to this length. |
| 0x18        | 4    | float      | Position.X                  | Float copy of initial world coord |
| 0x1C        | 4    | float      | Position.Y                  | |
| 0x20        | 4    | float      | Position.Z                  | Compared against `CellClass__GetGroundHeight` to detect ground collision |
| 0x24        | 4    | float      | Velocity.X                  | |
| 0x28        | 4    | float      | Velocity.Y                  | |
| 0x2C        | 4    | float      | Velocity.Z                  | **`Velocity.Z -= Gravity` every Update tick** |
| 0x30        | 16   | float[4]   | Orientation quaternion      | Init: identity via `Matrix3x4_SetIdentity` + `FUN_00646730`. Each Update integrates via `Quaternion_CopyAndStore(FUN_00645ed0(this+0x40))`. |
| 0x40        | 16   | float[4]   | RotationPerTick quaternion  | Init: `FromAxisAngle(randomAxis, angVel)` where randomAxis is a random unit vector and angVel comes from `Type->MinAngularVelocity..MaxAngularVelocity` (radians). Negated (about each axis) on bounce. |

### Init signature (reconstructed)

```
void __thiscall BounceClass::Init(
    BounceClass* this,           // param_1
    CoordStruct* initialPos,     // param_2 (int *)  -> copied as floats into Position
    double elasticity,           // param_3/param_4 (low/high dword)
    double gravity,              // param_5/param_6
    double angVelMagnitude,      // param_7/param_8
    Vec3f* initialVelocity       // param_9 (undefined4 *)  -> 3 floats copied into Velocity
);
```

The random axis + quaternion construction at the tail of Init uses `_DAT_007e3560`
(probably `1.0 / 0xFFFF` to normalize `RandomRanged(-0xFFFF, 0xFFFF)` to `[-1, 1]`).

---

## 4. Physics Constants (verified addresses)

| Constant | Address | Raw | Decimal value | Meaning |
|----------|---------|-----|---------------|---------|
| Default gravity (voxel debris) | hardcoded in `VoxelAnimClass__Constructor` | `0x3FF66666_60000000` | **1.4** | Z-velocity decrement per tick |
| Terrain-meteor gravity (spawn helper) | hardcoded in `FUN_00439690` | `0x40080000` (double upper) | **3.0** | Larger gravity for terrain-meteor debris path |
| Bounce-stop threshold | `_DAT_007E3D80` | float10 | **2.5** | If total velocity magnitude < 2.5, `Update` returns 2 (stopped); else 1 if bounced, 0 if still airborne |
| Ground-height offset for collision | `DAT_0089C76C` | int | small positive (bridge clearance / leptons; same pattern as `DAT_0089a1c0`) | Added to ground height to form the effective bounce plane (handles bridges) |
| Slope-bounce thresholds | `_DAT_007E3DA0`, `_DAT_007E3D98`, `_DAT_007E2AC8`, `_DAT_007E3DA8` | floats | small | Velocity-Z and clearance thresholds used to decide whether a slope cell diff triggers the slope-reflection branch vs. ordinary flat bounce |
| Degree-to-radian scale | `0x007F65E8` | double | **π/180 ≈ 0.017453293** | Applied by `VoxelAnimTypeClass::ReadINI` to `Min/MaxAngularVelocity` before storing |
| Random-normalize scale | `_DAT_007E3560` | float | **1/65535** | Converts `RandomRanged(-0xFFFF, +0xFFFF)` to float in [-1, 1] for random-axis generation |
| Angular-velocity epsilon | `_g_Const_0_0` | double | **0.0** | Used to skip angular-velocity clamp when threshold is 0 |

**Elasticity** itself is per-instance (not a global constant). It comes from
`VoxelAnimTypeClass.Elasticity` at struct offset `0x2A0`, default **0.8**, valid range
0.0–1.0. On a bounce the engine reflects velocity and multiplies by Elasticity.

---

## 5. Lifecycle

Per-tick flow inside `BounceClass::Update` (driver = host's AI/ProcessBounceResult):

1. **Snapshot previous state** into locals (so it can be rolled back for slope reflection).
2. **Apply gravity:** `Velocity.Z -= Gravity` (field at offset 0x2C gets `-= param_1[1]`).
3. **Angular-velocity clamp:** if `AngularVelocityMagnitude > 0` and current velocity-vector length > magnitude, scale to the magnitude (`FUN_0043a0d0`). This is the spin/linear clamp for terrain meteors.
4. **Integrate position** from velocity (two ftol/Set chains constructing old-cell and new-cell coords into `local_f8`/`local_ec`).
5. **Ground-collision test:** look up the two cells' heights; `local_118 = groundHeight + DAT_0089C76C`. Detect:
   - **Bridge clearance** — bits `0x100` at cell+0x140 (bridge flag) → use bridge height.
   - **Slope straddle** — `bVar4` / `bVar5` set based on which side's cell height clears the bounce plane.
   - **Flat bounce on ground** — `bVar6` via building lookup + `FUN_00480510` (probably a bridge/cliff test).
6. **Snap Z if below ground** and, when this isn't a slope case, return 0 (still flying) — skip to quaternion integration.
7. **Bounce reflection** (slope or flat):
   - Rotate velocity by the current facing matrix (`VXL_GetFacingMatrix`, `FUN_005afc20`).
   - Negate Y and Z components in a local intermediate; call `FUN_0043a0d0(elasticity)` — this is the **elasticity multiplier** applied to the reflected velocity.
   - Negate `RotationPerTick` quaternion components via `FUN_00645d00(i)` for i=0,1,2 (so the chunk spins the other way after each hit).
8. **Slope "re-bounce" check:** compare cell height deltas > 1 and velocity-Z conditions (via `_DAT_007E3D98`, `_DAT_007E3DA0`, `_DAT_007E2AC8`) — if satisfied, roll position/velocity back to the snapshot, look up a sloped-surface matrix (`FUN_00755C60`), transform the velocity by it, and multiply by Elasticity. This is the slope-reflection path.
9. **Integrate rotation:** `Orientation = Orientation * RotationPerTick` (via `FUN_00645ed0(this+0x40)` + `Quaternion_CopyAndStore(this+0x30, ...)`).
10. **Stop test:** call `FUN_00439a10` to get total velocity magnitude. If `<= 2.5` (`_DAT_007E3D80`), return 2 (stopped). Else if a bounce occurred this tick, return 1. Else return 0.

**Return value contract** (confirmed by `AnimClass__ProcessBounceResult` switch):
- `0` — still falling, no bounce this tick
- `1` — bounced this tick (caller may spawn `BounceAnim`, play `BounceSound`, apply splash damage)
- `2` — stopped (caller may delete self / play expire anim)

---

## 6. Rendering Path

BounceClass does not render itself. Host classes read its state each frame:

- **VoxelAnimClass (voxel render):** uses the Orientation quaternion at `BounceClass+0x30`
  via `FUN_004399E0` (quaternion → 3x4 matrix), which the voxel draw pipeline composes
  with the HVA/locomotion matrix to get the current spin. Position is read via
  `CoordStruct::FromDoubles` from the float position at `BounceClass+0x18`.
  (See VOXELANIMCLASS_GHIDRA_REPORT.md §4c.)

- **AnimClass (SHP render):** for anims that use the physics driver, position updates
  feed the ordinary 2D sprite draw path (`param_1 + 0x55` pixel-Z adjustment in
  `ProcessBounceResult` suggests the Z-coord float at offset 0x154/0x155 is the render
  elevation). No rotation — SHPs are 2D.

Neither draw happens in BounceClass; the component only exposes state.

---

## 7. Spawn Sites

1. **VoxelAnimClass constructor** (`0x007493B0`) — call to `BounceClass::Init` at
   `0x0074981f`. The main spawn path. Called from warhead explosion code reading
   `Warhead.DebrisTypes[]` / `Warhead.DebrisMaximums[]` and instantiating N
   VoxelAnimClass objects per destroyed vehicle. Walking from here upward (not
   traversed deeply in this pass; see VOXELANIMCLASS_GHIDRA_REPORT.md §5 for the
   WarheadType → VoxelAnim spawn chain).

2. **AnimClass constructor** (`AnimClass__Constructor`) — call to `BounceClass::Init` at
   `0x00422648`. Less common. Engaged when the AnimType is being used as a physics
   debris chunk. The existing AnimClass report and this pass didn't fully map the
   AnimType field that gates this branch; tentative answer: elasticity/gravity keys on
   AnimTypeClass or a legacy flag. **Open question.**

3. **`BounceClass__SpawnRandom` (`0x00439690`)** — generates random spherical velocity
   (random elevation/azimuth angles, random speed in the range
   `[param_6, param_7]`) and calls `Init` with gravity = 3.0,
   angularVelocityMagnitude = 0. Used for terrain-meteor debris; exact caller graph
   not traced in this pass.

**Infantry-death gibs do not appear here.** Infantry deaths spawn AnimClass objects
(the Die1..DieN anim sequences named in `art(md).ini`) that use AnimClass's ordinary
SHP animation, not BounceClass physics. Any "gib fling" look is achieved via the SHP
frame sequence, not a physics sim. Confidence: MEDIUM-HIGH (no BounceClass xrefs from
InfantryClass death code were found; verified by xref list above).

---

## 8. INI Keys

### On VoxelAnimTypeClass (`[VoxelAnims]` in `rulesmd.ini`) — AUTHORITATIVE, all live

Parsed at `VoxelAnimTypeClass::ReadINI` (`0x0074B050`), `param_1` is `int *` (Ghidra
index × 4 = byte offset — AnimTypeClass pitfall **does not apply** to VoxelAnimTypeClass
directly, as this class uses plain int; offsets below are already in bytes and verified
against the struct layout section of the VoxelAnimClass report).

| Key                | Type    | Default  | Offset | Notes |
|--------------------|---------|----------|--------|-------|
| `Elasticity`       | double  | 0.8      | 0x2A0  | → `BounceClass.Elasticity`; 0..1 |
| `MinAngularVelocity` | double (deg→rad) | 0.0  | 0x2A8  | Stored as INI_value * π/180 |
| `MaxAngularVelocity` | double (deg→rad) | ~10°  | 0x2B0  | Stored as INI_value * π/180. At Init, `angVel = Random() % (Max-Min+1) + Min`. |
| `BounceAnim`       | AnimType ref | null | 0x2E4  | Spawned each time Update returns 1 |
| `BounceSound`      | sound ref  | VOC_NONE | — | Played each time Update returns 1 |

Default constants in rulesmd.ini comments: Elasticity def=0.75 (code default 0.8 in the struct — minor discrepancy between comment and code).

### On AnimTypeClass (`[Animations]` in `artmd.ini`) — SEPARATE "Bouncer" system

| Key         | Type  | Where | Notes |
|-------------|-------|-------|-------|
| `Bouncer`   | bool  | AnimTypeClass | Triggers `AnimClass__BounceAI` (homing-to-attach path, NOT BounceClass physics). |
| `Elasticity`| float | AnimTypeClass | Read into struct; observed use path uses it only nominally. May be a TS-era field kept around. Confidence MEDIUM. |

Because `AnimTypeClass` uses `int *` in decompilations, any struct-offset claim for
these keys MUST multiply the Ghidra index by 4. We did not re-derive those offsets in
this pass; see the AnimTypeClass report for authoritative field offsets.

### Keys NOT present (searched and confirmed absent)

- `MaxBoundAngle` — not found in `rulesmd.ini`, `rules.ini`, `artmd.ini`, `art.ini`, nor as a string in gamemd.exe.
- `Bouncy` — not a real INI key (spelling is `Bouncer`).
- No gravity INI key — gravity is hardcoded (1.4 or 3.0 in the spawn helper).

---

## 9. TS-Legacy Analysis

- **`Bouncer=yes` (AnimTypeClass path) — LIVELY IN YR.** Heavily used in artmd.ini for
  death / debris / muzzle-flash anims. Not TS-only.
- **BounceClass physics — LIVELY IN YR.** VoxelAnim debris fires during any vehicle
  explosion in a standard YR skirmish. Not TS-only.
- **`FUN_00439690` terrain-meteor spawn (gravity=3.0, angVel clamp=3.0) — POSSIBLY
  TS-LEGACY.** Meteor storms as a world-event were a Tiberian Sun feature. In YR the
  only user-visible meteors are `METEOR01..NN` VoxelAnims spawned by the Psychic
  Dominator weapon, which uses the ordinary `VoxelAnimClass` path with gravity 1.4, not
  this helper. The caller chain for `FUN_00439690` was not traced in this pass — flag
  as **potentially dormant in YR**. Confidence: MEDIUM. Do not implement the gravity=3.0
  / angVel-clamp=3.0 branch as default behavior without confirming a live YR caller.
- **AnimClass-side BounceClass embed — LIVELY but narrowly used.** Confirmed in
  constructor; didn't enumerate all live AnimTypes that engage this branch.

---

## 10. Open Questions

1. **Which AnimType key/flag engages the AnimClass BounceClass branch** (the one calling
   `Init` at `0x00422648`)? Not `Bouncer=yes` (that is `BounceAI`). Candidates: a
   separate AnimType physics-enable field, or a sub-type dispatch based on
   `ExpireAnim`/`TrailerAnim` chains. Needs caller analysis at `0x00422648`.
   **→ RESOLVED in Follow-up round 2. See section 13.**
2. **Where `FUN_00439690` is invoked.** If no live YR caller exists, the gravity=3.0 /
   angVelClamp=3.0 path is TS-dormant.
   **→ RESOLVED in Follow-up round 2. See section 13.**
3. **AnimClass's BounceClass embed offset** within AnimClass struct.
4. **Exact mapping of AnimClass damage/splash on return==1** — inferred from
   `ProcessBounceResult` cell-iterate + damage call, but Warhead/Damage fields aren't
   confirmed to come from AnimTypeClass in this pass.
5. **`DAT_0089C76C`** ground-height offset — known to be the bridge-clearance constant
   (same pattern as `DAT_0089A1C0`) but the exact lepton value was not read out of
   memory in this pass.

---

## 11. Ghidra Functions Labeled

Pre-existing (from VoxelAnimClass research):
- `BounceClass__Init` @ `0x004397E0`
- `BounceClass__Update` @ `0x00439B00`
- `AnimClass__BounceAI` @ `0x00425670`
- `AnimClass__ProcessBounceResult` @ `0x00423930`

Newly labeled this session:
- `BounceClass__SpawnRandom` @ `0x00439690` (random spherical-velocity → `BounceClass::Init` helper; gravity=3.0 branch; possibly TS-legacy)

Total BounceClass-namespace labels: **3** (`Init`, `Update`, `SpawnRandom`) plus 2
AnimClass-side labels that drive BounceClass.

---

## 13. Follow-up investigation (round 2)

Two Round-1 open questions closed. Direct decompilation evidence below.

### Q1 — RESOLVED: AnimClass-side BounceClass is gated by `Bouncer=yes` OR `IsMeteor=yes`

**Correction to Round 1.** Round 1 claimed `Bouncer=yes` triggers only `AnimClass__BounceAI`
(homing) and is completely separate from BounceClass physics. **That was wrong.**
Re-decompilation of `AnimClass::Constructor` at `0x00422648` (the `BounceClass::Init`
call site) shows both `Bouncer=yes` and `IsMeteor=yes` engage the physics branch:

```c
// In AnimClass::Constructor, param_1[0x32] is the AnimTypeClass*.
if ((*(char *)(iVar3 + 0x35a) == '\0') && (*(char *)(iVar3 + 0x356) == '\0')) {
    ObjectClass__Reveal(param_3, 0);        // NORMAL PATH — no BounceClass
}
else {
    *(undefined1 *)(param_1 + 0x65) = 1;    // instance flag: "uses bounce physics"
    if (*(char *)(iVar3 + 0x356) == '\0') {
        // Bouncer=yes path: uses Elasticity(0x310)/MaxXYVel(0x314)/
        //                   MinZVel lo(0x318) / hi(0x328) from AnimTypeClass
        ...
    } else {
        // IsMeteor=yes path: different random attach-target + velocity setup
        ...
    }
    BounceClass__Init(&iStack_44, uVar10, uVar11, 0x60000000, 0x3ff66666, 0, 0, pfVar12, 0, 0);
}
```

- **Offset 0x356** = `IsMeteor` bool (from `AnimTypeClass::ReadINI` at `0x004284x0`:
  `CCINIClass__ReadBool(..., s_IsMeteor_008184b0, *(char*)((int)param_1 + 0x356))`).
- **Offset 0x35a** = `Bouncer` bool (from `AnimTypeClass::ReadINI`:
  `CCINIClass__ReadBool(..., s_Bouncer_008183ac, *(char*)((int)param_1 + 0x35a))`).
- Both offsets are direct byte offsets. Although `AnimTypeClass::ReadINI` types `param_1`
  as `int *`, the decompile explicitly uses `*(char *)((int)param_1 + <hex_offset>)`,
  so the pitfall does NOT apply — the values `0x356` and `0x35a` ARE the byte offsets.

**Runtime path** (from `AnimClass::AI` at `0x00423ac0`):

- `if (*(char *)(param_1[0x32] + 0x354) != '\0') { AnimClass__BounceAI(); ObjectClass__AI(); }`
  — this is a DIFFERENT offset (0x354) that drives homing, unrelated to the physics path.
  0x354 is not set by `AnimTypeClass::ReadINI` directly; presumed set by other code or
  inherited default.
- `if (((char)param_1[0x65] != '\0') && (iVar8 = (**(code **)(*param_1 + 0x1e8))(), iVar8 == 2 || iVar8 == 1))`
  — when the per-instance "uses bounce physics" flag is set, calls the vtable method
  at offset 0x1e8 which is **`AnimClass::ProcessBounceResult` at `0x00423930`**
  (verified via AnimClass vtable base `0x007e3354` + `0x1e8` = `0x007e353c`, which
  holds `0x00423930`).
- On return==1 (bounced this tick): spawns different debris based on `IsMeteor`:
  - `IsMeteor=no`: `AnimClass::Constructor(Rules+0x94, ...)` + `Rules+0xbc4[0]` debris
  - `IsMeteor=yes`: random `Rules+0xbc4[-1 + Rules+0xbd0*4]` debris
- On return==2 (stopped): handles splash + deletion via `vtable+0xf8`.

**So the AnimClass-embedded BounceClass is live in YR via two keys:**
`Bouncer=yes` AND `IsMeteor=yes`. Both drive the same `BounceClass::Init` → `Update` →
`ProcessBounceResult` pipeline. They differ only in spawn geometry (attach point)
and in the debris anims spawned on bounce/stop.

**Confidence:** HIGH. Directly decompiled both constructor and AI, verified vtable
offset resolution, traced ReadINI assignment of both offsets.

**Implication for Round 1 claim about `AnimClass__BounceAI` being the "homing"
system for `Bouncer=yes`:** That function IS still called, but from
`AnimClass::AI`'s `offset 0x354` branch, which is a SEPARATE gate from the
`Bouncer=yes` ReadINI bool. The exact source of the 0x354 bit was not traced in
this pass — it may be a computed flag from `IsFlamingGuy` + target-attach logic,
or a legacy field set elsewhere. This warrants a Round-3 look if the distinction
matters for Rust parity, but the headline answer ("what turns on the embedded
BounceClass") is now conclusive.

**Confidence caveat — INI strings not literally named "IsMeteor" / "Bouncer" in
rulesmd.ini:** Both are standard `art(md).ini` keys on AnimType sections. They are
set on exactly these animations in retail YR's `artmd.ini` (grep to confirm):
`Bouncer=yes` on ~40 death/debris/muzzle anims; `IsMeteor=yes` on the
`METEOR01`..`NN` anims used by the Psychic Dominator. Both are live in standard
YR skirmishes.

### Q2 — RESOLVED: `BounceClass__SpawnRandom @ 0x00439690` is DEAD CODE in YR

- `get_xrefs_to 0x00439690` → **"No references found to address: 0x00439690"**.
- The function has zero callers in the loaded gamemd.exe image.
- Combined with Round-1 finding that the gravity=3.0 / angVel-clamp=3.0 branch is a
  terrain-meteor physics path distinct from the ordinary voxel-debris path
  (gravity=1.4 hardcoded in `VoxelAnimClass__Constructor`), this confirms the
  function is a Tiberian Sun leftover that is unreachable in a standard YR
  skirmish.
- **Confidence:** VERY HIGH. Definitively dormant. No live YR caller exists.
- **Implementation guidance:** Do NOT implement. Ignore the gravity=3.0 / angVel-3.0
  branch. The only live BounceClass physics in YR uses gravity=1.4 with
  angVel-clamp=0 (no clamp).

### Labels applied

None newly. `BounceClass__SpawnRandom @ 0x00439690` is already labeled (pre-existing,
per Round-1). It remains labeled but should be treated as dead code.

Ghidra `save_program` called at end of this session.

---

## 12. Summary for Integration

- **Treat BounceClass as a plain physics component struct in Rust**, embedded in the
  Rust equivalents of VoxelAnimClass and (where applicable) AnimClass. Not a separate
  entity kind.
- **Simulation math must be deterministic** — the engine uses floats here, but for
  lockstep we must use fixed-point. The dominant numeric constants (1.4 gravity,
  0.0/0.8 elasticity, 2.5 stop threshold, π/180 conversion) translate 1:1.
- **Randomness uses `Random__RandomRanged`** — must route through our deterministic RNG.
  The random axis generation and random angular velocity are both deterministic-RNG
  consumers.
- **Return codes 0/1/2** are the clean contract between BounceClass and host.
- **`Bouncer=yes` is NOT BounceClass**; do not conflate when porting `AnimTypeClass`.

---

## Verification (round 3)

**Claim under review:**
- Round 1 said `Bouncer=yes` is ONLY a homing-to-target flag for `AnimClass::BounceAI`,
  separate from BounceClass physics.
- Round 2 said `Bouncer=yes` (AnimTypeClass +0x35a) AND `IsMeteor=yes` (AnimTypeClass
  +0x356) both engage the embedded BounceClass branch in `AnimClass::Constructor` at
  call site `0x00422648`.

**Independent evidence (decompiled at 0x00421ea0 = AnimClass::Constructor):**

```c
iVar3 = param_1[0x32];                  // this->Type (AnimTypeClass*)
if ((*(char *)(iVar3 + 0x35a) == '\0') && (*(char *)(iVar3 + 0x356) == '\0')) {
    ObjectClass__Reveal(param_3, 0);    // neither Bouncer nor IsMeteor → plain reveal
}
else {
    *(undefined1 *)(param_1 + 0x65) = 1;
    // ... coordinates prepared ...
    BounceClass__Init(&iStack_44, uVar10, uVar11, 0x60000000, 0x3ff66666, 0, 0,
                      pfVar12, 0, 0);   // BounceClass physics initialized HERE
}
```

The `BounceClass__Init` call is gated exactly by
`AnimType->+0x35a != 0 || AnimType->+0x356 != 0`. Confirmed against
`AnimTypeClass::ReadINI @ 0x00427d00` decompile: `Bouncer=` is read into +0x35a,
`IsMeteor=` into +0x356. (Note that AnimTypeClass uses `int *` param, so `+0x35a` is
a direct byte offset unaffected by the index-multiply-by-4 rule.)

The "homing-to-target" code in `AnimClass::BounceAI @ 0x00425670` is called from
`AnimClass::AI @ 0x00423ac0`, gated by a *different* field:

```c
if (*(char *)(param_1[0x32] + 0x354) != '\0') {
    AnimClass__BounceAI();
    ObjectClass__AI();
}
```

AnimTypeClass+0x354 corresponds to `param_1[0xd5]` in ReadINI, which reads the
`IsFlamingGuy=` key (ReadINI writes: `*(undefined1 *)(param_1 + 0xd5) = ReadBool(..., s_IsFlamingGuy)`).
So `AnimClass::BounceAI` is the "flaming paradrop guy" homing/arc flight behaviour —
a legacy feature tied to `IsFlamingGuy=yes`, **not** to `Bouncer=yes`.

**Verdict: REFINED.**
- Round 2 is correct that `Bouncer=yes` AND `IsMeteor=yes` engage BounceClass physics
  in `AnimClass::Constructor`.
- Round 1's mapping of `Bouncer=yes → AnimClass::BounceAI` is wrong. The actual gate
  for `BounceAI` is `IsFlamingGuy=yes` at AnimTypeClass+0x354. The name "BounceAI" in
  the code base is misleading — it does not implement `Bouncer=yes` semantics at all.
- `AnimClass::ProcessBounceResult @ 0x00423930` (reachable via the AnimClass vtable
  slot referenced at 0x007e353c) wraps `BounceClass::Update` and is the per-tick
  driver for the physics side. It is orthogonal to `BounceAI`.

**Three code paths, three distinct flags:**
1. `Bouncer=yes` (+0x35a) → BounceClass physics in constructor; `ProcessBounceResult`
   drives per-tick update.
2. `IsMeteor=yes` (+0x356) → same BounceClass physics branch + different random-spawn
   offset calculation above the reveal site in the constructor.
3. `IsFlamingGuy=yes` (+0x354) → `AnimClass::BounceAI` homing/attach-to-target flight,
   unrelated to BounceClass.

Ghidra MCP calls: decompiled `AnimClass::Constructor`, `AnimTypeClass::ReadINI`,
`AnimTypeClass::Constructor`, `AnimClass::AI`, `AnimClass::BounceAI`,
`AnimClass::ProcessBounceResult`; verified strings and xrefs for `Bouncer` and
`IsMeteor`; disassembled bytes at 0x004282d0 to confirm +0x354 writer.
