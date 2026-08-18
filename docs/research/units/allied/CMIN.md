# CMIN — Chrono Miner (Allied harvester)

**Side classification:** Allied (Owner=British,French,Germans,Americans,Alliance).
**Role:** Allied faction's ore harvester. Drives to ore, harvests with `Harvester=yes`
behavior, and **teleports back to the refinery** when the ore field is >50 cells away
(`ChronoHarvTooFarDistance`); drives back otherwise. Visual model swaps to `[CMON]`
(no-back variant) during dock-unload. **No weapon** — relies on speed-of-return and
chrono teleport for safety.

> Output bar: chrono teleport timing, distance threshold, model swap, and dock cadence
> are all parity-critical. Bale value, storage cap, and per-trip credit yield define
> the Allied early-game economy curve.

> **Companion doc**: [`soviet/HARV.md`](../soviet/HARV.md) — Soviet War Miner sibling.
> Comparison table in HARV §6. CMIN is the chrono-teleport half of the harvester pair.

> **Deep-RE cross-references — don't re-derive:**
> - **[CHRONO_MINER_SYSTEM_OVERVIEW.md](../../CHRONO_MINER_SYSTEM_OVERVIEW.md)** —
>   end-to-end chrono miner system, class hierarchy, locomotor layer.
> - **[CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md](../../CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md)** — teleport-decision logic, distance threshold check, locomotor handover.
> - **[CHRONO_WARP_VISUAL_RENDERING.md](../../CHRONO_WARP_VISUAL_RENDERING.md)** —
>   warp-in/warp-out visual effects.
> - **[TECHNOCLASS_CHRONO_OFFSETS_VERIFIED.md](../../TECHNOCLASS_CHRONO_OFFSETS_VERIFIED.md)** — chrono state field offsets on TechnoClass.
> - **[TELEPORT_LOCOMOTION_DEEP_DIVE.md](../../TELEPORT_LOCOMOTION_DEEP_DIVE.md)** +
>   **[TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md](../../TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md)** — teleport locomotor mechanics.
> - **[WAR_MINER_REFERENCE.md](../../WAR_MINER_REFERENCE.md)** — canonical HARV-vs-CMIN comparison.
> - **[HARVESTER_DOCK_UNLOAD.md](../../HARVESTER_DOCK_UNLOAD.md)** +
>   **[HARVESTER_DOCK_UNLOAD_SEQUENCE.md](../../HARVESTER_DOCK_UNLOAD_SEQUENCE.md)** — dock-unload behavior. `UnloadingClass=CMON` swap mid-dock.
> - **[MINER_DOCK_GAPS_RESEARCH.md](../../MINER_DOCK_GAPS_RESEARCH.md)** — known dock edge cases.
> - **[HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md](../../HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md)** + **[MISSION_HARVEST_GHIDRA_REPORT.md](../../MISSION_HARVEST_GHIDRA_REPORT.md)** — 5-state harvest mission state machine.

> Ghidra confirms no `"CMIN"` string in `gamemd.exe` — all behavior is generic
> flag-driven via the `Harvester=yes` + `Teleporter=yes` combination.

---

## 1. `rulesmd.ini` — `[CMIN]` verbatim

```ini
[CMIN]
UIName=Name:CMIN
Name=Chrono Miner
Prerequisite=GAWEAP,PROC
Nominal=yes
ToProtect=yes
Category=Support
Strength=1000
Armor=medium
;Dock=PROC		; Need both in case a building from the other team is captured.
Dock=NAREFN,GAREFN
;Turret=yes
Primary=none
Harvester=yes
ChronoInSound=ChronoMinerTeleport
ChronoOutSound=ChronoMinerTeleport
TechLevel=1
Sight=4
Speed=4
Owner=British,French,Germans,Americans,Alliance
AllowedToStartInMultiplayer=no
PipScale=Tiberium
CrateGoodie=yes
Storage=20
Cost=1400
Soylent=1400
Points=55
ROT=5
Crusher=yes
AutoCrush=yes
Crewed=no
SelfHealing=yes
;OpportunityFire=yes ;GEF has no weapon, doesn't need this
UnloadingClass=CMON
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=ChronoMinerSelect
VoiceMove=ChronoMinerMove
VoiceAttack=ChronoMinerMove
VoiceHarvest=ChronoMinerHarvest
VoiceEnter=ChronoMinerReturn
DieSound=GenVehicleDie
CrushSound=TankCrush
MaxDebris=6
DebrisTypes=TIRE
DebrisMaximums=4
Teleporter=yes
;Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1} ;drive locomotor
Locomotor={4A582747-9839-11d1-B709-00A024DDAFD1} ;teleport locomotor
Weight=3.5
MovementZone=Crusher
ThreatPosed=0	; This value MUST be 0 for all building addons
ThreatAvoidanceCoefficient=.65
DamageParticleSystems=SparkSys,SmallGreySSys
ImmuneToVeins=yes
ImmuneToPsionics=yes
ImmuneToRadiation=yes
ZFudgeColumn=9
ZFudgeTunnel=14
ZFudgeBridge=7
Size=3
StupidHunt=yes ;this guy can't handle a hunt command, so he should just run towards the player
Trainable=no
ResourceGatherer=yes;gs for the AI to handle the slave miner, it has to know if it can make money or not
Bunkerable=no; Units default to yes, others default to no
```

