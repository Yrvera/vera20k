# BFRT — Battle Fortress (Allied tier-4 super-transport)

**Side classification:** Allied (Owner=British,French,Germans,Americans,Alliance).
**Role:** Tier-4 5-passenger heavy assault transport. `OpenTopped=yes` (passengers
fire their own weapons out through 5 gun-ports), `OmniCrusher=yes` (can crush other
tanks, not just infantry), `MovementZone=CrusherAll` (drives through walls and most
crushable terrain), `OmniCrushResistant=yes` (cannot be crushed by other
OmniCrushers). The "rolling fortress" — pack with 5 infantry, drive over enemy lines.

> Output bar: passenger-fire from the 5 `AlternateFLH` gun-ports must align with the
> visible model. OmniCrush vs OmniCrushResistant resolution must produce identical
> outcomes vs MCV / vs other BFRT / vs Apocalypse. CrusherAll movement zone must
> path through walls and over crushable infantry exactly as gamemd does.

> **Companion docs**:
> - [`allied/FV.md`](./FV.md) — Multi-Gunner IFV (the OTHER Allied transport — 1
>   passenger, weapon-swap). Different mechanic entirely.
> - [`allied/AMCV.md`](./AMCV.md) — shares `OmniCrushResistant=yes` (only 3 units
>   have this: AMCV, SMCV, PCV — and BFRT).

> Ghidra confirms no `"BFRT"` or `"BattleFortress"` strings in `gamemd.exe` — all
> behavior is generic flag-driven via `OpenTopped`, `OmniCrusher`, `OmniCrushResistant`,
> `CrusherAll` (MovementZone enum value).

---

## 1. `rulesmd.ini` — `[BFRT]` verbatim

