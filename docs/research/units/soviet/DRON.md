# DRON — Terror Drone

**Side classification:** Soviet (Owner=Russians,Confederation,Africans,Arabs).
**Role:** Anti-vehicle parasite. Fast-moving (Speed=10) ground unit that fires the
`DroneJump` weapon at close range (Range=1.83) — on hit, the Drone enters
"LimboLaunch" mode (vanishes off-map, becomes the projectile), parasitically attaches
to the target via `WarheadTypeClass::Parasite`, then deals periodic damage from inside
until the host dies or the Drone is killed by a `BombDisarm`-style counter.

> Output bar: the Terror Drone is parity-critical — its host-kill timer, escape window
> for the host (Service Depot / Engineer), and AoE-defuse interactions all need to
> match gamemd exactly. The drone's "scuttling spider" feel hinges on movement
> animation tempo, suppression-threshold-bypass, and ParasiteClass state machine.

> **Deep-RE cross-reference — DO NOT re-derive:**
> [PARASITE_CLASS_GHIDRA_REPORT.md](../../PARASITE_CLASS_GHIDRA_REPORT.md) — full
> reverse-engineering of `ParasiteClass` lifecycle, fields, state machine, attach/release
> logic. The Terror Drone's host-kill behavior is **driven entirely by
> `Warhead.Parasite=yes`** at `WarheadType+0x159`. Gates verified at
> `TechnoClass::Init_Managers @ 0x006F3F40`.

> Ghidra confirms no `"DRON"` / `"TerrorDrone"` strings in `gamemd.exe` for the unit
> ID — all behavior is generic flag-driven via `[Parasite] Parasite=yes` warhead flag +
> `[DroneJump] LimboLaunch=yes` weapon flag.

---

## 1. `rulesmd.ini` — `[DRON]` verbatim

```ini
[DRON]
UIName=Name:DRON
Name=Terror Drone
Category=AFV
Prerequisite=NAWEAP
Primary=DroneJump
Secondary=VirtualScanner
NavalTargeting=6
Strength=100
SuppressionThreshold=5; damage below this amount won't suppress the parasite
ReselectIfLimboed=yes ; If selected when limbo on attack of infantry, reselect when unlimbo
DefaultToGuardArea=yes ; the much awaited terror drone default to move and attack when resting
Armor=special_1
TechLevel=4
Turret=no
IsTilter=no
CrateGoodie=no
Sight=4
Speed=10 ; gs Don't go higher than 20, or he gets stuck running in circles
Owner=Russians,Confederation,Africans,Arabs
Cost=500
Soylent=500
Points=20
ROT=40
AllowedToStartInMultiplayer=no
Crusher=no
Crewed=no
IsSelectableCombatant=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=TerrorDroneSelect
VoiceAttack=TerrorDroneAttackCommand
VoiceMove=TerrorDroneMove
VoiceFeedback=
DieSound=TerrorDroneDie
MoveSound=TerrorDroneMoveLoop
MaxDebris=2
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1};<-drive   mech->{55D141B8-DB94-11d1-AC98-006008055BB5}
;MovementZone=Normal ;gs FLAW needs to be changed to this when The Flaw is fixed
MovementZone=Destroyer
ThreatPosed=25	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
Weight=.5
ImmuneToPsionics=yes
ImmuneToRadiation=yes
Parasiteable=no
Trainable=no
Explodes=no
AccelerationFactor=5 ; really fast
DeaccelerationFactor=5 ; This is TS's mizspelingg knot min
ZFudgeColumn=8
ZFudgeTunnel=13
;Bombable=no
Size=2
Accelerates=false
Bunkerable=no; Units default to yes, others default to no
```

