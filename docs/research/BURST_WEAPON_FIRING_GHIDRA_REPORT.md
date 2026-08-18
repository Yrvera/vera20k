# Burst Weapon Firing Sequence — Ghidra Research Report

**Date:** 2026-04-21
**Binary:** gamemd.exe (Yuri's Revenge 1.001)
**Confidence:** HIGH for struct offsets, GetROF behavior, and burst-index cycling (decompiled directly). HIGH for the claim that Fire_At fires exactly one bullet per call and `Burst=N` is expressed via ROF-timer shortening. MEDIUM for the InfantryTypeClass `BurstDelay%d` array — only BurstDelay0/BurstDelay1 are safely addressable; BurstDelay2/BurstDelay3 collide with an adjacent DynamicVectorClass structure.
**Active in YR:** YES (live). Burst=N is heavily used on Flak, Gattling, Tesla Trooper, IFV tips, sub-missile V3, Aegis, etc.

---

## 1. Overview

"Burst" in YR is a **rapid-succession N-shot sequence fired from a single weapon**, dispatched
from separate per-tick `Fire_At` calls. The engine does NOT loop inside `Fire_At` to launch
N bullets at once. Instead:

1. The weapon's `Burst=N` (WeaponTypeClass+0x9C) is metadata read by the AI dispatcher.
2. Each **tick** during an active attack, `GetFireError` is evaluated and (if OK) `Fire`
   (vtable+0x3CC) is called, which calls `Fire_At` — one bullet per call.
3. `Fire_At` increments `this->CurrentBurstIndex` (TechnoClass+0x3B8) and wraps via
   `% Burst`.
4. After the shot, `GetROF` (vtable+0x318) returns a **small** value (3-5 frames for
   non-infantry; a per-shot INI value for infantry) as long as we are "mid-burst" (i.e.,
   `CurrentBurstIndex` is still within `[1, Burst-1]`). On the final shot of the burst
   (when `CurrentBurstIndex` is about to wrap to 0), `GetROF` returns the weapon's full
   `ROF=` value, applying veterancy / naval / crate multipliers.
5. `Fire_At` sets the shared fire timer `this+0x2F8` to the value returned by `GetROF`.
   `GetFireError` reads that timer and rejects further fires (FIRE_BUSY) until the
   timer elapses.

So **Burst=N means "fire N shots spaced by 3-5 frames, then reload for `ROF` frames"**. The
burst "state machine" is really just: (a) the `CurrentBurstIndex` counter, and (b) the
`GetROF`-returns-a-short-value-while-mid-burst policy.

**Spread pattern:** For generic burst weapons there is **no spatial spread** — all N shots
leave the same muzzle with the same trajectory computation (aim is re-computed per tick
because `Fire_At` re-queries target position and recomputes bullet velocity). For
`IsGattling` weapons (which are **orthogonal** to Burst but also set Burst=N on each
stage-weapon), an 8-entry scatter table at `DAT_00b0eaa8` offsets the muzzle position
based on `CurrentBurstIndex` so successive shots alternate among the barrel positions.

**Retargeting mid-burst:** Because each shot is an independent `Fire_At` call originating
from mission-attack dispatch, the target is re-resolved every tick. If the original
target dies before the burst completes, the next tick's `SelectWeaponAgainst` /
`GetFireError` will see the null target and the burst silently aborts — **partial bursts
are the norm when targets die**.

---

## 2. Class Layouts / Key Offsets

### WeaponTypeClass (source: `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md`, verified)

| Offset | INI Key | Type | Default | Notes |
|---|---|---|---|---|
| `0x9C` | `Burst=` | int | 1 | Number of shots per burst sequence |

**NOTE:** WeaponTypeClass has **no** `BurstDelay=` field. The per-shot delay is either
3-5 frames of random jitter (non-infantry) or read from `InfantryTypeClass.BurstDelay%d`.

### InfantryTypeClass — `BurstDelay0..3` array (new finding)

Parsed by `InfantryTypeClass::ReadINI` (Ghidra-labeled `UnitTypeClass__ReadINI` at
`0x00747620`; labeling is misleading — the presence of `AltImage`, `WalkFrames`,
`FiringFrames`, `FiringSyncFrame%d`, and `BurstDelay%d` keys proves this is actually the
InfantryTypeClass override):

```c
// Loop at 0x00747c64
iVar11 = 0;
do {
    FUN_007c8ef4(local_138, s_BurstDelay_d_00845ca0, iVar11);  // sprintf "BurstDelay%d"
    uVar4 = CCINIClass__ReadInt(iVar1, local_138, *puVar5);
    *puVar5 = uVar4;
    iVar11 = iVar11 + 1;
    puVar5 = puVar5 + 1;
} while (iVar11 < 4);
```

| Offset | INI Key | Type | Default | Notes |
|---|---|---|---|---|
| `0xE40` | `FiringSyncFrame0` | int | 0 | Primary-weapon anim frame to trigger shot |
| `0xE44` | `FiringSyncFrame1` | int | 0 | Secondary-weapon anim frame to trigger shot |
| `0xE48` | `BurstDelay0` | int | 0 (from constructor) | Burst-index-1 delay before 2nd shot |
| `0xE4C` | `BurstDelay1` | int | 0 (from constructor) | Burst-index-2 delay |
| `0xE50` | `BurstDelay2` | int | (collides w/ DVC) | See caveat below |
| `0xE54` | `BurstDelay3` | int | (collides w/ DVC) | See caveat below |

**IMPORTANT CAVEAT:** `InfantryTypeClass::Constructor` (`0x005236A0`) initializes a
DynamicVectorClass at index `[0x394]` (byte `0xE50`) with `&PTR_FUN_007eb6d4`. The
`BurstDelay2`/`BurstDelay3` INI writes at bytes `0xE50`/`0xE54` will **corrupt** that
DVC's vtable pointer and internal size fields. Conclusion: **only `BurstDelay0` and
`BurstDelay1` are safely usable.** This is consistent with GetROF's usage (it only reads
indices 1..4, i.e., up to `BurstDelay3`, but the gate `iVar5 < 5` was presumably a
future-proofing upper bound and in practice only Burst values of 2 or 3 are safe).

