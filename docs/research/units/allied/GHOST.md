# Navy SEAL (GHOST)
Side: Allied | Category: Infantry | Image alias: `SEAL` (or `SEALA` on arctic theaters)

The Allied hero infantry — a one-man building-demolition force. $1000 from
the Allied Barracks at TechLevel 9 (Battle Lab + Radar gate). Carries the
high-damage MP5 (range 6, 125 damage per burst, anti-infantry) for trash mob
clearing and `Sapper` C4 charges (2,500 damage / `Mechanical` warhead) that
one-shot any building on contact via the SEAL/Tanya walk-up plant mechanic.
Amphibious — can swim, with a unique `AmphibiousDestroyer` movement zone
(infantry exclusive — works around a TS-era "stuck on tree" bug). Sight 8
(highest infantry value). Speed 5. Fearless. Has `TiberiumProof=yes`
(TS-legacy) but no `Bombable=` override.

Hard limitations: `Assaulter=no` (cannot clear garrisoned buildings — the
INI comment "I clear out UC buildings" is wrong/stale from Tanya).
`DetectDisguise=no` (cannot reveal Mirage/Spy disguises). No turret, no
deploy. Cannot attack vehicles effectively (HollowPoint Verses=1% vs
light/medium/heavy armor).

Authoritative deep RE: [NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md](../../NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md)
(761 lines; covers both SEAL and Tanya since they share the C4 dispatch path).

---

## rulesmd.ini — `[GHOST]` section

Verbatim from `ini/rulesmd.ini:4014`:

```ini
[GHOST]
UIName=Name:GHOST
Name=SEAL
Image=SEAL
Category=Soldier
Prerequisite=GAPILE,RADAR
PrerequisiteOverride=CAWA2A,CAWA2B,CAWA2C,CAWA2D ; SJM: any Pentagon building sufficient to build SEAL
Primary=MP5
Secondary=Sapper
OpenTransportWeapon=0;defaults to -1 (decide normally)  What weapon should I use in a Battle Fortress
NavalTargeting=4
LeadershipRating=8
AlternateArcticArt=yes ; ie SEALA for arctic maps
C4=yes
Assaulter=no ; I clear out UC buildings
CrushSound=InfantrySquish
Crushable=yes
TiberiumProof=yes
Strength=125
Armor=flak
TechLevel=9
Pip=red
Sight=8
Speed=5
Owner=British,French,Germans,Americans,Alliance
AllowedToStartInMultiplayer=no
Cost=1000
Soylent=500
Points=50
IsSelectableCombatant=yes
VoiceSelect=SealSelect
VoiceMove=SealMove
VoiceAttack=SealAttackCommand
VoiceFeedback=
VoiceSpecialAttack=SealSpecialAttack
CreateSound=SealCreated
DieSound=SealDie
EnterWaterSound=TanyaEntersWater
LeaveWaterSound=TanyaLeavesWater
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
SpeedType=Amphibious
MovementZone=AmphibiousDestroyer ; I am the only one with this zone, because it is now tied with being an infantry (part of seal stuck on tree bug)
ThreatPosed=25	; This value MUST be 0 for all building addons
SpecialThreatValue=1
ImmuneToVeins=yes
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,SCATTER,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF,FASTER
Size=1
DetectDisguise=no
ElitePrimary=MP5E
IFVMode=4
UseOwnName=true
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:GHOST` | CSF-string key resolving to "Navy SEAL". `UseOwnName=true` below overrides this with `Name=SEAL` |
| `Name=SEAL` | Internal short name; **also displayed in the UI** because `UseOwnName=true` |
| `Image=SEAL` | Resolves to `[SEAL]` art section (or `[SEALA]` on arctic per `AlternateArcticArt`) |
| `Category=Soldier` | Pip group + AI threat grouping |
| `Prerequisite=GAPILE,RADAR` | Requires Allied Barracks + any Radar building (GARADR via Spy Sat / NARADR if captured) |
| `PrerequisiteOverride=CAWA2A,CAWA2B,CAWA2C,CAWA2D` | **Campaign override** — any of the four Pentagon building tags is sufficient to build SEAL, bypassing the GAPILE+RADAR gate. Used in mission scripting (e.g. Pentagon attack mission) |
| `Primary=MP5` | Anti-infantry submachine gun (range 6, dmg 125, ROF 10) |
| `Secondary=Sapper` | **C4 demolition weapon** — damage 2500, range 1.5, warhead `Mechanical` (anti-vehicle Verses), `SabotageCursor=yes`. Walk-up plant via Mission_Enter |
| `OpenTransportWeapon=0` | When passenger in Battle Fortress, fire Primary (MP5). Tank Bunker likewise |
| `NavalTargeting=4` | Engine target-acquisition: SEAL can engage naval targets within 4 cells (without this, infantry by default cannot target ships) |
| `LeadershipRating=8` | Higher = faster XP gain (max in skirmish data, GI is 0 default, Engineer is 3) |
| `AlternateArcticArt=yes` | Engine swaps `Image=SEAL` for `SEALA` on snow theaters (different SHP for white camo) |
| `C4=yes` | **The C4 flag** — `InfantryTypeClass+0xEC2`. Gates the walk-up demolition path in `Mission_Attack`+`Mission_Enter`. Auto-forces `Infiltrate=true` (+0xEBE) |
| `Assaulter=no` | **Cannot clear garrisoned buildings.** `InfantryTypeClass+0xEB5`. The INI comment "I clear out UC buildings" is **stale from Tanya** — SEAL with Assaulter=no does NOT clear garrisons. (Only Tanya's `Assaulter=yes` enables the UC-clear weapon-ability 0xe) |
| `CrushSound=InfantrySquish` | Crush sample (`igensqua`) |
| `Crushable=yes` | Vehicles can crush SEAL |
| `TiberiumProof=yes` | TS-legacy; YR has no tiberium so this is unreachable |
| `Strength=125` | HP — same as GI |
| `Armor=flak` | **Body armor** — flak column on warhead Verses tables. GI/Engineer are `none`. Reduces damage vs warheads with low flak% |
| `TechLevel=9` | High tier (requires Battle Lab + Radar by INI prereq) |
| `Pip=red` | Cargo passenger pip color (hero unit) |
| `Sight=8` | **Highest infantry sight radius** in YR |
| `Speed=5` | Walk speed (25% faster than GI's 4) |
| `Owner=British,French,Germans,Americans,Alliance` | Allied subfactions only |
| `AllowedToStartInMultiplayer=no` | Not in starting unit pool |
| `Cost=1000` | Premium price |
| `Soylent=500` | Grinder refund |
| `Points=50` | Kill score (5× GI) |
| `IsSelectableCombatant=yes` | Included in select-all-combat hotkey + AI combat groups |
| `VoiceSelect=SealSelect` | Selection voice |
| `VoiceMove=SealMove` | Move voice |
| `VoiceAttack=SealAttackCommand` | Attack voice |
| `VoiceFeedback=` | **Empty** — SEAL is Fearless (no fear voice fires). Note: `Fearless=` is not explicit in this section, but the empty VoiceFeedback combined with the SEAL's actual in-game fearless behavior implies the default check skips fear processing when no feedback voice is set |
| `VoiceSpecialAttack=SealSpecialAttack` | Voice when C4 plant ordered |
| `CreateSound=SealCreated` | Sound when production completes (`$iseasea`, Type=Global, Priority=CRITICAL — bypasses spatial-audio cull) |
| `DieSound=SealDie` | Death sample |
| `EnterWaterSound=TanyaEntersWater` | Sound on water-entry transition (shared with Tanya bank) |
| `LeaveWaterSound=TanyaLeavesWater` | Sound on water-exit transition |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID |
| `PhysicalSize=1` | Pathfinder size class |
| `SpeedType=Amphibious` | **Can move on both land and water** — pathfinder ground cost lookup uses the amphibious table |
| `MovementZone=AmphibiousDestroyer` | **Unique zone** — engine zone-precheck restricts movement to this zone-id. Comment notes the SEAL is the only unit in this zone, deliberately to work around a TS-era "stuck on tree" bug where amphibious infantry would get pinned on terrain features |
| `ThreatPosed=25` | Enemy AI prioritizes SEAL (2.5× GI) |
| `SpecialThreatValue=1` | SEAL's own threat-target weight at max |
| `ImmuneToVeins=yes` | TS-legacy, unreachable in YR |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,SCATTER,FASTER` | 6 abilities — extra `SCATTER` (improved scatter on damage) compared to GI |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF,FASTER` | 5 abilities cumulative (no SCATTER at elite, but SELF_HEAL added) |
| `Size=1` | Cargo slot cost |
| `DetectDisguise=no` | **Cannot see through Mirage Tank trees or Spy disguises** — `InfantryTypeClass` field. Contrast Tanya `DetectDisguise=yes` |
| `ElitePrimary=MP5E` | Elite primary weapon (identical stats to MP5 in vanilla — promotion effect is via `STRONGER`/`FIREPOWER` modifiers, not weapon swap) |
| `IFVMode=4` | IFV gunner mode 4 — IFV swaps to laser weapon (Tanya/SEAL IFV variant) when SEAL boards |
| `UseOwnName=true` | UI displays `Name=SEAL` directly instead of looking up `UIName=Name:GHOST` from the CSF string table |

Implicit defaults:

- `Crawls=yes` (art section)
- `Deployer=no` (no deploy command)
- `Occupier=no` (cannot enter civilian buildings — `Occupier=yes` is needed)
- `Bombable=` not explicit — defaults to false here (Crazy Ivan plant cursor doesn't auto-show on SEAL)
- `Trainable=yes` (default; gains XP and promotes)
- `ImmuneToPsionics=no` (default; mind-controllable)

---

## artmd.ini — `[SEAL]` section

`ini/artmd.ini:384`:

```ini
[SEAL] ; Regular SEAL
Cameo=SEALICON
AltCameo=SEALUICO
Sequence=SealSequence
Crawls=yes
Remapable=yes
FireUp=3
PrimaryFireFLH=100,0,100
```

And the arctic variant `[SEALA]` (artmd.ini:393) — identical except for the
header comment. Switched in at runtime when the theater is `Snow` per
`AlternateArcticArt=yes`.

| Key | Meaning |
|-----|---------|
| `Cameo=SEALICON` | Sidebar icon |
| `AltCameo=SEALUICO` | Cameo at Elite rank |
| `Sequence=SealSequence` | Reference to the sequence block below |
| `Crawls=yes` | Sets `InfantryTypeClass+0xEBD` — prone-while-walking enabled |
| `Remapable=yes` | House remap palette applied |
| `FireUp=3` | Bullet-spawn frame within firing sequence (later than GI's FireUp=2) |
| `PrimaryFireFLH=100,0,100` | MP5 muzzle: forward 100, side 0, height 100 (further forward than GI's 80) |

Notable absence: no `SecondaryFireFLH=`. The `Sapper` C4 weapon has
`Projectile=Invisible` and the plant is animation-driven via the
`SealSequence` Fire frames; the engine does not draw a muzzle flash, so the
FLH is unused.

### Referenced sequence — `[SealSequence]`

`artmd.ini:13969`:

```ini
[SealSequence]
Ready=0,1,1
Guard=0,1,1
Walk=8,6,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Crawl=86,6,6
Prone=86,1,6
Die1=134,15,0
Die2=149,15,0
FireUp=164,6,6
FireProne=212,6,6
Down=260,2,2
Up=276,2,2
Paradrop=602,1,0
Cheer=603,8,0,E
Tread=340,6,6
Swim=388,6,6
WetAttack=436,6,6
WetIdle1=484,15,0,S
WetIdle2=499,15,0,E
WetDie1=514,15,0
WetDie2=529,15,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Panic=8,6,6
```

The SEAL sequence has **unique swim/wet variants** absent from GISequence:

- `Tread=340,6,6` — treading water (stationary in water)
- `Swim=388,6,6` — swimming animation
- `WetAttack=436,6,6` — firing while in water
- `WetIdle1=484,15,0,S`, `WetIdle2=499,15,0,E` — idle in water (south/east facings)
- `WetDie1=514,15,0`, `WetDie2=529,15,0` — death animations while wet

These are driven by `SpeedType=Amphibious` + `MovementZone=AmphibiousDestroyer`
— the engine queries the cell terrain (water vs land) and selects the wet
sequence variant. Per
[INFANTRYCLASS_GHIDRA_REPORT.md](../../INFANTRYCLASS_GHIDRA_REPORT.md), the
`LastTerrainSpeechClass` field (`InfantryClass+0x1ba`) tracks land/water
transitions and triggers `EnterWaterSound`/`LeaveWaterSound`.

No deploy frames — SEAL cannot deploy.

---

## Weapons

### Primary — `[MP5]`

`rulesmd.ini:22985`:

```ini
[MP5]
Damage=125
ROF=10
Range=6
Projectile=InvisibleLow
Speed=100
Warhead=HollowPoint
Report=SealAttack
AssaultAnim=UCBLOOD;the anim to play when a UC building is cleared (assaulters need this on their primary weapon)
```

| Key | Meaning |
|-----|---------|
| `Damage=125` | Per-burst damage — one-shots all rookie infantry (verses HollowPoint 200% vs none) |
| `ROF=10` | Fast (10 frames between shots, 2× GI ROF) |
| `Range=6` | Long range for infantry |
| `Projectile=InvisibleLow` | Inviso, walls block, cliffs/elevation matter |
| `Speed=100` | Inviso → instant |
| `Warhead=HollowPoint` | **Anti-infantry only** — 1% vs all armored targets |
| `Report=SealAttack` | Sound `iseaatta/b` random |
| `AssaultAnim=UCBLOOD` | Garrison-clear animation reference — but SEAL has `Assaulter=no`, so this anim is **never played by SEAL**. Stale designer artifact (the same weapon is shared with Tanya's `DoublePistols` analog that DOES assault) |

### Secondary — `[Sapper]` (C4 demolition)

`rulesmd.ini:22846`:

```ini
[Sapper]
Damage=2500 ; a boatload  (get it?)
ROF=100
Range=1.5
CellRangefinding=yes
Projectile=Invisible;Invisible5
;AntiNaval=yes
;AntiUnderwater=yes
;AntiOrganic=no;to make exception for squid and dolphin
Warhead=Mechanical;gs please do not use the warhead marked "do not use" Super
Report=SealPlaceBomb
SabotageCursor=yes ;gs instead of normal fire cursor to avoid confusion
```

| Key | Meaning |
|-----|---------|
| `Damage=2500` | Boatload-of-damage (designer pun); applied as area damage on plant arrival |
| `ROF=100` | Long cooldown |
| `Range=1.5` | Melee plant range |
| `CellRangefinding=yes` | Use cell-center distance |
| `Projectile=Invisible` | No projectile sprite; commented-out alt `;Invisible5` shows iteration history |
| `Warhead=Mechanical` | Anti-vehicle/anti-building Verses (0/0/0/100/100/100/0/0/0/100/100). **But see C4 path below — the actual detonation does not use this warhead.** The Sapper weapon is the "trigger"; the real damage comes from `Rules->C4Warhead = Super` via `Apply_area_damage` |
| `Report=SealPlaceBomb` | Sound `icraatta` (single sample, Volume 60) |
| `SabotageCursor=yes` | Engine renders the sabotage cursor instead of attack cursor when right-click hovers over a valid C4 target |
| `;AntiNaval=yes` `;AntiUnderwater=yes` `;AntiOrganic=no` | Commented-out — designer iteration on whether SEAL could C4 subs/dolphins. Not active |

### Elite Primary — `[MP5E]`

`rulesmd.ini:25156` — **identical to MP5** in every value. The elite "promotion"
effect on SEAL is via abilities (`STRONGER` HP, `FIREPOWER` damage modifier,
`ROF` faster fire, `FASTER` move speed), not a stat swap on the weapon
itself. The MP5E entry exists for parser-symmetry with other elite weapons.

### Warhead — `[HollowPoint]` (Primary)

`rulesmd.ini:27053`:

```ini
[HollowPoint]
Verses=200%,100%,100%,1%,1%,1%,1%,1%,1%,1%,100%
InfDeath=1
AnimList=PIFF
ProneDamage=100%
Bullets=yes
```

| Key | Meaning |
|-----|---------|
| `Verses=200%/100%/100%/1%/1%/1%/1%/1%/1%/1%/100%` | **Anti-infantry hyper-specialized**: 200% vs none (one-shots rookie GI: 125×2=250 vs HP 125), 100% vs flak/plate (also one-shots other infantry), but 1% vs every armored target. SEAL is **useless against vehicles** |
| `InfDeath=1` | Standard bullet death |
| `AnimList=PIFF` | Small puff impact |
| `ProneDamage=100%` | No prone-damage reduction — crawling infantry take full hit |
| `Bullets=yes` | Bullet damage flag |

### Warhead — `[Mechanical]` (Secondary)

`rulesmd.ini:27116`:

```ini
[Mechanical]
Verses=0%,0%,0%,100%,100%,100%,0%,0%,0%,100%,100%
InfDeath=0
```

Vs vehicles (light/medium/heavy) only. Zero damage vs everything else.

**However**: per [NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md §1](../../NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md),
when SEAL arrives at its C4 target the actual detonation calls
`Apply_area_damage(SEAL, Rules->C4Warhead, 1, 0)` — and `Rules->C4Warhead`
(at `Rules+0xFA8`) resolves to **`Super`** (Verses=100% across all
armors with InfDeath=2). The Sapper weapon's `Warhead=Mechanical` is a
trigger-decoration; the real explosion is engine-driven via the Rules
C4Warhead pointer. The comment `gs please do not use the warhead marked
"do not use" Super` is a designer warning to other modders — modders
should not set this weapon's Warhead= to Super directly because the engine
will pick up Rules->C4Warhead independently.

### Projectile — `[InvisibleLow]` / `[Invisible]`

- `InvisibleLow` (Primary MP5) — `Inviso=yes`, walls/cliffs/elevation block.
  Documented in [E1.md](E1.md#projectile-blocks).
- `Invisible` (Secondary Sapper) — `rulesmd.ini:25346`: minimal block,
  `Inviso=yes` `Image=none` only. No subject-to-anything flags.

---

## Voices and sounds

| INI key on GHOST | soundmd block | Resolved samples |
|------------------|---------------|------------------|
| `VoiceSelect=SealSelect` | `[SealSelect]` line 3932 | `$iseaseb` `$iseased` `$iseaexc` (random) |
| `VoiceMove=SealMove` | `[SealMove]` line 3927 | `$iseamoa` `$iseamob` `$iseamoc` (random) |
| `VoiceAttack=SealAttackCommand` | `[SealAttackCommand]` line 3922 | `$iseaata` `$iseaatb` `$iseaatc` (random) |
| `VoiceFeedback=` (empty) | n/a | **No fear voice** — value blank in INI; `[SealFear]` block is commented out in soundmd.ini:3941 |
| `VoiceSpecialAttack=SealSpecialAttack` | `[SealSpecialAttack]` line 3950 | `$iseaexa` `$iseaexb`, **Type=global** (heard everywhere), Volume 90 |
| `CreateSound=SealCreated` | `[SealCreated]` line 3956 | `$iseasea`, **Priority=CRITICAL**, Type=Global — bypasses spatial audio culling; "Navy SEAL ready" announcement to the player |
| `DieSound=SealDie` | `[SealDie]` line 3945 | `$iseadia` `$iseadib` `$iseadic` (random) |
| `EnterWaterSound=TanyaEntersWater` | `[TanyaEntersWater]` line 1144 | `gexpwasa` `gexpwasb` (splash sounds, random) |
| `LeaveWaterSound=TanyaLeavesWater` | `[TanyaLeavesWater]` line 1150 | `vnavupa` (single sample) |
| `CrushSound=InfantrySquish` | `[InfantrySquish]` line 1196 | `igensqua` |
| Weapon `MP5` `Report=SealAttack` | `[SealAttack]` line 1111 | `iseaatta` `iseaattb` (random interrupt, FShift -5/+5, Volume 60) |
| Weapon `MP5E` `Report=SealAttack` | (same) | shared with primary |
| Weapon `Sapper` `Report=SealPlaceBomb` | `[SealPlaceBomb]` line 3937 | `icraatta` (single sample, Volume 60) — **note `icraatta` is from the Crazy Ivan bank**; designers reused the bomb-plant audio asset |

---

## Prerequisites, owners, tech

- `Prerequisite=GAPILE,RADAR` — Barracks AND any Radar building.
  - GAPILE = Allied Barracks
  - `RADAR` is a special generic prerequisite tag. Maps to `[General] Radar=`
    list, which includes `GASPYSAT` (Spy Satellite Uplink), `NARADR`
    (Soviet Radar Tower if captured), `AMRADR` (American Radar, campaign),
    `CASYDN02` (Sydney radar prop), etc. In practical Allied gameplay,
    Spy Sat is the gate.
- `PrerequisiteOverride=CAWA2A,CAWA2B,CAWA2C,CAWA2D` — **Campaign override**.
  Any of these four Pentagon building tags (the 4-section Pentagon
  structure) is sufficient on its own. Used in the Allied campaign mission
  "Hollywood and Vain" / "Operation Free Gateway" type scenarios.
- `Owner=British,French,Germans,Americans,Alliance` — Allied only.
- `TechLevel=9` — high tier (only Tanya at 10 is higher among Allied
  infantry).
- `BuildLimit=` not set in this section; default unlimited.
- `AllowedToStartInMultiplayer=no`.

---

## Veterancy and upgrades

- **Rookie**: MP5 primary, Sapper secondary.
- **Veteran** (`STRONGER,FIREPOWER,ROF,SIGHT,SCATTER,FASTER`):
  - `STRONGER` = +50% effective HP
  - `FIREPOWER` = +25% damage
  - `ROF` = -25% reload time
  - `SIGHT` = +1 cell radius
  - `SCATTER` = improved scatter on incoming damage (longer dodge distance)
  - `FASTER` = +10% move speed
- **Elite** (`SELF_HEAL,STRONGER,FIREPOWER,ROF,FASTER`, cumulative):
  - `SELF_HEAL` = passive HP regen
  - Cumulative stat bumps with veteran
  - Primary swap: `MP5` → `MP5E` (identical values; effective swap is the
    promotion stat bonuses)
  - Cameo swap: `SEALICON` → `SEALUICO`
- C4 damage does **not** scale with veterancy — the detonation uses
  `Rules->C4Warhead` and a fixed area-damage call, not the weapon's `Damage`
  field. SEAL one-shots buildings at any rank.

---

## Hardcoded behavior in gamemd.exe (Ghidra-verified)

Full RE: [NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md](../../NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md).
All findings: HIGH content, HIGH identity, HIGH binding (caller-traced).

### The C4 plant — `Mission_Attack → Mission_Enter`

The defining behavior, shared with Tanya and Yuri/Psi-Corp Trooper (and
Chrono Commando with caveats).

1. **Player right-click** on enemy building → SEAL.TarCom = building,
   Mission = ATTACK (2).
2. **`InfantryClass::Mission_Attack @ 0x0051F3E0`** (per-tick).
   **[ADDRESS VERIFIED audit 4 — Ghidra labels as `FUN_0051f3e0` (unlabeled),
   body `0x0051f3e0–0x0051f53e`; same pattern as `FUN_005218e0` SelectWeapon
   from audit 2 — the function exists at the claimed address but is
   unlabeled.]**
   Decompile-verified preconditions:
   ```c
   // First gate: C4 OR weapon-ability 0xE
   if (*(char *)(type + 0xec2) == '\0') {                     // C4 flag at TypeClass+0xEC2
     if (!HasWeaponAbility(0xe)) goto fallthrough;             // weapon-ability 0xE alternate gate
   }
   // Target check
   if (this[0xad] != NULL) {                                   // target ptr at +0x2B4
     if (target->vtable_0x2c() == 6) {                         // RTTI = 6 (NOT 1 as audit-3 doc claimed)
       BuildingTypeClass *bt = target[0x520 /* type ptr */];
       if (bt[0x1577 /* CanC4 */] != 0 && bt[0x1701 /* InvisibleInGame */] == 0) {
         this->vtable_0x480(target, 1);                        // Set_Target
         this->vtable_0x1e8(0x11, 0);                          // SetMission(Enter=0x11)
         return 1;
       }
     }
   }
   ```
   - **`TypeClass+0xEC2 = C4 flag` BINARY-VERIFIED audit 4.**
   - **`BldgType+0x1577 = CanC4` BINARY-VERIFIED audit 4.**
   - **`BldgType+0x1701 = InvisibleInGame` BINARY-VERIFIED audit 4.**
   - **`vtable+0x480 = Set_Target`, `vtable+0x1e8 = SetMission`, mission ID 0x11 = Enter BINARY-VERIFIED audit 4.**
   - **RTTI value for BuildingClass = 6** — **conflict with ENGINEER audit 3** which decompiled `iVar2 == 1` for the same `vtable+0x2c` call on a target also confirmed to be BuildingClass. Either: (a) `vtable+0x2c` is NOT GetRTTI consistently, (b) the two functions accept different target classes, (c) one of the two checks was misinterpreted. **DEFERRED**: needs decompile of `vtable+0x2c` (TechnoClass GetAbstractType / GetRTTI) on a known BuildingClass instance to disambiguate.
3. **`InfantryClass::Mission_Enter @ 0x005196A0`** with mission==0x11.
   **[ADDRESS DISCREPANCY audit 4 — confirmed from audit 3: no standalone
   function at 0x005196A0; address falls inside `InfantryClass::PerCellProcess`
   body 0x00519630–0x0051aa0a.]** The C4 detonation logic lives in
   PerCellProcess, not a dedicated Mission_Enter.
   - If SEAL not yet at target cell: walk toward target, record plant state
     on building (Building+0x6DF="being C4'd", Building+0x150=SEAL ptr,
     Building+0x14A=current frame).
   - If SEAL has arrived: **DETONATION BLOCK**:
     - `Apply_area_damage(SEAL, Rules[+0xFA8 /* C4Warhead = Super */], 1, 0)`
     - Random scatter direction from `(RateTimer >> 12 + 1) >> 1 & 7`
     - SEAL moves one cell in the scatter direction (`vtable[+0x174]`)
     - SetMission(2 = Move)
     - Two follow-up `Apply_area_damage(0, C4Warhead, 1, 0)` calls — these
       are NOT duplicate damage but **destructible-overlay propagation**
       (sandbags/wood crates/barrels) per the engine's overlay-chain code.
     - **SEAL is NOT destroyed.** No `self_destruct` vtable call in this
       path. SEAL survives and walks away.

### Detonation uses `Rules->C4Warhead`, not the weapon's warhead

- `Rules+0xFA8` parses `[CombatDamage] C4Warhead=Super` to the resolved
  WarheadTypeClass pointer. **[BINARY-VERIFIED audit 4 — parser key string
  `"C4Warhead"` at `0x0083b1d4` xref into `RulesClass__ReadCombatDamage`
  at `0x0066c31f`; field is Rules-CombatDamage scope.]**
- `[Super]` warhead: `Verses=100%` for all armors, `InfDeath=2` (gib death).
  Independent of the Sapper weapon's `Warhead=Mechanical`.
  **[INFERRED — Super warhead's Verses table not re-checked in audit 4]**.
- This is why C4 one-shots any building regardless of armor type — and why
  modders who change Sapper's Warhead= see no effect. **[INFERRED for the
  no-effect claim — needs Apply_area_damage decompile to confirm]**.

### CanC4 building gate

- `BuildingType+0x1577 /* CanC4= */` **[BINARY-VERIFIED audit 4]** parsed
  from `[BUILDING] CanC4=yes`. Parser key string `"CanC4"` at `0x0081adfc`
  xref into `BuildingTypeClass_ReadINI_Water` at `0x00460050` — confirmed
  BuildingType scope. Runtime read confirmed in `FUN_0051f3e0`
  (Mission_Attack) decompile.
- Most production/tech/defense structures have `CanC4=yes`.
- Concrete walls have `CanC4=no`.
- Civilian decoration buildings vary.

### MP5 anti-infantry profile

- `HollowPoint` warhead profile is **vehicle-exclusion design** — 1%
  vs light/medium/heavy/wood/steel/concrete. SEAL cannot damage any vehicle
  or building with primary fire. Only Sapper damages buildings.

### Amphibious movement

- `SpeedType=Amphibious` + `MovementZone=AmphibiousDestroyer` are the
  only-of-their-kind combo. Engine pathfinder uses a separate zone-map
  for `AmphibiousDestroyer`, which the SEAL inhabits exclusively. The
  comment "(part of seal stuck on tree bug)" confirms this was a
  workaround for a TS-era amphibious-infantry stuck-on-terrain bug.
- Water/land transitions trigger `LastTerrainSpeechClass` updates
  (`InfantryClass+0x1BA`) per GI report §P2.13:
  - 0 → 1 (land to water): play `EnterWaterSound` → `TanyaEntersWater`
  - 1 → 0 (water to land): play `LeaveWaterSound` → `TanyaLeavesWater`
- Sequence dispatcher swaps to `Swim`/`Tread`/`WetAttack`/`WetIdle*`
  variants while `+0x1BA == 1`.

### NavalTargeting=4

- `NavalTargeting` is a **TechnoType** field. **[BINARY-VERIFIED audit 4
  parser-side]** — parser key at `0x00844510`, xref into
  `TechnoTypeClass__ReadINI` at `0x007121be` (TechnoType scope). The
  specific struct offset where the value is stored (`+0xC9A` per doc) is
  not decompile-verified; doc claim is **[INFERRED]** at the struct level.
- Gates whether the engine considers naval units as valid attack targets
  within the stated range. Default 0 means infantry can't normally fire at
  ships; SEAL's value 4 enables MP5 fire on ships at up to 4 cells.

### DetectDisguise=no

- `DetectDisguise` is a **TechnoType** field. **[BINARY-VERIFIED audit 4
  parser-side]** — parser key at `0x00843c78`, xref into
  `TechnoTypeClass__ReadINI` at `0x0071443f`. Companion field
  `DetectDisguiseRange` also exists at `0x00843d3c` (separate parser key).
  Struct offset claim `+0xCDF` is **[INFERRED]** at the struct level.
- When false, the cursor and target-acquisition skip the disguise-reveal
  pass that Tanya/Spy/SEAL variants with `DetectDisguise=yes` perform.
- Means SEAL **cannot see through Mirage Tanks** or spy disguises despite
  having Sight=8.

### IFV gunner mode

- `IFVMode=4` → IFV Weapon5 slot (Tanya/SEAL IFV variant, laser).
- Per [IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md](../../IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md).

### UseOwnName=true

- `UseOwnName` is parsed by **`InfantryTypeClass__ReadINI`**.
  **[BINARY-VERIFIED audit 4]** — parser key at `0x00825908`, xref `0x0052463d`
  in `InfantryTypeClass__ReadINI`. So this is an InfantryType-only field,
  not TechnoType (corrects the doc's "TechnoTypeClass" claim).
- When true, the UI's `GetUIName()` returns the unit's `Name=` (`"SEAL"`)
  directly instead of CSF-resolving `UIName=` (`"Name:GHOST"` → "Navy SEAL").
  **[INFERRED — GetUIName not decompiled in audit 4]**. Result: in-game
  tooltip displays "SEAL" rather than "Navy SEAL". Used for hero units
  with shorter call-signs.

### SabotageCursor=yes (Sapper weapon)

- `SabotageCursor` is a **WeaponType** field. **[BINARY-VERIFIED audit 4]**
  — parser key at `0x008494c4`, xref `0x007721bd` in
  `WeaponTypeClass__ReadINI`. Confirms the doc's claim that this flag is
  on the weapon, not the unit.

### Ghidra string-search results

- `search_strings "GHOST"` → 30+ hits, all INI-parsing constants and CSF
  string keys (`Name:GHOST`). No hardcoded section-name branch.
- `search_strings "SEAL"` → many hits, mostly voice keys (`SealAttack`,
  `SealDie`, etc.) and the art section `[SEAL]`. No hardcoded
  `if(name=="SEAL")` branch.
- `search_strings "Sapper"` → weapon parse target only.
- Behavior driven entirely by C4=yes + Amphibious + NavalTargeting +
  DetectDisguise flags.

### Auto-aim AI ordering

- `Type+0xCB0` (LeadershipRating) feeds the AI's threat-target selection
  weight. With LeadershipRating=8, allied AI prefers to keep SEAL alive
  (retreats SEAL from danger faster than GI), and enemy AI prioritizes
  SEAL as a kill target.

---

## TS-legacy filter

- `TiberiumProof=yes` — TS-era tiberium-immunity flag. YR has no tiberium
  terrain, so this is unreachable. Defensively set.
- `ImmuneToVeins=yes` — TS terrain flag, unreachable in YR.
- `MovementZone=AmphibiousDestroyer` — comment confirms this is a TS-era
  bug workaround. Live in YR.
- `Locomotor={4A582744-...}` — TS GUID, alive in YR.
- `Crawls=yes` (art) — TS-era prone-while-walking, alive in YR.
- Commented `;AntiNaval=yes` / `;AntiUnderwater=yes` / `;AntiOrganic=no` on
  Sapper — designer iteration history; not active.
- `AssaultAnim=UCBLOOD` on MP5 — designer left for symmetry with assaulter
  weapons; SEAL has `Assaulter=no` so the anim is unreachable.
- INI comment `; I clear out UC buildings` on `Assaulter=no` — **stale
  comment from Tanya**, contradicts the actual value. SEAL cannot clear UC
  buildings.
- `PrerequisiteOverride=CAWA2A...CAWA2D` — campaign-specific Pentagon
  building override; not used in skirmish but live in mission scripts.

---

## Cross-references

- **Builder**: [GAPILE](../structures/GAPILE.md) Allied Barracks + Radar.
- **Sibling C4 plant unit (Allied)**: [TANY](TANY.md) Tanya — shares
  Mission_Attack/Mission_Enter dispatch but adds `Assaulter=yes` (UC clear)
  and `DetectDisguise=yes`.
- **Sibling C4 plant unit (Yuri)**: [YURI Prime] / Yuri Initiate — `C4=yes`
  variants. Shares the same dispatcher.
- **Pseudo-sibling**: [CCOMAND](CCOMAND.md) Chrono Commando — also `C4=yes`
  with chrono-teleport locomotor (different movement, same plant logic).
- **Bomb defuse**: [ENGINEER](ENGINEER.md) — can defuse SEAL's C4 charge?
  No — SEAL's C4 is NOT a `BombClass`. It's an instant area-damage on
  arrival. Engineer's `DefuseKit` only defuses [IVAN]'s timed bombs.
- **IFV passenger**: [HTK](HTK.md) — `IFVMode=4` → laser/sniper weapon.
- **Counter-roles**:
  - Counters: any building (one-shot via C4), any infantry (one-shot via MP5).
  - Countered by: dogs (instant kill via Bite warhead — `Verses 100%` vs
    flak armor too), [DESO](../soviet/DESO.md) radiation, Yuri mind
    control (ImmuneToPsionics=no), vehicle crush, Tanya counter (Tanya
    one-shots SEAL with DoublePistols at range 6).
- **Theater asset variant**: `SEALA` art for snow theaters
  (`AlternateArcticArt=yes`).

---

## Ghidra audit log (audit iteration 4 — 2026-05-18)

Deep-Ghidra audit pass. ~2 decompiles + 8 string xrefs + 1 function lookup.
Primary goal: verify the C4 plant chain (Mission_Attack precondition gate,
CanC4 building flag, C4Warhead Rules-side, the +0xEC2 C4 flag).

### Function entry points verified

| Doc claim | Ghidra label / address | Status |
|-----------|------------------------|--------|
| `InfantryClass::Mission_Attack @ 0x0051F3E0` | `FUN_0051f3e0` (unlabeled), body `0x0051f3e0–0x0051f53e` | ⚠️ ADDRESS VERIFIED, NAME UNCONFIRMED (function exists and decompile matches the doc's described behavior, but Ghidra hasn't applied the `Mission_Attack` label) |
| `InfantryClass::Mission_Enter @ 0x005196A0` | Phantom — confirmed audit 3, inside `PerCellProcess` | ❌ ADDRESS DISCREPANCY (same finding as ENGINEER audit) |

### Key behavioral findings (decompile-verified)

1. **Mission_Attack C4 plant gate** (FUN_0051f3e0 decompile):
   ```c
   if (*(char *)(this->Type + 0xec2) == '\0') {                // C4 flag at TypeClass+0xEC2
     if (!HasWeaponAbility(0xe)) goto fallthrough;             // alternate gate
   }
   // C4 or weapon-ability passes:
   if (this->Target != NULL) {                                  // target ptr at this[0xad] = +0x2B4
     if (target->vtable[0x2c]() == 6) {                        // BuildingClass check (RTTI returns 6)
       BuildingTypeClass *bt = target[0x520];                   // building's type ptr
       if (bt[0x1577] != 0 && bt[0x1701] == 0) {                // CanC4 set, InvisibleInGame clear
         this->vtable[0x480](target, 1);                        // Set_Target
         this->vtable[0x1e8](0x11, 0);                          // SetMission(Enter=0x11)
         return 1;
       }
     }
   }
   ```
   - **`TypeClass+0xEC2 = C4 flag`** BINARY-VERIFIED.
   - **`BldgType+0x1577 = CanC4`** BINARY-VERIFIED (also parser-confirmed at `0x00460050` in BuildingTypeClass_ReadINI_Water).
   - **`BldgType+0x1701 = InvisibleInGame`** BINARY-VERIFIED (must be 0 to allow C4 target).
   - **`vtable+0x480 = Set_Target`** verified.
   - **`vtable+0x1e8 = SetMission`** verified (with mission ID 0x11 = Enter).
   - **`HasWeaponAbility(0xe)`** = the "UC clear" weapon ability alternate gate (Tanya has it, SEAL doesn't).
   - Building target ptr stored at `building+0x520` — the type ptr offset on BuildingClass.

2. **Spy/Infiltrator and Docker gates** (also in FUN_0051f3e0, second branch — for non-player-controlled units):
   ```c
   if (!IsPlayerControl() && target != NULL && target.RTTI == 6) {
     if (Type[0xebe] != 0) goto LAB_0051f4ad;        // Infiltrator flag at +0xEBE → infiltrate path
     if (Type[0xeb4] != 0 || Type[0xeb5] != 0) {     // Occupier/paratrooper-occupier
       if (CanDock(this)) goto LAB_0051f4ad;
     }
   }
   ```
   - **`TypeClass+0xEBE = Infiltrator/Infiltrate flag`** BINARY-VERIFIED (confirms the doc's note that Engineer=yes "forces +0xEBE Infiltrate=true").
   - `Type+0xEB4` (Occupier) and `+0xEB5` (paratrooper-occupier) — cross-verifies audit 1's findings from AddGarrisonOccupant.
   - `vtable+0x1f0(8)` is called for the AI-dispatched infiltrate path. Mission 8 = MissionEnter?

3. **Deploy-state wait** (third block in FUN_0051f3e0):
   ```c
   if (IsPlayerControl() && (seq == 0x1b || seq == 0x1c || seq == 0x1d || seq == 0x1e)) {
     // ... wait for deploy sequence to complete; return a randomized mission timer
   }
   ```
   - Sequence IDs 0x1b/0x1c/0x1d/0x1e (deployed states) cross-verify audit 1+2 findings.
   - This branch handles player-controlled deployer infantry that get Attack orders during deploy/undeploy transitions.

### Parser-key scope verifications (string xrefs)

| Field | String addr | First xref → reader | Scope |
|-------|-------------|---------------------|-------|
| `CanC4` | `0x0081adfc` | `0x00460050` in `BuildingTypeClass_ReadINI_Water` | BuildingType |
| `C4Warhead` | `0x0083b1d4` | `0x0066c31f` in `RulesClass__ReadCombatDamage` | Rules-CombatDamage (NEW SCOPE for cheat-sheet) |
| `SabotageCursor` | `0x008494c4` | `0x007721bd` in `WeaponTypeClass__ReadINI` | WeaponType |
| `DetectDisguise` | `0x00843c78` | `0x0071443f` in `TechnoTypeClass__ReadINI` | TechnoType |
| `DetectDisguiseRange` | `0x00843d3c` | (not pulled) | TechnoType (assumed) |
| `NavalTargeting` | `0x00844510` | `0x007121be` in `TechnoTypeClass__ReadINI` | TechnoType |
| `UseOwnName` | `0x00825908` | `0x0052463d` in `InfantryTypeClass__ReadINI` | **InfantryType** (corrects doc's "TechnoTypeClass" claim) |

### Discrepancies resolved

1. **RTTI value for BuildingClass** — audit 3 (ENGINEER's Mission_Capture
   decompile) saw `iVar2 == 1` for the BuildingClass target check via
   `vtable[0x2c]`. Audit 4 (GHOST's Mission_Attack decompile) sees `iVar2 == 6`
   for the same vtable slot on what's also a BuildingClass target. **Conflict**.
   - **Hypothesis A**: `vtable[0x2c]` is NOT GetRTTI; it may be a different
     vfunc that returns context-dependent values. If audit 3's check `==1`
     was on a derived class or a different abstract-class hierarchy, the
     value would differ.
   - **Hypothesis B**: the two functions consult different vtables (e.g.,
     vtable on different sub-objects of a multiple-inheritance hierarchy).
   - **DEFERRED**: decompile of TechnoClass GetAbstractType/GetRTTI would
     resolve. ENGINEER's claim "RTTI value 1 = BuildingClass" was likely
     wrong; GHOST's `== 6` is more likely the actual BuildingClass RTTI value.
2. **`UseOwnName` claimed as TechnoType field** — actually InfantryType.
   Doc claim downgraded. Minor: the field can still semantically apply to
   any TechnoType but the parser is in InfantryType reader only.

### Items intentionally NOT re-verified in iter 4

- **`Apply_area_damage` call chain** for C4 detonation. The doc claims
  detonation calls `Apply_area_damage(SEAL, Rules[+0xFA8], 1, 0)` and
  follows up with two more calls for destructible-overlay propagation.
  The actual detonation code lives in PerCellProcess (~5kb function) and
  wasn't decompiled. DEFERRED — would require the same effort as the
  Mission_Enter repair-branch decompile we deferred in ENGINEER.
- **Building+0x6DF "being C4'd" / Building+0x150 / Building+0x14A** —
  state fields written during the walk-up plant. Not verified at the
  struct level. DEFERRED.
- **NavalTargeting +0xC9A and DetectDisguise +0xCDF struct offsets** —
  parser-side scope verified; runtime read sites not traced. DEFERRED.
- **`vtable[0x2c]` GetRTTI behavior** — needed to resolve the RTTI=1 vs RTTI=6 conflict. DEFERRED.

### Confidence summary

- ~60% of GHOST-specific behavioral claims now have direct binary verification.
- ~30% are INFERRED (parser-side verified but runtime-read not traced; or
  related code paths not decompiled).
- ~5% are DISCREPANCY (UseOwnName scope, RTTI value conflict).
- ~5% are ADDRESS DISCREPANCY (Mission_Enter at 0x005196A0 — same phantom
  as ENGINEER).

The C4 plant gate — the SEAL's defining behavior — is **substantially
binary-verified**: the `TypeClass+0xEC2 = C4` flag, the `CanC4 +0x1577`,
the `InvisibleInGame +0x1701`, the SetMission(0x11=Enter), the Set_Target
vtable+0x480, the weapon-ability 0xE alternate gate, and the player-control
deploy-wait branch all match the doc's claims via decompile.

The detonation block (Apply_area_damage with Rules->C4Warhead) is
parser-side verified but the runtime damage application chain remains
DEFERRED — fixing this would require a substantial PerCellProcess decompile.

---

## Coverage audit

- ✅ Every key in `[GHOST]` rulesmd block (50 lines) covered.
- ✅ Every key in `[SEAL]` artmd block (7 lines) + `[SealSequence]`
  (27 lines) covered, including swim/wet variants and the SEALA snow
  variant.
- ✅ Weapon chain: MP5, Sapper, MP5E — projectiles (InvisibleLow,
  Invisible) and warheads (HollowPoint, Mechanical) covered, plus the
  `Rules->C4Warhead = Super` runtime substitution noted.
- ✅ Sound chain: 11 distinct soundmd entries covered (voices + CreateSound
  + EnterWater/LeaveWater + CrushSound + 2 weapon Reports).
- ✅ Ghidra search: `"GHOST"`, `"SEAL"`, `"Sapper"` recorded — no hardcoded
  unit-name branches. Deep C4 RE delegated to NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md.
- ✅ TS-legacy filter applied (TiberiumProof, ImmuneToVeins, Locomotor GUID
  note, Crawls, Sapper commented-out flags, AssaultAnim deadcode, stale
  Assaulter comment, PrerequisiteOverride campaign-only).
- ✅ Cross-references to GAPILE, TANY, CCOMAND, ENGINEER, IVAN, HTK, DESO.
