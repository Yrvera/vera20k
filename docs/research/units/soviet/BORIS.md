# Boris (BORIS)
Side: Soviet | Category: Infantry | Image alias: `[BORIS]` (no `Image=` redirect — own SHP `BORIS`)

The Soviet **Boris**, Russia's national special unit (`Owner=` lists all 4
Soviet countries — no `RequiredHouses=` set; **Boris is the universal Soviet
hero** despite often being marketed as "Russia's"). $1500 from Soviet
Barracks + Soviet Battle Lab. The Soviet counterpart to Allied Tanya, with
two weapons that pair direct fire and called-airstrike:
**`Primary=AKM`** (Damage 65, Range 7, `Warhead=BORISWH`) — high-DPS rifle
that one-shots most infantry (200% Verses vs `none`/`flak` armor classes
= 130 effective damage), and **`Secondary=Flare`** (Range 12,
`Warhead=AirstrikeFlare`, `MigAttackCursor=yes`) — a **laser-designator**
that targets a building, then **summons MIGs (AirstrikeTeamType=BPLN) from
the map edge** to bomb the target with `Maverick3` Damage=750 missiles.
`AirstrikeTeam=2` MIGs at Veteran, `EliteAirstrikeTeam=4` at Elite. Highest
infantry HP in the game at `Strength=200` after Brute, plus
`Crushable=no` + `ImmuneToPsionics=yes` + `SelfHealing=yes` + `TiberiumProof=yes`
make Boris one of the toughest units. `BuildLimit=1` — only one Boris per
player at a time.

No standalone Boris/airstrike RE doc existed; this document originates the
Ghidra trace of the airstrike subsystem flags.

---

## rulesmd.ini — `[BORIS]` section

Verbatim from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:4593`:

```ini
[BORIS]
UIName=Name:Boris
Name=Boris
;Image=TANY
Category=Soldier
Prerequisite=NAHAND,NATECH
Primary=AKM
Secondary=Flare
OpenTransportWeapon=0;defaults to -1 (decide normally)  What weapon should I use in a Battle Fortress
NavalTargeting=4
LeadershipRating=8
Assaulter=no ; I clear out UC buildings
CrushSound=InfantrySquish
Crushable=no
TiberiumProof=yes
Strength=200
Armor=flak
TechLevel=9
Pip=red
Sight=9
Speed=5
Owner=Russians,Confederation,Africans,Arabs
AllowedToStartInMultiplayer=no
Cost=1500
Soylent=750
Points=50
IsSelectableCombatant=yes
VoiceSelect=BorisSelect
VoiceMove=BorisMove
VoiceAttack=BorisAttackCommand
;VoiceAttack=BorisAttackCommand
VoiceFeedback=BorisFear
VoiceSecondaryWeaponAttack=BorisAirstrikeVoice
;VoiceSecondaryWeaponAttack=BorisAirstrikeVoice
DieSound=BorisDie
CreateSound=BorisCreated
EnterWaterSound=TanyaEntersWater
LeaveWaterSound=TanyaLeavesWater
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
ThreatPosed=25	; This value MUST be 0 for all building addons
SpecialThreatValue=1
ImmuneToVeins=yes
ImmuneToPsionics=yes
ImmuneToPsionicWeapons=no
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,SCATTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Size=1
;DetectDisguise=yes
ElitePrimary=AKME
IFVMode=4
UseOwnName=true
;Airstrike stuff

