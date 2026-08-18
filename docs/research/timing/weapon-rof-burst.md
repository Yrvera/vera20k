# Weapon ROF / Burst / Ammo / Reload

## Overview

**Player-visible effect:** when a unit attacks, three timing surfaces
combine. `ROF=` is how many frames the unit waits between shots (or
between burst sequences). `Burst=` is how many shots come out
back-to-back when the trigger pulls (Burst=1 = single shot; Burst=2 =
"double tap"; Burst=N on a Gattling cycles through the barrels). `Ammo=`
(per-techno-type) caps the total shots before the unit must return to a
refueling pad (aircraft) or wait `Reload=` ticks (everything else).

**Mechanism in plain terms:** every techno keeps a single 4-field
"FireTimer" struct at `TechnoClass + 0x2EC..0x2F8`. After every shot,
the active weapon's `GetROF()` virtual is called to return the next
cooldown, written into `+0x2F8`. `GetFireError()` blocks subsequent
attempts until that countdown elapses. `GetROF` returns one of two
values: a **short** random delay (3–5 frames for non-infantry, or the
infantry-type's `BurstDelay0/1` slot) while mid-burst, and the **full**
`ROF=` value (with veterancy / naval / crate multipliers) on the last
shot of the burst. `CurrentBurstIndex` (`+0x3B8`) is the modular
counter — it increments on every fire and wraps `% weapon.Burst`. So
"Burst=N" doesn't mean "loop N times in one Fire_At call"; it means
"return a short delay for N-1 consecutive Fire_At calls, then return
the full ROF."

For aircraft and other ammo-limited units, each shot decrements
`Ammo` (per-instance). When `Ammo == 0`, the unit must dock with a
helipad / refinery / airfield, where `Reload` (per-techno-type, in
ticks) determines how long the reload takes. The global
`[General] ReloadRate=.3` (minutes per ammo point) is a separate
aircraft-only cadence used by the aircraft reload state machine.

The clock is the **game-tick clock** — `TechnoClass + 0x2EC` is a
`g_CurrentFrameCounter` snapshot, and the cooldown countdown is in
game ticks. So firing cadence scales with GameSpeed exactly like
everything else: at Fastest the wall-clock pace is up to ~6× the
Slowest pace, but the ROF-frame count is identical in both.

---

## INI surface

### `rulesmd.ini` — per-`[Weapon]` section

```ini
[Vulcan]
Damage=50
ROF=26
Range=5.5
Projectile=InvisibleLow
Speed=100
Warhead=SA
Report=SentryGunAttack
Anim=MGUN-N,MGUN-NE,MGUN-E,MGUN-SE,MGUN-S,MGUN-SW,MGUN-W,MGUN-NW
```

```ini
[Gattling]
Damage=30
ROF=10
Range=7;5.5
...
```

| Key | Type | Default | WeaponTypeClass byte offset | Notes |
|---|---|---|---|---|
| `ROF=` | int | `0` | `0xB0` (`[0x2c]`) | Cooldown in **game frames** between bursts (i.e., between `Fire_At` calls that return the *full* ROF) |
| `Burst=` | int | `1` | `0x9C` (`[0x27]`) | Shots per burst sequence. 1 = single-shot; 2+ = burst |
| `Damage=` | int | `0` | `0xA4` (`[0x29]`) | Per-shot damage (multiplied per-shot, not per-burst) |
| `Speed=` | int (0–255) | `0` | `0xA8` (`[0x2a]`) | Projectile speed; INI 0–100 mapped via `CCINIClass::ReadSpeed` (`0x00474810`) |
| `Range=` | int (leptons) | `0` | `0xB4` (`[0x2d]`) | Max range; INI in cells (double) → stored as leptons (× 256) via `ReadRange` (`0x00474620`) |
| `MinimumRange=` | int (leptons) | `0` | `0xB8` (`[0x2e]`) | Min range; same conversion |
| `Projectile=` | `BulletTypeClass*` | `NULL` | `0xA0` | Bullet type — affects projectile physics, not ROF |
| `Warhead=` | `WarheadTypeClass*` | `NULL` | `0xAC` | Damage-application warhead |
| `Anim=` | DVC of `AnimTypeClass*` | empty | `0xF4..0x10F` (28-byte DynamicVectorClass) | Per-direction muzzle anims (1 or 8 entries) |
| `Report=` | DVC of `int` (sound idx) | empty | `0xBC..0xD7` | Firing sound list |
| `DownReport=` | DVC of `int` | empty | `0xD8..0xF3` | Firing-downward sound list |
| `OccupantAnim=` | `AnimTypeClass*` | `NULL` | `0x110` | Muzzle anim when firing from a garrisoned building |
| `AssaultAnim=` | `AnimTypeClass*` | `NULL` | `0x114` | Muzzle anim when clearing a garrison |
| `OpenToppedAnim=` | `AnimTypeClass*` | `NULL` | `0x118` | Muzzle anim when firing from open-topped transport |
| `FireOnce=` | bool | `false` | `0x135` | Unit ceases firing after 1 shot then stops the attack mission entirely |
| `OmniFire=` | bool | `false` | `0x12B` | Can fire without turning the turret to face the target |
| `FireWhileMoving=` | bool | **`true`** | `0x141` | If false, unit halts to fire |
| `FireInTransport=` | bool | **`true`** | `0x143` | If false, unit can't fire from inside a transport |
| `Suicide=` | bool | `false` | `0x144` | Unit dies after firing |
| `Charges=` | bool | `false` | `0x148` | "Charges between shots" gate — used with `IsLaser` / Tesla |
| `DecloakToFire=` | bool | **`true`** | `0x133` | Must decloak before firing (adds decloak-anim time to first shot) |
| `LaserDuration=` | int8 | `10` | `0x14E` | Duration of laser beam visual in frames |
| `IsSonic` / `UseFireParticles` / `UseSparkParticles` / `IsRailgun` | bool | `false` | various | Make `GetROF` return the full ROF every shot (no burst shortening) |

**`BurstDelay=` is NOT a `[Weapon]` key.** Per
[BURST_WEAPON_FIRING_GHIDRA_REPORT.md](../BURST_WEAPON_FIRING_GHIDRA_REPORT.md):
no `BurstDelay=` exists in `WeaponTypeClass::ReadINI`. The per-shot
delay for non-infantry is hardcoded `Random::RandomRanged(3, 5)`. For
infantry, the per-shot delay is read from the `[InfantryType]` section
via the `BurstDelay0` / `BurstDelay1` keys (see below).

### `rulesmd.ini` — per-`[InfantryType]` section (BurstDelay)

`InfantryTypeClass::ReadINI` (Ghidra-labeled `UnitTypeClass__ReadINI`
at `0x00747620`) parses 4 keys via `sprintf("BurstDelay%d", N)`:

```c
// Loop at 0x00747c64
iVar11 = 0;
do {
    FUN_007c8ef4(local_138, s_BurstDelay_d_00845ca0, iVar11);  // "BurstDelay%d"
    uVar4 = CCINIClass__ReadInt(iVar1, local_138, *puVar5);
    *puVar5 = uVar4;
    iVar11 = iVar11 + 1;
    puVar5 = puVar5 + 1;
} while (iVar11 < 4);
```

| Key | InfantryTypeClass byte offset | Default | Status |
|---|---|---|---|
| `BurstDelay0=` | `0xE48` | `0` (constructor) | **Safe to set** — used by Burst=2 on infantry |
| `BurstDelay1=` | `0xE4C` | `0` | **Safe to set** — used by Burst=3 on infantry |
| `BurstDelay2=` | `0xE50` | — | **UNSAFE** — overlaps a `DynamicVectorClass` vtable; setting will crash |
| `BurstDelay3=` | `0xE54` | — | **UNSAFE** — same overlap risk |

**Shipping `rulesmd.ini` does NOT set `BurstDelay*=` on any infantry**
(per the existing report). All shipping infantry-burst weapons fall
through to the `RandomRanged(3, 5)` default.

### `rulesmd.ini` — per-`[TechnoType]` (Ammo / Reload)

| Key | TechnoTypeClass byte offset | Default | Notes |
|---|---|---|---|
| `InitialAmmo=` | `0x680` | `-1` (no ammo cap) | Starting ammo count for the unit |
| `Ammo=` | `0x684` | `-1` (no ammo cap) | Max ammo capacity (separate from initial; differs only via crate / upgrade) |
| `EmptyReload=` | `0x69C` | (constructor default) | Frames to reload when fully empty |
| `Reload=` | `0x698` | (constructor default) | Frames to reload one ammo point (general) |
| `ReloadIncrement=` | `0x6A0` | (constructor default) | Per-shot increment to reload counter |

Reload keys read by `TechnoTypeClass::ReadINI` @ `0x00714710`:

```
// from 0x00714871, ReadInt for "Reload" stores at offset 0x698
// from 0x0071488b, ReadInt for "EmptyReload" stores at offset 0x69C
// from 0x007148a5, ReadInt for "ReloadIncrement" stores at offset 0x6A0
```

Verified via direct disassembly read of the ReadINI block.

### `rulesmd.ini` — `[General]` (global aircraft reload)

```ini
ReloadRate=.3           ; minutes to reload each ammo point for aircraft or helicopters
```

Read by `RulesClass::ReadGeneral` @ `0x00670c86` via `CCINIClass::ReadDouble`:

```
// 8b 86 0C 15 00 00   mov eax, [esi+0x150C]   ; high dword of default
// 8b 8e 08 15 00 00   mov ecx, [esi+0x1508]   ; low dword of default
// 50 51               push them
// 68 6c be 83 00      push 0x83be6c           ; "ReloadRate"
// 52 8b cf            push <section>, mov ecx,<this>
// e8 3d 77 eb ff      call CCINIClass::ReadDouble
// dd 9e 08 15 00 00   fstp qword [esi+0x1508] ; store double result
```

So **`ReloadRate` lives at `RulesClass + 0x1508`** as a `double` (in
**minutes**, default `.3` = 18 seconds wall-clock per ammo point). At
the standard game speed this is **only consulted by aircraft / helicopter
docking logic** (Rocketeer, Harrier, Black Eagle, Hornet, Yuri Disc do
not use ammo this way — they use the per-techno `Reload` field). Detail
of the aircraft reload state machine lives in a future doc on aircraft
docking; cross-referenced here.

### `rulesmd.ini` — Spawn / Slave / Manual reload

```ini
UnitReload=yes          ; (sample: appears on yuri sub spawner pad and aircraft carrier)
```

| Key | Read in | Notes |
|---|---|---|
| `SpawnReloadRate=` | (per-TechnoType — used by `SpawnManagerClass`) | Spawnee return + reload cadence; owned by [SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md) |
| `SlaveReloadRate=` | (per-TechnoType — used by `SlaveManagerClass`) | Slave dump cadence; owned by [SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md](../SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md) |
| `ManualReload=` | per-TechnoType | True = unit doesn't auto-reload; player must manually trigger |
| `UnitReload=` | per-TechnoType | True = unit can reload by returning to a refit pad (used by sub spawner pad / aircraft carrier) |

These are **production / spawn / docking** timings rather than the
weapon-firing cadence. Listed here for completeness; their detailed
semantics are owned by their respective system docs.

### `artmd.ini` — per-`[Weapon]`-Anim block

Per-weapon muzzle animations are listed via `Anim=` (above). Their
per-frame timing follows the standard `Rate=` / `Delay=` rules in
[animation-rate-delay.md](animation-rate-delay.md). The muzzle-flash
animation does **not** drive the ROF cooldown; it plays for whatever
duration its `End=` and `Rate=` dictate, independent of when the next
shot fires.

---

## Hardcoded constants

### `Random::RandomRanged(3, 5)` — non-infantry inter-shot delay

From `TechnoClass::GetROF` @ `0x006FCFA0`, mid-burst branch (when
`CurrentBurstIndex > 0` and the unit is not infantry, or the infantry's
`BurstDelay[burst_idx-1]` is `-1`):

```c
return Random::RandomRanged(3, 5);
```

So between mid-burst shots: **3, 4, or 5 game ticks** (uniform random).
At Medium GameSpeed (≈20 ticks/sec) this is **150–250 ms wall-clock**;
at Fastest (uncapped, ~60 ticks/sec ceiling) it's ~50–83 ms. Effect:
two shots of a Burst=2 weapon look like a "double tap" rather than
"BANG BANG" simultaneous fire.

### End-of-burst ROF jitter

```c
jitter = Random::RandomRanged(0, 2);   // 0, 1, or 2
rof = Math::ftol(weapon.ROF * (1.0 + jitter/<scale>));
```

The last shot of a burst applies a small random scaling to the
weapon's `ROF=` so that multiple units with identical ROF don't lock
into a perfect rhythm. Exact `<scale>` divisor not extracted in this
iteration — flag for follow-up if a "fire-sync-jitter" detail becomes
load-bearing.

### Veterancy / naval / crate multipliers (applied in `GetROF` end-of-burst branch)

```c
if (IsVeteran && type.VeteranAbilities & FIREPOWER)  rof *= vet_mult;
if (IsElite   && type.EliteAbilities  & FIREPOWER)   rof *= elite_mult;
if (IsNaval   && barrel_count > 0)                   rof /= barrel_count;
if (this.has_crate_powerup && ~is_building)          rof *= crate_mult;
```

- **Veteran/Elite FIREPOWER bit**: faster ROF when the unit is promoted
  *and* its type has the FIREPOWER veterancy bit set (in
  `VeteranAbilities` / `EliteAbilities` masks). Exact multipliers
  (`vet_mult`, `elite_mult`) live in `RulesClass` globals; identification
  deferred to a veterancy-effects doc.
- **Naval barrel-count division**: ships with multi-barrel artillery
  (`type.GetBarrelCount() > 0` and `IsNaval`) divide ROF by barrel
  count — each barrel effectively shares the same overall cooldown,
  so a 2-barrel cruiser fires twice as often as a 1-barrel cruiser at
  the same `ROF=`. Independent of `Burst=`; orthogonal mechanic.
- **Crate powerup**: when a unit has picked up a firepower crate
  (`+0x691` byte? — flag bit at this address), ROF is scaled. The "not
  building" exemption (`~is_building`) means buildings do **not**
  benefit from crate firepower bonuses (which is correct because
  buildings can't pick up crates).

### Building-multi-barrel shortcut

```c
if (this->WhatAmI() == 6 && this->byte_0x2FC > 1)
    return 1;
```

`WhatAmI() == 6` = `BuildingClass`. `byte_0x2FC > 1` = "currently
firing a multi-barrel shot" (multi-tower defense gates like Prism
Tower coupling). When set, `GetROF` returns **1** — the building can
fire on the next tick. Used by Prism Tower's beam-relay multi-fire
sequence (each tower in the chain immediately ready to fire when
multi-relay is active).

### FireTimer struct in `TechnoClass`

| Byte offset | Field | Type | Purpose |
|---|---|---|---|
| `0x2EC` | `FireTimer.StartFrame` | int | `g_CurrentFrameCounter` snapshot at last shot |
| `0x2F0` | `FireTimer.Range` | uint | Scratch copy of range used in projectile init |
| `0x2F4` | `FireTimer.InitialValue` | int | Initial cooldown for `GetTimeRemaining` math |
| `0x2F8` | `FireTimer.ROF` | int | **Active cooldown count** — set by `GetROF`; decrements via `g_CurrentFrameCounter - StartFrame` math |
| `0x3B8` | `CurrentBurstIndex` | int | 0..Burst-1; incremented in `Fire_At`; wraps `% Burst` |

The cooldown check pattern (used by `GetFireError`):

```c
int elapsed = g_CurrentFrameCounter - this->field_0x2EC;   // StartFrame
if (elapsed < this->field_0x2F8) return FIRE_BUSY;          // not yet
return FIRE_OK;
```

Same pattern as `HouseClass::Update`'s timer fields (per
[logic-vs-render-loop.md](logic-vs-render-loop.md)). All techno-side
timers use this `g_CurrentFrameCounter` snapshot delta convention.