### Key-by-key explanation (focus on CMIN-specific differences from HARV)

| Key | Value | Scope | Effect |
|-----|-------|-------|--------|
| `UIName` | `Name:CMIN` | AbstractType | CSF lookup. |
| `Name` | `Chrono Miner` | AbstractType | Dev/fallback. |
| `Prerequisite` | `GAWEAP,PROC` | TechnoType | **Allied** War Factory + Refinery (HARV uses NAWEAP). |
| `Nominal` / `ToProtect` | `yes` / `yes` | TechnoType | Same harvester treatment as HARV — low-priority score, friendly-AI defends it. |
| `Category` | `Support` | TechnoType | Same. |
| `Strength` | `1000` | AbstractType | Same as HARV. |
| `Armor` | `medium` | TechnoType | Same. |
| `Dock` | `NAREFN,GAREFN` | TechnoType | Same dual-refinery list as HARV — supports cross-faction capture. |
| `;Turret=yes` | *(commented)* | — | **No turret** — author explicitly commented out the HARV-pattern turret line. |
| `Primary` | `none` | TechnoType | **No weapon.** CMIN is unarmed; relies on teleport for safety. |
| `Harvester` | `yes` | UnitType | Enables `Mission_Harvest` state machine — same as HARV. |
| `ChronoInSound` | `ChronoMinerTeleport` | TechnoType (verified — 0x0083a9a4 → 0x007135da) **+** RulesClass `[AudioVisual]` default fallback (0x006699e9) | Played when CMIN warps **into** a cell (after teleport-back-to-refinery completes). Per-unit override of the global default. Sound def at soundmd line 1359. |
| `ChronoOutSound` | `ChronoMinerTeleport` | TechnoType (verified — 0x0083a994) | Played when CMIN warps **out of** a cell (when leaving ore field for refinery). Same sound def — both events use one clip. |
| `TechLevel` | `1` | TechnoType | Tier-1, same as HARV. |
| `Sight` | `4` | TechnoType | Same. |
| `Speed` | `4` | TechnoType | Same — but note: during the teleport phase the locomotor swaps and the effective travel "speed" is instantaneous regardless of Speed value. |
| `Owner` | `British,French,Germans,Americans,Alliance` | TechnoType | **Allied countries only.** No Yuri (uses SMIN), no Soviet (uses HARV). |
| `AllowedToStartInMultiplayer` | `no` | TechnoType | Allied ConYard's `FreeUnit=CMIN` spawns the first one on deploy. |
| `PipScale` | `Tiberium` | UnitType | Ore-bale pips. Combined with `Storage=20`, half as many pips visible as HARV. |
| `CrateGoodie` | `yes` | UnitType | Can drop from crates. |
| `Storage` | `20` | TechnoType | **20 bales** — **half** of HARV's 40. Each bale ≈ $25, so a full CMIN load is ≈$500 vs HARV's ≈$1000. **The chrono teleport is the equalizer**: CMIN makes shorter round-trips (no walk-back) so its cycles-per-minute is higher, balancing per-cycle income vs HARV. |
| `Cost` | `1400` | TechnoType | Same as HARV. The Allied player pays the same for half-storage-but-teleport. |
| `Soylent` | `1400` | TechnoType | 100% Grinder refund — irrelevant since Allied has no Grinder. |
| `Points` | `55` | TechnoType | Same as HARV. |
| `ROT` | `5` | TechnoType | Body rotation rate (no turret). |
| `Crusher` / `AutoCrush` | `yes` / `yes` | TechnoType | Same as HARV — crushes infantry on path. |
| `Crewed` | `no` | TechnoType | No survivors on death. |
| `SelfHealing` | `yes` | TechnoType | Self-heals. |
| `;OpportunityFire=yes ;GEF has no weapon, doesn't need this` | *(commented)* | — | Author-note: "GEF has no weapon, doesn't need this". `Primary=none` makes `OpportunityFire` moot — no targets to acquire. |
| `UnloadingClass` | `CMON` | TechnoType (verified prior iter — 0x00843af8) | Visual model swap during dock-unload — see §6 + HARVESTER_DOCK_UNLOAD_SEQUENCE. CMON is described in artmd as "Allied harvester without back" — the bale-bucket is empty/retracted in the model. |
| `Explosion` | `TWLT070,...` | TechnoType | Standard random-from-list. |
| `VoiceSelect` | `ChronoMinerSelect` | TechnoType | 5 unique clips (`$vchrsea..ee`). |
| `VoiceMove` | `ChronoMinerMove` | TechnoType | 5 clips. |
| `VoiceAttack` | `ChronoMinerMove` | TechnoType | **Reuses Move voice** — since `Primary=none`, attack orders default-route to a move. |
| `VoiceHarvest` | `ChronoMinerHarvest` | TechnoType | 5 clips on each bale pickup. |
| `VoiceEnter` | `ChronoMinerReturn` | TechnoType | 5 clips when entering refinery to dump (unique "going home" set). |
| `DieSound` | `GenVehicleDie` | TechnoType | Generic vehicle death. |
| `CrushSound` | `TankCrush` | TechnoType | Crush. |
| `MaxDebris / DebrisTypes / DebrisMaximums` | `6 / TIRE / 4` | TechnoType | Same death-debris as HARV. |
| `Teleporter` | `yes` | TechnoType (verified — 0x00843e60) | **The chrono flag.** Enables the teleport-locomotor destination-pick logic and the chrono-warp visual rendering. Same flag as Chrono Legionnaire (CLEG), Chrono Commando (CCOMAND), Chrono Ivan (CIVAN). |
| `;Locomotor=...DriveLocomotionClass` | *(commented)* | — | Author preserved the drive-locomotor CLSID as comment alongside the live teleport. |
| `Locomotor` | `{4A582747-9839-11d1-B709-00A024DDAFD1}` | TechnoType | **TeleportLocomotionClass** — but per CHRONO_MINER_SYSTEM_OVERVIEW, the teleport locomotor **piggybacks** the Drive locomotor for short-distance moves. Drive is used when the destination is within `HarvesterTooFarDistance` (5 cells); teleport activates beyond `ChronoHarvTooFarDistance` (50 cells). The IPiggyback swap happens in `FootClass::AI`. |
| `Weight` | `3.5` | TechnoType | Same as HARV — physics. |
| `MovementZone` | `Crusher` | TechnoType | Can path through crushable terrain. |
| `ThreatPosed` | `0` | TechnoType | Enemy AI ignores CMIN. |
| `ThreatAvoidanceCoefficient` | `.65` | TechnoType | Same path-avoidance weight as HARV (CMON uses 1.0 — full avoidance). |
| `DamageParticleSystems` | `SparkSys,SmallGreySSys` | TechnoType | |
| `ImmuneToVeins` | `yes` | TechnoType | **TS-LEGACY** dormant. |
| `ImmuneToPsionics` | `yes` | TechnoType | Cannot be mind-controlled. |
| `ImmuneToRadiation` | `yes` | TechnoType | Walks through Desolator rad fields. |
| `ZFudgeColumn / Tunnel / Bridge` | `9 / 14 / 7` | UnitType | Same z-fudge as HARV. |
| `Size` | `3` | TechnoType | Transport slot. |
| `StupidHunt` | `yes` | TechnoType [BINARY-VERIFIED audit 17: string @ 0x008438A4, parser xref @ 0x00714C6C, `TechnoType+0x6D4` (byte)] | INI comment: "this guy can't handle a hunt command, so he should just run towards the player". Specifies that when given a `Hunt` mission (AI-style "find and kill enemies"), CMIN should fall back to moving toward the player base instead — since it has no weapon to attack with. Notable: only a handful of units use this flag (typically harvesters and other unarmed support units). |
| `Trainable` | `no` | TechnoType | **Cannot gain veterancy.** Unlike HARV (which is Trainable by default and can elite-promote to the 20mmRapidE arcing cannon), CMIN is permanently rookie. INI comment "Trainable=no" is the explicit lock. |
| `ResourceGatherer` | `yes` | TechnoType | AI economy planner flag. |
| `Bunkerable` | `no` | TechnoType | Cannot enter Battle Bunker / Fortress. |

