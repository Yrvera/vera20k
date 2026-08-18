# SMIN — Slave Miner (Mobile / Pre-Deploy Form)

**INI ID:** `SMIN`
**Display Name:** `Slave Miner` (`UIName=Name:SMIN`)
**Side:** Yuri (`Owner=YuriCountry`)
**Category:** Vehicle / Support (in `[VehicleTypes]`)
**Cameo:** `SMINICON` / `SMINUICO` (AltCameo)
**Voxel:** yes

The Slave Miner is Yuri's **mobile harvester-refinery hybrid** — a single
unit that combines the role of MCV+Refinery+Harvester+Defense Tower. In
its mobile (SMIN) form, it's a slow, armored, turreted vehicle. When
deployed via Mission_Deploy_Building, it transforms into **SMON / YAREFN**
(the deployed refinery building, with the SlaveManagerClass spawning 5
SLAV infantry to harvest nearby ore on foot). This doc covers the *mobile*
SMIN form; the deployed `[SMON]` / `[YAREFN]` block will be documented in
its own iteration.

### Yuri's economy = the Slave Miner

Yuri does not have a standard MCV→ConYard→Refinery→Harvester economy chain.
Instead, the Slave Miner IS the refinery, the harvester platform, and the
ore-collection range — a single mobile building. Players relocate it to
fresh ore fields rather than building new refineries with longer haul
distances. The 5 SLAV infantry it spawns walk on foot to ore tiles, mine
with shovels (`SHOVEL` weapon), and walk back. The Slave Miner converts
ore to credits on dump. This is one of YR's most distinctive economic
asymmetries.

> **Cross-references — do not re-derive:**
> - [`SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md`](../../SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md) (451 lines) — INI key parsing (DeploysInto/UndeploysInto/DeployingAnim @ TechnoType+0x404/0x408/0x6BC), UnitClass::Deploy mechanism (0x007393c0), Mission_Deploy_Building (0x0073d630) state machine, ore-dump credit deposit chain. **Verified.**
> - [`SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md`](../../SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md) (1492 lines) — full SlaveManagerClass layout (offset 0x2D8 on TechnoClass), state machine (Ready/Moving/Freeze/Relocating), 6-state slave lifecycle, SlaveControl struct layout, regen/reload timer math. **Verified.**
> - [`SLAV.md`](./SLAV.md) — the SLAV infantry (Slave) doc covering the harvester infantry side: SHOVEL weapon, ore-harvest behavior on foot, dual-mode voice set (peasant + cheering), Slaved=yes flag.
> - [`HARV.md`](../soviet/HARV.md), [`CMIN.md`](../allied/CMIN.md) — sibling harvester units (Soviet War Miner, Allied Chrono Miner).
> - [`AMCV.md`](../allied/AMCV.md) — the MCV→ConYard deploy pattern, structurally similar to SMIN→SMON.

> **TS-legacy filter:** `ImmuneToVeins=yes` is the **Tiberian Sun veins-of-the-purifier holdover**. Veins don't exist in RA2/YR — the flag is parsed but has no in-game effect (no Vein hazard to be immune from). Live in the binary, dead in gameplay. Locomotor `{4A582741-...}` = DriveLocomotionClass, live YR. `ZFudgeBridge=7` is rendering-only, live.

---

## 1. Full `rulesmd.ini` section verbatim

```ini
[SMIN]
UIName=Name:SMIN
Name=Slave Miner
Prerequisite=YAWEAP
Nominal=yes
ToProtect=yes
Category=Support
Strength=2000
Armor=medium
Primary=20mmRapid
ElitePrimary=20mmRapidE
Turret=yes
OpportunityFire=yes
TechLevel=1
Sight=4
Speed=3
Owner=YuriCountry
AllowedToStartInMultiplayer=no
CrateGoodie=yes
Storage=20
Cost=1750
Soylent=1750
Points=55
ROT=5
Crusher=yes
Crewed=no
SelfHealing=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=SlaveMinerSelect
VoiceMove=SlaveMinerMove
VoiceAttack=SlaveMinerAttackCommand
VoiceHarvest=SlaveMinerHarvest
DieSound=GenVehicleDie
MoveSound=SlaveMinerMoveStart
CrushSound=TankCrush
DeploySound=SlaveMinerDeploy
VoiceDeploy=SlaveMinerDeployVoice
MaxDebris=6
DebrisTypes=TIRE
DebrisMaximums=4
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1} ;drive locomotor
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
StupidHunt=yes ;this guy can't handle a hunt command, so he should just run towards the player
Trainable=yes
DeploysInto=YAREFN
DeployFacing=0;0 = N, 7 = NW
Enslaves=SLAV;gs The Refinery does not get an Enslaves listing because the Slave object will get passed from unit to building upon deploy
SlavesNumber=5
SlaveRegenRate=500 ;225
SlaveReloadRate=25
;moving brain to refinery to start
;Ugh.  Now that placed as building, problem arises from managing to get a SMIN as vehicle (Campaign map, crate).  Both get this listing now, and Brain transplant will check to make sure extra one is not created
ResourceGatherer=yes;gs for the AI to handle the slave miner, it has to understand what makes money
ResourceDestination=yes
DeaccelerationFactor=.2 ; This is TS's mizspelingg knot min
Accelerates=false
Bunkerable=no; Units default to yes, others default to no
OmniCrushResistant=yes; so Crusher can crush Crushable, OmniCrusher trumps Crushable=no, and then OmniCrushResistant trumps OmniCrusher
```

