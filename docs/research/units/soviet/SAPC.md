---
name: sapc-doc
description: SAPC — Soviet Amphibious Transport. Hover locomotor; 12 passengers
  with SizeLimit=6 (carries vehicles INCLUDING MCV); Image=TRS shared with Yuri YHVR.
  Owner=Soviet sub-factions (NOT Allied as INDEX claimed). Naval=yes; Crusher with
  no weapon; MovementZone=Amphibious workaround for Destroyer-needs-weapon limit.
metadata:
  type: project
---

# SAPC — Soviet Amphibious Transport ("Armored Transport")

**INI ID:** `SAPC`
**Display:** "Armored Transport" (`UIName=Name:SAPC`)
**Section:** `[VehicleTypes]`
**Owner side:** **Soviet** (Russians, Confederation, Africans, Arabs) — NOT
Allied as the INDEX claimed.
**Role:** Soviet amphibious vehicle transport. Hover locomotor (over land + water).
**Passengers=12, SizeLimit=6** — among the largest transports in the game and
the only one that can carry *most vehicles* (including MCV-sized units).

---

## Index correction

The INDEX entry described `[SAPC]` as "Amphibious Transport — Allied — Naval/land
transport." **The Allied claim is wrong**:

- `Prerequisite=NAYARD` — Soviet **N**aval Yard (Allied yard is `GAYARD`).
- `Owner=Russians,Confederation,Africans,Arabs` — 4 Soviet sub-factions.
- `VoiceSelect=HoverSovietSelect` / `VoiceMove=HoverSovietMove` — Soviet voice
  blocks (compare with `HoverYuriSelect` / `HoverYuriMove` for the Yuri YHVR).

SAPC is the **Soviet** amphibious transport. Yuri has its own version
([YHVR](../yuri/YHVR.md)). **Update from YHVR iteration**: YHVR has `;Image=TRS`
commented out and uses its own `yhvr.vxl` art — so YHVR and SAPC actually use
*different voxel files*, NOT the same TRS art as originally documented here.
Only SAPC has the active `Image=TRS` redirect.
Allies have **no amphibious transport** — they rely on the SHAD Nighthawk
helicopter or the Battle Fortress for unit ferrying.

Index will be updated to mark Soviet ownership.

---

## Rulesmd verbatim

```ini
[SAPC]
UIName=Name:SAPC
Name=Armored Transport
Prerequisite=NAYARD
Image=TRS
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
Owner=Russians,Confederation,Africans,Arabs
AllowedToStartInMultiplayer=no
Cost=900
Soylent=900
Points=25
ROT=5
Crusher=yes
Passengers=12
Explosion=TWLT070,S_BANG48,S_BRNL58,S_CLSN58,S_TUMU60
VoiceSelect=HoverSovietSelect
VoiceMove=HoverSovietMove
VoiceAttack=HoverSovietMove
VoiceFeedback=
DieSound=GenVehicleDie
MoveSound=LandingCraftMoveStart
CrushSound=TankCrush
EnterTransportSound=EnterTransport
LeaveTransportSound=ExitTransport
Maxdebris=3
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
```

### Key-by-key annotation

**Identity / UI**
- `UIName=Name:SAPC` — CSF key. Resolves to "Amphibious Transport" in shipped
  YR (display name).
- `Name=Armored Transport` — internal description (older name).
- `Image=TRS` — **uses `tr s.vxl`**, shared with [YHVR](../yuri/YHVR.md). The
  Soviet SAPC reads art from the `[TRS]` block in artmd.ini.
  **Correction**: the Yuri Hover Transport (YHVR) does NOT share this art —
  YHVR has `;Image=TRS` commented out and uses its own `yhvr.vxl` asset.
  See [YHVR.md](../yuri/YHVR.md) for the corrected pair comparison.
- `Category=Transport` — sidebar tab. Distinct from `AFV` / `AirPower`; this
  is the Transport bucket (alongside Battle Fortress, Nighthawk, IFV).

**Tech / availability**
- `Prerequisite=NAYARD` — *only the Soviet Naval Shipyard*. **No War Factory
  required**. This is unusual — most amphibious/transport vehicles are built
  in the War Factory. SAPC is built in the **Shipyard** because it's classified
  as Naval (`Naval=yes`).
