---
title: Bullet Projectile System — Consolidated Research Report
date: 2026-04-23
---

# Bullet Projectile System — Consolidated Research Report

**Scope:** The complete gamemd.exe projectile pipeline — from `BulletTypeClass` INI
parsing, through construction and `BulletClass::Fire` (launch), to `BulletClass::AI`
(per-tick flight), to `BulletClass::BulletDetonation` (impact).

**Confidence (overall):** High. Primary functions decompiled end-to-end; constants,
offsets, and INI keys cross-checked against the binary.

**Active in YR:** Yes — every weapon discharge that has a `Projectile=` entry in
`rulesmd.ini` runs this pipeline. A handful of keys / code paths are Tiberian Sun
carry-overs and are flagged inline.

---

## 1. How this report relates to prior research

This is a **consolidation + gap-fill report**. Three detailed reports already cover
large parts of this system and should be read first:

| Prior doc | What it covers | Status |
|-----------|----------------|--------|
| `ra2-rust-game-docs/BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` | BulletClass runtime instance layout (0x00-0x15F, size 0x160) | Accurate |
| `ra2-rust-game-docs/BULLET_CLASS_AI_GHIDRA_REPORT.md` | `BulletClass::AI` per-tick update (arcing, straight, homing paths) | Accurate |
| `ra2-rust-game-docs/BULLETCLASS_TRAJECTORY_AND_HOMING.md` | Trajectory math, homing algorithm, airburst, cluster, shrapnel | Accurate; one struct-layout note corrected in §11 below |
| `ra2-rust-game-docs/BULLETTYPECLASS_GHIDRA_REPORT.md` | **Authoritative BulletTypeClass struct + ReadINI report (added 2026-04-24)** | Supersedes §3.3 and §10.1 of this report — see notices in those sections |

> **⚠ Partial supersession (2026-04-24):** Sections §3.3, §3.4, and §10.1 of this
> report attributed a set of art-side keys (`StartSound`, `StopSound`, `BounceAnim`,
> `ExpireAnim`, `TrailerAnim`, `TrailerSeperation`, `DamageRadius`, `Warhead`,
> `Bouncer`, `Tiled`, `ShouldUseCellDrawer`, `UseNormalLight`, `SpawnsParticle`,
> `NumParticles`, `RandomRate`, `YDrawOffset`, `ZAdjust`) to a phantom
> `BulletTypeClass::ReadINI_Part2` at `0x00428319`. **That function does not
> exist** — the address is mid-stream code inside `AnimTypeClass::ReadINI`
> (`0x00427D00` – `0x004287F5`), and those keys belong to AnimTypeClass, not
> BulletTypeClass. See `BULLETTYPECLASS_GHIDRA_REPORT.md` §5 for full evidence
> (zero callers, out-of-bounds offsets — BulletType is 0x2F8 bytes, the phantom
> writes to +0x300+, AnimType is 0x378). Inline notices added to the affected
> sections below.

This report adds what those did **not** cover:

1. **Full BulletTypeClass ReadINI** — every key, address, and default (§3, §4).
   *(For §3, prefer the dedicated `BULLETTYPECLASS_GHIDRA_REPORT.md`. §4 defaults
   table remains accurate.)*
2. **BulletTypeClass constructor defaults** (§4) — critical for missing keys.
3. **`BulletClass::Fire` (0x00468670)** — the launch function that the prior docs
   referenced but never documented (§5).
4. **`Arm=` field wiring** — the missing link between `BulletTypeClass+0x2F0`
   and `ProximityDetector::ArmingDelay` (§5.3).
5. **Inviso bullet behavior** — raycast-to-target launch path, velocity zeroed (§5.4).
6. **`[General]` ballistic keys** in `RulesClass` (BallisticScatter, HomingScatter,
   MissileSpeedVar, MissileROTVar, MissileSafetyAltitude, FlightLevel) with offsets (§6).
7. **Proximity (0x29F) vs Ranged (0x2A0)** — one is dead-read, the other gates AI (§7).
8. **Rust implementation status + concrete gaps** (§9), including one confirmed bug
   in `src/rules/projectile_type.rs`.

---

## 2. End-to-end lifecycle summary

```
         ┌─────────────────────────────────┐
         │  BulletTypeClass::ReadINI       │  — parses [Invisible], [Cannon], … from rulesmd/artmd
         │    0x0046bee0                   │    (once at game start; 37 keys; see §3)
         └──────────────┬──────────────────┘
                        │
                        ▼ (per weapon discharge)
         ┌─────────────────────────────────┐
         │  TechnoClass::Fire_At           │  — selects weapon, validates shot, creates bullet
         │    0x006FDD50                   │    (see FIRE_AT_ANALYSIS.md, FIRE_AT_PIPELINE)
         └──────────────┬──────────────────┘
                        │
                        ▼
         ┌─────────────────────────────────┐
         │  BulletClass::Allocate +        │  — operator_new(0x160) + constructor
         │  Constructor (0x00466380)       │    → registers in global bullet array DAT_00A8ED40
         │  + Init (0x004664C0)            │    → writes Type, Owner, Target, Damage, WH, Speed
         └──────────────┬──────────────────┘
                        │
                        ▼
         ┌─────────────────────────────────┐
         │  BulletClass::Fire (0x00468670) │  — THE LAUNCH FUNCTION (see §5)
         │                                 │    → copies velocity
         │                                 │    → sets SourceCoord, TargetCoord
         │                                 │    → Inviso path: raycast, snap to target, zero vel
         │                                 │    → FlakScatter+Inviso: horizontal scatter
         │                                 │    → ProximityDetector::Set (arm=BulletType.Arm)
         │                                 │    → if ROT>0: normalize vel to magnitude 1
         │                                 │    → DisplayClass::Submit_Object (visible)
         └──────────────┬──────────────────┘
                        │
                        ▼ (every game tick)
         ┌─────────────────────────────────┐
         │  BulletClass::AI (0x004666E0)   │  — per-tick update (prior doc: BULLET_CLASS_AI)
         │                                 │    → ROT<=0: arcing / straight / vertical
         │                                 │    → ROT>0:  homing (HomingTrack @ 0x005B20F0)
         │                                 │    → ProximityDetector::Check each tick
         │                                 │    → detonation triggers (many)
         └──────────────┬──────────────────┘
                        │
                        ▼ (on detonation)
         ┌─────────────────────────────────┐
         │  BulletClass::BulletDetonation  │  — warhead damage, cluster loop, shrapnel,
         │    0x00468D80                   │    airburst sub-munitions
         │  → WarheadTypeClass::Detonate   │    (prior doc: BULLETCLASS_TRAJECTORY_AND_HOMING §5-6)
         │    0x004690B0                   │
         └─────────────────────────────────┘
```

---

## 3. BulletTypeClass::ReadINI — complete INI reader

**Address:** `0x0046BEE0`
**Caller:** `0x0046BE10` (wrapper). Called once per BulletType entry during
rules/art parsing.
**`param_1` type:** `int` (direct byte offsets).

The reader calls `ObjectTypeClass::ReadINI` (inherited base — reads `Name=`, `Image=`,
etc.) and then reads bullet-specific keys. Every key below is verified from the
decompilation.

### 3.1 Keys read from the rules section (iVar1 = param_1 + 0x24)

| INI Key | Read Fn | Offset | Type | Default | Notes |
|---------|---------|--------|------|---------|-------|
| `Arm` | ReadInt | 0x2F0 | int | 0 | Arming delay (ticks) for ProximityDetector. Used only by BulletClass::Fire (§5.3). |
| `ROT` | ReadInt | 0x2DC | int | 0 | Rate of turn per tick; >0 = homing missile, <=0 = ballistic/straight. |
| `CourseLockDuration` | ReadInt | 0x2E0 | int | 0 | Ticks of locked heading after launch (homing only). |
| `Elasticity` | ReadDouble | 0x2C8 | double | **0.75** | Bounce energy retention for arcing bullets. |
| `Acceleration` | ReadInt | 0x2D0 | int | 3 | Speed change per tick (homing ramp, or straight-fight normalization). |
| `Color` | ReadColor | 0x2D4 | int(RGB) | 0,0,0 | Trail/line color packed RGB. |
| `Arcing` | ReadBool | 0x29B | bool | false | Enables ballistic gravity path. |
| `Floater` | ReadBool | 0x295 | bool | false | Uses alternate gravity (`FUN_0048ACF0`) instead of `Rules.Gravity`. TS-era; no standard YR unit sets it. |
| `SubjectToCliffs` | ReadBool | 0x296 | bool | false | Arcing-path cliff deflection via BounceCheck. |
| `SubjectToElevation` | ReadBool | 0x297 | bool | false | Affects trajectory over varying terrain. |
| `SubjectToWalls` | ReadBool | 0x298 | bool | false | Wall deflection via BounceCheck. |
| `VeryHigh` | ReadBool | 0x299 | bool | false | Exempts from fly-by approach detonation; augments homing terrain avoidance. |
| `Shadow` | ReadBool | 0x29A | bool | **true** | Draw a ground shadow. |
| `Dropping` | ReadBool | 0x29C | bool | false | "HasDropped" / drop-bomb behavior; TS-era, no standard YR unit sets it. |
| `Level` | ReadBool | 0x29D | bool | false | Straight-line ground-hugging movement. |
| `Inviso` | ReadBool | 0x29E | bool | false | Invisible bullet; raycast + instant-impact path in Fire (§5.4). |
| `Proximity` | ReadBool | 0x29F | bool | false | **Effectively dead at runtime** — read and stored, but no code path in BulletClass::AI or BulletClass::Fire reads byte offset +0x29F. See §7. |
| `Ranged` | ReadBool | 0x2A0 | bool | false | **This is the real prox-fuse gate.** When true (or `ROT>0`), ProximityDetector::Check runs each tick. See §7. |
| `Inaccurate` | ReadBool | 0x2A2 | bool | false | No target-snap on detonation. |
| `FlakScatter` | ReadBool | 0x2A3 | bool | false | Combined with Inviso: applies horizontal scatter in Fire (§5.2). In BounceCheck: triggers bounce below target altitude. |
| `AA` | ReadBool | 0x2A4 | bool | false | Valid against aircraft. |
| `AG` | ReadBool | 0x2A5 | bool | **true** | Valid against ground. |
| `Degenerates` | ReadBool | 0x2A6 | bool | false | Damage decrements each tick (min 5) in AI. |
| `Bouncy` | ReadBool | 0x2A7 | bool | false | Reflects velocity off ground in arcing path. |
| `Airburst` | ReadBool | 0x294 | bool | false | Detonates in-air, spawns AirburstWeapon sub-bullets. |
| `Cluster` | ReadInt | 0x2AC | int | 1 | Sub-munition count for detonation loop (non-Airburst). |
| `Scalable` | ReadBool | 0x2EC | bool | false | TS-era render flag; no confirmed runtime use. |
| `AirburstWeapon` | ReadString → FindOrAllocate | 0x2B0 | WeaponTypeClass* | NULL | Sub-weapon for airburst. |
| `ShrapnelWeapon` | ReadString → FindOrAllocate | 0x2B4 | WeaponTypeClass* | NULL | Sub-weapon for shrapnel spawning. |
| `ShrapnelCount` | ReadInt | 0x2B8 | int | 0 | Number of shrapnel bullets (negative = distance-based). |
| `DetonationAltitude` | ReadInt | 0x2BC | int | 0 | Z threshold for Vertical/Straight detonation. |
| `Vertical` | ReadBool | 0x2C0 | bool | false | Straight vertical descent (V3 rocket terminal). |
| `FirersPalette` | ReadBool | 0x2A9 | bool | false | Use firer's house color (copied into BulletClass+0x114 at Init). |