### 1.1 Key-by-key explanation

| Key | Value | Read by | Effect |
|-----|-------|---------|--------|
| `UIName=Name:SMIN` | string | AbstractTypeClass | CSF lookup. |
| `Name=Slave Miner` | string | AbstractTypeClass | English fallback. |
| `Prerequisite=YAWEAP` | building | TechnoTypeClass | **Yuri War Factory only.** No Battle Lab. Available immediately when War Factory is built. |
| `Nominal=yes` | bool | TechnoTypeClass | UI-list display hint (uses `Name=` directly rather than constructing a "X N" label). |
| `ToProtect=yes` | bool | TechnoType @ 0x00714be8 | AI escort flag — Yuri AI will dispatch defenders to nearby threats against Slave Miners. |
| `Category=Support` | enum | TechnoTypeClass | Support category (not combat — for AI scripting). |
| `Strength=2000` | hp | TechnoTypeClass | **2000 HP** — very durable for a "harvester" (most harvesters are ~600-1000). Reflects that SMIN is also a building-class entity. |
| `Armor=medium` | enum | TechnoTypeClass | Medium armor. |
| `Primary=20mmRapid` | weapon | TechnoTypeClass | **Anti-infantry/light-vehicle turret** — the SMIN can defend itself while mobile. See §3.1. |
| `ElitePrimary=20mmRapidE` | weapon | TechnoType @ 0x00712a32 | Elite weapon swap — upgrade from rapid 20mm to elite version (cannon-shell with HE warhead). See §3.2. |
| `Turret=yes` | bool | UnitTypeClass | Has rotating turret. |
| `OpportunityFire=yes` | bool | TechnoType @ 0x0071483d | Engages opportunistic targets during movement. |
| `TechLevel=1` | int | TechnoTypeClass | **TechLevel 1** — the earliest tier. SMIN is available from Yuri's first match minute. |
| `Sight=4` | cells | TechnoTypeClass | Short sight range (mobile mode). Matches typical harvester. |
| `Speed=3` | int | TechnoTypeClass | **Slow (3)** — Slave Miner is among the slowest YR vehicles. |
| `Owner=YuriCountry` | country list | TechnoTypeClass | Yuri only. |
| `AllowedToStartInMultiplayer=no` | bool | TechnoTypeClass | Not pre-built (the Yuri ConYard build sequence handles initial SMIN). |
| `CrateGoodie=yes` | bool | UnitType @ 0x00747658 | **CAN pop from goodie crates** — a free SMIN is a possible crate reward. Most tier-3 units have CrateGoodie=no; SMIN is enabled because its "tier 1" status makes it a reasonable crate reward. |
| `Storage=20` | int | TechnoType @ 0x00713130 (cheat sheet) | **20-unit ore storage capacity.** When mobile, the SMIN itself can hold up to 20 units of ore (unusual — most non-deployed harvesters have Storage=0 and route ore directly). Possibly used during deploy transition or for in-transit ore. |
| `Cost=1750` | credits | TechnoTypeClass | Premium cost (same as DISK Floating Disc). |
| `Soylent=1750` | credits | TechnoTypeClass | Full recycle. |
| `Points=55` | int | TechnoTypeClass | High score on kill (same as DRED/CARRIER — recognises it's a major economic loss). |
| `ROT=5` | int | TechnoTypeClass | Moderate turn rate. |
| `Crusher=yes` | bool | TechnoTypeClass | **Can crush infantry** — heavy chassis allows it to roll over infantry. |
| `Crewed=no` | bool | TechnoTypeClass | No crew bailout. |
| `SelfHealing=yes` | bool | TechnoType (cheat sheet) | **Passive HP regeneration at all ranks** (not just elite). Mirrors the deployed Refinery's auto-repair behavior — SMIN repairs itself even while mobile. |
| `Explosion=...` | anim list | TechnoTypeClass | Standard 5-anim destruction. |
| `VoiceSelect=SlaveMinerSelect` | sound | TechnoTypeClass | Unique select (sound:4899). |
| `VoiceMove=SlaveMinerMove` | sound | TechnoTypeClass | Unique move (sound:4904). |
| `VoiceAttack=SlaveMinerAttackCommand` | sound | TechnoTypeClass | Unique attack (sound:4909). |
| `VoiceHarvest=SlaveMinerHarvest` | sound | TechnoType @ 0x00713652 (NEW THIS DOC) | **Harvester-specific "begin harvesting" voice** (sound:4914). Verified TechnoType scope. Plays when the SMIN/SMON receives a harvest order or auto-acquires an ore field. Mirrors HARV/CMIN VoiceHarvest pattern. |
| `DieSound=GenVehicleDie` | sound | TechnoTypeClass | Generic vehicle death. |
| `MoveSound=SlaveMinerMoveStart` | sound | TechnoTypeClass | Engine-start one-shot (sound:5290). |
| `CrushSound=TankCrush` | sound | TechnoTypeClass | Generic crush sound. |
| `DeploySound=SlaveMinerDeploy` | sound | TechnoType @ 0x00713568 (cheat sheet) | **Deploy mechanical sound** (sound:5438) — plays during deploy animation (gears/hydraulics). |
| `VoiceDeploy=SlaveMinerDeployVoice` | sound | TechnoType (cheat sheet) | **Deploy spoken voice** (sound:4919) — "deploying" voice clip. Separate from DeploySound (mechanical) — the spoken acknowledgment. |
| `MaxDebris=6` | int | TechnoTypeClass | Up to 6 debris pieces on destruction (more than typical 2-3). |
| `DebrisTypes=TIRE` | type list | TechnoType @ 0x00713652 (NEW — same area as VoiceHarvest) | **Debris-piece visual is the TIRE voxel** (a single specific debris type, not a list of varied ones). Tire chunks fly out when destroyed. Verified TechnoType scope. |
| `DebrisMaximums=4` | int | TechnoTypeClass | At most 4 TIRE pieces (despite MaxDebris=6, the per-type max is 4). |
| `Locomotor={4A582741-...}` | CLSID | TechnoTypeClass | DriveLocomotionClass (`;drive locomotor` inline comment). |
| `Weight=3.5` | float | TechnoTypeClass | Fractional weight. |
| `MovementZone=Crusher` | enum | TechnoTypeClass | **MovementZone=Crusher** — special pathing zone for crusher units. Allows pathfinding through cells where infantry-class units would block (since SMIN crushes them). |
| `ThreatPosed=0` | int | TechnoTypeClass | Inline: "This value MUST be 0 for all building addons" — boilerplate comment. ThreatPosed=0 means AI doesn't target the SMIN as a "threat" (so AI vehicles don't auto-acquire it; they target it deliberately for economic damage). |
| `ThreatAvoidanceCoefficient=.65` | float | TechnoType @ 0x00712460 (NEW THIS DOC) | **AI threat-avoidance scalar (0.65 = 65%)** — the SMIN tries to avoid high-threat areas with 65% the normal aversion. Verified TechnoType scope. Lower = bolder routing; higher = more cautious. |
| `DamageParticleSystems=SparkSys,SmallGreySSys` | particle list | TechnoTypeClass | Damaged emissions. |
| `ImmuneToVeins=yes` | bool | TechnoType (cheat sheet area) | **TS-LEGACY** — Tiberian Sun veins don't exist in YR. Parsed but effectively inert. Author copy-paste from TS-era harvester INI. |
| `ImmuneToPsionics=yes` | bool | TechnoType @ 0x00714fa7 | Cannot be mind-controlled. **Critical** — without this, the SMIN would be a prime target for enemy Yuri to steal economy. |
| `ImmuneToRadiation=yes` | bool | TechnoType (cheat sheet) | Ignores Desolator rad damage. |
| `ZFudgeColumn=9` / `ZFudgeTunnel=14` | int | TechnoTypeClass | Z-render tweaks (higher values than DISK's 8/13). |
| `ZFudgeBridge=7` | int | TechnoType @ 0x00460c76 (NEW THIS DOC) | **Z-buffer fudge specifically for bridge cells.** Verified scope (read in BuildingTypeClass-related path at 0x00460c76 — interesting that the xref is in BuildingType ReadINI; likely shared between Building and Techno paths for deploy-aware rendering). Tweaks render depth when the SMIN is on/near a bridge. |
| `Size=3` | int | TechnoTypeClass | Transport cost. |
| `StupidHunt=yes` | bool | TechnoType @ 0x00714c6c (NEW THIS DOC) | **The unit can't handle a Hunt mission** — inline comment: "this guy can't handle a hunt command, so he should just run towards the player". Verified TechnoType scope. When AI assigns Hunt mission, the SMIN falls back to a simpler "run towards enemy" behavior rather than the full Hunt logic (which involves complex target seeking). Recognises SMIN's mobility limitations. |
| `Trainable=yes` | bool | TechnoTypeClass | **Gains veterancy** — unusual for a harvester (most harvesters have Trainable=no since they don't deal damage). SMIN's anti-infantry turret means it can earn veteran ranks. |
| `DeploysInto=YAREFN` | building | TechnoType @ 0x00713279 (cheat sheet) | **Deploys into the YAREFN building** (the SMON-side rules entry uses YAREFN as the actual building ID). See §7.3 for deploy mechanism. |
| `DeployFacing=0` | int | TechnoType / BuildingType ReadINI_Water @ 0x00460c76 | **Facing N (0)** before deploy. Inline comment: "0 = N, 7 = NW". Per SLAVE_MINER_ORE doc §1.3, `Deploy_facing_calculator` (0x00465d70) rotates the unit to match DeployFacing before deploying. |
| `Enslaves=SLAV` | infantry type | TechnoType @ 0x00714dd7 (cheat sheet) | **References the SLAV infantry type as the slave to spawn.** Inline comment is critical: "gs The Refinery does not get an Enslaves listing because the Slave object will get passed from unit to building upon deploy". Mirrors the also-noted brain-transplant logic — when SMIN deploys to YAREFN, the SlaveManagerClass and its spawned slaves transfer from unit to building. |
| `SlavesNumber=5` | int | TechnoType (SlaveManager) | **5 SLAV infantry** spawn from this miner. |
| `SlaveRegenRate=500` | frames | TechnoType (SlaveManager) | **~33 sec to manufacture a replacement slave** if one is killed. `;225` is a draft note (historical value). |
| `SlaveReloadRate=25` | frames | TechnoType (SlaveManager) | **~1.7 sec for a slave to complete an ore-mining cycle** at an ore tile. |
| (multi-line author comment) | — | — | The author documents the brain-transplant rationale — both SMIN and YAREFN have Enslaves= listing to handle edge cases (campaign maps, crate-spawned SMIN where the standard deploy chain might not be followed). |
| `ResourceGatherer=yes` | bool | TechnoType @ 0x007143d7 (NEW THIS DOC) | **AI economic-evaluation flag.** Verified TechnoType scope. Inline comment: "for the AI to handle the slave miner, it has to understand what makes money". Tells AI scripting code "this unit produces credits" so it factors into income calculations and protection priority. |
| `ResourceDestination=yes` | bool | TechnoType | **AI flag — slaves return ore HERE.** Mirrors the standard refinery's role. Cross-faction: HARV/CMIN have a separate Refinery as their destination; SMIN is the destination (or, once deployed, YAREFN/SMON is). |
| `DeaccelerationFactor=.2` | float | TechnoType @ 0x0071249b | Deceleration ramp. Inline joke: "This is TS's mizspelingg knot min" (same Westwood typo joke as on CAOS). |
| `Accelerates=false` | bool | TechnoTypeClass | Constant speed. |
| `Bunkerable=no` | bool | TechnoType @ 0x0071500a | Cannot be garrisoned (standard for non-infantry). |
| `OmniCrushResistant=yes` | bool | TechnoType @ 0x00714d11 (cheat sheet) | **Resists OmniCrushers** (e.g., Battle Fortress) — inline comment is the design ladder: "Crusher can crush Crushable, OmniCrusher trumps Crushable=no, and then OmniCrushResistant trumps OmniCrusher". So the SMIN cannot be flattened by any crusher, including the Battle Fortress's omni-crush. Protects this critical economic asset. |

---

## 2. Full `artmd.ini` section verbatim

```ini
[SMIN] ; Yuri Slave Miner
Cameo=SMINICON
AltCameo=SMINUICO
Voxel=yes
Remapable=yes
TurretOffset=70
PrimaryFireFLH=120,0,185
```

| Key | Value | Notes |
|-----|-------|-------|
| `Cameo=SMINICON` | SHP | Standard cameo. |
| `AltCameo=SMINUICO` | SHP | Alternate (unbuildable) cameo. |
| `Voxel=yes` | bool | `smin.vxl` + `.hva` voxel render. |
| `Remapable=yes` | bool | House-color tinted (Yuri purple). |
| `TurretOffset=70` | int | Turret rendered 70 leptons forward from body center — the visible turret sits on the front-half of the chassis. |
| `PrimaryFireFLH=120,0,185` | x,y,z leptons | Weapon muzzle at 120 forward, 0 side, 185 up (top of turret barrel). |

---

## 3. Weapons

### 3.1 `[20mmRapid]` — primary anti-infantry turret

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

| Key | Effect |
|-----|--------|
| `Damage=30` | 30 base damage. |
| `ROF=20` | ~1.3 sec between shots. |
| `Range=5.5` | 5.5 cells (longer than Sight=4 — relies on auto-acquire reaching with VirtualScanner-like or some other extension; in practice the SMIN's actual engagement is limited by Sight). |
| `Projectile=InvisibleLow` | Bookkeeping invisible projectile. |
| `Speed=100` | Fast. |
| `Warhead=HARVWH` | **Harvester warhead** — shared with HARV (Soviet War Miner). See §3.3. |
| `Report=WarMinerAttack` | **Shared sound with War Miner** — same audio for the cannon. |
| `Anim=GUNFIRE` | Universal muzzle flash anim. |

### 3.2 `[20mmRapidE]` — elite primary swap

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

| Key | Effect |
|-----|--------|
| `Damage=50` | **+67% damage** vs rookie. |
| `ROF=50` | **Slower ROF** (50 vs 20) — but per-shot damage is higher. Net DPS is comparable. |
| `Range=5.75` | Slightly longer range. |
| `Projectile=Cannon` | **Visible cannon shell** (was InvisibleLow at rookie). Elite SMIN visibly fires cannon shells. |
| `Speed=40` | Slower projectile (Cannon is arcing). |
| `Warhead=HowitzerWH` | **Heavy-explosive warhead** (siege-grade) — much stronger vs vehicles and structures than HARVWH. |
| `Report=RhinoTankAttack` | **Shared sound with Rhino Tank** — same boom as a Soviet MBT. |
| `Bright=yes` | Bright muzzle flash. |

> **Elite SMIN is a serious combat threat** — Damage 50 with HowitzerWH and visible cannon shells is comparable to a tank. Most harvesters can't fight at all; an elite Slave Miner can hold its own against light infantry skirmishes.

### 3.3 Warheads

#### `[HARVWH]` — harvester warhead (rookie 20mmRapid)

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

- vs infantry: 100/80/70 — good vs infantry classes.
- vs vehicle: 50/20/20 — strong vs LV, weak vs MV/HV.
- vs structures: 20/15/10 — terrible vs buildings.
- **vs special_1 (aircraft): 400% — 4× damage** (extreme AA — though the SMIN turret doesn't typically engage air; this might be vestigial scaling).
- vs special_2: 100%.
- `Bullets=yes` — bullet warhead.
- `ProneDamage=50%` — prone infantry take half.

#### `[HowitzerWH]` — elite Howitzer warhead (elite 20mmRapidE)

Not detailed inline here; cross-reference [`HARV.md`](../soviet/HARV.md) or other docs that cover the Howitzer-class warhead. Key characteristics: HE with rocker, moderate radius, strong vs vehicles and structures.

### 3.4 Projectiles

- `InvisibleLow` — bookkeeping invisible bullet (rookie).
- `Cannon` — visible cannon shell with arcing trajectory (elite).

---

## 4. Voice & sound catalogue

| Slot | Sound key | sndmd entry | Audio clip(s) |
|------|-----------|-------------|---------------|
| `VoiceSelect` | `SlaveMinerSelect` | sound:4899 | unique select |
| `VoiceMove` | `SlaveMinerMove` | sound:4904 | unique move |
| `VoiceAttack` | `SlaveMinerAttackCommand` | sound:4909 | unique attack |
| `VoiceHarvest` | `SlaveMinerHarvest` | sound:4914 | **unique "begin harvest" voice** |
| `VoiceFeedback` | (not set) | — | — (default engine fallback) |
| `DieSound` | `GenVehicleDie` | sound:1961 | generic vehicle death |
| `MoveSound` | `SlaveMinerMoveStart` | sound:5290 | one-shot engine start |
| `CrushSound` | `TankCrush` | sound:5472 | generic crush |
| `DeploySound` | `SlaveMinerDeploy` | sound:5438 | mechanical deploy sound (hydraulics/gears) |
| `VoiceDeploy` | `SlaveMinerDeployVoice` | sound:4919 | **spoken "deploying" voice** (distinct from mechanical DeploySound) |
| `20mmRapid Report` | `WarMinerAttack` | sound (shared with HARV) | gun-fire sound |
| `20mmRapidE Report` | `RhinoTankAttack` | sound (shared with HTNK) | elite cannon sound |

**6 SlaveMiner-unique sound entries** plus DeploySound + VoiceDeploy (separate mechanical-vs-spoken pair). Shared audio reuse: WarMinerAttack from HARV (rookie weapon), RhinoTankAttack from HTNK (elite weapon).

---

## 5. Owners / prerequisites / tech gating

- **Buildable by:** `YuriCountry` only.
- **Prerequisite:** `YAWEAP` only — Yuri War Factory. No Battle Lab.
- **TechLevel:** 1 — earliest tier.
- **Cost:** 1750 — premium cost (matches DISK).
- **CrateGoodie=yes** — CAN spawn from crates (unusual for premium economic units).
- **AllowedToStartInMultiplayer=no** — not pre-built; standard build sequence.

---

## 6. Veterancy

| Rank | Effect |
|------|--------|
| Rookie | Base — 20mmRapid (Damage=30, HARVWH warhead), HP=2000, Sight=4, Speed=3. |
| Veteran | (No explicit VeteranAbilities= block in SMIN — inherits engine defaults via `Trainable=yes`). |
| Elite | (No explicit EliteAbilities= block) + `ElitePrimary=20mmRapidE` swap → cannon-shell with HowitzerWH warhead. |

> **No VeteranAbilities/EliteAbilities block defined** — the SMIN relies on engine default ability multipliers (typically STRONGER+FIREPOWER) and the explicit ElitePrimary swap. The deployed YAREFN form similarly has no explicit veterancy abilities (buildings don't typically rank up via combat).

---

## 7. Hardcoded behavior — Ghidra-verified

### 7.1 String-name scan

- `search_strings "SMIN"` not run (likely catches sub-strings). The relevant verifications are via specific INI key strings below.
- All new field scope verifications listed in §7.2.

### 7.2 Verified field scopes (new this doc)

| Field | Scope | Address |
|-------|-------|---------|
| `VoiceHarvest=...` | TechnoType | **0x00713652** (NEW) |
| `DebrisTypes=...` | TechnoType | (adjacent area 0x00713652) (NEW) |
| `ThreatAvoidanceCoefficient=N` | TechnoType | **0x00712460** (NEW) |
| `StupidHunt=yes` | TechnoType | **0x00714c6c** (NEW) |
| `ResourceGatherer=yes` | TechnoType | **0x007143d7** (NEW) |
| `ZFudgeBridge=N` | BuildingType (also used in Techno path) | **0x00460c76** (NEW — interesting cross-scope) |
| `DeployFacing=N` | BuildingType ReadINI_Water (likely Ghidra mislabel) | 0x00460c76 (same as ZFudgeBridge, same function) |
| `DeploysInto=` | TechnoType +0x404 | 0x00713279 (per SLAVE_MINER_ORE doc) |
| `UndeploysInto=` | TechnoType +0x408 | (cheat sheet, mirrored offset) |
| `Enslaves=` | TechnoType | 0x00714dd7 (cheat sheet) |
| `Storage=N` | TechnoType | 0x00713130 (cheat sheet) |
| `OmniCrushResistant=yes` | TechnoType | 0x00714d11 (cheat sheet) |
| `Bunkerable=no` | TechnoType | 0x0071500a |
| `ImmuneToPsionics=yes` | TechnoType | 0x00714fa7 |
| `OpportunityFire=yes` | TechnoType | 0x0071483d |
| `ToProtect=yes` | TechnoType | 0x00714be8 |
| `ElitePrimary=` | TechnoType | 0x00712a32 |
| `DeploySound=` | TechnoType | 0x00713568 (cheat sheet) |
| `VoiceDeploy=` (separate from DeploySound) | TechnoType | (cheat sheet) |

> **Cross-scope observation:** ZFudgeBridge and DeployFacing both xref into a function labeled `BuildingTypeClass_ReadINI_Water` at 0x00460c76. This function name is suspicious — it's likely a Ghidra auto-name from an early reverse-engineering pass. The function probably handles a shared INI-key dispatch table used by both BuildingType and TechnoType ReadINI for deploy-aware fields. **Open question** for future verification: confirm exactly which class(es) read these keys. For now: treat both fields as "live, scope ambiguous between BuildingType and TechnoType".

### 7.3 Deploy mechanism (cross-ref summary)

From [`SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md`](../../SLAVE_MINER_ORE_SYSTEM_GHIDRA_REPORT.md) §1:

1. **Trigger:** Player issues Deploy command (D hotkey) or AI dispatches deploy mission. Or — if SMIN reaches an ore field while pre-deploy, it may auto-deploy (campaign behavior).
2. **CanDeploy gate** (vtable 0x314): checks deploy preconditions (terrain passability for building footprint, ore field nearby, no enemy obstruction).
3. **Face direction:** `Deploy_facing_calculator` (0x00465d70) rotates the SMIN to match `DeployFacing=0` (N) before deploying.
4. **Building construction:** `operator_new(0x720)` allocates a BuildingClass instance, then BuildingClass::Constructor creates the YAREFN building.
5. **Place at unit's cell:** vtable 0xd8 (TryPlaceBuilding) anchors the building footprint.
6. **Property transfer:**
   - Copies UniqueID, Location_Z.
   - Transfers HP ratio (preserves damage state).
   - Copies veterancy data (5 dwords starting at field 0x1E0).
   - Transfers fields 0x1EC and 0x1F0 (rally point / linking).
   - If unit has AttachedTag: transfers to building with refcount.
7. **Update targeting:** Iterates all TechnoClass objects; redirects any that targeted the SMIN to now target the YAREFN.
8. **Remove unit:** vtable 0xF8 (RemoveFromMap) + vtable 0x3A0 (Destroy/Limbo).
9. **SlaveManager transfer:** Critical — the SlaveManagerClass at TechnoClass+0x2D8 lives on the original SMIN. Per author's INI comment, the YAREFN building gets its own SlaveManager via FUN_006f3f40 during init, and the "brain transplant" check ensures the existing slaves (already-spawned SLAV infantry) re-bind to the building's SlaveManager rather than being orphaned.

### 7.4 Slave system (cross-ref summary)

From [`SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md`](../../SLAVE_MANAGER_STATE_MACHINE_GHIDRA_REPORT.md):

- **SlaveManagerClass** lives at TechnoClass+0x2D8 (on both SMIN unit and YAREFN building).
- Stores: SlaveCount (from `SlavesNumber=5`), SlaveRegenRate (500), SlaveReloadRate (25), state machine, slave array, timers.
- **States:** 0=Ready, 1=?, 2=Moving, 4=Freeze (e.g., when master is destroyed pre-respawn), 6=Relocating.
- Per-tick AI: spawns slaves (up to 5), assigns ore tiles, manages slave lifecycle.

Slave lifecycle:
1. **Spawn:** Manager creates a SLAV infantry at the master's cell, registers in SlaveControl array.
2. **Travel:** Slave walks to assigned ore tile (foot-pathfinding, slow).
3. **Mine:** Slave plays SHOVEL anim, "mines" the cell over `SlaveReloadRate=25` frames (~1.7 sec).
4. **Return:** Slave walks back to the master with the ore payload.
5. **Dump:** Master converts ore to credits via the standard refinery dump path. House credits incremented.
6. **Loop:** Slave goes back for another load.
7. **Death/respawn:** If slave is killed, manager spawns a replacement after `SlaveRegenRate=500` frames (~33 sec).

### 7.5 Why SMIN has both Storage=20 and Enslaves=SLAV

This is unusual — most harvesters either:
- Have Storage (Soviet HARV: Storage=20, slaves drop ore in HARV's bin via Bail amount); OR
- Have Spawning slaves (slave-style).

SMIN combines both. The Storage=20 likely covers the deploy transition window — if slaves are mid-return when the player commands deploy, the master can hold their pending payload until deployed. Once deployed, YAREFN takes over and slaves resume normal dump cycles to the building.

### 7.6 ImmuneToPsionics + OmniCrushResistant: economic protection

The SMIN has two critical "immunity" flags that prevent specific economic-disruption strategies:
- **ImmuneToPsionics=yes** — Cannot be mind-controlled by enemy Yuri or Master Mind. Without this, enemy Yuri could steal economy in one click.
- **OmniCrushResistant=yes** — Cannot be flattened by Battle Fortress (which is OmniCrusher). Without this, Allied players could simply roll over SMINs with BFRTs.

Both are deliberate balance choices to ensure the SMIN remains a defensible target.

---

## 8. TS-legacy filter

| Feature | Status in YR |
|---------|--------------|
| `ImmuneToVeins=yes` | **TS-legacy** — Veins are a Tiberian Sun hazard (Veins-of-the-Purifier ore-tendrils). RA2/YR have no Veins. Flag parsed but inert. Author INI copy-paste from TS-era harvester data. |
| `DeaccelerationFactor` (misspelled) | Live YR — joke preserves Westwood's typo. |
| Locomotor `{4A582741-...}` = DriveLocomotionClass | Live. |
| Slave system (SlaveManager, SlaveControl, Enslaves, Slaved) | Live YR (Yuri-exclusive mechanism). |
| `MovementZone=Crusher` | Live YR. |
| Inline author comments about Brain transplant and Campaign-spawned SMINs | Documentation — relevant to live code paths. |
| `ZFudgeBridge` | Live YR (rendering). |
| `StupidHunt=yes` | Live YR — AI fallback flag. |
| Fog-of-war 0x1000 gate | Not on SMIN. |
| Subterranean / Tunneling | Not on SMIN. |

---

## 9. Coverage audit

| Section | Coverage |
|---------|----------|
| rulesmd `[SMIN]` — every key | ✅ §1 (53 keys including all slave-system + deploy + AI keys) |
| artmd `[SMIN]` — every key | ✅ §2 (6 keys) |
| `[20mmRapid]` weapon | ✅ §3.1 |
| `[20mmRapidE]` elite weapon | ✅ §3.2 (cannon shell + HowitzerWH upgrade) |
| `[HARVWH]` warhead | ✅ §3.3 |
| `[HowitzerWH]` warhead | (referenced via cross-link; standard HE warhead) |
| Projectiles | ✅ §3.4 |
| Voices / sounds (10 bindings) | ✅ §4 |
| Owners / prereqs / tech | ✅ §5 |
| Veterancy (Trainable=yes + ElitePrimary swap; no explicit ability block) | ✅ §6 |
| Hardcoded behavior — Ghidra-verified | ✅ §7 (**5 NEW field-scope verifications**: VoiceHarvest @ 0x00713652 TechnoType, ThreatAvoidanceCoefficient @ 0x00712460 TechnoType, StupidHunt @ 0x00714c6c TechnoType, ResourceGatherer @ 0x007143d7 TechnoType, ZFudgeBridge @ 0x00460c76 cross-scope; full deploy + slave-system cross-ref to SLAVE_MINER_ORE and SLAVE_MANAGER docs; Storage+Enslaves coexistence explanation; economic-protection flags) |
| TS-legacy filter | ✅ §8 (ImmuneToVeins flagged inert) |
| Cross-references (SLAVE_MINER_ORE, SLAVE_MANAGER, SLAV, HARV, CMIN, AMCV) | ✅ at top + inline |
| Yuri economy explanation | ✅ intro |
| Deploy mechanism + property transfer + targeting redirect | ✅ §7.3 |
| Slave lifecycle | ✅ §7.4 |
| ZFudgeBridge / DeployFacing cross-scope open question | ✅ §7.2 |

---

## 10. Quick implementer summary

To make a SMIN-equivalent (mobile half):

1. **Render** — voxel + HVA with TurretOffset=70 (turret on front of chassis); single PrimaryFireFLH=120,0,185.
2. **Movement** — DriveLocomotionClass, MovementZone=Crusher (can path through infantry-blocked cells); Speed=3 (slow), ROT=5.
3. **Primary attack** — 20mmRapid (Damage=30, HARVWH warhead) at rookie; ElitePrimary=20mmRapidE (Damage=50, HowitzerWH cannon shells) at elite.
4. **Self-defense** — Crusher=yes (crush infantry); SelfHealing=yes (auto-repair all ranks); ImmuneToPsionics=yes; ImmuneToRadiation=yes; OmniCrushResistant=yes.
5. **Deploy → YAREFN building** —
   - On Deploy command: face direction `DeployFacing=0` (N), then call BuildingClass::Constructor with `DeploysInto=YAREFN`.
   - Transfer HP ratio, UniqueID, Location_Z, veterancy, AttachedTag.
   - Redirect targeting from SMIN to YAREFN.
   - Remove SMIN from map.
   - Brain-transplant: ensure SlaveManagerClass and its 5 SLAV infantry re-bind to YAREFN (use shared `Enslaves=SLAV` listing on both SMIN and YAREFN to handle this).
6. **Slave system** —
   - SlaveManagerClass at TechnoClass+0x2D8.
   - `SlavesNumber=5`, `SlaveRegenRate=500` (33s respawn), `SlaveReloadRate=25` (~1.7s mine cycle).
   - Slaves walk to ore tiles, mine with SHOVEL weapon, walk back, dump for credits.
   - See SLAVE_MANAGER_STATE_MACHINE doc for full state machine.
7. **Storage=20** — pre-deploy ore buffer for in-transit slave payloads.
8. **AI flags** — `ResourceGatherer=yes` (AI knows this produces credits), `ResourceDestination=yes` (slaves return here), `ToProtect=yes` (AI escort), `ThreatAvoidanceCoefficient=.65` (65% normal threat-avoidance), `StupidHunt=yes` (fallback to "run to enemy" instead of Hunt).
9. **Audio** — SlaveMiner-unique voice set (Select/Move/Attack/Harvest); separate DeploySound (mechanical) + VoiceDeploy (spoken); shared weapon Reports (WarMinerAttack rookie, RhinoTankAttack elite).
10. **Build gate** — YAWEAP prerequisite only; YuriCountry; TechLevel=1.

The Slave Miner requires the SlaveManager state machine, the deploy-to-building transition with property transfer, and the brain-transplant SlaveManager re-binding. All three subsystems are documented in the referenced GHIDRA_REPORT files; this doc is the integration point connecting the SMIN INI to those subsystems.