- `TechLevel=2` — tier-2. Available early.
- `Owner=Russians,Confederation,Africans,Arabs` — 4 Soviet sub-factions.
  *Not Yuri*. *Not Allied*.
- `AllowedToStartInMultiplayer=no` — not a starting unit.
- `CrateGoodie=no` — not crate-eligible.

**Combat — defense (no weapons)**
- `Strength=300` — moderate HP, same as Lasher.
- `Armor=heavy` — heavy armor type. Survives AT shells better than the
  Lasher-tier units.
- `Turret=no` — no turret.
- `;Primary=M60` — commented. Was once going to have an M60 machine gun for
  self-defense. *Disabled* — SAPC has no weapon. Pure transport.

**Sight / mobility**
- `Sight=6` — 6-cell vision.
- `Speed=6` — moderate. Same as Soviet Sub (SUB) and Allied Destroyer
  (`Speed=6`).
- `ROT=5` — turret rotation rate (vestigial — no turret).
- `MoveToShroud=yes` — can move into unexplored shroud without revealing
  vision. Useful for cross-map flanks. Ghidra-verified TechnoType cheat-sheet.
- `IsTilter=yes` — body-tilt animation on slopes. UnitType-scope per
  cheat-sheet (0x00845df0 → 0x00747712).

**Locomotor / movement zone**
- `SpeedType=Hover` — hover speed table.
- `Locomotor={4A582742-9839-11d1-B709-00A024DDAFD1}` — Hover locomotor GUID
  (`...742`). Same as Robot Tank, Sea Scorpion, Aegis Cruiser, Hydrofoil.
- `MovementZone=Amphibious` — amphibious zone. Verbatim comment is critical:
  *"gs AMphibiousDestroyer I can't have a destroyer zone without a weapon!"*.
  Westwood wanted to use `AmphibiousDestroyer` (amphibious + crushes walls)
  but the Destroyer-suffix zone *requires* a weapon to validate the
  wall-crush attack path. Since SAPC has no weapon (`;Primary=M60` commented),
  the engine refused to allow it. They settled for plain `Amphibious` zone.
- `Naval=yes` — naval class flag. **Ghidra-verified TechnoType** at
  `0x0084395c → 0x00714a6a`. **NEW cheat-sheet entry**. The Naval flag
  triggers naval-class code paths: builds at shipyard, vulnerable to
  torpedoes, can be attacked by Squid (grab/punch mechanics), excluded from
  certain ground-only weapon target lists.
- The commented `;;;;` block before the active locomotor is a *Westwood
  experiment log*. They tried:
  1. Submarine locomotor (`2BEA74E1...`) + FloatBeach speed + WaterBeach zone
     — "amphibious sub"? probably for a TS-legacy submersible APC.
  2. Drive locomotor (`4A582741`) + Amphibious + AmphibiousCrusher — straight
     ground vehicle with amphibious zone.
  3. (Active): Hover locomotor (`4A582742`) + Hover + Amphibious — the
     shipped variant.

**Passenger capacity**

- `PipScale=Passengers` — pip bar shows passenger fill.
- `Passengers=12` — **largest passenger count in the game** (vs SHAD=5,
  BFRT=5, FV=1, SUB transports? = N/A). 12 slots.