### 3.2 Keys read from the art section (iVar2 = param_1 + 0x1F8)

These are only read if `Image=` resolves to a valid art entry.

| INI Key | Read Fn | Offset | Type | Default | Notes |
|---------|---------|--------|------|---------|-------|
| `Trailer` | ReadString → AnimTypeClass::FindByName | 0x2D8 | AnimTypeClass* | NULL | Trail effect anim. |
| `SpawnDelay` | ReadInt | 0x2E4 | int | 3 | Default trailer spawn interval (used when TrailerSeperation==0). |
| `Rotates` | ReadBool **(inverted)** | 0x2A1 | bool | true (default) | Sprite rotation enable. Read with default `!current`, stored as `!read_value` — this inverts in-memory. `Rotates=no` → field becomes **true**, `Rotates=yes` → field becomes **false**. Confusing but matches binary exactly. |
| `Flat` | ReadBool | 0x2F7 | bool | false | Flat-to-ground render flag. |
| `AnimLow` | ReadInt (stored as byte) | 0x2F4 | byte | 0 | First sprite frame in cycle. |
| `AnimHigh` | ReadInt (stored as byte) | 0x2F5 | byte | 0 | Last sprite frame in cycle. |
| `AnimRate` | ReadInt (stored as byte) | 0x2F6 | byte | 0 | Ticks per animation frame. |
| `AnimPalette` | ReadBool | 0x2A8 | bool | false | Use anim-defined palette. |

### 3.3 ~~Second-pass reader — `BulletTypeClass::ReadINI_Part2` (0x00428319)~~ — SUPERSEDED

> **❌ SUPERSEDED 2026-04-24** — see `BULLETTYPECLASS_GHIDRA_REPORT.md` §5.
>
> The "function" at `0x00428319` is **not** a real function and **does not belong
> to BulletTypeClass**. It is mid-stream code inside `AnimTypeClass::ReadINI`
> (`0x00427D00` – `0x004287F5`). Verified four ways: (1) zero callers / zero
> xrefs to 0x00428319; (2) `unaff_ESI`/`unaff_EDI` at function entry — Ghidra
> tell that the registers were set by an enclosing function; (3) BulletTypeClass
> instance size is **0x2F8 bytes** (verified at `operator_new(0x2F8)` site
> `0x006C2E30`), so writes to +0x300, +0x344, +0x35A, +0x35D would be
> out-of-bounds; (4) the matching `AnimTypeClass::ReadINI` decompiled comment
> header explicitly lists `YDrawOffset(0x344)` and `ZAdjust(0x348)` as AnimType
> fields, and the function calls `operator_new(0x378)` (= AnimType size).
>
> **Practical consequence:** None of the keys in the table below are read by
> BulletTypeClass. They are AnimType keys, parsed when the AnimType pointed to
> by the BulletType's `Image=` is loaded. Do not implement them on a Rust
> ProjectileType — they belong on a future AnimType struct.
>
> The original (incorrect) table is preserved below for historical context only.

**Discovered after the §3.1 table was published.** There is a **second reader**
that runs after `BulletTypeClass::ReadINI` and parses additional art-side keys
into offsets beyond 0x2F7. Everything in this table is verified from the
decompilation of `BulletTypeClass::ReadINI_Part2` at `0x00428319`.

| INI Key | Read Fn | Offset | Type | Notes |
|---------|---------|--------|------|-------|
| `StartSound` / `Report` | ReadString → VocClass::FindByName | 0x2F8 | int (voc idx) | Falls back to `Report=` if `StartSound=` is absent. |
| `StopSound` | ReadString → VocClass::FindByName | 0x2FC | int (voc idx) | |
| `BounceAnim` | ReadString → AnimType lookup | 0x300 | AnimTypeClass* | Anim spawned on bounce. |
| `ExpireAnim` | ReadString → AnimType lookup | 0x304 | AnimTypeClass* | Anim spawned on expiry. |
| `TrailerAnim` | ReadString → AnimType lookup | 0x308 | AnimTypeClass* | Alternate trailer anim (distinct from `Trailer=` at +0x2D8). |
| `TrailerSeperation` | ReadInt | **0x30C** | int | **Stored here, not at +0x2E8.** Appears dead at runtime — no confirmed AI read of +0x30C (see §3.4). |
| `DamageRadius` | ReadInt | 0x334 | int | Overrides warhead radius for this projectile. |
| `Warhead` (on BulletType) | ReadString → FindOrAllocate | 0x330 | WarheadTypeClass* | Self-warhead (distinct from WeaponType.Warhead). |
| `Bouncer` | ReadBool | 0x35A | bool | **Distinct from `Bouncy=` at +0x2A7.** |
| `Tiled` | ReadBool | 0x35B | bool | Render tiling flag. |
| `ShouldUseCellDrawer` | ReadBool | 0x35C | bool | Render via cell drawer. |
| `UseNormalLight` | ReadBool | 0x35D | bool | Lighting mode. |
| `SpawnsParticle` | ReadString → ParticleSystemType | 0x2CC | ParticleSystemTypeClass* | Particle emitter. |
| `NumParticles` | ReadInt | 0x2D0 | int | Particle count. |
| `RandomRate` | ReadMinMax | 0x2E4 / 0x2E8 | int / int | **This is what actually drives the trailer spawn cadence at runtime.** Min → `+0x2E4 = 900/min`, Max → `+0x2E8 = 900/max`. See §3.4. |
| `YDrawOffset` | ReadInt | 0x344 | int | Render Y offset. |
| `ZAdjust` | ReadInt | 0x348 | int | Render Z adjustment. |

### 3.4 How the trailer spawn cadence actually works

> **⚠ Partially superseded 2026-04-24.** The pseudocode and the +0x2E4/+0x2E8
> branch in `BulletClass::AI` below are correct, but the claim that
> `RandomRate=Min,Max` writes those fields on BulletType is **wrong** (it
> belongs to the phantom Part2 — see §3.3 SUPERSEDED notice). On BulletType,
> +0x2E4 is set only by `SpawnDelay=` (raw ticks, from ReadINI), and +0x2E8 is
> never written — constructor zeros it and no INI key reaches it. The "max"
> branch is therefore permanently dead in standard YR data. The 900/rate
> conversion DOES exist, but in `AnimTypeClass::ReadINI` against the AnimType
> at the same byte offsets, not against the BulletType. See
> `BULLETTYPECLASS_GHIDRA_REPORT.md` §5.2.

BulletClass::AI uses offsets **+0x2E4 and +0x2E8** to decide when to spawn a
trailer anim each tick:

```c
if (BulletType.Trailer != NULL) {                 // +0x2D8
    if (BulletType.SpawnRateMax /* +0x2E8 */ == 0) {
        if (g_CurrentFrameCounter % BulletType.SpawnRateMin /* +0x2E4 */ == 0)
            spawn_trailer_anim();
    } else {
        if (g_CurrentFrameCounter % BulletType.SpawnRateMax /* +0x2E8 */ == 0)
            spawn_trailer_anim();
    }
}
```

The two values at +0x2E4 and +0x2E8 are set in **two stages**:

1. `ReadINI` writes `SpawnDelay=` into +0x2E4 directly as ticks per spawn.
2. `ReadINI_Part2` reads `RandomRate=Min,Max` (a MinMax pair in the art section).
   If `min != -1`: `+0x2E4 = (min > 0) ? 900/min : 0` (**overrides SpawnDelay**).
   If `max != -1`: `+0x2E8 = (max > 0) ? 900/max : 0`.
3. Post-processing: if `+0x2E8 < 0` → clamp to 0. If `+0x2E8 < +0x2E4` →
   `+0x2E4 = +0x2E8` (ensures min ≤ max after conversion).

**Practical meaning:**
- `SpawnDelay=` behaves as a trailer spawn interval **only when `RandomRate=`
  is absent** in the art section.
- `RandomRate=Min,Max` produces a two-tier cadence (`900/min` and `900/max`
  after conversion) — the AI picks the max-tier if non-zero, else falls back
  to min-tier.
- **`TrailerSeperation=` INI key is dead** in standard YR — it writes +0x30C,
  but no AI code path reads +0x30C. Confidence: High (exhaustive byte-pattern
  search for reads from +0x30C in bullet-range functions).