### Key-by-key explanation

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:DRON` | AbstractType | CSF lookup. |
| `Name` | `Terror Drone` | AbstractType | Dev fallback. |
| `Category` | `AFV` | TechnoType | AFV classifier (despite "drone"). |
| `Prerequisite` | `NAWEAP` | TechnoType | Soviet War Factory only. |
| `Primary` | `DroneJump` | TechnoType | Parasite-attach weapon — see §3. |
| `Secondary` | `VirtualScanner` | TechnoType | INI comment: "This is so units with range one weapons will scan out farther when looking for targets in guard". `[VirtualScanner]` has `Range=5, NeverUse=yes, Damage=1, InvisibleAll, Warhead=SA` — it's a **dummy weapon used only for guard-range expansion**. The drone scans for targets within 5 cells (using VirtualScanner) but can only actually fire at 1.83 cells (using DroneJump). |
| `NavalTargeting` | `6` | TechnoType (verified — 0x00844510 → 0x007121be) | **AI targeting hint for naval ops.** Value `6` likely corresponds to "anti-naval" or "naval-attack-priority" enum value. Drones can attack ships (squids do too); this flag may control how AI weights drone deployment vs naval threats. |
| `Strength` | `100` | AbstractType | 100 HP — fragile. Three GI shots will kill a drone. |
| `SuppressionThreshold` | `5` | TechnoType (verified — 0x008436ec → 0x0071506d) | **Damage below 5 won't suppress** the unit's parasite-attach behavior. INI comment confirms. Standard infantry suppression makes units crawl/seek cover; the drone ignores small-arms suppression so it can keep running to its target. |
| `ReselectIfLimboed` | `yes` | TechnoType (verified — 0x00843d74 → 0x007142b4) | INI comment: "If selected when limbo on attack of infantry, reselect when unlimbo". When the drone fires `LimboLaunch=yes` (vanishes into the target), if the player had it selected, the selection persists — when the drone unlimbo (escapes or is freed), it's reselected. Quality-of-life for player micro. |
| `DefaultToGuardArea` | `yes` | TechnoType (verified — 0x00843784 → 0x00714f44) | INI comment: "the much awaited terror drone default to move and attack when resting". The drone's default mission is `Guard_Area` instead of `Guard` — meaning it actively patrols/intercepts threats in its area instead of just standing still. **Critical for the "drone protective bubble" play pattern** where you station drones around your base and they auto-engage attackers. |
| `Armor` | `special_1` | TechnoType | **Verses-slot 10.** The HARV's HARVWH has `special_1=400%` — confirming HARV's anti-drone bonus. Most warheads have `special_1=100%` or less. |
| `TechLevel` | `4` | TechnoType | Tier-4-ish, but available after Radar via NAWEAP only (no Radar prereq listed; combined with TechLevel=4 implies a build-tree-level gate). |
| `Turret` | `no` | UnitType | Body-only aim. |
| `IsTilter` | `no` | UnitType | Voxel doesn't tilt on slopes (drone has no traditional hull). |
| `CrateGoodie` | `no` | UnitType | Excluded from crate pool. |
| `Sight` | `4` | TechnoType | Short reveal. |
| `Speed` | `10` | TechnoType | **Tied with IFV as fastest ground vehicle.** INI comment: "gs Don't go higher than 20, or he gets stuck running in circles". The author noted a high-speed pathing bug — drones at speed 21+ cycle in place. |
| `Owner` | 4 Soviet countries | TechnoType | Soviet only (no Yuri). |
| `Cost` | `500` | TechnoType | Cheap. |
| `Soylent` | `500` | TechnoType | 100% Grinder refund. |
| `Points` | `20` | TechnoType | Score on kill. |
| `ROT` | `40` | TechnoType | **Very fast rotation** (vs MBTs' ROT=5). The drone can pivot near-instantly to face new targets. |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Not preplaced. |
| `Crusher` | `no` | TechnoType | **Cannot crush infantry.** Drone is too small. |
| `Crewed` | `no` | TechnoType | No survivors on death. |
| `IsSelectableCombatant` | `yes` | TechnoType | Counts in select-all-combat. |
| `Explosion` | `TWLT070,...` | TechnoType | Standard death pool. |
| `VoiceSelect` | `TerrorDroneSelect` | TechnoType | 2 clips (`vtersela vterselb`). Minimalist. |
| `VoiceAttack` | `TerrorDroneAttackCommand` | TechnoType | **Single clip `vtermova`** (same as VoiceMove — the drone's attack response is the move-sound). |
| `VoiceMove` | `TerrorDroneMove` | TechnoType | Single clip `vtermova`. |
| `VoiceFeedback` | *(empty)* | TechnoType | No under-attack voice. |
| `DieSound` | `TerrorDroneDie` | TechnoType | 2 clips. |
| `MoveSound` | `TerrorDroneMoveLoop` | TechnoType | **Looping engine sound** — 7 layered clips (`vterlo1a..3`), `Control=Random loop all attack decay`, `Priority=high`, `Attack=3`. The drone's signature "skittering insect" loop. |
| `MaxDebris` | `2` | TechnoType | 2 debris pieces. |
| `Locomotor` | `{4A582741-...}` | TechnoType | DriveLocomotionClass. INI comment shows TS-mech CLSID alternative — design history. |
| Commented `;MovementZone=Normal ;gs FLAW needs to be changed to this when The Flaw is fixed` | — | — | Same "FLAW" pathfinding bug noted in MTNK doc. Drone uses Destroyer zone instead. |
| `MovementZone` | `Destroyer` | TechnoType | Can traverse some crushable terrain. |
| `ThreatPosed` | `25` | TechnoType | Mid-high AI threat. |
| `DamageParticleSystems` | `SparkSys,SmallGreySSys` | TechnoType | Damage emitters. |
| `Weight` | `0.5` | TechnoType | **Very light** — bridges can support more drones than tanks. |
| `ImmuneToPsionics` | `yes` | TechnoType | Cannot be mind-controlled. |
| `ImmuneToRadiation` | `yes` | TechnoType | Desolator rad fields don't damage drones — important for terror-counter-rush. |
| `Parasiteable` | `no` | TechnoType (verified — 0x00843768 → 0x00714f86) | **The drone cannot have parasites attached to it.** Other Terror Drones cannot infest a drone (drone-vs-drone is harmless). Squids and other Parasite-warhead weapons also fail vs drones. |
| `Trainable` | `no` | TechnoType | Cannot gain veterancy. Drone is a 1-trick unit — XP wouldn't change its role. |
| `Explodes` | `no` | TechnoType (verified prior iter — 0x0083355c) | No chain-reaction death (vs Apocalypse's `=yes`). |
| `AccelerationFactor` | `5` | TechnoType | INI comment: "really fast". Higher acceleration than most vehicles. |
| `DeaccelerationFactor` | `5` | TechnoType | INI comment: "This is TS's mizspelingg knot min" (intentional typo joke). Deceleration rate. |
| `ZFudgeColumn` | `8` | UnitType | Standard z-fudge. |
| `ZFudgeTunnel` | `13` | UnitType | TS-legacy. |
| Commented `;Bombable=no` | — | — | Inert. Default `Bombable=yes` — Crazy Ivan can plant bombs on Terror Drones. |
| `Size` | `2` | TechnoType | **Size=2** — can fit in Battle Fortress (SizeLimit=2) but NOT in Multi-Gunner IFV (SizeLimit=1). |
| `Accelerates` | `false` | TechnoType | No accel ramp (despite `AccelerationFactor=5` — different mechanic). |
| `Bunkerable` | `no` | TechnoType | Cannot enter Battle Bunker. |

### Notable absent keys
- No `Image=` redirect — reads its own `[DRON]` artmd block.
- No `ElitePrimary=` — combined with `Trainable=no`, no elite weapon swap.
- No `VeteranAbilities=` / `EliteAbilities=` lines — no stat boosts even theoretically.
- No `OpportunityFire=yes` — but `DefaultToGuardArea=yes` achieves similar effect (auto-patrols and intercepts).
- No `OmniCrusher` / `OmniCrushResistant` — drone is small, normal crush rules apply (Crusher=yes vehicles squish it).
- No `OmniFire=yes` — single-target attacks.
- No `Teleporter=` — drone moves normally.

---

## 2. `artmd.ini` — `[DRON]` section

```ini
[DRON] ; Terror Drone
Voxel=no
Remapable=yes
Cameo=DRONICON
PrimaryFireFLH=0,0,30
WalkFrames=6
FiringFrames=4
```

| Key | Value | Effect |
|-----|-------|--------|
| `Voxel` | **`no`** | **Sprite-rendered, not voxel.** Unusual for a vehicle. Drone uses SHP (sprite) animation like infantry — the `WalkFrames=6` and `FiringFrames=4` below explain why. |
| `Remapable` | `yes` | House-color remap. |
| `Cameo` | `DRONICON` | Sidebar cameo. |
| (no `AltCameo`) | — | No Yuri-skinned alt cameo. |
| `PrimaryFireFLH` | `0,0,30` | Firing offset: at unit center, Z=30 (low height). Drone's spider-leap is from its own center. |
| `WalkFrames` | `6` | **6-frame walk cycle**. SHP-style animation key. |
| `FiringFrames` | `4` | **4-frame fire/jump cycle**. The "Terror Drone attacks" anim has 4 frames between LimboLaunch's vanish-into-target. |

Notable absent keys:
- No `Sequence=` — vehicles don't use infantry frame tables; instead `WalkFrames` + `FiringFrames` are vehicle-specific anim keys for sprite vehicles.
- No `TurretOffset=` — no turret (Turret=no in rulesmd).
- No `SecondaryFireFLH=` — Secondary `VirtualScanner` is `NeverUse=yes`, no visible fire.

---

## 3. Weapon — `[DroneJump]`

```ini
[DroneJump]
Damage=50
ROF=60
Range=1.83
Projectile=JUMP
Speed=30
Warhead=Parasite
LimboLaunch=yes ; Limbo shooter at launch (one shot or become the bullet)
Report=TerrorDroneAttack
PenetratesBunker=yes;If shot at a bunkered tank, no means the bunker gets the damage, yes means the unit does
;In Terror Drone case, it just handles the bad case where a TD'd guy makes it into a bunker.
FireInTransport=no;can't fire out of the BattleFortress
```

| Key | Value | Effect |
|-----|-------|--------|
| `Damage` | `50` | Per-hit damage **during the parasite phase**. The drone applies 50 dmg per tick / cycle from inside the host. Combined with `ROF=60` (60 ticks between cycles = 1s @ 60fps), the drone does roughly 50 dmg/sec to its host. Most vehicles die in ~6-10 cycles. |
| `ROF` | `60` | Cooldown between parasite-damage applications. |
| `Range` | `1.83` | **Short attack range.** Drone must close to <2 cells to jump onto a target. |
| `Projectile` | `JUMP` | "Drone jump" projectile (`Image=DRONP`, AA=no, Arm=2, ROT=8, Proximity=yes, Ranged=yes, FirersPalette=yes, SubjectToWalls=yes). The projectile uses `Image=DRONP` due to engine quirk: INI comment "Hmm...Requires an Image entry to get at Rotates=. Violates the same name default rule" — meaning the engine ignores `Rotates=true` unless an explicit `Image=` is set. |
| `Speed` | `30` | Projectile speed. |
| `Warhead` | `Parasite` | **The hardcoded warhead with `Parasite=yes`** — triggers ParasiteClass attach on hit. See §4. |
| `LimboLaunch` | `yes` | INI comment: "Limbo shooter at launch (one shot or become the bullet)". **The drone vanishes from the map on firing** — it doesn't fire a separate projectile and remain. Instead, the drone IS the projectile. On hit, the drone enters parasite-attached state inside the target. Verified WeaponType-scoped at 0x0084952c → 0x00772107. |
| `Report` | `TerrorDroneAttack` | The drone's attack sound. |
| `PenetratesBunker` | `yes` | **⚠ INI scope quirk**: this key is verified as **WarheadType-scoped** (0x00847e08 → `WarheadTypeClass__ReadINI_Body` @ 0x0075d52f), NOT WeaponType-scoped. Placing it in `[DroneJump]` (a weapon block) means it's **likely ignored** — the engine reads it from `WarheadTypeClass::ReadINI` against the `[Parasite]` warhead block, not from the weapon block. The intent (per INI comment: "If shot at a bunkered tank, no means the bunker gets the damage, yes means the unit does") is to have drones penetrate bunkers to attach to the unit inside. If this line is ignored, drones may not penetrate bunkers as intended. **Open question** — see §7.4. |
| `FireInTransport` | `no` | INI comment: "can't fire out of the BattleFortress". Drone inside a Battle Fortress cannot launch — must be ejected first. Verified WeaponType-scoped (cheat sheet). |

### `[JUMP]` projectile

```ini
[JUMP]
Image=DRONP ;Hmm...Requires an Image entry to get at Rotates=.  Violates the same name default rule
AA=no
Arm=2
ROT=8 ;requires to use Rotates
Shadow=no
Proximity=yes
Ranged=yes
FirersPalette=yes ; Borrows the convertClass from the firing unit - gets house color too
SubjectToCliffs=no
SubjectToElevation=no
SubjectToWalls=yes
```

- `Image=DRONP` — uses Drone-Projectile sprite (mid-jump animation). Without this `Image=` line, the engine defaults to `JUMP.SHP` but cannot enable rotation. Engine quirk acknowledged in INI comment.
- `AA=no` — cannot target aircraft (drone can't jump up).
- `Arm=2` — projectile arm-time (delay before damage applies).
- `ROT=8` — projectile rotation speed (the drone tumbles mid-jump).
- `Proximity=yes` — explodes when near target, not on direct hit.
- `Ranged=yes` — fuse-based, will detonate on timer or proximity.
- `FirersPalette=yes` — uses firer's house color (drone color matches owner).
- `SubjectToWalls=yes` — walls block the drone's jump.
- `SubjectToCliffs=no, SubjectToElevation=no` — drone jump ignores terrain height.

---

## 4. Warhead — `[Parasite]`

```ini
[Parasite]; Terror Drone
Verses=100%,100%,100%,100%,100%,100%,0%,0%,0%,0%,0%
Parasite=yes
InfDeath=1
Rocker=yes
```

| Slot | Armor | Verses | Notes |
|------|-------|--------|-------|
| 1 | none | 100% | (theoretically can attach to infantry — but Drone Range=1.83 vs infantry's small footprint may make it tricky) |
| 2 | flak | 100% | (Flak Trooper) |
| 3 | plate | 100% | (Tanya/SEAL) |
| 4 | light | 100% | Grizzly/Mirage/IFV |
| 5 | medium | 100% | Apocalypse-base armor |
| 6 | heavy | 100% | Rhino/MBT |
| 7 | wood | **0%** | Cannot parasite-attach to buildings |
| 8 | steel | 0% | Same |
| 9 | concrete | 0% | Same |
| 10 | special_1 | **0%** | **Drones cannot parasite each other** (drone's own armor is `special_1`) |
| 11 | special_2 | 0% | |

| Key | Effect |
|-----|--------|
| `Parasite` | `yes` | **The hardcoded flag.** Verified WarheadType-scoped at 0x0081717c → 0x0075d83b. Triggers `ParasiteClass::Attach` on hit. Full mechanism in [PARASITE_CLASS_GHIDRA_REPORT.md](../../PARASITE_CLASS_GHIDRA_REPORT.md). |
| `InfDeath` | `1` | Small-arms infantry death (if drone attaches to infantry — rare). |
| `Rocker` | `yes` | Target rocks visually on hit (cosmetic shake). |

Notable: drone has **0% vs all special_X armors** — drones cannot infest each other or
any unit using `Armor=special_1/2`. Combined with `Parasiteable=no` on DRON itself,
this is a double-lock against drone-vs-drone infestation.

### Full Parasite lifecycle — cross-reference (do NOT re-derive)

Per [PARASITE_CLASS_GHIDRA_REPORT.md](../../PARASITE_CLASS_GHIDRA_REPORT.md):

1. **Setup at construction** (`TechnoClass::Init_Managers @ 0x006F3F40`): if the unit's Primary weapon's warhead has `Parasite=yes` (bit at WarheadType+0x159), allocate a 88-byte `ParasiteClass` and attach to FootClass+0x69C.
2. **Fire** (`LimboLaunch=yes` on DroneJump): drone vanishes from map (limbo state), the bullet representing the drone travels to target.
3. **Hit** (warhead detonate): `ParasiteClass::Attach(target, attacker=drone)` sets up the attached-state. The victim's FootClass+0x694 is set to the attacker pointer.
4. **Periodic damage**: every ROF ticks (60), the parasite applies Damage=50 to the host using a synthetic damage path. Host's HP decreases.
5. **Host actions**:
   - **Service Depot** — repair removes the parasite.
   - **Engineer** — engineer in transit (if the drone-host is a transport with passengers) may eject the drone? (verify in deep doc).
   - **BombDisarm-warhead hit** — counters the parasite-attach state (Chrono Legionnaire, SPY can do this).
   - **Friendly fire on the host** — vehicle-to-vehicle damage will hit the host normally, can kill faster, but doesn't kill the parasite directly.
6. **Forced release**: when host dies, drone re-emerges at host's death location with reduced HP. Drone can re-acquire targets.

The full state machine + 88-byte struct layout + all field offsets + every release-path
xref is documented in the deep-RE report. See §10 cross-reference.

---

## 5. Voices / sounds

```ini
[TerrorDroneAttackCommand]
Sounds= vtermova
FShift= -10 10
Volume=40