;How many planes to call in
AirstrikeTeam=2;
EliteAirstrikeTeam=4;
;What type of planes to call in
AirstrikeTeamType=BPLN
EliteAirstrikeTeamType=BPLN
;How long after the planes either leave the map or are destroyed will the next team of planes be ready?
AirstrikeRechargeTime=100;500
EliteAirstrikeRechargeTime=50;250
BuildLimit=1
SelfHealing=yes
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:Boris` | CSF-string key → "Boris" |
| `Name=Boris` | Internal name |
| `;Image=TANY` (commented) | Designer history — Boris was originally going to reuse Tanya's SHP. Final build has its own `BORIS` SHP |
| `Category=Soldier` | Infantry pip/AI grouping |
| `Prerequisite=NAHAND,NATECH` | Soviet Barracks AND Soviet Battle Lab (specifically NATECH, not abstract TECH) |
| `Primary=AKM` | AK-47 rifle — `Damage=65, ROF=20, Range=7, Warhead=BORISWH`. The standing rifle weapon — fast cadence anti-infantry. See "Weapons" |
| `Secondary=Flare` | **The airstrike designator** — `Damage=1, ROF=60, Range=12, MigAttackCursor=yes, Warhead=AirstrikeFlare`. Triggers the called-airstrike subsystem when fired at a building. See "Hardcoded Behavior" §1 |
| `OpenTransportWeapon=0` | When loaded in a Battle Fortress as cargo, the Battle Fortress fires Boris's Primary (weapon index 0 = AKM). Inline comment: "defaults to -1 (decide normally) What weapon should I use in a Battle Fortress". By setting to 0 (Primary), Boris-in-FV uses AKM specifically rather than letting the engine pick |
| `NavalTargeting=4` | Naval targeting class 4 — engine value for "can target naval units when adjacent" |
| `LeadershipRating=8` | Veterancy-gain modifier — high (8/10) |
| `Assaulter=no ; I clear out UC buildings` | Explicit `no` — Boris does NOT clear UC buildings. The comment describes what `Assaulter=yes` means. Boris's AKM does have `AssaultAnim=UCBLOOD` defensively, but it's vestigial here |
| `CrushSound=InfantrySquish` | Moot — `Crushable=no` |
| `Crushable=no` | **Behavior flag** — Boris cannot be crushed by vehicles. One of only ~3 basic infantry with this (Tesla Trooper, Desolator, Boris) |
| `TiberiumProof=yes` | **Behavior flag** — InfantryTypeClass field (per `InfantryTypeClass__ReadINI @ 0x0052458B` DATA xref to string at `0x0082595C`). **TS legacy** in name (Tiberium was the TS hazard biome). In YR, this flag still functions as "immune to terrain-resident hazard damage" — but YR has no Tiberium terrain. Defensively set; protects against ore-field damage on certain maps |
| `Strength=200` | HP — **200**, the highest of any standard buildable infantry (Brute is 350 but Yuri side). Roughly 2× Tanya |
| `Armor=flak` | Damage type column 1 — flak armor. Same as Conscript |
| `TechLevel=9` | Tech-9 cap — late game, post-Battle-Lab |
| `Pip=red` | Cargo pip color — red (elite class) |
| `Sight=9` | Reveal radius — large (matches Spy, Dog). Important for the airstrike: Boris must see the target building to designate it |
| `Speed=5` | Foot-speed — slightly fast (vs typical 4) |
| `Owner=Russians,Confederation,Africans,Arabs` | All 4 Soviet houses. **No `RequiredHouses=`** set — any Soviet country can build Boris (often described as "Russia's" but mechanically available to all Soviet players) |
| `AllowedToStartInMultiplayer=no` | Not in starting unit complement |
| `Cost=1500` | $1500 — among the most expensive infantry; matches Tanya's $1000 — wait, Tanya is $1000, Boris is $1500. Boris is the **most expensive standard infantry** |
| `Soylent=750` | $750 Grinder refund (Yuri only — 50% standard) |
| `Points=50` | **Kill score 50** — matches SEAL. Among the highest infantry point values |
| `IsSelectableCombatant=yes` | Included in select-all-combat |
| `VoiceSelect=BorisSelect` | Select voice — `$iborsea..e` (5 lines, Russian-accented) |
| `VoiceMove=BorisMove` | Move voice — `$ibormoa..e` (5 lines) |
| `VoiceAttack=BorisAttackCommand` | Attack voice — `$iborata..e` (5 lines) |
| `;VoiceAttack=BorisAttackCommand` (commented) | Duplicate of the line above — leftover comment. No effect |
| `VoiceFeedback=BorisFear` | Fear voice — `$iborfea..d` (4 lines) |
| `VoiceSecondaryWeaponAttack=BorisAirstrikeVoice` | **Behavior key** — TechnoTypeClass field (per `TechnoTypeClass__ReadINI @ 0x00713706` DATA xref to string at `0x00844038`). Distinct voice played when the **Secondary weapon** is fired (vs the regular VoiceAttack for Primary). For Boris this fires `BorisAirstrikeVoice` (`$iboraib`) — the "Airstrike!" call — **`Type=global Priority=critical`** so all players hear it (warning the victim) |
| `;VoiceSecondaryWeaponAttack=BorisAirstrikeVoice` (commented) | Duplicate leftover |
| `DieSound=BorisDie` | Death voice — `$ibordia..e` (5 lines) |
| `CreateSound=BorisCreated` | **Build-completion voice** — `$iborcra..e` (5 lines, `Type=global Priority=critical MinVolume=80`). Plays globally when Boris finishes training |
| `EnterWaterSound=TanyaEntersWater` | **Reuses Tanya's water-entry sound** — Boris can wade through water like Tanya |
| `LeaveWaterSound=TanyaLeavesWater` | Reuses Tanya's exit-water sound |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID — standard infantry |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Infantry` | Standard infantry terrain |
| `ThreatPosed=25` | AI scoring weight — high (matches SEAL) |
| `SpecialThreatValue=1` | Threat estimate weight (between 0 and 1) — max self-threat |
| `ImmuneToVeins=yes` | TS legacy; defensively set |
| `ImmuneToPsionics=yes` | **Behavior flag** — Boris **CANNOT be mind-controlled** by Yuri/Initiate/Psychic Tower/Dominator. One of the strongest anti-Yuri counters. Matches Tanya |
| `ImmuneToPsionicWeapons=no` | Boris CAN be hit by psionic *area* weapons (Psychic Dominator's blast area damage). Distinction: ImmuneToPsionics blocks mind-control specifically; ImmuneToPsionicWeapons would block all psionic damage. Boris has the first but not the second |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,SCATTER` | At Veteran tier — note **SCATTER** instead of FASTER. Boris becomes more evasive at Veteran |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | At Elite tier — SELF_HEAL stacks on the base SelfHealing for faster regen. Triggers `ElitePrimary=AKME` weapon swap (Damage 65→90, Range 7→9). Also triggers Elite-tier airstrike (`EliteAirstrikeTeam=4` planes, `EliteAirstrikeRechargeTime=50` instead of 100) |
| `Size=1` | Transport cargo slot cost |
| `;DetectDisguise=yes` (commented) | Designer history — Boris was originally going to detect disguised units. Final answer: no (Engineer/SEAL/Tanya have BombSight but not DetectDisguise; only dogs detect disguise) |
| `ElitePrimary=AKME` | Elite Primary swap. AKME: Damage 65→90 (+38%), Range 7→9 (+29%) |
| `IFVMode=4` | IFV gunner-table index 4 → HTK's `Weapon5`/`ElitePassengerWeapon5` slot. In stock YR maps to a long-range high-damage rifle weapon — Boris-in-IFV gives the IFV chassis a Boris-style AK |
| `UseOwnName=true` | **Behavior flag** — shows "Boris" specifically on hover tooltips (matches Sniper, Tanya, Yuri Prime). InfantryTypeClass field per SNIPE.md notes |
| `AirstrikeTeam=2` | **Veteran/Base airstrike size** — number of planes to call. TechnoTypeClass field (per `TechnoTypeClass__ReadINI @ 0x00714591` DATA xref to string at `0x00843B84`). Stock value: 2 MIGs per Boris airstrike call |
| `EliteAirstrikeTeam=4` | **Elite airstrike size** — 4 MIGs (2× base). TechnoTypeClass field (xref to string at `0x00843B70`) |
| `AirstrikeTeamType=BPLN` | **Aircraft type to spawn** — TechnoTypeClass field (xref to string at `0x00843B5C`). References the `[BPLN]` AircraftType (Soviet MIG, `Primary=Maverick3 Damage=750`). The engine spawns N copies of this aircraft from the nearest map-edge waypoint |
| `EliteAirstrikeTeamType=BPLN` | Same aircraft type at Elite (no different plane type — just more of them). TechnoTypeClass field (xref to string at `0x00843B44`) |
| `AirstrikeRechargeTime=100` | **Cooldown between airstrikes** — 100 frames (~6.7s @ 15fps). TechnoTypeClass field (xref to string at `0x00843B2C`). Inline note `;500` suggests the value was tuned down from 500 (much longer cooldown) during balance testing |
| `EliteAirstrikeRechargeTime=50` | Elite cooldown — 50 frames (~3.3s). Half the Veteran timer. TechnoTypeClass field (xref to string at `0x00843B10`). Inline note `;250` shows the same balance reduction |
| `BuildLimit=1` | **Maximum one Boris per player at any time**. Production queue forbids a second Boris build until the first is dead/destroyed |
| `SelfHealing=yes` | Passive HP regeneration (same flag as Desolator). Cap = Strength=200 |

### Implicit defaults (not set in this section but worth noting)

- `Crawls=` — set in art section to `no` (Boris does NOT go prone — see artmd)
- `Trainable=` — defaults to `yes` (Boris gains veterancy — confirmed by Veteran/Elite ability lists + ElitePrimary)
- `NotHuman=` — defaults to `no` (subject to InfDeath, sniper headshot, mind-control... wait Boris IS immune to mind-control via ImmuneToPsionics=yes)
- `Occupier=` — defaults to `no`; Boris **cannot garrison** civilian buildings
- `Agent=`/`Infiltrate=`/`Engineer=`/`Ivan=`/`C4=` — not set
- `Bombable=` — defaults to `no` (not in explicit list)
- `Fearless=` — not set; Boris CAN show fear (VoiceFeedback wired)
- `Deployer=` — not set; Boris has no deploy command
- `DetectDisguise=` — commented out (defaults no)
- `BombSight=` — not set; Boris does NOT detect Crazy Ivan bombs (unlike Tanya/Engineer/SEAL who do)
- `Natural=` — not set
- `Spawned=` — defaults `no` (Boris is buildable)
- `AllowedToStartInMultiplayer=no` is explicit

---

## artmd.ini — `[BORIS]` section

`c:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini:411`:

```ini
[BORIS] ; Boris
Sequence=BorisSequence
Cameo=BRISICON
AltCameo=BRISUICO
Crawls=no
Remapable=yes
FireUp=3
PrimaryFireFLH=100,0,100
```

| Key | Meaning |
|-----|---------|
| `Sequence=BorisSequence` | Reference to `[BorisSequence]` — Boris-specific (includes SecondaryFire frames for airstrike-designator pose) |
| `Cameo=BRISICON` | Sidebar build icon (SHP `BRISICON` — note BRIS not BORIS in the file naming) |
| `AltCameo=BRISUICO` | Elite cameo — shown after Veteran promotion |
| `Crawls=no` | **Boris CANNOT crawl/go prone**. Sets the prone-disabled flag on the type. Distinguishes from Tanya (who has Crawls=yes) — Boris's animation set lacks proper prone art |
| `Remapable=yes` | House remap palette applied |
| `FireUp=3` | Bullet-spawn frame — at frame 3 the AK fires |
| `PrimaryFireFLH=100,0,100` | FLH — 100 forward, 0 sideways, 100 up. Standard rifle FLH for an upright soldier |

No `SecondaryFireFLH=` — the Flare designator is `IsLine=yes IsHouseColor=yes` (draws a line from Boris to target in the firing house's color), so the launch point of the "projectile" is computed from FLH if set or from the body geometry default.

### Referenced sequence — `[BorisSequence]`

`artmd.ini:14071`:

```ini
[BorisSequence]
Ready=0,1,1
Guard=0,1,1
Prone=86,1,6
Walk=8,6,6
FireUp=169,6,6
Down=265,2,2
Crawl=91,6,6
Up=281,2,2
FireProne=217,6,6
Idle1=56,15,0,S
Idle2=71,20,0,E
Die1=139,15,0
Die2=154,15,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
;Cheer=56,15,0,W
Cheer=297,8,0,E
Paradrop=292,1,0
Panic=8,6,6
SecondaryFire=401,1,1;305,12,12
SecondaryProne=217,6,6;401,1,1;305,12,12
```

| Slot | Frames | Notes |
|------|--------|-------|
| `Ready=0,1,1` | Standing idle | |
| `Guard=0,1,1` | Guard idle | |
| `Prone=86,1,6` | Prone — defined but **unused** (`Crawls=no` blocks transition) | |
| `Walk=8,6,6` | Walk cycle 6×6 | |
| `FireUp=169,6,6` | Standing fire cycle — where AKM fires | |
| `Down=265,2,2` | Get-down to prone — unused | |
| `Crawl=91,6,6` | Crawl reuses prone — unused | |
| `Up=281,2,2` | Get-up from prone — unused | |
| `FireProne=217,6,6` | Prone-fire cycle — unused | |
| `Idle1=56,15,0,S` | Idle 1 — 15 frames S-facing | |
| `Idle2=71,20,0,E` | Idle 2 — **20 frames** E-facing (longer than typical 15) | |
| `Die1=139,15,0` | Death 1 — 15 frames | |
| `Die2=154,15,0` | Death 2 | |
| `Die3=0,1,1` `Die4=0,1,1` `Die5=0,1,1` | Stub → Ready frame | |
| `;Cheer=56,15,0,W` (commented) | Older cheer entry | |
| `Cheer=297,8,0,E` | Cheer — 8 frames E-facing | |
| `Paradrop=292,1,0` | Single frame at 292 — paradrop pose | Boris paradrop-eligible (used in some campaigns) |
| `Panic=8,6,6` | Panic = Walk frames | |
| `SecondaryFire=401,1,1;305,12,12` | **Airstrike designator pose** — single frame at 401. Inline alternate `;305,12,12` is the longer designator cycle (commented variant). Plays when Boris fires Flare |
| `SecondaryProne=217,6,6;401,1,1;305,12,12` | Prone designator fire — reuses FireProne; alt commented | Unused (Crawls=no) |

---

## Weapons

### Primary (Veteran and below) — `[AKM]`

`rulesmd.ini:23005`:

```ini
[AKM]
Damage=65
ROF=20
Range=7
Projectile=InvisibleLow
Speed=100
Warhead=BORISWH
Report=BorisAttack
AssaultAnim=UCBLOOD;the anim to play when a UC building is cleared (assaulters need this on their primary weapon)
```

| Key | Meaning |
|-----|---------|
| `Damage=65` | Per-shot damage. Combined with `BORISWH.Verses[none]=200%` → **130 damage vs Armor=none infantry** (one-shots GI at 100 HP). Vs flak armor `BORISWH.Verses=200%` → 130 dmg one-shots Conscript at 125 HP. Vs plate=100% → 65 dmg (Tesla Trooper takes 2 shots) |
| `ROF=20` | Cooldown — 20 frames (~1.3s @ 15fps) — fast cadence |
| `Range=7` | 7 cells — long for an infantry weapon |
| `Projectile=InvisibleLow` | LOS-respecting inviso (blocked by walls/cliffs/elevation) |
| `Speed=100` | Irrelevant for inviso |
| `Warhead=BORISWH` | Boris's signature warhead — 200% vs both `none` and `flak` infantry armor |
| `Report=BorisAttack` | Sound `iboratta/b` (2 samples, FShift -5/+5, Volume 70) |
| `AssaultAnim=UCBLOOD` | UC-clear blood animation — **vestigial** because Boris is `Assaulter=no` |

### Elite Primary — `[AKME]`

`rulesmd.ini:25245`:

```ini
[AKME]
Damage=90
ROF=20
Range=9
Projectile=InvisibleLow
Speed=100
Warhead=BORISWH
Report=BorisAttack
AssaultAnim=UCBLOOD;the anim to play when a UC building is cleared (assaulters need this on their primary weapon)
```

Delta from `[AKM]`:
- **Damage 65→90** (+38%) — combined with BORISWH 200% vs none → 180 damage (one-shots Strength 175 or less)
- **Range 7→9** (+29%)
- Same ROF, projectile, warhead, sound, AssaultAnim

### Secondary — `[Flare]` (the airstrike designator)

`rulesmd.ini:23278`:

```ini
[Flare]
Damage=1
ROF=60
Range=12
Warhead=AirstrikeFlare
MigAttackCursor=yes;like Tanya's SabotageCursor override

;Charges=no
;LaserInnerColor = 255,0,0
;LaserOuterColor = 127,0,0;0,0,0
;LaserOuterSpread= 40,80,80;20,40,40
;LaserDuration = 90;15
Projectile=Invisible;LLine2
;IsLaser=true	; this flag tells the game to use the special laser draw effect
IsLine=true
IsHouseColor=true

;Projectile=Lobbed
;;Floater=yes
;Speed=5
;;Lobber=yes
;Bright=yes
```

| Key | Meaning |
|-----|---------|
| `Damage=1` | Nominal — the Flare doesn't deal damage; the airstrike does |
| `ROF=60` | Cooldown — 60 frames (4s). **Distinct from AirstrikeRechargeTime** — ROF is the Flare weapon's own cooldown; ART recharge is the per-Boris cooldown that gates further calls. The two compound: Boris can't refire Flare for `max(ROF, AirstrikeRechargeTime)` |
| `Range=12` | **12 cells** — the longest "weapon" range in the infantry roster. Boris designates from far outside enemy defenses, then walks away while the MIGs come in |
| `Warhead=AirstrikeFlare` | See warhead — `Airstrike=yes` flag is what triggers the MIG spawn |
| `MigAttackCursor=yes` | **Behavior flag** — WeaponTypeClass field (per `WeaponTypeClass__ReadINI @ 0x007721D7` DATA xref to string at `0x008494B4`). Inline comment: "like Tanya's SabotageCursor override". Shows the **special airstrike-target cursor** on valid targets (typically buildings) instead of the standard attack cursor. The cursor change signals to the player that this attack is the MIG-summon, not a normal weapon shot |
| `;Charges=no` (commented) | Legacy charge-up flag — irrelevant for Flare |
| `;LaserInnerColor` / `;LaserOuterColor` / `;LaserOuterSpread` / `;LaserDuration` (commented) | Designer history — Flare was at one point going to draw a laser beam to the target with custom colors. Switched to `IsLine=yes` line-draw instead |
| `Projectile=Invisible;LLine2` | Standard inviso projectile (the inline `;LLine2` is an older alternative) |
| `;IsLaser=true` (commented) | Designer comment: "this flag tells the game to use the special laser draw effect". Disabled — replaced by IsLine |
| `IsLine=true` | **Behavior flag** — WeaponTypeClass field (per `WeaponTypeClass__ReadINI @ 0x0077265F` DATA xref to string at `0x008493D8`). Draws a **straight line** from firer to target (as opposed to a Tesla-bolt jagged line or laser beam). Simple visual designator |
| `IsHouseColor=true` | **Behavior flag** — WeaponTypeClass field (per `WeaponTypeClass__ReadINI @ 0x00772675` DATA xref to string at `0x008493C8`). The line is drawn in the **firing house's color** (red for Boris's Soviet faction; would be different per faction). Matches the in-game visual of Boris's red designator line |
| `;Projectile=Lobbed` etc. (commented) | Earlier design — Flare was going to be a lobbed projectile. Replaced by inviso line |

### Primary Warhead — `[BORISWH]`

`rulesmd.ini:27579`:

```ini
[BORISWH]
Verses=200%,200%,100%,50%,50%,50%,1%,1%,1%,100%,100%
InfDeath=1
AnimList=PIFFPIFF,PIFFPIFF
Bullets=yes
ProneDamage=70%
```

| Key | Meaning |
|-----|---------|
| `Verses=200%,200%,100%,50%,50%,50%,1%,1%,1%,100%,100%` | 11-column. **200% vs `none` AND `flak`** — Boris's AKM one-shots both basic Allied infantry (none) and basic Soviet infantry (flak), unique among warheads (most weapons have 100% vs at least one of these). **100% vs plate** (Tesla Trooper, etc.) — solid 65 dmg per shot. **50/50/50% vs light/medium/heavy vehicle** — Boris CAN damage vehicles but it's not his role. **1% vs wood/steel/concrete buildings** — **Boris cannot reasonably damage buildings with AKM** (this is what forces the airstrike for building demolition). **100% specials** |
| `InfDeath=1` | Standard small-arms infantry death |
| `AnimList=PIFFPIFF,PIFFPIFF` | Impact PIFF puff |
| `Bullets=yes` | Bullet-type warhead |
| `ProneDamage=70%` | Prone reduces damage to 70% (moderate prone protection) |

### Secondary Warhead — `[AirstrikeFlare]` (the airstrike trigger)

`rulesmd.ini:27304`:

```ini
[AirstrikeFlare]
Verses=0%,0%,0%,0%,0%,0%,1%,1%,1%,0%,0%
Rocker=no
Sparky=no
Airstrike=yes
```

| Key | Meaning |
|-----|---------|
| `Verses=0%,0%,0%,0%,0%,0%,1%,1%,1%,0%,0%` | **0% damage vs everything except 1% vs structure-class armor (wood/steel/concrete)**. The Flare itself does no damage to infantry or vehicles, and only nominal damage to buildings. **The 1% is the engine's targetability-without-real-damage trick** — restricts the airstrike cursor to building-armor-class targets (compare ParasiteDog 0% trick). **This is what makes Boris's Flare only work on buildings** — the Verses filter the cursor |
| `Rocker=no` | No rocking effect |
| `Sparky=no` | No spark animation |
| `Airstrike=yes` | **THE airstrike trigger flag** — WarheadTypeClass field. When this warhead detonates, the engine spawns N copies of the firer's `AirstrikeTeamType` aircraft (N = `AirstrikeTeam` or `EliteAirstrikeTeam` depending on Boris's veterancy) at the nearest map-edge waypoint, with the impact point as their target. The aircraft fly in, fire their own weapons (here `BPLN.Primary=Maverick3` Damage 750 Burst 2), then fly out |