**CAVEAT 2:** `BurstDelay=` does **not appear in any `[Weapon]` section** of shipping
`rulesmd.ini`. The key exists in the parser but is never set in retail YR. All
shipping burst weapons fall through to the random-jitter default in `GetROF`.

### TechnoClass (per-instance runtime state)

Source: `TECHNOCLASS_STRUCT_LAYOUT.md` and `FIRE_AT_ANALYSIS.md`, verified against
`Fire_At` (`0x006FDD50`) and `GetROF` (`0x006FCFA0`) decompilation.

| Byte offset | Field | Type | Purpose |
|---|---|---|---|
| `0x2A0` | `GattlingScatterIndex` | int | Random base index into the 8-entry scatter table (Gattling only) |
| `0x2EC` | `FireTimer.StartFrame` | int | `g_CurrentFrameCounter` at moment of last shot |
| `0x2F0` | `FireTimer.Range` | uint | Copy of `uStack_a0` at fire time (ROF result) |
| `0x2F4` | `FireTimer.InitialValue` | int | Used by GetFireError to compare remaining cooldown |
| `0x2F8` | `FireTimer.ROF` | int | Value returned by GetROF on last shot — the active cooldown |
| `0x3B8` | **`CurrentBurstIndex`** | int | 0..Burst-1; incremented per shot in Fire_At; wraps via `% Burst` |
| `0x43C` | `BarrelRotationIndex` (Gattling scatter angle) | int | Separate angular-offset index for arcing bullets (unrelated to burst) |

**`CurrentBurstIndex` lifecycle** (from Fire_At, `0x006FDD50`):
```c
// End of Phase 11 in FIRE_AT_ANALYSIS.md:
this->CurrentBurstIndex = this->CurrentBurstIndex + 1;
// ... set ROF timer ...
this->CurrentBurstIndex = this->CurrentBurstIndex % *(int *)(uVar18 + 0x9c);  // % weapon.Burst
```

This runs at the end of every non-DiskLaser fire. So if `Burst=2`:
- Before shot 1: `CurrentBurstIndex == 0`.
- After shot 1: incremented to 1, `1 % 2 == 1`.
- Before shot 2: `CurrentBurstIndex == 1`.
- After shot 2: incremented to 2, `2 % 2 == 0`.
- Back to start.

The post-burst state (`CurrentBurstIndex == 0`) is what `GetROF` uses to decide "full ROF
vs short delay". See §3.2.

### InfantryTypeClass additional burst-relevant fields

Source: `0x005206B0` `InfantryClass::Fire_At_Target` decompilation:

```c
iVar3 = *(int *)&param_1[1].field_0x1a0;       // InfantryType* (this+0x5A0)
cVar1 = param_1[1].field_0x1bb;                 // byte at this+0x5BB — "is-prone" flag
iVar7 = *(int *)(iVar3 + 0xe40);               // FiringSyncFrame0 (standing)
if (cVar1 != '\0')
    iVar7 = *(int *)(iVar3 + 0xe44);           // FiringSyncFrame1 (prone)

if (iVar2 != 0) {                              // secondary weapon
    if (cVar1 != '\0' &&
        *(int *)(*(int *)(iVar3 + 0xe3c) + 0x5c8) != 0)   // prone+secondary
        iVar7 = *(int *)(iVar3 + 0xe4c);                  // reuses BurstDelay1 slot
    if (*(int *)(*(int *)(iVar3 + 0xe3c) + 0x5a4) != 0)   // secondary available
        iVar7 = *(int *)(iVar3 + 0xe48);                  // reuses BurstDelay0 slot
}
```