### `Fire_At` per-call work

`TechnoClass::Fire_At` @ `0x006FDD50` does, per call (one bullet per call):

```c
// (1) Verify weapon and target
// (2) Compute FLH (Fire Location + Height) via vtable+0xB0
// (3) Allocate BulletClass, set position, velocity, owner, target
// (4) bullet.Launch (vtable+0x1F0)
// (5) Increment CurrentBurstIndex:
this->CurrentBurstIndex = this->CurrentBurstIndex + 1;
// (6) Get next ROF and arm the cooldown:
rof = this->vtable.GetROF();
if (this->field_0x298) rof /= 2;          // half-ROF modifier flag
this->field_0x2F8 = rof;
this->field_0x2EC = g_CurrentFrameCounter;
this->field_0x2F0 = uStack_a8;
this->field_0x2F4 = rof;
// (7) Wrap burst index:
this->CurrentBurstIndex = this->CurrentBurstIndex % weapon.Burst;
```

The `this->field_0x298` "half-ROF flag" is set elsewhere (likely Iron
Curtain or a veterancy-related buff — TODO confirm); when set, every
ROF return value is halved before being applied. **Player-visible
effect when set:** unit fires twice as fast. Identification deferred.

### Gattling scatter table

`DAT_00b0eaa8` — 8-entry octagonal scatter (verified in existing report):

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

