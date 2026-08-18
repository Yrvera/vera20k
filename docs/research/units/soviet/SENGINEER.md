# Soviet Engineer (SENGINEER)
Side: Soviet | Category: Infantry | Image alias: `Image=ENGINEER` (shares SHP/sequence/cameo)

The Soviet faction's Engineer. Mechanically identical to the Allied
[ENGINEER](../allied/ENGINEER.md) and Yuri [YENGINEER](../yuri/YENGINEER.md) —
$500 from NAHAND, captures enemy buildings, repairs damaged friendly buildings,
defuses Crazy Ivan bombs, becomes a Medic when boarding a captured IFV. The
only differences from the canonical Allied ENGINEER dossier are:

1. `Owner=` / `ForbiddenHouses=` — restricted to the **four Soviet houses**
   (Russians, Confederation, Africans, Arabs).
2. `VoiceSelect/Move/Attack/Feedback/SpecialAttack/Enter/Capture=` — Soviet
   voice bank (`EngSov*`).
3. `DieSound=EngSovDie` — Soviet die sound (shared with YENGINEER).
4. **No separate `[SENGINEER]` artmd section** — art lookup is routed via the
   rules-side `Image=ENGINEER`, so SENGINEER inherits ENGINEER's cameo
   (`ENGNICON`), AltCameo, and sequence verbatim. Allied and Soviet engineers
   are visually identical in-game; only voice and `Owner=` differ.

Everything else — INI key set, stats, weapon chain, hardcoded
capture/repair/disarm/IFV behavior — is **bit-identical** to ENGINEER.
This is a quick-reference doc; cross-reference the canonical
[ENGINEER.md](../allied/ENGINEER.md) for the full surface.

---

## rulesmd.ini — `[SENGINEER]` section

Verbatim from `ini/rulesmd.ini:4461`:

```ini
[SENGINEER]
UIName=Name:ENGINEER
Image=ENGINEER
Name=Soviet Engineer
Category=Soldier
Primary=DefuseKit
Secondary=VirtualScanner ; gs the computer uses range to determine what buildings to run to and capture
Prerequisite=Barracks
CrushSound=InfantrySquish
LeadershipRating=3
Strength=75
Armor=none
TechLevel=1
Sight=4
BombSight=4 ; detecting ivan's little friends
Speed=4
Pip=Blue
Engineer=yes
Owner=Russians,Confederation,Africans,Arabs,YuriCountry,British,French,Germans,Americans,Alliance
ForbiddenHouses=British,French,Germans,Americans,Alliance,YuriCountry
AllowedToStartInMultiplayer=no
Cost=500
Soylent=250
Points=5
IsSelectableCombatant=no
VoiceSelect=EngSovSelect
VoiceMove=EngSovMove
VoiceAttack=EngSovMove
VoiceFeedback=EngSovFear
VoiceSpecialAttack=EngSovAttackCommand
VoiceEnter=EngSovMove
VoiceCapture=EngSovAttackCommand
DieSound=EngSovDie
Locomotor={4A582744-9839-11d1-B709-00A024DDAFD1}
PhysicalSize=1
MovementZone=Infantry
ThreatPosed=0	; This value MUST be 0 for all building addons
SpecialThreatValue=1	; this should be between 0 and 1
ImmuneToVeins=yes
GuardRange=9
Size=1
PreventAttackMove=yes
IFVMode=1
Trainable=no
```

### Keys that differ from `[ENGINEER]`

| Key | SENGINEER value | ENGINEER value | Notes |
|-----|------|------|-------|
| `UIName=Name:ENGINEER` | Same CSF key | Same | Resolves to "Engineer" — Soviet faction does not get a distinct in-game name; tooltip says "Engineer" |
| `Image=ENGINEER` | Redirect to ENGINEER SHP | (no Image=) | Soviet Engineer uses the **same SHP** as Allied Engineer — no Soviet-specific sprite. Voice is the only player-visible distinction (the cameo is also shared via Image redirect) |
| `Name=Soviet Engineer` | Internal short name | "Engineer" | Internal only — `UIName=` overrides the display |
| `Owner=` | Lists all 10 houses | Lists all 10 houses | Identical full list — filter happens via ForbiddenHouses |
| `ForbiddenHouses=` | `British,French,Germans,Americans,Alliance,YuriCountry` (5 Allied + 1 Yuri) | Excludes Soviet+Yuri | Net Owner: **Russians, Confederation, Africans, Arabs** (the four Soviet houses) |
| `VoiceSelect=EngSovSelect` | Soviet voice bank | EngAllSelect | |
| `VoiceMove=EngSovMove` | Soviet voice bank | EngAllMove | |
| `VoiceAttack=EngSovMove` | Reuses move (same pattern as Allied) | EngAllMove | Engineer has no attack — VoiceAttack fires on right-click invalid targets |
| `VoiceFeedback=EngSovFear` | Soviet fear voice | EngAllFear | |
| `VoiceSpecialAttack=EngSovAttackCommand` | Soviet capture/special voice | EngAllAttackCommand | |
| `VoiceEnter=EngSovMove` | Reuses move (same pattern as Allied) | EngAllMove | Plays on board-transport / enter-building |
| `VoiceCapture=EngSovAttackCommand` | Soviet capture voice | EngAllAttackCommand | Fires just before consumption on successful capture |
| `DieSound=EngSovDie` | Soviet die sound | EngAllDie | Soviet-specific; also reused by YENGINEER |