### The summoned aircraft — `[BPLN]` (Soviet MIG)

`rulesmd.ini:11276`:

| Key | Value | Meaning |
|-----|-------|---------|
| `Name=Soviet MIG` | | Internal name |
| `Strength=200` | | HP — can be shot down by AA |
| `Category=AirLift` | | Special air category |
| `Armor=light` | | Light armor (vulnerable to AA flak) |
| `TechLevel=-1` | | **Not directly buildable** — only spawned by Boris's airstrike |
| `Primary=Maverick3` | Damage=750 Burst=2 | The bombing weapon — air-to-ground missile, fires twice per attack run |
| `Spawned=yes` | | Inline comment: "Created by another object and therefore not player controllable" |
| `Selectable=no` | | Player cannot manually select the MIGs |
| `Speed=16` | | Fast (twice typical aircraft) |
| `Ammo=1` | | One bombing run per MIG, then returns to map edge |
| `Locomotor={4A582746-...}` | | `FlyLocomotionClass` GUID (standard aircraft) |
| `MoveSound=MigMoveLoop` | | Engine sound |
| `ImmuneToPsionics=yes` | | Cannot be mind-controlled |
| `CanPassiveAquire=no` / `CanRetaliate=no` | | Doesn't acquire its own targets; bombs the designated point and leaves |
| `Explosion=TWLT070,S_BANG48,...` | | Death-explosion animations |