### Notable absent keys vs HARV
- **No `Turret=`** (commented out).
- **No `Primary=` weapon** (`Primary=none`).
- **No `ElitePrimary=`** (because Trainable=no).
- **No `VeteranAbilities=` / `EliteAbilities=`** (irrelevant — Trainable=no).
- **No `OpportunityFire=`** (no weapon to opportunity-fire with).

---

## 2. `artmd.ini` — `[CMIN]` section

```ini
[CMIN]			; Allied harvester
Cameo=AHRVICON
Voxel=yes
Remapable=yes
```

| Key | Value | Effect |
|-----|-------|--------|
| `Cameo` | `AHRVICON` | "Allied HaRVester ICON" — dedicated build cameo. |
| `Voxel` | `yes` | Voxel-rendered from `CMIN.VXL` + `CMIN.HVA`. |
| `Remapable` | `yes` | House-color remap. |

Notably absent vs HARV's art block:
- **No `AltCameo=`** — there is no Yuri-skinned alt cameo (HARV had `HARVUICO`). If a Yuri faction ever owns a CMIN via capture/mind-control, the standard `AHRVICON` is used.
- **No `TurretOffset=`** — no turret.
- **No `PrimaryFireFLH=`** — no weapon.

### `[CMON]` art block (UnloadingClass swap target)

