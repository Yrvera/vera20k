# Tanya (TANY)
Side: Allied | Category: Infantry | Image alias: `[TANY]` (no `Image=` redirect)

The premier Allied hero — one Tanya per house (`BuildLimit=1`). $1500 from
Battle Lab + Barracks. Dual `DoublePistols` (range 6, dmg 125 per shot at
ROF 5 — twice as fast as SEAL's MP5) plus `Sapper` C4 charges that one-shot
buildings via the same walk-up plant mechanic as Navy SEAL. Strength 200
(60% more HP than SEAL), Speed 6 (20% faster than SEAL), `Crushable=no`
(vehicles can't crush her), `ImmuneToPsionics=yes` (cannot be mind-controlled —
the **anti-Yuri counter** when paired with Allied teams), passive
`SelfHealing=yes` from rookie rank, and amphibious like SEAL.

Per INI: `DetectDisguise=` is commented out (defaults to no), and
`Assaulter=no` — Tanya cannot clear garrisoned buildings and cannot reveal
disguises despite popular belief; both INI keys are inactive in vanilla.

Authoritative deep RE: [NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md](../../NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md)
(covers both Tanya and SEAL since they share the C4 dispatch path).

---

## rulesmd.ini — `[TANY]` section

Verbatim from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:4068`:

```ini
[TANY]
UIName=Name:TANYA
Name=Tanya
Category=Soldier
Prerequisite=GAPILE,GATECH
Primary=DoublePistols
Secondary=Sapper
OpenTransportWeapon=0;defaults to -1 (decide normally)  What weapon should I use in a Battle Fortress
NavalTargeting=4
LeadershipRating=8
C4=yes
Assaulter=no ; I clear out UC buildings
CrushSound=InfantrySquish
Crushable=no
TiberiumProof=yes
Strength=200
Armor=flak
TechLevel=9
Pip=red
Sight=8
Speed=6
Owner=British,French,Germans,Americans,Alliance
AllowedToStartInMultiplayer=no
Cost=1500
Soylent=750
Points=50
IsSelectableCombatant=yes
VoiceSelect=TanyaPrimeSelect
VoiceMove=TanyaPrimeMove
VoiceAttack=TanyaPrimeAttackCommand
VoiceFeedback=TanyaPrimeFear
VoiceSpecialAttack=TanyaPrimeAttackCommand
DieSound=TanyaPrimeDie
CreateSound=TanyaPrimeCreated
EnterWaterSound=TanyaEntersWater
LeaveWaterSound=TanyaLeavesWater
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
SpeedType=Amphibious
MovementZone=AmphibiousDestroyer ; I am the only one with this zone, because it is now tied with being an infantry (part of seal stuck on tree bug)
ThreatPosed=25	; This value MUST be 0 for all building addons
SpecialThreatValue=1
ImmuneToVeins=yes
ImmuneToPsionics=yes
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,SCATTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Size=1
;DetectDisguise=yes
ElitePrimary=DoublePistolsE
EliteSecondary=Sapper
IFVMode=4
BuildLimit=1
SelfHealing=yes
UseOwnName=true
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:TANYA` | CSF-string key resolving to "Tanya". Overridden by `UseOwnName=true` → UI shows `Name=Tanya` |
| `Name=Tanya` | Displayed in UI per `UseOwnName=true` |
| `Category=Soldier` | Pip group + AI threat grouping |
| `Prerequisite=GAPILE,GATECH` | Allied Barracks + Allied Battle Lab. **No Radar requirement** (contrast SEAL's GAPILE,RADAR) |
| `Primary=DoublePistols` | Dual M1911-style pistols — range 6, dmg 125, ROF 5 (very fast), warhead HollowPoint2 |
| `Secondary=Sapper` | **Shared C4 weapon with SEAL** — dmg 2500, Mechanical warhead, but actual detonation uses `Rules->C4Warhead=Super` |
| `OpenTransportWeapon=0` | When passenger in Battle Fortress, fire Primary (DoublePistols) |
| `NavalTargeting=4` | Can engage naval at range 4 |
| `LeadershipRating=8` | High XP-gain rate; AI prioritizes |
| `C4=yes` | `InfantryTypeClass+0xEC2` — gates C4 plant path. Auto-forces Infiltrate (+0xEBE) |
| `Assaulter=no` | **Cannot clear garrisons.** Stale INI comment "I clear out UC buildings" is wrong — same copy-paste artifact as SEAL |
| `CrushSound=InfantrySquish` | Crush sample (sound block exists but Tanya is uncrushable so unreachable) |
| `Crushable=no` | **Vehicles cannot crush Tanya** — engine `CanCrushCheck` returns false. Distinguishes Tanya from SEAL (`Crushable=yes`) |
| `TiberiumProof=yes` | TS-legacy, unreachable in YR |
| `Strength=200` | HP — 60% more than SEAL (125), 1.6× a GI |
| `Armor=flak` | Body-armor type — same as SEAL |
| `TechLevel=9` | High tier |
| `Pip=red` | Hero pip color |
| `Sight=8` | Highest infantry sight |
| `Speed=6` | Walk speed — 20% faster than SEAL (5), 50% faster than GI (4) |
| `Owner=British,French,Germans,Americans,Alliance` | Allied only |
| `AllowedToStartInMultiplayer=no` | Excluded from starting unit pool |
| `Cost=1500` | Premium hero cost (50% more than SEAL) |
| `Soylent=750` | Grinder refund |
| `Points=50` | Kill score |
| `IsSelectableCombatant=yes` | In select-all-combat + AI combat groups |
| `VoiceSelect=TanyaPrimeSelect` | Selection voice bank (note: "Prime" suffix differentiates from RA1 Tanya voice banks the YR re-used campaign assets from) |
| `VoiceMove=TanyaPrimeMove` | Move acknowledgement |
| `VoiceAttack=TanyaPrimeAttackCommand` | Attack acknowledgement |
| `VoiceFeedback=TanyaPrimeFear` | Fear voice — **Tanya HAS fear voice** (SEAL's is empty/Fearless-equivalent) |
| `VoiceSpecialAttack=TanyaPrimeAttackCommand` | C4 plant voice — reuses attack bank |
| `DieSound=TanyaPrimeDie` | Death sample |
| `CreateSound=TanyaPrimeCreated` | Production-complete voice (Type=Global, Priority=CRITICAL — "Tanya reporting" announcement) |
| `EnterWaterSound=TanyaEntersWater` | Splash sample on water-entry |
| `LeaveWaterSound=TanyaLeavesWater` | Exit-water sample |
| `Locomotor={4A582744-...}` | WalkLocomotionClass GUID |
| `PhysicalSize=1` | Pathfinder size class |
| `SpeedType=Amphibious` | Can swim |
| `MovementZone=AmphibiousDestroyer` | Same unique zone as SEAL — both occupy it (designer note says "I am the only one with this zone" is per-unit text, not a single global) |
| `ThreatPosed=25` | Enemy AI prioritizes Tanya |
| `SpecialThreatValue=1` | Self threat-weight max |
| `ImmuneToVeins=yes` | TS-legacy, unreachable |
| `ImmuneToPsionics=yes` | **Cannot be mind-controlled** by Yuri/Yuri Prime/Psychic Tower. Critical anti-Yuri property |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,SCATTER` | 5 abilities — **no FASTER** (contrast SEAL who has it) |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | 4 abilities — **no FASTER** at elite either. Tanya doesn't speed up with rank |
| `Size=1` | Cargo slot cost |
| `;DetectDisguise=yes` | **Commented out** → defaults to `no`. Per the INI, Tanya **cannot detect disguises** despite popular gameplay belief. This may be a designer regression / bug; the INI as shipped does not enable disguise detection |
| `ElitePrimary=DoublePistolsE` | Elite primary — dmg 125, ROF 10, range 8. **ROF is slower** than rookie (10 vs 5) but range is longer (8 vs 6); Elite Tanya outranges but fires at half-rate per weapon — the `ROF` veterancy modifier (~25% faster reload) partially offsets but does not fully cover the gap |
| `EliteSecondary=Sapper` | **Same Sapper at Elite** — C4 doesn't promote |
| `IFVMode=4` | Same as SEAL — IFV Tanya/SEAL laser weapon |
| `BuildLimit=1` | **Hard limit: one Tanya per house.** Sidebar greys out the cameo once Tanya exists. If Tanya dies, can rebuild |
| `SelfHealing=yes` | **Passive HP regen from rookie rank.** Distinguishes from SEAL who only gets `SELF_HEAL` as an Elite ability. The `[General] SelfHealInfantry` rate applies |
| `UseOwnName=true` | UI shows `Name=Tanya` directly |

Implicit defaults:

- `Crawls=yes` (art section)
- `Bombable=` not set (default false; Ivan cursor doesn't auto-suggest Tanya as bomb target)
- `Trainable=yes` (default; gains XP)
- `Occupier=no` (cannot enter civilian buildings)
- `Deployer=no` (no deploy command)

---

## artmd.ini — `[TANY]` section

`c:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini:402`:

```ini
[TANY] ; Tanya
Sequence=TanyaSequence
Cameo=TANYICON
AltCameo=TANYUICO
Crawls=yes
Remapable=yes
FireUp=3
PrimaryFireFLH=100,0,100
```

| Key | Meaning |
|-----|---------|
| `Sequence=TanyaSequence` | Reference to the sequence block below |
| `Cameo=TANYICON` | Sidebar icon (rookie/veteran) |
| `AltCameo=TANYUICO` | Cameo at Elite |
| `Crawls=yes` | Sets `InfantryTypeClass+0xEBD` — prone-while-walking |
| `Remapable=yes` | House remap palette applied |
| `FireUp=3` | Bullet-spawn frame within firing sequence |
| `PrimaryFireFLH=100,0,100` | DoublePistols muzzle FLH: forward 100, side 0, height 100 |

No `SecondaryFireFLH=` (Sapper C4 is animation-driven, no muzzle flash).
No `AlternateArcticArt=` (Tanya doesn't have a snow variant unlike SEAL/SEALA).

### Referenced sequence — `[TanyaSequence]`

`artmd.ini:14042`:

```ini
[TanyaSequence]
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
Tread=410,6,6
Swim=506,6,6
WetAttack=554,6,6
WetIdle1=292,15,0,S
WetIdle2=307,15,0,E
WetDie1=322,20,0
WetDie2=342,20,0
Panic=8,6,6

Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
```

Same overall layout as SealSequence but with TANYA.shp's frame offsets:

- `Tread=410` vs SEAL `340` (different SHP layout)
- `Swim=506` vs SEAL `388`
- `WetAttack=554` vs SEAL `436`
- `WetIdle1=292`, `WetIdle2=307` vs SEAL `484`, `499`
- `WetDie1=322,20,0` `WetDie2=342,20,0` — note **20 frames** (SEAL had 15) for the wet death animations; Tanya has longer wet-death animation
- The trailing `Die3/4/5=0,1,1` after a blank line is unusual INI style but
  the parser tolerates it (each `key=` line is independent)

Same wet-vs-dry sequence swap mechanic as SEAL (see GHOST.md).

---

## Weapons

### Primary — `[DoublePistols]`

`rulesmd.ini:22995`:

```ini
[DoublePistols]
Damage=125
ROF=5
Range=6
Projectile=InvisibleLow
Speed=100
Warhead=HollowPoint2
Report=TanyaAttack
AssaultAnim=UCBLOOD;the anim to play when a UC building is cleared (assaulters need this on their primary weapon)
```

| Key | Meaning |
|-----|---------|
| `Damage=125` | One-shots all rookie infantry (125 vs 100% Verses) |
| `ROF=5` | **2× faster than SEAL's MP5** (5 vs 10 frames between shots). Tanya fires 2 shots in the time SEAL fires 1 |
| `Range=6` | Same as MP5 |
| `Projectile=InvisibleLow` | Walls/cliffs block, instant hit |
| `Speed=100` | Instant |
| `Warhead=HollowPoint2` | Anti-infantry only, slightly different from SEAL's `HollowPoint` |
| `Report=TanyaAttack` | Sound `itanatta/b` random, VShift +15, Volume 70 |
| `AssaultAnim=UCBLOOD` | Garrison-clear anim reference — but Tanya has `Assaulter=no` so unreachable. Stale designer artifact |

### Secondary — `[Sapper]` (shared with SEAL)

See [GHOST.md](GHOST.md#secondary--sapper-c4-demolition) for full breakdown.
Same dmg 2500, range 1.5, warhead `Mechanical`, `SabotageCursor=yes`,
`Report=SealPlaceBomb`. Detonation uses `Rules->C4Warhead=Super` independent
of the weapon's own warhead.

### Elite Primary — `[DoublePistolsE]`

`rulesmd.ini:25235`:

```ini
[DoublePistolsE]
Damage=125
ROF=10
Range=8
Projectile=InvisibleLow
Speed=100
Warhead=HollowPoint2
Report=TanyaAttack
AssaultAnim=UCBLOOD
```

| Key | Meaning |
|-----|---------|
| `Damage=125` | Same as rookie — no damage bump (relies on FIREPOWER ability modifier ~+25%) |
| `ROF=10` | **HALVED rate of fire** vs rookie (10 vs 5). At Elite, weapon-spec ROF is slower — the elite `ROF` ability (-25%) only partially offsets this. **Net effect**: Elite Tanya fires slower per single shot than rookie Tanya, but each shot reaches farther |
| `Range=8` | +2 cell range vs rookie |
| `Warhead=HollowPoint2` | Same |

This is the kind of detail that the parity bar covers — even if it "seems
wrong," it's what the INI ships, and the engine honors it exactly. If
in-game testing shows Elite Tanya feels faster overall, that's the FIREPOWER
+ ROF veterancy abilities compensating, not a different weapon profile.

### Elite Secondary — `[Sapper]`

Same as rookie. Tanya's C4 doesn't promote.

### Warhead — `[HollowPoint2]`

`rulesmd.ini:27061`:

```ini
[HollowPoint2]
Verses=100%,100%,100%,0%,0%,0%,1%,1%,1%,1%,100%
InfDeath=1
AnimList=PIFF
ProneDamage=100%
Bullets=yes
```

| Key | Meaning |
|-----|---------|
| `Verses=100%,100%,100%,0%,0%,0%,1%,1%,1%,1%,100%` | **Anti-infantry only**, slightly different from SEAL's `HollowPoint` (which has 200% vs none and 1% vs vehicles). Tanya: 100% vs infantry armors (none/flak/plate), **0% vs vehicles** (cannot damage tanks at all — even less than SEAL's 1%), 1% vs buildings, 100% vs special_2 |
| `InfDeath=1` | Standard bullet death |
| `AnimList=PIFF` | Small impact puff |
| `ProneDamage=100%` | Full damage vs prone (no reduction) |
| `Bullets=yes` | Bullet damage flag |

### Comparison to SEAL's `HollowPoint`

| Armor column | Tanya `HollowPoint2` | SEAL `HollowPoint` |
|--------------|---------------------|--------------------|
| none | 100% | **200%** |
| flak | 100% | 100% |
| plate | 100% | 100% |
| light/medium/heavy | 0% | 1% |
| wood/steel/concrete | 1% | 1% |
| special_1 | 1% | 1% |
| special_2 | 100% | 100% |

Net: SEAL has 2× overkill vs rookie GI (250 damage); Tanya hits the HP cap
exactly at 125. Both one-shot rookie GI. Tanya literally cannot scratch a
vehicle; SEAL grazes for 1.25 damage per shot.

### Projectile — `[InvisibleLow]`

See [E1.md](E1.md#projectile-blocks). Inviso, instant, walls/cliffs/elevation
block.

---

## Voices and sounds

| INI key on TANY | soundmd block | Resolved samples |
|-----------------|---------------|------------------|
| `VoiceSelect=TanyaPrimeSelect` | `[TanyaPrimeSelect]` line 5013 | `$itapsea` `$itapseb` `$itapsec` `$itapsed` `$itapsee` (random) |
| `VoiceMove=TanyaPrimeMove` | `[TanyaPrimeMove]` line 5018 | `$itapmoa` `$itapmob` `$itapmoc` `$itapmod` (random) |
| `VoiceAttack=TanyaPrimeAttackCommand` | `[TanyaPrimeAttackCommand]` line 5023 | `$itapata` `$itapatb` `$itapatc` `$itapatd` (random) |
| `VoiceFeedback=TanyaPrimeFear` | `[TanyaPrimeFear]` line 5039 | `$itapfea` `$itapfeb` `$itapfec` `$itapfed` (random, Volume 85) |
| `VoiceSpecialAttack=TanyaPrimeAttackCommand` | (same as VoiceAttack) | reuses attack bank for C4 plant |
| `DieSound=TanyaPrimeDie` | `[TanyaPrimeDie]` line 5044 | `$itapdia` `$itapdib` `$itapdic` `$itapdid` `$itapdie` (random) |
| `CreateSound=TanyaPrimeCreated` | `[TanyaPrimeCreated]` line 5033 | `$itapcra` `$itapcrb` `$itapcrc` `$itapcrd`, **Type=global**, **Priority=critical**, MinVolume=90 — "Tanya here" announcement |
| `EnterWaterSound=TanyaEntersWater` | `[TanyaEntersWater]` line 1144 | `gexpwasa` `gexpwasb` (shared with SEAL) |
| `LeaveWaterSound=TanyaLeavesWater` | `[TanyaLeavesWater]` line 1150 | `vnavupa` (shared with SEAL) |
| `CrushSound=InfantrySquish` | `[InfantrySquish]` line 1196 | `igensqua` (unreachable — Tanya is `Crushable=no`) |
| Weapon `DoublePistols` `Report=TanyaAttack` | `[TanyaAttack]` line 1136 | `itanatta` `itanattb` (random interrupt, FShift -5/+5, VShift 15, Volume 70) |
| Weapon `DoublePistolsE` `Report=TanyaAttack` | (same) | shared |
| Weapon `Sapper` `Report=SealPlaceBomb` | `[SealPlaceBomb]` line 3937 | `icraatta` (Crazy Ivan bomb-plant sample reused) |

Unused: `[TanyaPrimePsyResist]` at soundmd.ini:5028 with samples
`$itaprea $itapreb $itaprec $itapred $itapree` — designer left a "resisted
mind control" voice bank, but no INI key references it. The
`ImmuneToPsionics=yes` path on Tanya does not currently play any voice
when she resists psionic attempts. **Unreachable in vanilla.**

---

## Prerequisites, owners, tech

- `Prerequisite=GAPILE,GATECH` — Allied Barracks + Allied Battle Lab.
  Distinct from SEAL (GAPILE,RADAR).
- `Owner=British,French,Germans,Americans,Alliance` — Allied only.
- `TechLevel=9` — same as SEAL.
- `BuildLimit=1` — one Tanya at a time per house.
- `AllowedToStartInMultiplayer=no`.
- No `PrerequisiteOverride=`, no `ForbiddenHouses=`, no `RequiredHouses=`,
  no `AIBasePlanningSide=`.

---

## Veterancy and upgrades

- **Rookie**: DoublePistols + Sapper. `SelfHealing=yes` already active —
  passive HP regen from start.
- **Veteran** (`STRONGER,FIREPOWER,ROF,SIGHT,SCATTER`):
  - `STRONGER` = +50% HP (300 effective HP)
  - `FIREPOWER` = +25% damage
  - `ROF` = -25% reload time
  - `SIGHT` = +1 cell
  - `SCATTER` = improved damage-scatter dodge
  - **No FASTER** at veteran (distinct from SEAL)
- **Elite** (`SELF_HEAL,STRONGER,FIREPOWER,ROF`, cumulative):
  - `SELF_HEAL` = adds a second heal tick on top of `SelfHealing=yes`'s base
    rate. Some YR docs suggest these stack additively
  - Cumulative STRONGER/FIREPOWER/ROF
  - Primary swap: `DoublePistols` → `DoublePistolsE` (range 6→8, ROF 5→10)
  - Secondary: still `Sapper` (no swap)
  - Cameo swap: `TANYICON` → `TANYUICO`
- C4 damage doesn't scale with veterancy (uses fixed `Rules->C4Warhead`).
- `Crushable=no` independent of veterancy — always uncrushable.

---

## Hardcoded behavior in gamemd.exe (Ghidra-verified)

The C4 plant pipeline is identical to SEAL's. Full RE in
[NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md](../../NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md).
Confidence: HIGH for content/identity/binding (caller-traced).

### Shared C4 plant — `Mission_Attack` (vtable-dispatched) [BINARY-VERIFIED audit 7]

Same as SEAL: `InfantryClass::Mission_Attack` lives at `FUN_0051F3E0`
(unlabeled but identity-confirmed by GHOST audit 4 + audit 7) — it's a
**vtable-bound virtual** (vtable data xref at `0x007EB268`). The function
tests `Type+0xEC2 (C4)` or `HasWeaponAbility(0xE)` (the UC-clear alternate
gate), then:

- For a building target with RTTI=6, `BldgType+0x1577 (CanC4) != 0` and
  `BldgType+0x1701 (InvisibleInGame) == 0`: calls `vtable+0x480 = Set_Target`
  with the building, then `vtable+0x1E8 = SetMission(0x11 = Enter, 0)` and
  returns 1. [BINARY-VERIFIED audit 4 + 7]
- For non-player-controlled with Infiltrator/Occupier targeting: calls
  `vtable+0x1F0(8)` (different mission ID — likely Move or Capture set with
  arg=8). [BINARY-VERIFIED]
- For deployed-state infantry (sequence IDs 0x1B/0x1C/0x1D/0x1E): runs the
  `vtable+0x428` deployed-fire path. [BINARY-VERIFIED]
- Otherwise falls through to `FootClass__Mission_Attack` (normal attack).

**`InfantryClass::Mission_Enter @ 0x005196A0` is [INCORRECT / PHANTOM]** — no
function exists at that address. The address falls **inside the body of
`InfantryClass__PerCellProcess` @ 0x00519630** (~5.4KB function spanning
0x00519630–0x0051AA0A). The Mission Enter / C4 detonation logic lives inside
PerCellProcess, not at a standalone entry point. (Same pattern as ENGINEER
audit 3.) [PHANTOM CONFIRMED audit 7]

The Apply_area_damage detonation chain after walk-up arrival remains
DEFERRED (it's somewhere inside PerCellProcess but precise offset within
that 5.4KB blob has not been pinned in any audit yet).

### Crushable=no — vehicle crush rejection [BINARY-VERIFIED audit 7]

`TechnoClass::CanCrushCheck @ 0x005F6CD0` [BINARY-VERIFIED, entry exact;
audit 2 + 7]:

- **`ObjectTypeClass+0x22D = Crushable` (bool)** [BINARY-VERIFIED via
  `ObjectTypeClass__ReadINI` @ 0x005F92D0 reading `Crushable=` INI key with
  `CCINIClass__ReadBool(piVar9, s_Crushable_00832bd8, *(byte*)((int)param_1 + 0x22d))`]
- CanCrushCheck branch 2 reads this Crushable via `vtable+0x88` on the
  potential victim's type. When `Crushable=no`, the flag is 0, the
  check fails, and CanCrushCheck returns 0 = cannot crush. **The TANY
  doc's previous claim of "TechnoTypeClass+0x4xx" was vague — corrected
  to ObjectTypeClass+0x22D.**
- Branch 1 alternate path: if the CRUSHER's type has `+0xD29 OmniCrusher`
  set (and other gates pass), it can crush regardless of target's
  Crushable. **Audit 2's interpretation of +0xD29 as "Crushable-related
  flag on target" was [INCORRECT] — it is actually `OmniCrusher` on the
  crusher.** [CORRECTED audit 7, BINARY-VERIFIED via
  `TechnoTypeClass__ReadINI` at 0x00714CF0 reading `OmniCrusher=` into
  byte +0xD29]
- CanCrushCheck callers verified [audit 7]: DriveLocomotionClass /
  ShipLocomotionClass `Process_Drive_Track`, `UnitClass::Can_Enter_Cell`,
  `UnitClass::PerCellProcess`, `UnitClass::What_Action_OnObject`.

### ImmuneToPsionics=yes — mind-control rejection [BINARY-VERIFIED audit 7]

- **`TechnoTypeClass+0xD35 = ImmuneToPsionics` (byte)**. The TANY doc's
  previous claim of `InfantryTypeClass+0xCD7` was [INCORRECT — wrong class
  and wrong offset]; the flag is on **TechnoTypeClass** (so the inheritance
  reaches all unit types, not just infantry), and at byte **+0xD35**.
  [BINARY-VERIFIED via `TechnoTypeClass__ReadINI` @ 0x00714FA7 storing the
  ReadBool result into `*(byte*)((int)param_1 + 0xd35)`]
- Read by **`CaptureManagerClass::CanCapture @ 0x00471C90`** [BINARY-VERIFIED,
  audit 7 — `CanCapture` is invoked from `CaptureUnit @ 0x00471D40` early in
  the capture pipeline]:
  ```c
  in_EAX = vtable+0x84(param_2);            // GetTechnoType on capture target
  if (*(char *)(in_EAX + 0xd35) == '\0') {  // ImmuneToPsionics must be 0
      // ... then check IsMindControlled, IsInvulnerable (vtable+0x160),
      //     mission != 0x12/0x13, capture-slot capacity
      return 1;
  }
  ```
  If `+0xD35 != 0`, the function returns 0 → capture rejected → Yuri's
  mind-control fails on this target.
- **Anti-Yuri gameplay role**: Tanya is the single most reliable Allied
  unit vs Yuri mass-mind-control because the controller weapon literally
  cannot land.

### SelfHealing=yes — passive HP regen [BINARY-VERIFIED audit 7]

- **`TechnoTypeClass+0xD14 = SelfHealing` (byte)**. TANY doc's previous claim
  of "TechnoTypeClass+0xC92" was [INCORRECT]. Verified via
  `TechnoTypeClass__ReadINI` storing the ReadBool result into a byte at
  int-index 0x345 (= byte 0xD14). [BINARY-VERIFIED audit 7]
- The consumer (per-tick AI health-regen) was not re-decompiled in this
  audit; the field is read by the per-tick TechnoClass/InfantryClass AI
  path tied to `[General] SelfHealInfantry` interval. **Consumer
  decompile DEFERRED.**
- The Elite `SELF_HEAL` veterancy ability is a SEPARATE additive bit (an
  EliteAbilities flag in TechnoTypeClass+0x2AE array), not the same field
  as the `SelfHealing=` byte at +0xD14.

### BuildLimit=1 — sidebar enforcement [BINARY-VERIFIED audit 7]

- **`TechnoTypeClass+0x3B8 = BuildLimit` (int)**. TANY doc's previous
  claim of "TechnoTypeClass+0x6F8" was [INCORRECT]. Verified via
  `TechnoTypeClass__ReadINI` at int-index 0xEE (= byte 0x3B8). [BINARY-VERIFIED
  audit 7]
- Consumer (FactoryClass::CanBuild / sidebar refresh) DEFERRED — not
  re-decompiled this pass.

### DetectDisguise=no (commented out → default) [BINARY-VERIFIED audit 7]

- **`TechnoTypeClass+0xD31 = DetectDisguise` (byte)** — TechnoType-scope, not
  InfantryType. TANY doc's previous claim of "InfantryTypeClass+0xCDF" was
  [INCORRECT — wrong class and wrong offset]. [BINARY-VERIFIED audit 6 via
  TechnoTypeClass__ReadINI; re-confirmed audit 7]
- The commented `;DetectDisguise=yes` in TANY's INI means the parser does
  not read this key, so the byte stays at its default (false).
- Result: Tanya's cursor sees disguised Mirage Tanks and Spies as their
  disguise; same as SEAL.
- **This contradicts popular gameplay belief** but is what the shipping INI
  encodes. If Westwood intended Tanya to detect disguises, the INI was
  shipped with that line accidentally commented out and never restored.

### IFVMode=4

- Same as SEAL — IFV Weapon5 slot (Tanya/SEAL laser variant).

### UseOwnName=true

- Same flag as SEAL — UI shows `Name=Tanya` instead of CSF-resolving
  `UIName=Name:TANYA`.

### Ghidra string-search results

- `search_strings "TANY"` → INI parse targets and CSF keys only. No
  hardcoded section-name branch.
- `search_strings "Tanya"` → many hits, all in voice keys
  (TanyaPrimeSelect, TanyaPrimeMove, etc.) and water-sound keys
  (TanyaEntersWater) and CSF string constants. No hardcoded `if(name=="Tanya")`.
- Behavior is fully driven by the C4=yes + Crushable=no +
  ImmuneToPsionics=yes + SelfHealing=yes + BuildLimit=1 flag set.

### Auto-target priority

- LeadershipRating=8 + ThreatPosed=25 + SpecialThreatValue=1 — AI weighs
  Tanya at maximum on both sides (own AI retreats Tanya from danger;
  enemy AI prioritizes her).

---

## Ghidra audit log (audit iteration 7 — 2026-05-18)

Independent re-verification pass against gamemd.exe. ~15 decompiles spanning
the C4 plant chain, crush rejection, mind-control immunity, and the surrounding
INI parser store sites.

### Function entry points re-verified

| Doc claim | Verified at exact address |
|-----------|---------------------------|
| `InfantryClass::Mission_Attack @ 0x0051F3E0` | ✅ exact — function exists as `FUN_0051F3E0` (body 0x0051f3e0–0x0051f53e); identity confirmed via the C4 plant gate decompile + GHOST audit 4. Referenced via vtable data slot at `0x007EB268` (virtual dispatch). |
| `InfantryClass::Mission_Enter @ 0x005196A0` | ❌ **[PHANTOM]** — no function at this address. Falls inside `InfantryClass::PerCellProcess @ 0x00519630` (body extends to 0x0051AA0A). Mission Enter / C4 detonation logic is embedded inside PerCellProcess. (Same pattern as ENGINEER audit 3 + SPY audit 6.) |
| `TechnoClass::CanCrushCheck @ 0x005F6CD0` | ✅ exact (entry-point label correct; matches audit 2). |
| `CaptureManagerClass::CaptureUnit @ 0x00471D40` | ✅ exact (matches audit 1). Decompiled this pass — the immunity check lives in a sibling function `CanCapture @ 0x00471C90`, not in CaptureUnit itself. |
| `CaptureManagerClass::CanCapture @ 0x00471C90` | ✅ exact (NEW — discovered this audit as the actual ImmuneToPsionics consumer). |
| `ObjectTypeClass::ReadINI @ 0x005F92D0` | ✅ exact (NEW — Crushable/Bombable/Strength/Immune/Insignificant/IgnoresFirestorm/UseLineTrail/Voxel/AlternateArcticArt all stored here). |

### C4 plant dispatch — Mission_Attack body re-decompiled

`FUN_0051F3E0` is the InfantryClass virtual that handles attack-order
acquisition for C4 / Infiltrator / Occupier / Deployed-fire / normal-attack
branching:

```c
// Branch 1: C4 plant on a building
if (*(char *)(param_1[0x1b0] + 0xec2) == '\0') {  // Type+0xEC2 = C4 flag
    cVar1 = TechnoClass__HasWeaponAbility(0xe);   // alternate UC-clear gate
    if (cVar1 != '\0') goto LAB_0051f400;
} else {
    LAB_0051f400:
    if ((int *)param_1[0xad] != 0) {                  // target ptr
        iVar2 = (**(code **)(*(int *)param_1[0xad] + 0x2c))();  // GetRTTI
        if (iVar2 == 6) {                             // target is BuildingClass
            iVar2 = *(int *)(param_1[0xad] + 0x520);  // Bldg+0x520 = type ptr
            if ((*(char *)(iVar2 + 0x1577) != '\0') &&  // BldgType+0x1577 CanC4
                (*(char *)(iVar2 + 0x1701) == '\0')) {  // BldgType+0x1701 InvisibleInGame must be off
                (**(code **)(*param_1 + 0x480))(param_1[0xad], 1);  // Set_Target
                (**(code **)(*param_1 + 0x1e8))(0x11, 0);           // SetMission(Enter=0x11)
                return 1;
            }
        }
    }
}

// Branch 2: Non-player Infiltrator / Occupier targeting
if (!HouseClass__IsPlayerControl() && param_1[0xad] != 0) {
    if (vtable+0x2c() == 6) {                  // target is building
        iVar2 = param_1[0x1b0];                // type ptr
        if (*(char *)(iVar2 + 0xebe) != '\0') {  // Type+0xEBE Infiltrator
            vtable+0x480(target, 1);
            vtable+0x1f0(8);                   // SetMission with different arg
            return 1;
        }
        if (*(char *)(iVar2 + 0xeb4) != '\0' ||  // Type+0xEB4 Occupier
            *(char *)(iVar2 + 0xeb5) != '\0') {  // Type+0xEB5 paratrooper-occupier
            if (BuildingClass__CanDock(...)) ...;
        }
    }
}

// Branch 3: Deployed-state firing (sequence IDs 0x1B–0x1E)
if (IsPlayerControl && (param_1[0x1b1] == 0x1b || 0x1c || 0x1d || 0x1e)) {
    vtable+0x428();
    ...
    return timer;
}

// Fallback: normal attack
return FootClass__Mission_Attack();
```

All BINARY-VERIFIED. The TANY C4 plant uses the Branch 1 path with target
mission 0x11 = Enter.

### Struct offsets BINARY-VERIFIED (this audit)

**TechnoTypeClass (`int *param_1` in `TechnoTypeClass__ReadINI`):**

| Offset | Field | INI key | Notes |
|--------|-------|---------|-------|
| +0x29C | VeteranAbilities array start | `VeteranAbilities=` (list) | Tanya: STRONGER,FIREPOWER,ROF,SIGHT,SCATTER — note no FASTER |
| +0x2AE | EliteAbilities array start | `EliteAbilities=` (list) | Tanya: SELF_HEAL,STRONGER,FIREPOWER,ROF |
| +0x2C0 | SpecialThreatValue (double, 8 bytes) | `SpecialThreatValue=` | |
| +0x3B8 | BuildLimit (int) | `BuildLimit=` ✅ — was claimed "+0x6F8" in doc, **CORRECTED** |
| +0x5FC | LeadershipRating (int) | `LeadershipRating=` | |
| +0x600 | NavalTargeting (int) | `NavalTargeting=` | NEW |
| +0x670 | ThreatPosed (int) | `ThreatPosed=` | NEW |
| +0x688 | IFVMode (int) | `IFVMode=` | NEW |
| +0xC91 | ImmuneToVeins (byte) | `ImmuneToVeins=` | NEW |
| +0xD14 | SelfHealing (byte) | `SelfHealing=` ✅ — was claimed "+0xC92" in doc, **CORRECTED** |
| +0xD29 | OmniCrusher (byte) | `OmniCrusher=` | **CORRECTION to audit 2** — was misinterpreted as "Crushable-related" |
| +0xD35 | ImmuneToPsionics (byte) | `ImmuneToPsionics=` ✅ — was claimed "InfantryType+0xCD7" in doc, **CORRECTED**: wrong class AND wrong offset |
| +0xD50 | OpenTransportWeapon (int, -1 sentinel) | `OpenTransportWeapon=` | **CORRECTION to audit 1** — was vaguely called "pre-deploy weapon override"; actually OpenTransportWeapon |
| +0xDBC | IsSelectableCombatant (byte) | `IsSelectableCombatant=` | NEW |

**ObjectTypeClass (parent of TechnoType — fields inherited by every unit/structure type):**

| Offset | Field | INI key | Notes |
|--------|-------|---------|-------|
| +0xA0 | Strength (int) | `Strength=` | Re-confirms audit 5 (was display-name in audit 1, corrected audit 5) |
| +0x22D | Crushable (byte) | `Crushable=` ✅ | **The flag that Tanya's `Crushable=no` actually sets** — confirms TANY behavior |
| +0x22E | Bombable (byte) | `Bombable=` | NEW — gates Crazy Ivan bomb cursor |
| +0x231 | LegalTarget (byte) | `LegalTarget=` | NEW |
| +0x232 | Insignificant (byte) | `Insignificant=` | NEW |
| +0x233 | Immune (byte) | `Immune=` | NEW |
| +0x236 | Voxel (byte) | `Voxel=` | NEW |
| +0x237 | NewTheater (byte) | `NewTheater=` | NEW |
| +0x239 | IgnoresFirestorm (byte) | `IgnoresFirestorm=` | NEW |
| +0x23A | UseLineTrail (byte) | `UseLineTrail=` | NEW |

**InfantryTypeClass:**

| Offset | Field | INI key | Notes |
|--------|-------|---------|-------|
| +0xEBD | Crawls (byte) | `Crawls=` (art section) | BINARY-VERIFIED — final ReadBool in the InfantryTypeClass__ReadINI capability-flag chain |

### Parser-scope verifications (this audit, via INI key xrefs)

| INI key | Reader xref | Scope |
|---------|-------------|-------|
| `Crushable` | `ObjectTypeClass__ReadINI` @ 0x005F940A | **ObjectType** (inherited) ✅ |
| `OmniCrusher` | `TechnoTypeClass__ReadINI` @ 0x00714CF0 | **TechnoType** ✅ |
| `ImmuneToPsionics` | `TechnoTypeClass__ReadINI` @ 0x00714FA7 | **TechnoType** ✅ |
| `SelfHealing` | `TechnoTypeClass__ReadINI` @ 0x00714AD9 | **TechnoType** ✅ |
| `BuildLimit` | `TechnoTypeClass__ReadINI` @ 0x0071314A | **TechnoType** ✅ |
| `IFVMode` | `TechnoTypeClass__ReadINI` @ 0x00714794 | **TechnoType** ✅ |
| `NavalTargeting` | `TechnoTypeClass__ReadINI` @ 0x007121CB | **TechnoType** ✅ |
| `OpenTransportWeapon` | `TechnoTypeClass__ReadINI` @ 0x00714E68 | **TechnoType** ✅ |
| `ImmuneToVeins` | `TechnoTypeClass__ReadINI` @ 0x00714C36 | **TechnoType** ✅ |
| `IsSelectableCombatant` | `TechnoTypeClass__ReadINI` @ 0x00715761 | **TechnoType** ✅ |
| `LeadershipRating` | `TechnoTypeClass__ReadINI` @ 0x00714343 | **TechnoType** ✅ |
| `ThreatPosed` | `TechnoTypeClass__ReadINI` @ 0x007149DB | **TechnoType** ✅ |
| `UseOwnName` | `InfantryTypeClass__ReadINI` @ 0x0052463D | **InfantryType** ✅ |
| `Crawls` | `InfantryTypeClass__ReadINI` @ 0x005246AE | **InfantryType** ✅ |
| `Assaulter` | `InfantryTypeClass__ReadINI` @ 0x005244EF | **InfantryType** ✅ |
| `TiberiumProof` | `InfantryTypeClass__ReadINI` @ 0x0052458B | **InfantryType** ✅ |
| `DeployedCrushable` | `InfantryTypeClass__ReadINI` @ 0x00524627 | **InfantryType** ✅ |

### CaptureUnit / CanCapture call chain — BINARY-VERIFIED

`CaptureManagerClass::CaptureUnit @ 0x00471D40` starts by calling
`CaptureManagerClass::CanCapture @ 0x00471C90` on the target; if CanCapture
returns 0, CaptureUnit immediately returns 0 (mind-control fails). CanCapture
is the actual `+0xD35 ImmuneToPsionics` consumer.

CanCapture also checks:
- Target's owner != attacker's owner (via vtable+0x3c = GetOwnerHouse)
- ImmuneToPsionics byte at TechnoType+0xD35
- Target's `+0xB9` slot (mind-control marker)
- `TechnoClass::IsMindControlled()`
- Target's `+0xB3` (existing capture link)
- `vtable+0x160 = IsInvulnerable()` returns 0
- Capture-slot capacity (`+0x34 < +0x3c` or override flags)
- Target mission != 0x12 / 0x13

Then CaptureUnit proceeds to `vtable+0x3D4 = ChangeOwner` and inserts the
target into the capture manager's link array.

### Items NOT re-verified this pass (DEFERRED)

- **C4 detonation chain inside PerCellProcess** — the actual
  `Apply_area_damage(this, Rules->C4Warhead, 1, 0)` call site is somewhere
  inside the 5.4KB `InfantryClass::PerCellProcess` body. Walking that whole
  function to find it is out of scope for this iteration. DEFERRED.
- **vtable+0x1F0 mission ID 8 semantics** — Mission_Attack's non-player
  Infiltrator branch calls a different SetMission virtual (slot +0x1F0)
  with arg 8. Whether this is the same mission as 0x11 (Enter) or a
  different one (Capture? Move?) was not separately traced. DEFERRED.
- **SelfHealing consumer (per-tick AI regen path)** — the field is read but
  the consumer (probably `FootClass::AI` or `InfantryClass::AI` tied to
  `[General] SelfHealInfantry`) was not decompiled this pass. DEFERRED.
- **BuildLimit consumer** — `FactoryClass::CanBuild` / sidebar refresh code
  not decompiled. DEFERRED.
- **VeteranAbilities / EliteAbilities array layout** — start offsets pinned
  but per-ability bit/byte layout (STRONGER vs FIREPOWER vs ROF etc.) not
  separately decompiled. DEFERRED.
- **Sapper detonation Rules->C4Warhead substitution** — claim verified via
  cross-reference to the SEAL/GHOST audit 4 finding; not re-verified
  end-to-end in this pass.

### Confidence summary

**HIGH** for: Mission_Attack body branches and offsets, all corrected
struct-offset claims (BuildLimit, SelfHealing, ImmuneToPsionics,
DetectDisguise, Crushable, OmniCrusher, OpenTransportWeapon), ObjectType /
TechnoType / InfantryType parser-scope assignments, CanCapture ImmuneToPsionics
consumer chain.

**MEDIUM** for the audit 2 reinterpretation of +0xD29 — confirmed
BINARY-VERIFIED as `OmniCrusher` via ReadINI store, which means audit 2's
"Crushable-related flag on target" label was wrong (it's a crusher-side
capability override, not a target-side flag). Cumulative cheat-sheet
updated.

**LOW / unverified** for the C4 detonation chain through PerCellProcess and
the per-tick SelfHealing regen path — both DEFERRED.

---

## TS-legacy filter

- `TiberiumProof=yes` — TS terrain immunity, unreachable in YR.
- `ImmuneToVeins=yes` — TS terrain, unreachable.
- `MovementZone=AmphibiousDestroyer` — TS-era bug workaround note in the
  INI comment ("part of seal stuck on tree bug"). Live in YR.
- `Locomotor={4A582744-...}` — TS GUID, alive.
- `Crawls=yes` (art) — TS-era prone-while-walking, alive.
- `AssaultAnim=UCBLOOD` on DoublePistols/DoublePistolsE — designer artifact;
  Tanya has `Assaulter=no` so unreachable.
- INI comment `; I clear out UC buildings` on `Assaulter=no` — **stale**;
  same copy-paste artifact as SEAL.
- `;DetectDisguise=yes` (commented out) — designer intent left unfinished;
  effective value is the default (no).
- `[TanyaPrimePsyResist]` voice block in soundmd — unreachable; no INI key
  references it.
- `CrushSound=InfantrySquish` despite `Crushable=no` — defensive value;
  the crush sound never plays because the crush itself is rejected.
- Trailing `Die3=0,1,1` / `Die4=0,1,1` / `Die5=0,1,1` in TanyaSequence after
  a blank line — non-canonical INI style but parser-tolerated.

---

## Cross-references

- **Builder**: [GAPILE](../structures/GAPILE.md) Allied Barracks +
  [GATECH](../structures/GATECH.md) Allied Battle Lab.
- **Sibling C4 plant unit**: [GHOST](GHOST.md) Navy SEAL — shares
  `Mission_Attack→Mission_Enter` dispatch, Sapper weapon, amphibious
  movement. Differences: SEAL is `Crushable=yes`, `ImmuneToPsionics=no`,
  `SelfHealing` only at Elite, no BuildLimit.
- **Sibling hero (Yuri)**: [YURIPR](../yuri/YURIPR.md) Yuri Prime — same
  hero-class tier with hardcoded special ability.
- **Sibling hero (Soviet)**: [BORIS](../soviet/BORIS.md) — Soviet hero
  with AKM rifle + airstrike call-down.
- **Sibling C4 plant unit (Allied chrono)**: [CCOMAND](CCOMAND.md) Chrono
  Commando — `C4=yes` with chrono-shift locomotor.
- **IFV passenger**: [HTK](HTK.md) — `IFVMode=4` → laser weapon.
- **Counter-roles**:
  - Counters: any building (one-shot via C4), any infantry (one-shot via
    DoublePistols), Yuri mind-controllers (immune).
  - Countered by: dogs (one-shot melee), [DESO](../soviet/DESO.md)
    radiation (eventually wins despite SelfHealing), [TANY] mirror
    (Tanya v Tanya is roll-of-the-dice).
- **No theater variant** (no TANYA arctic SHP — unlike SEALA).

---

## Coverage audit

- ✅ Every key in `[TANY]` rulesmd block (53 lines including commented
  `;DetectDisguise=yes`) covered above.
- ✅ Every key in `[TANY]` artmd block (8 lines) covered, plus
  `[TanyaSequence]` (25 lines).
- ✅ Weapon chain: DoublePistols, Sapper, DoublePistolsE — all three
  covered with projectile (InvisibleLow) and warhead (HollowPoint2,
  Mechanical via SEAL cross-ref). Detonation `Rules->C4Warhead=Super`
  substitution noted.
- ✅ Sound chain: 13 distinct soundmd entries covered + unused
  `[TanyaPrimePsyResist]` flagged.
- ✅ Ghidra search: `"TANY"`, `"Tanya"` recorded — no hardcoded
  section-name branches; behavior is flag-driven. Deep C4 RE shared with
  SEAL in NAVY_SEAL_TANYA_C4_GHIDRA_REPORT.md.
- ✅ TS-legacy filter applied (TiberiumProof, ImmuneToVeins, Locomotor GUID,
  Crawls, AmphibiousDestroyer zone bug-workaround note, stale Assaulter
  comment, commented-out DetectDisguise, unreached PsyResist voice, AssaultAnim
  deadcode, CrushSound vs Crushable=no irrelevance, Die3-5 trailing INI style).
- ✅ Cross-references to GAPILE, GATECH, GHOST, YURIPR, BORIS, CCOMAND, HTK, DESO.