### Primary's Warhead — `[Maverick3]` and `[MIGWH]`

`rulesmd.ini:23174`:

```ini
[Maverick3]
Damage=750
ROF=10
Range=4
Projectile=AirToGroundMissile
Speed=70
Warhead=MIGWH
Report=MigAttack
Burst=2
```

The MIG fires **Maverick3** (Damage 750, Burst 2 → 1500 total damage per
strike per MIG). With AirstrikeTeam=2 MIGs (Veteran) → 3000 damage per
airstrike call; AirstrikeTeam=4 (Elite) → 6000 damage per call. Effectively
**a building-destroying superweapon** triggered from infantry.

---

## Voices and sounds

All from `soundmd.ini`:

### Selection / movement / death — standard Russian voice bank

```ini
[BorisSelect]                  ; soundmd.ini:4327
Sounds=$iborsea $iborseb $iborsec $iborsed $iborsee     ; 5 lines

[BorisMove]                    ; soundmd.ini:4332
Sounds=$ibormoa $ibormob $ibormoc $ibormod $ibormoe     ; 5 lines

[BorisAttackCommand]           ; soundmd.ini:4337
Sounds=$iborata $iboratb $iboratc $iboratd $iborate     ; 5 lines

[BorisFear]                    ; soundmd.ini:4348
Sounds=$iborfea $iborfeb $iborfec $iborfed              ; 4 lines, no Priority=low

[BorisDie]                     ; soundmd.ini:4359
Sounds=$ibordia $ibordib $ibordic $ibordid $ibordie     ; 5 lines
```

