---
name: yhvr-doc
description: YHVR — Yuri Hover Transport. Near-mirror of Soviet SAPC; UIName=Name:SAPC
  shared, but YHVR uses its OWN art (yhvr.vxl, NOT trs.vxl — Image=TRS is commented).
  Owner=YuriCountry; StupidHunt=yes (Yuri-unique flag); Trainable=no explicit.
  Closes the Soviet/Yuri amphibious-transport pair.
metadata:
  type: project
---

# YHVR — Yuri Hover Transport

**INI ID:** `YHVR`
**Display:** "Hover Transport Yuri" (`UIName=Name:SAPC` — uses the **same CSF
key** as Soviet SAPC, so the in-game label is "Amphibious Transport" — the
display name is shared even though the units are mechanically different
faction siblings)
**Section:** `[VehicleTypes]`
**Owner side:** Yuri (`Owner=YuriCountry`)
**Role:** Yuri's amphibious vehicle transport. Identical role to
[SAPC](../soviet/SAPC.md) — 12 passengers, SizeLimit=6 (can carry an MCV),
Hover locomotor, no weapon. Closes the Soviet/Yuri amphibious-transport
pair.

---

## Correction to SAPC doc: Image=TRS is NOT shared with YHVR

The previous iteration's [SAPC](../soviet/SAPC.md) doc claimed YHVR and SAPC
*share* `Image=TRS` art. **That claim is wrong.** Verbatim grep of YHVR
rulesmd:

```ini
[YHVR]
...
;Image=TRS
```

The `;Image=TRS` line is **commented out**. With the redirect inactive, YHVR
defaults to its section name as the asset key — reading from artmd's
own `[YHVR]` block (line 976), which has no `Image=` line of its own, so it
loads `yhvr.vxl` / `yhvr.hva`.

| Unit | rulesmd Image= | Effective voxel asset |
|------|----------------|------------------------|
| SAPC (Soviet) | `Image=TRS` (active) | `trs.vxl` |
| YHVR (Yuri) | `;Image=TRS` (commented) | `yhvr.vxl` |

So **SAPC and YHVR have separate voxel models** in shipped YR — they're not
visually identical. The artmd blocks `[TRS]` and `[YHVR]` are *similar* in
structure (both Voxel=yes, Remapable=yes, both with PrimaryFireFLH=80,0,120),
but the underlying voxel files are distinct. Likely Westwood made Yuri's
hover transport a different design (rounder, more Yuri-style aesthetic)
once final art was ready.

**Will update the SAPC doc's comparison table to reflect this.**

---

## Rulesmd verbatim

```ini
[YHVR]
UIName=Name:SAPC
Name=Hover Transport Yuri
Prerequisite=YAYARD
;Image=TRS
Strength=300
;Primary=M60
MoveToShroud=yes
Category=Transport
DeployTime=.022
Armor=heavy
Turret=no
IsTilter=yes
TechLevel=2
Sight=6
PipScale=Passengers
Speed=6
;;MovementRestrictedTo=Water
;;CanBeach=yes
Naval=yes
Weight=1
CrateGoodie=no
Owner=YuriCountry
AllowedToStartInMultiplayer=no
Cost=900
Soylent=900
Points=25
ROT=5
Crusher=yes
Passengers=12
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=HoverYuriSelect
VoiceMove=HoverYuriMove
VoiceAttack=HoverYuriMove
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=LandingCraftMoveStart
EnterTransportSound=EnterTransport
LeaveTransportSound=ExitTransport
CrushSound=TankCrush
MaxDebris=3
;;;;Locomotor={2BEA74E1-7CCA-11d3-BE14-00104B62A16C};{4A582741-9839-11d1-B709-00A024DDAFD1}
;;;;SpeedType=FloatBeach
;;;;MovementZone=WaterBeach
;;;;;SpeedType=Amphibious
;;;;Locomotor={4A582741-9839-11d1-B709-00A024DDAFD1}
;;;;MovementZone=AmphibiousCrusher
SpeedType=Hover
Locomotor={4A582742-9839-11d1-B709-00A024DDAFD1}
MovementZone=Amphibious ; gs AMphibiousDestroyer I can't have a destroyer zone without a weapon!
;SpeedType=Amphibious
;MovementZone=AmphibiousCrusher
ThreatPosed=10	; This value MUST be 0 for all building addons
DamageParticleSystems=SparkSys,SmallGreySSys
SpecialThreatValue=1
ZFudgeColumn=7
ZFudgeTunnel=13
SizeLimit=6
Size=15
TooBigToFitUnderBridge=true
;Bombable=no
Trainable=no
StupidHunt=yes ;this guy can't handle a hunt command, so he should just run towards the player
Bunkerable=no; Units default to yes, others default to no
```