When `TechnoType.IsGattling=yes`, `Fire_At` steps `GattlingScatterIndex`
by `8 / weapon.Burst` per shot (e.g., `Burst=4` → step 2, hitting
indices 0/2/4/6). The first shot of a burst picks a random starting
index. **This is the only built-in spatial spread for bursts.** Used
by Yuri Gattling Tank, Allied Gattling Cannon. Spotted infantry burst
weapons (Flak Trooper Burst=2) get their visual "alternating barrels"
from the firing animation rotating, not from this scatter table.

Cross-ref [GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md](../GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md)
for Gattling's separate "stage escalation" mechanic (which stages of
the gattling weapon set are active over time as the unit holds the
trigger).

### "Sticky" weapons bypass burst shortening

```c
if (weapon.IsSonic ||
    (weapon.UseSparkParticles && this.sparkParticleSys) ||
    (weapon.UseFireParticles && this.fireParticleSys) ||
    (weapon.IsRailgun && this.railgunParticleSys))
    return weapon.ROF;
```

For these four flag categories, `GetROF` returns the **full** weapon
ROF on every shot — burst is silently neutralized. These are
continuous-beam / sustained-particle weapons where the shot duration
itself encodes the "burst". Examples: Soviet Apocalypse rocket
particle system, Allied Prism beam particle, Sonic Disrupter (`IsSonic`),
Railgun units.