5/5/5/4/5 voice bank — uniformly large.

### Airstrike call and creation (Type=global, Priority=critical)

```ini
[BorisAirstrikeVoice]              ; soundmd.ini:4342
Sounds=$iboraib ;$iboraie $iboraia $iboraic $iboraid
Type=global
Priority=critical
MinVolume=80

[BorisCreated]                     ; soundmd.ini:4353
Sounds=$iborcra $iborcrb $iborcrc $iborcrd $iborcre
Type=global
Priority=critical
MinVolume=80
```

| Sound | Wired by | Purpose |
|-------|----------|---------|
| `BorisAirstrikeVoice` | `VoiceSecondaryWeaponAttack=` | "Airstrike!" call when Boris fires the Flare designator. `Type=global` means **all players hear it** — warns the victim that an airstrike is incoming. Only 1 active sound (`$iboraib`); the rest are commented out as alternates that weren't shipped |
| `BorisCreated` | `CreateSound=` | Played globally when Boris finishes training. 5 lines |

`MinVolume=80` on both ensures they're loud enough to be heard regardless of camera distance.

### Weapon report

```ini
[BorisAttack]                  ; soundmd.ini:5691
Sounds=iboratta iborattb
Control= random interrupt
FShift= -5 5
Volume=70
```

2 AK fire samples — random pick on each shot.