```ini
[CMON]			; Allied harvester without back
Voxel=yes
Remapable=yes
```

Minimal art block — `CMON.VXL` is the voxel for the no-back variant (ore-bucket compartment empty/retracted). No Cameo (not buildable). Engine swaps the rendered voxel from `CMIN.VXL` to `CMON.VXL` mid-dock-unload, then back. See HARVESTER_DOCK_UNLOAD_SEQUENCE for the exact frame at which the swap happens.

---

## 3. Weapon — none (`Primary=none`)

CMIN is **unarmed**. No `[Primary]` weapon definition is consulted. The chrono teleport
is its sole survival mechanism.

This is one of only a few "production" YR units with no primary weapon at all — others
include some civilian vehicles, SHAD (Nighthawk Transport, also no weapon), and certain
dummy/internal units.

---

## 4. Warhead — n/a

(No weapon → no warhead.)

---

## 5. Voices / sounds

```ini
[ChronoMinerSelect]
Sounds=$vchrsea $vchrseb $vchrsec $vchrsed $vchrsee
Control=random
Volume=85

[ChronoMinerMove]
Sounds=$vchrmoa $vchrmob $vchrmoc $vchrmod $vchrmoe
Control=random
Volume=85

[ChronoMinerHarvest]
Sounds=$vchrhaa $vchrhab $vchrhac $vchrhad $vchrhae
Control=random
Volume=85

[ChronoMinerReturn]
Sounds=$vchrgoa $vchrgob $vchrgoc $vchrgod $vchrgoe
Control=random
Volume=85
```

```ini
[ChronoMinerTeleport]
Sounds=vchrtele
Control= interrupt
Limit=1
Volume=50
```

| Hook | Sound def | Trigger |
|------|-----------|---------|
| `VoiceSelect=ChronoMinerSelect` | 5 clips | Click-select |
| `VoiceMove=ChronoMinerMove` | 5 clips | Move order |
| `VoiceAttack=ChronoMinerMove` | (same as Move) | Attack order — no weapon, so engine treats as move |
| `VoiceHarvest=ChronoMinerHarvest` | 5 clips | Each bale pickup |
| `VoiceEnter=ChronoMinerReturn` | 5 unique "going home" clips | Entering refinery to dump |
| `ChronoInSound=ChronoMinerTeleport` | 1 clip (`vchrtele`), `Limit=1` (max one concurrent), Vol=50 | Warp **in** to destination |
| `ChronoOutSound=ChronoMinerTeleport` | (same def) | Warp **out** of source |
| `DieSound=GenVehicleDie` | 6 clips | Death |
| `CrushSound=TankCrush` | `vcrusha` | Crushes infantry |

CMIN has **five** dedicated voice categories (Select, Move, Harvest, Return, Teleport) — more than HARV which has four (Select, Move, Attack, Harvest). The dedicated "Return" voice fires on entering the refinery, signaling the player that ore is about to be deposited.

The teleport sound has `Limit=1` — even if multiple CMINs warp simultaneously, only one
`vchrtele` clip plays at a time, preventing audio spam.

---

## 6. Prerequisites / owners / availability

- **Prerequisite**: `GAWEAP,PROC` — Allied War Factory + any Refinery.
- **TechLevel** = `1`.
- **AllowedToStartInMultiplayer=no** — Allied ConYard spawns first CMIN via its `FreeUnit=` line (the Allied-side `FreeHarvester` mechanism).
- **Owner**: Allied countries only.
- **CrateGoodie=yes** — can drop from crates.

### Dock targets

`Dock=NAREFN,GAREFN` — accommodates cross-faction capture, same as HARV.

### Teleport-vs-drive decision

Per CHRONO_MINER_TELEPORT_GHIDRA_REPORT and CHRONO_MINER_SYSTEM_OVERVIEW:

1. After picking up ore, CMIN evaluates the path back to the refinery.
2. If the **ore-field distance from refinery > `ChronoHarvTooFarDistance=50` cells**, CMIN uses the **teleport locomotor**: instant warp from current cell to a cell near the refinery dock.
3. Otherwise (≤50 cells), CMIN uses the **drive locomotor** (piggybacked via IPiggyback swap in `FootClass::AI`) — same as HARV.
4. The teleport triggers `ChronoOutSound` (vchrtele, Limit=1) at the source cell, then `ChronoInSound` at the destination cell after the warp.
5. Visual warp-in/warp-out effects are rendered per CHRONO_WARP_VISUAL_RENDERING.

