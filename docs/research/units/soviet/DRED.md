# DRED — Dreadnought (Soviet Naval Spawner Siege)

**INI ID:** `DRED`
**Display Name:** `Dreadnought` (`UIName=Name:DRED`)
**Side:** Soviet (tech-restricted to the Big Three Soviet houses + Confederation — see `Owner=`)
**Category:** Vehicle / Naval
**Cameo:** `DREDICON`
**Voxel:** yes, with a dedicated "empty" voxel `DREDWO` swapped in once both missiles are away (`NoSpawnAlt=yes`).

The Dreadnought is the Soviet naval long-range siege ship. It carries two
`DMISL` spawned-aircraft missiles that fire in salvo, despawn on impact, and
regenerate from the parent ship over time. Its tactical role mirrors the V3
(see [`V3.md`](./V3.md)) but at sea — same `SpawnManagerClass` mechanism,
same hardcoded `RocketStruct` flight profile, just a different `RocketStruct`
slot (Rules.DMisl*), a different impact warhead (`DMislWarhead` / `DMislEliteWarhead`),
heavier ship hull, and a 2-missile salvo per fire cycle.

> **Cross-references — do not re-derive:**
> - [`SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md`](../../SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md) — full SpawnManager state machine, RocketStruct lookup table, kamikaze missile flow, Rules-based per-missile-family field offsets (0x4E4, 0x4E8, 0x514 for DMisl*). The Dreadnought is one of three "missile spawner" entries in §3 (V3 / Dread / Boomer Sub).
> - [`ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`](../../ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md) — RocketLocomotionClass (CLSID `{B7B49766-...}`) used by `DMISL`. Handles tilt / takeoff / cruise / dive states for all three missile families.
> - [`AIRBURST_SUB_WEAPON_SPAWN_GHIDRA_REPORT.md`](../../AIRBURST_SUB_WEAPON_SPAWN_GHIDRA_REPORT.md) — sibling research on spawned-projectile patterns.
> - [`V3.md`](./V3.md) — the V3 launcher uses the same Spawner=yes pattern; this doc references that one for shared concepts.
> - [`HARV.md`](./HARV.md), [`HTNK.md`](./HTNK.md) — sibling Soviet vehicles with similar Soviet-Big-Three `Owner=` patterns.

> **TS-legacy filter applied** — no fog-of-war 0x1000 gate, no ImmuneToVeins, no subterranean/tunnel logic in this unit; locomotor is ShipLocomotionClass (live in YR). The commented-out `;ForbiddenHouses=Russians` and `;BuildLimit=1` are RA2-era authoring drafts left as INI residue.

---

## 1. Full `rulesmd.ini` section verbatim

```ini
[DRED]
UIName=Name:DRED
Name=Dreadnought
Prerequisite=NAYARD,NATECH
Primary=DredLauncher
CanPassiveAquire=no ; Won't try to pick up own targets
Spawns=DMISL
SpawnsNumber=2
SpawnRegenRate=80
SpawnReloadRate=0 ; missile spawn don't come back
NoSpawnAlt=yes ; alternate voxel for out of spawns: xxxxWO (DREDWO)
FireAngle=32
ToProtect=yes
Category=Support
Strength=800
Naval=yes
Armor=heavy
TechLevel=6
Sight=7
Speed=4
CrateGoodie=no
Owner=Russians,Confederation,Africans,Arabs
;ForbiddenHouses=Russians
AllowedToStartInMultiplayer=no
Cost=2000
Soylent=2000
Turret=no
Points=55
Weight=4
ROT=1
Crusher=no ;gs yes
Crewed=no
;OmniFire=yes ;GEF moved to weapon
IsSelectableCombatant=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=GenSovWaterSelect
VoiceMove=GenSovWaterMove
VoiceAttack=GenSovWaterAttackCommand
VoiceFeedback=
DieSound=
SinkingSound=GenLargeWaterDie
MoveSound=DreadnoughtMoveStart
Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C};{4A582741-9839-11d1-B709-00A024DDAFD1}
SpeedType=Float
MovementZone=Water
ThreatPosed=25	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
TooBigToFitUnderBridge=true
GuardRange=10
;BuildLimit=1
Size=50
```

### 1.1 Key-by-key explanation (every line)