All other 35 keys are byte-identical to `[ENGINEER]` — see the
[ENGINEER dossier](../allied/ENGINEER.md) for key-by-key explanation
(Strength/Armor/TechLevel/Sight/Speed/Pip/Engineer/AllowedToStartInMultiplayer/
Cost/Soylent/Points/IsSelectableCombatant/Locomotor/PhysicalSize/MovementZone/
ThreatPosed/SpecialThreatValue/ImmuneToVeins/GuardRange/Size/PreventAttackMove/
IFVMode/Trainable, plus the full weapon binding).

### Implicit defaults (same as ENGINEER)

- `Crawls=` — inherited from `[ENGINEER]` art section (via `Image=`) → `yes`.
- `Bombable=` — defaults to `false`.
- `Crushable=` — defaults `yes` (infantry).
- `ImmuneToPsionics=` — defaults `no` → Soviet Engineer can be mind-controlled.
- `Occupier=` / `Deployer=` — both default `no`.

---

## artmd.ini — no `[SENGINEER]` section

`grep "^\[SENGINEER\]" artmd.ini` → **no match**.

There is no dedicated art block for SENGINEER. The rules-side
`Image=ENGINEER` directive causes the art system to resolve the section name
to `[ENGINEER]` in artmd.ini (line 429), inheriting:

```ini
[ENGINEER] ; Allied/Soviet Engineer  (from artmd.ini:429)
Cameo=ENGNICON
AltCameo=ENGNUICO
Sequence=EngineerSequence
Crawls=yes
Remapable=yes
FireUp=2
```

This means:

- **Cameo**: `ENGNICON` — the **same cameo** as Allied Engineer (no
  Soviet-themed variant exists). YENGINEER is the only faction that gets a
  distinct cameo (`YENGICON`) via its own artmd block override.
