# AAHeatSeeker2 / GUARDWH Detonation Parameters - Ghidra Research Report

**Address(es):** `BulletClass::BulletDetonation @ 0x00468D80`, `WarheadTypeClass::Detonate @ 0x004690B0`, `Apply_area_damage @ 0x00489280`, `Warhead__SelectExplosionAnim @ 0x0048A4F0`, `AnimClass::Constructor @ 0x00421EA0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** exact detonation-call parameters, impact-coordinate rewrite rules, GUARDWH damage payload, `CellSpread=.5` / `PercentAtMax=.5` participation, AnimList selection, and impact sound timing for stock `AAHeatSeeker2` fired by GGI `MissileLauncher` / `MissileLauncherE`.
**Non-Scope:** full generic warhead math, exact homing turn math, DRAGON draw frame mapping, target invalidation writers, and all non-GUARDWH warheads.
**Confidence:** High for call arguments, coordinate rewrites, damage payload, CellSpread radius use, AnimList selection, and impact-time sound source. Medium for semantic names of some target virtuals because this slot did not resolve every concrete vtable label.
**Active in YR:** Yes. Evidence: stock `[MissileLauncher] Projectile=AAHeatSeeker2` / `Warhead=GUARDWH` in `rulesmd.ini:22569-22576`, parent GGI lifecycle trace through `BulletClass::AI -> BulletClass::BulletDetonation`, and direct decompilation of the standard detonation path.

## 1. Overview

The GGI missile detonates through the normal bullet path, not a special GGI-only shortcut. `BulletClass::BulletDetonation @ 0x00468D80` prepares an impact `CoordStruct`, then calls `WarheadTypeClass::Detonate @ 0x004690B0` with `ECX = BulletClass*` and one stack argument: a pointer to that coordinate.

For stock AAHeatSeeker2, `Airburst=no`, `Inaccurate=no`, `Cluster` defaults to `1`, and `GUARDWH` is a normal non-special warhead. The result is exactly one `GUARDWH` detonation using the missile's stored damage payload: `40` for `[MissileLauncher]`, `50` for `[MissileLauncherE]`.

## 2. Class Layout / Key Offsets

| Class | Offset | Field | Evidence | Active in YR |
|---|---:|---|---|---|
| `BulletClass` | `+0x6C` | damage payload | `BulletClass::Init @ 0x004664C0`; consumed at `0x00469A56` and `0x00469BBA` | Yes |
| `BulletClass` | `+0x90` | alive flag | `BulletDetonation @ 0x00469038` loop guard | Yes |
| `BulletClass` | `+0x9C/+0xA0/+0xA4` | current coord | copied at `0x00468D8A..0x00468DAC` | Yes |
| `BulletClass` | `+0xAC` | `BulletTypeClass*` | read for `Airburst`, `Arcing`, `Inaccurate`, `Cluster`, `ROT` | Yes |
| `BulletClass` | `+0xB0` | owner/firer | passed to `Apply_area_damage` as source object | Yes |
| `BulletClass` | `+0x10C` | target pointer | coordinate snapping and special-warhead target logic | Conditional: only while target pointer remains non-null |
| `BulletClass` | `+0x128` | `WarheadTypeClass*` | `GUARDWH`; read throughout `Detonate` | Yes |
| `BulletClass` | `+0x130` | `WeaponTypeClass*` | radiation check only here; `MissileLauncher` has no scoped rad use | Conditional |
| `BulletClass` | `+0x150` | damage scalar | `0x100` default from `BulletClass::Init`; multiplies damage before area damage | Yes |
| `BulletTypeClass` | `+0x294` | `Airburst` | `BulletDetonation @ 0x00468FF4..0x004690A1`; AAHeatSeeker2 does not set it | No for AAHeatSeeker2 |
| `BulletTypeClass` | `+0x29B` | `Arcing` | fallback target coord branch at `0x00468ECF` | No for AAHeatSeeker2 |
| `BulletTypeClass` | `+0x29E` | coordinate-randomize flag | `Detonate @ 0x00469AC1..0x00469AF0`; AAHeatSeeker2 does not set it | No for AAHeatSeeker2 |
| `BulletTypeClass` | `+0x2A2` | `Inaccurate` | target-snap gate at `0x00468DC7..0x00468DCF` | No for AAHeatSeeker2 |
| `BulletTypeClass` | `+0x2AC` | `Cluster` | loop count at `0x00469020..0x0046908F`; constructor default `1` | Yes |
| `WarheadTypeClass` | `+0x104/+0x108/+0x114` | `AnimList` vector/count | `Warhead__SelectExplosionAnim @ 0x0048A4F0` | Yes |
| `WarheadTypeClass` | `+0x124` | `CellSpread` | `Apply_area_damage @ 0x004892DD`, `0x00489592` | Yes |
| `WarheadTypeClass` | `+0x12C` | `PercentAtMax` | parsed field in prior warhead report; consumed downstream by object receive-damage, not directly in `Apply_area_damage` | Yes |

## 3. Core Logic

### 3.1 `BulletDetonation -> Detonate` call shape

Verified binary finding. `BulletClass::BulletDetonation` starts by copying the bullet's current coord into a stack `CoordStruct`. At the non-airburst call site:

- `0x0046902C`: `LEA EDX,[ESP+0x0C]`
- `0x00469030`: `MOV ECX,ESI`
- `0x00469032`: `PUSH EDX`
- `0x00469033`: `CALL 0x004690B0`

The airburst branch uses the same call shape at `0x0046909A..0x004690A1`. `WarheadTypeClass::Detonate` then reads the bullet through `ESI/ECX`, not through a warhead `this` pointer.

**Active in YR:** Yes. The parent report verifies standard GGI missiles reach this path; AAHeatSeeker2 has `Airburst=no`, so the non-airburst call site is used.

### 3.2 Impact coordinate rewrite rules before detonation

Verified binary finding. The stack coordinate starts as `BulletClass+0x9C/+0xA0/+0xA4`.

If `Inaccurate` (`BulletType+0x2A2`) is false, `BulletDetonation` may snap the coordinate to the target:

1. If `BulletClass+0x10C` is non-null, compute 3D distance from bullet current coord to target vtable `+0x48` coord. If distance `< 0x20` leptons and `Airburst` is false, rewrite the impact coord to target `+0x48` coord. Evidence: `0x00468DD5..0x00468E9B`.
2. If warhead `+0x154` is false and `Airburst` is false, a second target snap block runs. For an eligible target pointer saved in `EBX`, if vtable `+0x78 != 2` and `FUN_005F6360(bullet,target) < 0x80`, impact coord becomes target vtable `+0xA4`. Evidence: `0x00468F23..0x00468FE0`.
3. Otherwise, if `BulletClass+0x10C` exists and `FUN_005F6360 < 0x2A`, impact coord becomes target vtable `+0x58`; for `WhatAmI()==6` with nonzero building art offsets at type `+0xEBC/+0xEC0/+0xEC4`, it is then replaced by target vtable `+0xA4`. Evidence: `0x00468F60..0x00468FF0`.
4. The fallback from `BulletClass+0xD0` is gated by `!stack_arg`, `!Arcing`, and `ROT <= 0`; AAHeatSeeker2 has `ROT=60`, so this fallback is not active for this projectile. Evidence: `0x00468EC7..0x00468F23`, `rulesmd.ini:25687`.

**Active in YR:** Conditional. Active for standard AAHeatSeeker2 only when `BulletClass+0x10C` remains non-null and the distance gates pass. If the target pointer is null, the detonation coordinate remains the bullet's current coord for this slice.

### 3.3 Target pointer vs cell-target behavior

Verified binary finding. `BulletDetonation` does not require a target pointer to detonate. The target pointer only influences coordinate snapping. If `BulletClass+0x10C` is null, none of the target vtable coordinate calls execute and the stack coord remains the bullet coord copied at function entry.

The function is therefore compatible with object targets and coordinate/cell-style targets, but only object-like targets with the expected vtable methods participate in the snap branches. This slot did not expand into all possible `AbstractClass` target subclasses.

**Active in YR:** Yes for object target pointers from normal GGI fire; Conditional for cell targets, depending on the runtime target object behind `+0x10C`.

### 3.4 Damage payload passed into GUARDWH

Verified binary finding. `BulletClass::Init @ 0x004664C0` stores the incoming weapon damage into `BulletClass+0x6C`. `WarheadTypeClass::Detonate` computes area-damage input as:

```text
(0x150) * +(0x6C) >> 8
```

`BulletClass::Init` initializes `+0x150 = 0x100`, so stock missiles pass the stored weapon damage unchanged. Evidence: `BulletClass::Init @ 0x004664C0`; `Detonate @ 0x00469A56..0x00469A83`.

For the scoped weapons:

| Weapon | Damage payload | Warhead | Evidence | Active in YR |
|---|---:|---|---|---|
| `[MissileLauncher]` | `40` | `GUARDWH` | `rulesmd.ini:22569-22576`, `BulletClass::Init +0x6C` | Yes |
| `[MissileLauncherE]` | `50` | `GUARDWH` | `rulesmd.ini:25123-25130`, `BulletClass::Init +0x6C` | Yes |

### 3.5 `Apply_area_damage` parameters

Verified binary finding. The normal GUARDWH path calls `Apply_area_damage @ 0x00489280` with:

| Parameter | Value for AAHeatSeeker2/GUARDWH | Evidence | Active in YR |
|---|---|---|---|
| `ECX` | `CoordStruct*` passed into `WarheadTypeClass::Detonate` | `0x00469A7F` loads `[EBP+0x8]` into `ECX` | Yes |
| `EDX` | damage = `Bullet+0x150 * Bullet+0x6C >> 8` | `0x00469A56..0x00469A66` | Yes |
| stack arg 1 | owner/source object = `Bullet+0xB0` | `0x00469A5C`, `0x00469A82` | Yes |
| stack arg 2 | warhead = `Bullet+0x128` (`GUARDWH`) | `0x00469A75..0x00469A7E` | Yes |
| stack arg 3 | destroy overlay/tiberium flag = `1` | `0x00469A7C` | Yes |
| stack arg 4 | owner house pointer = `owner+0x21C`, or `0` if owner null | `0x00469A69..0x00469A75` | Yes |

GUARDWH is normal, not MindControl/Ivan/Electric/Parasite/Temporal/Locomotor/Airstrike/BombDisarm/Disguise/NukeMaker. Evidence: `rulesmd.ini:26902-26912` lacks those special flags; `Detonate @ 0x0046920B..0x00469A56` falls to the normal branch when all flags are false.

### 3.6 `CellSpread=.5` and `PercentAtMax=.5`

Verified binary finding. `Apply_area_damage` uses `CellSpread` two ways:

1. Damage inclusion radius in leptons: it loads `warhead+0x124`, multiplies by `256.0` (`0x007E2224` contains float `256.0`), and converts with `Math__ftol`. For GUARDWH `.5`, this radius is `128` leptons. Evidence: `0x004892DD..0x004892EE`, memory `0x007E2224 = 00008043`, `rulesmd.ini:26911`.
2. Cell-list iteration radius: it loads `CellSpread`, adds the double at `0x007E5160` (`0.99`), converts, and indexes `CellSpreadTable @ 0x007ED3D0`. For `.5`, this indexes radius `1`, so the candidate cell pass can inspect the center plus the first surrounding ring while the final object filter still requires distance `<= 128` leptons. Evidence: `0x00489592..0x004895AA`, memory `0x007E5160`, `TERRAIN_CLASS_GHIDRA_REPORT.md:758`.

`Apply_area_damage` passes raw measured distance to each object's vtable `+0x16C` receive-damage call, along with the `GUARDWH` pointer. `PercentAtMax=.5` is therefore part of the downstream receive-damage falloff calculation, not a prefilter in `Apply_area_damage`. Evidence: `0x00489A91..0x00489AB6`, `WARHEAD_DETONATE_GHIDRA_REPORT.md:449-458`, `rulesmd.ini:26912`.

**Active in YR:** Yes. GUARDWH sets both keys in stock YR, and the normal detonation path passes that warhead pointer to every hit object.

### 3.7 Impact AnimList and sound timing

Verified binary finding. After damage handling, `WarheadTypeClass::Detonate` selects the impact animation by calling `Warhead__SelectExplosionAnim @ 0x0048A4F0` with:

| Parameter | Value | Evidence | Active in YR |
|---|---|---|---|
| `ECX` | `Bullet+0x6C` raw damage (`40` / `50`) | `0x00469BBA` | Yes |
| `EDX` | `Bullet+0x128` warhead (`GUARDWH`) | `0x00469BC4` | Yes |
| stack arg 1 | land/overlay context value computed just before selection | `0x00469AF0..0x00469BA2`, pushed at `0x00469BCA` | Yes |
| stack arg 2 | pointer to bullet current coord (`Bullet+0x9C`) | `0x00469BA2..0x00469BCF` | Yes |

For non-EMEffect GUARDWH, selection is damage-banded: `index = min(damage, count * 25 - 1) / 25`. Evidence: `Warhead__SelectExplosionAnim @ 0x0048A4F0`.

GUARDWH list from `rulesmd.ini:26909`:

```text
0 XGRYSML1
1 XGRYSML2
2 EXPLOSML
3 XGRYMED1
4 XGRYMED2
5 EXPLOMED
6 EXPLOLRG
7 TWLT070
```

Therefore normal GGI damage `40` selects index `1` (`XGRYSML2`), and elite damage `50` selects index `2` (`EXPLOSML`). Both selected art entries have `Report=Explosion13` in `artmd.ini:16155-16157` and `artmd.ini:16221-16224`.

The selected `AnimClass` is constructed only after detonation, with draw flags `0x2600`, facing from `FUN_0048ACE0`, and the impact coordinate that was prepared earlier. Evidence: `0x00469C4E..0x00469C93`. This makes the explosion visual and its animation report sound impact-time effects, not fire-time and not DRAGON-trailer effects.

**Active in YR:** Yes. Stock GUARDWH has the AnimList and no `EMEffect`, and `AnimClass::Constructor` is reached on the normal detonation path when the selected anim pointer is non-null.

## 4. INI Keys

| Section | Key | Stock YR value | Runtime effect in this slice | Active in YR |
|---|---|---|---|---|
| `[MissileLauncher]` | `Damage` | `40` | stored at `Bullet+0x6C`; area damage and AnimList band | Yes |
| `[MissileLauncher]` | `Projectile` | `AAHeatSeeker2` | selects BulletType fields below | Yes |
| `[MissileLauncher]` | `Warhead` | `GUARDWH` | stored at `Bullet+0x128` | Yes |
| `[MissileLauncherE]` | `Damage` | `50` | same path; selects next AnimList band | Yes |
| `[AAHeatSeeker2]` | `ROT` | `60` | makes `BulletDetonation` skip the non-ROT fallback coord at `+0xD0` | Yes |
| `[AAHeatSeeker2]` | `Inaccurate` | absent/default false | enables target snap branches | Yes via default |
| `[AAHeatSeeker2]` | `Airburst` | absent/default false | uses Cluster loop, not airburst spawn branch | Yes via default |
| `[AAHeatSeeker2]` | `Cluster` | absent/default `1` | exactly one warhead detonation in non-airburst path | Yes via constructor default |
| `[GUARDWH]` | `CellSpread` | `.5` | 128-lepton inclusion radius; radius-1 candidate cell ring | Yes |
| `[GUARDWH]` | `PercentAtMax` | `.5` | falloff input inside receiver damage logic | Yes |
| `[GUARDWH]` | `AnimList` | `XGRYSML1,...,TWLT070` | impact anim selection by raw damage band | Yes |
| `[GUARDWH]` | `Conventional` | `yes` | normal bridge/overlay participation; not a special-warhead branch | Yes |
| `[GUARDWH]` | `ProneDamage` | `50%` | parsed warhead field; not re-expanded here | Conditional: only infantry prone damage contexts |

## 5. Integration Points

| Integration point | Verified behavior | Active in YR |
|---|---|---|
| `BulletClass::AI -> BulletDetonation` | parent report verifies the standard GGI missile reaches `0x00468D80` on impact/proximity/approach conditions | Yes |
| `BulletDetonation -> WarheadTypeClass::Detonate` | `ECX=BulletClass*`, stack arg `CoordStruct*` | Yes |
| `WarheadTypeClass::Detonate -> Apply_area_damage` | passes impact coord, scaled damage, owner, warhead, destroy flag `1`, owner house | Yes |
| `WarheadTypeClass::Detonate -> Warhead__SelectExplosionAnim` | selects GUARDWH animation by raw damage band | Yes |
| `WarheadTypeClass::Detonate -> AnimClass::Constructor` | creates impact anim at detonation time with flags `0x2600` | Yes |
| impact sound | selected AnimType's `Report=` / `StartSound=` is the sound source; for 40/50 GUARDWH bands this is `Explosion13` | Yes |

## 6. Current Rust Implementation Status

Not audited in depth for this slot. The parent lifecycle report already records the current VERA20k gap: damage/effects are still tied to the weapon fire event in key paths, while gamemd delays damage and presentation until `BulletClass::BulletDetonation`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BulletClass::BulletDetonation @ 0x00468D80` call arguments | verified | disassembly `0x0046902C..0x00469033`, `0x0046909A..0x004690A1` | none |
| target snap from current bullet coord to target coord | verified | `0x00468DD5..0x00468FF0` | semantic names for target virtuals deferred |
| AAHeatSeeker2 non-airburst cluster path | verified | `0x00468FF4..0x0046908F`; BulletType default Cluster=1 docs | none |
| `WarheadTypeClass::Detonate @ 0x004690B0` normal branch | verified | `0x00469A3F..0x00469A88` | none for GUARDWH |
| `Apply_area_damage @ 0x00489280` scoped arguments | verified | `0x00469A56..0x00469A83`; decompile/disassembly | none |
| `CellSpread=.5` inclusion radius | verified | `0x004892DD..0x004892EE`, constant `256.0` at `0x007E2224` | none |
| `CellSpread=.5` cell candidate ring | verified | `0x00489592..0x004895AA`, add constant at `0x007E5160` | none |
| `PercentAtMax=.5` downstream role | touched-not-exhausted | `Apply_area_damage` passes warhead and distance to vtable `+0x16C`; prior warhead report | exact receiver formula not re-decompiled in this slot |
| `Warhead__SelectExplosionAnim @ 0x0048A4F0` | verified | direct decompile; call at `0x00469BBA..0x00469BCF` | none |
| `AnimClass::Constructor @ 0x00421EA0` call for impact | verified | `0x00469C4E..0x00469C93` | constructor internals not re-expanded |
| impact sound source | verified from art + prior sound doc | `artmd.ini` Report fields; prior `ANIMATION_SOUNDS_GHIDRA_REPORT.md` | no per-sound playback stack re-decompile here |