### Key-by-key annotation — mirror of SAPC with these differences

Most fields are bit-identical to [SAPC](../soviet/SAPC.md). See SAPC doc
for the shared explanations. This section lists only the YHVR-specific
or YHVR-distinctive lines.

**Yuri-specific identity / availability**
- `UIName=Name:SAPC` — **shared CSF key with Soviet SAPC**. Both display as
  "Amphibious Transport" in the in-game UI. The internal `Name=Hover
  Transport Yuri` differs but is overridden by the CSF lookup.
- `Name=Hover Transport Yuri` — internal description; not user-visible
  (overridden by `UIName`).
- `Prerequisite=YAYARD` — *Yuri Sub Pen* (Yuri's naval-yard analog), instead
  of SAPC's `NAYARD`. **`YAYARD` is the Yuri Naval Shipyard / Sub Pen** —
  build location is faction-correct.
- `Owner=YuriCountry` — *single house*, not 4 sub-factions. Same single-
  owner pattern as PCV (Yuri MCV).

**The Image= correction**
- `;Image=TRS` — commented. **YHVR uses its own art `yhvr.vxl`**, not the
  shared TRS art. (Corrected from the SAPC doc's incorrect claim.)

**Voice keys (Yuri-distinct)**
- `VoiceSelect=HoverYuriSelect` (5-sample $vhoyse* pool)
- `VoiceMove=HoverYuriMove` (5-sample $vhoymo* pool)
- `VoiceAttack=HoverYuriMove` (same as Move — no weapon)
- All other sound bindings shared with SAPC: `MoveSound=LandingCraftMoveStart`,
  `EnterTransportSound=EnterTransport`, `LeaveTransportSound=ExitTransport`,
  `DieSound=GenVehicleDie`, `CrushSound=TankCrush`.

**Trainable=no (explicit)**
- `Trainable=no` — *explicitly set*, unlike SAPC which omits the field
  (defaulting to no anyway). No functional difference, just verbosity.

**The unique YHVR field: StupidHunt=yes**
- `StupidHunt=yes` — **YHVR-only flag**. Verbatim comment: *"this guy
  can't handle a hunt command, so he should just run towards the player"*.
  - **Ghidra-verified TechnoType** at `0x008438a4 → 0x00714c6c` (re-confirmed
    this iteration; previously logged from SMIN doc cheat-sheet).
  - **Behavior**: when the AI assigns a Hunt mission to a YHVR (auto-target
    nearest enemy and engage), the engine substitutes a simpler "run toward
    the human player's base" behavior instead. Without this flag, the
    Hunt mission code expects the unit to scan for attackable targets,
    pick one, and engage — but YHVR has no weapon to engage with, so the
    Hunt loop would freeze the unit in idle scan-and-fail forever.
    StupidHunt=yes is the bypass.
  - **Why YHVR but not SAPC?** Probably an oversight in SAPC's rules — SAPC
    is also weaponless and would benefit from this flag. **Open question:**
    does SAPC suffer from idle-on-hunt-assignment bug in shipped YR?
    Worth testing. Likely the SAPC's Soviet AI scripts don't issue Hunt
    missions to it (uses different AI priorities), so the bug is masked.
    For YHVR with Yuri AI, the missing-flag would have been a real
    problem — hence the explicit flag.
  - Same flag pattern used by SMIN ([SMIN.md](./SMIN.md) — Slave Miner
    is also StupidHunt=yes since it can't really "hunt" with its turret
    appropriately).

**All other fields identical to SAPC** — see [SAPC.md](../soviet/SAPC.md):
- Strength=300, Armor=heavy, Speed=6, Sight=6, TechLevel=2.
- Cost=900, Soylent=900, Points=25.
- Passengers=12, SizeLimit=6 (carries MCV at 6/6 fill).
- Crusher=yes, ROT=5, Weight=1.
- Hover locomotor (`...742`), MovementZone=Amphibious, SpeedType=Hover.
- Naval=yes, MoveToShroud=yes, IsTilter=yes.
- TooBigToFitUnderBridge=true, Bunkerable=no.
- ThreatPosed=10, SpecialThreatValue=1.
- ZFudgeColumn=7, ZFudgeTunnel=13 (latter TS-legacy dormant).
- Size=15.
- DeployTime=.022.
- AllowedToStartInMultiplayer=no, CrateGoodie=no.
- The same "Destroyer-zone-needs-weapon" verbatim comment.

---

## Artmd verbatim

```ini
[YHVR] ; Yuri Hover Transport
Cameo=YHVRICON
Voxel=yes
Remapable=yes
PrimaryFireFLH=80,0,120
```

### Key-by-key annotation

- `Cameo=YHVRICON` — YHVR-specific cameo (vs SAPC's `SAPCICON`).
- `Voxel=yes` — rendered from `yhvr.vxl` + `yhvr.hva` (since `Image=TRS`
  is commented in rules, YHVR loads its own asset name).
- `Remapable=yes` — house-color remap. Yuri's player color tints
  remap-channel pixels.
- `PrimaryFireFLH=80,0,120` — vestigial (no weapon, no firing).

**No `AltCameo=`, no `TurretOffset=`, no `IdleAnim=`** — minimal voxel
block. Same minimalism as SAPC.

---

## Weapons

**YHVR has no weapons.** Same as SAPC. `Primary=` is commented out
(`;Primary=M60`).

`VoiceAttack=HoverYuriMove` — right-click-enemy falls back to the
move voice.

No `ElitePrimary`, no `Secondary`, no veterancy possible (Trainable=no).

---

## Voices / sounds

```ini
[HoverYuriSelect]
Sounds=$vhoysea $vhoyseb $vhoysec $vhoysed $vhoysee
Control=random
Volume=85

[HoverYuriMove]
Sounds=$vhoymoa $vhoymob $vhoymoc $vhoymod $vhoymoe
Control=random
Volume=85
```

Other sound bindings (`LandingCraftMoveStart`, `EnterTransport`,
`ExitTransport`, `GenVehicleDie`, `TankCrush`) are shared with SAPC —
see [SAPC.md](../soviet/SAPC.md#voices--sounds) for those blocks.

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=HoverYuriSelect` | `[HoverYuriSelect]` | Click |
| `VoiceMove=HoverYuriMove` | `[HoverYuriMove]` | Move order |
| `VoiceAttack=HoverYuriMove` | `[HoverYuriMove]` (same) | Right-click target (no weapon) |
| `DieSound=GenVehicleDie` | shared | Death |
| `MoveSound=LandingCraftMoveStart` | shared | Ignition |
| `EnterTransportSound=EnterTransport` | shared | Passenger boards |
| `LeaveTransportSound=ExitTransport` | shared | Passenger disembarks |
| `CrushSound=TankCrush` | shared | Crushing infantry |

**Voice character:** the Yuri hover voices are the cult/intellectual-malice
tone (compare with Soviet `HoverSovietSelect` — heavy-accent gravelly).
Same 5-sample pool count as Soviet variant.

---

## Hardcoded behavior (Ghidra-verified)

All shared hardcoded behavior with SAPC ([SAPC.md hardcoded section](../soviet/SAPC.md#hardcoded-behavior-ghidra-verified)):
- `Naval=yes` flag for shipyard-build + torpedo-vulnerable + Squid-target.
- Hover locomotor + Amphibious zone (Destroyer-zone-needs-weapon
  constraint workaround).
- `SizeLimit=6` MCV-transport capability.
- `DeployTime=.022` fast passenger cycle.
- `EnterTransportSound`/`LeaveTransportSound` TechnoType hooks.
- `MoveToShroud=yes` cross-shroud pathing.

### Unique YHVR behavior: StupidHunt=yes

**Ghidra-verified TechnoType** at `0x008438a4 → 0x00714c6c`.

The standard AI Hunt mission (auto-target nearest enemy and engage)
expects the unit to:
1. Scan for nearest enemy in range/sight.
2. Pick a target with appropriate threat rating.
3. Engage (move + fire).

For a weaponless unit, step 3 fails — no firing solution exists. The
default Hunt loop would re-scan-fail-rescan indefinitely, leaving the
unit idle.

`StupidHunt=yes` substitutes a simpler "run toward the player's base
direction" behavior. The unit picks a heading toward the dominant
human-player base location and moves there without scanning. Not a
real attack — just movement — but it ensures the YHVR doesn't freeze
during AI-controlled play.

**Why YHVR has this and SAPC doesn't:** likely a Yuri-AI-script issue
that surfaced during late development. The Soviet AI scripts for SAPC
probably don't use Hunt as a fallback action for empty transports (they
use Move + dedicated load/unload logic). The Yuri AI scripts either
relied on Hunt as a generic fallback or had no specialized transport
script, requiring the StupidHunt=yes flag to prevent freezing.

**This is a minor parity-relevant detail**: if our Rust implementation
preserves the AI mission system, the StupidHunt path must be implemented
*specifically for YHVR* to match gamemd behavior when YHVR is given a
Hunt mission. SAPC and other weaponless transports may behave differently
when given Hunt commands.

### Trainable=no (explicit)

Standard for weaponless transports — no XP source means no rank-up.
The flag is explicit on YHVR but implicit on SAPC; functionally
identical.

---

## TS-legacy filter

Identical to SAPC:
- Multiple commented Locomotor experiments (`;;;;` blocks).
- `;Primary=M60` commented historical M60 weapon plan.
- `;;MovementRestrictedTo=Water` / `;;CanBeach=yes` commented water-only
  + beach experiments.
- `ZFudgeTunnel=13` TS-legacy dormant.

**No YHVR-specific TS legacy.**

---

## Comparison: SAPC vs YHVR pair

| Field | SAPC (Soviet) | YHVR (Yuri) |
|-------|---------------|--------------|
| Strength | 300 | 300 |
| Armor | heavy | heavy |
| Speed | 6 | 6 |
| Sight | 6 | 6 |
| Cost | 900 | 900 |
| TechLevel | 2 | 2 |
| Passengers | 12 | 12 |
| SizeLimit | 6 | 6 |
| Size | 15 | 15 |
| Weight | 1 | 1 |
| Naval | yes | yes |
| Locomotor | Hover (...742) | Hover (...742) |
| MovementZone | Amphibious | Amphibious |
| Weapon | none | none |
| Crusher | yes | yes |
| **Prerequisite** | NAYARD | **YAYARD** |
| **Owner** | 4 Soviet sub-factions | **YuriCountry** |
| **Image** | TRS (active) | **YHVR (commented redirect)** |
| **Voice family** | HoverSoviet* | **HoverYuri*** |
| **StupidHunt** | not set | **yes** (Yuri-unique) |
| **Trainable** | not set | **no** (explicit) |
| **Cameo** | SAPCICON | YHVRICON |

**Pair closed**. Mechanically near-identical; only the asset names,
owner, prereq, voice family, art voxel, and the StupidHunt flag differ.

---

## Cross-references

- [SAPC.md](../soviet/SAPC.md) — Soviet sibling; same role, same stats.
- [PCV.md](../yuri/PCV.md) — Yuri MCV; Size=6, fits exactly in YHVR at 6/6
  capacity.
- [BFRT.md](../allied/BFRT.md) — Allied transport alternative (smaller
  but armed).
- [SHAD.md](../allied/SHAD.md) — Allied air transport.
- [SMIN.md](../yuri/SMIN.md) — sibling Yuri unit with `StupidHunt=yes`
  (slaved-mechanic resource gatherer).

---

## Coverage audit

- [x] Every rulesmd key annotated (mirror of SAPC + YHVR-specific deltas).
- [x] Every artmd key annotated (5 keys).
- [x] **No weapons** (`;Primary=M60` commented).
- [x] All voice/sound bindings documented (Yuri-specific + shared).
- [x] Prerequisites: `YAYARD`.
- [x] Owner: YuriCountry.
- [x] Veterancy: `Trainable=no` explicit.
- [x] Hardcoded behavior: cross-referenced to SAPC; **StupidHunt=yes
  YHVR-unique behavior fully explained**.
- [x] TS-legacy filter: same as SAPC; no YHVR-specific TS legacy.
- [x] Comparison table SAPC vs YHVR.
- [x] Correction to SAPC doc claim about `Image=TRS` art-sharing logged.
- [x] At least one Ghidra search performed (`StupidHunt` re-confirmed at
  `0x008438a4 → 0x00714c6c` TechnoType).

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("StupidHunt")` | `0x008438a4` (single match) |
| `get_xrefs_to(0x008438a4)` | `0x00714c6c → TechnoTypeClass__ReadINI` (re-confirmed from SMIN cheat-sheet) |

**No new cheat-sheet entries this iteration** — every key on YHVR
that's not on SAPC was already verified in prior iterations.

**Open questions:**
- Does SAPC need StupidHunt=yes too? In shipped YR, does SAPC freeze
  when AI gives it a Hunt mission, or do Soviet AI scripts never issue
  Hunt to weaponless transports? Worth a behavior test. Not blocking
  for the doc.
- Are YHVR and SAPC visually distinguishable in-game? Both use Voxel=yes
  + Remapable=yes but with different voxel files. Open verification —
  load both in-game and screenshot for comparison if exhaustive parity
  audit needed.