| Key | Value | Read by | Effect |
|-----|-------|---------|--------|
| `UIName=Name:DRED` | string | AbstractTypeClass | CSF lookup token for the localized UI string "Dreadnought". |
| `Name=Dreadnought` | string | AbstractTypeClass | Internal English fallback name (used when CSF lookup misses). |
| `Prerequisite=NAYARD,NATECH` | building list | TechnoTypeClass | **Both** Soviet Naval Yard AND Soviet Battle Lab required to build (logical AND). |
| `Primary=DredLauncher` | weapon | TechnoTypeClass | The "launcher" virtual weapon — `Spawner=yes` placeholder that fires `Spawns=` entries; see §3.1. |
| `CanPassiveAquire=no` | bool | TechnoTypeClass @ 0x00714473 | Dreadnought does **not** auto-acquire targets in range; it must be told to fire. Same as V3 (designer comment: "Won't try to pick up own targets"). |
| `Spawns=DMISL` | aircraft type | TechnoTypeClass / SpawnManagerClass | Names the spawned child aircraft type (`[DMISL]` aircraft section). |
| `SpawnsNumber=2` | int | TechnoTypeClass @ 0x00714ee1 | **2 missiles in the magazine** (the salvo size — Burst on the launcher also fires 2). |
| `SpawnRegenRate=80` | frames | TechnoTypeClass @ 0x00714ec0 | After a missile is consumed (killed on impact since `SpawnReloadRate=0`), wait 80 frames before regenerating a replacement. At 15 fps this is ~5.3 seconds. |
| `SpawnReloadRate=0` | frames | TechnoTypeClass @ 0x00714f02 | **0 = the missile is destroyed on impact and does NOT physically return for reload.** Compare with HORNET/ASW which fly back and physically dock to reload. The inline comment confirms this: "missile spawn don't come back". |
| `NoSpawnAlt=yes` | bool | ObjectTypeClass @ 0x005f943e | **When all spawnable missiles are away/depleted, the unit's voxel renders with an alternate body voxel suffixed `WO`** — here that is `DREDWO` ("Dread Without"). The empty missile racks are visually missing. Broader scope (ObjectType) than Spawner-related TechnoType flags. |
| `FireAngle=32` | int | TechnoTypeClass @ 0x00714b5d | The **vertical** launch angle (in BAM units, 0=horizontal 256=full circle) at which the missile leaves the launcher. 32/256 = 45° upward — missiles arc up & out, not straight forward. (Verified TechnoTypeClass scope.) |
| `ToProtect=yes` | bool | TechnoTypeClass @ 0x00714be8 | AI auto-protect marker — the AI will dispatch escorts to nearby enemies that threaten Dreadnoughts. Verified TechnoType scope. |
| `Category=Support` | enum | TechnoTypeClass | UI/AI category — `Support` (as opposed to AFV/IFV/Recon). Affects AI scripting and team-composition logic. |
| `Strength=800` | hp | TechnoTypeClass | 800 HP hull. |
| `Naval=yes` | bool | UnitTypeClass | Marks this as a naval unit (build-from-naval-yard, water-only). |
| `Armor=heavy` | enum | TechnoTypeClass | Armor class — affects warhead Verses[] multiplier indexing (heavy = column 4 of standard Verses). |
| `TechLevel=6` | int | TechnoTypeClass | Visible at TechLevel ≥6 (Soviet Battle Lab gates this). `-1` would hide from build list. |
| `Sight=7` | cells | TechnoTypeClass | 7-cell visual range. |
| `Speed=4` | int | TechnoTypeClass / UnitType | Maximum movement speed. Slow ship. |
| `CrateGoodie=no` | bool | UnitTypeClass @ 0x00747658 | Cannot pop out of a crate (unit-crate good-ies pool exclusion). |
| `Owner=Russians,Confederation,Africans,Arabs` | country list | TechnoTypeClass | **All four Big-Three+1 Soviet houses** (Russians/Iraq, Cuba/Confederation, Libya/Africans, Iraq alt/Arabs). NOT YuriCountry. |
| `;ForbiddenHouses=Russians` | (commented) | — | Inert — left over from RA2-era authoring. |
| `AllowedToStartInMultiplayer=no` | bool | TechnoTypeClass | Not in the player's pre-built starting roster. |
| `Cost=2000` | credits | TechnoTypeClass | Build cost. |
| `Soylent=2000` | credits | TechnoTypeClass | Yuri-Grinder recycle value (100% of cost — recycles for full price). |
| `Turret=no` | bool | UnitTypeClass | No rotating turret — the hull faces the target. The missiles raise from the deck via voxel animation. |
| `Points=55` | int | TechnoTypeClass | Score-screen kill value. |
| `Weight=4` | int | TechnoTypeClass | Used by transport-loading and AI weight calculations. |
| `ROT=1` | int | TechnoTypeClass | Rate of Turn — extremely slow turning (1 = 1/256 of a circle per frame; a Dread takes ~17 sec to do a full 360°). |
| `Crusher=no` | bool | TechnoTypeClass | Cannot crush infantry (waterborne, no relevance). The `;gs yes` comment suggests it was once toggled — currently no. |
| `Crewed=no` | bool | TechnoTypeClass | Does not pop a parachuting survivor crewman on destruction. |
| `;OmniFire=yes ;GEF moved to weapon` | (commented) | — | Inline note: OmniFire is set on the **weapon** [DredLauncher] instead of the techno. |
| `IsSelectableCombatant=yes` | bool | TechnoTypeClass | Counts as a real combat unit for selection (group-selecting "all combat units" includes it). |
| `Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60` | anim list | TechnoTypeClass | Random-pick on-destruction explosion animation set (5 entries). |
| `VoiceSelect=GenSovWaterSelect` | sound | TechnoTypeClass | Shared "generic Soviet water-unit select" voice (also used by Typhoon Sub, Sea Scorpion). |
| `VoiceMove=GenSovWaterMove` | sound | TechnoTypeClass | Shared "generic Soviet water-unit move" voice. |
| `VoiceAttack=GenSovWaterAttackCommand` | sound | TechnoTypeClass | Shared "generic Soviet water-unit attack" voice. |
| `VoiceFeedback=` | (empty) | TechnoTypeClass | No feedback voice (silent on under-attack feedback). |
| `DieSound=` | (empty) | TechnoTypeClass | No die sound — instead the `SinkingSound` plays once during the sink animation. |
| `SinkingSound=GenLargeWaterDie` | sound | TechnoType @ 0x00712fb0 (override) + Rules @ 0x006699a7 (global default) | **DUAL-READ pattern** — per-unit override of the global default. Plays when a destroyed naval unit begins sinking. `gnavsina` clip with predelay-interrupt control. |
| `MoveSound=DreadnoughtMoveStart` | sound | TechnoTypeClass | One-shot engine start when the ship begins moving (`vdrestaa/b/c` random-pick, with 0-400 frame pre-delay). |
| `Locomotor={2BEA74E1-...};{4A582741-...}` | CLSID | TechnoTypeClass | **The active locomotor is the first GUID — `{2BEA74E1-7CCA-11d3-BE14-00104B62A16C}` = `ShipLocomotionClass`**. INI `;` starts an inline comment, so `{4A582741-9839-11d1-B709-00A024DDAFD1}` (DriveLocomotionClass) is commented-out legacy. See [`BRIDGE_LOCOMOTOR_NONCOVERAGE_JUSTIFICATION.md`](../../BRIDGE_LOCOMOTOR_NONCOVERAGE_JUSTIFICATION.md) §2 for the CLSID→class table. |
| `SpeedType=Float` | enum | TechnoTypeClass | Movement type "Float" — affects passability tables and pathfinding zone (water). |
| `MovementZone=Water` | enum | TechnoTypeClass | Pathfinder zone — Water (cannot enter land cells). |
| `ThreatPosed=25` | int | TechnoTypeClass | AI threat-evaluation weight. Comment notes "MUST be 0 for all building addons" (unrelated to ships, but the comment is boilerplate). |
| `DamageParticleSystems=SparkSys,SmallGreySSys` | particle list | TechnoTypeClass | While damaged (yellow/red HP), emit these particle systems for the hit/smoke effect. |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | ability list | TechnoTypeClass | At veteran rank: +stronger HP, +firepower, +ROF, +sight, +speed. |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | ability list | TechnoTypeClass | At elite rank: gains **passive self-heal**, plus the four standard upgrades. Notably no SIGHT/FASTER on elite (sight/speed stay at the veteran value). |
| `TooBigToFitUnderBridge=true` | bool | UnitTypeClass @ 0x0074774e | **UnitType-scope** (NOT TechnoType — see cheat sheet). Blocks the ship from pathing under a bridge cell — it would collide visually. |
| `GuardRange=10` | cells | TechnoTypeClass | If guarding (G hotkey), engage targets within 10 cells. Larger than Sight (7) — the Dread will fire on radar-revealed targets up to 10 cells (limited by weapon Range=25, so this is a guard-engagement cap, not weapon range). |
| `;BuildLimit=1` | (commented) | — | Currently no per-player limit. Disabled. |
| `Size=50` | int | TechnoTypeClass | Cargo-size cost (effectively "you can't fit this in a transport" — Size=50 vs typical 1-5 for infantry, 3 for tanks). |

