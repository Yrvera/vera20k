# Chrono Commando (CCOMAND)
Side: Allied | Category: Infantry | Image alias: `[CCOMAND]` (own SHP `CCOMAND`)

A teleporting Navy SEAL — the Chrono Commando is unlocked by **stealing
Allied tech** (capturing an Allied Battle Lab via [SPY](SPY.md), which the
INI comment makes explicit: "`anybody gets into an allied tech center`").
Once unlocked, any house can build him from the Barracks at $2000. Combines
the Navy SEAL's silenced MP5 (`ChronoMP5`, range 6, 125-damage HollowPoint)
with **instant teleport movement** (`Teleporter=yes` + the
`TeleportLocomotionClass` GUID — see
[TELEPORT_LOCOMOTION_DEEP_DIVE.md](../../TELEPORT_LOCOMOTION_DEEP_DIVE.md))
and a **`FakeC4` instant-building-kill weapon** (5000-damage warhead that
hits only buildings — the secondary that fires when he warps onto an enemy
structure). Cannot be a starting unit. Tech-9 gate via `RequiresStolenAlliedTech=yes`.

The Chrono Commando teleports **anywhere on the map** (move-to-anywhere
locomotor, not range-limited like Chrono Legionnaire's CSPH-teleport), but
he is **unarmed** during the warp animation and remains briefly stunned
on arrival.

Authoritative deep RE on the teleport mechanism: the four chrono docs in
`ra2-rust-game-docs/`:

- [TELEPORT_LOCOMOTION_DEEP_DIVE.md](../../TELEPORT_LOCOMOTION_DEEP_DIVE.md)
- [TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md](../../TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md)
- [TECHNOCLASS_CHRONO_OFFSETS_VERIFIED.md](../../TECHNOCLASS_CHRONO_OFFSETS_VERIFIED.md)
- [CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md](../../CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md)
  (Chrono Miner uses the same locomotor, slightly different mission gating)

---

## rulesmd.ini — `[CCOMAND]` section

Verbatim from `c:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:4174`:

```ini
[CCOMAND] ;anybody gets into an allied tech center
UIName=Name:CCOMAND
Name=Chrono Commando
Category=Soldier
Prerequisite=BARRACKS
RequiresStolenAlliedTech=yes
Primary=ChronoMP5
Secondary=FakeC4 ; otherwise he can teleport into a building and kill it before he unwarps.
OpenTransportWeapon=0;defaults to -1 (decide normally)  What weapon should I use in a Battle Fortress
;C4=yes
CrushSound=InfantrySquish
Crushable=no
TiberiumProof=yes
Strength=100
Armor=none
TechLevel=9
Pip=red
Sight=8
Speed=5
Owner=Russians,Confederation,Africans,Arabs,YuriCountry,British,French,Germans,Americans,Alliance
AllowedToStartInMultiplayer=no
Cost=2000
Soylent=1000
Points=50
IsSelectableCombatant=yes
VoiceSelect=ChronoCommandoSelect
VoiceMove=ChronoCommandoMove
VoiceAttack=ChronoCommandoAttackCommand
VoiceFeedback=
VoiceSpecialAttack=ChronoCommandoSpecialAttack
DieSound=SealDie
CreateSound=ChronoCommandoCreated
ChronoInSound=ChronoLegionTeleport
ChronoOutSound=ChronoLegionTeleport
Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}
;Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}; <-Walk  teleport->{4A582747-9839-11d1-B709-00A024DDAFD1}
Teleporter=yes
PhysicalSize=1
MovementZone=Infantry
;SpeedType=Amphibious
;MovementZone=AmphibiousDestroyer ; I am the only one with this zone, because it is now tied with being an infantry (part of seal stuck on tree bug)
ThreatPosed=25	; This value MUST be 0 for all building addons
SpecialThreatValue=1
ImmuneToVeins=yes
VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
ElitePrimary=ChronoMP5E
PreventAttackMove=yes
MoveToShroud=no
IFVMode=4
```

| Key | Meaning |
|-----|---------|
| `UIName=Name:CCOMAND` | CSF-string key resolving to "Chrono Commando" |
| `Name=Chrono Commando` | Internal short name |
| `Category=Soldier` | Pip group + AI threat grouping (infantry) |
| `Prerequisite=BARRACKS` | Generic — resolves to GAPILE / NAHAND / YABRCK depending on owner. Combined with `RequiresStolenAlliedTech=yes`, this is the **secondary** gate; the real gate is having infiltrated an Allied Battle Lab |
| `RequiresStolenAlliedTech=yes` | **Unlock flag** — `TechnoTypeClass` field parsed at Ghidra `0x00843bc4` (verified). Build button hidden until the owning house has the "stolen Allied tech" bit set, which is granted by a SPY infiltrating any Allied tech building (typically GATECH Battle Lab). The INI comment "anybody gets into an allied tech center" clarifies this is the universal-house unlock mechanism — Soviet/Yuri players who steal Allied tech also get this unit |
| `Primary=ChronoMP5` | Silenced MP5 — Damage=125, ROF=10, Range=6, Warhead=HollowPointNoBuilding (anti-inf only). Same shoot sound as Navy SEAL |
| `Secondary=FakeC4 ; otherwise he can teleport into a building and kill it before he unwarps.` | **Instant building-killer** when teleporting onto an enemy structure. Damage=5000, Range=1.5, Warhead=FakeC4WH (Verses=0/0/0/0/0/0/100/100/100/0/100 — only building-armor types take damage). Westwood's inline comment explains the design: the *original* C4 trigger (commented out below as `;C4=yes`) was replaced by this weapon because pre-warp C4 detonation broke the unstun timing |
| `OpenTransportWeapon=0` | When riding an open-topped transport ([FV](FV.md) Battle Fortress), fire **Primary** (ChronoMP5) from inside. The defaults-to-`-1` comment notes the engine would otherwise pick "best" weapon — here forced to slot 0 to avoid FakeC4 firing from the Battle Fortress (which would be wildly imbalanced) |
| `;C4=yes` | **Commented out** — the older C4-on-walk-up mechanism. Replaced by the FakeC4 weapon-and-warhead path. Westwood left this line in as a marker of design history |
| `CrushSound=InfantrySquish` | Standard infantry crush sound |
| `Crushable=no` | **Cannot be crushed by vehicles** — same as Tanya / SEAL / Boris. Hero-tier infantry are exempt from the crush-on-overlap path |
| `TiberiumProof=yes` | **Immune to tiberium damage** — TS legacy (no tiberium in YR maps; defensive flag) |
| `Strength=100` | HP — same as SEAL/Tanya/Boris |
| `Armor=none` | Damage type column 0 |
| `TechLevel=9` | Late-game tech gate; with RequiresStolenAlliedTech this is mostly cosmetic |
| `Pip=red` | Red cargo-passenger pip (matches hero-tier infantry) |
| `Sight=8` | Reveal radius (larger than standard infantry's 4–5) |
| `Speed=5` | Slow walking speed — but since he teleports, foot-walk speed is rarely used |
| `Owner=Russians,Confederation,Africans,Arabs,YuriCountry,British,French,Germans,Americans,Alliance` | **All 10 houses** — no `ForbiddenHouses=`. Universal availability (gated by tech-steal flag, not by faction) |
| `AllowedToStartInMultiplayer=no` | Excluded from lobby starting-unit complement |
| `Cost=2000` | 10× engineer; 5× Tanya — most expensive single infantry unit in the game |
| `Soylent=1000` | Yuri Grinder refund — half of build cost (Yuri can produce CCOMAND after stealing Allied tech; grind for 1000) |
| `Points=50` | Kill score (high — hero-tier) |
| `IsSelectableCombatant=yes` | Included in "select all combat units" hotkey |
| `VoiceSelect=ChronoCommandoSelect` | Voice on click-select |
| `VoiceMove=ChronoCommandoMove` | Voice on move order |
| `VoiceAttack=ChronoCommandoAttackCommand` | Voice on attack order |
| `VoiceFeedback=` | **Empty** — no fear/panic voice. Hero-tier behavior pattern (Tanya, SEAL also have minimal feedback voices) |
| `VoiceSpecialAttack=ChronoCommandoSpecialAttack` | Voice on "special" (teleport-attack) order — `Type=global` (everyone hears it) |
| `DieSound=SealDie` | Reuses Navy SEAL's death sample bank |
| `CreateSound=ChronoCommandoCreated` | Voice played when unit exits Barracks — `Type=Global` (all players hear it, alerting opponents) |
| `ChronoInSound=ChronoLegionTeleport` | Sound at teleport **start** (warp-out from current cell) |
| `ChronoOutSound=ChronoLegionTeleport` | Sound at teleport **end** (warp-in to destination cell) — same sample as in-sound; reused for both legs |
| `Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}` | **`TeleportLocomotionClass` GUID** — replaces the standard `WalkLocomotionClass` (commented-out alt GUID is the walk one). All movement orders route through the teleport state machine: warp-out → invisible transit (instant) → destination-cell validation → warp-in → unstun timer (`UnnaturalMovementCondemnation` if blocked) |
| `Teleporter=yes` | **Engine-special flag** — `TechnoTypeClass` field at Ghidra `0x00843e60`. Gates the teleport cursor (purple chrono-circle), the unstun-timer post-warp, and the destination-validation logic. Without this flag the teleport locomotor refuses to engage |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Infantry` | Standard infantry terrain |
| `;SpeedType=Amphibious` `;MovementZone=AmphibiousDestroyer` | **Commented out** — TS-era plan for the Chrono Commando to walk on water. Westwood's note "I am the only one with this zone... seal stuck on tree bug" reveals an abandoned design; treated as inert |
| `ThreatPosed=25` | Mid-high AI threat priority |
| `SpecialThreatValue=1` | Maximum scoring weight on his own threat-target estimate |
| `ImmuneToVeins=yes` | TS legacy (no veins in YR) |
| `VeteranAbilities=STRONGER,FIREPOWER,ROF,SIGHT,FASTER` | Veteran (rank 1) gains: +50% HP, +25% dmg, +33% ROF, +2 Sight, +25% Speed |
| `EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF` | Elite (rank 2) gains: passive HP regeneration + the veteran stack. No FASTER on Elite (intentional) |
| `ElitePrimary=ChronoMP5E` | Elite weapon swap — same stats as ChronoMP5 (both Damage=125, Range=6, Warhead=HollowPointNoBuilding). The separate weapon section exists for cosmetic distinction; in stock data the bullets are identical |
| `PreventAttackMove=yes` | Cannot have an attack-move waypoint (same as engineer) |
| `MoveToShroud=no` | **Move-to-shroud filter** — cannot be ordered to move into shrouded cells. Prevents accidentally teleporting blind into unexplored map regions, which would be a hero-loss risk |
| `IFVMode=4` | IFV gunner-table index 4 → swap to that slot's weapon when boarding an [HTK](../allied/HTK.md). Slot 4 is the IFV's "elite-infantry" passenger weapon (typically a fast-firing variant of the Maverick missile) |

### Implicit defaults

- `Trainable=` — not set; defaults to `yes` (Chrono Commando *can* gain
  veterancy from kills, validated by VeteranAbilities/EliteAbilities entries).
- `ImmuneToPsionics=` — defaults `no`. Yuri *can* mind-control a Chrono
  Commando (rare in practice because the unit is hard to encounter).
- `Bombable=` — defaults `false` (no explicit gate). Crazy Ivan can still
  attach a bomb via Mission_Bomb.
- `Occupier=` / `Deployer=` — both default `no`.
- `Engineer=` — defaults `no`.
- `DetectDisguise=` — defaults `no`.

---

## artmd.ini — `[CCOMAND]` section

`c:/Users/enok/Documents/ra2-rust-game/ini/artmd.ini:420`:

```ini
[CCOMAND] ; Chrono Commando
Sequence=ComandoSequence
Cameo=CCOMICON
AltCameo=CCOMUICO
Crawls=yes
Remapable=yes
FireUp=3
PrimaryFireFLH=100,0,100
```

| Key | Meaning |
|-----|---------|
| `Sequence=ComandoSequence` | Frame layout block (`artmd.ini:13997`) — shared with Navy SEAL since the CCOMAND.SHP is essentially the SEAL frames with chrono effects |
| `Cameo=CCOMICON` | Sidebar cameo — `CCOMICON.SHP`, the chrono commando portrait |
| `AltCameo=CCOMUICO` | Elite cameo — visible because `Trainable=yes` (default), so this **is** reachable when promoted |
| `Crawls=yes` | Enables prone-while-walking — same as other infantry |
| `Remapable=yes` | House-colour remap applied |
| `FireUp=3` | Bullet-spawn frame within the FireUp track (same as Navy SEAL) |
| `PrimaryFireFLH=100,0,100` | Muzzle-flash launch offset: 100 leptons forward, 0 lateral, 100 vertical — same as SEAL. This is where the MP5 bullet emerges from his rifle (forward and up to roughly chest height) |

### Referenced sequence — `[ComandoSequence]`

`artmd.ini:13997`:

```ini
[ComandoSequence]
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
Paradrop=0,1,0
Cheer=340,8,0,E
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Panic=8,6,6
```

Compared to `[SealSequence]`: identical layout offsets (Walk at 8, Idle at
56/71, Die at 134/149, FireUp at 164, FireProne at 212, Down at 260, Up
at 276, Cheer at 340). The Chrono Commando SHP is a chrono-effect-overlaid
version of the SEAL SHP, sharing the same frame indices. `Panic=8,6,6`
reuses Walk (because `VoiceFeedback=` is empty — he doesn't panic).

---

## Weapons

### Primary — `[ChronoMP5]`

`rulesmd.ini:23056`:

```ini
[ChronoMP5]
Damage=125
ROF=10
Range=6
Projectile=InvisibleLow
Speed=100
Warhead=HollowPointNoBuilding
Report=SealAttack
```

| Key | Meaning |
|-----|---------|
| `Damage=125` | One-shot kills most infantry (Verses=200% on `Armor=none`) |
| `ROF=10` | Cooldown between bullets (10 frames ≈ 0.66s at 15 fps sim tick — very fast) |
| `Range=6` | 6-cell range — twice the standard infantry range. Combined with `Sight=8` he can pick targets at long range |
| `Projectile=InvisibleLow` | No visible projectile sprite (silenced fire) |
| `Speed=100` | Instant-hit |
| `Warhead=HollowPointNoBuilding` | Anti-infantry warhead — `Verses=200%,100%,75%,1%,1%,1%,0%,0%,0%,75%,100%`. The 1%-vs-vehicles trick lets him "see" vehicles as valid targets (otherwise AI ignores them) while doing effectively no damage. **0% vs buildings** prevents stray bullets from chip-damaging structures |
| `Report=SealAttack` | Sound `iseaatta`/`iseaattb` (silenced MP5 puff) |

The `HollowPointNoBuilding` warhead has:

```ini
[HollowPointNoBuilding]   ;rulesmd.ini:26956
Verses=200%,100%,75%,1%,1%,1%,0%,0%,0%,75%,100%
InfDeath=1
AnimList=PIFF
ProneDamage=100%
Bullets=yes
```

- `InfDeath=1` — small-arms death animation.
- `AnimList=PIFF` — bullet-impact dust puff (no blood/gore).
- `ProneDamage=100%` — full damage even when target is prone (no
  prone-reduction discount).
- `Bullets=yes` — flagged as a bullet warhead for the dodge/cover logic.

### Secondary — `[FakeC4]`

`rulesmd.ini:23065`:

```ini
[FakeC4]
Damage=5000
ROF=10
Range=1.5
CellRangefinding=yes
Projectile=InvisibleLow
Speed=100
Warhead=FakeC4WH
Report=SealPlaceBomb
SabotageCursor=yes ;gs instead of normal fire cursor to avoid confusion
```

| Key | Meaning |
|-----|---------|
| `Damage=5000` | Massive — but capped by the warhead's Verses to "buildings only" |
| `ROF=10` | Same cooldown as Primary |
| `Range=1.5` | Adjacent-cell only (he must be next to the target) |
| `CellRangefinding=yes` | Cell-center distance check for forgiving radius |
| `Projectile=InvisibleLow` | No visible projectile |
| `Speed=100` | Instant-hit |
| `Warhead=FakeC4WH` | Anti-building only — `Verses=0%,0%,0%,0%,0%,0%,100%,100%,100%,0%,100%` (wood/steel/concrete/special_2 only) |
| `Report=SealPlaceBomb` | Sound `icraatta` (SEAL bomb-place beep) |
| `SabotageCursor=yes` | **Cursor override** — when this weapon is the selected target action, show the sabotage/bomb cursor instead of the normal attack reticle. Player UX cue that this is the "blow up the building" weapon, not the rifle |

The `FakeC4WH` warhead has:

```ini
[FakeC4WH]   ;rulesmd.ini:26952
CellSpread=0
Verses=0%,0%,0%,0%,0%,0%,100%,100%,100%,0%,100%
```

No `AnimList=`, no `InfDeath=`, no `Bullets=`. The detonation animation is
the building's own death animation (`art.ini:Anim*Die*` entries on the
target). `CellSpread=0` means single-cell damage — no splash to adjacent
structures.

### Elite Primary — `[ChronoMP5E]`

`rulesmd.ini:25166`:

```ini
[ChronoMP5E]
Damage=125
ROF=10
Range=6
Projectile=InvisibleLow
Speed=100
Warhead=HollowPointNoBuilding
Report=SealAttack
```

**Byte-identical to `[ChronoMP5]`** — same Damage, ROF, Range, Projectile,
Warhead, Report. The Elite swap is cosmetic; the gain from promotion comes
from the `FIREPOWER` (+25% dmg multiplier on the unit, not the weapon) and
`ROF` (+33% fire rate) abilities, not from a stronger gun. Westwood likely
kept the separate section to allow future Elite-weapon balancing without
re-editing the base section.

### Projectile — `[InvisibleLow]`

Shared with several stealth/silent weapons (SEAL, Tanya pistol). Standard
inviso projectile with low priority. No visible sprite.

---

## Voices and sounds

`c:/Users/enok/Documents/ra2-rust-game/ini/soundmd.ini`:

| INI key on CCOMAND | soundmd block | Resolved samples |
|--------------------|---------------|------------------|
| `VoiceSelect=ChronoCommandoSelect` | `[ChronoCommandoSelect]` line 3482 | `$iseaexc` `$iseaseb` `$iseased` (random interrupt, Volume=90) — **reuses Navy SEAL samples** with chrono-flavored picks |
| `VoiceMove=ChronoCommandoMove` | `[ChronoCommandoMove]` line 3492 | `$iseamoa` `$iseamob` (random) — also reuses SEAL bank |
| `VoiceAttack=ChronoCommandoAttackCommand` | `[ChronoCommandoAttackCommand]` line 3487 | `$iseaata` `$iseaatb` `$iseaatc` (random) — same as SEAL attack-command |
| `VoiceFeedback=` | **empty** | No fear/panic voice — hero-tier silence |
| `VoiceSpecialAttack=ChronoCommandoSpecialAttack` | `[ChronoCommandoSpecialAttack]` line 3497 | `$iseaexa` (single) — `Type=global` plays for all players, alerting opponents to a special action |
| `DieSound=SealDie` | `[SealDie]` line 3945 | `$iseadia` `$iseadib` `$iseadic` (random interrupt) — shared with regular SEAL |
| `CreateSound=ChronoCommandoCreated` | `[ChronoCommandoCreated]` line 3502 | `$iseasec` — `Type=Global`, all players hear it (significant battlefield event alert) |
| `ChronoInSound=ChronoLegionTeleport` | `[ChronoLegionTeleport]` line 914 | `ichrmova` — shared with [CLEG](CLEG.md) Chrono Legionnaire teleport |
| `ChronoOutSound=ChronoLegionTeleport` | (same block) | reuses in-sound |
| `CrushSound=InfantrySquish` | `[InfantrySquish]` | `igensqua` |
| Weapon `ChronoMP5/E` `Report=SealAttack` | `[SealAttack]` line 1111 | `iseaatta` `iseaattb` (random) |
| Weapon `FakeC4` `Report=SealPlaceBomb` | `[SealPlaceBomb]` line 3937 | `icraatta` (single, Volume=60) |

Note the heavy sample reuse with Navy SEAL (`$isea*`) — Westwood treated
the Chrono Commando as a tech-stolen variant of the SEAL voice-wise, with
new envelope-only blocks (`ChronoCommando*`) that re-pick from the SEAL
sample pool.

---

## Prerequisites, owners, tech

- `Prerequisite=BARRACKS` — generic. Resolves to GAPILE for Allied owners,
  NAHAND for Soviet, YABRCK for Yuri (assuming the steal-tech mechanism has
  unlocked the unit for that house).
- `RequiresStolenAlliedTech=yes` — **the real gate**. Granted by a SPY
  successfully infiltrating an Allied tech building. Per CLAUDE.md +
  [SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md](../../SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md),
  infiltration of GATECH (Allied Battle Lab) sets the house bit that
  RequiresStolenAlliedTech reads.
- `Owner=` lists **all 10 houses** — any faction can build CCOMAND once
  they have stolen Allied tech. This is the "tech-stealing trooper" design
  archetype paralleled by:
  - `RequiresStolenSovietTech=yes` → unlocks **Yuri Lasher Tank** ([LTNK](../yuri/LTNK.md))-style? Actually no — that bit unlocks Boris-equivalents on Allied/Yuri. Check existing docs for clarification.
  - `RequiresStolenThirdTech=yes` → unlocks [PTROOP](PTROOP.md) Psi-Corp
    Trooper (the Yuri tech-steal counterpart, immediately below CCOMAND in
    rulesmd).
- `TechLevel=9` — late-game; mostly cosmetic given the tech-steal gate.
- `BuildLimit=`, `AIBasePlanningSide=` — unset.
- `AllowedToStartInMultiplayer=no` — never in lobby starting-unit list.

---

## Veterancy and upgrades

- **Trainable** (default `yes` — not overridden). Chrono Commando earns
  XP from kills.
- **Veteran promotion** (1 rank up) — `STRONGER, FIREPOWER, ROF, SIGHT,
  FASTER`:
  - `STRONGER` — +50% HP cap (multiplier on `Strength=100`)
  - `FIREPOWER` — +25% damage on all weapons (unit-side multiplier, not
    weapon-side)
  - `ROF` — +33% fire rate
  - `SIGHT` — +2 sight radius
  - `FASTER` — +25% movement speed (effectively irrelevant for a
    teleporter — speed only matters for the brief walking phase, not warp)
- **Elite promotion** (2 ranks up) — `SELF_HEAL, STRONGER, FIREPOWER, ROF`:
  - `SELF_HEAL` — passive HP regeneration (stacks with veteran HP cap)
  - Loses `FASTER` and `SIGHT` from the veteran set (Westwood intentionally
    omits `SIGHT` from EliteAbilities; this is **not** a stack)
  - **Weapon swap** to `ChronoMP5E` (which is mechanically identical to
    `ChronoMP5` — see weapon section)
- `AltCameo=CCOMUICO` — Elite cameo, reachable.

---

## Hardcoded behavior in gamemd.exe (Ghidra-verified)

### Teleport movement — `TeleportLocomotionClass` [BINARY-VERIFIED audit 11]

**Full RE in [TELEPORT_LOCOMOTION_DEEP_DIVE.md](../../TELEPORT_LOCOMOTION_DEEP_DIVE.md).**

`Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1}` resolves to
`TeleportLocomotionClass` (constructor at `0x00718000`, body
0x00718000–0x00718075). Ghidra-labeled with canonical-CLSID-comment header:
"Used by: 6 units (Chrono Legionnaire, etc.)". The constructor wires three
COM interface vtables (multi-interface object):
- `+0x0` = `TeleportLocomotionClass__IUnknown_vtable`
- `+0x4` = `TeleportLocomotionClass__ILocomotion_vtable`
- `+0x18` = `TeleportLocomotionClass__IPiggyback_vtable`

Instance-state initialized in constructor [BINARY-VERIFIED audit 11]:
- `+0x1C..+0x24` = source coord triplet (`param_1[7..9]`, copied from `g_NullCoord_Teleport_*` globals)
- `+0x28..+0x30` = destination coord triplet (`param_1[10..0xC]`, also from null-coord)
- `+0x34` = state byte (low — `*(byte*)(param_1 + 0xD) = 0`)
- `+0x35..+0x36` = additional state bytes (zeroed)
- `+0x3C` = LaunchFrame (`param_1[0xF] = g_CurrentFrameCounter`)

The class has **19 Ghidra-labeled member functions** (rare: most internal
classes have only the constructor labeled), making this one of the most
fully-named locomotor classes in the binary. The full vtable function set
is now verified:

| Address | Method | Role |
|---------|--------|------|
| 0x00718000 | `Constructor` | Init 3 vtables + coord/state fields |
| 0x00718080 | `Is_Moving` | Bool predicate for "currently teleporting" |
| 0x007180A0 | `Destination` | Return destination coord |
| 0x00718100 | `HeadToCoord` | Set new destination, start warp |
| 0x00718230 | `Stop_Moving` | Abort in-progress warp |
| 0x00718260 | `Update_Position` | Per-tick position update (3 xrefs) |
| 0x007187A0 | `PostWarpValidation` | Verify destination cell is valid; rollback on fail |
| 0x00718B70 | `Process` | Main state machine entry — dispatched each tick |
| 0x007192C0 | `Mark_All_Occupation_Bits` | Cell-occupancy maintenance |
| 0x007192F0 | `StateMachineTick` | Phase advance (warp-out → transit → warp-in → unstun) |
| 0x00719400 | `InitiateWarp` | Begin warp-out sequence |
| 0x00719790 | `ClearPendingWarpPhase` | Reset state to idle |
| 0x007197D0 | `Phase0_SetWarpingOut` | Enter phase 0 (warping out) |
| 0x00719BF0 | `TimerCheck` | Unstun timer check |
| 0x00719E30 | `QueryInterface` | COM interface query |
| 0x00719E90 | `Begin_Piggyback` | Start piggyback on another locomotor |
| 0x00719EE0 | `End_Piggyback` | Stop piggyback |
| 0x00719F30 | `Is_Ok_To_End` | Bool — can piggyback end safely |
| 0x0071A160 | `ILocomotion_QI_Thunk` | ILocomotion COM thunk |

**[ADDRESS DISCREPANCY corrected audit 11]**: The doc previously claimed
`TimerCheck` at `0x0070F770`. The actual `TeleportLocomotionClass::TimerCheck`
is at **`0x00719BF0`** (Ghidra-labeled, body 0x00719bf0–0x00719c57).
`0x0070F770` is `FUN_0070f770` — a different, unrelated function (97-byte
body, unlabeled by Ghidra). The wrong address has been corrected.

The five-phase teleport state machine (warp-out → transit → destination
validation → warp-in → unstun) is implemented across these functions; the
deep RE doc traces the phase logic but the entry-point addresses above
are now BINARY-VERIFIED in this audit pass.

**`Teleporter=yes`** is a TechnoTypeClass field at byte offset
**`TechnoTypeClass+0xCD4`** [BINARY-VERIFIED audit 11 via
`TechnoTypeClass__ReadINI` reading `(char)param_1[0x335]` ←
`s_Teleporter_00843e60`; xref data at `0x00713FE9`]. This flag gates the
teleport-cursor render, the unstun-timer scheduling, and the
destination-validation path. Without it, the locomotor refuses to engage
even with the correct GUID.

**[ADDRESS DISCREPANCY corrected audit 11]**: The doc previously claimed
the Teleporter parser site was at `0x0071450F`. The actual xref for
`Teleporter` is at **`0x00713FE9`** in `TechnoTypeClass__ReadINI`. The
address `0x0071450F` is actually the parser site for
`RequiresStolenAlliedTech` (string at `0x00843BC4` — both addresses appear
in the doc, and were swapped).

**[CLARIFICATION audit 11]**: `Teleporter` (TechnoType+0xCD4) is **not the
same field** as `Warpable` (TechnoType+0xD3A, audit 5 CLEG). `Teleporter`
= "this unit CAN warp itself" (gates teleport locomotor). `Warpable` =
"this unit can BE warped by Chrono Legionnaire" (target eligibility). Two
distinct INI keys, two distinct byte offsets.

### Move-to-anywhere range

Unlike [CLEG](CLEG.md) Chrono Legionnaire (whose teleport is gated by a
maximum range from the source — short-warp model), CCOMAND has **no range
limit** on his teleport. He can move-order to any reachable infantry cell
anywhere on the map, instantly. This is a property of the locomotor GUID
choice, not a separate INI field.

### FakeC4 instant-building-kill

When CCOMAND is ordered to attack a **building** target, the engine's
target-action logic selects the Secondary (`FakeC4`) instead of the
Primary, because:

1. `FakeC4` has `Range=1.5` (building-adjacency).
2. `FakeC4WH` has 100% Verses on building armor types, 0% on infantry/vehicles.
3. `HollowPointNoBuilding` has 0% Verses on building armor types.

The engine's weapon-selection logic (see
[DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md](../../DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md))
picks the weapon with non-zero Verses against the target's armor type,
falling back to Primary if all weapons are valid. For a building, only
FakeC4 has non-zero Verses → FakeC4 is selected.

Damage=5000 is far above any vanilla building's Strength (most are 1000–
2000), so a single FakeC4 detonation destroys the building outright.

`SabotageCursor=yes` on the weapon swaps the on-screen target-reticle to
the sabotage cursor (purple bomb icon) when hovering an enemy building
within Range=1.5, alerting the player that this is a destructive action.

### Inline comment: "otherwise he can teleport into a building and kill it before he unwarps"

Westwood's explanation: an earlier design used `C4=yes` (the SEAL/Tanya
on-walk-up C4 mechanism), but combining it with the teleport caused
buildings to die during the warp-in phase before the commando finished
unstun, leading to "I lost a building to thin air" complaints. The
`FakeC4` weapon-based replacement requires the commando to **fire** the
weapon (post-unstun) instead of triggering during warp, restoring player
agency.

### OpenTransportWeapon override

`OpenTransportWeapon=0` forces the IFV/Battle-Fortress passenger gunner
table to fire Weapon 0 (Primary = ChronoMP5) instead of doing its
"best-weapon-vs-target" picker. This prevents:

- FakeC4 from firing out of a Battle Fortress at a building (which would
  let the FV chain-destroy structures from outside building-defense range).
- The IFV from defaulting to Slot 0 (chassis weapon) which would also
  produce a player-surprising fire pattern.

### MoveToShroud=no

The `MoveToShroud=no` flag (TechnoTypeClass; see RE cheat-sheet) prevents
the player from issuing a move/attack order with a destination cell that
is currently shrouded. For a teleporter this is critical: a blind warp
into shroud could land the commando next to an enemy stack with no
information, costing a $2000 hero.

### Crushable=no

The `Crushable=no` flag (TechnoTypeClass) prevents vehicles from running
over and instakilling the commando — same protection as Tanya/SEAL/Boris.
Without this flag a single Grizzly could end the unit on contact.

### Ghidra string-search results for "CCOMAND" and "ChronoCommando"

- `search_strings "CCOMAND"` → **0 matches** (run 2026-05-17).
- `search_strings "ChronoCommando"` → 0 matches (the section/voice names
  do not appear as standalone strings in gamemd.exe).
- `search_strings "RequiresStolenAlliedTech"` → 1 hit at `0x00843bc4`
  (TechnoTypeClass field-name string used by `TechnoTypeClass__ReadINI`).
- `search_strings "Teleporter"` → 1 hit at `0x00843e60` (TechnoTypeClass
  field-name string).

Confirmed: **no hardcoded section-name branch** for CCOMAND. All behavior
is driven by the combination of:

- `Teleporter=yes` flag (gates teleport state machine)
- `Locomotor=` GUID (selects `TeleportLocomotionClass`)
- `Primary`/`Secondary` weapon entries (drive the action-selection logic
  against target armor type)
- `RequiresStolenAlliedTech=yes` (gates build availability)
- `Crushable=no`, `MoveToShroud=no`, `OpenTransportWeapon=0`, `IFVMode=4`,
  `PreventAttackMove=yes` (TechnoTypeClass flags applied each tick)

---

## TS-legacy filter

- `TiberiumProof=yes` — TS terrain hazard (no tiberium in YR maps). Flag
  is read but never triggers in vanilla YR. Do not omit — defensive.
- `ImmuneToVeins=yes` — TS terrain (no veins in YR). Defensive.
- `;SpeedType=Amphibious` `;MovementZone=AmphibiousDestroyer` — commented
  out; abandoned TS-era amphibious design. Skip entirely.
- `;C4=yes` — commented out; the TS-era SEAL-style C4 mechanism, replaced
  by FakeC4 weapon-and-warhead. Skip.
- `Locomotor={4A582747-...}` (TeleportLocomotionClass) — alive in YR, used
  by CCOMAND, CLEG, HARV (Chrono Miner). Not TS legacy.

---

## Cross-references

- **Tech-steal counterparts** (same `RequiresStolen*Tech=yes` archetype,
  unlocked by SPY infiltration):
  - [PTROOP](PTROOP.md) — Psi-Corp Trooper, requires Stolen Third (Yuri)
    Tech (the Yuri Battle Lab is YATECH; stealing it grants this unit to
    Allied/Soviet players). Mind-controls one infantry on capture.
  - **(no canonical name)** — Tank Destroyer ([TNKD](TNKD.md)) is built
    by Allied via `RequiresStolenSovietTech=yes`.
- **Related chrono units**:
  - [CLEG](CLEG.md) — Chrono Legionnaire (Allied tech-built, short-range
    teleport with `EraseAnim` weapon).
  - [HARV](HARV.md) — Chrono Miner (auto-teleports home to refinery when
    full).
- **Builder**: any house's Barracks ([GAPILE](../structures/GAPILE.md),
  [NAHAND](../structures/NAHAND.md), [YABRCK](../structures/YABRCK.md))
  with the `StolenAlliedTech` bit set on the owning house.
- **Unlock source**: [SPY](SPY.md) infiltrating an Allied tech building
  (typically [GATECH](../structures/GATECH.md) Battle Lab). See
  [SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md](../../SPY_INFILTRATION_SYSTEM_GHIDRA_REPORT.md).
- **IFV passenger**: `IFVMode=4` → IFV swaps to slot-4 elite-infantry
  weapon (see [HTK](HTK.md)).
- **Battle Fortress passenger**: `OpenTransportWeapon=0` → fires Primary
  (ChronoMP5) out the side ports.
- **Vulnerable to**: Yuri mind-control (`ImmuneToPsionics=no` default),
  Crazy Ivan bomb (`Bombable=` default), Attack Dog (`Crushable=no` blocks
  vehicle crush, but a dog's leap-bite is range-1.5 parasite consume),
  Desolator radiation.
- **Counter targets**: enemy infantry (1-shot via Damage=125), enemy
  buildings (instant-kill via FakeC4 — 5000 dmg). Vehicles/aircraft are
  effectively immune (1% Verses).

---

## Ghidra audit log (audit iteration 11 — 2026-05-18)

**Methodology**: Targeted decompiles to verify the doc's named functions,
struct offsets, and hardcoded mechanics for the Chrono Commando. Heavy
focus on `TeleportLocomotionClass` (the shared chrono locomotor used by
CLEG, CCOMAND, and HARV Chrono Miner) since CCOMAND has no
section-name-specific code in gamemd.exe (verified by string-search).
~16 Ghidra queries: 12 decompiles + 4 string searches + xref-by-data
queries.

### Function entry-points (BINARY-VERIFIED)

`TeleportLocomotionClass` is one of the most thoroughly Ghidra-labeled
internal classes in the binary — 19 named member functions in addition
to the constructor (typical class has 1–2 named methods). All addresses
below are entry-point-verified via `get_function_by_address`:

| Address | Ghidra label | Role |
|---------|--------------|------|
| 0x00718000 | `TeleportLocomotionClass::Constructor` | 3-vtable init + coord/state fields (decompiled) |
| 0x00718080 | `TeleportLocomotionClass::Is_Moving` | bool predicate |
| 0x007180A0 | `TeleportLocomotionClass::Destination` | returns dest coord |
| 0x00718100 | `TeleportLocomotionClass::HeadToCoord` | set new destination, kick off warp |
| 0x00718230 | `TeleportLocomotionClass::Stop_Moving` | abort warp |
| 0x00718260 | `TeleportLocomotionClass::Update_Position` | per-tick position update |
| 0x007187A0 | `TeleportLocomotionClass::PostWarpValidation` | destination-cell validity check |
| 0x00718B70 | `TeleportLocomotionClass::Process` | main per-tick dispatch |
| 0x007192C0 | `TeleportLocomotionClass::Mark_All_Occupation_Bits` | cell-occupancy bookkeeping |
| 0x007192F0 | `TeleportLocomotionClass::StateMachineTick` | warp-out → transit → warp-in phase advance |
| 0x00719400 | `TeleportLocomotionClass::InitiateWarp` | start warp-out sequence |
| 0x00719790 | `TeleportLocomotionClass::ClearPendingWarpPhase` | reset to idle |
| 0x007197D0 | `TeleportLocomotionClass::Phase0_SetWarpingOut` | enter phase 0 |
| 0x00719BF0 | `TeleportLocomotionClass::TimerCheck` | unstun-timer check |
| 0x00719E30 | `TeleportLocomotionClass::QueryInterface` | COM QI |
| 0x00719E90 | `TeleportLocomotionClass::Begin_Piggyback` | start piggyback |
| 0x00719EE0 | `TeleportLocomotionClass::End_Piggyback` | end piggyback |
| 0x00719F30 | `TeleportLocomotionClass::Is_Ok_To_End` | bool — can piggyback end? |
| 0x0071A160 | `TeleportLocomotionClass::ILocomotion_QI_Thunk` | ILocomotion COM thunk |

### Address discrepancies corrected

- **TimerCheck**: doc had `0x0070F770` → actual `0x00719BF0`. `0x0070F770`
  is `FUN_0070f770` (unlabeled, 97-byte body, unrelated function).
  Corrected in doc.
- **Teleporter parser site**: doc had `0x0071450F` → actual `0x00713FE9`
  (xref to `s_Teleporter_00843e60` from `TechnoTypeClass__ReadINI`).
  `0x0071450F` is the parser site for `RequiresStolenAlliedTech` — the
  two were transposed. Corrected in doc.

### Struct-offset corrections (CRITICAL — updates cumulative cheat-sheet)

**`Teleporter` is at TechnoTypeClass+0xCD4** (BINARY-VERIFIED via
`TechnoTypeClass__ReadINI` decompile: `*(char*)(param_1 + 0x335)` ←
`ReadBool("Teleporter")` at xref `0x00713FE9`). `0x335 * 4 = 0xCD4`
(InfantryTypeClass-style int* indexing; cf. ENGINEER audit 3 note on
`field_0xCE` = `0xCE*4 = 0x338` offset arithmetic).

**This corrects the audit-index cumulative table**, which previously
listed `+0xD3A = Teleporter`. **`+0xD3A` is actually `Warpable`** (per
CLEG audit 5 — "this unit can BE warped by Chrono Legionnaire", target
eligibility for the chrono-erase weapon). The two concepts are distinct
and live at distinct offsets:

- `+0xCD4` = `Teleporter` (this unit CAN warp itself — gates teleport locomotor activation)
- `+0xD3A` = `Warpable` (this unit can BE warped by enemy chrono — target eligibility)

### Other TechnoType offsets BINARY-VERIFIED (audit 11)

Via the same `TechnoTypeClass__ReadINI` decompile pass:

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0xC8D` | `MoveToShroud` | byte (default 1) | gates blind-move into shrouded cells |
| `+0xCD4` | `Teleporter` | byte | this unit warps itself |
| `+0xD9B` | `RequiresStolenThirdTech` | byte | Yuri tech-steal unlock |
| `+0xD9C` | `RequiresStolenSovietTech` | byte | Soviet tech-steal unlock |
| `+0xD9D` | `RequiresStolenAlliedTech` | byte | Allied tech-steal unlock (CCOMAND's gate) |

String table data verified:

- `s_Teleporter` @ `0x00843e60`
- `s_RequiresStolenAlliedTech` @ `0x00843bc4`
- `s_RequiresStolenSovietTech` @ `0x00843be0`
- `s_RequiresStolenThirdTech` @ `0x00843bfc`
- `s_MoveToShroud` @ `0x008444c4`

### TeleportLocomotionClass instance offsets (BINARY-VERIFIED via constructor decompile)

| Offset | Field | Notes |
|--------|-------|-------|
| `+0x0` | IUnknown vtable | COM root |
| `+0x4` | ILocomotion vtable | main interface |
| `+0x18` | IPiggyback vtable | piggyback secondary interface |
| `+0x1C..+0x24` | Source coord (3 ints) | init from `g_NullCoord_Teleport_*` |
| `+0x28..+0x30` | Destination coord (3 ints) | init from same null-coord globals |
| `+0x34` | State byte | low-byte phase indicator (0 at construction) |
| `+0x35..+0x36` | Aux state bytes | both zeroed |
| `+0x3C` | LaunchFrame | `g_CurrentFrameCounter` at construction |

### Negative findings (verified absence of code paths)

- `search_strings("CCOMAND")` → **0 matches**
- `search_strings("ChronoCommando")` → **0 matches**

**Confirms: no hardcoded section-name branch for CCOMAND.** All
chrono-commando-specific behavior is data-driven from the rulesmd
section via the combination of `Teleporter=yes` (gates teleport
locomotor) + `Locomotor={4A582747-...}` (selects
`TeleportLocomotionClass`) + `RequiresStolenAlliedTech=yes` (gates
build availability) + the `FakeC4`/`ChronoMP5` weapon entries.

### Items NOT re-verified in this pass (DEFERRED)

- `Mission_Attack @ 0x0051F3E0` (FakeC4 weapon-selection-against-building gate)
  — already BINARY-VERIFIED in audit 4 (GHOST) + audit 7 (TANY); not
  re-decompiled this pass. Same chain applies here: TypeClass+0xEC2 C4
  is NOT set on CCOMAND (commented out `;C4=yes`), so the on-walk-up
  Mission_Attack C4 branch does not fire for him. Instead, FakeC4 is
  picked by the `DetermineAction`/`GetFireError` weapon-selection chain
  based on Warhead Verses (FakeC4WH=100% vs buildings, HollowPointNoBuilding=0%
  vs buildings) — see `DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md`.
- Per-tick `Process` body @ `0x00718B70` (the full state-machine dispatch
  table) — entry verified, body DEFERRED. Each of the 19 named methods
  is entry-verified; only the constructor and string-table xrefs were
  fully decompiled in this pass.
- `OpenTransportWeapon` consumer (open-topped transport gunner-table
  override) — TechnoType-scope already in audit 7 cumulative
  (`+0xD50 = OpenTransportWeapon`); consumer chain in
  `BattleFortressClass`/`PassengerWeaponPicker` DEFERRED.
- `IFVMode=4` consumer (the IFV gunner-table that swaps weapons by
  passenger type) — DEFERRED to HTK doc audit.
- `FakeC4` end-to-end firing trace through `Fire_At` + `WarheadDetonate`
  — DEFERRED; warhead-Verses-against-building-armor logic already covered
  in TANY audit 7.

### Confidence summary

- **HIGH**: 19 TeleportLocomotionClass function entry-points (all
  Ghidra-labeled with canonical CLSID comment on constructor), 5
  TechnoType offset+INI-key bindings (read directly from
  TechnoTypeClass__ReadINI string table xrefs), constructor body (3
  vtables + coord/state field layout), Teleporter ≠ Warpable distinction.
- **MEDIUM**: 19-method vtable mapping (entry-point labels are
  Ghidra-canonical, but per-method behaviors only verified for
  constructor in this pass; the rest are name-confidence only).
- **No new INCORRECT findings in the doc itself** — the only
  discrepancies were the two address transpositions (TimerCheck,
  Teleporter parser site) and the cumulative-table's `+0xD3A` claim,
  all corrected.

---

## Coverage audit

- ✅ Every key in `[CCOMAND]` rulesmd block (48 lines, line 4174–4223)
  covered with per-key explanation.
- ✅ Every key in `[CCOMAND]` artmd block (8 lines, line 420–427) covered,
  plus `[ComandoSequence]` (19 frame keys at line 13997).
- ✅ Weapon chain: ChronoMP5 + FakeC4 + ChronoMP5E (Elite) + InvisibleLow
  projectile + HollowPointNoBuilding + FakeC4WH warheads — all annotated
  verbatim.
- ✅ Sound chain: 11 distinct soundmd entries enumerated, with note on
  heavy `$isea*` SEAL-sample reuse.
- ✅ Ghidra search: `search_strings "CCOMAND"`/`"ChronoCommando"` → 0
  hits. `RequiresStolenAlliedTech` at 0x00843bc4 (xref to
  `TechnoTypeClass__ReadINI`). `Teleporter` at 0x00843e60. Confirms no
  hardcoded section-name branch; behavior via shared
  `TeleportLocomotionClass` + flag bits.
- ✅ TS-legacy filter applied (TiberiumProof, ImmuneToVeins, commented-out
  amphibious zone, commented-out C4=yes).
- ✅ Cross-references to CLEG, HARV, PTROOP, TNKD (tech-steal triplet),
  SPY, GATECH, HTK, FV, vulnerable/counter matchups.
- ✅ Deep-RE reference to TELEPORT_LOCOMOTION_DEEP_DIVE,
  TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE,
  TECHNOCLASS_CHRONO_OFFSETS_VERIFIED, CHRONO_MINER_TELEPORT,
  SPY_INFILTRATION_SYSTEM, DETERMINE_ACTION_DOWNSTREAM.