So for infantry, `InfantryClass::Fire_At_Target` checks whether the current animation
frame (`this->field_0xF8`) matches `FiringSyncFrame0/1` (or `BurstDelay0/1` for the
secondary weapon). Only when they match is `vtable+0x3CC` (Fire) dispatched.

**The BurstDelay slots are dual-purpose:** they serve as both (a) per-shot delay values
for GetROF, and (b) alternate firing-sync frames for the secondary weapon's firing
animation. The game treats them the same way because both encode "frame number within
the firing animation at which the shot should launch".

---

## 3. Core Logic

### 3.1 Fire_At: one bullet per call, burst-index cycling

Source: `FIRE_AT_ANALYSIS.md` and direct decompilation of `TechnoClass::Fire_At`
(`0x006FDD50`).

`Fire_At` does **NOT loop** to dispatch multiple projectiles. It creates exactly one
`BulletClass` and calls `bullet.Launch` (`vtable+0x1F0`). At the end, it bumps
`CurrentBurstIndex`:

```c
this->CurrentBurstIndex = this->CurrentBurstIndex + 1;
rof = this->vtable.GetROF();                     // 0x006FCFA0
if (this->field_0x298) rof /= 2;                 // half-ROF modifier flag
this->field_0x2F8 = rof;                         // active cooldown
this->field_0x2EC = g_CurrentFrameCounter;
this->field_0x2F0 = uStack_a8;                   // range/coord scratch
this->field_0x2F4 = rof;                         // initial value
this->CurrentBurstIndex = this->CurrentBurstIndex % weapon.Burst;
```

### 3.2 GetROF — the actual burst scheduler (address `0x006FCFA0`)

This is where the per-shot delay is decided. Full decompilation:

```c
int TechnoClass::GetROF(this, weapon_index):
    // Building multi-barrel shortcut
    if (this->WhatAmI() == 6 && this->byte_0x2FC > 1)
        return 1;

    weapon = this->GetWeapon(weapon_index);             // vtable+0x3F8
    if (!weapon) return 1;

    // "Sticky" weapons (sonic/particle/railgun): no burst shortening, return full ROF
    if (weapon.IsSonic ||
        (weapon.UseSparkParticles && this.sparkParticleSys) ||
        (weapon.UseFireParticles && this.fireParticleSys) ||
        (weapon.IsRailgun && this.railgunParticleSys))
        return weapon.ROF;                              // weapon+0xB0

    is_infantry = (this->WhatAmI() == 1);
    burst_idx = this->CurrentBurstIndex;                // +0x3B8

    if (burst_idx < weapon.Burst) {
        // === MID-BURST — short inter-shot delay ===
        if (0 < burst_idx && burst_idx < 5 && is_infantry) {
            // Infantry: try BurstDelay[burst_idx-1]
            // Layout: infantry_type[0xE44 + burst_idx*4]
            //   idx=1 -> 0xE48 = BurstDelay0
            //   idx=2 -> 0xE4C = BurstDelay1
            //   idx=3 -> 0xE50 = BurstDelay2 (unsafe - overlaps DVC)
            //   idx=4 -> 0xE54 = BurstDelay3 (unsafe - overlaps DVC)
            int delay = infantry_type[0xE44 + burst_idx*4];
            if (delay != -1) return delay;
            // fall through if sentinel -1
        }
        return Random::RandomRanged(3, 5);              // non-infantry or fallback
    }

    // === END OF BURST — full ROF with modifiers ===
    jitter = Random::RandomRanged(0, 2);
    rof = Math::ftol(weapon.ROF * (1.0 + jitter/...));  // small random scaling
    if (IsVeteran && type.VeteranAbilities & FIREPOWER)  rof *= vet_mult;
    if (IsElite && type.EliteAbilities & FIREPOWER)     rof *= elite_mult;
    if (IsNaval && barrel_count > 0)                    rof /= barrel_count;
    if (this.has_crate_powerup && ~is_building)         rof *= crate_mult;
    return rof;
```

**Key observations:**

1. **Per-shot delay is NOT in WeaponTypeClass.** There is no `BurstDelay=` INI key on
   weapons. The per-shot delay is either:
   - **Infantry with Burst>=2:** `InfantryType.BurstDelay[burst_idx-1]`. If set to 0,
     fires instantly; if -1, falls through to random 3-5.
   - **Non-infantry (units, buildings, aircraft):** random integer in `[3, 5]` frames.

2. **Random jitter on final-shot ROF:** The last shot's cooldown is also jittered via
   `RandomRanged(0, 2)`. This is a small variance that prevents multiple units with
   identical ROF from perfectly synchronizing their fire cadence — a classic RA2
   "just a little random" design.

3. **Sonic/particle weapons skip burst shortening.** Even if `Burst=N` is set on a
   `IsSonic=yes` weapon, it returns the full `ROF` on every shot. So burst is silently
   neutralized for these special-visual weapons. Not a bug — these weapons typically
   use `Burst=1` anyway.