## 8. Open Questions - Final State

[RESOLVED] OQ-AAH-GUARDWH-001 - What exact arguments does `BulletDetonation` pass to `WarheadTypeClass::Detonate`? `ECX=BulletClass*`, stack arg `CoordStruct*` to prepared impact coordinate. Evidence: `0x0046902C..0x00469033`.

[RESOLVED] OQ-AAH-GUARDWH-002 - Does AAHeatSeeker2 use airburst or repeated cluster detonation? Non-airburst cluster loop with default `Cluster=1`, so exactly one `GUARDWH` detonation. Evidence: `BulletType+0x294` branch at `0x00468FF4`, `BulletType+0x2AC` loop at `0x00469020`, constructor default in `BULLETTYPECLASS_GHIDRA_REPORT.md`.

[RESOLVED] OQ-AAH-GUARDWH-003 - What coordinate is detonated? Start with bullet coord; conditionally snap to target coords when non-inaccurate and distance gates pass; otherwise current bullet coord. Evidence: `0x00468D8A..0x00468FF0`.

[RESOLVED] OQ-AAH-GUARDWH-004 - What damage reaches GUARDWH? Stored weapon damage, normally unchanged because `Bullet+0x150=0x100`: `40` rookie, `50` elite. Evidence: `0x004664C0`, `0x00469A56..0x00469A83`, `rulesmd.ini:22570`, `rulesmd.ini:25124`.

