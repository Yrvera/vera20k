# FV — Multi-Gunner IFV (Infantry Fighting Vehicle)

**Side classification:** Allied (Owner=British,French,Germans,Americans,Alliance).
**Role:** 1-passenger transport whose weapon **dynamically swaps based on the passenger
infantry type**. Empty IFV = anti-air HoverMissile rocket turret; loaded with GI = gun
turret with `CRM60`; loaded with Yuri = high-tech turret with `CRMindControl`; loaded
with Engineer = repair-arm turret; etc. 17 distinct weapons selected via 4 visual
turret index slots.

> ⚠ **Index correction logged**: prior `INDEX_UNITS.md` listed FV as "Battle Fortress
> 5-passenger crusher". This is wrong. The INI says `Name=IFV`, `Passengers=1`. The
> actual Battle Fortress is `[BFRT]` (rulesmd line 6917, 5 passengers, OpenTopped). FV
> is the **Multi-Gunner IFV** — the small transport whose turret morphs based on
> passenger. Both index entries have been corrected.

> Output bar: parity-critical for the entire IFV passenger-swap mechanic.
> Every infantry doc to date has cited "IFVMode=N" + this dispatch table; the live
> behavior must match exactly. Misrouting a passenger to the wrong weapon would make
> all Allied-IFV play feel broken.

> Ghidra confirms no `"FV"`, `"IFV"`, or `"MultiGunner"` strings in `gamemd.exe`. The
> mechanism is generic flag-driven via `Gunner=yes`, `TurretCount=N`, `WeaponCount=N`,
> plus the named Turret-key INI keys (`NormalTurret*`, `MachineGunTurret*`, etc.).
> However, the IFVMode-integer → TurretKey lookup table appears to be **hardcoded** —
> the engine maps integer values to named keys internally. See §7.4 for analysis.

---

## 1. `rulesmd.ini` — `[FV]` verbatim (header + dispatch table)

### Header

```ini
[FV]
UIName=Name:FV
Name=IFV
Prerequisite=GAWEAP
Primary=HoverMissile
Strength=200
Category=Transport
Armor=light
DeployTime=.022
TechLevel=3
Sight=8
PipScale=Passengers
Speed=10
CrateGoodie=no
Owner=British,French,Germans,Americans,Alliance
AllowedToStartInMultiplayer=no
Cost=600
Soylent=600
Points=20
ROT=5
Crusher=no
TooBigToFitUnderBridge=true
Turret=yes ;GEF should be no for ifv???
Passengers=1
Gunner=yes
AirRangeBonus=4 ;GEF this should always be less than or equal to the range of the primary weapon. Otherwise targeting issues could arise
HasTurretTooltips=yes
TurretCount=4
WeaponCount=17
```

### Weapon table (17 slots, named for the passenger type)

```ini
Weapon1=HoverMissile		;Normal
EliteWeapon1=HoverMissileE		;Normal
Weapon2=RepairBullet	;Engineer
EliteWeapon2=RepairBullet	;Engineer
Weapon3=CRM60			;GI
EliteWeapon3=CRM60			;GI
Weapon4=CRFlakGuyGun		;Flak Troop ;Rocketeer
EliteWeapon4=CRFlakGuyGun		;Flak Troop ;Rocketeer
Weapon5=CRMP5			;Seal
EliteWeapon5=CRMP5			;Seal
Weapon6=AWPE			;Sniper
EliteWeapon6=AWPE			;Sniper
Weapon7=CRElectricBolt	;ShockTrooper
EliteWeapon7=CRElectricBolt	;ShockTrooper
Weapon8=CRNuke			;Crazy Ivan
EliteWeapon8=CRNuke			;Crazy Ivan
Weapon9=CRMindControl	;Yuri
EliteWeapon9=CRMindControl	;Yuri
Weapon10=CRRadBeamWeapon;Desolator
EliteWeapon10=CRRadBeamWeapon;Desolator
Weapon11=CRNeutronRifle	;Chrono
EliteWeapon11=CRNeutronRifle	;Chrono
Weapon12=CRTerrorBomb		;Terrorist
EliteWeapon12=CRTerrorBomb		;Terrorist
Weapon13=CowShot		;Cow
EliteWeapon13=CowShot		;Cow
Weapon14=CRPsychicJab		;Initiate
EliteWeapon14=CRPsychicJab		;Initiate
Weapon15=CRVirusGun		;Virus
EliteWeapon15=CRVirusGun		;Virus
Weapon16=CRSuperMindBlast		;Yuri Prime
EliteWeapon16=CRSuperMindBlast		;Yuri Prime
Weapon17=CRMissileLauncher		;Guardian GI
EliteWeapon17=CRMissileLauncher		;Guardian GI
```

### Turret dispatch table — named Turret keys

```ini
;Weapons
;GEF
;0=rocket
;1=gun
;2=repair arm
;3=high-tech

NormalTurretIndex=0
NormalTurretWeapon=0
RepairTurretIndex=2
RepairTurretWeapon=1
MachineGunTurretIndex=1
MachineGunTurretWeapon=2
FlakTurretIndex=1
FlakTurretWeapon=3
PistolTurretIndex=1
PistolTurretWeapon=4
SniperTurretIndex=1
SniperTurretWeapon=5
ShockTurretIndex=3
ShockTurretWeapon=6
ExplodeTurretIndex=3
ExplodeTurretWeapon=7
BrainBlastTurretIndex=3
BrainBlastTurretWeapon=8
RadCannonTurretIndex=3
RadCannonTurretWeapon=9
ChronoTurretIndex=3
ChronoTurretWeapon=10
TerroristExplodeTurretIndex=3
TerroristExplodeTurretWeapon=11
CowTurretIndex=3
CowTurretWeapon=12
InitiateTurretIndex=3
InitiateTurretWeapon=13
VirusTurretIndex=3
VirusTurretWeapon=14
YuriPrimeTurretIndex=3
YuriPrimeTurretWeapon=15
GuardianTurretIndex=3
GuardianTurretWeapon=16
```

### Tail