- **AltCameo**: `ENGNUICO` — never displayed (`Trainable=no`).
- **Sequence**: `[EngineerSequence]` — the same 18-key block at
  `artmd.ini:13902` used by all three engineers. See
  [ENGINEER §EngineerSequence](../allied/ENGINEER.md#referenced-sequence---engineersequence)
  for the full frame list.
- **Crawls=yes**, **Remapable=yes**, **FireUp=2** — all inherited.

The Soviet house palette remap still applies because `Remapable=yes` — the
engineer's body colour shifts to Soviet team-colour even though the SHP is
shared with Allied. Voice bank and `Owner=` filter are the only INI-level
markers of the Soviet variant.

---

## Weapons

Identical to ENGINEER:

- **Primary** `[DefuseKit]` — `rulesmd.ini:24005`. Damage=1, ROF=20, Range=1.5,
  Warhead=BombDisarm, FireOnce=yes, FireInTransport=no, Report=DefuseKit.
  Disarms attached Crazy Ivan bombs.
- **Secondary** `[VirtualScanner]` — `rulesmd.ini:23619`. Range=5,
  NeverUse=yes, pure scan-radius extender for the AI's capture-target search.
- **Warhead** `[BombDisarm]` — `rulesmd.ini:27376`. Only key is `BombDisarm=yes`.
- **Projectile** `[InvisibleAll]` — `rulesmd.ini:25407`. Inviso=yes,
  Image=none, AA=yes, AG=yes.

See [ENGINEER §Weapons](../allied/ENGINEER.md#weapons) for the full annotated
INI blocks.

---

## Voices and sounds

`ini/soundmd.ini`:

| INI key on SENGINEER | soundmd block | Resolved samples |
|---------------------|---------------|------------------|
| `VoiceSelect=EngSovSelect` | `[EngSovSelect]` line 3760 | `$ienssea` `$iensseb` `$ienssec` (random, Volume=85) |
| `VoiceMove=EngSovMove` | `[EngSovMove]` line 3755 | `$iensmoa` `$iensmob` `$iensmoc` (random) |
| `VoiceAttack=EngSovMove` | (same as VoiceMove) | reuses move bank |
| `VoiceFeedback=EngSovFear` | `[EngSovFear]` line 3765 | `$iensfea` `$iensfeb` `$iensfec` (random, Priority=low, Volume=90) |
| `VoiceSpecialAttack=EngSovAttackCommand` | `[EngSovAttackCommand]` line 3750 | `$iensata` `$iensatb` (random — only 2 samples) |
| `VoiceEnter=EngSovMove` | (same as VoiceMove) | reuses move bank |
| `VoiceCapture=EngSovAttackCommand` | (same as VoiceSpecialAttack) | capture-complete voice |
| `DieSound=EngSovDie` | `[EngSovDie]` line 3775 | `$iensdia` `$iensdib` `$iensdic` (random, Volume=85) |
| `CrushSound=InfantrySquish` | `[InfantrySquish]` | `igensqua` |
| Weapon `DefuseKit` `Report=DefuseKit` | `[DefuseKit]` sound block | `gdefuse` (single) |

### Unreferenced soundmd block

`[EngSovPowerDown]` at `soundmd.ini:3771` (`$ienspow`) exists but is **not
referenced** by `[SENGINEER]` — there is no `PowerDown=` voice key on
infantry. This sound is invoked by a separate code path when a Soviet
structure powers down (engineer-themed VO, possibly an EVA-style cue from
TS-era plumbing). Not relevant to the unit; production leftover or used by
external code for power-state announcements.

---

## Prerequisites, owners, tech

- `Prerequisite=Barracks` — generic. For Soviet houses resolves to `NAHAND`
  (Soviet Barracks).
- `Owner=` (all 10) ∩ `¬ForbiddenHouses=` (excludes 5 Allied + 1 Yuri) →
  effective owner: **Russians, Confederation, Africans, Arabs** (all 4
  Soviet houses).
- `TechLevel=1` — buildable from match start.
- `AllowedToStartInMultiplayer=no` — never in lobby starting-unit list.
- `BuildLimit=`, `RequiredHouses=`, `AIBasePlanningSide=` — all unset.

---

## Veterancy and upgrades

- `Trainable=no` — engineer excluded from veterancy XP.
  `VeteranAbilities=` and `EliteAbilities=` are unset.
- `AltCameo=ENGNUICO` (inherited via Image=ENGINEER) is never displayed
  since promotion is impossible.

---

## Hardcoded behavior in gamemd.exe (Ghidra-verified)

**There is no SENGINEER-specific code in gamemd.exe.** All behavior is driven
by the `Engineer=yes` flag (sets `InfantryTypeClass+0xEC3`), which routes
through the same paths documented in
[ENGINEER_CAPTURE_GHIDRA_REPORT.md](../../ENGINEER_CAPTURE_GHIDRA_REPORT.md)
and summarised in [ENGINEER.md §Hardcoded behavior](../allied/ENGINEER.md#hardcoded-behavior-in-gamemdexe-ghidra-verified):

- **Capture**: `InfantryClass::Mission_Capture @ 0x005202F0` — checks `+0xEC3`,
  enemy BuildingClass target, distance < 0x80, then ChangeOwner + consume.
- **Repair**: `InfantryClass::Mission_Enter @ 0x005196A0` — friendly damaged
  BuildingClass target → Health=Strength, consume.
- **Bridge repair**: Enter CABHUT, consume, rebuild segment overlay.
- **Bomb disarm**: `DefuseKit` `Warhead=BombDisarm` triggers `BombClass`
  disarm. `BombSight=4` per-house bomb-visibility radius.
- **IFV gunner**: `IFVMode=1` → IFV Weapon2 (Medic heal-beam). Soviet players
  can't normally build an IFV (`HTK` is Allied-only) but the flag is set
  defensively — a captured IFV with a SENGINEER passenger swaps to Medic.
- **AI threat**: `ThreatPosed=0`, `SpecialThreatValue=1`, `GuardRange=9`.

### Ghidra string-search results for "SENGINEER" and "EngSov"

- `search_strings "SENGINEER"` → 1 hit at `0x0081aca0` for the longer string
  `"NeedsEngineer"` — a TS-era BuildingType flag (parsed but largely unused in
  YR), **not** a reference to `[SENGINEER]`. There is **no** standalone
  "SENGINEER" string in gamemd.exe (run 2026-05-17).
- `search_strings "EngSov"` → **0 matches** (run 2026-05-17).

Confirmed: gamemd.exe contains **no hardcoded branch** keyed off this
section's name or any of its voice keys. The engine reads the section into
the same `InfantryTypeClass` template as every other infantry, and only the
`Engineer=yes` bit matters at runtime.

The string `"SENGINEER"` only appears in:

- The rulesmd.ini section header itself (parsed by the INI loader, no
  hardcoded comparison).
- Map files that explicitly place a SENGINEER unit (e.g. Soviet campaign
  missions where engineers are pre-placed).

No vtable override, no special case.

---

## TS-legacy filter

Same as ENGINEER:

- `ImmuneToVeins=yes` — TS terrain, no veins in YR. Defensive flag.
- `Locomotor={4A582744-...}` — TS-era WalkLocomotionClass GUID, alive in YR.
- `Crawls=yes` (inherited from ENGINEER art) — TS-era prone-while-walking,
  alive in YR.
- `EngineerCaptureLevel=` (Rules+0x17F8/+0x17FC) — TS-era HP threshold,
  parsed but **unread** by the capture path. Do not implement.
- `MultiEngineer=` (Rules+0x14B4) — TS-era "multiple engineers required"
  toggle. UI checkbox only, **not read** by the capture path. Do not
  implement.
- `[EngSovPowerDown]` soundmd block — TS-era power-state VO, not referenced
  by this section. Possibly used by separate building-power code.
- `NeedsEngineer` BuildingType flag — TS-era, found in
  `search_strings`; not relevant to SENGINEER infantry behavior.

---

## Cross-references

- **Canonical dossier**: [ENGINEER](../allied/ENGINEER.md) — full key-by-key
  rules + hardcoded behavior. This doc only enumerates Soviet-specific
  deltas.
- **Counterparts**:
  - [ENGINEER](../allied/ENGINEER.md) — Allied Engineer (the canonical doc).
  - [YENGINEER](../yuri/YENGINEER.md) — Yuri Engineer.
- **Builder**: NAHAND (Soviet Barracks).
- **Capture targets**: any building with `Capturable=yes`, irrespective of
  owner-faction.
- **Repair targets**: damaged friendly buildings, including own Soviet
  structures and tech buildings.
- **Bomb defuse**: same `BombClass::Disarm` path triggered by `[BombDisarm]`
  warhead — disarms bombs placed by [IVAN](../soviet/IVAN.md). Ironic since
  Crazy Ivan is also Soviet.
- **IFV passenger**: SENGINEER **cannot board the Allied [HTK] IFV** in
  practice — IFV is Allied-only — but `IFVMode=1` is set defensively. If a
  Soviet player captures an HTK (via [TERROR](../soviet/TERROR.md)-bomb
  weapon recovery or by an engineer ironically capturing the IFV's owner's
  Service Depot), then a SENGINEER inside would swap to Medic Weapon2 per
  the IFV gunner table.

---

## Coverage audit

- ✅ Every key in `[SENGINEER]` rulesmd block (44 lines, line 4461–4504)
  covered — explicit table for the 11 keys that differ from ENGINEER, plus
  reference to canonical dossier for the 33 identical keys.
- ✅ artmd: confirmed **no `[SENGINEER]` section exists**. Art lookup
  routes via `Image=ENGINEER` to ENGINEER's art block; inherited keys
  (Cameo=ENGNICON, AltCameo, Sequence=EngineerSequence, Crawls, Remapable,
  FireUp) noted.
- ✅ Weapon chain: DefuseKit + VirtualScanner + BombDisarm + InvisibleAll —
  all identical to ENGINEER, delegated to canonical doc.
- ✅ Sound chain: 8 distinct soundmd entries enumerated. `[EngSovPowerDown]`
  flagged as unreferenced (separate building-power code path).
- ✅ Ghidra search: `search_strings "SENGINEER"` → only "NeedsEngineer"
  substring hit (TS-era BuildingType flag); `search_strings "EngSov"` → 0.
  Confirms no hardcoded section-name branch.
- ✅ TS-legacy filter applied (ImmuneToVeins, EngineerCaptureLevel,
  MultiEngineer, Locomotor GUID, unused EngSovPowerDown, NeedsEngineer).
- ✅ Cross-references to ENGINEER, YENGINEER, NAHAND, IVAN, TERROR, HTK,
  BombClass, capture/repair target categories.