### `BurstDelay[burst_idx-1]` infantry lookup formula

```c
if (0 < burst_idx && burst_idx < 5 && is_infantry) {
    int delay = infantry_type[0xE44 + burst_idx*4];  // = +0xE48 + (idx-1)*4
    if (delay != -1) return delay;
    // fall through to RandomRanged(3,5)
}
```

So infantry indices 1..4 read `BurstDelay0`..`BurstDelay3`. Sentinel
`-1` = "not set" = fall through to the random 3–5 default. Per the
unsafe-overlap caveat above, only indices 1 and 2 (`BurstDelay0` /
`BurstDelay1`) should ever be set in INI.

### `Reload` defaults and behavior

`Reload=` (`TechnoType + 0x698`) is read with a 5th parameter from
the constructor's default; the constructor's stored default isn't
extracted in this iteration but the read pattern matches the standard
`ReadInt(section, "Reload", *default_ptr)`. **For aircraft**, the
reload state machine uses `Reload` directly as game-tick countdown
when the aircraft has docked at a helipad / airfield. **For ground
units with `Ammo=N`** (rare — most are unbounded), `Reload` is the
per-ammo-point recharge interval.

Format strings present in the binary:

| Address | String |
|---|---|
| `0x0081aae4` | `"UnitReload"` |
| `0x0083be6c` | `"ReloadRate"` |
| `0x008437a8` | `"SpawnReloadRate"` |
| `0x008437f4` | `"SlaveReloadRate"` |
| `0x00843a40` | `"ReloadIncrement"` |
| `0x00843a50` | `"EmptyReload"` |
| `0x00843a5c` | `"Reload"` |
| `0x00843aec` | `"InitialAmmo"` |
| `0x00844138` | `"ManualReload"` |

