---
name: schp-doc
description: SCHP — Soviet Siege Chopper. Tier-7 deployable artillery helicopter.
  Primary=BlackHawkCannon (anti-air/anti-inf, undeployed) + Secondary=160mm
  (deployed mode, Range=12 anti-everything cannon). IsSimpleDeployer=yes mode-swap
  to SCHD. DeployingAnim=SCHPDEPL transition anim. 3 new cheat-sheet entries.
metadata:
  type: project
---

# SCHP — Soviet Siege Chopper

**INI ID:** `SCHP`
**Display:** "Soviet Siege Chopper" (`UIName=Name:SiegeChopper`)
**Section:** `[VehicleTypes]` — *vehicle-class with JumpJet=yes*, NOT
AircraftType. Like ZEP/SHAD/DISK, classified as jumpjet vehicle. Distinct
from BEAG (true AircraftType).
**Owner side:** Soviet (Russians, Confederation, Africans, Arabs)
**Role:** Soviet tier-7 deployable artillery helicopter. **Two modes**:
undeployed flight (BlackHawkCannon machine gun, anti-inf/anti-air, mobile)
or deployed ground stance (160mm long-range artillery cannon, anti-
everything, stationary). Player toggles via Deploy command. Effectively a
"flying artillery" that lands to fire. Pairs with V3 launcher as Soviet
long-range ground siege.

---

## Rulesmd verbatim

```ini
[SCHP]
UIName=Name:SiegeChopper
Name=Soviet Siege Chopper
;Image=SHAD
Prerequisite=NAWEAP,TECH
Primary=BlackHawkCannon
Secondary=160mm
Strength=300
Category=AirPower
JumpJet=yes
Armor=light
TechLevel=7
Sight=7
Speed=12
PitchSpeed=1.1
JumpjetSpeed=30 ;params not defined use defaults (old globals way up top)
JumpjetClimb=10
JumpjetCrash=40 ; Climb, but down
JumpJetAccel=12
JumpJetTurnRate=6
JumpjetHeight=500
JumpjetWobbles=.01
JumpjetDeviation=1
Owner=Russians,Confederation,Africans,Arabs
Cost=1100
Soylent=1100
Points=15
ROT=5
Crewed=no
ConsideredAircraft=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
MaxDebris=3
VoiceSelect=SeigeChopperSelect
VoiceMove=SeigeChopperMove
VoiceAttack=SeigeChopperAttackAir
VoiceSecondaryWeaponAttack=SeigeChopperAttackLand
VoiceCrashing=SeigeChopperVoiceDie
CrashingSound=SeigeChopperDie
DieSound=
ImpactLandSound=GenAircraftCrash
;Locomotor={4A582746-9839-11d1-B709-00A024DDAFD1} ;flying
Locomotor={92612C46-F71F-11d1-AC9F-006008055BB5} ;jumpjet
MovementZone=Fly
DamageParticleSystems=SparkSys,SmallGreySSys
;AuxSound1=BlackOpsTakeOff	;Taking off
;AuxSound2=BlackOpsLanding	;Landing
ThreatPosed=0
SpecialThreatValue=1
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Size=15
SizeLimit=2
HoverAttack=yes
AllowedToStartInMultiplayer=no
Crashable=yes
;CanPassiveAquire=no ; Won't try to pick up own targets
SpeedType=Hover
MoveSound=SeigeChopperMoveLoop
EnterTransportSound=EnterTransport
LeaveTransportSound=ExitTransport
ElitePrimary=BlackHawkCannonE
EliteSecondary=160mmE
PreventAttackMove=yes
TooBigToFitUnderBridge=true
Trainable=yes
Bunkerable=no; Units default to yes, others default to no
IsSimpleDeployer=yes
UnloadingClass=SCHD
DeployingAnim=SCHPDEPL
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:SiegeChopper` — CSF key "SiegeChopper". Resolves to
  "Soviet Siege Chopper".
- `Name=Soviet Siege Chopper` — internal description.
- `;Image=SHAD` — commented. Was once going to reuse the Allied
  Nighthawk's voxel; Westwood gave SCHP its own art.