- The Scalable rate-limiter (§8.2) dynamically inflates +0x2E4 by
  `(scalable_count - 5) / 3` to thin trails when many scalable bullets coexist.

### 3.4 Final step in ReadINI

```c
// End of ReadINI:
if (Inviso == false) {
    FUN_005f9070();  // likely loads the SHP image
}
// else: skip image load entirely — Inviso bullets have no sprite
```

This confirms **Inviso bullets skip SHP loading**. No render asset is bound.

---

## 4. BulletTypeClass — constructor defaults (0x0046BBC0)

When a BulletType is first allocated, these defaults apply **before** ReadINI
overrides them. For keys missing from an INI section, the default below is what
the engine uses.

| Offset | Field | Default |
|--------|-------|---------|
| 0x294 | Airburst | 0 (false) |
| 0x295 | Floater | 0 |
| 0x296 | SubjectToCliffs | 0 |
| 0x297 | SubjectToElevation | 0 |
| 0x298 | SubjectToWalls | 0 |
| 0x299 | VeryHigh | 0 |
| 0x29A | **Shadow** | **1** (true) |
| 0x29B | Arcing | 0 |
| 0x29C | Dropping | 0 |
| 0x29D | Level | 0 |
| 0x29E | Inviso | 0 |
| 0x29F | Proximity | 0 |
| 0x2A0 | Ranged | 0 |
| 0x2A1 | (Rotates in-memory) | 1 (means "sprite rotates" — inverted at ReadINI, see §3.2) |
| 0x2A2 | Inaccurate | 0 |
| 0x2A3 | FlakScatter | 0 |
| 0x2A4 | AA | 0 |
| 0x2A5 | **AG** | **1** (true) |
| 0x2A6 | Degenerates | 0 |
| 0x2A7 | Bouncy | 0 |
| 0x2A8 | AnimPalette | 0 |
| 0x2A9 | FirersPalette | 0 |
| 0x2AC | **Cluster** | **1** (minimum 1 detonation) |
| 0x2B0 | AirburstWeapon | NULL |
| 0x2B4 | ShrapnelWeapon | NULL |
| 0x2B8 | ShrapnelCount | 0 |
| 0x2BC | DetonationAltitude | 0 |
| 0x2C0 | Vertical | 0 |
| 0x2C8 | **Elasticity** | **0.75** (0x3FE8000000000000 = 0.75) |
| 0x2D0 | Acceleration | 3 |
| 0x2D4 | Color | 0 |
| 0x2D8 | Trailer | NULL |
| 0x2DC | ROT | 0 |
| 0x2E0 | CourseLockDuration | 0 |
| 0x2E4 | SpawnDelay | 3 |
| 0x2EC | Scalable | 0 |
| 0x2F0 | Arm | 0 |
| 0x2F4 | AnimLow | 0 |
| 0x2F5 | AnimHigh | 0 |
| 0x2F6 | AnimRate | 0 |
| 0x2F7 | Flat | 0 |

**Notable non-zero defaults to mirror in Rust:** `Shadow=true`, `AG=true`,
`Cluster=1`, `Elasticity=0.75`, `Acceleration=3`, `SpawnDelay=3`, and the
inverted `Rotates` storage at +0x2A1.

---

## 5. BulletClass::Fire — the launch function

**Address:** `0x00468670`
**Signature:** `uint __thiscall BulletClass::Fire(BulletClass* this, CoordStruct* target_coord, double* velocity)`
**Caller:** `TechnoClass::Fire_At` (via virtual dispatch, vtable+0x1F0).
**NEW finding:** This function was referenced by prior docs as "BulletClass::Fire"
but never documented. It's the boundary between "allocated bullet object" and
"bullet flying in the world."

### 5.1 Flow overview

```c
BulletClass::Fire(this, target_coord_ptr, velocity_ptr) {
    // 1. Reveal the bullet (mark as visible/on-map). If it fails, abort.
    if (!ObjectClass::Reveal(this))  return 0;

    // 2. Copy the 6 ints of velocity (= 3 doubles) from caller into this+0xE8
    memcpy(&this->Velocity, velocity_ptr, 24);

    // 3. SourceCoord (this+0x134/0x138/0x13C) = target_coord argument
    //    NOTE: Ghidra's param naming is misleading — the "target_coord" arg here
    //    is actually the muzzle/source position from Fire_At (see FIRE_AT_ANALYSIS).
    this->SourceCoord = *target_coord_ptr;

    // 4. Initialize LastCell to the cell at source (this+0x14C packed X,Y)
    this->LastCell = pack_cell(SourceCoord >> 8);

    // 5. Remove from display layer, prepare for re-submission
    DisplayClass::RemoveFromLayer(this);

    // 6. Read the current target position via vtable+0x58 (Target->GetCoords)
    //    Stored locally as iStack_18/14/10
    CoordStruct target_pos = this->Target->GetCoords();

    // 7. TargetCoord (this+0x140/0x144/0x148) = target's current position
    this->TargetCoord = target_pos;

    // 8. FlakScatter + Inviso scatter (§5.2)
    if (BulletType.FlakScatter && BulletType.Inviso) { ... }

    // 9. Inviso path (§5.4)
    if (BulletType.Inviso) { ... }

    // 10. Arm field -> ProximityDetector (§5.3)
    int arm = (Target != NULL && Target->GetLayer() == 2) ? 0 : BulletType.Arm;
    ProximityDetector::Set(&this->Prox, &SourceCoord, &target_pos, arm, 0x7FFFFFFF);

    // 11. Homing velocity normalization (§5.5)
    if (BulletType.ROT > 0) { normalize_velocity_to_unit_length(); }

    // 12. Submit to display if alive
    if (this->IsAlive) DisplayClass::Submit_Object(this);

    return 1;
}
```

### 5.2 FlakScatter + Inviso horizontal scatter

**Trigger:** `FlakScatter=yes` AND `Inviso=yes` on the same BulletType.
**Effect:** Displace the target coordinate by a random horizontal offset, scaled
by distance and clamped by `Rules.BallisticScatter`.

```c
float dx = target.X - source.X;
float dy = source.Y - target.Y;          // note: Y inverted
float dz = target.Z - source.Z;
double dist = sqrt(dx*dx + dy*dy + dz*dz);
int scatter_range = RulesClass.BallisticScatter * 2;           // Rules+0x1734
int rand_scatter  = Random::RandomRanged(0, scatter_range);
int dist_int      = ftol(dist);
int owner_modifier = *(int *)(this->Owner + 0xB4);             // per-firer scale
int jitter_distance = (rand_scatter * dist_int) / owner_modifier;

int rand_facing = Random::RandomRanged(0, 0x7FFFFFFE);         // random angle
short facing_norm = ftol(rand_facing) - 0x3FFF;
double angle_rad = facing_norm * (-(2*PI / 65536));   // (corrected 2026-05-29: was positive 2*PI/65536; binary constant at _LAB_007e2810=0xBF19222D989F5E57 is negative — OPERATOR_OR_ORDER_DRIFT; verified via read_memory 0x007e2810)

new_target.X = cos(angle_rad) * jitter_distance + source.X;
new_target.Y = sin(angle_rad) * jitter_distance + source.Y;
new_target.Z = target.Z;                                       // Z unchanged
```

This produces the "Flak Cannon pattern around an aircraft" — Inviso+FlakScatter
bullets don't fly; they pick a scattered point near the target and detonate there.

### 5.3 Arm field → ProximityDetector wiring (THE MISSING LINK)

The prior docs document `ProximityDetector::Check` (returns 0/1/2) and note that
it has an `ArmingDelay` field at +0x14, but could not locate where the delay is
written. **This is the answer:**

```c
// In BulletClass::Fire, late in the function:
int arm;
if (Target != NULL) {
    int layer = Target->GetLayer();       // vtable+0x2C
    if (layer == 2) arm = 0;              // ground-layer target: no arming delay
    else            arm = BulletType.Arm; // air/sub/building: use configured delay
} else {
    arm = BulletType.Arm;
}

ProximityDetector::Set(
    &this->Prox,          // ECX
    &this->SourceCoord,   // current pos (for initial watermark)
    &target_pos,           // reference (written to Prox+0x18/1C/20)
    arm,                   // arming_delay -> Prox+0x14
    0x7FFFFFFF             // max_life (clamp) -> Prox+0x08
);
```

And `ProximityDetector::Set` at **0x004E1130** (previously labeled FUN_004E1130):

```c
void Set(this, cur_pos_ptr, ref_ptr, arm_delay, max_life) {
    if (max_life <= arm_delay) max_life = arm_delay;   // (corrected 2026-05-29: was strict <; binary uses <=: `if (param_5 <= param_4) param_5 = param_4` — OPERATOR_OR_ORDER_DRIFT; verified via decompile_function 0x004E1130)
    this[+0x00] = g_CurrentFrameCounter;   // CreationFrame
    this[+0x04] = <uninit stack slot>;     // never read — compiler artifact
    this[+0x08] = max_life;                 // upper clamp on arm_delay
    this[+0x0C] = g_CurrentFrameCounter;   // ArmingFrame  (used by Check)
    this[+0x10] = <uninit stack slot>;     // never read
    this[+0x14] = arm_delay;                // ArmingDelay  (used by Check)
    this[+0x18] = ref_ptr->X;               // ReferenceX
    this[+0x1C] = ref_ptr->Y;
    this[+0x20] = ref_ptr->Z;
    this[+0x24] = ftol(sqrt(|cur_pos - ref|^2));  // ClosestDistance watermark
}
```

**Practical meaning:** `Arm=N` in an INI makes the proximity detector return 0
("not close enough, keep flying") for the first N ticks after launch, regardless
of distance to target. This is the engine's equivalent of a weapon arming time.
Forced to 0 when the target is on the ground layer — so anti-ground projectiles
with `Arm>0` behave as `Arm=0`. Only air / submarine / building-layer targets
see the delay.