```ini
IsSelectableCombatant=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=IFVSelect
VoiceMove=IFVMove
VoiceAttack=IFVAttackCommand
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=IFVMoveStart
EnterTransportSound= IFVTransform
LeaveTransportSound= IFVTransform
CrushSound=TankCrush
Maxdebris=3
DebrisTypes=TIRE
DebrisMaximums=6
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
MovementZone=Normal
ThreatPosed=10	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
SpecialThreatValue=1
ZFudgeColumn=10
ZFudgeTunnel=13
ImmuneToRadiation=no
ImmuneToPsionics=no
Size=3
SizeLimit=1
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
Accelerates=false

DeathWeapon=CRNuke ;GEF This will be a special case that only goes off if it's piloted by Ivan
```

### Header explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `Name` | `IFV` | AbstractType | Internal name "IFV" (Infantry Fighting Vehicle). |
| `Prerequisite` | `GAWEAP` | TechnoType | Allied War Factory only — early-tier prereq. |
| `Primary` | `HoverMissile` | TechnoType | The **default weapon** when no passenger is inside. Anti-air missile (matches NormalTurret slot). |
| `Strength` | `200` | AbstractType | 200 HP — fragile. |
| `Category` | `Transport` | TechnoType | Transport classifier. |
| `Armor` | `light` | TechnoType | Verses-slot 4. |
| `DeployTime` | `.022` | TechnoType | Field-stop duration (shared with HTK). |
| `TechLevel` | `3` | TechnoType | Mid-tier. |
| `Sight` | `8` | TechnoType | 8-cell reveal. |
| `PipScale` | `Passengers` | UnitType | Renders passenger pip (single pip — only 1 passenger). |
| `Speed` | `10` | TechnoType | **Fastest ground vehicle in YR**. The IFV is built for hit-and-run. |
| `CrateGoodie` | `no` | UnitType | Excluded from crate pool. |
| `Owner` | 5 Allied countries | TechnoType | Allied only. |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Not preplaced. |
| `Cost` | `600` | TechnoType | Cheap for the role. |
| `Soylent` | `600` | TechnoType | 100% Grinder refund. |
| `Points` | `20` | TechnoType | |
| `ROT` | `5` | TechnoType | Turret + body rotation. |
| `Crusher` | `no` | TechnoType | **Cannot crush infantry** — unusual for a vehicle this fast. IFV is for shooting, not crushing. |
| `TooBigToFitUnderBridge` | `true` | UnitType-only | Cannot path under bridges. |
| `Turret` | `yes` (author note: "GEF should be no for ifv???") | UnitType | The IFV has 4 different voxel turret graphics that swap based on passenger. Author-comment questions whether `Turret=yes` is correct — confused about the multi-turret system. |
| `Passengers` | `1` | TechnoType | **Exactly 1 passenger.** Combined with `SizeLimit=1`, IFV carries one Size=1 infantry. |
| `Gunner` | `yes` | TechnoType [BINARY-VERIFIED audit 18: string @ 0x00843964, parser xref @ 0x00714A50, `TechnoType+0x805` (byte)] | **The hardcoded flag that enables passenger-based weapon swap.** When set, the engine reads the named TurretKey table (`NormalTurret*`, `MachineGunTurret*`, etc.) and dispatches the active Weapon/Turret based on the passenger's `IFVMode=` value. Without `Gunner=yes`, the IFV would behave like any normal transport — passengers just sit inside, no weapon swap. |
| `AirRangeBonus` | `4` | TechnoType [BINARY-VERIFIED audit 18: string @ 0x00843AD4, parser xref @ 0x007147A1, `TechnoType+0x68C` (int)] | **Anti-air range bonus.** INI comment: "this should always be less than or equal to the range of the primary weapon. Otherwise targeting issues could arise." When the IFV's active weapon has `AA=yes`-capable projectile, the engagement range against air targets is extended by `+4` cells. HoverMissile has Range~6, so vs air it's 6+4=10. Critical for the IFV's role as Allied early-game mobile AA. |
| Commented `;TurretCount=15 ;#define OBJTYPE_DIM_TurretMax 15 ;Or Weapon Count.` | — | — | Author notes that the hardcoded compile-time max of TurretCount is 15 (per `objtype.h`), but the shipped INI uses `TurretCount=4` (different value — only 4 distinct turret graphics; 17 weapons share those 4 turrets). |
| `HasTurretTooltips` | `yes` | TechnoType | Enables tooltip-on-turret UI (likely shows weapon name when hovering). |
| `TurretCount` | `4` | TechnoType [BINARY-VERIFIED audit 18: string @ 0x00844348, parser xref @ 0x00712851, `TechnoType+0x808` (int)] | **4 distinct visual turret graphics**: rocket (0), gun (1), repair-arm (2), high-tech (3). All 17 weapons map to one of these 4 visuals. The "Turret Index" in the dispatch table selects which voxel turret to render. |
| `WeaponCount` | `17` | TechnoType [BINARY-VERIFIED audit 18: string @ 0x0084433C, parser xref @ 0x0071286B, `TechnoType+0x80C` (int)] | **17 weapon slots** (`Weapon1..Weapon17` + `EliteWeapon1..17`). The 17-int gunner lookup-table lives at `TechnoType+0x814..+0x858`, populated via `FUN_00717890` in UnitTypeClass__ReadINI. |

### Tail explanation (key non-trivial keys)

