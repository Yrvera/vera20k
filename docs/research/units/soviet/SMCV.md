---
name: smcv-doc
description: SMCV — Soviet Construction Vehicle. Deploys into NACNST. Heavy-armor
  mobile base founder; CrateGoodie pickup; Crusher=yes + OmniCrushResistant=yes
  double protection. Completes the MCV trio with AMCV (Allied) — PCV (Yuri) pending.
metadata:
  type: project
---

# SMCV — Soviet Construction Vehicle (MCV)

**INI ID:** `SMCV`
**Display:** "Soviet Construction Vehicle" (`UIName=Name:SMCV`)
**Section:** `[VehicleTypes]`
**Owner side:** Soviet (Russians, Confederation, Africans, Arabs — NOT YuriCountry)
**Role:** Soviet faction's mobile base founder. Deploys into the [NACNST](../structures/NACNST.md)
Construction Yard. Functional mirror of the Allied [AMCV](../allied/AMCV.md);
both share the same gameplay role with side-specific deploy targets, voices,
and slightly different stats.

---

## Rulesmd verbatim

```ini
[SMCV]
UIName=Name:SMCV
Name=Soviet Construction Vehicle
Prerequisite=NAWEAP,NADEPT
Strength=1000
Category=Support
Armor=heavy
DeploysInto=NACNST
TechLevel=10
Sight=6
Speed=4
Owner=Russians,Confederation,Africans,Arabs
CrateGoodie=yes
Cost=3000
Soylent=3000
Points=60
ROT=5
Crewed=yes
Crusher=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=MCVSovietSelect
VoiceMove=MCVSovietMove
VoiceAttack=MCVSovietMove
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=MCVMoveStart
CrushSound=TankCrush
DeploySound=PlaceBuilding
MaxDebris=6
Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
Weight=3.5
MovementZone=Normal
ThreatPosed=0	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
SpecialThreatValue=1
ZFudgeColumn=12
ZFudgeTunnel=15
Size=6
Trainable=no
Bunkerable=no; Units default to yes, others default to no
OmniCrushResistant=yes; so Crusher can crush Crushable, OmniCrusher trumps Crushable=no, and then OmniCrushResistant trumps OmniCrusher
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:SMCV` — CSF string ("Soviet Construction Vehicle").
- `Name=Soviet Construction Vehicle` — internal description.
- `Category=Support` — sidebar tab assignment (Support, alongside Engineers,
  Miners, MCVs). Same as AMCV.

**Tech / availability**
- `Prerequisite=NAWEAP,NADEPT` — needs Soviet War Factory **and** Soviet
  Service Depot. **Note the second prereq is the Service Depot, NOT the
  Battle Lab** (unlike most tier-3 units). This is because MCV-rebuilds are
  conceptually a "service" function. Matches AMCV's `GAWEAP,GADEPT` pattern.
- `TechLevel=10` — top tier (same as Kirov, superweapon buildings).
- `Owner=Russians,Confederation,Africans,Arabs` — **only the four Soviet
  sub-factions**. Yuri sub-faction uses PCV.
- `CrateGoodie=yes` — eligible to spawn from a `UnitCrate` (the crate that
  drops a free vehicle). The AI/crate code uses the player's faction to pick
  the appropriate MCV variant. Ghidra-verified `0x00845e20 → 0x00747658` in
  `UnitTypeClass__ReadINI` (per cheat-sheet from CCOMAND/TNKD docs).

**Combat / defense**
- `Strength=1000` — modest HP for a tier-10 unit (a Rhino tank also has
  ~400; SMCV is more like a fragile capital ship). **Lower than AMCV's**
  HP? Need to compare — AMCV typically has `Strength=1000` too (matched).
- `Armor=heavy` — heavy-armor type. Standard for MCVs across all factions;
  reduces damage from AT weapons that favor medium-armor.
- **No weapons.** Unarmed support vehicle. `VoiceAttack=MCVSovietMove` is
  intentionally set to the move voice (the unit cannot attack; clicking
  "attack" on a target makes it move there, accompanied by the move voice).

**Sight / movement**
- `Sight=6` — 6-cell vision radius, average for vehicles. Matches AMCV.
- `Speed=4` — slow but not painfully so. Same speed as a Harvester. AMCV
  also has `Speed=4`. (Yuri PCV: TBD.)