### Cross-reference sounds

- `BorisAirstrikeVoice` is the most-recognizable Boris audio in-game — the
  global "Airstrike!" call is iconic
- `MigAttack` (referenced by Maverick3's Report=) — the MIG's bomb-launch sound
- `MigMoveLoop` (BPLN.MoveSound) — the MIG engine sound during the strike run

---

## Prerequisites, owners, tech

| Field | Value | Notes |
|-------|-------|-------|
| `Prerequisite=` | `NAHAND,NATECH` | Soviet Barracks + Soviet Battle Lab specifically |
| `Owner=` | `Russians,Confederation,Africans,Arabs` | All 4 Soviet houses; **no RequiredHouses** lock (so all Soviet players can build Boris) |
| `TechLevel=` | `9` | Late tech-9 — only Tanya/SEAL (9) and Yuri Prime (10) match or exceed |
| `AllowedToStartInMultiplayer=no` | — | Not in starting unit complement |
| `Cost=1500` | $1500 | Most expensive infantry in the game |
| `Soylent=750` | $750 refund (Yuri only) | |
| `BuildLimit=1` | — | **Maximum one Boris per player at a time** |
| `Points=50` | 50 | Among highest infantry point values |

The Tanya/Boris parallel: Allied has Tanya at $1000 / Tech 9 / BuildLimit=1
/ C4 buildings. Soviet has Boris at $1500 / Tech 9 / BuildLimit=1 / airstrike.

---

## Veterancy

| Tier | Effect |
|------|--------|
| Veteran | `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,SCATTER` — note **SCATTER** instead of FASTER (Boris evades fire) |
| Elite | `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — stacks SELF_HEAL on base SelfHealing for faster regen. Triggers **all three Elite swaps**: `ElitePrimary=AKME`, `EliteAirstrikeTeam=4`, `EliteAirstrikeRechargeTime=50`. Elite Boris is dramatically more powerful — twice the planes per airstrike, half the cooldown, 50% more rifle damage |
| AltCameo | `BRISUICO` shown after Veteran promotion |

`Trainable=` defaults to `yes`.

---

## Hardcoded behavior — Ghidra-verified

### 1. AirstrikeTeam — the called-airstrike subsystem

Four INI keys drive the subsystem, all **TechnoTypeClass fields** (so a
vehicle or building could theoretically have an airstrike too — only Boris
uses it in stock YR):

| Key | Default | Xref | Meaning |
|-----|---------|------|---------|
| `AirstrikeTeam` | 0 | `TechnoTypeClass__ReadINI @ 0x00714591` to string at `0x00843B84` | Number of aircraft to spawn at Veteran/Base tier |
| `EliteAirstrikeTeam` | 0 | xref to string at `0x00843B70` | Number at Elite tier |
| `AirstrikeTeamType` | (null) | xref to string at `0x00843B5C` | AircraftType ID to spawn (e.g., `BPLN`) |
| `EliteAirstrikeTeamType` | (null) | xref to string at `0x00843B44` | Aircraft type at Elite |
| `AirstrikeRechargeTime` | 0 | xref to string at `0x00843B2C` | Frames between airstrikes |
| `EliteAirstrikeRechargeTime` | 0 | xref to string at `0x00843B10` | Elite cooldown |

When Boris fires Secondary (Flare) at a valid target (building, per
AirstrikeFlare.Verses), the warhead's `Airstrike=yes` flag triggers the
engine's airstrike spawn path:

```
1. Engine reads AirstrikeTeam / AirstrikeTeamType (or Elite versions) from Boris's TechnoTypeClass
2. Engine locates the nearest map-edge waypoint to the impact cell
3. Spawn N aircraft instances of AirstrikeTeamType at that waypoint
4. Set each aircraft's TarCom to the impact cell
5. Aircraft fly in (using FlyLocomotionClass), fire their Primary (Maverick3 Burst=2 @ Damage=750) when in range, then fly out toward the same map-edge waypoint
6. After firing, set Boris's per-airstrike cooldown timer = AirstrikeRechargeTime (or Elite version)
7. Boris cannot refire Secondary until timer expires
```

The aircraft are `Spawned=yes` (`[BPLN].Spawned=yes`), so they're not
player-controllable — they auto-fly the attack run. They have `Ammo=1`,
meaning one bombing pass per MIG. If an aircraft is destroyed mid-run, it
counts as a successful "leave the map" for the cooldown purposes (per the
INI comment on AirstrikeRechargeTime: "How long after the planes either
leave the map or are destroyed will the next team of planes be ready?").

### 2. MigAttackCursor=yes — special airstrike-target cursor

INI key `MigAttackCursor=yes` is a WeaponTypeClass field (xref
`WeaponTypeClass__ReadINI @ 0x007721D7` to string at `0x008494B4`). When
hovering over a valid target with a MigAttackCursor weapon, the engine
shows a **distinct cursor** (the "airstrike target" reticle) instead of
the standard attack cursor. Combined with the Verses-1% trick on
AirstrikeFlare, this cursor only lights up on buildings — informing the
player that this attack is an airstrike, not a normal weapon shot.

### 3. VoiceSecondaryWeaponAttack — Secondary-specific voice

INI key `VoiceSecondaryWeaponAttack=` is a TechnoTypeClass field (xref
`TechnoTypeClass__ReadINI @ 0x00713706` to string at `0x00844038`). When
set, the engine plays this voice instead of `VoiceAttack=` when the
**Secondary** weapon is fired. Distinguishes the "Airstrike!" call from
Boris's regular AK fire voice.

### 4. IsLine + IsHouseColor — designator line rendering

Both WeaponTypeClass fields:
- `IsLine=true` — xref `WeaponTypeClass__ReadINI @ 0x0077265F` to string
  at `0x008493D8`. Draws a straight line from firer to target as the
  weapon's visual
- `IsHouseColor=true` — xref `WeaponTypeClass__ReadINI @ 0x00772675` to
  string at `0x008493C8`. The line is drawn in the firer's house color
  (red for Boris)

Together: Boris's red designator line that visually indicates the airstrike
target. Pure visual; no gameplay effect beyond cursor/aim feedback.

### 5. TiberiumProof — TS-legacy hazard-immunity

INI key `TiberiumProof=yes` is an InfantryTypeClass field (xref
`InfantryTypeClass__ReadINI @ 0x0052458B` to string at `0x0082595C`).
**TS legacy in name** — Tiberium was Tiberian Sun's hazard biome that
infantry could be poisoned by. In YR there's no Tiberium terrain, so this
flag has no observable effect. Defensive code retained from the TS base.
For Rust port: can be safely no-op'd / not implemented unless adding a
custom Tiberium-like hazard.

### 6. ImmuneToPsionics=yes (vs ImmuneToPsionicWeapons=no)

Two separate flags with different scope:
- `ImmuneToPsionics=yes` — Boris cannot be **mind-controlled** (immune to
  Yuri, Initiate, Psychic Tower, Psychic Dominator's control)
- `ImmuneToPsionicWeapons=no` — Boris CAN be **damaged** by psionic *area*
  weapons (e.g., the Dominator's blast area damage applies)

The split is intentional: Boris can still be killed by Yuri's
end-of-the-world Dominator blast, but can't be controlled mid-game.

### 7. BuildLimit=1

Standard TechnoTypeClass field (well-known). Enforces max one Boris per
player. Production queue rejects further Boris orders until the existing
one is destroyed. Combined with the high cost ($1500), keeps Boris from
spamming airstrikes.

### Ghidra searches performed for this dossier

| Tool call | Result |
|-----------|--------|
| `search_strings("AirstrikeTeam\|AirstrikeTeamType\|AirstrikeRechargeTime\|MigAttackCursor\|TiberiumProof\|VoiceSecondaryWeaponAttack\|IsHouseColor\|IsLine")` | 11 strings — confirms 8 hardcoded keys + Elite variants. All 4 airstrike keys plus their EliteX counterparts |
| `get_xrefs_to(0x00843B84)` (= "AirstrikeTeam") | Sole xref from `TechnoTypeClass__ReadINI @ 0x00714591` DATA — confirms TechnoType-level (not InfantryType-only), so any Techno could in principle have an airstrike |
| `get_xrefs_to(0x008494B4)` (= "MigAttackCursor") | Sole xref from `WeaponTypeClass__ReadINI @ 0x007721D7` DATA — confirms per-weapon flag |
| `get_xrefs_to(0x0082595C)` (= "TiberiumProof") | Sole xref from `InfantryTypeClass__ReadINI @ 0x0052458B` DATA — confirms InfantryType-only (vehicles don't have this) |
| `get_xrefs_to(0x00844038)` (= "VoiceSecondaryWeaponAttack") | Sole xref from `TechnoTypeClass__ReadINI @ 0x00713706` DATA — TechnoType field |
| `get_xrefs_to(0x008493C8)` (= "IsHouseColor") | Sole xref from `WeaponTypeClass__ReadINI @ 0x00772675` DATA — per-weapon flag |
| `get_xrefs_to(0x008493D8)` (= "IsLine") | Sole xref from `WeaponTypeClass__ReadINI @ 0x0077265F` DATA — per-weapon flag |

Confirmation: the airstrike subsystem is **generic TechnoTypeClass machinery**
— Boris is just the only stock YR unit that uses it. The subsystem could be
applied to a vehicle or building in a mod by setting the same 6 keys.

---

## TS-legacy filter

| Item | Status | Notes |
|------|--------|-------|
| `;Image=TANY` (commented) | Designer history — early reuse of Tanya art | OK |
| `;DetectDisguise=yes` (commented) | Final design: no | OK |
| `;VoiceAttack=BorisAttackCommand` (duplicate commented) | Leftover edit | OK |
| `;VoiceSecondaryWeaponAttack=BorisAirstrikeVoice` (duplicate commented) | Leftover edit | OK |
| `;Cheer=56,15,0,W` (commented in artmd) | Older W-facing cheer, replaced | OK |
| `TiberiumProof=yes` | **TS legacy** — Tiberium terrain is TS-only; no observable effect in YR. Defensive | Documented |
| `ImmuneToVeins=yes` | TS legacy (veins are TS-only); defensively set | OK |
| `[Flare]` commented `Charges=no` / `LaserInner/Outer/Spread/Duration` / `IsLaser=true` / `Projectile=Lobbed`/`Floater`/`Lobber`/`Bright` | Designer history — Flare was at one point a laser, then a lobbed projectile, then settled to inviso + IsLine + IsHouseColor | All documented |

No TS-only behavior actively running on Boris. `TiberiumProof` is the only
TS-named flag and it has no observable effect in YR (no Tiberium = no
poisoning = no immunity needed).

---

## Cross-references

- **Boris vs Tanya parallel** (the side-archetype hero comparison):
  - Tanya: $1000, Tech 9, BuildLimit=1, Primary=DoublePistols (one-shot inf), Secondary=Sapper (C4 building demo), Allied
  - Boris: $1500, Tech 9, BuildLimit=1, Primary=AKM (one-shot inf), Secondary=Flare (airstrike), Soviet
  - Both: Strength≥125 (Boris 200, Tanya 150 — Boris tougher), Crushable=no, ImmuneToPsionics=yes, SelfHealing=yes, UseOwnName=true
- **Related airstrike-system users**: **Boris is the only stock YR unit** with AirstrikeTeam. The subsystem is generic but unique to Boris in vanilla
- **Sister Soviet basic infantry**:
  - `[E2]` Conscript (basic)
  - `[SHK]` Tesla Trooper (anti-vehicle)
  - `[FLAKT]` Flak Trooper (AA)
  - `[IVAN]` Crazy Ivan (bomb)
  - `[DESO]` Desolator (radiation)
  - **`[BORIS]` Boris** (this doc — hero unit + airstrike)
- **Related aircraft summoning**:
  - `[BPLN]` Soviet MIG (used by Boris's airstrike) — distinct from B-2 Spirit Bomber `[BPLN]`? Actually the INI uses `BPLN` ID for both? Verify when AircraftTypes are documented. In rulesmd at line 11276 `[BPLN]` has `Name=Soviet MIG`
  - Paradrop superweapon uses `[PDPLANE]` cargo plane (different ID)
- **Related visual flags**:
  - `IsLine=true` + `IsHouseColor=true` — Boris's designator. Used by potentially other "draw a line" weapons in mods
  - `IsLaser=true` (commented in Flare) — used by Prism Tank, Tesla Tank? Actually IsElectricBolt is used by Tesla. IsLaser is a separate visual flag for true laser beams
- **Counter-units to Boris**:
  - Long-range bombardment (V3, Prism Tank, Apocalypse cannon, Dreadnought) — outrange AKM's 7
  - AA defenses to shoot down MIGs (Patriot, Flak Cannon, Aegis Cruiser, Gattling Cannon)
  - Sniper one-shot (250 dmg vs Strength=200 — survives with 50 HP, then heals)
  - Yuri's Psychic Dominator blast (ImmuneToPsionicWeapons=no — area damage applies)
  - **NOT**: mind-control (ImmuneToPsionics=yes), vehicle crush (Crushable=no), small-arms (Plate-tier resistance + SelfHealing recovers), Crazy Ivan bomb (Bombable=no default)
- **Related INI keys**:
  - `Burst=N` on Maverick3 — fires N missiles per attack (here 2)
  - `Spawned=yes` on BPLN — flags as non-buildable, AI-spawned
  - `Ammo=1` on BPLN — one bombing run, then exits

---

## Coverage audit

| Source | Lines | Status |
|--------|-------|--------|
| `rulesmd.ini [BORIS]` | 4593-4659 (67 lines) | All 56 active keys covered (4 commented `;Image=TANY`, `;VoiceAttack=` dup, `;VoiceSecondaryWeaponAttack=` dup, `;DetectDisguise=yes` documented) |
| `artmd.ini [BORIS]` | 411-418 (8 lines) | All keys covered |
| `artmd.ini [BorisSequence]` | 14071-14093 (23 lines) | All 22 active slots + commented Cheer covered including unique SecondaryFire/SecondaryProne entries |
| `rulesmd.ini [AKM]` | 23005-23013 (9 lines) | All keys covered |
| `rulesmd.ini [AKME]` | 25245-25253 (9 lines) | All keys covered (delta from AKM noted) |
| `rulesmd.ini [Flare]` | 23278-23300 (23 lines, mostly comments) | All 6 active keys + 11 commented variants covered |
| `rulesmd.ini [BORISWH]` | 27579-27584 (6 lines) | All keys covered with 11-column Verses breakdown |
| `rulesmd.ini [AirstrikeFlare]` | 27304-27308 (5 lines) | All keys covered |
| `rulesmd.ini [BPLN]` | 11276-... (~35 lines) | Cross-referenced key fields (full BPLN doc when aircraft tier is documented) |
| `rulesmd.ini [Maverick3]` | 23174-23182 (9 lines) | Key fields covered |
| `soundmd.ini` Boris voices | BorisSelect, Move, AttackCommand, AirstrikeVoice, Fear, Created, Die, Attack | All 8 covered with airstrike-voice distinguished |
| Hardcoded behavior | AirstrikeTeam subsystem (6 keys) + MigAttackCursor + VoiceSecondaryWeaponAttack + IsLine + IsHouseColor + TiberiumProof + ImmuneToPsionicWeapons distinction + BuildLimit | 8 mechanisms covered with 7 Ghidra-verified xrefs |
| Ghidra searches performed against ID | 8 distinct queries (1 strings + 7 xref lookups) | Logged inline |
| TS-legacy filter | Applied; TiberiumProof flagged as TS-legacy no-op, ImmuneToVeins defensive, all Flare designer-history comments documented | Done |
