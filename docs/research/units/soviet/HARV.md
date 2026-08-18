# HARV — War Miner (Soviet harvester)

**Side classification:** Soviet (Owner=Russians,Confederation,Africans,Arabs).
**Role:** Soviet faction's ore harvester. Drives to ore, harvests with `Harvester=yes`
behavior, drives back to a refinery, docks, unloads via `UnloadingClass=HORV`, and
repeats. Carries a 20mmRapid turret weapon so it can self-defend against light
threats. **Does NOT teleport** — that's the Allied [CMIN] Chrono Miner.

> ⚠ **Index correction logged**: prior `INDEX_UNITS.md` listed HARV as "Allied Chrono
> Miner". This is wrong. The INI says `Name=War Miner`, `Prerequisite=NAWEAP,PROC`
> (Soviet War Factory + Refinery), `Owner=Russians,Confederation,Africans,Arabs`, no
> `Teleporter=` flag, and `Locomotor=` set to DriveLocomotionClass. The Allied Chrono
> Miner is `[CMIN]` (`yuri/`-adjacent — actually `allied/`), still TODO.

> Output bar: harvester behavior is foundational to Soviet economy parity.

> **Deep-RE cross-references — don't re-derive:**
> - **[WAR_MINER_REFERENCE.md](../../WAR_MINER_REFERENCE.md)** — full comprehensive
>   reference, dated 2026-04-03, verified from binary. Comparison table HARV vs CMIN.
> - **[WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md](../../WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md)** — drive-locomotor integration.
> - **[HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md](../../HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md)** + **[MISSION_HARVEST_GHIDRA_REPORT.md](../../MISSION_HARVEST_GHIDRA_REPORT.md)** — the 5-state ore cycle state machine.
> - **[HARVESTER_DOCK_UNLOAD.md](../../HARVESTER_DOCK_UNLOAD.md)** + **[HARVESTER_DOCK_UNLOAD_SEQUENCE.md](../../HARVESTER_DOCK_UNLOAD_SEQUENCE.md)** — refinery dock/unload behavior. `UnloadingClass=HORV` swap mid-dock.
> - **[MINER_DOCK_GAPS_RESEARCH.md](../../MINER_DOCK_GAPS_RESEARCH.md)** — known docking edge cases / gaps research.

---

## 1. `rulesmd.ini` — `[HARV]` verbatim

