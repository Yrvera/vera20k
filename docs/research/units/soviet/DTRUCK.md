# DTRUCK — Demolitions Truck (Soviet suicide nuke truck)

**Side classification:** Soviet (Owner=Russians,Confederation,Africans,Arabs;
**RequiredHouses=Africans** — Libyan native build, Secret Lab universal pool).
**Role:** Soviet kamikaze unit. Drives to target, fires `Demobomb` weapon at Range=1
with `Suicide=yes` — the truck destroys itself in a massive Damage=300, CellSpread=8
explosion (with `RadLevel=100` rad-field aftermath). On any death (including
enemy-killed before reaching target), `Explodes=yes` + `DeathWeapon=Demobomb` triggers
the same Demobomb detonation. **Essentially impossible to neutralize without killing
it at a safe distance.**

> Output bar: the Demobomb's 8-cell radius + 150% steel-building Verses makes this
> the single most dangerous siege unit in the Soviet lineup. Parity-critical: blast
> radius, falloff curve (`PercentAtMax=.1` at edge — sharp falloff after centre),
> `InfDeath=4` burn-death animation, and the `RadLevel=100` rad-field-leftover all
> must reproduce gamemd's "nuclear truck wipes a base" feel exactly.

> **Companion docs**:
> - [`allied/TNKD.md`](../allied/TNKD.md) — German Tank Destroyer. DTRUCK and TNKD
>   are 2 of 3 `SecretUnits` (the 3rd is TTNK Tesla Tank). All three are
>   `RequiredHouses=`-locked natives, unlocked universally via Secret Lab.

> Ghidra confirms `gamemd.exe` contains no plain `"DTRUCK"` string — only the CSF
> lookup key `Name:DTRUCK` at 0x008299c4. All behavior is generic flag-driven via
> `[Demobomb] Suicide=yes` (WeaponType) + `Explodes=yes` (TechnoType) +
> `DeathWeapon=Demobomb` (RulesClass+TechnoType dual-read pattern, per-unit override).

---

## 1. `rulesmd.ini` — `[DTRUCK]` verbatim

