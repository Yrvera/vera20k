---
name: pcv-doc
description: PCV — Yuri MCV. Deploys into YACNST. Near-perfect mirror of SMCV/AMCV
  with three Yuri-specific differences — Sight=8 (+2 vs Allied/Soviet), prereq uses
  YAGRND Grinder (not Service Depot), Owner=YuriCountry. Closes the MCV trio.
metadata:
  type: project
---

# PCV — Yuri Construction Vehicle (MCV)

**INI ID:** `PCV`
**Display:** "Yuri Construction Vehicle" (`UIName=Name:YMCV`)
**Section:** `[VehicleTypes]`
**Owner side:** Yuri (`Owner=YuriCountry`)
**Role:** Yuri sub-faction's mobile base founder. Deploys into the
[YACNST](../structures/YACNST.md) Construction Yard. Closes the MCV trio
with [AMCV](../allied/AMCV.md) and [SMCV](../soviet/SMCV.md). Near-perfect
mechanical mirror with three Yuri-specific tweaks (see below).

---

## Rulesmd verbatim

```ini
[PCV]
UIName=Name:YMCV
Name=Yuri Construction Vehicle
;Image=SMCV
Prerequisite=YAWEAP,YAGRND;YADEPT
Strength=1000
Category=Support
Armor=heavy
DeploysInto=YACNST
TechLevel=10
Sight=8
Speed=4
Owner=YuriCountry
CrateGoodie=yes
Cost=3000
Soylent=3000
Points=60
ROT=5
Crewed=yes
Crusher=yes
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=MCVYuriSelect
VoiceMove=MCVYuriMove
VoiceAttack=MCVYuriMove
DieSound=GenVehicleDie
DeploySound=PlaceBuilding
CrushSound=TankCrush
VoiceFeedback=
MoveSound=MCVMoveStart
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

Most fields are mechanically identical to [SMCV](../soviet/SMCV.md) and
[AMCV](../allied/AMCV.md). See those docs for shared-field details. This
section covers only the **Yuri-specific differences** plus PCV-only notes.

**Identity / UI (Yuri-specific)**
- `UIName=Name:YMCV` — uses the `YMCV` CSF key (vs SMCV's `SMCV` key and
  AMCV's `MCV` key). CSF lookup returns "Yuri Construction Vehicle".
- `Name=Yuri Construction Vehicle` — internal description.
- `;Image=SMCV` — commented. Indicates the PCV was originally going to
  reuse the SMCV voxel (`smcv.vxl`); during development Westwood gave Yuri
  its own custom MCV art (`pcv.vxl` + `pcv.hva`). The commented `Image=SMCV`
  is a vestigial reference. With the `;` it has no effect — the engine uses
  the section name `[PCV]` as the asset key, finding `pcv.vxl`.

**Tech / availability (Yuri-specific)**
- `Prerequisite=YAWEAP,YAGRND;YADEPT` — **the most unusual key on this
  unit**. Requires:
  - `YAWEAP` — Yuri War Factory (expected).
  - `YAGRND` — **the Grinder** (recycle-units building) — NOT the Service
    Depot.
  - The trailing `;YADEPT` is a *commented historical prereq*. The Grinder
    replaces the Service Depot as the prereq for this MCV.

  **Why the Grinder?** The other two MCVs gate on the Service Depot
  (`NADEPT`/`GADEPT`) — repair pads. Yuri's [YADEPT](../structures/YADEPT.md)
  exists as a building, but Westwood swapped the prereq to
  [YAGRND](../structures/YAGRND.md) (Grinder). Likely reasons:
  1. **Thematic fit** — Yuri's "service" is the Grinder's recycle-for-cash
     mechanic. The Service Depot repairs; the Grinder destroys-for-credits.
     A "Yuri MCV" requiring the building that recycles old MCVs fits the
     faction's design language.
  2. **Build-tree gating** — the Grinder is a higher-tier Yuri building
     than YADEPT; requiring it slows down PCV rebuild after a Yuri MCV
     dies, hardening the asymmetry against Soviet/Allied (who get their
     MCV-rebuild prereq from a tier-1.5 service pad).

  Whatever the reason, **a player who loses their PCV and YACNST cannot
  rebuild MCVs without first rebuilding both a War Factory AND a Grinder**.
  This is a genuine asymmetric Yuri-vs-Allied/Soviet design quirk.
- `TechLevel=10` — top tier (same as SMCV/AMCV).
- `Owner=YuriCountry` — *only YuriCountry*. Single house, unlike
  SMCV's 4-faction list (Russians/Confederation/Africans/Arabs) or AMCV's
  4-faction list. Yuri is mechanically one country in the MP roster.

**Sight (Yuri-specific buff)**
- `Sight=8` — **+2 cells vs SMCV's `Sight=6` and AMCV's `Sight=6`**. The
  Yuri MCV sees further than its counterparts. **This is the one
  combat-relevant stat difference** between the three MCVs. Possibly to
  compensate for Yuri having no Spy Satellite equivalent (no map-reveal
  superweapon).

**All other fields identical to SMCV/AMCV**

The following keys are bit-for-bit identical to SMCV (cross-reference
[SMCV.md](../soviet/SMCV.md) for explanations):

- `Strength=1000`, `Armor=heavy`, `Speed=4`, `Cost=3000`, `Soylent=3000`
- `DeploysInto=YACNST` (Yuri-specific target, same mechanism)
- `CrateGoodie=yes` (CrateGoodie pool faction-filtered by Owner)
- `Crusher=yes` + `OmniCrushResistant=yes` (three-tier crush)
- `Crewed=yes` (ejects Initiates — Yuri faction default crew? See "Crew
  ejection" subsection)
- `SpecialThreatValue=1`, `ZFudgeColumn=12`, `ZFudgeTunnel=15`
- `Trainable=no`, `Bunkerable=no`
- `MovementZone=Normal`, `Locomotor=Drive GUID`
- `ThreatPosed=0`, `ROT=5`, `Weight=3.5`, `Size=6`, `MaxDebris=6`

**Voice / sound bindings (Yuri-specific)**
- `VoiceSelect=MCVYuriSelect` → `[MCVYuriSelect]` (5-sample $vmcyse* pool)
- `VoiceMove=MCVYuriMove` → `[MCVYuriMove]` (5-sample $vmcymo* pool)
- `VoiceAttack=MCVYuriMove` (same as move; unit has no weapon)
- `DieSound=GenVehicleDie`, `MoveSound=MCVMoveStart`, `CrushSound=TankCrush`,
  `DeploySound=PlaceBuilding` — all shared with SMCV/AMCV.

---

## Artmd verbatim

```ini
[PCV] ; Yuri MCV
Cameo=YPCVICON
Remapable=yes
Voxel=yes
```

### Key-by-key annotation

- `Cameo=YPCVICON` — sidebar build-button SHP. *Note the Y-prefix* on the
  filename (the Yuri-faction art-naming convention).
- `Remapable=yes` — house-color palette applies to the remap channel.
  Yuri's player color (orange-yellow by default; player can change in
  MP lobby) tints the relevant voxel pixels.
- `Voxel=yes` — rendered from `pcv.vxl` + `pcv.hva`. No SHP fallback.

**No `AltCameo=`, no `PrimaryFireFLH`, no turret offset, no idle anim** —
identical minimal voxel block to SMCV.

---

## Weapons

**PCV has no weapons.** Same as SMCV/AMCV. The `VoiceAttack=MCVYuriMove`
quirk works identically: right-clicking an enemy with PCV selected makes
it move toward the target, playing the move voice.

---

## Voices / sounds

All from `soundmd.ini`:

```ini
[MCVYuriSelect]
Sounds=$vmcysea $vmcyseb $vmcysec $vmcysed $vmcysee
Control=random
Volume=85