4. **The 3-5 frame inter-shot default** (Random::RandomRanged(3, 5)) makes burst shots
   fire at roughly ~15 FPS-equivalent spacing at 15fps game rate, or every 200-333ms at
   the 66ms/frame YR sim rate. Effectively "two quick shots" visually.

### 3.3 Spread direction pattern

**For regular burst weapons (no `IsGattling`):** No spread. Each tick's `Fire_At` re-runs
the entire aim-and-velocity pipeline (§3.8 of FIRE_AT_ANALYSIS.md):
- Target position is re-fetched via `vtable+0xA4`.
- `GetFLH` (vtable+0xB0) recomputes muzzle position.
- `atan2` recomputes facing from muzzle to target.
- The bullet launches with the same trajectory as the previous shot (target hasn't
  moved much in 3-5 frames).

Any "spread" visually comes from the bullet's `Inaccurate=yes` flag (BulletTypeClass at
`0x2A2`), which adds random angle+distance jitter, and `FlakScatter=yes` (`0x2A3`),
which adds proportional-to-distance inaccuracy. These are per-shot, not per-burst —
the same `Inaccurate` calculation runs on EVERY fire regardless of burst index.

**For `IsGattling` weapons** (see `GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md`
for the stage machinery; the scatter-table logic is independent of the stage system
and runs in Fire_At regardless of gattling stage):

Fire_At's early branch (`if (*(char *)(iVar9 + 0x691) != '\0')` — `TechnoType.IsGattling`):

```c
if (this->CurrentBurstIndex == 0) {
    this->GattlingScatterIndex = Random::RandomRanged(0, 7);
} else {
    this->GattlingScatterIndex =
        (this->GattlingScatterIndex + (8 / weapon.Burst)) & 0x80000007;
    // handle negative modulo
}
int offset_idx = this->GattlingScatterIndex * 0xC;   // 12 bytes per entry
muzzle_x = DAT_00b0eaa8[offset_idx+0] + this.Location_X;
muzzle_y = DAT_00b0eaa8[offset_idx+4] + this.Location_Y;
muzzle_z = DAT_00b0eaa8[offset_idx+8] + this.Location_Z;
```

The 8-entry scatter table at `0x00B0EAA8` is initialized on first use to an **octagonal
pattern** around the unit's center (radius 256 leptons, ~1 cell):

| Index | X | Y | Z |
|---|---|---|---|
| 0 | 256 | 0 | 0 |
| 1 | 180 | 180 | 0 |
| 2 | 0 | 256 | 0 |
| 3 | -180 | 180 | 0 |
| 4 | -256 | 0 | 0 |
| 5 | -180 | -180 | 0 |
| 6 | 0 | -256 | 0 |
| 7 | 180 | -180 | 0 |

So Gattling bursts:
- **First shot of burst:** random starting octant (Random::RandomRanged(0,7)).
- **Subsequent shots:** step by `8 / Burst` octants. For `Burst=2`, step = 4 → uses
  two opposite octants. For `Burst=4`, step = 2 → uses four octants 90° apart.
- The `& 0x80000007` with a negative-modulo fixup ensures the index wraps 0..7.

**This is the ONLY built-in spatial spread for bursts.** Flak Trooper (Burst=2) does NOT
get this — it uses `IsGattling=no`. Its "alternating barrels" effect comes from the
infantry firing animation switching which barrel-offset is shown, not from engine-side
coordinate offsetting.

### 3.4 Turret animation / firing-FLH cycling per burst shot

Source: `Fire_At` FLH block and `FIRE_AT_ANALYSIS.md` §3 (Phase 3).

The FLH (Fire Location + Height) is fetched per shot via `vtable+0xB0` (`GetFLH`), and
that call takes `weapon_index` as parameter. `GetFLH` can read per-burst-index FLH
offsets from the art INI (e.g. `PrimaryFireFLH.Burst0`, `.Burst1`, etc. — these are
Ares extensions, not in vanilla YR). In shipping YR, FLH is **per-weapon-index only**,
not per-burst-index.

**What IS per-burst-index in shipping YR:**
- `CurrentBurstIndex` is used by building multi-barrel animations. The check at the end
  of Fire_At:
  ```c
  if (IsNaval && WhatAmI() == 6) {  // building/naval
      this->MultiBarrelIndex++;
      this->MultiBarrelIndex %= vtable.GetBarrelCount();  // +0x408
  }
  ```
  But this uses a **separate** counter (`+0x69C`, `MultiBarrelIndex`) that cycles
  independently of `CurrentBurstIndex`.

- Gattling scatter (above).

**What's NOT per-burst-index in shipping YR:**
- The firing-muzzle anim (`Anim=MGUN-N,...`) — selected by turret facing direction, not
  by burst index.
- Per-shot damage or warhead — Burst shares all weapon fields; there's no "Burst[i] uses
  different warhead" system.

