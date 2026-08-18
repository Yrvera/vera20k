# Yuri Engineer (YENGINEER)
Side: Yuri | Category: Infantry | Image alias: `Image=ENGINEER` (shares SHP/sequence)

The Yuri faction's Engineer. Mechanically identical to the Allied
[ENGINEER](../allied/ENGINEER.md) and Soviet [SENGINEER](../soviet/SENGINEER.md) —
$500 from YABRCK, captures enemy buildings, repairs damaged friendly buildings,
defuses Crazy Ivan bombs, becomes a Medic when boarding an IFV. The only
differences from the canonical Allied ENGINEER dossier are:

1. `Owner=` / `ForbiddenHouses=` — restricted to **YuriCountry** only.
2. `VoiceSelect/Move/Attack/Feedback/SpecialAttack/Enter/Capture=` — Yuri voice
   bank (`EngYuri*`).
3. `DieSound=EngSovDie` — reuses the Soviet engineer die sound (no
   YuriCountry-specific death scream).
4. `Cameo=YENGICON` / `AltCameo=YENGUICO` — different sidebar icon (Yuri-themed
   purple-tinged variant of the Allied/Soviet engineer cameo).

Everything else — INI key set, stats, weapon chain, art sequence, hardcoded
capture/repair/disarm/IFV behavior — is **bit-identical** to ENGINEER.
This is a quick-reference doc; cross-reference the canonical
[ENGINEER.md](../allied/ENGINEER.md) for the full surface.

---

## rulesmd.ini — `[YENGINEER]` section

Verbatim from `ini/rulesmd.ini:5058`:

```ini
[YENGINEER]
UIName=Name:ENGINEER
Image=ENGINEER
Name=Yuri Engineer
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
ForbiddenHouses=British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs
AllowedToStartInMultiplayer=no
Cost=500
Soylent=250
Points=5
IsSelectableCombatant=no
VoiceSelect=EngYuriSelect
VoiceMove=EngYuriMove
VoiceAttack=EngYuriCapture
VoiceFeedback=EngYuriFear
VoiceSpecialAttack=EngYuriCapture
VoiceEnter=EngYuriCapture
VoiceCapture=EngYuriCapture
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

| Key | YENGINEER value | ENGINEER value | Notes |
|-----|------|------|-------|
| `UIName=Name:ENGINEER` | Same CSF key | Same | Both resolve to "Engineer" — Yuri faction does not get a distinct in-game name; tooltip says "Engineer" |
| `Image=ENGINEER` | Redirect to ENGINEER SHP | (no Image=) | Yuri Engineer uses the **same SHP** as Allied Engineer — no Yuri-specific sprite. Voice + cameo are the only player-visible distinctions |
| `Name=Yuri Engineer` | Internal short name | "Engineer" | Internal only — `UIName=` overrides the display |
| `Owner=` | Lists all 10 houses | Lists all 10 houses | Identical full list — filter happens via ForbiddenHouses |
| `ForbiddenHouses=` | `British,French,Germans,Americans,Alliance,Russians,Confederation,Africans,Arabs` (all 5 Allied + all 4 Soviet) | Excludes Soviet+Yuri | Net Owner: **YuriCountry only** |
| `VoiceSelect=EngYuriSelect` | Yuri voice bank | EngAllSelect | |
| `VoiceMove=EngYuriMove` | Yuri voice bank | EngAllMove | |
| `VoiceAttack=EngYuriCapture` | Yuri capture voice — note **VoiceAttack reuses VoiceCapture**, not VoiceMove | EngAllMove | Yuri engineer's "attack" command voice is the same as its capture voice; chatter is more aggressive than the Allied counterpart's neutral move-voice reuse |
| `VoiceFeedback=EngYuriFear` | Yuri fear voice | EngAllFear | |
| `VoiceSpecialAttack=EngYuriCapture` | Yuri capture voice | EngAllAttackCommand | |
| `VoiceEnter=EngYuriCapture` | Yuri capture voice — reuses capture for enter-transport too | EngAllMove | Slightly off-tone vs Allied (capture-themed vocalisation when boarding an IFV) |
| `VoiceCapture=EngYuriCapture` | Yuri capture voice | EngAllAttackCommand | Plays just before engineer is consumed on capture |
| `DieSound=EngSovDie` | **Soviet** die sound | EngAllDie | YuriCountry has no dedicated EngYuriDie binding — though `[EngYuriDie]` exists in soundmd (line 4489), `DieSound=` here uses `EngSovDie`. Either is a Westwood data choice or oversight; the doc reports what the INI actually sets |

All other 35 keys are byte-identical to `[ENGINEER]` — see the
[ENGINEER dossier](../allied/ENGINEER.md) for key-by-key explanation
(Strength/Armor/TechLevel/Sight/Speed/Pip/Engineer/AllowedToStartInMultiplayer/
Cost/Soylent/Points/IsSelectableCombatant/Locomotor/PhysicalSize/MovementZone/
ThreatPosed/SpecialThreatValue/ImmuneToVeins/GuardRange/Size/PreventAttackMove/
IFVMode/Trainable etc.).

### Implicit defaults (same as ENGINEER)

- `Crawls=` — set in artmd section to `yes`.
- `Bombable=` — defaults to `false` (no explicit override).
- `Crushable=` — defaults `yes` (infantry).
- `ImmuneToPsionics=` — defaults `no` → Yuri Engineer can be mind-controlled by
  enemy Yuri units (yes, even by another YuriCountry player).
- `Occupier=` / `Deployer=` — both default `no`.

---

## artmd.ini — `[YENGINEER]` section

`ini/artmd.ini:437`:

```ini
[YENGINEER] ; Yuri Engineer
Image=ENGINEER
Cameo=YENGICON
AltCameo=YENGUICO
Sequence=EngineerSequence
Crawls=yes
Remapable=yes
FireUp=2
```

| Key | Meaning |
|-----|---------|
| `Image=ENGINEER` | **Art-side override** redirecting frame source to `ENGINEER.SHP`. Rules-side already has `Image=ENGINEER`, so this duplicates and ensures the art lookup resolves to the same SHP file |
| `Cameo=YENGICON` | Sidebar icon — **YENGICON.SHP**, the Yuri-themed engineer cameo. Single-frame remappable cameo |
| `AltCameo=YENGUICO` | Elite cameo — **never displayed** because `Trainable=no` on YENGINEER |
| `Sequence=EngineerSequence` | Shared `[EngineerSequence]` block — same 18-key sequence as ENGINEER (Ready/Guard/Prone/Walk/FireUp/Down/Crawl/Up/FireProne/Idle1/Idle2/Die1..5/Paradrop/Cheer/Panic). See [ENGINEER.md §artmd](../allied/ENGINEER.md#artmdini--engineer-section) |
| `Crawls=yes` | InfantryTypeClass+0xEBD — prone-while-walking enabled |
| `Remapable=yes` | YuriCountry house palette remap applied to engineer body |
| `FireUp=2` | Bullet-spawn frame within DefuseKit firing — frame 2 of the FireUp sequence |

`PrimaryFireFLH=` / `SecondaryFireFLH=` are absent — same as ENGINEER, the
DefuseKit weapon uses `Projectile=InvisibleAll` so no visible muzzle flash
is needed.

---

## Weapons

Identical to ENGINEER:

- **Primary** `[DefuseKit]` — `rulesmd.ini:24005`. Damage=1, ROF=20, Range=1.5,
  Warhead=BombDisarm, FireOnce=yes, FireInTransport=no, Report=DefuseKit
  (`gdefuse`). Disarms attached Crazy Ivan bombs.
- **Secondary** `[VirtualScanner]` — `rulesmd.ini:23619`. Range=5, NeverUse=yes,
  pure scan-radius extender for the AI's capture-target search loop.
- **Warhead** `[BombDisarm]` — `rulesmd.ini:27376`. Only key is `BombDisarm=yes`.
- **Projectile** `[InvisibleAll]` — `rulesmd.ini:25407`. Inviso=yes,
  Image=none, AA=yes, AG=yes.

See [ENGINEER §Weapons](../allied/ENGINEER.md#weapons) for the full annotated
INI blocks.

---

## Voices and sounds

`ini/soundmd.ini`:

| INI key on YENGINEER | soundmd block | Resolved samples |
|---------------------|---------------|------------------|
| `VoiceSelect=EngYuriSelect` | `[EngYuriSelect]` line 4469 | `$ienysea` `$ienyseb` `$ienysec` `$ienysed` `$ienysee` (random, Volume=85) |
| `VoiceMove=EngYuriMove` | `[EngYuriMove]` line 4474 | `$ienymoa` `$ienymob` `$ienymoc` `$ienymod` `$ienymoe` (random) |
| `VoiceAttack=EngYuriCapture` | `[EngYuriCapture]` line 4479 | `$ienyata` `$ienyatb` `$ienyatc` `$ienyatd` `$ienyate` (random) |
| `VoiceFeedback=EngYuriFear` | `[EngYuriFear]` line 4484 | `$ienyfea` `$ienyfeb` `$ienyfec` `$ienyfed` `$ienyfee` (random) |
| `VoiceSpecialAttack=EngYuriCapture` | (same as VoiceAttack) | re-uses capture bank |
| `VoiceEnter=EngYuriCapture` | (same as VoiceAttack) | re-uses capture bank |
| `VoiceCapture=EngYuriCapture` | (same as VoiceAttack) | re-uses capture bank |
| `DieSound=EngSovDie` | `[EngSovDie]` line 3775 | `$iensdia` `$iensdib` `$iensdic` (random) — **Soviet** die sound |
| `CrushSound=InfantrySquish` | `[InfantrySquish]` | `igensqua` |
| Weapon `DefuseKit` `Report=DefuseKit` | `[DefuseKit]` sound block | `gdefuse` (single) |

`[EngYuriDie]` exists at soundmd line 4489 (`$ienydia` `$ienydib` `$ienydic`
`$ienydid` `$ienydie`) but is **not referenced** by any TechnoType because
YENGINEER uses `DieSound=EngSovDie`. This is unused content — likely a
Westwood production-stage placeholder.

---

## Prerequisites, owners, tech

- `Prerequisite=Barracks` — generic key. For YuriCountry, resolves to
  `YABRCK` (Yuri Barracks).
- `Owner=` (all 10) ∩ `¬ForbiddenHouses=` (everything except YuriCountry)
  → effective owner: **YuriCountry only**. Only Yuri faction can build this
  unit.
- `TechLevel=1` — buildable from match start.
- `AllowedToStartInMultiplayer=no` — never in lobby starting-unit list.
- `BuildLimit=`, `RequiredHouses=`, `AIBasePlanningSide=` — all unset.

---

## Veterancy and upgrades

- `Trainable=no` — engineer is excluded from veterancy XP. `VeteranAbilities=`
  and `EliteAbilities=` are unset; defaults are unreachable.
- `AltCameo=YENGUICO` is referenced but **never displayed** since promotion is
  impossible.
- No weapon swap, no cameo swap, no health bonus paths.

---

## Hardcoded behavior in gamemd.exe (Ghidra-verified)

**There is no YENGINEER-specific code in gamemd.exe.** All behavior is driven by
the `Engineer=yes` flag (sets `InfantryTypeClass+0xEC3`), which routes through
the same paths documented in
[ENGINEER_CAPTURE_GHIDRA_REPORT.md](../../ENGINEER_CAPTURE_GHIDRA_REPORT.md)
and summarised in [ENGINEER.md §Hardcoded behavior](../allied/ENGINEER.md#hardcoded-behavior-in-gamemdexe-ghidra-verified):

- **Capture**: `InfantryClass::Mission_Capture @ 0x005202F0` — checks `+0xEC3`,
  enemy BuildingClass target, distance < 0x80, then ChangeOwner + consume.
- **Repair**: `InfantryClass::Mission_Enter @ 0x005196A0` — friendly damaged
  BuildingClass target → Health=Strength, consume.
- **Bridge repair**: enter CABHUT, consume, rebuild segment overlay.
- **Bomb disarm**: `DefuseKit` `Warhead=BombDisarm` triggers `BombClass` disarm.
  `BombSight=4` per-house bomb-visibility radius.
- **IFV gunner**: `IFVMode=1` → IFV Weapon2 (Medic heal-beam).
- **AI threat**: `ThreatPosed=0`, `SpecialThreatValue=1`, `GuardRange=9`.

### Ghidra string-search results for "YENGINEER" and "EngYuri"

- `search_strings "YENGINEER"` → **0 matches** (run 2026-05-17).
- `search_strings "EngYuri"` → **0 matches** (run 2026-05-17).

Confirmed: gamemd.exe contains **no hardcoded branch** keyed off this
section's name or any of its voice keys. The engine reads the section into
the same `InfantryTypeClass` template as every other infantry, and only the
`Engineer=yes` bit matters at runtime.

The string `"YENGINEER"` only appears in:

- The rulesmd.ini section header itself (parsed by the INI loader, no
  hardcoded comparison).
- Map files that explicitly place a YENGINEER unit.

No vtable override, no special case.

### Cross-faction interaction note — Yuri Engineer captures Yuri buildings

Because Yuri-vs-Yuri matches are possible and the capture path checks
`!IsAlliedWith(target)`, a YuriCountry player's YENGINEER will capture another
YuriCountry player's enemy buildings just fine. There is no faction-name
filter at the capture-mission level — only the alliance bit.

---

## TS-legacy filter

Same as ENGINEER:

- `ImmuneToVeins=yes` — TS terrain, no veins in YR. Defensive flag.
- `Locomotor={4A582744-...}` — TS-era WalkLocomotionClass GUID, alive in YR.
- `Crawls=yes` (art) — TS-era prone-while-walking, alive in YR.
- `EngineerCaptureLevel=` (Rules+0x17F8/+0x17FC) — TS-era HP threshold, parsed
  but **unread** by the capture path. Do not implement.
- `MultiEngineer=` (Rules+0x14B4) — TS-era "multiple engineers required"
  toggle. UI checkbox only, **not read** by the capture path. Do not
  implement.
- `[EngYuriDie]` soundmd block exists but is unreferenced (production
  leftover).

---

## Cross-references

- **Canonical dossier**: [ENGINEER](../allied/ENGINEER.md) — full key-by-key
  rules + hardcoded behavior. This doc only enumerates Yuri-specific deltas.
- **Counterparts**:
  - [ENGINEER](../allied/ENGINEER.md) — Allied Engineer (the canonical doc).
  - [SENGINEER](../soviet/SENGINEER.md) — Soviet Engineer.
- **Builder**: YABRCK (Yuri Barracks) — Yuri-side dossier pending.
- **Capture targets**: any building with `Capturable=yes`, irrespective of
  owner-faction (Allied, Soviet, Yuri, Neutral).
- **Repair targets**: damaged friendly buildings, including own YuriCountry
  structures and tech buildings.
- **Bomb defuse**: same `BombClass::Disarm` path triggered by `[BombDisarm]`
  warhead — disarms bombs placed by IVAN.
- **IFV passenger**: YENGINEER **cannot board the Allied [HTK] IFV** in
  practice — IFV is Allied-only — but `IFVMode=1` is set defensively. If a
  Yuri player captures an HTK, then a YENGINEER inside would swap to Medic
  Weapon2 (heal-beam) per the IFV gunner table.

---

## Coverage audit

- ✅ Every key in `[YENGINEER]` rulesmd block (42 lines, line 5058–5101)
  covered — explicit table for the 11 keys that differ from ENGINEER, plus
  reference to canonical dossier for the 31 identical keys.
- ✅ Every key in `[YENGINEER]` artmd block (7 lines, line 437–444) covered.
  `Image=ENGINEER` art-side override noted; shared `[EngineerSequence]`
  cross-referenced.
- ✅ Weapon chain: DefuseKit + VirtualScanner + BombDisarm + InvisibleAll —
  all identical to ENGINEER, delegated to canonical doc.
- ✅ Sound chain: 9 distinct soundmd entries enumerated. `[EngYuriDie]`
  flagged as unreferenced production leftover.
- ✅ Ghidra search: `search_strings "YENGINEER"` → 0 hits; `search_strings
  "EngYuri"` → 0 hits. Confirms no hardcoded section-name branch.
- ✅ TS-legacy filter applied (ImmuneToVeins, EngineerCaptureLevel,
  MultiEngineer, Locomotor GUID, unused EngYuriDie).
- ✅ Cross-references to ENGINEER, SENGINEER, YABRCK, HTK, BombClass, IVAN,
  capture/repair target categories.