```ini
[DTRUCK]
UIName=Name:DTRUCK
Image=TRUCKA
Name=Demolitions Truck
Prerequisite=NAWEAP,RADAR
Category=AFV
Primary=Demobomb
Secondary=none
Strength=150
Armor=light
Turret=no
TechLevel=10
Sight=5
Speed=5 ;changed on 11/29 from 6 to 5
Owner=Russians,Confederation,Africans,Arabs
RequiredHouses=Africans
AllowedToStartInMultiplayer=no
Cost=1500
Soylent=1500
Points=40
ROT=5
Crusher=no
SelfHealing=no
Crewed=no
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=DemoTruckSelect
VoiceMove=DemoTruckMove
VoiceAttack=DemoTruckAttackCommand
VoiceFeedback=
DieSound=DemoTruckDie
MoveSound=DemoTruckMoveStart
CreateSound=DemoTruckCreated
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
Weight=2
MovementZone=Normal
ThreatPosed=50	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
MaxDebris=2
DebrisTypes=TIRE
DebrisMaximums=4
Size=3
DeathWeapon=Demobomb
Explodes=yes
CanPassiveAquire=no ; Won't try to pick up own targets
CanRetaliate=no; Won't fire back when hit
Trainable=no
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:DTRUCK` | AbstractType | CSF lookup (verified in binary at 0x008299c4 — exists but only as the CSF table key, not as a code-side hardcoded reference). |
| `Image` | `TRUCKA` | AbstractType | **Art redirect to `[TRUCKA]`** — the civilian "Truck variant A" art block. DTRUCK reuses the civilian truck voxel + cameo + PrimaryFireFLH. See §2. |
| `Name` | `Demolitions Truck` | AbstractType | Dev fallback. |
| `Prerequisite` | `NAWEAP,RADAR` | TechnoType | Soviet War Factory + Radar — early-tier prereq. |
| `Category` | `AFV` | TechnoType | AFV classifier. |
| `Primary` | `Demobomb` | TechnoType | Suicide weapon (§3). Damage=300, Suicide=yes. |
| `Secondary` | `none` | TechnoType | Explicit `none` — no secondary weapon. The author wrote `=none` rather than omitting the key. |
| `Strength` | `150` | AbstractType | 150 HP — extremely fragile. Two GI hits will kill it. Combined with `Explodes=yes` + `DeathWeapon=Demobomb`, **a single early-killed DTRUCK self-destructs**, often damaging the killer. |
| `Armor` | `light` | TechnoType | Verses-slot 4. |
| `Turret` | `no` | UnitType | Body-mounted "weapon" — but Demobomb is Range=1 so target alignment is irrelevant. |
| `TechLevel` | `10` | TechnoType | High tech level — combined with `RequiredHouses=Africans` makes DTRUCK rare. |
| `Sight` | `5` | TechnoType | 5-cell reveal. |
| `Speed` | `5` | TechnoType | INI comment: "changed on 11/29 from 6 to 5" — author-history nerf. Slow enough that escort intercept can save targets, fast enough to threaten un-defended bases. |
| `Owner` | `Russians,Confederation,Africans,Arabs` | TechnoType | All 4 Soviet houses can OWN a DTRUCK (via capture, Secret Lab grant, or crate). |
| `RequiredHouses` | `Africans` | TechnoType (verified prior iter — 0x00843bb4) | **Only Libya (Africans country) can natively BUILD DTRUCK** from its War Factory. Other Soviet houses (Russians, Confederation, Arabs) unlock it via Secret Lab (per `[General] SecretUnits=TNKD,TTNK,DTRUCK`). |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Not preplaced. |
| `Cost` | `1500` | TechnoType | $1500 for a one-shot kamikaze unit. Expensive per shot, but the damage potential ($1500 truck destroys $3000+ of enemy buildings + units) makes it cost-positive when it reaches target. |
| `Soylent` | `1500` | TechnoType | 100% Grinder refund (Yuri can capture and recycle uneconomically). |
| `Points` | `40` | TechnoType | High score on kill — reflects high threat value. |
| `ROT` | `5` | TechnoType | Body rotation. |
| `Crusher` | `no` | TechnoType | **Cannot crush infantry.** The truck doesn't squish — keeps the "civilian-vehicle-with-bomb" feel. |
| `SelfHealing` | `no` | TechnoType | **Explicitly does NOT self-heal** — most vehicles default to no but DTRUCK's explicit `=no` emphasizes the design: damage is permanent, DTRUCK must reach target intact. |
| `Crewed` | `no` | TechnoType | No survivors (you don't survive a nuke explosion). |
| `Explosion` | `TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60` | TechnoType | Standard random death pool — but note: this is the **basic** death anim. The actual nuke detonation animation is `[DemobombWH] AnimList=DEMTEXP`, fired via the `DeathWeapon=Demobomb` mechanism. |
| `VoiceSelect` | `DemoTruckSelect` | TechnoType | 4 unique clips. |
| `VoiceMove` | `DemoTruckMove` | TechnoType | 3 unique clips. |
| `VoiceAttack` | `DemoTruckAttackCommand` | TechnoType | 5 unique clips. |
| `VoiceFeedback` | *(empty)* | TechnoType | No under-attack voice. |
| `DieSound` | `DemoTruckDie` | TechnoType | **Unique death sound `vdemdiea`**, FShift ±5, vol 90 (very loud) — this is the "nuclear boom" sound. Also referenced as the Demobomb weapon's `Report=` — the **same clip plays on suicide-fire AND on enemy-killed-detonate**. |
| `MoveSound` | `DemoTruckMoveStart` | TechnoType | 3 unique clips, low priority, vol 40. |
| `CreateSound` | `DemoTruckCreated` | TechnoType | **GLOBAL critical-priority voice** (`Type=global, Priority=critical, MinVolume=90, Volume=90`) — plays for ALL players when ANY player builds a DTRUCK. The "Demolitions Truck ready!" alert lets opponents know a threat is in play. Combined with the high cost + RequiredHouses, this audio tell is intentional. (Compare to most build-complete sounds which are local-only.) |
| `Locomotor` | `{4A582741-...}` | TechnoType | DriveLocomotionClass. |
| `Weight` | `2` | TechnoType | **Lighter than tanks** (Rhino=3.5) — truck is a civilian-tier chassis. |
| `MovementZone` | `Normal` | TechnoType | Standard land — cannot path through crushable terrain. |
| `ThreatPosed` | `50` | TechnoType | **High AI threat** (vs Rhino's 40, Mirage's 15). AI prioritizes destroying DTRUCKs over standard tanks. |
| `DamageParticleSystems` | `SparkSys,SmallGreySSys` | TechnoType | Smoke/spark damage emitters. |
| `MaxDebris / DebrisTypes / DebrisMaximums` | `2 / TIRE / 4` | TechnoType | Death debris (only 2 pieces — truck explosion mostly consumes the chassis). |
| `Size` | `3` | TechnoType | Standard tank-size transport slot. |
| `DeathWeapon` | `Demobomb` | TechnoType per-unit override (verified prior iter — 0x0083b11c) | **Per-unit override of the global death weapon.** When DTRUCK dies (any cause: enemy fire, friendly fire, suicide, self-explosion), the engine fires `[Demobomb]` weapon at the death location. Combined with `Explodes=yes` below, this triggers the nuke detonation. Same per-unit override pattern as FV's `DeathWeapon=CRNuke` (Ivan special case). |
| `Explodes` | `yes` | TechnoType (verified prior iter — 0x0083355c → 0x007122c5) | **Activates the `DeathWeapon`-fire-on-death path.** Without this, `DeathWeapon=` would be inert (the engine wouldn't invoke the death-weapon mechanism). Together with `DeathWeapon=Demobomb`, every dead DTRUCK triggers a Demobomb detonation. |
| `CanPassiveAquire` | `no` | TechnoType | INI comment: "Won't try to pick up own targets". DTRUCK won't auto-acquire enemies — must be manually directed. Reasonable: a suicide unit shouldn't randomly drive into the nearest target. |
| `CanRetaliate` | `no` | TechnoType (verified — 0x00843c40 → 0x0071448d) | INI comment: "Won't fire back when hit". Even when shot at, DTRUCK won't reflexively detonate on its attacker — must be player-ordered or destroyed for the bomb to go off. This prevents the AI from accidentally suicide-firing in response to a light scout shot. |
| `Trainable` | `no` | TechnoType | Cannot gain veterancy. One-shot unit — XP doesn't apply. |

### Notable absent keys
- No `ElitePrimary=` (Trainable=no → no veterancy → no elite swap).
- No `VeteranAbilities=` / `EliteAbilities=` lists.
- No `OpportunityFire=` (combined with `CanPassiveAquire=no`, `CanRetaliate=no` — strictly manual order).
- No `Bunkerable=no` (defaults yes — DTRUCK could enter Battle Fortress, though placing a self-detonating truck inside an OpenTopped vehicle has obvious risk).
- No `OmniCrusher` / `OmniCrushResistant`.
- No `Teleporter=`.
- No `ImmuneToPsionics` — **Yuri can mind-control DTRUCK**. Captured DTRUCK becomes a Yuri-controlled suicide unit, then explodes for Yuri's benefit. Single most punishing capture in the game.

---

## 2. `artmd.ini` — `[TRUCKA]` (referenced via `Image=TRUCKA`)

DTRUCK's `Image=TRUCKA` redirects to:

```ini
[TRUCKA]
Voxel=yes
Cameo=TRKAICON
Remapable=yes
PrimaryFireFLH=40,32,96
SecondaryFireFLH=-32,80,120
PBarrelLength=192
```

| Key | Value | Effect |
|-----|-------|--------|
| `Voxel` | `yes` | Voxel-rendered from `TRUCKA.VXL`. |
| `Cameo` | `TRKAICON` | **Reuses the civilian Truck A cameo** — no dedicated DTRUCK cameo. The player sees a generic truck icon in the sidebar, not a "nuclear truck" warning visual. |
| `Remapable` | `yes` | House-color remap. |
| `PrimaryFireFLH` | `40,32,96` | Firing offset: X=40 (mid-forward), Y=32 (slightly right), Z=96 (cabin height). For DTRUCK this is the "bomb-detonation origin" position. |
| `SecondaryFireFLH` | `-32,80,120` | Secondary FLH — DTRUCK has no Secondary weapon (`=none`), so this is unused. The asymmetric Y=+80 reflects the civilian-truck design (mid-back trailer position). |
| `PBarrelLength` | `192` | Primary barrel length (voxel-render parameter for projectile spawn-line). Irrelevant for DTRUCK since Demobomb has `Speed=35` and `Range=1` — the projectile barely travels. |

**Notable**: `[TRUCKA]` is shared with the **civilian Truck variant A** (no dedicated
DTRUCK art block exists). The civilian Truck is unarmed (no Primary weapon), so the
PrimaryFireFLH/SecondaryFireFLH/PBarrelLength entries exist only for DTRUCK's
inheritance. This is the YR art-reuse pattern: military version (`DTRUCK`) defined
in rulesmd with `Image=TRUCKA`, civilian version (probably `TRUCKA` rulesmd entry)
uses the same art directly.

---

## 3. Weapon — `[Demobomb]`

```ini
[Demobomb]
Damage=300 ;was 400, changed 11/30
ROF=80
Range=1
Projectile=InvisibleLow
Speed=35
RadLevel=100
Warhead=DemobombWH
Report=DemoTruckDie
Suicide=yes
```

| Key | Value | Effect |
|-----|-------|--------|
| `Damage` | `300` | INI comment: "was 400, changed 11/30" — author-history nerf. Per-hit damage at the warhead's full-damage centre. Combined with `CellSpread=8` AoE (see §4), the actual damage to a single cell varies with falloff. |
| `ROF` | `80` | 80-tick cooldown — irrelevant. DTRUCK is one-shot (Suicide=yes destroys the firer). |
| `Range` | `1` | **Range=1 cell** — must be adjacent to target. DTRUCK has to drive INTO the target before firing. |
| `Projectile` | `InvisibleLow` | Inviso projectile. |
| `Speed` | `35` | Projectile speed (irrelevant — Range=1 means instant). |
| `RadLevel` | `100` | **Verified WeaponType-scoped** (cheat sheet: WeaponTypeClass__ReadINI 0x00772xxx range). **Spawns a radiation field at impact with level 100.** Per the radiation system: the rad field persists for some duration, dealing periodic damage to units that walk through. Same mechanic Desolator uses. **Notable**: DTRUCK leaves a nuclear-fallout patch after detonation — both the explosion damage AND the lingering rad field. |
| `Warhead` | `DemobombWH` | High-explosive nuke warhead — see §4. |
| `Report` | `DemoTruckDie` | **Re-uses the DieSound** — same clip plays on suicide-fire AND on enemy-killed-explosion. The audio doesn't distinguish "DTRUCK kamikazed by player order" from "DTRUCK killed by enemy fire and exploded". |
| `Suicide` | `yes` | WeaponType (verified — 0x00843050 → WeaponTypeClass__ReadINI @ 0x0077228d; plus 2 runtime xrefs in `FUN_006f1550` at 006f1271/006f16dd — the engine's combat-resolve path). **The hardcoded "this weapon kills its firer" flag.** When DTRUCK fires this weapon, the engine destroys DTRUCK immediately as part of the fire-resolve. The death triggers `Explodes=yes` + `DeathWeapon=Demobomb` (chain), effectively firing the same weapon twice — but the engine has guards to prevent double-detonation (Ghidra-trace required for exact mechanism). |

---

## 4. Warhead — `[DemobombWH]`

```ini
[DemobombWH]
CellSpread=8
PercentAtMax=.1 ;was .25
Verses=100%,100%,100%,100%,50%,50%,80%,150%,10%,100%,100%
InfDeath=4
Sparky=no
Tiberium=yes
AnimList=DEMTEXP
```

| Key | Value | Effect |
|-----|-------|--------|
| `CellSpread` | `8` | **Massive 8-cell radius AoE.** Affects everything within 8 cells of the detonation. Comparable to nuke superweapon scale. |
| `PercentAtMax` | `.1` | INI comment: "was .25" — author-history nerf. **Damage at the edge of the AoE is 10% of centre damage.** Sharp falloff — full 300 damage at centre, ~30 at 8 cells out. Linear interpolation between (or some curve — exact formula TBD via Ghidra). |
| `Verses` | `100%,100%,100%,100%,50%,50%,80%,150%,10%,100%,100%` | Per-armor multipliers: |
| | | • Slots 1-4 (none/flak/plate/light): 100% — full damage to all infantry and light vehicles |
| | | • Slots 5-6 (medium/heavy): 50% — reduced damage to MBTs (still significant at 150 dmg/centre vs 400 Rhino HP) |
| | | • Slot 7 (wood): 80% — strong vs civilian buildings |
| | | • Slot 8 (steel): **150%** — **bonus damage vs steel buildings** (military structures take MORE damage than the base 300) |
| | | • Slot 9 (concrete): 10% — heavy fortifications resist nuke surprisingly well |
| | | • Slots 10-11 (special_1/2): 100% — drones and others |
| `InfDeath` | `4` | **Burn-death animation** (per InfDeath table: 4=burn, used by SAFlame, TerrorBombWH, DemobombWH). Infantry caught in the blast die with the burning corpse animation. |
| `Sparky` | `no` | Does NOT add spark VFX (the explosion is large enough). |
| `Tiberium` | `yes` | Tiberium-flagged warhead — relevant for the global `IsTiberium`-affected behaviors (anti-tiberium-cluster targeting, etc.). For YR with no real tiberium, this is mostly a flag-marker for the warhead category. |
| `AnimList` | `DEMTEXP` | **Demolition Truck Explosion animation.** Dedicated nuke-blast anim, not shared with any other warhead. Plays at the detonation centre. |

**Verses summary**: DTRUCK is a **steel-building killer first**, then a unit/civilian-building killer second. The 150% steel-armor multiplier makes it especially effective vs War Factories, Refineries, Battle Labs — the high-value military structures that anchor an opponent's economy.

---

## 5. Voices / sounds

```ini
[DemoTruckSelect]
Sounds= $vdemsea $vdemseb $vdemsec $vdemsed
Control= random
Volume=85

[DemoTruckMove]
Sounds= $vdemmoa $vdemmob $vdemmoc
Control= random
Volume=85

[DemoTruckAttackCommand]
Sounds= $vdemata $vdematb $vdematc $vdematd $vdemate
Control= random
Volume=85

[DemoTruckCreated]
Sounds=$vdemsea
Type=global
Priority=critical
MinVolume=90
Volume=90
```

```ini
[DemoTruckDie]
Sounds=vdemdiea
FShift=-5 5
Volume=90

[DemoTruckMoveStart]
Sounds= vdemstaa vdemstab vdemstac
Control= random predelay
Delay=0 400
Priority=low
FShift= -2 2
VShift=10
Volume=40
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=DemoTruckSelect` | 4 unique clips | Click-select |
| `VoiceMove=DemoTruckMove` | 3 unique clips | Move order |
| `VoiceAttack=DemoTruckAttackCommand` | 5 unique clips | Attack order |
| `VoiceFeedback=` *(empty)* | — | No under-attack |
| `DieSound=DemoTruckDie` | 1 clip `vdemdiea`, FShift ±5, vol 90 (loud) | Death — also re-used as Demobomb's `Report=` |
| `MoveSound=DemoTruckMoveStart` | 3 unique clips, predelay 0–400ms, low pri, vol 40 | Engine start |
| **`CreateSound=DemoTruckCreated`** | 1 clip `$vdemsea` (reuses VoiceSelect[0]), **`Type=global`, `Priority=critical`, `MinVolume=90, Volume=90`** | **Plays globally for ALL players when ANY player builds a DTRUCK.** The hardcoded "every enemy gets warned" alert. |

**Audio significance**: the `Type=global Priority=critical` `CreateSound` is unusual —
most CreateSound entries (typical for Tanya/Boris hero units) play only for the
owning player. DTRUCK's global priority audio is comparable to **superweapon-ready
alerts** — gamemd treats DTRUCK as a strategic-grade threat that all players deserve
to know about. Combined with the high cost and Secret-Lab gate, this CreateSound
shapes the metagame: opponents start scanning for DTRUCKs as soon as they hear the
alert.

---

## 6. Prerequisites / owners / availability

- **Prerequisite**: `NAWEAP,RADAR` — Soviet War Factory + Radar.
- **TechLevel** = `10`.
- **Owner**: 4 Soviet houses (CAN own).
- **`RequiredHouses=Africans`** — only Libya BUILDS natively.
- **CrateGoodie**: `no` — explicitly excluded from crate pool.
- **`AllowedToStartInMultiplayer=no`** — not preplaced.
- **Cost** = $1500.

### Acquisition paths (matches TNKD pattern)

| Path | Mechanism | Probability |
|------|-----------|-------------|
| **Native build (Libya only)** | `RequiredHouses=Africans` | Always available for Libyan players |
| **Secret Lab capture** | `[General] SecretUnits=TNKD,TTNK,DTRUCK` — captured CASLAB rolls 1 of 3 | 1-in-3 |
| **Capture from Libyan player** | Engineer/MC of an existing DTRUCK | Player-skill |
| **Mind-control** | Yuri steals DTRUCK | Yuri-only counter, devastating outcome |

### Strategic deployment

DTRUCK is the Soviet **base-killer** unit. Optimal use patterns:
1. **Cluster-rush**: 3-5 DTRUCKs simultaneously to overwhelm AA/defense — even if 3 are killed, 1-2 reaching target wipes out a base.
2. **IFV/transport delivery**: load DTRUCKs into Soviet/Allied transports for stealth deployment. (Risky — transport death + DTRUCK chain-detonation = self-damage.)
3. **Iron Curtain protect**: Soviet Iron Curtain on DTRUCK makes it invulnerable mid-approach — the only reliable "guaranteed delivery" tactic.

Counters:
- **Long-range artillery** (V3, Prism Tank, Apocalypse) — kill DTRUCK at safe distance.
- **AA-free zone surroundings** — DTRUCK is ground, so any anti-armor works; clear paths matter most.
- **Yuri mind-control** — flip the DTRUCK and detonate it on the Soviet player's own base.
- **Service Depot + spam** — DTRUCK can't be repaired (`SelfHealing=no`) — accumulated damage stays.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 DTRUCK-specific code in `gamemd.exe`

| Query | Result |
|-------|--------|
| `DTRUCK` | Only `"Name:DTRUCK"` at 0x008299c4 (CSF lookup) — no plain-ID code reference |

⇒ **No DTRUCK-specific code path.** All behavior is generic flag-driven.

### 7.2 Flag-scope verification (this iteration)

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `Suicide` | 0x00843050 | WeaponTypeClass__ReadINI @ 0x0077228d (INI parse) + `FUN_006f1550` @ 006f1271 + 006f16dd (runtime usage) | **WeaponType** |
| `CanRetaliate` | 0x00843c40 | TechnoTypeClass__ReadINI @ 0x0071448d | TechnoType |

Plus prior verifications (carried):
- `RequiredHouses` — TechnoType
- `Explodes` — TechnoType + OverlayType (dual scope)
- `DeathWeapon` — RulesClass global + TechnoType per-unit override (dual-read)
- `RadLevel` — WeaponType (cheat sheet 0x00772xxx range)
- `CanPassiveAquire` — TechnoType (DRON iter)
- `SecretUnits` — RulesClass global (TNKD iter)

### 7.3 Live behaviors driven by these flags

| Behavior | Driver | Notes |
|----------|--------|-------|
| Builds only for Libya (Africans country) | `RequiredHouses=Africans` | Build-availability resolver gates |
| Universal via Secret Lab | `[General] SecretUnits=TNKD,TTNK,DTRUCK` | 1-in-3 random grant on CASLAB capture |
| Suicide-fire on Primary | `[Demobomb] Suicide=yes` | Engine destroys firer as part of weapon-resolve |
| Detonates on any death | `Explodes=yes` + `DeathWeapon=Demobomb` | Same Demobomb explosion as suicide-fire |
| Cannot be retaliated-fire | `CanRetaliate=no` | Won't auto-detonate on enemy hit |
| No auto-target | `CanPassiveAquire=no` | Manual-order only |
| No HP regen | `SelfHealing=no` | One-life unit |
| No veterancy | `Trainable=no` | One-shot, no rank |
| Spawns rad field on detonate | `[Demobomb] RadLevel=100` | Lingering radiation patch |
| Sharp AoE falloff | `[DemobombWH] CellSpread=8, PercentAtMax=.1` | 100% damage at centre, 10% at edge |
| Bonus damage vs steel structures | `[DemobombWH] Verses[8]=150%` | Anti-military-building optimized |
| Global build-complete alert | `[DemoTruckCreated] Type=global, Priority=critical, MinVolume=90` | All players get audio warning |
| Death sound is the explosion | `DieSound=DemoTruckDie` + `[Demobomb] Report=DemoTruckDie` | Same clip plays on suicide-fire and on enemy-killed-explosion |

### 7.4 Behaviors NOT present

- No `OmniCrusher` / `OmniCrushResistant`.
- No `ImmuneToPsionics` — Yuri-MC vulnerability is intentional.
- No `Teleporter=`.
- No `Spawns=`.
- No `ImmuneToRadiation` — DTRUCK takes damage from its own rad field if it survived (only relevant in chain-detonation scenarios).
- No `Crusher=yes` — civilian-truck doesn't crush.

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `Tiberium=yes` (warhead flag) | YES (no tiberium in YR) | Likely dormant — flag is read but no live consumer triggers for it in YR. |
| (no `ImmuneToVeins`) | — | Not set. |
| (no `ZFudgeTunnel`) | — | Not set — DTRUCK doesn't have the legacy z-fudge keys. |

Notable: DTRUCK is one of the **cleanest TS-legacy-free** vehicle entries — no ImmuneToVeins, no ZFudgeTunnel. The unit was likely written cleanly for YR rather than ported from a TS legacy entry.

---

## 9. Veterancy

**`Trainable=no`** — DTRUCK cannot gain veterancy. No `VeteranAbilities=`, no
`EliteAbilities=`, no `ElitePrimary=` keys. Stuck at rookie permanently. Justified by
the one-shot kamikaze role — XP would be impossible to apply to a unit that dies on
first attack.

---

## 10. Cross-references

### Direct dependencies
- `[Demobomb]` — weapon (§3)
- `[InvisibleLow]` — projectile
- `[DemobombWH]` — warhead (§4)
- `[DEMTEXP]` (artmd) — dedicated explosion animation
- `[TRUCKA]` (artmd, via `Image=TRUCKA`) — art block (shared with civilian truck)
- `[NAWEAP]` / `[RADAR]` — prereqs
- `[DemoTruckSelect/Move/AttackCommand/Created/Die/MoveStart]` (soundmd) — voices + sounds
- `[General] SecretUnits=TNKD,TTNK,DTRUCK` (rulesmd line 265) — Secret Lab pool

### Conceptual companions
- **TNKD** ([`allied/TNKD.md`](../allied/TNKD.md)) — Allied SecretUnit. Same `RequiredHouses=Germans` + Secret Lab pattern.
- **TTNK (Tesla Tank)** ([`soviet/TTNK.md`](./TTNK.md) — TODO) — third SecretUnit member.
- **CASLAB (Tech Secret Lab)** ([`structures/CASLAB.md`](../structures/CASLAB.md) — TODO) — capturing this building rolls 1 of 3 SecretUnits.
- **DESO (Desolator)** ([`soviet/DESO.md`](./DESO.md)) — both use `RadLevel=` to spawn rad fields. DESO is the dedicated rad specialist; DTRUCK's rad is a side-effect.
- **NAMISL (Nuclear Missile Silo)** — Soviet superweapon. The Demobomb is essentially a "mobile mini-nuke" by comparison.

### Deep-RE docs
- **[NUKE_SUPERWEAPON_GHIDRA_REPORT.md](../../NUKE_SUPERWEAPON_GHIDRA_REPORT.md)** — covers the Soviet Nuclear Missile (NAMISL). Demobomb is a similar but smaller-scale detonation. Read for similarities in rad-field handling and AoE-falloff curves.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[DTRUCK]` rulesmd key explained | ✅ §1 |
| `Image=TRUCKA` redirect + artmd block expanded | ✅ §2 |
| **Suicide weapon + DeathWeapon + Explodes triple-mechanism** documented | ✅ §1 + §3 |
| Weapon + projectile + warhead with **150% steel-armor verse** highlighted | ✅ §3–§4 |
| **`RadLevel=100` lingering rad field noted** | ✅ §3 |
| All voices + **CreateSound=DemoTruckCreated global priority alert** documented | ✅ §5 |
| Prereqs / owners / `RequiredHouses=Africans` + SecretUnits acquisition paths | ✅ §6 |
| Strategic deployment + counters | ✅ §6 |
| Hardcoded behavior — Ghidra searches + 2 new flag-scope verifications (Suicide, CanRetaliate) | ✅ §7 |
| TS-legacy filter (notably clean — no ImmuneToVeins/ZFudgeTunnel) | ✅ §8 |
| Veterancy (Trainable=no → permanent rookie) | ✅ §9 |
| Cross-refs to TNKD/TTNK/CASLAB SecretUnits set + NUKE_SUPERWEAPON deep doc | ✅ §10 |

**Open follow-ups (parity-critical):**
- **Suicide-fire + Explodes chain mechanism**: when DTRUCK fires Demobomb, the Suicide=yes flag destroys the firer. The death triggers `Explodes=yes` + `DeathWeapon=Demobomb` — which would fire Demobomb a SECOND time. Does gamemd actually double-detonate, or does the engine guard against this? Ghidra-trace `FUN_006f1550` (the runtime Suicide handler) for the guard logic. Parity-critical — if the doubled detonation is real, it would explain why DTRUCKs sometimes seem to do "more damage than 300 × falloff" should produce.
- **`RadLevel=100` rad-field duration**: how long does the rad field persist after detonation? What's the per-tick damage rate? Compare to DESO's rad weapon for parity.
- **Yuri-MC + DTRUCK detonation**: when Yuri captures a DTRUCK, does the engine's "manual attack order" path differ from a Soviet-owned DTRUCK? Likely same path — but worth a quick check.
- **`CreateSound=DemoTruckCreated Type=global` semantics**: confirm the audio actually plays for ALL players, not just the owning player. Most CreateSound entries are local — DTRUCK's global pri-critical is unusual and parity-relevant.
- **DEMTEXP animation duration & visual**: the dedicated nuke-explosion anim — what's its frame count, sound layer, particle system attachments? Worth a brief artmd grep.
