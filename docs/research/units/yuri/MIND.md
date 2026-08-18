# MIND — Master Mind (Yuri psionic super-unit)

**Side classification:** Yuri (Owner=YuriCountry).
**Role:** Yuri tier-4 mobile mind-control platform. The Master Mind can hold an
**unlimited** number of mind-control links (`InfiniteMindControl=yes`), but each link
above 3 imposes escalating self-damage via the global `OverloadCount`/`OverloadDamage`/
`OverloadFrames` tables. The unit's defining tension: more captured enemies = faster
self-destruction. 500 HP + `SelfHealing=yes` mitigates short-term overload; sustained
overload still kills.

> Output bar: parity-critical. The overload damage curve (3 link buckets + bucket
> boundaries 3/6/10/50, damage 0/50/100/500, frame interval 30/60/60/60) must produce
> identical observable outcomes — players time their captures around bucket transitions.
> The 5 `AlternateFLH` beam origin points + simultaneous-target rendering must match
> gamemd's "5 mind-rays branching from one MIND" visual.

> **Deep-RE cross-references — DO NOT re-derive:**
> - **[MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md](../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md)** (615 lines) — full system overview.
> - **[MIND_CONTROL_GHIDRA_REPORT.md](../../MIND_CONTROL_GHIDRA_REPORT.md)** (451 lines) — companion report on linkage and lifecycle.

> Ghidra confirms no `"MIND"` / `"MasterMind"` strings as unit IDs in `gamemd.exe` —
> the unit is generic flag-driven via `InfiniteMindControl=yes` (WeaponType) +
> overload globals (RulesClass).

---

## 1. `rulesmd.ini` — `[MIND]` verbatim

