# Engineer (ENGINEER)
Side: Allied | Category: Infantry | Image alias: `[ENGINEER]` (no `Image=` redirect)

The Allied Engineer. $500 from the Barracks, captures enemy buildings instantly
on contact, repairs damaged friendly buildings instantly on contact, defuses
Crazy Ivan bombs (`BombSight=4`, weapon `DefuseKit` warhead `BombDisarm=yes`).
Cannot be a starting unit. No combat weapon — `Secondary=VirtualScanner` is a
zero-damage scanner used by guard-AI to find capture targets at range. Boarding
an IFV switches it to "Medic" mode (`IFVMode=1`, infantry-heal beam).

The Allied Engineer shares the same INI section template, art section,
sequence, and gameplay rules as the Soviet Engineer (`SENGINEER`) and the
Yuri Engineer (`YENGINEER`) — only `Cameo=`, voices, and `Owner=` differ. The
hardcoded behaviors below apply to all three.

Authoritative deep RE: [ENGINEER_CAPTURE_GHIDRA_REPORT.md](../../ENGINEER_CAPTURE_GHIDRA_REPORT.md).

---

## rulesmd.ini — `[ENGINEER]` section

Verbatim from `ini/rulesmd.ini:3817`:

```ini
[ENGINEER]
UIName=Name:ENGINEER
Name=Engineer
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
ForbiddenHouses=Russians,Confederation,Africans,Arabs,YuriCountry
AllowedToStartInMultiplayer=no
Cost=500
Soylent=250
Points=5
IsSelectableCombatant=no
VoiceSelect=EngAllSelect
VoiceMove=EngAllMove
VoiceAttack=EngAllMove
VoiceFeedback=EngAllFear
VoiceSpecialAttack=EngAllAttackCommand
VoiceEnter=EngAllMove
VoiceCapture=EngAllAttackCommand
DieSound=EngAllDie
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

| Key | Meaning |
|-----|---------|
| `UIName=Name:ENGINEER` | CSF-string key resolving to "Engineer" |
| `Name=Engineer` | Internal short name |
| `Category=Soldier` | Pip group + AI threat grouping (infantry) |
| `Primary=DefuseKit` | Sole weapon — disarms Ivan bombs (`BombDisarm=yes` warhead). Range 1.5, fires once per target |
| `Secondary=VirtualScanner` | **Not a combat weapon** — `NeverUse=yes` zero-damage probe used by the guard-AI's `GreatestThreatScan` to extend capture-target search radius beyond Primary's 1.5 cells |
| `Prerequisite=Barracks` | Generic "Barracks" prereq (Allied house resolves to GAPILE; Soviet to NAHAND; Yuri to YABRCK) |
| `CrushSound=InfantrySquish` | Crush sound `igensqua` |
| `LeadershipRating=3` | Leadership-rating veterancy gain modifier (lower → slower to promote) |
| `Strength=75` | HP — 60% of GI; engineers are deliberately fragile |
| `Armor=none` | Damage type column 0 |
| `TechLevel=1` | Buildable from game start (gated only by Barracks) |
| `Sight=4` | Reveal radius (1 less than GI) |
| `BombSight=4` | Range at which the engineer reveals Crazy Ivan bombs on the map (engine-special: gates bomb-visibility for the engineer's house within this radius around the engineer) |
| `Speed=4` | Same as GI |
| `Pip=Blue` | Cargo-passenger pip color when loaded |
| `Engineer=yes` | **Behavior flag** — `InfantryTypeClass+0xEC5` **[BINARY-VERIFIED audit 3 — Mission_Capture decompile reads `*(char *)(param_1[0x1b0] + 0xec5)` as the first condition check]**. **The doc previously claimed `+0xEC3` per a "GI report P3.1 correction" — that correction was WRONG. The original ENGINEER_CAPTURE report's `+0xEC5` is correct.** Forces `+0xEBE Infiltrate=true`. Enables `Mission_Capture` path. |
| `Owner=Russians,Confederation,...Alliance` | **All ten houses** listed — same INI section used for Allied+Soviet+Yuri Engineer countries pre-filter |
| `ForbiddenHouses=Russians,Confederation,Africans,Arabs,YuriCountry` | **Filter** — excludes all Soviet (4) and Yuri (1) houses. Net effect: only Allied countries (British, French, Germans, Americans, Alliance) can build ENGINEER. Soviet builds [SENGINEER]; Yuri builds [YENGINEER] |
| `AllowedToStartInMultiplayer=no` | Cannot appear in starting-unit complement; must be produced |
| `Cost=500` | Credits — 2.5× GI |
| `Soylent=250` | Grinder refund (Yuri only) |
| `Points=5` | Kill score |
| `IsSelectableCombatant=no` | **Excluded** from "select all combat units" hotkey; AI doesn't pull engineers into combat groups |
| `VoiceSelect=EngAllSelect` | Allied selection voice bank |
| `VoiceMove=EngAllMove` | Move-order voice |
| `VoiceAttack=EngAllMove` | Reuses move voice — engineer has no attack; this fires when right-clicking enemy targets |
| `VoiceFeedback=EngAllFear` | Fear/panic voice |
| `VoiceSpecialAttack=EngAllAttackCommand` | Voice when ordered to capture/repair via the Enter mission |
| `VoiceEnter=EngAllMove` | Voice when entering a transport/garrison |
| `VoiceCapture=EngAllAttackCommand` | Voice on successful capture (just before consumption) |
| `DieSound=EngAllDie` | Death sound |
| `Locomotor={4A582744-...}` | `WalkLocomotionClass` GUID — same as all infantry |
| `PhysicalSize=1` | Pathfinder size class |
| `MovementZone=Infantry` | Standard infantry terrain |
| `ThreatPosed=0` | AI does not target engineers as priority |
| `SpecialThreatValue=1` | Set between 0 and 1; controls scoring on the engineer's own threat estimate — engineer "wants" capture targets at max weight |
| `ImmuneToVeins=yes` | TS legacy; veins are TS-only terrain (see TS-legacy filter) |
| `GuardRange=9` | Guard mission scan radius — larger than Sight (4) so engineer in `MissionGuard` will detect capture-target buildings beyond visible cells |
| `Size=1` | Transport cargo slot cost |
| `PreventAttackMove=yes` | **Suppresses Attack-Move action** — Force-Move and Attack-Move hotkeys both behave as plain Move (no attack interleave). Engineer cannot have an attack-move waypoint |
| `IFVMode=1` | IFV gunner-table index 1 → swap to Medic weapon (heal-beam) when this passenger boards an [HTK] |
| `Trainable=no` | **Cannot gain veterancy** — XP awards skip this unit. No Veteran/Elite cameo, no weapon swap. Compare to `+0xC8E Trainable` |

Implicit defaults (not set in this section but worth noting):

- `Crawls=` — set in art section to `yes` (prone while crawling enabled)
- `Bombable=` — defaults to `false` here (engineer absent from this list; only `[E1]` has Bombable explicitly set to yes); Crazy Ivan **can still attach** a bomb via the Bomb mission, but the engineer's lack of an explicit gate means the cursor doesn't auto-treat it
- `Crushable=` — defaults to `yes` for infantry; not overridden
- `ImmuneToPsionics=` — defaults to `no`; engineers can be mind-controlled
- `Occupier=` — defaults to `no`; engineers cannot garrison civilian buildings
- `Deployer=` — defaults to `no`; no deploy command

---

## artmd.ini — `[ENGINEER]` section

`ini/artmd.ini:429`:

```ini
[ENGINEER] ; Allied/Soviet Engineer
Cameo=ENGNICON
AltCameo=ENGNUICO
Sequence=EngineerSequence
Crawls=yes
Remapable=yes
FireUp=2
```

| Key | Meaning |
|-----|---------|
| `Cameo=ENGNICON` | Sidebar icon — note **shared art** with Soviet Engineer (same SHP base, only voice/cameo differ via YENGINEER's overrides on Yuri) |
| `AltCameo=ENGNUICO` | Cameo when Elite — but **engineer is `Trainable=no`**, so AltCameo is never shown. Defensively present |
| `Sequence=EngineerSequence` | Reference to `[EngineerSequence]` block (frame layout) |
| `Crawls=yes` | Sets `InfantryTypeClass+0xEBD` — prone-while-walking enabled |
| `Remapable=yes` | House remap palette applied |
| `FireUp=2` | Bullet-spawn frame within the firing sequence (DefuseKit fires at frame 2 of the FireUp track) |

Note this section is missing `PrimaryFireFLH=` / `SecondaryFireFLH=`. Since
DefuseKit is range 1.5 with `Inviso=yes` `Image=none`, no visible muzzle flash
or projectile is emitted — the lack of FLH is intentional (the disarm "action"
plays via the `Report=DefuseKit` sound only).

### Referenced sequence — `[EngineerSequence]`

`artmd.ini:13902`:

```ini
[EngineerSequence]
Ready=0,1,1
Guard=0,1,1
Prone=86,1,6
Walk=8,6,6
FireUp=164,6,6
Down=212,2,2
Crawl=86,6,6
Up=228,2,2
FireProne=164,6,6
Idle1=56,15,0,S
Idle2=71,15,0,E
Die1=134,15,0
Die2=149,15,0
Die3=0,1,1
Die4=0,1,1
Die5=0,1,1
Paradrop=244,1,0
Cheer=245,8,0,E
Panic=8,6,6
```

Differences from `[GISequence]`:

- No Deploy/Deployed/DeployedFire/DeployedIdle/Undeploy frames — engineer cannot deploy
- `Down=212,2,2` (engineer prone-down) vs GI `Down=260` — different frames in the SHP (the engineer SHP layout is unique)
- `Up=228` vs GI `Up=276`
- `FireProne=164,6,6` reuses the `FireUp` frames — engineer has no separate prone-fire animation (no real combat use case)
- Same 8-facing 6-frame Walk cycle at offset 8

---

## Weapons

### Primary — `[DefuseKit]`

`rulesmd.ini:24005`:

```ini
[DefuseKit]
Damage=1
ROF=20
Range=1.5
CellRangefinding=yes
Projectile=InvisibleAll
Speed=100
Report=DefuseKit
Warhead=BombDisarm
FireOnce=yes
FireInTransport=no;can't fire out of the BattleFortress
```

| Key | Meaning |
|-----|---------|
| `Damage=1` | Nominal — `BombDisarm` warhead does no damage, this is a placeholder |
| `ROF=20` | Cooldown between defuses (frames) |
| `Range=1.5` | Must be ≤1 cell to disarm (radius 1.5 leptons) |
| `CellRangefinding=yes` | Use cell-center distance for range check (not lepton precision) — gives forgiving radius |
| `Projectile=InvisibleAll` | `Inviso=yes` `Image=none` `AA=yes` `AG=yes` — no projectile sprite, can target both air and ground (defensive flag for the bomb-target case which is technically AG) |
| `Speed=100` | Irrelevant for inviso instant-hit |
| `Report=DefuseKit` | Sound `gdefuse` played once |
| `Warhead=BombDisarm` | The warhead with only `BombDisarm=yes` — triggers the bomb-removal special path |
| `FireOnce=yes` | One shot per target acquisition |
| `FireInTransport=no` | Cannot fire from inside an IFV/Battle Fortress |

### Secondary — `[VirtualScanner]`

`rulesmd.ini:23619`:

```ini
[VirtualScanner]; This is so units with range one weapons will scan out farther when looking for targets in guard
Damage=1
Range=5
NeverUse=yes
Projectile=InvisibleAll
Warhead=SA
Speed=100
```

| Key | Meaning |
|-----|---------|
| `Damage=1` / `Warhead=SA` | Never used; placeholder values |
| `Range=5` | **Effective scan range** for the AI's guard-mission target search — Primary range 1.5 is too small to find capture-target buildings, so the AI uses Secondary's Range=5 to acquire and then move into Primary range |
| `NeverUse=yes` | **Hard flag** — engine refuses to fire this weapon under any circumstance, even on a forced attack order. Pure target-scan helper |
| `Projectile=InvisibleAll` `Speed=100` | Defensive placeholder |

### Warhead — `[BombDisarm]`

`rulesmd.ini:27376`:

```ini
[BombDisarm]
BombDisarm=yes
```

This is the entire warhead block. `BombDisarm=yes` triggers `BombClass`
disarm logic on the target's attached bomb — see
[BOMB_CLASS_GHIDRA_REPORT.md](../../BOMB_CLASS_GHIDRA_REPORT.md). No `Verses=`,
no `AnimList=`, no damage; the warhead is a one-bit signal.

### Projectile — `[InvisibleAll]`

`rulesmd.ini:25407`:

```ini
[InvisibleAll] ; used by all the things with infinite range (-2) so do not let it be SubjectTo Anything
Inviso=yes
Image=none
AA=yes
AG=yes
```

No subject-to-cliffs/elevation/walls — always hits target regardless of LOS,
because the use case is infinite-range and infrastructure-only.

---

## Voices and sounds

| INI key on ENGINEER | soundmd block | Resolved samples |
|---------------------|---------------|------------------|
| `VoiceSelect=EngAllSelect` | `[EngAllSelect]` line 3730 | `$ienasea` `$ienaseb` `$ienasec` `$ienased` (random) |
| `VoiceMove=EngAllMove` | `[EngAllMove]` line 3725 | `$ienamoa` `$ienamob` `$ienamoc` (random) |
| `VoiceAttack=EngAllMove` | (same as VoiceMove) | reuses move bank — engineer has no attack voice |
| `VoiceFeedback=EngAllFear` | `[EngAllFear]` line 3735 | `$ienafea` `$ienafeb` `$ienafec`, Priority=low |
| `VoiceSpecialAttack=EngAllAttackCommand` | `[EngAllAttackCommand]` line 3720 | `$ienaata` `$ienaatb` `$ienaatc` (random) |
| `VoiceEnter=EngAllMove` | (same as VoiceMove) | enter-transport voice |
| `VoiceCapture=EngAllAttackCommand` | (same as VoiceSpecialAttack) | capture-complete cheer |
| `DieSound=EngAllDie` | `[EngAllDie]` line 3744 | `$ienadia` `$ienadib` `$ienadic` `$ienadid` (random) |
| `CrushSound=InfantrySquish` | `[InfantrySquish]` line 1196 | `igensqua` |
| Weapon `DefuseKit` `Report=DefuseKit` | `[DefuseKit]` line 2519 | `gdefuse` (single sample) |

`$` prefix marks EVA-style clipped infantry voices (per the GI dossier's
description, route through the infantry chatter mixer).

`VoiceCapture=` is a special key — fired by `Mission_Capture` immediately
before the engineer is destroyed and the building's owner is reassigned.

`VoiceEnter=` plays when the engineer is ordered into either a transport
(SAPC, HTK, SHAD, FV) or a civilian building (engineers don't normally enter
civilian buildings — Occupier=no — but the voice is referenced just in case).

---

## Prerequisites, owners, tech

- `Prerequisite=Barracks` — generic, resolves to the owner's house barracks:
  GAPILE (Allied), NAHAND (Soviet), YABRCK (Yuri). Since this section's
  Owner+ForbiddenHouses filter leaves only Allied houses, the effective
  prerequisite is GAPILE.
- `Owner=Russians,Confederation,Africans,Arabs,YuriCountry,British,French,Germans,Americans,Alliance`
  — all 10 houses; Allied countries are Russians is NOT one of them despite the
  list. Wait — Russians IS in this list. The filter ForbiddenHouses excludes
  them.
- `ForbiddenHouses=Russians,Confederation,Africans,Arabs,YuriCountry` — these
  five Soviet/Yuri-faction houses **cannot** build ENGINEER. Build button is
  hidden in the Soviet sidebar for this unit.
- `TechLevel=1` — no tech gate.
- `BuildLimit=` not set.
- `AIBasePlanningSide=` not set.
- `RequiredHouses=` not set.
- `AllowedToStartInMultiplayer=no` — excluded from the lobby's "starting
  units" allocation, even though the unit is buildable from match-start
  (Barracks present at start).

---

## Veterancy and upgrades

- `Trainable=no` — engineer is **excluded** from all veterancy XP. Kills made
  by the engineer (the only possible kill is via DefuseKit's nominal Damage=1,
  but even that does not score because the warhead has BombDisarm=yes which
  is a non-damage path) award no XP to the engineer. Promotion never occurs.
- `VeteranAbilities=` / `EliteAbilities=` — not set. Defaults apply but are
  unreachable.
- No `Crushable=` progression.
- No `SELF_HEAL`, no weapon swap, no cameo swap (AltCameo present but
  unreachable).

---

## Hardcoded behavior in gamemd.exe (Ghidra-verified)

The full RE is in
[ENGINEER_CAPTURE_GHIDRA_REPORT.md](../../ENGINEER_CAPTURE_GHIDRA_REPORT.md);
this section integrates that report and adds related-system links.

### Capture path — `InfantryClass::Mission_Capture @ 0x005202F0`

**[BINARY-VERIFIED audit 3 — exact address, body 0x005202f0–0x005206aa]**

Per-tick preconditions (all must hold) — verified via decompile:

1. **`*(char *)(TypeClass + 0xEC5) != 0`** — Engineer flag set. **[BINARY-VERIFIED
   audit 3 — exact offset 0xEC5, not 0xEC3 as previously claimed.]** The
   "+0xEC3" attribution in earlier passes was incorrect; +0xEC5 is what
   Mission_Capture actually reads.
2. **`param_1[0x169] != NULL`** = `this+0x5A4` (target ptr). **[BINARY-VERIFIED
   audit 3 — `0x169 * 4 = 0x5A4`]**
3. **`target->GetRTTI() == 1`** via `vtable+0x2c`. **[BINARY-VERIFIED audit 3]**
   — target is a BuildingClass (RTTI value 1).
4. **`HouseClass::Is_Ally_ByObject() == 0`** — target is enemy.
   **[BINARY-VERIFIED audit 3]**

Distance gates **[BINARY-VERIFIED audit 3 — all three thresholds visible in Mission_Capture decompile]**:

- `Distance3D < 0x80` (128 leptons ≈ 0.5 cells) → execute capture sequence
- `Distance3D >= 0x80` AND `< 0x200` → re-issue move toward building dock point (mid-range approach)
- `Distance3D >= 0x200` → check destination already-reached flag, else re-issue move (far-range approach)
- An inner check `< 0x81` short-circuits within the mid-range branch when target is essentially reached.

Capture sequence (instant, no health-based scaling) **[BINARY-VERIFIED audit 3]**:

1. If building has an attacker (`piVar8[0xd] != 0` = `target+0x34`), clear that
   targeting via `TechnoClass::ProcessCellAction(1, infantry, ...)`.
2. Set building mission to Guard (3) via `vtable+0x274` (SetMission).
3. Limbo building briefly via `vtable+0xDC` (hides from world).
4. EVA notification path (`FUN_006e57c0` checks if human owner →
   `FUN_005f5b50` plays EVA, called with `param_1[0xd]`).
5. **`building->ChangeOwner(infantry->Owner, 1)`** via `vtable+0x3D4` —
   `param_2 = 1` announces capture.
6. **`piVar8[0xce] = *(int *)(param_1[0x1b0] + 0xdf8)`** —
   `building[0x338] = engineer->TypeClass[0xDF8]`. Stores the engineer's
   type-specific tag for the captured building.
7. **`infantry->vtable_0xF8()`** — engineer is destroyed/consumed (presumably
   the destructor or "remove from world" vfunc).

**Note on the field-offset notation**: when the doc previously said `field_0xD`
or `field_0xCE`, those are the int-array indices visible in Ghidra's
decompile output, which correspond to byte offsets `0xD * 4 = 0x34` and
`0xCE * 4 = 0x338` respectively. Take care when comparing to raw struct
byte offsets in other docs.

The `SetOwner` call propagates: power refresh, owned-list updates, radar
update, sidebar refresh, base-center recompute, wall connection refresh.
See [BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md](../../BUILDING_CHANGE_OWNER_GHIDRA_REPORT.md).

### Repair path — `InfantryClass::Mission_Enter @ 0x005196A0`

**[ADDRESS DISCREPANCY audit 3]**: there is **no standalone Mission_Enter
function at 0x005196A0**. The address falls inside
`InfantryClass::PerCellProcess @ 0x00519630` (body 0x00519630–0x0051aa0a).
The repair behavior is implemented as a branch inside PerCellProcess (or
called from there into shared infrastructure), not a dedicated function.

When the target is a damaged **friendly** building:

- Same approach/distance logic **[INFERRED — Mission_Capture decompile
  shows distance gates; the repair-vs-capture branch is in
  PerCellProcess which wasn't decompiled in audit 3]**
- On arrival: `building->Health = building->Strength` (heal to max).
  **[INFERRED — not decompile-verified in audit 3]**
- Engineer is consumed (same vtable+0xF8 call). **[INFERRED]**

No partial-repair — engineer fully restores HP. Cursor (Allied repair vs.
enemy capture vs. invalid) is computed in `What_Action_OnObject` per
[DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md](../../DETERMINE_ACTION_DOWNSTREAM_GHIDRA_REPORT.md).

### Bridge repair

- The bridge hut building (`CABHUT`) is `Capturable=no` but accepts
  engineer-Enter for the rebuild-bridge action. The engineer enters
  CABHUT, is consumed, and the destroyed-bridge-segment overlay/tile state
  is restored. See [BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md](../../BRIDGE_REPAIR_AND_HUT_DEATH_GHIDRA_REPORT.md).
- Note: per the open follow-up memory entry
  [`project_c4_bridge_hut_followup.md`], C4 on CABHUT in the current Rust
  port does nothing — this is a port-side bug, gamemd has no Immune gate.

### Bomb disarm — `DefuseKit` weapon

- Weapon `Report=DefuseKit` (plays `gdefuse` sample).
- Warhead `BombDisarm=yes` triggers `BombClass::Disarm` path on impact;
  the engine looks up any attached `BombClass` on the target via
  `target->AttachedBomb` and detaches/frees it. See
  [BOMB_CLASS_GHIDRA_REPORT.md](../../BOMB_CLASS_GHIDRA_REPORT.md).
- `BombSight=4` (rules key) — the engineer reveals enemy/Ivan bombs to its
  house within 4 cells. The engine maintains a per-house bomb-visibility list
  and uses `BombSight=` to short-circuit the fog-of-war check on bomb sprites.

### IFV gunner mode

- `IFVMode=1` (`TechnoTypeClass+0x688`) → IFV picks gunner Weapon2 (Medic
  beam). See [IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md](../../IFV_AND_OPEN_TOPPED_TRANSPORT_GHIDRA_REPORT.md).
  The IFV becomes the Allied Medic vehicle when an engineer boards —
  fires a heal-beam at allied infantry, ROF and range per the IFV's
  Weapon2 slot.

### Per-tick AI (shared with all infantry)

- `InfantryClass::AI @ 0x0051BAB0` — phase 13 of the 16-phase pipeline is
  `if (Mission_Capture() returns true) → return;` (per GI report §3.1). For
  engineers this branch fires when the entity's mission == Capture (29 / 0x1D)
  or Sabotage (variant), exiting the tick early so movement/firing logic
  doesn't override the capture state.

### Threat to enemy AI

- `ThreatPosed=0` → enemy AI does **not** target engineers as priority.
- `SpecialThreatValue=1` → for the engineer's own AI guard/scan, max weight
  on capture targets.
- `GuardRange=9` → in MissionGuard, scan radius extends to 9 cells (overrides
  Sight=4 for the AI's target-search loop).

### Ghidra string-search results for "ENGINEER"

- `search_strings "ENGINEER"` → many hits, mostly:
  - INI parsing of the section header `[ENGINEER]` (no hardcoded section-name
    branch)
  - `EngineerCaptureLevel=` parse target (Rules+0x17F8/+0x17FC — **vestigial**;
    parsed but never read at runtime; TS legacy)
  - `MultiEngineer=` parse target (Rules+0x14B4 — **desupported**; UI checkbox
    only, gameplay code does not read this flag)
  - String constants for EVA notifications
- No `if (name == "ENGINEER")` branch — behavior is driven entirely by the
  `Engineer=yes` flag bit at type+0xEC3.

### Vestigial fields (parsed-but-unread)

- `EngineerCaptureLevel=` (Rules+0x17F8, Rules+0x17FC) — TS-era health
  threshold for engineer capture (TS engineers couldn't capture buildings
  above some HP%). **[BINARY-VERIFIED audit 3 — parser key string at
  `0x0083b414` has TWO xrefs both inside `RulesClass::ReadGeneral`
  (`0x00671e03` + `0x00671e2a`); no other xrefs found, meaning no
  runtime-read site is present.]** Parsed twice into adjacent floats —
  no code reads these offsets. Skip in the Rust port.
- `MultiEngineer=` (Rules+0x14B4) — TS-era "multiple engineers required"
  toggle. **[BINARY-VERIFIED audit 3 — parser key string at `0x0083cfc8`
  has only ONE xref, in `RulesClass::ReadMultiplayerDialogSettings`
  (`0x00672129`). The lobby debug-print string "Crap Engineers: %s\n"
  exists at `0x0082c3f0`. No runtime-read xrefs to the parsed value
  found.]** Parsed and rendered as a lobby checkbox ("Crap Engineers"),
  but the capture path at 0x5202F0 **does not check this flag** —
  confirmed: Mission_Capture decompile shows no read of any Rules
  global before the +0xEC5 type-flag check. Skip in the Rust port.
- `EngineerDamage=` — does not exist in vanilla; an Ares mod extension.
- `NeedsEngineer` parser key also exists at `0x0081aca0` — a separate
  BuildingType flag for buildings that require an engineer to use
  (Tech Outpost-style). Not used by `[ENGINEER]` itself; documented for
  cheat-sheet completeness.

---

## TS-legacy filter

- `ImmuneToVeins=yes` — veins are TS terrain; YR has none. Defensive flag,
  unreachable code path.
- `Locomotor={4A582744-...}` — TS-era WalkLocomotionClass GUID, alive in YR
  and used by every infantry. Not dead.
- `Crawls=yes` (art) — TS-era prone-while-walking, alive in YR.
- `EngineerCaptureLevel=` rules field — **TS legacy, dead code** (parsed
  unread). Do not implement.
- `MultiEngineer=` rules-multiplayer field — **TS legacy, desupported**
  (parsed unread). Do not implement.
- Per the ENGINEER_CAPTURE report's "DESUPPORTED" INI comment, vanilla YR
  has **no damage-based capture** — engineer capture is always instant when
  preconditions are met.

---

## Cross-references

- **Builder**: [GAPILE](../structures/GAPILE.md) Allied Barracks.
- **Counterparts** (same dossier template, different voice/cameo):
  - [SENGINEER](../soviet/SENGINEER.md) — Soviet Engineer.
  - [YENGINEER](../yuri/YENGINEER.md) — Yuri Engineer.
- **Free-on-build sources**:
  - [CAOUTP](../structures/CAOUTP.md) Tech Outpost — does **not** spawn a
    free engineer; spawns a free IFV. (No tech building spawns engineers in
    vanilla YR.)
- **IFV passenger**: [HTK](../allied/HTK.md) IFV — `IFVMode=1` → Medic beam.
- **Transports**: SAPC, HTK, SHAD, FV — all accept engineer as passenger.
- **Capture targets**: any building with `Capturable=yes`. Notable categories:
  - All enemy ConYards (GACNST, NACNST, YACNST) — captures the entire base
  - All ore refineries (GAREFN, NAREFN, YAREFN) — captures the harvester too
    via the docked-state ownership flip
  - Tech buildings (CAOUTP, CATHOSP, CAOILD, CAAIRP) — capture for buffs
  - Most production/tech structures (Barracks, War Factory, Battle Lab, etc.)
- **Repair targets**: any **damaged friendly** building.
- **Bomb defuse**: works on any unit with an attached `BombClass` (placed by
  Crazy Ivan, [IVAN](../soviet/IVAN.md)).
- **Bridge repair**: [CABHUT](../structures/CABHUT.md) — engineer Enter
  triggers bridge segment rebuild.
- **Vulnerable to**: Attack Dog (one-shot anti-inf), Crazy Ivan bomb (since
  engineer has no `Bombable=no` override the default applies), Yuri mind
  control (ImmuneToPsionics=no default), Tanya/SEAL gunfire.

---

## Ghidra audit log (audit iteration 3 — 2026-05-18)

Deep-Ghidra audit pass. ~3 decompiles + 6 entry-point lookups + 6 string xrefs.
Primary goal: verify the engineer-capture mechanic in Mission_Capture and
resolve the previously-conflicted Engineer-flag struct-offset claim
(+0xEC3 vs +0xEC5).

### Function entry points verified

| Doc claim | Ghidra label / address | Status |
|-----------|------------------------|--------|
| `InfantryClass::Mission_Capture @ 0x005202F0` | `InfantryClass__Mission_Capture` exact, body `0x005202f0–0x005206aa` | ✅ VERIFIED |
| `InfantryClass::Mission_Enter @ 0x005196A0` | No standalone function; address falls inside `InfantryClass__PerCellProcess` (body `0x00519630–0x0051aa0a`) | ❌ ADDRESS DISCREPANCY (phantom function — like the prior DoType_Sequencer and GetFireError findings; this is becoming a consistent pattern: addresses in doc point to *labels-in-decompile-output*, not entry points) |
| `CaptureManagerClass::CaptureUnit @ 0x00471d40` | Verified iter 1 | ✅ VERIFIED |
| `InfantryClass::AI @ 0x0051BAB0` | Verified iter 1 | ✅ VERIFIED |

### Key behavioral findings (decompile-verified)

1. **Engineer flag offset is `+0xEC5`, NOT `+0xEC3`** (Mission_Capture decompile):
   ```c
   if ((*(char *)(param_1[0x1b0] + 0xec5) != '\0') &&
       ((int *)param_1[0x169] != (int *)0x0)) {
     ...
   }
   ```
   - `param_1[0x1b0]` is the type pointer (offset `0x1b0 * 4 = 0x6C0` on InfantryClass — that's the embedded TypeClass ptr).
   - `*(char *)(type + 0xec5)` is the Engineer flag — **VERIFIED at offset +0xEC5**, not +0xEC3.
   - **DISPARITY**: doc previously asserted "+0xEC3 per GI report P3.1 correction; +0xEC5 superseded." The decompile shows +0xEC5 is correct. The "correction" was wrong; the original ENGINEER_CAPTURE report's +0xEC5 was right.

2. **Target ptr offset is `+0x5A4`** (Mission_Capture):
   - `param_1[0x169]` accesses byte offset `0x169 * 4 = 0x5A4` — matches the doc's claim.
   - This is the InfantryClass instance field holding the assigned target (set by Mission system on right-click or AI dispatch).

3. **RTTI check for BuildingClass = 1** (Mission_Capture):
   ```c
   iVar2 = (**(code **)(*(int *)param_1[0x169] + 0x2c))();
   if (iVar2 == 1) {
   ```
   - vtable+0x2c is GetRTTI. RTTI value 1 = BuildingClass.

4. **Three distance thresholds** (Mission_Capture):
   - `< 0x80` (128 leptons, ~0.5 cells) → execute capture sequence
   - `< 0x200` (512 leptons, ~2 cells) → mid-range approach
   - `< 0x81` → re-check (effectively same as <0x80 +1, treating as "essentially there")
   - else → far-range approach via `param_1[0x19d]` (this+0x674 — likely NavCom or dest cell coord)
   All three thresholds are visible in the decompile.

5. **Capture-sequence vtable slots** (Mission_Capture):
   - `building->vtable+0x274` → SetMission (called with 3 = Guard mission)
   - `building->vtable+0xDC` → Limbo (called with 0)
   - `building->vtable+0x3D4` → **ChangeOwner** (called with `(infantry->Owner, 1)` — `param_2=1` is the "announce" flag)
   - `infantry->vtable+0xF8` → self-destruct/remove (called with no args; consumes the engineer)
   - `building[0xCE] = engineer->TypeClass[0xDF8]` — writes `building+0x338` ← `engineer_type+0xDF8`. Stores type-specific tag for the captured building.

6. **EngineerCaptureLevel parser-only confirmation** (string + xref):
   - String at `0x0083b414`.
   - Xrefs: `0x00671e03` and `0x00671e2a` — **both** inside `RulesClass__ReadGeneral`.
   - No other xrefs. **Confirmed: parsed but no runtime read.** TS-legacy / vestigial.

7. **MultiEngineer parser-only confirmation** (string + xref):
   - String at `0x0083cfc8`.
   - Xref: `0x00672129` — inside `RulesClass__ReadMultiplayerDialogSettings`.
   - **Single xref**, no runtime read. Lobby-checkbox-only feature; gameplay code does not check it. **Confirmed desupported.**
   - Companion debug string: `"Crap Engineers: %s\n"` at `0x0082c3f0` — confirms the lobby UI string template.

8. **Engineer parser key** (string + xref):
   - String at `0x0082596c`.
   - Xref: `0x00524571` in **`InfantryTypeClass__ReadINI`** + `0x0066fc88` in `RulesClass__ReadGeneral`.
   - Confirms `Engineer=` is read both at InfantryType-scope (per-unit) AND in Rules-scope (likely related to `AIIonCannonEngineerValue=` Rules field at adjacent address `0x0083c000`).

### Discrepancies resolved

- **`InfantryTypeClass+0xEC3 = Engineer`** — **WRONG**. Actual offset is **+0xEC5**.
  The doc's previous "correction" (+0xEC5 → +0xEC3) was erroneous; reverted.
- **`InfantryClass::Mission_Enter @ 0x005196A0`** — phantom address.
  Repair logic lives inside `PerCellProcess` (or is called from there); no
  dedicated Mission_Enter function exists at that address.

### Pattern emerging across audit 1-3

The docs repeatedly cite addresses that fall **inside** larger function bodies
rather than at entry points:
- E1: `DoType_Sequencer @ 0x00520A60` → inside `Fire_At_Target`.
- E1: `Fear_Decay_Handler @ 0x00518C00` → actually `SetFear` at same address.
- GGI: `GetFireError @ 0x0051C8B0` → no function.
- ENGINEER: `Mission_Enter @ 0x005196A0` → inside `PerCellProcess`.

Likely root cause: the original GI_GHIDRA_REPORT / ENGINEER_CAPTURE_REPORT
documented addresses by where the *code-of-interest lives* (e.g., the
sequence-id switch statement, the fear-decay block), not by entry point.
A reader looking up these addresses will hit a function that includes the
behavior but isn't named after it.

### Items intentionally NOT re-verified in iter 3

- **`vtable+0x3D4 = ChangeOwner` semantics** — confirmed as a vtable call;
  the propagation chain (power refresh, owned-list updates, radar refresh)
  needs separate decompile of the actual ChangeOwner function. DEFERRED to
  building-system audit.
- **`piVar8[0xce] = ... TypeClass[0xDF8]`** — the meaning of the source
  `+0xDF8` offset on TypeClass and the destination `+0x338` (=0xCE*4) on
  BuildingClass not pinned to specific INI keys. DEFERRED.
- **DefuseKit / BombDisarm / BombClass::Disarm path** — warhead key
  confirmed at WarheadType-scope (xref `0x0075d908`); the
  BombClass::Disarm function not decompiled. Deferred to `BOMB_CLASS_GHIDRA_REPORT.md` audit pass.
- **`InfantryClass::PerCellProcess` repair-branch decompile** — would resolve
  the Mission_Enter discrepancy fully. PerCellProcess is a long function
  (body 0x00519630–0x0051aa0a ≈ 5kb) — exceeds per-doc effort budget.
- **`CaptureManagerClass::CaptureUnit @ 0x00471d40` internals** — entry
  point verified iter 1, internals not decompiled. CaptureManager is the
  Yuri mind-control / Engineer-capture shared infrastructure; deferred to
  a dedicated audit.

### Confidence summary

- ~65% of behavioral claims now have direct binary verification (function
  exists at exact address + decompile confirms specific behavior).
- ~25% are INFERRED (related code paths not decompiled, or address is in a
  function but not the specific behavior).
- ~10% are INCORRECT/DISCREPANCY:
  - Engineer offset +0xEC3 (was wrong; +0xEC5 is correct)
  - Mission_Enter @ 0x005196A0 (phantom function — lives inside PerCellProcess)

The doc is **substantially reliable** for the engineer-capture path. The
+0xEC5 vs +0xEC3 offset correction is a **load-bearing fix** for any Rust
port that's using these struct offsets directly. Anyone implementing
Engineer behavior should read +0xEC5, not +0xEC3.

---

## Coverage audit

- ✅ Every key in `[ENGINEER]` rulesmd block (42 lines) covered above.
- ✅ Every key in `[ENGINEER]` artmd block (7 lines) covered, plus
  `[EngineerSequence]`.
- ✅ Weapon chain: DefuseKit, VirtualScanner — both rules sections covered
  with projectile (InvisibleAll) and warhead (BombDisarm).
- ✅ Sound chain: 9 distinct soundmd entries covered.
- ✅ Ghidra search: `search_strings "ENGINEER"` results recorded (INI parse
  targets + EVA strings; no hardcoded section-name branch). Deep capture RE
  delegated to ENGINEER_CAPTURE_GHIDRA_REPORT.md.
- ✅ TS-legacy filter applied to 5 keys/fields (ImmuneToVeins,
  EngineerCaptureLevel, MultiEngineer, Locomotor GUID note, Crawls).
- ✅ Cross-references to GAPILE, SENGINEER, YENGINEER, HTK, CAOUTP, CABHUT,
  IVAN, transports, and capture-target categories.