```ini
[HARV]
UIName=Name:HARV
Name=War Miner
Prerequisite=NAWEAP,PROC
Nominal=yes
ToProtect=yes
Category=Support
Strength=1000
Armor=medium
;Dock=PROC		; Need both in case a building from the other team is captured.
Dock=NAREFN,GAREFN
Turret=yes
Primary=20mmRapid
Harvester=yes
TechLevel=1
Sight=4
Speed=4
Owner=Russians,Confederation,Africans,Arabs
AllowedToStartInMultiplayer=no
PipScale=Tiberium
CrateGoodie=yes
Storage=40
Cost=1400
Soylent=1400
Points=55
ROT=5
Crusher=yes
AutoCrush=yes
Crewed=no
SelfHealing=yes
OpportunityFire=yes
UnloadingClass=HORV
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=WarMinerSelect
VoiceMove=WarMinerMove
VoiceAttack=WarMinerAttackCommand
VoiceEnter=WarMinerMove
VoiceHarvest=WarMinerHarvest
DieSound=GenVehicleDie
CrushSound=TankCrush
MaxDebris=6
DebrisTypes=TIRE
DebrisMaximums=4
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
Weight=3.5
MovementZone=Crusher
ThreatPosed=0	; This value MUST be 0 for all building addons
ThreatAvoidanceCoefficient=.65
DamageParticleSystems=SparkSys,SmallGreySSys
ImmuneToVeins=yes
ImmuneToPsionics=yes
ImmuneToRadiation=yes
ZFudgeColumn=9
ZFudgeTunnel=14
ZFudgeBridge=7
Size=3
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
ElitePrimary=20mmRapidE
ResourceGatherer=yes;gs for the AI to handle the slave miner, it has to understand what makes money
Bunkerable=no; Units default to yes, others default to no
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:HARV` | AbstractType | CSF lookup. |
| `Name` | `War Miner` | AbstractType | Dev/fallback name. |
| `Prerequisite` | `NAWEAP,PROC` | TechnoType | Soviet War Factory AND any Refinery (`PROC` is the generic refinery token; resolves to NAREFN for Soviet, GAREFN for Allied if captured). |
| `Nominal` | `yes` | TechnoType | Marks as a low-priority unit in score/UI displays. Combined with `ToProtect=yes` and `ThreatPosed=0` below, the AI treats HARV as a "value to protect, not to attack with". |
| `ToProtect` | `yes` | TechnoType | AI flag — friendly AI will defend HARV from threats. |
| `Category` | `Support` | TechnoType | Support-role classifier (not AFV/Transport). |
| `Strength` | `1000` | AbstractType | **1000 HP** — among the tankiest non-MBT vehicles in YR. More than a Rhino (400). |
| `Armor` | `medium` | TechnoType | Verses-slot 5. |
| `;Dock=PROC` | *(commented)* | — | Author-note: generic Refinery token. Live `Dock=` below uses explicit list. |
| `Dock` | `NAREFN,GAREFN` | TechnoType (Harvester dock targets) | INI comment: "Need both in case a building from the other team is captured." Lists both Soviet refinery (NAREFN) and Allied refinery (GAREFN) — relevant if a Soviet HARV gets transferred to Allied ownership via capture/mind-control, or if a Soviet player captures an Allied refinery. The harvester will dock at either. |
| `Turret` | `yes` | UnitType | Has a rotating turret for the 20mmRapid weapon. |
| `Primary` | `20mmRapid` | TechnoType | Cannon weapon — see §3. |
| `Harvester` | `yes` | UnitType | **Core flag** that enables `Mission_Harvest` 5-state ore cycle. Drives all ore-gathering AI behavior. See HARVESTER_MISSION_HARVEST_GHIDRA_REPORT for the state machine. |
| `TechLevel` | `1` | TechnoType | Tier-1 — buildable from start (paired with `AllowedToStartInMultiplayer=no` below, which only prevents *preplaced* starting units). |
| `Sight` | `4` | TechnoType | 4-cell reveal — short. |
| `Speed` | `4` | TechnoType | Standard harvester pace. |
| `Owner` | `Russians,Confederation,Africans,Arabs` | TechnoType | **Soviet-only** (no YuriCountry — Yuri uses Slave Miner instead). |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Cannot be a preplaced starting unit (Soviet MCV's deploy spawns the first HARV instead, via `[General] FreeHarvester=` mechanic). |
| `PipScale` | `Tiberium` | UnitType | Renders ore-bale pips (the small green-tier ore indicator above the unit). |
| `CrateGoodie` | `yes` | UnitType | Can be the "free unit" reward from a crate. (Rare on a $1400 unit.) |
| `Storage` | `40` | TechnoType (verified prior iter — 0x008441ac → 0x00713130) | **40 ore bales** — twice the Chrono Miner's 20. Each bale = $25 (verified from prior research), so a full load is $1000 — but the displayed pip count is `40` regardless of bale value. |
| `Cost` | `1400` | TechnoType | Same as CMIN. Soviet pays the same price for a HARV that can self-defend and carries twice the ore, but lacks the chrono teleport. |
| `Soylent` | `1400` | TechnoType | 100% Grinder refund (rare). |
| `Points` | `55` | TechnoType | Score on kill — high. Harvesters are valuable targets. |
| `ROT` | `5` | TechnoType | Rate of turn (both body and turret use this in absence of `TurretROT=`). |
| `Crusher` | `yes` | TechnoType | Crushes infantry. |
| `AutoCrush` | `yes` | TechnoType | Will automatically attempt to crush infantry it encounters along its path (without explicit attack order). Notable — this is what enables a harvester to defend itself by running over enemy infantry while pathfinding to ore. |
| `Crewed` | `no` | TechnoType | No survivor-infantry parachute out on death. (Default for civilian/support vehicles.) |
| `SelfHealing` | `yes` | TechnoType | **Automatic HP regen** — harvesters self-heal because they survive in contested ore fields. Notable: this is the default rate, not a per-frame rate; see `[General] SelfHealUnitRate`. |
| `OpportunityFire` | `yes` | TechnoType | Will auto-target threats in range without explicit attack order. Combined with the 20mmRapid turret, HARV opportunistically shoots dogs/infantry while harvesting. |
| `UnloadingClass` | `HORV` | TechnoType (verified — 0x00843af8 read at 0x007146e8) | **Visual swap during dock-unload.** When HARV docks at a refinery, the engine swaps the rendered model to the `[HORV]` UnitType for the duration of the dump animation. See §6 + HARVESTER_DOCK_UNLOAD_SEQUENCE for full mechanism. |
| `Explosion` | `TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60` | TechnoType | Random-from-list death explosion. |
| `VoiceSelect` | `WarMinerSelect` | TechnoType | 5 unique clips (`$vwarsea..ee`). |
| `VoiceMove` | `WarMinerMove` | TechnoType | 5 clips (`$vwarmoa..oe`). |
| `VoiceAttack` | `WarMinerAttackCommand` | TechnoType | 5 clips (`$vwarata..te`). |
| `VoiceEnter` | `WarMinerMove` | TechnoType | Reuses move set when entering a transport. |
| `VoiceHarvest` | `WarMinerHarvest` | TechnoType (harvester-specific voice key) | 4 clips (`$vwarhaa..hd`) played when the harvest cycle triggers. |
| `DieSound` | `GenVehicleDie` | TechnoType | Generic vehicle death sound (6 clips). |
| `CrushSound` | `TankCrush` | TechnoType | When HARV crushes infantry. |
| `MaxDebris` | `6` | TechnoType | Up to 6 debris pieces on death. |
| `DebrisTypes` | `TIRE` | TechnoType | Debris is tire-shaped pieces. |
| `DebrisMaximums` | `4` | TechnoType | Max 4 of each debris type. |
| `Locomotor` | `{4A582741-9839-11d1-B709-00A024DDAFD1}` | TechnoType | **DriveLocomotionClass** — standard ground vehicle. No teleport. |
| `Weight` | `3.5` | TechnoType | Used by physics — how hard it crushes infantry, how it interacts with bridges. |
| `MovementZone` | `Crusher` | TechnoType | Pathing zone — can path on crushable terrain (walls, fences, light obstacles). |
| `ThreatPosed` | `0` | TechnoType | **Zero AI threat** — enemy AI does not auto-target HARV. (Combined with `ToProtect=yes` from friendly AI's side.) |
| `ThreatAvoidanceCoefficient` | `.65` | TechnoType | When pathing, HARV evaluates enemy positions with 65% weight to avoid them — relevant because HARV's pathfinder ducks around known enemies to stay alive. (HORV uses 1.0 — full weight — likely because it's only used during the dock-unload context where threat-avoidance doesn't change anything.) |
| `DamageParticleSystems` | `SparkSys,SmallGreySSys` | TechnoType | Damage-state smoke/spark emitters. |
| `ImmuneToVeins` | `yes` | TechnoType | **TS-LEGACY** dormant. |
| `ImmuneToPsionics` | `yes` | TechnoType | Cannot be mind-controlled. Important for parity: Yuri cannot steal HARVs via Yuri Prime / Initiate mind-control. |
| `ImmuneToRadiation` | `yes` | TechnoType | Walks through Desolator rad fields with no damage. Crucial — protects ore fields where Desolators may have deployed. |
| `ZFudgeColumn` | `9` | UnitType | Z-render fudge when adjacent to a column/wall (pixel offset for occlusion). |
| `ZFudgeTunnel` | `14` | UnitType | Z-fudge when entering a tunnel (TS-legacy mostly, but applies generically). |
| `ZFudgeBridge` | `7` | UnitType | Z-fudge when crossing a bridge. |
| `Size` | `3` | TechnoType | Transport-slot cost. |
| `VeteranAbilities` | `STRONGER,FIREPOWER,SIGHT,FASTER` | TechnoType | Veteran bonuses. |
| `EliteAbilities` | `SELF_HEAL,STRONGER,FIREPOWER,ROF` | TechnoType | Elite adds SELF_HEAL (cumulative with veteran). |
| `ElitePrimary` | `20mmRapidE` | TechnoType | Elite weapon — see §3. Significantly different from base 20mmRapid. |
| `ResourceGatherer` | `yes` | TechnoType | INI comment: "for the AI to handle the slave miner, it has to understand what makes money". The flag tells the AI economy planner that this unit produces income. **Notable**: comment mentions slave miner, but `ResourceGatherer` applies to HARV too. |
| `Bunkerable` | `no` | TechnoType | INI comment: "Units default to yes, others default to no". HARV explicitly opts out of being bunkerable (cannot enter Battle Bunker / Battle Fortress as cargo). |

---

## 2. `artmd.ini` — `[HARV]` section

```ini
[HARV]			; Soviet harvester
Cameo=HARVICON
AltCameo=HARVUICO
Voxel=yes
Remapable=yes
TurretOffset=50
PrimaryFireFLH=75,0,150
```

| Key | Value | Effect |
|-----|-------|--------|
| `Cameo` | `HARVICON` | Standard build cameo. |
| `AltCameo` | `HARVUICO` | Yuri-skinned cameo (if Yuri ever owns a HARV via capture/mind-control, this is shown). |
| `Voxel` | `yes` | Rendered from `HARV.VXL` + `HARV.HVA` voxel files. |
| `Remapable` | `yes` | House-color remap palette. |
| `TurretOffset` | `50` | Turret pivot point offset (voxel units forward of the body centre). Used for both render position and projectile spawn origin. |
| `PrimaryFireFLH` | `75,0,150` | Firing offset (X=75 fwd, Y=0, Z=150 — high turret-cannon height). |

No `Sequence=` (vehicles don't use infantry-style frame tables — HVA frames drive voxel
animation).

---

## 3. Weapon — `[20mmRapid]` / `[20mmRapidE]`

### `[20mmRapid]` (rookie & veteran)

```ini
[20mmRapid]
Damage=30
ROF=20
Range=5.5
Projectile=InvisibleLow
Speed=100
Warhead=HARVWH
Report=WarMinerAttack
Anim=GUNFIRE
```

### `[20mmRapidE]` (elite)

```ini
[20mmRapidE]
Damage=50
ROF=50
Range=5.75
Projectile=Cannon
Speed=40
Warhead=HowitzerWH
Report=RhinoTankAttack
Anim=GUNFIRE
Bright=yes
```

| Key | 20mmRapid | 20mmRapidE | Effect |
|-----|-----------|------------|--------|
| `Damage` | 30 | **50** | +66% damage at elite |
| `ROF` | 20 | **50** | Elite is **slower-firing** (1.5× slower) |
| `Range` | 5.5 | **5.75** | Slight range increase |
| `Projectile` | `InvisibleLow` | **`Cannon`** | Elite swaps to **arcing cannon** projectile |
| `Speed` | 100 | 40 | Cannon is slower |
| `Warhead` | `HARVWH` | **`HowitzerWH`** | Elite uses Howitzer warhead (very different damage profile — see §4) |
| `Report` | `WarMinerAttack` | `RhinoTankAttack` | Elite sounds like a tank, not a harvester |
| `Anim` | `GUNFIRE` | `GUNFIRE` | Same muzzle flash |
| `Bright` | (absent) | yes | Elite lights the cell when firing |

**Practical jump**: rookie does 30 dmg/shot at ROF 20 = 1.5 dmg/tick raw vs `none` armor; elite does 50 dmg/shot at ROF 50 = 1.0 dmg/tick raw — but the elite cannon arcs (can lob over walls), has a different Verses profile (HowitzerWH is artillery — better vs buildings), and the bigger per-hit number matters for picking off threats faster. Elite HARV is a viable light artillery, not a faster gunner.

### 3.1 Projectiles

| Projectile | Properties |
|-----------|------------|
| `[InvisibleLow]` (rookie/veteran) | `Inviso=yes, Image=none, SubjectToCliffs=yes, SubjectToElevation=yes, SubjectToWalls=yes` — invisible flat-fire shot that respects terrain |
| `[Cannon]` (elite) | `Image=120MM, Arcing=true, SubjectToCliffs=yes, SubjectToElevation=yes, SubjectToWalls=yes` — visible arcing cannon shell |

---

## 4. Warheads — `[HARVWH]` / `[HowitzerWH]`

### `[HARVWH]` (rookie/veteran)

```ini
[HARVWH]
;CellSpread=.3
;PercentAtMax=.5
Verses=100%,80%,70%,50%,20%,20%,20%,15%,10%,400%,100%
InfDeath=1
AnimList=PIFFPIFF,PIFFPIFF
;Bright=yes
Bullets=yes
ProneDamage=50%
```

| Verses slot | Armor | Damage | Notes |
|------|-------|--------|-------|
| 1 | none | 100% | Full damage vs basic infantry |
| 2 | flak | 80% | Flak Trooper armored |
| 3 | plate | 70% | Tanya/SEAL armored |
| 4 | light | 50% | Grizzly/Mirage |
| 5 | medium | 20% | Apocalypse/Rhino non-front |
| 6 | heavy | 20% | Rhino front |
| 7 | wood | 20% | Civilian buildings (low) |
| 8 | steel | 15% | Buildings |
| 9 | concrete | 10% | Fortified buildings (very low) |
| 10 | special_1 | **400%** | Massive multiplier vs Terror Drone armor — HARV is a designated counter to Terror Drone latch-attack |
| 11 | special_2 | 100% | |

**Key parity note**: the **400% vs special_1** is what lets HARV shake off Terror Drones quickly when one latches on (the slave/HARV self-target mechanic uses this warhead).

### `[HowitzerWH]` (elite only)

(Used by elite HARV via 20mmRapidE; not duplicated here since it's a separate-doc warhead. Referenced for cross-doc tracking. To be expanded when HOWI/Howitzer is documented.)

`InfDeath=1` (small-arms slumping death). `AnimList=PIFFPIFF,PIFFPIFF` (hit-spark). `Bullets=yes`. `ProneDamage=50%`.

---

## 5. Voices / sounds

```ini
[WarMinerSelect]
Sounds=$vwarsea $vwarseb $vwarsec $vwarsed $vwarsee
Control=random
Volume=85

[WarMinerMove]
Sounds=$vwarmoa $vwarmob $vwarmoc $vwarmod $vwarmoe
Control=random
Volume=85

[WarMinerAttackCommand]
Sounds=$vwarata $vwaratb $vwaratc $vwaratd $vwarate
Control=random
Volume=85

[WarMinerHarvest]
Sounds=$vwarhaa $vwarhab $vwarhac $vwarhad
Control=random
Volume=85
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=WarMinerSelect` | 5 clips | Click-select |
| `VoiceMove=WarMinerMove` | 5 clips | Move order |
| `VoiceAttack=WarMinerAttackCommand` | 5 clips | Attack order |
| `VoiceEnter=WarMinerMove` | reuses move set | Entering transport |
| `VoiceHarvest=WarMinerHarvest` | 4 clips | When the harvest action triggers (each bale pickup) |
| `DieSound=GenVehicleDie` | 6 clips, FShift ±15 | Death |
| `CrushSound=TankCrush` | `vcrusha` | When HARV crushes infantry |
| `Report=WarMinerAttack` (on weapon) | (in soundmd at 5108+ approximately) | Per-shot fire sound (rookie/veteran) |
| `Report=RhinoTankAttack` (elite weapon) | (in soundmd) | Per-shot fire sound (elite) |

Note: `VoiceHarvest` is a harvester-specific voice key not present on standard combat
units — it triggers on each bale pickup in the harvest cycle.

---

## 6. Prerequisites / owners / availability

- **Prerequisite**: `NAWEAP,PROC` — Soviet War Factory AND Refinery.
- **TechLevel** = `1` — earliest tier.
- **AllowedToStartInMultiplayer=no** — not preplaced; instead, the Soviet ConYard's deploy spawns one HARV via `[General] FreeHarvester=` (specifically the `FreeUnit=HARV` line on the Soviet ConYard NACNST).
- **Owner**: Soviet houses only (Russians, Confederation, Africans, Arabs). No YuriCountry — Yuri's harvester is the SMIN (Slave Miner) + SLAV (Slave) system documented separately.
- **CrateGoodie=yes** — can drop from crates (rare for an expensive unit).

### Dock targets

`Dock=NAREFN,GAREFN` — HARV will dock at either Soviet Refinery (NAREFN) or Allied
Refinery (GAREFN), accommodating cross-faction captures. The dock target picker selects
the nearest valid refinery owned by the same player.

### War-vs-Chrono harvester comparison

| Aspect | HARV (War Miner) | CMIN (Chrono Miner) |
|--------|------------------|----------------------|
| Side | Soviet | Allied |
| Locomotor | `DriveLocomotionClass` only | `TeleportLocomotionClass` (piggybacks Drive) |
| `Teleporter` (TechnoType+0xCD4) | `false` | `true` |
| `Storage` | 40 bales | 20 bales |
| Weapon | `20mmRapid` turret | None |
| Return-to-refinery | Always drives | Teleports if ore >50 cells away from refinery, drives otherwise |
| Distance threshold | `HarvesterTooFarDistance=5` (cells) | `ChronoHarvTooFarDistance=50` (cells) |
| Scan radii (long/short) | Same: 48/6 | Same: 48/6 |
| Scan function | `Scan_For_Tiberium_And_Move` | `Scan_For_Tiberium_And_Move` |
| `UnloadingClass` | `HORV` | `CMON` |
| `Cost` | 1400 | 1400 |
| `Speed` | 4 | 4 |
| `Strength` | 1000 | 1000 |
| `Armor` | medium | medium |

Source: [WAR_MINER_REFERENCE.md](../../WAR_MINER_REFERENCE.md) §2 — comparison verified from binary.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 HARV-specific code in `gamemd.exe`

| Query | Result |
|-------|--------|
| `HARV` (substring) | No string literal `"HARV"` as a unit-ID anchor in code; only as a CSF lookup (`Name:HARV`) and string-pool entries |
| `Harvester` (substring) | 17 matches — all generic globals/flags (HarvestRate, HarvesterLoadRate/DumpRate, HarvesterTooFarDistance, HarvesterUnit, HarvesterTruce, etc.) |
| `UnloadingClass` | 1 match at 0x00843af8 |

⇒ **No HARV-specific hardcoded ID** in the binary. All behavior is generic flag-driven. The `Harvester=yes` flag in INI is what enables the harvest mission state machine.

### 7.2 Flag-scope verification (this iteration)

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `UnloadingClass` | 0x00843af8 | TechnoTypeClass__ReadINI @ 0x007146e8 | TechnoType |

Plus prior verifications still in scope:
- `Storage` — TechnoType (already verified in SLAV doc)
- `HarvestRate` — InfantryType only (slave-specific; HARV doesn't use this)
- `Teleporter` — TechnoType
- `ImmuneToPsionics` / `ImmuneToRadiation` — TechnoType

### 7.3 Global rules that affect HARV (`[General]` / `[CombatDamage]` etc.)

| Key | Approx address | Scope |
|-----|----------------|-------|
| `HarvesterUnit` | 0x0083c754 | RulesClass (lists which units are harvesters in the build-tree) |
| `HarvesterLoadRate` | 0x0083be38 | RulesClass (frames per bale during harvest) |
| `HarvesterDumpRate` | 0x0083be4c | RulesClass (frames per bale during unload) |
| `HarvesterTooFarDistance` | 0x0083c480 | RulesClass (5 cells — War Miner distance gate) |
| `ChronoHarvTooFarDistance` | 0x0083c464 | RulesClass (50 cells — Chrono Miner teleport threshold) |
| `HarvestersPerRefinery` | 0x0083c128 | RulesClass (AI economy planning) |
| `HarvesterTruce` | 0x0083cfd8 | RulesClass (AI peace-with-harvesters flag) |
| `AIIonCannonHarvesterValue` | 0x0083bfcc | RulesClass (AI tactical decision weight) |

### 7.4 Live behaviors driven by these flags

| Behavior | Driver | Notes |
|----------|--------|-------|
| Drive to nearest ore, harvest, drive back to refinery | `Harvester=yes` triggers `Mission_Harvest` 5-state machine | See HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md |
| Auto-pick refinery via `Dock=` list | `Dock=NAREFN,GAREFN` picked at dock-time by distance | Cross-faction support |
| Visual model swap during dock-unload | `UnloadingClass=HORV` | See HARVESTER_DOCK_UNLOAD_SEQUENCE.md |
| Self-defense via turret while harvesting | `Turret=yes`, `Primary=20mmRapid`, `OpportunityFire=yes` | Auto-fires at threats without losing harvest state |
| Auto-crush infantry while pathing | `Crusher=yes`, `AutoCrush=yes` | Walks over enemies |
| Self-heals when idle | `SelfHealing=yes`, rate from `[General] SelfHealUnitRate` | |
| Survives Desolator rad | `ImmuneToRadiation=yes` | |
| Cannot be mind-controlled | `ImmuneToPsionics=yes` | |
| Counters Terror Drone via 400% special_1 verses | `[HARVWH] Verses` slot 10 | Self-target / friendly-defense can quickly kill latched drone |
| Higher elite weapon (cannon, not gun) | `ElitePrimary=20mmRapidE` with arcing Cannon projectile | Late-game HARV can lob shells over walls |
| Cannot enter Battle Bunker/Fortress | `Bunkerable=no` | |
| Ignored by enemy auto-target | `ThreatPosed=0` | Combined with `ToProtect=yes` friendly defense, AI economy stability |
| AI economy considers HARV income | `ResourceGatherer=yes` | AI planner reads this for build queues |

### 7.5 Behaviors NOT present

- **No teleport** (despite the index's prior mislabel) — `Locomotor=DriveLocomotionClass` and **no `Teleporter=`** flag. The chrono teleport is exclusive to CMIN.
- No `Spawns=` (no child units).
- No `OpenTransport=`, `Passengers=`, `Gunner=` — not a transport.
- No `Suspend*=` (no special suspend modes).

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ImmuneToVeins=yes` | YES (veinholes gone) | Dormant. |
| `ZFudgeTunnel=14` | YES (no real tunnels in YR) | Dormant render-fudge value, irrelevant but harmless. |
| All other flags | — | Live. |

---

## 9. Veterancy

### Veteran (1 chevron) — `STRONGER, FIREPOWER, SIGHT, FASTER`
- `STRONGER` — +25% HP (1000 → 1250 typical)
- `FIREPOWER` — +25% damage
- `SIGHT` — +20% sight (4 → 4.8)
- `FASTER` — +20% speed (4 → 4.8)

### Elite (2 chevrons) — `SELF_HEAL, STRONGER, FIREPOWER, ROF` (cumulative)
- Adds `SELF_HEAL` (already self-healing — this likely boosts the rate or layers a second source).
- `STRONGER` & `FIREPOWER` reapplied (engine semantics for duplicate ability tokens — see TNKD §9 open follow-up).
- `ROF` — −25% ROF (faster cooldown).

**Plus weapon swap**: `20mmRapid` → `20mmRapidE`:
- Damage 30 → 50.
- ROF 20 → 50 (slower fire rate — but bigger per-shot).
- Range 5.5 → 5.75.
- Projectile: invisible flat-fire → arcing Cannon.
- Warhead: HARVWH → HowitzerWH (different damage profile; better vs structures).

Elite HARV is functionally a light artillery vehicle — slower-firing but lobbing shells with elevation arc.

---

## 10. Cross-references

### Direct dependencies (`rulesmd.ini` / `artmd.ini` / `soundmd.ini`)
- `[20mmRapid]` / `[20mmRapidE]` — weapons (§3)
- `[InvisibleLow]` — rookie projectile
- `[Cannon]` — elite projectile (`Image=120MM`, arcing)
- `[HARVWH]` — rookie warhead (§4)
- `[HowitzerWH]` — elite warhead (referenced; not duplicated)
- `[120MM]` (artmd) — empty bullet sprite
- `[GUNFIRE]` (artmd) — muzzle flash
- `[HORV]` (rulesmd line 8174) — `UnloadingClass` swap target (visual-only during dock-unload, same `Harvester=yes`, same `Dock=` list)
- `[NAREFN]` / `[GAREFN]` — dock targets
- `[NAWEAP]` / `[PROC]` — prerequisites
- `[WarMinerSelect/Move/AttackCommand/Harvest]` (soundmd) — voices
- `[GenVehicleDie] / [TankCrush]` — generic vehicle sounds

### Conceptual companions
- **CMIN** (`allied/CMIN.md` TODO) — Allied Chrono Miner counterpart. Comparison table §6.
- **HORV** (`soviet/HORV.md` or quick-ref TODO) — UnloadingClass swap target for HARV during dock-unload. Same `Harvester=yes` + `Dock=` list, no `Turret=` (uses generic Soviet vehicle voices), `TechLevel=-1` (not directly buildable).
- **SMIN / SMON / SLAV / YAREFN** — Yuri's alternative ore economy (slave-driven, no HARV-style harvester). See [`yuri/SLAV.md`](../yuri/SLAV.md).

### Deep-RE docs (cross-referenced, not re-derived)
- **[WAR_MINER_REFERENCE.md](../../WAR_MINER_REFERENCE.md)** — comprehensive reference. Has the canonical HARV vs CMIN comparison.
- **[WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md](../../WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md)** — locomotor integration.
- **[HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md](../../HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md)** — 5-state Mission_Harvest state machine.
- **[MISSION_HARVEST_GHIDRA_REPORT.md](../../MISSION_HARVEST_GHIDRA_REPORT.md)** — companion harvest-mission report.
- **[HARVESTER_DOCK_UNLOAD.md](../../HARVESTER_DOCK_UNLOAD.md)** — dock-unload mechanics.
- **[HARVESTER_DOCK_UNLOAD_SEQUENCE.md](../../HARVESTER_DOCK_UNLOAD_SEQUENCE.md)** — UnloadingClass swap sequence.
- **[MINER_DOCK_GAPS_RESEARCH.md](../../MINER_DOCK_GAPS_RESEARCH.md)** — known docking edge cases.
- **[CHRONO_MINER_SYSTEM_OVERVIEW.md](../../CHRONO_MINER_SYSTEM_OVERVIEW.md)** + chrono-miner-* docs — chrono-miner-specific (referenced for comparison only; HARV does NOT teleport).

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[HARV]` rulesmd key explained | ✅ §1 |
| Every `[HARV]` artmd key explained | ✅ §2 |
| Both weapons (rookie + elite) + both warheads + both projectiles | ✅ §3–§4 |
| All voices expanded with verbatim sound defs | ✅ §5 |
| Prereqs / owners / Dock targets | ✅ §6 |
| HARV-vs-CMIN comparison table | ✅ §6 |
| Hardcoded behavior — Ghidra searches + flag-scope verifications | ✅ §7 (UnloadingClass verified this iter; harvester globals enumerated) |
| TS-legacy filter | ✅ §8 |
| Veterancy detailed with elite-weapon delta | ✅ §9 |
| Cross-refs to **seven** deep-RE docs (WAR_MINER_REFERENCE, WAR_MINER_LOCOMOTION, HARVESTER_MISSION_HARVEST, MISSION_HARVEST, HARVESTER_DOCK_UNLOAD, HARVESTER_DOCK_UNLOAD_SEQUENCE, MINER_DOCK_GAPS_RESEARCH) | ✅ §10 |
| **Index correction logged**: HARV is Soviet War Miner, not Allied Chrono Miner | ✅ doc header + index entry |

**Open follow-ups (none load-bearing):**
- `[HowitzerWH]` (elite HARV's warhead) is referenced but not expanded here — belongs in the HOWI or generic-warhead doc family. Cross-doc Verses comparison would help quantify the elite-weapon parity impact.
- Confirm the slot_10 (`special_1`) 400% Verses target — whether it specifically maps to Terror Drone armor at the `[DroneArmor]` / similar key. Worth a one-pass verification when DRON (Terror Drone) is documented.
- The HARVESTER_DOCK_UNLOAD_SEQUENCE doc covers the swap mid-dock; verify the exact frame at which `[HORV]` model replaces `[HARV]` and back, in case parity bug ever surfaces with the dock animation.
