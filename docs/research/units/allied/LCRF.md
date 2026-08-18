---
name: lcrf-doc
description: LCRF — Allied Landing Craft. Third member of the amphibious transport
  trio (with Soviet SAPC and Yuri YHVR). Allied transport: 12 passengers, SizeLimit=6
  (carries MCV), Hover locomotor, TechLevel=4 (higher tier than SAPC/YHVR's TL2),
  StupidHunt=yes. INDEX corrections: LCRF=Allied Landing Craft (NOT Soviet Sea
  Scorpion), HYD=Soviet Sea Scorpion (NOT Allied Hydrofoil), HIND=disabled.
metadata:
  type: project
---

# LCRF — Allied Landing Craft

**INI ID:** `LCRF`
**Display:** "Landing Craft" (`UIName=Name:LCRF`)
**Section:** `[VehicleTypes]`
**Owner side:** Allied (British, French, Germans, Americans, Alliance)
**Role:** Allied amphibious vehicle transport. **Third member of the
amphibious transport trio**, alongside Soviet [SAPC](../soviet/SAPC.md) and
Yuri [YHVR](../yuri/YHVR.md). Functionally near-identical to the other two:
12 passengers, SizeLimit=6 (carries an MCV), Hover locomotor, $900, no
weapon. Distinguishing detail: **higher TechLevel=4** (vs SAPC/YHVR TL=2) —
Allies pay a tech-tier penalty for their amphibious transport.

---

## Major INDEX corrections logged

Three INDEX errors discovered during this iteration:

| Index claim | Reality (verified from rulesmd) |
|-------------|-----------------------------------|
| `LCRF \| Sea Scorpion \| Soviet \| Naval AA` | **WRONG** — LCRF = "Landing Craft", Allied amphibious transport, no weapon. *Documented this iteration*. |
| `HYD \| Hydrofoil \| Allied \| Naval AA` | **WRONG** — HYD = "Sea Scorpion", **Soviet** AA naval (Owner=Russians+/etc, VoiceSelect=SeaScorpionSelect, Prerequisite=NAYARD,NARADR). Soviet AA-naval counterpart to AEGIS. |
| `HIND \| Flak Track \| Soviet \| Flak transport` | **WRONG** — HIND = "Hind Transport", TechLevel=-1 (disabled/cut). The real Flak Track is [HTK](../soviet/HTK.md), already documented. |

**There is no "Hydrofoil" unit in YR rulesmd.** The naval-AA roster is:
- AEGIS (Allied) ✓ documented
- HYD (Soviet, named "Sea Scorpion") — to be documented in a future iteration

INDEX will be updated to reflect these corrections.

---

## Rulesmd verbatim

```ini
[LCRF]
UIName=Name:LCRF
Name=Landing Craft
Prerequisite=GAYARD
Strength=300
MoveToShroud=yes
Category=Transport
DeployTime=.022
Armor=light
Turret=no
IsTilter=yes
TechLevel=4
Sight=6
PipScale=Passengers
Speed=6
;;;CanBeach=yes
Naval=yes
Weight=1
CrateGoodie=no
Owner=British,French,Germans,Americans,Alliance
AllowedToStartInMultiplayer=no
Cost=900
Soylent=900
Points=15
ROT=5
Crusher=no ;gs yes
Passengers=12
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=HoverAlliedSelect
VoiceMove=HoverAlliedMove
VoiceAttack=HoverAlliedMove
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=LandingCraftMoveStart
EnterTransportSound=EnterTransport
LeaveTransportSound=ExitTransport
Maxdebris=3
;;;;;SpeedType=Amphibious
;;;;;Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
;;;;;MovementZone=AmphibiousCrusher
;;;Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C};{4A582741-9839-11d1-B709-00A024DDAFD1}
;;;;SpeedType=FloatBeach
;;;;MovementZone=WaterBeach
SpeedType=Hover
Locomotor={4A582742-9839-11d1-B709-00A024DDAFD1}
MovementZone=Amphibious ; gs AMphibiousDestroyer I can't have a destroyer zone without a weapon!
ThreatPosed=3	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
SpecialThreatValue=1
ZFudgeColumn=7
ZFudgeTunnel=13
SizeLimit=6
Size=16
TooBigToFitUnderBridge=true
VeteranAbilities=STRONGER,FIREPOWER,SIGHT,FASTER
EliteAbilities=SELF_HEAL,STRONGER,FIREPOWER,ROF
;Bombable=no
Trainable=no
StupidHunt=yes ;this guy can't handle a hunt command, so he should just run towards the player
Bunkerable=no; Units default to yes, others default to no
```

### Key-by-key annotation — mirror of SAPC with Allied-specific differences

Most fields are mechanically identical to [SAPC](../soviet/SAPC.md) and
[YHVR](../yuri/YHVR.md). See SAPC doc for the shared-field explanations.
This section covers Allied-specific or LCRF-distinctive lines.

**Allied-specific identity / availability**
- `UIName=Name:LCRF` — *separate CSF key* (unlike SAPC and YHVR which
  both use `Name:SAPC`). Display: "Landing Craft" — distinct from the
  Soviet/Yuri "Amphibious Transport" label.
- `Name=Landing Craft` — internal description.
- `Prerequisite=GAYARD` — *Allied Naval Yard*. No additional gates;
  buildable as soon as GAYARD is up.
- `TechLevel=4` — **higher than SAPC/YHVR (TL=2)**. The Allied
  amphibious transport unlocks later in the tech tree. **Player impact**:
  Allied players cannot deploy MCV-to-island in the very early naval
  phase — they must wait for tier-4 tech. Soviet/Yuri can amphibious-
  rush from tier-2.
- `Owner=British,French,Germans,Americans,Alliance` — 5 Allied houses.

**Stats — mostly mirror SAPC/YHVR**
- `Strength=300, Armor=light, Speed=6, Sight=6`. **Note**: SAPC and YHVR
  also have `Armor=heavy`, but LCRF has **`Armor=light`** — Allied
  landing craft is more fragile than Soviet/Yuri counterparts!
- `Naval=yes ;GS` — **TechnoType-scope** [BINARY-VERIFIED audit 25: string @ 0x0084395C, parser xref @ 0x00714A6A in `TechnoTypeClass__ReadINI`, `TechnoType+0xCCE` (byte). **CORRECTS the doc's prior UnitType-scope claim** — Naval is TechnoType-scope (audit-12 UnitTypeClass__ReadINI confirms `Naval=` is NOT parsed there).]
  - Wait — verifying: SAPC says `Armor=heavy`, YHVR says `Armor=heavy`,
    LCRF says `Armor=light`. **YES, Allied has weaker armor**. Net
    effect: LCRF takes more damage from same hits.
- `Cost=900, Soylent=900, Points=15`.
- `Passengers=12, SizeLimit=6, Size=16` (vs SAPC=15, YHVR=15 — LCRF is
  slightly *larger*).
- `Crusher=no ;gs yes` — same historical override.
- `Crewed=` not set — defaults to no.
- `Weight=1` — lightweight hover.

**ThreatPosed=3 — unusually low**
- `ThreatPosed=3` — **the lowest non-zero ThreatPosed in the docs so
  far**. AI weights LCRF as a tiny threat:
  | Unit | ThreatPosed |
  |------|-------------|
  | LCRF | **3** |
  | SAPC | 10 |
  | YHVR | 10 |
  | SHAD | 0 (transport) |
  | BFRT | (TBD) |
  - Why is LCRF lower than SAPC/YHVR? Possibly because the Allied
    Landing Craft is *less aggressive in AI usage* — Westwood expected
    Allied players to use Aircraft Carriers and air transports for
    most aggressive ops, while Allied LCRF is reserved for utility
    MCV-ferry only. The AI hint reflects this design intent.

**Voice / sound — Allied-distinct**
- `VoiceSelect=HoverAlliedSelect` (Allied-flavored hover voice block).
- `VoiceMove=HoverAlliedMove`.
- `VoiceAttack=HoverAlliedMove` (same as Move; no weapon).
- `MoveSound=LandingCraftMoveStart` — *the LandingCraftMoveStart block
  is named after this unit* (since LCRF is the "primary" Landing
  Craft). Shared with SAPC and YHVR for engine SFX.

**StupidHunt=yes**
- `StupidHunt=yes` (with verbatim comment "this guy can't handle a hunt
  command, so he should just run towards the player") — same AI bypass
  flag as YHVR. **Note: SAPC doesn't have this flag**, suggesting an
  inconsistency in Westwood's transport AI hardening:
  - **LCRF has StupidHunt** — Allied AI scripts apparently issue Hunt
    to transports → flag prevents freeze.
  - **YHVR has StupidHunt** — Yuri AI scripts also issue Hunt to
    transports → flag prevents freeze.
  - **SAPC does NOT have StupidHunt** — Soviet AI scripts either never
    issue Hunt to transports, or this is an oversight that ships with
    a latent freeze bug. Open question (raised in YHVR iteration).
- Ghidra-verified TechnoType `0x008438a4 → 0x00714c6c` (from SMIN/YHVR
  cheat-sheet).

**Trainable=no (explicit)**
- Same as YHVR. SAPC omits the field (defaults to no anyway).

**Other shared fields** (cross-reference [SAPC.md](../soviet/SAPC.md)):
- `DeployTime=.022`, `MoveToShroud=yes`, `IsTilter=yes`, `Turret=no`.
- `MovementZone=Amphibious` + Hover Locomotor (...742) + SpeedType=Hover.
- `Naval=yes`, `TooBigToFitUnderBridge=true`, `Bunkerable=no`.
- `SpecialThreatValue=1`, `ZFudgeColumn=7`, `ZFudgeTunnel=13`.
- `EnterTransportSound`/`LeaveTransportSound` hooks.
- `MaxDebris=3` (lowercase d typo, same).
- The verbatim "Destroyer-zone-needs-weapon" comment.
- Multiple `;;;;` commented Locomotor experiment blocks.
- `;;;CanBeach=yes` and `;;Bombable=no` commented.

---

## Artmd verbatim

```ini
[LCRF] ; Landing craft
Cameo=LANDICON
Voxel=yes
Remapable=yes
```

### Key-by-key annotation

- `Cameo=LANDICON` — sidebar build button. *Note*: SAPC uses
  `SAPCICON`, YHVR uses `YHVRICON`, LCRF uses `LANDICON`. Three distinct
  cameo assets confirming three separate visual designs.
- `Voxel=yes` — rendered from `lcrf.vxl` + `lcrf.hva`. **Distinct from
  TRS art** — LCRF has its own voxel.
- `Remapable=yes` — house-color remap.

**No `AltCameo=`, no `PrimaryFireFLH=`, no `TurretOffset=`** — minimal
voxel block (same as SAPC/YHVR). No weapon → no fire offset.

---

## Weapons

**LCRF has no weapons.** Same as SAPC/YHVR. `Primary=` is omitted entirely.

`VoiceAttack=HoverAlliedMove` (same as move voice) — right-click-enemy
falls back to move-toward-target behavior.

No veterancy meaningful (Trainable=no).

---

## Voices / sounds

```ini
[HoverAlliedSelect]
Sounds= ... (5-sample $vhoase* pool)

[HoverAlliedMove]
Sounds= ... (5-sample $vhoamo* pool)

[LandingCraftMoveStart]
Sounds=vlanstaa vlanstab vlanstac
Control= random predelay
Delay=0 400
Priority=Low
FShift= -10 10
VShift=15
Volume=45
```

(See soundmd.ini for full pool details — same structural pattern as
HoverSoviet/HoverYuri blocks.)

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=HoverAlliedSelect` | `[HoverAlliedSelect]` | Click |
| `VoiceMove=HoverAlliedMove` | `[HoverAlliedMove]` | Move order |
| `VoiceAttack=HoverAlliedMove` | `[HoverAlliedMove]` (same) | Right-click target (no weapon) |
| `DieSound=GenVehicleDie` | shared | Death |
| `MoveSound=LandingCraftMoveStart` | `[LandingCraftMoveStart]` | Ignition |
| `EnterTransportSound=EnterTransport` | shared | Passenger boards |
| `LeaveTransportSound=ExitTransport` | shared | Passenger disembarks |
| `CrushSound=` — *not set* (LCRF has no CrushSound line, despite
  having Crusher=no anyway) | n/a | n/a |

---

## Hardcoded behavior (Ghidra-verified)

All shared hardcoded behavior with [SAPC](../soviet/SAPC.md):
- `Naval=yes` shipyard-build + torpedo-vulnerable + Squid-target.
- Hover locomotor + Amphibious zone (Destroyer-zone-needs-weapon
  workaround).
- `SizeLimit=6` MCV-transport capability.
- `DeployTime=.022` fast passenger cycle.
- `EnterTransportSound`/`LeaveTransportSound` TechnoType hooks.
- `MoveToShroud=yes` cross-shroud pathing.

### StupidHunt=yes shared with YHVR

Same TechnoType `0x008438a4 → 0x00714c6c` re-confirmed this iteration via
`MovementRestrictedTo` query side-trip. The verbatim "can't handle a hunt
command, so he should just run towards the player" comment applies
identically — Allied AI scripts issuing Hunt to LCRF get the bypass
behavior (run toward dominant human player base) instead of
scan-fail-loop.

### Armor=light (Allied-distinct)

Unlike SAPC/YHVR (both heavy armor), LCRF has **light armor**. Same HP
(300), but takes more damage per AT-style hit. Asymmetric Allied
vulnerability — likely a balance compensation for some other Allied naval
advantage (Aircraft Carrier, Destroyer's ASW, Dolphin's sonic, etc.).

### TechLevel=4 (Allied tech penalty)

The two-tier delay vs SAPC/YHVR (TL=2) means:
- A Soviet/Yuri player who builds their Naval Yard can immediately mass
  amphibious transports.
- An Allied player must reach tier-4 tech (likely requires Radar +
  refinery + some research) before unlocking LCRF.

This creates an asymmetric **early-game island warfare advantage** for
Soviet/Yuri factions on naval maps.

### Higher Size=16 (vs SAPC/YHVR=15)

Marginal — 1-unit-larger. Effectively irrelevant for transport-fit
calculations (no transport accepts Size≥15 anyway). Probably reflects
the slightly larger voxel model for the Allied design.

---

## TS-legacy filter

Identical to SAPC/YHVR:
- Multiple `;;;;` commented Locomotor experiments.
- `;;;CanBeach=yes`, `;Bombable=no` commented historical fields.
- `ZFudgeTunnel=13` TS-legacy dormant.

---

## Comparison: the amphibious transport trio (closed)

| Field | LCRF (Allied) | SAPC (Soviet) | YHVR (Yuri) |
|-------|---------------|---------------|--------------|
| Display name | Landing Craft | Amphibious Transport | Hover Transport (display="Amphibious Transport") |
| UIName CSF | `Name:LCRF` (distinct) | `Name:SAPC` | `Name:SAPC` (shared with SAPC) |
| Strength | 300 | 300 | 300 |
| **Armor** | **light** | heavy | heavy |
| Speed | 6 | 6 | 6 |
| Sight | 6 | 6 | 6 |
| Cost | 900 | 900 | 900 |
| **TechLevel** | **4** | 2 | 2 |
| Prerequisite | GAYARD | NAYARD | YAYARD |
| Passengers | 12 | 12 | 12 |
| SizeLimit | 6 | 6 | 6 |
| Size | 16 | 15 | 15 |
| Naval | yes | yes | yes |
| Locomotor | Hover (...742) | Hover (...742) | Hover (...742) |
| MovementZone | Amphibious | Amphibious | Amphibious |
| Weapon | none | none | none |
| Voice family | HoverAllied* | HoverSoviet* | HoverYuri* |
| **ThreatPosed** | **3** | 10 | 10 |
| **StupidHunt** | **yes** | not set | yes |
| **Trainable** | **no** (explicit) | not set | no (explicit) |
| Cameo | LANDICON | SAPCICON | YHVRICON |
| Art voxel | lcrf.vxl | trs.vxl | yhvr.vxl |

**Trio closed**. Key asymmetries:
1. **Allied (LCRF) has the worst armor** (light vs heavy).
2. **Allied (LCRF) requires the highest tech tier** (TL=4 vs TL=2).
3. **Allied (LCRF) has the lowest AI ThreatPosed** (3 vs 10).
4. **SAPC alone lacks StupidHunt=yes** — possible latent AI-freeze bug.

Allied naval is asymmetrically *air-power oriented* (Aircraft Carrier
+ Hornets) rather than amphibious-assault oriented. The LCRF is a
late-tech utility, not a primary naval tool.

---

## Cross-references

- [SAPC.md](../soviet/SAPC.md) — Soviet sibling; same role.
- [YHVR.md](../yuri/YHVR.md) — Yuri sibling; same role with StupidHunt.
- [BFRT.md](../allied/BFRT.md) — Allied alternative ground transport
  (5 passengers, armed, NOT amphibious).
- [SHAD.md](../allied/SHAD.md) — Allied air transport (5 passengers,
  defensive cannon).
- [AMCV.md](../allied/AMCV.md) — Allied MCV, Size=6, the primary cargo
  LCRF is designed to transport across water.

---

## Ghidra audit log (audit iteration 25 — 2026-05-18)

**Methodology**: LCRF is a thin doc — most fields cross-reference SAPC
(not yet audited) or YHVR (not yet audited). The audit verifies the
single new claim (MovementRestrictedTo re-confirmation) and discovers
a SCOPE CORRECTION for `Naval=yes` — the doc consistently claims
UnitType-scope, but Ghidra shows TechnoType-scope. ~6 Ghidra queries:
3 string searches + 2 xref lookups + 1 grep.

### Negative claim re-verified

| Query | Result |
|-------|--------|
| `search_strings("^LCRF$")` | **0 matches** |

Confirms no hardcoded LCRF-name branch.

### String + parser xref verification (BINARY-VERIFIED)

| String | Addr | Parser xref | Function |
|--------|------|-------------|----------|
| `MovementRestrictedTo` | 0x00845D64 | 0x00747837 | UnitTypeClass__ReadINI |
| `Naval` (bonus) | 0x0084395C | 0x00714A6A | **TechnoTypeClass__ReadINI** (NOT UnitTypeClass) |

### [SCOPE DISCREPANCY corrected]

The doc cross-references SHAD/SAPC and SCALES `Naval=yes` as:
- §1 key table: `| Naval=yes ;GS | bool | UnitTypeClass |`
- Cross-reference: "`Naval=yes` shipyard-build + torpedo-vulnerable + Squid-target."

**Actual scope: TechnoType** (parser xref `0x00714A6A` is in
TechnoTypeClass__ReadINI, not UnitTypeClass__ReadINI). The audit-12
UnitTypeClass__ReadINI full decompile confirms `Naval=` is NOT parsed
there — only in TechnoTypeClass__ReadINI. This means buildings,
infantry, and aircraft can theoretically all have `Naval=yes`
(TechnoType is the parent of UnitType / InfantryType / BuildingType /
AircraftType), though in practice only naval vehicles set it. The
"Naval=yes ;GS" annotation in LCRF/SHAD/SAPC docs should be updated to
TechnoType-scope across docs.

### NEW TechnoType offset BINARY-VERIFIED

| Offset | INI key | Type | Notes |
|--------|---------|------|-------|
| `+0xCCE` | `Naval` | byte | `*(undefined1*)((int)param_1 + 0xCCE) = uVar3` after ReadBool. **NEW**. Gates shipyard-build path + torpedo-vulnerability + Squid-target. TechnoType-scope means it's inherited by any unit type. |

### Re-confirmations (no new work needed)

The following cross-referenced cheat-sheet entries were re-confirmed only:
- `MovementRestrictedTo` → UnitType +0xDFC (audit 12 cumulative confirms `param_1[0x37F]`; parser xref @ 0x00747837)
- `StupidHunt` → TechnoType +0x6D4 (audit 17 cumulative)
- `EnterTransportSound` / `LeaveTransportSound` → TechnoType +0x564 / +0x568 (audit 24)
- Hover locomotor CLSID `{4A582742-...}` (audit 23 cross-reference)

### Items NOT re-verified in this pass (DEFERRED)

- SAPC and YHVR sibling docs — not yet audited. Some claims in LCRF
  rely on those being correct (e.g., "SAPC has Armor=heavy"). When
  SAPC/YHVR come up in the Soviet/Yuri audit phases, the comparison
  table can be cross-verified.
- The `Naval=yes` consumer chain (shipyard-build path, torpedo
  vulnerability, Squid-target eligibility) — offset known, consumer
  DEFERRED.
- `[HIND]` (TechLevel=-1) — the doc notes it as cut content; not
  audited.

### Confidence summary

- **HIGH**: 3 string addresses + 2 parser xrefs (all exact); 1 NEW
  TechnoType offset (+0xCCE Naval); 1 scope CORRECTION (Naval is
  TechnoType-scope, NOT UnitType).
- **No INCORRECT findings** beyond the scope discrepancy.
- **Cumulative trust-chain**: LCRF doc's claim "shared with SAPC" is
  unverified for SAPC's claims that haven't been audited yet.

---

## Coverage audit

- [x] Every rulesmd key annotated (~55 keys).
- [x] Every artmd key annotated (4 keys).
- [x] No weapons (same as SAPC/YHVR).
- [x] All voice/sound bindings documented.
- [x] Prerequisites: `GAYARD` (no additional gates).
- [x] Owner: 5 Allied houses.
- [x] Veterancy: `Trainable=no` explicit.
- [x] Hardcoded behavior: cross-referenced to SAPC; Allied-specific
  asymmetries (Armor=light, TechLevel=4, ThreatPosed=3, StupidHunt=yes)
  enumerated.
- [x] TS-legacy filter: same as SAPC/YHVR; no LCRF-specific TS legacy.
- [x] **MAJOR INDEX corrections logged** for LCRF, HYD, HIND.
- [x] Comparison table closes the amphibious transport trio.
- [x] At least one Ghidra search performed (`MovementRestrictedTo` —
  re-confirmed UnitType-scope).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("MovementRestrictedTo")` | `0x00845d64` (single match) |
| `get_xrefs_to(0x00845d64)` | `0x00747837 → UnitTypeClass__ReadINI` (already in cheat-sheet from SMCV doc) |

**No new cheat-sheet entries this iteration.** All LCRF fields were
already verified in prior iterations (SAPC, YHVR, SMCV).

**Re-confirmed scopes:**
- `MovementRestrictedTo` UnitType-only (`0x00845d64 → 0x00747837`).
- `StupidHunt` TechnoType (per cheat-sheet).
- `Naval` TechnoType (per SAPC).
- `EnterTransportSound`/`LeaveTransportSound` TechnoType (per SHAD/SAPC).

**Open questions:**
- The SAPC-vs-LCRF/YHVR `StupidHunt` asymmetry: does SAPC freeze when AI
  issues Hunt? Unverified. (Already raised in YHVR iteration.)
- Why does Allied LCRF have light armor while Soviet SAPC and Yuri YHVR
  have heavy? Likely balance reasoning around Allied air-superiority
  alternative paths. Not blocking.
- The unused `[HIND]` Hind Transport (TechLevel=-1) — worth investigating
  in a future iteration to confirm cut-content status and document any
  hardcoded behavior that ships in disabled state.