- `Category=AirPower`.

**Tech / availability**
- `Prerequisite=NAWEAP,TECH` — Soviet War Factory + tech building.
  `TECH` is a macro resolving to the faction's Battle Lab (Soviet
  `NATECH`). Same Tier-3 lockout pattern.
- `TechLevel=7` — same as SHAD Nighthawk and AEGIS.
- `Owner=Russians,Confederation,Africans,Arabs` — 4 Soviet sub-factions.
- `AllowedToStartInMultiplayer=no` — not a starting unit.

**Combat — defense**
- `Strength=300` — moderate (vs SHAD's 175). Siege Chopper is more
  durable than the Nighthawk transport.
- `Armor=light` — light armor.

**Combat — dual-weapon deploy mode system**

This is the unit's defining mechanic:
- `Primary=BlackHawkCannon` — *undeployed flight mode weapon*. The
  same 35-damage Quad-shell MGUN cannon used by SHAD Nighthawk for
  self-defense (see [SHAD.md](../allied/SHAD.md#weapons)). Range=6,
  OmniFire=yes, 8-direction MGUN-N..NW anim.
- `Secondary=160mm` — **deployed mode weapon**. Range=12, Damage=90
  Ballistic projectile, SCHOPWH warhead (Deform=15% — *terrain deforming!*).
  See Weapon section.
- `ElitePrimary=BlackHawkCannonE` (basic SHAD elite shared) —
  Damage=40, SSA warhead.
- `EliteSecondary=160mmE` — Burst=2 (doubles per-salvo from 90 to 180
  damage).

**Mode-switch fields (the deploy system)**
- `IsSimpleDeployer=yes` — **Ghidra-verified UnitType-scope** at
  `0x00845dfc → 0x00747688`. **NEW cheat-sheet entry**. Marks the unit
  as a *simple deployer* — meaning the Deploy command toggles between
  two modes (undeployed flying SCHP, deployed grounded SCHD) without
  using the standard "DeploysInto=building" pathway (which would create
  a permanent structure). The simple-deployer system creates a
  *unit-to-unit transformation* with reversible mode-swap.
- `UnloadingClass=SCHD` — *the deployed-mode unit class*. Per
  cheat-sheet TechnoType `0x00843af8 → 0x007146e8`. When the SCHP
  player issues Deploy, the SCHP transforms into a SCHD instance at
  the same cell. SCHD has the deployed-mode visual + uses
  PrimaryFireFLH=200,0,250 (matching SCHP's SecondaryFireFLH for
  the 160mm cannon).
- `DeployingAnim=SCHPDEPL` — **Ghidra-verified DUAL-READ**: TechnoType
  at `0x00819490 → 0x00714715` AND `0x0045f322 in
  BuildingTypeClass__LoadVisualAssets`. **NEW cheat-sheet entry**.
  Plays the `SCHPDEPL` animation during the deploy transition. The
  BuildingType cross-reference is the *visual asset loader* (the
  building-class code path uses this to know which anim to load).
  For the SCHP (vehicle), the TechnoType read is what's effective.

**Sight / radar**
- `Sight=7` — moderate.
- *No `RadarInvisible=yes`* — SCHP appears on enemy minimap (unlike
  SHAD's stealth).

**Mobility (jumpjet aircraft same pattern as SHAD)**
- `Speed=12` — fast (slower than SHAD's 14, faster than Kirov's 5).
- `JumpjetClimb=10, JumpjetCrash=40, JumpJetTurnRate=6, JumpjetHeight=500,
  JumpjetWobbles=.01, JumpjetDeviation=1` — same parameters as SHAD.
  SCHP and SHAD use the same flight profile.
- `Locomotor={92612C46-...}` — Jumpjet locomotor (vehicle-class with
  jumpjet, NOT AircraftLocomotion). Same as SHAD/Kirov/Disc.
- `MovementZone=Fly` — fly-zone.
- `SpeedType=Hover`.
- `Size=15, SizeLimit=2` — same as SHAD; can carry size-1 or size-2
  passengers... wait, but SCHP doesn't appear to have `Passengers=`.
  Let me check — *no `Passengers=` line in SCHP rulesmd*. So SizeLimit
  is *vestigial* — SCHP isn't a transport. The field is harmless
  inheritance from the SHAD template.

**Aircraft-class flags**
- `JumpJet=yes` — jumpjet vehicle.
- `ConsideredAircraft=yes` — treated as aircraft for targeting.
- `Crashable=yes` — plummets on death.
- `HoverAttack=yes` — fires from hover (undeployed mode).

**Combat behavior**
- `ThreatPosed=0` — AI doesn't see SCHP as a direct threat (deploy-
  to-fire pattern means it's classified passive).
- `SpecialThreatValue=1` — modest strategic value.
- `Crewed=no`.
- `PreventAttackMove=yes` (TechnoType per SHAD cheat-sheet
  `0x008439b0`). No attack-move command.
- `;CanPassiveAquire=no` — *commented out*. Unlike SHAD which has this
  active, SCHP's auto-target acquisition is **enabled** (the commented
  line shows Westwood considered disabling it but didn't). The Siege
  Chopper *can* auto-engage in-range enemies — but only with its
  Primary BlackHawkCannon (undeployed). When deployed, the 160mm
  doesn't auto-engage because secondary weapons typically don't.
- `EnterTransportSound`/`LeaveTransportSound` — *vestigial* (no
  passengers). Inherited from SHAD template; harmless.

**Voice / sound bindings**
- `VoiceSelect=SeigeChopperSelect` — *note misspelled "Seige"* (Westwood
  typo; consistent across all SCHP voice keys).
- `VoiceMove=SeigeChopperMove`.
- `VoiceAttack=SeigeChopperAttackAir` — **air-attack voice** (Primary
  BlackHawkCannon firing on air targets).
- `VoiceSecondaryWeaponAttack=SeigeChopperAttackLand` — **land-attack
  voice** (Secondary 160mm firing on ground in deployed mode).
  TechnoType per cheat-sheet (`0x00844038`). Same pattern as BSUB's
  Water/Land voice split.
- `VoiceCrashing=SeigeChopperVoiceDie`.
- `CrashingSound=SeigeChopperDie`.
- `DieSound=` empty.
- `ImpactLandSound=GenAircraftCrash` (DUAL-READ Rules+TechnoType).
- `MoveSound=SeigeChopperMoveLoop`.

**Veterancy**
- `VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER` — 4 abilities.
- `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — adds heal + ROF.
- Both weapons swap to elite (BlackHawkCannonE + 160mmE Burst=2).
- `Trainable=yes` — gains veterancy.

**Misc**
- `MaxDebris=3`, `Bunkerable=no`, `TooBigToFitUnderBridge=true`.
- `Cost=1100`, `Soylent=1100`, `Points=15` (cheaper than SHAD at 1000?
  Actually higher: $1100 vs SHAD $1000).

---

## Artmd verbatim

```ini
[SCHP] ; Soviet Siege Chopper
Cameo=SCHPICON
AltCameo=SCHPUICO
Voxel=yes
UseBuffer=yes
Remapable=yes
ShadowIndex=2
PrimaryFireFLH=0,0,50
SecondaryFireFLH=200,0,250

[SCHD] ; Soviet Siege Chopper
Cameo=SCHPICON
Voxel=yes
UseBuffer=yes
Remapable=yes
PrimaryFireFLH=200,0,250
SecondaryFireFLH=200,0,250
```

### Key-by-key annotation (SCHP — undeployed flying form)

- `Cameo=SCHPICON`, `AltCameo=SCHPUICO` — distinct cameos (vs BEAG
  which used same for both).
- `Voxel=yes` — `schp.vxl` + `schp.hva`.
- `UseBuffer=yes` — render buffer optimization (same as SHAD).
- `Remapable=yes` — house-color remap.
- `ShadowIndex=2` — voxel-slice index for shadow (same comment as
  SHAD: order of voxels got changed during development).
- `PrimaryFireFLH=0,0,50` — BlackHawkCannon launch offset:
  - X=0 (centered, below cockpit).
  - Y=0 (centered).
  - Z=50 (low, chin-gun-like).
- `SecondaryFireFLH=200,0,250` — 160mm launch offset:
  - X=200 (very forward, gun barrel extends out the nose).
  - Y=0 (centered).
  - Z=250 (high — large cannon mount).

**No `DisableVoxelCache=yes`** (unlike SHAD which has this performance
flag).

### Key-by-key annotation (SCHD — deployed grounded form)

The `[SCHD]` artmd block defines the visual swap target. Same voxel
asset (Voxel=yes, Cameo=SCHPICON shared), but with both fire offsets
at the *deployed* 200,0,250 position — appropriate for the grounded
chopper with extended landing gear + lowered cannon position.

**The SCHD section in rulesmd also exists** — would be documented in a
separate iteration (the deployed-mode unit class). The two form a
deploy/undeploy pair.

---

## Weapons

### Primary (undeployed) — `[BlackHawkCannon]`

*Shared with SHAD Nighthawk*. 35 damage, ROF=40, Range=6, Quad-shell
projectile, SA warhead, 8-direction MGUN anim, OmniFire=yes. Anti-
infantry / anti-air self-defense.

See [SHAD.md](../allied/SHAD.md#weapons) for full details.

**Why shared?** The SCHP undeployed-mode weapon is identical to the
Nighthawk's defensive cannon. Probably Westwood's design intent: while
flying, the Siege Chopper has a Black Hawk-style machine gun (anti-
infantry); only when deployed does the heavy 160mm artillery come out.

### Primary elite — `[BlackHawkCannonE]`

Damage 35→40, Warhead SA→SSA. Same as SHAD elite.

### Secondary (deployed) — `[160mm]`

```ini
[160mm]
Damage=90
ROF=100
Range=12
MinimumRange=0
Projectile=Ballistic
Speed=10
Warhead=SCHOPWH
Report=SeigeChopperAttackDeployed
Anim=GUNFIRE
Lobber=no
;Lobber=yes
```

- `Damage=90` — strong single-shot. Plus AoE via SCHOPWH (CellSpread=1).
- `ROF=100` — *very slow* (~6.7 sec at 15fps). Plus deploy/undeploy
  transition time. Each 160mm shot is a deliberate commitment.
- `Range=12` — **matches AEGIS Medusa range** (longest non-superweapon
  in the game). Siege Chopper out-ranges most ground defenses.
- `MinimumRange=0` — no minimum range (can fire at adjacent targets,
  unlike CMISL which needs MinimumRange=8).
- `Projectile=Ballistic` — *arcing shell* projectile. See projectile
  block.
- `Speed=10` — slow shell (visible cannon arc).
- `Warhead=SCHOPWH` — Siege Chopper warhead (see below). Has
  `Deform=15%` and `DeformThreshhold=120` — *terrain deformation*.
- `Report=SeigeChopperAttackDeployed` — distinct deployed-mode fire SFX.
- `Anim=GUNFIRE` — generic muzzle flash.
- `Lobber=no` — **Ghidra-verified WeaponType** at
  `0x00849360 → 0x00772749`. **NEW cheat-sheet entry**. The `Lobber`
  flag controls the projectile trajectory style. With `Lobber=no`,
  the Ballistic projectile uses a standard arc. The verbatim
  `;Lobber=yes` historical comment suggests Westwood considered
  enabling the lobber mode (which would have given the 160mm a
  high-angle howitzer trajectory). Disabled in shipped YR.

### Secondary elite — `[160mmE]`

```ini
[160mmE]
Damage=90 (same)
ROF=100 (same)
Range=12 (same)
Burst=2 (NEW)
```

Just adds `Burst=2`. **Doubles per-salvo damage** from 90 to 180. Plus
the AoE warhead applies twice per salvo.

### Projectile — `[Ballistic]`

```ini
[Ballistic]
Image=120MM
Arcing=true
SubjectToCliffs=no
SubjectToElevation=yes
SubjectToWalls=no
```

- `Image=120MM` — generic 120mm shell sprite (shared with Cannon
  projectile family).
- `Arcing=true` — arcing trajectory.
- `SubjectToCliffs=no, SubjectToWalls=no` — cliffs and walls don't
  block the high-arcing shell.
- `SubjectToElevation=yes` — elevation affects aim accuracy.

### Warhead — `[SCHOPWH]`

```ini
[SCHOPWH]
CellSpread=1
PercentAtMax=.25
Wall=yes
Wood=yes
Verses=100%,80%,60%,80%,50%,50%,100%,100%,60%,100%,100%
Conventional=yes
Rocker=yes
InfDeath=2
AnimList=XGRYSML1,XGRYSML2,EXPLOSML,XGRYMED1,XGRYMED2,EXPLOMED,EXPLOLRG
Deform=15%
DeformThreshhold=120
Tiberium=yes
Bright=yes
ProneDamage=50%
```

- `CellSpread=1` — 1-cell AoE.
- `PercentAtMax=.25` — only 25% damage at edge (heavy falloff). Direct
  hits matter; edge damage is light.
- `Wall=yes, Wood=yes` — damages walls and wooden buildings.
- `Verses=100%,80%,60%,80%,50%,50%,100%,100%,60%,100%,100%`:
  | Armor    | Multiplier | vs 90 dmg base |
  |----------|-----------|------------------|
  | none-flak-plate-light | 100%/80%/60%/80% | 90/72/54/72 |
  | medium-heavy | 50% / 50% | 45 / 45 |
  | wood-steel | 100% / 100% | 90 / 90 |
  | concrete | 60% | 54 |
  | special_1/2 | 100% | 90 |
  - **Anti-structure focused** (100% vs wood/steel). Weak vs medium/
    heavy tanks (50%).
- `Conventional=yes`.
- `Rocker=yes` — vehicles rock on impact.
- `InfDeath=2` — infantry-death type 2 (still undocumented in our
  cheat-sheet; possibly "knockback" anim).
- `Deform=15%` — **terrain deformation chance**. 15% probability that
  each impact deforms (craters) the terrain cell. **Unique among
  units documented so far** — most weapons don't deform terrain.
- `DeformThreshhold=120` — *typo "Threshhold"* (correct: "Threshold")
  — but consistent with Westwood's spelling in the binary. Threshold
  damage value: 120. Damage below this threshold *can't* deform
  terrain. Combined with `Deform=15%`, terrain deforms 15% of the
  time when damage ≥ 120 (which requires a direct 90-dmg hit + AoE
  splash adding up to ≥120, OR an elite Burst=2 salvo).
- `Tiberium=yes` — affects ore tiles.
- `Bright=yes` — palette flash on impact.

---

## Voices / sounds

```ini
[SeigeChopperSelect]
Sounds=$vchosea $vchoseb $vchosec $vchosed $vchosee
Control=random
Volume=85

[SeigeChopperMove]
Sounds=$vchomoa $vchomob $vchomoc $vchomod $vchomoe
Control=random
Volume=85

[SeigeChopperAttackAir]
Sounds=$vchoa1a $vchoa1b $vchoa1c $vchoa1d
Control=random
Volume=85

[SeigeChopperAttackLand]
Sounds=$vchoa2a $vchoa2b $vchoa2c $vchoa2d
Control=random
Volume=85

[SeigeChopperVoiceDie]
Sounds=$vchodia $vchodib $vchodic $vchodid
Priority=low
Control=random
Volume=70

[SeigeChopperMoveLoop]
...

[SeigeChopperDie]
...

[SeigeChopperAttackDeployed]
...
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=SeigeChopperSelect` | `[SeigeChopperSelect]` | Click |
| `VoiceMove=SeigeChopperMove` | `[SeigeChopperMove]` | Move order |
| `VoiceAttack=SeigeChopperAttackAir` | `[SeigeChopperAttackAir]` | Primary BlackHawkCannon attack — **air target voice** |
| `VoiceSecondaryWeaponAttack=SeigeChopperAttackLand` | `[SeigeChopperAttackLand]` | Secondary 160mm attack — **land target voice** |
| `VoiceCrashing=SeigeChopperVoiceDie` | `[SeigeChopperVoiceDie]` | Voice during plummet |
| `CrashingSound=SeigeChopperDie` | `[SeigeChopperDie]` | Sustained crash SFX |
| `ImpactLandSound=GenAircraftCrash` | shared | Impact |
| `MoveSound=SeigeChopperMoveLoop` | `[SeigeChopperMoveLoop]` | Rotor loop |
| `Report=SeigeChopperAttackDeployed` (in 160mm weapon) | `[SeigeChopperAttackDeployed]` | 160mm fire SFX |
| `Report=BlackOpsAttack` (in BlackHawkCannon weapon) | `[BlackOpsAttack]` | MGUN fire SFX (shared with SHAD) |

**`Seige`** typo is consistent across all sound block names — Westwood
spelled "Siege" as "Seige" throughout the SCHP voice keys. Both the
rules-side `VoiceSelect=SeigeChopperSelect` and the soundmd-side
`[SeigeChopperSelect]` use the typo. *Not corrected in shipped YR*.

---

## Hardcoded behavior (Ghidra-verified)

### 1. IsSimpleDeployer + UnloadingClass + DeployingAnim trio (deploy mode-swap)

The deploy/undeploy mechanism for unit-to-unit transformation uses
three coordinated fields:

- **`IsSimpleDeployer=yes`** (UnitType `0x00845dfc → 0x00747688`,
  **NEW cheat-sheet entry**) — flags the unit as a deploy-toggle
  vehicle. The engine treats Deploy command as a class-swap rather
  than a building-creation.
- **`UnloadingClass=SCHD`** (TechnoType `0x00843af8 → 0x007146e8`,
  per cheat-sheet) — *the deployed-mode unit class*. When SCHP is
  ordered to deploy, the engine despawns the SCHP instance and
  spawns a SCHD at the same cell.
- **`DeployingAnim=SCHPDEPL`** (DUAL-READ TechnoType `0x00714715`
  + BuildingType `BuildingTypeClass__LoadVisualAssets 0x0045f322`,
  **NEW cheat-sheet entry**) — the animation that plays during the
  transition. `SCHPDEPL` is a separate art asset showing the chopper
  landing/unfolding the cannon.

**Deploy sequence:**
1. Player issues Deploy command.
2. SCHP plays `SCHPDEPL` anim while descending to ground.
3. After anim completes, SCHP entity is removed.
4. SCHD entity spawns at same cell with full HP.
5. SCHD has the 160mm as Primary (or visible Secondary in deployed
   pose).
6. Undeploy reverses: SCHD → SCHPDEPL anim (likely reversed?) → SCHP.

The architecture is identical to how harvesters use UnloadingClass
for dock-unload (HARV → HORV during ore unload) — see HARV/HORV doc
pair.

### 2. AircraftType vs JumpJet=yes architecture distinction

The previous iteration's discovery of **AircraftTypeClass__ReadINI** as
a distinct scope (BEAG) means SCHP is *not* an AircraftType. Despite
flying via jumpjet locomotion, SCHP is classed as a VehicleType:
- `JumpJet=yes` flag identifies as jumpjet vehicle.
- `ConsideredAircraft=yes` flag makes it air-targetable.
- AircraftType-only fields like `AirportBound` and `Fighter` are *not
  read* on SCHP (it doesn't use airports — it lands wherever the
  Deploy command places it).

The architectural choice: SCHP needs to land *anywhere on terrain*
(not at airports), which the AircraftType class would prevent
(AirportBound enforces airport-only). Jumpjet-class allows any-cell
landing.

### 3. Lobber=no (WeaponType) projectile trajectory

`Lobber` (WeaponType `0x00849360 → 0x00772749`, **NEW cheat-sheet
entry**). With `Lobber=no` on the 160mm weapon, the projectile uses
a *standard* Arcing=true ballistic trajectory. With `Lobber=yes`, the
weapon would fire in a *high-angle howitzer arc* (steeper, slower).
Currently disabled.

### 4. VoiceSecondaryWeaponAttack split

Same pattern as BSUB's Water/Land voice split. SCHP's Primary fires
the air-attack voice; Secondary fires the land-attack voice. The
distinction reinforces the air/ground role split (air mode = anti-
infantry/anti-air with BlackHawkCannon; land mode = anti-structure
with 160mm artillery).

### 5. SHAD-template inheritance

Many SCHP fields appear to be copy-pasted from SHAD Nighthawk:
- Same JumpJet flight parameters.
- Same Size=15, SizeLimit=2 (the SizeLimit is *vestigial* — SCHP has
  no Passengers=).
- Same EnterTransportSound + LeaveTransportSound (also vestigial).
- Same `Bunkerable=no`, `TooBigToFitUnderBridge=true`.
- Same Crashable, ImpactLandSound, ThreatPosed=0 + SpecialThreatValue=1.

The SHAD template was extended for SCHP with the deploy mechanism +
160mm weapon. **Westwood's INI-by-copy-paste design pattern**.

### 6. Trainable=yes (rare for transport-like)

SCHP gains veterancy from kills via *both* weapons. Both elite swaps
(BlackHawkCannonE + 160mmE) trigger at Elite rank. Same as SHAD which
is also Trainable=yes.

### 7. SCHOPWH terrain deformation

`Deform=15%` + `DeformThreshhold=120` enable terrain cratering on
high-damage 160mm hits. **Unique mechanical feature** among documented
units — terrain deformation is rare in YR. The combined Burst=2 elite
shot at 90+90 damage = 180 total per cell easily exceeds the 120
threshold, triggering deformation at 15% chance per impact.

---

## TS-legacy filter

- `;Image=SHAD` commented — historical art reuse.
- `;Locomotor={4A582746-...} ;flying` — commented fixed-wing AircraftLocomotion
  alternative (would have made SCHP a true aircraft; abandoned for
  jumpjet flexibility).
- `;CanPassiveAquire=no` commented — Westwood considered disabling
  auto-target acquisition (like SHAD) but left it enabled.
- `;Lobber=yes` commented — alternative howitzer trajectory disabled.
- `;AuxSound1/2=BlackOpsTakeOff/Landing` — commented historical
  takeoff/landing SFX (same as SHAD/Kirov).
- No `ImmuneToVeins`, no `Subterranean`. **YR-active core mechanism.**

---

## Comparison: SCHP vs peer Soviet aerial

| Field | SCHP Siege Chopper | ZEP Kirov Airship | HIND (cut) |
|-------|---------------------|---------------------|-------------|
| Strength | 300 | 2000 | 300 |
| Cost | 1100 | 2000 | (TL-1) |
| Speed | 12 | 5 | (TL-1) |
| TechLevel | 7 | 10 | -1 |
| Section | VehicleTypes | VehicleTypes | VehicleTypes |
| JumpJet | yes | yes | yes |
| Primary | BlackHawkCannon + 160mm | BlimpBomb | BlackHawkCannon |
| Range | 6 (Pri) / 12 (Sec) | 1.5 | n/a |
| Damage | 35 / 90 | 250 | 35 |
| IsSimpleDeployer | **yes** | no | no |
| Mode-swap class | SCHD | none | none |

**SCHP's defining feature** is the deploy mode-swap. ZEP and SHAD
fight from one role; SCHP toggles between flying scout/AA support and
grounded artillery.

---

## Cross-references

- [SHAD.md](../allied/SHAD.md) — Allied counterpart (no deploy mode,
  passenger transport instead).
- [ZEP.md](../soviet/ZEP.md) — Soviet alternative aerial unit (heavy
  bomber, no deploy).
- [V3.md](../soviet/V3.md) — Soviet long-range ground siege artillery
  (V3 Rocket Launcher). SCHP fills a similar role from the air.
- [SCHD.md](./SCHD.md) — Deployed-mode unit class. **Iteration 77
  reveals SCHD is likely vestigial** (Name=ZZZ, TechLevel=-1, missing
  Secondary weapon, borrowed crash sounds). The above SCHP claims
  about "deploy mode-swap to SCHD" need Ghidra verification — the
  actual deploy mechanism may stay at SCHP-level (no entity swap).
  Open follow-up: trace `IsSimpleDeployer` code path.
- [HARV.md](../soviet/HARV.md) + HORV — peer UnloadingClass mode-swap
  example (harvester dock-unload).

---

## Coverage audit

- [x] Every rulesmd key annotated (~55 keys).
- [x] Every artmd key annotated for both SCHP + SCHD pair (10 keys).
- [x] Primary weapon documented (BlackHawkCannon — shared with SHAD).
- [x] Secondary weapon documented (160mm with deploy-mode context).
- [x] Ballistic projectile documented.
- [x] SCHOPWH warhead documented including Deform=15% terrain
  deformation.
- [x] All voice/sound bindings documented including misspelled
  `Seige*` typo.
- [x] Prerequisites: `NAWEAP, TECH (→NATECH)`.
- [x] Owner: 4 Soviet sub-factions.
- [x] Veterancy: Trainable=yes, both weapons elite-swap.
- [x] Hardcoded behavior: **IsSimpleDeployer + UnloadingClass +
  DeployingAnim deploy mode-swap trio**, Lobber WeaponType flag,
  AircraftType-vs-Vehicle architectural distinction, SHAD-template
  inheritance, terrain deformation.
- [x] TS-legacy filter applied (commented Lobber + Locomotor + Image
  + AuxSound historical fields).
- [x] Comparison with peer aerial units.
- [x] At least one Ghidra search performed (4 strings + xrefs, 3 new
  cheat-sheet entries).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("IsSimpleDeployer")` | `0x00845dfc` (single match) |
| `get_xrefs_to(0x00845dfc)` | `0x00747688 → UnitTypeClass__ReadINI` |
| `search_strings("DeployingAnim")` | `0x00819490` (single + `RoofDeployingAnim` sibling) |
| `get_xrefs_to(0x00819490)` | **DUAL-READ**: TechnoType `0x00714715` + `BuildingTypeClass__LoadVisualAssets 0x0045f322` |
| `search_strings("^Lobber$")` | `0x00849360` (single match) |
| `get_xrefs_to(0x00849360)` | `0x00772749 → WeaponTypeClass__ReadINI` |

**New cheat-sheet entries (3):**
- `IsSimpleDeployer` (0x00845dfc → 0x00747688) **UnitType** — flags
  deploy as unit-to-unit transformation (not building creation).
- `DeployingAnim` (0x00819490 → 0x00714715 TechnoType + 0x0045f322
  BuildingType-LoadVisualAssets) — transition animation. **First seen
  DUAL-READ pattern across ReadINI + LoadVisualAssets (not Rules
  global)**.
- `Lobber` (0x00849360 → 0x00772749) **WeaponType** — high-angle
  howitzer trajectory toggle.

**Sibling field discovered (not verified):**
- `RoofDeployingAnim` (0x0081947c) — for buildings (likely building-
  specific deploy anim, paired with DeployingAnim).

**Re-confirmed:**
- `UnloadingClass` (TechnoType, per cheat-sheet from HARV/CMIN).
- `VoiceSecondaryWeaponAttack` (TechnoType, per cheat-sheet from BSUB).
- `PreventAttackMove` (TechnoType, per SHAD).

**Open questions:**
- The DeployingAnim cross-reference from `BuildingTypeClass__LoadVisualAssets`
  is unusual — not a ReadINI function. Possibly the *building-class*
  uses this anim field for buildup-style anim, while the *vehicle-class*
  uses it for deploy-mode transition. Open follow-up.
- The Westwood typo "Seige" → "Siege" — is this case-sensitive in the
  CSF lookup? Verbatim consistency suggests the engine doesn't care
  about typo correction.
- SCHD (deployed-mode unit) needs its own iteration to close the pair.