### 3.5 Retargeting mid-burst

Because `Fire_At` is invoked by mission-attack dispatch each tick, **the target pointer
is re-read every tick**. The sequence for a Burst=2 weapon:

**Tick T** (burst_idx 0 → 1):
1. Mission_Attack → GetFireError(target=T) → FIRE_OK
2. Fire_At → bullet created, aimed at T → CurrentBurstIndex = 1
3. GetROF returns ~3-5 → FireTimer = 3-5

**Tick T+1** (waiting for timer):
- Mission_Attack → GetFireError → FIRE_BUSY (timer > 0)
- No fire this tick.

**Tick T+3** (timer expired; burst_idx 1 → 0, but still "mid-burst" in the modular sense):
- If target T is still alive: Mission_Attack → GetFireError(T) → FIRE_OK → Fire_At →
  second shot aimed at T's new position → CurrentBurstIndex = 2 % 2 = 0 → GetROF returns
  full ROF.
- **If target T died between T and T+3:** Mission_Attack re-runs
  `SelectWeaponAgainst(new_target_or_null)`. If `target == null`, `Fire_At` returns
  immediately with no bullet. The `CurrentBurstIndex` stays at 1 until another target
  is acquired and the next Fire_At is called — at which point the second shot of the
  "old" burst completes against the NEW target.

**Consequence: `CurrentBurstIndex` is not reset when a target dies or is lost.** It's
just a modular counter. The practical effect is usually invisible (the next engagement
starts with burst_idx=1, so the very first shot of the next burst uses the short
inter-shot delay instead of a full ROF reload — net effect: a free "leftover" quick
shot). Modders and edge-case analysts should note this.

### 3.6 Interaction with `Airburst=yes` and `Projectile.Shrapnel=yes`

**These are different mechanics that compose additively with Burst.**

- `Airburst=yes` (on WeaponTypeClass warhead side) triggers a **secondary volley** when
  the projectile expires mid-flight (handled by `BulletClass::Airburst` — see
  `BULLET_CLASS_AI_GHIDRA_REPORT.md`). Each primary-weapon shot that has `Airburst=yes`
  spawns N sub-projectiles from the detonation point, independent of Burst. Combining
  `Burst=2` with `Airburst=yes` on the same weapon gives: 2 primary shots (via Burst
  mechanism), each of which airbursts into M secondary projectiles → 2×M total detonations
  per ROF cycle. Used on `[V3Airburst]`.

- `Shrapnel=yes` / `Shrapnel*` (on BulletTypeClass, at offsets 0x29F+, see
  `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md`) similarly spawns extra projectiles on bullet
  impact. Composes identically with Burst: `Burst=3` + `Shrapnel=yes` with
  `ShrapnelCount=5` → 3 projectiles per ROF, each producing 5 shrapnel on impact.

- `FlakScatter=yes` (on BulletTypeClass `+0x2A3`) is a **per-bullet accuracy modifier**,
  not a burst interaction. It makes each bullet's impact proportional-inaccurate with
  distance. Commonly seen on Flak weapons.

None of these three flags interact with `CurrentBurstIndex` — they all compose on top
of the per-shot Burst dispatch.

### 3.7 `Inaccurate=` / `RandomNoise` interaction with burst

- `Inaccurate=yes` is a **BulletTypeClass flag** (offset `0x2A2`), not WeaponTypeClass.
  It adds per-bullet random angle+distance scatter (Fire_At phase 8, uses
  `RulesClass+0x1734 = BallisticScatter`). **Every shot in a burst runs the inaccuracy
  calculation independently** — so Burst=N with `Inaccurate=yes` produces N independently
  scattered impact points, not a concentrated cluster.

- There is no `RandomNoise` INI key in the parser. The user's gap-scan brief mentions
  it, but `search_strings "RandomNoise"` returns zero matches in gamemd.exe. Assume the
  user meant either `Inaccurate`, `FlakScatter`, or the Random::RandomRanged jitter in
  GetROF itself.

### 3.8 Gattling "stage" vs "Burst" — they are orthogonal

`IsGattling=yes` replaces the primary/secondary weapon selection with a **stage index**
that cycles through up to 6 weapon pairs (ground/AA) — see
`GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md`. Each stage weapon can itself have
`Burst=N`. The composition:
- `CurrentGattlingStage` (TechnoClass+0x140) selects which weapon from the list is used
  for this tick.
- Once the weapon is picked, normal Fire_At runs with that weapon's `Burst=N`, using
  `CurrentBurstIndex` (+0x3B8) to cycle the Gattling scatter-table offsets.
- `GattlingValue` (+0x144) accumulates on fire and drops the stage back down on
  non-fire ticks.

Example: Yuri Gattling Tank (`[YTNK]`) at stage 2 uses weapon `AGGattling3` which has
`Burst=4`. So shot sequence per ROF cycle is 4 bullets from `AGGattling3`, fired 3-5
frames apart, each one offset by (8/4)=2 octants around the scatter ring. This is the
classic "spinning up" feel.