[RESOLVED] OQ-AAH-GUARDWH-005 - How does `CellSpread=.5` enter the path? Inclusion radius is `ftol(.5 * 256)=128` leptons; candidate cells use the `CellSpread+0.99` table index, giving radius index `1`. Evidence: `0x004892DD`, `0x00489592`, constants at `0x007E2224` and `0x007E5160`.

[RESOLVED] OQ-AAH-GUARDWH-006 - Which impact anim is selected? Damage-banded GUARDWH AnimList: `40 -> XGRYSML2`, `50 -> EXPLOSML`. Evidence: `0x0048A4F0`, `rulesmd.ini:26909`.

[RESOLVED] OQ-AAH-GUARDWH-007 - Is sound fire-time or impact-time? Impact-time; selected AnimClass is constructed after detonation selection, and the selected AnimType has the report sound. Evidence: `0x00469C4E..0x00469C93`, `artmd.ini:16155-16157`, `artmd.ini:16221-16224`.

[DEFERRED] OQ-AAH-GUARDWH-008 - Exact receive-damage falloff formula using `PercentAtMax=.5`. Reason: out of this slot's scope; this report verifies the parameter propagation and distance handoff, not the full receiver math. Category: out-of-scope.

## Sources

- Ghidra MCP read-only decompiles/disassembly:
  - `BulletClass::BulletDetonation @ 0x00468D80`
  - `WarheadTypeClass::Detonate @ 0x004690B0`
  - `Apply_area_damage @ 0x00489280`
  - `Warhead__SelectExplosionAnim @ 0x0048A4F0`
  - `BulletClass::Init @ 0x004664C0`
  - `TechnoClass::Fire_At @ 0x006FDD50`
- INI files:
  - `ini/rulesmd.ini`
  - `ini/artmd.ini`
- Prior reports:
  - `GGI_MISSILELAUNCHER_AAHEATSEEKER2_PROJECTILE_LIFECYCLE_GHIDRA_REPORT.md`
  - `DRAGON_RENDER_AND_GUARDWH_IMPACT_PRESENTATION_GHIDRA_REPORT.md`
  - `WARHEAD_DETONATE_GHIDRA_REPORT.md`
  - `BULLETTYPECLASS_GHIDRA_REPORT.md`
  - `BULLET_CLASS_AI_GHIDRA_REPORT.md`
  - `TERRAIN_CLASS_GHIDRA_REPORT.md`
  - `ANIMATION_SOUNDS_GHIDRA_REPORT.md`