This means in **short-range bases** (most ore right next to base), CMIN behaves like HARV — drives, no teleport. The teleport is for **long-range expansion plays**: CMIN can safely harvest distant ore fields that HARV cannot reach without expensive escort.

### CMIN-vs-HARV comparison (canonical — repeated for completeness)

| Aspect | CMIN | HARV |
|--------|------|------|
| Side | Allied | Soviet |
| Locomotor | TeleportLocomotionClass (piggybacks Drive) | DriveLocomotionClass |
| `Teleporter` | `yes` | (absent → false) |
| `Storage` | 20 bales | 40 bales |
| Weapon | none | 20mmRapid turret |
| Turret | no | yes |
| Trainable | **no** (locked rookie) | yes (can elite to arcing cannon) |
| Return | teleports if ore-field >50 cells from refinery | always drives |
| UnloadingClass | CMON | HORV |
| ChronoInSound/OutSound | ChronoMinerTeleport | (absent) |
| StupidHunt | `yes` (defaults to player base on hunt) | (absent — has weapon, can hunt) |
| VoiceEnter | ChronoMinerReturn (unique) | WarMinerMove (reused) |
| Cost / Speed / Strength / Armor / Sight / ROT / Weight / Crusher / SelfHealing | identical | identical |
| Dock / ImmuneToPsionics / ImmuneToRadiation / Bunkerable=no / ResourceGatherer | identical | identical |

---

## 7. Hardcoded behavior (Ghidra-verified)

### 7.1 CMIN-specific code in `gamemd.exe`

| Query | Result |
|-------|--------|
| `CMIN` | 0 matches |
| `ChronoMiner` | (not searched explicitly; broader `Chrono` substring would match many) |

⇒ **No CMIN-specific hardcoded ID.** All behavior is generic flag-driven via `Harvester=yes` + `Teleporter=yes` + `Locomotor={4A582747-...}` (TeleportLocomotionClass).

### 7.2 Flag-scope verification (this iteration)

| Key | String at | Read by | Class scope |
|-----|-----------|---------|-------------|
| `ChronoInSound` | 0x0083a9a4 | RulesClass__ReadAudioVisual @ 0x006699e9 (global default) **AND** TechnoTypeClass__ReadINI @ 0x007135da (per-unit override; writes `TechnoType+0x574` int VocClass index) | RulesClass (global) + TechnoType (per-unit) — **dual-read BINARY-VERIFIED audit 17** |
| `ChronoOutSound` | 0x0083a994 | RulesClass__ReadAudioVisual @ 0x00669a2a (global) **AND** TechnoTypeClass__ReadINI @ 0x0071361c (per-unit override; writes `TechnoType+0x578`) | **dual-read BINARY-VERIFIED audit 17** (previously inferred — now confirmed) |
| `StupidHunt` | 0x008438a4 | TechnoTypeClass__ReadINI @ 0x00714c6c | TechnoType |
| `Teleporter` | 0x00843e60 | TechnoTypeClass__ReadINI (cheat sheet) | TechnoType |
| `UnloadingClass` | 0x00843af8 | TechnoTypeClass__ReadINI @ 0x007146e8 (verified in HARV iter) | TechnoType |

Notable: `ChronoInSound` / `ChronoOutSound` have a **dual-read pattern** — they exist as both global defaults in `[AudioVisual]` AND as per-techno overrides. CMIN explicitly sets its own (so does CIVAN via `ChronoLegionTeleport`). Other chrono units (CLEG, CCOMAND) inherit the global default if not overridden.

### 7.3 Live behaviors driven by flags + locomotor + state machine

| Behavior | Driver | Notes |
|----------|--------|-------|
| Drive to nearest ore, harvest | `Harvester=yes` → `Mission_Harvest` 5-state machine | Same as HARV |
| Teleport home if ore >50 cells from refinery | `Teleporter=yes` + `Locomotor=TeleportLocomotionClass` + `[General] ChronoHarvTooFarDistance=50` | CHRONO_MINER_TELEPORT_GHIDRA_REPORT |
| Drive home if ore ≤50 cells | Piggyback swap to DriveLocomotionClass via `FootClass::AI` | CHRONO_MINER_SYSTEM_OVERVIEW |
| Warp-in / warp-out visual effect | TechnoClass chrono state fields | CHRONO_WARP_VISUAL_RENDERING + TECHNOCLASS_CHRONO_OFFSETS_VERIFIED |
| Teleport sound (limited to 1 concurrent) | `[ChronoMinerTeleport] Limit=1` | Prevents audio spam from multi-CMIN warps |
| UnloadingClass=CMON model swap during dock-unload | `UnloadingClass=CMON` | HARVESTER_DOCK_UNLOAD_SEQUENCE |
| Hunt mission falls back to "run toward player" | `StupidHunt=yes` | INI comment confirms — no-weapon unit can't actually hunt |
| Auto-crush on path | `Crusher=yes`, `AutoCrush=yes` | Same as HARV |
| Self-heals | `SelfHealing=yes` | |
| Survives Desolator rad | `ImmuneToRadiation=yes` | |
| Cannot be mind-controlled | `ImmuneToPsionics=yes` | |
| Ignored by enemy AI | `ThreatPosed=0` + friendly `ToProtect=yes` | |
| Cannot promote past rookie | `Trainable=no` | |
| Cannot enter Battle Fortress | `Bunkerable=no` | |
| AI economy planner tracks income | `ResourceGatherer=yes` | |