---

## 4. INI Keys

| Key | Class | Offset | Parser | Notes |
|---|---|---|---|---|
| `Burst=` | WeaponType | 0x9C | `WeaponTypeClass::ReadINI` at `0x00772080` | Shot count; default 1 (corrected 2026-05-29: was `0x007722C1`; that address is the `Burst=` ReadInt call site mid-function, not the function entry; entry confirmed via `get_function_by_address 0x00772080` — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT) |
| `BurstDelay0=` | InfantryType | 0xE48 | `InfantryTypeClass::ReadINI` at `0x00747C64` loop | Per-shot delay for Burst index 1 (2nd shot); default 0; sentinel -1 falls through to random 3-5 |
| `BurstDelay1=` | InfantryType | 0xE4C | same | For Burst index 2; default 0 |
| `BurstDelay2=` | InfantryType | 0xE50 | same | UNSAFE — corrupts adjacent DVC. Do not set. |
| `BurstDelay3=` | InfantryType | 0xE54 | same | UNSAFE — corrupts adjacent DVC. Do not set. |
| `FiringSyncFrame0=` | InfantryType | 0xE40 | same (separate loop) | Anim frame to fire primary weapon; default 0 |
| `FiringSyncFrame1=` | InfantryType | 0xE44 | same | Anim frame to fire secondary; default 0 |
| `IsGattling=` | TechnoType | 0xCD5 | `TechnoTypeClass::ReadINI` | Enables stage system (see gattling report) |

**Note:** `Burst=` must be set on the **WeaponType** section, not the unit. Shipping YR
uses values 1..4 on weapons. Values > 4 may work but become increasingly impractical
because the infantry BurstDelay array is only 2 slots safely.

### YR retail usage survey (`ini/rulesmd.ini`)

Grep for `^Burst=` returns 80+ matches. Representative sample of weapons that use
`Burst>1`:

| Weapon | Burst | Used by | Notes |
|---|---|---|---|
| `[FlakTrackGun]` | (no) | Flak Track AG | **Burst=1 on shipping** — the "two shots" visual is from `Anim=GUNFIRE` being called twice by the firing animation, NOT from Burst |
| `[FlakGuyGun]` | (no) | Flak Trooper AG | Same — Burst=1 |
| `[JumpCannon]` | 2 | Rocketeers | 2 shots per ROF |
| `[AGGattling]..[AGGattling3]` | (no/1) | YTNK ground | Burst driven by scatter table via IsGattling, NOT Burst= |
| `[V3Airburst]` | (line 23237) | V3 missile split | Composes with Airburst |
| `[VirusShot]` | 2 | Virus infantry | Pair shots |
| `[DesolatorShot]` | 2/3 | Desolator | |
| `[TeslaTroopGun]` | 2 | Tesla Trooper | |
| `[IFV*]` | 2-4 | IFV tip weapons (several variants) | |
| `[Aegis*]` | 4 | Aegis Cruiser | High burst |

*CAUTION on FlakTrackGun/FlakGuyGun: the user's gap-scan brief suggested these use
Burst=. Direct inspection of `rulesmd.ini` shows they do NOT. The multiple-shot visual
comes from the unit's `Anim=GUNFIRE` combined with FiringFrames driving the firing
animation loop. Verify before implementing Burst handling assuming Flak.*

No `BurstDelay%d=` keys found in shipping `rulesmd.ini`. Feature is parser-only for
modder use.

---

## 5. Integration Points

**Upstream (who calls Fire_At):**
- `UnitClass::Fire_At_Target` (`0x00736DF0`) — per-tick mission-attack handler for ground
  units. Handles fire result codes (0=OK, 2=REARM, 3=ROTATING, 4=FACING, 5=RANGE,
  9=FACING). Only code 0 triggers `vtable+0x3CC` (Fire → Fire_At).
- `InfantryClass::Fire_At_Target` (`0x005206B0`) — per-tick for infantry. Uses
  FiringSyncFrame/BurstDelay to gate firing until animation frame matches.
- `BuildingClass::Mission_Attack` (`0x0044ACF0`) — per-tick for turrets/gattling
  cannons. Similar pattern.
- `AircraftClass::Fire_At` (`0x00415EE0`) — thin wrapper delegating to
  `TechnoClass::Fire_At`; adds payload-drop handling.
- `FUN_00741340` (infantry Fire_At helper at TechnoClass level) — wraps Fire_At with
  ammo decrement and InfantryType-specific state updates. Reads
  `InfantryType+0xE40+iVar5*4` (FiringSyncFrame for burst_idx < 2) to gate a state
  machine flag at `this+0x5A0 / this+0x5A4`.