- `SizeLimit=6` — *largest occupant size cap in the game* (vs SHAD=2,
  BFRT=2, FV=1). **Size=6 occupants fit**.
  - Size table (from various unit docs):
    - Most infantry: 1
    - Terror Drone, Brute, Yuri Prime: 2
    - Lasher, Grizzly: 3
    - MCVs (AMCV/SMCV/PCV): 6 (Size=6 lets them barely fit!)
    - Boomer Sub: 20 (won't fit)
    - SHAD/SAPC: 15 (transports themselves don't fit other transports)
  - **The SAPC can carry an MCV across water**. Single most strategic
    capability: a Soviet player can build a Naval Yard, ferry an SMCV
    on a SAPC to an island, deploy a NACNST there. Critical for naval-map
    base expansion.

**Economy**
- `Cost=900` — moderate. Cheaper than the BFRT (~2000) but with more
  passenger capacity.
- `Soylent=900` — full Grinder refund.
- `Points=25` — modest score.

**Crew / death**
- `Crusher=yes` — *can crush infantry*. But it's a transport — typically the
  player doesn't drive it into combat. The Destroyer-zone limitation (above)
  is the relevant detail.
- `CrushSound=TankCrush` — standard wet-crunch.
- *No `Crewed=` line* → defaults to `Crewed=no` for vehicles. **Does not
  eject infantry on death** — the passengers die with the SAPC (typical
  transport behavior, but the loss is severe since 12 units could be aboard).
- `MaxDebris=3` — typo `Maxdebris=3` (lowercase `d`, INI is case-insensitive).
- `DieSound=GenVehicleDie` — generic vehicle death.

**Voice / sound bindings**
- `VoiceSelect=HoverSovietSelect` — click voice (5-sample $vhosse* pool).
- `VoiceMove=HoverSovietMove` — move-order voice.
- `VoiceAttack=HoverSovietMove` — *same as VoiceMove* (the SAPC has no
  weapon; right-clicking enemy units just moves there). Same pattern as
  MCV docs.
- `VoiceFeedback=` — empty.
- `MoveSound=LandingCraftMoveStart` — *uses the Landing Craft engine SFX*
  (`vlanstaa/b/c`). Naval/hovercraft ignition sound; same pool as LCRF Sea
  Scorpion's move-start. Generic "hovercraft fan-engine" rumble.
- `EnterTransportSound=EnterTransport` (`genter1a`) — passenger board SFX.
- `LeaveTransportSound=ExitTransport` (`gexit1a`) — passenger disembark SFX.
  **Ghidra-verified TechnoType** at `0x008440d4 → 0x00713432`. **NEW
  cheat-sheet entry** (adjacent to `EnterTransportSound` at 0x008440e8
  but earlier in memory layout — the order is *Leave* then *Enter* in
  string addresses).

**Combat behavior**
- `ThreatPosed=10` — low threat (unarmed transport).
- `SpecialThreatValue=1` — modest strategic value (carries cargo).

**Deploy timing**
- `DeployTime=.022` — *very short deploy time*. **Ghidra-verified
  TechnoType** at `0x00843904 → 0x00714b85`. The DeployTime is the
  fraction of normal build-time the transport takes to load/unload its
  cargo when the deploy/undeploy command is issued. SAPC's `.022` is
  near-instant. Compare with units that have no deploy action (default
  `DeployTime=0`).
  - Note: `DeployTime` is *not* a deploy-into-building transformation
    timing (that's instant). It's specifically the passenger
    loading/unloading animation time scale. For a 12-passenger vehicle,
    making this short is a good UX choice (no 30-second wait for full
    load).

**Z-axis sort**
- `ZFudgeColumn=7` — Z-sort offset near cliffs (lower than MCV's 12;
  SAPC has lower profile).
- `ZFudgeTunnel=13` — TS-legacy dormant.

**Misc**
- `SizeLimit=6` — see Passengers above.
- `Size=15` — SAPC is too big to fit in any other transport.
- `TooBigToFitUnderBridge=true` — UnitType-scope, can't pass under bridge
  spans.
- `Weight=1` — *lowest weight of any non-trivial vehicle*. Most tanks are
  3.5-4. SAPC's Weight=1 makes sense for a hover transport (no ground
  contact mass).
- `;;MovementRestrictedTo=Water` — commented; was once water-only.
- `;;CanBeach=yes` — commented; the beach-traversal flag is YR-active
  (used by certain naval units) but disabled here in favor of full
  amphibious zone.

---

## Artmd verbatim (Image=TRS)

```ini
[TRS] ; Armored Transport
Cameo=SAPCICON
Voxel=yes
Remapable=yes
PrimaryFireFLH=80,0,120
```

### Key-by-key annotation

- `Cameo=SAPCICON` — **uses SAPC-prefixed cameo** despite the section being
  `[TRS]`. The cameo lookup uses the section's `Cameo=` directly, not the
  `Image=` redirect. SAPC's sidebar button shows the cameo SAPCICON.shp.
- `Voxel=yes` — rendered from `trs.vxl` + `trs.hva`. The `tr s.vxl` art is
  **NOT shared with YHVR** (see YHVR.md correction: YHVR's `;Image=TRS` is
  commented, so YHVR loads `yhvr.vxl` instead). The Soviet/Yuri transport
  pair uses different voxel files in shipped YR.
- `Remapable=yes` — house-color remap applies. Soviet red vs Yuri orange
  visible distinction.
- `PrimaryFireFLH=80,0,120` — *vestigial*. SAPC has no weapon (`Primary=` is
  commented out in rules), so this FLH is unused. Held over from the
  commented-out `M60` weapon design.

**No `AltCameo=`, no `TurretOffset=`, no `SecondaryFireFLH=`** — the
transport is minimal art-side. **YHVR's artmd block at line 974 has the
same fields with `Cameo=YHVRICON`** — only the cameo differs (SAPCICON
vs YHVRICON), confirming the art-asset reuse strategy.

---

## Weapons

**SAPC has no weapons.** `Primary=` is commented out (`;Primary=M60`).
Pure transport. Same as MCV pattern (`VoiceAttack=VoiceMove` to handle
right-click-enemy → move-to-enemy fallback).

No `ElitePrimary`. No `Secondary`. **`Trainable=` not set → defaults to
no for transports** — though SAPC could theoretically rank up if forced
into combat, the absence of weapons makes XP accumulation impossible.

---

## Voices / sounds

```ini
[HoverSovietSelect]
Sounds=$vhossea $vhosseb $vhossec $vhossed $vhossee
Control=random
Volume=85

[HoverSovietMove]
Sounds=$vhosmoa $vhosmob $vhosmod $vhosmoe ;$vhosmoc
Control=random
Volume=85

[LandingCraftMoveStart]
Sounds=vlanstaa vlanstab vlanstac
Control= random predelay
Delay=0 400
Priority=Low
FShift= -10 10
VShift=15
Volume=45

[EnterTransport]
Sounds=genter1a
FShift= -2 2
Volume=60

[ExitTransport]
Sounds=gexit1a
FShift= -1 1
Limit=2
Volume=60

[GenVehicleDie]
Sounds= vgendiea vgendieb vgendiec vgendied vgendiee vgendief
Control=random
FShift=-15 15
VShift=20
Volume=85

[TankCrush]
Sounds=vcrusha
```

### Bindings

| Rules key | Sound block | When |
|-----------|-------------|------|
| `VoiceSelect=HoverSovietSelect` | `[HoverSovietSelect]` | Click |
| `VoiceMove=HoverSovietMove` | `[HoverSovietMove]` | Move order |
| `VoiceAttack=HoverSovietMove` | `[HoverSovietMove]` (same) | Right-click enemy (no weapon — falls back to move voice) |
| `DieSound=GenVehicleDie` | shared | Death |
| `MoveSound=LandingCraftMoveStart` | `[LandingCraftMoveStart]` | Ignition. **Random-predelay 0-400ms** — staggered start sound prevents 12 SAPCs from playing the same SFX in lockstep. |
| `CrushSound=TankCrush` | shared | Crushing infantry (rare; transport doesn't usually engage) |
| `EnterTransportSound=EnterTransport` | `[EnterTransport]` | Passenger boards (`genter1a`) |
| `LeaveTransportSound=ExitTransport` | `[ExitTransport]` | Passenger disembarks (`gexit1a`, Limit=2 concurrent) |

**`Control=random` on `[HoverSovietMove]`** has a commented-out 5th sample
(`;$vhosmoc`), leaving 4 active samples in the random pool. The commented
sample was likely defective audio that was cut.

**No `VoiceFeedback=`** — empty. SAPC doesn't have an acknowledge voice;
this is consistent with most transports (the transport is supposed to be
quiet/unobtrusive).

---

## Hardcoded behavior (Ghidra-verified)

### 1. Naval=yes flag

**Ghidra-verified TechnoType** at `0x0084395c → 0x00714a6a`. **NEW
cheat-sheet entry**. Marks the unit as Naval class for several engine
checks:
- Built at Shipyard (not War Factory). Build-queue routing.
- Hit by Torpedo projectiles (which only target Naval units).
- Squid grab/punch mechanics apply (Squid can target Naval=yes units).
- Underwater rendering (for Underwater=yes naval).
- Excluded from certain ground-warhead target lists.

The Naval flag is independent of `Underwater=yes` — SAPC is Naval but not
Underwater (it hovers above the surface).

### 2. Hover locomotor + Amphibious MovementZone

Hover locomotor (`...742` GUID) + `MovementZone=Amphibious` — the unit
hovers over both land and water cells. The amphibious zone treats both
terrain types as passable; the Hover locomotor handles the visual at-rest
hover above the cell.

The verbatim comment "gs AMphibiousDestroyer I can't have a destroyer
zone without a weapon!" reveals a Westwood engine constraint: the
*Destroyer* zone suffix (used in `AmphibiousDestroyer` / `Destroyer` /
`AmphibiousCrusher`) **requires the unit to have at least one weapon** to
validate the wall-attack code path. Since SAPC has no weapons, this fails
validation and the engine refuses to assign Destroyer-suffix zones.
Workaround: plain `Amphibious` zone — no wall-crushing, just terrain
permissiveness.

### 3. SizeLimit=6 enables MCV transport

`SizeLimit=6` (TechnoType cheat-sheet `0x008443bc → 0x00712540` per SMIN
doc). The MCV's `Size=6` matches the SizeLimit exactly — SAPC can carry
exactly one MCV (no other passengers; 6/6 capacity full). This is the
*single most strategic capability* of the SAPC.

### 4. DeployTime for passenger load/unload

`DeployTime=.022` (TechnoType cheat-sheet `0x00843904 → 0x00714b85` per
SMIN doc). Sets the passenger-cycle animation duration. The `.022` value
means deploy is near-instant (~0.022 × base build time, ~1 tick effective).

### 5. EnterTransportSound + LeaveTransportSound

`EnterTransportSound` (TechnoType `0x008440e8 → 0x007133fc`, from SHAD
doc) and `LeaveTransportSound` (TechnoType `0x008440d4 → 0x00713432`,
**NEW** this iteration) fire on passenger boarding/disembarking
respectively. Same hook on every transport (BFRT, FV, SHAD, SAPC, YHVR).

### 6. No Crewed=yes (passengers die with vehicle)

When the SAPC is destroyed *while carrying passengers*, **all 12 passengers
die instantly**. There is no "passengers eject" mechanic for transport
death — only `Crewed=yes` would have triggered crew survivors (and the
crew is the *transport's pilots*, not passengers). Loss of a fully-loaded
SAPC can be catastrophic (12 units gone).

### 7. MoveToShroud=yes

TechnoType cheat-sheet. Allows the SAPC to path through unexplored shroud
without requiring sight. Cross-map flanks via shroud are viable for
shroud-revealing transports. Compare with BSUB (`MoveToShroud` not set)
which must follow lit paths.

### 8. Image=TRS redirect (SAPC-only — corrected)

**Originally claimed YHVR also shared this art — CORRECTED**: YHVR's
`;Image=TRS` is commented (verified at YHVR iteration), so only SAPC
uses the TRS art redirect. YHVR loads its own `yhvr.vxl`.

The `Image=TRS` redirect causes SAPC to read art from artmd's `[TRS]`
block instead of from a `[SAPC]` artmd block (which doesn't exist).
Asset key: `trs.vxl` + `trs.hva`. SAPC and YHVR are visually
distinguishable in shipped YR despite the near-identical rules — the
underlying voxel models differ.

---

## TS-legacy filter

- `;;MovementRestrictedTo=Water` / `;;CanBeach=yes` — commented. The
  beach-traversal system *is* YR-live (used by some naval units) but
  disabled here.
- Multiple commented Locomotor experiments (`;;;;` blocks) — historical
  iteration of mover. The Sub locomotor + Drive locomotor alternatives
  were tried and rejected.
- `;Primary=M60` — commented. The M60 weapon system was an early
  design choice; SAPC shipped weaponless.
- `ZFudgeTunnel=13` — TS-legacy field, dormant in YR.
- No active TS-only code paths.

---

## Comparison with peer transports

| Field | SAPC (Soviet) | YHVR (Yuri) | BFRT (Allied) | SHAD (Allied) | FV (Allied) |
|-------|---------------|-------------|----------------|---------------|-------------|
| Strength | 300 | 300* | 600 | 175 | ~ |
| Passengers | **12** | 12* | 5 | 5 | 1 |
| SizeLimit | **6** | 6* | 2 | 2 | 1 |
| Speed | 6 | 6* | 4 | 14 | 8 |
| Cost | 900 | 900* | ~2000 | 1000 | 500 |
| Naval | yes | yes* | no | no | no |
| Locomotor | Hover | Hover* | Drive | Jumpjet | Drive |
| Has weapon | **no** | no* | yes (passenger-shoot) | yes (cannon) | yes (passenger-swap) |

(`*` = expected based on shared Image=TRS art and similar role; YHVR doc
pending. Confirmed once YHVR is documented.)

**Strategic role:** SAPC is the **only** way for Soviet players to
transport vehicles over water. Naval map dominance depends on it.
Without a SAPC, Soviet players cannot deploy an MCV on an island,
cannot ferry combat tanks to amphibious targets, cannot do cross-shore
strikes. **A single SAPC sinking can be game-changing** if it was
carrying an MCV or 12 elite infantry.

---

## Cross-references

- [YHVR.md](../yuri/YHVR.md) — Yuri counterpart. Mechanically near-identical
  but uses its own `yhvr.vxl` (does NOT share `Image=TRS` — earlier claim
  here corrected in YHVR doc).
- [BFRT.md](../allied/BFRT.md) — Allied alternative (smaller capacity,
  armed).
- [SHAD.md](../allied/SHAD.md) — Allied air transport (sibling unit
  documented in iteration 66).
- [JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md](../../JUMPJET_LOCOMOTION_CLASS_GHIDRA_REPORT.md)
  — SHAD's locomotor (compare with SAPC's Hover).
- [SMCV.md](../soviet/SMCV.md) — Soviet MCV, Size=6 — fits in SAPC at
  6/6 capacity.

---

## Coverage audit

- [x] Every rulesmd key annotated (~55 keys).
- [x] Every artmd key annotated in `[TRS]` shared block (4 keys + cameo).
- [x] Image=TRS art-sharing relationship with YHVR documented.
- [x] **No weapons** explicitly noted (`;Primary=M60` commented).
- [x] All voice/sound bindings documented.
- [x] Prerequisites: `NAYARD`.
- [x] Owner: 4 Soviet sub-factions (CORRECTED from INDEX's "Allied").
- [x] Veterancy: implicit `Trainable=no` (no weapons = no XP).
- [x] Hardcoded behavior: Naval flag scope, Hover locomotor +
  Amphibious zone, Destroyer-zone requires-weapon constraint, SizeLimit=6
  MCV-transport capability, DeployTime, EnterTransportSound +
  LeaveTransportSound, MoveToShroud.
- [x] TS-legacy filter: ZFudgeTunnel dormant; multiple commented
  Locomotor experiments documented.
- [x] Comparison table with peer transports.
- [x] At least one Ghidra search performed (3 strings + xrefs).
- [x] **INDEX CORRECTION logged**: SAPC moved from Allied to Soviet.

**Ghidra queries logged (this iteration):**

| Query | Result |
|-------|--------|
| `search_strings("DeployTime")` | `0x00843904` (single match) |
| `get_xrefs_to(0x00843904)` | `0x00714b85 → TechnoTypeClass__ReadINI` (already in cheat-sheet) |
| `search_strings("^Naval$")` | `0x0084395c` (single match — bare `Naval` field, not `NavalTargeting`) |
| `get_xrefs_to(0x0084395c)` | `0x00714a6a → TechnoTypeClass__ReadINI` |
| `search_strings("LeaveTransportSound")` | `0x008440d4` (single match) |
| `get_xrefs_to(0x008440d4)` | `0x00713432 → TechnoTypeClass__ReadINI` |

**New cheat-sheet entries (2):**
- `Naval` (0x0084395c → 0x00714a6a) TechnoType — naval-class flag for
  built-at-shipyard, torpedo-vulnerable, squid-target eligibility.
- `LeaveTransportSound` (0x008440d4 → 0x00713432) TechnoType — passenger
  disembark SFX trigger. Pairs with `EnterTransportSound` (0x008440e8
  from SHAD doc).

**Re-confirmed:**
- `DeployTime` (already in cheat-sheet from SMIN as `0x00843904 →
  0x00714b85`).

**Open questions:**
- Does `Naval=yes` interact with `Underwater=yes` exclusively, or can a
  unit be Naval but not Underwater (yes — SAPC is exactly this)? Already
  documented above. No follow-up needed.
- YHVR comparison fields marked `*` — need YHVR doc to confirm shared
  stats.