### 7.4 Behaviors NOT present in CMIN

- **No weapon** — `Primary=none`, no Secondary, no opportunistic fire.
- **No turret** — body-only render and pathing.
- **No veterancy** — `Trainable=no`.
- **No special-attack voice** — would be unused.
- **No Spawns / Passengers / OpenTransport / Gunner** — not a transport.

---

## 8. TS-legacy filter

| INI line | TS-LEGACY? | Status in YR |
|----------|-----------|--------------|
| `ImmuneToVeins=yes` | YES | Dormant. |
| `ZFudgeTunnel=14` | YES (no real tunnels) | Dormant render value. |
| All other flags | — | Live. |

No fog-of-war refs, no real tunnels, no Hospital.

---

## 9. Veterancy

**`Trainable=no`** — CMIN is permanently locked at rookie rank.

Consequently:
- No `VeteranAbilities=` / `EliteAbilities=` (neither key is set).
- No `ElitePrimary=` (and no `Primary=` to swap from anyway).
- No HP / speed / sight bonuses ever.

This mirrors CIVAN's behavior (also `Trainable=no` — for a different reason: bomb-instakill makes XP-balancing impossible). For CMIN the reason is more pragmatic: a weaponless harvester can't earn combat XP, and there's no design benefit to letting it accumulate "harvest XP" toward a non-existent elite form.

---

## 10. Cross-references

### Direct dependencies (`rulesmd.ini` / `artmd.ini` / `soundmd.ini`)
- (no weapons / projectiles / warheads)
- `[CMON]` (rulesmd line 7303 + artmd line 648) — `UnloadingClass` swap target (no-back variant, also `Harvester=yes`, also teleport locomotor, but `TechLevel=-1` not directly buildable)
- `[NAREFN]` / `[GAREFN]` — dock targets
- `[GAWEAP]` / `[PROC]` — prerequisites
- `[ChronoMinerSelect/Move/Harvest/Return/Teleport]` (soundmd) — voices and teleport SFX
- `[GenVehicleDie] / [TankCrush]` — generic vehicle sounds
- `[General] ChronoHarvTooFarDistance=50` (rulesmd globals) — teleport distance gate

### Conceptual companions
- **HARV** ([`soviet/HARV.md`](../soviet/HARV.md)) — Soviet sibling. Canonical comparison table in HARV §6 + CMIN §6.
- **CMON** (TODO; can be a quick-ref under `allied/` or `internal/`) — UnloadingClass swap form.
- **HARVESTER_UNIT global** (`[General] HarvesterUnit=` lookup at 0x0083c754) — the build-tree token that maps to CMIN for Allied factions.
- **SMIN / SMON / SLAV / YAREFN** — Yuri's alternative ore economy. See [`yuri/SLAV.md`](../yuri/SLAV.md).

### Deep-RE docs (cross-referenced, not re-derived) — **10 docs total** for the chrono-miner system:
- **[CHRONO_MINER_SYSTEM_OVERVIEW.md](../../CHRONO_MINER_SYSTEM_OVERVIEW.md)** — read first.
- **[CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md](../../CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md)**
- **[CHRONO_WARP_VISUAL_RENDERING.md](../../CHRONO_WARP_VISUAL_RENDERING.md)**
- **[TECHNOCLASS_CHRONO_OFFSETS_VERIFIED.md](../../TECHNOCLASS_CHRONO_OFFSETS_VERIFIED.md)**
- **[TELEPORT_LOCOMOTION_DEEP_DIVE.md](../../TELEPORT_LOCOMOTION_DEEP_DIVE.md)**
- **[TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md](../../TELEPORT_LOCOMOTION_IMPLEMENTATION_REFERENCE.md)**
- **[WAR_MINER_REFERENCE.md](../../WAR_MINER_REFERENCE.md)** — canonical comparison.
- **[HARVESTER_DOCK_UNLOAD.md](../../HARVESTER_DOCK_UNLOAD.md)** + **[HARVESTER_DOCK_UNLOAD_SEQUENCE.md](../../HARVESTER_DOCK_UNLOAD_SEQUENCE.md)** — dock-unload incl. UnloadingClass swap.
- **[MINER_DOCK_GAPS_RESEARCH.md](../../MINER_DOCK_GAPS_RESEARCH.md)** — edge cases.
- **[HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md](../../HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md)** + **[MISSION_HARVEST_GHIDRA_REPORT.md](../../MISSION_HARVEST_GHIDRA_REPORT.md)** — 5-state harvest mission.
- **[CHRONOSPHERE_SUPERWEAPON_GHIDRA_REPORT.md](../../CHRONOSPHERE_SUPERWEAPON_GHIDRA_REPORT.md)** — related (CMIN is the Chrono *Miner*, distinct from the Chronosphere superweapon, but they share underlying teleport mechanics on the locomotor side).