---

## Tick / frame topology

| Stage | Clock | Where |
|---|---|---|
| `Fire_At` per-call cooldown arm | game-tick | `TechnoClass::Fire_At` at `0x006FDD50` |
| `GetFireError` cooldown check | game-tick | reads `g_CurrentFrameCounter - field_0x2EC` vs `field_0x2F8` |
| `GetROF` virtual dispatch | game-tick | `0x006FCFA0` |
| `CurrentBurstIndex` increment | game-tick | once per `Fire_At` |
| Ammo decrement | game-tick | per shot (in `Fire_At`'s post-launch block) |
| Reload countdown (per-type `Reload`) | game-tick | counted in `g_CurrentFrameCounter` units |
| `[General] ReloadRate` (aircraft) | game-tick (`minutes * 60 * tick_rate`) | aircraft state machine; resolved against current GameSpeed when the docking countdown is established |
| Muzzle anim playback | game-tick (via anim Rate) | spawned `AnimClass` instance; see [animation-rate-delay.md](animation-rate-delay.md) |

### Per-attack sequence (Burst=2 example, normal unit)

Assume `ROF=26`, `Burst=2`, target alive throughout.

**Tick T** (`CurrentBurstIndex` was 0):
1. Mission_Attack → `GetFireError` returns FIRE_OK (cooldown = 0)
2. `Fire_At` → bullet launched at target
3. `CurrentBurstIndex` becomes 1
4. `GetROF` returns `RandomRanged(3, 5)` = (say) **4**
5. FireTimer armed: `field_0x2F8 = 4`, `field_0x2EC = T`
6. `CurrentBurstIndex % 2` = 1 (still mid-burst)

**Tick T+1, T+2, T+3** (`elapsed < 4`):
- `GetFireError` returns FIRE_BUSY — no fire

**Tick T+4** (`elapsed >= 4`):
1. FIRE_OK
2. `Fire_At` → bullet launched (re-aimed at target's now-current position)
3. `CurrentBurstIndex` becomes 2
4. `GetROF` runs end-of-burst branch:
   - jitter = (say) 1
   - rof = ftol(26 * (1.0 + 1/<scale>)) = ~28
   - veterancy/crate multipliers applied
5. FireTimer armed: `field_0x2F8 = 28`, `field_0x2EC = T+4`
6. `CurrentBurstIndex % 2` = 0 (burst complete)

**Tick T+5..T+31** (`elapsed < 28`):
- FIRE_BUSY

**Tick T+32**: ready to start next burst.

Net wall-clock cadence at GameSpeed=Medium (≈20 ticks/sec):
- Two shots ~200 ms apart (the burst)
- ~1.4 s gap (the ROF)
- Repeat

### Burst index does NOT reset on target loss

If target dies mid-burst, `CurrentBurstIndex` stays at 1. The next
engagement starts with `burst_idx == 1`, meaning the very first shot
of the next burst uses the short inter-shot delay instead of full ROF
— net effect: a "free leftover quick shot" against the new target.
This is the engine's deterministic behavior; not a bug per the parity
bar but worth porting faithfully.

---

## Multipliers and modifiers

### `CurrentBurstIndex` cycling (mid-burst vs end-of-burst)

The mod-N counter is the entire "burst state machine". No separate flag.

### Veterancy FIREPOWER bit

Per-techno-type. When set in `VeteranAbilities` or `EliteAbilities`,
ROF is scaled. Multiplier values in `RulesClass` — see future
veterancy doc.

### Naval barrel count

When unit `IsNaval` and `vtable.GetBarrelCount() > 0`, ROF is divided
by barrel count. So a 4-barrel naval unit fires 4× as often as a
1-barrel naval unit at the same `ROF=`. **Does not affect Burst** —
each barrel gets its own full burst.

### Crate firepower powerup

Per-instance flag on the `TechnoClass`. When set and unit is not a
building, ROF is scaled. Identification of the exact flag bit deferred.

### Half-ROF modifier `field_0x298`

A per-instance flag on `TechnoClass`. When set, ROF is halved on every
return from `GetROF` (applied in `Fire_At` after `GetROF` returns,
before storing to `field_0x2F8`). Likely linked to Iron Curtain or a
specific buff — TODO identify.

### `weapon.Burst` directly

Smaller `Burst=` = fewer mid-burst shots before the full-ROF cooldown
kicks in. `Burst=1` → every shot returns the full ROF (no shortening).

### Sonic / particle / railgun "sticky" exemption

Bypass burst shortening — full ROF every shot. Effectively forces
`Burst=1` behavior regardless of the configured `Burst=`.

### Building multi-barrel

`WhatAmI() == 6 && byte_0x2FC > 1` → return 1 (immediate ready). Used
by Prism Tower beam-relay.

### `[General] ReloadRate` (aircraft only)

Default `.3` minutes = 18 wall-clock seconds. Converted to game ticks
at the time the aircraft begins docking-reload. The conversion is
`reload_ticks = ReloadRate_minutes * 60 * effective_tick_rate`, where
`effective_tick_rate` depends on current GameSpeed (see
[game-speed-master-clock.md](game-speed-master-clock.md)). So at
GameSpeed=Slowest (≈10 ticks/sec), 18s = 180 ticks; at Fastest
(≈60 ticks/sec), 18s = 1080 ticks. **Player-visible effect:** aircraft
take the same wall-clock time to reload regardless of GameSpeed.

**Confidence on the wall-clock-constancy claim: MEDIUM** — the
conversion happens at countdown-arm time. If GameSpeed changes
mid-reload, the countdown does not re-scale; only newly-armed reloads
get the new rate. This is consistent with how the engine handles
`g_CurrentFrameCounter`-based timers in general.

---

## Edge cases

### Aircraft and `Ammo`

Aircraft (Rocketeer, Harrier, Black Eagle, Hornet, Yuri Disc) are
"hard-wired to require ammo" (per the in-INI comment). When ammo
drops to 0, the aircraft's mission state transitions to "return to
helipad" and on landing the reload state machine consumes
`ReloadRate * 60 * tick_rate` ticks per ammo point until full. Detailed
state machine deferred to a future aircraft-docking doc.

### Ground units with `Ammo=N`

Sentry Gun (`[Vulcan]` weapon with the Tower, Ammo=100 from sample) /
some Yuri units have finite ammo on the ground. When empty, the
per-type `Reload`/`EmptyReload`/`ReloadIncrement` triple determines
the cadence. Most ground units have `Ammo=-1` (default) → unlimited.

### `FireOnce=yes`

Per `WeaponTypeClass + 0x135`. After firing, the unit stops the
attack mission entirely. Used by suicide units and one-shot weapons
(some V3 variants?). The cooldown still arms but is irrelevant
because the unit won't fire again.

### `Suicide=yes`

Per `WeaponTypeClass + 0x144`. Unit dies after firing. Used by Yuri
Demolition Truck. Burst is meaningless here because the unit dies on
the first shot.

### `Charges=yes`

Per `WeaponTypeClass + 0x148`. Used by Tesla units. Acts as a gate
that requires the unit to be in a "charging up" mission state for
some duration before each shot; the charge anim plays during this.
The exact charge-time field is somewhere on the techno type;
identification deferred to a tesla-charge doc.

### `DecloakToFire=yes` (default)

Per `WeaponTypeClass + 0x133`. If the unit is cloaked, it must first
decloak (which takes `CloakingSpeed` ticks per
[cloak-uncloak-delay.md](cloak-uncloak-delay.md)) before the shot can
fire. This adds ~10–15 ticks to the first shot's effective cadence.

### Pause behavior

Per [logic-vs-render-loop.md](logic-vs-render-loop.md): the per-entity
vtable-`+0x5c` AI loop runs unconditionally during pause, but the
gameplay block (which includes Mission_Attack dispatch) is gated by
`g_GameState == 0`. So during the in-game menu pause, units that
already have an attack mission and a ready cooldown will **not** fire
new shots (because Mission_Attack doesn't run), but their FireTimer
keeps counting (because `g_CurrentFrameCounter` advances). Net effect:
opening the menu freezes attacks, but unfreezing them resumes with
the cooldown already partially / fully elapsed.

### Save / load mid-burst

`Fire_At`'s state (FireTimer fields, `CurrentBurstIndex`) is part of
the techno save state. Reload restores all fields. Animation in
flight (muzzle flash already spawned, bullet in flight) is restored
because the AnimClass / BulletClass instances are themselves saved.
Reload during a half-fired Burst=2 → unit resumes with `burst_idx ==
1` and the appropriate cooldown remaining.

### Replay determinism

`Random::RandomRanged` is deterministic across peers/replay. So the
3..5 inter-shot delay and the end-of-burst jitter both repeat
identically. Replays look bit-identical.

### Retargeting mid-burst (see existing report)

Per [BURST_WEAPON_FIRING_GHIDRA_REPORT.md](../BURST_WEAPON_FIRING_GHIDRA_REPORT.md):
if the target dies between shots, `CurrentBurstIndex` does not reset.
The next engagement begins with `burst_idx == 1` (leftover from the
incomplete burst), so the first shot of the next engagement uses the
short inter-shot delay. Faithful to gamemd; port verbatim.

### Iron Curtain / EMP / Stasis affecting firing

`field_0x298` (half-ROF flag) may be set by one of these buffs — TODO
confirm. Iron Curtain doesn't prevent firing (units under IC keep
firing); EMP does prevent firing (clears the active mission); Mind
Control retargets but doesn't stop firing.

---

## TS-legacy filter

| Field / branch | TS-legacy? | Notes |
|---|---|---|
| `ROF=` / `Burst=` core | **Live in YR** | Universal weapon timing. |
| `Range=` / `MinimumRange=` / `Damage=` / `Speed=` / `Warhead=` / `Projectile=` | **Live in YR** | Core weapon fields. |
| `Anim=` / `Report=` / `DownReport=` | **Live in YR** | Muzzle anim and sound. |
| `OccupantAnim=` / `AssaultAnim=` / `OpenToppedAnim=` | **Live in YR** | Garrison + transport firing. |
| `BurstDelay0` / `BurstDelay1` on infantry | **Parsed, never used in shipping YR** | All shipping infantry-burst weapons fall through to the random default. |
| `BurstDelay2` / `BurstDelay3` | **Unsafe** | Memory-corrupting overlap. Don't use. |
| `FireOnce=` / `Suicide=` / `Charges=` | **Live in YR** | Demolition Truck, Tesla, etc. |
| `OmniFire=` / `FireWhileMoving=` / `FireInTransport=` | **Live in YR** | |
| `DecloakToFire=` | **Live in YR** | Cloaked units. |
| `TerrainFire=` / `SabotageCursor=` / `MigAttackCursor=` / `DisguiseFireOnly=` | **Live in YR** | |
| `IsSonic` / `UseFireParticles` / `UseSparkParticles` / `IsRailgun` sticky-ROF exemption | **Live in YR** | Sonic Disrupter, particle weapons. |
| `IsLaser` / `DiskLaser` / `LaserDuration` / `IsHouseColor` / `LaserOuterSpread` / `IsBigLaser` / `IsLine` | **Live in YR** | Prism, GGI laser, etc. |
| `IsRadBeam` / `IsRadEruption` / `RadLevel` | **Live in YR** | Desolator. |
| `IsElectricBolt` / `DrawBoltAsLaser` | **Live in YR** | Tesla. |
| `IsMagBeam` | **Live in YR** | Magnetron. |
| `Camera=` / `RevealOnFire=` | **Live in YR** | |
| `InfiniteMindControl=` / `DrainWeapon=` | **Live in YR** | Yuri Prime, Yuri Clone. |
| `ManualReload=` | **Live in YR** | |
| `UnitReload=` | **Live in YR** | Sub Spawner Pad / Aircraft Carrier. |
| `[General] ReloadRate` | **Live in YR** | Aircraft reload. |
| `IPC sub-weapon / TS Cyborg-cannon weapons` | **TS-only, may exist in binary** | Not present in YR INI; would need a Tiberian Sun trigger to fire — defensive code only. |

---

## Cross-references

- [game-speed-master-clock.md](game-speed-master-clock.md) — defines
  `g_CurrentFrameCounter` that FireTimer counts in
- [logic-vs-render-loop.md](logic-vs-render-loop.md) — confirms
  Mission_Attack runs in the gameplay block (paused by `g_GameState != 0`)
  but FireTimer advances regardless
- [animation-rate-delay.md](animation-rate-delay.md) — muzzle-flash
  anim Rate / Delay
- [weapon-charge-and-muzzle.md](weapon-charge-and-muzzle.md) — `Charges=yes`
  charge anim, recoil frames, FLH (Fire Location + Height) per shot
- [cloak-uncloak-delay.md](cloak-uncloak-delay.md) — `DecloakToFire=yes`
  pre-shot decloak delay
- [BURST_WEAPON_FIRING_GHIDRA_REPORT.md](../BURST_WEAPON_FIRING_GHIDRA_REPORT.md) —
  existing detailed report on burst dispatch, Gattling scatter,
  Burst+Airburst+Shrapnel composition
- [WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md](../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md) —
  full struct layout reference
- [GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md](../GATTLING_WEAPON_STAGE_SYSTEM_GHIDRA_REPORT.md) —
  the *stage escalation* mechanic (separate from Burst); a Gattling
  weapon has multiple stages it transitions through as the unit holds
  the trigger, each stage being a different weapon with its own ROF /
  Burst / Damage
- [TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md](../TECHNOCLASS_COMBAT_WEAPON_SYSTEMS_REPORT.md) —
  combat dispatch chain (Mission_Attack → GetFireError → Fire → Fire_At)
- [AIRBURST_SUB_WEAPON_SPAWN_GHIDRA_REPORT.md](../AIRBURST_SUB_WEAPON_SPAWN_GHIDRA_REPORT.md) —
  Airburst secondary-volley composition with Burst
- [SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md) —
  `SpawnReloadRate` semantics (Aircraft Carrier, Boomer)
- [SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md](../SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md) —
  `SlaveReloadRate` semantics (Slave Miner)

---

## Coverage audit

| Item | Disposition |
|---|---|
| `[Weapon] ROF` | Owned here |
| `[Weapon] Burst` | Owned here |
| `[Weapon] Damage` / `Speed` / `Warhead` / `Projectile` | Cross-referenced to weapon-system / bullet-system docs (timing-irrelevant) |
| `[Weapon] Range` / `MinimumRange` | Owned here (range gates firing eligibility, not timing per se) |
| `[Weapon] Anim` / `Report` / `DownReport` / `OccupantAnim` / `AssaultAnim` / `OpenToppedAnim` | Visual / audio; cross-referenced to [animation-rate-delay.md](animation-rate-delay.md) and [voice-cooldown-overlap.md](voice-cooldown-overlap.md) |
| `[Weapon] FireOnce` / `Suicide` / `Charges` / `DecloakToFire` | Owned here |
| `[Weapon] OmniFire` / `FireWhileMoving` / `FireInTransport` | Owned here |
| `[Weapon] IsSonic` / `UseFireParticles` / `UseSparkParticles` / `IsRailgun` sticky-ROF | Owned here |
| `[Weapon] LaserDuration` | Cross-referenced to [weapon-charge-and-muzzle.md](weapon-charge-and-muzzle.md) |
| `[Weapon] Other visual flags (IsLaser/DiskLaser/IsRadBeam/...)` | Visual; cross-referenced to combat / particle docs |
| `[InfantryType] BurstDelay0/1` | Owned here |
| `[InfantryType] BurstDelay2/3` | Owned here (flagged as unsafe — do not use) |
| `[InfantryType] FiringSyncFrame0/1` | Cross-referenced to [infantry-sequence-timing.md](infantry-sequence-timing.md) (the sync between firing animation frame and shot dispatch) |
| `[TechnoType] InitialAmmo` / `Ammo` | Owned here |
| `[TechnoType] Reload` / `EmptyReload` / `ReloadIncrement` | Owned here |
| `[TechnoType] ManualReload` / `UnitReload` | Owned here |
| `[TechnoType] SpawnReloadRate` | Cross-referenced to [SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md](../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md) |
| `[TechnoType] SlaveReloadRate` | Cross-referenced to [SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md](../SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md) |
| `[General] ReloadRate` (aircraft reload, .3 min default) | Owned here |
| `[General] AmmoCrateDamage` | Cross-referenced (crate / explosion damage — not a timing) |
| TechnoClass FireTimer (`+0x2EC..+0x2F8`) | Owned here |
| TechnoClass `CurrentBurstIndex` (`+0x3B8`) | Owned here |
| TechnoClass `field_0x298` half-ROF flag | Owned here (flagged — exact source / semantics deferred) |
| TechnoClass `GattlingScatterIndex` (`+0x2A0`) | Cross-referenced to existing burst-firing report |
| Random 3–5 inter-shot delay | Owned here |
| End-of-burst ROF jitter | Owned here |
| Veterancy / naval barrel-count / crate ROF multipliers | Identified here; deferred to veterancy / naval / crate docs for multiplier values |
| Building multi-barrel `WhatAmI() == 6 && byte_0x2FC > 1` shortcut | Owned here |
| Gattling scatter table | Cross-referenced to existing burst-firing report |
| Sticky-ROF (Sonic/SparkParticle/FireParticle/Railgun) | Owned here |

---

## Ghidra queries log (this iteration)

| Query | Result |
|---|---|
| Read [WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md](../WEAPONTYPECLASS_FULL_STRUCT_LAYOUT.md) lines 1–230 | Confirmed `ROF` @ `0xB0` ([0x2c]), `Burst` @ `0x9C` ([0x27]), `Range` @ `0xB4`, all defaults, and the full per-key INI map |
| Read [BURST_WEAPON_FIRING_GHIDRA_REPORT.md](../BURST_WEAPON_FIRING_GHIDRA_REPORT.md) lines 1–399 | Confirmed `Fire_At` @ `0x006FDD50`, `GetROF` @ `0x006FCFA0`, `CurrentBurstIndex` @ `+0x3B8`, FireTimer @ `+0x2EC..+0x2F8`, infantry BurstDelay0/1 at `+0xE48/+0xE4C`, mid-burst random 3–5 delay, sticky-ROF exemption list, end-of-burst jitter |
| `search_strings "Reload"` | 8 hits: `UnitReload`, `ReloadRate`, `SpawnReloadRate`, `SlaveReloadRate`, `ReloadIncrement`, `EmptyReload`, `Reload`, `ManualReload` |
| `search_strings "Ammo"` | 2 hits: `AmmoCrateDamage`, `InitialAmmo` |
| `get_xrefs_to 0x0083be6c` (`ReloadRate`) | `RulesClass::ReadGeneral` @ `0x00670c86` |
| `read_memory 0x00670c70 len=64` | Decoded the ReadDouble call → `ReloadRate` stored at `RulesClass + 0x1508` (double) |
| `get_xrefs_to 0x00843a5c` (`Reload`) | `TechnoTypeClass::ReadINI` @ `0x00714871` |
| `get_xrefs_to 0x00843a50` (`EmptyReload`) | `TechnoTypeClass::ReadINI` @ `0x0071488b` |
| `get_xrefs_to 0x00843a40` (`ReloadIncrement`) | `TechnoTypeClass::ReadINI` @ `0x007148a5` |
| `get_xrefs_to 0x00843aec` (`InitialAmmo`) | `TechnoTypeClass::ReadINI` @ `0x00714755` |
| `read_memory 0x0071486b len=128` | Decoded the byte offsets: `Reload` → `+0x698`, `EmptyReload` → `+0x69C`, `ReloadIncrement` → `+0x6A0` |
| `read_memory 0x0071472b len=128` | Decoded `InitialAmmo` → `+0x680` and a second-field-after at `+0x684` (probably `Ammo=`) |
| `grep -nE "^Ammo=\|^Burst=\|^ROF=" ini/rulesmd.ini` | Confirmed range of values: ROF .3 to ~26+, Ammo 1 (aircraft), Ammo 5 (V3), Ammo 100 (Sentry Gun) |
| Read `[Vulcan]`, `[Gattling]`, `[M60]` sections | Confirmed Vulcan ROF=26 (slow defense), Gattling ROF=10 (fast), M60 ROF=20 (medium); none of them set Burst (default 1) |