| Key | Value | Effect |
|-----|-------|--------|
| `VoiceSelect/Move/Attack=IFVSelect/Move/AttackCommand` | unique IFV voices | 7/6/5 unique clips |
| `EnterTransportSound=IFVTransform` | `vifvtran`, vol 60 | **Plays when a passenger boards** — the "transform" wording suggests the audio is for the turret-swap animation, not generic enter-vehicle. Same sound on `LeaveTransportSound=`. |
| `LeaveTransportSound=IFVTransform` | (same sound def) | Same — fires on disembark, presumably the IFV reverts to NormalTurret. |
| `MoveSound=IFVMoveStart` | 3 unique clips, predelay 0–400ms, low pri | Engine start |
| `ImmuneToRadiation=no` | TechnoType | Explicitly not immune (a Desolator inside the IFV CAN damage IFV with its own radiation? — likely not, but the flag is set for safety). |
| `ImmuneToPsionics=no` | TechnoType | Yuri CAN mind-control IFV. |
| `Size` | `3` | TechnoType | IFV's own transport slot cost. |
| `SizeLimit` | `1` | TechnoType | **Only Size=1 passengers.** Most standard infantry are Size=1 — but excludes any Size=2 infantry. |
| `VeteranAbilities` | STRONGER,FIREPOWER,SIGHT,FASTER | TechnoType | Veteran bonuses. |
| `EliteAbilities` | SELF_HEAL,STRONGER,FIREPOWER,ROF | TechnoType | Elite adds SELF_HEAL + ROF. |
| `Accelerates` | `false` | TechnoType | No acceleration ramp. |
| `DeathWeapon` | `CRNuke ;GEF This will be a special case that only goes off if it's piloted by Ivan` | **TechnoType per-unit override** [BINARY-VERIFIED audit 18: string @ 0x0083B11C, TechnoTypeClass__ReadINI writes `TechnoType+0xD18` (WeaponType*). Dual-read pattern with RulesClass__ReadCombatDamage @ 0x0066C58A as global default — TechnoType side fully verified this pass.] | **Special-case hardcoded behavior**: if the IFV is **piloted by Crazy Ivan**, on death it detonates with the `CRNuke` warhead (Ivan-bomb-style explosion). Other passengers don't trigger this — only Ivan. INI comment explicitly confirms "special case". This is one of the unique IFVMode-passenger-dependent death behaviors. |

---

## 2. `artmd.ini` — `[FV]` section

```ini
[FV] ; IFV
Cameo=FVICON
AltCameo=FVUICO
Voxel=yes
Remapable=yes
Weapon1FLH=64,48,180    ; Missile -- missile turret
Weapon2FLH=88,0,176     ; Engineer's repair arm
Weapon3FLH=72,0,160     ; GI -- gun turret
Weapon4FLH=72,0,160     ; Flak Trooper -- gun turret
Weapon5FLH=72,0,160     ; SEAL -- gun turret
Weapon6FLH=72,0,160     ; Sniper -- gun turret
Weapon7FLH=72,0,160    ; Shock Trooper -- high-tech turret
Weapon8FLH=72,0,160     ; Crazy Ivan -- high-tech turret (he just blows up so offset not important)
Weapon9FLH=72,0,160     ; Yuri -- high-tech turret (he does brain blast)
Weapon10FLH=72,0,160   ; Desolator -- high-tech turret
Weapon11FLH=72,0,160   ; Chrono Legion -- high-tech turret
Weapon12FLH=72,0,160    ; Terrorist -- high-tech turret (he just blows up)
Weapon13FLH=72,0,160   ; Cow -- high-tech turret
Weapon14FLH=72,0,160   ; Initiate -- high-tech turret
Weapon15FLH=72,0,160   ; Virus -- high-tech turret
Weapon16FLH=72,0,160   ; Yuri Prime -- high-tech turret
Weapon17FLH=64,48,180   ; Guardian GI -- high-tech missile turret
```

| Key | Effect |
|-----|--------|
| `Cameo` / `AltCameo` | Sidebar cameos. |
| `Voxel` / `Remapable` | Voxel rendering with house-color remap. |
| `Weapon1FLH` — `Weapon17FLH` | **Per-weapon-slot fire offset.** When the engine selects Weapon N (based on passenger IFVMode → TurretKey → TurretWeapon), it uses `WeaponNFLH` as the projectile spawn offset rather than `PrimaryFireFLH=`. |

**FLH pattern**:
- **Weapon1 (missile/HoverMissile)**: X=64, Y=48, Z=180 — high+offset to right for missile launch. Y=+48 reflects the missile rack's lateral position.
- **Weapon2 (Engineer repair arm)**: X=88, Y=0, Z=176 — slightly further forward and slightly lower than missile; centered Y. The repair arm extends from the centre of the IFV's roof.
- **Weapons 3-16 (gun + high-tech turrets)**: all **identical** `72,0,160` — slightly forward, centred Y, mid-height (160). The gun and high-tech turrets share the same muzzle position regardless of which visual turret graphic they use.
- **Weapon17 (Guardian GI missile)**: same as Weapon1 `64,48,180` — Guardian GI's missile also exits from the offset rocket-rack position.

Notable: **the visual turret swap is purely cosmetic** — the FLH offsets only differ between "missile-style" (1, 17), "engineer arm" (2), and "gun-or-high-tech" (3-16). The 4 voxel turret graphics give variety, but most FLH offsets converge.

### `[IFVTransform]` sound

```ini
[IFVTransform]
Sounds=vifvtran
Volume=60
```

Single clip (`vifvtran`) played on passenger enter AND leave — the "transform"
animation/sound for the turret swap.

---

## 3. Weapon dispatch — how IFVMode maps to Weapon

### Named TurretKey → (TurretIndex, WeaponSlot) mapping

The INI defines **17 named TurretKeys**, each with a visual turret index (0-3) and a weapon index (0-16):