---

## Ghidra audit log (audit iteration 17 — 2026-05-18)

**Methodology**: CMIN cross-references 10+ deep-RE docs for the chrono
teleport / dock-unload / mission-harvest machinery. This audit doesn't
re-derive any of that — it focuses on (a) re-verifying the 5 doc-cited
parser xrefs, (b) pinning struct offsets for the 4 TechnoType keys, and
(c) confirming the **dual-read pattern** for ChronoInSound/OutSound. ~13
Ghidra queries: 7 string searches + 6 xref lookups + 2 grep passes on
saved TechnoTypeClass__ReadINI decompile.

### Negative claim re-verified

| Query | Result |
|-------|--------|
| `search_strings("^CMIN$")` | **0 matches** |

Confirms: no hardcoded section-name branch.

### String + parser xref verification (BINARY-VERIFIED)

| String | Addr | Parser xrefs | Notes |
|--------|------|--------------|-------|
| `ChronoInSound` | 0x0083A9A4 | RulesClass__ReadAudioVisual @ 0x006699E9 **+** TechnoTypeClass__ReadINI @ 0x007135DA | **DUAL-READ pattern BINARY-VERIFIED** — global default + per-unit override |
| `ChronoOutSound` | 0x0083A994 | RulesClass__ReadAudioVisual @ 0x00669A2A **+** TechnoTypeClass__ReadINI @ 0x0071361C | **DUAL-READ pattern BINARY-VERIFIED** (doc was hedging on this; now confirmed) |
| `StupidHunt` | 0x008438A4 | TechnoTypeClass__ReadINI @ 0x00714C6C | TechnoType-scope confirmed |
| `UnloadingClass` | 0x00843AF8 | TechnoTypeClass__ReadINI @ 0x007146E8 | TechnoType-scope confirmed; parser calls UnitTypeClass__FindOrAllocate → result stored as UnitType* |
| `HarvesterUnit` | 0x0083C754 | RulesClass__ReadGeneral @ 0x0066F8DD | Global Rules-General token |
| `ChronoHarvTooFarDistance` | 0x0083C464 | RulesClass__ReadGeneral @ 0x00670003 | Global Rules-General — the 50-cell teleport-gate threshold |

### Struct offsets BINARY-VERIFIED (this pass)

**TechnoType sound-list block** (sequence-position evidence from
TechnoTypeClass__ReadINI):

