---
name: sref-doc
description: SREF — Prism Tank. Allied tier-3 beam-chain siege artillery. Comet/SuperComet
  IsLaser=yes IsHouseColor=yes weapons; ShrapnelWeapon chain to secondary
  CometFragment beams (BulletType-level field, new ReadINI scope discovered).
  TurretCount=4/WeaponCount=1 "abusive" weapon-via-turret pattern.
metadata:
  type: project
---

# SREF — Prism Tank

**INI ID:** `SREF`
**Display:** "Prism Tank" (`UIName=Name:SREF`)
**Section:** `[VehicleTypes]`
**Owner side:** Allied (British, French, Germans, Americans, Alliance)
**Role:** Allied tier-3 siege artillery. Fires a charged prism beam (Comet) with
~Range=10 (longest non-naval tank range in the game) and **shrapnel-chain
secondary beams** that ricochet to nearby targets. Pairs with Mirage Tank as the
Allied "tier-3 ground vehicle" duo. Fragile (Strength=150, Armor=light) but
out-ranges most defenders and deals massive damage to clustered targets.

---

## Rulesmd verbatim

```ini
[SREF]
UIName=Name:SREF
Name=Prism Tank
Prerequisite=GAWEAP,GATECH
; SJM removed; see abusive section below...
; Primary=Comet
; ElitePrimary=SuperComet ; Elite Weapon
Strength=150
Category=AFV
Armor=light
; SJM: begin abuse of turret-changing code ----
Turret=yes ;temp until tank art done
TurretCount=4
WeaponCount=1
Weapon1=Comet
EliteWeapon1=SuperComet ; Elite Weapon
IsChargeTurret=true
; SJM: end abuse ------------------------------
IsTilter=yes
TooBigToFitUnderBridge=true
TechLevel=8
Sight=8
Speed=4
CrateGoodie=yes
Crusher=yes
Owner=British,French,Germans,Americans,Alliance
Cost=1200
Soylent=1200
Points=50
ROT=5
IsSelectableCombatant=yes
AllowedToStartInMultiplayer=no
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=PrismTankSelect
VoiceMove=PrismTankMove
VoiceAttack=PrismTankAttackCommand
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=PrismTankMoveStart
CrushSound=TankCrush
Maxdebris=3
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
;MovementZone=Normal ;gs FLAW needs to be changed to this when The Flaw is fixed
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
Size=3
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:SREF` — CSF key. Resolves to "Prism Tank".
- `Name=Prism Tank` — internal description.
- `Category=AFV` — AI threat-bucket assignment.

**Tech / availability**
- `Prerequisite=GAWEAP,GATECH` — needs Allied War Factory **and** Allied
  Battle Lab. Tier-3 lockout.
- `TechLevel=8` — tier-8 (one tier below MCV/Kirov tier-10).
- `Owner=British,French,Germans,Americans,Alliance` — *all four Allied
  sub-factions plus the generic "Alliance" house*. Not available to
  Russians/Soviet/Yuri lines.
- `AllowedToStartInMultiplayer=no` — cannot be a starting unit in MP;
  must be built up through tech tree.
- `CrateGoodie=yes` — eligible from UnitCrate pickups for Allied players.

**The "abusive" turret-changing block (key feature)**

The verbatim INI comments from SJM (Steve J. Maetzold) are explicit:

```
; SJM: begin abuse of turret-changing code ----
Turret=yes ;temp until tank art done
TurretCount=4
WeaponCount=1
Weapon1=Comet
EliteWeapon1=SuperComet
IsChargeTurret=true
; SJM: end abuse ------------------------------
```

What's happening:
- The engine's `TurretCount=N` + `Weapon%dN` syntax is normally used by
  multi-turret vehicles like FV ([FV](../allied/FV.md), with `TurretCount=4
  WeaponCount=17` per-passenger turret swap) or YTNK ([YTNK](../yuri/YTNK.md),
  Gattling Tank stages). The Prism Tank **abuses** this system to declare
  a single weapon-slot via the multi-turret pathway *despite having only
  one logical turret* — likely a workaround for late art delivery where
  the tank art wasn't ready yet (`;temp until tank art done` is the
  smoking gun comment).
- `TurretCount=4` — four turret-slot frames (vestigial; the visible art
  only renders one).
- `WeaponCount=1` — one weapon per turret slot.
- `Weapon1=Comet` — first weapon slot. **This is the basic primary**.
- `EliteWeapon1=SuperComet` — elite-rank weapon swap for slot 1.
  *Equivalent to `ElitePrimary=` but routed through the turret-weapon
  system.*
- `IsChargeTurret=true` — *charge-up animation gate*. The turret runs a
  pre-fire charge-up cycle (visible energy build-up on the prism) before
  emitting the beam. **Ghidra-verified TechnoType-scope** at
  `0x0084432c → 0x00712885`. The flag is read into TechnoType but the
  charge-up code only fires when the unit has a turret + WeaponCount-based
  weapon system.