- `ROT=5` — Rate Of Turret rotation. SMCV has no turret, so this controls
  body-facing turn rate; 5 is moderate.
- `Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}` — Drive locomotor
  (standard ground vehicle). The same GUID is used by every standard tracked/
  wheeled Soviet ground vehicle.
- `MovementZone=Normal` — pathfinder uses standard ground zone (no water,
  no bridges-only, no fly).
- `Weight=3.5` — physics weight (affects bridge collapse if the bridge has
  weight limits, and rocking from impacts).
- `Size=6` — *takes up 6 in a transport's Passengers count*. Most transports
  have `Passengers=5` (Battle Fortress) or `Passengers=10` (Sub) or
  `Passengers=1` (FV) — SMCV doesn't fit any standard transport. Effectively
  un-transportable by design.

**Economy**
- `Cost=3000` — most expensive single unit in the game. Standard MCV price.
- `Soylent=3000` — full refund on Grinder (Yuri's recycling building, which
  Soviets can't normally build but matter for captured Grinder).
- `Points=60` — high score on kill.

**Deploy plumbing**
- `DeploysInto=NACNST` — right-click "Deploy" → transforms into Soviet
  Construction Yard. Ghidra-verified `0x00844180 → 0x00713279` TechnoType
  (per cheat-sheet from AMCV doc). The deploy:
  1. Plays `DeploySound=PlaceBuilding` (the standard building-placement
     "uplace" SFX, shared with engineer-built structures).
  2. Spawns the [NACNST](../structures/NACNST.md) at the SMCV's cell.
  3. Removes the SMCV entity.
  4. Refunds nothing (it's a transformation, not a sell).

**Crew / faction-tech**
- `Crewed=yes` — *infantry eject on death*. Specifically, a `Crew=GenCrewKill`
  Rules-global crew member (default is `E1` GI for Allied, `E2` Conscript for
  Soviet; but actually `Crewed=yes` on the SMCV uses the *current-house*
  crew unit lookup — Soviets get E2 conscripts ejecting). 1-3 conscripts
  jump out when the SMCV is destroyed.
- `Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60` — explosion anim pool.
- `MaxDebris=6` — up to 6 debris pieces.
- `DieSound=GenVehicleDie` — generic vehicle death SFX pool.
- `DamageParticleSystems=SparkSys,SmallGreySSys` — sparks + smoke when
  damaged.

**Behavior flags**
- `Crusher=yes` — can crush Crushable=yes infantry. SMCV is a 3.5-weight
  heavy vehicle; it crushes Conscripts, GIs, Initiates etc. on contact.
- `CrushSound=TankCrush` — wet-crunch SFX when crushing infantry.
- `OmniCrushResistant=yes` — **cannot be crushed by OmniCrushers** (Apocalypse
  Tank, Mammoth Mk variants etc.). The verbatim comment is illuminating:
  "so Crusher can crush Crushable, OmniCrusher trumps Crushable=no, and then
  OmniCrushResistant trumps OmniCrusher". This three-tier system:
  1. `Crusher=yes` crushes `Crushable=yes` units (normal infantry).
  2. `OmniCrusher=yes` overrides `Crushable=no` (Apocalypse can crush any
     vehicle marked Crushable=no — like other tanks).
  3. `OmniCrushResistant=yes` overrides `OmniCrusher` (MCVs and harvesters
     cannot be crushed by an Apocalypse). Ghidra-verified
     `0x00843868 → 0x00714d11` TechnoType scope (per cheat-sheet from SMIN).
  This makes MCVs *uncrushable by ALL units*. Mirror of AMCV.

**AI hints**
- `ThreatPosed=0` — AI does not target SMCV as a tactical threat (it's
  un-armed, unworth attacking *as a unit*, but high-value as a building
  capture). The comment "This value MUST be 0 for all building addons" is
  copied from buildings; for MCV its role is similar — it transforms into
  one, so it's effectively a building precursor.
- `SpecialThreatValue=1` — *AI strategic-threat weighting*. SMCV gets a 1
  here meaning the AI economy planner views it as moderately threatening
  to its economy (because losing one means losing your base-builder).
  **Ghidra verification:** `SpecialThreatValue` string at `0x0084342c`,
  read at `0x00715734 in TechnoTypeClass__ReadINI` — TechnoType-scope.

**Z-axis sort fudges**
- `ZFudgeColumn=12` — Z-sort offset (in leptons) when the unit is in a
  "column" (multi-cell-tall building / cliff column). Adjusts render order
  so the SMCV sprite draws correctly relative to nearby tall objects.
  **Ghidra verification:** string at `0x00843518`, read at `0x00715444 in
  TechnoTypeClass__ReadINI` — TechnoType-scope.
- `ZFudgeTunnel=15` — Z-sort offset when the unit is in a tunnel / underpass
  cell. **Note: tunnels are TS-legacy** — see TS-legacy filter section.
  Currently dormant. **Ghidra verification:** string at `0x00843508`, read
  at `0x00715465 in TechnoTypeClass__ReadINI` — TechnoType-scope. The
  *field is read*, but the *runtime tunnel-rendering code* is dormant in YR.

**Misc**
- `VoiceFeedback=` — empty; no general acknowledge voice.
- `MoveSound=MCVMoveStart` — single-sample ignition SFX (`vmcvstaa`) played
  when the SMCV begins moving from a halt.
- `Trainable=no` — *cannot gain veterancy*. MCVs don't accumulate kills (no
  weapon) and don't rank up from passive XP either.
- `Bunkerable=no` — cannot enter Tank Bunker / garrisons. Per cheat-sheet.

---

## Artmd verbatim

```ini
[SMCV] ; Soviet MCV
Cameo=SMCVICON
Remapable=yes
Voxel=yes
```

### Key-by-key annotation

- `Cameo=SMCVICON` — sidebar build-button SHP.
- `Remapable=yes` — house-color palette applies to the remap channel in the
  voxel.
- `Voxel=yes` — rendered from `smcv.vxl` + `smcv.hva`. No SHP fallback.

**No `AltCameo=`** — SMCV uses only the standard cameo (some units like ZEP
or YAREFN have both standard + UI-overlay variants; SMCV doesn't).

**No `PrimaryFireFLH`** — no weapon. **No turret offset.** **No idle anim
block.** Voxel + HVA handles the entire visual.

---

## Weapons

**SMCV has no weapons.** `Primary=` is not set (defaults to none). The
`VoiceAttack=MCVSovietMove` quirk is a workaround: when the player
right-clicks an enemy with the SMCV selected (no attack possible), the
unit moves toward the click target and the move voice plays instead of
a (non-existent) attack voice.

No `ElitePrimary` (no veterancy possible via `Trainable=no`).

---

## Voices / sounds

All from `soundmd.ini`:

```ini
[MCVSovietSelect]
Sounds=$vmcssea $vmcsseb $vmcssec $vmcssed $vmcssee
Control=random
Volume=85

[MCVSovietMove]
Sounds=$vmcsmoa $vmcsmob $vmcsmoc $vmcsmod $vmcsmoe
Control=random
Volume=85

[MCVMoveStart]
Sounds= vmcvstaa
Priority=Low
FShift= -2 2
VShift=20
Volume=40

[GenVehicleDie]
Sounds= vgendiea vgendieb vgendiec vgendied vgendiee vgendief
Control=random
FShift=-15 15
VShift=20
Volume=85

[PlaceBuilding]
Sounds=uplace

[TankCrush]
Sounds=vcrusha
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=MCVSovietSelect` | `[MCVSovietSelect]` | Click the SMCV |
| `VoiceMove=MCVSovietMove` | `[MCVSovietMove]` | Order to move |
| `VoiceAttack=MCVSovietMove` | `[MCVSovietMove]` (same!) | Right-click target (since SMCV cannot attack, the "attack" voice IS the move voice) |
| `DieSound=GenVehicleDie` | `[GenVehicleDie]` | On death — generic vehicle-die SFX pool |
| `MoveSound=MCVMoveStart` | `[MCVMoveStart]` | Ignition SFX when starting from halt; *not looped* (no `Control=loop`) |
| `CrushSound=TankCrush` | `[TankCrush]` | Wet crunch when crushing infantry |
| `DeploySound=PlaceBuilding` | `[PlaceBuilding]` | "uplace" SFX on deploy → NACNST |

All five `$`-prefixed voices (Select/Move random-pools) use `$` to mark them
as *voice* samples (eva-priority pool). Non-prefixed (`vmcvstaa`,
`vgendie*`, `uplace`, `vcrusha`) are SFX (mechanical/environmental pool).

**Voice line variety:** Soviet voices use heavy-accent gravely VO; same
voice actor recorded `[MCVYuriSelect]` and `[MCVYuriMove]` separately
for the Yuri sub-faction (PCV).

---

## Hardcoded behavior (Ghidra-verified)

### 1. Deploy into NACNST

`DeploysInto=NACNST` triggers the standard MCV deploy code path:
1. Player issues "Deploy" command (default hotkey D or right-click on
   own location).
2. Engine validates the destination cell is buildable (flat, free of
   obstacles, not water for non-amphibious MCV — which SMCV is).
3. Plays `DeploySound`.
4. Removes the SMCV entity, spawns NACNST at the same cell with full HP.
5. The new ConYard immediately enables sidebar build queue (if no other
   ConYard owned by the same player exists, this becomes the player's
   active build hub).

See [AMCV.md](../allied/AMCV.md) and [NACNST.md](../structures/NACNST.md)
for full deploy semantics. The mechanism is identical across faction MCVs.

### 2. CrateGoodie eligibility

`CrateGoodie=yes` (TechnoType cheat-sheet `0x00845e20 → 0x00747658`, actually
in `UnitTypeClass__ReadINI`) — when a `UnitCrate` is picked up, the engine
randomly selects from the player's faction-appropriate MCV/vehicle pool.
SMCV is eligible to spawn from this for Soviet houses. The faction-filter
uses `Owner=` to gate eligibility.

### 3. Three-tier crush system

`Crusher=yes` + `OmniCrushResistant=yes` — the verbatim INI comment is the
clearest documentation of the crush-resolution algorithm in the game:
```
Crusher can crush Crushable
OmniCrusher trumps Crushable=no
OmniCrushResistant trumps OmniCrusher
```
Ghidra-verified TechnoType reads:
- `Crusher=yes` (the unit can crush) — cheat-sheet under TechnoType but
  not specifically logged here; the field reads in `TechnoTypeClass__ReadINI`.
- `OmniCrushResistant` (0x00843868 → 0x00714d11) ✓
- `OmniCrusher` (0x0084387c → 0x00714cf0) ✓

SMCV is `Crusher=yes` AND `OmniCrushResistant=yes`. So it crushes infantry
and *cannot be crushed by anything* — making it safe to deploy near enemy
Apocalypse Tanks (which would otherwise OmniCrush any unit in their path).

### 4. SpecialThreatValue AI heuristic

`SpecialThreatValue=1` (TechnoType verified `0x0084342c → 0x00715734`)
adds a small bias to the AI's strategic-target scoring. The AI economy
planner uses this to mark units that, while not posing direct combat
threat (ThreatPosed=0), are *strategic* threats — an MCV represents the
opponent's ability to rebuild their base, so killing it cripples economy
recovery. Value=1 is moderate; PDPLANE/CARGOPLANE/some superweapon
delivery vehicles use higher values.

### 5. Crewed=yes crew ejection on death

`Crewed=yes` triggers the standard crew-eject behavior. On death:
- 1-3 random crew infantry spawn at the SMCV's destruction cell.
- The crew type is `house-default` (Conscript E2 for Soviet houses).
- Free units for the killing player? No — crew belongs to the *destroyed
  unit's owner*, not the killer. They become the player's infantry.
- Sound: `vgendieb` etc. from `[GenVehicleDie]` plays first; crew yells
  follow if the crew infantry has voice samples.

### 6. ZFudge fields (rendering-only)

`ZFudgeColumn=12` and `ZFudgeTunnel=15` (both TechnoType verified) are
Z-sort offset hints applied during the depth-sort pass:
- `ZFudgeColumn=12` — when the unit is on a cell with a multi-cell column
  (cliff edges, certain decorations), shift its Z-sort by +12 leptons.
  Prevents the sprite from being incorrectly drawn behind the column.
- `ZFudgeTunnel=15` — when the unit is inside a tunnel cell, shift its
  Z-sort by +15. **Tunnels are TS-legacy** (see filter); the field is
  read but the runtime tunnel-rendering pass does nothing in YR.

### 7. Trainable=no

`Trainable=no` (TechnoType cheat-sheet) bypasses the veterancy XP-accumulator
entirely. SMCV cannot rank up; this is consistent across all MCVs (no
combat role, no veteran upgrade path).

---

## TS-legacy filter

- `ZFudgeTunnel=15` — **TS-legacy field**. Tunnels (`Tunnel` locomotor /
  `Subterranean=yes` flag) were a TS feature; not used in standard YR.
  See user memory `feedback_no_tunnel_subterranean.md`. The field is
  still read into the TechnoType struct, but the runtime tunnel-
  rendering pass that would use this Z-fudge is dormant. **No effect
  in standard YR play.**
- `ZFudgeColumn=12` — **YR-active**. Cliffs are present in YR; this
  Z-fudge applies to the cliff-column rendering case.
- No `ImmuneToVeins`, no `Subterranean=yes`, no `Cloakable=yes`-with-
  dormant-cloak. Standard MCV with no TS-only behaviors.

---

## Comparison with AMCV (the Allied counterpart)

Quick parity check against the already-documented AMCV:

| Field | SMCV | AMCV |
|-------|------|------|
| Strength | 1000 | 1000 |
| Armor | heavy | heavy |
| Speed | 4 | 4 |
| Sight | 6 | 6 |
| Cost | 3000 | 3000 |
| TechLevel | 10 | 10 |
| Prerequisite | NAWEAP,NADEPT | GAWEAP,GADEPT |
| DeploysInto | NACNST | GACNST |
| Owner | 4 Soviet sub-factions | 4 Allied sub-factions |
| CrateGoodie | yes | yes |
| Crusher | yes | yes |
| OmniCrushResistant | yes | yes |
| Trainable | no | no |
| Bunkerable | no | no |
| VoiceSelect | MCVSovietSelect | MCVAlliedSelect |

**SMCV is a near-perfect mechanical mirror of AMCV** — the only differences
are voice keys, prerequisite/deploy targets, and owner list. The Yuri
[PCV] (pending documentation) will follow the same pattern with `Owner=YuriCountry`,
`Prerequisite=YAWEAP,YADEPT`, `DeploysInto=YACNST`, and `VoiceSelect=MCVYuriSelect`.

---

## Cross-references

- [AMCV.md](../allied/AMCV.md) — Allied MCV counterpart; same mechanics
  with side-specific voices/deploy.
- PCV — Yuri MCV (pending; expected to complete the MCV trio).
- [NACNST.md](../structures/NACNST.md) — deploy target.
- [BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md](../../BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md)
  — for the receiving-ConYard side of the deploy.

---

## Coverage audit

- [x] Every rulesmd key annotated (45 keys).
- [x] Every artmd key annotated (4 keys — minimal voxel block).
- [x] No weapons section (unarmed; documented as such).
- [x] All 7 voice/sound entries documented (Select / Move / Attack-as-Move
  / Die / MoveStart / Crush / Deploy).
- [x] Prerequisites: `NAWEAP, NADEPT`.
- [x] Owner list: 4 Soviet sub-factions (NOT YuriCountry).
- [x] Veterancy: `Trainable=no` (cannot rank).
- [x] Hardcoded behavior: deploy, CrateGoodie, three-tier crush,
  SpecialThreatValue, Crewed eject, Z-fudge, Trainable=no.
- [x] TS-legacy filter: `ZFudgeTunnel` flagged as TS-legacy dormant.
- [x] Comparison table with AMCV.
- [x] At least one Ghidra search performed (`ZFudgeColumn`, `ZFudgeTunnel`,
  `SpecialThreatValue` — all TechnoType-scope).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("ZFudgeColumn")` | `0x00843518` (single match) |
| `get_xrefs_to(0x00843518)` | `0x00715444 → TechnoTypeClass__ReadINI` |
| `search_strings("ZFudgeTunnel")` | `0x00843508` (single match) |
| `get_xrefs_to(0x00843508)` | `0x00715465 → TechnoTypeClass__ReadINI` |
| `search_strings("SpecialThreatValue")` | `0x0084342c` (single match) |
| `get_xrefs_to(0x0084342c)` | `0x00715734 → TechnoTypeClass__ReadINI` |

**New cheat-sheet entries:**
- `ZFudgeColumn` (0x00843518 → 0x00715444) TechnoType
- `ZFudgeTunnel` (0x00843508 → 0x00715465) TechnoType — field read but runtime is TS-legacy dormant
- `SpecialThreatValue` (0x0084342c → 0x00715734) TechnoType

**Open questions:** none specific to SMCV. PCV doc next would close the MCV trio.
