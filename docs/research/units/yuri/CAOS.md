# CAOS — Chaos Drone (Yuri Berserker-Gas Vehicle)

**INI ID:** `CAOS`
**Display Name:** `Chaos Drone` (`UIName=Name:ChaosDrone`)
**Side:** Yuri (`Owner=YuriCountry`)
**Category:** Vehicle / AFV (in `[VehicleTypes]`)
**Cameo:** `CAOSICON` / `CAOSUICO` (AltCameo)
**Voxel:** yes

The Chaos Drone is Yuri's anti-infantry / anti-vehicle disruptor — a small,
fast, **unarmed** chassis that emits a `PsychGasCreate`-warhead invisible
projectile, detonating berserk gas at the target cell. **Any non-allied,
non-building, non-psionic-immune unit in the 3-cell radius immediately enters
a persistent "berserk" state** (TechnoClass+0x298 ← 1), in which their
target-acquisition filter bypasses alliance checks — they attack any nearby
unit, **friend or foe**, until the berserk timer expires.

The Chaos Drone itself deals only the warhead's direct (Verses-scaled) damage;
the *real* effect is friendly-fire chaos in an enemy formation.

> **Cross-references — do not re-derive:**
> - [`CHAOS_DRONE_BERSERK_GHIDRA_REPORT.md`](../../CHAOS_DRONE_BERSERK_GHIDRA_REPORT.md) (628 lines) — exhaustive verified report on the berserk mechanic: WarheadType+0x16D `Psychedelic` flag, TechnoType+0x690 `BerserkFriendly`, TechnoType+0x6C0 `AttackFriendlies`, TechnoClass+0x298 `berserk_flag`, TechnoClass+0x29C `berserk_timer`, set/clear/decrement disassembly, Mission_Hunt dispatch, GetFireError TARGET-side filter, full alliance-bypass logic in Scan_Cell_For_Target. **All deep RE is in that doc; cross-reference rather than re-derive.**
> - [`DISK.md`](./DISK.md) — sibling Yuri tier-3 unit with dedicated DiskLaserClass.
> - [`TELE.md`](./TELE.md) — sibling Yuri tier-3 unit with hardcoded LocomotorBeam mechanic.
> - [`MIND.md`](./MIND.md) — sibling Yuri tier-3 with mind-control mechanic.
> - [`DRON.md`](../soviet/DRON.md) — sibling "anti-vehicle drone" pattern from the Soviet side.

> **TS-legacy filter:** `;Image=DRON`, `;Burst=4`, `;Anim=CDGAS`, `;MovementZone=Normal ;gs FLAW`, `;Bombable=no`, `;SprayAttack=yes` are all INI comments — drafts or disabled features. Locomotor with `;<-drive mech->` separator is an in-INI comment chain showing original drive/mech alternatives. The `Deployer=yes` on a VehicleType is an **orphan INI key** — `Deployer` is InfantryType-scope only (verified at 0x0052460d → InfantryTypeClass__ReadINI). On CAOS it parses but has no engine effect.

---

## 1. Full `rulesmd.ini` section verbatim

```ini
[CAOS]
UIName=Name:ChaosDrone
Name=Chaos Drone
;Image=DRON
Category=AFV
Prerequisite=YAWEAP
Primary=ChaosAttack
Secondary=VirtualScanner
NavalTargeting=6
Strength=130 ;225; 175
SuppressionThreshold=5; damage below this amount won't suppress the parasite
ReselectIfLimboed=yes ; If selected when limbo on attack of infantry, reselect when unlimbo
DefaultToGuardArea=yes ; the much awaited terror drone default to move and attack when resting
Armor=light
TechLevel=4
Turret=no
IsTilter=no
CrateGoodie=no
Sight=6
Speed=8
Owner=YuriCountry
Cost=1000
Soylent=1000
Points=20
ROT=40
AllowedToStartInMultiplayer=no
Crusher=no
Crewed=no
IsSelectableCombatant=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=ChaosDroneSelect
VoiceAttack=ChaosDroneAttackCommand
VoiceMove=ChaosDroneMove
VoiceFeedback=
DieSound=ChaosDroneDie
MoveSound= ChaosDroneMoveStart
MaxDebris=2
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1};<-drive   mech->{55D141B8-DB94-11d1-AC98-006008055BB5}
;MovementZone=Normal ;gs FLAW needs to be changed to this when The Flaw is fixed
MovementZone=Destroyer
ThreatPosed=25	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
Weight=.5
ImmuneToPsionics=yes
ImmuneToRadiation=yes
Parasiteable=yes
Trainable=no
Explodes=no
AccelerationFactor=5 ; really fast
DeaccelerationFactor=5 ; This is TS's mizspelingg knot min
ZFudgeColumn=8
ZFudgeTunnel=13
;Bombable=no
Size=2
Accelerates=false
BerserkFriendly=yes
;SprayAttack=yes	;GEF this guy randomly spreads stuff all over
CanPassiveAquire=no ; Won't try to pick up own targets ;GEF don't want him accidently chaos gassing your own guys
CanRetaliate=no; Won't fire back when hit ;GEF same reason as above 
Deployer=yes
```