---

## 2. Full `artmd.ini` section verbatim

```ini
[DRED]
Cameo=DREDICON
Voxel=yes
Remapable=yes
PrimaryFireFLH=30,43,92 ; offset for take off -- position for missile voxels
```

### 2.1 Key-by-key

| Key | Value | Notes |
|-----|-------|-------|
| `Cameo=DREDICON` | SHP | Build-list cameo. |
| `Voxel=yes` | bool | Renders as a voxel (.vxl + .hva pair: `dred.vxl`, `dred.hva`). With `NoSpawnAlt=yes` also looks for `DREDWO.vxl` for the empty state. |
| `Remapable=yes` | bool | House color recolors the body. |
| `PrimaryFireFLH=30,43,92` | x,y,z leptons | **F**iring **L**ocation **H**eight — the world-space offset (forward 30, right 43, up 92 leptons relative to ship origin) where the spawned DMISL appears at launch. The inline comment confirms: "offset for take off -- position for missile voxels". Used by `SpawnManagerClass` when seating a freshly-regenerated missile on the launcher. |

> **No Voxel.Sequence or Animation block in artmd[DRED].** The missile-tilt animation is driven *programmatically* by `RocketLocomotionClass` and the `RocketStruct` fields (`DMislPauseFrames`, `DMislTiltFrames`, `DMislPitchInitial`, `DMislPitchFinal`) read from Rules — not a frame-by-frame SHP/HVA sequence. See `SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md` §3 for the RocketStruct table.

---

## 3. Weapons

### 3.1 `[DredLauncher]` — virtual launcher

```ini
[DredLauncher]
Damage=50
ROF=50
Burst=2
Range=25
;Range=-2
MinimumRange=8; the missiles need time to align
Spawner=yes
Projectile=InvisibleHigh
Speed=15
Warhead=Special
OmniFire=yes
```

| Key | Effect |
|-----|--------|
| `Damage=50` | **Cosmetic on a Spawner weapon** — the actual damage comes from the spawned DMISL's hardcoded warhead (see §3.3). The 50 is a placeholder. |
| `ROF=50` | Frames between fire commands — 50 frames at 15 fps = ~3.3 sec per salvo cycle. |
| `Burst=2` | **2 spawn-launches per fire command** — that is, fires both missiles together (matches `SpawnsNumber=2`). |
| `Range=25` | 25 cells maximum (radar-strike-from-coast range; among the longest in the game). |
| `;Range=-2` | (commented) — legacy "infinite/automatic" range, disabled. |
| `MinimumRange=8` | **Cannot fire at targets closer than 8 cells** — missiles need flight time to align (inline comment confirms). Use a destroyer/Sea Scorpion to defend. |
| `Spawner=yes` | **The fire action releases a unit from the parent's SpawnManager** rather than emitting a projectile from this weapon. Tied to `[DRED] Spawns=DMISL`. |
| `Projectile=InvisibleHigh` | Bookkeeping projectile — invisible, used to satisfy the projectile-required field; the actual visual is the spawned DMISL. |
| `Speed=15` | Speed of the invisible carrier projectile — irrelevant in practice. |
| `Warhead=Special` | **Misleading** — `Special` is a *house* (`[Special]` country block at rulesmd:3335), not a warhead. Since this is a Spawner weapon, no warhead actually detonates from the launcher itself. The damage warhead applied at the missile's impact is the Rules-global `DMislWarhead` / `DMislEliteWarhead` (see §3.3). |
| `OmniFire=yes` | Can fire in any direction without rotating the hull (relevant because `Turret=no`). |