[TerrorDroneMove]
Sounds= vtermova
FShift= -10 10
Volume=40

[TerrorDroneMoveLoop]
Sounds=vterlo1a vterlo1b vterlo1c vterlo2a vterlo2b vterlo2c vterlo3
Control=Random loop all attack decay
Priority= high
Attack=3
VShift=10
FShift=-10 0
Volume=25

[TerrorDroneSelect]
Sounds= vtersela vterselb
Control= random
FShift= -10 10
Volume=50

[TerrorDroneDie]
Sounds= vterdiea vterdieb
Control=random
VShift=20
Volume=75
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=TerrorDroneSelect` | 2 clips, FShift ±10, vol 50 | Click-select |
| `VoiceMove=TerrorDroneMove` | **1 clip `vtermova`**, FShift ±10, vol 40 | Move order — same clip as attack |
| `VoiceAttack=TerrorDroneAttackCommand` | 1 clip `vtermova` (same as move) | Attack order |
| `VoiceFeedback=` *(empty)* | — | No under-attack |
| `DieSound=TerrorDroneDie` | 2 clips, VShift +20, vol 75 (loud) | Death |
| `MoveSound=TerrorDroneMoveLoop` | **7 clips, `Control=Random loop all attack decay`**, `Priority=high`, `Attack=3`, FShift -10 to 0, VShift +10, vol 25 | **Looping skitter sound** while drone is moving |
| `Report=TerrorDroneAttack` (weapon) | (in soundmd line 1899-ish — referenced) | Per-attack jump sound |

The looping `TerrorDroneMoveLoop` is the drone's signature audio — 7 layered clips
played randomly in a continuous loop with **attack=3** (3-tick attack envelope) and
**decay** (envelope fade-out). This creates the "constantly skittering insect" feel.
`Priority=high` ensures the loop isn't dropped under audio pressure.

---

## 6. Prerequisites / owners / availability

- **Prerequisite**: `NAWEAP` — Soviet War Factory only.
- **TechLevel** = `4` — combined with Radar building default unlock, available early-mid game.
- **Owner**: 4 Soviet countries.
- **CrateGoodie**: `no` — excluded from crate pool.
- **Cost** = $500. Very cheap. Drone rushes are economically devastating because each $500 drone can kill a $900+ tank.
- **`AllowedToStartInMultiplayer=no`** — not preplaced.

### Strategic positioning

The Terror Drone is a **hard counter to vehicles** with three key vulnerabilities:
1. **Anti-infantry weapons** (HARV's HARVWH 400% vs special_1, basic infantry small-arms) — drone dies fast under any concentrated fire.
2. **Service Depot** — instantly removes parasites from vehicles. Players "garage-park" infested vehicles to scrape drones off.
3. **`Parasiteable=no` units** — drones cannot infest other drones, MCVs (depends on flag), or specific protected units.

But the drone shines:
- **vs harvesters** — un-escorted CMIN/HARV can be killed in seconds; even with escort, drones jump faster than the escort can react.
- **vs MBTs** — a single drone-attach can kill a Rhino before Service Depot is reachable.
- **vs Apocalypse** — the 800 HP APOC takes ~16 cycles (16s) to die from a drone — manageable, but the APOC is denied combat during that time.

The `DefaultToGuardArea=yes` makes drones excellent **base perimeter sentries** —
station 4-6 drones around mineral fields and they auto-engage any vehicle that
enters their patrol radius.

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 DRON-specific code in `gamemd.exe`

| Query | Result |
|-------|--------|
| `DRON` | 0 matches |
| `TerrorDrone` | (would match many keys — not specifically the unit ID; verified no plain "DRON" string) |

⇒ **No DRON-specific hardcoded ID** — all behavior is generic flag-driven via the warhead-level `Parasite=yes` and weapon-level `LimboLaunch=yes`.

### 7.2 Flag-scope verification (this iteration)

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `Parasite` (warhead flag) | 0x0081717c | WarheadTypeClass__ReadINI @ 0x0075d83b | **WarheadType** (bit at WarheadType+0x159) |
| `Parasiteable` | 0x00843768 | TechnoTypeClass__ReadINI @ 0x00714f86 | TechnoType |
| `DefaultToGuardArea` | 0x00843784 | TechnoTypeClass__ReadINI @ 0x00714f44 | TechnoType |
| `ReselectIfLimboed` | 0x00843d74 | TechnoTypeClass__ReadINI @ 0x007142b4 | TechnoType |
| `SuppressionThreshold` | 0x008436ec | TechnoTypeClass__ReadINI @ 0x0071506d | TechnoType |
| `NavalTargeting` | 0x00844510 | TechnoTypeClass__ReadINI @ 0x007121be | TechnoType |
| `LimboLaunch` (weapon flag) | 0x0084952c | WeaponTypeClass__ReadINI @ 0x00772107 | **WeaponType** |
| `PenetratesBunker` | 0x00847e08 | WarheadTypeClass__ReadINI_Body @ 0x0075d52f | **WarheadType** (despite being in INI weapon block — see §7.4 quirk) |

### 7.3 Live behaviors driven by these flags

| Behavior | Driver | Notes |
|----------|--------|-------|
| Parasite attach on hit | `[Parasite] Parasite=yes` + ParasiteClass setup at `TechnoClass::Init_Managers @ 0x006F3F40` | See [PARASITE_CLASS_GHIDRA_REPORT.md](../../PARASITE_CLASS_GHIDRA_REPORT.md) |
| LimboLaunch: drone vanishes on fire | `[DroneJump] LimboLaunch=yes` | One-shot or become the bullet |
| ReselectIfLimboed: maintains player selection | `ReselectIfLimboed=yes` | Selected drone stays "selected" through limbo |
| Default to area-guard (auto-patrol) | `DefaultToGuardArea=yes` | Idle drone moves & attacks within area |
| Cannot be parasite-attached | `Parasiteable=no` | Drone immune to other parasites |
| Drone-vs-drone fails | `[Parasite] Verses[10]=0%` AND `Parasiteable=no` (double-lock) | Drones cannot infest each other |
| Suppression-bypass for small damage | `SuppressionThreshold=5` | Drone keeps charging through light fire |
| AI naval targeting | `NavalTargeting=6` | Affects AI's drone-vs-ship preference |
| Cannot crush | `Crusher=no` | Drone is too small |
| Sprite-rendered (not voxel) | artmd `Voxel=no` + `WalkFrames=6`, `FiringFrames=4` | Unique among vehicles |
| Cannot board Battle Fortress except by Size=2 | `Size=2` matches BFRT's `SizeLimit=2` | Drone IS one of the units BFRT explicitly allows in |
| Immune to psi and rad | `ImmuneToPsionics=yes`, `ImmuneToRadiation=yes` | |
| No veterancy | `Trainable=no` | Stuck at rookie |

### 7.4 ⚠ `PenetratesBunker=yes` placement quirk

The `PenetratesBunker=yes` line is in the weapon block `[DroneJump]` but the flag is
verified as **WarheadType-scoped** (0x00847e08 → `WarheadTypeClass__ReadINI_Body`).
INI semantics: a section header `[X]` causes the engine to read keys from that section
into the type matching `X` (Weapon, Warhead, Unit, etc.). `[DroneJump]` is parsed as a
WeaponType — its keys go to WeaponTypeClass. The WeaponTypeClass reader **does not**
handle `PenetratesBunker`, so this line is **likely ignored**.

The intent (per INI comment: "If shot at a bunkered tank, no means the bunker gets the
damage, yes means the unit does") clearly wants drones to penetrate bunkers. Since the
key is misplaced:

1. **If the line is truly ignored**: drones cannot penetrate bunkers — bunker takes the damage, tank inside is safe. (Likely current behavior.)
2. **If the engine has a fallback** (reads `PenetratesBunker` from the weapon's warhead block instead): then `[Parasite]` warhead would need the flag — but it doesn't have one. So fallback also yields "no penetration".

**Parity-critical**: verify in-game whether a Terror Drone fired at a bunkered tank
infests the tank or just damages the bunker. If gamemd's drone DOES penetrate, there
must be hardcoded special-case logic (LimboLaunch + Parasite-warhead might have its own
override).

### 7.5 Behaviors NOT present

- No `Veteran*Abilities` lists — Trainable=no means they wouldn't apply anyway.
- No `OmniCrusher` — drone is small.
- No `Spawns=`.
- No `Teleporter=`.

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ZFudgeTunnel=13` | YES | Dormant render value. |
| Commented `;Bombable=no` | n/a (commented) | Inert. |
| Commented `;MovementZone=Normal ;gs FLAW needs to be changed` | n/a (workaround) | Inactive. |
| Locomotor INI comment `<-drive mech->{55D141B8-...}` | YES — `55D141B8-...` is TS-era MechLocomotion (Cyborg from TS) | Drone uses Drive locomotor, mech is commented-out alternative. |
| `DeaccelerationFactor` (intentional typo joke) | n/a | Live key, just funny. |