### 1.1 Key-by-key explanation

| Key | Value | Read by | Effect |
|-----|-------|---------|--------|
| `UIName=Name:ChaosDrone` | string | AbstractTypeClass | CSF lookup. |
| `Name=Chaos Drone` | string | AbstractTypeClass | English fallback. |
| `;Image=DRON` | (commented) | — | Would have used the Terror Drone (DRON) voxel; disabled. Active artmd `[CAOS]` block has `Voxel=yes` with its own `caos.vxl`. |
| `Category=AFV` | enum | TechnoTypeClass | AFV category. |
| `Prerequisite=YAWEAP` | building | TechnoTypeClass | **Yuri War Factory only** — no Battle Lab gate. Available early. |
| `Primary=ChaosAttack` | weapon | TechnoTypeClass | The berserk-gas-emitter virtual weapon. See §3.1. |
| `Secondary=VirtualScanner` | weapon | TechnoTypeClass | NeverUse=yes weapon for target-scanning extension. See §3.2. |
| `NavalTargeting=6` | enum | TechnoType @ 0x007121be | `NAVAL_NONE = 6` — never shoot into water. Comment block at rulesmd line ~3691 enumerates this. The Chaos Drone refuses naval engagement. |
| `Strength=130 ;225; 175` | hp | TechnoTypeClass | **130 HP** — fragile. The two commented values (`;225; 175`) are historical balance iterations. |
| `SuppressionThreshold=5` | int | TechnoType @ 0x0071506d (cheat sheet) | Damage below 5 doesn't suppress the parasite (legacy field for parasite-bearing weapons; CAOS has no parasite weapon so this is inert). Inline comment hints at copy-paste from Terror Drone INI block. |
| `ReselectIfLimboed=yes` | bool | TechnoType @ 0x007142b4 (cheat sheet) | If the drone is in selection when it limbos for an attack of infantry, reselect when unlimbo. Inline comment confirms intent. |
| `DefaultToGuardArea=yes` | bool | TechnoType @ 0x00714f44 (cheat sheet) | When idle/resting, default to "guard area" mode (move-and-attack near initial position). Inline comment: "the much awaited terror drone default to move and attack when resting". |
| `Armor=light` | enum | TechnoTypeClass | Light armor — dies fast under any fire. |
| `TechLevel=4` | int | TechnoTypeClass | TechLevel 4 — mid-tier (vs DISK/MIND/TELE at 2). |
| `Turret=no` | bool | UnitTypeClass | No turret — the drone IS the emitter. |
| `IsTilter=no` | bool | UnitType @ 0x00747712 | Body does NOT tilt when turning. |
| `CrateGoodie=no` | bool | UnitType @ 0x00747658 | No crate pop. |
| `Sight=6` | cells | TechnoTypeClass | Short sight range (smaller than weapon range — but VirtualScanner secondary extends targeting). |
| `Speed=8` | int | TechnoTypeClass | **Fast (8)** — among the fastest ground units (most tanks are 4-6). Drone needs speed to deliver gas to enemy formations and escape. |
| `Owner=YuriCountry` | country list | TechnoTypeClass | Yuri only. |
| `Cost=1000` | credits | TechnoTypeClass | Same as Magnetron. |
| `Soylent=1000` | credits | TechnoTypeClass | Full recycle value. |
| `Points=20` | int | TechnoTypeClass | Score on kill. |
| `ROT=40` | int | TechnoTypeClass | **Fast turn rate (40)** — agile, can reorient quickly. |
| `AllowedToStartInMultiplayer=no` | bool | TechnoTypeClass | Not pre-built. |
| `Crusher=no` | bool | TechnoTypeClass | No crush. |
| `Crewed=no` | bool | TechnoTypeClass | No crew bailout on destruction. |
| `IsSelectableCombatant=yes` | bool | TechnoTypeClass | Counts as combat unit. |
| `Explosion=...` | anim list | TechnoTypeClass | Standard 5-anim destruction. |
| `VoiceSelect=ChaosDroneSelect` | sound | TechnoTypeClass | Unique select voice (sound:1338). |
| `VoiceAttack=ChaosDroneAttackCommand` | sound | TechnoTypeClass | Unique attack voice (sound:1343). |
| `VoiceMove=ChaosDroneMove` | sound | TechnoTypeClass | Unique move voice (sound:1348). |
| `VoiceFeedback=` | (empty) | TechnoTypeClass | None. |
| `DieSound=ChaosDroneDie` | sound | TechnoTypeClass | Unique death sound (sound:1353). |
| `MoveSound= ChaosDroneMoveStart` | sound | TechnoTypeClass | One-shot engine start (sound:5299). Note INI leading space before key (harmless). |
| `MaxDebris=2` | int | TechnoTypeClass | Up to 2 debris pieces. |
| `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1};<-drive   mech->{55D141B8-DB94-11d1-AC98-006008055BB5}` | CLSID | TechnoTypeClass | **Active locomotor is the first GUID `{4A582741-...}` = DriveLocomotionClass.** Everything after `;` is INI comment — the inline arrow `<-drive   mech->` separates the *active* drive locomotor from the *original* mech locomotor `{55D141B8-...}` (MechLocomotionClass). The drone uses standard drive, not mech walk. |
| `;MovementZone=Normal ;gs FLAW needs to be changed to this when The Flaw is fixed` | (commented) | — | Author comment about a known engine bug — the Chaos Drone *should* use `MovementZone=Normal` (which would allow it through normal-passability terrain) but a "FLAW" forced fallback to `Destroyer`. The active line below uses Destroyer. **Live-INI quirk:** the drone navigates as if it were an MBT-class ground unit, which may cause minor pathing oddities. |
| `MovementZone=Destroyer` | enum | TechnoTypeClass | Active value — Destroyer (MBT-class) pathing zone. |
| `ThreatPosed=25` | int | TechnoTypeClass | AI threat weight. |
| `DamageParticleSystems=SparkSys,SmallGreySSys` | particle list | TechnoTypeClass | Damaged emissions. |
| `Weight=.5` | float | TechnoTypeClass | **Tiny weight (.5)** — half a standard unit. Affects AI calculations and transport-loading. |
| `ImmuneToPsionics=yes` | bool | TechnoType @ 0x00714fa7 | **Cannot be mind-controlled.** The Chaos Drone is robotic — psionic abilities have no effect. |
| `ImmuneToRadiation=yes` | bool | TechnoType (cheat sheet) | **Immune to Desolator radiation.** Robotic chassis ignores rad damage. |
| `Parasiteable=yes` | bool | TechnoType @ 0x00714f86 (cheat sheet) | **CAN be parasitized by Terror Drone.** Even though the Chaos Drone is robotic, the Terror Drone's parasite (DroneJump warhead) can still attach and disable it. |
| `Trainable=no` | bool | TechnoTypeClass | Does NOT gain veterancy ranks. The Chaos Drone stays rookie forever — no Veteran/Elite progression. |
| `Explodes=no` | bool | TechnoType @ 0x007122c5 | No death explosion damage (Explosion= anim still plays for visual). |
| `AccelerationFactor=5` | float | TechnoType @ 0x007124bc (NEW) | **Acceleration ramp factor (5 = "really fast" per inline comment).** Verified TechnoType scope. Most units use 1.0; CAOS uses 5 for snappy stop/start. |
| `DeaccelerationFactor=5` | float | TechnoType @ 0x0071249b (NEW) | **Deceleration factor.** Inline comment is a famous typo joke: "This is TS's mizspelingg knot min" (mocking "deceleration" spelling and TS's "mizspelingg knot min" naming convention; "knot" = nautical-speed pun, "knot min" ≈ "not in"). Verified TechnoType scope. |
| `ZFudgeColumn=8` / `ZFudgeTunnel=13` | int | TechnoTypeClass | Z-buffer render fudges (same as DISK/TELE). |
| `;Bombable=no` | (commented) | — | Would have made the unit immune to Ivan bombs; disabled, so it IS bombable. |
| `Size=2` | int | TechnoTypeClass | Small transport-cost. |
| `Accelerates=false` | bool | TechnoTypeClass | Constant speed. (Note the apparent contradiction with `AccelerationFactor=5` — `Accelerates=false` means no gradual speed-up to top speed, while `AccelerationFactor` controls something subtler like turn-into-direction acceleration; both can coexist.) |
| `BerserkFriendly=yes` | bool | TechnoType @ 0x007148fa (CHAOS_DRONE doc §2.2) | **The Chaos Drone is IMMUNE to friendly fire from berserk units.** Verified TechnoType scope. The semantic is: when ANY berserk attacker tries to target a unit with `BerserkFriendly=yes`, `GetFireError` returns FIRE_ILLEGAL. The drone won't get caught by its own gas's friendly-fire effect — its berserk-affected enemies cannot retaliate against it. |
| `;SprayAttack=yes` | (commented) | — | Would have made the drone "randomly spread stuff all over" per inline comment. Disabled. Without it, the drone fires once per target order, not in a random spray pattern. |
| `CanPassiveAquire=no` | bool | TechnoType @ 0x00714473 | **Does NOT auto-acquire targets.** Inline comment: "don't want him accidently chaos gassing your own guys". The player must manually order each attack — otherwise the drone might gas a friendly formation. Same pattern as MIND/Magnetron. |
| `CanRetaliate=no` | bool | TechnoType (cheat sheet) | Will not fire back when hit. Inline: "same reason as above" — accidental friendly chaos gassing. |
| `Deployer=yes` | bool | InfantryType @ 0x0052460d (NEW) | **ORPHAN INI KEY on a VehicleType.** Verified: `Deployer` is read ONLY by InfantryTypeClass__ReadINI; UnitTypeClass and TechnoTypeClass don't read it. On CAOS this key is parsed by INI loader but never bound to any field. **No engine effect.** Likely an authoring leftover or copy-paste from an infantry block. |

---

## 2. Full `artmd.ini` section verbatim

```ini
[CAOS] ; Chaos Drone
Cameo=CAOSICON
AltCameo=CAOSUICO
Voxel=yes
Remapable=yes
```

| Key | Value | Notes |
|-----|-------|-------|
| `Cameo=CAOSICON` | SHP | Standard cameo. |
| `AltCameo=CAOSUICO` | SHP | Alternate (unbuildable) cameo. |
| `Voxel=yes` | bool | `caos.vxl` + `.hva`. |
| `Remapable=yes` | bool | House-color tinted. |

> **No PrimaryFireFLH=, no TurretOffset=.** The Chaos Drone has no turret (`Turret=no`) and no FLH offset for its weapon (uses default origin — center of drone body). The invisible projectile launches from the body and detonates at the target cell.

---

## 3. Weapons

### 3.1 `[ChaosAttack]` — primary (berserk gas emitter)

```ini
[ChaosAttack]
Damage=600
ROF=45
Range=3;1.83;GEF Since we have a new long range gas effect now, might as well make the Chaos Drone only approach as much as it needs to in order to affect the target
Projectile=InvisibleLow
Speed=30
Warhead=PsychGasCreate
Report=ChaosDroneAttack
;Burst=4
OmniFire=yes
AreaFire=yes
;Anim=CDGAS
```

| Key | Effect |
|-----|--------|
| `Damage=600` | **600 damage** — high raw value, but PsychGasCreate's Verses=0% vs structures and 50% vs vehicles means actual damage is moderate. The damage is also used as the **berserk timer duration in frames** (per CHAOS_DRONE doc §3.2: `this->berserk_timer (+0x29C) = new_damage`). So a 600-damage hit means 600 frames = ~40 seconds of berserk state. |
| `ROF=45` | ~3 sec between gas emissions. |
| `Range=3` | 3-cell engagement range. The `;1.83;GEF Since we have a new long range gas effect...` inline note documents the design: range was originally 1.83 cells (close-up) but bumped to 3 once the long-range gas effect (CellSpread=3 on the warhead) was added — drone now stays just outside its own gas radius. |
| `Projectile=InvisibleLow` | Bookkeeping invisible projectile. |
| `Speed=30` | Irrelevant. |
| `Warhead=PsychGasCreate` | **The Psychedelic warhead that triggers berserk.** See §3.3. |
| `Report=ChaosDroneAttack` | Attack sound (sound:5503). |
| `;Burst=4` | (commented) — would have fired 4 gas bursts per command; disabled (single-shot per fire cycle). |
| `OmniFire=yes` | Fire in any direction (since Turret=no). |
| `AreaFire=yes` | **WeaponType @ 0x0077283e (NEW).** Verified WeaponType scope. Marks this as an area-effect weapon (used by Desolator deploy, Chaos Drone gas, similar AoE weapons). |
| `;Anim=CDGAS` | (commented) — the gas cloud animation; the warhead `AnimList=CDGAS` handles it now instead. |

### 3.2 `[VirtualScanner]` — secondary (target-scanner extender)

```ini
[VirtualScanner]; This is so units with range one weapons will scan out farther when looking for targets in guard
Damage=1
Range=5
NeverUse=yes
Projectile=InvisibleAll
Warhead=SA
Speed=100
```

| Key | Effect |
|-----|--------|
| Inline comment | **"This is so units with range one weapons will scan out farther when looking for targets in guard"** — explains the entire purpose. |
| `Damage=1` | Token. |
| `Range=5` | **5 cells** — extends the drone's target-acquisition reach beyond the Primary's 3-cell range. |
| `NeverUse=yes` | **WeaponType @ 0x0077216f (NEW).** Verified WeaponType scope. **The weapon will never actually be fired** — it exists only to influence target-acquisition logic. The engine uses Secondary weapon's Range for guard/auto-acquire scanning even if NeverUse=yes prevents actual firing. |
| `Projectile=InvisibleAll` | Bookkeeping. |
| `Warhead=SA` | Some standard anti-something warhead (irrelevant since never fires). |
| `Speed=100` | Irrelevant. |

> **Mechanism:** With `Range=3` on Primary, the drone would only consider gassing targets within 3 cells. By setting Secondary to `Range=5 NeverUse=yes`, the scanning code uses the wider 5-cell radius for target acquisition, while actual firing still uses Primary at 3 cells. The drone moves into 3-cell range to fire when an enemy enters the wider 5-cell scan radius. Sight=6 covers the visual aspect.

### 3.3 `[PsychGasCreate]` warhead — THE berserk trigger

```ini
[PsychGasCreate]
CellSpread=3
PercentAtMax=1
Verses=100%,100%,100%,50%,50%,50%,0%,0%,0%,100%,100%
InfDeath=1
AnimList=CDGAS
Psychedelic=yes
```

| Key | Effect |
|-----|--------|
| `CellSpread=3` | **3-cell radius gas cloud.** |
| `PercentAtMax=1` | Full damage at edge (uniform within radius). |
| `Verses=100%,100%,100%,50%,50%,50%,0%,0%,0%,100%,100%` | **100% vs infantry classes (0/1/2), 50% vs vehicle classes (3/4/5), 0% vs all building classes (6/7/8), 100% vs special_1/special_2 (9/10).** Buildings are immune. |
| `InfDeath=1` | Infantry hit die via small-arms death anim (the gas doesn't visually mangle them). |
| `AnimList=CDGAS` | Spawns the `CDGAS` particle/animation at the impact cell — the visible green-purple gas cloud. |
| `Psychedelic=yes` | **WarheadType +0x16D (verified at 0x0075d8ea in CHAOS_DRONE doc §2.1).** **The flag that triggers the berserk state machine.** When set, per-target dispatch in `TechnoClass::ReceiveDamage` enters the Psychedelic branch (CHAOS_DRONE doc §3.2): if target is not allied, not psionic-immune, not a building, then set `target.berserk_flag (+0x298) = 1` and `target.berserk_timer (+0x29C) = computed_damage_value`. Additionally: if target is a vehicle with a team, remove from team via `TeamClass::RemoveMember`; reset target's archive target; queue `Mission_Hunt` so it starts seeking targets. |

> **Filter chain (verified, see CHAOS_DRONE_BERSERK_GHIDRA_REPORT §3.2):**
> 1. `HouseClass::IsAlliedWith(target_owner, source_owner)` → if allied: return 0 (no damage, no berserk).
> 2. `target_type.ImmuneToPsionics (+0xD35)` → if yes: return 0.
> 3. `target.WhatAmI() == 6` (BuildingClass discriminator) → if yes: return 0.
> 4. Compute damage via `FUN_00489180`, store as berserk_timer in frames.
> 5. If `berserk_flag == 0`: set to 1, team-decouple, queue Mission_Hunt, reset archive target.
> 6. Subsequent gas hits **refresh the timer only** (re-enter the same flow but the `if (berserk_flag == 0)` gate prevents re-team-decouple/re-mission-queue).

### 3.4 `[PsychGas]` warhead — sustained gas cloud (PsychCloudSys / GasCloudSys)

The gas cloud's particle system (`AnimList=CDGAS` spawns a particle anim) continues to emit damage hits using `[PsychGas]` warhead on units that remain in the cloud:

```ini
[PsychGas]
;Spread=512
CellSpread=1
PercentAtMax=1
Verses=100%,100%,100%,50%,50%,50%,25%,25%,25%,100%,100%
;Verses=200%,150%,100%,20%,0%
InfDeath=1
Particle=GasCloudSys
ProneDamage=300%    ; Gas concentrates at gound level
Psychedelic=yes
```

- `CellSpread=1` — smaller per-tick radius.
- `Verses=100/100/100/50/50/50/25/25/25/100/100` — Same anti-vehicle, anti-infantry pattern but **25% vs buildings** (so the lingering cloud DOES damage buildings, unlike the initial PsychGasCreate burst).
- `Particle=GasCloudSys` — emitted from the gas particle system.
- `ProneDamage=300%` — **Prone infantry take 3× damage.** Inline comment: "Gas concentrates at ground level". Deployed GI/Conscripts take massive damage from sustained gas.
- `Psychedelic=yes` — same flag, but per CHAOS_DRONE §3.2 the berserk state is already set by the initial PsychGasCreate hit; subsequent PsychGas hits only refresh the timer.

### 3.5 No Elite variant

Notable: **`Trainable=no` on the chassis means the Chaos Drone never ranks up.** No `ElitePrimary=` is defined. The drone fights at rookie stats permanently. This is unusual for a tier-3-cost (1000 credits) unit but consistent with "drone" design semantics (drones are disposable tools, not crew).

### 3.6 Projectiles

`Projectile=InvisibleLow` — bookkeeping invisible projectile that follows a low trajectory. `Projectile=InvisibleAll` — universal-direction invisible.

---

## 4. Voice & sound catalogue

| Slot | Sound key | sndmd entry | Audio clip(s) |
|------|-----------|-------------|---------------|
| `VoiceSelect` | `ChaosDroneSelect` | sound:1338 | unique select |
| `VoiceMove` | `ChaosDroneMove` | sound:1348 | unique move |
| `VoiceAttack` | `ChaosDroneAttackCommand` | sound:1343 | unique attack |
| `VoiceFeedback` | (empty) | — | — |
| `DieSound` | `ChaosDroneDie` | sound:1353 | unique death |
| `MoveSound` | `ChaosDroneMoveStart` | sound:5299 | one-shot engine start (note INI leading space in `MoveSound= ChaosDroneMoveStart` — harmless) |
| `ChaosAttack Report` | `ChaosDroneAttack` | sound:5503 | gas-emit attack sound |

Chaos Drone has 6 unique `ChaosDrone*` sound entries — relatively rich audio set for a tier-3 utility unit.

---

## 5. Owners / prerequisites / tech gating

- **Buildable by:** `YuriCountry` only.
- **Prerequisite:** `YAWEAP` only — Yuri War Factory. **No Battle Lab requirement** (unlike DISK/MIND/TELE which need YATECH).
- **TechLevel:** 4 (lower than tier-3 hardcoded units which sit at TechLevel 2; the higher TechLevel here just means it shows in build list at TechLevel ≥4 — but with no tech prerequisite, it's effectively available as soon as the War Factory is built).
- **Cost:** 1000 — same as Magnetron.
- `AllowedToStartInMultiplayer=no` → not pre-built.
- `CrateGoodie=no` → not from crates.

---

## 6. Veterancy

**None.** `Trainable=no` — the Chaos Drone does NOT gain veterancy ranks. No Veteran, no Elite — only Rookie stats. No `VeteranAbilities=` or `EliteAbilities=` block in the INI (notice their absence vs DISK/TELE/MIND blocks).

---

## 7. Hardcoded behavior — Ghidra-verified

### 7.1 String-name scan

- `search_strings "Psychedelic"` (not run directly here; CHAOS_DRONE doc verified at 0x00847d30 → WarheadType+0x16D via 0x0075d8ea).
- `search_strings "BerserkFriendly"` → 0x008439f8 → TechnoType @ 0x007148fa (verified — see CHAOS_DRONE doc §2.2).
- `search_strings "AreaFire"` → 0x008492f4 → WeaponType @ 0x0077283e (NEW THIS DOC).
- `search_strings "NeverUse"` → 0x008494f0 → WeaponType @ 0x0077216f (NEW THIS DOC).
- `search_strings "AccelerationFactor"` → 0x008443e0 → TechnoType @ 0x007124bc (NEW THIS DOC).
- `search_strings "DeaccelerationFactor"` → 0x008443f4 → TechnoType @ 0x0071249b (NEW THIS DOC).
- `search_strings "Deployer"` → 0x00825928 → **InfantryType @ 0x0052460d (NEW THIS DOC)** — confirms `Deployer=yes` is InfantryType-only; on CAOS VehicleType it's an orphan INI key.

### 7.2 Verified field scopes (new this doc)

| Field | Scope | Address |
|-------|-------|---------|
| `Psychedelic=yes` (Warhead) | WarheadType +0x16D | 0x0075d8ea (per CHAOS_DRONE doc) |
| `BerserkFriendly=yes` | TechnoType +0x690 | 0x007148fa (per CHAOS_DRONE doc) |
| `AreaFire=yes` (Weapon) | WeaponType | **0x0077283e** (NEW) |
| `NeverUse=yes` (Weapon) | WeaponType | **0x0077216f** (NEW) |
| `AccelerationFactor=N` | TechnoType | **0x007124bc** (NEW) |
| `DeaccelerationFactor=N` | TechnoType | **0x0071249b** (NEW) |
| `Deployer=yes` | **InfantryType only** | **0x0052460d** (NEW — confirms infantry-only scope) |
| `CanRetaliate=no` | TechnoType | 0x0071448d (cheat sheet) |
| `ImmuneToRadiation=yes` | TechnoType | cheat-sheet entry |
| `Parasiteable=yes` | TechnoType | 0x00714f86 (cheat sheet) |
| `SuppressionThreshold=N` | TechnoType | 0x0071506d (cheat sheet) |
| `DefaultToGuardArea=yes` | TechnoType | 0x00714f44 (cheat sheet) |
| `NavalTargeting=6` | TechnoType | 0x007121be |
| `ReselectIfLimboed=yes` | TechnoType | 0x007142b4 |

### 7.3 Berserk state machine (cross-ref summary)

From [`CHAOS_DRONE_BERSERK_GHIDRA_REPORT.md`](../../CHAOS_DRONE_BERSERK_GHIDRA_REPORT.md):

**Set phase** (in `TechnoClass::ReceiveDamage` @ 0x00701900):
- Filter: not allied + not ImmuneToPsionics + not a building.
- Compute damage via `FUN_00489180` → store as berserk_timer (in frames) at TechnoClass+0x29C.
- If berserk_flag (+0x298) was 0:
  - Set berserk_flag = 1.
  - If target is vehicle (flag & 4) with TeamPtr (+0x5D4): call `TeamClass::RemoveMember` to decouple from AI team.
  - Reset target's ArchiveTarget via vtable[0x3C8].
  - Queue `Mission_Hunt` (0x0F) via vtable[0x1E8].

**Decrement phase** (in `TechnoClass::AI_Update` @ 0x006F9E50):
- Each tick, decrement berserk_timer (+0x29C).
- When timer reaches 0 or below: clear berserk_flag (+0x298) = 0.

**Effect phase** (in `TechnoClass::Scan_Cell_For_Target` @ 0x006F8960):
- During target scanning, berserk_flag bypasses the alliance-filter — every candidate (friend or foe) is evaluated for attack.
- Combined with the queued Mission_Hunt: the unit actively seeks targets, with alliance bypassed.

**Friendly-fire immunity** (in `TechnoClass::GetFireError` @ 0x006FC1E1):
- If attacker.berserk_flag != 0 AND target.type.BerserkFriendly (+0x690) != 0: return FIRE_ILLEGAL.
- **The Chaos Drone is BerserkFriendly=yes**, so berserk units it creates cannot target it back. Yuri's other tier-3 units would NOT be BerserkFriendly (only specific designs).

### 7.4 Why CanPassiveAquire=no AND CanRetaliate=no

Both flags are set to prevent the drone from accidentally chaos-gassing friendly units. The drone is unintentionally dangerous to its own side — even though `BerserkFriendly=yes` protects it from incoming berserk retaliation, its initial gas attack would still damage friendly units in the 3-cell radius and possibly cascade into friendly-fire chains (a friendly unit briefly gassed could attack other friendlies before the berserk timer expires).

**By forcing manual targeting (CanPassiveAquire=no) and no retaliation (CanRetaliate=no)**, the Chaos Drone is a *deliberate* weapon — the player picks every target and the drone does nothing unprompted. This is consistent with the same pattern on MIND (Master Mind) and TELE (Magnetron), all of which have powerful disruptive effects requiring careful tactical placement.

### 7.5 The "Deployer=yes" orphan and the "MovementZone FLAW" comment

Two INI peculiarities document engine quirks/bugs:

1. **`Deployer=yes` on a VehicleType**: This key is read only by InfantryTypeClass (verified at 0x0052460d). On CAOS it parses without binding — **no engine effect**. Likely copy-paste from an InfantryType (perhaps the Desolator, which is a Deployer). Cosmetic INI noise.

2. **`;MovementZone=Normal ;gs FLAW needs to be changed to this when The Flaw is fixed`**: Author comment documenting a known engine bug. The drone *should* use MovementZone=Normal for proper pathing, but "The Flaw" (likely a pathfinder bug specific to small/light units) forced fallback to MovementZone=Destroyer. The bug was never fixed; the comment remains as historical record.

These are useful reminders that the INI files contain authoring drift and developer notes alongside live values — not every key is necessarily live.

---

## 8. TS-legacy filter

| Feature | Status in YR |
|---------|--------------|
| Locomotor `{4A582741-...}` = DriveLocomotionClass | Live in YR. |
| `{55D141B8-...}` MechLocomotionClass (in comment) | Inert — INI-commented after `;`. |
| `Psychedelic=yes` warhead flag | Live YR (Chaos-Drone-exclusive routing). |
| `BerserkFriendly=yes` flag | Live YR (TechnoType +0x690). |
| `AreaFire=yes` weapon flag | Live YR. |
| `NeverUse=yes` weapon flag | Live YR (used by VirtualScanner secondary). |
| `;Image=DRON`, `;Burst=4`, `;Anim=CDGAS`, `;Bombable=no`, `;SprayAttack=yes`, `;MovementZone=Normal` | INI comments — all disabled. |
| `;225; 175` historical Strength values | INI comments — old balance values. |
| `Deployer=yes` on VehicleType | **Orphan INI key** (InfantryType-only field; no effect on CAOS). Not strictly TS-legacy but functionally dead. |
| Fog-of-war 0x1000 gate | Not on CAOS. |
| ImmuneToVeins / Subterranean / Tunneling | Not on CAOS. |
| `Tiberium=yes` on warhead | Not present here (Chaos Drone warheads don't have it). |

---

## 9. Coverage audit

| Section | Coverage |
|---------|----------|
| rulesmd `[CAOS]` — every key | ✅ §1 (51 keys including 8 commented draft/annotation entries) |
| artmd `[CAOS]` — every key | ✅ §2 (4 keys; no FLH/TurretOffset noted) |
| `[ChaosAttack]` weapon | ✅ §3.1 (10 keys + 2 commented) |
| `[VirtualScanner]` weapon | ✅ §3.2 (NeverUse=yes scan-extender pattern) |
| `[PsychGasCreate]` warhead | ✅ §3.3 + cross-ref to CHAOS_DRONE doc berserk chain |
| `[PsychGas]` warhead (sustained cloud) | ✅ §3.4 |
| No elite variant | ✅ §3.5 (Trainable=no, no ElitePrimary) |
| Projectiles | ✅ §3.6 |
| Voices / sounds (7 bindings) | ✅ §4 |
| Owners / prereqs / tech | ✅ §5 |
| Veterancy | ✅ §6 (Trainable=no, no ranks) |
| Hardcoded behavior — Ghidra-verified | ✅ §7 (**5 NEW field-scope verifications added to cheat sheet**: AreaFire @ 0x0077283e WeaponType, NeverUse @ 0x0077216f WeaponType, AccelerationFactor @ 0x007124bc TechnoType, DeaccelerationFactor @ 0x0071249b TechnoType, Deployer @ 0x0052460d **InfantryType-only**; full berserk state machine cross-ref to CHAOS_DRONE doc) |
| TS-legacy filter | ✅ §8 |
| Cross-references (CHAOS_DRONE_BERSERK, DISK, TELE, MIND, DRON) | ✅ at top + inline |
| INI quirks: orphan Deployer key, MovementZone FLAW comment | ✅ §7.5 |

---

## 10. Quick implementer summary

To make a CAOS-equivalent:

1. **Render** — voxel + HVA; no turret, no FLH offset (weapon emits from body center).
2. **Movement** — DriveLocomotionClass (standard ground); fast (Speed=8, ROT=40); MovementZone=Destroyer (live-INI value despite author comment about "FLAW").
3. **Primary attack (ChaosAttack → PsychGasCreate)** —
   - Invisible projectile fired at target cell (Range=3, AreaFire=yes).
   - On detonate: `PsychGasCreate` warhead with Psychedelic=yes flag, CellSpread=3.
   - Per-target in radius (filter: not allied, not psionic-immune, not building):
     - Apply Verses-scaled damage (100/100/100/50/50/50/0/0/0/100/100).
     - Set berserk_flag = 1 (or refresh if already set).
     - Set berserk_timer = damage value in frames.
     - First-time only: decouple from AI team, reset ArchiveTarget, queue Mission_Hunt.
4. **Sustained cloud** — `AnimList=CDGAS` spawns persistent particle anim that emits `[PsychGas]` warhead hits at 1-cell radius for the cloud's duration; prone infantry take 3× damage.
5. **Berserk state machine** — see §7.3 / CHAOS_DRONE doc:
   - Set in ReceiveDamage Psychedelic branch.
   - Decrement in AI_Update per tick.
   - Clear when timer reaches 0.
   - Effect: bypass alliance filter in Scan_Cell_For_Target → target friends and foes.
   - Friendly-fire immunity for any unit with `BerserkFriendly=yes`.
6. **Secondary (VirtualScanner)** — NeverUse=yes weapon with Range=5; extends target-acquisition scan radius without ever actually firing.
7. **No veterancy** — Trainable=no, no rank progression, no Elite swap. Permanent rookie stats.
8. **Self-protection flags** —
   - `BerserkFriendly=yes` → immune to berserk friendly fire.
   - `CanPassiveAquire=no` → no auto-target.
   - `CanRetaliate=no` → no auto-return-fire.
   - `ImmuneToPsionics=yes` → cannot be mind-controlled.
   - `ImmuneToRadiation=yes` → ignores rad damage.
9. **Audio** — Chaos Drone-unique voice set (Select/Move/Attack/Die).
10. **AI flags** — `DefaultToGuardArea=yes` (guard-area default mission); `ThreatPosed=25`.
11. **Build gate** — YAWEAP prerequisite (no Battle Lab); YuriCountry only.

The Chaos Drone is one of the few units in YR that requires a dedicated state-machine field on the target TechnoClass (berserk_flag + berserk_timer). The implementation chain is small but spans 4 functions: ReceiveDamage (set), AI_Update (decrement/clear), Scan_Cell_For_Target (alliance bypass), GetFireError (BerserkFriendly target gate). Cross-reference CHAOS_DRONE_BERSERK_GHIDRA_REPORT for all addresses and disassembly.