### 3.2 Spawned aircraft `[DMISL]`

```ini
[DMISL]
UIName=Name:DMISL
Name=Dread Missile
FireAngle=1
Strength=50
Category=AirPower
Armor=special_2
Spawned=yes
MissileSpawn=yes
TechLevel=-1
Sight=0
RadarInvisible=no
Landable=yes
MoveToShroud=yes
Ammo=1 ;Aircraft are hard wired to require ammo
Speed=18;20
Owner=Russians,Confederation,Africans,Arabs
Cost=50
Points=20
ROT=4
Crewed=no
Explodes=no
GuardRange=30
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
MaxDebris=2
Locomotor={B7B49766-E576-11d3-9BD9-00104B972FE8}
MovementZone=Fly
ThreatPosed=10	; This value MUST be 0 for all building addons
DamageParticleSystems=SmallGreySSys	; Sparks don't work well here.  SJM
AuxSound1=DreadnoughtAttack ;Taking off
;AuxSound2=DROPDWN1 ;Landing
ImmuneToPsionics=yes
;VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
;EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
NoShadow=yes
Selectable=no
Trainable=no
FlyBack=true
DontScore=yes
```

| Key | Notes |
|-----|-------|
| `Spawned=yes` | TechnoType @ 0x00714e7d — only created by a `Spawner=yes` weapon, never directly buildable. |
| `MissileSpawn=yes` | TechnoType @ 0x00714f23 — **classifies this spawn as a missile** (vs a Hornet/ASW that fly home). Combined with `SpawnReloadRate=0` on the parent, the missile is destroyed at impact instead of returning. Also gates the RocketStruct path in SpawnManagerClass (see SPAWN_MANAGER_CLASS_GHIDRA_REPORT §3 / §4). |
| `TechLevel=-1` | Hidden from build list (only obtainable via spawn). |
| `Sight=0` | No vision contribution while in flight. |
| `Landable=yes` | Marked as a landable aircraft — but `SpawnReloadRate=0` overrides; it never lands back. |
| `MoveToShroud=yes` | TechnoType @ 0x00713f4b? (cf cheat sheet — actually this is TechnoType-level). Allows the missile to fly through shrouded cells to reach its target. Without this, the missile would refuse to enter unexplored map cells. |
| `Ammo=1` | Inline comment: "Aircraft are hard wired to require ammo". One shot. |
| `Speed=18;20` | Missile flight speed (cells/sec scaled internal units). The `;20` is a commented original value — current live = 18. |
| `Owner=Russians,Confederation,Africans,Arabs` | Mirrors parent. |
| `Cost=50` | Spawn-replenishment cost? Actually for missile spawns this is largely cosmetic — `SpawnRegenRate=80` on the parent regenerates ammo for free; cost shows in inferred kill-score economics. |
| `ROT=4` | In-flight steering. Combined with the hardcoded `DMislTurnRate=0.08` from Rules, dictates pitch maneuverability. |
| `Explodes=no` | TechnoType @ 0x007122c5 — does NOT detonate its own weapon on death. Avoids double-detonation. The hardcoded path applies `DMislWarhead` once on impact. |
| `Locomotor={B7B49766-E576-11d3-9BD9-00104B972FE8}` | **RocketLocomotionClass** — see [`ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`](../../ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md). Implements tilt→takeoff→cruise→dive phases. RocketStruct lookup uses `Rules.DMislType==DMISL` to select the DMisl* parameter block. |
| `MovementZone=Fly` | Pathfinder zone — air. |
| `AuxSound1=DreadnoughtAttack` | The launch-takeoff sound (`vdreatta/b` random, FShift -10/+10, Limit 2, Volume 60). |
| `;AuxSound2=DROPDWN1` | (commented) — would have been a landing sound; disabled. |
| `ImmuneToPsionics=yes` | TechnoType @ 0x00714fa7 — Yuri & Yuri Prime cannot mind-control the missile in flight. |
| `;VeteranAbilities…/;EliteAbilities…` | (commented) — the missile itself does not rank up (`Trainable=no`); veteran/elite stats belong to the parent Dread. |
| `NoShadow=yes` | Skip ground-shadow render (it's high up). |
| `Selectable=no` | Cannot be box-selected mid-flight. |
| `Trainable=no` | Does not gain veterancy. |
| `FlyBack=true` | Marker that the spawn would fly back — but **SpawnReloadRate=0 overrides**, so the flag is effectively ignored. (Compare HORNET, where FlyBack drives the return-and-dock loop.) |
| `DontScore=yes` | TechnoType @ 0x00713f4b — destroying a missile in flight does not award points to the killer. |

### 3.2.1 `[DMISL]` artmd (verbatim)

```ini
[DMISL]
SpawnDelay=2;1
Voxel=yes
Remapable=no
CanBeHidden=no
```

- `SpawnDelay=2` — frame delay between back-to-back spawns from a single launcher (so a `Burst=2` salvo doesn't draw both missiles simultaneously on top of each other; the second appears 2 frames after the first).
- `Voxel=yes` — voxel render (`dmisl.vxl` + `.hva`).
- `Remapable=no` — fixed grey body, no house tint.
- `CanBeHidden=no` — render even in shroud.
- **No `PrimaryFireFLH=`** — DMISL does not itself emit a projectile; it *is* the projectile.

### 3.3 Impact warhead — hardcoded

`DMISL` has **no `Primary=` weapon and no `Warhead=`** in its rulesmd block. The damage it applies at impact is controlled by two Rules globals from `[CombatDamage]` (rulesmd:820-822):

```ini
DMislWarhead=DMISLWH        ; this is the warhead on a DredMissile
DMislEliteWarhead=DMISLEWH  ; this is the warhead on a DredMissile when the launcher is elite
```

**Ghidra confirmation:**
- String `DMislWarhead` @ `0x0083b1a8` → xref'd from `RulesClass__ReadCombatDamage` @ `0x0066c3db`; stored into Rules field `+0xfb4`.
- String `DMislEliteWarhead` @ `0x0083b184` → xref'd from `RulesClass__ReadCombatDamage` @ `0x0066c458`; stored into Rules field `+0xfbc`.
- Both are read via `WarheadTypeClass__FindOrAllocate()` — i.e., they resolve to global Warhead pointers, not per-type warheads.

> **Pattern note (same as V3):** The veteran/elite warhead swap is *parent-veterancy*-based — Rules consults the launching Dreadnought's rank at the moment of fire and selects DMislWarhead (rookie+veteran) or DMislEliteWarhead (elite). Missile-in-flight veterancy is irrelevant since the missile itself is `Trainable=no`.

#### 3.3.1 `[DMISLWH]` (standard impact)

```ini
[DMISLWH]
CellSpread=1.5
PercentAtMax=.25
Wall=yes
Wood=yes
Verses=100%,90%,80%,100%,80%,80%,85%,65%,28%,80%,0%
Conventional=yes
Rocker=no
InfDeath=2
AnimList=XGRYSML1,XGRYSML2,EXPLOSML,XGRYMED1,XGRYMED2,EXPLOMED,EXPLOLRG,TWLT070
Deform=10%
DeformThreshhold=300
Tiberium=yes
Sparky=no
Bright=yes
ProneDamage=70%     ; Presumes air burst
```

- `CellSpread=1.5` — 1.5-cell explosion radius.
- `PercentAtMax=.25` — damage at the edge of CellSpread is 25 % of damage at center; linear falloff in between.
- `Verses=100%,90%,80%,100%,80%,80%,85%,65%,28%,80%,0%` — damage vs each armor class:
    | idx | Armor | % |
    |-----|-------|---|
    | 0 | none | 100% |
    | 1 | flak (infantry) | 90% |
    | 2 | plate (infantry, vet) | 80% |
    | 3 | light (light vehicle) | 100% |
    | 4 | medium (medium vehicle) | 80% |
    | 5 | heavy (heavy vehicle) | 80% |
    | 6 | wood (light building) | 85% |
    | 7 | steel (medium building) | 65% |
    | 8 | concrete (heavy building) | 28% |
    | 9 | special_1 | 80% |
    | 10 | special_2 (DMISL self) | 0% |
  → Excellent vs all vehicle classes and infantry, mediocre vs concrete, **0 % vs other DMISL** (cannot chain-kill in mid-air).
- `Wall=yes`, `Wood=yes` — destroys walls and wood props.
- `Conventional=yes` — counts as non-special (no special-immunity protection).
- `Rocker=no` — does not produce a screen shake.
- `InfDeath=2` — infantry hit by this die using **death anim type 2** (per the InfDeath cheat sheet: small-arms isn't actually 2 — 2 is "burnt" in some lists; this matches the standard explosion infantry death frames for "no special death"). The `2` here matches V3HE which uses InfDeath=2 too.
- `AnimList=XGRYSML1,XGRYSML2,EXPLOSML,XGRYMED1,XGRYMED2,EXPLOMED,EXPLOLRG,TWLT070` — random-pick impact animation from this 8-entry list.
- `Deform=10%` / `DeformThreshhold=300` — 10 % chance per impact ≥ 300 damage to deform the terrain (crater the ground cell).
- `Tiberium=yes` — can detonate ore deposits as secondary chain reaction.
- `Sparky=no` — does not throw sparks separately (the AnimList covers visual).
- `Bright=yes` — emits a momentary lighting flash on impact.
- `ProneDamage=70%` — infantry in **prone** posture (deployed GI/Conscript) take 70 % of normal damage. Inline comment "Presumes air burst" explains why prone reduces damage: airburst shrapnel rains down; prone bodies present less profile.

#### 3.3.2 `[DMISLEWH]` (elite-launcher impact)

```ini
[DMISLEWH]
CellSpread=3
PercentAtMax=.5
Wall=yes
Wood=yes
Verses=100%,90%,80%,100%,80%,80%,85%,65%,28%,80%,0%
Conventional=yes
Rocker=no
InfDeath=2
;AnimList=XGRYSML1,XGRYSML2,EXPLOSML,XGRYMED1,XGRYMED2,EXPLOMED,EXPLOLRG,TWLT070
AnimList=MININUKE
Tiberium=yes
Sparky=no
Bright=yes
ProneDamage=70%     ; Presumes air burst
```

Differences vs `DMISLWH`:
- `CellSpread=3` (vs 1.5) — **2× the radius** at elite.
- `PercentAtMax=.5` (vs .25) — **edge damage doubled** (50 % falloff vs 25 %). Combined with double radius, total damaged area is ~4× and edge cells take 2× as much.
- `AnimList=MININUKE` — uses the mini-nuke anim ("mininuke" SHP) — distinct dramatic flash and mushroom-fragment visual.
- No `Deform=` block — *commented out the original AnimList, replaced with MININUKE; deform behavior reverts to default (which is 0/off in absence of the key, but MININUKE animation has its own visual crater)*.

### 3.4 Per-missile damage values (Rules global, not on warhead)

```ini
; from [General] block, rulesmd:172-184
DMislPauseFrames=20
DMislTiltFrames=60
DMislPitchInitial=0
DMislPitchFinal=0.5
DMislTurnRate=0.08
DMislRaiseRate=1
DMislAcceleration=0.8
DMislAltitude=768
DMislDamage=300        ; Exploding DMisl does this much damage
DMislEliteDamage=600   ; Exploding DMisl does this much damage (elite launcher)
DMislBodyLength=128
DMislLazyCurve=no      ; The rocket's path is a big, lazy curve.  V3 yes.  DMisl no.
DMislType=DMISL
```

These all bind into the `RocketStruct` slot in Rules (see `SPAWN_MANAGER_CLASS_GHIDRA_REPORT` §3, §6, §12). Key player-visible deltas vs V3:
- `DMislPitchInitial=0` (horizontal start vs V3's 0.21 — V3 starts at ~12° above horizontal). The Dread missile lies flat on the deck before tilting.
- `DMislTurnRate=0.08` (vs V3's 0.05 — DMisl turns ~60 % faster mid-flight, can track moving targets better).
- `DMislAcceleration=0.8` (vs V3's 0.4 — DMisl accelerates 2× harder).
- `DMislLazyCurve=no` (V3 is `yes`) — DMisl flies a tight ballistic arc directly to target; V3 takes a long sweeping curve.
- **`DMislDamage=300` / `DMislEliteDamage=600`** — base damage the warhead applies (multiplied by Verses[] vs target armor).

> Verified Ghidra: strings `DMislDamage` (0x0083b9f0), `DMislEliteDamage` (0x0083b9dc), `DMislBodyLength` (0x0083b9cc), `DMislLazyCurve` (0x0083b9bc), `DMislType` (0x0083b9b0), `DMislAltitude` (0x0083b9fc), `DMislAcceleration` (0x0083ba0c), `DMislRaiseRate` (0x0083ba20), `DMislTurnRate` (0x0083ba30), `DMislPitchFinal` (0x0083ba40), `DMislPitchInitial` (0x0083ba50), `DMislTiltFrames` (0x0083ba64), `DMislPauseFrames` (0x0083ba74) — all 15 strings present. Cross-reference details in `SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md` §6 (verified RocketStruct field offset table).

### 3.5 Salvo timing (player-visible)

Combining `[DredLauncher]` Burst=2, ROF=50, `[DRED]` SpawnsNumber=2, SpawnRegenRate=80, and `[General]` DMislPauseFrames=20+DMislTiltFrames=60:

| Tick (frames) | Event |
|---|---|
| 0 | Fire command issued; SpawnManager releases missile 1 |
| 2 | (`SpawnDelay=2`) — missile 2 released |
| 20 | DMislPauseFrames elapses — missile begins tilting |
| 80 | DMislTiltFrames elapses (20+60) — missile fires off the rack and enters RocketLocomotion cruise; rack visually empties |
| 80…X | Missile flies to target (depending on distance / Altitude profile / TurnRate) |
| X | Missile impacts → `DMislWarhead` detonates → missile entity destroyed (SpawnReloadRate=0) |
| X+80 | (SpawnRegenRate=80 from time of consumption) — replacement missile materializes in rack |
| 50 (ROF from previous fire) | Next fire command earliest — but blocked while no missile in rack |

In practice, after a salvo of 2 the ship is missile-empty for ~80 frames (~5.3 sec at 15 fps) before reloading one missile, then another 80 for the second. Sustained DPS is therefore strongly back-loaded — the burst is devastating but rate-limited.

---

## 4. Voice & sound catalogue

| Slot | Sound key | sndmd entry | Audio clip(s) |
|------|-----------|-------------|---------------|
| `VoiceSelect` | `GenSovWaterSelect` | sound:4206 | `$vwassea $vwasseb $vwassec` (random) |
| `VoiceMove` | `GenSovWaterMove` | sound:4201 | `$vwasmoa $vwasmob $vwasmoc $vwasmod` (random) |
| `VoiceAttack` | `GenSovWaterAttackCommand` | sound:4196 | `$vwasata $vwasatb $vwasatc` (random) |
| `VoiceFeedback` | (empty) | — | — |
| `DieSound` | (empty) | — | — (silent on death — sink takes over) |
| `SinkingSound` | `GenLargeWaterDie` | sound:1979 | `gnavsina` (predelay-interrupt, Limit=2, Volume=85) |
| `MoveSound` | `DreadnoughtMoveStart` | sound:1435 | `vdrestaa vdrestab vdrestac` (random predelay, Priority=low, Delay 0-400, FShift -5/+5, VShift 20, Volume 50) |
| (on missile launch) `[DMISL] AuxSound1` | `DreadnoughtAttack` | sound:1428 | `vdreatta vdreattb` (random interrupt, FShift -10/+10, Limit 2, Volume 60) |

All three "GenSovWater*" voice sets are shared with other Soviet naval combatants (Typhoon Sub, Sea Scorpion) — same `$vwa*` files. `gnavsina` is the universal large-ship sinking groan (also used by Carrier, Aegis when they replace it via dual-read of SinkingSound).

---

## 5. Owners / prerequisites / tech gating

- **Buildable by:** `Russians`, `Confederation`, `Africans`, `Arabs` — i.e., the four Soviet country IDs in standard rulesmd ([Countries] entries — see [`HARV.md`](./HARV.md) §5 for full Soviet country mapping).
- **NOT buildable by:** YuriCountry, French, Germans, British, Americans, Alliance (Korea).
- **Prerequisite:** `NAYARD,NATECH` — both Soviet Naval Yard AND Soviet Battle Lab. Yuri can't build it (Yuri has YAYARD/YATECH instead).
- **TechLevel:** 6 — equivalent to top-tier tech (same as Apocalypse, Kirov, V3).
- **CrateGoodie=no** → cannot pop from goodie crates.
- **AllowedToStartInMultiplayer=no** → not in pre-built starting roster.

---

## 6. Veterancy

| Rank | Effect |
|------|--------|
| Rookie | Base — `DMislDamage=300` warhead, `DMISLWH` (1.5-cell radius) |
| Veteran | `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` — Cost-bonus HP, +damage, +ROF, +Sight, +Speed. Still applies `DMISLWH` (rookie warhead — elite swap only at elite rank). |
| Elite | `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` — adds passive self-heal (auto-regen); +damage/+ROF stack with veteran. **Missile impact switches to `DMislEliteDamage=600` / `DMISLEWH` warhead (3-cell radius, mini-nuke anim).** Note: no SIGHT/FASTER upgrade at elite. |

> The elite warhead swap is checked at the moment of fire against the *launcher's* rank — not the missile's (missile is Trainable=no).

---

## 7. Hardcoded behavior — Ghidra-verified

### 7.1 Confirmed unit-/family-specific code paths

| Behavior | Hardcoded? | Evidence |
|----------|-----------|----------|
| **Dreadnought as a `Spawner=yes` weapon parent** | Generic (SpawnManagerClass) | The Dread shares this pattern with V3 and Boomer Sub. SpawnManagerClass identifies "missile spawner" via `IsMissileSpawn` flag (struct +0x14) set when `SpawnType ∈ {V3RocketType, DMislType, CMislType}`. See `SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md` §4 (struct layout) and §3 (RocketStruct selection). |
| **DMisl-specific flight profile (pause/tilt/pitch/curve)** | Yes (RocketStruct) | Rules holds three independent RocketStruct blocks (V3 / DMisl / CMisl). At missile launch, `RocketLocomotionClass::Process` picks the DMisl block by matching `Owner.TypeClass → DMislType==DMISL`. Per `SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md` §3 the slot index is determined per-spawn. |
| **`DMislLazyCurve=no` → tight arc vs V3's big curve** | Yes | RocketStruct boolean controls a branch in the cruise-state pathing of RocketLocomotionClass. See `ROCKET_LOCOMOTION_CLASS_GHIDRA_REPORT.md`. |
| **`DMislDamage=300` applied via `DMislWarhead=DMISLWH`** | Yes | `RulesClass__ReadCombatDamage` @ 0x0066c3db reads `DMislWarhead` from CombatDamage section into Rules+0xfb4. At missile impact, the hardcoded missile-detonate path looks up Rules+0xfb4 (or +0xfbc for elite) and calls the warhead's Detonate on the target cell, with damage = `Rules.DMislDamage` × Verses[target_armor]. |
| **Elite warhead/damage swap on launcher elite rank** | Yes | Inline launcher-rank check at fire time selects between Rules+0xfb4/+0xfbc and `DMislDamage`/`DMislEliteDamage`. Same pattern as V3 (V3.md §7). |
| **`NoSpawnAlt=yes` → swap to DREDWO voxel when out of missiles** | Yes (ObjectType-scope) | `ObjectTypeClass__ReadINI` @ 0x005f943e reads the flag into the ObjectType (broader than TechnoType). The render path checks SpawnManager.RemainingSpawns and selects `<ID>WO` voxel when 0 + flag set. Verified live string `NoSpawnAlt` @ 0x00832bc0. |
| **`Spawns=DMISL` + `SpawnsNumber=2` + `SpawnReloadRate=0`** | Generic SpawnManager flags | Verified TechnoType scope (cheat sheet): SpawnsNumber @ 0x00714ee1, SpawnRegenRate @ 0x00714ec0, SpawnReloadRate @ 0x00714f02, Spawned @ 0x00714e7d, MissileSpawn @ 0x00714f23. |
| **`FireAngle=32`** | TechnoType @ 0x00714b5d | Verified TechnoType. Used by missile-launch initial velocity vector. |
| **`MinimumRange=8` on launcher** | Generic WeaponType | Standard WeaponType field. Live and enforced. |
| **`Burst=2` on launcher** | Generic WeaponType | Standard WeaponType field. With Spawner=yes, the Burst count drives how many spawns are released per fire command. |
| **`OmniFire=yes` on launcher** | Generic WeaponType | Standard WeaponType — bypasses hull-facing requirement (the Dread doesn't have a turret). |
| **`ToProtect=yes`** | TechnoType @ 0x00714be8 | Verified TechnoType scope. AI uses this flag to dispatch escort defenders. |
| **`SinkingSound=GenLargeWaterDie`** | DUAL-READ — Rules @ 0x006699a7 + TechnoType @ 0x00712fb0 | Per-unit override of the global default. Per Audio system docs, played once when a sinking unit transitions to drowning state. |
| **`TooBigToFitUnderBridge=true`** | UnitType-scope @ 0x0074774e | UnitType, not TechnoType. Blocks pathfinder from routing this unit through cells under bridge spans. |
| **`Locomotor={2BEA74E1-...}` = ShipLocomotionClass** | Live YR locomotor | Cross-ref `BRIDGE_LOCOMOTOR_DRIVE_SHIP_GHIDRA_REPORT.md`. |

### 7.2 NO unit-name string scan match

A Ghidra `search_strings` for `DREDWO`, `DredImpact` returned **0 matches** — the engine has **no hardcoded string-name references to `DRED`** beyond the auto-derived `WO` suffix logic (which is built at runtime from the unit's ID + literal "WO" string via the NoSpawnAlt code path).

A Ghidra `search_strings` for `DredLauncher` returned **0 matches** — confirming the weapon name is *only* an INI binding; the engine doesn't special-case this weapon by name.

A Ghidra `search_strings` for `DMisl` returned **15 matches** — all are the `[General]` rules keys (DMislWarhead, DMislEliteWarhead, DMislType, DMislLazyCurve, DMislBodyLength, DMislEliteDamage, DMislDamage, DMislAltitude, DMislAcceleration, DMislRaiseRate, DMislTurnRate, DMislPitchFinal, DMislPitchInitial, DMislTiltFrames, DMislPauseFrames). **No `[DMISL]` aircraft-section keys are referenced by name in the binary** — that section is read generically by AircraftTypeClass.

> **Bottom line:** the Dreadnought has **no hardcoded behavior keyed to its INI ID**. All its quirks come from generic mechanisms (SpawnManager + RocketLocomotion + ShipLocomotion + standard weapon firing) driven by RocketStruct slot DMisl* which it happens to be wired to via `Spawns=DMISL`. The only DRED-specific knobs are INI values, not code branches.

---

## 8. TS-legacy filter (Tiberian Sun ghosts)

| Feature | Status in YR |
|---------|--------------|
| `Locomotor={4A582741-...}` second GUID after `;` | **INI comment** — disabled, not loaded. DriveLocomotionClass would have made it a land vehicle. |
| `;ForbiddenHouses=Russians` | INI comment — inert. |
| `;BuildLimit=1` | INI comment — inert (no per-player limit). |
| `;OmniFire=yes` on the techno block | INI comment — OmniFire is set on the weapon `[DredLauncher]` instead. |
| `Conventional=yes` on warhead | Conventional-damage flag, fully live in YR. |
| `Tiberium=yes` on warhead | **Note: RA2 has no Tiberium**. This flag is a TS holdover that *technically* still drives ore-vein chain explosions in YR — but with no Tiberium and ore-vein detonation behavior intact, this just means the warhead can detonate ore-clusters as a chain reaction. The flag is live, but its name is misleading. |
| Fog-of-war shroud reset (0x1000) | Not gated by this unit. |
| `ImmuneToVeins` | Not on DRED. Naval unit — n/a. |
| `Subterranean` / tunneling | Not on DRED. |

---

## 9. Coverage audit

| Section | Coverage |
|---------|----------|
| rulesmd `[DRED]` — every key | ✅ §1 (41 keys, all enumerated incl. commented ones) |
| artmd `[DRED]` — every key | ✅ §2 (4 active keys + inline comment) |
| `[DredLauncher]` weapon | ✅ §3.1 (10 keys) |
| `[DMISL]` aircraft section | ✅ §3.2 (28 keys + commented entries) |
| `[DMISL]` artmd | ✅ §3.2.1 (4 keys) |
| `[DMISLWH]` warhead | ✅ §3.3.1 (16 keys) |
| `[DMISLEWH]` warhead | ✅ §3.3.2 (15 keys + diff vs standard) |
| Rules `[General]` DMisl* keys (13 keys) | ✅ §3.4 |
| Voices / sounds (8 slots) | ✅ §4 |
| Owners / prereqs / tech | ✅ §5 |
| Veterancy | ✅ §6 |
| Hardcoded behavior — Ghidra-verified | ✅ §7 (1 verified ID-scan, 1 family-scan, 14 individual key scope verifications cross-ref'd to cheat sheet) |
| TS-legacy filter | ✅ §8 |
| Cross-references | ✅ at top + inline |

---

## 10. Quick implementer summary

To make a DRED-equivalent unit in this engine:

1. **Render** — voxel + HVA pair; on `NoSpawnAlt=yes` and `SpawnManager.RemainingSpawns==0`, swap to `<ID>WO` voxel.
2. **Movement** — ShipLocomotionClass (water-only, Speed=4, ROT=1, TooBigToFitUnderBridge gate).
3. **Spawner** — generic SpawnManager state machine with N=`SpawnsNumber` missile slots, regen `SpawnRegenRate` frames per slot, no physical reload (`SpawnReloadRate=0` = MissileSpawn pattern: missile dies on impact, regen replaces it).
4. **Fire** — `Burst=N` salvo, `ROF`-throttled, `MinimumRange` enforced, `OmniFire` allows fire without hull rotation, `FireAngle=32` sets initial launch pitch.
5. **Missile flight** — RocketLocomotionClass with the *DMisl* RocketStruct slot (Pause→Tilt→Cruise→Dive). See `SPAWN_MANAGER_CLASS_GHIDRA_REPORT.md` §3 for the full 12-field block.
6. **Impact** — apply Rules global `DMislWarhead` (or `DMislEliteWarhead` if launcher is elite) at target cell, with damage = `DMislDamage` (or `DMislEliteDamage` if elite) × Verses[armor] × falloff(CellSpread, PercentAtMax).
7. **Veterancy upgrades** — STRONGER/FIREPOWER/ROF/SIGHT/FASTER at vet; SELF_HEAL/STRONGER/FIREPOWER/ROF at elite (+ swap to elite warhead path on fire).
8. **Audio** — generic Soviet water voice set + DreadnoughtMoveStart on movement + GenLargeWaterDie on sinking (dual-read pattern: per-techno overrides Rules global) + per-missile DreadnoughtAttack on launch.
9. **AI** — `ToProtect=yes` flags this for escort dispatch; `ThreatPosed=25` weights threat evaluation.

No DRED-specific code paths are required — only correct wiring of generic systems and respect for the RocketStruct DMisl* parameter block.