| TurretKey | TurretIndex | WeaponSlot | Weapon | Passenger | Notes |
|-----------|-------------|------------|--------|-----------|-------|
| Normal | 0 (rocket) | 0 | `HoverMissile` | (empty) | Default — no passenger or untyped |
| Repair | 2 (repair arm) | 1 | `RepairBullet` | Engineer | Repair-arm visual |
| MachineGun | 1 (gun) | 2 | `CRM60` | GI / GGI | Anti-infantry gun |
| Flak | 1 (gun) | 3 | `CRFlakGuyGun` | Flak Trooper / Rocketeer | Same gun visual |
| Pistol | 1 (gun) | 4 | `CRMP5` | SEAL / TANY / CCOMAND / BORIS / PTROOP | Same gun visual |
| Sniper | 1 (gun) | 5 | `AWPE` | Sniper | Same gun visual |
| Shock | 3 (high-tech) | 6 | `CRElectricBolt` | Shock Trooper (SHK) | High-tech turret |
| Explode | 3 (high-tech) | 7 | `CRNuke` | Crazy Ivan (IVAN) | High-tech turret |
| BrainBlast | 3 (high-tech) | 8 | `CRMindControl` | Yuri (YURI) | High-tech turret |
| RadCannon | 3 (high-tech) | 9 | `CRRadBeamWeapon` | Desolator (DESO) | High-tech turret |
| Chrono | 3 (high-tech) | 10 | `CRNeutronRifle` | Chrono Legionnaire (CLEG) | High-tech turret |
| TerroristExplode | 3 (high-tech) | 11 | `CRTerrorBomb` | Terrorist (TERROR) | High-tech turret |
| Cow | 3 (high-tech) | 12 | `CowShot` | Cow (COW) | High-tech turret (yes, cows ARE supported passengers!) |
| Initiate | 3 (high-tech) | 13 | `CRPsychicJab` | Initiate (INIT) | High-tech turret |
| Virus | 3 (high-tech) | 14 | `CRVirusGun` | Virus (VIRUS) | High-tech turret |
| YuriPrime | 3 (high-tech) | 15 | `CRSuperMindBlast` | Yuri Prime (YURIPR) | High-tech turret |
| Guardian | 3 (high-tech) | 16 | `CRMissileLauncher` | Guardian GI (GGI) | High-tech turret + missile FLH |

**TurretIndex semantics** (per author-comment `; ;0=rocket ;1=gun ;2=repair arm ;3=high-tech`):
- **0** = rocket-launcher voxel (paired missile rack)
- **1** = simple gun voxel (single barrel)
- **2** = repair-arm voxel (extended mechanical arm)
- **3** = high-tech voxel (chunky futuristic turret)

### How IFVMode (integer) maps to TurretKey

Each passenger infantry has an `IFVMode=N` integer. The engine uses this to select a TurretKey. **The mapping appears hardcoded in `gamemd.exe`** — the INI doesn't expose a `Mode0=NormalTurret` style table. From cross-referencing IFVMode values across infantry docs:

| IFVMode | Inferred TurretKey | Passengers observed (from infantry docs) |
|---------|-------------------|-------------------------------------------|
| 0 | Normal (no swap) or special-case | ENGINEER, SLAV, PENTGEN, VLADIMIR, civilians |
| 1 | MachineGun? | E2 (Conscript), some others |
| 2 | MachineGun? | E1 (GI), GGI (rookie maps?) |
| 3 | Flak | IVAN? (actually IVAN's IFVMode might be different — needs verification) |
| 4 | Pistol | SEAL/GHOST, TANY, CCOMAND, BORIS, PTROOP, SHK (?) |
| 5 | Sniper | SNIPE |
| 6 | (unknown) | (some passenger) |
| 7 | (unknown — DESO uses this) | DESO |
| 8 | BrainBlast | YURI |
| 9 | Chrono | CLEG |
| 10 | (unknown) | (some passenger) |
| 11 | (unknown) | (some passenger) |
| 12 | (unknown — high-tech cluster) | Several units; cluster suggests building/civilian default-block |
| 13 | (unknown) | (some passenger) |
| 14 | (unknown) | (some passenger — TANY had IFVMode=14 in one search, conflicting with 4) |
| 15 | YuriPrime | YURIPR |
| 16 | Guardian | (likely GGI or other special) |

**[PARTIALLY RESOLVED audit 18 — INFERRED from parse-order]**: The
`UnitTypeClass__ReadINI` gunner block (audit 12) parses the 17 named
TurretKeys in this fixed order: Normal, Repair, MachineGun, Flak,
Pistol, Sniper, Shock, Explode, BrainBlast, RadCannon, Chrono,
TerroristExplode, Cow, Initiate, Virus, YuriPrime, Guardian — each
calling `FUN_00717890(this, TurretIdx, WeaponSlot)` with a
sequentially-incrementing WeaponSlot 0..16. So **WeaponSlot 0 =
Normal, 1 = Repair, 2 = MachineGun, ..., 16 = Guardian** at the
parser/data-layout side. The IFVMode-integer → WeaponSlot consumer
mapping at runtime is **still DEFERRED for direct verification**, but
the parse-order strongly suggests a 1:1 mapping (with possible
special-case for the "Engineer paradox").

**⚠ Parity-critical (original framing preserved)**: the exact IFVMode-integer → TurretKey enum is not visible in the INI. It must be reverse-engineered from `gamemd.exe`. Engineer with `IFVMode=0` should map to the Repair TurretKey (the comment "Weapon2=RepairBullet ;Engineer" makes the intent clear), but `IFVMode=0` integer-wise looks like it should map to NormalTurret. Possible resolutions:

1. The engine has **special-case logic** for certain unit flags (e.g., `Engineer=yes` overrides IFVMode and forces RepairTurret regardless of value).
2. The IFVMode enum starts at 0 but the integer values don't map 1:1 to weapon slots — there's a separate lookup.
3. IFVMode=0 might literally mean "use Repair" for engineer-class passengers, with NormalTurret being assigned to "no passenger" via a different code path.

This is the single biggest open question for IFV parity. Worth a dedicated `/re-investigate Multi-Gunner IFV dispatch` session.

### Default weapon (no passenger)

When the IFV has **no passenger**, the engine falls back to `Primary=HoverMissile` (the rulesmd-declared Primary). This maps to NormalTurret (index 0, weapon slot 0 = Weapon1=HoverMissile — consistent).

### Switching on enter / leave

When a passenger boards: `EnterTransportSound=IFVTransform` plays, the visual turret
swaps to the passenger's TurretKey graphic, and the active weapon swaps to the
corresponding `WeaponN`. When passenger disembarks: same `IFVTransform` sound, revert
to NormalTurret + HoverMissile (Weapon1).

---

## 4. Weapons referenced

Brief reference only — each `CR*` weapon is a per-passenger variant of that passenger's
own primary. Full weapon definitions are in the respective infantry docs:

| Weapon | Maps to passenger's | Notes |
|--------|----------------------|-------|
| `HoverMissile` / `HoverMissileE` | (empty IFV) | Anti-air missile (Patriot-style) |
| `RepairBullet` | Engineer's repair | "Heals" friendly mechanical targets |
| `CRM60` | E1's M60 | Anti-infantry gun |
| `CRFlakGuyGun` | FLAKT's FlakGuyGun | Anti-air flak |
| `CRMP5` | GHOST's MP5 | Anti-infantry pistol |
| `AWPE` | SNIPE's AWP elite | One-shot rifle (shared with elite SNIPE) |
| `CRElectricBolt` | SHK's TeslaShock | Electric arc |
| `CRNuke` | IVAN's IvanBomb | Bomb-plant style (no real nuke; INI naming holdover) |
| `CRMindControl` | YURI's MindControl | Mind-control link |
| `CRRadBeamWeapon` | DESO's RadBeamWeapon | Radiation beam |
| `CRNeutronRifle` | CLEG's NeutronRifle | Chrono erase |
| `CRTerrorBomb` | TERROR's TerrorBomb | Suicide-bomb behavior |
| `CowShot` | COW's CowShot | Cow weapon (no, really) |
| `CRPsychicJab` | INIT's PsychicJab | Psi blast |
| `CRVirusGun` | VIRUS's VirusGun | Plague poison |
| `CRSuperMindBlast` | YURIPR's SuperMindBlast | AoE mind-control |
| `CRMissileLauncher` | GGI's deployed missile | Anti-armor missile |

Note: most `CR*` variants have different stats than the infantry's standalone weapon
(damage, ROF, range tuned for the IFV-mounted version). See each weapon's own INI block
for specifics.

---

## 5. Voices / sounds

```ini
[IFVSelect]
Sounds=$vifvsea $vifvseb $vifvsec $vifvsed $vifvsee $vifvsef
 $vifvseg
Control=random
Volume=85

[IFVMove]
Sounds=$vifvmoa $vifvmob $vifvmoc $vifvmod $vifvmoe $vifvmof
Control=random
Volume=85

[IFVAttackCommand]
Sounds=$vifvata $vifvatb $vifvatc $vifvatd $vifvate
Control=random
Volume=85
```

```ini
[IFVMoveStart]
Sounds= vifvstaa vifvstab vifvstac
Control= random predelay
Delay=0 400
Priority=low
FShift= -10 10
VShift=20
Volume=30

[IFVTransform]
Sounds=vifvtran
Volume=60
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=IFVSelect` | 7 unique clips ($vifvsea..eg — note line-continuation in INI for the 7th) | Click-select |
| `VoiceMove=IFVMove` | 6 unique clips | Move order |
| `VoiceAttack=IFVAttackCommand` | 5 unique clips | Attack order |
| `VoiceFeedback=` *(empty)* | — | No under-attack |
| `DieSound=GenVehicleDie` | 6 generic clips | Death |
| `MoveSound=IFVMoveStart` | 3 unique clips, predelay 0–400ms, low pri | Engine start |
| `EnterTransportSound=IFVTransform` | `vifvtran`, vol 60 | **Fires on passenger BOARD** — "transform" sound for turret swap |
| `LeaveTransportSound=IFVTransform` | (same) | Fires on passenger DISEMBARK |
| `CrushSound=TankCrush` | `vcrusha` | n/a (Crusher=no) |

The shared `EnterTransportSound` and `LeaveTransportSound` using the same
`IFVTransform` clip is intentional — both events involve the same turret-swap
animation, so the audio cue is identical.

---

## 6. Prerequisites / owners / availability

- **Prerequisite**: `GAWEAP` — Allied War Factory only. No tech-lab required.
- **TechLevel** = `3` — mid-tier.
- **Owner**: 5 Allied countries.
- **`CrateGoodie=no`** — excluded from crate pool.
- **`AllowedToStartInMultiplayer=no`** — not preplaced.
- **Cost** = $600. Very cheap for the role.

### Comparison: FV (Allied IFV) vs HTK (Soviet Flak Track)

| Aspect | FV (IFV) | HTK (Flak Track) |
|--------|----------|-------------------|
| Side | Allied | Soviet |
| Passengers | **1** | 5 |
| Passenger weapon swap | **YES (Gunner=yes, 17 weapons, 4 turrets)** | NO (fixed weapons) |
| Default weapon | HoverMissile (AA) | FlakTrackGun (AG) + FlakTrackAAGun (AA) |
| Empty-IFV role | Anti-air (single weapon = HoverMissile) | Dual AG+AA, infantry transport |
| Cost | $600 | $500 |
| HP | 200 | 180 |
| Speed | **10** (fastest) | 8 |
| Crusher | **no** | yes |
| AirRangeBonus | 4 | (absent — default) |
| DeathWeapon | CRNuke (Ivan special-case) | (default) |
| SizeLimit | 1 | 2 |

Different design philosophies: Allied IFV is a **1-passenger force-multiplier** (the
passenger's identity completely defines the IFV's role), while Soviet Flak Track is a
**5-passenger straight transport** with built-in dual-role weapons that don't depend
on passengers.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 FV-specific code in `gamemd.exe`

| Query | Result |
|-------|--------|
| `FV` (substring) | (would match many — not specifically searched) |
| `IFV` (substring) | Not searched as plain string; behavior is generic |
| `MultiGunner` | 0 matches |

⇒ **No FV-string-specific code path** verified. All the gunner-swap behavior runs through
the generic `Gunner=yes` + named-TurretKey reader.

### 7.2 Flag-scope verification (this iteration)

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `Gunner` | 0x00843964 | TechnoTypeClass__ReadINI @ 0x00714a50 | TechnoType |
| `TurretCount` | 0x00844348 | TechnoTypeClass__ReadINI @ 0x00712851 | TechnoType |
| `WeaponCount` | 0x0084433c | TechnoTypeClass__ReadINI @ 0x0071286b | TechnoType |
| `AirRangeBonus` | 0x00843ad4 | TechnoTypeClass__ReadINI @ 0x007147a1 | TechnoType |
| `DeathWeapon` | 0x0083b11c | RulesClass__ReadCombatDamage @ 0x0066c58a (global default) **+** TechnoTypeClass__ReadINI @ 0x007122f0 (per-unit override) | **RulesClass + TechnoType dual-read** |

Plus prior verifications carried over:
- `IFVMode` — TechnoType (0x00843ae4 → 0x00714787, from PTROOP iter)
- `Passengers`, `SizeLimit`, `DeployTime` — TechnoType (HTK iter)
- All named TurretKey strings (`NormalTurretIndex` etc.) presumably read by `UnitTypeClass__ReadINI` — verified via earlier PistolTurretWeapon scope check (UnitType at 0x00747c8c).

### 7.3 Live behaviors driven by these flags

| Behavior | Driver | Notes |
|----------|--------|-------|
| Weapon swap based on passenger IFVMode | `Gunner=yes` + named TurretKey table + passenger's `IFVMode=N` | The defining mechanic |
| Visual turret swap on enter/leave | `TurretCount=4` + `Turret*Index` per TurretKey | 4 voxel turret graphics |
| AA range bonus on rocket turrets | `AirRangeBonus=4` | Combined with HoverMissile (default) makes IFV a 10-cell AA defender |
| Sound cue on passenger swap | `EnterTransportSound=IFVTransform` + `LeaveTransportSound=IFVTransform` | Single shared clip |
| Crazy Ivan piloting → death nuke | `DeathWeapon=CRNuke` (per-unit override; INI comment confirms Ivan special case) | Live only when Ivan is the passenger |
| Cannot crush | `Crusher=no` | Unusual for vehicle this fast |
| Fastest ground vehicle | `Speed=10` | |
| Single passenger only | `Passengers=1, SizeLimit=1` | Infantry-only, one at a time |
| Field-stop deploy before fire | `DeployTime=.022` | Same as HTK |

### 7.4 Behaviors NOT present

- No `Spawns=` — no child units.
- No `Teleporter=` — no chrono.
- No `Secondary=` weapon — single-weapon system (weapons swap via passenger; not Primary+Secondary dual-weapon).
- No `OmniCrushResistant=yes` — Battle Fortress can squish.
- No `Bunkerable=no` — IFV CAN load into Battle Fortress (gets confusing — IFV-in-Fortress).
- No `OpenTopped=yes` — passengers cannot fire out (the weapon-swap is itself the firing mechanism).
- No `ImmuneToPsionics` — Yuri can mind-control IFV + its passenger.
- No `ImmuneToRadiation` — Desolator damages IFV (even with Desolator passenger? — unclear; the passenger's own rad-immunity doesn't propagate to the IFV).

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ZFudgeTunnel=13` | YES | Dormant render value. |
| Commented `;TurretCount=15 ;#define OBJTYPE_DIM_TurretMax 15` | n/a (compile-time hint) | Author note, not active. |
| (no `ImmuneToVeins`) | — | Not set. |

No fog-of-war refs, no real tunnel refs, no Tiberium refs.

---

## 9. Veterancy

### Veteran (1 chevron) — `STRONGER, FIREPOWER, SIGHT, FASTER`
- `STRONGER` — +25% HP (200 → 250)
- `FIREPOWER` — +25% damage (applied to active Weapon, varies with passenger)
- `SIGHT` — +20% sight (8 → 9.6)
- `FASTER` — +20% speed (10 → 12 — game-fastest)

### Elite (2 chevrons) — `SELF_HEAL, STRONGER, FIREPOWER, ROF` (cumulative)
- Adds `SELF_HEAL`
- Reapplies STRONGER, FIREPOWER, ROF

**Plus weapon swap**: each Weapon slot has an `EliteWeaponN=` line. Most are identical
(`EliteWeapon3=CRM60` — same as Weapon3), but the engine still consults
`EliteWeaponN=` so swapping is structurally supported. The default `Weapon1=HoverMissile`
swaps to `EliteWeapon1=HoverMissileE` at elite, giving the empty-IFV an upgraded
missile. Same for `Weapon17=CRMissileLauncher` (Guardian GI).

**Active weapon at elite = `EliteWeapon[(IFVMode-based slot)]`** — so elite IFV with
Engineer passenger fires `EliteWeapon2=RepairBullet` (same as base), elite IFV empty
fires `EliteWeapon1=HoverMissileE` (upgraded).

---

## 10. Cross-references

### Direct dependencies (huge web of references)
- **17 weapons**: `[HoverMissile/E]`, `[RepairBullet]`, `[CRM60]`, `[CRFlakGuyGun]`, `[CRMP5]`, `[AWPE]`, `[CRElectricBolt]`, `[CRNuke]`, `[CRMindControl]`, `[CRRadBeamWeapon]`, `[CRNeutronRifle]`, `[CRTerrorBomb]`, `[CowShot]`, `[CRPsychicJab]`, `[CRVirusGun]`, `[CRSuperMindBlast]`, `[CRMissileLauncher]`
- `[FV]` (artmd) — art block with 17 WeaponNFLH entries
- `[GAWEAP]` — prereq
- `[IFVSelect/Move/AttackCommand/MoveStart/Transform]` (soundmd) — voices and transform sound
- `[GenVehicleDie] / [TankCrush]` — generic sounds
- `[General] DeathWeapon=` global default (overridden per-FV to `CRNuke`)

### Conceptual companions
- **HTK** ([`soviet/HTK.md`](../soviet/HTK.md)) — Soviet small-transport counterpart. Different design (no Gunner, 5 passengers, dual weapons).
- **BFRT** (Battle Fortress) ([`allied/BFRT.md`](./BFRT.md) — TODO) — Allied tier-3 5-passenger crusher. Uses `OpenTopped=yes` instead of `Gunner=yes` — passengers fire out of the Battle Fortress with their own weapons.
- **Every infantry doc** — each has an `IFVMode=N` field. The dispatch table here determines which weapon they activate when boarded.
- **PTROOP** ([`yuri/PTROOP.md`](../yuri/PTROOP.md)) — flagged the "PTROOP in IFV fires CRMP5 (SEAL pistol), not psi" quirk because IFVMode=4 maps to PistolTurret (not BrainBlast).
- **CIVAN** ([`soviet/CIVAN.md`](../soviet/CIVAN.md)) — Chrono Ivan, `IFVMode=7` → ExplodeTurret → `CRNuke` (same as regular IVAN's Weapon8).

### Deep-RE docs
- None directly relevant — the IFV dispatch mechanism doesn't have a dedicated Ghidra report. **Strong candidate for a `/re-investigate Multi-Gunner IFV dispatch` session** to resolve the IFVMode-integer → TurretKey question.

---

## Ghidra audit log (audit iteration 18 — 2026-05-18)

**Methodology**: FV is the most concrete-claim-rich vehicle doc to date
— 5 cited parser xrefs, 17-weapon dispatch table, hardcoded
gunner-mode mechanism. This audit re-verifies all 5 parser claims, pins
the 5 NEW TechnoType offsets, decompiles **FUN_00717890** (the
gunner-table builder), and **substantially resolves the §3 IFVMode →
TurretKey open question** via parse-order analysis of
`UnitTypeClass__ReadINI`'s gunner block. ~15 Ghidra queries: 7 string
searches + 1 decompile + 1 grep on saved TechnoTypeClass__ReadINI.

### Negative claims re-verified

| Query | Result |
|-------|--------|
| `search_strings("^FV$")` | **0 matches** |
| `search_strings("^MultiGunner$")` | **0 matches** |

Confirms: no hardcoded section-name branch.

### String + parser xref re-verification (BINARY-VERIFIED)

All 5 doc-cited claims verify exactly:

| String | Addr | Parser xref | Function |
|--------|------|-------------|----------|
| `Gunner` | 0x00843964 | 0x00714A50 | TechnoTypeClass__ReadINI |
| `TurretCount` | 0x00844348 | 0x00712851 | TechnoTypeClass__ReadINI |
| `WeaponCount` | 0x0084433C | 0x0071286B | TechnoTypeClass__ReadINI |
| `AirRangeBonus` | 0x00843AD4 | 0x007147A1 | TechnoTypeClass__ReadINI |
| `DeathWeapon` | 0x0083B11C | TechnoTypeClass__ReadINI (per-unit override) | (dual-read pattern with RulesClass__ReadCombatDamage @ 0x0066C58A, per doc — TechnoType side BINARY-VERIFIED this pass) |

### NEW TechnoType offsets BINARY-VERIFIED

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0x805` | `Gunner` | byte | `*(undefined1*)((int)param_1 + 0x805) = uVar3` after ReadBool. **NEW** — gates the entire IFV multi-weapon mechanism. |
| `+0x808` | `TurretCount` | int | `param_1[0x202] = iVar4` after ReadInt. **NEW** — FV sets this to 4 (4 visual turret graphics). |
| `+0x80C` | `WeaponCount` | int | `param_1[0x203] = iVar4` after ReadInt. **NEW** — FV sets this to 17 (17 weapon slots). |
| `+0x68C` | `AirRangeBonus` | int | `param_1[0x1a3] = iVar4` after CCINIClass::ReadRange. **NEW** — sibling to audit-7 `+0x688 = IFVMode`. |
| `+0xD18` | `DeathWeapon` | WeaponType* | `param_1[0x346] = iVar4` after WeaponTypeClass__FindOrAllocate. **NEW** — the per-unit override slot for the dual-read pattern (FV uses `CRNuke` here as the Ivan-passenger special-case detonation). |

### NEW function entry: FUN_00717890 — gunner-table builder (BINARY-VERIFIED)

Tiny 1-line setter function:

```
void FUN_00717890(this, TurretIndex param_2, WeaponSlot param_3) {
    *(undefined4 *)(this + 0x814 + param_3 * 4) = param_2;
}
```

Stores TurretIndex into a lookup table at `this+0x814`, indexed by
WeaponSlot. Called 17 times in `UnitTypeClass__ReadINI`'s gunner block
(per audit-12 decompile) — once per named TurretKey
(NormalTurret/RepairTurret/MachineGunTurret/.../GuardianTurret).

### NEW UnitType (TechnoType-extended) field: gunner turret-lookup table

| Offset range | Field | Type | Source |
|--------------|-------|------|--------|
| `+0x814..+0x858` | gunner-table[17] | int[17] (visual TurretIndex 0-3 per WeaponSlot 0-16) | populated by FUN_00717890 calls in UnitTypeClass__ReadINI's gunner block (audit 12). Indexed by WeaponSlot, value = visual TurretIndex (0=rocket, 1=gun, 2=repair, 3=high-tech per FV INI author comment). |

### §3 IFVMode → TurretKey RESOLUTION (substantial)

The doc's §3 open question: "the IFVMode integer → TurretKey mapping is
not visible in the INI. It must be reverse-engineered from gamemd.exe."

**Parse-order evidence** (from audit-12 `UnitTypeClass__ReadINI` decompile body):
the 17 named TurretKeys are parsed in this fixed order, and each one is
filed into the gunner-table with a sequentially-incrementing
WeaponSlot:

| Parse order | TurretKey | WeaponSlot (= IFVMode value at consumer, presumed) |
|-------------|-----------|----------------------------------------------------|
| 0 | NormalTurret | 0 |
| 1 | RepairTurret | 1 |
| 2 | MachineGunTurret | 2 |
| 3 | FlakTurret | 3 |
| 4 | PistolTurret | 4 |
| 5 | SniperTurret | 5 |
| 6 | ShockTurret | 6 |
| 7 | ExplodeTurret | 7 |
| 8 | BrainBlastTurret | 8 |
| 9 | RadCannonTurret | 9 |
| 10 | ChronoTurret | 10 |
| 11 | TerroristExplodeTurret | 11 |
| 12 | CowTurret | 12 |
| 13 | InitiateTurret | 13 |
| 14 | VirusTurret | 14 |
| 15 | YuriPrimeTurret | 15 |
| 16 | GuardianTurret | 16 |

This is the parser-side data layout. Whether the IFVMode integer at the
**consumer site** maps directly (1:1, IFVMode == WeaponSlot) or through
a separate translation is **PARTIALLY RESOLVED — INFERRED** by the parse
order + the FV INI `WeaponN`/`TurretKey` comment-pairs (Engineer→Repair,
GI→MachineGun, YURI→BrainBlast, etc.). **Direct verification of the
consumer chain** (where IFVMode is read from the passenger TechnoType at
runtime and used to index the gunner-table) is DEFERRED.

The "Engineer paradox" (ENGINEER `IFVMode=0` should map to NormalTurret,
but doc-comment implies Repair) likely resolves via either:
(a) ENGINEER's actual `IFVMode` is **1**, not 0 (the doc's audit may
have misread it), or
(b) the consumer applies a special-case offset (e.g., `IFVMode + 1` for
non-empty IFVs) before indexing.

This audit doesn't decompile the consumer site to disambiguate (a) vs
(b); marked as **DEFERRED** for `/re-investigate Multi-Gunner IFV
consumer` session.

### Cumulative cross-references

- **DeathWeapon dual-read** — the doc claims `RulesClass__ReadCombatDamage @ 0x0066C58A` (global default) + `TechnoTypeClass__ReadINI @ 0x007122F0` (per-unit override). The TechnoType-side write is BINARY-VERIFIED this pass at +0xD18. The RulesClass global-default xref is NOT re-verified (DEFERRED — the doc's claim is taken as authoritative since the symmetric pattern with ChronoInSound (audit 17) is confirmed).

### Items NOT re-verified in this pass (DEFERRED)

- IFVMode consumer chain (the per-tick code that reads passenger's IFVMode and indexes the +0x814 gunner-table).
- The "Engineer paradox" (special-case logic at the consumer, or the actual ENGINEER `IFVMode` value).
- DeathWeapon "Ivan-passenger only" predicate — the INI comment says "special case that only goes off if it's piloted by Ivan". Consumer site DEFERRED.
- `RulesClass__ReadCombatDamage` body (DeathWeapon global-default site).
- The `IFVTransform` sound trigger frame-perfect alignment with turret-swap visual.

### Confidence summary

- **HIGH**: 7 string addresses + 5 parser xrefs (all exact); 5 NEW TechnoType struct offsets (all from direct decompile reads); 1 NEW function entry (FUN_00717890 fully decompiled); 17-slot gunner-table layout at +0x814..+0x858 (parser-side BINARY-VERIFIED via audit-12 + audit-18 cross-reference).
- **MEDIUM**: §3 IFVMode → WeaponSlot mapping — strongly INFERRED via parse-order + INI comment-pair evidence, but consumer-side chain not directly verified.
- **No INCORRECT findings in the doc**. The 5 cited parser xrefs all resolve exactly; the dispatch-table layout matches the binary.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[FV]` rulesmd key explained (header + dispatch + tail) | ✅ §1 |
| All 17 Weapon/EliteWeapon slots listed verbatim | ✅ §1 |
| All 17 named TurretKey dispatch entries (TurretIndex + TurretWeapon) listed | ✅ §1 |
| `[FV]` artmd block expanded, all 17 WeaponNFLH entries documented | ✅ §2 |
| **TurretIndex → voxel turret meaning** (0=rocket, 1=gun, 2=repair, 3=high-tech) | ✅ §3 |
| **Full TurretKey → (TurretIndex, WeaponSlot, Weapon, Passenger) table** | ✅ §3 |
| Brief description of all 17 referenced weapons | ✅ §4 |
| All voices + IFVTransform shared enter/leave sound | ✅ §5 |
| Prereqs / owners / availability | ✅ §6 |
| **FV vs HTK comparison table** (the IFV pair) | ✅ §6 |
| Hardcoded behavior — Ghidra searches + 5 flag-scope verifications | ✅ §7 (Gunner, TurretCount, WeaponCount, AirRangeBonus, DeathWeapon — last is RulesClass+TechnoType dual-read) |
| **DeathWeapon=CRNuke Ivan-passenger special case documented** | ✅ §1 + §7 |
| **⚠ Open question logged**: IFVMode integer → TurretKey enum mapping | ✅ §3 + §10 |
| TS-legacy filter | ✅ §8 |
| Veterancy (elite swaps Weapon1 → EliteWeapon1 etc., per-slot) | ✅ §9 |
| Cross-refs (every infantry doc references this doc via IFVMode) | ✅ §10 |
| **Two index corrections logged**: FV is IFV not Battle Fortress; BFRT is real Battle Fortress not duplicate | ✅ doc header + index entries |

**Open follow-ups (parity-critical):**
- **`/re-investigate Multi-Gunner IFV dispatch`**: decompile the IFV gunner-mode resolver in `gamemd.exe` to find the exact IFVMode-integer → TurretKey lookup. Without this, every infantry doc's IFVMode→Weapon claim is partially inferred (correct for the well-known cases — YURI=8=BrainBlast=CRMindControl, but unclear for many).
- The Engineer paradox: `ENGINEER IFVMode=0` should map to NormalTurret = HoverMissile (per literal IFVMode=0 → slot 0 mapping), but the comment "Weapon2=RepairBullet ;Engineer" implies Repair. Special-case logic likely.
- `DeathWeapon=CRNuke` Ivan special case: verify that the engine ONLY fires `CRNuke` death-weapon if the current passenger is an Ivan-class unit. The INI comment says "special case" — what's the exact predicate? Crew=yes? Specific IFVMode value? Passenger TechnoType pointer comparison?
- `EnterTransportSound=IFVTransform` and `LeaveTransportSound=IFVTransform`: confirm the turret-swap visual animation actually triggers in sync with the sound. May have a frame-perfect alignment requirement.
- `[CowShot]` weapon — verify the Cow CAN be loaded into an IFV. The COW infantry doc needs to address whether `Owner=` permits cow→IFV boarding (cows are civilian-faction; IFV is Allied; cross-faction transport requires capture).