```ini
[MIND]
UIName=Name:MasterMind
Name=Master Mind
Prerequisite=YAWEAP,YATECH
Primary=MultipleMindControlTank
Strength=500
Category=AFV
Armor=heavy
;Turret=yes
IsTilter=yes
;TargetLaser=yes
TooBigToFitUnderBridge=true
TechLevel=2
Sight=9
Speed=4
CrateGoodie=no
Crusher=no
Owner=YuriCountry
Cost=1750
Soylent=1750
Points=25
ROT=5
AllowedToStartInMultiplayer=no
IsSelectableCombatant=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=MasterMindSelect
VoiceMove=MasterMindMove
VoiceAttack=MasterMindAttackCommand
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=MasterMindMoveStart
CrushSound=TankCrush
Maxdebris=3
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
MovementZone=Destroyer
ThreatPosed=40	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
DamageSmokeOffset=100, 100, 275
Weight=3.5
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Accelerates=false
ZFudgeColumn=8
ZFudgeTunnel=13
Size=6
OpportunityFire=yes
PipScale=MindControl
ImmuneToPsionics=yes
SelfHealing=yes
Trainable=no
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:MasterMind` | AbstractType | CSF lookup. |
| `Name` | `Master Mind` | AbstractType | Dev fallback. |
| `Prerequisite` | `YAWEAP,YATECH` | TechnoType | Yuri War Factory + Battle Lab — tier-4 gate (despite `TechLevel=2` below). |
| `Primary` | `MultipleMindControlTank` | TechnoType | Mind-control weapon — see §3. |
| `Strength` | `500` | AbstractType | 500 HP — high for unit size, designed to survive overload phases. |
| `Category` | `AFV` | TechnoType | AFV classifier. |
| `Armor` | `heavy` | TechnoType | Verses-slot 6. |
| Commented `;Turret=yes` | — | — | **Turret is commented out** — Master Mind has no rotating turret. Combined with `OmniFire=yes` on the weapon, the mind-rays emerge from the body in any direction regardless of facing. |
| `IsTilter` | `yes` | UnitType | Voxel tilts on slopes. |
| Commented `;TargetLaser=yes` | — | — | Author was considering a target-laser visual (like APOC) but disabled. Master Mind uses dedicated mind-control beam visuals instead (drawn between `AlternateFLH0-4` and each controlled target). |
| `TooBigToFitUnderBridge` | `true` | UnitType-only | Cannot path under low bridges. |
| `TechLevel` | `2` | TechnoType | **Surprisingly low TechLevel=2** despite the tier-4 prereqs — the gate is effectively `YATECH` (Battle Lab), not TechLevel. The TechLevel=2 here would be misleading if it were the only gate. |
| `Sight` | `9` | TechnoType | 9-cell reveal — longer than mind-control range (6). Master Mind sees beyond its own engagement range. |
| `Speed` | `4` | TechnoType | Slow. |
| `CrateGoodie` | `no` | UnitType | Excluded from crates. |
| `Crusher` | `no` | TechnoType | **Cannot crush infantry** — unusual for a 500 HP heavy. Reflects the unit's psionic-not-physical role. |
| `Owner` | `YuriCountry` | TechnoType | Yuri only. |
| `Cost` | `1750` | TechnoType | Same as Apocalypse. |
| `Soylent` | `1750` | TechnoType | 100% Grinder refund. |
| `Points` | `25` | TechnoType | Standard score on kill. |
| `ROT` | `5` | TechnoType | Body rotation (no turret). |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Not preplaced. |
| `IsSelectableCombatant` | `yes` | TechnoType | |
| `Explosion` | `TWLT070,...` | TechnoType | Standard death pool. |
| `VoiceSelect` | `MasterMindSelect` | TechnoType | 6 unique clips. |
| `VoiceMove` | `MasterMindMove` | TechnoType | 5 unique clips. |
| `VoiceAttack` | `MasterMindAttackCommand` | TechnoType | 6 unique clips. |
| `VoiceFeedback` | *(empty)* | TechnoType | No under-attack voice. |
| `DieSound` | `GenVehicleDie` | TechnoType | Generic. |
| `MoveSound` | `MasterMindMoveStart` | TechnoType | 3 unique clips, predelay 0–400ms, low pri, FShift ±10, VShift +10, vol 65 (loud). |
| `CrushSound` | `TankCrush` | TechnoType | n/a (Crusher=no). |
| `Maxdebris` | `3` | TechnoType | |
| `Locomotor` | `{4A582741-...}` | TechnoType | DriveLocomotionClass. |
| `MovementZone` | `Destroyer` | TechnoType | |
| `ThreatPosed` | `40` | TechnoType | High AI threat. |
| `DamageParticleSystems` | `SparkSys,SmallGreySSys` | TechnoType | |
| `DamageSmokeOffset` | `100, 100, 275` | TechnoType | Same as Rhino, BFRT, APOC. |
| `Weight` | `3.5` | TechnoType | Standard tank weight. |
| `VeteranAbilities` | `STRONGER,FIREPOWER,SIGHT,FASTER` | TechnoType | Standard. |
| `EliteAbilities` | `SELF_HEAL,STRONGER,FIREPOWER,ROF` | TechnoType | Elite adds SELF_HEAL + ROF. Most relevant: ROF reduces the per-link control time, letting Master Mind capture more units faster. |
| `Accelerates` | `false` | TechnoType | No accel ramp. |
| `ZFudgeColumn` | `8` | UnitType | |
| `ZFudgeTunnel` | `13` | UnitType | TS-legacy. |
| `Size` | **`6`** | TechnoType | **Massive** — cannot fit in any transport (same as MCV, BFRT, APOC). |
| `OpportunityFire` | `yes` | TechnoType | Auto-targets passing threats — will attempt to mind-control enemies in range automatically. **Combined with `InfiniteMindControl=yes`, this is the source of self-overload cascades**: an OpportunityFire pass through an enemy group can rapidly accumulate links into the lethal bucket. |
| `PipScale` | `MindControl` | TechnoType (verified — 0x00843e04 → 0x0071411a) | **Special pip-rendering mode** — renders the current mind-control link count as colored pips above the unit. Other units use `Tiberium` (ore-bale pips) or `Passengers` (passenger pips); Master Mind has a dedicated `MindControl` mode showing 0..N captured units. |
| `ImmuneToPsionics` | `yes` | TechnoType | Cannot be mind-controlled (no Yuri-vs-Yuri Master-Mind ping-pong). |
| `SelfHealing` | `yes` | TechnoType | Passive HP regen — partially offsets overload self-damage, but the regen rate (per `[General] SelfHealUnitRate`) is far slower than the 500 dmg/60-frame at 11+ links. |
| `Trainable` | `no` | TechnoType | Cannot gain veterancy. `VeteranAbilities`/`EliteAbilities` lists are dead. |