**Downstream (who reads `CurrentBurstIndex`):**
- `GetROF` (`0x006FCFA0`) — decides per-shot vs full-reload delay (the main consumer).
- `Fire_At` Gattling branch — cycles scatter-table offsets on each burst shot.
- `Fire_At` DiskLaser branch — same `CurrentBurstIndex++ ... % Burst` pattern.
- Building multi-barrel — uses a **separate** `MultiBarrelIndex` (+0x69C), not
  CurrentBurstIndex.

**Tick ordering:**
- `TechnoClass::AI_Update` at `0x006F9E50` increments `+0xC4` and dispatches mission.
- Mission_Attack (UnitClass/InfantryClass/BuildingClass) checks FireError → calls Fire.
- Fire (vtable+0x3CC) calls Fire_At.
- Fire_At increments CurrentBurstIndex, sets FireTimer, returns.
- Next tick's GetFireError sees FireTimer > 0 → FIRE_BUSY → no fire.
- Once FireTimer elapses, next fire proceeds.

---

## 6. Current Rust Implementation Status

Source: grep for `burst|Burst` in `src/`.

**What exists:**
- `src/rules/weapon_type.rs:54,193` — `WeaponType::burst: i32`, parsed from `Burst=` INI
  key, default 1. ✓ correct.
- `src/sim/combat/mod.rs`:
  - `AttackTarget::burst_remaining: u8` (line 123) — runtime counter.
  - `AttackTarget::burst_delay_ticks: u8` (line 125) — short inter-shot timer.
  - `BURST_INTER_SHOT_DELAY: u8 = 1` (line 130) — hardcoded 1-tick inter-shot delay.
  - Per-tick decrement of burst_delay (line 785).
  - Burst loop in combat advance (lines 1068-1206): if burst_remaining > 0 and
    burst_delay == 0, dispatch next shot and refresh burst_delay.

**Fidelity gaps vs gamemd.exe:**

1. **Per-shot delay is hardcoded to 1 tick.** Binary uses `Random::RandomRanged(3, 5)`
   for non-infantry, and per-infantry-type `BurstDelay0/1` values (default 0 → instant).
   A faithful port should:
   - For infantry with the 2nd+ shot: read `InfantryType.burst_delay[burst_idx - 1]`,
     falling through to random 3-5 if absent.
   - For all other units: `deterministic_rng.range(3, 5+1)` inclusive (sim RNG, not
     rand!).
   - Current `BURST_INTER_SHOT_DELAY = 1` is too short by 3-5×.

2. **Burst state lives on `AttackTarget` (a combat-only struct), not a persistent
   `CurrentBurstIndex` on the entity.** In the binary, `CurrentBurstIndex` (+0x3B8) is a
   TechnoClass field that persists across engagements — the cycling is `% Burst` and not
   reset to 0 between targets. This gives the "leftover quick shot on next engagement"
   edge-case behavior described in §3.5. Our Rust impl appears to reset burst state
   when the AttackTarget is dropped or retargeted — minor fidelity difference, unlikely
   to matter in practice.

3. **Random-jitter on final-shot ROF is missing.** Binary adds `Random::RandomRanged(0,
   2)` to the ROF value. Our `rof_to_cooldown_ticks` does not. Missing jitter means
   multiple units with identical ROF will synchronize their cadence exactly, which
   diverges from RA2 observed behavior. Easy fix: add `+ sim_rng.range(0, 3)` to ROF
   at the end-of-burst computation.

4. **Gattling scatter-table muzzle offset is not implemented.** The 8-entry
   `DAT_00b0eaa8` octagonal pattern and the `8 / Burst` stepping logic are missing
   from our renderer. (Gattling stage system itself is also not implemented — see
   GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md §6.)

5. **`BurstDelay0=` / `BurstDelay1=` INI keys are not parsed** on InfantryType. Since
   shipping YR INIs don't set them, this is low priority — but for mod compatibility
   it should be parsed.

6. **Sonic/particle weapons bypass burst shortening.** Our impl doesn't have the
   `GetROF` early-exit for `IsSonic`, `UseFireParticles`, `UseSparkParticles`,
   `IsRailgun`. If any of these weapons shipped with `Burst>1` (none do), our impl
   would diverge. Low priority.

---

## 7. Open Questions

1. **Confidence: LOW.** Does `CurrentBurstIndex` ever get reset when a unit's target is
   cleared, or does it truly persist across engagements as a raw modular counter? I
   could not find any code path that writes `this->field_0x3b8 = 0` outside of Fire_At's
   `% Burst` wraparound. The "leftover quick shot" hypothesis is inference, not
   verified. Would need to instrument a test with `Burst=3` and verify in-game that a
   unit that fires 2 of 3 shots, then retargets, fires its 3rd shot at the new target
   with the short delay.

2. **Confidence: LOW.** Does the game use any form of TS-legacy "RandomNoise" for
   burst spread? No strings matched. The user's brief mentioned it but the feature does
   not appear in the YR parser. Treat as non-existent / TS ghost.