**Rust impact:** `src/rules/projectile_type.rs:48-49` labels offset 0x2F0 as
`speed: i32` with comment `read via "Speed" key`. This is **wrong**. The key is
`Arm=`, it represents proximity arming delay in ticks, and there is no BulletType
"speed" field at all — projectile speed comes from `WeaponType.Speed`
(stored in `BulletClass+0x110` as `TargetSpeed` during `BulletClass::Init`).

### 5.4 Inviso bullet launch behavior

**Trigger:** `Inviso=yes`.
**Effect:** The bullet never flies as a visible object; it snaps to the
impact point during Fire and its velocity is zeroed.

```c
if (BulletType.Inviso) {
    HouseClass* firer_house = this->Owner ? this->Owner->Owner : NULL;

    // Raycast/lookup to find the actual impact point.
    // FUN_005880a0 is a cell-to-impact helper that returns a CoordStruct.
    CoordStruct impact_pos = FUN_005880a0(tmp_arr, &source_coord, &target_pos, firer_house);

    if (impact_pos == <invalid sentinel {0x0089de30, 0x0089de34, 0x0089de38}>) {
        // Raycast failed — fallback
        void* fallback_cell = FUN_004cc100(BulletType, firer_house);
        if (fallback_cell == NULL) {
            SetCoords(this, &source_coord);          // stay at source
        } else {
            SetCoords(this, fallback_cell->GetCoords());  // go to fallback cell
        }
        this->TargetSpeed = 0;

        // ZERO the velocity vector
        // (Zero-vel guard kicks in immediately if all components were already 0)
        if (Vel == {0,0,0}) { VelX = 100.0; }   // guard
        double speed = sqrt(Vel . Vel);
        double factor = 0.0 / speed;             // = 0
        Vel *= factor;                            // = {0,0,0}
    } else {
        // Raycast succeeded — snap to impact point at ground height
        impact_pos.Z = CellClass::GetGroundHeight(impact_pos);
        SetCoords(this, &impact_pos);
    }
}
```

**Practical meaning:**
1. Inviso bullets are rendered invisibly and have zero travel time.
2. Their position becomes the impact point immediately at Fire.
3. Detonation happens on the next AI tick via the proximity detector (since the
   bullet is already at the reference coord, half-distance = 0 < 32, Check returns 1).
4. Examples in YR: all `Invisible*` BulletTypes, `InvisibleLow`, `InvisibleMedium`,
   `InvisibleHigh`, `InvisibleAll`, `Null`, `PsychicControl`. These are used for
   instant-hit weapons (small arms, beams, "at-range" damage).

### 5.5 Homing velocity normalization

```c
if (BulletType.ROT > 0) {
    if (Vel == {0,0,0}) { VelX = 100.0; }   // zero-velocity guard
    double mag = sqrt(Vel . Vel);
    double factor = 1.0 / mag;               // normalize to unit length
    Vel *= factor;                            // magnitude = 1 lepton/tick
}
```

Homing missiles **start at speed 1** and ramp up via `Acceleration` per tick in
AI (see prior BULLETCLASS_TRAJECTORY_AND_HOMING §2.1). Straight (`ROT<=0`)
bullets skip this step — their velocity is whatever the caller passed in
(from `TechnoClass::Fire_At`: `direction * WeaponType.Speed`).

---

## 6. [General] / [CombatDamage] ballistic keys in RulesClass

All offsets verified from `RulesClass::ReadCombatDamage` (0x0066CA20) and
`RulesClass::ReadGeneral` (0x0066D530) decompilations.

| INI Key | Section | Read Fn | Offset | Notes |
|---------|---------|---------|--------|-------|
| `BallisticScatter` | [CombatDamage] | ReadRange | **0x1734** | Max scatter distance. Used in BulletClass::Fire for FlakScatter+Inviso (§5.2). Stored as packed high/low range. |
| `HomingScatter` | [CombatDamage] | ReadRange | **0x1730** | Max scatter for homing missiles. Referenced in homing code paths. |
| `Gravity` | [General] | ReadInt | **0x16B8** | Gravity constant for arcing bullets. Default 6. Applied each tick as `VelZ -= Gravity`. |
| `MissileSpeedVar` | [General] | ReadDouble | **0x0590** | Speed fluctuation % for guided missiles. |
| `MissileROTVar` | [General] | ReadDouble | **0x0598** | ROT (turn-rate) fluctuation % for guided missiles. |
| `MissileSafetyAltitude` | [General] | ReadInt | **0x05A0** | **Altitude a missile climbs to before detonating if target dies mid-flight.** This is what BulletClass::AI reads for the "target lost + too high → detonate" check in the homing path — prior docs mislabeled it as `FlightLevel` (see §11). |
| `FlightLevel` | [General] | ReadInt | **0x07B4** | **Aircraft cruise altitude**, NOT bullet-related. Distinct from MissileSafetyAltitude. Prior doc mix-up fixed here. |
| `ParachuteMaxFallRate` | [General] | — | — | Not projectile-related. |
| `NoParachuteMaxFallRate` | [General] | — | — | Not projectile-related. |

**Active in YR:** All projectile-relevant keys above are live.

**Correction tracing:** The existing `BULLETCLASS_TRAJECTORY_AND_HOMING.md`
§2.9 Lost-target Handling states the threshold is "Rules.FlightLevel
(RulesClass+0x5A0)." That is wrong on two counts: (1) the field at +0x5A0 is
`MissileSafetyAltitude`, not `FlightLevel`; (2) `FlightLevel` is at +0x7B4 and
is used by aircraft flight AI, not by bullet detonation. All "FlightLevel"
references in the bullet docs should be read as "MissileSafetyAltitude."

---

## 7. Proximity (0x29F) vs Ranged (0x2A0) — clarification

Both flags are readable from INI and both default to false. But they behave
very differently at runtime:

### Ranged (0x2A0) — the real gate

Read in `BulletClass::AI` at the proximity-check dispatch:

```c
if ((BulletType.ROT < 1) && (BulletType.Ranged == 0)) {
    prox_result = 0;   // skip ProximityDetector::Check
} else {
    prox_result = ProximityDetector::Check(&this->Prox, &new_pos);
}
```

→ ProximityDetector::Check runs **iff** `ROT > 0` OR `Ranged=yes`. Anything
else — a straight-line bullet with no proximity gate — relies solely on
cell-based collision and the "same-cell-as-target" check.

### Proximity (0x29F) — effectively dead

Binary-pattern search for `9F 02 00 00` (little-endian encoding of offset 0x29F)
turns up exactly three BulletType-range hits:
- 0x0046BC1A — `BulletTypeClass::Constructor` (initializes the field to 0)
- 0x0046C0B0 / 0x0046C0CA — `BulletTypeClass::ReadINI` (writes the field)
- 0x0046C5C4 — inside `BulletTypeClass::ReadINI` (part of the ReadBool chain)

**No reads of `BulletType+0x29F` appear in `BulletClass::AI`, `BulletClass::Fire`,
`BulletClass::BulletDetonation`, `BulletClass::BounceCheck`, or any TechnoClass
function.** The `Proximity=` key is parsed, stored, and never consulted.

**Practical meaning:** Writing `Proximity=yes` in an INI has **no engine effect**
in Yuri's Revenge. Mods that expect it to enable the proximity detector need to
use `Ranged=yes` instead. This matches Ares/ModEnc's long-standing guidance.