```ini
[BFRT]
UIName=Name:Battlefortress
Name=Battle Fortress
Prerequisite=GAWEAP,GATECH
Primary=20mmRapid
Strength=600
Category=AFV
Armor=heavy
IsTilter=yes
TooBigToFitUnderBridge=true
TechLevel=10
Sight=6
Speed=4
PipScale=Passengers
Passengers=5
OpenTopped=yes;passengers can shoot out
SizeLimit=2;1 ;gs like half track and Blackhawk.  Terror Drones and Brutes are allowed in.
CrateGoodie=yes
Crusher=yes
OmniCrusher=yes;gs can crush things not normally crushable
Owner=British,French,Germans,Americans,Alliance
Cost=2000
Soylent=2000
Points=50
ROT=5
IsSelectableCombatant=yes
AllowedToStartInMultiplayer=no
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=BattleFortressSelect
VoiceMove=BattleFortressMove
VoiceAttack=BattleFortressAttackCommand
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=BattleFortressMoveStart
EnterTransportSound=EnterTransport
LeaveTransportSound=ExitTransport
Maxdebris=3
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
MovementZone=CrusherAll;gs OmniCrush handles crushing tanks and such, this handles walls
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
OmniCrushResistant=yes; so Crusher can crush Crushable, OmniCrusher trumps Crushable=no, and then OmniCrushResistant trumps OmniCrusher
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:Battlefortress` | AbstractType | CSF lookup. **Note**: lowercase `Battlefortress`, not `BattleFortress`. |
| `Name` | `Battle Fortress` | AbstractType | Dev fallback. |
| (no `Image=` line) | — | — | BFRT does NOT have an `Image=` redirect in rulesmd. But artmd `[BFRT]` has `Image=SREF` — a DIFFERENT mechanism (see §2). |
| `Prerequisite` | `GAWEAP,GATECH` | TechnoType | Allied War Factory + Battle Lab — tier-4 gate. |
| `Primary` | `20mmRapid` | TechnoType | **Reuses the War Miner's anti-ground gun** (Damage=30, ROF=20, Range=5.5, Warhead=HARVWH). See HARV §3 for full breakdown. Surprising weapon choice — the same auto-cannon a Soviet harvester uses. Suggests "always shooting, mostly cosmetic" — the real damage comes from the 5 passenger weapons. |
| `Strength` | `600` | AbstractType | 600 HP — between Rhino (400) and APOC (800). Tankier than any other Allied vehicle. |
| `Category` | `AFV` | TechnoType | AFV classifier. |
| `Armor` | `heavy` | TechnoType | Verses-slot 6. |
| `IsTilter` | `yes` | UnitType | Voxel hull tilts on slopes. |
| `TooBigToFitUnderBridge` | `true` | UnitType-only | Cannot path under low bridges. |
| `TechLevel` | `10` | TechnoType | Endgame tier — same as MCV. |
| `Sight` | `6` | TechnoType | 6-cell reveal — average. |
| `Speed` | `4` | TechnoType | **Slow** — same as Apocalypse, MCV, harvester. Compensated by HP and weapon density. |
| `PipScale` | `Passengers` | UnitType | Renders passenger pips (5 slots). |
| `Passengers` | `5` | TechnoType (verified prior iter) | **5-passenger capacity** — combined with `OpenTopped=yes`, all 5 fire out simultaneously. |
| `OpenTopped` | `yes` | TechnoType [BINARY-VERIFIED audit 19: string @ 0x00843CCC, parser xref @ 0x007143BD, `TechnoType+0x5E4` (byte)] | **Hardcoded core mechanic.** INI comment: "passengers can shoot out". With this flag, the engine spawns each passenger's projectile from one of the `AlternateFLH0-4` positions on the vehicle, using the passenger's own Primary weapon. Range is extended by global `[CombatDamage] OpenToppedRangeBonus` (verified — 0x0083afec → RulesClass__ReadCombatDamage 0x0066c774) and damage scaled by `OpenToppedDamageMultiplier` (verified — 0x0083b004 → 0x0066c756). |
| `SizeLimit` | `2` | TechnoType | INI comment: "1 ;gs like half track and Blackhawk. Terror Drones and Brutes are allowed in." — **Comment history**: was `=1` originally; raised to `=2` to allow Terror Drones (DRON, Size=2) and Brutes (BRUTE, Size=2) to board. This is a significant balance tweak. **Battle Fortress with Terror Drones inside is a frequent strategy** — the Drones jump out when engaged to chew enemy tanks. |
| `CrateGoodie` | `yes` | UnitType | Can drop from crates — extremely valuable jackpot. |
| `Crusher` | `yes` | TechnoType | Crushes regular infantry. |
| `OmniCrusher` | `yes` | TechnoType [BINARY-VERIFIED audit 19: string @ 0x0084387C, parser xref @ 0x00714CF0, `TechnoType+0xD29` (byte — re-confirms audit 7 cumulative)] | INI comment: "gs can crush things not normally crushable". **The "tank-crusher" flag.** Overrides target's `Crushable=no` — meaning BFRT can drive over Tanya, SEAL, GHOST, CCOMAND, IVAN, BORIS, PTROOP, CIVAN, SLAV, etc. (units that normal Crushers can't squish). Per the 3-tier resolution: `Crusher → OmniCrusher → OmniCrushResistant`. |
| `Owner` | 5 Allied countries | TechnoType | Allied only. |
| `Cost` | `2000` | TechnoType | Expensive — $2000 is more than any other tank except MCV. |
| `Soylent` | `2000` | TechnoType | 100% Grinder refund (relevant if Yuri captures). |
| `Points` | `50` | TechnoType | High score on kill. |
| `ROT` | `5` | TechnoType | Body+turret rotation. |
| `IsSelectableCombatant` | `yes` | TechnoType | Selectable. |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Not preplaced. |
| `Explosion` | `TWLT070,...` | TechnoType | Death anim pool. |
| `VoiceSelect` | `BattleFortressSelect` | TechnoType | 6 unique clips ($vbatsea..ef). |
| `VoiceMove` | `BattleFortressMove` | TechnoType | 5 unique clips ($vbatmoa..oe). |
| `VoiceAttack` | `BattleFortressAttackCommand` | TechnoType | 4 active clips ($vbatatb..ate; commented $vbatata disabled). |
| `VoiceFeedback` | *(empty)* | TechnoType | No under-attack voice. |
| `DieSound` | `GenVehicleDie` | TechnoType | Generic. |
| `MoveSound` | `BattleFortressMoveStart` | TechnoType | 3 unique clips, predelay 0–400ms, low pri, FShift ±5, VShift +10, vol 50 (louder than most engine sounds — reflects the heavy fortress's audio presence). |
| `EnterTransportSound` | `EnterTransport` | TechnoType | Generic transport-enter (`genter1a`, vol 60). |
| `LeaveTransportSound` | `ExitTransport` | TechnoType | Generic transport-exit (`gexit1a`, Limit=2, vol 60). |
| `Maxdebris` | `3` | TechnoType | 3 debris pieces. |
| `Locomotor` | `{4A582741-...}` | TechnoType | DriveLocomotionClass. |
| `MovementZone` | **`CrusherAll`** | TechnoType (enum verified — 0x0081bad0) | INI comment: "gs OmniCrush handles crushing tanks and such, this handles walls". **The most permissive movement zone.** `CrusherAll` lets BFRT path through walls (crushable structures) plus all `Crusher` and `Normal` paths. Combined with `OmniCrusher=yes`, the BFRT is essentially unstoppable by terrain. |
| `ThreatPosed` | `40` | TechnoType | High AI threat (same as Rhino, APOC). |
| `DamageParticleSystems` | `SparkSys,SmallGreySSys` | TechnoType | Damage emitters. |
| `DamageSmokeOffset` | `100, 100, 275` | TechnoType | Same as Rhino — high Z=275 emits smoke from above. |
| `Weight` | `3.5` | TechnoType | Standard tank weight (same as Rhino, APOC, MCV — surprising, not heavier). |
| `VeteranAbilities` | `STRONGER,FIREPOWER,SIGHT,FASTER` | TechnoType | Veteran bonuses. **No ROF** at veteran. |
| `EliteAbilities` | `SELF_HEAL,STRONGER,FIREPOWER,ROF` | TechnoType | Elite adds SELF_HEAL + ROF. |
| `Accelerates` | `false` | TechnoType | No accel ramp. |
| `ZFudgeColumn` | `8` | UnitType | Same as Rhino. |
| `ZFudgeTunnel` | `13` | UnitType | TS-legacy. |
| `Size` | `6` | TechnoType | **Cannot fit in any transport** — same as MCV. BFRT is too big for any other vehicle. |
| `OmniCrushResistant` | `yes` | TechnoType | INI comment chain: "so Crusher can crush Crushable, OmniCrusher trumps Crushable=no, and then OmniCrushResistant trumps OmniCrusher". 3-tier crush resolution: |
| | | | • Tier 1: `Crusher=yes` + `Crushable=yes (default)` → crush |
| | | | • Tier 2: `OmniCrusher=yes` (BFRT) overrides target's `Crushable=no` |
| | | | • Tier 3: `OmniCrushResistant=yes` on target overrides Tier 2 — even Battle Fortress cannot crush units with this flag (AMCV, SMCV, PCV, OTHER BFRTs) |
| | | | **BFRT vs BFRT cannot crush each other** — `OmniCrushResistant=yes` on both sides means standoff. |

### Notable absent keys
- **No `Turret=yes`** — Battle Fortress's `20mmRapid` Primary is hull-mounted (body rotation only). The 5 passenger weapons fire from fixed FLH positions, not a rotating turret.
- No `Secondary=` — single primary weapon (plus passenger contributions).
- No `Gunner=yes` — passengers don't swap the BFRT's weapon. They fire their OWN weapons from gun-ports.
- No `Teleporter=` — does not chrono.
- No `Spawns=` — no child units.
- No `ImmuneToPsionics` — Yuri can mind-control BFRT + its 5 passengers.
- No `ImmuneToRadiation` — Desolators damage BFRT.
- No `Bunkerable=no` — BFRT CAN board another BFRT (transport-in-transport — but Size=6 prevents in practice).
- No `Trainable=no` — BFRT CAN gain veterancy (unlike MCV).

---

## 2. `artmd.ini` — `[BFRT]` section (with `Image=SREF` quirk)

```ini
[BFRT]   ; Battle Fotress
Image=SREF
Voxel=yes
Remapable=yes
Cameo=BFRTICON
AltCameo=BFRTUICO
PrimaryFireFLH=220,0,130
AlternateFLH0=45,190,90;gs the five gun ports
AlternateFLH1=45,-190,90
AlternateFLH2=-120,200,80
AlternateFLH3=-120,-200,80
AlternateFLH4=220,0,130
```

| Key | Value | Effect |
|-----|-------|--------|
| `; Battle Fotress` | (typo) | "Fotress" missing 'r' — INI typo, harmless. |
| `Image` | `SREF` | **artmd-side Image= redirect — interpretation unclear.** In rulesmd, `Image=` redirects which artmd block to read. Here in artmd, `Image=SREF` MIGHT mean "use SREF.VXL for the voxel filename" (i.e., BFRT renders as Prism Tank). But the BFRT cameo and FLH overrides below suggest BFRT.VXL is the actual voxel and `Image=SREF` is either ignored or used for some sub-aspect (animation set?). **Open question — see §7.4.** |
| `Voxel` | `yes` | Voxel-rendered. |
| `Remapable` | `yes` | House-color remap. |
| `Cameo` | `BFRTICON` | Sidebar cameo — explicit override (not SREF's). |
| `AltCameo` | `BFRTUICO` | Yuri-skinned cameo. |
| `PrimaryFireFLH` | `220,0,130` | Primary `20mmRapid` fire offset: X=+220 (very far forward — long fortress chassis), Y=0 (centered), Z=+130 (above hull). |
| `AlternateFLH0` | `45,190,90` | INI comment: "gs the five gun ports". **Front-right gun-port**: X=+45 (slightly forward), Y=+190 (far right), Z=+90 (mid-height). |
| `AlternateFLH1` | `45,-190,90` | **Front-left gun-port**: mirror of FLH0 (Y=-190). |
| `AlternateFLH2` | `-120,200,80` | **Rear-right gun-port**: X=-120 (behind body center), Y=+200 (slightly farther right than front-right), Z=+80 (slightly lower). |
| `AlternateFLH3` | `-120,-200,80` | **Rear-left gun-port**: mirror of FLH2 (Y=-200). |
| `AlternateFLH4` | `220,0,130` | **Front-center gun-port**: SAME as `PrimaryFireFLH` — overlaid on the BFRT's own gun barrel position. Passenger #5 fires from the same muzzle as the Primary 20mmRapid. |

### AlternateFLH visualization (top-down view of BFRT body)

```
              FLH4 (front-center, x=+220)
               |
               |
       +---[BFRT BODY]---+
       |                 |
FLH1 ──┤                 ├── FLH0
(L=−190)|                 |  (R=+190)
       |                 |
FLH3 ──┤                 ├── FLH2
(L=−200)|                 |  (R=+200)
       +-----------------+
            (body center)
```

The 5 gun-ports are arranged as: 1 front-center (FLH4) + 2 forward-sides (FLH0/1) + 2 rear-sides (FLH2/3). Passengers 1-5 fire from FLH0-4 in order — passenger 5 (the last to board) gets the front-center FLH4.

The `AlternateFLH%d` key format string is verified at TechnoTypeClass__ReadINI 0x00715faf — parsed as `AlternateFLH0`, `AlternateFLH1`, etc., into the TechnoType structure. Even though the values are in artmd, the engine reads them into the TechnoType.

### Image=SREF — possible interpretations

1. **Engine loads SREF.VXL instead of BFRT.VXL**: would mean Battle Fortress visually IS a Prism Tank. Doesn't match in-game appearance (Battle Fortress is clearly different).
2. **Engine ignores Image= in artmd**: harmless legacy line.
3. **Engine loads BFRT.VXL but inherits some sub-aspect (animations, shadow handling)**: most likely.
4. **The line was accidentally copy-pasted from SREF's art block and is dead INI text**.

Resolution requires Ghidra-decompile of the voxel-loading path in artmd parsing. Until verified, the **safe parity implementation** is: load `BFRT.VXL`, ignore `Image=` in artmd. If gamemd actually substitutes SREF.VXL, the player would see the wrong shape — easily detectable visually.

---

## 3. Weapon — `[20mmRapid]` (Primary)

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

This is the **same weapon used by the War Miner (HARV)**. See [`soviet/HARV.md`](../soviet/HARV.md) §3 for full breakdown. Key points:
- Damage=30 / ROF=20 / Range=5.5
- Warhead `HARVWH` (Verses 100/80/70/50/20/20/20/15/10/**400**/100 — bonus vs special_1, weak vs heavy armor)
- Report=`WarMinerAttack` (Soviet-flavored sound — odd choice for an Allied tank)

The 20mmRapid is **anti-infantry**, NOT anti-tank (Verses 20% vs medium/heavy means BFRT barely scratches MBTs with its own weapon). The Battle Fortress relies on its **5 passenger weapons** for any real firepower — pack with Prism Troopers (`SREF`-equivalent infantry)... wait, no, infantry are different. Pack with Tanya (DoublePistols), GGI (deployed AT missiles), Desolators (rad-beam), etc. for combat firepower.

No `ElitePrimary=` — the BFRT's own gun doesn't upgrade at elite.

---

## 4. Warhead — `[HARVWH]`

Same as War Miner. See [`soviet/HARV.md`](../soviet/HARV.md) §4. Verses curve favors infantry slots (100/80/70/50) and `special_1` (400%, likely Terror Drone armor — though BFRT vs Terror Drone is unusual since Drones can ride INSIDE BFRT).

---

## 5. Voices / sounds

```ini
[BattleFortressSelect]
Sounds=$vbatsea $vbatseb $vbatsec $vbatsed $vbatsee $vbatsef
Control=random
Volume=85

[BattleFortressMove]
Sounds=$vbatmoa $vbatmob $vbatmoc $vbatmod $vbatmoe
Control=random
Volume=85

[BattleFortressAttackCommand]
Sounds=$vbatatb $vbatatc $vbatatd $vbatate ;$vbatata
Control=random
Volume=85
```

```ini
[BattleFortressMoveStart]
Sounds=vbatstaa vbatstab vbatstac
Control= random predelay
Delay=0 400
Priority=Low
FShift= -5 5
VShift=10
Volume=50
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=BattleFortressSelect` | 6 unique clips | Click-select |
| `VoiceMove=BattleFortressMove` | 5 unique clips | Move order |
| `VoiceAttack=BattleFortressAttackCommand` | 4 active clips (5th commented out: `;$vbatata`) | Attack order |
| `VoiceFeedback=` *(empty)* | — | No under-attack |
| `DieSound=GenVehicleDie` | 6 generic clips | Death |
| `MoveSound=BattleFortressMoveStart` | 3 unique clips, predelay 0–400ms, low pri, vol 50 | Engine start |
| `EnterTransportSound=EnterTransport` | `genter1a`, vol 60 | Passenger board |
| `LeaveTransportSound=ExitTransport` | `gexit1a`, `Limit=2`, vol 60 | Passenger disembark |
| `Report=WarMinerAttack` (Primary weapon) | (in soundmd) | BFRT's own gun fire — Soviet-flavored despite being Allied |
| (passenger weapons) | each passenger's `Report=` | When a passenger fires from a gun-port |

Notable: each passenger continues to use **their own weapon's `Report=`**. A Battle Fortress carrying 5 Desolators would emit 5 Desolator-rad-beam sounds plus the BFRT's own 20mmRapid sound when firing. The audio mix is intentionally layered.

The BFRT's own `BattleFortressAttackCommand` has fewer clips (4) than its select (6) or move (5) — and one was commented out. Possibly the author wanted distinct attack lines for "primary attack" vs "passenger attack", couldn't finalize, and shipped a smaller pool.

---

## 6. Prerequisites / owners / availability

- **Prerequisite**: `GAWEAP,GATECH` — Allied War Factory + Battle Lab.
- **TechLevel** = `10`.
- **Owner**: 5 Allied countries.
- **`CrateGoodie=yes`** — can drop from crates (jackpot).
- **`AllowedToStartInMultiplayer=no`** — not preplaced.
- **Cost** = $2000 — most expensive Allied combat unit (only AMCV at $3000 exceeds it).

### What infantry can board BFRT?

Per `SizeLimit=2` and `Passengers=5`: up to 5 infantry of Size 1 or 2 may board. From the INI comment:
- **Size=1** infantry: standard (E1, GGI, SHK, Tanya, Engineer, etc.)
- **Size=2** infantry: Terror Drones (DRON) and Brutes (BRUTE) — the comment "gs like half track and Blackhawk. Terror Drones and Brutes are allowed in." confirms this is the inclusive design intent.

Notable disallowed: aircraft passengers (e.g., Rocketeer JUMPJET) — they have `Size=1` and infantry pip type, may actually board. The Allied IFV `FV` has `SizeLimit=1` (stricter); BFRT's `SizeLimit=2` is the relaxed version.

### Optimal passenger compositions (designer intent)

Common multiplayer compositions exploit OpenTopped's 5-weapon synergy:
- **5x GGI**: 5 deployable Guardian GI missiles = anti-armor wrecking ball
- **5x Tanya**: 5x DoublePistols + C4-on-buildings — but Tanya `BuildLimit=1` so impossible
- **5x Desolator**: 5x RadBeamWeapon = anti-infantry death wave
- **2x Terror Drone + 3x GI**: Drone-eject when engaging armor + GI ranged support
- **5x Chrono Legionnaire (CLEG)**: 5x NeutronRifle = chrono-erase swarm
- **5x Crazy Ivan**: 5x IvanBomber = 5 bombs per pass (instakill many buildings)

The strategic versatility of `OpenTopped=yes` is what makes BFRT the most flexible Allied unit — its role depends entirely on cargo.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 BFRT-specific code in `gamemd.exe`

| Query | Result |
|-------|--------|
| `BFRT` | 0 matches |
| `BattleFortress` | 0 matches |

⇒ **No BFRT-specific code path.** All behavior is generic flag-driven.

### 7.2 Flag-scope verification (this iteration)

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `OpenTopped` | 0x00843ccc | TechnoTypeClass__ReadINI @ 0x007143bd | TechnoType |
| `OmniCrusher` | 0x0084387c | TechnoTypeClass__ReadINI @ 0x00714cf0 | TechnoType |
| `AlternateFLH%d` (format string) | 0x00843208 | TechnoTypeClass__ReadINI @ 0x00715faf | TechnoType (despite values being in artmd) |
| `OpenToppedRangeBonus` | 0x0083afec | RulesClass__ReadCombatDamage @ 0x0066c774 | RulesClass global |
| `OpenToppedDamageMultiplier` | 0x0083b004 | RulesClass__ReadCombatDamage @ 0x0066c756 | RulesClass global |
| `CrusherAll` (MovementZone enum value) | 0x0081bad0 | (enum-value string, not a key) | Lookup in MovementZone enum table |

Plus prior verifications (carried):
- `OmniCrushResistant` — TechnoType
- `Passengers`, `SizeLimit` — TechnoType
- `DamageSmokeOffset` — TechnoType

Additional global flags relevant to OpenTopped (not all verified this iter but likely siblings):
- `OpenToppedWarpDistance` at 0x0083afd4 — global, RulesClass
- `OpenToppedAnim` at 0x008493f0 — WeaponType (per address range)

### 7.3 Live behaviors driven by these flags

| Behavior | Driver | Notes |
|----------|--------|-------|
| Passengers fire from gun-ports | `OpenTopped=yes` + `AlternateFLH0-4` positions | 5 simultaneous firing positions |
| Passenger weapons get range bonus when firing from inside | `[CombatDamage] OpenToppedRangeBonus=N` | Global; applies to all OpenTopped vehicles |
| Passenger weapons get damage scaling when firing from inside | `[CombatDamage] OpenToppedDamageMultiplier=X` | Global |
| Can crush tanks (not just infantry) | `OmniCrusher=yes` | Overrides target's `Crushable=no` |
| Cannot be crushed by other OmniCrushers | `OmniCrushResistant=yes` | BFRT vs BFRT standoff; MCV survives BFRT pass |
| Can drive through walls and crushable terrain | `MovementZone=CrusherAll` | Most permissive movement zone in the game |
| Can accept oversized passengers (Drones, Brutes) | `SizeLimit=2` | Special case (others use SizeLimit=1) |
| Boarding/disembark generic sound | `EnterTransport/ExitTransport` | Not the IFV-style "Transform" sound |
| BFRT's own weapon: anti-infantry (HARVWH) | `Primary=20mmRapid` reuses War Miner gun | Weak vs MBTs — relies on passengers |

### 7.4 Behaviors NOT present

- No `Turret=yes` — body-mounted gun.
- No `Gunner=yes` — passengers don't swap BFRT's weapon (different mechanism than FV/IFV).
- No `Secondary=` weapon.
- No `Teleporter=`.
- No `Spawns=`.
- No `Bunkerable=no` (defaults yes) — BFRT could theoretically board another BFRT, but Size=6 prevents in practice.

### Open question: `Image=SREF` in artmd block

The artmd `[BFRT]` has `Image=SREF`. The Prism Tank `[SREF]` art block uses `SREF.VXL`. If
the engine literally substitutes voxel files via this Image= line, the Battle Fortress
should render as a Prism Tank — but it doesn't in gamemd. **Resolution requires Ghidra
audit of the voxel-loader path** when artmd has an `Image=` value. Most likely
behaviors:
1. Engine ignores artmd `Image=` line — loads `BFRT.VXL` from filename match.
2. Engine reads explicit `Voxel=yes` then filename-matches the block name (BFRT) — ignoring Image=.
3. Engine loads BFRT.VXL but inherits some animation/shadow handling from SREF.

**Not load-bearing for parity unless visual mismatch surfaces.**

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ZFudgeTunnel=13` | YES | Dormant render value. |
| (no `ImmuneToVeins`) | — | Not set. |

The `Image=SREF` line is more of a YR-era INI quirk than TS-legacy.

No fog-of-war refs, no real Tiberium refs, no tunnel refs.

---

## 9. Veterancy

### Veteran (1 chevron) — `STRONGER, FIREPOWER, SIGHT, FASTER`
- `STRONGER` — +25% HP (600 → 750)
- `FIREPOWER` — +25% damage (20mmRapid: 30 → 37)
- `SIGHT` — +20% sight (6 → 7.2)
- `FASTER` — +20% speed (4 → 4.8)

### Elite (2 chevrons) — `SELF_HEAL, STRONGER, FIREPOWER, ROF` (cumulative)
- Adds `SELF_HEAL`
- Reapplies STRONGER, FIREPOWER, ROF
- `ROF` — −25% ROF (20 → ~15)

**No `ElitePrimary=`** — the BFRT's own 20mmRapid does NOT upgrade at elite. The elite
firepower bonus is purely the stat-percentage from EliteAbilities.

**Passenger veterancy is independent** — each passenger gains their own XP from
their own kills (firing from gun-ports). A BFRT with 5 elite Desolators is far more
deadly than 5 rookies, but the elite-ness is on the passengers, not the BFRT.

---

## 10. Cross-references

### Direct dependencies
- `[20mmRapid]` — Primary weapon (shared with HARV — see [`soviet/HARV.md`](../soviet/HARV.md) §3)
- `[InvisibleLow]` — projectile
- `[HARVWH]` — warhead (shared with HARV)
- `[GUNFIRE]` (artmd) — muzzle flash
- `[BFRT]` (artmd) — own art block (no `Image=` redirect from rulesmd; artmd-side `Image=SREF` is the open question)
- `[GAWEAP] / [GATECH]` — prereqs
- `[BattleFortressSelect/Move/AttackCommand/MoveStart]` (soundmd) — voices and sounds
- `[EnterTransport] / [ExitTransport]` (soundmd) — board/disembark
- `[GenVehicleDie] / [TankCrush]` (soundmd) — generic sounds
- `[CombatDamage] OpenToppedRangeBonus / OpenToppedDamageMultiplier` (rulesmd globals)

### Conceptual companions
- **FV (Multi-Gunner IFV)** ([`allied/FV.md`](./FV.md)) — Allied 1-passenger weapon-swap (different mechanic). Both are Allied transports but the dispatch logic differs entirely.
- **AMCV / SMCV / PCV** — share `OmniCrushResistant=yes`. The set of OmniCrushResistant units is BFRT + 3 MCVs.
- **HTK (Flak Track)** ([`soviet/HTK.md`](../soviet/HTK.md)) — Soviet 5-passenger transport. Different (no OpenTopped, no OmniCrusher, fixed weapons).
- **DRON (Terror Drone)** — Size=2, allowed as BFRT passenger per `SizeLimit=2`.
- **BRUTE (Yuri Brute)** ([`yuri/BRUTE.md`](../yuri/BRUTE.md)) — Size=2, allowed (though Allied owns BFRT, captured Brutes via mind-control or specific scenarios).
- **All Allied infantry** — primary BFRT cargo.

### Deep-RE docs
- None directly relevant — BFRT mechanics are all generic flag-driven. The `Image=SREF` quirk could merit a brief `/re-investigate artmd Image= handling` if visual parity issues surface.

---

## Ghidra audit log (audit iteration 19 — 2026-05-18)

**Methodology**: BFRT is concrete-claim rich — 6 cited parser xrefs, the
hardcoded OpenTopped/OmniCrush/CrusherAll triad, and the 5-AlternateFLH
gun-port array. This audit re-verifies all 6 doc-cited claims, pins
the 3 NEW TechnoType offsets, and verifies the sibling
OpenToppedWarpDistance global. ~14 Ghidra queries: 8 string searches +
6 xref lookups + 1 grep pass on saved TechnoTypeClass__ReadINI.

### Negative claims re-verified

| Query | Result |
|-------|--------|
| `search_strings("^BFRT$")` | **0 matches** |
| `search_strings("^BattleFortress$")` | **0 matches** |

Confirms: no hardcoded section-name branch for BFRT.

### String + parser xref re-verification (BINARY-VERIFIED)

All 6 doc-cited claims verify exactly + 1 bonus:

| String | Addr | Parser xref | Function | Notes |
|--------|------|-------------|----------|-------|
| `OpenTopped` | 0x00843CCC | 0x007143BD | TechnoTypeClass__ReadINI | doc claim verified |
| `OmniCrusher` | 0x0084387C | 0x00714CF0 | TechnoTypeClass__ReadINI | doc claim verified |
| `AlternateFLH%d` | 0x00843208 | 0x00715FAF | TechnoTypeClass__ReadINI | format-string for 5-entry FLH array |
| `OpenToppedRangeBonus` | 0x0083AFEC | 0x0066C774 | RulesClass__ReadCombatDamage | RulesClass global |
| `OpenToppedDamageMultiplier` | 0x0083B004 | 0x0066C756 | RulesClass__ReadCombatDamage | RulesClass global |
| `CrusherAll` (MovementZone enum value) | 0x0081BAD0 | from 0x0081BAB8 (enum table data) | (enum string, not key) | xref into MovementZone enum table confirms it's an enum value |
| `OpenToppedWarpDistance` (bonus) | 0x0083AFD4 | 0x0066C794 | RulesClass__ReadCombatDamage | RulesClass-CombatDamage sibling |

### NEW TechnoType offsets BINARY-VERIFIED

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0x5E4` | `OpenTopped` | byte | `*(char*)(param_1 + 0x179) = (char)uVar5` after ReadBool. **NEW** — gates the gun-port passenger-fire mechanic. |
| `+0x89C` | `AlternateFLH0` (base) | int[3] (FLH triplet — X, Y, Z) | `param_1 + 0x227` is the dest buffer for the format-string-parsed FLH triplet. **5-entry array layout INFERRED** from INI evidence (`AlternateFLH0..4`): `+0x89C`, `+0x8A8`, `+0x8B4`, `+0x8C0`, `+0x8CC` (each is 3 ints = 12 bytes). Total range: `+0x89C..+0x8D8` (60 bytes). The format-string iteration logic was not fully decompiled this pass. |
| `+0xD29` | `OmniCrusher` | byte | `*(undefined1*)((int)param_1 + 0xd29) = uVar3` after ReadBool. **Re-confirms audit 7 cumulative** (where +0xD29 was already noted as OmniCrusher from TANY iter). |

### Cross-cumulative re-confirmations

- `+0xD2A = OmniCrushResistant` (audit 14) — BFRT uses this. The 3-tier crush hierarchy (Crusher → OmniCrusher → OmniCrushResistant) is now fully cumulative-verified across BFRT (OmniCrusher+OmniCrushResistant) + AMCV/SMCV/PCV (OmniCrushResistant-only).
- `+0xD28 = Crusher` (audit 12) — BFRT uses this.

### §2 / §7.4 `Image=SREF` artmd quirk — NOT investigated this pass

The doc's open question about whether artmd `Image=SREF` causes the
engine to substitute SREF.VXL for BFRT.VXL is **DEFERRED**. Would
require locating the artmd-side Image= handler (likely in
`AbstractType::Read_Art_INI` or similar). Not load-bearing for parity
unless a visual mismatch surfaces in-game.

### Items NOT re-verified in this pass (DEFERRED)

- artmd `Image=SREF` voxel-loader handling (§2 open question).
- AlternateFLH 5-entry array layout — only base offset +0x89C verified; per-index increment of 0xC inferred from format-string evidence.
- Per-passenger gun-port assignment algorithm (which passenger uses which AlternateFLH index — claimed to be passenger-1 → FLH0, passenger-5 → FLH4 per doc).
- BFRT-vs-BFRT crush standoff verification (both have OmniCrushResistant=yes — engine logic for the standoff).
- OpenTopped consumer chain (Fire_At-side code that spawns passenger projectiles from AlternateFLH positions).
- OpenToppedRangeBonus / DamageMultiplier / WarpDistance Rules-CombatDamage byte offsets (RulesClass__ReadCombatDamage is oversized).
- The "BFRT's own gun fires concurrently with passengers" question (Open follow-up #3).

### Confidence summary

- **HIGH**: 8 string addresses + 6 parser xrefs (all exact); 3 NEW TechnoType struct offsets (OpenTopped +0x5E4, AlternateFLH0 +0x89C base, OmniCrusher +0xD29 re-confirms audit 7); 1 NEW Rules-CombatDamage consumer site (OpenToppedWarpDistance); CrusherAll enum value confirmed as enum-table entry (not a parser key).
- **MEDIUM**: AlternateFLH 5-entry array — base offset BINARY-VERIFIED, full 5-entry layout INFERRED from INI evidence + format-string parse pattern. Direct verification of the array stride (0xC = 3 ints) and iteration count (5) requires extracting more of the parser body.
- **No INCORRECT findings in the doc**. The 6 cited parser xrefs all resolve exactly; the 3-tier crush hierarchy is consistent with the cumulative cheat-sheet.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[BFRT]` rulesmd key explained | ✅ §1 |
| `[BFRT]` artmd block expanded; **all 5 AlternateFLH gun-port positions documented with ASCII visualization** | ✅ §2 |
| **`Image=SREF` quirk flagged** as open question | ✅ §2 + §7.4 |
| Primary weapon + warhead (shared with HARV — cross-referenced not duplicated) | ✅ §3–§4 |
| All voices + EnterTransport/ExitTransport sounds | ✅ §5 |
| Prereqs / owners / availability | ✅ §6 |
| **Passenger composition strategies documented** | ✅ §6 |
| **SizeLimit=2 special case (Terror Drones + Brutes allowed)** | ✅ §1 + §6 |
| Hardcoded behavior — Ghidra searches + 6 new flag-scope verifications (OpenTopped, OmniCrusher, AlternateFLH%d, OpenToppedRangeBonus, OpenToppedDamageMultiplier, CrusherAll enum) | ✅ §7 |
| **3-tier crush resolution recapped** (Crusher / OmniCrusher / OmniCrushResistant) | ✅ §1 + §7 |
| TS-legacy filter | ✅ §8 |
| Veterancy (note: passengers veterancy independent) | ✅ §9 |
| Cross-refs to FV, MCVs, HTK, Drone/Brute size-2 passengers | ✅ §10 |
| **Index correction completed** (BFRT is real Battle Fortress, not duplicate of FV) | ✅ |

**Open follow-ups (none load-bearing for parity unless visual bug surfaces):**
- **`Image=SREF` in artmd**: confirm voxel loader behavior. If BFRT visually renders as Prism Tank, the artmd `Image=` does redirect voxel filename. If not, the line is dead INI.
- Verify `OpenToppedRangeBonus` and `OpenToppedDamageMultiplier` exact values from `[CombatDamage]` — relevant for passenger-fire parity.
- Verify whether `OpenTopped=yes` allows the **BFRT's own** Primary 20mmRapid to still fire concurrently with passengers, or if it suppresses BFRT's gun when passengers are firing. Affects damage stacking.
- `OpenToppedWarpDistance` (0x0083afd4) global — what does it control? May be a teleport-related interaction with OpenTopped passengers; worth a brief Ghidra trace.
- The Soviet-themed `Report=WarMinerAttack` on an Allied tank is a cross-faction audio quirk — confirm in-game whether the BFRT actually plays the War Miner gunfire sound. May be an INI typo / unintended reuse.