No fog-of-war refs, no real veinhole refs.

---

## 9. Veterancy

**`Trainable=no`** — Terror Drone cannot gain veterancy. No `VeteranAbilities=`, no
`EliteAbilities=`, no `ElitePrimary=` keys. Drone stays at rookie rank for its entire
lifetime — same design rationale as CIVAN (instakill mechanic doesn't balance with XP).

---

## 10. Cross-references

### Direct dependencies
- `[DroneJump]` — weapon (§3)
- `[VirtualScanner]` — Secondary "fake" weapon for guard-scan range
- `[JUMP]` — projectile (§3.1)
- `[Parasite]` — warhead (§4)
- `[DRONP]` (artmd) — projectile sprite (drone mid-jump)
- `[DRON]` (artmd) — sprite art block (Voxel=no, WalkFrames=6, FiringFrames=4)
- `[NAWEAP]` — prereq
- `[TerrorDroneSelect/Move/AttackCommand/Attack/MoveLoop/Die]` (soundmd) — voices and signature loop sound

### Conceptual companions
- **SQD (Giant Squid)** ([`soviet/SQD.md`](./SQD.md) — TODO) — naval parasite counterpart. Uses `SquidGrab` weapon + `Parasite` warhead. Same ParasiteClass mechanism.
- **DOG family** (ADOG, DOG, YDOG, YADOG) — uses `ParasiteDog` warhead with limited Verses (no vs vehicles). Different attach behavior.
- **HARV / CMIN** — drone's prime targets; HARV has 400% vs special_1 (counter-drone bonus).
- **Service Depot** (Allied/Soviet/Yuri Repair Pads) — counter to drone-attach.
- **CCOMAND / SPY** — likely have `BombDisarm` or similar counter-flags that can defuse drones.

### Deep-RE docs (cross-referenced, NOT re-derived)
- **[PARASITE_CLASS_GHIDRA_REPORT.md](../../PARASITE_CLASS_GHIDRA_REPORT.md)** — 88-byte struct layout, full state machine, attach/release/forced-release logic, all xrefs to relevant code paths. **Read first for any Terror Drone or Giant Squid implementation work.**

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[DRON]` rulesmd key explained | ✅ §1 |
| Every `[DRON]` artmd key explained — sprite (`Voxel=no`) noted as unique among vehicles | ✅ §2 |
| Weapon + projectile + warhead all expanded | ✅ §3–§4 |
| Full parasite lifecycle cross-referenced (not duplicated) to PARASITE_CLASS_GHIDRA_REPORT | ✅ §4 |
| Signature `TerrorDroneMoveLoop` skitter-sound documented | ✅ §5 |
| Prereqs / owners / strategic positioning | ✅ §6 |
| Hardcoded behavior — Ghidra searches + 8 new flag-scope verifications | ✅ §7 (Parasite/Parasiteable/DefaultToGuardArea/ReselectIfLimboed/SuppressionThreshold/NavalTargeting/LimboLaunch/PenetratesBunker) |
| **⚠ PenetratesBunker scope quirk flagged** (key in weapon block but verified WarheadType-scoped → likely ignored) | ✅ §7.4 |
| TS-legacy filter | ✅ §8 |
| Veterancy (Trainable=no → permanently rookie) | ✅ §9 |
| Cross-refs to PARASITE_CLASS_GHIDRA_REPORT deep doc + companion units | ✅ §10 |

**Open follow-ups (parity-critical):**
- **`PenetratesBunker=yes` in `[DroneJump]` weapon block — does gamemd actually penetrate bunkers?** Critical for parity. If gamemd drones DO penetrate (visible in-game behavior), there must be hardcoded special-case logic — Ghidra-trace the bunker-hit resolution path with a parasite-warhead bullet.
- The Engineer-removes-parasite mechanism — confirm via PARASITE_CLASS report's "release paths" section whether Engineer entering an infested vehicle ejects the drone, or only Service Depot does.
- Drone-on-bridge behavior: does a drone falling from a destroyed bridge survive? `Weight=0.5` suggests minimal terrain interaction, but engine-level bridge handling may differ.
- The `Speed=10 ; Don't go higher than 20, or he gets stuck running in circles` comment — verify the high-speed pathing bug is in fact at speed 21+, and whether it's been worked around or remains a quirk.
- `[JUMP] Image=DRONP` engine quirk — INI comment "Requires an Image entry to get at Rotates=. Violates the same name default rule" — verify this default-rule-violation is still present in the binary (i.e., omitting `Image=` actually disables `Rotates=true`).
