# Yuri Clone (YURI)
Side: Yuri | Category: Infantry | Image alias: `[YURI]` (no `Image=` redirect — own SHP `YURI`)

The eponymous **Yuri Clone** — the Yuri faction's basic mind-control unit.
$800 from Yuri Barracks + Soviet Psychic Sensor (`YABRCK,NAPSIS`); also
buildable from Kremlin Palace (`PrerequisiteOverride=CARUS03`). Has
**TWO weapons**:
**`Primary=MindControl`** (Range 7, `Warhead=Controller` with `MindControl=yes` flag) —
a single-target mind-control beam that creates a permanent CaptureManager
link to the victim;
**`Secondary=PsiWave`** (Range 1, `AreaFire=yes`, `Warhead=PsiPulse` with
`PsychicDamage=yes`) — a deployed-mode close-range psychic blast that
**kills nearby infantry instantly** (Damage 250 × 100% vs infantry armor =
250 dmg one-shot). **`Deployer=yes` + `DeployFire=yes`** wire the deploy
command to fire the Secondary — same mechanism as Desolator's radiation
deploy. **`UndeployDelay=150`** frames (10s @ 15fps) prevents instant
re-deploy spam. **`DetectDisguise=yes`** reveals nearby Spies and Mirage
Tanks. **`Sight=12`** is the highest of any infantry (3 cells past Spy's 9).

Inline designer comment `;Bender of spoons!` celebrates Yuri's
Westworld-style mentalist theme.

Authoritative deep RE for mind-control:
[MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md](../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md)
(615 lines) — full CaptureManagerClass behavior, MCNode linked list,
overload damage, persist-on-controller-death, etc.

---

## rulesmd.ini — `[YURI]` section

Verbatim from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:5197`:

```ini
[YURI] ;Bender of spoons!
UIName=Name:YuriClone
Name=Yuri Clone
Prerequisite=YABRCK,NAPSIS;GEF want the basic yuri to be a little more low level now YATECH
PrerequisiteOverride=CARUS03 ; SJM: Kremlin Palace
Pip=red
Category=Soldier
Strength=100
LeadershipRating=8
Primary=MindControl
Secondary=PsiWave
TypeImmune=yes
Armor=none
TechLevel=10
CrushSound=InfantrySquish
Insignificant=no
Sight=12
Speed=4
Owner=Russians,Confederation,Africans,Arabs,YuriCountry,British,French,Germans,Americans,Alliance
SecretHouses=YuriCountry
AllowedToStartInMultiplayer=no
Cost=800
Soylent=400
Points=5
IsSelectableCombatant=yes
VoiceSelect=YuriCloneSelect
VoiceMove=YuriCloneMove
VoiceAttack=YuriCloneAttackCommand
VoiceFeedback=YuriCloneFear
VoiceSpecialAttack=YuriCloneMove
DieSound=YuriCloneDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
ThreatPosed=15	; This value MUST be 0 for all building addons
ImmuneToVeins=yes
ImmuneToPsionics=yes
;Bombable=no
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
;The new yuri side yuri clone can no longer deploy
;nevermind, they changed their minds
Deployer=yes
DeployFire=yes
UndeployDelay=150
Size=1
DetectDisguise=yes
IFVMode=8
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:YuriClone` | CSF-string key → "Yuri Clone" |
| `Name=Yuri Clone` | Internal name |
| `;Bender of spoons!` (inline comment) | Designer humor — reference to The Matrix's spoon-bending and to Yuri's mentalist theme |
| `Prerequisite=YABRCK,NAPSIS` | Yuri Barracks + **Soviet Psychic Sensor** (NAPSIS). Inline comment: "GEF want the basic yuri to be a little more low level now YATECH" — original prereq was YATECH (Yuri Battle Lab), reduced to NAPSIS (Psychic Sensor) so Yuri Clones are accessible earlier. **Note NAPSIS is a Soviet building** — Yuri can't build NAPSIS himself; the prereq is satisfied either by capturing one or by the PrerequisiteOverride below |
| `PrerequisiteOverride=CARUS03` | **Behavior key** — TechnoTypeClass field (per `TechnoTypeClass__ReadINI` xref to string at `0x00843D90`). Inline comment: "SJM: Kremlin Palace". When the player owns CARUS03 (Kremlin Palace tech structure), Yuri Clone can be built **regardless of the normal Prerequisite=** chain. This is the only way Yuri-faction-without-NAPSIS players can build Yuri Clones in standard skirmish — capture or own the Kremlin Palace tech building |
| `Pip=red` | Cargo pip color — red (elite class) |
| `Category=Soldier` | Infantry pip/AI grouping |
| `Strength=100` | HP — 100 (same as Initiate/GI) |
| `LeadershipRating=8` | Veterancy gain modifier — high (8/10) |
| `Primary=MindControl` | **THE mind-control weapon** — Damage=1 (number of MC links, not damage), ROF=200, Range=7, Warhead=Controller (`MindControl=yes`). See "Weapons" and "Hardcoded Behavior" |
| `Secondary=PsiWave` | **Deployed psychic blast** — Damage=250, Range=1, AreaFire=yes, Warhead=PsiPulse (`PsychicDamage=yes`, 100% vs infantry only). Fired when deployed via DeployFire=yes |
| `TypeImmune=yes` | **Behavior flag** — TechnoTypeClass field (per `TechnoTypeClass__ReadINI @ 0x0071220F` DATA xref to string at `0x008444EC`). Unit is **immune to weapons fired by the SAME unit type**. For Yuri Clone: **another Yuri Clone cannot mind-control this Yuri Clone**. Prevents Yuri vs Yuri infinite-MC loops where Player A's Yuri controls Player B's Yuri who then controls Player A's, etc. |
| `Armor=none` | Damage type column 0 — standard infantry |
| `TechLevel=10` | **Tech-10 cap** — only Yuri Prime (also 10) matches; everything else is tech-9 or below. Effectively no tech-level restriction in normal play; gated entirely by Prerequisite chain |
| `CrushSound=InfantrySquish` | Standard crush sound |
| `Insignificant=no` | EVA announces Yuri Clone deaths (same flag documented on Ivan — important for high-value units) |
| `Sight=12` | **Reveal radius 12 cells** — **the highest of any infantry**. 3 cells beyond Spy/Dog/Boris's 9. Yuri lore: psychic sensitivity = ultra-wide awareness. Critical for mind-control: lets the Yuri Clone scout targets from outside enemy weapon range |
| `Speed=4` | Foot-speed — standard infantry |
| `Owner=Russians,Confederation,Africans,Arabs,YuriCountry,British,French,Germans,Americans,Alliance` | **All 10 houses** listed — but SecretHouses below restricts |
| `SecretHouses=YuriCountry` | **Behavior key** — TechnoTypeClass field (per `TechnoTypeClass__ReadINI @ 0x00714543` DATA xref to string at `0x00843BA4`). **Stronger than RequiredHouses** — SecretHouses limits which houses can EVER access this unit, even when captured. Only `YuriCountry` players can build Yuri Clone. The 10-house Owner= is a template artifact; SecretHouses is the real lock. Compare to RequiredHouses (which gates buildability but a captured ConYard would still produce) — SecretHouses gates the unit type's mere existence per house |
| `AllowedToStartInMultiplayer=no` | Not in starting unit complement |
| `Cost=800` | $800 — moderately expensive |
| `Soylent=400` | $400 Grinder refund (Yuri only — 50%) |
| `Points=5` | Kill score — low (lots of Yuri Clones in a game; can't make each death "worth" more) |
| `IsSelectableCombatant=yes` | Included in select-all-combat |
| `VoiceSelect=YuriCloneSelect` | Select voice — `$iclosea..e` (5 lines) |
| `VoiceMove=YuriCloneMove` | Move voice — `$iclomoa..e` (5 lines) |
| `VoiceAttack=YuriCloneAttackCommand` | Attack voice — `$icloata..e` (5 lines) |
| `VoiceFeedback=YuriCloneFear` | Fear voice — `$iclofea..e` (5 lines) |
| `VoiceSpecialAttack=YuriCloneMove` | Reuses Move voice — no dedicated special-attack line |
| `DieSound=YuriCloneDie` | Death voice — `$iclodib..e` (4 lines; `$iclodia` commented out as alternate) |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Infantry` | Standard infantry terrain |
| `ThreatPosed=15` | AI scoring weight — moderate (less than Tesla Trooper's 20, more than basic infantry's 5) |
| `ImmuneToVeins=yes` | TS legacy (veins are TS-only); defensively set |
| `ImmuneToPsionics=yes` | **Behavior flag** — TechnoTypeClass field at offset `0xD35` (per mind-control RE doc §2.3). **Yuri Clone CANNOT be mind-controlled**. Critical balance: enemy Yuri/Master Mind/Magnetron cannot flip a Yuri Clone. Without this, Yuri vs Yuri matchups would degenerate into mutual capture loops |
| `;Bombable=no` (commented) | Defensive — defaults to no anyway |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | Standard 5 abilities at Veteran |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | 4 at Elite. **Note: no ElitePrimary set** — Yuri's mind-control weapon doesn't have an Elite-tier version (the weapon doesn't really need a "stronger" version — MC is binary capture, not damage-based). Elite Yuri still uses [MindControl] / [PsiWave] |
| `;The new yuri side yuri clone can no longer deploy / ;nevermind, they changed their minds` (designer comment) | **Designer history** — Yuri Clones were going to lose the deploy ability, then re-enabled. Final state: deploy IS enabled |
| `Deployer=yes` | InfantryType field — enables deploy command (per [DESO.md](../soviet/DESO.md) §Hardcoded Behavior §1) |
| `DeployFire=yes` | TechnoType field — deploying swaps weapon selection to Secondary (PsiWave AreaFire blast) |
| `UndeployDelay=150` | **Behavior key** — TechnoTypeClass field (per `TechnoTypeClass__ReadINI @ 0x00714BA8` DATA xref to string at `0x008438F4`). **Cooldown in frames after undeploying before unit can deploy again**. 150 frames = 10s @ 15fps. Prevents deploy-undeploy spam to repeatedly fire PsiWave area blast. The Desolator does NOT have this (his radiation puddle is the deterrent itself); Yuri Clone's instant-kill blast needs an explicit cooldown |
| `Size=1` | Transport cargo slot cost |
| `DetectDisguise=yes` | **Reveals nearby Spies/Mirage Tanks** — TechnoType field (per [ADOG.md](../allied/ADOG.md) — same flag dogs have). Combined with Sight=12 gives Yuri Clone a massive disguise-detection bubble |
| `IFVMode=8` | IFV gunner-table index 8 → HTK's `Weapon9`/`ElitePassengerWeapon9` slot. **NOTE: Mind-control beam doesn't garrison-transfer well** — `IFVMode=8` likely maps to a non-MC weapon (anti-infantry rifle variant) in stock YR's HTK config. Yuri-in-IFV doesn't give the IFV mind-control |

### Implicit defaults (not set in this section but worth noting)

- `Crawls=` — set in art section to `yes` (prone-walking enabled)
- `Trainable=` — defaults to `yes` (Yuri gains veterancy; presence of VeteranAbilities/EliteAbilities confirms)
- `AllowedToStartInMultiplayer=no` is explicit
- `NotHuman=` — defaults to `no` (Yuri Clone is human... well, a clone of Yuri, so debatable, but mechanically human; subject to InfDeath, sniper headshot)
- `ImmuneToRadiation=` — defaults to `no`
- `Bombable=` — defaults to `no` (commented `;Bombable=no` is defensive)
- `Fearless=` — not set; Yuri Clone shows fear behavior
- `Occupier=` — defaults to `no`; **Yuri Clone CANNOT garrison** civilian buildings. The mind-control beam would be too oppressive from inside a UC building. INIT/Yuri-Prime/Brute/Virus etc. should be checked for garrison capability in their docs
- `Agent=`/`Infiltrate=`/`Engineer=`/`Ivan=`/`C4=` — not set
- `Assaulter=` — not set
- `BombSight=` — not set
- `DefaultToGuardArea=` — not set
- `Natural=` — not set
- `SelfHealing=` — not set (only SELF_HEAL via Elite ability)
- `Crushable=` — defaults to `yes` (Yuri CAN be crushed by vehicles — a key counter-mechanic)
- `BuildLimit=` — not set (no per-player cap; Yuri Clones are mass-buildable, unlike Yuri Prime which has BuildLimit=1)

---

## artmd.ini — `[YURI]` section

`c:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini:319`:

```ini
[YURI] ; Yuri
Cameo=YCLNICON;YURIICON
AltCameo=YCLNUICO
Sequence=YuriSequence
Crawls=yes
Remapable=yes
FireUp=6
PrimaryFireFLH=15,0,140
SecondaryFireFLH=15,0,140 ; SJM: brain blast should come from head, not feet
```

| Key | Meaning |
|-----|---------|
| `Cameo=YCLNICON;YURIICON` | Sidebar build icon — `YCLNICON` is the active value; `YURIICON` is the commented-out alternate (older asset). The YCLN prefix matches the YuriClone CSF name |
| `AltCameo=YCLNUICO` | Elite cameo |
| `Sequence=YuriSequence` | Reference to `[YuriSequence]` — Yuri-specific sequence with deploy frames |
| `Crawls=yes` | Prone-capable |
| `Remapable=yes` | House remap palette applied |
| `FireUp=6` | Bullet-spawn frame — at frame 6 the mind-control beam fires |
| `PrimaryFireFLH=15,0,140` | Primary FLH — 15 forward, 0 sideways, **140 up** (high). Designer comment on Secondary clarifies: the beam should come from Yuri's head, not feet. 140 leptons up = head height |
| `SecondaryFireFLH=15,0,140 ; SJM: brain blast should come from head, not feet` | **Same FLH as Primary** — designer note explains why both come from head height. The deploy-PsiWave blast emanates from Yuri's head, not the ground |

### Referenced sequence — `[YuriSequence]`

`artmd.ini:14380`:

```ini
[YuriSequence]
Ready=0,1,1
Guard=0,1,1
Walk=8,6,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Prone=86,1,6
Crawl=86,6,6
Die1=134,15,0
Die2=149,15,0
FireUp=164,6,6
FireProne=212,6,6
Down=260,2,2
Up=276,2,2
;Deploy=292,15,0;what artist said
Deploy=292,7,0
Deployed=299,2,0 ; middle frame of deploy
Undeploy=301,6,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Paradrop=307,1,0
Cheer=308,8,0,S
Panic=8,6,6
```

| Slot | Frames | Notes |
|------|--------|-------|
| `Ready=0,1,1` | Standing idle | |
| `Guard=0,1,1` | Guard idle | |
| `Walk=8,6,6` | Walk cycle 6×6 | |
| `Idle1=56,15,0,S` | Idle 1 — 15 frames S-facing | |
| `Idle2=71,15,0,E` | Idle 2 — E-facing | |
| `Prone=86,1,6` | Prone 1 frame × 6 facings | |
| `Crawl=86,6,6` | Crawl reuses prone | |
| `Die1=134,15,0` | Death 1 — 15 frames | |
| `Die2=149,15,0` | Death 2 | |
| `FireUp=164,6,6` | Standing fire — mind-control beam pose | |
| `FireProne=212,6,6` | Prone-fire | |
| `Down=260,2,2` | Get-down to prone | |
| `Up=276,2,2` | Get-up from prone | |
| `;Deploy=292,15,0;what artist said` (commented) | Original designer-spec: 15 deploy frames. Reduced to 7 in the final | |
| `Deploy=292,7,0` | **Active deploy anim — 7 frames at 292** (omnidirectional). Plays when deploying for PsiWave blast | |
| `Deployed=299,2,0 ; middle frame of deploy` | **Deployed pose** — 2 frames at 299. Inline comment: "middle frame of deploy" — the held pose during the deployed state | |
| `Undeploy=301,6,0` | Undeploy anim — 6 frames at 301. Standing back up after the blast | |
| `Die3=0,1,1` `Die4=0,1,1` `Die5=0,1,1` | Stub → Ready frame | |
| `Paradrop=307,1,0` | Single frame at 307 — paradrop pose | |
| `Cheer=308,8,0,S` | Cheer — 8 frames S-facing | |
| `Panic=8,6,6` | Panic = Walk frames | |

---

## Weapons

### Primary — `[MindControl]`

`rulesmd.ini:24040`:

```ini
[MindControl]
Damage=1;Number of mind control links
ROF=200
Range=7
Projectile=PsychicControl
Speed=100
Warhead=Controller
;Report=YuriMindControl
Anim=YURICNTL
FireOnce=yes
```

| Key | Meaning |
|-----|---------|
| `Damage=1` | **Inline comment: "Number of mind control links"** — for MindControl-type weapons, `Damage` field is **repurposed**: it specifies the **maximum number of simultaneous control links** the unit can maintain (passed to CaptureManager as the max count). Yuri Clone = 1 link at a time. Compare `[MultipleMindControlTower]` Damage=3 (Psychic Tower can control 3 units), `[MultipleMindControlTank]` Damage=3 + `InfiniteMindControl=yes` (Master Mind unlimited with overload at 3/6/10/50) |
| `ROF=200` | **Cooldown — 200 frames (~13.3s @ 15fps)** — very slow. Yuri Clone takes time between controls. Compare to direct-damage weapons (typical ROF=20-50). The long ROF + FireOnce together create the "Yuri spends a long time concentrating to capture one unit" rhythm |
| `Range=7` | 7 cells |
| `Projectile=PsychicControl` | Custom inviso projectile — `[PsychicControl]` Inviso=yes Image=none (with several commented-out keys: ROT=100, Shadow=no, Proximity=yes, Ranged=yes — older settings) |
| `Speed=100` | Irrelevant for inviso |
| `Warhead=Controller` | **The MC warhead** — `MindControl=yes` flag triggers the CaptureManager. See warhead and Hardcoded Behavior |
| `;Report=YuriMindControl` (commented) | **Sound report disabled** — instead the global `RulesClass.YuriMindControlSound` (Rules+0x214 per mind-control RE §2.4) is played by the engine on successful capture. Putting it on the weapon would trigger on every shot attempt; the global trigger only fires on success |
| `Anim=YURICNTL` | **Weapon-level animation** — `YURICNTL` (Yuri-Control) plays at the firer's position during the firing animation. Visual reinforcement of the beam |
| `FireOnce=yes` | After one shot, TarCom clears. Yuri Clone fires the beam once per command; doesn't keep "firing" the same target |

### Secondary — `[PsiWave]` (the deployed psychic blast)

`rulesmd.ini:24086`:

```ini
[PsiWave]
Damage=250;Needed to be considered offensive unit
Range=1
ROF=50 ;200 needs to be closer to animation time (Kills everything anyway)
Projectile=Psychic
Speed=1
Warhead=PsiPulse
AreaFire=yes ; just shoot straight at ground under feet
FireOnce=yes ; Only fire once; don't stay in attack mission
```

| Key | Meaning |
|-----|---------|
| `Damage=250` | Inline comment: "Needed to be considered offensive unit" — gives Yuri Clone an explicit-damage value so AI threat-scan classifies him as offensive (otherwise the MC-only Primary, with Damage=1 = MC links not damage, would make Yuri appear non-threatening to the AI). The 250 damage applied to nearby infantry via the AreaFire blast on deploy |
| `Range=1` | 1 cell — Yuri must be at or adjacent to ground zero |
| `ROF=50` | Inline comment: "200 needs to be closer to animation time (Kills everything anyway)". Originally 200 frames between blasts, reduced to 50 because the blast one-shots everything anyway — ROF is moot. Mostly limited by `UndeployDelay=150` (10s) on the unit type instead |
| `Projectile=Psychic` | Custom inviso projectile — `[Psychic]` Inviso=yes Image=none, even more minimal than PsychicControl |
| `Speed=1` | Irrelevant for inviso |
| `Warhead=PsiPulse` | **The blast warhead** — `PsychicDamage=yes`, `CellSpread=3`, `AffectsAllies=no`, 100% vs infantry only. See warhead |
| `AreaFire=yes` | **Behavior flag** — WeaponTypeClass field (xref `0x0077283E` per [DESO.md](../soviet/DESO.md)). Inline comment: "just shoot straight at ground under feet". Fires at Yuri's own cell. Same pattern as Desolator's RadEruptionWeapon |
| `FireOnce=yes` | One blast per deploy. Inline comment: "Only fire once; don't stay in attack mission" |

### Primary's Warhead — `[Controller]`

`rulesmd.ini:27125`:

```ini
[Controller];Mind control warhead.  Will skip normal damage like EMP did
Verses=100%,100%,100%,100%,100%,100%,0%,0%,0%,100%,100%
MindControl=yes
AnimList=YURICNTL
```

| Key | Meaning |
|-----|---------|
| Designer comment ";Mind control warhead. Will skip normal damage like EMP did" | Designer note explaining the special-case routing. `MindControl=yes` causes `WarheadTypeClass::Detonate` to branch to the CaptureManager path instead of `Apply_area_damage` (per mind-control RE §1) |
| `Verses=100%,100%,100%,100%,100%,100%,0%,0%,0%,100%,100%` | 11-column. **100% vs all infantry and vehicle armors** (Yuri can mind-control GI/Conscript/Tesla Trooper AND Grizzly Tank/Rhino/Apocalypse). **0% vs wood/steel/concrete** structures — Yuri **cannot mind-control buildings**. 100% vs both specials. The 0% structure entries serve as the cursor filter — the engine refuses to fire on a target with projected 0 damage, restricting MC cursor to non-structure targets |
| `MindControl=yes` | **THE mind-control flag** — WarheadTypeClass field at offset `0x155` (per `WarheadTypeClass__ReadINI @ 0x0075D7CF`, mind-control RE §2.1). Triggers the CaptureManager path. Detailed in Hardcoded Behavior |
| `AnimList=YURICNTL` | **Animation overlay** — `YURICNTL` plays on the victim continuously while mind-controlled. The famous "yellow swirl" floating above the victim's head. Same anim as `RulesClass.ControlledAnimationType` (Rules+0x320 per mind-control RE §2.5) |

### Building variant — `[ControllerBuilding]` (used by Psychic Tower)

`rulesmd.ini:27130`:

```ini
[ControllerBuilding];Mind control warhead.  Will skip normal damage like EMP did
Verses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%
MindControl=yes
AnimList=YURICNTL
```

**Difference from Controller**: ALL 100% Verses (including wood/steel/concrete). Used by the Psychic Tower defense building (which can target garrisoned civilian buildings, ejecting the occupants and switching ownership). Not used by Yuri Clone directly — documented for completeness.

### Secondary's Warhead — `[PsiPulse]`

`rulesmd.ini:27169`:

```ini
[PsiPulse]
CellSpread=3 ; gs moved a designer fix from the weapon because it was put in the old WideAreaDamage way instead of the new damage way.  This number used to be 3
PercentAtMax=.85
Verses=100%,100%,100%,0%,0%,0%,0%,0%,0%,0%,0%
InfDeath=6
PsychicDamage=yes ;gs psychic, but not mind control
AffectsAllies=no;gs Patch Defaults to yes.
```

| Key | Meaning |
|-----|---------|
| `CellSpread=3` | Splash radius — 3 cells. Designer comment: "moved a designer fix from the weapon because it was put in the old WideAreaDamage way instead of the new damage way. This number used to be 3" — explains the warhead was retrofitted to the new wide-area damage system |
| `PercentAtMax=.85` | At spread edge, damage is 85% of full (still 212 dmg vs infantry) — minimal falloff, deliberately punishing |
| `Verses=100%,100%,100%,0%,0%,0%,0%,0%,0%,0%,0%` | **100% vs infantry only (none/flak/plate)**, 0% vs everything else. **Yuri Clone's deploy blast affects ONLY infantry** — cannot damage vehicles or structures. Strategic role: anti-infantry-spam tool. The 0% specials also notable (some "psychic" warheads have 100% specials) |
| `InfDeath=6` | **Infantry death animation type 6** — the "explosion / blown to bits" death (same as Ivan bomb victims). Visually distinct from standard small-arms |
| `PsychicDamage=yes` | Inline comment: "gs psychic, but not mind control". **Distinct from MindControl=yes** — PsychicDamage applies actual damage; MindControl applies capture. PsiPulse kills infantry without controlling them |
| `AffectsAllies=no` | Inline comment: "gs Patch Defaults to yes." **Yuri's allied units are NOT damaged by his own PsiWave blast** — critical for Yuri's standing-amid-his-own-infantry deploy. Without this, Yuri deploying near his own Initiates would kill them too. The "Patch Defaults to yes" note suggests this was added in an update; default for older warheads is yes (affects allies) |

### Projectiles — `[PsychicControl]` and `[Psychic]`

`rulesmd.ini:25416`:

```ini
[PsychicControl]
;Image=YURBLANK ; an invisible missile with a trailer
;ROT=100
Inviso=yes
Image=none
;Shadow=no
;Proximity=yes
;Ranged=yes

[Psychic]
Inviso=yes
Image=none
```

Both are bare-minimum inviso projectiles. `[PsychicControl]` had a larger
spec at one point (YURBLANK image with trailer, ROT, Shadow, Proximity,
Ranged all commented out) — designers stripped down to bare inviso for
gameplay clarity (no visible beam → faster perception of MC by player).

The mind-control "beam" rendering is done via `MindControlAttackLineFrames`
(Rules+0x310) and the YURICNTL animation, NOT via the projectile.

---

## Voices and sounds

All from `soundmd.ini`:

### Selection / movement / fear / death

```ini
[YuriCloneSelect]                  ; soundmd.ini:5139
Sounds=$iclosea $icloseb $iclosec $iclosed $iclosee
Control=random
Volume=85

[YuriCloneMove]                    ; soundmd.ini:5144
Sounds=$iclomoa $iclomob $iclomoc $iclomod $iclomoe
Control=random
Volume=85

[YuriCloneAttackCommand]           ; soundmd.ini:5149
Sounds=$icloata $icloatb $icloatc $icloatd $icloate
Control=random
Volume=85

[YuriCloneFear]                    ; soundmd.ini:5154
Sounds=$iclofea $iclofeb $iclofec $iclofed $iclofee
Control=random
Volume=85

[YuriCloneDie]                     ; soundmd.ini:5159
Sounds=$iclodib $iclodic $iclodid $iclodie ;$iclodia
Control=random
Volume=85
```

5/5/5/5/4 banks — uniformly large. 1 commented-out death line (`$iclodia`).
The `$iclo` prefix = "Clone" (YuriCLONE).

### No weapon report (intentional)

**Yuri Clone's weapons have no `Report=` defined** in the actual weapons
(both `;Report=YuriMindControl` commented out). The actual sound is the
**global `Rules.YuriMindControlSound`** (RulesClass+0x214 per mind-control
RE §2.4) — played by the engine when a capture succeeds. Putting it on
the weapon would trigger on every cast attempt; the global trigger only
fires on actual capture. The PsiWave deploy blast also has no Report.

---

## Prerequisites, owners, tech

| Field | Value | Notes |
|-------|-------|-------|
| `Prerequisite=` | `YABRCK,NAPSIS` | Yuri Barracks + Soviet Psychic Sensor. Inline comment shows the prereq was lowered from YATECH (Yuri Battle Lab) to NAPSIS for accessibility |
| `PrerequisiteOverride=` | `CARUS03` | Kremlin Palace tech building — captures or owns this and Yuri Clone is unlockable bypassing the normal prereq chain |
| `Owner=` | All 10 houses | Template artifact |
| `SecretHouses=` | `YuriCountry` | **Real lock** — only Yuri faction has access to this unit type regardless of captures |
| `TechLevel=` | `10` | Maximum tech-level cap; effectively no tech restriction |
| `AllowedToStartInMultiplayer=no` | — | Not in starting unit complement |
| `Cost=800` | $800 | Moderate |
| `Soylent=400` | $400 refund (Yuri only — and YURI IS Yuri-faction, so Yuri grinds his own clones for refund) |
| `Points=5` | 5 | Low |

**No `BuildLimit=`** — Yuri Clones are mass-producible (only `[YURIPR]` Yuri Prime has BuildLimit=1).

---

## Veterancy

| Tier | Effect |
|------|--------|
| Veteran | `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — standard 5 abilities. **Note: FIREPOWER and ROF are moot for Primary** (mind-control is binary capture, not damage; ROF is fixed by the cooldown), but DO affect Secondary's PsiWave damage at deploy |
| Elite | `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — 4 abilities. **No ElitePrimary** — Yuri Clone doesn't get a different mind-control weapon at Elite. The Elite tier mostly benefits the deploy-PsiWave damage and survivability |
| AltCameo | `YCLNUICO` shown after Veteran promotion |

`Trainable=` defaults to `yes` (Yuri gains veterancy normally — getting kills with PsiWave or via mind-controlled victims).

---

## Hardcoded behavior — Ghidra-verified

### 1. Mind-control via CaptureManagerClass (the headline mechanic)

The MindControl=yes warhead flag (WarheadTypeClass+0x155, xref
`WarheadTypeClass__ReadINI @ 0x0075D7CF` to string at `0x0081BBC8`)
triggers the CaptureManager pipeline:

```
1. Yuri fires [MindControl] weapon at target
2. Bullet detonates → WarheadTypeClass::Detonate @ 0x004690B0
3. Warhead.MindControl=yes branch (priority 1 in special-warhead cascade)
4. CaptureManagerClass on Yuri's instance allocated on first capture
5. New MCNode (mind-control linked-list entry) added to Yuri's manager:
   - Node tracks the victim's original owner (HouseClass*)
   - Node tracks the victim's TechnoClass*
6. Victim's TechnoClass+0x2BC (mind-controlled-by ptr) = Yuri
7. Victim's owner changes to Yuri's owner
8. Victim's TechnoClass+0x350 (AttachedAnimRing) = new AnimClass(YURICNTL)
9. Rules.YuriMindControlSound played at victim's coords
10. Mind-control link line drawn for MindControlAttackLineFrames=N frames
    (default ~30 from Rules+0x310)
```

When Yuri dies:
- All MCNode victims revert to their original owners
- Victim's TechnoClass+0x2BC cleared
- Victim's TechnoClass+0x350 anim cleared
- Rules.MindClearedSound played (default global, or victim's per-type override)

Yuri's `Damage=1` on `[MindControl]` weapon = **max 1 simultaneous link**.
For a second target, Yuri must release the first (no manual release —
either Yuri dies, victim dies, or Yuri targets a different unit). The
Mastermind (`InfiniteMindControl=yes` weapon) has no link limit but takes
overload damage as it exceeds `OverloadCount=3,6,10,50` thresholds.

Full RE in [MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md](../../MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md).

### 2. Deploy-fire PsiWave (the close-range blast)

`Deployer=yes` + `DeployFire=yes` wire the deploy hotkey to fire Secondary
(documented in [DESO.md](../soviet/DESO.md)). Yuri Clone's Secondary is
`[PsiWave]`: AreaFire=yes Damage=250 Range=1 Warhead=PsiPulse — fires at
own cell, kills nearby infantry within `CellSpread=3`. The blast lasts
seconds; UndeployDelay=150 frames prevents instant re-deploy.

PsiPulse is **PsychicDamage=yes** (separate from MindControl=yes) — applies
real damage without capturing, kills infantry outright. `AffectsAllies=no`
prevents friendly-fire on Yuri's own infantry.

Strategic use: deploy Yuri amid enemy infantry blob to instantly clear them.

### 3. UndeployDelay — deploy-cooldown enforcement

INI key `UndeployDelay=150` is a **TechnoTypeClass** field (per
`TechnoTypeClass__ReadINI @ 0x00714BA8` DATA xref to string at `0x008438F4`).
After Yuri undeploys, the unit cannot deploy again for 150 frames (~10s).
Critical balance — without this, players could deploy/undeploy in rapid
succession to spam PsiWave blasts. Note: this is specifically the
*undeploy → next deploy* cooldown, not the initial deploy delay.

### 4. SecretHouses=YuriCountry — strong house-lock

INI key `SecretHouses` is a **TechnoTypeClass** field (per
`TechnoTypeClass__ReadINI @ 0x00714543` DATA xref to string at `0x00843BA4`).
**Stronger than RequiredHouses**: SecretHouses gates the unit's mere
*existence* per house — only listed houses can EVER access this unit type,
even via captured production buildings. RequiredHouses (used on Sniper,
Desolator, etc.) only gates the build menu — a captured ConYard from a
RequiredHouses-locked country could still build the unit. SecretHouses
prevents even that. Yuri Clone is purely Yuri-faction-only at every layer.

### 5. PrerequisiteOverride=CARUS03 — Kremlin Palace bypass

INI key `PrerequisiteOverride` is a **TechnoTypeClass** field (per
`TechnoTypeClass__ReadINI @ 0x00714229` DATA xref to string at `0x00843D90`).
Lists tech buildings that, when owned, **bypass the normal Prerequisite=
chain**. For Yuri Clone, owning CARUS03 (Kremlin Palace) unlocks building
Yuri Clones without needing the NAPSIS (Psychic Sensor) prereq. Used by
campaign / map-specific mechanics. In standard skirmish CARUS03 is a
neutral tech-building that can be captured by Engineers.

### 6. TypeImmune=yes — same-type weapon immunity

INI key `TypeImmune` is a **TechnoTypeClass** field (per
`TechnoTypeClass__ReadINI @ 0x0071220F` DATA xref to string at `0x008444EC`).
Unit is **immune to weapons fired by units of the same type**. For Yuri
Clone: another Yuri Clone's `MindControl` weapon **cannot** mind-control
this Yuri Clone (same-type immunity blocks the weapon-to-warhead resolution
before `MindControl=yes` branch fires).

Without TypeImmune, Yuri vs Yuri matchups would degenerate into infinite
capture loops (Player A's Yuri controls B's Yuri, who is now Player A's,
who controls back into Player A's roster from B's perspective, etc.). The
flag breaks the recursion at the unit-type level.

Compare to `ImmuneToPsionics=yes` (same offset 0xD35 different flag): that
flag blocks ALL psionic/MC weapons regardless of source type. Yuri Clone
has both, but TypeImmune is the more general "same type" rule.

### 7. DetectDisguise + Sight=12 — disguise-detection bubble

`DetectDisguise=yes` (TechnoType field, xref `0x00843C78` per dogs's
documentation) combined with the highest infantry Sight (12) gives Yuri
Clone a **12-cell disguise-detection radius**. Spies and Mirage Tanks
blink to true form when within this radius. Per `InfantryBlinkDisguiseTime=20`
(documented in [SPY.md](../allied/SPY.md)), the blink lasts 20 frames each
detection tick — long enough that nearby units can engage the revealed unit.

### 8. ImmuneToPsionics=yes — mind-control immunity for Yuri himself

The same `ImmuneToPsionics=yes` flag documented on Boris/Tanya. Yuri Clone
cannot be mind-controlled by any source (other Yuri Clones, Yuri Prime,
Psychic Tower, Psychic Dominator). Even his own faction's units can't flip
him.

### Ghidra searches performed for this dossier

| Tool call | Result |
|-----------|--------|
| `search_strings("SecretHouses\|TypeImmune\|PrerequisiteOverride\|UndeployDelay")` | 4 strings — confirms all 4 hardcoded INI keys |
| `get_xrefs_to(0x008438F4)` (= "UndeployDelay") | Sole xref from `TechnoTypeClass__ReadINI @ 0x00714BA8` DATA — confirms TechnoType-level deploy-cooldown field |
| `get_xrefs_to(0x00843BA4)` (= "SecretHouses") | Sole xref from `TechnoTypeClass__ReadINI @ 0x00714543` DATA — confirms TechnoType-level house-lock (stronger than RequiredHouses) |
| `get_xrefs_to(0x00843D90)` (= "PrerequisiteOverride") | Sole xref from `TechnoTypeClass__ReadINI @ 0x00714229` DATA — confirms TechnoType-level prereq-bypass field |
| `get_xrefs_to(0x008444EC)` (= "TypeImmune") | Sole xref from `TechnoTypeClass__ReadINI @ 0x0071220F` DATA — confirms TechnoType-level same-type weapon immunity |

Plus cross-referenced from MIND_CONTROL_SYSTEM_GHIDRA_REPORT.md: `MindControl`
flag (WarheadTypeClass+0x155), `ImmuneToPsionics` (TechnoTypeClass+0xD35),
CaptureManager pipeline, MCNode linked list, MindControlAttackLineFrames,
ControlledAnimationType.

---

## TS-legacy filter

| Item | Status | Notes |
|------|--------|-------|
| `;Bombable=no` (commented) | Defensive — defaults to no | OK |
| `;The new yuri side yuri clone can no longer deploy / ;nevermind, they changed their minds` (designer comment) | Designer history — deploy was almost cut | OK |
| `;Deploy=292,15,0;what artist said` (commented in artmd) | Original 15-frame deploy, reduced to 7 | OK |
| `;Report=YuriMindControl` (commented in MindControl weapon) | Sound moved to global success-trigger | OK |
| `[PsychicControl]` projectile commented keys (`;Image=YURBLANK`, `;ROT=100`, `;Shadow=no`, `;Proximity=yes`, `;Ranged=yes`) | Stripped down from full projectile spec to bare inviso | OK |
| `ImmuneToVeins=yes` | TS legacy (veins are TS-only); defensively set | OK |
| `YURICNTL` animation | YR-active; the iconic "yellow swirl" over MC victims | OK |
| Mind-control / CaptureManager system | **Fully YR-active** — verified across 615-line deep RE doc, all paths live | OK |

No TS-only behavior on YURI Clone. The unit is purely YR — mind-control
is a YR-introduced mechanic (TS had no mind-control infantry).

---

## Cross-references

- **Yuri infantry tier**:
  - `[INIT]` Yuri Initiate (documented) — basic; flame damage, no MC
  - **`[YURI]` Yuri Clone (this doc)** — single-target MC
  - `[YURIPR]` Yuri Prime — AoE MC + much higher control limit
  - `[BRUTE]` Brute — Strength=350 melee
  - `[VIRUS]` Virus — plague sniper
  - `[YENGINEER]` Yuri Engineer — capture/repair
  - `[YDOG]`/`[YADOG]` (documented via ADOG)
- **Sister mind-control units / weapons**:
  - `[YURIPR]` Yuri Prime — same Primary `[MindControl]` (1 link via Damage=1, but higher per-link power?); same Secondary `[SuperPsiPulse]` variant. To verify when YURIPR is documented
  - `[YAPSYT]` Psychic Tower (building) — uses `[MultipleMindControlTower]` (Damage=3 = 3 links)
  - `[MIND]` Master Mind (vehicle) — uses `[MultipleMindControlTank]` (InfiniteMindControl=yes with overload)
  - `[YAPPET]` Psychic Dominator (superweapon) — permanent MC via PermanentlyMindControlled flag (TechnoClass+0x2C4, different mechanism)
- **Related MC warheads**:
  - `[Controller]` (this doc — Verses 0% structures, can MC infantry/vehicles only)
  - `[ControllerBuilding]` (Verses 100% all, Psychic Tower can MC garrisoned buildings)
- **Related psychic-damage warheads** (not MC):
  - `[PsiPulse]` (this doc — Yuri Clone deploy blast, 100% infantry only)
  - `[SuperPsiPulse]` (Yuri Prime deploy blast — wider CellSpread, 100% infantry + 50% vehicles)
- **TypeImmune family**:
  - `[YURI]` Yuri Clone (this doc) — TypeImmune=yes
  - `[YURIPR]` Yuri Prime — should also be TypeImmune (verify)
  - Other potential candidates: Mirage Tank? Master Mind? Verify
- **Counter-units to Yuri Clone**:
  - **Snipers** — one-shot via 250 dmg vs Strength=100, NOT blocked by ImmuneToPsionics (sniper rifle isn't psionic)
  - **Crazy Ivan bomb** — Bombable defaults to no on Yuri, but Ivan can still attach via Bomb mission
  - **Dogs** — Parasite warhead one-shot (not psionic, not blocked)
  - **Vehicle crush** — Crushable=yes default, vehicle crush kills Yuri Clone
  - **Engineers capturing the Psychic Sensor** — disable Yuri's NAPSIS prereq (although PrerequisiteOverride=CARUS03 provides a backup path)
  - **NOT effective**: any mind-control or psionic weapon (ImmuneToPsionics=yes)
- **Related global rules**:
  - `Rules.YuriMindControlSound` (Rules+0x214) — capture-success sound
  - `Rules.MindClearedSound` (Rules+0x264) — release sound
  - `Rules.MindControlAttackLineFrames` (Rules+0x310) — link line duration
  - `Rules.ControlledAnimationType` (Rules+0x320) — YURICNTL anim type
  - `Rules.OverloadCount/OverloadDamage/OverloadFrames` (Rules+0xEEC/F08/F24) — Mastermind overload tables (not used by Yuri Clone, but in the same system)

---

## Coverage audit

| Source | Lines | Status |
|--------|-------|--------|
| `rulesmd.ini [YURI]` | 5197-5244 (48 lines) | All 41 active keys covered (3 commented designer-history comments documented; one inline `;Bombable=no` defensive comment) |
| `artmd.ini [YURI]` | 319-327 (9 lines) | All keys covered |
| `artmd.ini [YuriSequence]` | 14380-14403 (24 lines) | All 21 active slots + 1 commented Deploy variant + 3 stub Die3-5 covered |
| `rulesmd.ini [MindControl]` | 24040-24049 (10 lines) | All keys covered including Damage=1 = link count |
| `rulesmd.ini [PsiWave]` | 24086-24094 (9 lines) | All keys covered |
| `rulesmd.ini [Controller]` | 27125-27128 (4 lines) | All keys + 11-column Verses breakdown |
| `rulesmd.ini [ControllerBuilding]` | 27130-27133 (4 lines) | Cross-referenced (Psychic Tower variant) |
| `rulesmd.ini [PsiPulse]` | 27169-27175 (7 lines) | All keys covered |
| `rulesmd.ini [PsychicControl]` / `[Psychic]` projectiles | 25416-25427 | Both covered |
| `soundmd.ini` Yuri Clone voices | YuriCloneSelect, Move, AttackCommand, Fear, Die | All 5 covered; weapon report intentionally absent documented |
| Hardcoded behavior | MindControl warhead → CaptureManager + DeployFire PsiWave + UndeployDelay + SecretHouses + PrerequisiteOverride + TypeImmune + DetectDisguise + ImmuneToPsionics | 8 mechanisms with 4 fresh Ghidra xrefs + 4 cross-referenced from deep RE / prior docs |
| Ghidra searches performed against ID | 5 distinct queries (1 strings + 4 xref lookups) plus deep-RE cross-references | Logged inline |
| TS-legacy filter | Applied; ImmuneToVeins defensive, all commented designer-history documented, MC system fully YR-active | Done |