- The commented `; Primary=Comet / ; ElitePrimary=SuperComet` lines were
  the *original* simpler syntax; SJM swapped to the abusive multi-turret
  syntax to enable `IsChargeTurret=true` (only the multi-turret system
  supports the charge animation).
- `Turret=yes ;temp until tank art done` — the comment suggests the
  `Turret=yes` flag was temporary because Prism Tank's eventual art was
  going to be turretless (the charge-coils on the body). In the shipped
  game the tank *does* have a visible rotating prism, but the comment
  is preserved.

**Combat — defense**
- `Strength=150` — **half of LTNK's HP**. The Prism Tank is a fragile
  glass cannon. Compare:
  | Tank | Strength |
  |------|----------|
  | SREF Prism Tank | **150** |
  | LTNK Lasher | 300 |
  | MTNK Grizzly | 300 |
  | HTNK Rhino | 400 |
- `Armor=light` — **light armor type**. Soft target — most AT weapons hit
  hard. The Prism Tank survives via *range, not durability*.

**Combat — weapons (cross-reference)**

The two weapon slots are documented in detail in the "Weapons" section:
- `Weapon1=Comet` (basic) — Damage=100, Range=10, IsLaser, IsHouseColor,
  LargeCometP projectile with ShrapnelWeapon=CometFragment, ShrapnelCount=5.
- `EliteWeapon1=SuperComet` (elite) — Damage=150 (+50%), Range=10,
  SuperCometP projectile with ShrapnelWeapon=SuperCometFragment,
  ShrapnelCount=5. SuperCometFragment fires 8 secondaries (vs CometFragment's
  3). Roughly **2× the chain spread at elite**.

**Sight / mobility**
- `Sight=8` — 8-cell vision. Matches `Range=10` of the Comet weapon, so the
  Prism Tank can see *most* of its own attack range (last 2 cells are
  fired blind / requires spotter).
- `Speed=4` — slow. Tied with MCV / Apocalypse for slowest tank-class
  speed.
- `ROT=5` — turret rotation rate (moderate).
- `Locomotor=Drive` — standard ground locomotor.
- `MovementZone=Destroyer` — wall-crushing zone.
  - `;MovementZone=Normal ;gs FLAW needs to be changed to this when
    The Flaw is fixed` — commented-out historical note. Greg Smith ("gs")
    flagged a bug ("The Flaw") that prevented switching to `Normal`. Whatever
    that flaw was, it never got fixed; Destroyer zone shipped.