Confidence: **High** (exhaustive byte-pattern search for the field's offset).

---

## 7B. `Scalable=yes` is live — trail rate-limiter

**Follow-up to §3.1.** Earlier scans did not find a reader of BulletType+0x2EC
(`Scalable`). A targeted byte-pattern search (`8A 8? EC 02 00 00` for
`mov reg8, [reg+0x2EC]`) turned up one hit outside BulletType's own reader, at
**`0x0074142D`** inside **`UnitClass::Fire`** (FUN_00741340). The code is:

```c
// In UnitClass::Fire, right after TechnoClass::Fire_At returns a new bullet:
BulletClass* bullet = TechnoClass::Fire_At(this);
if (bullet && bullet->Type->Scalable /* +0x2EC */) {
    FUN_0046B280(bullet);    // scalable-list registrar + trail-rate throttle
}
```

And `FUN_0046B280` (0x0046B280) does:

```c
void RegisterScalable(BulletClass* this) {
    // 1. Append this bullet to a global scalable list (grow-as-needed vector).
    global_scalable_list.push_back(this);   // DAT_0089DE18..DAT_0089DE2C

    // 2. Inflate this bullet's trailer spawn interval based on list size:
    int count = global_scalable_list.size;
    int throttle = (count - 5) / 3;         // negative clamped to 0
    if (throttle < 0) throttle = 0;

    int current_rate = this->Type->SpawnDelay;  // +0x2E4 (post-RandomRate)
    FUN_0046C840(current_rate + throttle);      // sets effective spawn rate
}
```

**Practical meaning:** When more than 5 scalable bullets exist simultaneously,
each newly-fired scalable bullet gets its trailer spawn interval extended by
`(count - 5) / 3` ticks. This is the engine's anti-screen-clutter mechanism for
mass-firing scalable weapons. The classic candidate is the Chrono Miner's
teleport sparkle or any particle-heavy visual.

**Active in YR:** Yes. Confidence: High (direct decompilation of both the
caller gate in UnitClass::Fire and the throttle implementation in FUN_0046B280).
**Note:** Only `UnitClass::Fire` gates on Scalable — InfantryClass's and
BuildingClass's Fire paths do not. So `Scalable=yes` only has an effect when
fired from a vehicle.

---

## 8. Global bullet storage

Bullets are registered in a global dynamic array referenced by `DAT_00A8ED40`
(variable name from prior BULLET_CLASS_LAYOUT doc). Registration happens in
`BulletClass::Constructor` (`0x00466380`) via the pattern seen in
`BulletTypeClass::Constructor` (two DAT-table registrations: the class-list at
`DAT_00a83c88/90/94` and the instance-list at `DAT_00b0f670/678/680/684`).

Deregistration happens in `BulletClass::AI` during the "limbo cleanup" branch
(when `IsWaitingForAnim` is true and the bounce-anim has finished):

```c
// From AI decompilation — the limbo cleanup path
if (this->IsWaitingForAnim && this->BounceAnim == NULL) {
    find_this_in_global_array(...);
    shift_remaining_entries_down();  // O(N) compaction
    --g_BulletCount;
    this->IsWaitingForAnim = 0;
    BulletClass::BulletDetonation(0);
    this->UnInit();   // vtable+0xF8
    return;
}
```

**Scale note for this engine's target:** the compaction loop is O(N) on every
bullet teardown. For 20k-unit scale, if many bullets detonate per tick, this
is a bottleneck to replace with a swap-remove or BTreeMap-style store — same
category as the `EntityStore` decision in CLAUDE.md.

---

## 9. Rust implementation status

### 9.1 Projectile type parsing

- [src/rules/projectile_type.rs](../src/rules/projectile_type.rs) — parses 37 fields.
  **Coverage is good**, matching the BulletTypeClass ReadINI key set almost
  completely. The `id`, `aa`, `ag`, `arcing`, `rot`, `airburst`, `cluster`,
  `trailer`, etc. fields all line up.

- **Confirmed bug:** `projectile_type.rs:47-49` declares a `speed: i32` field
  with comment `Binary offset: +0x2F0 (labeled "Arm" in the binary, read via "Speed" key)`.
  This is wrong on every count:
    - The INI key is `Arm=`, not `Speed=`.
    - The field's meaning is **arming delay (ticks) for the proximity detector**,
      not projectile speed.
    - BulletTypeClass has no speed field at all; projectile speed comes from
      `WeaponType.Speed` and is set into `BulletClass+0x110` (`TargetSpeed`)
      during `BulletClass::Init`.
  Fix: rename `speed` → `arm`, update the comment, and remove any downstream
  consumers that expect a projectile-level speed here (if any exist).

- **Missing fields** (in INI, not in Rust):
    - `Vertical=` (offset 0x2C0). Called out in Rust as `vertical: bool` — OK.
    - `Flat=` (offset 0x2F7). Present as `flat: bool` per the scan — OK.
    - `Arm=` — mislabeled as `speed` (see above).
  So after the `speed`→`arm` fix, the Rust side is feature-complete for INI parsing.

- **Defaults:** The Rust defaults need to be audited against §4. Notable
  non-zero defaults to verify: `Shadow=true`, `AG=true`, `Cluster=1`,
  `Elasticity=0.75`, `Acceleration=3`, `SpawnDelay=3`. If these are currently
  `Default::default()` (all zero/false), weapons that omit the key in INI
  will behave differently from gamemd.exe.

### 9.2 Runtime projectile simulation — the big gap

- `src/sim/movement/rocket_movement.rs` — implements a 4-phase rocket state
  machine (Launch → Ascending → Terminal → Detonation) with deterministic
  SimFixed math and a parabolic arc. **This module is not wired to weapon fire.**
  `attach_rocket_state()` is never called from any combat code.

- `src/sim/combat/mod.rs` — weapons apply damage **instantly** via direct
  `ReceiveDamage` on the target. There is no intermediate projectile entity,
  no travel time, and no flight physics.

- **Gap summary compared to gamemd.exe:**
  | System | Status |
  |--------|--------|
  | BulletClass-equivalent entity | ❌ Not present. Weapon fire → instant damage. |
  | Arcing/ballistic trajectory | ❌ Not in sim; rocket_movement has a simplified arc but it's orphaned. |
  | Homing (ROT>0) with HomingTrack logic | ❌ Not implemented. |
  | Straight-line flight with velocity integration | ❌ Not implemented. |
  | ProximityDetector (half-distance arming + detonation) | ❌ Not implemented. |
  | Arm delay | ❌ Not implemented (and field is currently mislabeled as speed). |
  | Inviso path (raycast-to-impact, zero velocity) | N/A — all hits are currently instant, which matches Inviso behavior coincidentally. |
  | Airburst sub-munitions | ❌ Not implemented. |
  | Cluster loop | ❌ Not implemented. |
  | ShrapnelWeapon / ShrapnelCount | ❌ Not implemented. |
  | BounceCheck (Bouncy, SubjectToCliffs/Walls, FlakScatter, AA, Level) | ❌ Not implemented. |
  | Trailer animation spawning | ❌ Not implemented. |
  | Degenerates damage decay | ❌ Not implemented. |
  | Bridge-crossing detonation | ❌ Not implemented. |
  | Approach-rate fly-by detection | ❌ Not implemented. |
  | Out-of-bounds forced detonation | ❌ Not implemented. |

- Bottom line: the Rust sim currently models weapons as **hit-scan**. To hit the
  99% parity bar, every visible projectile in YR (all non-Inviso types) needs a
  real bullet entity with per-tick flight — which is the natural next step once
  `attach_rocket_state()` gets wired to weapon fire.

### 9.3 Ordering in the tick

The existing tick order (per `src/app_sim_tick.rs` / `src/sim/world/mod.rs`)
runs `rocket_movement` in Phase 2 (before combat in Phase 5). gamemd.exe runs
`BulletClass::AI` via the global object AI update **after** weapons fire (i.e.,
newly fired bullets AI-tick on the same frame they're created). The Rust order
already matches the "render-visible flight" intent; the main wiring issue is
that combat doesn't spawn any bullets for the movement phase to tick.

---

## 9.1 BulletClass::BulletDetonation — verified decompilation

**Address:** `0x00468D80` (aka `BulletClass::BulletDetonation`, also referred to as
`BulletClass::Detonate` in some prior docs).
**`param_1` type:** `int` (direct byte offsets from BulletClass*).

Called by `BulletClass::AI` on every detonation trigger. Its job is to apply
**pre-impact damage** (to the exact target) and then dispatch **cluster / airburst**
detonations via the warhead. It does NOT apply area damage itself — that happens
inside `WarheadTypeClass::Detonate`.

### 9.1.1 Full flow

```c
void BulletDetonation(BulletClass* this) {
    CoordStruct cur = this->Location;      // +0x9C/+0xA0/+0xA4

    // 1. Is target still on the map? (vtable+0x54 = IsOnMap)
    AbstractClass* live_target = NULL;
    if (this->Target /* +0x10C */ != NULL && this->Target->IsOnMap()) {
        live_target = this->Target;
    }

    // 2. PRE-IMPACT DAMAGE — only for accurate, non-EMEffect, non-Airburst bullets
    if (this->Type->Inaccurate /* +0x2A2 */ == 0) {
        // 2a. If target exists, measure 3D distance (straight) to bullet:
        if (this->Target != NULL) {
            CoordStruct* tpos = this->Target->GetCoords();  // vtable+0x48
            CoordStruct delta = { cur.X - tpos->X,
                                  cur.Y - tpos->Y,
                                  cur.Z - tpos->Z };
            int dist = ftol(sqrt(delta·delta));
            // Target-snap opportunity: if within 32 leptons AND not Airburst
            //                          AND not Inaccurate, re-read target coords
            // (the re-read's result is observed to be discarded — compiler artifact
            //  or a side-effect path we haven't traced)
            if (dist < 32 && !this->Type->Airburst && !this->Type->Inaccurate) {
                this->Target->GetCoords();
            }
        }

        // 2b. Pre-impact damage: gated on NOT EMEffect AND NOT Airburst
        //     (warhead+0x154 is EMEffect per WARHEAD_DETONATE_GHIDRA_REPORT)
        if (this->Warhead->EMEffect /* +0x154 */ == 0
            && this->Type->Airburst /* +0x294 */ == 0) {

            if (live_target == NULL || live_target->GetLayer() /* vtable+0x78 */ == 2) {
                // GROUND-target branch (or no-target)
                if (this->Target != NULL
                    && ObjectClass::Distance_AdjForFoundation(this, this->Target) < 42) {
                    this->Target->GetCoords();       // vtable+0x58 (side-effect-only call)
                    if (this->Target->WhatAmI() == 6 /* Building */) {
                        BuildingTypeClass* btc = this->Target->TypeClass;
                        // Only turreted buildings take pre-impact damage
                        if (btc->TurretOffsetX /* +0xEBC */ != 0
                            || btc->TurretOffsetY /* +0xEC0 */ != 0
                            || btc->TurretOffsetZ /* +0xEC4 */ != 0) {
                            this->Target->ReceiveDamage(...);  // vtable+0xA4
                        }
                    }
                }
            } else {
                // AIRBORNE-target branch (layer != 2)
                if (ObjectClass::Distance_AdjForFoundation(this, live_target) < 128) {
                    live_target->ReceiveDamage(...);           // vtable+0xA4
                }
            }
        }
    }

    // 3. CLUSTER / AIRBURST DISPATCH
    if (this->Type->Airburst /* +0x294 */ == 0) {
        int count = 0;
        if (this->Type->Cluster /* +0x2AC */ > 0) {
            while (true) {
                WarheadTypeClass::Detonate(this);        // 0x004690B0
                if (!this->IsAlive) break;               // bullet destroyed itself
                int scatter = Random::RandomRanged(256, 512);   // 0x100..0x200
                FUN_0049F420(scatter, 0);                 // applies scatter offset
                if (++count >= this->Type->Cluster) return;
            }
        }
    } else {
        // Airburst: exactly one detonation (sub-bullets spawned inside Warhead::Detonate)
        WarheadTypeClass::Detonate(this);
    }
}
```

### 9.1.2 Key corrections vs prior doc (BULLETCLASS_TRAJECTORY_AND_HOMING §4.1)

1. **The "IsSpecial" gate is actually `EMEffect`.** Prior doc said "if NOT
   WarheadType->IsSpecial (offset 0x154) AND NOT Airburst." The field at
   warhead+0x154 is `EMEffect` (per the existing `WARHEAD_DETONATE_GHIDRA_REPORT`
   struct table). So the correct reading is: "skip pre-impact damage for
   EMEffect or Airburst warheads." That matches gameplay — EMP bullets don't
   apply pre-impact damage because the warhead entirely handles the effect.

2. **Distance metric is foundation-adjusted.** The decompiled helper
   `FUN_005F6360` at `0x005F6360` is `ObjectClass::Distance_AdjForFoundation`:
   it computes 3D distance between two objects, then if the second operand is
   a Building (WhatAmI()==6), subtracts `(foundation_width + foundation_height) × 64`
   leptons. So "< 42 leptons" and "< 128 leptons" are distances **measured from
   the edge of the building's footprint**, not from its center. For a 4×3
   building (War Factory) that's 7×64 = 448 leptons of footprint subtracted;
   for a 2×2 Prism Tower it's 4×64 = 256 leptons.

3. **Pre-impact damage to ground buildings has a narrow gate:**
    - Target must be on ground (layer == 2), and
    - Distance (foundation-adjusted) < 42 leptons, and
    - Target must be a Building (WhatAmI == 6), and
    - Building must have at least one non-zero TurretOffset (0xEBC/0xEC0/0xEC4)

   This explains why e.g. a shell that lands near a Prism Tower deals immediate
   damage to the tower, but a shell that lands near a Refinery doesn't — only
   turreted defenses take pre-impact damage. All other buildings wait for the
   cluster detonation's area damage.

4. **Airborne target gate is much wider:** distance < 128 leptons, no
   WhatAmI/turret check. Any airborne target within 128 leptons of the bullet's
   impact position takes pre-impact damage.

5. **Cluster loop bails on `IsAlive == false`.** If the bullet destroys itself
   during a detonation (e.g., triggers a chain reaction that calls UnInit), the
   cluster loop exits early. So the nominal "Cluster" count is a **maximum** —
   the actual detonation count can be less.

6. **Cluster scatter is uniform [256, 512] leptons.** Confirmed
   `Random::RandomRanged(0x100, 0x200)` per iteration. `FUN_0049F420` applies
   the scatter to the current detonation position (likely XY-only given
   typical scatter behavior; exact breakdown in FUN_0049F420 not re-traced).

7. **Airburst bypasses the cluster loop entirely.** A single call to
   `WarheadTypeClass::Detonate`. The AirburstWeapon sub-bullets are spawned
   inside that single Warhead::Detonate call (see Step 5 of prior warhead doc).
   This means `Cluster=N` on an `Airburst=yes` BulletType is **ignored**.

### 9.1.3 Active in YR

All paths live. No TS-only gates in this function.

---

## 9.2 WarheadTypeClass::Detonate — address correction and cross-reference

**Correction:** My first-pass report listed `WarheadTypeClass::Detonate` at
`0x00469790`. The correct address is **`0x004690B0`**, verified via Ghidra's
function table (`WarheadTypeClass__Detonate @ 004690b0`). The `BulletDetonation`
function at 0x00468D80 calls it twice (once per cluster iteration; once on the
Airburst path).

The existing `WARHEAD_DETONATE_GHIDRA_REPORT.md` at that address is already
comprehensive. I verified it end-to-end against the current binary; summary
of what it covers so this report can index it:

| Section | What it documents |
|---------|-------------------|
| §1-§2 | `BulletClass` + `WarheadTypeClass` struct offsets (damage, flags, CellSpread, Verses[11], etc.) |
| §3 | 10-way mutually-exclusive special-warhead dispatch (MindControl / IvanBomb / ElectricAssault / Parasite / Temporal / IsLocomotor / Airstrike / BombDisarm / MakesDisguise / NukeMaker) |
| §4 | `Apply_area_damage` at `0x00489280` — splash-damage target collection and delivery |
| §5 | Per-special-type detailed logic |
| §6 | Bright flash, combat light, screen shake |
| §7 | Crater/impact animation selection (`FUN_0048A4F0`) — damage/25 indexing into AnimList |

### 9.2.1 Additional notes not in the existing doc

**Ore vs vein destruction — the TS naming trap (verified).** In
`Apply_area_damage` Step 6a (§4 of warhead doc), the overlay-destruction gate
reads:

```c
if (typeEntry->IsTiberium) {
    if (!typeEntry->IsVein || warhead->Wood) {
        if (destroyTiberium) CellClass::Reduce_Tiberium();
    }
}
```

This is **exactly** the trap flagged in `CLAUDE.md`:
> *"`Tiberium=yes` on a WarheadTypeClass does NOT gate ore destruction — it only
> gates vein destruction."*

The flag that gates **vein** destruction is `Wood` (at warhead+0x147), despite
the name. Any warhead destroys ore (tiberium) if its bullet's `destroyTiberium`
flag is set; but only warheads with `Wood=yes` destroy veins. `Wood` is a
Tiberian-Sun-era name kept for binary compatibility.

**PercentAtMax falloff formula location.** The existing doc notes the
interpolation happens inside `ReceiveDamage`, not in `Apply_area_damage`, but
does not give the exact formula. The caller `Apply_area_damage` passes raw
lepton distance (not a normalized 0..1 ratio) to `ReceiveDamage`. Inside
`ReceiveDamage`, the damage scales as:

```
ratio = distance_leptons / (CellSpread * 256)    // cells → leptons
scale = 1.0 - (1.0 - PercentAtMax) * ratio       // linear interpolation
final_damage = base_damage * scale               // clamped to >= 1
```

This formula is inferable from the existing doc's description but has not been
independently re-decompiled in this pass. Confidence: Medium. Flagged as an
open question (§10.7).

**Bridge damage probability gate.** Existing doc §4 Step 10 documents:
`Random(1, Rules->BridgeStrength) < damageCount → destroy bridge`. Confirmed
against the binary. Rules.BridgeStrength is at `RulesClass+0x1740` (ReadInt)
per `ReadCombatDamage`. Default value comes from INI and is typically ~50 in
`rulesmd.ini`.

**Barrel chain reaction.** Existing doc §4 Step 10 notes that rock/barrel
overlays trigger recursive `Apply_area_damage` with `Rules.BarrelDamage` and
`Rules.BarrelExplode`. This is **live in YR** — any warhead that hits a barrel
overlay chain-explodes. Implement carefully for lockstep parity: the random
debris choice (15% chance per DebrisType) and particle spawn (25% chance) both
consume from the main RNG stream, so skipping them silently breaks replay sync.

**Nuke flash gate.** Existing doc §3 Step 4 notes `if (warhead == Rules->NukeWarhead)`
triggers a special whiteout. The Rules offset for `NukeWarhead` is `0xF8C`
(verified in `RulesClass::ReadCombatDamage` alongside `FlameDamage`,
`V3Warhead`, etc.).

### 9.2.2 Corrections to prior reports

- **`BULLETCLASS_TRAJECTORY_AND_HOMING.md` §4.1 "Detonation Function":**
  States "`WarheadTypeClass::Detonate`" at unspecified address. Cross-reference
  with `0x004690B0`.

- **`BULLET_CLASS_AI_GHIDRA_REPORT.md` table of Functions Called:** Lists
  `0x00468D80 | BulletClass::Detonate | Warhead detonation logic`. That's
  correct for BulletClass::BulletDetonation, but subtly — 0x00468D80 is the
  *bullet-side* detonation dispatcher, while 0x004690B0 is the *warhead-side*
  execution. Both should be listed.

- **First-pass of this consolidated report (§5.1):** Mentioned
  `WarheadTypeClass::Detonate at 0x00469790`. That was **wrong**; the correct
  address is **0x004690B0**. Fixed in §9.2 and in the Sources list.

---

## 9.3 Three §10 open questions — resolutions (2026-04-24)

### 9.3.1 PercentAtMax falloff formula (§10.7 partial resolution)

The damage-scaling function is **`FUN_00489180`** at `0x00489180`, called from
`ObjectClass::ReceiveDamage` (0x005F5390) via:

```c
iVar4 = this->GetTechnoType();             // vtable+0x88
iVar4 = FUN_00489180(*(typeClass + 0x9C),  // base damage
                     distance_from_impact); // second param
*damage_ptr = iVar4;
```

`FUN_00489180` reads **`warhead->PercentAtMax`** at **offset 0x12C** (a `float`,
per the existing warhead struct doc). Structure of the decompilation:

```c
uint Modulate_Damage(uint damage, int warhead_or_ctx, ..., int mode_flag) {
    if (damage == 0 || game_flags & 0x20 || warhead_or_ctx == 0) return 0;

    if ((int)damage < 0) {
        // Healing path: return (mode_flag > 7 ? damage : 0)
        return (mode_flag > 7) ? damage : 0;
    }

    float percent_at_max = *(float *)(warhead_or_ctx + 0x12C);  // PercentAtMax
    // ... FPU-heavy multiplication/interpolation ...
    // (exact scaling expression is obscured in Ghidra's output — multiple
    //  Math__ftol() calls consume FPU state that the decompiler cannot trace)
    int scaled = ftol(...);

    // Clamp to Rules.MaxDamage (RulesClass + 0x16C8)
    int max = *(int *)(g_RulesClass + 0x16C8);
    if (scaled >= max) return max;
    return scaled;
}
```

**Binary-verified facts:**
- Uses `warhead->PercentAtMax` (+0x12C) as the edge-damage ratio.
- Final output is clamped to **`Rules.MaxDamage`** at **`RulesClass+0x16C8`**.
- Healing (negative damage) bypasses the PercentAtMax math entirely, gated by
  `mode_flag > 7`.
- The scaling compares `(float)damage * PercentAtMax != (float)damage` as an
  early-out (if PercentAtMax == 1.0, no scaling is applied).

**NOT verified from binary (FPU opacity):**
- The exact expression for the mid-range falloff between center and edge.
- Linear vs quadratic vs cell-quantized interpolation.
- Whether distance is normalized to cells or used as raw leptons.

**Best-effort reconstruction** (consistent with the decomp structure and with
community-documented behavior — linear interpolation between 100% at center
and `PercentAtMax` at the edge of `CellSpread`):

```c
ratio = distance_leptons / (CellSpread * 256);   // 0..1, cells → leptons
scale = 1.0 - (1.0 - PercentAtMax) * ratio;      // 1.0 → PercentAtMax
final = ftol(damage * scale);                    // clamped >= 1 for buildings
if (final >= Rules.MaxDamage) final = Rules.MaxDamage;
return final;
```

Confidence: **Medium** — the structural reading is binary-verified, but the
precise interpolation expression is a likely-but-unverified inference.
Recommendation: port the linear formula above, add logging at damage
application, and diff against a recorded gamemd.exe session if precision
matters.

### 9.3.2 FUN_0049F420 cluster scatter (§10.8 fully resolved)

**Address:** `0x0049F420`. Called from the cluster loop in `BulletDetonation`
as `FUN_0049F420(scatter_distance, 0)` where `scatter_distance` is
`Random::RandomRanged(256, 512)` in leptons.

Decompilation shows the function takes an **output coord pointer** and an
**input reference coord pointer** (via register args), plus the scatter
magnitude and a nudge flag. Flow:

```c
void ScatterAroundPoint(CoordStruct* out, CoordStruct* ref,
                         int scatter_dist, char nudge_flag)
{
    byte facing_byte = Random::Next();                    // 0..255
    short facing_16 = ((facing_byte << 8) - 0x3FFF);      // center on 0x3FFF
    double angle_rad = facing_16 * (-(2*PI / 65536));      // facing → radians (corrected 2026-05-29: was positive 2*PI/65536; binary _LAB_007e2810=0xBF19222D989F5E57 is negative — OPERATOR_OR_ORDER_DRIFT; verified via read_memory 0x007e2810)

    int sin_comp = ftol(Sin_lookup(angle_rad));
    int cos_comp = ftol(Cos_lookup(angle_rad));
    // Magnitude from caller is folded in via FPU state — not visible in
    // Ghidra's decomp, but the 0x1FF = 511 clamp below confirms it's
    // bounded by the scatter_dist arg.

    out->X = ref->X + sin_comp;
    out->Y = ref->Y + cos_comp;
    out->Z = ref->Z;                                       // Z preserved

    // Safety clamp: if either axis offset > 511 leptons (> 2 cells),
    // revert to the reference coord
    if (|sin_comp| > 0x1FF || |cos_comp| > 0x1FF) {
        out->X = ref->X;
        out->Y = ref->Y;
    }

    // Nudge (param_4): adds +128 leptons to both X and Y if set
    if (nudge_flag) {
        out->X += 0x80;
        out->Y += 0x80;
    }
}
```

**Definitive answers:**
- **Axis:** XY-only. Z is copied unchanged from the reference. No use of
  `DetonationAltitude`.
- **Distribution:** uniform random **direction** (full 360° via random facing),
  **magnitude** from caller (Random[256..512] for Cluster).
- **Safety clamp:** offsets exceeding 511 leptons on either axis are zeroed —
  the detonation reverts to the reference coord.
- **Nudge flag:** the cluster call path passes 0, so the +128 nudge is unused
  for Cluster. Used by other callers (likely airburst spawners).
- **Lockstep implication:** each cluster iteration consumes **two** RNG draws —
  `RandomRanged(256, 512)` (magnitude) then `Random::Next()` (direction).
  Rust port must reproduce both in order.

Confidence: **High**.

### 9.3.3 vtable+0xA4 on ObjectClass — **NOT ReceiveDamage** (§10.9 resolved, with a §9.1 correction)

Reading the BulletClass vtable at `0x007E46E4`:

| Vtbl Offset | Target Address | Method (verified or inferred) |
|-------------|----------------|-------------------------------|
| +0x48 | `0x005F65A0` | `ObjectClass::GetCoords` (verified by prior docs) |
| +0x54 | `0x005F6B90` | `IsOnMap` |
| +0x78 | `0x00468B90` | `GetLayer` |
| **+0xA4** | **`0x0041BDD0`** | **`GetCoords_OutputParam` (NOT ReceiveDamage)** |

Decompilation of `0x0041BDD0`:

```c
void GetCoords_OutputParam(AbstractClass* this) {
    // Hidden output pointer passed via stack slot (Ghidra labels it
    // "unaff_retaddr" — compiler's return-struct-via-hidden-arg convention)
    CoordStruct local;
    CoordStruct* out = <hidden first stack slot>;
    CoordStruct* tmp = this->GetCoords(&local);   // vtable+0x48
    out->X = tmp->X;
    out->Y = tmp->Y;
    out->Z = tmp->Z;
}
```

**vtable+0xA4 is a GetCoords wrapper that writes to a caller-provided output
buffer** — not a damage application method.

### This correction affects §9.1 (BulletDetonation)

My earlier §9.1 claim that BulletDetonation "applies pre-impact damage" to
turreted buildings / near-miss aircraft via vtable+0xA4 was **wrong**. The
actual mechanism is a **target-snap of the detonation position**:

- The calls to `vtable+0xA4` fill a stack-local CoordStruct with the target's
  current coords.
- That CoordStruct is then passed (via register-based thiscall convention that
  Ghidra doesn't fully trace) to `WarheadTypeClass::Detonate` as the **impact
  position**.
- `WarheadTypeClass::Detonate` runs its `Apply_area_damage` at the snapped
  position instead of at the bullet's natural impact point.

So the "narrow gate" (turreted Building within 42 leptons, or airborne target
within 128 leptons) isn't "apply damage directly" — it's "override the
warhead's impact coord to the target's exact position." Damage is still dealt
by `Apply_area_damage` through its normal `ReceiveDamage` dispatch.

The **gameplay effect** is the same as the earlier interpretation (turreted
defenses and near-miss aircraft reliably take damage on close shots), but the
**mechanism** is a position-snap, not a direct damage call. For Rust porting:
implementing this as a detonation-position override, not as a separate damage
event, matches the binary more faithfully and avoids double-counting.

Confidence: **High**.

### 9.3.4 Corrected §9.1.1 summary

The pseudocode in §9.1.1 should read (corrected fragment, replacing the
"pre-impact damage" interpretation):

```c
// 2b. Detonation-position target-snap (was mislabeled "pre-impact damage")
CoordStruct detonation_pos = this->Location;    // default: bullet's impact pt

if (this->Warhead->EMEffect == 0 && this->Type->Airburst == 0) {
    if (live_target == NULL || live_target->GetLayer() == 2) {
        // Ground-target branch
        if (this->Target != NULL && Distance_AdjForFoundation(this, this->Target) < 42) {
            this->Target->GetCoords(&tmp);                // vtable+0x58
            if (this->Target->WhatAmI() == 6) {           // Building
                BuildingTypeClass* btc = this->Target->TypeClass;
                if (btc->TurretOffsetX || btc->TurretOffsetY || btc->TurretOffsetZ) {
                    this->Target->GetCoords_OutputParam(&detonation_pos);  // vtable+0xA4
                    //  ^ snaps detonation_pos to target's exact location
                }
            }
        }
    } else {
        // Airborne-target branch
        if (Distance_AdjForFoundation(this, live_target) < 128) {
            live_target->GetCoords_OutputParam(&detonation_pos);   // vtable+0xA4
        }
    }
}

// Later: WarheadTypeClass::Detonate(this, &detonation_pos)
```

No other §9.1 claims are affected by this correction.

---

## 10. Open questions (resolutions added 2026-04-23, 2026-04-24)

1. **TrailerSeperation reader.** ✅ **RESOLVED — re-resolved 2026-04-24.**
   `TrailerSeperation=` is **not a BulletTypeClass key at all**. It is an
   AnimTypeClass key, read by `AnimTypeClass::ReadINI` (`0x00427D00`) into
   AnimType+**0x30C**. The previous resolution attributed it to a phantom
   `BulletTypeClass::ReadINI_Part2` at `0x00428319` — that function does not
   exist; the address is mid-stream code inside `AnimTypeClass::ReadINI`. The
   `TrailerSeperation=` key on a bullet's `[Section]` in `rulesmd.ini` does
   nothing; on the bullet's `Image=` art section, it lands on the AnimType
   (which has its own +0x30C field) and is also unread by any AI code path.
   Net behavior — "TrailerSeperation= is dead in standard YR" — is unchanged,
   but for a different reason. See `BULLETTYPECLASS_GHIDRA_REPORT.md` §5.

2. **MissileSpeedVar / MissileROTVar / MissileSafetyAltitude offsets.**
   ✅ **RESOLVED:**
    - `MissileSpeedVar` (double) → RulesClass+**0x0590**
    - `MissileROTVar` (double) → RulesClass+**0x0598**
    - `MissileSafetyAltitude` (int) → RulesClass+**0x05A0**
    - `FlightLevel` (int) → RulesClass+**0x07B4** (aircraft cruise altitude,
      not a bullet field)

   Additional finding: the "target lost + too high → detonate" check in
   BulletClass::AI reads `g_RulesClass_Instance + 0x5A0`, which is
   **MissileSafetyAltitude**, not FlightLevel. Both prior bullet docs
   mislabeled this field; fix is folded into §6 and §11 below.

3. **`Scalable=yes` runtime behavior.** ✅ **RESOLVED.** Live in YR but only
   for vehicle-fired bullets. Used by `UnitClass::Fire` (FUN_00741340) at
   `0x0074142D` to dispatch into `FUN_0046B280`, which registers the bullet
   in a global scalable-list and inflates its trailer spawn interval by
   `(list_count - 5) / 3` ticks. Full analysis in §7B.

4. **`Proximity=yes` confirmation.** ✅ **RESOLVED.** Extended byte-pattern
   search (`8A 8? 9F 02 00 00` for byte-read from `[reg+0x29F]`) produced four
   extra hits inside `TechnoClass::ReceiveDamage` — but those are reads of
   **TechnoTypeClass+0x29F** (via `GetTechnoType` vtable+0x84), not
   BulletTypeClass+0x29F. They fall in the veteran/elite retaliation path and
   correspond to a TechnoType flag, not the projectile's `Proximity=` field.
   Original finding holds: **`Proximity=yes` on a BulletType has no runtime
   effect in YR.** Confidence upgraded from High to **Very High**.

5. **BulletClass::Fire uninit stack slots in ProximityDetector::Set.** Still
   low priority — confirmed harmless (fields are never read). No further work.

6. **Bullet global array size / growth policy.** Still not measured in this
   pass. Replace the data structure in the Rust port regardless — see §8.

7. **PercentAtMax exact falloff formula.** ✅ **PARTIAL RESOLUTION** (§9.3.1).
   Scaling function identified as `FUN_00489180` at `0x00489180`. Uses
   `warhead->PercentAtMax` at offset 0x12C, clamped to `Rules.MaxDamage` at
   `RulesClass+0x16C8`. **Linear interpolation formula remains inferred, not
   binary-verified**, due to FPU state opacity in the decomp. Confidence:
   Medium. Recommended path: port the linear formula and diff against a
   recorded gamemd.exe session if precision matters.

8. **`FUN_0049F420` cluster scatter behavior.** ✅ **RESOLVED** (§9.3.2).
   XY-only; Z preserved from reference coord. No `DetonationAltitude` use.
   Uniform random direction × caller-supplied magnitude. Offsets > 511 leptons
   on either axis are clamped to zero (detonation at reference). Two RNG draws
   per iteration — magnitude via `RandomRanged(256, 512)`, then direction via
   `Random::Next()` — must be reproduced in order for lockstep.

9. **`vtable+0xA4` resolution on ObjectClass.** ✅ **RESOLVED with a §9.1
   correction** (§9.3.3). vtable+0xA4 resolves to `0x0041BDD0`, which is a
   **`GetCoords_OutputParam` wrapper**, NOT `ReceiveDamage`. This invalidates
   the earlier "pre-impact damage" interpretation of BulletDetonation's
   close-target branches: the actual mechanism is a **detonation-position
   target-snap** (warhead impact coord is overridden to target's exact
   coords). Gameplay effect is unchanged; the mechanism is position-snap,
   not direct damage. §9.1.1 pseudocode corrected in §9.3.4.

---

## 11. Corrections to prior reports

- **`BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` §ProximityDetector Sub-Object:**
  The note "initialized by `FUN_004E1100` and queried by `FUN_004E11F0`" is
  correct. Add: **`FUN_004E1130` is ProximityDetector::Set**, called by
  `BulletClass::Fire` (0x00468670) to wire up `ArmingDelay` from `BulletType.Arm`,
  reference coords from target, and the initial watermark distance.

- **`BULLET_CLASS_AI_GHIDRA_REPORT.md` §8 Miss / Scatter / Inaccurate:**
  The section lists `Inaccurate`, `FlakScatter`, `Cluster`. Add: the
  `FlakScatter` **horizontal scatter in Fire** only applies when combined
  with `Inviso=yes` (see §5.2). Without Inviso, `FlakScatter` only affects
  BounceCheck behavior.

- **`BULLETCLASS_TRAJECTORY_AND_HOMING.md` §3.1 ProximityDetector Layout:**
  Fields +0x04 and +0x10 are written to from an uninitialized stack slot
  inside `ProximityDetector::Set`. They are never read by `Check`. Best
  treated as **unused padding** rather than real fields. (Prior doc already
  marks them "Uncertain"; this report upgrades to "Unused.")

- **`BULLETCLASS_TRAJECTORY_AND_HOMING.md` §2.9 Lost Target Handling:**
  Change `Rules.FlightLevel (RulesClass+0x5A0)` → **`Rules.MissileSafetyAltitude
  (RulesClass+0x5A0)`**. The field name was misidentified; `FlightLevel` is a
  separate key at RulesClass+0x7B4 used by aircraft flight AI. Same fix applies
  to **`BULLET_CLASS_AI_GHIDRA_REPORT.md`** §Key Constants table's
  `RulesClass+0x5A0 | FlightLevel` row — rename to `MissileSafetyAltitude`.

- **`BULLET_CLASS_AI_GHIDRA_REPORT.md` Key Struct Offsets table:**
  The row labeled `0x2E8 | TrailerSeperation | int | (art, separate reader)`
  is wrong, but **so was this report's prior fix** (which claimed the offset
  held `900 / RandomRate.Max` from a phantom Part2 reader). The verified state
  on **BulletType +0x2E8** is: constructor zeros it, no INI key writes it, only
  `BulletClass::AI` reads it (and the read branch is dead because the field is
  always 0). Replace the row with: `0x2E8 | (uninit by ReadINI) | int |
  constructor-zeroed; no INI writer; AI's "max" trailer-cadence branch is
  permanently dead`. There is **no** `0x30C` field on BulletType — that offset
  is out of bounds for the 0x2F8-byte BulletType struct. (`TrailerSeperation=`
  lands on the AnimType, not the BulletType — see
  `BULLETTYPECLASS_GHIDRA_REPORT.md` §5.)

---

## Sources

### Ghidra addresses decompiled
- `BulletTypeClass::Constructor` @ 0x0046BBC0
- `BulletTypeClass::ReadINI` @ 0x0046BEE0
- ~~`BulletTypeClass::ReadINI_Part2` @ 0x00428319~~ — **does not exist; address is mid-stream `AnimTypeClass::ReadINI` (0x00427D00). See §3.3 SUPERSEDED notice and `BULLETTYPECLASS_GHIDRA_REPORT.md` §5.**
- `BulletClass::Init` @ 0x004664C0
- `BulletClass::Fire` @ 0x00468670
- `BulletClass::AI` @ 0x004666E0 (verified context; full decomp reviewed)
- `BulletClass::Allocate` @ 0x0046B050 (COM CoCreateInstance wrapper)
- `UnitClass::Fire` (FUN_00741340) @ 0x00741340 — Scalable dispatch site
- `FUN_0046B280` @ 0x0046B280 — Scalable trail-rate throttle (§7B)
- `ProximityDetector::Set` (aka FUN_004E1130) @ 0x004E1130
- `ProximityDetector::Check` @ 0x004E11F0
- `RulesClass::ReadCombatDamage` @ 0x0066CA20 (BallisticScatter / HomingScatter offsets)
- `RulesClass::ReadGeneral` @ 0x0066D530 (Missile* and FlightLevel offsets — §6)
- `TechnoClass::ReceiveDamage` @ 0x00701900 (ruled out as a Proximity consumer — §10)
- `ObjectClass::ReceiveDamage` @ 0x005F5390 (§9.3.1 — damage pipeline entry point)
- `Modulate_Damage` (FUN_00489180) @ 0x00489180 (§9.3.1 — PercentAtMax scaler)
- `ScatterAroundPoint` (FUN_0049F420) @ 0x0049F420 (§9.3.2 — cluster scatter)
- `GetCoords_OutputParam` (FUN_0041BDD0) @ 0x0041BDD0 (§9.3.3 — vtable+0xA4)
- BulletClass vtable @ 0x007E46E4 (§9.3.3 — vtable slot dump)
- `BulletClass::BulletDetonation` @ 0x00468D80 (§9.1 — end-to-end verified)
- `WarheadTypeClass::Detonate` @ **0x004690B0** (§9.2 — address corrected from earlier first-pass value)
- `ObjectClass::Distance_AdjForFoundation` (FUN_005F6360) @ 0x005F6360 (foundation-subtracting distance metric, §9.1.2)
- ~~`BulletTypeClass::ReadINI_Part2` @ 0x00428319 (§3.3)~~ — **not a real function; mid-stream code inside `AnimTypeClass::ReadINI` @ `0x00427D00`. See §3.3 SUPERSEDED notice.**
- `FUN_0046B280` @ 0x0046B280 — Scalable rate throttle
- `UnitClass::Fire` (FUN_00741340) @ 0x00741340 — Scalable gate
- `RulesClass::ReadGeneral` @ 0x0066D530 (Missile*, FlightLevel offsets — §6)

### String address lookups (verified)
- `"Arm"` @ 0x0081B168 (BulletType+0x2F0)
- `"ROT"` @ 0x0081B164
- `"AA"` @ 0x0081B09C
- `"AG"` @ 0x0081B098
- `"BallisticScatter"` @ 0x0083ADA0 (Rules+0x1734)
- `"HomingScatter"` @ 0x0083AD58 (Rules+0x1730)
- `"FlightLevel"` @ 0x0083C854 (Rules+**0x07B4**)  <!-- corrected 2026-05-29: was (Rules+0x05A0) which is MissileSafetyAltitude; FlightLevel string at 0x0083C854 confirmed maps to param_1+0x7B4 — OPERATOR_OR_ORDER_DRIFT; verified via decompile_function 0x0066D530 -->
- `"MissileSpeedVar"` @ 0x0083CA8C
- `"MissileSafetyAltitude"` @ 0x0083CA9C

### Prior research docs referenced
- `C:/Users/enok/Documents/ra2-rust-game-docs/BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BULLET_CLASS_AI_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/BULLETCLASS_TRAJECTORY_AND_HOMING.md`
- `docs/FIRE_AT_PIPELINE_GHIDRA_REPORT.md` (in-repo)
- `C:/Users/enok/Documents/ra2-rust-game-docs/FIRE_AT_ANALYSIS.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/AIRBURST_SUB_WEAPON_SPAWN_GHIDRA_REPORT.md`

### INI files checked
- `ini/rulesmd.ini` — 58 BulletType sections, confirmed via parallel agent scan
- `ini/artmd.ini` — trailer / animation keys
- `ini/rules.ini` + `ini/art.ini` — base (no deltas relevant here)

### Rust files inspected (read-only)
- `src/rules/projectile_type.rs` (37 fields; bug in line 48 comment)
- `src/rules/weapon_type.rs` (projectile linkage)
- `src/sim/movement/rocket_movement.rs` (orphaned rocket state machine)
- `src/sim/combat/mod.rs` (instant-damage weapon fire)
- `src/sim/world/mod.rs` (tick ordering)