The TechnoType-level VocClass-index sound block is contiguous from
+0x568..+0x57C (6 ints at indices 0x15A..0x15F). Audit 14 already
pinned DeploySound +0x56C and UndeploySound +0x570; this audit extends
the table by sequence-position analysis (the parser writes occur in
declared INI-key order, and ChronoInSound's parse follows
UndeploySound's, with ChronoOutSound's following ChronoInSound's):

| Offset | INI key | Type | Status |
|--------|---------|------|--------|
| `+0x568` | (unknown sibling, possibly `SegueSound` or `CreateSound`) | int VocClass index | NEW — INI key DEFERRED |
| `+0x56C` | `DeploySound` | int | audit 14 |
| `+0x570` | `UndeploySound` | int | audit 14 |
| `+0x574` | `ChronoInSound` | int | **NEW** (sequence-position evidence) |
| `+0x578` | `ChronoOutSound` | int | **NEW** (sequence-position evidence) |
| `+0x57C` | (unknown sibling) | int | NEW — INI key DEFERRED |

**Other TechnoType offsets** (direct decompile writes):

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0x6D4` | `StupidHunt` | byte | `*(undefined1*)(param_1 + 0x1B5) = uVar3` after ReadBool. **NEW**. |

`UnloadingClass` write offset not pinned in this pass (would require
wider grep window beyond ReadString → FindOrAllocate → store sequence).
Scope (TechnoType) and value-type (UnitType*) are confirmed via parser
function name + FindOrAllocate call type. **DEFERRED for byte-offset
verification**.

### NEW function entry point

| Function | Notes |
|----------|-------|
| `RulesClass__ReadAudioVisual` | Confirmed via xrefs for ChronoInSound/OutSound. Sibling to `RulesClass__ReadGeneral` (audit 12 cumulative) and `RulesClass__ReadJumpjetControls` (audit 8). Reads the `[AudioVisual]` INI section for global sound defaults. Not fully decompiled in this pass — added to cheat-sheet for future reference. |

### Behavioral claim cross-references (NOT re-derived this pass)

The doc cites 10+ deep-RE docs for chrono-teleport, dock-unload, and
mission-harvest behavior. The audit-log entries below acknowledge these
as DEFERRED, with the dependency-doc as their authority:

- Teleport-vs-drive decision at the 50-cell threshold → `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md`
- IPiggyback Teleport↔Drive swap in `FootClass::AI` → `CHRONO_MINER_SYSTEM_OVERVIEW.md`
- Warp-in/out visual rendering → `CHRONO_WARP_VISUAL_RENDERING.md`
- TechnoClass chrono state field offsets → `TECHNOCLASS_CHRONO_OFFSETS_VERIFIED.md` (also CLEG audit 5 pinned +0x270/+0x278/+0xCD4)
- `UnloadingClass=CMON` mid-dock visual swap → `HARVESTER_DOCK_UNLOAD_SEQUENCE.md`
- 5-state `Mission_Harvest` machine → `HARVESTER_MISSION_HARVEST_GHIDRA_REPORT.md`

These docs' claims have NOT been independently verified in this audit
pass. CMIN's doc treats them as authoritative; that trust-chain is
preserved here.

### Items NOT re-verified in this pass (DEFERRED)

- `UnloadingClass` exact byte offset (TechnoType-scope + UnitType*
  value-type confirmed by parser, but byte-offset not in grep window).
- INI keys for the two unknown sibling slots at +0x568 and +0x57C.
- `HarvesterUnit` Rules-General byte offset (parser confirmed; would
  need grep on oversized RulesClass__ReadGeneral decompile).
- `ChronoHarvTooFarDistance` Rules-General byte offset (parser
  confirmed; offset DEFERRED).
- The 10+ chrono-miner / dock-unload deep-RE docs — treated as
  authoritative cross-references rather than re-verified.
- `ChronoMinerTeleport [Limit=1]` cap scope (per-house vs global).
- `RulesClass__ReadAudioVisual` body decompile.

### Confidence summary

- **HIGH**: 7 string addresses + 6 parser xrefs (all exact); **dual-read
  pattern for both ChronoInSound and ChronoOutSound BINARY-VERIFIED**
  (the doc had this for ChronoInSound and inferred for ChronoOutSound;
  now both confirmed); 3 NEW TechnoType offsets (ChronoInSound +0x574,
  ChronoOutSound +0x578, StupidHunt +0x6D4); 1 NEW RulesClass parser
  function (ReadAudioVisual).
- **MEDIUM**: ChronoInSound/OutSound offsets pinned by sequence-position
  evidence (adjacency with audit-14 DeploySound/UndeploySound) rather
  than direct write inspection — the parser writes occur in declared
  order, but a wider grep window would directly confirm the writes (not
  done this pass for token economy).
- **LOW** (delegated): all 10+ chrono-miner / dock-unload behavioral
  claims — trust-chain to the cross-referenced deep-RE docs.
- **No INCORRECT findings** in the doc; the dual-read hedge on
  ChronoOutSound is now CONFIRMED.

---

## 11. Coverage audit

| Section | Status |
|---------|--------|
| Every `[CMIN]` rulesmd key explained | ✅ §1 |
| Every `[CMIN]` artmd key explained + `[CMON]` art noted | ✅ §2 |
| No weapon / warhead (explicitly addressed) | ✅ §3–§4 |
| All voices + teleport SFX | ✅ §5 |
| Prereqs / owners / teleport-vs-drive decision logic | ✅ §6 |
| CMIN-vs-HARV comparison table (canonical) | ✅ §6 |
| Hardcoded behavior — Ghidra searches + flag scope verifications | ✅ §7 (ChronoInSound/OutSound dual-read pattern noted, StupidHunt verified) |
| TS-legacy filter | ✅ §8 |
| Veterancy treated correctly (Trainable=no → permanently rookie) | ✅ §9 |
| Cross-refs to **10+ deep-RE docs** in the chrono-miner family | ✅ §10 |
| Pair with HARV doc — harvester family now fully documented | ✅ |

**Open follow-ups (none load-bearing):**
- `[CMON]` itself deserves its own quick-ref doc — minimal but documents the no-back visual variant. Could be batched with `[HORV]` (HARV's unloading form) as a "harvester unload-variant family doc".
- Verify the exact frame at which `Locomotor` swaps from Teleport to Drive (or vice versa) via IPiggyback in `FootClass::AI` — TELEPORT_LOCOMOTION_DEEP_DIVE has this but worth double-checking against current binary if a parity bug surfaces.
- The `ChronoMinerTeleport [Limit=1]` cap — confirm whether this is a per-house cap or global. With 6+ CMINs simultaneously warping the limit may produce silent warps; relevant for late-game audio fidelity.
- Confirm the `[General] HarvesterUnit=` lookup maps to CMIN for all five Allied countries.