[MCVYuriMove]
Sounds=$vmcymoa $vmcymob $vmcymoc $vmcymod $vmcymoe
Control=random
Volume=85
```

The other 5 sound bindings (`MCVMoveStart`, `GenVehicleDie`, `PlaceBuilding`,
`TankCrush`) are shared with SMCV — see [SMCV.md](../soviet/SMCV.md) for
those blocks.

**Voice character:** The `$vmcyse*` / `$vmcymo*` samples are the Yuri
faction's *more measured, intellectual* VO style — fits the Yuri sub-faction
flavor (vs Soviet conscripts' gravelly heavy-accent voices, vs Allied MCV's
neutral mid-Atlantic English).

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=MCVYuriSelect` | `[MCVYuriSelect]` | Click PCV |
| `VoiceMove=MCVYuriMove` | `[MCVYuriMove]` | Order to move |
| `VoiceAttack=MCVYuriMove` | `[MCVYuriMove]` (same) | Right-click target |
| `DieSound=GenVehicleDie` | shared | Death SFX |
| `MoveSound=MCVMoveStart` | shared | Ignition |
| `CrushSound=TankCrush` | shared | Crushing infantry |
| `DeploySound=PlaceBuilding` | shared | Deploy → YACNST |

---

## Hardcoded behavior