- `Weight=3.5` — physics weight.
- `Size=3` — fits in a Battle Fortress (BFRT has `Passengers=5` capacity
  with `SizeLimit=2` — wait, the SizeLimit is per-occupant size cap, so
  the Prism Tank (Size=3) **cannot fit in a Battle Fortress**. Battle
  Fortress only accepts size-1 or size-2 occupants.
- `Accelerates=false` — instant speed (no acceleration ramp). Same as
  LTNK. Ghidra-verified TechnoType (per LTNK doc).
- `TooBigToFitUnderBridge=true` — cannot drive under bridges. UnitType-
  scope per cheat-sheet.
- `IsTilter=yes` — body-tilt animation on slopes. UnitType-scope per
  cheat-sheet (`0x00845df0 → 0x00747712`).

**Economy**
- `Cost=1200` — premium tier-3 price. Affordable in the late game; not
  uncommon to see 4-6 Prism Tanks together.
- `Soylent=1200` — full refund.
- `Points=50` — moderate score.

**Crew / death**
- *No `Crewed=` line* → defaults to `Crewed=no` for vehicles. **Does NOT
  eject infantry on death.** Unusual for Allied — Grizzly/Rhino/MCV all
  have `Crewed=yes`. The Prism Tank is "manned" by automated prism
  controls thematically.
- `Maxdebris=3` (lowercase `d` typo, same as LTNK — INI is case-insensitive).
- `DieSound=GenVehicleDie` — generic vehicle death.

**Behavior flags**
- `Crusher=yes` — crushes Crushable infantry. **No `OmniCrushResistant`**
  — can be crushed by Apocalypse.
- `IsSelectableCombatant=yes` — included in rubber-band combat-only filter.
- `ThreatPosed=40` — moderate AI threat. Same as Lasher despite much
  higher damage — likely because of fragility (low HP makes it a fragile
  threat).

**Veterancy**
- `VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER` — standard MBT
  veteran upgrades.
- `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — elite adds passive
  HP regen, ROF buff. Combined with the SuperComet weapon swap.
- Note: even at elite rank the Prism Tank only has Strength≈225 (150
  base + 50% STRONGER) — still very fragile.

**Z-axis sort**
- `ZFudgeColumn=8` — Z-sort offset near cliff columns.
- `ZFudgeTunnel=13` — Z-sort offset in tunnel cells. **TS-legacy dormant**.

---

## Artmd verbatim

```ini
[SREF]   ; prism tank
Voxel=yes
Remapable=yes
Cameo=SREFICON
AltCameo=SREFUICO
Weapon1FLH=48,0,184
```

### Key-by-key annotation

- `Voxel=yes` — rendered from `sref.vxl` + `sref.hva`. Turret voxel
  separate.
- `Remapable=yes` — house-color remap palette applies.
- `Cameo=SREFICON` — sidebar build-button SHP.
- `AltCameo=SREFUICO` — UI-overlay alt cameo.
- `Weapon1FLH=48,0,184` — **`Weapon1FLH=` matches the rulesmd
  `Weapon1=Comet` slot**, *not* the standard `PrimaryFireFLH`. The
  per-turret/per-weapon FLH naming is consistent with the multi-turret
  syntax used in `[FV]` (FV has Weapon1FLH..Weapon17FLH for its
  passenger-weapon swap system).
  - X=48 (forward of center; relatively short barrel)
  - Y=0 (centered)
  - Z=184 (184 leptons up — the prism array sits high on the turret)

**Note: BFRT (Battle Fortress) shares this art** — see `[BFRT]` in artmd:
`Image=SREF`. The Battle Fortress *reuses the Prism Tank's voxel* as its
visual. Same voxel asset, different unit semantics (Battle Fortress is
the transport variant). This sharing is unrelated to the rulesmd-level
weapon system; it's purely art-asset reuse. **Index note:** See [BFRT](../allied/BFRT.md)
for the Battle Fortress doc (already done) — the `Image=SREF` line means
"render BFRT using sref.vxl assets."

---

## Weapons

The Prism Tank uses the `Weapon1=` / `EliteWeapon1=` slot syntax (not
`Primary=`/`ElitePrimary=`). Both weapons are fully functional and
indistinguishable from the standard `Primary=` slot in gameplay.

### Basic — `[Comet]`

```ini
[Comet]
Damage=100
ROF=100
Range=10
Projectile=LargeCometP
Speed=40
Report=PrismTankAttack
Warhead=CometWH
Bright=yes
;LaserInnerColor = 216,0,184
;LaserOuterColor = 80,0,88
IsHouseColor=true
LaserOuterSpread= 0,0,0
LaserDuration = 15
IsLaser=true	; this flag tells the game to use the special laser draw effect
```

- `Damage=100` — solid single-shot damage.
- `ROF=100` — *very slow* fire rate (~6.7 seconds per shot at 15fps).
  Plus the `IsChargeTurret=true` charge-up. Effective DPS is moderate
  despite the high per-shot damage.
- `Range=10` — *longest non-naval range in the game*. Out-ranges all
  base defenses (Patriot ~7.5, Sentry Gun ~6, Pillbox ~6, Tesla Coil ~7).
  Can siege most defenses from outside their counter-fire range.
- `Projectile=LargeCometP` — the prism beam projectile carrier. See
  "Projectile" subsection below.
- `Speed=40` — projectile speed.
- `Report=PrismTankAttack` — fire SFX (`vsrfatta` style — see "Voices"
  section).
- `Warhead=CometWH` — see warhead block below.
- `Bright=yes` — palette-brightens cells one frame on fire (the famous
  prism flash).
- `;LaserInnerColor = 216,0,184` / `;LaserOuterColor = 80,0,88` —
  commented because `IsHouseColor=true` overrides them. House color
  takes priority over explicit RGB.
- `IsHouseColor=true` — **the beam uses the player's house color**.
  Player-color tinted laser. Ghidra-verified WeaponType `0x008492c4`
  approx (per cheat-sheet IsHouseColor area — exact address would
  require search).
- `LaserOuterSpread=0,0,0` — no jitter / outer-beam spread (clean
  straight beam).
- `LaserDuration=15` — laser beam visual lingers 15 frames (~1 second)
  before fading. Long visual tail makes the prism shot dramatic.
- `IsLaser=true` — *triggers the special laser-draw renderer*. See
  [LASER_DRAW_CLASS_GHIDRA_REPORT.md](../../LASER_DRAW_CLASS_GHIDRA_REPORT.md)
  for the full Laser class.

### Elite — `[SuperComet]`

```ini
[SuperComet]
Damage=150
ROF=100
Range=10
Projectile=SuperCometP
Speed=10
Report=PrismTankAttack
Warhead=CometWH
Bright=yes
;LaserInnerColor = 216,0,184
;LaserOuterColor = 80,0,88
IsHouseColor=true
LaserOuterSpread= 0,0,0
LaserDuration = 15
IsLaser=true
```

**Three changes vs `[Comet]`:**
1. `Damage=150` (vs 100) — **+50% damage**.
2. `Projectile=SuperCometP` (vs LargeCometP) — the projectile is the
   real upgrade (see ShrapnelWeapon analysis below).
3. `Speed=10` (vs 40) — **slower projectile**? Likely a quirk; the
   shrapnel-chain effect mechanically supersedes projectile speed.
   *Could be a Westwood balance choice.*

Everything else identical (ROF, Range, laser visuals).

### Projectile — `[LargeCometP]` (basic)

```ini
[LargeCometP]
ShrapnelWeapon=CometFragment
;ShrapnelCount=-10
ShrapnelCount=5
Inviso=yes
Image=none
SubjectToCliffs=yes
SubjectToElevation=no
SubjectToWalls=no
```

- `ShrapnelWeapon=CometFragment` — on impact, spawn a secondary
  `[CometFragment]` weapon targeting nearby objects. **The chain effect**.
- `ShrapnelCount=5` — spawn up to 5 fragment shots (`(10 - Range in cells)`
  comment on SuperCometP suggests a range-relative formula, but here
  it's just 5). The commented `-10` would have been "absolute 10
  shots"; lowered to 5 for balance.
- `Inviso=yes` — projectile itself is invisible; only the laser-beam
  visual and explosion show.
- `Image=none` — no SHP (handled by laser-draw).
- `SubjectToCliffs=yes` — terrain cliffs block the chain.
- `SubjectToElevation=no` — height differences don't affect aim.
- `SubjectToWalls=no` — *passes through walls*. The chain isn't
  blocked by garrison walls / brick.

### Projectile — `[SuperCometP]` (elite)

```ini
[SuperCometP]
ShrapnelWeapon=SuperCometFragment
ShrapnelCount=5 ; Means (10 - (Range in cells))
Inviso=yes
Image=none
SubjectToCliffs=yes
SubjectToElevation=no
SubjectToWalls=no
```

Same as LargeCometP except `ShrapnelWeapon=SuperCometFragment`.

### Secondary fragment weapons (the chain projectiles)

```ini
[CometFragment]
Damage=30
ROF=120
Range=3
Projectile=SmallCometP
Speed=10
Warhead=CometWH
Bright=yes
IsHouseColor=true
LaserOuterSpread= 0,0,0
LaserDuration = 15
IsLaser=true

[SuperCometFragment]
Damage=50
ROF=100
Range=5
Projectile=SuperSmallCometP
Speed=10
Report=
Warhead=CometWH
Bright=yes
IsHouseColor=true
LaserOuterSpread= 0,0,0
LaserDuration = 15
IsLaser=true
```

- Both inherit `Warhead=CometWH`, `IsHouseColor=true`, `IsLaser=true`.
- **CometFragment**: 30 damage, Range=3, `Projectile=SmallCometP` (no
  further shrapnel — chain ends here).
- **SuperCometFragment**: 50 damage (+67%), Range=5 (+67%),
  `Projectile=SuperSmallCometP` which has `ShrapnelWeapon=CometFragment`
  and `ShrapnelCount=3`. **The elite chain has TWO levels**:
  SuperComet → SuperCometFragment (×5) → CometFragment (×3 each) =
  **up to 5×3 = 15 tertiary beams per shot**.

Basic Prism chain (1 hit):
- LargeCometP → CometFragment ×5 (terminates).
- Total: 1 primary (100 dmg) + 5 secondaries (30 dmg each) = up to 6
  hits, 250 dmg potential.

Elite Prism chain (1 hit):
- SuperCometP → SuperCometFragment ×5 (50 dmg each, Range=5 spread).
- Each SuperCometFragment → SmallSuperCometP → CometFragment ×3 (30 dmg
  each, Range=3 spread).
- Total: 1 primary (150 dmg) + 5 secondaries (50 dmg each) + 15 tertiaries
  (30 dmg each) = up to **21 hits, 850 dmg potential** against tightly
  clustered targets.

**This is the "prism chain" effect** — the Prism Tank's signature
gameplay mechanic. Effectiveness scales with target density. Against a
clustered base, a single elite Prism shot can shred multiple structures.

### Warhead — `[CometWH]`

```ini
[CometWH]
Wall=no
Verses=100%,100%,100%,75%,50%,50%,200%,200%,200%,100%,100%
AnimList=XGRYSML1,XGRYSML2,EXPLOSML,XGRYMED1,XGRYMED2,EXPLOMED,EXPLOLRG,TWLT070
```

- `Wall=no` — *does not damage walls*. Cannot be used to clear wall
  segments — the chain shrapnel can't open base defenses through walls.
  This is a deliberate balance lever.
- `Verses=100%,100%,100%,75%,50%,50%,200%,200%,200%,100%,100%`:
  | Armor    | Multiplier | vs 100 base dmg |
  |----------|-----------|------------------|
  | none     | 100%      | 100 |
  | flak     | 100%      | 100 |
  | plate    | 100%      | 100 |
  | light    | 75%       | 75 |
  | medium   | **50%**   | 50 |
  | heavy    | **50%**   | 50 |
  | wood     | **200%**  | **200** |
  | steel    | **200%**  | **200** |
  | concrete | **200%**  | **200** |
  | special_1 | 100%     | 100 |
  | special_2 | 100%     | 100 |

  **The Prism Tank is an anti-structure weapon**. 200% vs wood/steel/
  concrete means structures take *double damage* per beam. Against a
  Power Plant or Refinery, a single basic Prism shot is 200 damage
  (vs 50 to a Rhino Tank). Plus 5 secondaries × 60 (30×2.0) = 300 chain
  damage = **500 total damage per shot vs base structures**.

  Weak vs medium/heavy tanks (50%) — Prism Tanks are *not* primary
  anti-tank. Use Mirage Tanks / Grizzlies for that.
- `AnimList=XGRYSML1...TWLT070` — 8-anim pool; engine picks by damage
  bracket.
- **No `InfDeath=`** — defaults to 0 (no infantry death anim override).
  Probably means infantry death uses default small-arms anim (1) since
  CometWH has no explicit override. Open question.

---

## Voices / sounds

```ini
[PrismTankSelect]   ; not shown above but follows naming convention
[PrismTankMove]
[PrismTankAttackCommand]
[PrismTankMoveStart]
[PrismTankAttack]  ; Report from weapon
```

Bindings:
| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=PrismTankSelect` | `[PrismTankSelect]` | Click |
| `VoiceMove=PrismTankMove` | `[PrismTankMove]` | Order to move |
| `VoiceAttack=PrismTankAttackCommand` | `[PrismTankAttackCommand]` | Order to attack |
| `Report=PrismTankAttack` (in both weapons) | `[PrismTankAttack]` | Fire SFX |
| `MoveSound=PrismTankMoveStart` | `[PrismTankMoveStart]` | Ignition |
| `DieSound=GenVehicleDie` | shared | Death |
| `CrushSound=TankCrush` | shared | Crushing infantry |

Standard Allied unit voice pool. No special quirks. Voice character is
clipped-British-officer style ("Spreading the rainbow!").

---

## Hardcoded behavior (Ghidra-verified)

### 1. Multi-turret weapon syntax — TurretCount/WeaponCount/Weapon1=

The Prism Tank uses the `TurretCount=N` + `Weapon%dN=` syntax to declare
its weapon slot. The system is the same one used by:
- [FV](../allied/FV.md) — IFV with `TurretCount=4 WeaponCount=17` for
  per-passenger weapon swap.
- [YTNK](../yuri/YTNK.md) — Gattling Tank with `WeaponStages=3` overlay.

The Prism Tank's use is "abusive" because it doesn't actually have
multiple turrets — it just uses the syntax to enable `IsChargeTurret=true`,
which requires the multi-turret weapon system to function.

### 2. IsChargeTurret=true charge-up animation

`IsChargeTurret=true` (Ghidra-verified TechnoType-scope at
`0x0084432c → 0x00712885`) gates the engine's pre-fire charge-up
sequence:
1. Player issues attack command.
2. Turret rotates to face target.
3. **Charge-up phase begins** — turret plays a "charge" animation
   (visible energy buildup on prism), withholding fire.
4. Charge completes → weapon fires.
5. ROF cooldown begins.

The charge animation timing is hardcoded (not INI-configurable that I'm
aware of). Adds ~1 second of "telegraphed" warning to each shot —
defenders can move out of the way if they see it coming. Combined with
the slow projectile speed, the elite SuperCometP (Speed=10) is very
slow — a fast-moving Apocalypse can sometimes dodge an aimed elite Prism.

### 3. Shrapnel chain (BulletType-scope, NEW SCOPE)

`ShrapnelWeapon` is read in `BulletTypeClass__ReadINI` at
`0x0081b03c → 0x0046c2ec`. **This is a NEW ReadINI scope** that
hasn't been logged in the cheat-sheet before:
- `WeaponTypeClass__ReadINI` (0x00772xxx range)
- `WarheadTypeClass__ReadINI` (0x0075Dxxx range)
- `BulletTypeClass__ReadINI` (0x0046xxxx range — NEW)
- `TechnoTypeClass__ReadINI` (0x00712-0x00715 range)
- `UnitTypeClass__ReadINI` (0x00747xxx range)
- `InfantryTypeClass__ReadINI` (0x00524xxx range)
- `RulesClass__ReadGeneral` (0x00671xxx range)
- `RulesClass__ReadCombatDamage` (0x0066Bxxx range)
- `RulesClass__ReadAudioVisual` (0x00669xxx range)

**The ShrapnelWeapon system runs on the projectile (BulletType), not on
the weapon (WeaponType).** When `[LargeCometP]` impacts, the
BulletType-level shrapnel-spawn code fires `ShrapnelCount` instances of
`ShrapnelWeapon` from the impact location, randomly targeting nearby
enemies within the secondary weapon's `Range`. This is the engine-side
implementation of the prism chain.

### 4. IsLaser=true laser draw

`IsLaser=true` (WeaponType-scope, listed in the cheat-sheet at
0x008492dc area) triggers the special laser-draw renderer. See
[LASER_DRAW_CLASS_GHIDRA_REPORT.md](../../LASER_DRAW_CLASS_GHIDRA_REPORT.md)
for the full LaserDrawClass internals. The Prism Tank uses the standard
laser draw with `IsHouseColor=true` for player-tinted beams.

### 5. IsHouseColor=true

`IsHouseColor=yes` overrides the explicit `LaserInnerColor` /
`LaserOuterColor` RGB values with the firing player's house color
(Allied = blue by default, Soviet = red, Yuri = orange, plus any
MP-lobby color overrides). WeaponType-scope per cheat-sheet.

### 6. AllowedToStartInMultiplayer=no

Standard flag preventing the Prism Tank from being a starting unit. Can
only be built mid-game. Same flag as Kirov, MCV-rebuild edge cases.

### 7. No Crewed=yes

Default `Crewed=no` for vehicles — no infantry eject. Probably a thematic
choice: the Prism Tank's prism array is automated, not crewed.

---

## TS-legacy filter

- `ZFudgeTunnel=13` — TS-legacy field, dormant in YR.
- `;MovementZone=Normal ;gs FLAW needs to be changed to this when The
  Flaw is fixed` — Westwood-internal bug they never fixed. Not TS-legacy;
  just an unfixed quirk that shipped. Effective `MovementZone=Destroyer`
  in the live game.
- No `ImmuneToVeins`, no `Subterranean`, no other TS-only fields.
- **The "abusive" turret-changing comment IS NOT TS-legacy** — it's a
  YR-specific Westwood workaround for the charge-turret system. YR-active.

---

## Comparison with peer tier-3 Allied vehicles

| Field | SREF Prism Tank | MGTK Mirage Tank | BFRT Battle Fortress |
|-------|-----------------|-------------------|----------------------|
| Strength | **150** | 200 | 600 |
| Armor | light | medium | heavy |
| Speed | 4 | 5 | 4 |
| Cost | 1200 | 1000 | ~2000 |
| TechLevel | 8 | 6 | 10 |
| Prereq | GAWEAP,GATECH | GAWEAP,GATECH | GAWEAP,GATECH |
| Primary | Comet (anti-structure) | MirageGun (anti-tank) | 20mmRapid (anti-everything) |
| Range | **10** | 7 | 5 |
| Damage/shot | 100 | 70 | 30 |
| Chain shrapnel | **yes** (5+15 chain) | no | no |
| Crewed | no | no | yes (passengers) |

**Prism Tank's role:** Siege artillery. Stand-off from defenses,
chain-blast clustered targets. Squishy and slow — must be protected by
escort (typically Mirage Tanks for anti-tank + Battle Fortress for
front-line tankiness).

---

## Cross-references

- [BFRT.md](../allied/BFRT.md) — Battle Fortress, shares the SREF voxel
  art (`Image=SREF` in BFRT artmd).
- [MGTK.md](../allied/MGTK.md) — Mirage Tank, peer tier-3 Allied
  vehicle (anti-tank instead of anti-structure).
- [LASER_DRAW_CLASS_GHIDRA_REPORT.md](../../LASER_DRAW_CLASS_GHIDRA_REPORT.md)
  — laser-draw renderer used by IsLaser weapons.
- [DISK_LASER_CLASS_GHIDRA_REPORT.md](../../DISK_LASER_CLASS_GHIDRA_REPORT.md)
  — DISK's specialized laser-draw subclass for ring effect.
- [YTNK.md](../yuri/YTNK.md) — Gattling Tank, uses the same multi-turret
  WeaponStages syntax for different ends.
- [FV.md](../allied/FV.md) — IFV, the canonical multi-turret weapon-swap
  user (TurretCount=4, WeaponCount=17).

---

## Ghidra audit log (audit iteration 22 — 2026-05-18)

**Methodology**: SREF introduces a **NEW parser-function scope**
(`BulletTypeClass__ReadINI`) for the `ShrapnelWeapon`/`ShrapnelCount`
keys — the first BulletType-scope addition since the cumulative cheat
sheet's WeaponType/WarheadType/Techno/Unit/Building/Infantry/Object
parsers. This audit verifies all SREF claims AND fully decompiles
`BulletTypeClass__ReadINI` for the cumulative — yielding 35+ NEW
BulletType offsets. ~12 Ghidra queries: 4 string searches + 3 xref
lookups + 1 grep + 1 get_function_by_address + 1 full
BulletTypeClass__ReadINI decompile.

### Negative claim re-verified

| Query | Result |
|-------|--------|
| `search_strings("^SREF$")` | **0 matches** |

Confirms doc's "no hardcoded section-name branch" claim.

### String + parser xref verification (BINARY-VERIFIED)

All 3 doc-cited claims verify exactly + 1 bonus:

| String | Addr | Parser xref | Function |
|--------|------|-------------|----------|
| `IsChargeTurret` | 0x0084432C | 0x00712885 | TechnoTypeClass__ReadINI |
| `ShrapnelWeapon` | 0x0081B03C | 0x0046C2EC | **BulletTypeClass__ReadINI** ← **NEW PARSER SCOPE** |
| `ShrapnelCount` (bonus) | 0x0081B02C | 0x0046C319 | BulletTypeClass__ReadINI |

### NEW function entry: `BulletTypeClass__ReadINI`

| Function | Entry | Body | Status |
|----------|-------|------|--------|
| `BulletTypeClass__ReadINI` | `0x0046BEE0` | `0x0046BEE0–0x0046C435` | **Fully decompiled this pass**. Sole parser for ~35 BulletType-scope keys. First entry from this address-space range in the cumulative parser table — separate from WeaponTypeClass__ReadINI (0x00772xxx) and WarheadTypeClass__ReadINI. Calls `ObjectTypeClass__ReadINI(param_2)` first (BulletType inherits from ObjectType). |

### NEW TechnoType offset BINARY-VERIFIED

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0x810` | `IsChargeTurret` | byte | `*(undefined1*)(param_1 + 0x204) = uVar3` after ReadBool. **NEW**. Slots cleanly between audit-18 +0x80C WeaponCount and +0x814 gunner-table — consistent with the doc's observation that IsChargeTurret only fires when the multi-turret weapon system is active. |

### NEW BulletType offsets BINARY-VERIFIED (35+ entries — first BulletType-scope audit)

The BulletType has a 256+ byte body with 35+ keys parsed in this layout (param_1 is int, so offsets are direct byte offsets):

| Offset | INI key | Type |
|--------|---------|------|
| `+0x1F8..+0x210` | Image (char[25] string) | — |
| `+0x294` | Airburst | byte |
| `+0x295` | Floater | byte |
| `+0x296` | SubjectToCliffs | byte |
| `+0x297` | SubjectToElevation | byte |
| `+0x298` | SubjectToWalls | byte |
| `+0x299` | VeryHigh | byte |
| `+0x29A` | Shadow | byte |
| `+0x29B` | Arcing | byte |
| `+0x29C` | Dropping | byte |
| `+0x29D` | Level | byte |
| `+0x29E` | Inviso | byte |
| `+0x29F` | Proximity | byte |
| `+0x2A0` | Ranged | byte |
| `+0x2A1` | !Rotates (inverted bool) | byte |
| `+0x2A2` | Inaccurate | byte |
| `+0x2A3` | FlakScatter | byte |
| `+0x2A4` | (unknown — DAT_0081b09c) | byte |
| `+0x2A5` | (unknown — DAT_0081b098) | byte |
| `+0x2A6` | Degenerates | byte |
| `+0x2A7` | Bouncy | byte |
| `+0x2A8` | AnimPalette | byte |
| `+0x2A9` | FirersPalette | byte |
| `+0x2AC` | Cluster | int |
| `+0x2B0` | AirburstWeapon | WeaponType* |
| **`+0x2B4`** | **ShrapnelWeapon** | WeaponType* (the SREF claim) |
| **`+0x2B8`** | **ShrapnelCount** | int |
| `+0x2BC` | DetonationAltitude | int |
| `+0x2C0` | Vertical | byte |
| `+0x2C8..+0x2CF` | Elasticity | double (8 bytes) |
| `+0x2D0` | Acceleration | int |
| `+0x2D4` | Color | int (RGB) |
| `+0x2D8` | Trailer | AnimType* |
| `+0x2DC` | (unknown — DAT_0081b164) | int |
| `+0x2E0` | CourseLockDuration | int |
| `+0x2E4` | SpawnDelay | int |
| `+0x2EC` | Scalable | byte |
| `+0x2F0` | (unknown — DAT_0081b168) | int |
| `+0x2F4` | AnimLow | byte |
| `+0x2F5` | AnimHigh | byte |
| `+0x2F6` | AnimRate | byte |
| `+0x2F7` | Flat | byte |

(BulletType inherits ObjectType, so the audit-21 ObjectType offsets +0x7E Image, +0x9C Armor, +0x1E8 NoSpawnAlt, +0x22D Crushable, etc. ALSO apply to BulletType — the `Image=` field at +0x7E is the ObjectType-scope one; the BulletType-specific image at +0x1F8 is parsed separately by the conditional block at the end of BulletTypeClass__ReadINI.)

### Shrapnel chain mechanic (BINARY-VERIFIED at parser layer)

The doc's §3 chain math (basic 6 hits, elite 21 hits) depends on the
`ShrapnelWeapon` + `ShrapnelCount` BulletType fields, NOW PINNED at
+0x2B4 and +0x2B8. The runtime spawn-mechanism (when a bullet impacts,
fire N shrapnel-weapons at nearby targets) is DEFERRED — that's a
BulletClass-side (instance) consumer, not the parser. The parser side
is BINARY-VERIFIED.

### Items NOT re-verified in this pass (DEFERRED)

- The shrapnel-spawn consumer chain (BulletClass impact handler that
  fires ShrapnelCount instances of ShrapnelWeapon).
- The IsChargeTurret consumer chain (the pre-fire charge animation
  trigger in UnitClass).
- `LASER_DRAW_CLASS_GHIDRA_REPORT.md` cross-reference (doc's §7.4) —
  trust-chain only.
- `IsLaser` / `IsHouseColor` consumer chains (the laser-draw renderer).
- The 2 unknown BulletType bytes at +0x2A4 and +0x2A5 (parser-call
  format `&DAT_0081b09c`/`&DAT_0081b098` — INI keys not pinned).

### Confidence summary

- **HIGH**: 4 string addresses + 3 parser xrefs (all exact); 1 NEW
  TechnoType struct offset (IsChargeTurret +0x810); **1 NEW parser
  function fully decompiled (BulletTypeClass__ReadINI) — the first
  BulletType-scope addition to the cheat sheet**; 35+ NEW BulletType
  offsets including the SREF claim (ShrapnelWeapon +0x2B4 + ShrapnelCount
  +0x2B8). This is the biggest cumulative addition since audit 21
  (ObjectTypeClass__ReadINI full decompile).
- **MEDIUM**: 2 unknown INI keys at BulletType +0x2A4 / +0x2A5 (parser
  reads from unnamed DAT addresses — keys not exposed).
- **No INCORRECT findings**. All 3 doc-cited claims verify exactly.
  The "abusive turret-changing block" interpretation is supported by
  the offset pattern (IsChargeTurret +0x810 lives in the gunner-cluster
  at +0x808/+0x80C/+0x810/+0x814).

---

## Coverage audit

- [x] Every rulesmd key annotated (~45 keys).
- [x] Every artmd key annotated (5 keys).
- [x] Multi-turret/Weapon1= syntax fully explained.
- [x] Both weapons documented (Comet basic + SuperComet elite).
- [x] All 4 projectiles documented (LargeCometP, SuperCometP, plus the
  shrapnel-chain SmallCometP/SuperSmallCometP feeding into
  CometFragment/SuperCometFragment).
- [x] Both fragment weapons documented (CometFragment, SuperCometFragment).
- [x] Warhead documented (CometWH — anti-structure 200% vs wood/steel/
  concrete).
- [x] Chain-effect math (basic 6 hits, elite 21 hits) computed.
- [x] All voice/sound bindings documented.
- [x] Prerequisites: `GAWEAP, GATECH`.
- [x] Owner: 5 Allied houses (4 sub-factions + Alliance).
- [x] Veterancy + Elite weapon swap (Comet → SuperComet).
- [x] Hardcoded behavior: multi-turret syntax abuse, IsChargeTurret,
  ShrapnelWeapon BulletType-scope, IsLaser + IsHouseColor visuals,
  AllowedToStartInMultiplayer=no, no Crewed.
- [x] TS-legacy filter: ZFudgeTunnel dormant; MovementZone "Flaw"
  unfixed Westwood bug.
- [x] Comparison table with peer tier-3 Allied vehicles.
- [x] At least one Ghidra search performed (`IsChargeTurret`,
  `ShrapnelWeapon` — including a **new ReadINI scope discovered**).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("IsChargeTurret")` | `0x0084432c` (single match) |
| `get_xrefs_to(0x0084432c)` | `0x00712885 → TechnoTypeClass__ReadINI` |
| `search_strings("ShrapnelWeapon")` | `0x0081b03c` (single match) |
| `get_xrefs_to(0x0081b03c)` | `0x0046c2ec → BulletTypeClass__ReadINI` **(NEW SCOPE)** |

**New cheat-sheet entries:**
- `IsChargeTurret` (0x0084432c → 0x00712885) TechnoType — gates the
  pre-fire charge-up animation; requires the multi-turret weapon system
  to be active.
- `ShrapnelWeapon` (0x0081b03c → 0x0046c2ec) **BulletType** — the
  projectile-level chain-fire setting. **First BulletType-scope entry
  in the cheat-sheet.** BulletTypeClass__ReadINI lives in the
  `0x0046xxxx` range; this is a separate reader function from
  WeaponTypeClass / WarheadTypeClass / TechnoTypeClass.

**Open questions:**
- CometWH has no explicit `InfDeath=` — does it default to small-arms
  death anim (1) or to a different default? Verify next time we audit
  the InfDeath table.