3. **Confidence: MEDIUM.** For `BurstDelay0=0` (the shipping-default case for any
   Burst-using infantry), GetROF returns 0, and the next Fire_At runs immediately on
   the following tick. Does this behave as "same-tick double shot" or "next-tick"? Our
   Rust model treats it as inter-tick-delay; the binary is tick-based-only (FireTimer is
   checked in frames/ticks, not sub-tick). So BurstDelay=0 → fire again on tick T+1.
   Safe, but worth noting: Tesla Trooper's visible "double bolt" is actually ticks T
   and T+1, not same-tick.

4. **Confidence: LOW.** How does the building multi-barrel index at `+0x69C` interact
   with `CurrentBurstIndex`? They are separate counters but both cycle on fire.
   Preliminary read: `MultiBarrelIndex` advances the visual turret-barrel-choice (e.g.,
   which gun of a twin-barrel turret animates), while `CurrentBurstIndex` governs the
   burst-shot sequencing. They appear independent. Would need a dedicated
   `/re-investigate building multi-barrel` pass to confirm.

5. **Confidence: LOW.** Does `DiskLaser=yes` weapons compose with Burst correctly? The
   DiskLaser short-circuit in Fire_At also increments `CurrentBurstIndex` and sets
   FireTimer, but returns NULL instead of a bullet. If `Burst=2` on a DiskLaser weapon,
   does it produce 2 DiskLasers? Inspection says yes — same code path, just with a
   DiskLaserClass instead of BulletClass. Not tested in YR shipping (DiskLaser is on
   the Floating Disc, `Burst=1` in retail).

---

## 8. Sources

**Ghidra decompilations performed:**
- `0x006FDD50` `TechnoClass::Fire_At` (already fully documented in
  `FIRE_AT_ANALYSIS.md`)
- `0x006FCFA0` `TechnoClass::GetROF` — **new in this report**, the missing piece
- `0x005206B0` `InfantryClass::Fire_At_Target`
- `0x0051DF70` `InfantryClass::Fire_At_Override`
- `0x00736DF0` `UnitClass::Fire_At_Target`
- `0x00415EE0` `AircraftClass::Fire_At`
- `0x00741340` `FUN_00741340` (infantry Fire_At wrapper with FiringSyncFrame gate)
- `0x00747620` (Ghidra-mislabeled `UnitTypeClass::ReadINI`; actually
  `InfantryTypeClass::ReadINI` — contains `BurstDelay%d` and `FiringSyncFrame%d` loops)
- `0x005240A0` `InfantryTypeClass::ReadINI` (actual, separate function)
- `0x005236A0` `InfantryTypeClass::Constructor` — confirms default init of BurstDelay
  slots.

**String xrefs:**
- `"Burst"` at `0x00849438` → `WeaponTypeClass::ReadINI` entry `0x00772080`; call site using this string is at `0x007722C1` within the function (corrected 2026-05-29: was listed as function address; actual function entry is `0x00772080` via `get_function_by_address 0x00772080` — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT)
- `"BurstDelay%d"` at `0x00845CA0` → `InfantryTypeClass::ReadINI` (mislabeled) at
  `0x00747B14`

**Related docs:**
- `FIRE_AT_ANALYSIS.md` — full Fire_At decomposition
- `WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md` — Burst offset at 0x9C
- `TECHNOCLASS_STRUCT_LAYOUT.md` — CurrentBurstIndex (+0x3B8), FireTimer
- `TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md` — overall combat pipeline
- `TECHNOCLASS_VTABLE_COMPLETE.md` — GetROF at vtable+0x318, entry 198
- `GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md` — IsGattling stage system, scatter
  table, how it composes with Burst
- `BULLET_CLASS_AI_GHIDRA_REPORT.md` — Airburst mechanism (separate)
- `BULLET_CLASS_LAYOUT_GHIDRA_REPORT.md` — Shrapnel fields (separate)

**INI files:**
- `ini/rulesmd.ini` — 80+ `Burst=` occurrences; zero `BurstDelay=` occurrences.
- No `ini/artmd.ini` entries for burst (as expected — burst is a gameplay mechanic).

---

## Summary (quick reference)

- `Burst=N` on a weapon → **N independent Fire_At calls**, one per tick.
- Per-shot delay: **3-5 frames random** (non-infantry) or
  `InfantryType.BurstDelay[burst_idx-1]` (infantry, rarely set in YR; safe values only
  at indices 0-1).
- Burst state: `TechnoClass+0x3B8 = CurrentBurstIndex`, `int`, cycles `% Burst`, never
  explicitly reset.
- Spread pattern: **NONE for non-Gattling**. Gattling uses an 8-entry octagonal scatter
  table with step `8/Burst` to cycle muzzle position.
- Retargeting mid-burst: silently handled — next Fire_At uses new target; partial bursts
  are normal when targets die.
- Airburst, Shrapnel, Inaccurate: compose independently with Burst; not special cases.