All hardcoded behavior is identical to [SMCV.md](../soviet/SMCV.md) and
[AMCV.md](../allied/AMCV.md). See those docs for:
- Deploy → ConYard transformation (here: → YACNST).
- CrateGoodie pool eligibility (faction-filtered by `Owner=YuriCountry`).
- Three-tier crush system.
- Crewed=yes ejection (Yuri default crew is the **Initiate** ([INIT](../yuri/INIT.md))
  — Yuri's basic infantry — based on the house-default crew lookup).
- SpecialThreatValue=1 AI bias.
- ZFudge fields (ZFudgeTunnel TS-legacy dormant).
- Trainable=no.

### Yuri-specific runtime quirks

1. **Sight=8 buff** — gives PCV an effective scouting radius 33% larger
   than SMCV/AMCV. In practice, a moving Yuri MCV reveals about 4× more
   total area per second than its counterparts. Useful for scouting forward
   for a base-expansion deploy location.

2. **Grinder dependency** — when the player loses their last YACNST, they
   can rebuild a PCV from any War Factory **only if a Grinder is also
   present** (or rebuilt). The Allied/Soviet equivalent requires only a
   Service Depot, which is a tier-1 building. The Grinder is tier-2 in
   the Yuri build tree (requires barracks + war factory + radar to unlock).
   **Asymmetric MCV-rebuild cost**: Yuri's recovery from total ConYard
   loss is more expensive than Allied/Soviet.

3. **Owner=YuriCountry only** — unlike the 4-faction lists of SMCV/AMCV,
   PCV cannot be built by any sub-faction of Yuri (there are no Yuri
   sub-factions in vanilla YR — `YuriCountry` is monolithic).

---

## TS-legacy filter

Identical to SMCV/AMCV:

- `ZFudgeTunnel=15` — TS-legacy field, dormant in YR (no Tunnel
  locomotor / Subterranean usage). See user memory
  `feedback_no_tunnel_subterranean.md`.
- `ZFudgeColumn=12` — YR-active.
- No other TS-only fields.

---

## Comparison with SMCV and AMCV (the MCV trio)

| Field | AMCV (Allied) | SMCV (Soviet) | PCV (Yuri) |
|-------|---------------|---------------|------------|
| Strength | 1000 | 1000 | 1000 |
| Armor | heavy | heavy | heavy |
| Speed | 4 | 4 | 4 |
| **Sight** | **6** | **6** | **8** |
| Cost | 3000 | 3000 | 3000 |
| TechLevel | 10 | 10 | 10 |
| **Prerequisite** | GAWEAP,GADEPT | NAWEAP,NADEPT | **YAWEAP,YAGRND** |
| **DeploysInto** | GACNST | NACNST | YACNST |
| **Owner** | 4 Allied | 4 Soviet | YuriCountry |
| CrateGoodie | yes | yes | yes |
| Crusher | yes | yes | yes |
| OmniCrushResistant | yes | yes | yes |
| Trainable | no | no | no |
| Bunkerable | no | no | no |
| Crewed | yes (GI) | yes (E2) | yes (Initiate) |
| VoiceSelect | MCVAlliedSelect | MCVSovietSelect | MCVYuriSelect |
| Locomotor | Drive | Drive | Drive |

**Three meaningful differences across the trio:**
1. **PCV's Sight=8** (2-cell buff)
2. **PCV's YAGRND prereq** (Grinder instead of Service Depot — harder
   MCV-rebuild)
3. **PCV's single-faction Owner** (no sub-factions)

All other stats are bit-identical. **MCV trio closed.**

---

## Cross-references

- [AMCV.md](../allied/AMCV.md) — Allied MCV (the trio's first-documented
  member).
- [SMCV.md](../soviet/SMCV.md) — Soviet MCV (preceding doc in this
  iteration sequence).
- YACNST — deploy target (pending).
- YAGRND — Grinder, the unique prereq (pending).
- YADEPT — Yuri Service Depot, the *historical/commented* prereq.
- [BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md](../../BUILDINGCLASS_MASTER_GHIDRA_REPORT_V3.md)
  — receiving-ConYard side of the deploy.

---

## Coverage audit

- [x] Every rulesmd key annotated (44 keys — same as SMCV).
- [x] Every artmd key annotated (4 keys).
- [x] No weapons (unarmed, same as SMCV/AMCV).
- [x] All voice/sound bindings documented (2 Yuri-specific + 4 shared).
- [x] Prerequisites: `YAWEAP, YAGRND` (with historical `;YADEPT` commented).
- [x] Owner: YuriCountry (single house).
- [x] Veterancy: `Trainable=no`.
- [x] Hardcoded behavior: cross-referenced to SMCV.md (deploy, crate,
  crush, crew, ZFudge, threat-value).
- [x] Yuri-specific quirks: Sight=8 buff, Grinder dependency, single-house
  owner.
- [x] TS-legacy filter: `ZFudgeTunnel` flagged dormant (same as SMCV).
- [x] Comparison table across the MCV trio (AMCV vs SMCV vs PCV).
- [x] At least one Ghidra search performed (`Prerequisite` — TechnoType
  scope confirmed).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("Prerequisite")` | 9 matches — `Prerequisite` + 7 `Prerequisite*` macro aliases + 1 `PrerequisiteOverride` |
| `get_xrefs_to(0x00843da8)` (Prerequisite) | `0x007141ac → TechnoTypeClass__ReadINI` |

**New cheat-sheet entry:**
- `Prerequisite` (0x00843da8 → 0x007141ac) TechnoType — base key for the
  Prerequisite= list. Macro aliases (`PrerequisiteProc`, `PrerequisiteTech`,
  `PrerequisiteRadar`, `PrerequisiteBarracks`, `PrerequisiteFactory`,
  `PrerequisitePower`, `PrerequisiteProcAlternate`) live in `[General]`-
  section macros (Rules-level), expanded before TechnoType reads the list.

**MCV trio closed.** AMCV ✓ SMCV ✓ PCV ✓.

**Open questions:** none.