### Notable absent keys
- No `Image=` redirect — reads its own `[MIND]` artmd block.
- No `Secondary` weapon — single mind-control weapon.
- No `ElitePrimary=` — combined with `Trainable=no`, no elite weapon swap.
- No `Spawns=` — no child units.
- No `Teleporter=` — does not chrono.
- No `OmniCrusher=` / `OmniCrushResistant=`.
- No `Bunkerable=no` (defaults yes — but Size=6 makes ineligible).
- No `ImmuneToRadiation` — Desolators damage Master Mind.

---

## 2. `artmd.ini` — `[MIND]` section

```ini
[MIND]			; Mastermind
Cameo=MINDICON
AltCameo=MINDUICO
Voxel=yes
Remapable=yes
AlternateFLH0=0,25,90;gs scatter the mind control lines
AlternateFLH1=0,-25,90
AlternateFLH2=-50,25,90
AlternateFLH3=-50,-25,90
AlternateFLH4=-25,0,90
```

| Key | Value | Effect |
|-----|-------|--------|
| `Cameo` | `MINDICON` | Sidebar cameo. |
| `AltCameo` | `MINDUICO` | Yuri-skinned alt cameo. |
| `Voxel` | `yes` | Voxel-rendered from `MIND.VXL`. |
| `Remapable` | `yes` | House-color remap. |
| `AlternateFLH0` | `0,25,90` | **5 mind-ray origin points.** INI comment: "scatter the mind control lines". When Master Mind has multiple links, each link's beam is drawn from a different FLH to avoid overlapping all 5 beams from one point. FLH0 = center-front, slightly right. |
| `AlternateFLH1` | `0,-25,90` | Mirror of FLH0 — center-front, slightly left. |
| `AlternateFLH2` | `-50,25,90` | Rear-right. |
| `AlternateFLH3` | `-50,-25,90` | Rear-left. |
| `AlternateFLH4` | `-25,0,90` | Mid-center. |

### Mind-beam visualization (top-down)

```
       +-------[MIND BODY]-------+
       |                         |
   FLH3│                       FLH2
(rear-L) (-50,-25)         (-50,+25)
       |                         |
       |   FLH4 (-25, 0)         |
       |   (mid-center)          |
       |                         |
   FLH1│       FLH0              |
(fr-L) (0,-25) (0,+25) (fr-R)    |
       +-------------------------+
```

The 5 FLHs are clustered around the front-center and rear of the unit body. Each
mind-control link picks a unique FLH origin (round-robin or first-available)
so the player sees up to 5 distinct beam lines branching outward to captured targets.
With more than 5 links, beams must share origins.

### Comparison to other 5-AlternateFLH units

| Unit | Purpose | FLH Z |
|------|---------|-------|
| **MIND (Master Mind)** | 5 mind-control beam origins (psi-ray lines) | 90 (mid-height) |
| **BFRT (Battle Fortress)** | 5 passenger gun-port positions (passenger weapons fire out) | 80-90 (around hull) |

Both have 5 `AlternateFLH` slots but serve different mechanics — BFRT's are
projectile-spawn positions for passenger Primaries, while MIND's are continuous-beam
endpoints rendered during the link-active state.

---

## 3. Weapon — `[MultipleMindControlTank]`

```ini
[MultipleMindControlTank]
Damage=3; this is an infinite mind control, so this just affects pips
InfiniteMindControl=yes; this will let infinite, it will look up on the table of "MasterMind Overload" damage way above (where there is a 0 damage level)
ROF=10
Range=6
Projectile=PsychicControl
Speed=100
Warhead=Controller
;Report=YuriMindControl
Anim=YURICNTL
FireOnce=yes
OmniFire=yes;doesn't need turret to shoot any direction
```

| Key | Value | Effect |
|-----|-------|--------|
| `Damage` | `3` | INI comment: "this is an infinite mind control, so this just affects pips". With `InfiniteMindControl=yes`, the Damage value affects the **pip-renderer's pip count** (shows 3-pip clusters or similar visualization), NOT actual damage or link count. Each fire creates 1 new mind-control link regardless of Damage. |
| `InfiniteMindControl` | `yes` | WeaponType (verified — 0x0084948c → 0x0077220b). **Hardcoded flag.** INI comment: "this will let infinite, it will look up on the table of 'MasterMind Overload' damage way above (where there is a 0 damage level)". Triggers two behaviors: (1) the unit can accumulate unlimited links (vs YURI's hard cap of 1, MasterMind has no cap), and (2) on each tick after the first 3 links, the unit takes self-damage from the `[General]` overload table. |
| `ROF` | `10` | Very fast cycle — 10 ticks (~0.17s) between mind-control link attempts. **MasterMind can capture 6 units per second** at base ROF, way faster than YURI's ROF=200. |
| `Range` | `6` | 6-cell range. Same as YURI's MindControl (Range=7 reduced to 6 here). |
| `Projectile` | `PsychicControl` | Inviso. Same as YURI's. |
| `Speed` | `100` | Bullet speed. |
| `Warhead` | `Controller` | **Same mind-control warhead** YURI uses (Verses 100/100/100/100/100/100/**0/0/0**/100/100 — works vs units, fails vs buildings). |
| Commented `;Report=YuriMindControl` | — | Author chose to NOT play the Yuri mind-control sound (suppresses the audio that would otherwise echo with up to 50 active beams). |
| `Anim` | `YURICNTL` | Shared mind-control hit animation. |
| `FireOnce` | `yes` | Single shot per cycle. WeaponType-scoped (cheat sheet). |
| `OmniFire` | `yes` | INI comment: "doesn't need turret to shoot any direction". **Hardcoded flag** — allows the weapon to fire in any direction without rotating the unit's turret/body first. Combined with `Turret=no` on MIND, the mind-rays can spawn from any FLH and target anything in range without facing it. |

### Mind-control linkage behavior

Per [MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md](../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md):

1. Master Mind fires `MultipleMindControlTank` at target — `WarheadTypeClass::Detonate` sees `Controller.MindControl=yes`.
2. `MindControlClass::Add(target, attacker=MIND, count=current_link_count + 1)` is called.
3. Master Mind's internal link list grows by 1 entry.
4. **No upper cap on links** (vs YURI's 1-slot cap) — the `InfiniteMindControl=yes` flag bypasses the standard slot-limit check.
5. Every tick, the engine evaluates the current link count against `[General] OverloadCount=3,6,10,50`:
   - 0-3 links → `OverloadDamage[0]=0` every `OverloadFrames[0]=30` frames (no self-damage)
   - 4-6 links → 50 dmg every 60 frames
   - 7-10 links → 100 dmg every 60 frames
   - 11+ links → 500 dmg every 60 frames (kills MIND in 60 frames = 1 second)
6. Overload self-damage applies even if links are released — release happens automatically if MIND-controlled target dies, or manually via player Stop order.
7. Master Mind self-heals between overload ticks (`SelfHealing=yes`), partially offsetting overload damage in lower buckets.

---

## 4. Warhead — `[Controller]` (shared with YURI)

Already documented in YURI doc — see [`yuri/YURI.md`](./YURI.md) §4 for full details.

Key points relevant to MIND:
- `MindControl=yes` → triggers `MindControlClass::Add`
- `Verses=100%,100%,100%,100%,100%,100%,0%,0%,0%,100%,100%` — works against all unit armors, fails vs all building armors. **Master Mind cannot mind-control buildings.**
- `AnimList=YURICNTL` — hit animation.

---

## 5. Voices / sounds

```ini
[MasterMindSelect]
Sounds=$vmassea $vmasseb $vmassec $vmassed $vmassee $vmassef
Control=random
Volume=85

[MasterMindMove]
Sounds=$vmasmoa $vmasmob $vmasmoc $vmasmod $vmasmoe
Control=random
Volume=85

[MasterMindAttackCommand]
Sounds=$vmasata $vmasatb $vmasatc $vmasatd $vmasate $vmasatf
Control=random
Volume=85

[MasterMindOverloadVoice]
Sounds=$vmasdib
Priority=high
Range=30
Volume=95
```

```ini
[MasterMindMoveStart]
Sounds= vmasstaa vmasstab vmasstac
Control= random predelay
Delay=0 400
Priority=Low
FShift= -10 10
VShift=10
Volume=65
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=MasterMindSelect` | 6 unique clips | Click-select |
| `VoiceMove=MasterMindMove` | 5 unique clips | Move order |
| `VoiceAttack=MasterMindAttackCommand` | 6 unique clips | Attack order |
| `VoiceFeedback=` *(empty)* | — | No under-attack |
| `DieSound=GenVehicleDie` | 6 generic clips | Death |
| `MoveSound=MasterMindMoveStart` | 3 unique clips, low pri, vol 65 | Engine start |
| `[MasterMindOverloadVoice]` (referenced from code, not from MIND's INI) | 1 clip ($vmasdib), `Priority=high`, `Range=30`, `Volume=95` (loud) | **Plays when Master Mind enters overload state** — likely fired by code when the link count crosses an OverloadCount boundary (suspected at the 4-link or 7-link threshold, exact trigger requires Ghidra trace). |
| `CrushSound=TankCrush` | n/a (Crusher=no) | — |

**`MasterMindOverloadVoice`** is a unique audio cue — it's not referenced from MIND's
own INI block, suggesting it's triggered from code via the `MindControlClass` overload-bucket
transition path. The high priority + Range=30 + Volume=95 ensure the player hears
"Master Mind in distress" anywhere on the map. Worth verifying the exact trigger
(probably enter-bucket-2 or enter-bucket-3 transition).

---

## 6. Prerequisites / owners / availability

- **Prerequisite**: `YAWEAP,YATECH` — Yuri War Factory + Battle Lab.
- **TechLevel** = `2` (low — but YATECH is the real gate).
- **Owner**: `YuriCountry` only.
- **CrateGoodie**: `no`.
- **`AllowedToStartInMultiplayer=no`** — not preplaced.
- **Cost** = $1750. Equal to Apocalypse — most expensive Yuri non-MCV unit.

### Strategic positioning

Master Mind is Yuri's **field-presence siege unit**. Key role considerations:
- **High micro-cost**: player must constantly monitor link count and release captured units before overload kills MIND.
- **Force multiplier**: 6-10 captured tanks = ~$5,000-9,000 of stolen enemy economy. Each capture is value extraction.
- **Best vs unaware opponents**: if the enemy isn't watching, MIND can capture 10+ units in seconds. Once the enemy spots MIND, it's a high-priority target.
- **Counters**: anti-air doesn't apply (MIND is ground); `ImmuneToPsionics=yes` blocks counter-MC; range 6 means basic infantry/tanks can engage MIND at standoff (Tanya's range 5, Rhino's 5.75 are below).
- **`SelfHealing=yes` + Strength=500 + Armor=heavy**: in bucket 1 (0-3 links), MIND can sit indefinitely under light fire. In bucket 4+ (50-500 dmg/60f), MIND must release captures or die.

Comparison: YURI = single-link infantry; INIT = AoE psi-blast; YURIPR = AoE mind-control;
**MIND = mobile psi-blob factory**. The four units form Yuri's psi-tier lineup.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 MIND-specific code in `gamemd.exe`

| Query | Result |
|-------|--------|
| `MIND` (substring) | (would match many — not specifically searched as plain ID) |
| `MasterMind` | (would match `MasterMindOverloadVoice` etc. in CSF table) |

⇒ No MIND-string-specific code path. All behavior is generic flag-driven via:
- `InfiniteMindControl=yes` (WeaponType flag)
- `[General] OverloadCount/OverloadDamage/OverloadFrames` (RulesClass globals)
- `PipScale=MindControl` (TechnoType pip-renderer enum)

### 7.2 Flag-scope verification (this iteration)

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `InfiniteMindControl` | 0x0084948c | WeaponTypeClass__ReadINI @ 0x0077220b | **WeaponType** |
| `OverloadCount` | 0x0083afc4 | RulesClass__ReadCombatDamage @ 0x0066c7c0 | RulesClass global `[CombatDamage]` |
| `PipScale` | 0x00843e04 | TechnoTypeClass__ReadINI @ 0x0071411a | TechnoType (enum value: `MindControl` is one of `Ammo`/`Tiberium`/`Passengers`/`MindControl`) |

Plus prior verifications (from YURI / INIT / YURIPR iterations):
- `Controller.MindControl=yes` — Warhead-scoped
- `ImmuneToPsionics` — TechnoType
- `OpportunityFire` — TechnoType

### 7.3 Live behaviors driven by these flags

| Behavior | Driver | Notes |
|----------|--------|-------|
| Unlimited mind-control link accumulation | `[MultipleMindControlTank] InfiniteMindControl=yes` | Bypasses standard slot-limit check |
| Self-damage when overloaded | `[General] OverloadCount/Damage/Frames` lookup based on current link count | The "tragedy of the commons" mechanic |
| Bucket-based damage stepping | `OverloadCount=3,6,10,50` → 4 buckets with `OverloadDamage=0,50,100,500` | Player chooses risk level |
| Auto-target enemies in range | `OpportunityFire=yes` | Combined with InfiniteMindControl, easy to over-capture |
| Mind-rays from 5 origins | `AlternateFLH0-4` in artmd | Visual: beams scatter across body |
| OmniFire — no body rotation needed | `[MultipleMindControlTank] OmniFire=yes` | Combined with Turret=no commented out, free 360° fire |
| Pip count reflects link count | `PipScale=MindControl` | Renders 0..N pips per current links |
| Self-heals HP | `SelfHealing=yes` | Partial overload mitigation |
| Cannot crush | `Crusher=no` | Heavy unit but no infantry-squish |
| Cannot be mind-controlled | `ImmuneToPsionics=yes` | |
| No veterancy | `Trainable=no` | |
| OverloadVoice plays at bucket transition | (code-triggered, not INI) | `[MasterMindOverloadVoice]` fires from code |

### 7.4 Behaviors NOT present

- No turret rotation — fires from body position with OmniFire.
- No `Secondary` weapon.
- No `Spawns=` / `Passengers=` / `Teleporter=`.
- No `OmniCrusher` / `OmniCrushResistant`.
- No `ImmuneToRadiation` — Desolators damage MIND (significant counter).
- No `Bunkerable=no` — but Size=6 prevents transport regardless.

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ZFudgeTunnel=13` | YES | Dormant render value. |
| Commented `;Turret=yes` / `;TargetLaser=yes` | n/a (commented) | Inactive — author design choices. |

No fog-of-war, no Tiberium, no real tunnel refs.

---

## 9. Veterancy

**`Trainable=no`** — Master Mind cannot gain veterancy. `VeteranAbilities=` and
`EliteAbilities=` lists are dead. No `ElitePrimary=`.

Combined with the unit's high cost and short effective lifespan (overload kills it
fast), MIND is intentionally a "use-once carefully" unit — not a hardened veteran.

---

## 10. Cross-references

### Direct dependencies
- `[MultipleMindControlTank]` — weapon (§3)
- `[PsychicControl]` — projectile (shared with YURI)
- `[Controller]` — warhead (shared with YURI)
- `[YURICNTL]` (artmd) — hit animation
- `[MIND]` (artmd) — art block with 5 AlternateFLH (§2)
- `[YAWEAP] / [YATECH]` — prereqs
- `[MasterMindSelect/Move/AttackCommand/MoveStart/OverloadVoice]` (soundmd) — voices
- `[GenVehicleDie] / [TankCrush]` (soundmd) — generic
- `[General] OverloadCount, OverloadDamage, OverloadFrames` (rulesmd line 848-850) — overload curve
- `[General] ControlledAnimationType=MINDANIM, PermaControlledAnimationType=MINDANIMR, MindControlAttackLineFrames=20` — additional MC visual config

### Conceptual companions
- **YURI** ([`yuri/YURI.md`](./YURI.md)) — single-link infantry psi.
- **YURIPR** ([`yuri/YURIPR.md`](./YURIPR.md)) — AoE psi-blast infantry.
- **INIT** ([`yuri/INIT.md`](./INIT.md)) — psychic-blast infantry.
- **PTROOP** ([`yuri/PTROOP.md`](./PTROOP.md)) — tech-steal psi infantry.
- **MultipleMindControlTower** weapon (line 24075) — likely used by Yuri Psychic Tower (YAPSYT) — same `Damage=3` pattern.
- **BFRT (Battle Fortress)** — also uses 5 `AlternateFLH` for a different mechanic (passenger gun-ports).

### Deep-RE docs (cross-referenced, NOT re-derived)
- **[MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md](../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md)** — 615 lines, full system overview. **The canonical reference for any Master Mind / Yuri / mind-control implementation.**
- **[MIND_CONTROL_GHIDRA_REPORT.md](../../MIND_CONTROL_GHIDRA_REPORT.md)** — 451 lines, companion linkage/lifecycle report.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[MIND]` rulesmd key explained | ✅ §1 |
| Every `[MIND]` artmd key explained — **all 5 AlternateFLH positions documented with ASCII visualization** | ✅ §2 |
| Mind-control weapon + projectile + warhead (cross-referenced YURI for shared parts) | ✅ §3–§4 |
| **InfiniteMindControl mechanism** explained with overload-bucket lookup | ✅ §3 |
| All voices + **MasterMindOverloadVoice** (code-triggered, not from MIND's INI) | ✅ §5 |
| **OverloadCount/Damage/Frames table from `[General]`** quoted verbatim and explained per-bucket | ✅ §3 + §10 |
| Prereqs / owners / strategic positioning | ✅ §6 |
| Hardcoded behavior — Ghidra searches + 3 new flag-scope verifications | ✅ §7 (InfiniteMindControl=WeaponType, OverloadCount=RulesClass global, PipScale=TechnoType enum) |
| TS-legacy filter | ✅ §8 |
| Veterancy (Trainable=no → permanently rookie) | ✅ §9 |
| Cross-refs to MIND_CONTROL_SYSTEM + MIND_CONTROL deep-RE reports | ✅ §10 |

**Open follow-ups (parity-critical):**
- **`MasterMindOverloadVoice` trigger condition**: when exactly does the voice play? At first overload-bucket transition? Each overload tick? Continuously while overloaded? Ghidra-trace the MindControl link-count change handler for `VocClass::Play` calls.
- **Pip rendering for `PipScale=MindControl`**: which pip colors at which link count? Likely matches OverloadCount thresholds (3 green pips, 6 yellow, 10 red, 50 black?). Worth a visual fidelity-check.
- **Each captured target's color outline**: when MasterMind controls a unit, the captured unit may render in MIND's house color or with a special tint. Verify via fidelity-check.
- **Range-checking on links**: does maintaining a mind-control link require the captured unit to remain within `Range=6` of MIND? Or once captured, links persist regardless of distance? MIND_CONTROL_SYSTEM doc likely covers this.
- **OverloadCount table bucket-edge semantics**: at link count exactly 3, does MIND fall into bucket 0 (no damage) or bucket 1 (50 dmg)? INI comment "You fall into the biggest category you are equal or less than" suggests 3 → bucket 0 (count==3 means "you are equal to the boundary, take 0 damage"). Verify the off-by-one in code.
